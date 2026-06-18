import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The overlays are served by the Rust `http` renderer under the `/overlay/`
// path prefix, with the built bundle embedded into the binary. We set `base`
// accordingly so the emitted asset URLs resolve when mounted there.
//
// In dev, the Vite dev server runs on its own origin and talks to the Rust
// SSE endpoint at http://127.0.0.1:1337/overlay/feed (CORS is enabled there).
export default defineConfig({
  base: "/overlay/",
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
