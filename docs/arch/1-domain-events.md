# Domain — events

**Keep in sync:** this file documents `src/domain/events.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

The events module defines the core domain types representing activities that occur during a stream. `StreamEvent` is the fundamental event type produced by Twitch (via EventSub WebSocket) and Livepix webhooks. It carries stream activity data (follows, subscriptions, donations, cheers, raids) and viewer counts. `SubTier` classifies the monetization level of subscriptions.

## Types created

### StreamEvent (enum)

The union type representing all possible stream activities. Each variant carries the minimal data required to capture that activity. Produced by:

- **Twitch EventSub WebSocket client** (`src/infrastructure/twitch/eventsub.rs`): parses incoming EventSub notifications and constructs `StreamEvent` variants from the JSON event payload, then broadcasts them on a channel.
- **Livepix webhook** (`src/infrastructure/livepix/webhook.rs`): receives donation notifications and constructs `StreamEvent::Donation` variants.

#### Follow variant
```rust
Follow { username: String }
```
Represents a new follower. Created when Twitch EventSub emits a `channel.follow` event.

#### Sub variant  
```rust
Sub { username: String, tier: SubTier, months: u32 }
```
Represents a subscription (new or recurring). Created from two EventSub event types:
- `channel.subscribe`: new subscription (defaults `months` to 1)
- `channel.subscription.message`: recurring/renewed subscription (extracts cumulative month count from the event)

#### Donation variant
```rust
Donation { username: String, amount_cents: u64, message: String }
```
Represents a monetary donation via Livepix (payment processor integration). Created by the Livepix webhook handler when a donation is received.

#### GiftSub variant
```rust
GiftSub { username: String, tier: SubTier, total: u32 }
```
Represents a subscription gift (one giver, multiple recipients). Created from the EventSub `channel.subscription.gift` event. If the giver is anonymous, `username` is set to `"Anonymous"`.

#### Cheer variant
```rust
Cheer { username: String, bits: u64, message: String }
```
Represents a Twitch Cheer (bits spent on the channel). Created from the EventSub `channel.cheer` event. If the cheerer is anonymous, `username` is set to `"Anonymous"`.

#### Raid variant
```rust
Raid { from_channel: String, viewers: u32 }
```
Represents an incoming raid (another streamer sending viewers to this channel). Created from the EventSub `channel.raid` event.

#### ViewerCountUpdate variant
```rust
ViewerCountUpdate { count: u32 }
```
Represents a snapshot of the current viewer count. Created by external polling or Twitch updates (producer not yet implemented in current codebase).

### SubTier (enum)

Classification of subscription monetization levels. Produced implicitly when constructing `StreamEvent::Sub` or `StreamEvent::GiftSub` variants by parsing Twitch tier strings.

#### Tier variants
- `Tier1`: $4.99/month tier
- `Tier2`: $9.99/month tier  
- `Tier3`: $24.99/month tier
- `Prime`: included with Amazon Prime membership

## Domain interactions (use-case flows)

### Flow 1: EventSub Follow Event → Broadcast → Stats & Waybar

```
 TWITCH EVENTSUB              EVENTSUB CLIENT              APP STATE          WAYBAR
  │                                  │                        │                 │
  │  channel.follow event            │                        │                 │
  │ ───────────────────────────────► │                        │                 │
  │                                  │  parse_event()         │                 │
  │                                  │  → StreamEvent::Follow │                 │
  │                                  │                        │                 │
  │                                  │  broadcast on channel  │                 │
  │                                  │ ───────────────────►  │                 │
  │                                  │                        │  stats.record() │
  │                                  │                        │  + log AppEvent │
  │                                  │                        │                 │
  │                                  │                        │  dispatch_event()
  │                                  │                        │  broadcasts again
  │                                  │                        │ ───────────────►│
  │                                  │                        │                 │ writes to
  │                                  │                        │                 │ stream_data.json
```

**What gets created at each step:**

1. **Twitch sends**: raw JSON event with `type: "channel.follow"`, `user_name: "alice"`
2. **TwitchClient::parse_event()**: `StreamEvent::Follow { username: "alice" }`
3. **TwitchClient broadcasts**: sends to `broadcast::Sender<StreamEvent>`
4. **AppState::dispatch_event()**: wraps as `AppEvent::Stream(StreamEvent::Follow {...})`, logs it with timestamp, updates `stats.followers_today += 1`
5. **Waybar listener** (via `infrastructure::waybar::event_writer`): converts to JSON, writes `stream_data.json` for Waybar display widget

---

### Flow 2: Sub (Recurring) Event → Stats Update → Presentation (TUI Highlight)

```
 TWITCH EVENTSUB              EVENTSUB CLIENT              APP STATE       PRESENTATION
  │                                  │                        │                 │
  │ channel.subscription.message     │                        │                 │
  │ (recurring sub for 6 months)     │                        │                 │
  │ ───────────────────────────────► │                        │                 │
  │                                  │ parse_event()          │                 │
  │                                  │ → StreamEvent::Sub {   │                 │
  │                                  │    tier: Tier1,        │                 │
  │                                  │    months: 6 }         │                 │
  │                                  │                        │                 │
  │                                  │ broadcast              │                 │
  │                                  │ ───────────────────►  │                 │
  │                                  │                        │ stats update    │
  │                                  │                        │ subs_today += 1 │
  │                                  │                        │                 │
  │                                  │                        │ log_event()     │
  │                                  │                        │ max 200 entries │
  │                                  │                        │                 │ UI renders
  │                                  │                        │ ───────────────►│ highlight
  │                                  │                        │                 │ with fade
