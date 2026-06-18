# Presentation split into outbound renderers: `tui` + `http`

We split the `presentation` layer into two sub-renderers — `tui/` (the terminal control panel) and
`http/` (an axum server that serves Overlays as OBS browser sources) — under the rule that
**presentation = OUTBOUND (render domain state into a view)**. Both subscribe to the same
`broadcast<StreamEvent>` and `broadcast<ChatMessage>`; neither owns the other.

We deliberately keep **inbound** HTTP — the Livepix donation webhook — in `infrastructure/`, even
though it is also axum. The boundary is direction, not protocol: receiving a webhook is an adapter
ingesting an external event (infrastructure); serving an overlay is rendering a view (presentation).

## Considered options

- **All HTTP in infrastructure** (consistency: "every axum thing is infra"). Rejected: it would file a
  view-renderer next to inbound adapters and contradict the render-vs-adapter boundary that makes the
  layer legible.
- **Move the Livepix webhook into `presentation/http` too** (unify "all HTTP"). Rejected: mixes inbound
  ingestion with outbound rendering in one folder.

## Consequence

A future reader sees two axum servers in two different layers. The discriminator to remember:
**inbound (webhook, ingest) → infrastructure; outbound (overlay, render) → presentation.**
