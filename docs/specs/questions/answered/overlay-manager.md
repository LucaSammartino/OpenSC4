# overlay-manager: answered questions

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV9 - §3.1 slot7:** specify radius/centre fields, the eight-part partition, half-chord row sample coordinates/rounding, city-edge clipping, texture mapping and terrain redisplay order, and empty/failing part cleanup. The prose states eight rectangles but not their equations/partition. [no host-computed eight rectangles or guessed disc approximation accepted; round add reports a gap]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §3.1 proves an eight-part outline partition omitting the centre, outer/90% inner radii, four integer split-row chord samples, ordered rounding and binary32 partition constants, shared full-footprint texture matrix, per-part clipping and redisplay order. Empty parts remain allocated; the native add has no partial-allocation rollback path. The H=20 split-row discriminator detects substituting exact decimal constants.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV4 - §§1,2,3 getters:** please specify constructor overlay/handle defaults, Init failure/repeat-Init behavior, factory slot used for key 0xAA592B77, slot 22's field identity/type, and scalar getter behavior for groups and handle -1 (besides alpha=0). [core owns acquired Arc references, rejects invalid/repeated Init atomically, defers shadow-binding lookup until its slot is known; unidentified/group scalar getters report a gap, null output getters leave output untouched]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §§1.1/2.3 specify defaults and unspecified inactive scratch, mutating repeat Init with no rollback/failure result, binding factory slot5, slot22 boolean shown, slot26 self-lit, group part0 getters, and −1/null-output behavior. No atomic native stale/null/Init rejection is implied.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV5 - §§2,3 slots12-15:** specify handle/group allocation and free-list reuse order, stale/out-of-range behavior, slot13/14 behavior on direct overlay IDs, and slot12's x/z offsets in the nine floats. Is a handle record always allocated for every add? Do any setters early-return on equal input or mark unchanged rectangles? [private LIFO arenas and atomic stale-record rejection; no public construction/re-placement until OV1; null calls do nothing]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §§2.3/3.2: FIFO reuse, no generations/range checks, direct IDs ignored by placement accessors, X/Z at public offsets0/4, direct city overlays need no placement handle. Position uses a native 1e−4 binary32 tolerance; full placement always re-places, unchanged static rectangles still mark once, equal shown/colour/state writes still mark; handle is freed before group parts.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV6 - §3 slot17:** what is the numerical value/precision of 0x00AB39A0, strict versus inclusive step comparison, and does a sub-step alpha write update the stored alpha, or compare against the last redrawn alpha? Specify clamping/NaN behavior, dynamic redraw behavior, and equal endpoint writes. [dynamic and endpoint changes supported; static non-endpoint changes report a gap atomically rather than accept an external threshold]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §3.3: binary64 0.05, strict greater-than; static sub-step/equal writes leave stored alpha unchanged, no clamp, static NaN rejected, dynamic always stores without redraw; endpoint equality is rejected on static managers. Native bytes and paper vectors cited there.
