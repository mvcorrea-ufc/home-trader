// engine/src/services/trading_service/mod.rs
// This file is the main module hub for the trading_service.
// It contains the MyTradingEngine struct, its impl blocks,
// and declares submodules for handlers and helpers.

// Use statements adjusted for the new module structure.
use super::{ // Imports from engine/src/services/mod.rs
    TradingEngine, LoadCsvRequest, LoadCsvResponse,
    MarketDataRequest, MarketDataResponse,
    IndicatorRequest, IndicatorResponse,
    TradeRequest, TradeResponse,
    // ProtoCandle as GrpcCandle, // Removed as unused at this top level
};
use crate::data::market_data::MarketDataStore;
// shared::models are moved to mod tests
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use std::sync::Arc;
use tokio::sync::RwLock;

// Declare submodules for handlers and helpers. These are now sibling modules.
pub mod helpers;
pub mod load_csv_data;
pub mod get_market_data;
pub mod calculate_indicator;
pub mod simulate_trade;

// MyTradingEngine struct definition
pub struct MyTradingEngine {
    market_data_store: Arc<RwLock<MarketDataStore>>,
}

// impl MyTradingEngine { new ... }
impl MyTradingEngine {
    pub fn new(market_data_store: Arc<RwLock<MarketDataStore>>) -> Self {
        MyTradingEngine { market_data_store }
    }
}

// impl TradingEngine for MyTradingEngine
#[tonic::async_trait]
impl TradingEngine for MyTradingEngine {
    async fn load_csv_data(&self, request: Request<LoadCsvRequest>) -> Result<Response<LoadCsvResponse>, Status> {
        let req_payload = request.into_inner();
        tracing::info!(
            symbol = %req_payload.symbol,
            path = %req_payload.file_path,
            "Received LoadCsvRequest in main service, dispatching to handler."
        );
        // Calls handler from sibling module
        load_csv_data::handle_load_csv_data(req_payload, self.market_data_store.clone()).await
    }

    type GetMarketDataStream = ReceiverStream<Result<MarketDataResponse, Status>>;
    async fn get_market_data(&self, request: Request<MarketDataRequest>) -> Result<Response<Self::GetMarketDataStream>, Status> {
        let req_payload = request.into_inner();
        tracing::info!(
            symbol = %req_payload.symbol,
            from_timestamp_ms = req_payload.from_timestamp,
            to_timestamp_ms = req_payload.to_timestamp,
            "Received GetMarketDataRequest in main service, dispatching to handler."
        );
        get_market_data::handle_get_market_data(req_payload, self.market_data_store.clone()).await
    }

    async fn calculate_indicator(&self, request: Request<IndicatorRequest>) -> Result<Response<IndicatorResponse>, Status> {
        let req_payload = request.into_inner();
        tracing::info!(
            symbol = %req_payload.symbol,
            indicator_type = %req_payload.indicator_type,
            parameters = %req_payload.parameters,
            "Received CalculateIndicatorRequest in main service, dispatching to handler."
        );
        calculate_indicator::handle_calculate_indicator(req_payload, self.market_data_store.clone()).await
    }

    async fn simulate_trade(&self, request: Request<TradeRequest>) -> Result<Response<TradeResponse>, Status> {
        let req_payload = request.into_inner();
        tracing::info!(
            symbol = %req_payload.symbol,
            action = %req_payload.action,
            quantity = req_payload.quantity,
            order_type = %req_payload.order_type,
            price = ?req_payload.price,
            "Received SimulateTradeRequest in main service, dispatching to handler."
        );
        simulate_trade::handle_simulate_trade(req_payload, self.market_data_store.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::market_data::MarketDataStore;
    use shared::models::{Candle as DomainCandle, TimeFrame}; // Moved here
    use tempfile::NamedTempFile;
    use std::io::Write;
    use chrono::Utc;
    // Removed: use crate::services::ProtoCandle as GrpcCandle; // This was causing unused import warning

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
        let request = Request::new(LoadCsvRequest { file_path: file_path.clone(), symbol: "WINFUT".to_string() });
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
        let request = Request::new(LoadCsvRequest { file_path: "non_existent_file.csv".to_string(), symbol: "TEST".to_string() });
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
        let request = Request::new(LoadCsvRequest { file_path: file_path.clone(), symbol: "WINFUT".to_string() });
        let result = engine.load_csv_data(request).await;
        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("CSV parsing system error")); // Confirms our error mapping
        // More general checks for the underlying csv::Error details
        assert!(status.message().to_lowercase().contains("fields"));
        assert!(status.message().to_lowercase().contains("header"));
    }

