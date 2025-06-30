// Engine settings, loaded from a config file or environment variables
use serde::Deserialize;
use std::fs;
use std::path::Path; // Removed PathBuf
use anyhow::{Context, Result}; // Ensure anyhow is in Cargo.toml for engine
use tracing::warn;

const DEFAULT_CONFIG_PATH_FROM_WORKSPACE_ROOT: &str = "gui/assets/config/default.json";


#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub engine: EngineSettings,
    // We can add other sections like `app`, `chart`, `data` from the spec's JSON if needed by the engine.
    // For now, only `engine` settings are actively used by the engine's core startup.
    // Example:
    // pub chart: serde_json::Value, // Or a strongly typed struct
    // pub data: DataSettings,
}

impl AppSettings {
    /// Loads application settings from a specified JSON file.
    /// If the path is relative, it's resolved from the current working directory.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let file_content = fs::read_to_string(path_ref)
            .with_context(|| format!("Failed to read configuration file from {:?}", path_ref))?;

        let app_settings: AppSettings = serde_json::from_str(&file_content)
            .with_context(|| format!("Failed to parse configuration file: {}", file_content))?;

        Ok(app_settings)
    }

    /// Attempts to load settings from a default path, typically for development.
    /// This path is relative to the workspace root.
    pub fn load_default_dev() -> Result<Self> {
        // Try to find the workspace root. This is a common pattern.
        // CARGO_MANIFEST_DIR is set by Cargo when running tests or `cargo run`.
        // For a deployed binary, this path needs to be determined differently (e.g., relative to executable).
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let workspace_root = Path::new(&manifest_dir).parent().unwrap_or_else(|| Path::new(".")); // Assumes engine is one level down
        let config_path = workspace_root.join(DEFAULT_CONFIG_PATH_FROM_WORKSPACE_ROOT);

        Self::load_from_file(&config_path)
    }
}


#[derive(Debug, Deserialize, Clone)]
#[serde(default)] // This will apply EngineSettings::default() if "engine" key is missing or for missing fields
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
            host: "127.0.0.1".to_string(), // Changed from localhost to be more explicit IP
            port: 50051,
            max_connections: 10,
            thread_pool_size: 4, // Note: Tokio manages its own thread pool. This is more for custom pools.
        }
    }
}

/// Utility function to get engine settings:
/// 1. Tries to load from `gui/assets/config/default.json` (relative to workspace root for dev).
/// 2. Falls back to `EngineSettings::default()` if loading fails.
pub fn get_engine_settings() -> EngineSettings {
    match AppSettings::load_default_dev() {
        Ok(app_settings) => {
            tracing::info!(path = %DEFAULT_CONFIG_PATH_FROM_WORKSPACE_ROOT, "Successfully loaded configuration.");
            app_settings.engine
        }
        Err(e) => {
            warn!(
                path = %DEFAULT_CONFIG_PATH_FROM_WORKSPACE_ROOT,
                error = ?e, // Using debug formatting for the error object
                "Failed to load configuration. Using default engine settings."
            );
            EngineSettings::default()
        }
    }
}
