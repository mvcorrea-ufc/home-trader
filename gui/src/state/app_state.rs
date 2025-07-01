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

    // --- Data related state ---
    // Storage for all loaded data, keyed by symbol
    pub all_market_data: HashMap<String, MarketData>,
    pub all_indicators: HashMap<String, Vec<Indicator>>, // Stores calculated indicators per symbol

    // Data for the currently active chart/symbol
    pub current_symbol_display: Option<String>,
    pub current_candles_display: Option<Vec<shared::models::Candle>>,
    pub current_indicators_display: Vec<shared::models::Indicator>,

    // UI feedback for data operations
    pub is_loading: bool,
    pub error_message: Option<String>,

    // --- UI specific state ---
    pub command_palette_visible: bool,
    // pub active_timeframe: Option<shared::models::TimeFrame>, // Future use

    // Configuration loaded from default.json or user settings
    // pub config: AppConfig, // This might hold the deserialized config from assets/config/default.json
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_theme: Theme::Dark,
            language: "pt-BR".to_string(),

            all_market_data: HashMap::new(),
            all_indicators: HashMap::new(),

            current_symbol_display: None,
            current_candles_display: None,
            current_indicators_display: Vec::new(),

            is_loading: false,
            error_message: None,

            command_palette_visible: false,
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

    // Method to update display data after loading/changing symbol
    pub fn set_display_data(&mut self, symbol: &str) {
        self.current_symbol_display = Some(symbol.to_string());

        if let Some(market_data_obj) = self.all_market_data.get(symbol) {
            self.current_candles_display = Some(market_data_obj.candles.clone());
        } else {
            self.current_candles_display = None;
        }

        if let Some(indicators_for_symbol) = self.all_indicators.get(symbol) {
            self.current_indicators_display = indicators_for_symbol.clone();
        } else {
            self.current_indicators_display = Vec::new();
        }
        self.error_message = None; // Clear previous error on new data load
    }

    pub fn add_market_data(&mut self, data: MarketData) {
        let symbol = data.symbol.clone();
        self.all_market_data.insert(symbol.clone(), data);
        // Optionally, directly set this as the display data
        // self.set_display_data(&symbol);
    }

    pub fn add_indicator_to_symbol(&mut self, symbol: &str, indicator: Indicator) {
        self.all_indicators.entry(symbol.to_string())
            .or_default()
            .push(indicator);

        // If this symbol is currently displayed, update its display indicators
        if self.current_symbol_display.as_deref() == Some(symbol) {
            if let Some(indicators_for_symbol) = self.all_indicators.get(symbol) {
                self.current_indicators_display = indicators_for_symbol.clone();
            }
        }
    }

    pub fn clear_indicators_for_symbol(&mut self, symbol: &str) {
        self.all_indicators.remove(symbol);
        if self.current_symbol_display.as_deref() == Some(symbol) {
            self.current_indicators_display = Vec::new();
        }
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
