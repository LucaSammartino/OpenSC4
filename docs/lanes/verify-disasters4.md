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
- vd4-fix-1: clean, ref0, fire 62/62 riot 546/546 exact, BUT the survey (now correct coverage) picked other cells 9,70/10,70/11,70: not the same scenario.
- Gate at e4c645b5: cargo 3614/0, clippy 0, ctest 62/62.
- Outstanding: vd4-fix-2 (build-runs/vd4-script-2.txt: base cells 58,81 57,84 58,84, riot 40,33 fixed).
- Next: qwait build-runs/game-results/vd4-fix-2.verdict (no --fresh); compare counts; then qbuild all, finish.
