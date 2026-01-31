use std::collections::HashSet;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio::time;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::app::{StreamEvent, SubTier};
use crate::config::TwitchConfig;

const EVENTSUB_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const HELIX_SUBSCRIPTIONS_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
const TOKEN_REFRESH_URL: &str = "https://id.twitch.tv/oauth2/token";

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(120);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Subscription type definition for EventSub registration.
struct SubDef {
    sub_type: &'static str,
    version: &'static str,
    /// Whether the condition uses `broadcaster_user_id` (true) or
    /// `to_broadcaster_user_id` (false, for raids).
    uses_broadcaster: bool,
    /// Whether `moderator_user_id` must also be set (channel.follow v2).
    needs_moderator: bool,
}

const SUBSCRIPTIONS: &[SubDef] = &[
    SubDef {
        sub_type: "channel.follow",
        version: "2",
        uses_broadcaster: true,
        needs_moderator: true,
    },
    SubDef {
        sub_type: "channel.subscribe",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.subscription.message",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.subscription.gift",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.cheer",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.raid",
        version: "1",
        uses_broadcaster: false,
        needs_moderator: false,
    },
];

/// Twitch EventSub WebSocket client.
///
/// Connects to the Twitch EventSub WebSocket, subscribes to channel events,
/// and broadcasts parsed `StreamEvent`s on the provided broadcast channel.
pub struct TwitchClient {
    config: TwitchConfig,
    event_tx: broadcast::Sender<StreamEvent>,
    http: reqwest::Client,
    /// Track seen message IDs for deduplication.
    seen_ids: HashSet<String>,
}

impl TwitchClient {
    pub fn new(config: TwitchConfig, event_tx: broadcast::Sender<StreamEvent>) -> Self {
        Self {
            config,
            event_tx,
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
                    connect_url = url;
                    delay = INITIAL_RECONNECT_DELAY;
                    // Server reconnects preserve subscriptions, connect immediately.
                }
                Ok(ReconnectAction::Disconnected) | Err(_) => {
                    tracing::warn!("disconnected, reconnecting in {delay:?}");
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

    /// Connect to the WebSocket, subscribe to events, and listen.
    ///
    /// Returns `Ok(ReconnectAction)` to indicate why the loop exited.
    async fn connect_and_listen(&mut self, url: &str) -> Result<ReconnectAction, ClientError> {
        let (ws, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::Connect(e.to_string()))?;

        let (mut sink, mut stream) = ws.split();

        tracing::info!("connected to EventSub WebSocket");

        // Step 1: Wait for the Welcome message (contains session_id).
        let session_id = wait_for_welcome(&mut stream).await?;
        tracing::info!(session_id, "received welcome");

        // Step 2: Subscribe to all event types.
        // Only subscribe on fresh connections (not server-initiated reconnects
        // which already preserve subscriptions). We can tell because server
        // reconnects provide a non-default URL.
        if url == EVENTSUB_URL {
            if let Err(e) = self.subscribe_events(&session_id).await {
                tracing::error!("failed to subscribe to events: {e}");
                return Err(e);
            }
        }

        // Step 3: Listen for messages until disconnect or reconnect.
        // The keepalive timeout is sent in the welcome message, but Twitch
        // defaults to 10 seconds. We use a generous timeout.
        let keepalive_timeout = Duration::from_secs(30);

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
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    tracing::info!("server closed connection");
                    return Ok(ReconnectAction::Disconnected);
                }
                Ok(Some(Ok(_))) => {
                    // Ping/Pong/Binary — ignore.
                }
                Ok(Some(Err(e))) => {
                    tracing::error!("WebSocket error: {e}");
                    return Err(ClientError::WebSocket(e.to_string()));
                }
                Ok(None) => {
                    tracing::info!("WebSocket stream ended");
                    return Ok(ReconnectAction::Disconnected);
                }
                Err(_) => {
                    tracing::warn!("keepalive timeout — server may be gone");
                    return Ok(ReconnectAction::Disconnected);
                }
            }
        }
    }

    /// Create EventSub subscriptions via the Helix REST API.
    async fn subscribe_events(&self, session_id: &str) -> Result<(), ClientError> {
        for def in SUBSCRIPTIONS {
            let mut condition = serde_json::Map::new();

            if def.uses_broadcaster {
                condition.insert(
                    "broadcaster_user_id".into(),
                    Value::String(self.config.broadcaster_user_id.clone()),
                );
            } else {
                // channel.raid uses to_broadcaster_user_id
                condition.insert(
                    "to_broadcaster_user_id".into(),
                    Value::String(self.config.broadcaster_user_id.clone()),
                );
            }

            if def.needs_moderator {
                condition.insert(
                    "moderator_user_id".into(),
                    Value::String(self.config.broadcaster_user_id.clone()),
                );
            }

            let body = serde_json::json!({
                "type": def.sub_type,
                "version": def.version,
                "condition": condition,
                "transport": {
                    "method": "websocket",
                    "session_id": session_id,
                }
            });

            let resp = self
                .http
                .post(HELIX_SUBSCRIPTIONS_URL)
                .header("Authorization", format!("Bearer {}", self.config.oauth_token))
                .header("Client-Id", &self.config.client_id)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ClientError::Http(e.to_string()))?;

            if resp.status().is_success() {
                tracing::info!(sub_type = def.sub_type, "subscribed");
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::error!(
                    sub_type = def.sub_type,
                    status = %status,
                    body,
                    "subscription failed"
                );
                // Don't abort on individual subscription failures —
                // some scopes may not be authorized.
            }
        }

        Ok(())
    }

    /// Handle a single text message from the WebSocket.
    fn handle_message(&mut self, text: &str) -> MessageResult {
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
            }

            other => {
                tracing::debug!(msg_type = other, "unhandled message type");
            }
        }

        MessageResult::Continue
    }

    /// Attempt to refresh the OAuth token using the refresh_token.
    ///
    /// Returns the new access token on success.
    #[allow(dead_code)]
    pub(crate) async fn refresh_token(&self) -> Result<String, ClientError> {
        let resp = self
            .http
            .post(TOKEN_REFRESH_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &self.config.refresh_token),
                ("client_id", &self.config.client_id),
            ])
            .send()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Http(format!("token refresh failed: {body}")));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;

        body["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ClientError::Http("no access_token in response".into()))
    }
}

