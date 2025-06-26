// Global application state for the GUI
// This will hold data like loaded symbols, active chart settings, theme, etc.
// Dioxus uses hooks like `use_shared_state` or `use_state` within components,
// but a global structure can be useful for organizing complex state or for state
// that needs to be managed outside of component lifecycle (e.g., by services).

use serde::{Deserialize, Serialize};
use shared::models::{MarketData, Indicator}; // Using shared models
use std::collections::HashMap;

// Example theme enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

// Example structure for application state
// This can be provided via Dioxus' shared state context if needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub current_theme: Theme,
    pub language: String, // e.g., "pt-BR", "en-US"

    // Data related state
    pub loaded_market_data: HashMap<String, MarketData>, // Keyed by symbol
    pub active_indicators: HashMap<String, Vec<Indicator>>, // Keyed by symbol, then list of active indicators

    // UI specific state
    pub command_palette_visible: bool,
    // pub active_symbol: Option<String>,
    // pub active_timeframe: Option<shared::models::TimeFrame>,

    // Configuration loaded from default.json or user settings
    // pub config: AppConfig, // This might hold the deserialized config from assets/config/default.json
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_theme: Theme::Dark,
            language: "pt-BR".to_string(),
            loaded_market_data: HashMap::new(),
            active_indicators: HashMap::new(),
            command_palette_visible: false,
            // active_symbol: None,
            // active_timeframe: None,
            // config: AppConfig::default(), // Assuming AppConfig has a default
        }
    }
}

// Methods to modify state could be here, or actions/reducers if using a more formal state management pattern.
impl AppState {
    pub fn set_theme(&mut self, theme: Theme) {
        self.current_theme = theme;
    }

    pub fn add_market_data(&mut self, data: MarketData) {
        self.loaded_market_data.insert(data.symbol.clone(), data);
    }

    // More methods as needed...
}


// Placeholder for the full application configuration structure that maps to default.json
// This might live in config/mod.rs or config/app_config.rs
// For now, just a simple placeholder if AppState needs it directly.
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AppConfig {
//     // fields from default.json
//     pub version: String,
// }
// impl Default for AppConfig { /* ... */ }

// How this AppState is used with Dioxus:
// 1. It can be created and managed by Dioxus' `use_shared_state_provider` at the root of the app.
// 2. Components can then access and modify it using `use_shared_state`.
// Example in app.rs:
// use_shared_state_provider(cx, AppState::default);
//
// Example in a component:
// let app_state = use_shared_state::<AppState>(cx).unwrap();
// if app_state.read().command_palette_visible { /* ... */ }
// app_state.write().set_theme(Theme::Light);
