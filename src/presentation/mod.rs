//! Presentation layer — the toolkit's two outbound renderers (ADR-0001):
//!
//! - [`tui`] — the terminal control panel (nav shell: topbar → sidebar → content).
//! - [`http`] — the axum server that serves Overlays as OBS browser sources.
//!
//! Inbound HTTP (e.g. the Livepix webhook) stays in `infrastructure/`.

pub mod http;
pub mod synthetic;
pub mod tui;

pub use tui::{RunChannels, run};
