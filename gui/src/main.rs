// GUI main entry point using Dioxus
#![allow(non_snake_case)] // Common for Dioxus components

use dioxus::prelude::*;
// Desktop specific imports for Dioxus
use dioxus_desktop::{Config as DesktopConfig, WindowBuilder};

mod app;
mod components;
mod config;
mod services;
mod state; // Application state module

use app::App;
use state::app_state::AppState; // Import your global AppState
use config::AppConfig; // Import your AppConfig for loading

fn main() {
    // Initialize tracing subscriber for logging (optional, but good for dev)
    // Consider making this configurable or conditional for release builds.
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Home Trader GUI (Dioxus Desktop)...");

    // Attempt to load application configuration
    let app_config = match AppConfig::load_default() {
        Ok(cfg) => {
            tracing::info!("Successfully loaded default configuration version {}.", cfg.version);
            cfg
        }
        Err(e) => {
            tracing::error!("Failed to load default configuration: {}. Exiting.", e);
            // In a real app, might try loading a user config, or fallback to minimal defaults.
            // For now, panic if the embedded default config can't be loaded, as it's critical.
            panic!("Failed to load critical default configuration: {}", e);
        }
    };

    // Dioxus Desktop configuration
    let desktop_config = DesktopConfig::default()
        .with_window(WindowBuilder::new()
            .with_title("Home Trader")
            .with_inner_size(dioxus_desktop::wry::application::dpi::LogicalSize::new(1280, 720))
        );
        // Optional: Custom HTML index if needed for specific CSS/JS loading patterns
        // .with_custom_index(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Home Trader</title></head><body><div id="main"></div></body></html>"#.to_string());


    // Launch the Dioxus desktop application
    // The root component is `App`.
    // We also initialize shared state here.
    dioxus_desktop::launch_cfg(
        move |cx| { // Note: `move` closure
            // Initialize and provide the global AppState to the application context
            // This makes AppState available to all child components via `use_shared_state`.
            // We use the loaded app_config to initialize parts of AppState or make it available.
            use_shared_state_provider(cx, || AppState::default()); // Initialize with default

            // Optionally, make the full AppConfig available as shared state too, if needed globally
            use_shared_state_provider(cx, || app_config.clone()); // app_config needs to be clonable

            // Or, directly initialize AppState using parts of app_config if AppState::new(config) exists
            // use_shared_state_provider(cx, || AppState::new(app_config.clone()));

            render! { App {} }
        },
        desktop_config
    );

    tracing::info!("Home Trader GUI finished.");
}
