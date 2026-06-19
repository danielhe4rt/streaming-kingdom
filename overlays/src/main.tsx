import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import CoworkingOverlay from "./overlays/CoworkingOverlay";
import "./index.css";

// One embedded SPA serves the single Coworking Overlay (one OBS browser source,
// camera behind the transparent cutout). The Rust `http` renderer serves this
// same bundle; there is no longer a chat-vs-frame path to pick.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <CoworkingOverlay />
  </StrictMode>,
);
