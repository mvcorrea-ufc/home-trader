// Simple Moving Average (SMA) indicator implementation
use super::IndicatorCalculator;
use shared::models::Candle;
use serde_json::Value;

pub struct Sma {
    name: String,
    period: usize,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self {
            name: format!("SMA({})", period),
            period,
        }
    pub fn new(period: usize) -> Self {
        if period == 0 {
            // Or return Result<Self, Error>
            panic!("SMA period must be greater than 0");
        }
        Self {
            name: format!("SMA({})", period),
            period,
        }
    }
}

impl IndicatorCalculator for Sma {
    fn name(&self) -> &str {
        &self.name
    }

    fn parameters(&self) -> Value {
        serde_json::json!({ "period": self.period })
    }

    fn calculate(&self, data: &[Candle]) -> Vec<f64> {
        if self.period == 0 { // Should be caught by new()
            return vec![f64::NAN; data.len()];
        }
        if data.len() < self.period {
            return vec![f64::NAN; data.len()];
        }

        let mut results = vec![f64::NAN; self.period - 1]; // No SMA for initial period

        // Calculate sum for the first window
        let mut sum: f64 = data.iter().take(self.period).map(|c| c.close).sum();
        results.push(sum / self.period as f64);

        // Slide the window
        for i in self.period..data.len() {
            sum = sum - data[i - self.period].close + data[i].close;
            results.push(sum / self.period as f64);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_candle(close: f64) -> Candle {
        Candle {
            symbol: "TEST".to_string(),
            timestamp: Utc::now(),
            open: close, high: close, low: close, close,
            volume: 0.0, trades: 0,
        }
    }

    fn assert_f64_vec_eq(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "Vectors differ in length");
        for (i, (val_a, val_b)) in a.iter().zip(b.iter()).enumerate() {
            if val_a.is_nan() && val_b.is_nan() {
                // Both are NaN, consider them equal for this test
            } else {
                assert!((val_a - val_b).abs() < 1e-9, "Mismatch at index {}: {} != {}", i, val_a, val_b);
            }
        }
    }


    #[test]
    fn test_sma_calculation() {
        let candles = vec![
            create_candle(1.0), create_candle(2.0), create_candle(3.0),
            create_candle(4.0), create_candle(5.0),
        ];
        let sma = Sma::new(3);
        let results = sma.calculate(&candles);
        // expected: NaN, NaN, (1+2+3)/3=2.0, (2+3+4)/3=3.0, (3+4+5)/3=4.0
        assert_f64_vec_eq(&results, &[f64::NAN, f64::NAN, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_sma_insufficient_data() {
        let candles = vec![create_candle(1.0), create_candle(2.0)];
        let sma = Sma::new(3);
        let results = sma.calculate(&candles);
        assert_f64_vec_eq(&results, &[f64::NAN, f64::NAN]);
    }

    #[test]
    fn test_sma_period_one() {
        let candles = vec![
            create_candle(1.0), create_candle(2.0), create_candle(3.0),
        ];
        let sma = Sma::new(1);
        let results = sma.calculate(&candles);
        // SMA(1) is just the close price
        assert_f64_vec_eq(&results, &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_sma_empty_data() {
        let candles: Vec<Candle> = Vec::new();
        let sma = Sma::new(3);
        let results = sma.calculate(&candles);
        assert_f64_vec_eq(&results, &[]);
    }

    #[test]
    #[should_panic(expected = "SMA period must be greater than 0")]
    fn test_sma_period_zero_panic() {
        Sma::new(0);
    }
}
