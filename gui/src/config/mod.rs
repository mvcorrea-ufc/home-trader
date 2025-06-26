// GUI configuration module
pub mod theme; // For theme-specific configurations (colors, fonts, etc.)
// Potentially app_config.rs for the main application config structure (mapping to default.json)

// Example: Structure for the entire application configuration loaded from JSON
// This would mirror the structure of assets/config/default.json
use serde::Deserialize;
// use super::state::app_state::Theme; // If theme enum is used here

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub version: String,
    pub app: AppSettings,
    pub engine: EngineConnSettings,
    pub chart: ChartConfig,
    pub indicators: IndicatorDefaults, // Consider nesting further if complex
    pub data: DataSettings,
    pub shortcuts: Shortcuts,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub theme: String, // "dark" or "light" - could map to Theme enum
    pub language: String,
    pub auto_save: bool,
    pub auto_save_interval: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EngineConnSettings {
    pub host: String,
    pub port: u16,
    // max_connections and thread_pool_size are engine's internal config, GUI might not need them directly
    // but they are in the example JSON.
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChartConfig {
    #[serde(rename = "type")]
    pub chart_type: String, // "candlestick"
    pub candle: CandleStyle,
    pub background: String,
    pub grid: GridStyle,
    pub crosshair: CrosshairStyle,
    pub time_scale: ScaleStyle,
    pub price_scale: ScaleStyle,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CandleStyle {
    pub bullish_color: String,
    pub bearish_color: String,
    pub border_width: u32,
    pub wick_width: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GridStyle {
    pub color: String,
    pub enabled: bool,
    pub style: String, // "dashed", "solid"
}

#[derive(Debug, Deserialize, Clone)]
pub struct CrosshairStyle {
    pub enabled: bool,
    pub color: String,
    pub style: String, // "solid", "dashed"
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScaleStyle {
    pub visible: bool,
    pub color: String,
    pub border_color: String,
    // mode for price_scale: "normal" (not in provided struct, but in JSON)
    #[serde(default)] // Add this if 'mode' is optional or might be missing
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndicatorDefaults {
    pub sma: IndicatorSetting,
    pub ema: IndicatorSetting,
    pub rsi: RsiSetting,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndicatorSetting {
    pub enabled: bool,
    pub periods: Vec<u32>,
    pub colors: Vec<String>,
    pub line_width: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RsiSetting {
    pub enabled: bool,
    pub period: u32,
    pub overbought: u32,
    pub oversold: u32,
    pub color: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataSettings {
    pub csv_delimiter: String, // Should be char, but JSON string is easier
    pub decimal_separator: String, // Should be char
    pub thousand_separator: String, // Should be char
    pub date_format: String,
    pub time_format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Shortcuts {
    pub command_palette: String,
    pub load_csv: String,
    pub save_project: String,
    pub exit: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub reset_zoom: String,
}

impl AppConfig {
    // Method to load config from the default.json file (or user-specific one)
    // For now, this would be called during AppState initialization or main.rs
    pub fn load_default() -> Result<Self, anyhow::Error> {
        // This path needs to be correct relative to where the binary runs,
        // or the config file needs to be embedded. For desktop apps, usually relative to exe or a known config dir.
        // For Dioxus desktop, assets are often bundled or need specific handling.
        // The spec places it in "gui/assets/config/default.json".
        // For now, let's assume it can be read from a relative path for dev.
        // A common pattern is to include_str! the default config.
        let config_str = include_str!("../../assets/config/default.json"); // Path relative to this .rs file
        let config: AppConfig = serde_json::from_str(config_str)?;
        Ok(config)
    }
}
