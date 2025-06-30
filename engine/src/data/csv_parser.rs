use crate::error::EngineError; // Import EngineError
use csv::{ReaderBuilder, StringRecord};
use shared::models::Candle;
use std::fs::File;
use std::io::BufReader;

// Module for Brazilian number and date/time format handling, as per spec section 7.1
pub mod brazilian_format {
    use crate::error::EngineError; // For returning CsvDataFormatError
    use std::str::FromStr;
    // Using anyhow::Error for internal error propagation within this module, then map to EngineError if needed.
    // Or directly use EngineError if preferred. For now, keeping anyhow for internal detailed errors.
    use anyhow::Result; // Removed unused 'anyhow' macro import
    use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

    // Parses decimals like "1.234,56" or "123,45" into f64
    pub fn parse_decimal(s: &str) -> Result<f64, EngineError> { // Changed to Result<_, EngineError>
        let normalized = s.trim()
            .replace('.', "")  // Remove thousand separators
            .replace(',', "."); // Replace decimal separator

        f64::from_str(&normalized)
            .map_err(|e| EngineError::CsvDataFormatError(format!("Failed to parse decimal '{}': {}", s, e)))
    }

    // Specifically for volume fields that might have a different thousand separator rule or be just a large number.
    pub fn parse_volume(s: &str) -> Result<f64, EngineError> { // Changed to Result<_, EngineError>
        parse_decimal(s) // Reuses parse_decimal which now returns Result<_, EngineError>
    }

    // Parses date "dd/mm/yyyy" and time "HH:MM:SS" into DateTime<Utc>
    pub fn parse_datetime(date_str: &str, time_str: &str) -> Result<DateTime<Utc>, EngineError> { // Changed
        let date = NaiveDate::parse_from_str(date_str, "%d/%m/%Y")
            .map_err(|e| EngineError::CsvDataFormatError(format!("Failed to parse date '{}': {}", date_str, e)))?;
        let time = NaiveTime::parse_from_str(time_str, "%H:%M:%S")
            .map_err(|e| EngineError::CsvDataFormatError(format!("Failed to parse time '{}': {}", time_str, e)))?;

        // Combine date and time, and assume it's in UTC.
        // If the CSV times are local, timezone conversion would be needed here.
        Ok(DateTime::from_naive_utc_and_offset(date.and_time(time), Utc))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::{Datelike, Timelike}; // For year(), month(), day(), hour(), etc.

        #[test]
        fn test_parse_decimal_simple() {
            assert_eq!(parse_decimal("123,45").unwrap(), 123.45);
        }

        #[test]
        fn test_parse_decimal_with_thousands() {
            assert_eq!(parse_decimal("1.234,56").unwrap(), 1234.56);
        }

        #[test]
        fn test_parse_decimal_large_number() {
            assert_eq!(parse_decimal("600.822.115,84").unwrap(), 600822115.84);
        }

        #[test]
        fn test_parse_datetime_valid() {
            let dt = parse_datetime("30/12/2024", "18:20:00").unwrap();
            assert_eq!(dt.year(), 2024);
            assert_eq!(dt.month(), 12);
            assert_eq!(dt.day(), 30);
            assert_eq!(dt.hour(), 18);
            assert_eq!(dt.minute(), 20);
            assert_eq!(dt.second(), 0);
        }

        #[test]
        fn test_parse_datetime_invalid_date() {
            assert!(parse_datetime("32/12/2024", "18:20:00").is_err());
        }

        #[test]
        fn test_parse_datetime_invalid_time() {
            assert!(parse_datetime("30/12/2024", "25:20:00").is_err());
        }

        #[test]
        fn test_parse_datetime_invalid_date_format() {
            assert!(parse_datetime("2024/12/30", "18:20:00").is_err());
        }
    }
}

pub struct BrazilianCsvParser;

impl BrazilianCsvParser {
    // CSV Header: Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade
    // Example Row: WINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;600.822.115,84;24.228
    pub fn load_candles_from_csv(file_path: &str, default_symbol: &str) -> Result<Vec<Candle>, EngineError> {
        let file = File::open(file_path).map_err(|e| EngineError::IoError{ source: e })?;
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true) // Assuming the first row is a header
            .from_reader(BufReader::new(file));

