//! Zones page placeholder - to be migrated from inline JS.

use dioxus::prelude::*;

#[component]
pub fn Zones() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Zones" }
            p { "This page is being migrated to Dioxus signals." }
        }
    }
}
