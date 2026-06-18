# Context: Infrastructure

**Layer:** `src/infrastructure/` · **Triage label:** `area:infrastructure`

The infrastructure layer adapts external services (Twitch, ElevenLabs, Livepix, Hyprland, OBS, Waybar) into the application's event and command channels. Each adapter handles connection lifecycle, authentication, real-time message delivery, and graceful degradation when external services become unavailable.

Each submodule has its own `infra:*` triage label:

- `twitch/` (`infra:twitch`) — IRC, EventSub, auth
- `elevenlabs/` (`infra:elevenlabs`) — text-to-speech
- `hyprland/` (`infra:hyprland`) — event listener, privacy monitor
- `livepix/` (`infra:livepix`) — donation webhooks
- `obs/` (`infra:obs`) — OBS control
- `waybar/` (`infra:waybar`) — config, events, style

## Glossary

### Twitch EventSub & Chat

**EventSub**:
Twitch's WebSocket-based subscription API that delivers real-time channel events (follows, subscriptions, cheers, raids, etc.) to a registered session.
_Avoid_: event subscription, Twitch events

**TwitchClient**:
A long-running async task that connects to the EventSub WebSocket, registers event subscriptions, listens for notifications, parses them into StreamEvent, and broadcasts to the application with automatic reconnection on failure.

**ChatClient**:
A long-running async task that connects to Twitch IRC, optionally authenticates with channel credentials, joins a channel, and forwards chat messages to the application with automatic reconnection on failure.

**session_id**:
A unique WebSocket session identifier issued by Twitch in the welcome message, required to register EventSub subscriptions for that connection.

**message deduplication**:
A mechanism that tracks seen EventSub notification message IDs in a HashSet to filter duplicate messages; clears on reconnect and caps at 10,000 entries to prevent unbounded memory growth.

**keepalive timeout**:
A 30-second deadline for receiving a keep-alive or data message from the EventSub WebSocket; exceeding it triggers a reconnection because Twitch may have abandoned the connection.

**token revocation**:
A state where Twitch explicitly denies further use of an OAuth token, signaled via an EventSub revocation message with status 'authorization_revoked', requiring automatic token refresh and reconnection.

**server-initiated reconnect**:
An EventSub reconnect directive sent by Twitch via a session_reconnect message, containing a new WebSocket URL that preserves the session's subscriptions without requiring re-registration.

**Helix**:
Twitch's REST API used by the EventSub module to register event subscriptions on a given session via POST to the subscriptions endpoint.

**OAuth token**:
A Twitch-issued bearer token used to authenticate requests to the EventSub subscription endpoint and IRC chat; obtained via the browser OAuth 2.0 flow and refreshable using a refresh token.

**StreamEvent**:
A domain type representing a single channel event (Follow, Sub, GiftSub, Cheer, Raid) emitted by the TwitchClient and broadcast to the application layer.

### ElevenLabs Text-to-Speech

**TTS Worker**:
A long-running async task that receives TtsRequest messages from a bounded channel, calls the ElevenLabs API, caches the resulting MP3 audio, and plays it via a local audio player.

**TtsRequest**:
A message struct containing the text to synthesize and the username of the donor, sent through an MPSC channel to the TTS worker.

**Voice ID**:
A unique ElevenLabs identifier for a specific pre-trained voice model, used in the API endpoint URL to select which voice synthesizes the text.

**Voice Settings**:
A JSON object (stability, similarity_boost, speed) sent with each ElevenLabs synthesis request to control the acoustic characteristics of the output audio.

**Cache Directory**:
The local filesystem path `~/.cache/streams-toolkit/tts/` where ElevenLabs MP3 responses are written before playback and deleted after.

**Graceful Disable**:
The behavior where TTS is silently disabled at startup if the `ELEVENLABS_API_KEY` environment variable or config value is empty, allowing the application to run without errors.

**Blocking Playback**:
The synchronous subprocess call to `mpv` or `ffplay` that causes the TTS Worker to pause and wait until audio playback finishes before processing the next request.

### Livepix Donations

**Donation**:
A financial transaction initiated by a stream viewer through Livepix, containing a username, amount in cents, optional message text, and currency identifier.

**Webhook**:
An HTTP POST callback from Livepix to our server (127.0.0.1:webhook_port/webhooks) that notifies us of new donation events with minimal payload; the full details are fetched via subsequent API call.

**Message enrichment**:
The process of fetching full donation details (username, amount, message, currency) from the Livepix API using a message_id received in a webhook payload.

**OAuth token cache**:
An Arc<Mutex<Option<String>>> that holds the current Livepix API access token across multiple webhook requests to avoid re-authenticating on every API call.

**StreamEvent::Donation**:
A domain event variant that represents a successfully processed donation, containing username, amount_cents, and message; broadcast to all subscribers (event log, alerts, TTS).

**client_credentials flow**:
OAuth2 authentication pattern where the server authenticates to Livepix using client_id and client_secret directly (not user-delegated); used for server-to-server API access.

**LivepixCommand**:
An enum (Start, Stop) that the TUI sends via channel to control the webhook server lifecycle.

