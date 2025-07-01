// Helper functions for trading_service RPC implementations
use crate::error::EngineError;
use shared::models::Candle as DomainCandle;
// Assuming ProtoCandle is accessible via crate::services::ProtoCandle
// This alias is defined in the main services/mod.rs or trading_service.rs usually.
// If trading_service.rs defines `ProtoCandle as GrpcCandle` from `super::generated::Candle`,
// then here we might need to use `crate::services::generated::Candle as GrpcCandle`
// or ensure `ProtoCandle` is re-exported at a higher level accessible here.
// For now, assuming `crate::services::ProtoCandle` is the way.
use crate::services::ProtoCandle as GrpcCandle;


pub fn to_grpc_candle(domain_candle: &DomainCandle) -> GrpcCandle {
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

pub fn from_grpc_timestamp(ts_millis: i64) -> Result<chrono::DateTime<chrono::Utc>, EngineError> {
    chrono::DateTime::from_timestamp_millis(ts_millis)
        .ok_or_else(|| EngineError::ProcessingError(format!("Invalid gRPC timestamp: {}", ts_millis)))
}
