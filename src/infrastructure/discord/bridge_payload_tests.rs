// BridgeSnapshot deserialization + mapping to the domain VoiceRoster.

use super::*;

#[test]
fn full_snapshot_maps_to_roster() {
    let json = r#"{
        "channelId": "123",
        "channelName": "coworking",
        "members": [
            { "userId": "42", "displayName": "nina",
              "avatarUrl": "https://cdn.discordapp.com/avatars/42/abc.png",
              "speaking": true, "selfMute": false, "selfDeaf": false,
              "serverMute": false, "serverDeaf": true }
        ]
    }"#;
    let roster = serde_json::from_str::<BridgeSnapshot>(json).unwrap().into_roster();

    assert_eq!(roster.channel_id.as_deref(), Some("123"));
    assert_eq!(roster.channel_name.as_deref(), Some("coworking"));
    assert_eq!(roster.members.len(), 1);
    let m = &roster.members[0];
    assert_eq!(m.user_id, "42");
    assert_eq!(m.display_name, "nina");
    assert!(m.speaking);
    assert!(m.server_deaf);
    assert!(!m.self_mute);
}

#[test]
fn empty_snapshot_is_a_cleared_roster() {
    let roster = serde_json::from_str::<BridgeSnapshot>("{}").unwrap().into_roster();
    assert_eq!(roster, VoiceRoster::default());
}

#[test]
fn member_defaults_fill_missing_fields() {
    let json = r#"{ "channelId": "1", "members": [ { "userId": "7" } ] }"#;
    let roster = serde_json::from_str::<BridgeSnapshot>(json).unwrap().into_roster();
    let m = &roster.members[0];
    assert_eq!(m.user_id, "7");
    assert_eq!(m.display_name, "");
    assert_eq!(m.avatar_url, None);
    assert!(!m.speaking && !m.self_mute && !m.self_deaf);
}
