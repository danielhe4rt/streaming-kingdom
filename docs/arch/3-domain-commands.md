# Domain — commands

**Keep in sync:** this file documents `src/domain/commands.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

Commands represent intentional user actions from the TUI that toggle feature modules on and off. Each command targets a specific system feature (Waybar, Privacy, Alerts, Livepix) and declares whether it should be enabled or disabled. Commands form a simple, stateless API: the presentation layer (TUI input handler) creates them, the application layer consumes them and updates feature toggles, and infrastructure/feature modules observe the resulting state changes.

## Types created

### FeatureCommand

A discriminated union enum with eight variants, each declaring a feature toggle intent:

- **EnableWaybar / DisableWaybar** — Control whether the Waybar status module is active. Waybar is a display integration that renders streamer status (e.g. current track via Spotify, stream events).
- **EnablePrivacy / DisablePrivacy** — Control blur/obfuscation of sensitive window titles and content. Used when streaming to hide personal data.
- **EnableAlerts / DisableAlerts** — Control the alerts browser module, which displays stream events (follows, subs, raids) in a visual popup/overlay.
- **EnableLivepix / DisableLivepix** — Control the Livepix integration, a screenshot/pixel-capture feature for streaming.

No producer creates commands within the domain itself; commands are generated solely by the presentation layer's input handler when the user presses Enter/Space on a toggleable integration in the TUI.

## Domain interactions (use-case flows)

### End-to-end: User toggles a feature in the TUI

```
 USER                                   SYSTEM
  │                                        │
  │  👆 presses Enter on "Alerts"         │
  │ ──────────────────────────────────►  │
  │                                        │  Input Handler: toggle_command()
  │                                        │  checks app.alerts_enabled
  │                                        │  → creates FeatureCommand::EnableAlerts
  │                                        │
  │                                        │  sends to cmd_tx (mpsc channel)
  │                                        │
  │                                        │  AppState: apply_command()
  │                                        │  sets alerts_enabled = true
  │                                        │  emits AppEvent::FeatureToggled
  │                                        │  → appends to event_log
  │                                        │
  │    "Alerts enabled ✓"                 │
  │    (logged to event log)              │
  │ ◄──────────────────────────────────  │
  │                                        │
```

**Detailed steps:**

1. **User input** — User presses Enter/Space while focused on an integration in the TUI Integrations pane (e.g., the "Alerts" row at cursor position 5).

2. **Command generation** — The `toggle_command()` function in `presentation/input.rs` reads the current app state and the cursor position, then returns the appropriate `FeatureCommand` variant. For example, if alerts are currently off, it returns `FeatureCommand::EnableAlerts`.

3. **Command dispatch** — The command is sent via the `command_tx` (MPSC sender), which is held by the presentation layer. The application layer owns the receiver (`command_rx`).

4. **Command application** — The application calls `process_pending_commands()` in its main loop, which:
   - Tries to receive any pending commands from `command_rx`
   - For each command, calls `apply_command(&cmd)`
   - `apply_command()` matches the command variant and updates the corresponding boolean flag (e.g., `self.alerts_enabled = true`)
   - Emits an `AppEvent::FeatureToggled { feature: "Alerts", enabled: true }` and logs it

5. **Observation** — Infrastructure/feature modules (Hyprland, Privacy, Alerts, Livepix, Waybar) observe the feature toggles directly via `AppState`, or indirectly through watching the feature-toggle event in the event log.

### State machines (implicit)

Each feature implements a simple toggle machine:

```
DISABLED  ←─ DisableCommand ─┐
   ↓                          │
ENABLED   ←─ EnableCommand ──┘
```

A `FeatureCommand::EnableWaybar` on an already-enabled Waybar is idempotent (sets the flag to true again). The same applies to disable commands.

### Event chain: no circular dependencies

1. **Presentation → Domain/Application:** Command created and sent.
2. **Application → Domain:** `apply_command()` mutates app state and emits an `AppEvent::FeatureToggled`.
3. **Application → Infrastructure:** Feature modules read the updated boolean toggle and adjust behavior (e.g., Privacy module may start or stop blurring, Alerts module may kill or spawn its browser window).
4. **Infrastructure → Application:** Lifecycle events (e.g., `WaybarSpawned`, `WaybarError`) are logged as separate `AppEvent` variants.

**No feedback loop:** Commands do not re-trigger; they are consumed once and their effect is immediately visible in app state.
