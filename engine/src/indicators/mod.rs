// Technical indicators module
pub mod ema;
pub mod rsi;
pub mod sma;

use shared::models::Candle;
use serde_json::Value;

// Common trait for all indicators
pub trait IndicatorCalculator: Send + Sync {
    fn name(&self) -> &str;
    fn parameters(&self) -> Value; // Parameters used for this indicator instance
    fn calculate(&self, data: &[Candle]) -> Vec<Option<f64>>; // Option<f64> to handle cases where indicator can't be calculated (e.g. insufficient data)
}
