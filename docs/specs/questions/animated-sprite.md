# animated-sprite: open questions
<!-- Questions about [animated-sprite.md](../animated-sprite.md). Append new entries at the END; when one is answered, move it to answered/animated-sprite.md. Rules: README.md. -->

<!-- from: re-sprites (2026-10-20) -->
- **AS1 - §2, §4.1:** the layout of the per-zoom sprite-animation record (type `0x69B6F01C`) and of a frame
  entry: which bytes are the page, the frame's pixel position and size, the two flip bits and the offsets, and
  what the animation table at `+0x34` (sequences with a first frame and a length) comes from. [the use as read]

  **PARTIAL 2026-10-23 (re-animsprite-as5b):** §4.3 now reconciles stream/block/holder/controller
  offsets against ReadTable and SetZoomResource: block +0x1C is the entry count, holder +0x1C copies
  block +0, entries begin at block +0x20. Neither reader nor reviewed creation/load routes installs
  controller +0x34. Allocation/installation provenance remains unproved; do not synthesize it from n.

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS3 - §1, §3; owner inventory and lifetime:** the task identifies 60 owner functions/10482 bytes, but
  the spec publishes only slots 0–22 and a few named helpers. Please publish the full public owner-method
  inventory (including inherited overrides/thunks), constructor defaults (frame, clock, direction, rate,
  play mode, skip byte, flags, zoom, transform, bounds, owner and initial reference count), sprite-interface
  IIDs/slot 0 behavior, refcount overflow/underflow rules, and SetPosition/SetTransform change-counter rules.
  [WIP only: explicit snapshots and explicit initial u16 count; checked overflow, basic IIDs 1/0x26A68449;
  no invented complete owner constructor, sprite QueryInterface, transform setter or missing method body]

  **PARTIAL 2026-10-23 (re-animsprite-as5b):** the 23/19-slot vtables in §3 were independently
  reread from the PE and agree with the published surfaces. Broad adjacent export intervals give
  72 functions/11,081 bytes but include other animated-model methods and allocator/free helpers;
  they do not reconcile the asserted 60/10,482 owner inventory. No complete-owner count is claimed.

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS6 - §3.1, §4.1 orientation and frame selection:** what axis/sign/order is used for noncommuting
  yaw/pitch/roll, and does -100 preserve the old component or omit its rotation? Is LookAt's first vector
  a target, a direction, or a source position; which local forward axis is used? Publish camera-relative
  yaw/pitch/base calculation (including the stated negative-pitch wrap), rounding at ±half-direction ties,
  sequence-first-frame application, native frame ordering/page packing/flip-byte decoding (AS1), and UV
  half-texel/edge conventions. [formula-only direction lookup with round ties away from zero/Euclidean modulo,
  decoded direction-major entries and pixel-edge UV helpers; no guessed orientation or camera-relative method]

  R2 partial evidence (2026-10-23, re-sprites-contracts4): §3.1 now supplies axis/sign/order, per-component
  omission/reset/counters and target-based LookAt; §4.1 supplies camera-relative angles, conditional
  nearest-even conversion, sequence-byte application and retained packing/UV rules. Independent proposed
  UNEXECUTED R2-R1–R3/D1 did not close this whole question at the R2 stop: exhaustive partial-mask
  instruction-store transcriptions and runtime numeric-mode provenance were then unestablished. AS1 controller provenance also
  remains open. No host-trig or portable arithmetic oracle is authorized by these formulas.

  **PARTIAL 2026-10-23 (re-sprites-contracts4, R3):** §§3.1.1–3.1.3 now supply the complete finite normal
  SetOrientation mask-0..7 CFG, each x87/transient versus binary32 boundary, stage snapshots/in-place
  assignment order, identity reset, flags/counters/untouched state and return. All trig in this body is
  hardware FCOS/FSIN, with no foreign argument conversion/return-format gap. §3.1.4 adds conditional
  R3-M/C/L/Z rational distinguishers, not native owner-call goldens. Geometric order and −100 omission
  are proved; no contradiction remains in the later LookAt paragraph. Whole-call PC/RC provenance,
  native angle-result qualification/nonfinite outcomes and AS1 controller provenance remain open.

  **PARTIAL 2026-10-23 (re-animsprite-as5b):** §§3.1.5/4.1.1 answer the AS10 spill/primitive
  boundaries and show asin's temporary CW preserves PC while forcing nearest RC. Startup requests
  PC53 but does not prove owner-entry PC/RC, foreign-call preservation or native trig goldens.
  This runtime qualification and AS1 provenance remain open; AS10 is archived separately.

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS7 - §5–6 quad, cache, draw and pick:** publish corner order/winding and offset origin/sign; the
  coordinate space of cached corners, bounding box, model transform and Pick triangles; whether model rotation
  and scale apply before/after billboarding; exact fallback camera-view-volume/metres-per-pixel equation;
  invalid/out-of-range zoom behavior; cache invalidation for camera-axis/mpp changes at unchanged zoom/rotation;
  box initialization/shrink/reset versus widening; lighting/self-lit shade values and shade/transparency restore
  order on failures. Which arguments accompany DrawPrims(2,10,4,...), and must draw state be restored too?
  [independent metric quad and transform/pick primitives only; no fabricated complete UpdateView/Draw/Pick,
  no provider may supply computed sprite quad output or original owner decisions]

  R2 partial evidence (2026-10-23, re-sprites-contracts4): §§5.1–6 now supply finite raw-input quad/box
  equations, corner order/space, reset/reflected-maximum bounds, signed zoom check, dirty/cache rules,
  outgoing draw-call order, no forced state restore and stale Pick triangles. R2-Q1–Q5 are independent
  proposed UNEXECUTED vectors. This question remains open for malformed/nonfinite input outcomes beyond
  explicitly described branch behavior and whole-call numeric-mode provenance; no native execution,
  foreign computed-quad provider, complete owner or default implementation promotion is claimed.

  **PARTIAL 2026-10-23 (re-animsprite-as5b):** §§4.1.1/6 now qualify ambient-mode/foreign-call
  boundaries and Pick's binary32 x87 X/Y versus integer Z copies. No renderer CW capture,
  malformed/nonfinite primitive execution, trap classification or numeric golden was obtained.
  Finite raw-input formulas remain valid within their stated conditions; this question stays open.

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS9 - public base overrides/sort:** model-instances §1 marks GetWorldBounds and GetSortValue as derived,
  but animated-sprite §§1–6 give no sort formula/return behavior or full bounds getter behavior. Publish those
  methods, GetResModel's sprite override (if any), bounds getter effects before loading/drawing, and any distinct
  Draw/Pick thunk adjustments needed for the 60-function inventory. [no guessed sort key, null resource override,
  lazy bounds recomputation or successful native getters; inventory explicitly lists these as missing]

  **PARTIAL 2026-10-23 (re-sprites-contracts4, R3 reconciliation):** retained R1/R2 §3.2 gives local-bound
  copy, translated-only world bounds without lazy refresh, null-resource return, texture-table sort output
  and undefined status for output getters; §§1.1/3.2 give defaults and dynamic thunk displacement.
  Cited bodies include 007FE0A0–007FE0DB, 007FD680–007FD701, 007FD640–007FD670, 004E13E0,
  007FE230 and 007FEE90. These normal rules remain published; the question's complete 60-function
  inventory/thunk/lifetime reconciliation remains AS3, not proved by this finite-AS6 round.

