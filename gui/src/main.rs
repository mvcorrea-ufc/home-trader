#![allow(non_snake_case)]
// use dioxus::prelude::*; // Unused import
use dioxus_desktop::{Config, LogicalSize}; // Import Config and LogicalSize for window setup

// Explicitly declare modules if app.rs is not automatically found as src/app.rs
// If app.rs is indeed src/app.rs, this line might not be strictly needed
// but doesn't hurt.
mod app;
mod components;
mod config;
mod services;
mod state;

fn main() {
    // Use the simplest launch function for Dioxus 0.5+
    // pub fn launch(root: fn() -> Element, platform_event_handlers: Vec<ExternalListener>, cfg: Config)
    // The middle argument seems to be for platform event handlers, which we don't have, so pass vec![].
    dioxus_desktop::launch::launch(
        app::App,
        vec![], // No platform event handlers for now
        Config::default().with_window(
            dioxus_desktop::WindowBuilder::new()
                .with_title("Home Trader")
                .with_inner_size(LogicalSize::new(800.0, 600.0)),
        ),
    );
}
