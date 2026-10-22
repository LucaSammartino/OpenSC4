# overlay-manager: answered questions

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV6 - §3 slot17:** what is the numerical value/precision of 0x00AB39A0, strict versus inclusive step comparison, and does a sub-step alpha write update the stored alpha, or compare against the last redrawn alpha? Specify clamping/NaN behavior, dynamic redraw behavior, and equal endpoint writes. [dynamic and endpoint changes supported; static non-endpoint changes report a gap atomically rather than accept an external threshold]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §3.3: binary64 0.05, strict greater-than; static sub-step/equal writes leave stored alpha unchanged, no clamp, static NaN rejected, dynamic always stores without redraw; endpoint equality is rejected on static managers. Native bytes and paper vectors cited there.
