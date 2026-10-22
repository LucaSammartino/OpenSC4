# water-ripples: open questions
<!-- Questions about [water-ripples.md](../water-ripples.md). Append new entries at the END; when one is answered, move it to answered/water-ripples.md. Rules: README.md. -->

<!-- from: re-ripples (2026-10-19) -->
- **WR1 - §1.1, §1.5:** the time unit of the animation tick's elapsed value, the frame period `0x491C9C07` (175000)
  and the 2,000,000 rebuild delay (microseconds is inferred).

<!-- from: re-ripples (2026-10-19) -->
- **WR2 - §1.2:** what the terrain vertex flags `0x10` and `0x01` mean (shore and water are inferred), and which side
  of the traced shore the normal N points to.

<!-- from: re-ripples (2026-10-19) -->
- **WR3 - §2.1, §2.2:** the meaning of the three bytes b1–b3 of a textured selection and of its modes 1–3.

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR4 - §1.2 ordered shore trace:** publish the terrain-helper trace algorithm: connectivity and neighbor priority, seed enumeration, initial direction, flag mutation/restoration, duplicate suppression, open/closed termination, and conversion from vertex indices to ordered control points. Neither §1.2 nor terrain.md defines the trace. [Core implements the exact seed bit test and returns `ShoreTraceWr4` on any seed; no original-owner output callback, no guessed flood fill or contour tracing. Blocks complete cSTEWaterRipple.]

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR6 - §§1.3,1.5 animation:** what RNG distribution/rounding generates the initial frame and 0.7–1.3 period, what is the initial clock, and on elapsed overshoot does Update advance one frame or catch up, discard or preserve remainder, and redraw a period for each crossed boundary? §6 vector 6 requires equality to advance despite §1.5 saying "past". [Raw units retained; exact-boundary advance/wrap implemented; overshoot returns a named gap without mutation. Random inputs are primitive bounded draws, integer range floor(7P/10)..floor(13P/10), not a claim about native sampling.]

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR7 - §§1.4,2.3 render geometry and states:** publish all four selection state parameter sets and the ripple alpha-blend state, plain/textured terrain vertex positions/heights/normals/UVs, masks determined by literal bytes/mode, triangle/index and tile submission ordering, texture key lookup/failure behavior, frustum bounds/tests, and depth-offset save/restore contract. Does only rectangle marking refresh kPreferOpaque, and does it do so regardless of clear? [Nonempty selection draw explicitly returns `SelectionGeometryWr7`; no draw-selection or draw-ripple owner-output delegation. Table iteration is diagnostic key order only. Blocks complete selection owner and ripple draw.]

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR8 - owner inventory/defaults/lifecycle:** the brief counts 65 functions/15308 bytes, but the spec and public SDK do not enumerate the owner API. Publish a complete per-owner method checklist with signatures, constructor/destructor and Init/Shutdown/Cleanup semantics, initial fields/refcount/return values, repeated/failed Init behavior, tuning absent-property/type behavior, and texture/factory acquisition/release timing. [Rust Arc/Drop and explicit shutdown clean all held references; tuning retained after first successful Init. These are safe core policies, not native lifecycle claims. No complete-owner credit.]

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR9 - §2.2 selection data boundaries:** what byte is stored by the packed-colour overload in the third-byte field, what is alpha outside the mode-3/no-add-signed fallback, what are the scalar widths, and what happens for empty lists, reversed/out-of-range rectangles/cells and palette indices >4? Publish the unrelated constants if deterministic palette overflow is required. [Opaque b1/b2/b3 stored without interpretations; packed b3 explicitly unknown (`None`), alpha literally 1 except specified 0.5 fallback. Out-of-range palette returns a named gap, invalid cells/rectangles are rejected atomically; no invented palette clamping or alpha from low colour byte.]

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR10 - §1.5 rebuild scheduling/edits:** define changed-rectangle units/area inclusivity, the branch meant by "otherwise", strip/rectangle contact tests, dirty-rectangle reset, edit-counter width/wrap, whether the arming tick contributes its elapsed time, whether repeated edits restart the delay, whether delay fires at equality, and immediate/zoom rebuild scope (redetect shores or reuse cached shores). [Literal core state uses cell width×height; accumulated area >12 resets/checks/arms, otherwise unions dirty rectangle and arms at area >400; counter delta >minPoints²/10; arm starts a fresh zero clock, subsequent ticks reach >=2000000, repeated arms retain it. These provisional boundaries have no native/runtime credit.]
