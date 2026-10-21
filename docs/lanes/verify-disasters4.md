| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-disasters4 (verify the non-exact disasters_shadow: fire 211/263, riot 2/354) | `verify-disasters4` / (engine lane) | in progress | - |

## Handoff
- Goal: find the first primary mismatch of disasters_shadow (fire first) under a scripted disaster, fix if spec-supported else file question.
- Done: worktree at main 4726020a.
- Baseline gate on main 4726020a: cargo 3614/0, clippy 0, ctest 62/62.
- Note: glue20 (merged train4) reported glue20-fire-2 exact (fire 256/256, riot 352/352); the 211/263 / 2/354 coverage figure is glue13's older dis-final-11.
- vd4-base-1 (main 4726020a): clean, ref_warnings=0, fire 130/140, riot 465/465, quiet 0/0; residue Q40 (spread Ignite refused by orig at step 66, rect=56,86..56,87 coverage=1) + Q42 lag.
- Diagnosis + fix e4c645b5: host passed bottom-right corners to grid slot 8, spec (simgrid 4.1, disasters 5.4) says w,h. DS41 filed (empty rect).
- Outstanding game request: vd4-fix-1 (same script, fix tip).
- Next: qwait build-runs/game-results/vd4-fix-1.verdict (no --fresh); compare counts; then qbuild all, finish.
