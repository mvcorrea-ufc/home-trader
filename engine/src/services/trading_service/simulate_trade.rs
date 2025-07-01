// Handler for the SimulateTrade RPC
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Response, Status}; // Removed Request
use uuid::Uuid;

use crate::data::market_data::MarketDataStore;
use crate::services::{TradeRequest, TradeResponse};
use shared::models::TimeFrame;
use crate::error::EngineError;

pub async fn handle_simulate_trade(
    req_payload: TradeRequest,
    market_data_store: Arc<RwLock<MarketDataStore>>
) -> Result<Response<TradeResponse>, Status> {
    tracing::debug!(symbol = %req_payload.symbol, action = %req_payload.action, "Handling SimulateTradeRequest in dedicated handler");

    let order_id = Uuid::new_v4().to_string();
    let timeframe = TimeFrame::Day1;

    let store = market_data_store.read().await;
    let candles_opt = store.get_candles(&req_payload.symbol, timeframe, None, None);

    if candles_opt.is_none() || candles_opt.as_ref().unwrap().is_empty() {
        tracing::warn!(symbol = %req_payload.symbol, ?timeframe, "No market data available to simulate trade (handler).");
        return Ok(Response::new(TradeResponse {
            success: false,
            message: format!("No market data available for symbol '{}' and timeframe {:?} to simulate trade.", req_payload.symbol, timeframe),
            order_id,
            filled_price: 0.0,
            filled_quantity: 0.0,
        }));
    }

    let candles = candles_opt.unwrap();
    let latest_candle = match candles.last() {
        Some(c) => c.clone(),
        None => {
            let err_msg = format!("Logic error: candles list was non-empty for symbol '{}' but last() is None (handler).", req_payload.symbol);
            tracing::error!("{}", err_msg);
            return Err(EngineError::MarketDataError(err_msg).into());
        }
    };
    drop(store);

    let (success, filled_price, message_detail) = match req_payload.order_type.to_uppercase().as_str() {
        "MARKET" => {
            let price = latest_candle.close;
            let msg = format!(
                "Market {} order for {} of {} simulated at {:.2}",
                req_payload.action.to_uppercase(), req_payload.quantity, req_payload.symbol, price
            );
            (true, price, msg)
        }
        "LIMIT" => {
            match req_payload.price {
                Some(limit_price) => {
                    match req_payload.action.to_uppercase().as_str() {
                        "BUY" => {
                            if latest_candle.low <= limit_price {
                                let msg = format!("Limit BUY order for {} of {} simulated at {:.2}", req_payload.quantity, req_payload.symbol, limit_price);
                                (true, limit_price, msg)
                            } else {
                                let msg = format!("Limit BUY order for {} not filled: market low {:.2} did not reach limit price {:.2}", req_payload.symbol, latest_candle.low, limit_price);
                                (false, 0.0, msg)
                            }
                        }
                        "SELL" => {
                            if latest_candle.high >= limit_price {
                                let msg = format!("Limit SELL order for {} of {} simulated at {:.2}", req_payload.quantity, req_payload.symbol, limit_price);
                                (true, limit_price, msg)
                            } else {
                                let msg = format!("Limit SELL order for {} not filled: market high {:.2} did not reach limit price {:.2}", req_payload.symbol, latest_candle.high, limit_price);
                                (false, 0.0, msg)
                            }
                        }
                        _ => {
                            let msg = format!("Unknown action '{}' for LIMIT order. Use 'BUY' or 'SELL'.", req_payload.action);
                            (false, 0.0, msg)
                        }
                    }
                }
                None => {
                    let msg = "Limit price is required for LIMIT orders.".to_string();
                    (false, 0.0, msg)
                }
            }
        }
        _ => {
            let msg = format!("Unsupported order type: '{}'. Use 'MARKET' or 'LIMIT'.", req_payload.order_type);
            (false, 0.0, msg)
        }
    };

    if success {
        tracing::info!(order_id = %order_id, symbol = %req_payload.symbol, action = %req_payload.action, order_type = %req_payload.order_type, quantity = req_payload.quantity, filled_price, message = %message_detail, "Trade simulated successfully (handler)");
        Ok(Response::new(TradeResponse { success: true, message: message_detail, order_id, filled_price, filled_quantity: req_payload.quantity }))
    } else {
        tracing::warn!(order_id = %order_id, symbol = %req_payload.symbol, action = %req_payload.action, order_type = %req_payload.order_type, price = ?req_payload.price, failure_reason = %message_detail, "Trade simulation failed (handler)");
        Ok(Response::new(TradeResponse { success: false, message: message_detail, order_id, filled_price: 0.0, filled_quantity: 0.0 }))
    }
}
