//! Message-body fragments and native Twitch emote references.

/// One ordered piece of a [`ChatMessage`](super::ChatMessage) body.
///
/// A body is a flat, ordered stream of plain-text runs interleaved with single
/// native Twitch emotes. The TUI renders only the text runs; the Coworking Overlay
/// renders the text runs as spans and the emotes as inline `<img>`s in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageFragment {
    /// A run of plain text.
    Text(String),
    /// A single native Twitch emote: its id and the CDN url to render.
    Emote { id: String, url: String },
}

/// One native Twitch emote occurrence in a message body (M1).
///
/// Carries the emote id and the half-open `[start, end)` **character** range it
/// occupies in the original message text — exactly the shape the IRC `emotes`
/// tag yields (already char-indexed and de-bugged by `twitch-irc`). Used by
/// [`ChatMessage::split_fragments`](super::ChatMessage::split_fragments) to
/// splice emotes into the ordered body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmoteSpan {
    /// The Twitch emote id (e.g. `25` for Kappa, or `301512758_TK`).
    pub id: String,
    /// Inclusive char index where the emote begins.
    pub start: usize,
    /// Exclusive char index where the emote ends.
    pub end: usize,
}

impl EmoteSpan {
    /// Build a span from an emote id and its half-open char range.
    pub fn new(id: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            id: id.into(),
            start,
            end,
        }
    }
}

/// Derive the CDN url for a native Twitch emote from its id (no API call).
///
/// Uses the Helix emote CDN v2 layout, requesting the static (`default`) dark
/// 3.0 (largest) variant so the Overlay always has a crisp image to scale down.
pub fn emote_cdn_url(id: &str) -> String {
    format!("https://static-cdn.jtvnw.net/emoticons/v2/{id}/default/dark/3.0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emote_cdn_url_uses_helix_v2_layout() {
        assert_eq!(
            emote_cdn_url("25"),
            "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/3.0"
        );
        // Channel-points modified emote ids are non-numeric but format the same.
        assert_eq!(
            emote_cdn_url("301512758_TK"),
            "https://static-cdn.jtvnw.net/emoticons/v2/301512758_TK/default/dark/3.0"
        );
    }
}