        let mut candles = Vec::new();
        // Map csv::Error to EngineError::CsvSystemError
        let headers = rdr.headers().map_err(|e| EngineError::CsvSystemError{ source: e })?.clone();

        for (idx, result) in rdr.records().enumerate() {
            // Map csv::Error to EngineError::CsvSystemError
            let record = result.map_err(|e| EngineError::CsvSystemError{ source: e })?;
            let line_num = idx + 2; // For user-friendly error messages (1-based index + header)

            let get_field_or_err = |name: &str| {
                Self::get_field(&record, &headers, name)
                    .map_err(EngineError::from) // Convert anyhow::Error from get_field to EngineError
                    .and_then(|opt_val| {
                        opt_val.ok_or_else(|| EngineError::CsvDataFormatError(format!("Missing '{}' field in CSV record at line {}", name, line_num)))
                    })
            };

            let symbol_str = Self::get_field(&record, &headers, "Ativo")?.unwrap_or(default_symbol); // get_field can return anyhow error
            let date_str = get_field_or_err("Data")?;
            let time_str = get_field_or_err("Hora")?;

            let open_str = get_field_or_err("Abertura")?;
            let high_str = get_field_or_err("Máximo")?;
            let low_str = get_field_or_err("Mínimo")?;
            let close_str = get_field_or_err("Fechamento")?;

            let volume_str = get_field_or_err("Volume")?;
            let trades_str = get_field_or_err("Quantidade")?;

            // brazilian_format functions now return Result<_, EngineError>
            let timestamp = brazilian_format::parse_datetime(date_str, time_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("{} at line {}", e, line_num)))?;

