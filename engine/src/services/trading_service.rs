// Implementation of the TradingEngine gRPC service
use super::{
    TradingEngine, LoadCsvRequest, LoadCsvResponse,
    MarketDataRequest, MarketDataResponse, ProtoCandle as GrpcCandle, // Use the alias
    IndicatorRequest, IndicatorResponse,
    TradeRequest, TradeResponse,
};
use crate::data::csv_parser::BrazilianCsvParser;
use crate::data::market_data::MarketDataStore;
use crate::error::EngineError; // Import EngineError
use crate::indicators::{IndicatorCalculator, Sma, Ema, Rsi};
use shared::models::{Candle as DomainCandle, TimeFrame};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
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
fn from_grpc_timestamp(ts_millis: i64) -> Result<chrono::DateTime<chrono::Utc>, EngineError> {
    chrono::DateTime::from_timestamp_millis(ts_millis)
        .ok_or_else(|| EngineError::ProcessingError(format!("Invalid gRPC timestamp: {}", ts_millis)))
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
        tracing::info!(
            symbol = %req.symbol,
            path = %req.file_path,
            "Received LoadCsvRequest"
        );

        // For now, using a placeholder TimeFrame. This should ideally come from request or config.
        let timeframe = TimeFrame::Day1; // Example, make this configurable or part of request

        // BrazilianCsvParser::load_candles_from_csv now returns Result<_, EngineError>
        let candles = match BrazilianCsvParser::load_candles_from_csv(&req.file_path, &req.symbol) {
            Ok(c) => c,
            Err(e) => { // e is EngineError
                // Specific context logged here, EngineError itself is logged when converted to Status
                tracing::error!(symbol = %req.symbol, path = %req.file_path, error_detail = ?e, "Failed during CSV loading process");
                return Err(e.into()); // Converts EngineError to tonic::Status
            }
        };

        let candles_loaded = candles.len() as i32;
        let mut store = self.market_data_store.write().await;

        // Assuming MarketDataStore::add_candles might return its own error type, e.g., Result<(), MarketDataError>
        // For now, let's assume it returns anyhow::Result<()> which can be mapped to EngineError
        match store.add_candles(&req.symbol, timeframe, candles) {
            Ok(_) => {
                tracing::info!(symbol = %req.symbol, count = candles_loaded, "Successfully loaded and stored CSV data");
                Ok(Response::new(LoadCsvResponse {
                    success: true,
                    message: format!("Loaded {} candles for symbol {}", candles_loaded, req.symbol),
                    candles_loaded,
                }))
            }
            Err(e) => { // e is anyhow::Error from MarketDataStore::add_candles
                tracing::error!(symbol = %req.symbol, error_detail = ?e, "Error storing candles");
                // Convert anyhow::Error directly to EngineError::AnyhowError then to Status
                Err(EngineError::from(e).into())
            }
        }
    }

    type GetMarketDataStream = ReceiverStream<Result<MarketDataResponse, Status>>;

    async fn get_market_data(&self, request: Request<MarketDataRequest>) -> Result<Response<Self::GetMarketDataStream>, Status> {
        let req = request.into_inner();
        tracing::info!(
            symbol = %req.symbol,
            from_timestamp_ms = req.from_timestamp,
            to_timestamp_ms = req.to_timestamp,
            "Received GetMarketDataRequest"
        );

        // For now, using a placeholder TimeFrame. This should ideally come from request or config.
        let timeframe = TimeFrame::Day1; // Example

        let from_ts = match from_grpc_timestamp(req.from_timestamp) {
            Ok(ts) => ts,
            Err(e) => return Err(e.into()), // Convert EngineError to tonic::Status
        };
        let to_ts = match from_grpc_timestamp(req.to_timestamp) {
            Ok(ts) => ts,
            Err(e) => return Err(e.into()), // Convert EngineError to tonic::Status
        };

        let store = self.market_data_store.read().await;
        let candles = store.get_candles(&req.symbol, timeframe, Some(from_ts), Some(to_ts));

        let (tx, rx) = mpsc::channel(4); // Buffer size for the stream

        tokio::spawn(async move {
            if let Some(domain_candles) = candles {
                if domain_candles.is_empty() {
                    tracing::warn!(symbol = %req.symbol, ?timeframe, from_ts = ?from_ts, to_ts = ?to_ts, "No market data found in the given range.");
                    let response = MarketDataResponse { candles: vec![] };
                    if let Err(e) = tx.send(Ok(response)).await {
                        tracing::error!(error = ?e, "Failed to send empty market data to stream");
                    }
                    return;
                }
                let grpc_candles: Vec<GrpcCandle> = domain_candles.iter().map(to_grpc_candle).collect();
                tracing::debug!(symbol = %req.symbol, count = grpc_candles.len(), "Streaming market data.");
                let response = MarketDataResponse { candles: grpc_candles };
                if let Err(e) = tx.send(Ok(response)).await {
                    tracing::error!(error = ?e, "Failed to send market data to stream");
                }
            } else { // This means store.get_candles(&req.symbol, ...) returned None
                tracing::warn!(symbol = %req.symbol, ?timeframe, "No market data available (symbol/timeframe not found in store).");
                // Send a NotFound status over the channel
                let status = Status::not_found(format!(
                    "Market data not found for symbol '{}' and timeframe {:?}",
                    req.symbol, timeframe
                ));
                if let Err(e) = tx.send(Err(status)).await { // status is already Status::not_found
                    tracing::error!(error = ?e, "Failed to send NotFound status to stream");
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn calculate_indicator(&self, request: Request<IndicatorRequest>) -> Result<Response<IndicatorResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            symbol = %req.symbol,
            indicator_type = %req.indicator_type,
            parameters = %req.parameters, // Display formatting for JSON string is fine
            "Received CalculateIndicatorRequest"
        );

        let store = self.market_data_store.read().await;
        // Assuming Day1 timeframe for now, this should be more dynamic
        let candles = store.get_candles(&req.symbol, TimeFrame::Day1, None, None);

        if candles.is_none() || candles.as_ref().unwrap().is_empty() {
            tracing::warn!(
                symbol = %req.symbol,
                ?timeframe,
                "No candle data found to calculate indicator"
            );
            return Err(EngineError::MarketDataError(format!("No candle data found for symbol '{}' and timeframe {:?} to calculate indicator", req.symbol, timeframe)).into());
        }
        let candle_data = candles.unwrap();

        let params: serde_json::Value = match serde_json::from_str(&req.parameters) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    indicator_type = %req.indicator_type,
                    parameters = %req.parameters,
                    error_detail = ?e,
                    "Invalid JSON parameters for indicator"
                );
                return Err(EngineError::ProcessingError(format!("Invalid JSON parameters for indicator '{}': {}", req.indicator_type, e)).into());
            }
        };

        // TODO: Indicator instantiation might itself return a Result<_, EngineError> if period is invalid (e.g. 0)
        // Currently, Sma::new, etc. panic on period=0. This will turn into a 500 error.
        // A more graceful way would be for new() to return Result and map that to EngineError::IndicatorError.
        // For now, relying on the panic being caught by Tonic.
        let indicator_calculator: Box<dyn IndicatorCalculator> = match req.indicator_type.to_lowercase().as_str() {
            "sma" => {
                // Parameter parsing could also return EngineError
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                if period == 0 { // Defend against panic if new() doesn't (though ours do now)
                    return Err(EngineError::IndicatorError("Indicator period cannot be 0".to_string()).into());
                }
                Box::new(Sma::new(period))
            }
            "ema" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                 if period == 0 {
                    return Err(EngineError::IndicatorError("Indicator period cannot be 0".to_string()).into());
                }
                Box::new(Ema::new(period))
            }
            "rsi" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
                 if period == 0 {
                    return Err(EngineError::IndicatorError("Indicator period cannot be 0".to_string()).into());
                }
                Box::new(Rsi::new(period))
            }
            _ => {
                tracing::error!(indicator_type = %req.indicator_type, "Unknown indicator type requested");
                return Err(EngineError::IndicatorError(format!("Unknown indicator type: {}", req.indicator_type)).into());
            }
        };

        let values = indicator_calculator.calculate(&candle_data);
        // The `calculate` method now returns Vec<f64> with f64::NAN for undefined values.
        // These NaN values will be serialized as "NaN" in JSON if this response were JSON,
        // or handled as per protobuf spec for double (which supports NaN).

        Ok(Response::new(IndicatorResponse {
            indicator_name: indicator_calculator.name().to_string(),
            values,
        }))
    }