    #[tokio::test]
    async fn test_load_csv_data_bad_data_format_correct_columns() {
        let engine = create_test_engine();
        let csv_content = "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade\nWINFUT;30/12/2024;18:20:00;NOT_A_NUMBER;124.090;123.938;123.983;600.822.115,84;24.228";
        let tmp_file = create_dummy_csv(csv_content);
        let file_path = tmp_file.path().to_str().unwrap().to_string();
        let request = Request::new(LoadCsvRequest { file_path: file_path.clone(), symbol: "WINFUT".to_string() });
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
        let request = Request::new(TradeRequest { symbol: "NODATA".to_string(), action: "BUY".to_string(), quantity: 10.0, price: None, order_type: "MARKET".to_string() });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("No market data available"));
    }

    fn sample_candle(symbol: &str, open: f64, high: f64, low: f64, close: f64) -> DomainCandle {
        DomainCandle { symbol: symbol.to_string(), timestamp: Utc::now(), open, high, low, close, volume: 1000.0, trades: 100 }
    }

    #[tokio::test]
    async fn test_simulate_trade_market_buy() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let request = Request::new(TradeRequest { symbol: "TEST".to_string(), action: "BUY".to_string(), quantity: 10.0, price: None, order_type: "MARKET".to_string() });
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
        let request = Request::new(TradeRequest { symbol: "TEST".to_string(), action: "BUY".to_string(), quantity: 5.0, price: Some(limit_price), order_type: "LIMIT".to_string() });
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
        let request = Request::new(TradeRequest { symbol: "TEST".to_string(), action: "BUY".to_string(), quantity: 5.0, price: Some(limit_price), order_type: "LIMIT".to_string() });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("not filled"));
    }

    #[tokio::test]
    async fn test_simulate_trade_limit_sell_fill() {
        let candle = sample_candle("TEST", 100.0, 102.0, 98.0, 101.0);
        let engine = create_test_engine_with_candle("TEST", candle.clone()).await;
        let limit_price = 101.5;
        let request = Request::new(TradeRequest { symbol: "TEST".to_string(), action: "SELL".to_string(), quantity: 7.0, price: Some(limit_price), order_type: "LIMIT".to_string() });
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
        let request = Request::new(TradeRequest { symbol: "TEST".to_string(), action: "SELL".to_string(), quantity: 7.0, price: Some(limit_price), order_type: "LIMIT".to_string() });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert!(response.message.contains("not filled"));
    }

     #[tokio::test]
    async fn test_simulate_trade_limit_no_price() {
        let candle = sample_candle("TEST", 100.0, 101.0, 99.0, 100.0);
        let engine = create_test_engine_with_candle("TEST", candle).await;
        let request = Request::new(TradeRequest {
            symbol: "TEST".to_string(),
            action: "BUY".to_string(),
            quantity: 1.0,
            price: None,
            order_type: "LIMIT".to_string(),
        });
        let response = engine.simulate_trade(request).await.unwrap().into_inner();
        assert!(!response.success);
        assert_eq!(response.message, "Limit price is required for LIMIT orders.");
    }

    #[tokio::test]
    async fn test_simulate_trade_unsupported_order_type() {
        let candle = sample_candle("TEST", 100.0, 101.0, 99.0, 100.0);
        let engine = create_test_engine_with_candle("TEST", candle).await;
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
        assert_eq!(response.message, format!("Unsupported order type: '{}'. Use 'MARKET' or 'LIMIT'.", order_type));
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
        assert_eq!(response.message, format!("Unknown action '{}' for LIMIT order. Use 'BUY' or 'SELL'.", action));
    }
}
