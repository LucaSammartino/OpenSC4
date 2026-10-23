# lot-models: open questions
<!-- Questions about [lot-models.md](../lot-models.md). Append new entries at the END; when one is answered, move it to answered/lot-models.md. Rules: README.md. -->

<!-- from: re-lotmodels (2026-10-20) -->
- **LM1 - §4:** who sets a cell texture's colour, alpha and mode (the lot developer is inferred), what modes
  other than 1-3 do, and what the check through `0x00B43D20` slot 46 that bypasses the flat-lot path tests.
  [as read]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM2 - §§1–4 public inventory and lifecycle:** the task counts 82 functions / 3 owners, but this spec names only constructor/destructor bodies and the listed shape/texture/draw/sort helpers. Please publish the complete per-owner body inventory and portable behavior of the remaining entries, inherited `cS3DModelInstanceBasic` operations, constructor field/default state, QueryInterface (including foundation's additional IID if any), initial reference counts, AddRef/Release transitions, and destruction/resource-release order. Rust-owned records do not claim those native contracts. [unresolved]

<!-- from: core-lot-models4 (2026-10-22) -->
- **LM8 - §§2–4 numeric domains:** please specify coordinate/height/scale precision, NaN/infinity handling, signed cell-coordinate multiplication/overflow, invalid zoom/rotation/orientation semantics and RGB/alpha conversion/clamping. Portable APIs currently accept only five zooms, four directions and finite geometry samples, use f32 coordinates/spans without hidden clamps, and treat other inputs as outside their domain, not a claim about native failure behavior. [unresolved]
