// ---------------------------------------------------------------------------
// VoiceRoster — the Discord voice-channel roster widget.
//
// Headless/presentational: props only; ui/ never imports feed DTOs. Lays out a
// row of VoiceMemberCards under the channel name. When `roster` is null (the
// streamer is not in a voice channel / Discord is disconnected) it renders
// nothing so the overlay layout never reserves dead space. Self-contained and
// reusable — the container decides where to mount it.
// ---------------------------------------------------------------------------

import VoiceMemberCard, { type VoiceMember } from "./VoiceMemberCard";

export interface VoiceRoster {
  channelId: string;
  channelName: string | null;
  members: VoiceMember[];
}

export interface VoiceRosterProps {
  roster: VoiceRoster | null;
}

export function VoiceRoster({ roster }: VoiceRosterProps) {
  // No channel → render nothing. The mapper already collapses the disconnected
  // frame (channelId:null) to null, so this also covers Discord being closed.
  if (!roster) return null;

  return (
    <div className="flex flex-col gap-[10px] rounded-[18px] border border-brand/[.22] bg-ink-900/80 px-[18px] py-[14px] [box-shadow:0_8px_24px_rgba(0,0,0,.45),inset_0_0_0_1px_rgba(201,164,255,.12)] backdrop-blur-sm">
      <div className="flex items-center gap-[8px]">
        {/* Discord-ish voice glyph drawn from brand tokens — a speaker mark. */}
        <span className="text-[15px] leading-none">🎧</span>
        <span className="font-saira-cond text-[13px] font-bold tracking-[.16em] text-brand-light">
          {roster.channelName?.toUpperCase() ?? "VOICE"}
        </span>
      </div>

      {roster.members.length === 0 ? (
        // Subtle empty state: in the channel, but alone. Keeps the panel from
        // collapsing to just its header.
        <span className="text-[13px] font-medium text-brand-faint">
          waiting for the crew…
        </span>
      ) : (
        <div className="flex flex-wrap gap-x-[14px] gap-y-[12px]">
          {roster.members.map((m) => (
            <VoiceMemberCard key={m.userId} member={m} />
          ))}
        </div>
      )}
    </div>
  );
}

export default VoiceRoster;
