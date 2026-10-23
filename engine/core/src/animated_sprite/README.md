# Animated sprite owner: WIP

Clean-room source: `docs/specs/animated-sprite.md` §§1–6/10;
`model-instances.md` §§1/6; `view-utilities.md` §7 (transform algebra,
triangle fan/pick primitive); `camera.md` §1.3 (rotation inverse).

This is **not a complete owner replacement**. No C ABI, plugin, shadow,
replacement map, resource reader/factory, or model_instances changes are made.
AS1 remains unanswered; AS3–AS9 request the contracts needed to finish the
requested 60-function owner. No missing native body is replaced by a provider.
The native full 60-function inventory cannot be reconstructed from the published
spec: the table below accounts for every public method it actually names.

## Public method reconciliation

| Published owner method | Implementation / remaining contract |
|---|---|
| Constructor / deleting destructor | Explicit `from_snapshot`; Rust destruction and arena final release drop counted data/owner references. Native defaults, initial count and allocator ABI absent: AS3. |
| Sprite slot 0 (shared body) | Body/IIDs absent: AS3. Only the separately published basic IID behavior is implemented. |
| SetPosition (slot 1) | Missing counter/kind/cache rules: AS3; no fabricated setter. |
| LookAt / SetOrientation (2–3) | Missing target/direction/axis/Euler order and unset-component rule: AS6. |
| Point/vector transforms and inverses (4–7) | `Transform::{point,vector,inverse_point,inverse_vector}` implement published kind-dependent algebra. Snapshot supplies rotation; inverse requires a rotation matrix, not a shear. |
| SetView (8) | `set_view`, raw bytes; changes dirty the frame. |
| SetAnimation (9) | Sequence representation, first-frame application and transition rules absent: AS1/AS5/AS6. |
| SetFrame / GetFrame / GetFrameCount (10–12) | Modulo setter with explicit sequence length; snapshot/frame table expose values. Native selected-sequence state absent: AS5. Zero length is an error. |
| AdvanceFrame (13) | Explicitly unsequenced path, forward loop/end/ping-pong and reverse interior. Reverse lower boundary is an error pending AS5, not an invented body. |
| SetRate (14) | Explicit raw rate only. Sentinel resolution absent: AS5; no time conversion. |
| Tick (15) | Explicitly unsequenced raw-unit clock; prescribed skip/no-skip quirk. Unresolved sentinels/non-progressing paths error transactionally. Native return codes other than documented 2 absent: AS5. |
| SetPlayMode (16) | Raw byte setter; extra side effects unspecified: AS5. |
| Shared slot 17 (always zero) | Published constant, no named API invented. |
| SetFade / SetHighlight / SetSelfLit (18–20) | §4.4 completing-comparison contracts, byte truncation/raw self-lit byte, returns and unrelated state preservation. Bool self-lit adapter retained. No nonfinite whole-call or trap-mode guarantee. |
| LoadFromResource / Load (21–22) | Typed foreign zoom prefix acquisition, IID 0x69B6F01C and stop-first-missing. Prefix is not reported as native Load success. Native return/reset/key/lifetime rules absent: AS4. |
| GetTextureResourceData / GetTextureData | Direction equation, decoded record lookup, page-edge UV/flip and scale helpers. Native bytes, camera angles and packing absent: AS1/AS6. |
| GetTransform / SetTransform | Snapshot getter/algebra; setter counter semantics absent: AS3. |
| UpdateView | Owned §5.1 exact-stage finite quad and reset/reflected box, using stored decoded/scaled metrics and raw camera primitives. Other numeric regions explicitly Unsupported. |
| Draw / Draw thunk | Owned cache invalidation, full-integer context comparisons, low-byte holder use, translation and individual context-call protocol. Refresh selection supports only the explicit zero-angle single-orientation raw-record domain below. No native thunk/ABI or complete owner claim. |
| Pick / Pick thunk | Owned stale translated-vertex fan selection with first-hit short circuit; only the individual foreign TrianglePick primitive is delegated. No refresh or visibility rejection. |
| GetAnimatedModelFromResourceKey / GetMeshAnimFromResourceKey / GetSpriteAnimFromResourceKey | `creation_route` retains the published swapped-looking type routing. No fake successful factory/load operation: AS4. |

