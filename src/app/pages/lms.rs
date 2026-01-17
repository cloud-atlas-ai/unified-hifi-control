//! LMS page placeholder - to be migrated from inline JS.

use dioxus::prelude::*;

#[component]
pub fn Lms() -> Element {
    rsx! {
        div { class: "container",
            h1 { "LMS" }
            p { "This page is being migrated to Dioxus signals." }
        }
    }
}
