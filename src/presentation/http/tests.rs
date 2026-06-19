//! Axum integration test for the Overlay `http` renderer.
//!
//! Boots the real router on an ephemeral port, then asserts the tracer-bullet
//! contract: `GET /overlay/coworking` serves the embedded Coworking Overlay page
//! and `GET /overlay/feed` streams a broadcast chat message as the M3 Feed DTO.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};

use crate::domain::{
    ChatBadge, ChatMessage, ChatMessageDeleted, ChatSignal, EmoteSpan, NowPlaying, PlaybackStatus,
    StreamEvent,
};

use super::{routes, OverlayState};

struct TestServer {
    addr: std::net::SocketAddr,
    chat_tx: broadcast::Sender<ChatSignal>,
    event_tx: broadcast::Sender<StreamEvent>,
    now_playing_tx: watch::Sender<Option<NowPlaying>>,
}

async fn boot_server() -> TestServer {
    let (chat_tx, _) = broadcast::channel(16);
    let (event_tx, _) = broadcast::channel(16);
    let (now_playing_tx, now_playing_rx) = watch::channel(None);
    let app = routes::router(OverlayState {
        chat_tx: chat_tx.clone(),
        event_tx: event_tx.clone(),
        now_playing: now_playing_rx,
    });

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start accepting connections.
    tokio::time::sleep(Duration::from_millis(50)).await;
    TestServer {
        addr,
        chat_tx,
        event_tx,
        now_playing_tx,
    }
}

#[tokio::test]
async fn coworking_overlay_serves_embedded_page() {
    let server = boot_server().await;
    let addr = server.addr;

    let response = reqwest::get(format!("http://{addr}/overlay/coworking"))
        .await
        .unwrap();
    assert_eq!(response.status(), 200, "coworking overlay should return 200");

    let body = response.text().await.unwrap();

    assert!(body.contains("<div id=\"root\">"), "page should mount React root");
    assert!(
        body.contains("/overlay/assets/"),
        "page should reference the embedded asset bundle, got: {body}"
    );
}

#[tokio::test]
async fn feed_streams_chat_message_as_dto() {
    let server = boot_server().await;
    let (addr, chat_tx) = (server.addr, server.chat_tx);

    // Open a raw TCP connection so we can read the SSE stream incrementally
    // without waiting for the (never-ending) response to complete.
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    // Let the SSE handler subscribe before we publish.
    tokio::time::sleep(Duration::from_millis(50)).await;

    chat_tx
        .send(ChatSignal::Message(
            ChatMessage::from_text(
                "msg-42",
                "danielhe4rt",
                Some("#FF7F50"),
                "rustlang",
                "hello overlay",
            )
            .with_badges(vec![ChatBadge {
                set: "moderator".into(),
                version: "1".into(),
                url: Some("https://cdn/mod.png".into()),
            }]),
        ))
        .unwrap();

    // Read until we see the SSE data frame for our message (bounded by timeout).
    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("msg-42") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for SSE frame; got: {acc}");
    assert!(acc.contains("data:"), "expected an SSE data frame, got: {acc}");
    assert!(acc.contains("\"kind\":\"chatMessage\""), "got: {acc}");
    assert!(acc.contains("\"msgId\":\"msg-42\""), "got: {acc}");
    assert!(acc.contains("\"color\":\"#FF7F50\""), "got: {acc}");
    assert!(acc.contains("hello overlay"), "got: {acc}");
    // The resolved badge url rides along on the feed DTO.
    assert!(acc.contains("\"setId\":\"moderator\""), "got: {acc}");
    assert!(acc.contains("\"url\":\"https://cdn/mod.png\""), "got: {acc}");
}

#[tokio::test]
async fn feed_carries_ordered_emote_fragments() {
    let server = boot_server().await;
    let (addr, chat_tx) = (server.addr, server.chat_tx);

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // "hey Kappa" → text run then a single native emote fragment.
    chat_tx
        .send(ChatSignal::Message(ChatMessage::from_fragments(
            "msg-emote",
            "randers",
            Some("#19E6E6"),
            "rustlang",
            "hey Kappa",
            &[EmoteSpan::new("25", 4, 9)],
        )))
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("msg-emote") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for SSE frame; got: {acc}");
    // Ordered fragments: a text run carrying "hey ", then an emote with its
    // CDN url derived from the id, in original order.
    assert!(acc.contains("\"kind\":\"text\""), "got: {acc}");
    assert!(acc.contains("\"kind\":\"emote\""), "got: {acc}");
    assert!(acc.contains("\"id\":\"25\""), "got: {acc}");
    assert!(
        acc.contains("static-cdn.jtvnw.net/emoticons/v2/25/default/dark/3.0"),
        "got: {acc}"
    );
    // The text run precedes the emote in the serialized fragment array.
    let text_at = acc.find("\"kind\":\"text\"").unwrap();
    let emote_at = acc.find("\"kind\":\"emote\"").unwrap();
    assert!(text_at < emote_at, "text fragment should precede emote; got: {acc}");
}

