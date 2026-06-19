//! Dev-only fake event emitter — a localhost tool to exercise the Overlay
//! without real Twitch / Livepix traffic.
//!
//! It injects synthetic stream events and chat messages straight into the same
//! broadcast channels the real adapters feed, so the Overlay Feed fans them to
//! an open Coworking Overlay exactly like production traffic. A tiny control
//! panel at `GET /overlay/dev` fires them with buttons. Events are built by the
//! shared `presentation::synthetic` module (ADR-0002).
//!
//! These routes are namespaced under `/overlay/dev` and the server is bound to
//! 127.0.0.1, so they are not reachable off-box. They are a testing aid; gate
//! them behind a config/env flag if you ever want them gone from a real stream.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::domain::{ChatSignal, MessageFragment};
use crate::presentation::synthetic::{self, SyntheticKind};

use super::OverlayState;

/// Map a URL path segment to a stream-event kind. Chat / now-playing / delete
/// are not stream events and are not exposed on this route.
fn kind_from_str(s: &str) -> Option<SyntheticKind> {
    Some(match s {
        "donation" => SyntheticKind::Donation,
        "sub" => SyntheticKind::Sub,
        "giftsub" => SyntheticKind::GiftSub,
        "follow" => SyntheticKind::Follow,
        "raid" => SyntheticKind::Raid,
        "cheer" => SyntheticKind::Cheer,
        "viewercount" => SyntheticKind::ViewerCount,
        _ => return None,
    })
}

/// `GET /overlay/dev/event/{kind}` — fire one synthetic stream event. Built by
/// the shared `presentation::synthetic` module (ADR-0002) so this panel and the
/// TUI Test events pane never drift.
pub async fn event(Path(kind): Path<String>, State(state): State<OverlayState>) -> Response {
    let Some(sk) = kind_from_str(&kind) else {
        return (StatusCode::BAD_REQUEST, format!("unknown event kind: {kind}")).into_response();
    };
    match synthetic::stream_event(sk, synthetic::next_seq()) {
        // Send error just means no Overlay is connected yet — harmless.
        Some(ev) => {
            let _ = state.event_tx.send(ev);
            (StatusCode::OK, format!("fired {kind}")).into_response()
        }
        None => (StatusCode::BAD_REQUEST, format!("not a stream event: {kind}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatQuery {
    text: Option<String>,
    user: Option<String>,
}

/// `GET /overlay/dev/chat?text=&user=` — push one synthetic chat message. The
/// preset comes from the shared builder; `text`/`user` override it when present.
pub async fn chat(Query(q): Query<ChatQuery>, State(state): State<OverlayState>) -> Response {
    let mut msg = synthetic::chat_message(synthetic::next_seq());
    if let Some(user) = q.user {
        msg.username = user;
    }
    if let Some(text) = q.text {
        msg.fragments = vec![MessageFragment::Text(text)];
    }

    let _ = state.chat_tx.send(ChatSignal::Message(msg));
    (StatusCode::OK, "fired chat").into_response()
}

/// `GET /overlay/dev` — the control panel: buttons that fetch the emit routes.
pub async fn panel() -> Html<&'static str> {
    Html(PANEL_HTML)
}

const PANEL_HTML: &str = r##"<!doctype html>
<html lang="pt-br">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Overlay Dev — fake events</title>
<style>
  :root { color-scheme: dark; }
  body { margin:0; font-family: system-ui, sans-serif; background:#12081f; color:#e9ddff;
         display:flex; flex-direction:column; align-items:center; gap:24px; padding:40px 16px; }
  h1 { font-size:20px; letter-spacing:.04em; margin:0; color:#c9a4ff; text-transform:uppercase; }
  p.hint { margin:0; color:#9b86c0; font-size:13px; max-width:520px; text-align:center; }
  .grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:12px; width:100%; max-width:620px; }
  button { cursor:pointer; border:1px solid rgba(201,164,255,.28); background:#241338; color:#e9ddff;
           border-radius:12px; padding:16px 12px; font-size:15px; font-weight:700; transition:transform .08s, background .15s; }
  button:hover { background:#321a4d; }
  button:active { transform:scale(.96); }
  .accent { border-color:#8b2fe8; box-shadow:0 0 0 1px rgba(139,47,232,.3) inset; }
  .row { display:flex; gap:8px; width:100%; max-width:620px; }
  .row input { flex:1; border-radius:12px; border:1px solid rgba(201,164,255,.28); background:#1a0f2b;
               color:#fff; padding:0 14px; font-size:15px; }
  #log { width:100%; max-width:620px; min-height:24px; font-size:13px; color:#1ed760; text-align:center; }
</style>
</head>
<body>
  <h1>● Overlay Dev — fake events</h1>
  <p class="hint">Abra a Coworking Overlay em outra aba/OBS (<code>/overlay/coworking</code>) e dispare eventos aqui. Eles entram nos mesmos canais que o Twitch/Livepix alimentam.</p>

  <div class="grid">
    <button class="accent" data-ev="donation">💸 Donation</button>
    <button class="accent" data-ev="sub">💜 Sub</button>
    <button data-ev="giftsub">🎁 Gift Sub</button>
    <button data-ev="follow">⭐ Follow</button>
    <button data-ev="raid">⚡ Raid</button>
    <button data-ev="cheer">💎 Cheer (bits)</button>
    <button data-ev="viewercount">👁 Viewer count</button>
    <button id="burst">🔥 Burst (5 aleatórios)</button>
  </div>

  <div class="row">
    <input id="chatText" placeholder="mensagem de chat (vazio = aleatória)">
    <button id="chatBtn" class="accent">💬 Enviar chat</button>
  </div>

  <div id="log">pronto.</div>

<script>
  const log = (m) => { document.getElementById("log").textContent = m; };
  const fire = async (kind) => {
    try { const r = await fetch("/overlay/dev/event/" + kind); log(await r.text()); }
    catch (e) { log("erro: " + e); }
  };
  document.querySelectorAll("button[data-ev]").forEach((b) =>
    b.addEventListener("click", () => fire(b.dataset.ev)));

  document.getElementById("chatBtn").addEventListener("click", async () => {
    const t = document.getElementById("chatText").value.trim();
    const url = "/overlay/dev/chat" + (t ? "?text=" + encodeURIComponent(t) : "");
    try { const r = await fetch(url); log(await r.text()); } catch (e) { log("erro: " + e); }
  });

  document.getElementById("burst").addEventListener("click", async () => {
    const kinds = ["donation","sub","giftsub","follow","raid","cheer"];
    for (let i = 0; i < 5; i++) {
      fire(kinds[Math.floor(Math.random() * kinds.length)]);
      await new Promise((r) => setTimeout(r, 700));
    }
    log("burst enviado.");
  });
</script>
</body>
</html>
"##;
