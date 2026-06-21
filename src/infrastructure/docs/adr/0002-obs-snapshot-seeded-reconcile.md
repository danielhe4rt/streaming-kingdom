# 2. OBS desired-state via snapshot-seeded reconcile, with a fail-loud privacy pair

Date: 2026-06-21

Status: Accepted

## Context

The Privacy Monitor (`infrastructure/hyprland/privacy_monitor.rs`) auto-blurs an OBS
**Capture source** when a sensitive window opens. Its single job is to never let a
secret reach the stream — yet it fails *silently* when the OBS state it assumes has
drifted:

- **Scenario B** — the streamer edits OBS by hand and deletes/renames the capture
  source or the `Composite Blur` filter. `enable_blur()` then targets something that
  no longer exists, the `obws` call errors, and the error is swallowed
  (`Non-fatal degradation`). The monitor still reports `Running`.
- **Scenario C** — OBS restarts or a different scene collection is loaded. The blur
  target is gone or renamed, and the monitor (which today does **not** auto-reconnect)
  points at nothing.

In both cases the operator opens a `.env` believing they are protected, and nothing
blurs. We want the toolkit to *guarantee* a valid blur target, and — by extension —
to rebuild the whole overlay scene to a known-good shape after drift ("auto-setup").

Three choices here are non-obvious and worth recording.

## Considered options

### Source of truth for "how the scene should be"

- **A — pure live reconcile over `obws`.** Read the live scene, patch differences via
  the WebSocket. *Rejected:* on Wayland/pipewire the API **cannot choose which monitor
  a screen-capture source grabs** — that selection is negotiated through the
  `xdg-desktop-portal` ScreenCast dialog, not via input settings. There is no headless
  way to provision the capture source from nothing.
- **B — restore the on-disk scene-collection JSON** (`~/.config/obs-studio/basic/scenes/*.json`).
  Full fidelity, captures even the capture source. *Rejected as primary:* OBS holds the
  collection in memory and **overwrites the file on graceful exit / collection switch**,
  and only **reads** it at startup or switch. So a file restore cannot repair drift
  *during* a live stream (the exact Scenario B moment); it needs OBS closed + reopened.
- **C — snapshot-seeded reconcile (chosen).** Capture a known-good OBS once into an
  **OBS snapshot** (a **Scene baseline**) via `obws` reads, then reconcile the live OBS
  toward it. The snapshot includes each pipewire source's **portal restore token**
  (it lives inside the source's input settings and round-trips via
  `GetInputSettings`/`SetInputSettings`), so the streamer picks the monitor by hand only
  once and reconcile can replay it live.

### When reconcile runs

Not continuously. On the **(re)connection** of the toolkit↔OBS link: at toolkit boot
with OBS up, and whenever the `obws` connection drops and re-establishes (covering OBS
restart / collection switch). This requires adding `obws` auto-reconnect, which the
code lacks today.

### How the privacy pair is guarded

A continuous OBS watcher was explicitly rejected as too heavy. Instead the
**Privacy pair** (Capture source + its Blur filter) is guarded **just-in-time**: at the
moment a sensitive window opens and blur is attempted, reconcile the pair first (recreate
the filter if missing — safe), then verify the blur took effect. This accepts one narrow
blind spot: a secret already on screen *with blur already active*, whose filter is then
deleted at that instant, is not caught until the next blur attempt or reconnect.

## Decision

1. **Baseline = `obws`-composed snapshot, versioned in the repo.** A new explicit TUI
   command captures the live OBS (`GetSceneList`, scene items + transforms,
   `GetInputList` + `GetInputSettings` incl. restore token, `GetSourceFilterList` +
   filters) into `obs-baseline.json`, synced to `~/.config/streams-toolkit/` per the
   project source-of-truth rule. Capture **refuses to save** a baseline whose
   privacy pair is absent. Single baseline to start; the format allows naming multiple
   later. The on-disk OBS scene JSON is at most an optional cold backup, never the
   primary source.

2. **Reconcile on (re)connection.** On toolkit↔OBS connect (boot + `obws` reconnect),
   reconcile live OBS toward the baseline — create-if-missing, fix-if-wrong. The whole
   scene (Level 3) is repaired **silently**; the privacy pair (Level 1) is the
   safety-critical exception and **fails loud** instead of degrading quietly.

3. **Just-in-time privacy guard, fail-safe blackout.** No continuous watcher. At blur
   time the Privacy Monitor reconciles the pair, enables blur, and verifies it. If blur
   still cannot be guaranteed (capture source gone), it **fails safe** by hiding the
   capture source in OBS (blackout) and raising a loud TUI alert — privacy wins over the
   embarrassment of a momentary black frame.

## Consequences

**Positive**
- The streamer can no longer be silently unprotected: a missing blur target is either
  repaired, or it blacks out and shouts. Scenario B/C are closed at the moments that
  matter.
- One unified engine (snapshot → baseline → reconcile) serves three asks: the dev-time
  snapshot, the overlay auto-setup, and the privacy guardrail.
- The pipewire monitor choice survives restarts via the captured restore token; the
  operator picks it by hand only once.

**Negative / trade-offs**
- Adds `obws` auto-reconnect and a reconcile engine — more moving parts in the OBS
  adapter than today's single filter toggle.
- A live fail-safe **blackout** can surprise the operator mid-stream; chosen
  deliberately (a `.env` on stream is a security incident, a black frame is not).
- `Non-fatal degradation` is no longer universal — the privacy pair is carved out as a
  fail-loud exception (see `infrastructure/CONTEXT.md` ambiguities).

**Known residual risk (not covered)**
- If the `obws` link drops while OBS keeps streaming the raw screen (e.g. the WebSocket
  server is disabled mid-stream), the toolkit is blind and cannot blackout. Narrow:
  `obws` loss almost always means OBS is gone, hence not streaming. Accepted.

**Reversibility**
- Adding the continuous watcher later (to close the narrow blind spot) only extends the
  privacy guard; the baseline/reconcile boundary stays put.
