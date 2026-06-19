import type React from "react";

// ---------------------------------------------------------------------------
// FooterBar — the full-width bottom background strip of the Coworking overlay.
//
// Headless/presentational: it paints the 122px-tall gradient band that anchors
// the Now Playing widget, the Fun Facts ticker, and the Event Alert. It owns
// only its zone geometry (the reference's bottom bar, line ~99) and renders
// whatever slot children the container places inside it.
// ---------------------------------------------------------------------------

export interface FooterBarProps {
  children?: React.ReactNode;
}

export function FooterBar({ children }: FooterBarProps) {
  return (
    <div
      // Reference line ~99: vertical gradient ink-850 → ink-900, thin brand
      // top border. Spans the full width, pinned to the bottom of the stage.
      className="absolute right-0 bottom-0 left-0 h-[122px] border-t border-brand/22 bg-[linear-gradient(0deg,var(--color-ink-900)_0%,var(--color-ink-850)_100%)]"
    >
      {children}
    </div>
  );
}

export default FooterBar;