#[tokio::test]
async fn feed_streams_single_message_delete_as_dto() {
    let server = boot_server().await;
    let (addr, chat_tx) = (server.addr, server.chat_tx);

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    // Let the SSE handler subscribe before we publish the moderation signal.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // A moderator deletes a single message (CLEARMSG) — the same broadcast
    // carries the delete to the Overlay Feed.
    chat_tx
        .send(ChatSignal::Deleted(ChatMessageDeleted::new("msg-42")))
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("chatMessageDeleted") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for SSE frame; got: {acc}");
    assert!(acc.contains("data:"), "expected an SSE data frame, got: {acc}");
    assert!(
        acc.contains("\"kind\":\"chatMessageDeleted\""),
        "got: {acc}"
    );
    assert!(acc.contains("\"msgId\":\"msg-42\""), "got: {acc}");
}

#[tokio::test]
async fn feed_streams_stream_event_as_dto() {
    let server = boot_server().await;
    let (addr, event_tx) = (server.addr, server.event_tx);

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    // Let the SSE handler subscribe to the stream-event broadcast before we
    // publish, otherwise the broadcast drops the event with no receivers.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // A donation fires — the Coworking Overlay's Footer Bar turns this into an Alert.
    event_tx
        .send(StreamEvent::Donation {
            username: "danielhe4rt".into(),
            amount_cents: 500,
            message: "vai rust!".into(),
        })
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("streamEvent") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for SSE frame; got: {acc}");
    assert!(acc.contains("data:"), "expected an SSE data frame, got: {acc}");
    // The outer `kind` selects the stream-event branch; the inner `type` (the
    // flattened domain discriminant) selects the Alert template.
    assert!(acc.contains("\"kind\":\"streamEvent\""), "got: {acc}");
    assert!(acc.contains("\"type\":\"donation\""), "got: {acc}");
    assert!(acc.contains("\"username\":\"danielhe4rt\""), "got: {acc}");
    assert!(acc.contains("\"amountCents\":500"), "got: {acc}");
}

#[tokio::test]
async fn feed_streams_now_playing_as_dto() {
    let server = boot_server().await;
    let (addr, now_playing_tx) = (server.addr, server.now_playing_tx);

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();

    // Let the SSE handler subscribe to the watch before we push a new track.
    // (Unlike a broadcast, the watch retains the latest value — but a fresh push
    // exercises the change path the observer drives.)
    tokio::time::sleep(Duration::from_millis(50)).await;

    // A new Spotify track starts playing — the watch carries it as ambient state,
    // and the feed turns it into a `nowPlaying` DTO for the Now Playing widget.
    now_playing_tx
        .send(Some(NowPlaying {
            title: "Money".into(),
            artist: "Pink Floyd".into(),
            album: "The Dark Side of the Moon".into(),
            art_url: Some("https://i.scdn.co/image/abc".into()),
            status: PlaybackStatus::Playing,
        }))
        .unwrap();

    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("nowPlaying") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for SSE frame; got: {acc}");
    assert!(acc.contains("data:"), "expected an SSE data frame, got: {acc}");
    assert!(acc.contains("\"kind\":\"nowPlaying\""), "got: {acc}");
    assert!(acc.contains("\"title\":\"Money\""), "got: {acc}");
    assert!(acc.contains("\"artist\":\"Pink Floyd\""), "got: {acc}");
    assert!(acc.contains("\"status\":\"playing\""), "got: {acc}");
    assert!(
        acc.contains("\"artUrl\":\"https://i.scdn.co/image/abc\""),
        "got: {acc}"
    );
}

#[tokio::test]
async fn dev_panel_serves_control_page() {
    let server = boot_server().await;
    let addr = server.addr;

    let response = reqwest::get(format!("http://{addr}/overlay/dev"))
        .await
        .unwrap();
    assert_eq!(response.status(), 200, "dev panel should return 200");

    let body = response.text().await.unwrap();
    assert!(body.contains("Overlay Dev"), "panel should render, got: {body}");
    assert!(
        body.contains("/overlay/dev/event/"),
        "panel should wire the emit routes, got: {body}"
    );
}

#[tokio::test]
async fn dev_event_route_injects_into_feed() {
    let server = boot_server().await;
    let addr = server.addr;

    // Subscribe to the feed first so the broadcast has a receiver.
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = "GET /overlay/feed HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    tokio::io::AsyncWriteExt::write_all(&mut stream, request.as_bytes())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Fire a fake donation through the dev route — it injects a StreamEvent into
    // the same broadcast the real Twitch/Livepix adapters feed.
    let fired = reqwest::get(format!("http://{addr}/overlay/dev/event/donation"))
        .await
        .unwrap();
    assert_eq!(fired.status(), 200, "dev event route should return 200");

    let mut buf = vec![0u8; 4096];
    let mut acc = String::new();
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            acc.push_str(&String::from_utf8_lossy(&buf[..n]));
            if acc.contains("streamEvent") {
                break;
            }
        }
    })
    .await;

    assert!(read.is_ok(), "timed out waiting for the injected event; got: {acc}");
    assert!(acc.contains("\"kind\":\"streamEvent\""), "got: {acc}");
    assert!(acc.contains("\"type\":\"donation\""), "got: {acc}");
}
