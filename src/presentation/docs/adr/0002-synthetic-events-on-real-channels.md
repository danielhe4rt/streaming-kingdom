# 2. Synthetic (test) events ride the real channels

Date: 2026-06-19

Status: Accepted

## Context

To preview the Coworking Overlay without real Twitch / Livepix / Spotify
traffic, the toolkit can fire **Synthetic Events** — fabricated stream events,
chat messages, and now-playing tracks — from a **Test events** sub-item in the
TUI (and from the `/overlay/dev` browser panel).

The question is what path a synthetic event takes. The Overlay only ever sees
data that rides the broadcast/watch channels the real adapters feed
(`event_tx`, `chat_tx`, `now_playing_tx`); the Overlay Feed (`GET /overlay/feed`)
is a subscriber of those. The TUI is *also* a subscriber of the same channels
(its `drain` records stats and appends to the event log). So a synthetic event
can either:

- ride those same channels (and thus be indistinguishable from a real event
  everywhere — Overlay reacts, TUI log shows it, stats counters move), or
- travel a separate, test-only path so it reaches the Overlay but never touches
  the TUI's stats/log.

## Decision

Synthetic events ride the **real** channels, with **no test/real distinction**.
A fired test donation is the exact same `StreamEvent::Donation` a real Livepix
donation would be; it fans out to the Overlay Feed and the TUI drain identically.

Both trigger surfaces (TUI Test events pane, `/overlay/dev` panel) build their
events from one shared `presentation::synthetic` module so the two never drift.

## Consequences

**Positive**
- Highest-fidelity test: exercises the entire real pipeline (channel → feed →
  Overlay, and channel → TUI drain → stats/log), not a parallel mock path.
- Zero extra plumbing: no second channel, no `is_synthetic` flag threaded through
  the domain event, the feed DTO, the drain, and the stats recorder.
- The synthetic builder is the single source of truth for both surfaces.

**Negative / trade-offs**
- A fired test event **pollutes the session**: followers/subs counters move and
  the event log fills with fabricated entries. Acceptable because test events are
  fired in a non-live testing session, and firing is deliberate (navigate to the
  Test events pane, move the cursor, press Enter).
- There is no way to tell, after the fact, that a logged event was synthetic.

**Reversibility**
- Moderate. Segregating later means either a second Overlay-only channel or an
  `is_synthetic` flag that the TUI drain/stats ignore — a change that touches the
  domain event, the channels, and the drain. The `presentation::synthetic`
  builder boundary keeps the blast radius of such a change contained.
