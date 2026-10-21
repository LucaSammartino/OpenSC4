| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: run FC4OccMgrShadow (original vs core) with occmgr replacement OFF, occmgr_shadow=1, to 2032; report exact-set counts and resync counters; fix narrow mismatches or file OM questions.
- Done: worktree at main 40f7799e.
- Baseline build PASS 186/186 (ctest result in build-runs/baseline-all.txt).
- Next: wait for verdict build-runs/game-results/vom4-orig-shadow-2032a.verdict (qwait without --fresh), grep its log for "occmgr shadow".
- Facts: request vom4-orig-shadow-2032a outstanding (turbo to 2032-01-01, disable=occmgr, occmgr_shadow=1, probe=1; occmgr is off by default anyway).
