# lot-models: open questions
<!-- Questions about [lot-models.md](../lot-models.md). Append new entries at the END; when one is answered, move it to answered/lot-models.md. Rules: README.md. -->

<!-- from: re-lotmodels (2026-10-20) -->
- **LM1 - §4:** who sets a cell texture's colour, alpha and mode (the lot developer is inferred), what modes
  other than 1-3 do, and what the check through `0x00B43D20` slot 46 that bypasses the flat-lot path tests.
  [as read]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM2 - §§1–4 public inventory and lifecycle:** the task counts 82 functions / 3 owners, but this spec names only constructor/destructor bodies and the listed shape/texture/draw/sort helpers. Please publish the complete per-owner body inventory and portable behavior of the remaining entries, inherited `cS3DModelInstanceBasic` operations, constructor field/default state, QueryInterface (including foundation's additional IID if any), initial reference counts, AddRef/Release transitions, and destruction/resource-release order. Rust-owned records do not claim those native contracts. [unresolved]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM4 - §3 retaining-wall construction:** please define box-to-cell edge ranges and inner/across-cell coordinates for each side, endpoint/traversal order, which positions receive 0.001 versus 0.002 m offsets, endpoint-specific ±0.2 m seam rules (especially terrain crossing the flat altitude), triangle splitting/diagonal/winding/count at equality/crossing, UV origin/u repeats, vertex lighting/colour calculation, and whether the across-edge flat test actually suppresses construction/draw. Current core can own edge bands, facing, texture and draw decisions but cannot truthfully produce the unspecified triangle buffer or recolouring. [unresolved]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM7 - §4 draw-state/combiner order:** please publish alpha/tinted draw-state fields not enumerated in §4, exact combiner initialization and per-mode operation/arguments, global-light/RGB units and evaluation order, flat-strip vertex alpha for each mode, whether alpha-pass filtering precedes or follows a zoom reload, and texture-failure/cache behavior. Does force-context value 8 bypass the translucent-pass skip? LM1 already covers unsupported modes and flat-path bypass. Core retains an effective mode/alpha combiner request, not an invented native combiner. [unresolved]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM8 - §§2–4 numeric domains:** please specify coordinate/height/scale precision, NaN/infinity handling, signed cell-coordinate multiplication/overflow, invalid zoom/rotation/orientation semantics and RGB/alpha conversion/clamping. Portable APIs currently accept only five zooms, four directions and finite geometry samples, use f32 coordinates/spans without hidden clamps, and treat other inputs as outside their domain, not a claim about native failure behavior. [unresolved]
