// ---------------------------------------------------------------------------
// useChatMessages — pure chat-list state (no SSE).
//
// Holds the bounded list the ChatPanel renders. It is deliberately transport-
// agnostic: useOverlayFeed drives it via push()/remove(), but tests (and any
// other source) can drive it directly. The cap keeps the panel from
// overflowing the camera-side column on a busy chat.
// ---------------------------------------------------------------------------

import { useCallback, useState } from "react";
import type { ChatMessageDto } from "../feed";

// Cap the visible backlog. push() trims the oldest beyond this so the panel's
// height stays bounded regardless of chat velocity.
const MAX = 8;

export interface ChatMessages {
  messages: ChatMessageDto[];
  push(dto: ChatMessageDto): void;
  remove(msgId: string): void;
}

export function useChatMessages(): ChatMessages {
  const [messages, setMessages] = useState<ChatMessageDto[]>([]);

  const push = useCallback((dto: ChatMessageDto) => {
    // Append, then drop the oldest beyond MAX (keep the tail).
    setMessages((prev) => [...prev, dto].slice(-MAX));
  }, []);

  // CLEARMSG moderation: drop the single deleted message by id.
  const remove = useCallback((msgId: string) => {
    setMessages((prev) => prev.filter((m) => m.msgId !== msgId));
  }, []);

  return { messages, push, remove };
}
