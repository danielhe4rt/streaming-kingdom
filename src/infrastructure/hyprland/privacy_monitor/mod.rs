//! Privacy monitor — blur OBS capture whenever a sensitive window is open.
//!
//! Sliced by concern:
//! - [`messages`] — `PrivacyCommand` / `PrivacyStatus` exchanged with the TUI
//! - [`sensitive_match`] — matching a window title against sensitive patterns
//! - [`window_events`] — snapshotting + listening to Hyprland window lifecycle
//! - [`blur_state`] — driving the OBS blur on/off as windows change
//! - [`monitor`] — the spawn entry-point and the monitor task

mod blur_state;
mod messages;
mod monitor;
mod sensitive_match;
mod window_events;

pub use messages::{PrivacyCommand, PrivacyStatus};
pub use monitor::spawn;
