# Domain — media player (now playing)

**Keep in sync:** this file documents `src/domain/media_player.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

The media-player module defines the domain types representing the **currently playing track** observed from the local media player (Spotify, via MPRIS / `playerctl`). `NowPlaying` is the value object describing the track; `PlaybackStatus` classifies whether it is playing, paused, or stopped.

Crucially, `NowPlaying` is **observed STATE**, not a domain event. Every other live signal in the toolkit (`StreamEvent`, `ChatMessage`) is a discrete occurrence carried on a `broadcast` channel and logged into the `AppEvent` history. Now-playing is different: there is at most one current track, latest-value-wins, and it is never replayed as history. It rides a `tokio::sync::watch::Sender<Option<NowPlaying>>` and is **deliberately never wrapped in `AppEvent`**. This distinction (state vs. events) is recorded in `src/infrastructure/docs/adr/0001-now-playing-observed-state.md`.

## Types created

### NowPlaying (struct)

The value object describing the current track. Produced by:

- **Media player observer** (`src/infrastructure/media_player/`): spawns a `playerctl --follow -p spotify` subprocess, parses each stdout line into a `NowPlaying`, deduplicates, and publishes only on change via the `watch` sender. When the player is stopped, missing, or exits, it publishes `None` so consumers fall back to their placeholder.

```rust
NowPlaying {
    title: String,
    artist: String,
    album: String,
    art_url: Option<String>,
    status: PlaybackStatus,
}
```

- `title` / `artist` / `album`: track metadata from MPRIS.
- `art_url`: the `mpris:artUrl` (real album cover) when available. `None` when the player exposes no art; consumers fall back to a placeholder cover.
- `status`: the current `PlaybackStatus`.

### PlaybackStatus (enum)

Classification of the player's playback state, mapped from the `playerctl` status string.

#### Status variants
- `Playing`: a track is actively playing (overlay animation runs).
- `Paused`: a track is loaded but paused (rendered, animation reflects paused state).
- `Stopped`: no track playing. Consumers treat this like "no player" and fall back to the placeholder.

## Domain interactions (use-case flows)

### Flow 1: playerctl track change → watch publish → Overlay Feed & TUI

```
 PLAYERCTL --follow          MEDIA PLAYER OBSERVER        APP STATE (watch)     CONSUMERS
  │                                  │                        │                    │
  │  stdout line (new track)         │                        │                    │
  │ ───────────────────────────────► │                        │                    │
  │                                  │  parse line            │                    │
  │                                  │  → NowPlaying {        │                    │
  │                                  │      status: Playing } │                    │
  │                                  │                        │                    │
  │                                  │  dedupe vs last        │                    │
  │                                  │  send Some(NowPlaying) │                    │
  │                                  │ ─────────────────────► │                    │
  │                                  │                        │  watch latest-value│
  │                                  │                        │ ──────────────────►│ Overlay Feed
  │                                  │                        │                    │ FeedEvent::NowPlaying
  │                                  │                        │ ──────────────────►│ TUI Spotify row
```

**What gets created at each step:**

1. **playerctl emits**: a stdout line with the new track's title/artist/album/art/status.
2. **Observer parses**: `NowPlaying { title, artist, album, art_url, status: Playing }`.
3. **Observer dedupes & publishes**: sends `Some(NowPlaying)` on the `watch::Sender` only when the value differs from the last sent.
4. **Overlay Feed**: reads the `watch` receiver, emits a `FeedEvent::NowPlaying` frame (a freshly-opened SSE connection immediately receives the current track).
5. **TUI**: reads the same `watch` receiver each tick for its Spotify Service row.

---

### Flow 2: player stops / playerctl absent → None → placeholder fallback

```
 PLAYERCTL                   MEDIA PLAYER OBSERVER        APP STATE (watch)     CONSUMERS
  │                                  │                        │                    │
  │  status: Stopped / exit / absent │                        │                    │
  │ ───────────────────────────────► │                        │                    │
  │                                  │  send None             │                    │
  │                                  │ ─────────────────────► │                    │
  │                                  │                        │ ──────────────────►│ placeholder
```

**What gets created:**

1. **playerctl reports** `Stopped`, exits (no player running), or is missing entirely.
2. **Observer publishes** `None` on the `watch::Sender`.
3. **Consumers** (overlay, TUI) fall back to their placeholder Now Playing widget.

## State vs. Event pattern

`NowPlaying` never flows through the `StreamEvent` / `AppEvent` dispatch pattern. Instead:

1. **Producer** (media player observer) parses and dedupes a `NowPlaying`.
2. **Publishes** `Some(NowPlaying)` (or `None`) on a `tokio::sync::watch::Sender` held by `AppState`.
3. **Consumers** clone the `watch::Receiver` and read the latest value: the Overlay Feed turns it into a `FeedEvent::NowPlaying` frame, and the TUI reads it each tick.

Because `watch` is latest-value-wins, any consumer that subscribes mid-track immediately sees the current track — which is exactly why a `broadcast` channel (no replay of the last value) was rejected in the ADR.
