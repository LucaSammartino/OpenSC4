| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-terrainview29: terrain 3D view; kind engine | Codex / replace-terrainview29 / detached HEAD (supervisor publishes reserved branch); 234b7d6e8 + final test/handoff commit | blocked: TR45/TR46; focused 5/5, fmt PASS, full gate incomplete; ABI exports unchanged; no seam reservation; no game labels, shadow unmeasured | 1. Resolve TR45/TR46. 2. Implement class/key and frame capture. 3. Complete full gate and (a)-(c), then enqueue kind engine. |

- Implemented only the portable terrain/water height snapshots: inclusive packed rectangles, independent message copies, live source water calculation, preserved terrain float bits, atomic rejection of invalid inputs.
- Focused: 5/5 pass against real compiled Terrain with current view source and `-D warnings`; `build-runs/terrainview-focused.log` contains the command and captured output. This is not a cargo workspace gate.
- (d) `qbuild all` attempted once at 234b7d6e8 and interrupted after >26 minutes compiling; no test/clippy/CTest verdict. Compilation log: `build-runs/qbuild-test.log`; interruption details: `build-runs/terrainview-focused.log`. Final delta adds one passing test only. fmt PASS, 1 owned file.
- Frame comparison: NOT RUN; no replacement class/key registered. TR46 requests the missing full frame/indices rules; native passes have not been forwarded or credited as replacement work.
- (a) House replace through 2032: NOT RUN; clean/ref_warnings unmeasured, no game log. Independent registration is blocked by TR45's native direct-call lifecycle boundary.
- (b) Owner shadow with key off and bidirectional save/load: NOT RUN; exact/total unmeasured, no game log. No plugin registration or C ABI change was made.
- (c) gen27 s0-grow/s1-grow with/without key: NOT RUN; no differential verdict or game log. No claim about the known native shutdown fault or automata fix.
- Questions filed: TR45 and TR46 in `docs/specs/questions/terrain.md`; no behavior spec edited.
- terrainview is NOT ready for the replace list. WIP queue file only; no engine enqueue or game requests submitted.
