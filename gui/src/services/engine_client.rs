// gRPC client for interacting with the TradingEngine service
// This will use the Rust code generated from trading.proto (likely via the shared or a dedicated proto crate).

use anyhow::Result;
// Assuming the generated gRPC client code will be accessible.
// This might require the `engine`'s generated code to be exposed in a way that `gui` can use it,
// or `gui` might need its own build step to generate client stubs from the same .proto file.
// For now, let's assume a path to the generated client.
// This will need to be adjusted based on how protobufs are shared/generated across workspace members.

// Use the client and message types from the `engine` crate's `services` module.
use engine::services::{
    TradingEngineClient,
    LoadCsvRequest, MarketDataRequest, IndicatorRequest, // ProtoCandle has been aliased
    // MarketDataResponse, LoadCsvResponse, IndicatorResponse, // Response types might be needed for full implementation
};
use shared::models::Candle as SharedCandle; // Alias to avoid confusion if ProtoCandle is brought in without alias
use tonic::transport::Channel;

// For now, let's define a struct and placeholder methods.
// The actual gRPC client setup will be more involved.

#[derive(Clone)] // Add Clone derive
pub struct EngineClient {
    client: TradingEngineClient<Channel>,
    // endpoint: String, // No longer needed if client is stored directly
}

impl EngineClient {
    pub async fn new(endpoint: String) -> Result<Self> {
        // Establish gRPC connection
        let client = TradingEngineClient::connect(endpoint).await?;
        Ok(Self { client })
    }

    // Placeholder methods mirroring the gRPC service
    pub async fn load_csv(&mut self, file_path: String, symbol: String) -> Result<String> {
        let request = tonic::Request::new(LoadCsvRequest { file_path, symbol });
        let response = self.client.load_csv_data(request).await?.into_inner();
        Ok(response.message)
        // tracing::info!("[GUI Client STUB] Load CSV: {} for {}", file_path, symbol);
        // Ok(format!("Successfully loaded {} for {} (stubbed)", file_path, symbol))
    }

    pub async fn get_market_data(&mut self, symbol: String /*, from: i64, to: i64*/) -> Result<Vec<SharedCandle>> {
        // For now, let's assume `from` and `to` are not used or handled by default in the engine for simplicity
        // In a real scenario, these would be important parameters.
        let request = tonic::Request::new(MarketDataRequest {
            symbol: symbol.clone(), // Clone symbol for the request
            from_timestamp: 0, // Placeholder, needs proper values
            to_timestamp: chrono::Utc::now().timestamp_millis(), // Placeholder, needs proper values
        });
        let mut stream = self.client.get_market_data(request).await?.into_inner();
        let mut candles = Vec::new();
        while let Some(response_part) = stream.message().await? {
            candles.extend(response_part.candles.into_iter().map(|proto_c| {
                // Convert engine::services::ProtoCandle to shared::models::Candle
                SharedCandle {
                    symbol: proto_c.symbol,
                    timestamp: chrono::DateTime::from_timestamp_millis(proto_c.timestamp)
                        .unwrap_or_else(|| chrono::Utc::now()), // Or handle error better
                    open: proto_c.open,
                    high: proto_c.high,
                    low: proto_c.low,
                    close: proto_c.close,
                    volume: proto_c.volume,
                    trades: proto_c.trades as u32, // Ensure type matches
                }
            }));
        }
        Ok(candles)
        // tracing::info!("[GUI Client STUB] Get Market Data for {}", symbol);
        // Ok(vec![
        //     // Sample candle
        //     SharedCandle {
        //         symbol,
        //         timestamp: chrono::Utc::now(),
        //         open: 100.0, high: 105.0, low: 99.0, close: 102.0,
        //         volume: 1000.0, trades: 100
        //     }
        // ])
    }

    pub async fn calculate_indicator(&mut self, symbol: String, indicator_type: String, parameters_json: String) -> Result<Option<shared::models::Indicator>> {
        let request = tonic::Request::new(IndicatorRequest {
            symbol: symbol.clone(),
            indicator_type: indicator_type.clone(),
            parameters: parameters_json,
        });
        let response = self.client.calculate_indicator(request).await?.into_inner();

        // Convert engine::services::IndicatorResponse to shared::models::Indicator
        // Assuming IndicatorResponse has fields like name, values, and parameters (which might need parsing if it's a string)
        // The current proto definition of IndicatorResponse only has name and values.
        // The shared::models::Indicator has name, parameters (serde_json::Value), and values.
        // We'll need to adjust this conversion. For now, let's assume parameters are passed back or reconstructed.
        if response.values.is_empty() {
            Ok(None)
        } else {
            Ok(Some(shared::models::Indicator {
                name: response.indicator_name, // Assuming this is the full name like "SMA(20)" or just "SMA"
                parameters: serde_json::from_str(&parameters_json).unwrap_or(serde_json::Value::Null), // Re-parse original params, or engine should return them
                values: response.values,
            }))
        }
    }

    // Add other client methods for SimulateTrade etc.
}

// Note: The actual gRPC client generation and connection is a significant piece of work.
// The `build.rs` in the `engine` crate generates Rust code from `trading.proto`.
// For the `gui` crate to use this, either:
// 1. The `engine` crate must expose these generated types publicly, and `gui` adds `engine` as a dependency.
//    (This is the approach taken)
// 2. The `.proto` file is moved to the `shared` crate (or a new `protos` crate), and both `engine`
//    and `gui` generate their respective server/client code from it. This is a cleaner approach for larger projects.
//    The plan currently has `trading.proto` in `engine/proto/`.
