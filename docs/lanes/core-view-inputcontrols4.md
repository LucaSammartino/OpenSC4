| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-inputcontrols4 (pure core view input tools) | `Codex` / `core-view-inputcontrols4` (engine lane) | in progress | Implement published supported owners, fixtures and inventory; full gate. |

## Handoff
- Goal: new production `engine/core/src/view_input_controls/`, published sections 1-8/10 only.
- Done: independent fixtures.md written first; initial production scroll, level, label, sign, demolition and filter algorithms implemented.
- Done: reused current ViewUtilities/Pixel/Vec3 and Terrain APIs; no native owner pass-through.
- Done: production milestone `558e8ba7`; full published method/state-field inventory (byte lengths absent, VIC5).
- Done: independent executable fixtures added for all five tools/three filters; VIC3-VIC7 appended to questions/view-input-controls.md.
- Next: wait focused qbuild quick sessions 83085/58241 (second was inadvertently overlapped; stop launching builds), address measured failures, then stable final all.
- Baseline: cargo 4987/0; clippy/build failed and tree-changed, invalidated by edits during the run. Original logs preserved in build-runs/baseline/.
- Formatting: all 9 new Rust files pass qbuild fmt; including existing lib.rs fails its pre-existing module-order/format diffs (preserved fmt-including-existing-lib.log); keep additive export at list end.
- No ABI changes, game requests, native-exact claims, or full base-control ownership.
- q.py cannot launch Bash here (E_ACCESSDENIED); use native PowerShell with capped output.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-view-inputcontrols4-r1-manifest.json`.
