# core-mysims-contractfix4 — correct MySim dispatch/pool cores to answered MS11/MS12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-contractfix4 (correct mysims dispatch §4.7.1 / pool §4.12 cores to MS11/MS12 answers) | `core-mysims-contractfix4` / `.claude/worktrees/core-mysims-contractfix4` | in progress | see Handoff |

## Handoff
- Goal: dispatch neighbor=-1, IsValid (non-ferry/city bounds, no occupant check), SetByOccupant clears byCommute,
  trip-state field, job-search notifications; pool import lengths mod 256, UserData portrait path. Core +
  conformance only (no plugin/CAPI).
- Done: (nothing yet)
- Next: read spec §4.7.1 (392-543), §4.12 (635-781), answered MS11/MS12; read engine/core/src/mysims/{dispatch,pool}.rs.
- Facts: base main d68b254a.
