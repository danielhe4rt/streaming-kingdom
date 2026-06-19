import type { ReactNode } from "react";

// ---------------------------------------------------------------------------
// FunFactsTicker — the idle-state marquee in the footer's right region.
//
// Headless/presentational: props only. Two stacked rows — a header ("HE4RT FUN
// FACTS" label + a full-width brand rule) on top, and a horizontally scrolling
// marquee of the given facts on its own line below. Each fact is followed by a
// brand heart separator; the whole sequence is rendered TWICE inside the moving
// track and the tickerScroll keyframe translates by -50%, so the second copy
// seamlessly takes the place of the first — an infinite, gapless loop.
//
// Keyword accenting: numbers and URLs are highlighted automatically; named
// entities come in via `highlightTerms` so the component stays content-agnostic.
// ---------------------------------------------------------------------------

export interface FunFactsTickerProps {
  facts: string[];
  /** Named entities (project/brand/tech) to accent. Numbers + URLs auto-accent. */
  highlightTerms?: string[];
}

// Auto-accented patterns: URLs/handles and numbers (incl. "27 mil").
const URL_PATTERN = "(?:https?:\\/\\/|discord\\.gg\\/)\\S+|@\\w+";
const NUMBER_PATTERN = "\\d[\\d.,]*(?:\\s?(?:mil|k|M))?";

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// One alternation regex with a single capture group, so String.split keeps the
// matched tokens at odd indices. Longer terms first so phrases win over their
// substrings (e.g. "He4rt Delas" before any bare word).
function buildAccentRegex(terms: string[]): RegExp {
  const named = [...terms].sort((a, b) => b.length - a.length).map(escapeRegExp);
  const pattern = [...named, URL_PATTERN, NUMBER_PATTERN].join("|");
  return new RegExp("(" + pattern + ")", "gi");
}

// Split a fact into plain + accented runs (accented runs land on odd indices).
function renderFact(fact: string, regex: RegExp): ReactNode[] {
  return fact
    .split(regex)
    .map((part, i) =>
      part === "" ? null : i % 2 === 1 ? (
        <span key={i} className="font-bold text-yellow">
          {part}
        </span>
      ) : (
        <span key={i}>{part}</span>
      ),
    );
}

// The brand heart that separates facts (replaces the old "•" bullet).
function Separator({ k }: { k: string }) {
  return (
    <span
      key={k}
      aria-hidden="true"
      className="mx-[20px] text-[15px] text-brand-bright"
    >
      ♥
    </span>
  );
}

export function FunFactsTicker({ facts, highlightTerms = [] }: FunFactsTickerProps) {
  const regex = buildAccentRegex(highlightTerms);

  // One pass of [fact, heart, fact, heart, …]; rendered twice for the loop.
  const pass = (prefix: string): ReactNode[] =>
    facts.flatMap((fact, idx) => [
      <span
        key={`${prefix}f${idx}`}
        className="whitespace-nowrap text-[21px] font-semibold text-brand-ticker"
      >
        {renderFact(fact, regex)}
      </span>,
      <Separator key={`${prefix}s${idx}`} k={`${prefix}s${idx}`} />,
    ]);

  return (
    // Footer's right region, full bar height. Two stacked rows: a header
    // (label + full-width brand rule) on top, the scrolling facts below.
    <div className="absolute right-0 bottom-0 left-[792px] flex h-[122px] flex-col justify-center gap-[12px] overflow-hidden">
      {/* Header row: the label with a solid brand rule filling the width. */}
      <div className="flex items-center gap-[22px]">
        <span className="flex-none whitespace-nowrap font-saira-cond text-[26px] font-extrabold uppercase tracking-[.05em] text-white">
          HE4RT FUN FACTS
        </span>
        <div className="h-[3px] flex-1 rounded-full bg-brand" />
      </div>

      {/* Facts row: the scrolling marquee on its own line, edge-faded. */}
      <div
        className="relative flex h-[30px] items-center overflow-hidden"
        // Edge fade: the scrolling text fades in on the right and out on the
        // left instead of hard-cutting at the row bounds.
        style={{
          WebkitMaskImage:
            "linear-gradient(90deg, transparent 0, #000 4%, #000 94%, transparent 100%)",
          maskImage:
            "linear-gradient(90deg, transparent 0, #000 4%, #000 94%, transparent 100%)",
        }}
      >
        {/* Moving track: duplicated sequence + tickerScroll -50% = seamless loop */}
        <div
          className="flex items-center whitespace-nowrap will-change-transform"
          style={{ animation: "tickerScroll 48s linear infinite" }}
        >
          {pass("a")}
          {pass("b")}
        </div>
      </div>
    </div>
  );
}

export default FunFactsTicker;