## Basic-instance contract

`SpriteArena` implements the published 16-bit reference count, acquired basic
interfaces (IIDs 1 and 0x26A68449), and most-derived final destruction. Initial
count is explicit; overflow fails without mutation (AS3). Handles are monotonic
Rust storage identities, not native addresses. The owner reference uses `Arc`;
its borrowed getter does not acquire another reference. The core never calls a
native Release on a borrowed pointer.

Custom data, properties and the 20-byte draw attributes are owned/copied values.
Getters use `&self`, without RefCell. Native derived GetWorldBounds/GetBounds,
GetSortValue, GetResModel and their pre-load behavior remain unpublished (AS9),
so no placeholder success/sort value/resource null is supplied.

## Foreign boundary

`AnimationRecord`, `SpriteTable`, `FrameEntry`, `TextureHandle` and `Arc` records
are decoded foreign **data**, not resource-owner implementations. A provider
only acquires a table by key/interface; it cannot compute sprite vertices,
choose animation frames, step clocks or perform original owner methods.
Direction-major typed entries are an explicit Rust data representation, not a
claim about the native eight-byte layout. No factory parsing or source-zoom
selection is duplicated from the separate resource lane.

`billboard_corners` specifies its anchor explicitly; it does not silently bind
that convention to native offsets. `Transform` algebra and metric quads are
independent components until AS6/AS7 permit complete native owner integration.

## Verification

The 29-test paragraph below records the inherited component suite, not the
current gate. New literal public-contract vectors and current measurements are
recorded in the lane report and engine/conformance/animated-sprite-contracts.md.

## Qualified cached algorithms

The new cached path does not use the old provisional direction-major
SpriteTable selector or billboard helper. Those remain portable components,
not native SetOrientation or published resource-selection implementations.

Callers explicitly install current stored cache state; this is a portable
suboperation input, not a factory/default/load claim. UpdateView computes
geometry itself from W/H/offsets and raw camera rows, magnification, and mpp
or already rounded GetViewVolume endpoints. Scale has already affected metrics.
Only arithmetic whose every stage is exact is admitted: nonzero operands and
results must have magnitude in [2^-12,2^12], and rational divisions must be
exact. This conservative subset makes the distinct native store boundaries
irrelevant to admitted results. Other finite inputs return Unsupported.
Signed-zero payloads are not promised. No x87 control-word closure is claimed.

Dirty Draw refresh supports raw holder divisors [1,1,1], angle limits
[binary32 pi, binary32 pi, 0, 0], owner back column (0,0,1), and low rotation
zero. Then asin/atan2 inputs have exact zero results, P=R=1, and both selector
B conversions receive zero. The engine selects entry zero and decodes its
published eight bytes, including temporal frame packing, page, UV flips and
qualified scale. This is an explicit narrow domain, not generic trig or
native-angle goldens. Other orientation/holder domains return Unsupported.
Clean cache draws do not run this selector. Out-of-range holders, absent
resources, malformed packing and texture bounds are checked portable errors;
native fault behavior and fault-time partial writes are outside this API.

Draw takes raw context and shade-state snapshots and invokes individual
foreign primitives; hosts never select a frame or return an entire quad/Draw.
All boolean callback returns are ignored. Only ResetShade/ResetTransparency
run at the end; no saved-state restoration or fault rollback is promised.
The owned repr(C) vertex has position/colour/UV and stride 24; it is not a
borrowed native instance layout, C ABI export or persistence schema.

AS1/2/3/4/5/6/7/9 whole-call, resource, control-word, trig, controller,
lifetime and default questions remain. The 60-function/10482-byte catalogue
is unreconciled. Native/game owner coverage remains UNEXECUTED.

29 Rust unit tests cover the five §10 scenarios at the supported component
level, raw timing thresholds at 1/2, residual timing, looping/reversal/failure,
asymmetric direction/base/zoom/UV/offset/flags, noncommuting transform and inverse,
fan picking order, missing/invalid resources, repeated resource adoption, shared
getters, counted owner replacement, reference overflow and final-release cleanup.
The zoom vector checks visibility, not a fabricated draw-command count. These
are executable Rust vectors; no complete owner conformance or game shadow is
claimed while AS3–AS9 remain open.
