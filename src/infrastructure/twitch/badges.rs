//! M2 — Twitch chat badge resolver (Helix).
//!
//! At startup we fetch Helix **global** + **channel** chat badges once into a
//! `set/version → url` map (reusing the existing Twitch `client_id` / oauth
//! token / `broadcaster_id`). Resolving a badge worn by a chatter is then a pure
//! lookup: [`BadgeMap::resolve`] returns the channel url when present and falls
//! back to the global one, so a channel badge **overrides** a global one for the
//! same `set/version` key. Native Twitch badges only.

use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::ChatBadge;

const GLOBAL_BADGES_URL: &str = "https://api.twitch.tv/helix/chat/badges/global";
const CHANNEL_BADGES_URL: &str = "https://api.twitch.tv/helix/chat/badges";

/// A resolved `set/version → image url` lookup, split into the two Helix scopes
/// so channel entries can override global ones at resolve time.
///
/// The map is built once at startup and then only read (a pure lookup per
/// message), so it is cheap to `clone` into the IRC adapter.
#[derive(Debug, Clone, Default)]
pub struct BadgeMap {
    /// Twitch-wide badges (`subscriber/0`, `moderator/1`, `vip/1`, …).
    global: HashMap<(String, String), String>,
    /// Channel-specific badges, keyed identically; these win on a tie.
    channel: HashMap<(String, String), String>,
}

impl BadgeMap {
    /// Build a map from already-parsed global + channel `(set, version, url)`
    /// triples. Kept separate from the network fetch so [`resolve`](Self::resolve)
    /// stays pure and unit-testable without HTTP.
    pub fn from_entries(
        global: impl IntoIterator<Item = (String, String, String)>,
        channel: impl IntoIterator<Item = (String, String, String)>,
    ) -> Self {
        let collect = |it: &mut dyn Iterator<Item = (String, String, String)>| {
            it.map(|(set, version, url)| ((set, version), url))
                .collect()
        };
        Self {
            global: collect(&mut global.into_iter()),
            channel: collect(&mut channel.into_iter()),
        }
    }

    /// Resolve one `set/version` pair to its CDN url (PURE).
    ///
    /// A channel entry overrides a global one for the same key; a key present in
    /// neither map is a miss (`None`).
    pub fn resolve(&self, set: &str, version: &str) -> Option<&str> {
        let key = (set.to_string(), version.to_string());
        self.channel
            .get(&key)
            .or_else(|| self.global.get(&key))
            .map(String::as_str)
    }

    /// Return a copy of `badges` with each badge's `url` filled in from this map
    /// (a resolver miss leaves `url` as `None`). Pure; this is what the IRC
    /// adapter calls per message before fan-out.
    pub fn resolve_all(&self, badges: &[ChatBadge]) -> Vec<ChatBadge> {
        badges
            .iter()
            .map(|badge| ChatBadge {
                set: badge.set.clone(),
                version: badge.version.clone(),
                url: self.resolve(&badge.set, &badge.version).map(str::to_string),
            })
            .collect()
    }
}

// ── Helix fetch ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BadgesResponse {
    data: Vec<BadgeSet>,
}

#[derive(Debug, Deserialize)]
struct BadgeSet {
    set_id: String,
    versions: Vec<BadgeVersion>,
}

#[derive(Debug, Deserialize)]
struct BadgeVersion {
    id: String,
    /// Highest-resolution CDN url Twitch returns for the badge.
    image_url_4x: String,
}

/// Flatten a Helix badges payload into `(set, version, url)` triples.
fn flatten(resp: BadgesResponse) -> Vec<(String, String, String)> {
    resp.data
        .into_iter()
        .flat_map(|set| {
            let set_id = set.set_id;
            set.versions
                .into_iter()
                .map(move |v| (set_id.clone(), v.id, v.image_url_4x))
        })
        .collect()
}

/// Fetch Helix global + channel chat badges once and build the [`BadgeMap`].
///
/// Reuses the existing user oauth token / `client_id` / `broadcaster_id`. A
/// failure on either request degrades gracefully to an empty scope rather than
/// blocking startup — the Overlay simply renders fewer (or no) badges.
pub async fn fetch(
    http: &reqwest::Client,
    client_id: &str,
    oauth_token: &str,
    broadcaster_id: &str,
) -> BadgeMap {
    let global = fetch_badges(http, GLOBAL_BADGES_URL, client_id, oauth_token).await;

    let channel_url = format!("{CHANNEL_BADGES_URL}?broadcaster_id={broadcaster_id}");
    let channel = if broadcaster_id.is_empty() {
        tracing::warn!("badges: broadcaster_id empty, skipping channel badges");
        Vec::new()
    } else {
        fetch_badges(http, &channel_url, client_id, oauth_token).await
    };

    tracing::info!(
        global = global.len(),
        channel = channel.len(),
        "badges: resolved Helix chat badges"
    );

    BadgeMap::from_entries(global, channel)
}

