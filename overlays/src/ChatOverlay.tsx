import { useEffect, useRef, useState } from "react";
import type { ChatMessageDto, FeedEventDto, FragmentDto } from "./feed";

// Keep the rendered chat bounded so the Overlay never grows a scrollbar and
// old messages age out. Later slices add time-based hideAfter; for the tracer
// bullet a simple cap is enough.
const MAX_MESSAGES = 30;

// In dev the Vite server runs on its own origin, so point the feed at the Rust
// server explicitly. In the embedded build the page is same-origin, so a
// relative path is correct.
const FEED_URL = import.meta.env.DEV
  ? "http://127.0.0.1:1337/overlay/feed"
  : "/overlay/feed";

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

function ChatLine({ message }: { message: ChatMessageDto }) {
  return (
    <div className="px-3 py-1 text-2xl leading-snug [text-shadow:_0_2px_4px_rgb(0_0_0_/_90%)]">
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
