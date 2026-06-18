//! Overlay routes — the Laravel-like route table for the `http` renderer.

use axum::Router;
use axum::routing::get;

use super::controllers;
use super::OverlayState;

/// Build the Overlay router. Kept separate from the server bootstrap so tests
/// can boot the exact same routes without binding a socket.
pub fn router(state: OverlayState) -> Router {
    Router::new()
        .route("/overlay/feed", get(controllers::feed))
        .route("/overlay/chat", get(controllers::chat))
        .route("/overlay/frame", get(controllers::frame))
        .route("/overlay/assets/*path", get(controllers::asset))
        .with_state(state)
}
