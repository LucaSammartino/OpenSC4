# Run glue17-shadows-4

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/glue17-shadows-4.req`
in `.claude/worktrees/glue17`; results `build-runs/game-results/glue17-shadows-4.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| glue17-shadows-4 | as `glue17-shadows-3` (a repeat of the same request) | **clean**, `ref_warnings=0`, rtc=0, overruns=0. Date 2032-01-04, 26.547 s real, 82.57 days/s. **All three shadows exact**: citysit 73/73 (1095 values), demolition 73/73 (146 values), dispatch 73/73 (876 values). One shadow line fewer than `-3` (the run stopped a fortnight earlier under turbo); the same 73 lines are exact on all three. Problems all 0. Slot 1. |
