# overlay-manager: open questions
<!-- Questions about [overlay-manager.md](../overlay-manager.md). Append new entries at the END; when one is answered, move it to answered/overlay-manager.md. Rules: README.md. -->

<!-- from: re-overlays (2026-10-20) -->
- **OV1 - §3:** the layout of the nine-float placement record (slots 12-14) and the last arguments of the add
  calls (slots 3-6, 9-11); a centre, extents and an orientation are inferred from how `PlaceOverlay`
  (`0x0073A1B0`) turns it into a cell rectangle and a texture matrix. [the record as copied]

<!-- from: re-overlays (2026-10-20) -->
- **OV2 - §3:** what slot 20 (`0x007364F0`, no function in the export) does, and how the add forms of slots 8, 9
  and 11 differ from slot 4 (slot 11 is the largest and is inferred to build a shadow from a model's
  footprint). [unread]

<!-- from: re-overlays (2026-10-20) -->
- **OV3 - §5:** what message `0xC9F775BB` is and what the managers do on it. [inferred: the overlays are
  placed again after a terrain or zoom change]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV4 - §§1,2,3 getters:** please specify constructor overlay/handle defaults, Init failure/repeat-Init behavior, factory slot used for key 0xAA592B77, slot 22's field identity/type, and scalar getter behavior for groups and handle -1 (besides alpha=0). [core owns acquired Arc references, rejects invalid/repeated Init atomically, defers shadow-binding lookup until its slot is known; unidentified/group scalar getters report a gap, null output getters leave output untouched]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV5 - §§2,3 slots12-15:** specify handle/group allocation and free-list reuse order, stale/out-of-range behavior, slot13/14 behavior on direct overlay IDs, and slot12's x/z offsets in the nine floats. Is a handle record always allocated for every add? Do any setters early-return on equal input or mark unchanged rectangles? [private LIFO arenas and atomic stale-record rejection; no public construction/re-placement until OV1; null calls do nothing]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV6 - §3 slot17:** what is the numerical value/precision of 0x00AB39A0, strict versus inclusive step comparison, and does a sub-step alpha write update the stored alpha, or compare against the last redrawn alpha? Specify clamping/NaN behavior, dynamic redraw behavior, and equal endpoint writes. [dynamic and endpoint changes supported; static non-endpoint changes report a gap atomically rather than accept an external threshold]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV7 - §§3,4 slots21/Draw:** specify the placement-to-world/texture equations, matrix axis/storage/composition conventions, slot21 matrix entries and behavior for size=0, and camera transform inputs/precision. The public text gives semantic scales 1/size and -1/size with second-axis offset 1+offset/size but not their world matrix positions. [core stores those semantic coefficients only; no externally computed owner matrices accepted; full world transform deferred with OV1]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV8 - §§2.2,4:** specify rectangle endpoint conventions, clipping/margin and cell-index insertion/update/removal ordering; decal/shadow classification (bit4 is described as both decal/shadow-part and decal eligibility); shadow selection/strength minimum, alpha-test equation, second transform/height coordinate, DrawRect land/water/dynamic array input formats, GPU restoration and counter reset/increment points. Does empty or skipped Draw preserve last-frame stats? [core uses explicitly internal half-open rectangles and sorted/deduplicated selection only; no host draw result, fabricated successful GPU pass or guessed stats; normal Draw reports a gap]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV9 - §3.1 slot7:** specify radius/centre fields, the eight-part partition, half-chord row sample coordinates/rounding, city-edge clipping, texture mapping and terrain redisplay order, and empty/failing part cleanup. The prose states eight rectangles but not their equations/partition. [no host-computed eight rectangles or guessed disc approximation accepted; round add reports a gap]
