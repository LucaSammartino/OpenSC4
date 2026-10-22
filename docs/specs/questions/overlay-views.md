# overlay-views: open questions
<!-- Questions about [overlay-views.md](../overlay-views.md). Append new entries at the END; when one is answered, move it to answered/overlay-views.md. Rules: README.md. -->

<!-- from: re-overlayviews (2026-10-20) -->
- **OVV1 - §1.1, §2.1:** which lot flags `0x304` (marks raised 15.5 m) and `0x100000` (marks hidden unless the lot
  says otherwise) stand for, and where a path's texture repeat factor r (path record `+0x38`) is set. [as read]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV2 - §1, cSC4PathView::PathInfo initialization and input contract:** publish the exact constructor,
  NewPath and PathInfo fields/defaults (including colour, repeat factor, priority signedness and opaque ID),
  colour/repeat mutation methods, duplicate-distance equality case, and path bounds/update invalidation rules.
  Are distances in §1.1 measured in x-z or full 3D, including the arrow suppression threshold?
  [Core uses explicit thickness and fixture-only colour/repeat inputs, signed stable priorities,
  strict squared x-z duplicate distance <1e-5, full 3D accumulated texture distance. Missing attributes
  fail with PathAttributesOvv2; no native initialization/default or complete-owner claim.]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV3 - §1.1, DrawPath / DrawArrowhead / normal helpers:** publish the exact bisector magnitude/miter rule,
  normal handedness, strip side ordering, arrow triangle tip/base coordinates and dimensions in terms of t,
  colour/UVs/winding, start/end shortening, draw ordering, and the two-arrow short-path case.
  [Core takes the literal unit bisector, falls back to the next unit normal below 0.0001, and places ±t
  sides with u=0/1. Eligible arrows fail with ArrowheadGeometryOvv3 before device mutation; no invented
  arrow shape or successful no-op. Ribbon geometry remains provisional pending exact proof.]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV4 - §2, SetCellColor and marked-cell state:** publish native r/g/b/a input types/ranges,
  byte conversion/rounding/clamping and packed byte order, default/common/per-cell and outline colours,
  and public cell mutation signatures. Can the outline pass be disabled independently of prefer-opaque?
  [Core retains explicit RGBA byte channels with no packed representation claim; SetCellColor(float...)
  fails with PackedColourOvv4. Explicit byte/replace_cells helpers do not claim native API equivalence.]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV5 - §2.1, DrawCells corner/altitude/bounds rules:** publish corner/index ordering, nearest-vertex
  half-way rounding, partially outside non-16m cell handling, fixed diagonal, lot-filter construction
  parameters, and order of frustum, water-max and lot-altitude replacement. Does water also affect
  non-16m cells or lot heights? What are zero/negative/nonfinite size and empty-cell behavior?
  [Core follows terrain-view §4.2 winding, half-way ties up, skips outside lower corners, clamps upper
  corners, and applies frustum then water then lot replacement only to 16m cells. Opaque flags 0x304 and
  0x100000 stay literal. Invalid sizes are rejected as a core safety rule, not a native contract.]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV6 - owner catalogue and lifecycle:** the task inventory says 54 functions across cSC4PathView,
  cSC4PathView::PathInfo, SC4MarkedCellView, cSTEVertBufExtn, SC4PathInfoView and SC4TrainJunctionView;
  current specs name a subset and ABI rows do not identify usable vtables. Publish the complete public
  method/address checklist: constructors/destructors, init/shutdown, getters/setters, bounds/transforms
  and reference ownership/failure semantics. [Core checklist limits coverage to published methods;
  Rust RAII does not claim native AddRef/Release or unpublished lifecycle. All six owners are withheld
  from complete-owner/runtime promotion.]

<!-- from: core-overlay-views4 (2026-10-22) -->
- **OVV7 - §3, cSTEVertBufExtn lifecycle and batching:** is index capacity min(capacity/(W+1), H)*W*6
  or is one vertex row subtracted? Are source indices absolute terrain indices, what is their width,
  and can hidden/degenerate indices point outside the copied range? Publish lock mode, initial lock,
  flush/relock policy, zero-n, index-capacity flush, lock/draw failure behavior, and release while locked
  or with pending indices. [Core uses literal row-count formula, explicit six-u32 absolute terrain
  indices per cell, compact coordinate rebasing, flush before vertex OR index overflow, lazy lock,
  no implicit draw on release, and balances its own lock on drop. These choices block promotion.]
