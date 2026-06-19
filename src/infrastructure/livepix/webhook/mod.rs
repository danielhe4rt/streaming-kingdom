//! Livepix donation webhook server, sliced by concern.
//!
//! - [`payload`] — webhook callback + API response deserialization types
//! - [`server_state`] — shared `ServerState` threaded through the handlers
//! - [`oauth`] — `client_credentials` OAuth flow + token caching
//! - [`message_api`] — fetching the full donation message from the REST API
//! - [`handlers`] — Axum route handlers (health + donation ingest/fan-out)
//! - [`server`] — the spawn entry-point and per-session server lifecycle

mod handlers;
mod message_api;
mod oauth;
mod payload;
mod server;
mod server_state;

pub use server::spawn;
