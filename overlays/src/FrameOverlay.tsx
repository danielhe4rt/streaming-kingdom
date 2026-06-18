import { useEffect, useState } from "react";
import type { FeedEventDto, StreamEventDto } from "./feed";
import { FEED_URL } from "./feed";
import { useFooterBar } from "./footerBar";

// ---------------------------------------------------------------------------
// Frame Overlay (issue #6)
//
// Replaces the old static PNG with a CSS branding frame (border, logo, socials,
// channel — hardcoded for v1) plus a live Footer Bar. The Footer Bar rotates
// Fun Facts when idle and plays queued Alerts when StreamEvents arrive over the
// Overlay Feed. A now-playing slot is present as STRUCTURE ONLY; its data source
// is wired later by the streamer.
// ---------------------------------------------------------------------------

// Hardcoded branding (v1). A later slice makes these configurable.
const BRAND = {
  channel: "danielhe4rt",
  logo: "</>",
  socials: [
    { label: "twitch", handle: "/danielhe4rt" },
    { label: "github", handle: "@danielhe4rt" },
    { label: "x", handle: "@danielhe4rt" },
  ],
};

// Hardcoded Fun Facts (v1). Rotated in the Footer Bar when no Alert is playing.
const FUN_FACTS = [
  "Rust has no garbage collector — ownership frees memory deterministically.",
  "This overlay is a React app embedded straight into the Rust binary.",
  "The Overlay Feed is a single SSE stream: one source, many overlays.",
  "`?` propagates errors so the happy path stays readable.",
  "Type !commands in chat to see what the bot can do.",
];

const FUN_FACT_INTERVAL_MS = 8000;
// How long an Alert holds the bar before returning to Fun Facts. Drives the
// state machine's `alertEnded` transition.
const ALERT_DURATION_MS = 6000;

function centsToCurrency(cents: number): string {
  return `R$ ${(cents / 100).toFixed(2)}`;
}

// One line of human-readable copy per StreamEvent type, used by the Alert.
function alertCopy(event: StreamEventDto): { title: string; detail: string } {
  switch (event.type) {
    case "donation":
      return {
        title: `${event.username} donated ${centsToCurrency(event.amountCents)}`,
        detail: event.message || "Thank you!",
      };
    case "sub":
      return {
        title: `${event.username} subscribed (${event.tier})`,
        detail:
          event.months > 1 ? `${event.months} months strong!` : "Welcome aboard!",
      };
    case "giftSub":
      return {
        title: `${event.username} gifted ${event.total} subs`,
        detail: "What a legend!",
      };
    case "raid":
      return {
        title: `${event.fromChannel} raided with ${event.viewers} viewers`,
        detail: "Welcome, raiders!",
      };
    case "cheer":
      return {
        title: `${event.username} cheered ${event.bits} bits`,
        detail: event.message || "Thanks for the bits!",
      };
    case "follow":
      return { title: `${event.username} followed`, detail: "Welcome!" };
    case "viewerCountUpdate":
      return { title: `${event.count} watching`, detail: "" };
  }
}

// The branding frame: a CSS border with the logo + channel up top and socials
// along the side. Replaces the old PNG; everything here is hardcoded for v1.
function FrameChrome() {
  return (
    <div className="pointer-events-none absolute inset-0">
      {/* Border ring */}
      <div className="absolute inset-4 rounded-3xl border-4 border-violet-500/80 [box-shadow:_0_0_24px_rgb(139_92_246_/_40%)]" />

      {/* Top-left logo + channel */}
      <div className="absolute left-10 top-8 flex items-center gap-3">
        <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-violet-600 text-2xl font-black text-white [text-shadow:_0_2px_4px_rgb(0_0_0_/_60%)]">
          {BRAND.logo}
        </div>
        <span className="text-3xl font-extrabold text-white [text-shadow:_0_2px_6px_rgb(0_0_0_/_80%)]">
          {BRAND.channel}
        </span>
      </div>

      {/* Right-side socials */}
      <div className="absolute right-10 top-8 flex flex-col items-end gap-1 text-right">
        {BRAND.socials.map((s) => (
          <div key={s.label} className="text-lg text-white/90 [text-shadow:_0_2px_4px_rgb(0_0_0_/_80%)]">
            <span className="font-semibold uppercase tracking-wide text-violet-300">
              {s.label}
            </span>{" "}
            {s.handle}
          </div>
        ))}
      </div>
    </div>
  );
}

