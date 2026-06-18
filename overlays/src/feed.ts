// ---------------------------------------------------------------------------
// Overlay Feed DTOs (M3)
//
// These mirror the Rust serde DTOs serialized onto `GET /overlay/feed`. The
// feed is a tagged union; this slice only emits `chatMessage`, but the shape is
// stable so later slices (stream events, deletions) just add variants.
// ---------------------------------------------------------------------------

export interface TextFragmentDto {
  kind: "text";
  text: string;
}

export interface EmoteFragmentDto {
  kind: "emote";
  id: string;
  url: string;
}

export type FragmentDto = TextFragmentDto | EmoteFragmentDto;

// One resolved native Twitch badge. The Rust side drops resolver misses, so a
// badge that reaches the feed always carries a renderable `url`.
export interface ChatBadgeDto {
  setId: string;
  version: string;
  url: string;
}

export interface ChatMessageDto {
  kind: "chatMessage";
  msgId: string;
  username: string;
  color: string;
  channel: string;
  badges: ChatBadgeDto[];
  fragments: FragmentDto[];
}

export type FeedEventDto = ChatMessageDto;
