// ---------------------------------------------------------------------------
// VoiceDock — the standalone "N NA SALA" voice card (the provided mock).
//
// A floating, self-contained card pinned top-right over a transparent page: the
// Discord glyph + member count header, a divider, and the avatar list. The list
// direction follows `layout` (`col` matches the mock; `row` lays the avatars out
// horizontally). Renders nothing when the streamer is not in a voice channel, so
// the page stays fully transparent. Presentational: props only.
// ---------------------------------------------------------------------------

import { VoiceDockAvatar } from "./VoiceDockAvatar";
import type { VoiceRoster } from "./VoiceRoster";

export type DockLayout = "row" | "col";

const DiscordGlyph = () => (
  <svg width="26" height="26" viewBox="0 0 24 24" fill="#8b2fe8">
    <path d="M20 4.4A17 17 0 0 0 15.7 3l-.2.4a13 13 0 0 1 3.7 1.2 12.7 12.7 0 0 0-11 0A12.6 12.6 0 0 1 12 3.4L11.8 3A17 17 0 0 0 7.5 4.4 18 18 0 0 0 4 17a17 17 0 0 0 5.2 2.6l.6-1a11 11 0 0 1-1.8-.9l.4-.3a12 12 0 0 0 10.2 0l.4.3a11 11 0 0 1-1.8.9l.6 1A17 17 0 0 0 23.5 17 18 18 0 0 0 20 4.4ZM9.3 14.7c-.8 0-1.5-.8-1.5-1.7s.7-1.7 1.5-1.7 1.5.8 1.5 1.7-.7 1.7-1.5 1.7Zm5.4 0c-.8 0-1.5-.8-1.5-1.7s.7-1.7 1.5-1.7 1.5.8 1.5 1.7-.7 1.7-1.5 1.7Z" />
  </svg>
);

export interface VoiceDockProps {
  roster: VoiceRoster | null;
  layout?: DockLayout;
}

export function VoiceDock({ roster, layout = "col" }: VoiceDockProps) {
  // Not in a channel (or alone-and-empty) → render nothing; page stays clear.
  if (!roster || roster.members.length === 0) return null;

  const isRow = layout === "row";

  return (
    <div
      className="absolute flex flex-col items-center"
      style={{
        right: 12,
        top: 12,
        width: isRow ? "auto" : 92,
        padding: isRow ? "13px 16px 16px" : "13px 0 16px",
        background: "linear-gradient(180deg,#1d0f31 0%,#160b26 100%)",
        border: "1px solid rgba(139,47,232,.32)",
        borderRadius: 20,
        boxShadow:
          "0 16px 44px rgba(0,0,0,.5), inset 0 0 0 1px rgba(201,164,255,.05)",
        animation: "sbIn .5s cubic-bezier(.18,.9,.3,1.1) both",
      }}
    >
      <div className="flex flex-col items-center gap-[5px]">
        <DiscordGlyph />
        <span
          className="font-saira-cond font-extrabold"
          style={{ fontSize: 12, letterSpacing: ".12em", color: "#c9a4ff" }}
        >
          {roster.members.length} NA SALA
        </span>
      </div>

      <div
        style={{
          width: 52,
          height: 1,
          background: "rgba(139,47,232,.3)",
          margin: "11px 0 15px",
        }}
      />

      <div
        className={`flex items-center ${isRow ? "flex-row" : "flex-col"}`}
        style={{ gap: 15 }}
      >
        {roster.members.map((m) => (
          <VoiceDockAvatar key={m.userId} member={m} />
        ))}
      </div>
    </div>
  );
}

export default VoiceDock;
