mod webhook;

pub use webhook::spawn;

// Re-export TtsRequest from elevenlabs so webhook.rs can use it via super::
pub use crate::infrastructure::elevenlabs::TtsRequest;

// ---------------------------------------------------------------------------
// Commands & status messages exchanged with the TUI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivepixCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
pub enum LivepixStatus {
    Running { port: u16 },
    Stopped,
    OAuthSuccess,
    OAuthError(String),
    WebhookReceived { username: String, amount: String },
    Error(String),
}
