# Infrastructure — obs

> **Keep in sync:** this file documents `src/infrastructure/obs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

OBS (Open Broadcaster Software) is a desktop application for live streaming and recording. We integrate with it via its **WebSocket API** (obws crate v0.14.0) to control blur filters on video sources, enabling automatic privacy protection when sensitive windows are detected on the streamer's desktop.

## What we use from it

**Protocol & Crate:**
- WebSocket-based OBS WebSocket API (via `obws` crate v0.14.0)
- Async Rust client that connects over TCP to OBS's WebSocket server

**Endpoints & Operations:**
- **Connection:** `obws::Client::connect(host, port, password?)` — establishes a WebSocket connection to OBS
- **Get filter:** `client.filters().get(source, filter_name)` — checks if a blur filter exists on a source
- **Create filter:** `client.filters().create(request)` — adds a new blur filter ("Composite Blur") with specific settings
- **Enable/disable filter:** `client.filters().set_enabled(request)` — toggles blur filter on/off without deleting it

**Filter Details:**
- Filter name: `"Composite Blur"`
- Filter kind: `"obs_composite_blur"` (the internal OBS filter type)
- Configuration: Blur region size (240×240 px) and speed (0,0 — static blur, no movement)

## How it's wired

**Public surface — three async functions in `src/infrastructure/obs/mod.rs`:**

1. **`try_connect_obs(host: &str, port: u16, password: &str) -> Option<obws::Client>`**
   - Initiates connection to OBS; returns `None` (with warning log) if OBS is unreachable
   - Non-fatal: the app continues without blur if OBS is offline
   - Called once during privacy monitor initialization

2. **`enable_blur(client: &obws::Client, capture_source: &str)`**
   - Ensures the blur filter exists and is enabled on the named source
   - If filter exists but is disabled, re-enables it
   - If filter doesn't exist, creates it with default settings
   - Logs errors but doesn't panic; graceful degradation if OBS API fails

3. **`disable_blur(client: &obws::Client, capture_source: &str)`**
   - Disables (but does not delete) the blur filter
   - Allows quick re-enablement without recreation
   - Logs at debug level; failure is non-critical

**Caller: Privacy Monitor**
The `infrastructure::hyprland::privacy_monitor` module spawns a tokio task that:
- Monitors window open/close/title-change events from the Hyprland window manager
- Calls `enable_blur()` when a sensitive window (matching configured patterns like `.env`, `credentials`, etc.) appears
- Calls `disable_blur()` when no sensitive windows are open
- Passes connection details to the monitor task during spawn

**Configuration & Environment Variables:**
From `src/application/config.rs`, the `ObsConfig` struct holds:
- `host`: Default `"localhost"`; override via `OBS_HOST` env var
- `port`: Default `4455`; override via `OBS_PORT` env var (parsed as u16)
- `password`: Default empty string; override via `OBS_PASSWORD` env var
- `capture_source`: Default `"Screen Capture"`; override via `OBS_CAPTURE_SOURCE` env var

All four are read from `~/.config/streams-toolkit/config.toml` and can be overridden by environment variables at startup.

**Data flow:**
```
[Application Layer]
  └─ privacy_monitor::spawn() receives obs_host, obs_port, obs_password, capture_source
     ↓
[Infrastructure OBS Layer]
  ├─ try_connect_obs() → obws::Client (or None)
  ├─ enable_blur(client, source) → obws API calls (get, create, set_enabled)
  └─ disable_blur(client, source) → obws API call (set_enabled)
     ↓
[OBS Desktop App]
  └─ WebSocket port 4455 (default)
```

## Gotchas

**Connection is optional:**
- If OBS is not running or the password is wrong, `try_connect_obs()` logs a warning and returns `None`
- The privacy monitor still functions but blur will not be applied
- This is intentional—the tool should not crash or block if OBS is unavailable

**Filter state management:**
- The code **does not delete** the blur filter, only enables/disables it
- This preserves filter configuration (blur size, speed) between toggles
- If the filter is manually deleted in OBS while the monitor is running, the next enable call will recreate it

**Password handling:**
- An empty password string is treated as "no password" and passed as `None` to obws
- OBS's default setup has no password; production setups should use `OBS_PASSWORD` env var

**WebSocket connection lifecycle:**
- The `obws::Client` is established once at monitor startup
- If the connection drops (e.g., OBS crashes), subsequent filter calls will fail with a logged error
- The monitor does not attempt to reconnect; a full restart of the privacy monitor is needed
- Future enhancement: add reconnection logic with exponential backoff
