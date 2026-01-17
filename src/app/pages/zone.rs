//! Zone page placeholder - to be migrated from inline JS.

use dioxus::prelude::*;

#[component]
pub fn Zone() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Zone" }
            p { "This page is being migrated to Dioxus signals." }
        }
    }
}
