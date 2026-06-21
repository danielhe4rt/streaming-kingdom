//! Per-connection lifecycle: connect, handshake, subscribe, then pump messages until exit.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::domain::AppEvent;

use super::EVENTSUB_URL;
use super::client::TwitchClient;
use super::error::ClientError;
use super::handshake::wait_for_welcome;
use super::message_handler::MessageResult;

/// Why a single connection's listen loop exited.
pub(super) enum ReconnectAction {
    /// Server sent a reconnect message with a new URL.
    ServerReconnect(String),
    /// Connection was lost (close frame, error, timeout).
    Disconnected,
}

impl TwitchClient {
    /// Connect to the WebSocket, subscribe to events, and listen.
    ///
    /// Returns `Ok(ReconnectAction)` to indicate why the loop exited.
    pub(super) async fn connect_and_listen(
        &mut self,
        url: &str,
    ) -> Result<ReconnectAction, ClientError> {
        let (ws, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::Connect(e.to_string()))?;

        let (mut sink, mut stream) = ws.split();

        tracing::info!("connected to EventSub WebSocket");
        let _ = self
            .app_event_tx
            .send(AppEvent::Info("Twitch: connected to EventSub".into()))
            .await;

        // Step 1: Wait for the Welcome message (contains session_id).
        let session_id = wait_for_welcome(&mut stream).await?;
        tracing::info!(session_id, "received welcome");
        let _ = self
            .app_event_tx
            .send(AppEvent::Info("Twitch: session established".into()))
            .await;

        // Step 2: Subscribe to all event types.
        // Only subscribe on fresh connections (not server-initiated reconnects
        // which already preserve subscriptions). We can tell because server
        // reconnects provide a non-default URL.
        if url == EVENTSUB_URL
            && let Err(e) = self.subscribe_events(&session_id).await
        {
            tracing::error!("failed to subscribe to events: {e}");
            return Err(e);
        }

        // Step 3: Listen for messages until disconnect or reconnect.
        // The keepalive timeout is sent in the welcome message, but Twitch
        // defaults to 10 seconds. We use a generous timeout.
        let keepalive_timeout = Duration::from_secs(30);
        let _ = self
            .app_event_tx
            .send(AppEvent::Info("Twitch: listening for events".into()))
            .await;

        loop {
            let msg = time::timeout(keepalive_timeout, stream.next()).await;

            match msg {
                Ok(Some(Ok(Message::Text(text)))) => {
                    match self.handle_message(&text) {
                        MessageResult::Continue => {}
                        MessageResult::Reconnect(url) => {
                            // Close current connection gracefully.
                            let _ = sink.close().await;
                            return Ok(ReconnectAction::ServerReconnect(url));
                        }
                        MessageResult::TokenRevoked => {
                            tracing::warn!("authorization revoked, attempting token refresh");
                            let _ = sink.close().await;
                            // Attempt refresh; if it fails, the outer loop
                            // will apply backoff and retry.
                            if let Err(e) = self.refresh_token().await {
                                tracing::error!("token refresh after revocation failed: {e}");
                                return Err(ClientError::Http(format!(
                                    "token refresh after revocation failed: {e}"
                                )));
                            }
                            // Refresh succeeded — reconnect from scratch to
                            // re-subscribe with the new token.
                            return Ok(ReconnectAction::Disconnected);
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    tracing::info!("server closed connection");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Info("Twitch: server closed connection".into()))
                        .await;
                    return Ok(ReconnectAction::Disconnected);
                }
                Ok(Some(Ok(_))) => {
                    // Ping/Pong/Binary — ignore.
                }
                Ok(Some(Err(e))) => {
                    tracing::error!("WebSocket error: {e}");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Error(format!("Twitch: WebSocket error: {e}")))
                        .await;
                    return Err(ClientError::WebSocket(e.to_string()));
                }
                Ok(None) => {
                    tracing::info!("WebSocket stream ended");
                    return Ok(ReconnectAction::Disconnected);
                }
                Err(_) => {
                    tracing::warn!("keepalive timeout — server may be gone");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Error(
                            "Twitch: keepalive timeout, reconnecting...".into(),
                        ))
                        .await;
                    return Ok(ReconnectAction::Disconnected);
                }
            }
        }
    }
}
