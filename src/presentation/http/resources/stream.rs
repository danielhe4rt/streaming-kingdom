//! Stream-event feed DTO (donation / sub / raid …) and the sub-tier enum.
//!
//! A presentation DTO mirroring the domain [`StreamEvent`], so the camelCase
//! wire contract the React Overlays consume lives in the presentation layer and
//! the domain type stays free of serialization concerns. The inner `type`
//! discriminant selects the Footer Bar's Alert template; the outer `kind` on
//! `FeedEvent` selects the stream-event branch.

use serde::Serialize;

use crate::domain::{StreamEvent, SubTier};

/// A stream event as it appears on the Overlay Feed (donation / sub / raid …).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEventDto {
    Follow {
        username: String,
    },
    Sub {
        username: String,
        tier: SubTierDto,
        months: u32,
    },
    Donation {
        username: String,
        // serde's `rename_all` is not applied across `#[serde(flatten)]`, so the
        // camelCase wire names are spelled out explicitly on the multi-word
        // fields the Footer Bar reads.
        #[serde(rename = "amountCents")]
        amount_cents: u64,
        message: String,
    },
    GiftSub {
        username: String,
        tier: SubTierDto,
        total: u32,
    },
    Cheer {
        username: String,
        bits: u64,
        message: String,
    },
    Raid {
        #[serde(rename = "fromChannel")]
        from_channel: String,
        viewers: u32,
    },
    ViewerCountUpdate {
        count: u32,
    },
}

/// The sub tier as the React side consumes it (camelCase variants).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SubTierDto {
    Tier1,
    Tier2,
    Tier3,
    Prime,
}

impl From<&SubTier> for SubTierDto {
    fn from(tier: &SubTier) -> Self {
        match tier {
            SubTier::Tier1 => SubTierDto::Tier1,
            SubTier::Tier2 => SubTierDto::Tier2,
            SubTier::Tier3 => SubTierDto::Tier3,
            SubTier::Prime => SubTierDto::Prime,
        }
    }
}

impl From<&StreamEvent> for StreamEventDto {
    fn from(event: &StreamEvent) -> Self {
        match event {
            StreamEvent::Follow { username } => StreamEventDto::Follow {
                username: username.clone(),
            },
            StreamEvent::Sub {
                username,
                tier,
                months,
            } => StreamEventDto::Sub {
                username: username.clone(),
                tier: tier.into(),
                months: *months,
            },
            StreamEvent::Donation {
                username,
                amount_cents,
                message,
            } => StreamEventDto::Donation {
                username: username.clone(),
                amount_cents: *amount_cents,
                message: message.clone(),
            },
            StreamEvent::GiftSub {
                username,
                tier,
                total,
            } => StreamEventDto::GiftSub {
                username: username.clone(),
                tier: tier.into(),
                total: *total,
            },
            StreamEvent::Cheer {
                username,
                bits,
                message,
            } => StreamEventDto::Cheer {
                username: username.clone(),
                bits: *bits,
                message: message.clone(),
            },
            StreamEvent::Raid {
                from_channel,
                viewers,
            } => StreamEventDto::Raid {
                from_channel: from_channel.clone(),
                viewers: *viewers,
            },
            StreamEvent::ViewerCountUpdate { count } => {
                StreamEventDto::ViewerCountUpdate { count: *count }
            }
        }
    }
}
