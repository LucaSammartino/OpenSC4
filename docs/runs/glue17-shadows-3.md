# Run glue17-shadows-3

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/glue17-shadows-3.req`
in `.claude/worktrees/glue17`; results `build-runs/game-results/glue17-shadows-3.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| glue17-shadows-3 | lane glue17 at `55c811a7` (the three differences moved out of the compared set and named in each shadow's game-start line); Konradshohe, `turbo = 1`, `end_date = 2032-01-01`, `seconds = 600`; `citysit_shadow = 1`, `demolition_shadow = 1`, `dispatch_shadow = 1`, `popup_dialogs = 0`; DLL `engine/build-x86/FreeCity4.dll`, sha256 `0d4a20e9` | **clean**, `ref_warnings=0`, rtc=0, overruns=0. Date 2032-01-18, 24.641 s real, 89.56 days/s. **All three shadows exact**: citysit 74/74 (1110 values compared, 0 mismatched), demolition 74/74 (148 values), dispatch 74/74 (888 values). Every line `record=N bytes roundtrip=same`, `mismatches=0`; the load lines are `citysit ... record=7 bytes`, `demolition ... record=2 bytes`, `dispatch ... record=22 bytes`. Problems all 0. Slot 2. |
