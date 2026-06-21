# Domain — app_event

> **Keep in sync:** this file documents `src/domain/app_event.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

The unified event log that captures all system activity into a single, serializable record. `AppEvent` is a domain-wide event type that wraps domain events (like `StreamEvent`) alongside infrastructure events (connection status, window management, privacy operations). Each event is timestamped as an `AppEventEntry` at log time, enabling rich diagnostics and replay-like inspection of system behavior within the TUI.

## Types created

### `AppEvent` (enum)

The core event type. Represents any noteworthy occurrence in the system. Grouped variants:

- **`Stream(StreamEvent)`** — a wrapped stream event (follow, sub, donation, cheer, raid, etc.). Created by the TUI run loop as it drains the `StreamEvent` broadcast (see Flow 1).
- **`PrivacyBlurEnabled { title }`** — privacy blur was activated with a given window title. Created by privacy feature.
- **`PrivacyBlurDisabled`** — privacy blur deactivated. Created by privacy feature.
- **`PrivacyStarted`** — privacy monitoring started. Created by privacy feature.
- **`PrivacyStopped`** — privacy monitoring stopped. Created by privacy feature.
- **`PrivacyError(msg)`** — privacy feature failed with an error message. Created by privacy feature on failure.
- **`FeatureToggled { feature, enabled }`** — a feature (Waybar, Alerts, Livepix, etc.) was toggled on/off. Created by application state when a command is applied.
- **`WaybarSpawned`**, **`WaybarKilled`**, **`WaybarError(msg)`** — Waybar process lifecycle. Created by Waybar integration.
- **`Info(msg)`**, **`Error(msg)`** — generic diagnostic messages from any subsystem. Created widely: infrastructure (Twitch, Hyprland, etc.), features (TTS), application.
- **`LivepixInfo(msg)`**, **`LivepixError(msg)`** — Livepix-specific messages. Created by Livepix integration.
- **`WindowOpened { title }`**, **`WindowClosed { address }`**, **`WindowTitleChanged { title }`** — Hyprland window lifecycle. Created by Hyprland event monitor. (Only `WindowClosed` carries the window `address`, which it renders truncated; the others log just the title.)
- **`WorkspaceChanged { name }`** — Hyprland workspace switched. Created by Hyprland event monitor.
- **`MonitorFocused { monitor }`** — Hyprland monitor focus changed. Created by Hyprland event monitor.
- **`WindowMoved { workspace }`** — Hyprland window moved to a workspace. Created by Hyprland event monitor.
- **`ChatMessage { username, text }`** — IRC chat message received. Created by Twitch IRC handler.

### `EventGroup` (enum)

Categorical bucket for filtering the event log in the TUI. Six possible values: `Stream`, `Privacy`, `System`, `Hyprland`, `Livepix`, `Chat`. Used for filtering which event types appear in the TUI event log pane.

### `AppEventEntry` (struct)

A timestamped wrapper around an event:
- **`event: AppEvent`** — the event itself.
- **`elapsed: Duration`** — time elapsed since app startup when this event was logged.

Created by `AppState::log_event()` when any event is recorded.

## Domain interactions (use-case flows)

### Flow 1: Incoming Twitch Stream Event → App Event Log

```
INFRASTRUCTURE                BROADCAST CHANNEL            PRESENTATION (TUI run loop)
      │                              │                              │
      │ StreamEvent arrives          │                              │
      │ from EventSub (e.g. Follow)  │                              │
      │ ───────────────────────────► │ event_tx.send(StreamEvent)   │
      │                              │ ───────────────────────────► │
      │                              │                              │ drain::stream_events()
      │                              │                              │  - app.stats.record(&ev)
      │                              │                              │  - app.log_event(
      │                              │                              │      AppEvent::Stream(ev))
      │                              │                              │  - maybe push TUI highlight
      │                              │                              │
      │                              │  (Waybar writer + Overlay    │ render_event_log()
      │                              │   feed consume the same      │  - filter by EventGroup
      │                              │   broadcast independently)   │  - format with elapsed
      │                              │                              │  - scroll-able pane
