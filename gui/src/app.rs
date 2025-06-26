// Main application component for the GUI
// This will house the root of the Dioxus application components.

#![allow(non_snake_case)] // Common for Dioxus components

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    // Basic state for now, will be expanded
    let theme = use_state(cx, || "dark".to_string());
    // let current_view = use_state(cx, || "chart"); // Example state for navigation

    cx.render(rsx! {
        div {
            class: "app theme-{theme}", // Example of how theme might be applied

            // Placeholder for Toolbar/Menubar
            // Toolbar {}

            // Placeholder for Main Content Area
            main {
                class: "main-content",
                // match current_view.get().as_str() {
                //     "chart" => rsx! { ChartView {} },
                //     "settings" => rsx! { SettingsView {} },
                //     _ => rsx! { div { "Unknown view" } }
                // }
                h1 { "Home Trader GUI" }
                p { "Welcome to the Home Trader application."}
                // Example: ChartComponent {}
                // Example: CommandPalette {}
            }

            // Placeholder for Statusbar
            // StatusBar {}
        }
    })
}

// Placeholder for components that will be defined later
// #[component]
// fn Toolbar(cx: Scope) -> Element {
//     cx.render(rsx!( div { class: "toolbar", "Toolbar placeholder" }))
// }

// #[component]
// fn ChartView(cx: Scope) -> Element {
//     cx.render(rsx!( div { class: "chart-view", "Chart View Placeholder" }))
// }

// #[component]
// fn SettingsView(cx: Scope) -> Element {
//     cx.render(rsx!( div { class: "settings-view", "Settings View Placeholder" }))
// }

// #[component]
// fn StatusBar(cx: Scope) -> Element {
//     cx.render(rsx!( div { class: "statusbar", "Statusbar placeholder" }))
// }