// ---------------------------------------------------------------------------
// Message parsing
// ---------------------------------------------------------------------------

/// Wait for the welcome message and extract the session_id.
async fn wait_for_welcome(
    stream: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<String, ClientError> {
    let deadline = time::Instant::now() + Duration::from_secs(15);

    loop {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        if remaining.is_zero() {
            return Err(ClientError::Connect("welcome timeout".into()));
        }

        match time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let msg: Value = serde_json::from_str(&text)
                    .map_err(|e| ClientError::Connect(format!("parse welcome: {e}")))?;

                if msg["metadata"]["message_type"].as_str() == Some("session_welcome") {
                    let session_id = msg["payload"]["session"]["id"]
                        .as_str()
                        .ok_or_else(|| {
                            ClientError::Connect("no session_id in welcome".into())
                        })?
                        .to_string();
                    return Ok(session_id);
                }
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => {
                return Err(ClientError::WebSocket(e.to_string()));
            }
            Ok(None) => {
                return Err(ClientError::Connect("stream ended before welcome".into()));
            }
            Err(_) => {
                return Err(ClientError::Connect("welcome timeout".into()));
            }
        }
    }
}

/// Parse a Twitch EventSub notification into a `StreamEvent`.
fn parse_event(sub_type: &str, event: &Value) -> Option<StreamEvent> {
    match sub_type {
        "channel.follow" => {
            let username = event["user_name"].as_str()?.to_string();
            Some(StreamEvent::Follow { username })
        }

        "channel.subscribe" => {
            let username = event["user_name"].as_str()?.to_string();
            let tier = parse_tier(event["tier"].as_str()?);
            Some(StreamEvent::Sub {
                username,
                tier,
                months: 1,
            })
        }

        "channel.subscription.message" => {
            let username = event["user_name"].as_str()?.to_string();
            let tier = parse_tier(event["tier"].as_str()?);
            let months = event["cumulative_months"].as_u64().unwrap_or(1) as u32;
            Some(StreamEvent::Sub {
                username,
                tier,
                months,
            })
        }

        "channel.subscription.gift" => {
            let username = if event["is_anonymous"].as_bool() == Some(true) {
                "Anonymous".to_string()
            } else {
                event["user_name"]
                    .as_str()
                    .unwrap_or("Anonymous")
                    .to_string()
            };
            let tier = parse_tier(event["tier"].as_str().unwrap_or("1000"));
            let total = event["total"].as_u64().unwrap_or(1) as u32;
            Some(StreamEvent::GiftSub {
                username,
                tier,
                total,
            })
        }

        "channel.cheer" => {
            let username = if event["is_anonymous"].as_bool() == Some(true) {
                "Anonymous".to_string()
            } else {
                event["user_name"]
                    .as_str()
                    .unwrap_or("Anonymous")
                    .to_string()
            };
            let bits = event["bits"].as_u64().unwrap_or(0);
            let message = event["message"].as_str().unwrap_or("").to_string();
            Some(StreamEvent::Cheer {
                username,
                bits,
                message,
            })
        }

        "channel.raid" => {
            let from_channel = event["from_broadcaster_user_name"].as_str()?.to_string();
            let viewers = event["viewers"].as_u64().unwrap_or(0) as u32;
            Some(StreamEvent::Raid {
                from_channel,
                viewers,
            })
        }

        _ => {
            tracing::debug!(sub_type, "unhandled subscription type");
            None
        }
    }
}

/// Convert Twitch tier string ("1000", "2000", "3000") to `SubTier`.
fn parse_tier(tier: &str) -> SubTier {
    match tier {
        "2000" => SubTier::Tier2,
        "3000" => SubTier::Tier3,
        _ => SubTier::Tier1,
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

enum ReconnectAction {
    /// Server sent a reconnect message with a new URL.
    ServerReconnect(String),
    /// Connection was lost (close frame, error, timeout).
    Disconnected,
}

enum MessageResult {
    Continue,
    Reconnect(String),
}

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
