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
use uuid::Uuid; // Added for order ID generation

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
    market_data_store: Arc<RwLock<MarketDataStore>>,
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

        let timeframe = TimeFrame::Day1;

        let candles = match BrazilianCsvParser::load_candles_from_csv(&req.file_path, &req.symbol) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(symbol = %req.symbol, path = %req.file_path, error_detail = ?e, "Failed during CSV loading process");
                return Err(e.into());
            }
        };

        let candles_loaded = candles.len() as i32;
        let mut store = self.market_data_store.write().await;

        match store.add_candles(&req.symbol, timeframe, candles) {
            Ok(_) => {
                tracing::info!(symbol = %req.symbol, count = candles_loaded, "Successfully loaded and stored CSV data");
                Ok(Response::new(LoadCsvResponse {
                    success: true,
                    message: format!("Loaded {} candles for symbol {}", candles_loaded, req.symbol),
                    candles_loaded,
                }))
            }
            Err(e) => {
                tracing::error!(symbol = %req.symbol, error_detail = ?e, "Error storing candles");
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

        let timeframe = TimeFrame::Day1;

        let from_ts = match from_grpc_timestamp(req.from_timestamp) {
            Ok(ts) => ts,
            Err(e) => return Err(e.into()),
        };
        let to_ts = match from_grpc_timestamp(req.to_timestamp) {
            Ok(ts) => ts,
            Err(e) => return Err(e.into()),
        };

        let store = self.market_data_store.read().await;
        let candles = store.get_candles(&req.symbol, timeframe, Some(from_ts), Some(to_ts));

        let (tx, rx) = mpsc::channel(4);

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
            } else {
                tracing::warn!(symbol = %req.symbol, ?timeframe, "No market data available (symbol/timeframe not found in store).");
                let status = Status::not_found(format!(
                    "Market data not found for symbol '{}' and timeframe {:?}",
                    req.symbol, timeframe
                ));
                if let Err(e) = tx.send(Err(status)).await {
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
            parameters = %req.parameters,
            "Received CalculateIndicatorRequest"
        );

        let timeframe = TimeFrame::Day1;
        let store = self.market_data_store.read().await;
        let candles = store.get_candles(&req.symbol, timeframe, None, None);

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

        let indicator_calculator: Box<dyn IndicatorCalculator> = match req.indicator_type.to_lowercase().as_str() {
            "sma" => {
                let period = params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                if period == 0 {
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

        Ok(Response::new(IndicatorResponse {
            indicator_name: indicator_calculator.name().to_string(),
            values,
        }))
    }

    async fn simulate_trade(&self, request: Request<TradeRequest>) -> Result<Response<TradeResponse>, Status> {
        let req = request.into_inner();
        tracing::info!(
            symbol = %req.symbol,
            action = %req.action,
            quantity = req.quantity,
            order_type = %req.order_type,
            price = ?req.price,
            "Received SimulateTradeRequest"
        );

        let order_id = Uuid::new_v4().to_string();
        let timeframe = TimeFrame::Day1;

        let store = self.market_data_store.read().await;
        let candles_opt = store.get_candles(&req.symbol, timeframe, None, None);

        // This early return is for when no market data exists at all for the symbol/timeframe
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
        let latest_candle = match candles.last() {
            Some(c) => c.clone(),
            None => {
                // This case should be practically unreachable if the above check passed.
                let err_msg = format!("Logic error: candles list was non-empty for symbol '{}' but last() is None.", req.symbol);
                tracing::error!("{}", err_msg);
                return Err(EngineError::MarketDataError(err_msg).into());
            }
        };
        drop(store); // Release read lock

        // Determine trade outcome
        let (success, filled_price, message_detail) = match req.order_type.to_uppercase().as_str() {
            "MARKET" => {
                let price = latest_candle.close;
                let msg = format!(
                    "Market {} order for {} of {} simulated at {:.2}",
                    req.action.to_uppercase(), req.quantity, req.symbol, price
                );
                (true, price, msg)
            }
            "LIMIT" => {
                match req.price {
                    Some(limit_price) => {
                        match req.action.to_uppercase().as_str() {
                            "BUY" => {
                                if latest_candle.low <= limit_price {
                                    let msg = format!(
                                        "Limit BUY order for {} of {} simulated at {:.2}",
                                        req.quantity, req.symbol, limit_price
                                    );
                                    (true, limit_price, msg)
                                } else {
                                    let msg = format!(
                                        "Limit BUY order for {} not filled: market low {:.2} did not reach limit price {:.2}",
                                        req.symbol, latest_candle.low, limit_price
                                    );
                                    (false, 0.0, msg)
                                }
                            }
                            "SELL" => {
                                if latest_candle.high >= limit_price {
                                    let msg = format!(
                                        "Limit SELL order for {} of {} simulated at {:.2}",
                                        req.quantity, req.symbol, limit_price
                                    );
                                    (true, limit_price, msg)
                                } else {
                                    let msg = format!(
                                        "Limit SELL order for {} not filled: market high {:.2} did not reach limit price {:.2}",
                                        req.symbol, latest_candle.high, limit_price
                                    );
                                    (false, 0.0, msg)
                                }
                            }
                            _ => {
                                let msg = format!("Unknown action '{}' for LIMIT order. Use 'BUY' or 'SELL'.", req.action);
                                (false, 0.0, msg)
                            }
                        }
                    }
                    None => {
                        // This specific "business logic error" (missing price for limit) results in a TradeResponse with success=false
                        // It does not return early from the main function with an Err(Status).
                        let msg = "Limit price is required for LIMIT orders.".to_string();
                        (false, 0.0, msg)
                    }
                }
            }
            _ => {
                let msg = format!("Unsupported order type: '{}'. Use 'MARKET' or 'LIMIT'.", req.order_type);
                (false, 0.0, msg)
            }
        };

        // Construct and log final response
        if success {
            tracing::info!(
                order_id = %order_id,
                symbol = %req.symbol,
                action = %req.action,
                order_type = %req.order_type,
                quantity = req.quantity,
                filled_price,
                message = %message_detail,
                "Trade simulated successfully"
            );
            Ok(Response::new(TradeResponse {
                success: true,
                message: message_detail,
                order_id,
                filled_price,
                filled_quantity: req.quantity,
            }))
        } else {
            tracing::warn!(
                order_id = %order_id,
                symbol = %req.symbol,
                action = %req.action,
                order_type = %req.order_type,
                price = ?req.price, // Log original requested price for context on failure
                failure_reason = %message_detail,
                "Trade simulation failed"
            );
            Ok(Response::new(TradeResponse {
                success: false,
                message: message_detail,
                order_id,
                filled_price: 0.0, // No fill, so 0.0
                filled_quantity: 0.0, // No fill
            }))
        }
    }
} // This is the single, correct closing brace for impl TradingEngine

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tonic::Request;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use chrono::Utc;

    fn create_test_engine() -> MyTradingEngine {
        let market_data_store = Arc::new(RwLock::new(MarketDataStore::new()));
        MyTradingEngine::new(market_data_store)
    }

    async fn create_test_engine_with_candle(symbol: &str, candle: DomainCandle) -> MyTradingEngine {
        let engine = create_test_engine();
        let mut store = engine.market_data_store.write().await;
        store.add_candles(symbol, TimeFrame::Day1, vec![candle]).unwrap();
        drop(store);
        engine
    }

    fn create_dummy_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", content).unwrap();
        file.flush().unwrap();
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
        assert!(status.message().contains("I/O error"));
    }

    #[tokio::test]
    async fn test_load_csv_data_parsing_error_bad_content() {
        let engine = create_test_engine();
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
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("CSV parsing system error"));
        assert!(status.message().contains("record 1 (line 2) has 4 fields, but the header has 9"));
    }

    #[tokio::test]
    async fn test_load_csv_data_bad_data_format_correct_columns() {
        let engine = create_test_engine();
        let csv_content = "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade\nWINFUT;30/12/2024;18:20:00;NOT_A_NUMBER;124.090;123.938;123.983;600.822.115,84;24.228";
        let tmp_file = create_dummy_csv(csv_content);
        let file_path = tmp_file.path().to_str().unwrap().to_string();

        let request = Request::new(LoadCsvRequest {
            file_path: file_path.clone(),
            symbol: "WINFUT".to_string(),
        });

        let result = engine.load_csv_data(request).await;
        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("CSV data format error"));
        assert!(status.message().contains("Error parsing 'Abertura'"));
        assert!(status.message().contains("Failed to parse decimal 'NOT_A_NUMBER'"));
    }

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
            timestamp: Utc::now(),
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
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;

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
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let limit_price = 99.0;

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
        let candle = sample_candle("TEST", 100.0, 102.0, 99.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let limit_price = 98.0;

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
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let limit_price = 101.5;

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
        let candle = sample_candle("TEST", 100.0, 101.0, 98.0, 100.5);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let limit_price = 101.5;

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
        let engine = create_test_engine();
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 1.0,
            price: None,
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert_eq!(response.message, "Limit price is required for LIMIT orders."); // Exact match
    }

    #[tokio::test]
    async fn test_simulate_trade_unsupported_order_type() {
        let engine = create_test_engine();
        let order_type = "FOOBAZ".to_string();
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 1.0,
            price: None,
            order_type: order_type.clone(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert_eq!(response.message, format!("Unsupported order type: '{}'. Use 'MARKET' or 'LIMIT'.", order_type)); // Exact match
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_unknown_action() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let action = "HOLD".to_string();
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: action.clone(),
            quantity: 1.0,
            price: Some(100.0),
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert_eq!(response.message, format!("Unknown action '{}' for LIMIT order. Use 'BUY' or 'SELL'.", action)); // Exact match
    }
}
```

Note: I've also updated the assertions in the tests `test_simulate_trade_limit_no_price`, `test_simulate_trade_unsupported_order_type`, and `test_simulate_trade_limit_unknown_action` to use `assert_eq!` for the exact message, as these messages are now quite stable from the refactored `simulate_trade`. I also fixed the assertion for `test_load_csv_data_parsing_error_bad_content` to correctly check for the unequal lengths message.
