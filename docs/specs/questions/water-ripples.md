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
- **WR7 - §§1.4,2.3 render geometry and states:** publish all four selection state parameter sets and the ripple alpha-blend state, plain/textured terrain vertex positions/heights/normals/UVs, masks determined by literal bytes/mode, triangle/index and tile submission ordering, texture key lookup/failure behavior, frustum bounds/tests, and depth-offset save/restore contract. Does only rectangle marking refresh kPreferOpaque, and does it do so regardless of clear? [Nonempty selection draw explicitly returns `SelectionGeometryWr7`; no draw-selection or draw-ripple owner-output delegation. Table iteration is diagnostic key order only. Blocks complete selection owner and ripple draw.]
  **PARTIAL 2026-10-22 (re-ripples-contracts4):** §§1.4/2.3 now prove state argument sets, one actual pass, ordinary finite heights/corners, UV byte transforms, transparency/mode, index tables, bucket/chain traversal, binding cache and failures, flat ripple bounds/grouping, depth restore, and all clear-only preference refreshes. Residual: dynamic plain black-entry RGB initialization, exact allocation/rehash submission order, NaN height operand choice. Do not mark whole-owner/draw equivalence from this subset.

<!-- from: core-terrain-ripples4 (2026-10-22) -->
- **WR8 - owner inventory/defaults/lifecycle:** the brief counts 65 functions/15308 bytes, but the spec and public SDK do not enumerate the owner API. Publish a complete per-owner method checklist with signatures, constructor/destructor and Init/Shutdown/Cleanup semantics, initial fields/refcount/return values, repeated/failed Init behavior, tuning absent-property/type behavior, and texture/factory acquisition/release timing. [Rust Arc/Drop and explicit shutdown clean all held references; tuning retained after first successful Init. These are safe core policies, not native lifecycle claims. No complete-owner credit.]
  **PARTIAL 2026-10-22 (re-ripples-contracts4):** §2.4 gives direct constructor/Init/Shutdown behavior and zero initial refcount, unchecked service failures, repeated Init differences and retained state. Residual: complete inherited/public API attribution/signatures, Cleanup/deleting/reference paths, tuning absence/type/ownership/reset, texture acquisition failure; inventory details in lane report. Do not equate the address-window inventory with the brief's owner totals.

