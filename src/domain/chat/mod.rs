//! Chat — enriched messages received from a Twitch IRC channel, sliced by concern.
//!
//! - [`emote`] — body fragments + native emote references and CDN urls
//! - [`badge`] — chat badges (the `set`/`version` pairs an author wears)
//! - [`message`] — the `ChatMessage` value and its text-only shaping path
//! - [`fragment_split`] — interleaving text runs and emotes into an ordered body
//! - [`moderation`] — single-message delete signals + the broadcast envelope

mod badge;
mod emote;
mod fragment_split;
mod message;
mod moderation;

pub use badge::ChatBadge;
pub use emote::{EmoteSpan, MessageFragment, emote_cdn_url};
pub use message::ChatMessage;
pub use moderation::{ChatMessageDeleted, ChatSignal};

/// Default display colour used when a viewer has never set a Twitch chat colour.
/// Matches Twitch's neutral grey so the Overlay always renders a visible nick.
pub const DEFAULT_CHAT_COLOR: &str = "#9147ff";
