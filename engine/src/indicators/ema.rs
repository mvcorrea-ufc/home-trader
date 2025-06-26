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
    }
}

impl IndicatorCalculator for Ema {
    fn name(&self) -> &str {
        &self.name
    }

    fn parameters(&self) -> Value {
        serde_json::json!({ "period": self.period })
    }

    fn calculate(&self, data: &[Candle]) -> Vec<Option<f64>> {
        if data.is_empty() || self.period == 0 {
            return vec![None; data.len()];
        }
        if data.len() < self.period {
            return vec![None; data.len()];
        }

        let mut results = vec![None; self.period -1];
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        // Calculate initial SMA for the first EMA value
        let initial_sum: f64 = data.iter().take(self.period).map(|c| c.close).sum();
        let mut previous_ema = initial_sum / self.period as f64;
        results.push(Some(previous_ema));

        for candle in data.iter().skip(self.period) {
            let ema = (candle.close - previous_ema) * multiplier + previous_ema;
            results.push(Some(ema));
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

    #[test]
    fn test_ema_calculation() {
        let candles = vec![
            create_candle(10.0), create_candle(11.0), create_candle(12.0),
            create_candle(13.0), create_candle(14.0),
        ];
        let ema = Ema::new(3); // Period 3
        let results = ema.calculate(&candles);
        // Expected:
        // SMA for first 3: (10+11+12)/3 = 11.0
        // EMA for 13: (13 - 11.0) * (2/(3+1)) + 11.0 = 2.0 * 0.5 + 11.0 = 1.0 + 11.0 = 12.0
        // EMA for 14: (14 - 12.0) * 0.5 + 12.0 = 2.0 * 0.5 + 12.0 = 1.0 + 12.0 = 13.0
        assert_eq!(results[2].unwrap(), 11.0); // Initial SMA
        assert_eq!(results[3].unwrap(), 12.0);
        assert_eq!(results[4].unwrap(), 13.0);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], None);
        assert_eq!(results[1], None);
    }
}
