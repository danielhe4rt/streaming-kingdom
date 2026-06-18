# Infrastructure — hyprland

**Keep in sync:** this file documents `src/infrastructure/hyprland`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

Hyprland is a dynamic tiling window manager for Wayland. The crate `hyprland` (version `0.4.0-beta.1`) exposes Hyprland's event socket API via both synchronous and asynchronous interfaces. Our integration consumes async event streams to monitor window lifecycle changes (opens, closes, title changes, workspace switches, monitor focus) and to track sensitive windows in real-time. The window manager broadcasts these events to a Unix socket that listeners can subscribe to.

## What we use from it

### Protocols & APIs
- **IPC event socket**: Hyprland exposes an async event listener via the `hyprland::event_listener::AsyncEventListener` API. This is the primary integration point.
- **Window state query**: We also call `hyprland::data::Clients::get_async()` once at privacy monitor startup to seed the current window state, rather than waiting for future events.
- **No authentication**: Hyprland's IPC is socket-based and local-only; no credentials are required.

### Key crate: `hyprland`
The `hyprland` crate provides:
- `AsyncEventListener::new()` – instantiates a new listener.
- `add_window_opened_handler()` – registers a callback for `WindowOpenedData` (containing `window_address` and `window_title`).
- `add_window_closed_handler()` – registers a callback for a window address.
- `add_window_title_changed_handler()` – registers a callback for `WindowTitleChangedData` (containing `address` and `title`).
- `add_workspace_changed_handler()` – registers a callback for `WorkspaceChangedData` (containing `name`).
- `add_active_monitor_changed_handler()` – registers a callback for `ActiveMonitorChangedData` (containing `monitor_name`).
- `add_window_moved_handler()` – registers a callback for `WindowMovedData` (containing `window_address` and `workspace_name`).
- `start_listener_async()` – blocks indefinitely, dispatching incoming socket events to registered handlers.
- `Clients::get_async()` – one-shot async query of all currently open windows and their properties (used to pre-populate state at startup).

### Events this module consumes
1. **Window lifecycle**: opened, closed, title changed.
2. **Workspace changes**: when user switches active workspace.
3. **Monitor focus**: when active monitor (display) changes.
4. **Window moves**: when a window is moved to a different workspace.

### Why we use it
- The event listener (`spawn()` in `event_listener.rs`) feeds window lifecycle events into the app's central event log (`AppEvent`), providing visibility into desktop activity for streaming/recording contexts.
- The privacy monitor (`privacy_monitor.rs`) uses the same listener to detect when sensitive files (matching patterns like `.env`, `credentials`, `.secret`, `.pem`, `id_rsa`) are opened in any window, triggering automatic OBS blur.

## How it's wired

### Public surface

The module exports two key entry points from `mod.rs`:

1. **`pub fn spawn(tx: mpsc::Sender<AppEvent>) -> tokio::task::JoinHandle<()>`** (`event_listener.rs`)
   - Called once at application startup (line 220 of `main.rs`).
   - Spawns a dedicated Tokio task that listens to Hyprland events and forwards them to the TUI event log.
   - Converts Hyprland events to domain `AppEvent` variants: `WindowOpened`, `WindowClosed`, `WindowTitleChanged`, `WorkspaceChanged`, `MonitorFocused`, `WindowMoved`.
   - Returns a `JoinHandle` for graceful shutdown (though not explicitly joined; lifecycle is tied to the app).
   - **No config needed** for this listener; it just connects to the local Hyprland socket.

2. **`pub fn spawn(cmd_rx, status_tx, config, obs_host, obs_port, obs_password, capture_source) -> JoinHandle`** (`privacy_monitor.rs`)
   - Called once at startup (line 160 of `main.rs`) with:
     - `cmd_rx`: receives `PrivacyCommand::Start` / `::Stop` from the TUI to toggle monitoring.
     - `status_tx`: sends `PrivacyStatus` updates back to the TUI (Running, Stopped, BlurEnabled, BlurDisabled, Error).
     - `config: Arc<PrivacyConfig>`: contains `sensitive_patterns: Vec<String>` (from `config.toml`).
     - `obs_host, obs_port, obs_password`: OBS connection details (passed from `ObsConfig`, overridable via env: `OBS_HOST`, `OBS_PORT`, `OBS_PASSWORD`).
     - `capture_source: Arc<str>`: OBS source name to blur (from `config.obs.capture_source`, overridable via `OBS_CAPTURE_SOURCE`).
   - Waits for the first `Start` command before entering monitor mode.
   - Seeds the window state by querying all open windows via `Clients::get_async()`.
   - Spawns an internal Hyprland listener (identical to the one in `event_listener.rs`) that sends window lifecycle events to an internal `mpsc::channel`.
   - Evaluates initial window state and subsequent events; if any window title matches a sensitive pattern (case-insensitive substring match), it calls `obs::enable_blur(client, capture_source)`. When the last sensitive window is closed, it calls `obs::disable_blur()`.
   - Can be stopped and restarted via `PrivacyCommand`.
   - Returns `JoinHandle` for potential graceful shutdown (stored in `_privacy_handle` in `main.rs`).

