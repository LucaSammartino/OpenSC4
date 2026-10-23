# overlay-views: answered questions

Qualification 2026-10-23 (re-overlay-views-contracts4): the four answers below remain finite-input
native contracts, as audited in overlay-views §3.3. The literal discriminators in §7.1 and
network-overlay-views §6.1 are **UNEXECUTED**. These answers do not close the distinct OVV4
outline-disable or OVV6 inventory/public-surface questions retained in the open file.

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV2 - §1, cSC4PathView::PathInfo initialization and input contract:** publish the exact constructor,
  NewPath and PathInfo fields/defaults (including colour, repeat factor, priority signedness and opaque ID),
  colour/repeat mutation methods, duplicate-distance equality case, and path bounds/update invalidation rules.
  Are distances in §1.1 measured in x-z or full 3D, including the arrow suppression threshold?
  [Core uses explicit thickness and fixture-only colour/repeat inputs, signed stable priorities,
  strict squared x-z duplicate distance <1e-5, full 3D accumulated texture distance. Missing attributes
  fail with PathAttributesOvv2; no native initialization/default or complete-owner claim.]~~

  **ANSWERED 2026-10-22 (re-overlay-views-contracts4):** DIFFERS — overlay-views §1.2: first argument
  is packed colour, priority binary32/equal-newest-first, repeat uninitialized until the two-argument
  SetPathTexture; strict x-z duplicate threshold is binary64 1e-5, equality appends; arrow distances
  and texture projection are 3D. No bounds cache/dirty invalidation is present in these bodies.

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV3 - §1.1, DrawPath / DrawArrowhead / normal helpers:** publish the exact bisector magnitude/miter rule,
  normal handedness, strip side ordering, arrow triangle tip/base coordinates and dimensions in terms of t,
  colour/UVs/winding, start/end shortening, draw ordering, and the two-arrow short-path case.
  [Core takes the literal unit bisector, falls back to the next unit normal below 0.0001, and places ±t
  sides with u=0/1. Eligible arrows fail with ArrowheadGeometryOvv3 before device mutation; no invented
  arrow shape or successful no-op. Ribbon geometry remains provisional pending exact proof.]~~

  **ANSWERED 2026-10-22 (re-overlay-views-contracts4):** DIFFERS — overlay-views §1.2: offset M×t/(M·M),
  unscaled outgoing-normal fallback for squared magnitude < binary64 .0001; exact triangle P,B−2tN,B+2tN,
  inward length min(L−t,4.5t), threshold 2t, start-before-end shortening, and longitudinal/side UV projection.

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV5 - §2.1, DrawCells corner/altitude/bounds rules:** publish corner/index ordering, nearest-vertex
  half-way rounding, partially outside non-16m cell handling, fixed diagonal, lot-filter construction
  parameters, and order of frustum, water-max and lot-altitude replacement. Does water also affect
  non-16m cells or lot heights? What are zero/negative/nonfinite size and empty-cell behavior?
  [Core follows terrain-view §4.2 winding, half-way ties up, skips outside lower corners, clamps upper
  corners, and applies frustum then water then lot replacement only to 16m cells. Opaque flags 0x304 and
  0x100000 stay literal. Invalid sizes are rejected as a core safety rule, not a native contract.]~~

  **ANSWERED 2026-10-22 (re-overlay-views-contracts4):** DIFFERS — overlay-views §2.2: explicit corner
  order and both diagonals, mask 0x1F4F, non16m grid clamping before scaling and lower-vertex conversion;
  water-max before frustum in both size branches, lots replace heights afterward only for 16m. Empty
  cells succeed without drawing; setter accepts invalid sizes, draw has no general safe out-of-range promise.

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV7 - §3, cSTEVertBufExtn lifecycle and batching:** is index capacity min(capacity/(W+1), H)*W*6
  or is one vertex row subtracted? Are source indices absolute terrain indices, what is their width,
  and can hidden/degenerate indices point outside the copied range? Publish lock mode, initial lock,
  flush/relock policy, zero-n, index-capacity flush, lock/draw failure behavior, and release while locked
  or with pending indices. [Core uses literal row-count formula, explicit six-u32 absolute terrain
  indices per cell, compact coordinate rebasing, flush before vertex OR index overflow, lazy lock,
  no implicit draw on release, and balances its own lock on drop. These choices block promotion.]~~

  **ANSWERED 2026-10-22 (re-overlay-views-contracts4):** DIFFERS — overlay-views §3.1: literal sizing
  without row subtraction; u16 row-pair-relative indices, unvalidated modulo rebasing; explicit initial
  lock, vertex-capacity-only flush, growing index storage, zero-n copies two vertices, ignored driver
  failures, release neither unlocks nor flushes nor clears the stored lock pointer.
