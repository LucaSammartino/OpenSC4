| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-terrainview29: terrain 3D view; kind engine | Codex / replace-terrainview29 / detached HEAD (supervisor publishes reserved branch); merge 798a803eb, code ddc7804ee | blocked: TR47; cargo 8021/8021, clippy 0 diagnostics, CTest 130/130, fmt PASS; ABI unchanged; no seam reservation; game none, shadow unmeasured | 1. Obtain TR47 observations/rules. 2. Implement direct-entry host bridge and class/key. 3. Compare frames and complete (a)-(c), then enqueue kind engine. |

- Round 3 merged main first. TR45 is answered: original terrain directly calls Init/Shutdown/index entries; an independent replacement requires the section 15.5.14 host bridge, not only factory registration. That bridge/class/key is not implemented.
- Implemented TR46's row-relative six-u16 index table, all split/winding/hide combinations, whole-grid initialization and per-cell rewrites; added verified plain floor/water row call order, counts and byte offsets. Existing height snapshots remain intact; this is a portable subset, not a complete renderer.
- Focused: 4/4 conformance tests pass, including the W=5/S=24 worked row and index vectors. fmt PASS, 4 owned files. Evidence: `build-runs/terrainview-r3-focused.log`; the failed q.py startup and corrected fmt attempts are distinguished there.
- (d) Round-3 `qbuild all`: PASS, one invocation (test debug=0, jobs=1); cargo 8021 passed/0 failed, clippy 0 diagnostics, native build 328/328, CTest 130/130. Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`. No code changed after this gate.
- Complete frame comparison: NOT RUN, no game log. Section 15.5.15.4 explicitly leaves full cache/edge-index bytes, boundary rounding and grid/plumbing/extension streams unestablished; native passes were not forwarded or credited as replacement work.
- New question TR47: `docs/specs/questions/terrain.md`, requesting those remaining fixtures/rules; stopped that item rather than inventing bytes or omitting passes. Removed stale open TR45/TR46 copies because their answers landed in main.
- (a) House replace to 2032: NOT RUN; clean/ref_warnings unmeasured, no game label/log; terrainview is not registered.
- (b) Key-off owner shadow and bidirectional key-on/off save/load: NOT RUN; shadow exact/total and save compatibility unmeasured, no game label/log.
- (c) gen27 s0-grow/s1-grow with/without key: NOT RUN; differential failures unmeasured, no game label/log. No claim about native shutdown faults or the automata fix.
- Credit: cSTETerrainView3D (42,756 B, engine/core/src/terrain) is NOT flipped; terrainview is NOT ready for the replace list. WIP queue only; no ready engine enqueue or game request.
- Next: obtain TR47 evidence, implement the specified host bridge and full view under default-off terrainview, then measure frame parity and (a)-(c) before ready enqueue.
