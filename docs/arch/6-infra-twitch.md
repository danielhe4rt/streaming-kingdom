# Infrastructure — twitch

> **Keep in sync:** this file documents `src/infrastructure/twitch`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

Twitch is a live streaming platform providing two primary APIs that this toolkit uses: (1) **EventSub** — a WebSocket-based subscription API delivering real-time channel events (follows, subscriptions, cheers, raids, etc.), and (2) **IRC** — the Twitch chat protocol for reading live messages from a channel. Both require OAuth 2.0 authentication. This module bridges those APIs with the application layer.

## What we use from it

### EventSub WebSocket (`eventsub.rs`)

**Protocol & Endpoint:**
- **WebSocket:** `wss://eventsub.wss.twitch.tv/ws` (TLS-secured WebSocket)
- **Crates:** `tokio-tungstenite` (0.26, with `native-tls`), `futures-util`, `tokio` (1.47.1, full features)
- **HTTP (for subscription registration):** `reqwest` (0.12, with JSON + native-TLS)

**Event subscriptions** (registered via Helix REST API `https://api.twitch.tv/helix/eventsub/subscriptions`):

| Type | Version | Used for |
|------|---------|----------|
| `channel.follow` | 2 | Emit `StreamEvent::Follow { username }` |
| `channel.subscribe` | 1 | Emit `StreamEvent::Sub` (new month) |
| `channel.subscription.message` | 1 | Emit `StreamEvent::Sub` (resub with cumulative months + message) |
| `channel.subscription.gift` | 1 | Emit `StreamEvent::GiftSub` (batch gifted subs) |
| `channel.cheer` | 1 | Emit `StreamEvent::Cheer { username, bits, message }` |
| `channel.raid` | 1 | Emit `StreamEvent::Raid { from_channel, viewers }` |

**OAuth token refresh:**
- Endpoint: `https://id.twitch.tv/oauth2/token` (POST)
- Triggered if a subscription fails with HTTP 401, if revocation is received, or on WebSocket reconnect after token revocation
- Updates `TwitchConfig.oauth_token` and `TwitchConfig.refresh_token` in memory and persists them via `application::config::save_twitch_tokens()`

**Message deduplication:**
- Tracks seen `metadata.message_id` in a `HashSet` (capped at 10,000 entries) to filter duplicate notifications
- Clears on reconnect to prevent stale IDs from blocking legitimate events

**Keepalive & reconnection:**
- Listens on a 30-second keepalive timeout; if no message received, reconnects
- Backoff strategy: initial 1s delay, doubles up to 120s max
- Server-initiated reconnects (via `session_reconnect` message) use the provided URL and preserve subscriptions; non-server disconnects reset to default URL

**Crates used:**
- `tokio` — async task spawning and channels
- `tokio-tungstenite` — WebSocket client
- `futures-util` — stream splitting, `SinkExt`, `StreamExt` traits
- `serde_json` — parsing event payloads
- `reqwest` — HTTP requests for subscription registration and token refresh
- `tokio::sync::{broadcast, mpsc}` — channels for broadcasting parsed events and app notifications

### IRC Chat (`irc.rs`)

**Protocol & Endpoint:**
- **IRC over TLS TCP:** Twitch IRC default endpoints (handled by the `twitch-irc` crate)
- **Crate:** `twitch-irc` (5.x, `transport-tcp-native-tls` feature only)

**Authentication:**
- If `login_name` and `oauth_token` are provided, connects as an authenticated user (can send messages)
- If not, connects anonymously (read-only chat)

**Behavior:**
- Joins the specified channel (from `TwitchConfig.channel`)
- Listens for `ServerMessage::Privmsg` (chat messages)
- Emits `ChatMessage { username, text, channel }` to the application layer
- Auto-reconnects on disconnect with the same backoff strategy as EventSub (1s → 120s)

**Crates used:**
- `twitch-irc` — IRC protocol and Twitch-specific chat features
- `tokio::sync::mpsc` — channels for sending chat messages and app events

