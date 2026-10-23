| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin5: cSC4View3DWin owner closure | Codex / core-view3dwin5 | in progress | Audit existing ui_windows::view3d against owner specs, implement and validate. |

## Handoff
- Goal: close cSC4View3DWin core owner; foreign window/renderer/camera/simulator traits; no C ABI or plugin changes.
- Worktree reused at reserved detached HEAD; initial working tree clean.
- Existing module: engine/core/src/ui_windows/view3d.rs. Do not duplicate.
- Implemented QI, control-stack lifetime, input routing, motion/zoom and paint scheduling in existing module.
- Focused test: cargo test --manifest-path engine/Cargo.toml -p fc4-core ui_windows::view3d: 21 passed (16 new).
- qbuild fmt passes for four owned Rust files (absolute paths required). Initial q.py test launch failed in Bash before compilation; direct PowerShell cargo succeeded.
- Added honest owner-closure table at engine/core/src/ui_windows/view3d/README.md; pending rows are not closure claims.
- Filed VW9–VW11 (exact command registration IDs/order, unspecified paint tail, paint default/reversed limits).
- Next: finish lifecycle/notifications/messages/cheats, GUI/panels/command routing, displays/save/triggers; integrate catalog teardown with host-aware controls; add conformance T2–T6, T14–T18.
- Full gate not yet run: implementation is incomplete; run once when stable final source is ready. No plugin/ABI edits.
- Milestone 4235510b: input/motion owner and 21 focused tests passing.
- Added cheats/filter/level/contours, four-reference logical save and trigger counts; focused run before status additions: 28 passed.
- Added status methods for clock/cost/cursor/rating/alerts/speed/mission. Filed VW12–VW13 for bucket/parse and cursor/fade ambiguities.
- Focused clippy found manual_clamp in SizeOf; corrected to f32::clamp (preserves NaN).
- Next immediate: run required full qbuild all on this stable milestone, fix any diagnostics, record evidence; owner remains incomplete (see README pending rows).
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin5-r1-manifest.json.