/// Fetch a single Helix badges endpoint, returning `(set, version, url)`
/// triples (empty on any error so startup never fails on badges).
async fn fetch_badges(
    http: &reqwest::Client,
    url: &str,
    client_id: &str,
    oauth_token: &str,
) -> Vec<(String, String, String)> {
    let resp = http
        .get(url)
        .header("Authorization", format!("Bearer {oauth_token}"))
        .header("Client-Id", client_id)
        .send()
        .await;

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            tracing::warn!(status = %r.status(), url, "badges: Helix request failed");
            return Vec::new();
        }
        Err(e) => {
            tracing::warn!(error = %e, url, "badges: Helix request errored");
            return Vec::new();
        }
    };

    match resp.json::<BadgesResponse>().await {
        Ok(body) => flatten(body),
        Err(e) => {
            tracing::warn!(error = %e, url, "badges: failed to parse Helix response");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(set: &str, version: &str, url: &str) -> (String, String, String) {
        (set.to_string(), version.to_string(), url.to_string())
    }

    #[test]
    fn resolve_hit_returns_global_url() {
        let map = BadgeMap::from_entries(
            vec![entry("moderator", "1", "https://cdn/global/mod.png")],
            vec![],
        );
        assert_eq!(
            map.resolve("moderator", "1"),
            Some("https://cdn/global/mod.png")
        );
    }

    #[test]
    fn resolve_miss_returns_none() {
        let map = BadgeMap::from_entries(
            vec![entry("moderator", "1", "https://cdn/global/mod.png")],
            vec![],
        );
        // Unknown set and unknown version of a known set are both misses.
        assert_eq!(map.resolve("staff", "1"), None);
        assert_eq!(map.resolve("moderator", "2"), None);
    }

    #[test]
    fn resolve_channel_overrides_global() {
        let map = BadgeMap::from_entries(
            vec![entry("subscriber", "0", "https://cdn/global/sub.png")],
            vec![entry("subscriber", "0", "https://cdn/channel/sub.png")],
        );
        // Same key in both scopes: the channel entry wins.
        assert_eq!(
            map.resolve("subscriber", "0"),
            Some("https://cdn/channel/sub.png")
        );
    }

    #[test]
    fn resolve_falls_back_to_global_when_channel_lacks_key() {
        let map = BadgeMap::from_entries(
            vec![entry("moderator", "1", "https://cdn/global/mod.png")],
            vec![entry("subscriber", "0", "https://cdn/channel/sub.png")],
        );
        // `moderator/1` only exists globally — channel scope falls back to it.
        assert_eq!(
            map.resolve("moderator", "1"),
            Some("https://cdn/global/mod.png")
        );
    }

    #[test]
    fn resolve_all_fills_urls_and_leaves_misses_none() {
        let map = BadgeMap::from_entries(
            vec![entry("moderator", "1", "https://cdn/global/mod.png")],
            vec![entry("subscriber", "12", "https://cdn/channel/sub12.png")],
        );

        let resolved = map.resolve_all(&[
            ChatBadge::new("moderator", "1"),
            ChatBadge::new("subscriber", "12"),
            ChatBadge::new("glitchcon2020", "1"),
        ]);

        assert_eq!(
            resolved[0].url.as_deref(),
            Some("https://cdn/global/mod.png")
        );
        assert_eq!(
            resolved[1].url.as_deref(),
            Some("https://cdn/channel/sub12.png")
        );
        assert_eq!(resolved[2].url, None);
    }

    #[test]
    fn flatten_expands_versions_into_triples() {
        let resp = BadgesResponse {
            data: vec![BadgeSet {
                set_id: "subscriber".into(),
                versions: vec![
                    BadgeVersion {
                        id: "0".into(),
                        image_url_4x: "https://cdn/sub0.png".into(),
                    },
                    BadgeVersion {
                        id: "12".into(),
                        image_url_4x: "https://cdn/sub12.png".into(),
                    },
                ],
            }],
        };
        let triples = flatten(resp);
        assert_eq!(
            triples,
            vec![
                entry("subscriber", "0", "https://cdn/sub0.png"),
                entry("subscriber", "12", "https://cdn/sub12.png"),
            ]
        );
    }
}
