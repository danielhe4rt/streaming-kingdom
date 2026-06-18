# Domain — stats

> **Keep in sync:** this file documents `src/domain/stats.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

This module models the *runtime metrics* of an active stream — the aggregated counters that measure audience engagement during a session. A `StreamStats` instance records key performance indicators (followers gained, subscriptions received, concurrent viewers) and is mutated in real-time as stream events arrive. These counters are session-scoped and reset when a new stream begins.

## Types created

### `StreamStats` — the aggregate statistics container

A mutable struct holding three running counters that summarize a stream session:

- **`viewer_count: u32`** — the live viewer count at the stream right now (updated by `ViewerCountUpdate` events).
- **`followers_today: u32`** — total follows received since the session started (incremented by `Follow` events).
- **`subs_today: u32`** — total subscriptions received since the session started (both regular `Sub` and `GiftSub` events increment this counter).

Created by `StreamStats::new()` in the `AppState` constructor when a toolkit session initializes (see `src/application/state.rs:56`). The `StreamStats` instance is stored in `AppState` and is mutated whenever the application's `dispatch_event()` method processes an incoming `StreamEvent`.

---

## Domain interactions (use-case flows)

### Flow: Stream event arrives → stats updated

The primary flow is **event-driven mutation**:

1. **Infrastructure layer** (Twitch IRC, EventSub) receives a raw platform event.
2. **Application layer** converts it to a domain `StreamEvent` and calls `AppState::dispatch_event()`.
3. **Inside `dispatch_event()`** (line 75–80, `src/application/state.rs`):
   - `self.stats.record(&event)` is called, which pattern-matches the event type.
   - **`StreamEvent::Follow { .. }`** → `followers_today += 1`
   - **`StreamEvent::Sub { .. }` or `StreamEvent::GiftSub { .. }`** → `subs_today += 1`
   - **`StreamEvent::ViewerCountUpdate { count }`** → `viewer_count = count` (overwrites, not increments)
   - **Other events** (`Donation`, `Cheer`, `Raid`) → ignored (no stat impact)
4. The event is also logged to the unified event log and broadcast to all subscribers.
5. Any UI module (TUI, Waybar) observing the event log or subscribing to the event broadcast will receive the update and can display the new stat values.

**Data flow diagram:**

```
Stream Event (Follow, Sub, GiftSub, ViewerCountUpdate)
       ↓
AppState::dispatch_event(event)
       ↓
   StreamStats::record(&event)
       ├─ Follow event?          → followers_today += 1
       ├─ Sub/GiftSub event?     → subs_today += 1
       ├─ ViewerCountUpdate?     → viewer_count = count
       └─ Other?                 → (no change)
       ↓
Event broadcast + logging
       ↓
UI consumers read updated counters from AppState.stats
```

**Key property:** `StreamStats` is a synchronous, in-memory aggregate; it has no persistence or recovery mechanism. On session end, counters are discarded (resetting for the next stream). The module is **not concerned** with *why* events happen or *what* happens downstream — it only maintains the running totals.

---

## Notes on interactions with other layers

- **Upstream (Infrastructure):** The `StreamStats` module does not depend on infrastructure details; it only knows about `StreamEvent`, which is emitted by Twitch listeners and other event sources.
- **Downstream (Presentation/Alerts):** UI modules and feature handlers (Waybar, TUI, alerts) are free to read `AppState::stats` to display or react to the current counters. There is no callback or pub/sub mechanism for stat changes specifically — consumers poll by reading the struct field.
- **Application layer coordination:** The `AppState` is responsible for orchestrating the flow; `StreamStats::record()` is a pure domain operation (stateless mutation) invoked by the application.
