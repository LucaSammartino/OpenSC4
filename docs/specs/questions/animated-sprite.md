# animated-sprite: open questions
<!-- Questions about [animated-sprite.md](../animated-sprite.md). Append new entries at the END; when one is answered, move it to answered/animated-sprite.md. Rules: README.md. -->

<!-- from: re-sprites (2026-10-20) -->
- **AS1 - §2, §4.1:** the layout of the per-zoom sprite-animation record (type `0x69B6F01C`) and of a frame
  entry: which bytes are the page, the frame's pixel position and size, the two flip bits and the offsets, and
  what the animation table at `+0x34` (sequences with a first frame and a length) comes from. [the use as read]

<!-- from: re-sprites (2026-10-20) -->
- **AS2 - §2, §7:** which models are sprite animations (resource type `0x29A5D1EC`) in the shipped data and
  which occupants use them, and whether the swapped-looking names of `0x007FD7D0` / `0x007FED50` reflect the Mac
  build. [the behaviour as read]

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS3 - §1, §3; owner inventory and lifetime:** the task identifies 60 owner functions/10482 bytes, but
  the spec publishes only slots 0–22 and a few named helpers. Please publish the full public owner-method
  inventory (including inherited overrides/thunks), constructor defaults (frame, clock, direction, rate,
  play mode, skip byte, flags, zoom, transform, bounds, owner and initial reference count), sprite-interface
  IIDs/slot 0 behavior, refcount overflow/underflow rules, and SetPosition/SetTransform change-counter rules.
  [WIP only: explicit snapshots and explicit initial u16 count; checked overflow, basic IIDs 1/0x26A68449;
  no invented complete owner constructor, sprite QueryInterface, transform setter or missing method body]

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS4 - §2 loading:** AS1 remains open; model-instances §6 identifies a 0x09ADCD75 table queried via
  IID 0x69B6F01C, whereas §2 calls that IID a resource type. Confirm the exact Load/LoadFromResource inputs,
  key type/group rewriting, zoom iteration order, reference ownership, missing-first/missing-later zoom return
  values, rollback versus retained prefix, repeated-load replacement, and which fields initialize/reset.
  [typed foreign decoded records; acquire in array order with IID 0x69B6F01C, stop at first absent zoom;
  expose prefix/stopping reason, do not call a prefix a successful native Load or implement resource readers]

<!-- from: core-animated-sprites4 (2026-10-22) -->
- **AS5 - §3 slots 9–16, §4 clock and sequence stepping:** please specify animation-table layout/step rules,
  SetAnimation invalid-index and state-reset behavior, SetRate(-100/-200) resolution and raw resource rate
  field (no inferred time conversions), SetPlayMode side effects, AdvanceFrame at reverse frame 0 in every
  mode, and single/empty-frame cases. Does clock subtraction use direction before or after reversal? What
  are native Tick/AdvanceFrame returns when the frame does not change or playback stops? How is the literal
  skip loop made to terminate for a single frame, negative rate/elapsed or zero direction with |position|>=1?
  [only explicitly named unsequenced operations; raw elapsed*rate*direction, literal advance-then-subtract,
  no-skip long-tick position=1; unresolved sentinels/reverse boundary/non-progress return errors transactionally;
  no native success return invented and no sequence-table stepping delegated to a foreign owner]

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

<!-- from: core-animsprite5 (2026-10-23) -->
- **AS10 - §3.1 LookAt and §4.1 Draw angle numeric boundaries:** which normalization intermediates
  are stored to binary32 in LookAt (squares/sums, length, reciprocal, normalized components and cross
  products), and what are the exact argument/return/store boundaries of Draw's asin/atan2 primitives?
  The formulas specify the geometric rule but do not give the complete spill schedule as they now do for
  SetOrientation. [literal difference/cross/component stores and reciprocal FSQRT formula on exact
  stages only; nonexact/degenerate LookAt and nonzero Draw angle acquisition remain Unsupported,
  also subject to AS6 whole-call PC/RC and native numeric qualification]

<!-- from: core-animsprite5 (2026-10-23) -->
- **AS11 - §2 foreign factory route:** for GetSpriteAnimFromResourceKey (type 0x5AD0E817), what IID,
  resource-manager options, output interface and reference-transfer rules accompany the published
  resource-manager fetch? [routing is preserved; no guessed successful foreign factory. The sprite
  construction/load route for type 0x29A5D1EC is implemented separately.]
