# Infrastructure — discord

> **Keep in sync:** this file documents `src/infrastructure/discord`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## The service

The toolkit shows a **live voice-channel roster** — who is in the streamer's Discord voice channel, with avatars, names, mute/deaf, and a live **speaking** indicator. The streamer runs **Vesktop** (Electron + Vencord), which exposes only **arRPC** on its IPC socket — and arRPC has **no voice API** (it ignores `GET_SELECTED_VOICE_CHANNEL` and serves no speaking events). The official RPC voice API and the Bot Gateway were both rejected (the gateway never delivers speaking without a bot joining the call; a user-token library would be a ToS-violating self-bot).

So the voice state is read where it actually lives — **inside Vesktop's renderer** (Discord's Flux stores) — via the **Chrome DevTools Protocol (CDP)**, with **no Vencord build, no bot, and the user's own account** (works in every server and DM).

## How it works

```
 ~/.config/vesktop-flags.conf:  --remote-debugging-port=9222
        │  (persistent flag — not a build)
        ▼
 Vesktop renderer exposes CDP on 127.0.0.1:cdp_port
        │  cdp.rs: GET /json/list → find Discord page → Runtime.evaluate(inject.js)
        ▼
 inject.js (in the renderer): reads Vencord.Webpack stores, subscribes to Flux
   voice/speaking events, pushes camelCase snapshots ──► ws://127.0.0.1:bridge_port
        ▼
 bridge.rs (WS ingress): snapshot JSON → domain VoiceRoster → watch channel
        ▼
 Overlay Feed → FeedEvent::VoiceRoster (SSE) → overlay
```

The module publishes a domain `VoiceRoster` (see `docs/arch/15-domain-voice.md`) on a `watch` channel, structurally identical to how the Spotify now-playing observer feeds the application layer.

## What we use from it

### Renderer reader (`inject.js`)

Runs inside Vesktop (no plugin install — it uses the *already-loaded* `window.Vencord.Webpack`). It resolves `SelectedChannelStore`, `VoiceStateStore`, `ChannelStore`, `UserStore`, `GuildMemberStore`, subscribes to Flux events, and pushes a snapshot on every change:

- `SPEAKING` → `speakingFlags` (`1` = speaking, `0` = stopped), filtered to `context === "default"`.
- `VOICE_STATE_UPDATES` → membership / mute / deaf change.
- `VOICE_CHANNEL_SELECT` → channel hop (clears the speaking set).

It opens a `WebSocket` to the toolkit and reconnects every 3s. Idempotent: re-evaluating returns `"already-injected"` without re-subscribing. `__PORT__` is templated with `bridge_port` before evaluation.

### CDP injector (`cdp.rs`)

Finds the Discord page target via `GET http://127.0.0.1:{cdp_port}/json/list`, opens its `webSocketDebuggerUrl`, and `Runtime.evaluate`s the payload. Re-injects every 10s so a Vesktop reload (which drops the injected state) is recovered automatically.

### WS ingress (`bridge.rs` + `bridge_payload.rs`)

A local WebSocket server on `127.0.0.1:bridge_port`. Each text frame is deserialized (`bridge_payload.rs`, camelCase contract mirroring `VoiceRoster`) and published on the watch channel. A dropped connection clears the roster (publish `None`). Localhost-only; the only client is our own injected reader.

## Wire contract (reader → toolkit)

One JSON message per change; a cleared roster (left voice) sends `channelId: null`, `members: []`.

```json
{
  "channelId": "684750059719622871",
  "channelName": "🗣 Conversando",
  "members": [
    { "userId": "204122995579551744", "displayName": "danielhe4rt",
      "avatarUrl": "https://cdn.discordapp.com/avatars/204.../028d....png",
      "speaking": true,
      "selfMute": false, "selfDeaf": false, "serverMute": false, "serverDeaf": false }
  ]
}
```

## How it's wired

### Public API (`mod.rs`)

`spawn(cmd_rx, status_tx, voice_roster_tx, config)` idles until `DiscordCommand::Start`, then runs `bridge::serve` + `cdp::inject_loop` together (either ending ends the session), publishing `None` and `DiscordStatus::Stopped` on the way out. Mirrors the Overlay server / Livepix `spawn()`. `DiscordStatus` is `Stopped` / `Running { channel }` / `Error`.

### Configuration

```toml
[discord]
# bridge_port = 1340   # local WS ingress the injected reader pushes to
# cdp_port = 9222      # must match Vesktop's --remote-debugging-port
```

| Field | Source | Used for |
|-------|--------|----------|
| `bridge_port` | `[discord]` config (or `DISCORD_BRIDGE_PORT`) | WS ingress the reader pushes to (default `1340`) |
| `cdp_port` | `[discord]` config (or `DISCORD_CDP_PORT`) | Vesktop DevTools port to inject over (default `9222`) |

Ports are kept distinct from overlays (`1111`) and arRPC's bridge (`1337`).

### TUI Services pane

A **Discord** row with a status dot reflects `DiscordStatus`: `Stopped` / `Running` / `Error`.

### Data flow

```
┌─ Vesktop renderer (CDP) ── cdp.rs inject ── inject.js ──ws──→ bridge.rs ──→ VoiceRoster watch
│                                                                              ↓
│                                                       Overlay Feed → FeedEvent::VoiceRoster (SSE)
│                                                       TUI Services pane → Discord row status
└─ status/errors ──→ DiscordStatus mpsc → TUI Services pane
```

## Gotchas

**Vesktop must run with the debug flag:** add `--remote-debugging-port=9222` to `~/.config/vesktop-flags.conf` (Vesktop's persistent flag file) and restart. Without it `cdp.rs` finds no target and reports `Error`; the rest of the toolkit is unaffected.

**Debug-port exposure:** `--remote-debugging-port` lets any local process control the Vesktop renderer (run arbitrary JS). Acceptable on a personal machine, but it is a real exposure. Delete the flags file to disable.

**Renderer fragility:** `inject.js` reads Discord's webpack stores via Vencord; a large Discord web update can change store shapes and require a tweak. The captured shapes today: voice state `{ userId, mute, deaf, selfMute, selfDeaf }`, speaking `{ userId, speakingFlags, context }`.

**Synthetic testing:** `GET /overlay/dev/event/voiceRoster` pushes a fake roster through the feed with no Discord at all — build/iterate the overlay widget without Vesktop.
