// ---------------------------------------------------------------------------
// Chat view-model mapper
//
// The boundary between the feed (transport DTOs) and the headless ui/ chat
// components. ui/chat/* never imports feed DTOs; this is where a wire-shaped
// ChatMessageDto is flattened into the decoupled props ChatBubble renders.
// ---------------------------------------------------------------------------

import type { ChatMessageDto } from "../feed";
import type { ChatBubbleProps } from "../ui/chat/ChatBubble";

export function toChatBubble(dto: ChatMessageDto): ChatBubbleProps {
  return {
    name: dto.username,
    // The resolver already dropped badge misses, so every badge has a url.
    badges: dto.badges.map((b) => ({ url: b.url, label: b.setId })),
    // FragmentDto and ChatPart share the same discriminant ("text"/"emote"),
    // but we re-shape explicitly so the emote part carries only `url`.
    parts: dto.fragments.map((f) =>
      f.kind === "text"
        ? { kind: "text", text: f.text }
        : { kind: "emote", url: f.url },
    ),
  };
}
