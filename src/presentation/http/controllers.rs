//! Overlay controllers — the handlers behind the `http` renderer's routes.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{BroadcastStream, WatchStream};

use super::assets;
use super::resources::FeedEvent;
use super::OverlayState;

/// `GET /overlay/feed` — the Overlay Feed: one SSE stream carrying enriched chat
/// plus stream events, each as the M3 [`FeedEvent`] DTO.
///
/// Three sources are merged into a single feed: the chat signal channel (new
/// messages + CLEARMSG deletions), the stream-event channel (donation / sub /
/// raid …), and the now-playing *state* watch (current Spotify track). One
/// source, one Overlay — the Coworking Overlay's chat panel reads
/// `chatMessage`s, its Footer Bar reads `streamEvent`s, and its Now Playing
/// widget reads `nowPlaying` off the *same* stream. The watch yields its
/// current value first, so a fresh SSE connection immediately gets the track.
/// Lagged frames (buffer overflow under bursts) are skipped rather than
/// terminating the stream, so the Overlays keep rendering.
pub async fn feed(State(state): State<OverlayState>) -> Response {
    let chat = BroadcastStream::new(state.chat_tx.subscribe()).filter_map(|result| match result {
        Ok(signal) => Some(FeedEvent::from_signal(&signal)),
        // Lagged: the consumer fell behind; drop the gap and keep streaming.
        Err(_) => None,
    });

    let events = BroadcastStream::new(state.event_tx.subscribe()).filter_map(|result| match result {
        Ok(event) => Some(FeedEvent::stream(&event)),
        Err(_) => None,
    });

    // Ambient now-playing state: WatchStream emits the current value first (so a
    // new connection immediately sees the track), then each subsequent change.
    // Skip the *leading* `None` (the initial "no track yet" watch value): emitting
    // a stale `stopped` frame before any track has played would falsely tell the
    // widget the player stopped. Once a real track has been seen, a later `None`
    // (the track stopped) does pass through as a `now_playing_cleared` frame.
    let now_playing = WatchStream::new(state.now_playing.clone())
        .skip_while(|track| track.is_none())
        .map(|track| match track {
            Some(track) => FeedEvent::now_playing(&track),
            None => FeedEvent::now_playing_cleared(),
        });

    // Ambient voice-roster state: same WatchStream pattern as now-playing. Skip
    // the leading `None` (the watch is seeded with `None`); once a real roster has
    // been seen, a later `None` (left voice / Discord closed) passes through as a
    // cleared roster (`channelId: null`, `members: []`) so the widget empties.
    let voice_roster = WatchStream::new(state.voice_roster_tx.subscribe())
        .skip_while(|roster| roster.is_none())
        .map(|roster| match roster {
            Some(roster) => FeedEvent::voice_roster(&roster),
            None => FeedEvent::voice_roster(&crate::domain::VoiceRoster::default()),
        });

    // Interleave all sources onto one SSE stream as they arrive.
    let stream = chat
        .merge(events)
        .merge(now_playing)
        .merge(voice_roster)
        .map(|feed_event| Ok::<_, Infallible>(Event::default().data(feed_event.to_json())));

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );

    // Allow the Vite dev server (separate origin) to read the feed in dev; the
    // embedded build is same-origin so this is a harmless no-op there.
    (
        [(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        sse,
    )
        .into_response()
}

/// `GET /overlay/coworking` — the single full-screen Coworking Overlay page
/// (embedded `dist/index.html`). This replaces the former Chat + Frame overlays:
/// one full-screen overlay now paints the camera frame, chat panel, and Footer
/// Bar together, all driven off the same Overlay Feed.
pub async fn coworking() -> Html<&'static str> {
    Html(assets::index_html())
}

/// `GET /overlay/assets/{*path}` — embedded JS/CSS for the Overlay bundle.
pub async fn asset(Path(path): Path<String>) -> Response {
    match assets::asset(&format!("assets/{path}")) {
        Some((content_type, bytes)) => {
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}
