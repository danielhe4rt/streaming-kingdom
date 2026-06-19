//! Translation of raw EventSub notification payloads into domain `StreamEvent`s.

use serde_json::Value;

use crate::domain::{StreamEvent, SubTier};

/// Parse a Twitch EventSub notification into a `StreamEvent`.
pub(super) fn parse_event(sub_type: &str, event: &Value) -> Option<StreamEvent> {
    match sub_type {
        "channel.follow" => {
            let username = event["user_name"].as_str()?.to_string();
            Some(StreamEvent::Follow { username })
        }

        "channel.subscribe" => {
            let username = event["user_name"].as_str()?.to_string();
            let tier = parse_tier(event["tier"].as_str()?);
            Some(StreamEvent::Sub {
                username,
                tier,
                months: 1,
            })
        }

        "channel.subscription.message" => {
            let username = event["user_name"].as_str()?.to_string();
            let tier = parse_tier(event["tier"].as_str()?);
            let months = event["cumulative_months"].as_u64().unwrap_or(1) as u32;
            Some(StreamEvent::Sub {
                username,
                tier,
                months,
            })
        }

        "channel.subscription.gift" => {
            let username = if event["is_anonymous"].as_bool() == Some(true) {
                "Anonymous".to_string()
            } else {
                event["user_name"]
                    .as_str()
                    .unwrap_or("Anonymous")
                    .to_string()
            };
            let tier = parse_tier(event["tier"].as_str().unwrap_or("1000"));
            let total = event["total"].as_u64().unwrap_or(1) as u32;
            Some(StreamEvent::GiftSub {
                username,
                tier,
                total,
            })
        }

        "channel.cheer" => {
            let username = if event["is_anonymous"].as_bool() == Some(true) {
                "Anonymous".to_string()
            } else {
                event["user_name"]
                    .as_str()
                    .unwrap_or("Anonymous")
                    .to_string()
            };
            let bits = event["bits"].as_u64().unwrap_or(0);
            let message = event["message"].as_str().unwrap_or("").to_string();
            Some(StreamEvent::Cheer {
                username,
                bits,
                message,
            })
        }

        "channel.raid" => {
            let from_channel = event["from_broadcaster_user_name"].as_str()?.to_string();
            let viewers = event["viewers"].as_u64().unwrap_or(0) as u32;
            Some(StreamEvent::Raid {
                from_channel,
                viewers,
            })
        }

        _ => {
            tracing::debug!(sub_type, "unhandled subscription type");
            None
        }
    }
}

/// Convert Twitch tier string ("1000", "2000", "3000") to `SubTier`.
fn parse_tier(tier: &str) -> SubTier {
    match tier {
        "2000" => SubTier::Tier2,
        "3000" => SubTier::Tier3,
        _ => SubTier::Tier1,
    }
}
