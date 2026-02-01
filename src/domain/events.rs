use serde::Serialize;

// ---------------------------------------------------------------------------
// Stream events – one producer (stream listener), N consumer modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    Follow {
        username: String,
    },
    Sub {
        username: String,
        tier: SubTier,
        months: u32,
    },
    Donation {
        username: String,
        amount_cents: u64,
        message: String,
    },
    GiftSub {
        username: String,
        tier: SubTier,
        total: u32,
    },
    Cheer {
        username: String,
        bits: u64,
        message: String,
    },
    Raid {
        from_channel: String,
        viewers: u32,
    },
    ViewerCountUpdate {
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SubTier {
    Tier1,
    Tier2,
    Tier3,
    Prime,
}
