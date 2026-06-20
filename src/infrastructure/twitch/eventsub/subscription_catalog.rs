//! The fixed catalog of EventSub subscription types this client registers.

/// Subscription type definition for EventSub registration.
pub(super) struct SubDef {
    pub sub_type: &'static str,
    pub version: &'static str,
    /// Whether the condition uses `broadcaster_user_id` (true) or
    /// `to_broadcaster_user_id` (false, for raids).
    pub uses_broadcaster: bool,
    /// Whether `moderator_user_id` must also be set (channel.follow v2).
    pub needs_moderator: bool,
}

pub(super) const SUBSCRIPTIONS: &[SubDef] = &[
    SubDef {
        sub_type: "channel.follow",
        version: "2",
        uses_broadcaster: true,
        needs_moderator: true,
    },
    SubDef {
        sub_type: "channel.subscribe",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.subscription.message",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.subscription.gift",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.cheer",
        version: "1",
        uses_broadcaster: true,
        needs_moderator: false,
    },
    SubDef {
        sub_type: "channel.raid",
        version: "1",
        uses_broadcaster: false,
        needs_moderator: false,
    },
];
