//! Error type shared across the EventSub client slices.

#[derive(Debug)]
pub(crate) enum ClientError {
    Connect(String),
    WebSocket(String),
    Http(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(e) => write!(f, "connect error: {e}"),
            Self::WebSocket(e) => write!(f, "WebSocket error: {e}"),
            Self::Http(e) => write!(f, "HTTP error: {e}"),
        }
    }
}
