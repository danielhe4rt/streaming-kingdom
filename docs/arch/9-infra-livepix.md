# Infrastructure — livepix

**Keep in sync:** this file documents `src/infrastructure/livepix`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

Livepix (https://livepix.gg) is a donation/tipping service for live streamers. It provides webhook events when viewers send donations or messages with financial transactions. Our integration receives these webhooks, enriches them via API calls, converts them to `StreamEvent::Donation` events, and optionally pipes donation messages through text-to-speech playback.

## What we use from it

**OAuth2 (client_credentials flow)**
- Endpoint: `https://oauth.livepix.gg/oauth2/token`
- Grant type: `client_credentials`
- Scopes requested: `account:read wallet:read webhooks messages:read`
- Credentials: `LIVEPIX_CLIENT_ID`, `LIVEPIX_CLIENT_SECRET` (from config, fallback to env vars)
- Purpose: authenticate server-to-server requests to fetch donation message details

**Messages API (v2)**
- Endpoint: `https://api.livepix.gg/v2/messages/{message_id}`
- Authentication: bearer token (from OAuth response)
- Purpose: fetch full donation details (username, amount, message text, currency) given a message ID from webhook

**Webhook events**
- Livepix sends HTTP POST to `http://127.0.0.1:{webhook_port}/webhooks` when a donation is received
- Payload contains `event` (string) and `resource` object with `type` ("message") and `id` (message_id)
- We filter for `event == "new"` and `resource.type == "message"`

**HTTP framework:** Axum 0.7 (async web server), `reqwest` for HTTP client calls, `serde_json` for payload parsing

## How it's wired

**Public entry point:** `pub fn spawn()`
- Located in `src/infrastructure/livepix/webhook/` (a folder module sliced by concern; `spawn` is re-exported from `webhook/server.rs`, so its public path is unchanged)
- Signature: spawns a long-running tokio task that manages the Livepix webhook server lifecycle

**Module layout** (`src/infrastructure/livepix/webhook/`):

| File | Concern |
|------|---------|
| `webhook/mod.rs` | Module wiring + `pub use server::spawn` |
| `webhook/payload.rs` | Webhook callback + API response deserialization types |
| `webhook/server_state.rs` | Shared `ServerState` threaded through the handlers |
| `webhook/oauth.rs` | `client_credentials` OAuth flow + token caching (`get_oauth_token`, `resolve_token`) |
| `webhook/message_api.rs` | Fetching full donation detail from the REST API (`fetch_message`) |
| `webhook/handlers.rs` | Axum route handlers (`health_check`, `handle_webhook`, donation fan-out) |
| `webhook/server.rs` | The `spawn` entry-point + per-session `run_server` lifecycle |
- Parameters:
  - `cmd_rx: mpsc::Receiver<LivepixCommand>` — receives `Start` / `Stop` commands from the TUI
  - `status_tx: mpsc::Sender<LivepixStatus>` — sends status updates back to the TUI (running port, OAuth success/failure, webhook received, errors)
  - `event_tx: broadcast::Sender<StreamEvent>` — broadcasts `StreamEvent::Donation` to all subscribers (event log, alerts, etc.)
  - `tts_tx: Option<mpsc::Sender<TtsRequest>>` — optional channel to forward donation messages to the TTS worker for audio playback
  - `config: LivepixConfig` — contains `client_id`, `client_secret`, `webhook_port`, and `tts` (ElevenLabs config)

**Exported types from `mod.rs`:**
- `LivepixCommand` enum: `Start`, `Stop`
- `LivepixStatus` enum: `Running { port }`, `Stopped`, `OAuthSuccess`, `OAuthError(String)`, `WebhookReceived { username, amount }`, `Error(String)`

**Configuration (from `src/application/config.rs`)**
```rust
pub struct LivepixConfig {
    pub client_id: String,
    pub client_secret: String,
    pub webhook_port: u16,  // default: 8000
    pub tts: TtsConfig,
}
```

Env var overrides:
- `LIVEPIX_CLIENT_ID` — overrides `config.toml` if set
- `LIVEPIX_CLIENT_SECRET` — overrides `config.toml` if set
- `ELEVENLABS_API_KEY` — overrides TTS config if set

**Data flow in/out:**

1. **Initialization** (in `main.rs`):
   - Load `LivepixConfig` from `~/.config/streams-toolkit/config.toml`, apply env overrides
   - Create mpsc channels for commands, status, and events
   - Spawn the livepix task via `infrastructure::livepix::spawn()`

2. **Server startup** (when `LivepixCommand::Start` received):
   - Authenticate via OAuth (POST to `https://oauth.livepix.gg/oauth2/token`)
   - Cache the access token in Arc<Mutex<Option<String>>>
   - Bind Axum server to `127.0.0.1:{webhook_port}`
   - Send `LivepixStatus::Running { port }` to TUI

3. **Webhook handling** (HTTP POST to `/webhooks`):
   - Parse JSON payload into `WebhookPayload`
   - Filter for `event == "new"` and `resource_type == "message"`
   - Spawn async task to fetch full message details via API (non-blocking HTTP response)
   - Call `fetch_message(state, message_id)` with cached/refreshed token
   - Extract `username`, `amount` (in cents), `message` text, `currency`

4. **Event broadcast** (when message fetched successfully):
   - Send `StreamEvent::Donation { username, amount_cents, message }` to broadcast channel
   - Send `LivepixStatus::WebhookReceived { username, amount }` to TUI
   - Trigger desktop notification via `notify-send` (if available)
   - Forward donation message to TTS queue via `tts_tx.try_send()` (non-blocking; drops if queue full)

5. **Server shutdown** (when `LivepixCommand::Stop` received):
   - Trigger graceful shutdown via oneshot channel
   - Await Axum server task completion
   - Send `LivepixStatus::Stopped` to TUI

## Gotchas

**Token caching & refresh:**
- OAuth tokens are cached in Arc<Mutex<Option<String>>> after initial authentication
- If `resolve_token()` finds cache empty, it re-authenticates automatically (logs warning)
- No explicit token expiration handling; depends on Livepix token TTL

**Webhook async processing:**
- HTTP response is sent immediately (StatusCode::OK) before fetching message details
- Message fetch happens in a spawned task; errors are logged but don't fail the webhook response
- If API call fails, `LivepixStatus::Error` is sent to TUI, but webhook is already acknowledged

**Non-blocking channels:**
- TTS queue uses `try_send()`, not `send()` — if queue is full (limit 8), donation message is silently dropped with a warning log
- Status updates use `await` but errors are ignored; assumes status channel won't block

**Local binding only:**
- Server binds to `127.0.0.1:{webhook_port}`, not `0.0.0.0`
- Livepix webhooks must be configured to route to this localhost address via port forwarding, reverse proxy, or ngrok (not done by this code)

**Desktop notifications:**
- Uses `notify-send` command with hardcoded category `stream.donate` and 12-second expiry
- If `notify-send` is not available on the system, the error is silently ignored