**LivepixStatus**:
An enum representing state changes (Running, Stopped, OAuthSuccess, OAuthError, WebhookReceived, Error) sent from the webhook server back to the TUI.

### Hyprland Window Manager Integration

**Window Address**:
A hexadecimal string identifier assigned by Hyprland to each open window, used as a unique key to track windows across lifecycle events.

**Sensitive Pattern**:
A configurable case-insensitive substring matched against window titles to detect whether a window contains sensitive information requiring blur.

**Privacy Monitor**:
A background task that listens to window lifecycle events, evaluates titles against sensitive patterns, and controls OBS blur activation.

**Async Event Listener**:
A long-running Hyprland IPC subscriber that receives real-time window and workspace events from the window manager and dispatches them to registered handlers.

**Blur State**:
A boolean flag tracking whether OBS source blur is currently active; toggled by the privacy monitor when sensitive windows are opened or closed.

**OBS Capture Source**:
A named source within OBS Studio (e.g., 'Screen Capture') that is configured for blur control via the hyprland privacy monitor.

### OBS WebSocket Integration

**Blur filter**:
An OBS video effect applied to a video source that obscures a rectangular region (240×240 px) of the frame.

**Capture source**:
A named video input in OBS (e.g. 'Screen Capture') to which filters can be applied.

**Non-fatal degradation**:
A feature that silently disables (does not crash the app) when its external dependency becomes unavailable.

**WebSocket connection**:
A persistent TCP connection to OBS's API server (default port 4455) over which filter control commands are sent.

### Waybar Status Bar Integration

**stream_data.json**:
JSON file at `~/.cache/streams-toolkit/stream_data.json` containing a list of up to 15 recent stream events that waybar polls and formats for display.

**WaybarEvent**:
A displayable event struct with event_type, username, and optional amount, converted from a domain StreamEvent for serialization to the data file.

**event_writer**:
Background task that listens on a broadcast channel for StreamEvents, converts them to WaybarEvents, maintains the recent events list in memory, and persists it to stream_data.json.

**stream bar**:
A Waybar bar configuration (bottom-positioned, named 'stream-events') that renders recent stream events and Spotify track info as a custom module.

**stream_events.py**:
Python 3 formatter script embedded in the binary and installed to ~/.config/streams-toolkit/scripts/, invoked by waybar every 2 seconds to read stream_data.json and output JSON with formatted HTML text for display.

