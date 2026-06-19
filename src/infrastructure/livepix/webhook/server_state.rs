//! Shared state threaded through the Axum handlers and helper calls.

use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, Mutex};

use crate::domain::StreamEvent;

use super::super::{LivepixStatus, TtsRequest};

#[derive(Clone)]
pub(super) struct ServerState {
    pub client_id: String,
    pub client_secret: String,
    pub http_client: reqwest::Client,
    pub token: Arc<Mutex<Option<String>>>,
    pub event_tx: broadcast::Sender<StreamEvent>,
    pub status_tx: mpsc::Sender<LivepixStatus>,
    pub tts_tx: Option<mpsc::Sender<TtsRequest>>,
}
