| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances5: model instance owners | Codex, `core-model-instances5` | ready: published eight-owner closure; cargo 6760, clippy clean, CTest 69/69 | Coordinator review, publish reserved branch and land |

## Changes

Audited the existing clean-room `engine/core/src/model_instances` implementation
against all sections of model-instances.md and its referenced playback/picking
contracts. Implementation commit: `bded8d264`; initial audit: `718a6a22c`.

- Added `AnimatedInstance::unbound` for native constructor state without resource
  access, with all inherited interface parts sharing its 32-bit native count.
- Exposed `AnimatedInstance::build_frame`; same-frame success bypasses the model,
  failed builds preserve the frame word, and SetFrame delegates validation to
  the resource's GetModelFrame operation.
- Added `SpriteInstanceBinding` and resource-side CreateModelInstance: allocate,
  retain a temporary through foreign binding, retain the successful result,
  release the temporary, and clear output on binding failure. Sprite binding and
  rendering algorithms remain with their separate owner as §0.1 requires.
- Added three focused tests covering construction/BuildFrame, delegated sprite
  lifetimes and resource binding, and SerialRead identity/borrowed data. Updated
  the failed-frame test to observe the rejected provider call too.
- Added the module README's **Owner closure** table and mappings for all 20
  conformance vectors. The existing settled basic/static/processor/transform and
  sprite parser operations remain covered; no Unsupported stubs were added.

No plugin/capi wiring, ABI changes, ABI version bump, generated ABI-entry edits,
or coverage-map edits. The worktree remains detached as reserved by the supervisor.

## Validation

Source stayed unchanged after `bded8d264`; subsequent commits only record results.

| Check | Terminal result | Evidence |
|---|---|---|
| Focused model_instances tests | PASS 45, failed 0 | Direct PowerShell cargo test, host target |
| Explicit owned-file qbuild fmt | PASS, diff_lines=0, four Rust files | animation.rs, resources.rs, mod.rs, tests.rs |
| Required qbuild all | PASS | Native queue `qbuild-20261024-003708-build-ctest` |
| Workspace tests within all | PASS 6760, failed 0, ignored 0; sum of 41 test-result summaries | `build-runs/qbuild-test.log` |
| Clippy -D warnings within all | PASS, diagnostics 0 | `build-runs/qbuild-clippy.log` |
| Native build within all | PASS 220/220 edges, 101.4 s | C/C++ cache 196 hit, 0 miss; Rust 0 hit, 3 miss; recorded in `b4a04eaff` |
| Full CTest within all | PASS 69/69, 187.8 s | Native operation total 293.5 s; recorded in `b4a04eaff` |
| Task's explicit qbuild verify | PASS | Native queue `qbuild-20261024-005050-build-ctest` |
| Explicit verify build | PASS 46/46 edges, 1.8 s | `build-runs/fc4verify-build.log`; C/C++ cache 30 hit, 0 miss |
| Explicit verify CTest | PASS 69/69, 79.7 s | `build-runs/fc4verify-ctest.log`; native operation total 83.2 s |

Both asynchronous operations were collected to their actual terminal results.
There are no outstanding requests. The explicit verify refreshed the native log
copies named above; the earlier full-check measurements are preserved in the gate
commit and this report. The first q.py focused-test invocation failed before
cargo because Git Bash CreateFileMapping returned Win32 error 5; its evidence is
`build-runs/q/20261024-002952-10932.log`. Direct PowerShell cargo then passed.
There were no failing source test results or failed full gates.

Game runs: **n/a**. This is a pure Rust module with no game-facing switch, shadow,
or replacement wiring; no game request was made and no shadow exactness is claimed.

## Existing open limits

No new questions were filed. The existing five questions in
`docs/specs/questions/model-instances.md` remain explicitly identified in the
module README; no new behavior was inferred from other sources.

| Sections remaining open | Existing question | Implemented boundary |
|---|---|---|
| §6.1 opaque field meanings | MI1 | Copy all five headers and trailing value without interpretation |
| §4 purpose of flip | MI2 | Literal negation of the lighting rotation is implemented and tested |
| §§2.1, 3.1, 4.1 exact x87 transient precision | MI11 | Published binary32 stores with wider f64 transients for bounds/score/playback; existing f32 point/pick helpers; no exact control-word-dependent last-bit claim |
| §10 vector 15 second-copy staging | MI12 | Test the source and independent frame-builder copy before the second instance's own SetModel; material state remains isolated |
| §6.1 wrapped interior-zero keys | MI13 | Conservative portable rejection, separately tested from proved I=0 failure and nonzero unsigned wrap |

Native convenience slots excluded by §0.1, generic persistence services, and the
separate sprite-instance algorithms are outside the published subset. Invalid
memory access, checked table sizing, safe enum misuse, and repeated/dangling
processor release paths retain the README's explicit portable safety choices.

## Next

Coordinator reviews the closure table and existing open limits, publishes these
detached commits to reserved branch `core-model-instances5`, and claims coverage
rows after review. No additional implementation or checks are pending for the
settled published subset.
