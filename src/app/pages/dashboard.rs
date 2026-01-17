//! Dashboard page placeholder.

use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Dashboard" }
            p { "Welcome to Unified Hi-Fi Control" }
        }
    }
}
