| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-occmgr4 (verify the occupant-manager shadow, native original + occmgr_shadow=1) | `verify-occmgr4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: run FC4OccMgrShadow (original vs core) with occmgr replacement OFF, occmgr_shadow=1, to 2032; report exact-set counts and resync counters; fix narrow mismatches or file OM questions.
- Done: worktree at main 40f7799e.
- Next: read engine/plugin/FC4OccMgrShadow.h 14-99, baseline `qbuild all`, write game request (label vom4-shadow-1).
- Facts: no game requests outstanding.
