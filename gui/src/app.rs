#![allow(non_snake_case)]
use dioxus::prelude::*;

#[component]
pub fn App() -> Element { // cx is implicitly available to rsx! macro
    rsx! {
        div { "Hello, Dioxus!" }
    }
}
