//! Page components for the Dioxus-based web UI.
//!
//! Each page is a Dioxus component that renders a full page using the Layout component.

pub mod dashboard;
pub mod settings;

pub use dashboard::DashboardPage;
pub use settings::SettingsPage;
