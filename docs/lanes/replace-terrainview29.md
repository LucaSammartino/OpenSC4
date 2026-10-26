| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-terrainview29: terrain 3D view; kind engine | Codex / replace-terrainview29 / detached HEAD (supervisor publishes reserved branch); 234b7d6e8, ebf08ae70, ee43bc4e7 + handoff | blocked: TR45/TR46; focused 6/6, fmt PASS, full gate incomplete; ABI exports unchanged; no seam reservation; no game labels, shadow unmeasured | 1. Resolve TR45/TR46. 2. Collect pending full gate or recover the stalled shell transport. 3. Implement class/key and frame capture, complete (a)-(c), enqueue kind engine. |

- Implemented only the portable terrain/water height snapshots: inclusive packed rectangles, independent message copies, live source water calculation, preserved terrain float bits, atomic rejection of invalid inputs.
- Round 2 adds conformance coverage for live sea-level reads, strict-positive water depths including signed zero, ignored SeaLevelChanged posts, and altitude NaN payload copies.
- Focused: 6/6 pass against real compiled Terrain with current view source and `-D warnings`; `build-runs/terrainview-r2-focused.log` contains the command and captured output. This is not a Cargo workspace gate. fmt PASS, 1 owned file.
- (d) Round-2 `qbuild all` invoked once at ee43bc4e7 (test debug=0, jobs=1); queued about 15 minutes, then compiled fc4-core from 10:42 UTC. No observed test/clippy/CTest verdict. Log: `build-runs/qbuild-test.log`. Build session 81789 remains pending; new shell calls stalled, so no complete gate is claimed. The previous interrupted attempt is described in `build-runs/terrainview-focused.log`.
- Frame comparison: NOT RUN; no replacement class/key registered. TR46 requests the missing full frame/indices rules; native passes have not been forwarded or credited as replacement work.
- (a) House replace through 2032: NOT RUN; clean/ref_warnings unmeasured, no game log. Independent registration is blocked by TR45's native direct-call lifecycle boundary.
- (b) Owner shadow with key off and bidirectional save/load: NOT RUN; exact/total unmeasured, no game log. No plugin registration or C ABI change was made.
- (c) gen27 s0-grow/s1-grow with/without key: NOT RUN; no differential verdict or game log. No claim about the known native shutdown fault or automata fix.
- Questions filed: TR45 and TR46 in `docs/specs/questions/terrain.md`; no behavior spec edited.
- terrainview is NOT ready for the replace list. WIP queue file only; no engine enqueue or game requests submitted.
