# Domain — chat

**Keep in sync:** this file documents `src/domain/chat.rs`. Whenever that code changes, update this doc in the same change. (See CLAUDE.md → Layer docs.)

## What this models

The **chat module** represents a message originating from a Twitch IRC channel. It is the raw, unprocessed representation of a user's chat input as received from the Twitch IRC gateway. A `ChatMessage` captures the sender, message text, and the channel where the message was posted.

## Types created

### ChatMessage

A single message from a user in a Twitch chat channel.

**Fields:**
- `username: String` — the display name of the user who sent the message.
- `text: String` — the raw message text as typed by the user.
- `channel: String` — the login name of the Twitch channel where the message was posted.

**What produces it:** The Twitch IRC client (`src/infrastructure/twitch/irc.rs`) receives a raw `ServerMessage::Privmsg` from the Twitch IRC server, parses its fields (`sender.name`, `message_text`, `channel_login`), and constructs a `ChatMessage` to send downstream via MPSC channel.

## Domain interactions (use-case flows)

### Flow: Raw chat message → AppEvent → TUI display

This flow shows how a chat message travels through the system from the IRC server to the user interface.

```
 IRC Server                            Application
  │                                        │
  │  [Twitch IRC ServerMessage::Privmsg]  │
  │ ──────────────────────────────────►  │
  │                                        │
  │                                        │ ChatClient: parse Privmsg
  │                                        │ extract: username, text, channel
  │                                        │ construct: ChatMessage
  │                                        │
  │                                        │ send via mpsc: ChatMessage
  │ ◄─ Infrastructure (irc.rs)            │
  │    forwards to application layer ────►│
  │                                        │
  │                                        │ Application: receive ChatMessage
  │                                        │ wrap into AppEvent::ChatMessage
  │                                        │ append to event_log (AppEventEntry)
  │                                        │ display in TUI event list
  │                                        │
```

**Step-by-step:**

1. **IRC Message Received** (infrastructure/twitch/irc.rs)
   - Twitch IRC server sends a `ServerMessage::Privmsg` containing:
     - `msg.sender.name` (username)
     - `msg.message_text` (the chat message)
     - `msg.channel_login` (the channel)

2. **ChatMessage Created** (domain/chat.rs)
   - The IRC client constructs a `ChatMessage` with the three fields extracted from the server message.

3. **Sent to Application** (infrastructure/twitch/irc.rs → application)
   - The `ChatMessage` is sent through an MPSC channel (`mpsc::Sender<ChatMessage>`).
   - The application layer receives it and wraps it into an `AppEvent::ChatMessage { username, text }` variant.

4. **Logged and Rendered**
   - The application appends the event to its event log (`Vec<AppEventEntry>`).
   - The presentation layer renders the event in the TUI event history or stats panel.
   - The TUI maintains a rolling window of recent events (with configurable max count).

**Note on channel field:** The `ChatMessage` struct includes a `channel` field that captures which Twitch channel the message came from. This is preserved during construction but is not currently used in the `AppEvent::ChatMessage` variant (which only stores `username` and `text`). This is acceptable for single-channel deployments; if multi-channel support is needed in the future, the channel could be restored to the `AppEvent` variant.

## Design rationale

- **Simplicity:** `ChatMessage` is a flat struct with no methods. It serves purely as a data carrier from IRC → application.
- **Separation of concerns:** IRC-specific parsing happens in the infrastructure layer; the domain struct has no knowledge of Twitch protocol details.
- **Event bridge:** Although `ChatMessage` is defined in the domain, it is not the primary event abstraction. The application wraps it into `AppEvent` (defined in `app_event.rs`) so that all system events (stream events, privacy events, chat, Hyprland window events, etc.) flow through a unified `AppEvent` enum.
