//! Dispatch of decoded WebSocket text frames to their EventSub message-type handlers.

use serde_json::Value;

use crate::domain::AppEvent;

use super::client::TwitchClient;
use super::event_parsing::parse_event;

/// Outcome of handling a single WebSocket message.
pub(super) enum MessageResult {
    Continue,
    Reconnect(String),
    /// Token was revoked — caller should attempt refresh and reconnect.
    TokenRevoked,
}

impl TwitchClient {
    /// Handle a single text message from the WebSocket.
    pub(super) fn handle_message(&mut self, text: &str) -> MessageResult {
        let msg: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("failed to parse message: {e}");
                return MessageResult::Continue;
            }
        };

        let msg_type = msg["metadata"]["message_type"].as_str().unwrap_or("");

        match msg_type {
            "session_keepalive" => {
                // Nothing to do — receipt of message resets keepalive timer.
            }

            "notification" => {
                // Deduplicate by message_id.
                if let Some(id) = msg["metadata"]["message_id"].as_str() {
                    if !self.seen_ids.insert(id.to_string()) {
                        tracing::debug!(id, "duplicate message, skipping");
                        return MessageResult::Continue;
                    }
                    // Cap dedup set size to avoid unbounded growth.
                    if self.seen_ids.len() > 10_000 {
                        self.seen_ids.clear();
                    }
                }

                let sub_type = msg["payload"]["subscription"]["type"]
                    .as_str()
                    .unwrap_or("");
                let event = &msg["payload"]["event"];

                if let Some(stream_event) = parse_event(sub_type, event) {
                    let _ = self
                        .app_event_tx
                        .try_send(AppEvent::Info(format!("Twitch: received {sub_type} event")));
                    let _ = self.event_tx.send(stream_event);
                }
            }

            "session_reconnect" => {
                if let Some(url) = msg["payload"]["session"]["reconnect_url"].as_str() {
                    tracing::info!("received reconnect directive");
                    return MessageResult::Reconnect(url.to_string());
                }
            }

            "revocation" => {
                let sub_type = msg["payload"]["subscription"]["type"]
                    .as_str()
                    .unwrap_or("unknown");
                let reason = msg["payload"]["subscription"]["status"]
                    .as_str()
                    .unwrap_or("unknown");
                tracing::warn!(sub_type, reason, "subscription revoked");
                let _ = self.app_event_tx.try_send(AppEvent::Error(format!(
                    "Twitch: {sub_type} subscription revoked ({reason})"
                )));

                if reason == "authorization_revoked" {
                    return MessageResult::TokenRevoked;
                }
            }

            other => {
                tracing::debug!(msg_type = other, "unhandled message type");
            }
        }

        MessageResult::Continue
    }
}
