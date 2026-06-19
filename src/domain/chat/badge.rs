//! Native Twitch chat badges and parsing them from the IRC `badges` tag.

use super::ChatMessage;

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

impl ChatMessage {
    /// Parse the raw IRC `badges` tag into `set/version` pairs (M1, pure).
    ///
    /// The tag is a comma-separated list of `set/version` entries (e.g.
    /// `moderator/1,subscriber/12`); an empty tag yields no badges. Malformed
    /// entries (missing the `/`, or an empty set/version) are skipped so a
    /// single bad pair never drops the whole message. The returned badges are
    /// unresolved — the Helix resolver fills their urls afterwards.
    pub fn parse_badges(tag: &str) -> Vec<ChatBadge> {
        tag.split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                if entry.is_empty() {
                    return None;
                }
                let (set, version) = entry.split_once('/')?;
                if set.is_empty() || version.is_empty() {
                    return None;
                }
                Some(ChatBadge::new(set, version))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_badges_splits_set_and_version_pairs() {
        let badges = ChatMessage::parse_badges("moderator/1,subscriber/12");
        assert_eq!(
            badges,
            vec![
                ChatBadge::new("moderator", "1"),
                ChatBadge::new("subscriber", "12"),
            ]
        );
        // Parsed pairs start unresolved; the resolver fills urls later.
        assert!(badges.iter().all(|b| b.url.is_none()));
    }

    #[test]
    fn parse_badges_empty_tag_yields_none() {
        assert!(ChatMessage::parse_badges("").is_empty());
    }

    #[test]
    fn parse_badges_skips_malformed_entries() {
        // `broadcaster` has no `/version`, `/2` has no set, `subscriber/` has no
        // version — all skipped — leaving only the well-formed `vip/1`.
        let badges = ChatMessage::parse_badges("broadcaster,/2,subscriber/,vip/1");
        assert_eq!(badges, vec![ChatBadge::new("vip", "1")]);
    }
}
