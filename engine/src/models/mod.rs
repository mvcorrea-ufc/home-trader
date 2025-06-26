// Engine-specific data models, if any.
// The primary Candle and MarketData models are in the `shared` crate.
// This module could be for:
// - Engine-internal representations that extend or adapt shared models.
// - Data structures specific to engine's internal state or processing, not meant for sharing.

// For now, it can remain empty or just re-export shared models if convenient for engine code.
// pub use shared::models::*;

// Or, if engine/src/models/candle.rs is created as per spec structure:
// pub mod candle;