// Rotating Fun Facts shown when the Footer Bar is idle.
function FunFacts() {
  const [index, setIndex] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setIndex((i) => (i + 1) % FUN_FACTS.length);
    }, FUN_FACT_INTERVAL_MS);
    return () => clearInterval(id);
  }, []);

  return (
    <div
      key={index}
      className="flex animate-[fadein_400ms_ease-out] items-center gap-3 text-xl text-white"
    >
      <span className="rounded-md bg-violet-600 px-2 py-0.5 text-sm font-bold uppercase tracking-wide">
        Did you know?
      </span>
      <span className="[text-shadow:_0_2px_4px_rgb(0_0_0_/_80%)]">
        {FUN_FACTS[index]}
      </span>
    </div>
  );
}

// Alert takeover. Calls `onEnd` once its hold duration elapses so the Footer Bar
// can advance the queue (the state machine's `alertEnded` transition).
function Alert({
  event,
  onEnd,
}: {
  event: StreamEventDto;
  onEnd: () => void;
}) {
  const { title, detail } = alertCopy(event);

  useEffect(() => {
    const id = setTimeout(onEnd, ALERT_DURATION_MS);
    return () => clearTimeout(id);
    // Re-arm whenever the playing event changes so each queued Alert gets its
    // own full hold before the next plays.
  }, [event, onEnd]);

  return (
    <div className="flex w-full animate-[alertin_500ms_cubic-bezier(0.16,1,0.3,1)] items-center gap-4">
      <span className="rounded-md bg-gradient-to-r from-fuchsia-600 to-violet-600 px-3 py-1 text-lg font-black uppercase tracking-wider text-white [text-shadow:_0_2px_4px_rgb(0_0_0_/_60%)]">
        Alert
      </span>
      <div className="flex flex-col">
        <span className="text-2xl font-extrabold text-white [text-shadow:_0_2px_6px_rgb(0_0_0_/_80%)]">
          {title}
        </span>
        {detail && <span className="text-lg text-violet-200">{detail}</span>}
      </div>
    </div>
  );
}

// Now-playing slot — STRUCTURE ONLY. The Spotify/playerctl data source is wired
// later by the streamer; for v1 it shows a placeholder so the layout is final.
function NowPlaying() {
  return (
    <div className="flex shrink-0 items-center gap-3 border-l border-white/20 pl-4">
      <span className="text-2xl">♪</span>
      <div className="flex flex-col leading-tight">
        <span className="text-xs uppercase tracking-wide text-violet-300">
          Now Playing
        </span>
        <span className="text-base text-white/70">— nothing playing —</span>
      </div>
    </div>
  );
}

// The Footer Bar: arbitrates Fun Facts ↔ Alerts and hosts the now-playing slot.
function FooterBar() {
  const { state, pushEvent, endAlert } = useFooterBar();

  useEffect(() => {
    const source = new EventSource(FEED_URL);

    source.addEventListener("message", (event) => {
      try {
        const dto = JSON.parse(event.data) as FeedEventDto;
        if (dto.kind === "streamEvent") {
          pushEvent(dto);
        }
        // Other feed kinds (chatMessage / chatMessageDeleted) belong to the
        // Chat Overlay; the Frame Overlay ignores them.
      } catch {
        // Ignore malformed frames; the feed is best-effort for a display source.
      }
    });

    return () => source.close();
  }, [pushEvent]);

  return (
    <div className="pointer-events-none absolute inset-x-10 bottom-10 flex h-20 items-center justify-between gap-6 rounded-2xl border border-violet-500/50 bg-black/70 px-6 backdrop-blur-sm">
      <div className="flex min-w-0 flex-1 items-center overflow-hidden">
        {state.mode === "alert" && state.current ? (
          <Alert event={state.current} onEnd={endAlert} />
        ) : (
          <FunFacts />
        )}
      </div>
      <NowPlaying />
    </div>
  );
}

export default function FrameOverlay() {
  return (
    <div className="relative h-full w-full overflow-hidden">
      <FrameChrome />
      <FooterBar />
    </div>
  );
}
