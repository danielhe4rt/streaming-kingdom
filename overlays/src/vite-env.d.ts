/// <reference types="vite/client" />

// Pulls in Vite's ambient client types so `import.meta.env` (DEV/PROD/etc.) is
// typed during a standalone `tsc --noEmit` type-check. `vite build` injects
// these implicitly, but a bare tsc run does not — without this reference,
// feed.ts's `import.meta.env.DEV` fails with TS2339. This file lives under
// `src/` so the tsconfig's `"include": ["src"]` picks it up automatically.
