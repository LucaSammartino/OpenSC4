| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-netocc27: pole terrain/geometry and effects groundwork (engine) | Codex / `replace-netocc27`, detached HEAD; code through `64cbc4cba`; 2 private C transport entries added, public header/version unchanged | blocked: NO44; cargo 7856/7856, clippy 0, CTest 118/118; native poles retained; netocc NOT ready for replace list | 1. Resolve NO44. 2. Finish tool bridge/model/render glue. 3. Measure (a)-(c), then enqueue. |

- Resumed gate from round 1: old session unavailable; saved CTest log showed 118/118 passed; superseded by the completed gate below.
- Round 2 commits: `c5bd7c1a7`, `accd8291d`, `cc7a6df83`, `64cbc4cba` (code/tests).
- Movement now uses terrain width/corner altitudes, rebuilds floor/walls and moves the zap effect; preserves transform, wires and bounds-cache quirks (spec 4.7).
- View rotation rebuilds the geometry; drawing submissions and delegate model movement/loading remain unwired, with SPEC-GAP comments.
- Terrain slot 6 uses a verified float/x87 interface prefix, covered by required generated ABI conformance; SDK declaration remains untouched.
- Private C additions: `fc4_netpole_set_position`, `fc4_netpole_rebuild_geometry`; no `fc4.h` or ABI-version edit.
- Focused core: 14/14 passed, including two new position tests. Initial CAPI compile failed the raw-pointer borrow lint; corrected and passed in the full gate.
- Corrected full gate (`64cbc4cba`): cargo 7856/7856, clippy 0 diagnostics, native build 47/47 edges, CTest 118/118; qbuild PASS.
- Gate logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
- First round-2 gate: cargo/clippy/build passed; CTest 117/118 (new SDK CellWidth call voided its ABI waiver); fixed by the typed prefix.
- Distinct earlier results: `build-runs/position-focused-first-result.txt`, `build-runs/position-gate-first-result.txt`; round-1 failure: `build-runs/qbuild-first-failure.txt`.
- (a) House list + netocc to 2032: NOT RUN; live pole factory remains native (guard in `engine/plugin/FC4Subsystems.cpp`).
- (b) Placement/extension/demolition/fire and equal power month lines: NOT RUN; native tool boundary unresolved.
- (c) Netocc shadow 72/72: NOT RUN in this lane; no new exactness or ref_warnings measurement claimed.
- Existing NO44: `docs/specs/questions/network-occupants.md`; exact helper views/container ownership and concrete-access audit remain unpublished. Generic ABI rows alone do not resolve it.
- Missing: native-tool bridge, cell-vector output, model-instance QI/model host and rendering submissions. Pipe classes not attempted.
- Netocc is NOT ready for the replace list. No game request or ready enqueue submitted; local merge-queue entry is wip.
- Next: resolve NO44, finish tool/model/render glue, measure all three acceptance criteria, then enqueue the completed lane.
