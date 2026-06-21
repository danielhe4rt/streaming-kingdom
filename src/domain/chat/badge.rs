//! Native Twitch chat badges — the `set`/`version` value an author wears.

/// One native Twitch chat badge worn by a message author (e.g.
/// `subscriber/12`, `moderator/1`).
///
/// The IRC adapter captures the `set`/`version` pair from the message `badges`
/// tag (M1). The CDN `url` is filled in once by the Helix badge resolver (M2);
/// it stays `None` when the resolver has no entry for the pair (a miss), which
/// the Overlay renders as no image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatBadge {
    /// The badge set id (the `name` in the IRC tag), e.g. `subscriber`.
    pub set: String,
    /// The badge version within the set, e.g. `12`.
    pub version: String,
    /// The resolved CDN image url, or `None` when unresolved (a resolver miss).
    pub url: Option<String>,
}

impl ChatBadge {
    /// Build an unresolved badge from a `set`/`version` pair (M1, IRC tags).
    pub fn new(set: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            set: set.into(),
            version: version.into(),
            url: None,
        }
    }
}
