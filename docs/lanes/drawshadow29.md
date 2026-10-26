| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| drawshadow29: 2D draw-stream oracle (engine) | Codex / drawshadow29 / detached HEAD; reserved branch drawshadow29 | wip; code 50cad807e, a9902adcd, 87c584f1f; ABI exports unchanged; full gate uncompleted; pilot complete=0 | 1. Exercise SubBudgetWindow selector commands before final pilot selection. 2. Resolve DS29-1 if MapView remains pilot; bind all draw/message outputs. 3. Complete full gate and exact full-run verification before enqueue. |

- Provisional pilot: cSC4WinMapView; largest reached implemented candidate, and gen27 actually enters Update; SubBudgetWindow still needs its selector-command route exercised (budget-window.md section 2.1).
- House reach: 7/30 MapView methods, 5,574 bytes, Update=0; build-runs/drawshadow29-house-r2a.reach.tsv.
- Gen27 reach: 13/30 MapView methods, 15,665 bytes, Update=24, DoMessage=178/32; build-runs/drawshadow29-gen27-r2a.reach.tsv.
- Other candidates: SubBudgetWindow, TreeView, ListBox each 0 reached methods/bytes in both traces.
- Added explicit opt-in `budgetwindow` script action (native budget slot 24), parser tests and engine/tests/scenarios/scripts/drawshadow29-budget.script. No script key/action means no effect.
- Current smoke: drawshadow29-budget-r3b clean, ref_warnings=0, foreign_exceptions=0, other warnings=0; main build-runs/game-results/drawshadow29-budget-r3b.verdict and .log; DLL SHA256 prefix 1a3952b7.
- Action log: line 2044 `budgetwindow -> ok=1 (ShowBudgetWindow)`; opens the main budget panel, not the modal sub-budget dialog. Reach: MapView 7/30, 5,574 B, Update=0; SubBudgetWindow/TreeView/ListBox 0. Trace: build-runs/drawshadow29-budget-r3b.reach.tsv.
- Prior game results: main build-runs/game-results/drawshadow29-house-r2a.verdict clean, ref_warnings=0; drawshadow29-gen27-r2a.verdict corrupt, foreign_exceptions=1, ref_warnings=0. Reach evidence only; no pilot comparison credit.
- Reusable module: FC4DrawStream.h and FC4DrawStreamCapture.*; recording context now replays native getter snapshots without drawing or calling borrowed objects again.
- Replay compares independently supplied call arguments, copies rectangles/origins, preserves borrowed engine identity and explicit image TGI keys, and refuses unsupported/malformed calls; query storage is bounded.
- Default-off drawstream_shadow key registered; capture installation is guarded against repetition; observer serialization cannot repeat native paint or swallow its exception.
- Final focused validation: qbuild quick PASS, build 48/48 edges; fc4_harness_logic_test.exe and fc4_draw_stream_test.exe exit 0. Intermediate stale parser help expectation failed on a9902adcd, corrected in 87c584f1f.
- Full gate: 50cad807e attempt stopped after >1,823 s awaiting three occupied cargo slots; resumed 61c1105bd attempt also capacity-blocked; final 87c584f1f attempt stopped after >311 s with the same barrier. No cargo/clippy/CTest verdict; see build-runs/qbuild-all-r3-status.txt and qbuild-resume-r3-status.txt. Older full-gate logs are stale.
- Pilot Draw/Update and message verification: 0 compared, 0 complete; no method-credit lines or whole-row credit claimed. Font/surface/owner/message adapters remain unbound; widget traces remain explicitly incomplete.
- Blocker: DS29-1 appended to docs/specs/questions/map-view.md; full live-owner snapshot offsets/types and bitmap presentation boundary are missing from the permitted interface sources. SDK/curated interfaces expose no alternative MapView snapshot API.
- Current game log confirms native capture installation (41 context slots) and the MapView gap diagnostic; TreeView/ListBox final counters are draws=0, events=0, complete=0. Smoke only, no ready enqueue.
- Next: send the documented sub-budget selector command through the game's window interfaces and remeasure; finish the selected owner's native snapshot and core adapters, record all message/command effects, then obtain a passing full gate and clean full game run with exact complete method lines.