### OAuth 2.0 Authentication (`auth.rs`)

**Endpoints:**
- **Authorization:** `https://id.twitch.tv/oauth2/authorize` (browser redirect)
- **Token exchange:** `https://id.twitch.tv/oauth2/token` (POST)
- **Token validation:** `https://id.twitch.tv/oauth2/validate` (GET with token header)

**Scope set** (`SCOPES` const):

```
"chat:read", "chat:write" (IRC chat)
"user:read:chat", "user:write:chat" (EventSub message/raiding)
"moderator:read:followers" (EventSub follow events)
"channel:read:subscriptions" (EventSub sub events)
"bits:read" (EventSub cheer events)
```

**OAuth flow** (`authenticate()` function):

1. Starts a temporary HTTP server on `localhost:3000` (redirect endpoint)
2. Constructs authorization URL with `force_verify=true` (always shows consent screen)
3. Opens browser via `xdg-open` (or prints URL if xdg-open fails)
4. Waits for callback with `code` parameter (using a `oneshot` channel)
5. Exchanges code for access + refresh tokens via POST to token endpoint
6. Returns `TokenResponse { access_token, refresh_token }`
7. Gracefully shuts down the HTTP server

**Validation** (`validate_token()` function):

- Sends GET request to validate endpoint with `Authorization: OAuth {token}` header
- Returns `true` if HTTP response is successful, `false` otherwise

**Crates used:**
- `axum` (0.7) — temporary callback HTTP server and router
- `tokio::sync::oneshot` — receive auth code from callback handler
- `reqwest` — POST for token exchange, GET for validation
- `serde_json` — parse token response

## How it's wired

### Public API

**Module exports** (`mod.rs`):

```rust
pub use eventsub::TwitchClient;
pub use irc::ChatClient;
pub mod auth;
```

### TwitchClient (EventSub)

**Constructor:**

```rust
pub fn new(
    config: TwitchConfig,                          // from application::config
    event_tx: broadcast::Sender<StreamEvent>,      // broadcast to application
    app_event_tx: mpsc::Sender<AppEvent>,          // notifications (status, errors)
) -> Self
```

**Main entry point:**

```rust
pub async fn run(mut self) {
    // Spawned as a tokio task in main.rs:146
    // Runs forever with reconnect loop
}
```

**Flow:**

1. Connects to EventSub WebSocket and waits for welcome (session_id)
2. Subscribes to all 6 event types via Helix API (if broadcaster_user_id is set and URL is default)
3. Listens for `notification` messages, deduplicates by message_id, parses into `StreamEvent::*`
4. Broadcasts events on `event_tx` (which feeds into the presentation/TUI)
5. Sends status/error notifications on `app_event_tx`
6. On WebSocket errors, token revocation, or keepalive timeout, reconnects with exponential backoff
7. On token revocation (401), refreshes token and reconnects
8. On server-initiated reconnect (via `session_reconnect` message), uses the provided URL without re-subscribing

### ChatClient (IRC)

**Constructor:**

```rust
pub fn new(
    channel: String,                    // from TwitchConfig.channel
    login_name: Option<String>,         // from TwitchConfig.channel (if authenticated)
    oauth_token: Option<String>,        // from TwitchConfig.oauth_token (if authenticated)
) -> Self
```

**Main entry point:**

```rust
pub async fn run(self, tx: mpsc::Sender<ChatMessage>, event_tx: mpsc::Sender<AppEvent>) {
    // Spawned as a tokio task in main.rs:207
    // Runs forever with reconnect loop
}
```

**Flow:**

1. If login_name + oauth_token are present, authenticates; otherwise connects anonymously
2. Joins the channel
3. Listens for PRIVMSG server messages and sends `ChatMessage` to `tx`
4. Sends status/error notifications on `event_tx`
5. On disconnect, reconnects with exponential backoff

### Configuration Requirements

