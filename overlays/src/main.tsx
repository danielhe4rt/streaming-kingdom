import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import CoworkingOverlay from "./overlays/CoworkingOverlay";
import VoiceOverlay from "./overlays/VoiceOverlay";
import "./index.css";

// One embedded SPA, two overlay pages picked by URL path. `/overlay/voice`
// renders the standalone Discord voice dock (its own OBS browser source / an
// embeddable transparent page); everything else renders the main Coworking
// overlay. Assets are absolute (`base: "/overlay/"`), so the same bundle serves
// both routes — the Rust `http` renderer returns this index.html for each.
const isVoice = window.location.pathname.endsWith("/voice");

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isVoice ? <VoiceOverlay /> : <CoworkingOverlay />}</StrictMode>,
);
