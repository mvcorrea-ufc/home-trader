// Technical indicators module
pub mod ema;
pub mod rsi;
pub mod sma;

pub use ema::Ema;
pub use rsi::Rsi;
pub use sma::Sma;

use shared::models::Candle;
use serde_json::Value;

// Common trait for all indicators
pub trait IndicatorCalculator: Send + Sync {
    fn name(&self) -> &str;
    fn parameters(&self) -> Value; // Parameters used for this indicator instance
    fn calculate(&self, data: &[Candle]) -> Vec<f64>; // Use f64::NAN for undefined values
}
