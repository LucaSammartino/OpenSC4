| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| drawshadow29: 2D draw-stream oracle (engine) | Codex / drawshadow29 / detached HEAD | r4 in progress; DS29-1 merged; ABI exports unchanged so far | Finish same-invocation surface oracle; gate; measure; enqueue actual HEAD. |

- Pilot: cSC4WinMapView (30,448 B); largest implemented candidate reached by the recorded runs.
- House reach: MapView 7/30 methods, 5,574 B, Update=0; build-runs/drawshadow29-house-r2a.reach.tsv.
- Gen27 reach: MapView 13/30 methods, 15,665 B, Update=24, DoMessage=178/32; build-runs/drawshadow29-gen27-r2a.reach.tsv.
- SubBudgetWindow, TreeView, ListBox: 0 reached methods/bytes in both traces.
- Budget action retained: drawshadow29-budget-r3b clean, ref_warnings=0; main build-runs/game-results/drawshadow29-budget-r3b.log confirms ShowBudgetWindow ok=1 (main panel).
- DS29-1 supplies O/O+4/O+8 bindings and the bitmap boundary: compare logical surface rows at MiniMap.SetOverlay, separately from later paint.
- Restored the reusable bounded context recorder/replay and default-off drawstream_shadow after merging main.
- r4 pilot measurements and full gate pending; previous rounds have no complete MapView method credit.
- Next: implement/capture the raster boundary, report operation count/exact/first difference, run qbuild all once and final game verification.
