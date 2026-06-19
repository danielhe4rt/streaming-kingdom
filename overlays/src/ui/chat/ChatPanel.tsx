// ChatPanel: the left-column chat zone. A zone/slot component, so it owns its
// absolute coordinates from the reference (left:0 top:100px width:710px
// bottom:122px — i.e. between the TopBar and FooterBar). It paints the gradient
// background + brand watermark + header chip, and renders its children (the
// ChatBubbles) bottom-aligned so the newest message sits at the bottom edge.
import type { ReactNode } from "react";

export interface ChatPanelProps {
  title: string;
  children: ReactNode;
}

export function ChatPanel({ title, children }: ChatPanelProps) {
  return (
    <div className="absolute left-0 top-[100px] bottom-[122px] w-[710px] overflow-hidden border-t-2 border-brand/35 bg-[linear-gradient(180deg,var(--color-ink-900)_0%,var(--color-ink-950)_100%)]">
      {/* Faint brand watermark — decorative He4rt mark, skewed/rotated and barely
          visible (opacity .05). pointer-events-none so it never blocks layout. */}
      <svg
        viewBox="0 0 120 120"
        width="420"
        height="420"
        className="pointer-events-none absolute left-[-70px] top-10 opacity-5 [transform:skewX(-8deg)_rotate(-6deg)]"
      >
        <g fill="var(--color-brand)">
          <rect x="22" y="14" width="20" height="92" rx="9" />
          <rect x="80" y="14" width="20" height="92" rx="9" />
          <path d="M61 80C42 65 42 47 55 47c4 0 6 3 6 6 0-3 2-6 6-6 13 0 13 18-6 33Z" />
        </g>
      </svg>

      {/* Header chip: glowing dot + title, in a rounded pill above the list. */}
      <div className="absolute left-4 top-4 z-[2] flex items-center gap-2 rounded-full border border-brand-light/[.28] bg-[rgba(20,10,31,.55)] px-[14px] py-[7px]">
        <span className="h-2 w-2 rounded-full bg-brand shadow-[0_0_10px_var(--color-brand)]" />
        <span className="font-saira-cond text-[14px] font-bold tracking-[.14em] text-brand-pale">
          {title}
        </span>
      </div>

      {/* Chat list: bottom-aligned column so newest message hugs the bottom. */}
      <div className="absolute inset-x-0 top-[54px] bottom-0 flex flex-col justify-end gap-6 overflow-hidden pt-4 pb-[22px]">
        {children}
      </div>
    </div>
  );
}

export default ChatPanel;
