// ChatBubble: one chat message rendered as the reference's two-part shape —
// a white name pill (with badges) on top, and a dark speech bubble with a
// little pointer triangle below. Headless/presentational and layout-flow
// (no absolute position) so ChatPanel can stack many of these in a column.
// Animates `chatIn` on mount so new messages slide up into place.
import { ChatBadge, type ChatBadgeProps } from "./ChatBadge";

// A message body is a sequence of parts so emotes can be interleaved with text.
export type ChatPart =
  | { kind: "text"; text: string }
  | { kind: "emote"; url: string };

export interface ChatBubbleProps {
  name: string;
  badges: ChatBadgeProps[];
  parts: ChatPart[];
}

export function ChatBubble({ name, badges, parts }: ChatBubbleProps) {
  return (
    <div className="flex flex-col items-start animate-[chatIn_0.42s_cubic-bezier(.18,.9,.3,1.1)_both]">
      {/* White name pill — flat left edge (border-radius 0 999px 999px 0) so it
          reads as flush against the panel's left wall. */}
      <div className="flex items-center gap-[9px] rounded-[0_999px_999px_0] bg-white py-2 pr-[18px] pl-4 shadow-[0_6px_16px_rgba(0,0,0,.35)]">
        {/* Dark ink color, never the user color — must stay legible on white. */}
        <span className="font-extrabold italic text-[22px] tracking-[-.01em] text-[#15101f]">
          {name}
        </span>
        {badges.map((badge, i) => (
          <ChatBadge key={i} {...badge} />
        ))}
      </div>

      {/* Speech bubble: indented under the pill, with a small triangle pointing
          up into the name pill. */}
      <div className="ml-6 -mt-0.5">
        <div className="ml-[14px] h-0 w-0 border-x-[9px] border-b-[9px] border-x-transparent border-b-panel" />
        <div className="max-w-[560px] rounded-[14px] border border-brand/[.22] bg-panel px-[18px] py-3">
          <span className="text-[20px] font-medium leading-[1.35] break-words text-brand-muted">
            {parts.map((part, i) =>
              part.kind === "emote" ? (
                <img
                  key={i}
                  src={part.url}
                  alt=""
                  className="inline-block h-6 w-6 align-middle"
                />
              ) : (
                <span key={i}>{part.text}</span>
              ),
            )}
          </span>
        </div>
      </div>
    </div>
  );
}

export default ChatBubble;
