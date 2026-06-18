# Infrastructure — waybar

**Keep in sync:** this file documents `src/infrastructure/waybar`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

Waybar is a minimal status bar for Wayland/X11 Linux environments, favored by systems like Hyprland. It can render custom modules via shell exec commands—waybar calls a script periodically, reads JSON output, and displays formatted text in the bar. This integration adds a bottom bar showing Spotify playback and recent stream events (follows, subscriptions, donations, raids, etc.) with color-coded labels and fading opacity for visual hierarchy.

## What we use from it

**Protocol**: Waybar IPC via shell exec + JSON

- **Config file**: `~/.config/waybar/config.jsonc` (JSON with comments). The module adds a `stream-events` bar object to the bars array.
- **Style file**: `~/.config/waybar/style.css`. The module appends CSS rules scoped to `window#stream-events` for the bottom bar's appearance.
- **Data exchange**: `~/.cache/streams-toolkit/stream_data.json` — a JSON array of events that waybar's custom modules read and format.
- **External trigger**: `omarchy-restart-waybar` CLI command (assumed to be available on the system) to signal waybar to reload configuration and restart.

**External dependencies**:
- `playerctl` — CLI tool to query Spotify metadata (artist, title) via D-Bus. Used by the Python formatter to show currently playing track.
- `python3` — required to run the waybar formatter script (`stream_events.py`).

**Crates used**:
- `serde_json` — for parsing/writing waybar config (JSON) and data file.
- `dirs` — to locate `~/.config` and `~/.cache` directories portably.
- `tokio::process::Command` — to spawn the `omarchy-restart-waybar` process asynchronously.

## How it's wired

### Public API (called by presentation layer)

The module exposes four public async/sync functions:

1. **`enable(output: &str) -> io::Result<()>`**
   - Called when user toggles waybar ON in the TUI.
   - Ensures the seed data file exists (`ensure_data_file()`), installs the Python formatter script (`ensure_scripts()`), merges the stream bar into Omarchy's waybar config, appends CSS styles, and restarts waybar via `omarchy-restart-waybar`.
   - `output` parameter (e.g., `"DP-1"`) specifies which display output the bottom bar appears on; if empty, defaults to `"DP-1"`.

2. **`disable() -> io::Result<()>`**
   - Called when user toggles waybar OFF or on shutdown.
   - Removes the stream bar from Omarchy's waybar config, removes the CSS section, and restarts waybar.

3. **`event_writer(event_rx: broadcast::Receiver<StreamEvent>)`**
   - Spawned as a background task in `main.rs`.
   - Subscribes to the broadcast channel of `StreamEvent`s (emitted by Twitch infra layer when real events arrive).
   - Converts each `StreamEvent` into a `WaybarEvent` (if displayable), inserts it at the front of the recent events list, truncates to `MAX_EVENTS` (15), and writes the list to `stream_data.json`.
   - Handles broadcast lag warnings and gracefully stops when the channel closes.

4. **`ensure_data_file() -> io::Result<()>`**
   - Called on startup from `main.rs`.
   - Creates `~/.cache/streams-toolkit/stream_data.json` with seed sample events if it doesn't exist, so the waybar bar has content to display before real Twitch events arrive.

### Submodules

**`config.rs`** — Manages waybar configuration JSON:
- `add_stream_bar(output: &str)` — Reads `~/.config/waybar/config.jsonc`, strips JSONC comments (handles `//` and `/* */` while respecting quoted strings), parses to JSON, removes any existing stream bar (idempotent), appends the stream bar object, and writes back. Creates a backup `config.jsonc.streams-bak` before modifying.
- `remove_stream_bar()` — Inverse: strips the stream bar from the config array and writes back.
- `stream_bar_object(output: &str)` — Builds the JSON object for the stream bar: sets layer to `top`, position to `bottom`, output display, height 40px, and defines a `custom/stream-events` module with exec command.
- The exec command is a shell snippet: checks if `stream_data.json` exists and is non-empty; if so, runs `python3 stream_events.py <data_path>`; otherwise echoes an empty JSON object with class `empty`.
- Module updates run every 2 seconds (`interval: 2` in the JSON).

**`events.rs`** — Converts domain events to display format:
- `WaybarEvent` struct: holds `event_type` (string key like `"follow"`, `"newSubscriber"`), `username`, and optional `amount`.
- `from_stream_event()` — Maps each `StreamEvent` variant to a `WaybarEvent` with correct type label and amount formatting. `ViewerCountUpdate` events are filtered out (return `None`). Subscription type is classified as `newSubscriber` (single month) or `recurringSubscriber` (multiple months).
- `write_data_file(path, events)` — Serializes the event list to JSON and writes to disk.

**`style.rs`** — Manages CSS styling:
- `add_stream_css()` — Appends (or would append) CSS rules to `~/.config/waybar/style.css`. Currently marked as a no-op (commented out) in the implementation.
- `remove_stream_css()` — Removes the CSS section (bounded by `/* === streams-toolkit: bottom bar === */` and `/* === end streams-toolkit === */` markers) from the style file.
- CSS scopes all stream bar rules under `window#stream-events` to avoid conflicts with Omarchy's top bar.
- Defines styles for the "Recent Events" tag (purple background, uppercase, bold) and event list area (monospace font, 14px size, uppercase).

