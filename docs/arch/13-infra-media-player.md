# Infrastructure — media_player

**Keep in sync:** this file documents `src/infrastructure/media_player`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

There is no networked "service" here — the source is the local **MPRIS** media-player bus exposed over D-Bus, read through the `playerctl` CLI. `playerctl` is already a project dependency (the Waybar formatter uses it), so this module adds no new crate. v1 targets **Spotify** specifically via `-p spotify`; broadening to other players is a one-flag change.

The design rationale (why a subprocess instead of native `zbus`, and why a `watch` channel instead of `broadcast`) is recorded in `src/infrastructure/docs/adr/0001-now-playing-observed-state.md`. Now Playing is **ambient state**, not an event: at most one current track, latest-value-wins, never replayed as history, and deliberately **never** wrapped in `AppEvent`.

## What we use from it

### Protocols & APIs
- **`playerctl --follow`**: we spawn `playerctl --follow -p spotify metadata --format '…'` once and read its stdout line by line. `--follow` emits a line on every track/status change, so the observer is event-driven — no polling loop and no per-tick subprocess.
- **No authentication**: MPRIS is a local D-Bus bus; no credentials are required.

### The command
```
playerctl --follow -p spotify metadata --format "{{status}}|{{title}}|{{artist}}|{{album}}|{{mpris:artUrl}}"
```
- stdout is **piped** and read with `tokio::io::BufReader` + `AsyncBufReadExt::lines()`.
- stderr is **inherited** so `playerctl`'s own diagnostics stay visible in the host logs.
- The `--format` template field order is fixed (`status|title|artist|album|artUrl`) and parsed positionally by `parse_line`. Keep the constant and the parser in sync.

### Why we use it
- Drives the Coworking Overlay **Now Playing** widget with the real current Spotify track, delivered via the Overlay Feed (`FeedEvent` kind `nowPlaying`).
- Feeds the TUI **Spotify Service** row, which reads the same `watch` receiver.

## How it's wired

### Public surface

```rust
pub fn spawn(
    now_playing_tx: tokio::sync::watch::Sender<Option<crate::domain::NowPlaying>>,
) -> tokio::task::JoinHandle<()>
```

Called once at application startup. The sender is the one held by `application::AppState` (constructed via `watch::channel(None)`); the matching receiver is handed to the Overlay Feed (`OverlayState.now_playing`) and the TUI.

### Semantics
- While the parsed status is **Playing** or **Paused**, it publishes `Some(NowPlaying)` carrying that status.
- When the status is **Stopped**, or `playerctl` is missing / exits / has no player, it publishes `None` so consumers fall back to their placeholder.
- **Dedupe**: it only sends when the value differs from the last sent value (`send_if_changed`), so consumers do not re-render on duplicate frames.

### Parsing
`fn parse_line(line: &str) -> Option<NowPlaying>` is a pure function (unit-tested under `#[cfg(test)]`):
- Splits on `|` with `splitn(5, '|')` so a literal `|` inside a title/album never eats later fields; the trailing `artUrl` keeps any `|` it contains.
- Status string maps `"Playing"` → `Playing`, `"Paused"` → `Paused`, anything else → `Stopped`.
- An empty `artUrl` (after trimming) becomes `None`, else `Some(trimmed)`.
- Returns `None` for an empty or malformed line (fewer than 5 fields). A malformed line is skipped — it does **not** clear the current track.

### Data flow
```
playerctl --follow -p spotify metadata (subprocess)
    ↓ stdout lines
BufReader::lines()
    ↓
parse_line()  →  Option<NowPlaying>
    ↓ (Stopped → None)  send_if_changed (dedupe)
watch::Sender<Option<NowPlaying>>  (held by AppState)
    ├─ Overlay Feed  →  FeedEvent::now_playing / now_playing_cleared  →  SSE
    └─ TUI           →  Spotify Service row
```

## Gotchas

### 1. Graceful degradation is the contract
If `playerctl` is not installed, fails to spawn, exits, or its stdout closes (Spotify quit), the observer logs at `debug`/`info`, publishes `None`, sleeps a ~3s backoff (`RETRY_BACKOFF`), and respawns. It must **never panic** — a missing player is a normal state, not an error.

### 2. A malformed line does not clear the track
Parse failures yield `None` from `parse_line` and are simply skipped. We only publish `None` on a genuine Stopped status or when the follow stream ends — not on a transient parse glitch.

### 3. `watch` is latest-value, not history
A freshly-opened SSE connection (or a newly-attached TUI) immediately sees the current track because `watch` hands the latest value to new receivers. This is precisely why now-playing is a `watch` and not a `broadcast` — see the ADR.

### 4. v1 is Spotify-only
The `-p spotify` flag scopes the observer to Spotify. Other MPRIS players are ignored. Removing/changing that flag broadens the source but may surface multiple players.

### 5. Brittle to `playerctl` format changes
The positional `--format` parsing is brittle to `playerctl` output/`--follow` behaviour changes. This is mitigated by the isolated, unit-tested `parse_line` and the graceful `None` fallback.
