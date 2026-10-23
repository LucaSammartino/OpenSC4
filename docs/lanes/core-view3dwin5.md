| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin5: cSC4View3DWin owner closure | Codex / core-view3dwin5 | continue; owner closure incomplete | Collect corrected gate session 40468, then finish pending owner contracts; no merge queue. |

## Handoff
- Goal: close existing cSC4View3DWin core owner against view3d-window/ui-windows/view3d; no plugin or C ABI changes.
- STOP: round-2 lane guard reports context 152k > 150k. Continuation required; no merge-queue file written.
- Round-2 commits: be6919b0 Init/shutdown/messages; 297fac15 GUI/city lifecycle; 451ff699 command/catalog tools; b38997a8 GUI ordering and clippy correction plus questions.
- Existing inherited milestones remain below as historical evidence. Current code is engine/core/src/ui_windows/view3d.rs and its submodules; do not duplicate.
- Focused tests: 41 passed after messages, 46 passed after GUI/city. Expanded suite initially 47/49 due to reversed mock Close(bool) expectations; corrected expectations are covered by the full gate's 6124 passing tests.
- Round-2 full gate 1 (451ff699): terminal FAIL; cargo 6124 passed/0 failed/0 ignored; clippy one collapsible_match lint (2 reported diagnostics including build warning); CMake ABI configure timed out 900 s; CTest skipped/0 run.
- Failed evidence preserved in build-runs/evidence-round2-gate1/; earlier-round evidence in build-runs/evidence-round2/ and evidence-gate-1/.
- CORRECTED FULL GATE OUTSTANDING: exec session 40468, collector functions cell 46. Source b38997a8. Do not launch another check. Collect this same operation to terminal, or inspect its already-completed result/logs if the collector finished.
- Last attempted log read was denied by the guard; corrected gate's terminal verdict is UNKNOWN. The active collector may itself be denied on its next wait; this does not establish process completion.
- Corrected gate uses _CL_=/Z7, private _MSPDBSRV_ENDPOINT_=fc4lane-core-view3dwin5-r2-final, CMAKE_TOOLCHAIN_FILE=<WT>/build-runs/cmake-release-probe.cmake. This selects Release without skipping compiler capability checks; prior /Z7-only attempt still timed out.
- Next exact step: collect session 40468, read bounded PASS/FAIL lines and current build-runs logs; update gate evidence. Do not repeat all unless a real diagnostic changes source or inputs.
- Owner closure remains incomplete: section 1.1 GZOnCommand/window override dispatch; section 5 pre-city-shutdown and refresh integration; section 6 advice/button integration; sections 6d–6f remaining interface getters/picking and host-aware catalog teardown; section 6h dialog/budget/mayor visibility; section 6i buttons/menu/popup geometry and remembered selections; section 6j widget data/ambient light; section 6k opaque identity/resolution; section 6l background/coverage helpers; paint tail VW10.
- Audit notification clock/mission payload conversion, focus-loss exit routing, final pre-city-init progress ordering, and lifecycle-object/save-reference synchronization before claiming closure. Existing trait callbacks are not closure evidence for owner logic still behind them.
- README owner table remains honest but some grouped related-spec rows need splitting to one row per section. T1–T18 now have tests; full owner coverage is not claimed.
- New questions VW14–VW16 appended to docs/specs/questions/view3d-window.md (null failures/overlay lifetime, pre-city shutdown, GZOnCommand). VW6–VW13 retained. Coordinator must regenerate question index; forbidden tools not run.
- No game request or shadow run: core-only work, no plugin/C API wiring, ABI unchanged. No game request outstanding.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin5-r2-manifest.json. Detached HEAD: commit normally, no refs/branch changes. Final documentation commit follows b38997a8.

## Superseded earlier handoff
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
