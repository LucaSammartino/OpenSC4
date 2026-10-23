| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin5: cSC4View3DWin owner closure | Codex / core-view3dwin5 | in progress | Audit existing ui_windows::view3d against owner specs, implement and validate. |

## Handoff
- Goal: close cSC4View3DWin core owner; foreign window/renderer/camera/simulator traits; no C ABI or plugin changes.
- Worktree reused at reserved detached HEAD; initial working tree clean.
- Existing module: engine/core/src/ui_windows/view3d.rs. Do not duplicate.
- Round 2 milestones: be6919b0 Init/messages (41 tests passed); 297fac15 GUI/city (46 passed); identified command and catalog-item tool dispatch added. Two mock Close(bool) expectations corrected after 47/49 focused result; production routing unchanged.
- Done commits: 4235510b input/motion; a65fad4f cheats/state/save/status; ac87b033 panels/commands; 384d472c legacy save compatibility.
- FINAL corrected full gate: cargo 6110 passed/0 failed/0 ignored; clippy exit=0 diagnostics=0; CMake compiler-ABI configure TIMEOUT 900 s; CTest skipped (0 run). Overall FAIL, stable source/HEAD.
- Both full checks collected to terminal: sessions 50209 and 27109. No checks outstanding. Failed first-gate logs preserved under build-runs/evidence-gate-1/; corrected logs build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-configure.log.
- Private _MSPDBSRV_ENDPOINT_=fc4lane-core-view3dwin5 did not fix compiler-ABI timeout; approved coord-verify-lane.ps1 uses this convention. Probe invokes cl.exe without sccache. Do not duplicate a pending check (none now).
- Next: collect stable final full gate (round 2), then remaining button/catalog popup/status/helper/save-reference rows. Formatting passed eight owned Rust files. Full gate uses _CL_=/Z7 to avoid pre-project /Zi PDB probe hang; prior logs preserved build-runs/evidence-round2.
- Existing tests cover VW-T1, T5–T18; source compatibility preserved: legacy View3DSave unchanged, full core body View3DRecord + save_record/load_record. Foreign opaque reference identity/resolution adapter still pending.
- Coverage: engine/core/src/ui_windows/view3d/README.md; pending rows are NOT closure claims. Split grouped rows further for final per-section coverage.
- Filed VW9–VW13 in docs/specs/questions/view3d-window.md. Existing VW6–VW8 remain open. No game requests, game/shadow validation, plugin/C API edits, or ABI export changes.
- Formatter passed seven owned Rust files; rustfmt check passed after compatibility correction. Initial focused milestones 21 then 28 tests passed before later additions.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin5-r2-manifest.json. Detached HEAD: commit normally, no branch/ref changes. No ready queue until owner closure and required gate pass.

## Earlier milestone notes
- Implemented QI, control-stack lifetime, input routing, motion/zoom and paint scheduling in existing module.
- Focused test: cargo test --manifest-path engine/Cargo.toml -p fc4-core ui_windows::view3d: 21 passed (16 new).
- qbuild fmt passes for four owned Rust files (absolute paths required). Initial q.py test launch failed in Bash before compilation; direct PowerShell cargo succeeded.
- Added honest owner-closure table at engine/core/src/ui_windows/view3d/README.md; pending rows are not closure claims.
- Filed VW9–VW11 (exact command registration IDs/order, unspecified paint tail, paint default/reversed limits).
- Remaining: lifecycle/notifications/window messages, GUI creation/teardown, remaining command/button/catalog routing, display integration; host-aware catalog teardown; conformance T2–T4. See README for exact pending rows.
- No plugin/ABI edits. No claim of owner closure or readiness.
- Milestone 4235510b: input/motion owner and 21 focused tests passing.
- Added cheats/filter/level/contours, four-reference logical save and trigger counts; focused run before status additions: 28 passed.
- Added status methods for clock/cost/cursor/rating/alerts/speed/mission. Filed VW12–VW13 for bucket/parse and cursor/fade ambiguities.
- Focused clippy found manual_clamp in SizeOf; corrected to f32::clamp (preserves NaN).
- Added panel state, fully identified motion/panel/query command dispatch and VW-T14/T15 tests after a65fad4f.
- Full gate 1 terminal FAIL: test/clippy could not compile legacy C API/conformance View3DSave literals; CMake compiler-ABI probe timed out at 900 s, CTest skipped. Session 50209 collected to terminal.
- Failed logs preserved in build-runs/evidence-gate-1/. Corrected core-only compatibility by restoring View3DSave fields and adding View3DRecord + save_record/load_record for the full body.
- Approved coord-verify-lane.ps1 sets a private _MSPDBSRV_ENDPOINT_ to prevent parallel PDB hangs; corrected gate will use fc4lane-core-view3dwin5 after observed compiler-ABI timeout (probe has no sccache launcher).
- Final qbuild fmt: exit=0 files=7 diff_lines=0 (all owned Rust files).
- Next immediate: run/collect corrected full gate without committing during the run (gate 1 also detected changed HEAD); then continue incomplete owner rows. No merge queue until closure.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin5-r1-manifest.json.