```

**What gets created at each step:**

1. **Twitch sends**: `channel.subscription.message` JSON with `tier: "1000"`, `cumulative_months: 6`
2. **parse_event()**: `StreamEvent::Sub { username: "bob", tier: Tier1, months: 6 }`
3. **AppState dispatch_event()**: 
   - Calls `stats.record(&event)` → `stats.subs_today += 1`
   - Wraps as `AppEvent::Stream(StreamEvent::Sub {...})`
   - Logs to `event_log` (bounded to `max_events`)
   - Broadcasts `StreamEvent` on the channel
4. **Presentation layer** (TUI): subscribes to the broadcast, receives the event, renders a highlight entry with the subscriber's name and a fade-out animation

---

### Flow 3: Livepix Donation Webhook → Donation Event → TTS & Highlight

```
 LIVEPIX WEBHOOK              LIVEPIX HANDLER              APP STATE       TTS ENGINE
  │                                  │                        │                 │
  │  POST /livepix/webhook           │                        │                 │
  │  {user: "charlie", amount: 1000} │                        │                 │
  │ ───────────────────────────────► │                        │                 │
  │                                  │ Parse donation         │                 │
  │                                  │ → StreamEvent::        │                 │
  │                                  │    Donation {...}      │                 │
  │                                  │                        │                 │
  │                                  │ send via app_event_tx  │                 │
  │                                  │ ───────────────────►  │                 │
  │                                  │                        │ log & broadcast │
  │                                  │                        │                 │ reads donation
  │                                  │                        │ ───────────────►│ text, queues
  │                                  │                        │                 │ TTS synthesis
```

**What gets created at each step:**

1. **Livepix POST**: HTTP request with donation JSON payload (`user: "charlie"`, `amount_cents: 1000`, `message: "Love your content!"`)
2. **Livepix webhook handler**: parses request, constructs `StreamEvent::Donation { username: "charlie", amount_cents: 1000, message: "Love your content!" }`
3. **sends via event_tx (mpsc)**: bridges into AppState's event channel
4. **AppState::log_event()**: wraps as `AppEvent::Stream(...)`, adds to event log with elapsed timestamp
5. **TTS worker** (via elevenlabs channel): reads donation message, synthesizes speech and plays on stream

---

### Flow 4: Raid Event → Broadcast → Stats & Notification

Raid events from the EventSub `channel.raid` subscription type:

```
 TWITCH EVENTSUB              EVENTSUB CLIENT              APP STATE
  │                                  │                        │
  │ channel.raid event               │                        │
  │ {from: "alice_plays", viewers:42}│                        │
  │ ───────────────────────────────► │                        │
  │                                  │ parse_event()          │
  │                                  │ → StreamEvent::Raid {  │
  │                                  │    from_channel: "...  │
  │                                  │    viewers: 42 }       │
  │                                  │                        │
  │                                  │ broadcast              │
  │                                  │ ───────────────────►  │
  │                                  │                        │ log_event()
  │                                  │                        │ (no stats update)
```

**What gets created:**

1. **Twitch EventSub**: `channel.raid` event with raid metadata
2. **parse_event()**: `StreamEvent::Raid { from_channel: "alice_plays", viewers: 42 }`
3. **AppState::log_event()**: wraps as `AppEvent::Stream(...)`, adds timestamped entry to the bounded event log
4. Note: unlike Follow/Sub/GiftSub, raids do NOT update stats (no stats.record() for raid events in the domain/stats.rs pattern matcher)

---

## Event Dispatch Pattern

All `StreamEvent`s flow through this pattern once created:

1. **Producer** (Twitch EventSub, Livepix webhook) creates the variant
2. **TwitchClient or webhook handler** broadcasts/sends to AppState
3. **AppState::dispatch_event()** (called by main.rs or infrastructure layer):
   - Calls `stats.record(event)` to update counters
   - Wraps as `AppEvent::Stream(event)` and logs with timestamp
   - Broadcasts the `StreamEvent` on the broadcast channel
   - Subscribers receive it: Waybar writer, TUI presentation, TTS processor, etc.

Each consumer (presentation, waybar, elevenlabs) is independent and non-blocking: they pull from broadcast receivers that were created at startup.

