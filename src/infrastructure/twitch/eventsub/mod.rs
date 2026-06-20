//! Twitch EventSub WebSocket client, sliced by concern.
//!
//! - [`client`] — the `TwitchClient` value and reconnect/backoff supervision loop
//! - [`connection`] — per-connection lifecycle (connect → handshake → subscribe → listen)
//! - [`handshake`] — awaiting the `session_welcome` and extracting the session id
//! - [`subscription_catalog`] — the fixed catalog of subscription types
//! - [`subscriptions`] — the registration loop (with 401 refresh + retry)
//! - [`subscription_request`] — building and sending one Helix subscription request
//! - [`message_handler`] — dispatching decoded frames by message type
//! - [`event_parsing`] — turning notification payloads into domain `StreamEvent`s
//! - [`token_refresh`] — OAuth token refresh and persistence
//! - [`error`] — the shared `ClientError`

use std::time::Duration;

mod client;
mod connection;
mod error;
mod event_parsing;
mod handshake;
mod message_handler;
mod subscription_catalog;
mod subscription_request;
mod subscriptions;
mod token_refresh;

pub use client::TwitchClient;

const EVENTSUB_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const HELIX_SUBSCRIPTIONS_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
const TOKEN_REFRESH_URL: &str = "https://id.twitch.tv/oauth2/token";

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(120);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
