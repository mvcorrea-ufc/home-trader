// Handler for the GetMarketData RPC
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status}; // Removed Request
use tokio::sync::mpsc;

use crate::data::market_data::MarketDataStore;
// Assuming ProtoCandle is accessible from crate::services module where it's aliased
use crate::services::{MarketDataRequest, MarketDataResponse, ProtoCandle as GrpcCandle};
use shared::models::{/*Candle as DomainCandle,*/ TimeFrame}; // DomainCandle not directly used here due to helpers
use super::helpers::{to_grpc_candle, from_grpc_timestamp};
use crate::error::EngineError; // from_grpc_timestamp returns EngineError

pub async fn handle_get_market_data(
    req_payload: MarketDataRequest,
    market_data_store: Arc<RwLock<MarketDataStore>>
) -> Result<Response<ReceiverStream<Result<MarketDataResponse, Status>>>, Status> {
    // Main method logs initial reception.
    tracing::debug!(symbol = %req_payload.symbol, "Handling GetMarketDataRequest in dedicated handler");

    let timeframe = TimeFrame::Day1;

    let from_ts = match from_grpc_timestamp(req_payload.from_timestamp) {
        Ok(ts) => ts,
        Err(e) => {
            tracing::error!(symbol = %req_payload.symbol, error_detail = ?e, "Invalid 'from' timestamp in GetMarketDataRequest");
            return Err(e.into());
        }
    };
    let to_ts = match from_grpc_timestamp(req_payload.to_timestamp) {
        Ok(ts) => ts,
        Err(e) => {
            tracing::error!(symbol = %req_payload.symbol, error_detail = ?e, "Invalid 'to' timestamp in GetMarketDataRequest");
            return Err(e.into());
        }
    };

    let store = market_data_store.read().await;
    // .get_candles returns Option<Vec<DomainCandle>>, which is an owned type.
    // So, the read lock on `store` is released after this line if `candles` is used later without store.
    let candles = store.get_candles(&req_payload.symbol, timeframe, Some(from_ts), Some(to_ts));
    drop(store); // Explicitly drop lock after data retrieval

    let (tx, rx) = mpsc::channel(4);

    // Clone what's needed for the spawned task. `req_payload.symbol` for logging.
    // `timeframe`, `from_ts`, `to_ts` are Copy or simple types.
    let symbol_for_log = req_payload.symbol.clone();

    tokio::spawn(async move {
        if let Some(domain_candles) = candles { // `candles` is moved into the async block
            if domain_candles.is_empty() {
                tracing::warn!(symbol = %symbol_for_log, ?timeframe, from_ts = ?from_ts, to_ts = ?to_ts, "No market data found in the given range (handler).");
                let response = MarketDataResponse { candles: vec![] };
                if let Err(e) = tx.send(Ok(response)).await {
                    tracing::error!(error = ?e, symbol = %symbol_for_log, "Failed to send empty market data to stream (handler)");
                }
                return;
            }
            let grpc_candles: Vec<GrpcCandle> = domain_candles.iter().map(to_grpc_candle).collect();
            tracing::debug!(symbol = %symbol_for_log, count = grpc_candles.len(), "Streaming market data (handler).");
            let response = MarketDataResponse { candles: grpc_candles };
            if let Err(e) = tx.send(Ok(response)).await {
                tracing::error!(error = ?e, symbol = %symbol_for_log, "Failed to send market data to stream (handler)");
            }
        } else {
            tracing::warn!(symbol = %symbol_for_log, ?timeframe, "No market data available (symbol/timeframe not found in store) (handler).");
            let status_msg = format!(
                "Market data not found for symbol '{}' and timeframe {:?}",
                symbol_for_log, timeframe
            );
            // Send a NotFound status over the channel
            // The EngineError mapping in error.rs handles MarketDataError("...not found") to tonic::Status::not_found
            // However, here we are constructing Status directly for the stream.
            let status = Status::not_found(status_msg);
            if let Err(e) = tx.send(Err(status)).await {
                tracing::error!(error = ?e, symbol = %symbol_for_log, "Failed to send NotFound status to stream (handler)");
            }
        }
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}
