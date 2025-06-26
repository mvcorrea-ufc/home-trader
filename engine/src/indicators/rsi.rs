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

    fn calculate(&self, data: &[Candle]) -> Vec<Option<f64>> {
        if data.len() <= self.period || self.period == 0 {
            return vec![None; data.len()];
        }

        let mut results = vec![None; self.period]; // RSI needs 'period' initial changes

        let mut gains = 0.0;
        let mut losses = 0.0;

        // Calculate initial average gain and loss
        for i in 1..=self.period {
            let change = data[i].close - data[i-1].close;
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change; // losses are positive values
            }
        }

        let mut avg_gain = gains / self.period as f64;
        let mut avg_loss = losses / self.period as f64;

        if avg_loss == 0.0 {
            results.push(Some(100.0)); // Avoid division by zero; if no losses, RSI is 100
        } else {
            let rs = avg_gain / avg_loss;
            results.push(Some(100.0 - (100.0 / (1.0 + rs))));
        }

        // Calculate subsequent RSI values
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
                results.push(Some(100.0));
            } else {
                let rs = avg_gain / avg_loss;
                results.push(Some(100.0 - (100.0 / (1.0 + rs))));
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

    #[test]
    fn test_rsi_calculation() {
        // Using example values from a known source for RSI calculation
        let candles = vec![
            create_candle(44.34), create_candle(44.09), create_candle(44.15), create_candle(43.61), create_candle(44.33),
            create_candle(44.83), create_candle(45.10), create_candle(45.42), create_candle(45.84), create_candle(46.08),
            create_candle(45.89), create_candle(46.03), create_candle(45.61), create_candle(46.28), create_candle(46.28), // RSI calc starts after this (14th period data point)
            create_candle(46.00), // 15th
            create_candle(46.03), // 16th
            create_candle(46.41), // 17th
            create_candle(46.22), // 18th
            create_candle(45.64), // 19th
            create_candle(46.25), // 20th
        ];

        let rsi_calculator = Rsi::new(14);
        let results = rsi_calculator.calculate(&candles);

        // Expected values for this dataset with 14 periods (approximate)
        // First RSI value is at index 14
        // Note: RSI calculations can have slight variations based on SMA vs EMA smoothing for average gain/loss.
        // This implementation uses a simple moving average for the initial period, then Wilder's smoothing.
        // The exact values might differ from some online calculators but the method should be consistent.

        // We expect 14 `None` values, then the first RSI.
        for i in 0..14 {
            assert_eq!(results[i], None, "Expected None at index {}", i);
        }

        // Example: RSI for the 15th data point (index 14 in 0-based array)
        // This test will need to be more precise with known values if specific RSI figures are targeted.
        // For now, checking that values are produced and are within 0-100 range.
        assert!(results[14].is_some());
        assert!(results[14].unwrap() >= 0.0 && results[14].unwrap() <= 100.0);
        // println!("RSI at 14: {:?}", results[14]); // ~70.53 based on some online calculators for this data

        // For example, for the data point 46.00 (index 14), the RSI should be around 70.53
        // This depends HEAVILY on the exact smoothing method (Wilder's, SMA, EMA) for avg gain/loss.
        // The current implementation uses Wilder's smoothing implicitly.

        // Let's verify a few points (approximate, actual values depend on precise test data alignment with a known calculator)
        // If using the data from https://school.stockcharts.com/doku.php?id=technical_indicators:relative_strength_index_rsi
        // Price: 44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03, 45.61, 46.28
        // This is 14 prices. The 15th price is 46.28. RSI for this point is ~69.99
        // The next price 46.00 gives RSI ~60.03
        // The next price 46.03 gives RSI ~60.51
        // The next price 46.41 gives RSI ~67.60

        // Due to the complexity of precise RSI validation without a reference implementation,
        // these tests are basic sanity checks. More rigorous tests would compare against known good values.
        if let Some(val) = results[14] { assert!(val > 50.0 && val < 80.0, "RSI[14] out of expected range: {}", val); } // Approx for 46.28
        if candles.len() > 15 && results.len() > 15 { if let Some(val) = results[15] { assert!(val > 50.0 && val < 70.0, "RSI[15] out of expected range: {}", val); }} // Approx for 46.00
        if candles.len() > 16 && results.len() > 16 { if let Some(val) = results[16] { assert!(val > 50.0 && val < 70.0, "RSI[16] out of expected range: {}", val); }} // Approx for 46.03
        if candles.len() > 17 && results.len() > 17 { if let Some(val) = results[17] { assert!(val > 60.0 && val < 80.0, "RSI[17] out of expected range: {}", val); }} // Approx for 46.41


    }

    #[test]
    fn test_rsi_insufficient_data() {
        let candles = vec![create_candle(1.0); 10]; // 10 data points
        let rsi = Rsi::new(14); // Period 14
        let results = rsi.calculate(&candles);
        assert_eq!(results, vec![None; 10]);
    }

    #[test]
    fn test_rsi_all_gains() {
        let candles = (1..=20).map(|i| create_candle(i as f64)).collect::<Vec<_>>();
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
        for i in 0..14 {
            assert_eq!(results[i], None);
        }
        for i in 14..20 {
            assert_eq!(results[i], Some(100.0));
        }
    }

    #[test]
    fn test_rsi_all_losses() {
        let candles = (1..=20).map(|i| create_candle(20.0 - i as f64)).collect::<Vec<_>>();
        let rsi = Rsi::new(14);
        let results = rsi.calculate(&candles);
         for i in 0..14 {
            assert_eq!(results[i], None);
        }
        for i in 14..20 {
            // It won't be exactly 0.0 due to avg_loss potentially being non-zero after initial period.
            // If avg_gain is 0, then RS is 0, RSI is 100 - (100 / (1 + 0)) = 0
            assert_eq!(results[i], Some(0.0));
        }
    }
}
