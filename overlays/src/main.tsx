import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ChatOverlay from "./ChatOverlay";
import FrameOverlay from "./FrameOverlay";
import "./index.css";

// One embedded SPA serves every Overlay; the Rust `http` renderer serves this
// same bundle at `/overlay/chat` and `/overlay/frame`. We pick the Overlay from
// the URL path so a single build covers both browser sources.
function pickOverlay() {
  if (window.location.pathname.endsWith("/overlay/frame")) {
    return <FrameOverlay />;
  }
  return <ChatOverlay />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>{pickOverlay()}</StrictMode>,
);
