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

**Coworking Overlay**:
The single full-screen Overlay (`/overlay/coworking`, 1920×1080) that renders the whole stream
scene: a **Top Bar**, a **Chat Panel**, a **Camera Window** (transparent cutout), and a **Footer Bar**.
Replaces the former separate **Chat Overlay** and **Frame Overlay** — one OBS browser source on top,
the camera/screen source behind it showing through the Camera Window. Consumes the **Overlay Feed**.

**Top Bar**:
The Coworking Overlay's branding header (logo, social icons, channel handle).

**Chat Panel**:
The Coworking Overlay's left column. Renders live chat with full parity to the old StreamElements
custom chat (Twitch badges, native emotes) inside a name-pill + speech-bubble style. Consumes the
**Overlay Feed**'s chat messages.

**Camera Window**:
The Coworking Overlay's transparent right-side cutout. The toolkit renders only its frame (border,
shadow, the yellow seam); the OBS camera or screen-capture source placed *behind* the Overlay shows
through the hole.

**Overlay Feed**:
The single SSE stream (`GET /overlay/feed`) carrying enriched chat + stream events. **One source,
N Overlays** consume it in parallel — the chat is a shared feed, not owned by any one Overlay.
_Avoid_: chat stream, websocket (it's SSE, and it carries more than chat).

**Footer Bar**:
The Coworking Overlay's bottom zone. Hosts the now-playing slot (left) and a shared ticker slot
(right) that arbitrates between **Fun Facts** (idle) and **Alerts** (takeover). Alerts queue — one at
a time, animate in → hold → out → back to Fun Facts.

**Fun Facts**:
Rotating text shown in the Footer Bar when no Alert is playing.

**Alert**:
An animated takeover of the Footer Bar triggered by a `StreamEvent` (donation / sub / raid).

### TUI

**Service**:
A row in the TUI representing one integration, classified as an **Input** or an **Output**.
- **Input** — feeds the toolkit (Twitch EventSub, Twitch Chat, Livepix, Hyprland, Spotify). Read-only/monitor.
- **Output** — driven by the toolkit (Waybar bar, Overlays). Toggleable on/off.

**Synthetic Event**:
A fabricated stream event / chat message / now-playing track that the streamer fires to preview the
Overlay without real Twitch/Livepix/Spotify traffic. It is **indistinguishable from a real event**
everywhere — the Overlay reacts, the TUI log shows it, and the stats counters move — because it rides
the same channels a real one does. _Avoid_: fake event, mock event.

**Test Events**:
The always-present **Test events** sub-item under the Overlays section that fires **Synthetic Events**
(every Overlay-rendered kind: each StreamEvent, a chat message, a now-playing track, and a
delete-last-message). A deliberate dev affordance — `J/K` picks a type, `Enter` fires. Also reachable
via the `/overlay/dev` browser panel; both build their events from one shared source.

## Relationships

- An **Overlay** consumes the **Overlay Feed** (SSE). Many Overlays ↔ one Feed.
- The **Coworking Overlay** owns one **Top Bar**, one **Chat Panel**, one **Camera Window**, and one **Footer Bar**; the Footer Bar plays zero-or-more **Alerts**, each from one **StreamEvent**.
- A **Service** is exactly one of **Input** or **Output**. **Overlays** is an Output that, when toggled on, starts the `http` renderer.
- The `tui` and `http` renderers both subscribe to the same `broadcast<StreamEvent>` and (now) `broadcast<ChatMessage>` — neither owns the other.

## Example dialogue

> **Dev:** "Quando um mod apaga uma mensagem, quem some ela da tela?"
> **Streamer:** "O **Chat Panel** da **Coworking Overlay** — ele ouve o delete pelo **Overlay Feed** e remove aquele nó."
> **Dev:** "E um sub novo durante um donation tocando no rodapé?"
> **Streamer:** "Entra na fila do **Footer Bar**. Toca o donation, depois o sub, depois volta pro **Fun Facts**."

## Flagged ambiguities

- "widget" (termo do StreamElements) foi aposentado em favor de **Overlay**.
- "overlay" antes significava o PNG estático da moldura; agora significa qualquer página servida pelo `http`.
- **Chat Overlay** e **Frame Overlay** (duas Overlays separadas, uma por OBS browser source) foram
  fundidas numa única **Coworking Overlay** full-screen com recorte de câmera. Os termos antigos só
  aparecem em histórico/issues; o presente é a Coworking Overlay.

## Decisions

See `docs/adr/` (system-wide) and `src/presentation/docs/adr/` (this context, when present).
