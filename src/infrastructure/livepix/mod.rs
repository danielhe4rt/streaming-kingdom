pub mod tts;
mod webhook;

pub use tts::TtsRequest;
pub use webhook::spawn;

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
