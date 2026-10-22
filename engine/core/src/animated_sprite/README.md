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
| SetFade / SetHighlight / SetSelfLit (18–20) | Raw flags and material effects, preserving unrelated bits; alpha-edge/invalid cases tracked in AS8. |
| LoadFromResource / Load (21–22) | Typed foreign zoom prefix acquisition, IID 0x69B6F01C and stop-first-missing. Prefix is not reported as native Load success. Native return/reset/key/lifetime rules absent: AS4. |
| GetTextureResourceData / GetTextureData | Direction equation, decoded record lookup, page-edge UV/flip and scale helpers. Native bytes, camera angles and packing absent: AS1/AS6. |
| GetTransform / SetTransform | Snapshot getter/algebra; setter counter semantics absent: AS3. |
| UpdateView | Metric billboard primitive and widening box helper only. Native offset/space/cache/fallback semantics absent: AS7. |
| Draw / Draw thunk | Published material state and visibility predicate; no successful Draw method or full command stream claimed. Coordinate/shade/restore/argument contracts absent: AS7. |
| Pick / Pick thunk | Fan triangle order with first-hit short circuit; primitive provider owns TrianglePick, not the sprite owner's decision. Native cached-vertex spaces absent: AS7/AS9. |
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

29 Rust unit tests cover the five §10 scenarios at the supported component
level, raw timing thresholds at 1/2, residual timing, looping/reversal/failure,
asymmetric direction/base/zoom/UV/offset/flags, noncommuting transform and inverse,
fan picking order, missing/invalid resources, repeated resource adoption, shared
getters, counted owner replacement, reference overflow and final-release cleanup.
The zoom vector checks visibility, not a fabricated draw-command count. These
are executable Rust vectors; no complete owner conformance or game shadow is
claimed while AS3–AS9 remain open.
