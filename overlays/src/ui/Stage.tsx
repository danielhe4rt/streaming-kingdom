import { useEffect, useState } from "react";

// ---------------------------------------------------------------------------
// Stage — the fixed 1920x1080 design canvas.
//
// Every zone/slot in this overlay is positioned with absolute coordinates that
// assume a 1920x1080 viewport (the reference design's outer container). OBS
// renders the browser source at exactly that size, so on-stream the scale is 1.
// But to keep the overlay legible while previewing in an arbitrary browser
// window, we scale the inner canvas to fit while preserving aspect ratio.
//
// The outer wrapper is fully transparent (OBS composites the camera/scene
// behind it); only the painted zones inside `children` are visible.
// ---------------------------------------------------------------------------

export interface StageProps {
  children: React.ReactNode;
}

const STAGE_W = 1920;
const STAGE_H = 1080;

// Contain-fit scale: the canvas never overflows the window in either axis.
function fitScale(): number {
  return Math.min(window.innerWidth / STAGE_W, window.innerHeight / STAGE_H);
}

export function Stage({ children }: StageProps) {
  const [scale, setScale] = useState(fitScale);

  useEffect(() => {
    const onResize = () => setScale(fitScale());
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return (
    // Transparent full-viewport wrapper, flex-centered so the scaled canvas
    // sits in the middle when the window aspect ratio differs from 16:9.
    <div className="fixed inset-0 flex items-center justify-center overflow-hidden bg-transparent">
      {/* Inner canvas mirrors the reference outer container (ref line ~27). */}
      <div
        className="relative overflow-hidden font-saira"
        style={{
          width: STAGE_W,
          height: STAGE_H,
          transform: `scale(${scale})`,
          transformOrigin: "center",
        }}
      >
        {children}
      </div>
    </div>
  );
}
