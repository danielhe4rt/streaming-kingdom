//! Embedded Overlay assets.
//!
//! The React + Vite + Tailwind bundle (built from `overlays/` with Bun) is
//! embedded into the binary at compile time per ADR-0002, so distribution stays
//! a single file. The build pipeline runs `bun run build` before `cargo build`
//! so `overlays/dist` exists when this macro expands.

use include_dir::{Dir, include_dir};

/// The built Coworking Overlay bundle (`overlays/dist`).
static DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/overlays/dist");

/// The Coworking Overlay HTML entrypoint (`dist/index.html`).
pub fn index_html() -> &'static str {
    DIST.get_file("index.html")
        .and_then(|f| f.contents_utf8())
        .unwrap_or("<!doctype html><title>Coworking Overlay</title><p>overlay bundle missing</p>")
}

/// A built asset (JS/CSS) by its path relative to `dist/`, with a best-effort
/// content type. Returns `None` when the asset isn't part of the bundle.
pub fn asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    let file = DIST.get_file(path)?;
    let content_type = content_type_for(path);
    Some((content_type, file.contents()))
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}
