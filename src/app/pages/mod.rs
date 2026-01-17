//! Dioxus fullstack page components.
//!
//! These pages use Dioxus signals and server functions instead of inline JavaScript.

mod settings;
mod dashboard;
mod zones;
mod lms;
mod hqplayer;
mod knobs;
mod zone;

pub use settings::Settings;
pub use dashboard::Dashboard;
pub use zones::Zones;
pub use lms::Lms;
pub use hqplayer::HqPlayer;
pub use knobs::Knobs;
pub use zone::Zone;
