// Implementation of the TradingEngine gRPC service
use super::{
    TradingEngine, LoadCsvRequest, LoadCsvResponse,
    MarketDataRequest, MarketDataResponse, ProtoCandle as GrpcCandle, // Use the alias
    IndicatorRequest, IndicatorResponse,
    TradeRequest, TradeResponse,
};
use crate::data::csv_parser::BrazilianCsvParser;
use crate::data::market_data::MarketDataStore;
use crate::indicators::{IndicatorCalculator, Sma, Ema, Rsi}; // Assuming these are the indicator types for now
use shared::models::{Candle as DomainCandle, TimeFrame}; // Domain Candle
use tonic::{Request, Response, Status, Streaming};
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::RwLock; // Using tokio's RwLock for async safety

// Helper function to convert domain Candle to gRPC Candle
fn to_grpc_candle(domain_candle: &DomainCandle) -> GrpcCandle {
    GrpcCandle {
        symbol: domain_candle.symbol.clone(),
        timestamp: domain_candle.timestamp.timestamp_millis(),
        open: domain_candle.open,
        high: domain_candle.high,
        low: domain_candle.low,
        close: domain_candle.close,
        volume: domain_candle.volume,
        trades: domain_candle.trades,
    }
}

// Helper function to convert gRPC timestamp to chrono DateTime<Utc>
fn from_grpc_timestamp(ts_millis: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp_millis(ts_millis).unwrap_or_default() // Or handle error appropriately
}


pub struct MyTradingEngine {
    // Example: Store market data in memory. In a real app, this might be a database connection.
    // Arc<RwLock<...>> is used for shared mutable state in an async context.
    market_data_store: Arc<RwLock<MarketDataStore>>,
    // Potentially channels for multi-threaded processing as per spec, to be integrated later.
}

impl MyTradingEngine {
    pub fn new(market_data_store: Arc<RwLock<MarketDataStore>>) -> Self {
        MyTradingEngine { market_data_store }
    }
}

#[tonic::async_trait]
impl TradingEngine for MyTradingEngine {
    async fn load_csv_data(&self, request: Request<LoadCsvRequest>) -> Result<Response<LoadCsvResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Received LoadCsvRequest for symbol '{}', path '{}'", req.symbol, req.file_path);

        // For now, using a placeholder TimeFrame. This should ideally come from request or config.
        let timeframe = TimeFrame::Day1; // Example, make this configurable or part of request

