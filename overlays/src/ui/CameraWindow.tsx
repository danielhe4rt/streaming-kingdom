// ---------------------------------------------------------------------------
// CameraWindow — the right-side camera cutout (reference lines ~68-80).
//
// This is a TRANSPARENT HOLE in the overlay: it paints no background fill, only
// a layered box-shadow ring + rounded corners, so the OBS camera source behind
// it shows through. Filling it would hide the camera. Alongside the cutout it
// renders the two yellow seam bars that mask the join between chat and camera.
//
// Coordinates are baked in from the reference (1920x1080 stage) so the parent
// just drops <CameraWindow /> into the Stage without positioning it.
// ---------------------------------------------------------------------------

export interface CameraWindowProps {
  // Optional chip text (e.g. "CÂMERA / TELA"). Hidden entirely unless provided.
  label?: string;
}

export function CameraWindow({ label }: CameraWindowProps) {
  return (
    <>
      {/* Seam: vertical yellow accent + its drop shadow (ref lines ~69-70). */}
      <div className="absolute left-[710px] top-[835px] bottom-0 w-[46px] bg-yellow" />
      <div className="absolute left-[710px] top-[868px] h-[170px] w-[46px] bg-black/[.18]" />

      {/* RIGHT cutout window (ref line ~73). NO background — stays transparent
          so the OBS camera shows through. The ring is three stacked shadows:
          white/22 hairline, ink/55 ring, then a soft drop shadow. */}
      <div
        className="absolute left-[756px] top-[12px] right-[12px] bottom-[130px] overflow-hidden rounded-[22px]"
        style={{
          boxShadow:
            "0 0 0 2px rgba(255,255,255,.22),0 0 0 6px rgba(20,10,31,.55),0 18px 60px rgba(0,0,0,.5)",
        }}
      >
        {/* Small "CAMERA / TELA"-style chip (ref lines ~75-78). Only when labeled. */}
        {label ? (
          <div className="absolute right-[18px] top-[16px] flex items-center gap-2 rounded-full border border-brand-light/35 bg-ink-900/[.62] px-[14px] py-[7px] backdrop-blur-[4px]">
            <span className="h-2 w-2 rounded-full bg-brand shadow-[0_0_10px_#8b2fe8]" />
            <span className="font-saira-cond text-[14px] font-bold tracking-[.14em] text-brand-pale">
              {label}
            </span>
          </div>
        ) : null}
      </div>
    </>
  );
}
