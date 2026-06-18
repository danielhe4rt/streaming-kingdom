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

export interface ChatMessageDto {
  kind: "chatMessage";
  msgId: string;
  username: string;
  color: string;
  channel: string;
  fragments: FragmentDto[];
}

export type FeedEventDto = ChatMessageDto;
