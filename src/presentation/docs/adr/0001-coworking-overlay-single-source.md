# 1. Coworking Overlay: one full-screen source replaces two composable Overlays

Date: 2026-06-18

Status: Accepted

## Context

The `http` renderer originally served two independent Overlays, each its own OBS
browser source:

- **Chat Overlay** (`/overlay/chat`) — a bare, bottom-aligned live-chat list.
- **Frame Overlay** (`/overlay/frame`) — a branding frame (border, logo, socials,
  channel) plus a Footer Bar (Fun Facts ↔ Alerts) and a now-playing slot.

The streamer composed a scene by stacking these sources (plus a camera and a
screen capture) manually in OBS. Both Overlays consumed the same **Overlay Feed**
(SSE): one source, N Overlays.

A new reference design (`Coworking + Chat`) is a single 1920×1080 layout where the
chat, branding, now-playing, Fun Facts ticker, and event alerts are all baked
around a **transparent camera cutout** on the right. It is not two stacked
sources — it is one scene frame with a hole the camera shows through.

The question: render this as a third composable Overlay, or collapse the existing
two into a single full-screen Overlay?

## Decision

Collapse Chat Overlay and Frame Overlay into a single **Coworking Overlay** served
at `/overlay/coworking`. It is one OBS browser source layered on top of the
camera/screen source, which shows through the Camera Window cutout. The
`/overlay/chat` and `/overlay/frame` routes, their controllers, and the
`ChatOverlay`/`FrameOverlay` React components are removed.

Internally the Overlay keeps the "one source, N Overlays" feed model and a
smart/dumb split: headless `ui/` components (props only, no feed) consume decoupled
view-model shapes; a `CoworkingOverlay` container owns the feed hooks and maps the
Overlay Feed DTOs onto those shapes. The arbitration of the alert-over-ticker reuses
the existing `footerBar` queue reducer.

## Consequences

**Positive**

- The scene matches the reference design exactly — fixed geometry, the camera
  cutout, the yellow seam — instead of relying on hand-aligned OBS source stacking.
- One browser source to add and position in OBS instead of several.
- Shared headless `ui/` components remain reusable by any future Overlay; the
  collapse is about *composition*, not about giving up the component library.

**Negative / trade-offs**

- Loses per-scene mix-and-match: you can no longer drop just the chat onto an
  unrelated scene by adding the chat source. A future scene that needs only chat
  must re-compose a new Overlay from the `ui/` components.
- It is a breaking change for any existing OBS setup pointing at `/overlay/chat`
  or `/overlay/frame` — those URLs now 404 and the single source must be repointed
  to `/overlay/coworking`.

**Reversibility**

- Moderately hard: re-introducing composable Overlays means restoring routes,
  controllers, and standalone components. The decoupled `ui/` layer keeps the cost
  bounded — the presentational pieces survive a reversal.
