// ---------------------------------------------------------------------------
// useOverlayFeed — the single SSE seam for an Overlay.
//
// Opens ONE EventSource against FEED_URL and demultiplexes the tagged-union
// FeedEventDto stream into typed callbacks (chat / deletion / stream event).
// This is the ONLY hook that talks to the network: keeping the EventSource
// here lets the headless ui/ components stay decoupled from ../feed.
//
// Handlers are stashed in a ref so the effect mounts the source exactly once
// (and tears it down on unmount) without re-subscribing every render when the
// caller passes fresh closures.
// ---------------------------------------------------------------------------

import { useEffect, useRef } from "react";
import {
  FEED_URL,
  type ChatMessageDto,
  type FeedEventDto,
  type NowPlayingDto,
  type StreamEventDto,
  type VoiceRosterDto,
} from "../feed";

// Handlers are optional so a single-concern overlay (e.g. the standalone voice
// dock) can subscribe to just the frames it cares about. Unhandled kinds are
// silently ignored.
export interface OverlayFeedHandlers {
  onChatMessage?(dto: ChatMessageDto): void;
  onChatDeleted?(msgId: string): void;
  onStreamEvent?(dto: StreamEventDto): void;
  onNowPlaying?(dto: NowPlayingDto): void;
  onVoiceRoster?(dto: VoiceRosterDto): void;
}

export function useOverlayFeed(handlers: OverlayFeedHandlers): void {
  // Latest handlers, read at event time so the effect never re-runs on a new
  // closure identity.
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => {
    const source = new EventSource(FEED_URL);

    source.onmessage = (event) => {
      // Best-effort feed: a malformed frame must not kill the stream.
      let dto: FeedEventDto;
      try {
        dto = JSON.parse(event.data) as FeedEventDto;
      } catch {
        return;
      }

      const h = handlersRef.current;
      switch (dto.kind) {
        case "chatMessage":
          h.onChatMessage?.(dto);
          break;
        case "chatMessageDeleted":
          h.onChatDeleted?.(dto.msgId);
          break;
        case "streamEvent":
          h.onStreamEvent?.(dto);
          break;
        case "nowPlaying":
          h.onNowPlaying?.(dto);
          break;
        case "voiceRoster":
          h.onVoiceRoster?.(dto);
          break;
      }
    };

    return () => source.close();
  }, []);
}
