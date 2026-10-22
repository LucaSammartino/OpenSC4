| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-effects4: model registration effects | Codex / `core-model-effects4` (engine lane, detached reserved worktree) | wip: documented subset committed 83ae0078, 8f92404c; cargo 4306 PASS, clippy 0 diagnostics, ctest 67/67; 17 new tests; GF19-GF24; ABI unchanged; no game/shadow/complete-owner credit | 1. Publish missing GF19-GF24 contracts. 2. Complete and verify the remaining native owner bodies in a separately authorized lane. |

## Result and scope

Implemented the behavior actually published in graphics-formats §3.11 Consumer 1 and vectors 48–49 of §8.5, with only the narrow instance frame inputs from §3.9. This is a useful portable core subset, **not the requested complete 23-function / 8406-byte native owner**. The named public section has no complete owner inventory or bodies for native lifetime, visibility/camera, space switches, or other public flags. No missing body was guessed or replaced by a provider returning original owner outputs. Questions GF19–GF24 request those contracts precisely.

Changes are `engine/core/src/model_reg_effects/{mod,transform,tests}.rs`, one additive module export in core, one integration test in formats, this report and the allowed question file. No C ABI, plugin, version, coverage map, generated ABI list, tooling or game files changed. No subagents, native analysis, branch/ref updates, merges, pushes or game requests were used.

## Implemented behavior

- Owned, name-sorted effect map; exact `fx_` filtering and digit-prefix stripping; full REGP key retained separately from effect name.
- Creation by a typed foreign factory; replacement stops/releases the old created reference before attempting new creation; failure removes the entry. Success copies records and starts stopped.
- Borrowed effect lookup, named/all start/stop with opaque arguments, and owned started state. No RefCell; owner and immutable getters are Send + Sync.
- Acquired Arc model attachment and created Box effect references, explicit stop/release ordering, detachment and drop. These are documented portable lifetime policies, not invented native reference-count bodies.
- Current animated record selection, nonanimated record 0, axis normalisation and ANGLE interpretation with the specified negative half-angle/matrix rows; GF13 zero-axis identity choice.
- Instance rotation/scale/translation composition, independent outer effect-placement composition, outgoing transform flags=5, and the specified scalar parameter-15 product.
- Safe missing-record diagnostics instead of out-of-bounds access. No clamping, interpolation, wrapping or reuse of fire-crew behavior.

`fc4-formats` already depends on core. `ModelInstance::regions` borrows the exact public type of `s3d::Model::regions` (`BTreeMap<String, Vec<[f32; 7]>>`), so core needs neither a crate cycle nor another parser. The integration vector calls the existing `Model::parse`, then passes its actual regions field through that provider. Duplicate parsed names append records and parser lowercasing selects the expected effect.

## Public owner inventory audit

The permitted ABI catalogue exposes 12 rows. It does not supply the missing function bodies. This table records coverage limits rather than fabricating the other entries of the requested 23-function inventory.

| Native slot | Public operation | Portable implementation and remaining limit |
|---|---|---|
| 0 | Init | Attached-instance list initialization implemented. Native returns unspecified. |
| 2 | SetModelInstance | `attach` owns an acquired model and rebuilds safely; native null/same/new-pointer behavior unprovided, GF20. |
| 4 | UpdateEffects(pos, dir, speed) | Documented parameter product implemented; argument types/pos/order unprovided, GF24. No native binding claimed. |
| 5 | UpdateEffects | Started-entry frame transforms implemented; checked invalid-record extension, GF23. Visibility branches unprovided, GF22. |
| 6 | StartAll(bool) | Opaque controls forwarded and started state maintained; native bool/default/extra behavior unprovided, GF21. |
| 7 | StopAll(bool) | Opaque control forwarded and started state maintained; native bool semantics unprovided, GF21. |
| 8 | AddEffect | Replacement/create/failure/record ownership implemented; replacement stop argument unprovided, GF20/GF21. |
| 9 | AddEffectAtRegion | Explicit region lookup implemented as a portable convenience; actual body/missing-region result unprovided, GF21. |
| 10 | GetEffect | Borrowed full-key lookup implemented; native acquisition and case rules unprovided, GF20/GF21. |
| 12 | StartEffectsNamed | Case-insensitive effect-name matching implemented for ASCII; third argument/native AL meaning unprovided, GF21. |
| 13 | StopEffectsNamed | Case-insensitive effect-name matching and opaque control implemented; locale/native return unprovided, GF21. |
| 16 | InitEffectsList | Published map walk, filtering, name extraction and failed-creation removal implemented. |

Constructor sites are named by §3.11, but constructor defaults, destruction/helpers and the rest of the 23-body inventory are not specified (GF19/GF20). No native AddRef/Release or visibility/index/flag slots were invented. The sorted core iterator is not a claim about an undocumented native index API. Updates use only documented started-state gating; no camera/visibility behavior is claimed.

## Verification

- Focused `qbuild quick`: **PASS**, 2594 Rust tests, 0 failures/ignored, capi/conformance compile-only success, native build 206/206 edges (183.0s).
- Final source commit **8f92404c**, one stable `qbuild all`: **PASS**, cargo workspace **4306 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, native build **4/4 edges** (55.8s), full CTest **67/67** (121.7s).
- **17 new passing tests**: 16 core plus one real-parser integration test. Includes vectors 48–49, noncommuting local/model/effect-placement rotations, the actual rotated/scaled owner update path, oblique non-unit axis, zero axis/angle, frame/nonanimated/bounds cases, creation/index order, stopped/start/stop/restart gating, opaque controls and parameter 15, failed replacement, copied records, acquired/borrowed references, reattachment/detachment/drop release order and thread-safe immutable getters.
- `qbuild fmt` passes for all four new Rust files. Including the touched `core/src/lib.rs` reports inherited module ordering/format differences (126 diff lines); only one additive export was made there, with no unrelated reformat. `git diff --check` passed.
- Logs remain in `build-runs/fc4verify-quick-test.log`, `fc4verify-quick-deps.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log`.
- Task explicitly said main was freshly gated and requested no repeated full baseline; used quick then one final full gate. No source edits during either build.
- Game/shadow/refaudit: **n/a**, core-only and no hook/shadow key. No game request or outstanding label; no live replacement or complete-owner credit.

## Questions and next steps

Appended **GF19–GF24** to `docs/specs/questions/graphics-formats.md`: complete inventory, lifecycle/reference ownership, control/index/lookup contracts, visibility/spaces/flags, unchecked-record domain, and motion-update arguments/order. Every portable choice outside the stated native contract has a corresponding code comment and question.

The clean-room rules permit editing only that question file under specs and prohibit running `tools/`; the coordinator must regenerate/reconcile the question index through its own authorized workflow. No generated index was edited.

Queue state is **wip**. After the missing public contracts are published, complete the native owner bodies and independent visibility/refcount/flag vectors in a separately authorized lane. Do not promote this subset to complete-owner/shadow/replacement coverage.

Commits: cabc8a57 (initial handoff), 83ae0078 (core subset and questions), 8f92404c (additional vectors and scoped formatting). Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-model-effects4-r1-manifest.json`.
