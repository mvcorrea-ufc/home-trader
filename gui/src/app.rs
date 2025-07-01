#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_desktop::use_window; // Import use_window

// Import necessary types
use crate::components::command_palette::CommandPalette;
use crate::config::AppConfig;
use crate::state::app_state::AppState;

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
            // Placeholder for other UI elements
            div {
                id: "main-content",
                style: "padding: 20px; text-align: center; color: #ddd; background-color: #222; height: 100vh;",
                h1 { "Home Trader" }
                p { "Press '{app_config.shortcuts.command_palette}' to open/close the command palette." }
                // Fallback button if shortcut is problematic
                button {
                    onclick: move |_| app_state_provider.write().command_palette_visible = !app_state_provider.read().command_palette_visible,
                    "Toggle Command Palette (Button)"
                }
            }
        }
    }
}
