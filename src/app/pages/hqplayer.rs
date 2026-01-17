//! HQPlayer page placeholder - to be migrated from inline JS.

use dioxus::prelude::*;

#[component]
pub fn HqPlayer() -> Element {
    rsx! {
        div { class: "container",
            h1 { "HQPlayer" }
            p { "This page is being migrated to Dioxus signals." }
        }
    }
}
