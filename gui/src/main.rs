#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_desktop; // Ensure dioxus_desktop is in scope

// Explicitly declare modules if app.rs is not automatically found as src/app.rs
// If app.rs is indeed src/app.rs, this line might not be strictly needed
// but doesn't hurt.
mod app;

fn main() {
    // Use the simplest launch function for Dioxus 0.5+
    dioxus_desktop::launch(app::App);
}
