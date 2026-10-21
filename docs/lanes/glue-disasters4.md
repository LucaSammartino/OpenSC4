| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters4 (`enable = disasters`: native cSC4DisasterLayer replacement glue) | `glue-disasters4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: `enable = disasters` glue: cFC4DisasterLayer replacing cSC4DisasterLayer, over fc4_disasters_* and FC4DisasterHost; keep native manager/instance lifetimes.
- Done: worktree created at main a94e4296.
- Next: read disasters.md §1-3 (lines 54-366), docs/abi rows for cSC4DisasterLayer, existing glue pattern (cFC4Dispatch).
- Facts: fire 124/124, riot 481/481 shadow already exact (verify-disasters4 vd4-fix-2).
