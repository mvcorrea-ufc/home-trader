// Theme specific configurations (colors, fonts, styles)
use serde::{Deserialize, Serialize};

// This file would define structs for theme properties,
// which can then be loaded or selected in the application state.
// The main AppConfig might point to a specific theme file or embed theme details.

// Example:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub background: String,
    pub foreground: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    // Chart specific colors might also go here or in ChartConfig
    pub chart_bullish: String,
    pub chart_bearish: String,
    // ... other color definitions
}

impl ThemePalette {
    pub fn default_dark() -> Self {
        // Values could come from the AppConfig defaults
        Self {
            background: "#1e1e1e".to_string(),
            foreground: "#d1d4dc".to_string(),
            primary: "#007acc".to_string(),
            secondary: "#565656".to_string(),
            accent: "#26a69a".to_string(),
            chart_bullish: "#26a69a".to_string(), // from default.json
            chart_bearish: "#ef5350".to_string(), // from default.json
        }
    }

    pub fn default_light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            foreground: "#000000".to_string(),
            primary: "#007acc".to_string(),
            secondary: "#e0e0e0".to_string(),
            accent: "#009688".to_string(),
            chart_bullish: "#4caf50".to_string(),
            chart_bearish: "#f44336".to_string(),
        }
    }
}

// The AppState.current_theme (Dark/Light enum) could be used to select
// which ThemePalette is currently active.
// Components could then refer to this active palette for their styling.
// E.g., in Dioxus, this could be part of the shared state.
// let app_state = use_shared_state::<AppState>(cx).unwrap();
// let current_palette = if app_state.read().current_theme == Theme::Dark {
//     ThemePalette::default_dark()
// } else {
//     ThemePalette::default_light()
// };
// ... style: "background-color: {current_palette.background};" ...
