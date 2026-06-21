// ---------------------------------------------------------------------------
// VoiceOverlay — the standalone Discord voice page served at /overlay/voice.
//
// A second, minimal OBS browser source / embeddable transparent page. It reuses
// the exact same SSE feed and roster hook as the Coworking overlay, but paints
// only the floating "N NA SALA" dock (VoiceDock) — nothing else, so it drops
// cleanly into any scene or <iframe>. Layout is chosen via `?layout=row|col`
// (default `col`, matching the mock).
// ---------------------------------------------------------------------------

import { useOverlayFeed } from "../hooks/useOverlayFeed";
import { useVoiceRoster } from "../hooks/useVoiceRoster";
import VoiceDock, { type DockLayout } from "../ui/voice/VoiceDock";

function layoutFromQuery(): DockLayout {
  return new URLSearchParams(window.location.search).get("layout") === "row"
    ? "row"
    : "col";
}

export default function VoiceOverlay() {
  const voice = useVoiceRoster();

  // Single-concern subscription: only the voiceRoster frames matter here.
  useOverlayFeed({ onVoiceRoster: voice.onDto });

  return <VoiceDock roster={voice.roster} layout={layoutFromQuery()} />;
}
