// ---------------------------------------------------------------------------
// Coworking Overlay — the single smart container for the live overlay.
//
// This is the one OBS browser source: a transparent 1920x1080 stage with the
// camera showing through the right-side cutout. It REPLACES the old split
// Chat + Frame overlays — instead of two sources picked by URL path, one
// Coworking Overlay composes the chat column, the camera window, and the
// footer bar (Now Playing + Fun Facts / Event Alerts) together.
//
// It is the only place that wires data into the headless ui/ layer: it owns the
// hooks (chat list, footer-bar arbitration, now-playing), subscribes the single
// SSE feed once via useOverlayFeed, and maps wire DTOs into view-model props
// (toChatBubble / toEventAlert) so the ui/ components stay feed-decoupled.
// ---------------------------------------------------------------------------

import { useEffect } from "react";
import { useChatMessages } from "../hooks/useChatMessages";
import { useFooterBar } from "../hooks/useFooterBar";
import { useNowPlaying } from "../hooks/useNowPlaying";
import { useOverlayFeed } from "../hooks/useOverlayFeed";
import { toChatBubble } from "../lib/chat";
import { toEventAlert } from "../lib/eventAlert";
import { FUN_FACTS, FUN_FACT_HIGHLIGHTS } from "../lib/funFacts";
import { Stage } from "../ui/Stage";
import TopBar from "../ui/TopBar";
import ChatPanel from "../ui/chat/ChatPanel";
import ChatBubble from "../ui/chat/ChatBubble";
import FooterBar from "../ui/footer/FooterBar";
import NowPlaying from "../ui/footer/NowPlaying";
import FunFactsTicker from "../ui/footer/FunFactsTicker";
import EventAlert from "../ui/footer/EventAlert";

// How long an Alert holds the footer bar before yielding back to Fun Facts.
// Mirrors the reference design / old FrameOverlay ALERT_DURATION_MS.
const ALERT_DURATION_MS = 6000;

export default function CoworkingOverlay() {
  const chat = useChatMessages();
  const footer = useFooterBar();
  const np = useNowPlaying();

  // Single SSE seam: fan the tagged-union feed into the chat list, the
  // footer-bar arbitration, and the now-playing widget. ui/ never touches the
  // network.
  useOverlayFeed({
    onChatMessage: chat.push,
    onChatDeleted: chat.remove,
    onStreamEvent: footer.pushEvent,
    onNowPlaying: np.onDto,
  });

  // Alert hold timer: whenever an Alert becomes current, hold it for
  // ALERT_DURATION_MS then advance the queue (back to Fun Facts if empty).
  // Keyed on footer.state.current so each queued Alert gets its own full hold;
  // the cleanup clears the pending timer if the current Alert changes/unmounts.
  const currentEvent = footer.state.current;
  useEffect(() => {
    if (!currentEvent) return;
    const id = setTimeout(footer.endAlert, ALERT_DURATION_MS);
    return () => clearTimeout(id);
  }, [currentEvent, footer.endAlert]);

  // Footer-right slot: an Alert takes over the ticker region while one is
  // playing; otherwise the Fun Facts ticker idles. viewerCountUpdate maps to
  // null (silent metric), so fall back to the ticker if the mapper opts out.
  const alert =
    footer.state.mode === "alert" && currentEvent
      ? toEventAlert(currentEvent)
      : null;

  return (
    <Stage>
      <TopBar channel="/DanielHe4rt" />

      <ChatPanel title="CHAT AO VIVO">
        {chat.messages.map((m) => (
          <ChatBubble key={m.msgId} {...toChatBubble(m)} />
        ))}
      </ChatPanel>

      

      <FooterBar>
        <NowPlaying track={np.track} />
        {alert ? (
          <EventAlert {...alert} />
        ) : (
          <FunFactsTicker facts={FUN_FACTS} highlightTerms={FUN_FACT_HIGHLIGHTS} />
        )}
      </FooterBar>
    </Stage>
  );
}
