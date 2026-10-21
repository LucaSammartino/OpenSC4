| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-disasters4 (verify the non-exact disasters_shadow: fire 211/263, riot 2/354) | `verify-disasters4` / (engine lane) | in progress | - |

## Handoff
- Goal: find the first primary mismatch of disasters_shadow (fire first) under a scripted disaster, fix if spec-supported else file question.
- Done: worktree at main 4726020a.
- Baseline gate on main 4726020a: cargo 3614/0, clippy 0, ctest 62/62.
- Note: glue20 (merged train4) reported glue20-fire-2 exact (fire 256/256, riot 352/352); the 211/263 / 2/354 coverage figure is glue13's older dis-final-11.
- Outstanding game request: vd4-base-1 (build-runs/vd4-script-1.txt: survey, 3 fire auto, riot auto; normal pacing to 2026-05-01, disasters_shadow=1).
- Next: qwait build-runs/game-results/vd4-base-1.verdict (no --fresh), grep the log for 'disasters shadow'.
