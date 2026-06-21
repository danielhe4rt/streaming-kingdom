//! Overlay routes — the Laravel-like route table for the `http` renderer.

use axum::Router;
use axum::routing::get;

use super::OverlayState;
use super::controllers;
use super::dev;

/// Build the Overlay router. Kept separate from the server bootstrap so tests
/// can boot the exact same routes without binding a socket.
pub fn router(state: OverlayState) -> Router {
    Router::new()
        .route("/overlay/feed", get(controllers::feed))
        .route("/overlay/coworking", get(controllers::coworking))
        // Dev-only fake event emitter (localhost). See http::dev.
        .route("/overlay/dev", get(dev::panel))
        .route("/overlay/dev/event/voiceRoster", get(dev::voice_roster))
        .route("/overlay/dev/event/:kind", get(dev::event))
        .route("/overlay/dev/chat", get(dev::chat))
        .route("/overlay/assets/*path", get(controllers::asset))
        .with_state(state)
}
