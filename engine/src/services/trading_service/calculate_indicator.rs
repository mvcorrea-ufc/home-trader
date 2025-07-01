// Handler for the CalculateIndicator RPC
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Response, Status}; // Removed Request
use serde_json; // For Value

use crate::data::market_data::MarketDataStore;
use crate::services::{IndicatorRequest, IndicatorResponse};
use shared::models::TimeFrame; // DomainCandle not directly used due to IndicatorCalculator taking &[Candle]
use crate::error::EngineError;
use crate::indicators::{IndicatorCalculator, Sma, Ema, Rsi};

pub async fn handle_calculate_indicator(
    req_payload: IndicatorRequest,
    market_data_store: Arc<RwLock<MarketDataStore>>
) -> Result<Response<IndicatorResponse>, Status> {
    tracing::debug!(symbol = %req_payload.symbol, indicator_type = %req_payload.indicator_type, "Handling CalculateIndicatorRequest in dedicated handler");

    let timeframe = TimeFrame::Day1;
    let store = market_data_store.read().await;
    // .get_candles returns Option<Vec<DomainCandle>>, which is an owned type.
    let candles = store.get_candles(&req_payload.symbol, timeframe, None, None);
    drop(store); // Explicitly drop lock after data retrieval

    if candles.is_none() || candles.as_ref().unwrap().is_empty() {
        tracing::warn!(
            symbol = %req_payload.symbol,
            ?timeframe,
            "No candle data found to calculate indicator (handler)"
        );
        return Err(EngineError::MarketDataError(format!("No candle data found for symbol '{}' and timeframe {:?} to calculate indicator", req_payload.symbol, timeframe)).into());
    }
    let candle_data = candles.unwrap(); // Now this is Vec<DomainCandle>

    let params: serde_json::Value = match serde_json::from_str(&req_payload.parameters) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                indicator_type = %req_payload.indicator_type,
                parameters = %req_payload.parameters,
                error_detail = ?e,
                "Invalid JSON parameters for indicator (handler)"
            );
            return Err(EngineError::ProcessingError(format!("Invalid JSON parameters for indicator '{}': {}", req_payload.indicator_type, e)).into());
        }
    };

    let indicator_calculator: Box<dyn IndicatorCalculator> = match req_payload.indicator_type.to_lowercase().as_str() {
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
            tracing::error!(indicator_type = %req_payload.indicator_type, "Unknown indicator type requested (handler)");
            return Err(EngineError::IndicatorError(format!("Unknown indicator type: {}", req_payload.indicator_type)).into());
        }
    };

    // IndicatorCalculator::calculate expects &[DomainCandle]
    let values = indicator_calculator.calculate(&candle_data);

    Ok(Response::new(IndicatorResponse {
        indicator_name: indicator_calculator.name().to_string(),
        values,
    }))
}
