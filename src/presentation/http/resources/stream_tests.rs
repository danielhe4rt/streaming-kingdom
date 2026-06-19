//! Serde-contract tests for `streamEvent` Feed DTOs (Coworking Overlay Footer Bar
//! Alerts). Sliced out of the former aggregate `tests.rs` (by-concern, file-size
//! rule); wired in by `#[path = "stream_tests.rs"] mod stream_tests;` in `mod.rs`.

use super::*;

#[test]
fn donation_serializes_to_feed_contract() {
    use crate::domain::StreamEvent;

    let feed = FeedEvent::stream(&StreamEvent::Donation {
        username: "danielhe4rt".into(),
        amount_cents: 500,
        message: "vai rust!".into(),
    });
    let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

    // The domain StreamEvent is flattened inline: the outer `kind` selects
    // the Footer Bar's stream-event branch, the inner `type` the template.
    assert_eq!(value["kind"], "streamEvent");
    assert_eq!(value["type"], "donation");
    assert_eq!(value["username"], "danielhe4rt");
    assert_eq!(value["amountCents"], 500);
    assert_eq!(value["message"], "vai rust!");
}

#[test]
fn raid_and_sub_carry_their_type_discriminant() {
    use crate::domain::{StreamEvent, SubTier};

    let raid = serde_json::to_value(FeedEvent::stream(&StreamEvent::Raid {
        from_channel: "ferris".into(),
        viewers: 42,
    }))
    .unwrap();
    assert_eq!(raid["kind"], "streamEvent");
    assert_eq!(raid["type"], "raid");
    assert_eq!(raid["fromChannel"], "ferris");
    assert_eq!(raid["viewers"], 42);

    let sub = serde_json::to_value(FeedEvent::stream(&StreamEvent::Sub {
        username: "viewer".into(),
        tier: SubTier::Tier1,
        months: 3,
    }))
    .unwrap();
    assert_eq!(sub["kind"], "streamEvent");
    assert_eq!(sub["type"], "sub");
    assert_eq!(sub["months"], 3);
}

#[test]
fn stream_event_to_json_emits_kind_and_type() {
    use crate::domain::StreamEvent;

    let json = FeedEvent::stream(&StreamEvent::Donation {
        username: "u".into(),
        amount_cents: 100,
        message: String::new(),
    })
    .to_json();
    assert!(json.contains("\"kind\":\"streamEvent\""), "got: {json}");
    assert!(json.contains("\"type\":\"donation\""), "got: {json}");
}
