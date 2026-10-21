| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-inputs4: LV1 native pass input capture timing and startup desirability snapshot | spec writer (RE), `.claude/worktrees/re-landvalue-inputs4` | in progress | see Handoff |

## Handoff
- Goal: answer LV1 (docs/specs/questions/landvalue.md): when the original reads each input across its multi-frame Step pass, startup snapshot timing, unmatched pass; spec the typed-input boundary in landvalue.md section 6/7.
- Done: lane file 50945016; landvalue.md 6.1 (per-state read table, slicing, object offsets, capture boundary, typed input record, startup) + LV1 answered DIFFERS, QUESTIONS index (commit 2).
- Next: final report, merge-queue json.
- Facts: DoIdle 0x006BE7B0, budget test before GetNext 0x006BE865; evidence log main build-runs/landed-evidence/verify-landvalue49/game-results/vlv49-a1.log lines 1841/3544/3907; a1 p2 unprovable (no per-call capture).
