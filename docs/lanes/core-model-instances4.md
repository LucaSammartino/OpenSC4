| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances4: model instance Rust owners | `core-model-instances4` / (engine lane, detached worktree) | ready; code 4ed5725a; cargo 4269/0, clippy 0 diagnostics, ctest 67/67; 31 model-owner tests; MI3–MI10 filed; ABI unchanged; game n/a | 1. Coordinator merge. 2. Resolve native contract questions before adapters/replacement. |

## Result

Implemented the complete published pure Rust owner contracts in `engine/core/src/model_instances`, exported by one additive line at the end of `engine/core/src/lib.rs`. Behavior sources are model-instances §§1–6,10, graphics-formats §§3.9/3.12 (and material/vertex definitions those sections reference), geometry-utilities bounds/ray helpers, view-utilities §7, and lighting §4.4. No native analysis, game files, CAPI/plugin/ABI edits, or coverage-map edits.

Milestones: `912f9519` adds the owners and initial vectors; `4ed5725a` completes the lifecycle/factory audit, 31 independent vectors, formatting and the local clippy annotation preserving the spec's playback operand order. The final gate ran on `4ed5725a` with frozen sources. This report-only commit follows it.

## Complete published owners and checklist

| Exact owner name | Implemented public contract |
|---|---|
| `cS3DModelInstanceBasic` | Basic/unknown query with counted result; 16-bit reference accounting and most-derived destruction; counted owner replacement; custom data, 32-bit properties and complete draw-record copy; derived world-bounds/sort dispatch; dirty-gated identity; ignored transform set, false pick, empty draw, null resource model. |
| `cS3DModelInstance` | Basic/container query parts; counted model and mesh vertex/index/primitive blocks; mesh/material access; stale local bounds and transformed world bounds; change-counted transform; indexed/unindexed picking; typed render dispatch; 16-bit properties; attributes and blend-pair mutations; first material/texture sort value and model getter. |
| `cS3DModelInstanceAnimated` | All static behavior through its contained instance; animated query; constructor playback defaults; frame/count/header and inert never-set subrange selection; transactional frame construction without duplicate rebuilds; speed sentinels/file rate; stored-f32 accumulator and one-step catch-up; forward/backward looping, ping-pong and hold. |
| `cS3DModelProcessor` | Separate owning instance/container references; multi-frame animation refusal; whole-container format eligibility; format-2 to format-10 conversion preserving positions/UV/bounds, white colours; original-block restoration on SetModel; lazy Newell area-weighted normals without normalization; lighting material flag and literal matrix negation; descending stable insertion sort; idempotent release/destruction. |
| `cS3DResSpriteAnim` | Counted creation/query/destruction; tag and truncation validation; all MI1 opaque values preserved; relative zoom keys, downward source/scales and conditional upward fill. |
| `cS3DResSpriteAnimTable` | Counted creation/query/destruction; tag, checked count arithmetic and exact-size validation; opaque 32-byte header and 8-byte entries preserved. |
| `cS3DResSpriteAnimFactory` | Creates both types as explicitly unread resources without invented header values; unsupported types rejected; type-dispatched serial reads; failed reads drop their new resource. |
| `cS3DTransform` | All published kind/row-matrix/translation/uniform-scale paths, including fallback kinds; dirty-only identity and change counter; no fabricated binary layout. |

External providers supply only model-resource frames/bounds/header, drawing, and lighting colours. Owner algorithms calculate playback, bounds transforms, pick/primitive walks, normals, order, resource parsing and zoom resolution themselves. Shared state uses `RwLock`, never `RefCell`. Rust references hold the most-derived allocation through final destruction.

## Measured verification

- Final `qbuild.py all`: **PASS**; workspace cargo **4269 passed / 0 failed / 0 ignored**; clippy **exit 0 / 0 diagnostics**; CMake **4/4 edges**; ctest **67/67** (124.1 seconds), fc4verify **181.0 seconds**.
- Core: **2402 passed**, including **31 new model-instance tests**. Two earlier quick gates passed with **2393 core tests**, all dependents compiled, and CMake **4/4**.
- Independent expected vectors cover asymmetric transforms, negative scales and enclosing bounds; separate multiply/add rounding and change-counter wrap; all eight primitive types and index offsets; first-hit-per-mesh/nearest-distance behavior; owner/query/release/destruction counts and concurrent getters; replacement failure cleanup; processor refusal, conversion/restoration/release; equal/unequal area normal weighting and zero weights; stable sorting ties/repeated sorts; lighting matrix negation including signed zero; animation frame/material changes, zero/file/negative speed, ping-pong/hold/catch-up; resource bytes, all truncations, tags, exact table size/count overflow, opaque retention and shared zoom sheets.
- `qbuild.py fmt` **PASS for all seven new Rust files**. The required check including shared `lib.rs` reports existing module-order/line-format differences. Its original baseline also fails `rustfmt --check` (recorded in `build-runs/fmt-baseline-lib.log`); our `lib.rs` diff is only the appended module. No mass formatting performed.
- Initial baseline all: cargo **4238/0**, clippy **0 diagnostics**, CMake **206/206**, ctest **66/67** because early edits overlapped cargo_test while modules were incomplete. An intermediate complete-code gate had cargo **4269/0** and ctest **67/67**, with one assign-op clippy lint. The final frozen-source gate above supersedes both failures.
- Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No outstanding build or game sessions.
- Game/shadow: **n/a**. Core-only owners have no runtime adapter or shadow hook; no native/shadow/replacement verification or default-runtime promotion is claimed. ABI exports/version unchanged.

## Questions and literal choices

Appended **MI3–MI10** to `docs/specs/questions/model-instances.md` only; the generated questions index is untouched for coordinator reconciliation. Choices are annotated in code: constructor/draw-layout/change-counter details (MI3), enclosing transformed bounds (MI4), signed ray parameter for nearest distance (MI5), material-copy/unsupported-format/processor-release semantics (MI6), upward-pass initialization and preserved scales (MI7), original container order on repeated sorts (MI8), malformed/non-finite safe behavior (MI9), and initial/overflow reference lifecycle (MI10). These are actual unpublished contract details, not placeholders for owner algorithms. MI1 opaque headers remain uninterpreted, and MI2 is literal component-wise negation.

## Next steps

Coordinator may merge the portable owners and tests. Resolve the numbered native-contract questions before ABI adapters or native/default replacement credit. No lane-side merge, push, restart, game request, ABI change, or coverage-map update is needed.
