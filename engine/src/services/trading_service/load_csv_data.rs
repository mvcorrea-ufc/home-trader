// Handler for the LoadCsvData RPC
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Response, Status}; // Removed Request as it's not used directly here

use crate::data::csv_parser::BrazilianCsvParser;
use crate::data::market_data::MarketDataStore;
use crate::error::EngineError;
use crate::services::{LoadCsvRequest, LoadCsvResponse}; // These come from services/mod.rs
use shared::models::TimeFrame;

pub async fn handle_load_csv_data(
    req_payload: LoadCsvRequest, // Changed from req to req_payload for clarity
    market_data_store: Arc<RwLock<MarketDataStore>>
) -> Result<Response<LoadCsvResponse>, Status> {
    // Original tracing::info for request reception is in the main trading_service.rs method
    // This handler can log its specific actions if needed, or we rely on the caller's log.
    // For now, let's assume the main method logs the initial reception.

    let timeframe = TimeFrame::Day1;

    let candles = match BrazilianCsvParser::load_candles_from_csv(&req_payload.file_path, &req_payload.symbol) {
        Ok(c) => c,
        Err(e) => {
            // Error already logged sufficiently by CsvParser or by the error mapping
            // tracing::error!(symbol = %req_payload.symbol, path = %req_payload.file_path, error_detail = ?e, "Failed during CSV loading process in handler");
            return Err(e.into());
        }
    };

    let candles_loaded = candles.len() as i32;
    let mut store = market_data_store.write().await;

    match store.add_candles(&req_payload.symbol, timeframe, candles) {
        Ok(_) => {
            // Success log can also be in the main method after this handler returns Ok.
            // tracing::info!(symbol = %req_payload.symbol, count = candles_loaded, "Successfully loaded and stored CSV data in handler");
            Ok(Response::new(LoadCsvResponse {
                success: true,
                message: format!("Loaded {} candles for symbol {}", candles_loaded, req_payload.symbol),
                candles_loaded,
            }))
        }
        Err(e) => {
            // Error already logged sufficiently by MarketDataStore or by the error mapping
            // tracing::error!(symbol = %req_payload.symbol, error_detail = ?e, "Error storing candles in handler");
            Err(EngineError::from(e).into())
        }
    }
}
