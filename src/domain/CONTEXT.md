# Context: Domain

**Layer:** `src/domain/` · **Triage label:** `area:domain`

The core language of the system — pure types with no I/O. Modules: `events`, `chat`,
`commands`, `stats`, `app_event`.

## Glossary

### Stream Events

**StreamEvent**: A tagged union representing all possible stream activities (follows, subscriptions, donations, cheers, raids, viewer count updates) with minimal domain data attached.
_Avoid_: Event, DomainEvent

**SubTier**: An enumeration classifying subscription monetization levels (Tier1, Tier2, Tier3, Prime).
_Avoid_: Tier, SubscriptionTier

**Follow**: A viewer's new follow event, always representing a first-time follow with no repetition within a session.

**Sub**: A viewer's new subscription, parsed from Twitch EventSub and carrying tier and month count (1 for new, N for recurring).

**GiftSub**: A subscription gifted by one viewer to multiple others, represented with giver name, tier, and recipient count.

**Donation**: A monetary gift from a viewer, processed via Livepix, carrying amount and optional message text.

**Cheer**: A viewer's bits donation, representing in-platform currency spent on the stream.

**Raid**: An incoming viewer migration from another streamer's channel, carrying source channel name and viewer count; treated as an information event, not a monetization event.

**ViewerCountUpdate**: The current live viewer count at the stream, replaced (not accumulated) by each new platform update.

### Unified Event Log

**AppEvent**: A domain type that represents any noteworthy occurrence in the system, wrapping stream events, feature toggles, infrastructure status changes, and diagnostic messages into a single enum.
_Avoid_: SystemEvent, LogEntry, Event

**AppEventEntry**: A timestamped container that pairs an AppEvent with its elapsed Duration since app startup, enabling temporal diagnostics in the event log.
_Avoid_: LogLine, TimestampedEvent, Entry

**EventGroup**: A categorical bucket (Stream, Privacy, System, Hyprland, Livepix, Chat) used to filter which AppEvent variants appear in the TUI event log pane.
_Avoid_: EventCategory, EventType, Group

### Chat

**ChatMessage**: A struct representing one user message received from a Twitch IRC channel.
Carries username, channel, a stable **msgId**, the author's **display colour**, their **badges**,
and the text as ordered **MessageFragment**s (text or emote) for parity rendering on the Chat Overlay.
_Avoid_: IrcMessage, TwitchMessage, ChatEvent

**MessageFragment**: One ordered piece of a ChatMessage's body — either a run of text or a single
native Twitch emote (id → CDN url). Lets the TUI render plain text and the Overlay render `<img>` emotes
from the same data.
_Avoid_: token, part, chunk

**ChatBadge**: A `set/version` pair on a ChatMessage (e.g. `subscriber/12`) whose image url is
resolved once at startup from the Helix badge maps (global + channel).
_Avoid_: icon, flair

**ChatMessageDeleted**: A moderation event (from IRC `CLEARMSG`) instructing consumers to remove a
single previously-shown message by **msgId**. Carried on the Overlay Feed so the Chat Overlay can
drop the DOM node. (`CLEARCHAT` timeout/ban handling is deferred past v1.)
_Avoid_: clear, purge, ban event

### Media Player (Now Playing)

**NowPlaying**: The currently-playing track as ambient **state** (not an event): title, artist, album,
an optional cover-art url, and a **PlaybackStatus**. Latest-value-wins — there is at most one NowPlaying
"now". Rendered by the Coworking Overlay's Now Playing widget and shown in the TUI's Spotify Service row.
_Avoid_: Track, Song, MediaState, Metadata (reserve "metadata" for the raw MPRIS payload the adapter parses).

**PlaybackStatus**: Whether the player is `Playing`, `Paused`, or `Stopped`. Drives the widget's
animation (disc/EQ active while Playing, frozen while Paused) and visibility (Stopped / no player →
the widget falls back to its placeholder).
_Avoid_: state (too generic), PlayState.

**MediaPlayer**: The external source of NowPlaying — an MPRIS player on D-Bus (v1 targets Spotify
specifically). The toolkit only *observes* it; it never controls playback.
_Avoid_: Spotify (the concept is the MPRIS player; Spotify is the v1 instance).

### Statistics

**StreamStats**: A mutable aggregate of three running counters (viewer_count, followers_today, subs_today) that tracks engagement metrics for a single stream session.
_Avoid_: stream metrics, session counters

**followers_today**: The cumulative count of Follow events received since the current stream session started, incremented once per unique follow event.
_Avoid_: follow count, new followers

**subs_today**: The cumulative count of subscription events (both individual subscriptions and gift subscriptions) received since the current stream session started.
_Avoid_: subscription count, total subs

**viewer_count**: The current live viewer count at the stream, updated by incoming ViewerCountUpdate events (replaced, not accumulated).
_Avoid_: concurrent viewers, live audience size

