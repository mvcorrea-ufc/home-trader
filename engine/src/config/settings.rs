// Engine settings, potentially loaded from a config file or environment variables
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct EngineSettings {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub thread_pool_size: usize,
    // Add other engine-specific settings here
}

impl Default for EngineSettings {
    fn default() -> Self {
        // Default values as per spec's JSON example
        EngineSettings {
            host: "localhost".to_string(),
            port: 50051,
            max_connections: 10,
            thread_pool_size: 4,
        }
    }
}

// TODO: Implement loading these settings from a configuration file
// (e.g., part of the global JSON config or a separate engine config)
