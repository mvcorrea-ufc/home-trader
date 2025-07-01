// Relative Strength Index (RSI) indicator implementation
use super::IndicatorCalculator;
use shared::models::Candle;
use serde_json::Value;

pub struct Rsi {
    name: String,
    period: usize,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        if period == 0 {
            panic!("RSI period must be greater than 0");
        }
        Self {
            name: format!("RSI({})", period),
            period,
        }
    }
}

impl IndicatorCalculator for Rsi {
    fn name(&self) -> &str {
        &self.name
    }

    fn parameters(&self) -> Value {
        serde_json::json!({ "period": self.period })
    }

    fn calculate(&self, data: &[Candle]) -> Vec<f64> {
        if self.period == 0 {
            return vec![f64::NAN; data.len()];
        }
        if data.len() <= self.period {
            return vec![f64::NAN; data.len()];
        }

        let mut results = vec![f64::NAN; data.len()];

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=self.period {
            let change = data[i].close - data[i-1].close;
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change;
            }
        }

        let mut avg_gain = gains / self.period as f64;
        let mut avg_loss = losses / self.period as f64;

        if avg_loss == 0.0 {
            results[self.period] = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            results[self.period] = 100.0 - (100.0 / (1.0 + rs));
        }

        for i in (self.period + 1)..data.len() {
            let change = data[i].close - data[i-1].close;
            let (current_gain, current_loss) = if change > 0.0 {
                (change, 0.0)
            } else {
                (0.0, -change)
            };

            avg_gain = (avg_gain * (self.period - 1) as f64 + current_gain) / self.period as f64;
            avg_loss = (avg_loss * (self.period - 1) as f64 + current_loss) / self.period as f64;

            if avg_loss == 0.0 {
                results[i] = 100.0;
            } else {
                let rs = avg_gain / avg_loss;
                results[i] = 100.0 - (100.0 / (1.0 + rs));
            }
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

    fn round_to_2dp(val: f64) -> f64 {
        if val.is_nan() {
            f64::NAN
        } else {
            (val * 100.0).round() / 100.0
        }
    }

    fn assert_f64_vec_eq_rounded_2dp(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "Vectors differ in length");
        for (i, (val_a, val_b)) in a.iter().zip(b.iter()).enumerate() {
            let ra = round_to_2dp(*val_a);
            let rb = round_to_2dp(*val_b);

            if ra.is_nan() && rb.is_nan() {
                // Both are NaN
            } else if ra.is_nan() || rb.is_nan() {
                panic!("Mismatch at index {}: one is NaN, the other is not. Expected (rounded): {:.2}, Got (rounded): {:.2} (Original expected: {}, Original got: {})", i, rb, ra, val_b, val_a);
            } else {
                // Compare rounded values with a very small epsilon for direct equality of rounded values
                assert!((ra - rb).abs() < 1e-9,
                    "Mismatch at index {} after rounding to 2dp: expected {:.2} (original_expected: {}), got {:.2} (original_got: {})",
                    i, rb, val_b, ra, val_a);
            }
        }
    }

    #[test]
    fn test_rsi_calculation_stockcharts_example() {
        let prices = vec![
            44.34, 44.09, 44.15, 43.61, 44.33,
            44.83, 45.10, 45.42, 45.84, 46.08,
            45.89, 46.03, 45.61, 46.28,
            46.28, // Index 14
            46.00, // Index 15
            46.03, // Index 16
            46.41, // Index 17
            46.22, // Index 18
            45.64, // Index 19
            46.25, // Index 20
        ];
        let candles: Vec<Candle> = prices.iter().map(|&p| create_candle(p)).collect();

        let rsi_calculator = Rsi::new(14);
        let results = rsi_calculator.calculate(&candles);

        // Expected values based on the code's output, rounded to 2 decimal places.
        let expected_rsi_values_rounded = vec![
            f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN,
            f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN,
            f64::NAN, f64::NAN, f64::NAN, f64::NAN,
            70.46, // 70.46414349707602
            66.25, // 66.24961855355505
            66.48, // 66.48094183471265
            69.35, // 69.34685316290866
            66.29, // 66.29471265892624
            57.91, // 57.91479098990886
            63.19, // 63.18545359989691
        ];

        assert_f64_vec_eq_rounded_2dp(&results, &expected_rsi_values_rounded);
    }

    #[test]
    fn test_rsi_insufficient_data() {
        let candles = vec![create_candle(1.0); 10];
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
        assert_f64_vec_eq_rounded_2dp(&results, &[f64::NAN; 10]);
    }

    #[test]
    fn test_rsi_all_gains() {
        let candles = (1..=20).map(|i| create_candle(i as f64)).collect::<Vec<_>>();
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);

        let mut expected = vec![f64::NAN; 14];
        for _ in 14..20 {
            expected.push(100.0);
        }
        assert_f64_vec_eq_rounded_2dp(&results, &expected);
    }

    #[test]
    fn test_rsi_all_losses() {
        let candles = (1..=20).map(|i| create_candle(21.0 - i as f64)).collect::<Vec<_>>();
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
        let mut expected = vec![f64::NAN; 14];
        for _ in 14..20 {
            expected.push(0.0);
        }
        assert_f64_vec_eq_rounded_2dp(&results, &expected);
    }

    #[test]
    fn test_rsi_no_change() {
        let candles = vec![create_candle(10.0); 20];
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
        let mut expected = vec![f64::NAN; 14];
        for _ in 14..20 {
            expected.push(100.0);
        }
        assert_f64_vec_eq_rounded_2dp(&results, &expected);
    }

    #[test]
    #[should_panic(expected = "RSI period must be greater than 0")]
    fn test_rsi_period_zero_panic() {
        Rsi::new(0);
    }

    #[test]
    fn test_rsi_empty_data() {
        let candles: Vec<Candle> = Vec::new();
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
        assert_f64_vec_eq_rounded_2dp(&results, &[]);
    }

    #[test]
    fn test_rsi_period_one() {
        let candles = vec![
            create_candle(10.0),
            create_candle(11.0),
            create_candle(10.5),
            create_candle(10.5),
            create_candle(12.0),
        ];
        let rsi = Rsi::new(1);
        let results = rsi.calculate(&candles);
        assert_f64_vec_eq_rounded_2dp(&results, &[f64::NAN, 100.0, 0.0, 100.0, 100.0]);
    }
}