**session-scoped**: Counters and state that exist only for the lifetime of a single stream session and reset to zero when a new session begins.
_Avoid_: stream-lifetime, temporary

**record**: The operation that pattern-matches an incoming StreamEvent and mutates the corresponding counter in StreamStats (increment for follows/subs, replace for viewer count).
_Avoid_: update, aggregate

### Feature Control

**FeatureCommand**: A domain type representing a user intent to toggle a system feature (Waybar, Privacy, Alerts, or Livepix) on or off, created by the TUI and consumed by the application state machine.
_Avoid_: toggle, instruction, action

**feature toggle**: A boolean state flag (waybar_enabled, privacy_enabled, alerts_enabled, livepix_enabled) that determines whether a subsystem is active.
_Avoid_: switch, flag, gate

**command channel**: An MPSC channel carrying FeatureCommands from the presentation layer to the application layer for asynchronous processing.
_Avoid_: queue, pipe, buffer

## Relationships

- A **StreamEvent** wraps one specific activity (follow, sub, donation, cheer, raid, or viewer count update) and carries minimal domain data (username, amount, tier, etc.).
- An **AppEvent** wraps one **StreamEvent** (via variant Stream) when the event originates from Twitch, or wraps other system events (Privacy, Hyprland, Chat, System, Livepix); used for unified logging.
- An **AppEventEntry** pairs one **AppEvent** with its elapsed timestamp since app startup.
- An **AppEvent** has exactly one **EventGroup** classification via the group() method, enabling TUI filtering.
- A **SubTier** classifies the monetization tier of a subscription within **Sub** and **GiftSub** variants.
- A **StreamStats** is owned by **AppState** (one-to-one) and mutates on each **StreamEvent** that matches Follow, Sub, GiftSub, or ViewerCountUpdate.
- A **StreamStats** is read by presentation modules (TUI, Waybar) but never written to directly by external consumers.
- A **ChatMessage** is wrapped by one **AppEvent** (the ChatMessage variant).
- A **FeatureCommand** is sent via a **command channel** (MPSC) to update one **feature toggle** in AppState.
- Each **FeatureCommand** variant (EnableWaybar, DisableWaybar, etc.) maps 1:1 to a **feature toggle** boolean.
- When a **FeatureCommand** is applied, it emits an **AppEvent** variant FeatureToggled, which is logged in the unified event log.
- A **NowPlaying** is ambient **state** carried on a `watch` channel (latest-value), unlike **StreamEvent** / **ChatMessage** which are discrete and broadcast. It is *not* wrapped in an **AppEvent** — it is observed live by the Overlay Feed and the TUI, not logged to history.

## Example Dialogue

**Developer**: "How do we track live engagement during a stream and ensure the TUI shows the right events?"

**Domain Expert**: "StreamStats maintains three session-scoped counters: viewer_count, followers_today, and subs_today. When the application receives a Follow or Sub event (a StreamEvent variant), it calls StreamStats::record() to increment the appropriate counter. That StreamEvent is then wrapped in an AppEvent::Stream, paired with an elapsed timestamp to create an AppEventEntry, and appended to the event log."

**Developer**: "So the event log shows everything—streams, chat, window changes, feature toggles?"

**Domain Expert**: "Yes, AppEvent is a union of all system occurrences. Each one has an EventGroup so the TUI can filter: toggle show_stream to false, and Raid and Follow events disappear from the view, but the counters in StreamStats are unaffected—they're independent of the log."

**Developer**: "And ChatMessage? That also becomes an AppEvent?"

**Domain Expert**: "Exactly. ChatMessage is the raw data carrier from the IRC client. The application wraps it as AppEvent::ChatMessage, logs it with a timestamp, and the TUI renders it if show_chat is true. The ChatMessage itself never updates stats—it's just a record of what was said."

**Developer**: "What about toggling features from the TUI?"

**Domain Expert**: "That creates a FeatureCommand (e.g., EnableWaybar) sent through the command channel. AppState applies it, updates the waybar_enabled feature toggle, and emits an AppEvent::FeatureToggled, which gets logged. The TUI checks the new state and reflects it in the UI."

## Flagged Ambiguities

- **Raid event semantics**: In the codebase, Raid only updates the event log, NOT stats counters (stats.rs has no match arm for it). Clarify whether raids are intentionally information-only events (no monetization impact) or a gap to fill in subs_today/followers_today counting.
- **Feature toggle vs. feature flag**: In this codebase, 'feature toggle' refers specifically to the runtime boolean state (waybar_enabled, etc.), while 'feature flag' is avoided to prevent confusion with CLI flags or feature-gate macros. Resolution: use **feature toggle** for the state boolean, not 'feature flag'.

## Decisions

See `docs/adr/` (system-wide) and `src/domain/docs/adr/` (this context, when present).
