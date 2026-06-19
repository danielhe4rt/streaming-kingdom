# 1. Now Playing as observed state via `playerctl --follow` on a `watch` channel

Date: 2026-06-18

Status: Accepted

## Context

The Coworking Overlay has a Now Playing widget that shipped as structure-only
(`useNowPlaying()` → null). We want it driven by the real current Spotify track.

A previous prototype polled MPRIS over D-Bus with a `dbus-send` loop every 2s,
parsing the reply with `grep`/`cut` and writing text files. We want the same data
(title / artist / album / cover) but expressed idiomatically inside the DDD layout:
a `domain::NowPlaying` value, an infrastructure adapter, and delivery over the
existing Overlay Feed.

Two choices here are non-obvious and worth recording, because every other live
signal in the toolkit (chat, stream events) is modelled the opposite way.

### 1. Acquisition: how the adapter reads MPRIS

- **`playerctl --follow` (chosen)** — spawn `playerctl --follow -p spotify metadata
  --format '…'` once and read stdout line by line. `--follow` emits a line on every
  track/status change, so it is event-driven with no polling loop and no per-tick
  subprocess. `playerctl` is already a project dependency (the Waybar formatter
  uses it), so no new crate. Matches the project's existing "subprocess where
  pragmatic" pattern (`mpv`/`ffplay` for TTS, `omarchy-restart-waybar`).
- **Native `zbus`** — subscribe to `org.mpris.MediaPlayer2.Player` `PropertiesChanged`
  in pure-Rust async D-Bus. The most "correct" option and dependency-free of
  subprocesses, but adds a crate and a non-trivial amount of low-level code to parse
  the `a{sv}` Metadata dictionary by hand.
- **`dbus-send` / `playerctl` polling (the old prototype)** — simplest mentally,
  but spawns a process every tick and lags a track change by up to the poll interval.

### 2. Delivery: what kind of channel carries it

Chat and stream events are discrete occurrences carried on `broadcast` channels and
logged into the `AppEvent` history. "Now playing" is different: it is **ambient
state** — there is at most one current track, latest-value-wins, and it is never
history we replay. A `broadcast` channel does not hand a newly-subscribed consumer
the last value, so an OBS browser source opened mid-track would sit on the
placeholder until the next track change.

## Decision

1. **Acquire** via a `playerctl --follow -p spotify metadata` subprocess observer in
   `infrastructure/media_player/`. It parses each stdout line into a
   `domain::NowPlaying`, deduplicates, and publishes only on change. If `playerctl`
   is missing or exits (no player running), it logs and retries with backoff and
   publishes `None` so consumers fall back to their placeholder. v1 targets Spotify
   specifically (`-p spotify`); broadening is a one-flag change.

2. **Deliver** over a `tokio::sync::watch::Sender<Option<NowPlaying>>` held by
   `AppState`. The Overlay Feed adds a third stream (`WatchStream`) and emits a
   `FeedEvent::NowPlaying` frame — a freshly-opened SSE connection immediately
   receives the current track. The TUI reads the same `watch` for its Spotify
   Service row. `NowPlaying` is deliberately **not** wrapped in `AppEvent`: it is
   observed state, not logged history.

## Consequences

**Positive**
- New overlay/TUI consumers see the current track instantly (watch latest-value).
- No polling, no per-tick process, no new D-Bus crate; reuses `playerctl`.
- Clean domain distinction: state (`watch`) vs events (`broadcast`).
- Parsing is isolated in a pure, unit-testable function.

**Negative / trade-offs**
- A subprocess dependency on `playerctl` at runtime (graceful-degrades if absent).
- `watch` is a second channel idiom in the codebase alongside `broadcast`; a reader
  must understand why now-playing is special (this ADR).
- Brittle to `playerctl` format/`--follow` behaviour changes; mitigated by the
  isolated parser + tests and graceful fallback.

**Reversibility**
- Swapping acquisition to native `zbus` later only touches the adapter — the
  `watch<Option<NowPlaying>>` boundary and everything downstream stay put.
