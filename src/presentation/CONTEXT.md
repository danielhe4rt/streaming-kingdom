# Context: Presentation

**Layer:** `src/presentation/` · **Triage label:** `area:presentation`

The toolkit's two **outbound renderers** of domain state. Sub-contexts:

- `tui/` — the terminal control panel (nav shell: topbar sections → sidebar sub-nav → content).
- `http/` — the HTTP server that serves **Overlays** to the browser (OBS browser sources),
  Laravel-like layout (`routes` / `controllers` / `resources` / `assets`).

Boundary rule: **presentation = OUTBOUND (render).** Inbound HTTP (e.g. the Livepix donation
webhook) stays in `infrastructure/` — receiving is an adapter concern, not a view.

## Glossary

### Overlays

**Overlay**:
A browser source served by the toolkit's `http` renderer and added as a layer in OBS.
_Avoid_: widget, browser source (use "Overlay" for the thing we serve; "browser source" is OBS's term for where it's mounted).

**Chat Overlay**:
The Overlay that renders live chat with full parity to the old StreamElements custom chat
(coloured nick, Twitch badges, native Twitch emotes). Consumes the **Overlay Feed**.

**Frame Overlay**:
The Overlay that renders the branding frame (border, logo, socials, channel) plus a **Footer Bar**
and a now-playing slot. Replaces the old static PNG.

**Overlay Feed**:
The single SSE stream (`GET /overlay/feed`) carrying enriched chat + stream events. **One source,
N Overlays** consume it in parallel — the chat is a shared feed, not owned by any one Overlay.
_Avoid_: chat stream, websocket (it's SSE, and it carries more than chat).

**Footer Bar**:
The Frame Overlay's bottom zone. A shared slot that arbitrates between **Fun Facts** (idle) and
**Alerts** (takeover). Alerts queue — one at a time, animate in → hold → out → back to Fun Facts.

**Fun Facts**:
Rotating text shown in the Footer Bar when no Alert is playing.

**Alert**:
An animated takeover of the Footer Bar triggered by a `StreamEvent` (donation / sub / raid).

### TUI

**Service**:
A row in the TUI representing one integration, classified as an **Input** or an **Output**.
- **Input** — feeds the toolkit (Twitch EventSub, Twitch Chat, Livepix, Hyprland). Read-only/monitor.
- **Output** — driven by the toolkit (Waybar bar, Overlays). Toggleable on/off.

## Relationships

- An **Overlay** consumes the **Overlay Feed** (SSE). Many Overlays ↔ one Feed.
- The **Frame Overlay** owns one **Footer Bar**; the Footer Bar plays zero-or-more **Alerts**, each from one **StreamEvent**.
- A **Service** is exactly one of **Input** or **Output**. **Overlays** is an Output that, when toggled on, starts the `http` renderer.
- The `tui` and `http` renderers both subscribe to the same `broadcast<StreamEvent>` and (now) `broadcast<ChatMessage>` — neither owns the other.

## Example dialogue

> **Dev:** "Quando um mod apaga uma mensagem, quem some ela da tela?"
> **Streamer:** "A **Chat Overlay** — ela ouve o delete pelo **Overlay Feed** e remove aquele nó."
> **Dev:** "E um sub novo durante um donation tocando no rodapé?"
> **Streamer:** "Entra na fila do **Footer Bar**. Toca o donation, depois o sub, depois volta pro **Fun Facts**."

## Flagged ambiguities

- "widget" (termo do StreamElements) foi aposentado em favor de **Overlay**.
- "overlay" antes significava o PNG estático da moldura; agora significa qualquer página servida pelo `http`. A moldura virou a **Frame Overlay**.

## Decisions

See `docs/adr/` (system-wide) and `src/presentation/docs/adr/` (this context, when present).
