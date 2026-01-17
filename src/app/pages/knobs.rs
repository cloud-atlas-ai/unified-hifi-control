//! Knobs page placeholder - to be migrated from inline JS.

use dioxus::prelude::*;

#[component]
pub fn Knobs() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Knobs" }
            p { "This page is being migrated to Dioxus signals." }
        }
    }
}