### Configuration

**From config.toml:**
```toml
[privacy]
sensitive_patterns = [".env", "credentials", ".secret", ".pem", "id_rsa"]

[obs]
host = "localhost"
port = 4455
password = "your-obs-password"
capture_source = "Screen Capture"
```

**Environment variable overrides** (checked in `application::config::ObsConfig::apply_env_overrides()`):
- `OBS_HOST` → overrides `config.obs.host`
- `OBS_PORT` → overrides `config.obs.port` (parsed as u16)
- `OBS_PASSWORD` → overrides `config.obs.password`
- `OBS_CAPTURE_SOURCE` → overrides `config.obs.capture_source`

No env vars exist for `privacy.sensitive_patterns`; they are only read from config.toml.

### Data flow

**Event listener (event_listener.rs):**
```
Hyprland socket
    ↓
AsyncEventListener (hyprland crate)
    ↓
[6 event handlers]
    ↓
mpsc::Sender<AppEvent> (to TUI event log)
```

**Privacy monitor (privacy_monitor.rs):**
```
TUI → PrivacyCommand (Start/Stop) → mpsc::Receiver
                                       ↓
                                    Privacy monitor task
                                       ├─ Seed: Clients::get_async()
                                       ├─ Listen: AsyncEventListener
                                       └─ Compare: is_sensitive(title, patterns)
                                           ├─ Match → obs::enable_blur()
                                           └─ No match → obs::disable_blur()
                                       ↓
                                    PrivacyStatus → mpsc::Sender (to TUI)
```

The privacy monitor maintains a `HashMap<String, String>` (address → title) of all currently open windows. On each window event (opened, closed, title changed), it updates this map and re-evaluates whether any title matches a sensitive pattern. This ensures blur remains active even if the focused window is non-sensitive, as long as *any* open window contains sensitive content.

## Gotchas

### 1. Window title matching is case-insensitive substring search
The function `is_sensitive(title: &str, patterns: &[String]) -> bool` converts both the window title and each pattern to lowercase, then checks if the pattern is a substring of the title. This is permissive (e.g., "env.txt" matches ".env") but by design. Be aware when configuring patterns.

### 2. Privacy monitor waits for Start command before activating
The monitor task does not enter listening mode until it receives the first `PrivacyCommand::Start` from the TUI. If the TUI never sends `Start`, the listener never runs. This is intentional to allow users to defer privacy monitoring.

### 3. OBS connection failure is non-fatal
If `obs::try_connect_obs()` fails (e.g., OBS is offline), the privacy monitor still runs and tracks window state, but blur is not applied. A `PrivacyStatus::Error` message is sent to the TUI, but the monitor continues. When OBS comes online, a manual restart of privacy monitoring is needed; the code does not auto-reconnect.

### 4. Window address format
Window addresses returned by Hyprland are hexadecimal strings (e.g., "0x123abc"). The code stores them as `String` and uses them as HashMap keys. No parsing or normalization is done.

### 5. Initial window seeding may miss rapid opens
At startup, `Clients::get_async()` is called once to query all open windows. If a sensitive window is opened *after* the privacy monitor starts but *before* it enters the listening loop, that window may not be tracked. However, once the listener is active, all subsequent opens/closes/title changes are caught.

### 6. Workspace and monitor events are informational only
The event listener forwards `WorkspaceChanged` and `MonitorFocused` events to the TUI event log for visibility, but the privacy monitor does not use them; it only cares about individual window titles regardless of which workspace/monitor they occupy.

### 7. Title changes in running editors
If a user has a sensitive file open in an editor (e.g., `nvim .env`) and the editor displays the filename in the window title, the pattern match will trigger blur. When the file is closed and the title changes to something like `nvim` (with no filename), blur is disabled. This is the intended behavior.