```

**Code flow:**
1. Infrastructure `twitch/eventsub/message_handler.rs` parses an incoming Twitch event into a `StreamEvent` and sends it on the `event_tx` broadcast.
2. The broadcast fans out to independent consumers: the TUI run loop, the Waybar writer, and the Overlay feed.
3. The TUI run loop drains the events in `drain::stream_events()` (`src/presentation/tui/drain.rs`), which:
   - Updates stats via `app.stats.record(&ev)`.
   - Logs `app.log_event(AppEvent::Stream(ev))`.
   - Optionally pushes a TUI highlight.
4. `log_event()` creates an `AppEventEntry` with current `elapsed` and appends to `app.event_log` (capped at `max_events`).
5. The TUI renders `app.event_log` in the event-log pane, reversed (newest first), with filters applied per `EventGroup`.

### Flow 2: Feature Command → Feature Toggle Event

```
TUI INPUT                              APPLICATION STATE                  EVENT LOG
      │                                      │                                │
      │ User toggles Waybar (keyboard)       │                               │
      │ ──────────────────────────────────►  │                               │
      │                                      │ apply_command()                │
      │                                      │  EnableWaybar                  │
      │                                      │  - set waybar_enabled = true   │
      │                                      │  - create AppEvent::FeatureTogg│
      │                                      │    led {                       │
      │                                      │      feature: "Waybar",        │
      │                                      │      enabled: true             │
      │                                      │    }                           │
      │                                      │  - call log_event(...)         │
      │                                      │ ──────────────────────────────►│
      │                                      │                                │ Entry appended
      │                                      │                                │
```

**Code flow:**
1. TUI input handler (e.g., key press) sends a `FeatureCommand::EnableWaybar`.
2. Application `apply_command()` matches the variant, updates the flag, and calls `log_event(AppEvent::FeatureToggled { ... })`.
3. Event is logged with current elapsed time.
4. On next frame, TUI sees the updated state and renders the new entry.

### Flow 3: Infrastructure Error → System Event

```
INFRASTRUCTURE (e.g., Twitch IRC)     APPLICATION STATE                  EVENT LOG / TUI
         │                                    │                               │
         │ Connection failed (IO error)       │                               │
         │ ────────────────────────────────► │                               │
         │ sends via event_tx:                │                               │
         │ AppEvent::Error("Twitch chat      │ log_event(event)              │
         │   disconnected, reconnecting...")  │ ──────────────────────────►  │
         │                                    │                               │ Visible in
         │                                    │                               │ EventLog pane
         │                                    │                               │
```

**Code flow:**
1. Infrastructure module (IRC, EventSub, etc.) encounters an error.
2. Sends an `AppEvent::Error(...)` or `AppEvent::Info(...)` via the channel to main loop.
3. Main loop receives and calls `app.log_event(event)`.
4. Entry is persisted and rendered in TUI.

### Flow 4: Hyprland Window Event → System Event

```
HYPRLAND MONITOR (ipc)                     INFRASTRUCTURE ADAPTER           APP STATE
         │                                         │                           │
         │ Window opened event                     │                           │
         │ (window_address, window_title)          │                           │
         │ ────────────────────────────────────►   │                           │
         │                                         │ keep title, create         │
         │                                         │ AppEvent::WindowOpened     │
         │                                         │ ───────────────────────►  │
         │                                         │                           │ log_event()
         │                                         │                           │
```

**Code flow:**
1. Hyprland adapter receives a window event from the compositor IPC (the raw event carries the window address and title).
2. Creates `AppEvent::WindowOpened { title }` or similar variants — the address is dropped for these (only `WindowClosed` keeps it).
3. Sends via event channel to application state.
4. Application calls `log_event()` to record it.
5. May also be filtered and displayed in TUI.

### Filtering in the TUI

The `EventGroup` method bridges the domain model to presentation concerns:

```rust
impl AppEvent {
    pub fn group(&self) -> EventGroup {
        match self {
            AppEvent::Stream(_) => EventGroup::Stream,
            AppEvent::PrivacyBlurEnabled { .. } | /* ... */ => EventGroup::Privacy,
            AppEvent::WindowOpened { .. } | /* ... */ => EventGroup::Hyprland,
            // etc.
        }
    }
}
```

The TUI calls this method when rendering (or filtering) to show only the enabled groups:

```rust
// Pseudocode from presentation/ui.rs
for entry in app.event_log.iter().rev() {
    if should_show_group(&entry.event.group(), &tui.filters) {
        // render this entry
    }
}
```

This keeps the event taxonomy decoupled from the presentation layer — the domain owns the grouping logic.

---

## Relationships

- An **`AppEvent`** wraps a **`StreamEvent`** when the event originates from Twitch (variant `Stream(...)`).
- An **`AppEventEntry`** contains exactly one **`AppEvent`** plus a timestamp.
- An **`AppEvent`** has exactly one **`EventGroup`** via the `group()` method.
- The **`AppState`** maintains a bounded `event_log: Vec<AppEventEntry>` capped at `max_events`.
