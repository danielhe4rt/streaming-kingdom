//! Axum integration test for the Overlay `http` renderer.
//!
//! Boots the real router on an ephemeral port, then asserts the tracer-bullet
//! contract: `GET /overlay/chat` serves the embedded Chat Overlay page and
//! `GET /overlay/feed` streams a broadcast chat message as the M3 Feed DTO.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::domain::{
    ChatBadge, ChatMessage, ChatMessageDeleted, ChatSignal, EmoteSpan, StreamEvent,
};

use super::{routes, OverlayState};

struct TestServer {
    addr: std::net::SocketAddr,
    chat_tx: broadcast::Sender<ChatSignal>,
    event_tx: broadcast::Sender<StreamEvent>,
}

async fn boot_server() -> TestServer {
    let (chat_tx, _) = broadcast::channel(16);
    let (event_tx, _) = broadcast::channel(16);
    let app = routes::router(OverlayState {
        chat_tx: chat_tx.clone(),
        event_tx: event_tx.clone(),
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
    }
}

#[tokio::test]
async fn chat_overlay_serves_embedded_page() {
    let server = boot_server().await;
    let addr = server.addr;

    let body = reqwest::get(format!("http://{addr}/overlay/chat"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

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
async fn frame_overlay_serves_embedded_page() {
    let server = boot_server().await;
    let addr = server.addr;

    // The Frame Overlay reuses the same embedded SPA as the Chat Overlay; the
    // React entrypoint switches on the URL path. So the page mounts the same
    // React root and asset bundle, just at /overlay/frame.
    let body = reqwest::get(format!("http://{addr}/overlay/frame"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(body.contains("<div id=\"root\">"), "page should mount React root");
    assert!(
        body.contains("/overlay/assets/"),
        "page should reference the embedded asset bundle, got: {body}"
    );
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

    // A donation fires — the Frame Overlay's Footer Bar turns this into an Alert.
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