            let open = brazilian_format::parse_decimal(open_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Abertura': {} at line {}", e, line_num)))?;
            let high = brazilian_format::parse_decimal(high_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Máximo': {} at line {}", e, line_num)))?;
            let low = brazilian_format::parse_decimal(low_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Mínimo': {} at line {}", e, line_num)))?;
            let close = brazilian_format::parse_decimal(close_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Fechamento': {} at line {}", e, line_num)))?;

            let volume = brazilian_format::parse_volume(volume_str)
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Volume': {} at line {}", e, line_num)))?;

            let trades = trades_str.replace('.', "").parse::<u32>()
                .map_err(|e| EngineError::CsvDataFormatError(format!("Error parsing 'Quantidade' {} as u32: {} at line {}", trades_str, e, line_num)))?;

            candles.push(Candle {
                symbol: symbol_str.to_string(),
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                trades,
            });
        }
        Ok(candles)
    }

    // Helper to get field by header name.
    // If the header name is not found, it returns Ok(None).
    // This is different from a mandatory field missing, which is checked by .ok_or_else in the calling code.
    fn get_field<'a>(record: &'a StringRecord, headers: &'a StringRecord, name: &str) -> Result<Option<&'a str>, EngineError> {
        match headers.iter().position(|header| header == name) {
            Some(pos) => Ok(record.get(pos)),
            None => {
                // It's not an error for a column potentially not existing (e.g. "Ativo" might be optional if default_symbol is used)
                // If a *mandatory* column is missing by header name, the .ok_or_else() in the caller handles it.
                // If we wanted to error if the *header itself* is not defined, this would be the place.
                // For now, if header `name` isn't in `headers`, we assume the field is simply not present in this record.
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_load_candles_from_csv_valid_data() {
        let csv_content = "\
Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade
WINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;600.822.115,84;24.228
PETR4;02/01/2023;10:00:00;23,50;23,80;23,40;23,75;1.000.000,00;1000";
        let tmp_file = create_test_csv(csv_content);
        let candles = BrazilianCsvParser::load_candles_from_csv(tmp_file.path().to_str().unwrap(), "FALLBACK").unwrap();

        assert_eq!(candles.len(), 2);

        assert_eq!(candles[0].symbol, "WINFUT");
        assert_eq!(candles[0].timestamp, brazilian_format::parse_datetime("30/12/2024", "18:20:00").unwrap());
        assert_eq!(candles[0].open, 124080.0); // Assuming "124.080" is 124080, not 124.080 - brazilian_format::parse_decimal interprets "." as thousand sep.
                                            // If "124.080" means 124 point 080, the parse_decimal needs adjustment for this specific field
                                            // The spec example "124.080" vs "600.822.115,84" is ambiguous.
                                            // Current parse_decimal: "124.080" -> 124080.0. "124,080" -> 124.080
                                            // The example "WINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;..."
                                            // If these are prices like 124.08, then the CSV data should be "124,080".
                                            // Given the context of stock prices, "124.080" is more likely 124.08.
                                            // This means the `parse_decimal` might need to be more context-aware or the input data format clarified.
                                            // For now, sticking to the current `parse_decimal` which removes '.' and treats ',' as decimal.
                                            // Let's assume the input CSV is "124,080" for 124.08 for OHLC.
                                            // The spec example "124.080" is problematic for parse_decimal.
                                            // Let's re-evaluate `parse_decimal` and the example.
                                            // Spec: "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade"
                                            // Spec: "WINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;600.822.115,84;24.228"
                                            // If 124.080 is meant to be 124.08, then the parse_decimal needs to handle this.
                                            // The current `parse_decimal` turns "124.080" into 124080.0. This is likely incorrect for prices.
                                            // Let's assume the CSV for prices would be "124,08" or "124,080".
                                            // If the example `124.080` is literal, it implies a value of one hundred twenty-four thousand and eighty.
                                            // This seems too high for a typical stock price tick.
                                            // Let's assume the spec meant "124,080" for prices if it's 124.08.
                                            // Or, if "124.080" is a direct representation of a number like in some European conventions
                                            // where '.' is thousands and ',' is decimal, then `parse_decimal` is correct.
                                            // The ambiguity is "124.080" vs "123.938". If these are mini-indice points, they are like this.
                                            // WINFUT (Mini Indice) prices are typically like 124080 points. So parse_decimal is correct.
        assert_eq!(candles[0].high, 124090.0);
        assert_eq!(candles[0].low, 123938.0);
        assert_eq!(candles[0].close, 123983.0);
        assert_eq!(candles[0].volume, 600822115.84);
        assert_eq!(candles[0].trades, 24228);

        assert_eq!(candles[1].symbol, "PETR4");
        assert_eq!(candles[1].open, 23.50); // "23,50" -> 23.50. This is fine.
        assert_eq!(candles[1].volume, 1000000.00); // "1.000.000,00" -> 1000000.0. Fine.
        assert_eq!(candles[1].trades, 1000); // "1000" -> 1000. Fine.
    }

    #[test]
    fn test_load_candles_from_csv_empty_file() {
        let csv_content = "Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade"; // Only header
        let tmp_file = create_test_csv(csv_content);
        let candles = BrazilianCsvParser::load_candles_from_csv(tmp_file.path().to_str().unwrap(), "FALLBACK").unwrap();
        assert!(candles.is_empty());
    }

    #[test]
    fn test_load_candles_from_csv_missing_field() {
        let csv_content = "\
Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume
WINFUT;30/12/2024;18:20:00;124.080;124.090;123.938;123.983;600.822.115,84"; // Missing Quantidade
        let tmp_file = create_test_csv(csv_content);
        let result = BrazilianCsvParser::load_candles_from_csv(tmp_file.path().to_str().unwrap(), "FALLBACK");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'Quantidade' field"));
    }

    #[test]
    fn test_load_candles_from_csv_invalid_data_format() {
        let csv_content = "\
Ativo;Data;Hora;Abertura;Máximo;Mínimo;Fechamento;Volume;Quantidade
WINFUT;30/12/2024;18:20:00;invalid;124.090;123.938;123.983;600.822.115,84;24.228";
        let tmp_file = create_test_csv(csv_content);
        let result = BrazilianCsvParser::load_candles_from_csv(tmp_file.path().to_str().unwrap(), "FALLBACK");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Error parsing 'Abertura'"));
    }
}
