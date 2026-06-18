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

/// `GET /overlay/feed` — the Overlay Feed: an SSE stream of chat as the M3 DTO.
///
/// Each broadcast `ChatSignal` (a new message or a CLEARMSG deletion) is mapped
/// to a [`FeedEvent`] and emitted as one SSE `data:` frame. Lagged frames
/// (buffer overflow under bursts) are skipped rather than terminating the
/// stream, so the Overlay keeps rendering.
pub async fn feed(State(state): State<OverlayState>) -> Response {
    let rx = state.chat_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(signal) => {
            let json = FeedEvent::from_signal(&signal).to_json();
            Some(Ok::<_, Infallible>(Event::default().data(json)))
        }
        // Lagged: the consumer fell behind; drop the gap and keep streaming.
        Err(_) => None,
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

/// `GET /overlay/assets/{*path}` — embedded JS/CSS for the Overlay bundle.
pub async fn asset(Path(path): Path<String>) -> Response {
    match assets::asset(&format!("assets/{path}")) {
        Some((content_type, bytes)) => {
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}
