use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("CSV parsing system error: {source}")]
    CsvSystemError {
        #[from]
        source: csv::Error,
    },

    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("CSV data format error: {0}")]
    CsvDataFormatError(String),

    #[error("Market data store error: {0}")]
    MarketDataError(String),

    #[error("Indicator calculation error: {0}")]
    IndicatorError(String),

    #[error("Trade simulation error: {0}")]
    SimulationError(String),

    // This can be used to wrap errors from anyhow if they don't fit other categories
    // or if a function using anyhow needs to return EngineError.
    #[error("Internal processing error: {0}")]
    ProcessingError(String),

    // Catch-all for anyhow errors when direct conversion is suitable
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

impl From<EngineError> for tonic::Status {
    fn from(err: EngineError) -> Self {
        tracing::error!("Mapping EngineError to tonic::Status: {:?}", err); // Log the error source
        match err {
            EngineError::ConfigError(msg) => tonic::Status::failed_precondition(format!("Configuration error: {}", msg)),
            EngineError::CsvSystemError { source } => tonic::Status::invalid_argument(format!("CSV parsing system error: {}", source)),
            EngineError::IoError { source } => tonic::Status::internal(format!("I/O error: {}", source)),
            EngineError::CsvDataFormatError(msg) => tonic::Status::invalid_argument(format!("CSV data format error: {}", msg)),

            EngineError::MarketDataError(msg) => {
                if msg.to_lowercase().contains("not found") {
                    tonic::Status::not_found(msg)
                } else {
                    tonic::Status::internal(format!("Market data error: {}", msg))
                }
            }
            EngineError::IndicatorError(msg) => tonic::Status::internal(format!("Indicator calculation error: {}", msg)),
            EngineError::SimulationError(msg) => tonic::Status::internal(format!("Trade simulation error: {}", msg)),
            EngineError::ProcessingError(msg) => tonic::Status::internal(format!("Processing error: {}", msg)),
            EngineError::AnyhowError(source) => tonic::Status::internal(format!("An internal error occurred: {}", source)),
        }
    }
}
