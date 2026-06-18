# Overlays served as an embedded React app driven by an SSE feed

We serve the toolkit's Overlays (Chat, Frame) as a React + Vite + Tailwind app (built with Bun, source
in `overlays/`) **embedded into the Rust binary** at release time, fed in real time by a single
**Server-Sent Events** stream (`GET /overlay/feed`) carrying enriched chat + stream events. One feed,
N Overlays consume it in parallel. This replaces the previous StreamElements custom-chat widget and the
static PNG frame.

## Why

The toolkit already holds the live Twitch chat connection and the `broadcast<StreamEvent>` stream.
Routing Overlays through our own HTTP server removes the third-party (StreamElements) dependency, cuts
latency (served from `127.0.0.1`), and lets Overlays react to internal events (donations, alerts) the
external service can't see. Embedding the built bundle keeps distribution a single binary.

## Considered options

- **Keep StreamElements.** Rejected: external dependency, added latency, blind to toolkit-internal events.
- **Template HTML in Rust (no JS framework).** Rejected: component parity (badges/emotes, queued alert
  bar, animations) is real UI work better expressed as headless React components than as Rust string
  templating.
- **WebSocket instead of SSE.** Rejected: Overlays are display-only (server→browser); SSE is simpler and
  auto-reconnects, which matters for an OBS browser source.
- **Serve `dist/` from disk instead of embedding.** Rejected for release (distribution would have to ship
  a loose asset folder); dev still uses the Vite dev server directly.

## Consequences

- A Node/Bun toolchain now lives in a Rust repo (`overlays/`), with a build step feeding `rust-embed`.
- In dev, the Vite dev server runs on a separate origin, so the SSE endpoint needs CORS for local dev.
- Chat must fan out to two consumers, so `ChatMessage` moves from `mpsc` to `broadcast` and gains the
  fields needed for parity (color, badges, emote fragments, msgId).
