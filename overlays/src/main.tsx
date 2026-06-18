import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ChatOverlay from "./ChatOverlay";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ChatOverlay />
  </StrictMode>,
);
