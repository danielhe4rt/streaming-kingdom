// ChatBadge: a single Twitch chat badge rendered as the real badge image.
// Headless/presentational — props only, no feed coupling. Sits inside the
// white name-pill of a ChatBubble. The reference used hardcoded colored icon
// chips (YT/knight/check); we deliberately render the actual badge image so
// the overlay shows whatever badges the chatter really has.
export interface ChatBadgeProps {
  url: string;
  label: string;
}

export function ChatBadge({ url, label }: ChatBadgeProps) {
  return (
    // ~23x23 slot matching the reference pill icons (rounded-[7px]). No colored
    // background — the badge artwork carries its own visuals. object-contain so
    // non-square badges aren't distorted; flex-none keeps it from shrinking.
    <img
      src={url}
      alt={label}
      title={label}
      className="h-[23px] w-[23px] flex-none rounded-[7px] object-contain"
    />
  );
}

export default ChatBadge;
