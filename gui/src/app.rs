#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_desktop::use_window; // Import use_window
use chrono::{TimeZone, Utc}; // For creating DateTime<Utc>

// Import necessary types
use crate::components::command_palette::CommandPalette;
use crate::components::chart::candlestick::CandlestickChart; // Import CandlestickChart
use crate::config::AppConfig;
use crate::state::app_state::AppState;
use shared::models::{Candle, Indicator}; // Import Candle and Indicator models
use serde_json::json; // For creating dummy json parameters for Indicator

#[component]
pub fn App() -> Element {
    // Load AppConfig
    let app_config = match AppConfig::load_default() {
        Ok(config) => config,
        Err(e) => {
            panic!("Failed to load default configuration: {:?}", e);
        }
    };

    // Provide AppState and AppConfig to the component tree
    let app_state_provider = use_shared_state_provider(cx, AppState::default);
    let app_config_provider = use_shared_state_provider(cx, || app_config.clone()); // Clone for the closure

    let window = use_window(cx);
    let app_state = use_shared_state::<AppState>(cx).unwrap();
    let app_config_for_shortcut = app_config_provider.read().clone();

    // Create sample candle data
    let sample_candles = use_ref(cx, || {
        vec![
            Candle {
                symbol: "SAMPLE".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 9, 30, 0).unwrap(),
                open: 100.0, high: 105.0, low: 98.0, close: 102.0, volume: 1000.0, trades: 100,
            },
            Candle {
                symbol: "SAMPLE".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 9, 31, 0).unwrap(),
                open: 102.0, high: 103.0, low: 99.0, close: 100.0, volume: 1200.0, trades: 120,
            },
            Candle {
                symbol: "SAMPLE".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 9, 32, 0).unwrap(),
                open: 100.0, high: 108.0, low: 100.0, close: 107.0, volume: 1500.0, trades: 150,
            },
            Candle {
                symbol: "SAMPLE".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 9, 33, 0).unwrap(),
                open: 107.0, high: 110.0, low: 105.0, close: 106.0, volume: 1300.0, trades: 130,
            },
            Candle {
                symbol: "SAMPLE".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 9, 34, 0).unwrap(),
                open: 106.0, high: 106.0, low: 102.0, close: 103.0, volume: 1100.0, trades: 110,
            },
        ]
    });

    let sample_sma_indicator = use_ref(cx, || {
        let candles_data = sample_candles.read();
        let period = 3; // SMA period
        let mut sma_values: Vec<f64> = Vec::new();

        if candles_data.len() >= period {
            for i in 0..candles_data.len() {
                if i < period -1 {
                    // Not enough data for full SMA, could push NaN or skip
                    // For plotting, often better to have a value, even if it's an estimate or partial
                    // Or, ensure indicator data starts only when enough points are available.
                    // For this simple example, let's just start SMA when enough data.
                    // Or let's pad with first few values, which is not correct SMA but fills data.
                    // A common way is to have fewer indicator points than candle points.
                    // For now, let's make the indicator values align with candles from the first possible point.
                    // So, the first `period-1` indicator values will be missing.
                    // The IndicatorOverlay will need to handle this (e.g. by starting its line later).
                    // Alternative: pad with NaN or some other value.
                    // For simplicity of rendering a continuous line for now, let's make a simple running average.
                    // This is NOT a true SMA for the first few points but makes the line continuous.
                     let current_slice = &candles_data[0..=i];
                     let sum: f64 = current_slice.iter().map(|c| c.close).sum();
                     sma_values.push(sum / (current_slice.len() as f64) );

                } else {
                    let current_slice = &candles_data[(i - period + 1)..=i];
                    let sum: f64 = current_slice.iter().map(|c| c.close).sum();
                    sma_values.push(sum / (period as f64));
                }
            }
        }

        vec![
            Indicator {
                name: "SMA".to_string(),
                parameters: json!({"period": period}),
                values: sma_values,
            }
        ]
    });


    // Effect for global keyboard listener
    // This is a common way to handle global events in Dioxus.
    // Note: Focus issues can sometimes affect keyboard event capture.
    // The main window or a root div needs to be focusable or events might not bubble up as expected.
    use_effect(cx, (), move |_| {
        let desktop_context = window.webview.clone();
        let shortcut_str = app_config_for_shortcut.shortcuts.command_palette.to_lowercase(); // e.g., "ctrl+p"

        let mut ctrl_pressed = false;

        let keydown_listener = desktop_context.new_event_handler("keydown", move |event:Event<KeyboardData>| {
            if event.data.key().to_string().to_lowercase() == "control" {
                ctrl_pressed = true;
            } else if ctrl_pressed && event.data.key().to_string().to_lowercase() == shortcut_str.trim_start_matches("ctrl+") {
                app_state.write().command_palette_visible = !app_state.read().command_palette_visible;
                if app_state.read().command_palette_visible {
                    // Attempt to focus the input field - this is tricky and might not work directly here
                    // It often requires JavaScript interop or specific Dioxus features.
                    // For now, we rely on autofocus property of the input field itself when it becomes visible.
                    tracing::info!("Command Palette Toggled ON via shortcut. Input field should autofocus.");
                }
            }
        });

        let keyup_listener = desktop_context.new_event_handler("keyup", move |event:Event<KeyboardData>| {
             if event.data.key().to_string().to_lowercase() == "control" {
                ctrl_pressed = false;
            }
        });

        async move {
            // This is where you'd drop the listeners if the component unmounts
            // However, for the root App component, this is less critical as it lives for the app's lifetime.
            // To be proper, one would store `keydown_listener` and `keyup_listener` and drop them here.
            // For simplicity in this example, we're omitting explicit drop.
            // Dioxus event listeners are usually cleaned up when the webview_context they are associated with is dropped.
        }
    });


    rsx! {
        div {
            // It's good practice to allow the root div to be focusable for global key events.
            // tabindex: "0", // Making the div focusable. Might not be strictly needed if window events work well.
            // style: "outline: none; width: 100%; height: 100%;", // Remove default focus outline

            // Render the CommandPalette component
            CommandPalette {},
            // Main content area
            div {
                id: "main-content",
                style: "padding: 20px; color: #ddd; background-color: #1e1e1e; height: calc(100vh - 40px); display: flex; flex-direction: column; align-items: center;", // Adjusted style

                // Title and Command Palette toggle info
                div {
                    style: "text-align: center; margin-bottom: 20px;",
                    h1 { "Home Trader" }
                    p { "Press '{app_config.shortcuts.command_palette}' to open/close the command palette." }
                    button {
                        onclick: move |_| app_state_provider.write().command_palette_visible = !app_state_provider.read().command_palette_visible,
                        style: "padding: 8px 12px; background-color: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;",
                        "Toggle Command Palette"
                    }
                }

                // Candlestick Chart
                div {
                    style: "margin-top: 20px; border: 1px solid #555; box-shadow: 0 0 10px rgba(0,0,0,0.5);",
                    CandlestickChart {
                        candles: sample_candles.read().clone(),
                        width: 800.0,
                        height: 450.0,
                        indicator_data: Some(sample_sma_indicator.read().clone()) // Pass sample SMA data
                    }
                }
                // Placeholder for other UI elements like Toolbar, Indicator controls etc.
            }
        }
    }
}
