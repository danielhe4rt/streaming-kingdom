import { useEffect, useState, type ReactNode } from "react";

// ---------------------------------------------------------------------------
// FunFactsTicker — the idle-state fun facts in the footer's right region.
//
// Headless/presentational: props only (no feed coupling). Two stacked rows — a
// header ("HE4RT FUN FACTS" label + a full-width brand rule) on top, and ONE
// fact below. Facts rotate: each holds on screen for 5–10s, then cross-fades to
// the next (not a marquee). Numbers/URLs are auto-accented; named entities come
// in via `highlightTerms` so the component stays content-agnostic.
// ---------------------------------------------------------------------------

export interface FunFactsTickerProps {
  facts: string[];
  /** Named entities (project/brand/tech) to accent. Numbers + URLs auto-accent. */
  highlightTerms?: string[];
}

// Each fact stays fully visible for a random hold in [HOLD_MIN, HOLD_MAX] before
// fading out; FADE_MS is the fade-out/in duration.
const HOLD_MIN_MS = 5000;
const HOLD_MAX_MS = 10000;
const FADE_MS = 500;

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

export function FunFactsTicker({ facts, highlightTerms = [] }: FunFactsTickerProps) {
  const regex = buildAccentRegex(highlightTerms);
  const [index, setIndex] = useState(0);
  const [visible, setVisible] = useState(true);

  // Hold the current fact (5–10s), then begin its fade-out.
  useEffect(() => {
    if (facts.length <= 1) return;
    const ms = HOLD_MIN_MS + Math.random() * (HOLD_MAX_MS - HOLD_MIN_MS);
    const hold = setTimeout(() => setVisible(false), ms);
    return () => clearTimeout(hold);
  }, [index, facts.length]);

  // Once faded out, swap to the next fact and fade back in.
  useEffect(() => {
    if (visible) return;
    const swap = setTimeout(() => {
      setIndex((i) => (i + 1) % facts.length);
      setVisible(true);
    }, FADE_MS);
    return () => clearTimeout(swap);
  }, [visible, facts.length]);

  const fact = facts[index] ?? "";

  return (
    // Footer's right region, full bar height. Two stacked rows: a header
    // (label + full-width brand rule) on top, the rotating fact below.
    <div className="absolute right-0 bottom-0 left-[792px] flex h-[122px] flex-col justify-center gap-[12px] overflow-hidden">
      {/* Header row: the label with a solid brand rule filling the width. */}
      <div className="flex items-center gap-[22px]">
        <span className="flex-none whitespace-nowrap font-saira-cond text-[26px] font-extrabold uppercase tracking-[.05em] text-white">
          HE4RT FUN FACTS
        </span>
        <div className="h-[3px] flex-1 rounded-full bg-brand" />
      </div>

      {/* Facts row: one fact at a time, cross-fading on rotation. */}
      <div className="flex h-[30px] items-center overflow-hidden">
        <span
          className="truncate text-[21px] font-semibold text-brand-ticker"
          style={{
            opacity: visible ? 1 : 0,
            transition: `opacity ${FADE_MS}ms ease`,
          }}
        >
          {renderFact(fact, regex)}
        </span>
      </div>
    </div>
  );
}

export default FunFactsTicker;
