# Discord Voice Roster — Design Spec

**Date:** 2026-06-19
**Status:** Approved (design), pending implementation plan
**Author:** danielhe4rt (with Claude)

## Summary

Add a **persistent, headless Discord voice roster** to streams-toolkit. While the
streamer is connected to a Discord voice channel, the toolkit shows who else is in
that same channel — their avatar, display name, mute/deaf state, and a live
**speaking** indicator — by streaming roster snapshots over the existing SSE
Overlay Feed (`GET /overlay/feed`) to a React widget. The data source is the
**local Discord RPC/IPC socket** against the running desktop client (the same
mechanism Discord's official StreamKit overlay uses), not the Bot Gateway.

The roster is modeled as **ambient state** (a `watch` channel carrying
`Option<VoiceRoster>`), exactly like the existing Spotify now-playing widget: every
Discord event mutates an in-memory roster, then a full fresh snapshot is published.
The feature **auto-follows** whichever voice channel the streamer is currently in.

## Motivation

The user runs coworking/podcast-style streams and wants viewers to see who is in
the Discord call and who is talking, similar to how the toolkit already ships Twitch
events through SSE to headless overlay UIs. Primary use case is a **persistent voice
roster** widget; because it is headless and feed-driven, the same data can later power
a coworking panel or speaker-spotlight UI without backend changes.

## Why Discord RPC (not the Bot Gateway)

Verified by deep research (19 sources, 25 adversarially-verified claims):

- **Discord RPC (local IPC socket)** natively dispatches `VOICE_STATE_CREATE/UPDATE/DELETE`
  and `SPEAKING_START/SPEAKING_STOP` for a subscribed `channel_id`, and exposes
  `GET_SELECTED_VOICE_CHANNEL`. Requires only an OAuth **user** authorization with the
  `rpc` + `rpc.voice.read` scopes — no bot token, no Gateway.
- **The Bot Gateway gives voice-channel membership only** (`VOICE_STATE_UPDATE`, gated by
  the `GUILD_VOICE_STATES` intent) and **never** delivers speaking events on the main
  gateway — so it cannot satisfy the "who is speaking" requirement without a bot joining
  the call. Rejected.
- The `rpc` scope is whitelist-gated, **but the application owner is auto-whitelisted on
  their own account**, so a self-created **private (non-team)** app works for personal use.
  Team-owned apps are denied `rpc.*` scopes.
- Automating a normal user account outside OAuth2 (self-bots) is forbidden. The RPC-via-
  owned-app flow is the sanctioned path and is **not** a self-bot.
- No Rust crate supports voice RPC (`discord-presence`, `discord-rich-presence` are
  Rich-Presence-only; `discord-ipc` is yanked). We hand-roll a thin IPC client. The
  `obs-discord-voice-overlay` Rust project is a working reference.

## Approach (chosen)

**Hand-rolled RPC/IPC client** as a new `src/infrastructure/discord/` adapter that
produces a domain `VoiceRoster` into a `watch` channel held in `AppState`. The existing
`feed()` controller fans it out over SSE as a new `FeedEvent::VoiceRoster` DTO. This is
structurally identical to how `twitch/eventsub.rs` and the Spotify now-playing observer
already work.

Alternatives rejected: wrapping a crate (none support voice); embedding
`streamkit.discord.com` as a browser source (keeps a dependency on Discord's servers,
not headless, no control over the data).

## Constraints

- **File size:** every new or changed file ≤ **150 LOC**, organized by **vertical
  slicing** (split by feature concern). Large test suites live in sibling `*_tests.rs`
  files rather than inline `#[cfg(test)]`.
- Rust backend, React/TypeScript overlays (Vite), DDD layering per `CONTEXT-MAP.md`.
- Local IPC only works when the Discord desktop client is running.
- Discord app must be **private (non-team)**.

## Data flow

```
 STREAMER                          SYSTEM (streams-toolkit)
  │  toggles "Discord" ON           │  infra/discord: connect $XDG_RUNTIME_DIR/discord-ipc-0
  │ ─────────────────────────────► │  HANDSHAKE {v:1, client_id}
  │                                 │  AUTHENTICATE {access_token}   ← cached token
  │                                 │  GET_SELECTED_VOICE_CHANNEL
  │                                 │  SUBSCRIBE VOICE_CHANNEL_SELECT
  │                                 │  SUBSCRIBE VOICE_STATE_* + SPEAKING_* {channel_id}
  │   overlay paints member cards   │  build VoiceRoster → watch::send(Some(roster))
  │ ◄───────────────────────────── │  feed() → SSE {kind:"voiceRoster", members:[…]}
  │  "nina" starts talking          │  SPEAKING_START {user_id} → roster.set_speaking(true)
  │   nina's ring glows             │  watch::send(snapshot)
  │  hops channel                   │  VOICE_CHANNEL_SELECT → unsub old, refetch, re-sub new
  │  leaves voice / Discord closes  │  watch::send(None) → SSE {channelId:null, members:[]}
```

The roster is **ambient state**: each RPC event mutates an in-memory roster, then a
full fresh snapshot is published on the `watch` channel. A new SSE connection
immediately receives the current snapshot (watch yields its current value first).

## Domain model — `src/domain/voice.rs`

```rust
/// One member present in the followed Discord voice channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceMember {
    pub user_id: String,
    pub display_name: String,        // nick → global_name → username
    pub avatar_url: Option<String>,  // CDN url from avatar hash, else default avatar
    pub speaking: bool,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub server_deaf: bool,
}

/// Ambient state of the voice channel the streamer is currently in.
/// `None` on the watch channel = not in a channel / Discord disconnected.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoiceRoster {
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub members: Vec<VoiceMember>,
}

impl VoiceRoster {
    fn upsert(&mut self, member: VoiceMember);            // VOICE_STATE_CREATE/UPDATE
    fn remove(&mut self, user_id: &str);                  // VOICE_STATE_DELETE
    fn set_speaking(&mut self, user_id: &str, on: bool);  // SPEAKING_START/STOP
}
```

The reducer is pure (no I/O) and is the primary unit-tested unit.

## Feed contract — `src/presentation/http/resources/`

The existing 580-LOC `resources.rs` is split into a `resources/` module (one file per
DTO group) as a targeted improvement, since we must add a variant there and the file
already exceeds the 150-LOC budget. Behavior is unchanged; existing tests move verbatim
into `resources/tests.rs`.

New variant on `FeedEvent` (tagged `kind`):

```rust
pub enum FeedEvent {
    ChatMessage(ChatMessageDto),
    ChatMessageDeleted(ChatMessageDeletedDto),
    StreamEvent { #[serde(flatten)] event: StreamEventDto },
    NowPlaying(NowPlayingDto),
    VoiceRoster(VoiceRosterDto),     // ← new
}
```

Wire shape:

```json
{
  "kind": "voiceRoster",
  "channelId": "123",
  "channelName": "coworking",
  "members": [
    { "userId": "42", "displayName": "nina",
      "avatarUrl": "https://cdn.discordapp.com/avatars/42/abc.png",
      "speaking": true, "selfMute": false, "selfDeaf": false,
      "serverMute": false, "serverDeaf": false }
  ]
}
```

Disconnect / left voice: `{ "kind":"voiceRoster", "channelId":null, "channelName":null, "members":[] }`.

`feed()` adds a `WatchStream` for the roster with `.skip_while(is_none)` (same leading-None
skip as now-playing) and merges it into the SSE stream.

## Wiring (parallels `now_playing`)

- `AppState`: add `voice_roster_tx: watch::Sender<Option<VoiceRoster>>` + `subscribe_voice_roster()`.
- `OverlayState`: add `voice_roster: watch::Receiver<Option<VoiceRoster>>`.
- `feed()`: merge the roster `WatchStream` into the SSE stream.

## IPC mechanics

`ipc.rs` (transport): connect to `$XDG_RUNTIME_DIR/discord-ipc-{0..9}`. Framing:
`[opcode: u32-LE][len: u32-LE][json bytes]`. Opcodes `0=HANDSHAKE, 1=FRAME, 2=CLOSE,
3=PING, 4=PONG`. All socket I/O is behind a small trait so `dispatch.rs` and the reducer
are testable against synthetic frames with no real Discord.

`connect.rs` (orchestrator): handshake → `auth::ensure_token` → AUTHENTICATE →
`GET_SELECTED_VOICE_CHANNEL` → subscribe channel voice/speaking events + `VOICE_CHANNEL_SELECT`
→ publish `Some(roster)` → read loop mutates roster and sends snapshots → on
`VOICE_CHANNEL_SELECT` unsubscribe old / refetch / re-subscribe new → on Stop/disconnect
publish `None`.

## OAuth setup (one-time)

First run with no valid cached token: send `AUTHORIZE {scopes:[rpc, rpc.voice.read]}`
over IPC → Discord desktop shows a native "Authorize <app>?" modal → on approval RPC
returns a `code` → exchange at `discord.com/api/oauth2/token` (client_id + client_secret +
code) → cache `{access_token, refresh_token, expires_at}` to
`~/.config/streams-toolkit/discord_token.json`. Subsequent runs load the cache and refresh
silently when expired.

Setup the user does once: create a **private, non-team** Discord app, copy Client ID +
Client Secret into config:

```toml
[discord]
client_id = "..."
client_secret = "..."   # your own app; local-only, for the one-time token exchange
# optional: socket override; token cache path. Add the token file to .gitignore.
```

## Lifecycle — toggled feature (like Livepix/Overlays)

- `FeatureCommand::EnableDiscord / DisableDiscord`, `AppState.discord_enabled`.
- `discord::spawn(cmd_rx, status_tx, voice_roster_tx, config)` idles until `Start`,
  connects, runs the dispatch loop until `Stop`, publishes `None` on stop/disconnect.
  Same shape as the Overlay server's `spawn()`.
- TUI Services pane gets a **Discord** row + status dot: `Stopped` / `Awaiting authorization` /
  `Running { channel }` / `Error(reason)`. If the client isn't running, the row shows an
  error and the feed roster stays `null`; other widgets keep working.

## File layout (every file ≤150 LOC)

```
src/domain/
  voice.rs                    VoiceMember, VoiceRoster + reducer        ~90
  voice_tests.rs              reducer unit tests                        ~120

src/infrastructure/discord/
  mod.rs                      spawn(), DiscordCommand/Status, surface   ~80
  connect.rs                  orchestrator                              ~140
  ipc.rs                      Unix-socket transport + frame codec       ~130
  ipc_tests.rs                frame round-trip                          ~60
  handshake.rs                HANDSHAKE + AUTHENTICATE                   ~90
  subscribe.rs                SUBSCRIBE/UNSUB + GET_SELECTED_VOICE_CHANNEL ~110
  dispatch.rs                 incoming frame → reducer calls            ~130
  payload.rs                  serde structs for RPC payloads            ~140
  mapping.rs                  payload → VoiceMember (avatar, name)      ~110
  mapping_tests.rs            avatar/name resolution tests              ~90
  auth/mod.rs                 token acquisition entry point             ~70
  auth/oauth.rs               AUTHORIZE flow + code→token exchange      ~120
  auth/token_cache.rs         load/save/expiry/refresh                  ~110
  auth/token_cache_tests.rs   cache round-trip + expiry                 ~70

src/presentation/http/resources/
  mod.rs                      FeedEvent enum + to_json()                ~90
  chat.rs                     ChatMessageDto, FragmentDto, ChatBadgeDto ~120
  stream.rs                   StreamEventDto, SubTierDto                ~120
  now_playing.rs              NowPlayingDto                             ~60
  voice_roster.rs             VoiceRosterDto, VoiceMemberDto  ← new     ~90
  tests.rs                    existing serde-contract tests (moved)     ~140

overlays/src/
  lib/voiceRoster.ts          TS types + parse                          ~40
  hooks/useVoiceRoster.ts     subscribe feed, hold latest roster        ~60
  ui/voice/VoiceRoster.tsx    container (lays out member cards)         ~80
  ui/voice/VoiceMemberCard.tsx single card: avatar, ring, mute/deaf     ~90
```

## Testing strategy

| Layer | Test | How |
|---|---|---|
| `domain/voice.rs` reducer | join/leave/update/speaking → roster | pure |
| `discord/mapping.rs` | avatar URL (normal/animated `a_`/missing→default), name precedence | pure |
| `discord/ipc.rs` | frame encode↔decode round-trip | pure codec |
| `discord/dispatch.rs` | synthetic frames → reducer calls | I/O behind trait |
| `auth/token_cache.rs` | save/load/expiry/refresh | tmp dir |
| `resources/voice_roster.rs` | `kind:"voiceRoster"` serde contract | mirrors existing DTO tests |
| Synthetic dev event | fake roster through the live feed, no Discord | extend `presentation/synthetic.rs` + `/overlay/dev/event/voiceRoster` |

The live `connect.rs` path is verified manually against the real Discord client; all
logic is isolated behind a trait so everything else is unit-testable.

## Docs (code-coupled, per CLAUDE.md)

- New `docs/arch/14-infra-discord.md` (IPC protocol, scopes, OAuth flow, config/env) and
  `docs/arch/15-domain-voice.md` (roster reducer + ubiquitous language).
- New rows in the `CLAUDE.md` "Layer docs" table mapping `src/infrastructure/discord/`
  and `src/domain/voice.rs` to those docs.

## BDD scenarios

```gherkin
Scenario: Roster appears when streamer is in a channel
  Given the Discord feature is connected and I am in "coworking" with nina and bob
  Then the SSE feed emits a voiceRoster frame listing me, nina, bob with avatars and names

Scenario: Speaking flag flips live
  Given the roster shows nina with speaking=false
  When Discord dispatches SPEAKING_START for nina
  Then a new snapshot is emitted with nina.speaking=true; SPEAKING_STOP flips it back

Scenario: Member join/leave updates the snapshot
  When VOICE_STATE_CREATE arrives for a new user, the next snapshot includes them
  When VOICE_STATE_DELETE arrives, the next snapshot omits them

Scenario: Channel hop re-targets the roster
  Given I am in channel A
  When VOICE_CHANNEL_SELECT fires for channel B
  Then the adapter unsubscribes A, fetches B's members, re-subscribes B, emits channelId=B

Scenario: Leaving voice clears the roster (backward-compatible feed)
  When I disconnect from voice (or Discord closes)
  Then a voiceRoster frame with channelId=null and members=[] is emitted
  And chat/stream/nowPlaying frames on the same feed are unaffected

Scenario: New overlay connection gets current roster immediately
  Given I am already in a channel when the overlay connects to /overlay/feed
  Then the watch yields the current snapshot as the first voiceRoster frame

Scenario: First-run authorization
  Given no cached token and valid client_id/secret
  When I enable Discord, AUTHORIZE is sent and Discord shows the authorize modal
  And on approval the token is exchanged, cached, and the roster connects

Scenario: Cached token reused, refreshed when expired
  Given a cached but expired token with a refresh_token
  When I enable Discord, the token is refreshed silently with no modal

Scenario: Discord client not running
  When I enable Discord but no IPC socket is found
  Then the TUI Discord row shows Error and the feed roster stays null; other widgets work

Scenario: Build the UI without Discord
  When I hit /overlay/dev/event/voiceRoster
  Then a synthetic roster snapshot flows through /overlay/feed and the widget renders it

Scenario: Every touched file stays within budget
  Then no new or modified file exceeds 150 LOC, tests living in sibling *_tests.rs files
```

## Open questions (non-blocking)

- Does `rpc.voice.read` alone authenticate, or is the gated `rpc` scope also required at
  runtime? We request both and confirm on the user's machine during first manual test.
- Confirm reconnection behavior: on Discord client restart, re-issue
  `GET_SELECTED_VOICE_CHANNEL` and re-SUBSCRIBE to resync.

## References

- Discord RPC docs: https://docs.discord.com/developers/topics/rpc
- obs-discord-voice-overlay (Rust reference): https://github.com/Eric-D/obs-discord-voice-overlay
- voice-channel-grabber: https://github.com/dichternebel/voice-channel-grabber
- discordjs/RPC (Node reference): https://github.com/discordjs/RPC
- StreamKit overlay: https://support.discord.com/hc/en-us/articles/223415707-Using-Discord-s-OBS-Streamkit-Overlay
```
