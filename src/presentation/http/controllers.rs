//! Overlay controllers — the handlers behind the `http` renderer's routes.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use super::assets;
use super::resources::FeedEvent;
use super::OverlayState;

/// `GET /overlay/feed` — the Overlay Feed: one SSE stream carrying enriched chat
/// plus stream events, each as the M3 [`FeedEvent`] DTO.
///
/// Two broadcasts are merged into a single feed: the chat signal channel (new
/// messages + CLEARMSG deletions) and the stream-event channel (donation / sub /
/// raid …). One source, N Overlays — the Chat Overlay reads `chatMessage`s and
/// the Frame Overlay's Footer Bar reads `streamEvent`s off the *same* stream.
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

    // Interleave both broadcasts onto one SSE stream as they arrive.
    let stream = chat.merge(events).map(|feed_event| {
        Ok::<_, Infallible>(Event::default().data(feed_event.to_json()))
    });

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

/// `GET /overlay/chat` — the Chat Overlay page (embedded `dist/index.html`).
pub async fn chat() -> Html<&'static str> {
    Html(assets::index_html())
}

/// `GET /overlay/frame` — the Frame Overlay page. Same embedded SPA as the Chat
/// Overlay; the React entrypoint picks the Overlay from the URL path, so the
/// branding frame + Footer Bar render here while chat renders at `/overlay/chat`.
pub async fn frame() -> Html<&'static str> {
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
