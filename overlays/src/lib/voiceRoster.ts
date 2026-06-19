// ---------------------------------------------------------------------------
// Voice-roster view-model mapper
//
// The boundary between the feed (VoiceRosterDto wire shape) and the headless
// ui/voice/VoiceRoster widget. ui/ never imports feed DTOs; this is where the
// ambient Discord roster is flattened into the props VoiceRoster renders.
//
// A null `channelId` (streamer left voice / Discord disconnected) maps to null
// so the widget renders nothing. The wire field names are camelCase, mirroring
// the Rust serde DTO exactly.
// ---------------------------------------------------------------------------

import type { VoiceRosterDto } from "../feed";
import type { VoiceRoster } from "../ui/voice/VoiceRoster";

export function toVoiceRoster(dto: VoiceRosterDto): VoiceRoster | null {
  if (dto.channelId === null) return null; // not in a channel → render nothing

  return {
    channelId: dto.channelId,
    channelName: dto.channelName,
    members: dto.members.map((m) => ({
      userId: m.userId,
      displayName: m.displayName,
      avatarUrl: m.avatarUrl,
      speaking: m.speaking,
      selfMute: m.selfMute,
      selfDeaf: m.selfDeaf,
      serverMute: m.serverMute,
      serverDeaf: m.serverDeaf,
    })),
  };
}
