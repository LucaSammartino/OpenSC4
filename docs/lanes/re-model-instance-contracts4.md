| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-model-instance-contracts4: portable model contracts | Codex / `re-model-instance-contracts4` (RE, detached) | ready; MI3–MI10 answered; 14 new independent spec vectors (20 total); questions/index and diff checks PASS; ABI unchanged; game/shadow n/a | 1. Coordinator reconcile portable implementation with DIFFERS rules. 2. Keep whole-owner/family coverage withheld for explicit exclusions. |

## Report

Resolved the eight implementation questions from the original export, instruction bytes, callers and executable
data. No engine files or implementation tests were read. MI1 stays opaque and MI2 stays literal, both open.

Milestones: `91e7137e` defaults/reference lifecycle; `bef445ac` processor/sprite/sort;
`ba5af834` bounds/picking/numeric limits; `aa22338a` exact published-owner inventory/resource lifecycle.

The resulting contract corrections are:

- MI3 **DIFFERS**: properties default to 2, draw flags are byte 0 and bytes 2–3 have no written constructor
  default. Transform counters are u8; Get increments destination, Set increments instance, neither copies source count.
- MI4 **DIFFERS**: full row-matrix/scale bounds enclose endpoint combinations, including negative scale;
  inverted sentinel bounds are reordered, not retained as empty. Bounds ignore the kind byte.
- MI5 **DIFFERS**: picking compares positive Euclidean distance to the hit point, not signed ray parameter.
  Equal/NaN comparisons do not improve the nearest field; first improvement ends each mesh.
- MI6 **DIFFERS**: material changes are instance-local, but processor SetModel returns false and clears both
  references for absent/empty/ineligible containers. Destruction restores original vertex identities.
  ReleaseModel alone does not restore, leaves a borrowed container pointer, and is not idempotent.
- MI7 **MATCHES**: upward fill starts without a source; leading own keys and non-own source scales retain
  downward results. All-own keys keep their downward ratios. Added unsigned wrap and I=0 failure boundaries.
- MI8 **DIFFERS**: repeated sort restarts from container order, but insertion precedes equal/unordered scores;
  ties reverse container order. The previous stable-sort assertion is corrected.
- MI9 **DIFFERS**: x87 transients are not blanket per-operation f32 arithmetic. NaN accumulator magnitude
  returns 0 without stepping; infinity takes the ≥2 branch. Primitive counts subtract without saturation,
  while signed iteration stops for nonpositive results. Invalid mesh skips/checked readers are portable choices.
- MI10 **DIFFERS**: constructors start count 0; basic uses u16, static/animated use u32; Release at 0 or 1
  destroys these instances. Resources instead atomically decrement a u32 first and destroy only at result 0;
  their zero-count release wraps without destruction. Factory uses its own locked u32 policy.

`docs/specs/model-instances.md` now names the eight actual owners, entry addresses, shared/inherited services,
and excluded native surfaces. Animated convenience slots 0–8, processor assignment, generic persistence
services and sprite-instance binding/rendering are explicitly outside the published operation subset. The
auto-name TransformDirection at `0x009F0090` is not accepted as vector-transform evidence. This is a docs-ready
question resolution, **not** a complete-native-owner, whole-family, shadow, replacement or runtime claim.

Fourteen independent expected vectors were added (20 total), covering defaults/packing/counters/count widths,
callback order, sprite fills, repeated ties/NaNs, processor returns/restoration, material isolation, asymmetric
negative-scale bounds, hit distance/sign, nonfinite playback, primitive/table boundaries and resource IID/counts.
They are specification vectors; none was executed against a runtime or compared to Rust results.

Validation: `python tools/questions.py index` regenerated the index; `python tools/questions.py check` PASS;
`git diff --check` PASS. MI3–MI10 moved, with MATCHES/DIFFERS answers and original provenance, to
`docs/specs/questions/answered/model-instances.md`. Picking and playback cross-references were reconciled in
view-utilities.md and graphics-formats.md. No new question IDs, ABI proposals, ABI-table edits or C ABI changes.
Cargo/clippy/ctest and game/shadow runs: n/a for this docs-only lane.

Evidence access: main-checkout re_query.py export first, bounded decomp reads where useful, then instruction
verification for widths, arithmetic, comparison directions and stores. PE constants read directly from
`bin/SimCity4.exe` (three default 1.0 words and sprite zoom sizes). REA was not needed or called. The quiet Bash
wrapper failed with E_ACCESSDENIED, so bounded local PowerShell commands were used. Commits succeeded on
detached HEAD despite Git's nonfatal packed-refs.lock permission warning; no ref/config repair was attempted.

Next for coordinator: reconcile the implementation with the seven DIFFERS answers, convert the independent
spec vectors into conformance tests as appropriate, and retain coverage exclusions until separately proved
and implemented. No game requests outstanding. No additional RE round or merge/push was started.
