// ---------------------------------------------------------------------------
// Now-playing view-model mapper
//
// The boundary between the feed (NowPlayingDto wire shape) and the headless
// ui/footer/NowPlaying widget. ui/ never imports feed DTOs; this is where the
// ambient Spotify state is flattened into the props NowPlaying renders.
//
// A "stopped" status (player gone/idle, empty strings) maps to null so the
// widget falls back to its placeholder. "playing" vs "paused" only differ by
// whether the disc/EQ animations run, so we collapse them into `isPlaying`.
// ---------------------------------------------------------------------------

import type { NowPlayingDto } from "../feed";
import type { NowPlayingTrack } from "../ui/footer/NowPlaying";

export function toNowPlaying(dto: NowPlayingDto): NowPlayingTrack | null {
  if (dto.status === "stopped") return null;

  return {
    title: dto.title,
    artist: dto.artist,
    artUrl: dto.artUrl,
    isPlaying: dto.status === "playing",
  };
}
