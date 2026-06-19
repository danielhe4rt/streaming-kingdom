// ---------------------------------------------------------------------------
// NowPlaying — the Spotify "now playing" widget in the footer's left corner.
//
// Headless/presentational: props only. Renders the album cover (when present)
// framed by the spinning conic disc, the EQ bars, the NOW PLAYING label, and
// the track title/artist (reference lines ~101-120). When `track` is null it
// falls back to a graceful placeholder so the layout never collapses. The disc
// spin and EQ bounce only run while `isPlaying` — paused tracks freeze both.
// ---------------------------------------------------------------------------

export interface NowPlayingTrack {
  title: string;
  artist: string;
  artUrl: string | null;
  isPlaying: boolean;
}

export interface NowPlayingProps {
  track: NowPlayingTrack | null;
}

// EQ bar animation delays (reference lines ~110-113): four bars, staggered so
// they bounce out of phase.
const EQ_DELAYS = ["0s", "0.25s", "0.5s", "0.15s"];

export function NowPlaying({ track }: NowPlayingProps) {
  // Placeholder copy matches the reference's default props (lines ~227-228).
  const title = track?.title ?? "lofi beats to code to";
  const artist = track?.artist ?? "He4rt Radio";
  const artUrl = track?.artUrl ?? null;
  // No track => placeholder, which we let "play" so the disc keeps spinning as
  // an idle motif. A real paused track freezes the spin + EQ bars.
  const isPlaying = track ? track.isPlaying : true;
  // Gating animations on a flag rather than conditional markup keeps the disc
  // mounted (no layout shift) — paused just holds the current frame.
  const spin = isPlaying ? "spin 6s linear infinite" : "none";

  return (
    // Reference line ~102: pinned bottom-left, 94px tall, row layout.
    <div className="absolute bottom-[14px] left-[14px] flex h-[94px] items-center">
      {/* Disc tile: rounded panel framing the album cover (or the bare conic
          spinner when no cover) + Spotify glyph. */}
      <div className="relative flex h-[94px] w-[94px] flex-none items-center justify-center overflow-hidden rounded-[18px] bg-[linear-gradient(145deg,var(--color-disc),#120a1f)] [box-shadow:0_8px_24px_rgba(0,0,0,.45),inset_0_0_0_1px_rgba(201,164,255,.18)]">
        {/* Spinning vinyl: conic brand gradient with an ink hub (reference ~104-106).
            When a cover exists it becomes the ring/frame behind it; otherwise it
            stands alone with its ink hub. */}
        <div
          className={
            artUrl
              ? "absolute inset-0 rounded-[18px] bg-[conic-gradient(from_0deg,var(--color-brand),#3a1f5c,var(--color-brand))]"
              : "flex h-[42px] w-[42px] items-center justify-center rounded-full bg-[conic-gradient(from_0deg,var(--color-brand),#3a1f5c,var(--color-brand))]"
          }
          style={{ animation: spin }}
        >
          {!artUrl && <div className="h-[14px] w-[14px] rounded-full bg-ink-900" />}
        </div>
        {/* Real album cover from mpris:artUrl, inset so the conic ring shows as a
            frame around it. */}
        {artUrl && (
          <img
            src={artUrl}
            alt={`${title} — ${artist}`}
            className="absolute inset-[6px] h-[82px] w-[82px] rounded-[14px] object-cover"
          />
        )}
        {/* Spotify glyph, top-left over the disc (reference ~107) */}
        <svg
          width="22"
          height="22"
          viewBox="0 0 24 24"
          fill="var(--color-spotify)"
          className="absolute top-[7px] left-[7px]"
        >
          <circle cx="12" cy="12" r="11" fill="var(--color-spotify)" />
          <path
            d="M7 9.5c3-.8 6.5-.5 9 1M7.5 12.2c2.5-.6 5.3-.4 7.3 1M8 14.8c2-.5 4-.3 5.6.8"
            stroke="#0a0a0a"
            strokeWidth="1.3"
            strokeLinecap="round"
            fill="none"
          />
        </svg>
      </div>

      {/* EQ bars: four spotify-green bars bouncing via barEq (reference ~109-114).
          Frozen when paused so they hold their resting height. */}
      <div className="ml-[12px] flex h-[26px] items-end gap-[3px]">
        {EQ_DELAYS.map((delay, i) => (
          <span
            key={i}
            className="w-[4px] rounded-[2px] bg-spotify"
            style={{
              animation: isPlaying
                ? `barEq 0.9s ease-in-out infinite ${delay}`
                : "none",
            }}
          />
        ))}
      </div>

      {/* Label + track text (reference ~115-119) */}
      <div className="ml-[14px] flex flex-col justify-center">
        <span className="font-saira-cond text-[13px] font-bold tracking-[.16em] text-spotify">
          NOW PLAYING
        </span>
        <span className="text-[18px] font-bold leading-[1.15] text-white">
          {title}
        </span>
        <span className="text-[14px] font-medium leading-[1.15] text-brand-faint">
          {artist}
        </span>
      </div>
    </div>
  );
}

export default NowPlaying;
