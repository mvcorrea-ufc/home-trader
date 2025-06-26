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
use tokio_stream::wrappers::ReceiverStream;
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
        trades: domain_candle.trades as i32,
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

    type GetMarketDataStream = ReceiverStream<Result<MarketDataResponse, Status>>;

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

        Ok(Response::new(ReceiverStream::new(rx)))
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

#[cfg(test)]
mod tests {
    use super::*;
    // MarketDataStore is in super scope. Explicit for clarity if needed:
    // use crate::data::market_data::MarketDataStore;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    // DomainCandle is already an alias for shared::models::Candle in the outer scope.
    // use shared::models::Candle as DomainCandle;
    use tonic::Request;
    use tempfile::NamedTempFile;
    use std::io::Write;

    // Helper to create a MyTradingEngine instance with a fresh MarketDataStore
    fn create_test_engine() -> MyTradingEngine {
        let market_data_store = Arc::new(RwLock::new(MarketDataStore::new()));
        MyTradingEngine::new(market_data_store)
    }

    // Helper to create a dummy CSV file
    fn create_dummy_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", content).unwrap();
        file.flush().unwrap(); // Ensure content is written to disk
        file
    }

    #[tokio::test]
    async fn test_load_csv_data_success() {
        let engine = create_test_engine();
        let csv_content = "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade\nWINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;600.822.115,84;24.228";
        let tmp_file = create_dummy_csv(csv_content);
        let file_path = tmp_file.path().to_str().unwrap().to_string();

        let request = Request::new(LoadCsvRequest {
            file_path: file_path.clone(),
            symbol: "WINFUT".to_string(),
        });

        let response = engine.load_csv_data(request).await.unwrap().into_inner();

        assert!(response.success);
        assert_eq!(response.candles_loaded, 1);
        assert!(response.message.contains("Loaded 1 candles"));

        // Verify data in store
        let store = engine.market_data_store.read().await;
        let candles_in_store = store.get_candles("WINFUT", TimeFrame::Day1, None, None);
        assert!(candles_in_store.is_some());
        assert_eq!(candles_in_store.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_load_csv_data_parsing_error_file_not_found() {
        let engine = create_test_engine();
        let request = Request::new(LoadCsvRequest {
            file_path: "non_existent_file.csv".to_string(),
            symbol: "TEST".to_string(),
        });

        let result = engine.load_csv_data(request).await;
        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("Failed to parse CSV"));
        assert!(status.message().contains("Failed to open CSV file"));
    }

    #[tokio::test]
    async fn test_load_csv_data_parsing_error_bad_content() {
        let engine = create_test_engine();
        // Malformed data: "NOT_A_NUMBER" for Abertura, and also missing columns compared to header.
        // The parser will likely fail on "Missing 'Máximo' field" first due to column count mismatch after header.
        let csv_content = "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade\nWINFUT;30/12/2024;18:20:00;NOT_A_NUMBER";
        let tmp_file = create_dummy_csv(csv_content);
        let file_path = tmp_file.path().to_str().unwrap().to_string();

        let request = Request::new(LoadCsvRequest {
            file_path: file_path.clone(),
            symbol: "WINFUT".to_string(),
        });

        let result = engine.load_csv_data(request).await;
        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("Failed to parse CSV"));
        // The error from csv_parser for missing fields is specific.
        assert!(status.message().contains("Missing 'Máximo' field"));
    }

    // Note: Testing the "Error storing candles" case is harder without deeper mocking
    // of MarketDataStore or making its error conditions easily triggerable.
    // For now, the success and parsing error cases provide good coverage for the RPC endpoint logic itself.
}
