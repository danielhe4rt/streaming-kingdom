import { useEffect, useRef, useState } from "react";
import type {
  ChatBadgeDto,
  ChatMessageDto,
  FeedEventDto,
  FragmentDto,
} from "./feed";
import { FEED_URL } from "./feed";

// Keep the rendered chat bounded so the Overlay never grows a scrollbar and
// old messages age out. Later slices add time-based hideAfter; for the tracer
// bullet a simple cap is enough.
const MAX_MESSAGES = 30;

function Fragment({ fragment }: { fragment: FragmentDto }) {
  if (fragment.kind === "emote") {
    return (
      <img
        src={fragment.url}
        alt=""
        className="inline-block h-6 w-6 align-middle"
      />
    );
  }
  return <span>{fragment.text}</span>;
}

function Badge({ badge }: { badge: ChatBadgeDto }) {
  return (
    <img
      src={badge.url}
      alt={`${badge.setId} badge`}
      title={badge.setId}
      className="mr-1 inline-block h-5 w-5 align-middle"
    />
  );
}

function ChatLine({ message }: { message: ChatMessageDto }) {
  return (
    <div className="px-3 py-1 text-2xl leading-snug [text-shadow:_0_2px_4px_rgb(0_0_0_/_90%)]">
      {message.badges.map((badge) => (
        <Badge key={`${badge.setId}/${badge.version}`} badge={badge} />
      ))}
      <span className="font-bold" style={{ color: message.color }}>
        {message.username}
      </span>
      <span className="text-white">: </span>
      <span className="text-white">
        {message.fragments.map((fragment, i) => (
          <Fragment key={i} fragment={fragment} />
        ))}
      </span>
    </div>
  );
}

export default function ChatOverlay() {
  const [messages, setMessages] = useState<ChatMessageDto[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const source = new EventSource(FEED_URL);

    source.addEventListener("message", (event) => {
      try {
        const dto = JSON.parse(event.data) as FeedEventDto;
        if (dto.kind === "chatMessage") {
          setMessages((prev) => {
            const next = [...prev, dto];
            return next.length > MAX_MESSAGES
              ? next.slice(next.length - MAX_MESSAGES)
              : next;
          });
        } else if (dto.kind === "chatMessageDeleted") {
          // Single-message moderation: drop the node with the matching msgId so
          // the deleted message vanishes from the Overlay in real time.
          setMessages((prev) => prev.filter((m) => m.msgId !== dto.msgId));
        }
      } catch {
        // Ignore malformed frames; the feed is best-effort for a display source.
      }
    });

    return () => source.close();
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="flex h-full w-full flex-col justify-end overflow-hidden">
      {messages.map((message) => (
        <ChatLine key={message.msgId} message={message} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
