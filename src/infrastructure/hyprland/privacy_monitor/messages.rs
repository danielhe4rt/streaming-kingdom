//! Commands & status messages exchanged with the TUI.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
pub enum PrivacyStatus {
    /// Hyprland listener is running.
    Running,
    /// Listener stopped cleanly.
    Stopped,
    /// A sensitive window was detected; blur activated.
    BlurEnabled { title: String },
    /// Switched away from sensitive window; blur removed.
    BlurDisabled,
    /// Non-fatal error (e.g. OBS not reachable).
    Error(String),
}