**JSONC parsing**:
The process of stripping single-line (//) and multi-line (/* */) comments from waybar's config.jsonc file while preserving quoted string content, then parsing as JSON.

**omarchy-restart-waybar**:
System CLI command invoked to signal waybar to reload configuration and restart the bars after config or style changes.

**playerctl**:
External CLI tool queried to fetch current Spotify track metadata (artist and title) via D-Bus, used by the Python formatter for the Spotify info header.

**MAX_EVENTS**:
Constant (value 15) limiting the number of recent events stored in memory and in stream_data.json by the event writer.

**opacity fading**:
Visual effect in the waybar display where the 4 most recent events are shown with progressively lower alpha values (100%, 75%, 50%, 25%) to indicate recency and age.

## Relationships

- A **TwitchClient** opens a WebSocket connection to **EventSub** and receives a **session_id** in the welcome message.
- The **TwitchClient** uses the **session_id** to register subscriptions via the **Helix** API, requesting multiple event types (follow, subscribe, cheer, raid, etc.).
- Each **EventSub** notification is deduplicated using **message deduplication** and parsed into a **StreamEvent**, which is broadcast to the application.
- A **token revocation** event triggers automatic **OAuth token** refresh, after which the **TwitchClient** reconnects to establish a new **session_id**.
- A **server-initiated reconnect** message provides a new WebSocket URL that preserves the current session's subscriptions without re-registering via **Helix**.
- The **ChatClient** optionally authenticates using an **OAuth token** tied to the channel name, then joins and listens for IRC PRIVMSG messages.
- Both **TwitchClient** and **ChatClient** implement the same reconnection pattern: backoff delays up to 120 seconds, triggered by disconnects or **keepalive timeout** expiry.
- A **TtsRequest** is sent to a **TTS Worker** via an MPSC channel.
- A **TTS Worker** uses a **Voice ID** and **Voice Settings** to construct an **ElevenLabs** API request.
- **ElevenLabs** MP3 responses are stored in the **Cache Directory** before **Blocking Playback** via `mpv` or `ffplay`.
- The **TTS Worker** implements **Graceful Disable** by returning `None` if no API key is configured.
- A **Webhook** payload from **Livepix** triggers **Message enrichment**, which fetches the full **Donation** details.
- A **Donation** becomes a **StreamEvent::Donation** which is broadcast to all subscribers.
- A **StreamEvent::Donation** can optionally produce a **TtsRequest** for the **ElevenLabs** worker.
- The **OAuth token cache** is refreshed via **client_credentials flow** when empty.
- **LivepixCommand** (Start/Stop) controls the lifecycle that produces **LivepixStatus** updates.
- A **Privacy Monitor** contains one **Async Event Listener** and maintains a map of **Window Address** → title.
- A **Sensitive Pattern** matches substrings in window titles to determine if a **Blur State** transition is needed.
- An **OBS Capture Source** receives blur control commands from the **Privacy Monitor** when **Blur State** changes.
- A **Privacy Monitor** manages one **WebSocket connection** to OBS.
- A **Privacy Monitor** controls one **Blur filter** per **Capture source**.
- A **Blur filter** toggles between enabled (active blur) and disabled (inactive blur) states via **WebSocket connection**.
- An **event_writer** task consumes **StreamEvent**s and produces **WaybarEvent**s.
- Each **WaybarEvent** is written to **stream_data.json** as part of a list capped at **MAX_EVENTS**.
- The **stream_events.py** script reads **stream_data.json** and formats events with **opacity fading**.
- The **stream bar** is a Waybar configuration object that executes **stream_events.py** every 2 seconds.
- The **stream bar** is merged into Omarchy's `config.jsonc` via **JSONC parsing**.
- **omarchy-restart-waybar** is invoked after **stream bar** config or style changes to apply them.

## Example dialogue

**Dev:** The Twitch token expired, but we're still receiving stream events. Did the TwitchClient recover?

**Domain Expert:** When EventSub sends a token revocation message, TwitchClient detects the authorization_revoked signal and calls refresh_token, which updates the in-memory config and persists it to config.toml. Then it reconnects from scratch to get a new session_id. The seen_ids HashSet is cleared on every reconnect because those message IDs are stale and won't repeat. The set caps at 10,000 to avoid unbounded growth during long sessions.

**Dev:** What about Livepix donations? If I send multiple donations in a row, are they all processed?

**Domain Expert:** Each Webhook from Livepix triggers a Message enrichment call that fetches the full Donation details. A StreamEvent::Donation is broadcast to all subscribers. If TTS is configured, we forward a TtsRequest to the ElevenLabs worker. The TTS Worker has Blocking Playback — it waits for mpv to finish before processing the next TtsRequest. The queue capacity is 8, so if you send more than 8 donations in rapid succession, older ones are dropped.

**Dev:** If I open a .env file in my editor during stream, what happens?

**Domain Expert:** The Privacy Monitor detects the Sensitive Pattern '.env' in the window title via the Async Event Listener. It toggles the Blur State and tells OBS to enable the Blur filter on the Screen Capture source via a WebSocket connection. If OBS is unavailable, it's Non-fatal degradation — we log a warning but the stream continues. The Blur filter settings are preserved between toggles.

**Dev:** The donation message appears in the status bar, but it's already fading out. Can I keep more events visible?

**Domain Expert:** The event_writer maintains MAX_EVENTS (15) in stream_data.json, but the stream_events.py formatter only displays the 4 most recent with opacity fading — 100%, 75%, 50%, 25%. Change MAX_EVENTS in mod.rs to keep more history, and update the Python formatter's display logic to show them. Waybar polls stream_data.json every 2 seconds via the stream bar.

## Ambiguities

- **'token'** vs **'OAuth token'**: Throughout code and config, 'token' usually means the OAuth access token specifically, not the refresh token. Use **'OAuth token'** for clarity when the distinction matters.

- **'session'** vs **'session_id'**: Code uses 'session' to mean the EventSub WebSocket session (e.g., 'session_reconnect' message), and 'session_id' to mean the unique ID field within that session. Distinguish **session_id** as the concrete identifier.

- **'reconnect'** (WebSocket) vs **'reconnect'** (HTTP POST for token refresh): EventSub reconnection is a WebSocket close + new connection. Token refresh is a separate HTTP POST. Both are handled by the code but they are orthogonal mechanisms.

- **'channel.follow'** (EventSub subscription type) vs **'channel'** (IRC channel name): Both use the word 'channel' but refer to different concepts. EventSub subscriptions are registered per broadcaster; IRC channels are joined by name. These are API details, not domain concepts; disambiguate in code comments.

- **'source'** in OBS context: Can mean either a video source in OBS (what we call **Capture source** here) or Rust module source code. In infrastructure domain docs, always read 'source' as **Capture source** unless in a code/git context.

- **'TTS'** vs **'TTS Worker'**: 'TTS' in comments can mean the **ElevenLabs** service itself or the local **TTS Worker** task. Use **'TTS'** for the external service, **'TTS Worker'** for our async task handling synthesis + playback.

- **'stream bar'** vs **'bottom bar'**: **stream bar** is the waybar bar object named 'stream-events' positioned at the bottom. 'Bottom bar' is a positional description. Use **stream bar** when referring to the configuration feature.

- **'events'** vs **'stream events'**: Broader codebase uses 'events' generically (app events, privacy events, etc.). **'Stream events'** specifically means events from Twitch stream (follows, subs, donations, etc.). In this module, assume 'events' means stream events unless otherwise qualified.

## Decisions

See `docs/adr/` (system-wide) and `src/infrastructure/docs/adr/` (this context, when present).
