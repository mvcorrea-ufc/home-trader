// Initially empty, to be populated with utility functions as needed.
// For example, functions for date/time manipulation, common calculations, etc.
// that are shared across the engine and GUI.

// Placeholder for potential future brazilian_format module if it's decided to move it to shared.
// For now, it's planned for engine/src/data/csv_parser.rs as per spec section 7.1.
/*
pub mod brazilian_format {
    use std::str::FromStr;
    use anyhow::{Result, anyhow};

    pub fn parse_decimal(s: &str) -> Result<f64> {
        let normalized = s.trim()
            .replace('.', "")  // Remove thousand separators
            .replace(',', "."); // Replace decimal separator

        f64::from_str(&normalized)
            .map_err(|e| anyhow!("Failed to parse decimal '{}': {}", s, e))
    }

    pub fn format_decimal(value: f64, decimals: usize) -> String {
        let formatted = format!("{:.decimals$}", value, decimals = decimals);
        formatted.replace('.', ",")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_decimal() {
            assert_eq!(parse_decimal("123,45").unwrap(), 123.45);
            assert_eq!(parse_decimal("1.234,56").unwrap(), 1234.56);
            assert_eq!(parse_decimal("600.822.115,84").unwrap(), 600822115.84);
        }
    }
}
*/
