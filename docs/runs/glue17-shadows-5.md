# Run glue17-shadows-5

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/glue17-shadows-5.req`
in `.claude/worktrees/glue17`; results `build-runs/game-results/glue17-shadows-5.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| glue17-shadows-5 | lane glue17 at `55c811a7` + the dispatch shadow's "not compared" sentence brought in line with the code (it still named the pylon list as one thing rather than as the source of `GetAvailablePylonCount`); Konradshohe, `turbo = 1`, `end_date = 2032-01-01`, `seconds = 600`; `citysit_shadow = 1`, `demolition_shadow = 1`, `dispatch_shadow = 1`, `popup_dialogs = 0`; DLL `engine/build-x86/FreeCity4.dll` | **clean**, `ref_warnings=0`, rtc=0, overruns=0. Date 2032-01-07, 26.484 s real, 82.88 days/s. **All three shadows exact**: citysit 73/73 (1095 values), demolition 73/73 (146 values), dispatch 73/73 (876 values), every line `roundtrip=same` and `mismatches=0`. The dispatch "not compared" line now reads `... the tool's pylon list and everything derived from it (Begin / End, slots 16, 17, and GetAvailablePylonCount, slot 21: the free units of the sources' stations, which a record-only shadow cannot see) ...`. Problems all 0. |
