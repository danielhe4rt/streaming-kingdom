# Domain — voice (Discord voice roster)

**Keep in sync:** this file documents `src/domain/voice.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

The voice module defines the domain types representing **who is present in the Discord voice channel the streamer is currently in, and who is talking**. `VoiceMember` is the value object for one person in the call; `VoiceRoster` is the ambient snapshot of the whole channel.

Like `NowPlaying` (see `docs/arch/12-domain-media-player.md`), the roster is **observed STATE, not a domain event**. It is latest-value-wins: there is at most one current roster, it is never replayed as history, and it is **never wrapped in `AppEvent`**. It rides a `tokio::sync::watch::Sender<Option<VoiceRoster>>` held by `AppState`. `None` on that channel means *not in a voice channel / Discord disconnected*. The source is the Discord **CDP bridge** (`src/infrastructure/discord/`, see `docs/arch/14-infra-discord.md`), which builds a **full snapshot per change** from Vesktop's Flux stores — so the toolkit replaces the roster wholesale rather than mutating it incrementally.

## Types created

### VoiceMember (struct)

One member present in the followed voice channel.

```rust
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
```

- `user_id`: Discord user snowflake; the identity key.
- `display_name`: resolved by precedence guild `nick` → `global_name` → `username` (done in the bridge's `inject.js`).
- `avatar_url`: CDN URL derived from the avatar hash; `None` → consumers fall back to a default avatar.
- `speaking`: live indicator driven by the `SPEAKING` Flux event (`speakingFlags` `1`/`0`).
- `self_mute` / `self_deaf`: the member muted/deafened themselves.
- `server_mute` / `server_deaf`: a moderator muted/deafened them server-side.

### VoiceRoster (struct)

Ambient state of the channel the streamer is currently in.

```rust
pub struct VoiceRoster {
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub members: Vec<VoiceMember>,
}
```

- `channel_id` / `channel_name`: identify the followed channel. Both `None` (and `members` empty) is the "not in a channel" state, which on the `watch` channel is published as `None`.
- `members`: every person currently in the channel, including the streamer.

`VoiceRoster` is `Default` (empty roster) and `PartialEq`/`Eq` so snapshots can be compared and deduplicated by the `watch` channel.

## Snapshot model

The roster is **replaced, never mutated in place**. On every relevant Flux event (`SPEAKING`, `VOICE_STATE_UPDATES`, `VOICE_CHANNEL_SELECT`), the bridge's `inject.js` rebuilds the entire roster from Vesktop's voice stores and pushes it; the toolkit deserializes it (`bridge_payload.rs`) and publishes a fresh `Some(VoiceRoster)`. Leaving voice (no selected channel) pushes a cleared roster, published as `None`. There is no incremental reducer — the full-snapshot-per-change model keeps the domain type a plain value object.

## Domain interactions (use-case flows)

### Flow 1: member speaks → new snapshot → published

```
 VESKTOP (inject.js)            BRIDGE (bridge.rs)        APP STATE (watch)    CONSUMERS
  │                                  │                         │                  │
  │  Flux SPEAKING {speakingFlags:1} │                         │                  │
  │  rebuild snapshot (speaking++)   │                         │                  │
  │ ──ws snapshot──────────────────► │                         │                  │
  │                                  │  send Some(roster)      │                  │
  │                                  │ ───────────────────────►│ watch latest-win │
  │                                  │                         │ ────────────────►│ Overlay Feed → FeedEvent::VoiceRoster
  │                                  │                         │ ────────────────►│ TUI Discord row
```

`speakingFlags: 0` runs the identical path with `speaking` cleared for that member.

### Flow 2: leave voice / Vesktop closes → None → cleared roster

```
 VESKTOP (inject.js)            BRIDGE (bridge.rs)        APP STATE (watch)     CONSUMERS
  │  left voice (no channel)         │                         │                  │
  │ ──ws {channelId:null,members:[]}►│                         │                  │
  │   (or WS disconnects)            │  send None              │                  │
  │                                  │ ───────────────────────►│ ────────────────►│ Overlay Feed → {channelId:null, members:[]}
  │                                  │                         │ ────────────────►│ TUI Discord row → Stopped/Error
```

When the WS connection itself drops (Vesktop closed), `bridge.rs` publishes `None` directly. Chat / stream / now-playing frames on the same feed are unaffected.

## State vs. Event pattern

`VoiceRoster` follows the same observed-state pattern as `NowPlaying`, and deliberately **not** the `StreamEvent` / `AppEvent` dispatch pattern used for discrete occurrences (follows, subs, chat messages):

1. **Producer** (Discord CDP bridge) builds a full roster snapshot per change.
2. **Publishes** `Some(VoiceRoster)` (or `None`) on a `tokio::sync::watch::Sender` held by `AppState` (`voice_roster_tx`).
3. **Consumers** read the latest value: the Overlay Feed (`feed()`) merges a `WatchStream` with a leading-`None` skip (same as now-playing) and emits `FeedEvent::VoiceRoster`; the TUI reads it for the Discord row.

Because `watch` is latest-value-wins, any overlay that connects mid-call immediately sees the current roster — a `broadcast` channel (no replay of the last value) would not, which is why `watch` is used.
