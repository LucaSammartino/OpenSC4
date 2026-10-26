# overlay-views: answered questions

Qualification 2026-10-23 (re-overlay-views-contracts4): the four answers below remain finite-input
native contracts, as audited in overlay-views §3.3. The literal discriminators in §7.1 and
network-overlay-views §6.1 are **UNEXECUTED**. These answers do not close the distinct OVV4
outline-disable or OVV6 inventory/public-surface questions as they stood on that date.
Update 2026-10-26 (re-ovv29): the OVV4/OVV6 entries appended below supersede that open status;
OVV4 retains only the specific runtime observation in overlay-views §2.3.

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

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV4 - §2, SetCellColor and marked-cell state:** publish native r/g/b/a input types/ranges,
  byte conversion/rounding/clamping and packed byte order, default/common/per-cell and outline colours,
  and public cell mutation signatures. Can the outline pass be disabled independently of prefer-opaque?
  [Core retains explicit RGBA byte channels with no packed representation claim; SetCellColor(float...)
  fails with PackedColourOvv4. Explicit byte/replace_cells helpers do not claim native API equivalence.]~~

  **Native evidence 2026-10-22 (re-overlay-views-contracts4):** §2.2 now proves float-vector input,
  bit-based conversion without clamping, common/per-cell/default/outline state and three unnamed mutation
  wrappers. Residual: identify/prove an independent outline-disable API, or prove its absence across
  the complete owner inventory; constructor-disabled and prefer-opaque suppression alone do not answer it.

  **ANSWERED 2026-10-26 (re-ovv29):** DIFFERS — overlay-views §§2.2–2.3 publish float-vector
  conversion/packed order/defaults and every recovered mutation signature, including region growth.
  No disable entry exists among the 17 recovered owner entries or either interface table;
  SetOutlineColor always enables, and zero alpha does not suppress an additive one/one pass.
  **Narrowed runtime fact:** source-inline reset outside those entries is not ruled out. A live
  P+0x44 watchpoint showing a nonconstructor 1→0 write, writer PC/stack and subsequent normal-only
  Update with nonempty cells and prefer-opaque=false would establish it. §2.3 supplies this precise
  observation and a worked mutation/pass case; no unproved reset API or complete-owner claim is added.

<!-- from: core-overlay-views4 (2026-10-22) -->
- ~~**OVV6 - owner catalogue and lifecycle:** the task inventory says 54 functions across cSC4PathView,
  cSC4PathView::PathInfo, SC4MarkedCellView, cSTEVertBufExtn, SC4PathInfoView and SC4TrainJunctionView;
  current specs name a subset and ABI rows do not identify usable vtables. Publish the complete public
  method/address checklist: constructors/destructors, init/shutdown, getters/setters, bounds/transforms
  and reference ownership/failure semantics. [Core checklist limits coverage to published methods;
  Rust RAII does not claim native AddRef/Release or unpublished lifecycle. All six owners are withheld
  from complete-owner/runtime promotion.]~~

  **Native evidence 2026-10-22 (re-overlay-views-contracts4):** §3.2 lists all 49 currently named
  non-template entries plus three unnamed marked-cell wrappers, all four five-slot primary interfaces,
  and the proved/residual scope by owner. The claimed 54-entry inventory is not reconciled. Shared
  QueryInterface/AddRef/Release dispatch, zero-release destruction and remaining setters still need proof.
  **Qualification 2026-10-23 (re-overlay-views-contracts4):** §3.4 now independently proves the shared
  ID-1 QI, u32 AddRef/Release and constructor-table-to-deleting-body chain for all four views,
  including zero-count deletion and train deletion without Shutdown. These portions are supported;
  OVV6 remains partial for historical inventory membership, additional public setters/interface
  completeness and caller Shutdown discipline. Two newly traced train cleanup addresses do not by
  themselves identify the historical 54 entries. §3.3 supplies the per-owner method ledger.

  **ANSWERED 2026-10-26 (re-ovv29):** DIFFERS — overlay-views §§3.2–3.5 supply explicit membership
  for an audited 54-address checklist (49 named + three marked wrappers + two train cleanup bodies),
  all four five-slot primary and six-slot secondary interfaces, RemoveRef versus Release and the
  folded GetRefCount body. Folded path-info setters/Shutdown and inline filter/hover/press operations
  are included with actual offsets/callers; no public bounds/transform slots are present. The observed
  path-info/train owners unregister/Shutdown/Release; allocation/invalid-input boundaries are explicit.
  Original source-inline method names and the provenance of an older task list are not binary API facts.
  §3.5's worked refcount case is UNEXECUTED; independent inline outline reset is bounded by OVV4.
