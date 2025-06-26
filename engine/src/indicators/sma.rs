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
    }
}

impl IndicatorCalculator for Sma {
    fn name(&self) -> &str {
        &self.name
    }

    fn parameters(&self) -> Value {
        serde_json::json!({ "period": self.period })
    }

    fn calculate(&self, data: &[Candle]) -> Vec<Option<f64>> {
        if data.len() < self.period {
            return vec![None; data.len()];
        }

        let mut results = vec![None; self.period -1]; // No SMA for initial period
        let mut sum: f64 = data.iter().take(self.period).map(|c| c.close).sum();
        results.push(Some(sum / self.period as f64));

        for i in self.period..data.len() {
            sum = sum - data[i - self.period].close + data[i].close;
            results.push(Some(sum / self.period as f64));
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
    fn test_sma_calculation() {
        let candles = vec![
            create_candle(1.0), create_candle(2.0), create_candle(3.0),
            create_candle(4.0), create_candle(5.0),
        ];
        let sma = Sma::new(3);
        let results = sma.calculate(&candles);
        assert_eq!(results, vec![None, None, Some(2.0), Some(3.0), Some(4.0)]);
    }

    #[test]
    fn test_sma_insufficient_data() {
        let candles = vec![create_candle(1.0), create_candle(2.0)];
        let sma = Sma::new(3);
        let results = sma.calculate(&candles);
        assert_eq!(results, vec![None, None]);
    }
}