### Data flow

```
StreamEvent (from Twitch infra)
    ↓ (broadcast channel)
event_writer (task) ← listens
    ├─ Convert via WaybarEvent::from_stream_event()
    ├─ Insert at front of recent events list
    ├─ Truncate to MAX_EVENTS (15)
    └─ write_data_file() to ~/.cache/streams-toolkit/stream_data.json
                ↓
            JSON file at rest
                ↓
    waybar custom/stream-events module (polls every 2 sec)
            ├─ Checks file exists
            └─ Exec: python3 stream_events.py <data_path>
                    ├─ Read JSON data
                    ├─ Fetch Spotify track info via playerctl
                    ├─ Format events with colors, labels, fading opacity
                    └─ Output JSON with 'text', 'tooltip', 'class' for waybar display
                                ↓
                        Rendered in waybar bottom bar
```

### Configuration

The module reads/writes Omarchy's standard waybar paths:
- **Config**: `$XDG_CONFIG_HOME/waybar/config.jsonc` (via `dirs::config_dir()`)
- **Style**: `$XDG_CONFIG_HOME/waybar/style.css`
- **Data**: `$XDG_CACHE_HOME/streams-toolkit/stream_data.json` (via `dirs::cache_dir()`)
- **Script**: `$XDG_CONFIG_HOME/streams-toolkit/scripts/stream_events.py` (installed from embedded source at runtime)

No environment variables are used directly by the Rust code; the application config (from `config.toml`) provides `waybar.output` (the display output string).

### Python formatter (`stream_events.py`)

Embedded in the binary as a const string and written to `~/.config/streams-toolkit/scripts/stream_events.py` on first run.

**Input**: Path to `stream_data.json` (via command-line argument).

**Output**: JSON object with keys:
- `text` — HTML-formatted string for waybar display (uses `<span>` tags for color, size, opacity).
- `tooltip` — Human-readable summary (e.g., "4 recent events").
- `class` — CSS class for styling (either `"empty"` or `"has-events"`).

**Logic**:
- Reads event list from JSON.
- Fetches Spotify track info via `playerctl --player=spotify metadata --format '{{artist}} - {{title}}'` (timeout 1s; silently omitted if unavailable or if Spotify is not playing).
- Keeps the last 4 events and displays them with fading opacity (100%, 75%, 50%, 25% alpha).
- Most recent event is shown larger and bold; earlier events are smaller with a `//` separator.
- Event colors are hardcoded (follow: gray, sub/resub: purple, gift: cyan, donation: orange, cheer: gold, raid: red).
- Amounts are formatted contextually: donations as `R$<amount>`, cheers as `<amount> bits`, subs as `x<amount>`.

## Gotchas

1. **JSONC comment stripping**: The config reader (`config.rs`) implements a bespoke JSONC-to-JSON parser to handle `//` and `/* */` comments while respecting quoted string contents. This is necessary because waybar's config is in JSONC format, but `serde_json` expects pure JSON. The parser is careful with escape sequences, but edge cases (e.g., a literal `/*` inside a quoted string) should be tested if waybar config becomes more complex.

2. **`add_stream_css()` is currently a no-op**: The function reads the style file for error checking but does not actually append CSS (the write logic is commented out). This was likely left incomplete. The `remove_stream_css()` function is fully implemented and works. To enable the feature, uncomment the section marked in `style.rs` lines 99–101.

3. **Waybar restart dependency**: The integration assumes `omarchy-restart-waybar` is available on the system PATH. If missing or renamed, the restart will fail silently (only logged as a warning). The stream bar config/style will be merged but waybar won't pick it up until manually restarted.

4. **Broadcast lag**: If the event writer task falls behind the event stream (e.g., disk I/O is slow, or many events arrive in quick succession), the broadcast channel may be lagged. Lagged events are logged as a warning but discarded; this means the display may miss events briefly. For a stream bar this is acceptable (users see recent events, not a complete history).

5. **Playerctl integration**: The Python formatter silently omits Spotify info if `playerctl` is not installed, not running, or times out (1s timeout). This is by design, so a missing dependency doesn't crash the bar. However, if Spotify integration is critical for your display, ensure `playerctl` is available and the Spotify client is running.

6. **File permissions**: On Unix systems, `ensure_scripts()` sets the Python script to mode `0o755` (executable) after writing. If the filesystem or umask prevents this, the script will be written but may not execute; this will appear as a shell error when waybar tries to run it.

7. **MAX_EVENTS truncation**: The event writer keeps a maximum of 15 events in memory and in the file, but the Python formatter only displays 4. The on-disk list is larger to provide a buffer for the Python script and for debugging. If you need to increase event history, change `MAX_EVENTS` in `mod.rs`.

8. **Seed events**: If `stream_data.json` doesn't exist, it is seeded with 8 sample events when the app starts. These are immediately overwritten as real events arrive. This prevents waybar from showing an "empty" state before the first real event, but it also means the display will change abruptly once Twitch events start flowing.
