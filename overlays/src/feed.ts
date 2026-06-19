// ---------------------------------------------------------------------------
// Overlay Feed DTOs (M3)
//
// These mirror the Rust serde DTOs serialized onto `GET /overlay/feed`. The
// feed is a tagged union over `kind`: chat messages, single-message deletions,
// and stream events (donation / sub / raid …) that the Frame Overlay's Footer
// Bar turns into Alerts. One source, N Overlays consume the same stream.
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

// Single-message moderation (CLEARMSG): one message was deleted. The Overlay
// removes the message node whose key matches `msgId`.
export interface ChatMessageDeletedDto {
  kind: "chatMessageDeleted";
  msgId: string;
}

// A stream event on the feed (donation / sub / raid …). The Rust side flattens
// the domain enum, so the discriminant is the inner `type` while the outer
// `kind` is always "streamEvent". The Frame Overlay's Footer Bar switches on
// `kind` first, then `type` to choose an Alert template.
export type SubTierDto = "tier1" | "tier2" | "tier3" | "prime";

export interface FollowEventDto {
  kind: "streamEvent";
  type: "follow";
  username: string;
}

export interface SubEventDto {
  kind: "streamEvent";
  type: "sub";
  username: string;
  tier: SubTierDto;
  months: number;
}

export interface DonationEventDto {
  kind: "streamEvent";
  type: "donation";
  username: string;
  amountCents: number;
  message: string;
}

export interface GiftSubEventDto {
  kind: "streamEvent";
  type: "giftSub";
  username: string;
  tier: SubTierDto;
  total: number;
}

export interface CheerEventDto {
  kind: "streamEvent";
  type: "cheer";
  username: string;
  bits: number;
  message: string;
}

export interface RaidEventDto {
  kind: "streamEvent";
  type: "raid";
  fromChannel: string;
  viewers: number;
}

export interface ViewerCountUpdateDto {
  kind: "streamEvent";
  type: "viewerCountUpdate";
  count: number;
}

export type StreamEventDto =
  | FollowEventDto
  | SubEventDto
  | DonationEventDto
  | GiftSubEventDto
  | CheerEventDto
  | RaidEventDto
  | ViewerCountUpdateDto;

// Ambient "now playing" STATE pushed onto the feed (kind "nowPlaying"). Unlike
// the stream events above it isn't a one-shot alert — it's the latest Spotify
// track mirrored from a tokio::sync::watch on the Rust side. `status` of
// "stopped" (with empty title/artist) means the player is gone or idle and the
// widget should fall back to its placeholder.
export interface NowPlayingDto {
  kind: "nowPlaying";
  title: string;
  artist: string;
  album: string;
  artUrl: string | null;
  status: "playing" | "paused" | "stopped";
}

// Ambient Discord voice-channel roster STATE pushed onto the feed (kind
// "voiceRoster"). Like nowPlaying it is the latest value mirrored from a
// tokio::sync::watch on the Rust side, not a one-shot alert. A null `channelId`
// (with empty `members`) means the streamer left voice / Discord disconnected,
// and the widget should render nothing. Field names are camelCase, matching the
// Rust serde DTO exactly; `avatarUrl` is nullable (Option<String>).
export interface VoiceMemberDto {
  userId: string;
  displayName: string;
  avatarUrl: string | null;
  speaking: boolean;
  selfMute: boolean;
  selfDeaf: boolean;
  serverMute: boolean;
  serverDeaf: boolean;
}

export interface VoiceRosterDto {
  kind: "voiceRoster";
  channelId: string | null;
  channelName: string | null;
  members: VoiceMemberDto[];
}

export type FeedEventDto =
  | ChatMessageDto
  | ChatMessageDeletedDto
  | StreamEventDto
  | NowPlayingDto
  | VoiceRosterDto;

// In dev the Vite server runs on its own origin, so the feed must point at the
// Rust server explicitly. In the embedded build the page is same-origin, so a
// relative path is correct.
export const FEED_URL = import.meta.env.DEV
  ? "http://127.0.0.1:1111/overlay/feed"
  : "/overlay/feed";
