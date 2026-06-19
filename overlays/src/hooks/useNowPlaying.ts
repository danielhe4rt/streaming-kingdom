// ---------------------------------------------------------------------------
// useNowPlaying — feed-driven state for the Footer Bar's NowPlaying widget.
//
// Holds the latest Spotify track as ambient state and exposes `onDto`, which
// the container wires as useOverlayFeed's `onNowPlaying` handler. Each frame
// off the feed is mapped through toNowPlaying (a "stopped" status clears back
// to null, so the widget shows its placeholder). The hook stays tiny: just the
// latest-value state plus the setter — no network, no timers.
// ---------------------------------------------------------------------------

import { useCallback, useState } from "react";
import type { NowPlayingDto } from "../feed";
import { toNowPlaying } from "../lib/nowPlaying";
import type { NowPlayingTrack } from "../ui/footer/NowPlaying";

export interface UseNowPlaying {
  track: NowPlayingTrack | null;
  onDto(dto: NowPlayingDto): void;
}

export function useNowPlaying(): UseNowPlaying {
  const [track, setTrack] = useState<NowPlayingTrack | null>(null);

  const onDto = useCallback((dto: NowPlayingDto) => {
    setTrack(toNowPlaying(dto));
  }, []);

  return { track, onDto };
}
