| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: run FC4OccMgrShadow (original vs core) with occmgr replacement OFF, occmgr_shadow=1, to 2032; report exact-set counts and resync counters; fix narrow mismatches or file OM questions.
- Done: worktree at main 40f7799e.
- Baseline build PASS 186/186 (ctest result in build-runs/baseline-all.txt).
- Baseline gate: cargo 3930/0, clippy 0, ctest 63/63.
- vom4-orig-shadow-2032a (turbo to 2032-01-01, disable=occmgr, occmgr_shadow=1, probe=1): clean, ref_warnings=0; 72 month lines, all followed=yes, resyncs=0, record exact 72/72 (23 months with set-aside, 311 set aside total), mismatches=0; queries followed core 4312/4312 (49 months), fresh core 2024/2024 (23 months); per slot 288/288 for 25,27,28-47; msgorder 27835/27835; boxes 13842/13842; ins/rem/move 13842/13993/0, silent moves 0; probe other-thread 0, overlaps 0; stray releases 0.
- Fix 61e000c7: occmgr_shadow keys registered in FC4Config kOtherKeys (were logged "unknown key").
- Next: wait build-runs/game-results/vom4-orig-shadow-normal-b.verdict (normal speed 300 s, DLL 54fce078), then final gate `qbuild all` and finish.