        match BrazilianCsvParser::load_candles_from_csv(&req.file_path, &req.symbol) {
            Ok(candles) => {
                let candles_loaded = candles.len() as i32;
                let mut store = self.market_data_store.write().await;
                match store.add_candles(&req.symbol, timeframe, candles) {
                    Ok(_) => {
                        tracing::info!("Successfully loaded {} candles for symbol '{}'", candles_loaded, req.symbol);
                        Ok(Response::new(LoadCsvResponse {
                            success: true,
                            message: format!("Loaded {} candles for symbol {}", candles_loaded, req.symbol),
                            candles_loaded,
                        }))
                    }
                    Err(e) => {
                        tracing::error!("Error storing candles for symbol '{}': {}", req.symbol, e);
                        Err(Status::internal(format!("Error storing candles: {}", e)))
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error parsing CSV for symbol '{}', path '{}': {}", req.symbol, req.file_path, e);
                Err(Status::internal(format!("Failed to parse CSV: {}", e)))
            }
        }
    }

    type GetMarketDataStream = mpsc::Receiver<Result<MarketDataResponse, Status>>;

    async fn get_market_data(&self, request: Request<MarketDataRequest>) -> Result<Response<Self::GetMarketDataStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Received GetMarketDataRequest for symbol '{}'", req.symbol);

        // For now, using a placeholder TimeFrame. This should ideally come from request or config.
        let timeframe = TimeFrame::Day1; // Example

        let from_ts = from_grpc_timestamp(req.from_timestamp);
        let to_ts = from_grpc_timestamp(req.to_timestamp);

        let store = self.market_data_store.read().await;
        let candles = store.get_candles(&req.symbol, timeframe, Some(from_ts), Some(to_ts));

        let (mut tx, rx) = mpsc::channel(4); // Buffer size for the stream

        tokio::spawn(async move {
            if let Some(domain_candles) = candles {
                if domain_candles.is_empty() {
                    tracing::warn!("No market data found for symbol '{}' in the given range.", req.symbol);
                     // Send an empty response if no data, or handle as an error/specific status
                    let response = MarketDataResponse { candles: vec![] };
                    if let Err(e) = tx.send(Ok(response)).await {
                        tracing::error!("Failed to send empty market data to stream: {:?}", e);
                    }
                    return;
                }
                let grpc_candles: Vec<GrpcCandle> = domain_candles.iter().map(to_grpc_candle).collect();
                // Stream data in chunks if necessary, here sending all at once
                let response = MarketDataResponse { candles: grpc_candles };
                if let Err(e) = tx.send(Ok(response)).await {
                    tracing::error!("Failed to send market data to stream: {:?}", e);
                }
            } else {
                tracing::warn!("No market data available for symbol '{}'", req.symbol);
                // Optionally send an error or an empty response
                // For now, the stream will just close if no data is found.
                // Consider sending a specific status or empty MarketDataResponse.
                // Example: tx.send(Err(Status::not_found("Market data not found"))).await;
            }
        });

        Ok(Response::new(rx))
    }

    async fn calculate_indicator(&self, request: Request<IndicatorRequest>) -> Result<Response<IndicatorResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Received CalculateIndicatorRequest for symbol '{}', type '{}'", req.symbol, req.indicator_type);

        let store = self.market_data_store.read().await;
        // Assuming Day1 timeframe for now, this should be more dynamic
        let candles = store.get_candles(&req.symbol, TimeFrame::Day1, None, None);

        if candles.is_none() || candles.as_ref().unwrap().is_empty() {
            return Err(Status::not_found(format!("No data for symbol {}", req.symbol)));
        }
        let candle_data = candles.unwrap();

        // Parse parameters - example, very basic
        let params: serde_json::Value = match serde_json::from_str(&req.parameters) {
            Ok(p) => p,
            Err(_) => return Err(Status::invalid_argument("Invalid JSON parameters")),
        };

        let indicator_calculator: Box<dyn IndicatorCalculator> = match req.indicator_type.to_lowercase().as_str() {
            "sma" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                Box::new(Sma::new(period))
            }
            "ema" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                Box::new(Ema::new(period))
            }
            "rsi" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                Box::new(Rsi::new(period))
            }
            _ => return Err(Status::invalid_argument(format!("Unknown indicator type: {}", req.indicator_type))),
        };

        let values = indicator_calculator.calculate(&candle_data)
            .into_iter()
            .filter_map(|v| v) // Remove None values, or handle them as per spec (e.g., send NaN or specific value)
            .collect();

        Ok(Response::new(IndicatorResponse {
            indicator_name: indicator_calculator.name().to_string(),
            values,
        }))
    }

    async fn simulate_trade(&self, request: Request<TradeRequest>) -> Result<Response<TradeResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Received SimulateTradeRequest for symbol '{}', action '{}'", req.symbol, req.action);
        // TODO: Implement trade simulation logic
        // This would involve:
        // 1. Checking account balance/positions (if stateful)
        // 2. Determining execution price based on current market data
        // 3. Updating portfolio/balance
        // 4. Storing trade record

        // Placeholder implementation
        Ok(Response::new(TradeResponse {
            success: true,
            message: "Trade simulated successfully (placeholder)".to_string(),
            order_id: "sim_order_123".to_string(),
            filled_price: req.price.unwrap_or(100.0), // Example price
            filled_quantity: req.quantity,
        }))
    }
}
