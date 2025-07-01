#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_desktop::use_window; // Import use_window
use chrono::{TimeZone, Utc}; // For creating DateTime<Utc>

// Import necessary types
use crate::components::command_palette::CommandPalette;
use crate::components::chart::candlestick::CandlestickChart; // Import CandlestickChart
use crate::config::AppConfig;
use crate::state::app_state::AppState;
use crate::services::engine_client::EngineClient; // Import EngineClient
use shared::models::{Candle, Indicator, MarketData}; // Import Candle, Indicator, MarketData
use serde_json::json; // For creating dummy json parameters for Indicator

#[component]
pub fn App() -> Element {
    // Load AppConfig
    let app_config = match AppConfig::load_default() {
        Ok(config) => config,
        Err(e) => {
            // Consider a more graceful error display than panic in a real app
            panic!("Failed to load default configuration: {:?}", e);
        }
    };

    // Provide AppState, AppConfig, and EngineClient (Option) to the component tree
    use_shared_state_provider(cx, AppState::default);
    use_shared_state_provider(cx, || app_config.clone());
    use_shared_state_provider::<Option<EngineClient>>(cx, || None);


    let window = use_window(cx);
    let app_state_ref = use_shared_state::<AppState>(cx).unwrap();
    let app_config_ref = use_shared_state::<AppConfig>(cx).unwrap();
    let engine_client_ref = use_shared_state::<Option<EngineClient>>(cx).unwrap();

    // Clone necessary items for the async future
    let app_config_clone_for_future = app_config_ref.read().clone();
    let mut engine_client_writer = engine_client_ref.clone(); // Clone the UseSharedState handle

    // Initialize EngineClient asynchronously
    use_future(cx, (), |_| {
        let engine_config = app_config_clone_for_future.engine.clone();
        let mut app_state_writer_for_error = app_state_ref.clone();
        async move {
            let endpoint = format!("http://{}:{}", engine_config.host, engine_config.port);
            match EngineClient::new(endpoint).await {
                Ok(client) => {
                    *engine_client_writer.write() = Some(client);
                    tracing::info!("Successfully connected to trading engine.");
                }
                Err(e) => {
                    let error_msg = format!("Failed to connect to trading engine: {}", e);
                    tracing::error!("{}", error_msg);
                    app_state_writer_for_error.write().error_message = Some(error_msg);
                }
            }
        }
    });

    // Get necessary state for rendering
    let app_state_reader = app_state_ref.read();
    let display_candles = app_state_reader.current_candles_display.clone();
    let display_indicators = app_state_reader.current_indicators_display.clone();
    let is_loading = app_state_reader.is_loading;
    let error_message = app_state_reader.error_message.clone();
    let current_symbol = app_state_reader.current_symbol_display.clone();
    // Drop the read lock
    drop(app_state_reader);

    // Effect for global keyboard listener
    // This is a common way to handle global events in Dioxus.
    // Use app_config_ref for shortcut string
    let app_config_for_shortcut = app_config_ref.read().clone();
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
                    // Use app_config_ref for shortcut display
                    p { "Press '{app_config_ref.read().shortcuts.command_palette}' to open/close the command palette." }
                    button {
                        // Use app_state_ref for onclick
                        onclick: move |_| app_state_ref.write().command_palette_visible = !app_state_ref.read().command_palette_visible,
                        style: "padding: 8px 12px; background-color: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;",
                        "Toggle Command Palette"
                    }
                }

                // Display loading status and error messages
                if is_loading {
                    rsx! { p { style: "color: yellow;", "Loading data..." } }
                }
                if let Some(err_msg) = &error_message {
                    rsx! { p { style: "color: red;", "Error: {err_msg}" } }
                }
                if let Some(symbol) = &current_symbol {
                    rsx! { h3 { "Displaying: {symbol}" } }
                }


                // Candlestick Chart
                div {
                    style: "margin-top: 20px; border: 1px solid #555; box-shadow: 0 0 10px rgba(0,0,0,0.5);",
                    // Pass dynamic data to CandlestickChart
                    // Ensure display_candles and display_indicators are correctly typed for the chart
                    // The chart component will need to handle Option<Vec<Candle>>
                    CandlestickChart {
                        candles: display_candles.unwrap_or_default(), // Pass empty vec if None, or chart handles Option
                        width: 800.0,
                        height: 450.0,
                        indicator_data: Some(display_indicators) // Pass current indicators
                    }
                }
                // Placeholder for other UI elements like Toolbar, Indicator controls etc.
            }
        }
    }
}