use uuid::Uuid; // Added for order ID generation

// ... (existing imports)

// ... (MyTradingEngine struct and new method) ...

#[tonic::async_trait]
impl TradingEngine for MyTradingEngine {
    // ... (load_csv_data, get_market_data, calculate_indicator methods) ...

    async fn simulate_trade(&self, request: Request<TradeRequest>) -> Result<Response<TradeResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            symbol = %req.symbol,
            action = %req.action,
            quantity = req.quantity,
            order_type = %req.order_type,
            price = ?req.price, // Debug format for Option<f64>
            "Received SimulateTradeRequest"
        );

        let order_id = Uuid::new_v4().to_string();
        let timeframe = TimeFrame::Day1; // Placeholder, consistent with other methods

        let store = self.market_data_store.read().await;
        // Fetch all candles to get the latest one. Could be optimized if store supports getting latest directly.
        let candles_opt = store.get_candles(&req.symbol, timeframe, None, None);

        if candles_opt.is_none() || candles_opt.as_ref().unwrap().is_empty() {
            tracing::warn!(symbol = %req.symbol, ?timeframe, "No market data available to simulate trade.");
            return Ok(Response::new(TradeResponse {
                success: false,
                message: format!("No market data available for symbol '{}' and timeframe {:?} to simulate trade.", req.symbol, timeframe),
                order_id,
                filled_price: 0.0,
                filled_quantity: 0.0,
            }));
        }

        let candles = candles_opt.unwrap();
        // Get the latest candle
        // .last() returns an Option, so we handle the case where it might be None (though prior checks make it unlikely)
        let latest_candle = match candles.last() {
            Some(c) => c.clone(), // Clone to work with it after dropping the read lock from store
            None => { // Should ideally not be reached if candles_opt.as_ref().unwrap().is_empty() was false
                let err_msg = format!("Logic error: candles list was non-empty for symbol '{}' but last() is None.", req.symbol);
                tracing::error!("{}", err_msg);
                return Err(EngineError::MarketDataError(err_msg).into());
            }
        };

        // Drop the read lock on the store as soon as candle data is retrieved and copied
        drop(store);

        let mut filled_price = 0.0;
        let mut success = false;
        let mut message = String::new();

        match req.order_type.to_uppercase().as_str() {
            "MARKET" => {
                filled_price = latest_candle.close; // Market order fills at last known close price
                success = true;
                message = format!(
                    "Market {} order for {} of {} simulated at {:.2}",
                    req.action.to_uppercase(), req.quantity, req.symbol, filled_price
                );
            }
            "LIMIT" => {
                let limit_price = match req.price {
                    Some(p) => p,
                    None => {
                        return Ok(Response::new(TradeResponse {
                            success: false,
                            message: "Limit price is required for LIMIT orders.".to_string(),
                            order_id,
                            filled_price: 0.0,
                            filled_quantity: 0.0,
                        }));
                    }
                };

                match req.action.to_uppercase().as_str() {
                    "BUY" => {
                        // Buy limit: fill if market price (latest_candle.low) went to or below your limit_price.
                        // Fill at your limit_price (or better, but simulation keeps it simple at limit_price).
                        if latest_candle.low <= limit_price {
                            filled_price = limit_price;
                            success = true;
                            message = format!(
                                "Limit BUY order for {} of {} simulated at {:.2}",
                                req.quantity, req.symbol, filled_price
                            );
                        } else {
                            message = format!(
                                "Limit BUY order for {} not filled: market low {:.2} did not reach limit price {:.2}",
                                req.symbol, latest_candle.low, limit_price
                            );
                        }
                    }
                    "SELL" => {
                        // Sell limit: fill if market price (latest_candle.high) went to or above your limit_price.
                        // Fill at your limit_price.
                        if latest_candle.high >= limit_price {
                            filled_price = limit_price;
                            success = true;
                            message = format!(
                                "Limit SELL order for {} of {} simulated at {:.2}",
                                req.quantity, req.symbol, filled_price
                            );
                        } else {
                            message = format!(
                                "Limit SELL order for {} not filled: market high {:.2} did not reach limit price {:.2}",
                                req.symbol, latest_candle.high, limit_price
                            );
                        }
                    }
                    _ => {
                        message = format!("Unknown action '{}' for LIMIT order. Use 'BUY' or 'SELL'.", req.action);
                    }
                }
            }
            _ => {
                message = format!("Unsupported order type: '{}'. Use 'MARKET' or 'LIMIT'.", req.order_type);
            }
        }

        if success {
            tracing::info!(
                order_id = %order_id,
                symbol = %req.symbol,
                action = %req.action,
                order_type = %req.order_type,
                quantity = req.quantity,
                filled_price, // This is f64, directly usable
                "Trade simulated successfully"
            );
            Ok(Response::new(TradeResponse {
                success: true,
                message, // This message already contains details
                order_id,
                filled_price,
                filled_quantity: req.quantity, // Assuming full quantity filled if successful
            }))
        } else {
            tracing::warn!(
                order_id = %order_id,
                symbol = %req.symbol,
                action = %req.action,
                order_type = %req.order_type,
                stale_message = %message, // Renamed to avoid conflict if message was a field name
                "Trade simulation failed"
            );
            Ok(Response::new(TradeResponse {
                success: false,
                message, // This message already contains details of why it failed
                order_id,
                filled_price: 0.0,
                filled_quantity: 0.0,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tonic::Request;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use chrono::{Utc, TimeZone};

    // Helper to create a MyTradingEngine instance with a fresh MarketDataStore
    fn create_test_engine() -> MyTradingEngine {
        let market_data_store = Arc::new(RwLock::new(MarketDataStore::new()));
        MyTradingEngine::new(market_data_store)
    }

    fn create_test_engine_with_candle(symbol: &str, candle: DomainCandle) -> MyTradingEngine {
        let engine = create_test_engine();
        let mut store = engine.market_data_store.blocking_write(); // Use blocking_write for test setup ease
        store.add_candles(symbol, TimeFrame::Day1, vec![candle]).unwrap();
        drop(store);
        engine
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

    #[tokio::test]
    async fn test_simulate_trade_no_market_data() {
        let engine = create_test_engine();
        let request = Request::new(TradeRequest {
            symbol: "NODATA".to_string(),
            action: "BUY".to_string(),
            quantity: 10.0,
            price: None,
            order_type: "MARKET".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("No market data available"));
    }

    fn sample_candle(symbol: &str, open: f64, high: f64, low: f64, close: f64) -> DomainCandle {
        DomainCandle {
            symbol: symbol.to_string(),
            timestamp: Utc::now(), // Exact timestamp isn't critical for this test logic
            open,
            high,
            low,
            close,
            volume: 1000.0,
            trades: 100,
        }
    }

    #[tokio::test]
    async fn test_simulate_trade_market_buy() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone());

        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 10.0,
            price: None,
            order_type: "MARKET".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(response.success);
        assert_eq!(response.filled_price, candle.close);
        assert_eq!(response.filled_quantity, 10.0);
        assert!(response.message.contains("Market BUY order"));
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_buy_fill() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0); // Low is 98.0
        let engine = create_test_engine_with_candle("TEST", candle.clone());
        let limit_price = 99.0; // Buy if price drops to 99.0 or lower

        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 5.0,
            price: Some(limit_price),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(response.success);
        assert_eq!(response.filled_price, limit_price);
        assert_eq!(response.filled_quantity, 5.0);
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_buy_no_fill() {
        let candle = sample_candle("TEST", 100.0, 102.0, 99.0, 101.0); // Low is 99.0
        let engine = create_test_engine_with_candle("TEST", candle.clone());
        let limit_price = 98.0; // Buy if price drops to 98.0 or lower - won't happen

        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 5.0,
            price: Some(limit_price),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("not filled"));
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_sell_fill() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0); // High is 102.0
        let engine = create_test_engine_with_candle("TEST", candle.clone());
        let limit_price = 101.5; // Sell if price rises to 101.5 or higher

        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "SELL".to_string(),
            quantity: 7.0,
            price: Some(limit_price),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(response.success);
        assert_eq!(response.filled_price, limit_price);
        assert_eq!(response.filled_quantity, 7.0);
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_sell_no_fill() {
        let candle = sample_candle("TEST", 100.0, 101.0, 98.0, 100.5); // High is 101.0
        let engine = create_test_engine_with_candle("TEST", candle.clone());
        let limit_price = 101.5; // Sell if price rises to 101.5 or higher - won't happen

        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "SELL".to_string(),
            quantity: 7.0,
            price: Some(limit_price),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("not filled"));
    }

     #[tokio::test]
    async fn test_simulate_trade_limit_no_price() {
        let engine = create_test_engine(); // No data needed as it should fail before data check
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 1.0,
            price: None, // Missing price for LIMIT order
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("Limit price is required"));
    }

    #[tokio::test]
    async fn test_simulate_trade_unsupported_order_type() {
        let engine = create_test_engine();
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 1.0,
            price: None,
            order_type: "FOOBAZ".to_string(), // Unsupported
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("Unsupported order type"));
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_unknown_action() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone());
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "HOLD".to_string(), // Unknown action
            quantity: 1.0,
            price: Some(100.0),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("Unknown action 'HOLD' for LIMIT order"));
    }
}
