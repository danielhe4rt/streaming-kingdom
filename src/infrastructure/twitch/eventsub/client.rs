//! The `TwitchClient` value and its top-level reconnect/backoff supervision loop.

use std::collections::HashSet;

use tokio::sync::{broadcast, mpsc};
use tokio::time;

use crate::application::TwitchConfig;
use crate::domain::{AppEvent, StreamEvent};

use super::connection::ReconnectAction;
use super::{EVENTSUB_URL, INITIAL_RECONNECT_DELAY, MAX_RECONNECT_DELAY};

/// Twitch EventSub WebSocket client.
///
/// Connects to the Twitch EventSub WebSocket, subscribes to channel events,
/// and broadcasts parsed `StreamEvent`s on the provided broadcast channel.
pub struct TwitchClient {
    pub(super) config: TwitchConfig,
    pub(super) event_tx: broadcast::Sender<StreamEvent>,
    pub(super) app_event_tx: mpsc::Sender<AppEvent>,
    pub(super) http: reqwest::Client,
    /// Track seen message IDs for deduplication.
    pub(super) seen_ids: HashSet<String>,
}

impl TwitchClient {
    pub fn new(
        config: TwitchConfig,
        event_tx: broadcast::Sender<StreamEvent>,
        app_event_tx: mpsc::Sender<AppEvent>,
    ) -> Self {
        Self {
            config,
            event_tx,
            app_event_tx,
            http: reqwest::Client::new(),
            seen_ids: HashSet::new(),
        }
    }

    /// Run the EventSub client forever, reconnecting on failure.
    ///
    /// This is intended to be spawned as a tokio task.
    pub async fn run(mut self) {
        let mut delay = INITIAL_RECONNECT_DELAY;
        let mut connect_url = EVENTSUB_URL.to_string();

        loop {
            match self.connect_and_listen(&connect_url).await {
                Ok(ReconnectAction::ServerReconnect(url)) => {
                    tracing::info!("reconnecting to server-provided URL");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Info("Twitch: server-initiated reconnect".into()))
                        .await;
                    connect_url = url;
                    delay = INITIAL_RECONNECT_DELAY;
                    // Server reconnects preserve subscriptions, connect immediately.
                }
                Ok(ReconnectAction::Disconnected) | Err(_) => {
                    tracing::warn!("disconnected, reconnecting in {delay:?}");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Error(format!(
                            "Twitch: disconnected, reconnecting in {delay:?}"
                        )))
                        .await;
                    time::sleep(delay).await;
                    delay = (delay * 2).min(MAX_RECONNECT_DELAY);
                    // Reset to default URL on unexpected disconnect.
                    connect_url = EVENTSUB_URL.to_string();
                    // Clear dedup set on reconnect — stale IDs won't repeat.
                    self.seen_ids.clear();
                }
            }
        }
    }
}
