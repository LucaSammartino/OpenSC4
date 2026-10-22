# lighting: open questions
<!-- Questions about [lighting.md](../lighting.md). Append new entries at the END; when one is answered, move it to answered/lighting.md. Rules: README.md. -->

<!-- from: re-lighting (2026-10-19) -->
- **LT1 - §2.1, §2.2:** which setting is the byte `+0x39C` of the `+0x0C` member of the settings object at
  `0x00B43CC4`? When it is set the fade colour is black, the fade strength 0 and no dawn or dusk message is posted.

<!-- from: re-lighting (2026-10-19) -->
- **LT2 - §4.3:** which network occupants answer true to slot 8 with `0x40000`? Their models are lit with the up
  normal and the default weight, ahead of the type tests.

<!-- from: re-lighting (2026-10-19) -->
- **LT3 - §3, §4.5:** what the fade term does on screen. It grows across the city from one edge (which edge follows the
  view rotation), scales with `2^-zoom` and with the time-of-day fade strength (`0x29189BF5`, 0.15 at midday), and is
  only recomputed on a view change.

<!-- from: re-lighting (2026-10-19) -->
- **LT4 - §5.6:** the meaning of the decal shadow values `0x085489C9` (a float pair the shipped exemplar lacks) and
  `0x085489CA` (12.0), and of the shadow parameters `0x085489CC` (0.4) and `0x085489CD` (0.55). Their users draw
  decal shadows outside this lane.

<!-- from: core-lighting4 (2026-10-22) -->
- **LT5 - §1.1–§1.4 constructor, statics and identity:** what are the initial reference count, successful QueryInterface AddRef behavior, last-announced colors/fractions, initial direction/cache values, and the initial statics for properties whose table gives only shipped values? What does a view change use for fade before city dimensions exist? [Core requires explicitly seeded shared statics; shipped() is an explicit fixture/configuration, not inferred native defaults. Unspecified instance fields start zero, last-announced/cache light colors white, reference count 1, successful QueryInterface acquires a reference, and the no-city fade gradient is zero. These choices are not exact-owner credit.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT6 - §2.2 light distance:** is the change test the Euclidean RGB distance, its square, or a different metric? What precision is used? [Literal Euclidean length against the threshold. A displacement (0.00008,0.00008,0) exceeds 0.0001 under this choice; a squared-distance comparison does not.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT7 - §1.5/§2.1/§4.2 image boundaries and failure:** does min-red scan active width×height pixels or padding too? When the last-row time sample spills beyond the allocation, or the surface map exists but model map is absent/shorter, what is the defined outcome? What Init cleanup is required if decoding/copying fails after earlier maps loaded? [Scan active pixels; preserve linear row spill when storage exists; return explicit safe Rust errors for unavailable pixels/model map. Failed initialization owns partial resources until Shutdown/destruction or retry; no foreign image/lighting methods are called.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT8 - §5.2/§5.5 terrain bounds:** what is the numeric epsilon at 0x00AA2270, and how does the upper clamp round before terrain slot 27? Are rectangular city grids supported by the sweep, and if so which row/column dimensions are used per rotation? What is required for sx=0? [Interior terrain normals and negative-coordinate lower clamps implemented; upper-edge/outside reads return ContractGap until epsilon is specified. Square sweep only; rectangular sweep returns ContractGap; nonfinite drift skips and retains the prior rectangle.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT9 - §6 terrain change rectangles:** are incoming dirty rectangles inclusive vertices or cells, and how does 0x007D6540 convert the changed stored vertex rectangle to terrain slot 67/68 world cells (expansion, endpoint inclusion and clipping)? On long-operation end, is RedisplayTerrain called when a sweep changes nothing, and which rectangle is used then? [Core currently accepts inclusive vertices, inverts the specified grid indexing exactly without cell expansion, and emits slot-67/68-style events for the same nonempty changed world vertex rectangle. This is provisional, not an exact slot-67/68 contract.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT10 - §5.6 CalcShadowProjection:** specify the plane basis, model-frame direction transform, complete input/output matrix layout, flag-dependent composition and arithmetic precision. Does the determinant test use a signed or absolute determinant? [Only the stated 2D affine least-squares subproblem with signed determinant cutoff 0.01 is implemented; complete projection returns ContractGap and preserves output, without an original-method provider.]

<!-- from: core-lighting4 (2026-10-22) -->
- **LT11 - §5.6 CalcShadowTerrainRect:** specify the full arguments, box-top ray origins, axis stepping, fractional direction handling, terrain sample coordinates, stop condition, rectangle endpoint inclusion and clipping after extension. Does it scan each box-edge ray or a bounding extremum? [Overview is insufficient for an exact extension algorithm; method returns ContractGap and preserves output. No substitute provider or guessed ray traversal.]
