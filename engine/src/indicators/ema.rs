// Exponential Moving Average (EMA) indicator implementation
use super::IndicatorCalculator;
use shared::models::Candle;
use serde_json::Value;

pub struct Ema {
    name: String,
    period: usize,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        Self {
            name: format!("EMA({})", period),
            period,
        }
    pub fn new(period: usize) -> Self {
        if period == 0 {
            panic!("EMA period must be greater than 0");
        }
        Self {
            name: format!("EMA({})", period),
            period,
        }
    }
}

impl IndicatorCalculator for Ema {
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
        if data.is_empty() {
            return Vec::new();
        }
        if data.len() < self.period {
            return vec![f64::NAN; data.len()];
        }

        let mut results = vec![f64::NAN; self.period - 1];
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        // Calculate initial SMA for the first EMA value
        let initial_sum: f64 = data.iter().take(self.period).map(|c| c.close).sum();
        let mut previous_ema = initial_sum / self.period as f64;
        results.push(previous_ema);

        for candle in data.iter().skip(self.period) {
            let ema = (candle.close - previous_ema) * multiplier + previous_ema;
            results.push(ema);
            previous_ema = ema;
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
                // Both are NaN
            } else {
                assert!((val_a - val_b).abs() < 1e-9, "Mismatch at index {}: {} != {}", i, val_a, val_b);
            }
        }
    }

    #[test]
    fn test_ema_calculation() {
        let candles = vec![
            create_candle(10.0), create_candle(11.0), create_candle(12.0), // SMA = 11.0
            create_candle(13.0), // EMA = (13-11)*0.5 + 11 = 1+11 = 12.0
            create_candle(14.0), // EMA = (14-12)*0.5 + 12 = 1+12 = 13.0
        ];
        let ema = Ema::new(3); // Period 3
        let results = ema.calculate(&candles);

        let expected = vec![f64::NAN, f64::NAN, 11.0, 12.0, 13.0];
        assert_f64_vec_eq(&results, &expected);
    }

    #[test]
    fn test_ema_insufficient_data() {
        let candles = vec![create_candle(1.0), create_candle(2.0)];
        let ema = Ema::new(3);
        let results = ema.calculate(&candles);
        assert_f64_vec_eq(&results, &[f64::NAN, f64::NAN]);
    }

    #[test]
    fn test_ema_period_one() {
        let candles = vec![
            create_candle(10.0), create_candle(11.0), create_candle(12.0),
        ];
        let ema = Ema::new(1);
        let results = ema.calculate(&candles);
        // EMA(1) is just the close price. Initial SMA(1) is data[0].close.
        // results[0] = 10.0
        // results[1] = (11-10)*1 + 10 = 11.0
        // results[2] = (12-11)*1 + 11 = 12.0
        assert_f64_vec_eq(&results, &[10.0, 11.0, 12.0]);
    }

    #[test]
    fn test_ema_empty_data() {
        let candles: Vec<Candle> = Vec::new();
        let ema = Ema::new(3);
        let results = ema.calculate(&candles);
        assert_f64_vec_eq(&results, &[]);
    }

    #[test]
    #[should_panic(expected = "EMA period must be greater than 0")]
    fn test_ema_period_zero_panic() {
        Ema::new(0);
    }

    // Test from a known source: https://www.iexplain.org/ema-calculation/
    // Prices: 22.27, 22.19, 22.08, 22.17, 22.18, 22.13, 22.23, 22.43, 22.24, 22.29
    // EMA Period: 10
    // First EMA (SMA10): 22.221
    // Second EMA: (22.29 - 22.221) * (2/11) + 22.221 = 0.069 * 0.181818 + 22.221 = 0.012545 + 22.221 = 22.2335
    #[test]
    fn test_ema_known_values() {
        let candles = vec![
            create_candle(22.27), create_candle(22.19), create_candle(22.08), create_candle(22.17),
            create_candle(22.18), create_candle(22.13), create_candle(22.23), create_candle(22.43),
            create_candle(22.24), // 9th candle
            create_candle(22.29), // 10th candle, SMA10 calculated here = 22.221
            create_candle(22.32), // 11th candle
            // create_candle(22.50), // 12th candle
        ];
        let ema_calculator = Ema::new(10);
        let results = ema_calculator.calculate(&candles);

        let mut expected_results = vec![f64::NAN; 9];

        // SMA for first 10 values:
        // (22.27+22.19+22.08+22.17+22.18+22.13+22.23+22.43+22.24+22.29) / 10 = 222.21 / 10 = 22.221
        let sma10 = 22.221;
        expected_results.push(sma10);

        // EMA for 11th value (22.32):
        // Multiplier = 2 / (10 + 1) = 2/11
        let multiplier = 2.0 / 11.0;
        // EMA = (22.32 - 22.221) * (2/11) + 22.221
        //     = 0.099 * 0.18181818... + 22.221
        //     = 0.0179999... + 22.221 = 22.238999... ~ 22.2390
        let ema11 = (22.32 - sma10) * multiplier + sma10;
        expected_results.push(ema11);

        assert_f64_vec_eq(&results, &expected_results);
    }
}
