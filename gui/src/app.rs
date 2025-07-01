#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_desktop::use_window; // For DesktopContext and use_window

// Import necessary types
use crate::components::command_palette::CommandPalette;
use crate::components::chart::candlestick::CandlestickChart;
use crate::config::AppConfig;
use crate::state::app_state::AppState;
use crate::services::engine_client::EngineClient;
use shared::models::{Candle, Indicator}; // Candle & Indicator used for CandlestickChart props

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
    use_shared_state_provider(AppState::default);
    use_shared_state_provider(|| app_config.clone());
    use_shared_state_provider::<Option<EngineClient>>(|| None);


    let window = use_window(); // Removed cx
    let app_state_ref = use_shared_state::<AppState>().unwrap(); // Removed cx
    let app_config_ref = use_shared_state::<AppConfig>().unwrap(); // Removed cx
    let engine_client_ref = use_shared_state::<Option<EngineClient>>().unwrap(); // Removed cx

    // Clone necessary items for the async future
    let app_config_clone_for_future = app_config_ref.read().clone();
    let mut engine_client_writer = engine_client_ref.clone(); // Clone the UseSharedState handle
    let app_state_handle_for_future = app_state_ref.clone(); // Clone for the future

    // Initialize EngineClient asynchronously
    use_future((), move |_| { // Removed cx, added move for captures
        // app_config_clone_for_future, engine_client_writer are captured by move
        // app_state_handle_for_future is also captured
        let engine_config_captured = app_config_clone_for_future.engine.clone(); // Clone data for async block

        async move {
            let endpoint = format!("http://{}:{}", engine_config_captured.host, engine_config_captured.port);
            match EngineClient::new(endpoint).await {
                Ok(client) => {
                    *engine_client_writer.write() = Some(client);
                    tracing::info!("Successfully connected to trading engine.");
                }
                Err(e) => {
                    let error_msg = format!("Failed to connect to trading engine: {}", e);
                    tracing::error!("{}", error_msg);
                    app_state_handle_for_future.write().error_message = Some(error_msg);
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

    // State for tracking Ctrl key press for global shortcut
    let ctrl_pressed_for_shortcut = use_ref(|| false);
    // Clone necessary handles for onkeydown/onkeyup closures
    let app_state_for_shortcut_handler = app_state_ref.clone();
    let app_config_for_shortcut_handler = app_config_ref.read().clone();


    rsx! {
        div {
            // Make the root div focusable and handle key events for global-like shortcuts
            tabindex: "0", // Important for receiving focus and key events
            style: "outline: none; width: 100%; height: 100%;", // Remove default focus outline
            onmounted: move |event| {
                // Attempt to focus the div when it's mounted to catch keyboard events.
                // This might require specific handling based on Dioxus version for focusing elements.
                // For now, rely on tabindex and user clicking into the app area.
                // Or, use JS interop to focus if absolutely necessary.
                tracing::info!("Root div mounted. Set tabindex=0 to allow focus for keyboard shortcuts.");
            },
            onkeydown: move |event: Event<KeyboardData>| {
                let shortcut_str = app_config_for_shortcut_handler.shortcuts.command_palette.to_lowercase();
                if event.key().to_string().to_lowercase() == "control" {
                    *ctrl_pressed_for_shortcut.write() = true;
                } else if *ctrl_pressed_for_shortcut.read() && event.key().to_string().to_lowercase() == shortcut_str.trim_start_matches("ctrl+") {
                    let mut app_state_writer = app_state_for_shortcut_handler.write();
                    app_state_writer.command_palette_visible = !app_state_writer.command_palette_visible;
                    if app_state_writer.command_palette_visible {
                        tracing::info!("Command Palette Toggled ON via shortcut. Input field should autofocus.");
                    }
                }
            },
            onkeyup: move |event: Event<KeyboardData>| {
                if event.key().to_string().to_lowercase() == "control" {
                    *ctrl_pressed_for_shortcut.write() = false;
                }
            },

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
                {is_loading.then(|| rsx! { p { style: "color: yellow;", "Loading data..." } })}
                {error_message.as_ref().map(|err_msg| rsx! { p { style: "color: red;", "Error: {err_msg}" } })}
                {current_symbol.as_ref().map(|symbol| rsx! { h3 { "Displaying: {symbol}" } })}


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
