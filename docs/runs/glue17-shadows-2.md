# Run glue17-shadows-2

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/glue17-shadows-2.req`
in `.claude/worktrees/glue17`; results `build-runs/game-results/glue17-shadows-2.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| glue17-shadows-2 | as `glue17-shadows-1` (a repeat of the same request) | **clean**, `ref_warnings=0`, rtc=0, overruns=0. Date 2032-01-11, 25.312 s real, 86.95 days/s. Identical shadow results to `-1` (demolition 74/74; citysit 74 lines with `maxCSI` and, from 2027-01, `tracked`; dispatch 74 lines with `availPylons(0)` and `availPylons(1)`). Problems all 0. Slot 2, wall 36 s. Together with `-1` this fixes the three differences as properties of the shadow's scope, not of the run. |