**From `application::config::TwitchConfig`:**

| Field | Source | Used by | Required for |
|-------|--------|---------|--------------|
| `client_id` | Config file or `TWITCH_CLIENT_ID` env var | auth, EventSub registration | EventSub, OAuth flow |
| `client_secret` | Config file or `TWITCH_CLIENT_SECRET` env var | auth, token refresh | OAuth flow, token refresh |
| `oauth_token` | Config file or `TWITCH_OAUTH_TOKEN` env var; persisted after refresh | EventSub registration, IRC auth, token validation | EventSub, IRC (if authenticated) |
| `refresh_token` | Config file or `TWITCH_REFRESH_TOKEN` env var; persisted after refresh | token refresh | Automatic token refresh |
| `broadcaster_user_id` | Config file or `TWITCH_BROADCASTER_USER_ID` env var | EventSub subscription conditions | EventSub subscriptions |
| `channel` | Config file or `TWITCH_CHANNEL` env var | IRC join target, chat credentials | IRC client |

### Startup sequence (from `main.rs`)

1. Validate existing token (if client_id + secret are set) → `auth::validate_token()`
2. If invalid, try refresh → `refresh_twitch_token()` (standalone helper in main.rs)
3. If refresh fails, run browser OAuth → `auth::authenticate()`
4. Persist obtained tokens → `application::config::save_twitch_tokens()`
5. If oauth_token + client_id are non-empty, spawn `TwitchClient` → runs EventSub
6. If channel is non-empty, spawn `ChatClient` → runs IRC

### Data flow

```
┌─ Twitch EventSub WebSocket (TwitchClient) ──→ StreamEvent broadcast channel
│                                              ↓
│                                        presentation/TUI consumes
│
└─ Twitch IRC (ChatClient) ──→ ChatMessage mpsc channel → presentation/TUI

Both send AppEvent notifications (status/errors) to a separate mpsc channel
which feeds into the application event log.
```

## Gotchas

**OAuth token validation edge case:** A token can be temporarily revoked by Twitch at any time. The `validate_token()` function performs a synchronous check, but validation is not guaranteed between checks. EventSub subscriptions may be revoked mid-session (revocation message), triggering automatic refresh and reconnect.

**EventSub subscription prerequisites:** Subscriptions are only attempted on a fresh connection (when URL matches default `wss://eventsub.wss.twitch.tv/ws`). Server-initiated reconnects (with different URL) skip re-subscription to preserve existing subscriptions on the server. However, if `broadcaster_user_id` is empty, subscriptions are silently skipped.

**Chat authentication:** IRC can authenticate with the channel name as login. This works because in Twitch, the channel name is the broadcaster's login name. If `oauth_token` is present but `channel` is empty, the chat client connects anonymously and cannot send messages.

**Message deduplication cap:** The seen message ID set is capped at 10,000 entries and cleared on reconnect. This means if a reconnect doesn't happen within 10,000 messages, older duplicates may reappear. In practice, reconnects are rare; this is a memory safeguard.

**Keepalive timeout:** The EventSub client uses a 30-second keepalive timeout, but Twitch defaults to 10-second keepalives. If the connection is very slow or under load, the client might timeout before Twitch sends a keepalive. Twitch will close the connection with a close frame if the timeout expires on their end.

**Token persistence:** Token refresh updates are written to `config.toml` via `save_twitch_tokens()`. This is best-effort; if the disk write fails, the refresh succeeds in memory but is lost on restart. The error is logged but does not fail the refresh.

**Rate limiting:** Twitch's EventSub subscription endpoint has per-app rate limits (not documented in public API, but empirically ~120 requests per minute). If subscriptions fail rapidly, check rate limit headers in error logs. The code does not implement exponential backoff for subscription failures.

**Channel name case sensitivity:** Twitch channel names are case-insensitive for IRC joins and EventSub conditions, but stored as lowercase in the API responses. The config accepts any case, and the code uses it as-is.
