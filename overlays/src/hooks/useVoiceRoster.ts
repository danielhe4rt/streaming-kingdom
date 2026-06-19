// ---------------------------------------------------------------------------
// useVoiceRoster — feed-driven state for the Discord voice-roster widget.
//
// Holds the latest roster snapshot as ambient state and exposes `onDto`, which
// the container wires as useOverlayFeed's `onVoiceRoster` handler. Each frame
// off the feed is mapped through toVoiceRoster (a null channelId clears back to
// null, so the widget renders nothing). The hook stays tiny: just the latest-
// value state plus the setter — no network, no timers.
// ---------------------------------------------------------------------------

import { useCallback, useState } from "react";
import type { VoiceRosterDto } from "../feed";
import { toVoiceRoster } from "../lib/voiceRoster";
import type { VoiceRoster } from "../ui/voice/VoiceRoster";

export interface UseVoiceRoster {
  roster: VoiceRoster | null;
  onDto(dto: VoiceRosterDto): void;
}

export function useVoiceRoster(): UseVoiceRoster {
  const [roster, setRoster] = useState<VoiceRoster | null>(null);

  const onDto = useCallback((dto: VoiceRosterDto) => {
    setRoster(toVoiceRoster(dto));
  }, []);

  return { roster, onDto };
}
