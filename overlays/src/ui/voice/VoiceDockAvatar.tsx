// ---------------------------------------------------------------------------
// VoiceDockAvatar — one member in the standalone voice dock.
//
// Faithful to the provided mock: a 56px circle (the Discord avatar, or a stable
// gradient + initial when there is no photo), an animated green speak-ring while
// the member is talking, and a state dot pinned bottom-right — green when
// speaking, red when muted/deafened, neutral otherwise. Presentational: props
// only, no feed/network knowledge.
// ---------------------------------------------------------------------------

import type { VoiceMember } from "./VoiceMemberCard";

// Avatar diameter in px. The mock uses 56 (the original ask was 40); change this
// one constant to resize the whole dock.
const SIZE = 56;

// No-photo fallback gradients, matching the mock's palette. Picked by a stable
// hash of the userId so a member always gets the same colour.
const GRADIENTS = [
  "linear-gradient(150deg,#9a3cff,#6d1fd0)",
  "linear-gradient(150deg,#ff6fae,#c43d86)",
  "linear-gradient(150deg,#1ed760,#0f9b46)",
  "linear-gradient(150deg,#ffcb05,#d99e00)",
];

function gradientFor(userId: string): string {
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = (hash * 31 + userId.charCodeAt(i)) | 0;
  }
  return GRADIENTS[Math.abs(hash) % GRADIENTS.length];
}

export function VoiceDockAvatar({ member }: { member: VoiceMember }) {
  const muted = member.selfMute || member.serverMute;
  const deafened = member.selfDeaf || member.serverDeaf;
  const initial = member.displayName.charAt(0).toUpperCase() || "?";

  // Speaking wins; otherwise red flags a muted/deaf member, else the idle dot.
  const dotColor = member.speaking
    ? "#1ed760"
    : muted || deafened
      ? "#ed4245"
      : "#3a2b52";

  return (
    <div className="relative" style={{ width: SIZE, height: SIZE }}>
      {member.speaking && (
        <div
          className="absolute rounded-full"
          style={{
            inset: -4,
            border: "3px solid #1ed760",
            animation: "speakRing 1.4s ease-in-out infinite",
          }}
        />
      )}

      <div
        className="flex items-center justify-center overflow-hidden rounded-full font-extrabold text-white"
        style={{
          width: SIZE,
          height: SIZE,
          fontSize: 24,
          background: member.avatarUrl ? "#160b26" : gradientFor(member.userId),
          boxShadow: "0 0 0 2px rgba(201,164,255,.16)",
        }}
      >
        {member.avatarUrl ? (
          <img
            src={member.avatarUrl}
            alt={member.displayName}
            className="h-full w-full object-cover"
          />
        ) : (
          initial
        )}
      </div>

      <div
        className="absolute rounded-full"
        style={{
          right: -1,
          bottom: -1,
          width: 18,
          height: 18,
          background: dotColor,
          border: "3px solid #160b26",
        }}
      />
    </div>
  );
}

export default VoiceDockAvatar;
