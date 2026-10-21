| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-construction-contractfix4 (apply OP13-OP16 answers to construction occupant, lr §9.1/9.2, ld §6.7.1/6.7.6/6.7.7) | `core-construction-contractfix4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: re-key ConstructionScalars to lr §9.1 object-frame roles; resume schedule from #14; refs #3/#4 live vs #8 load-only (+D8/+D9 refresh); Init = base Init + 4 subs + AddOccupantGroup(0x5007); sparks x/z extent; crane T/k/swing (ld §6.7.6). Keep v4 transform.
- Done: worktree created from main 802fa8d4.
- Next: read lr §9.1-9.2 (lines 549-660) and ld §6.7.1/6.7.6/6.7.7; edit engine/core/src/lot_records/construction.rs.
- Facts: one round only; park if unfinished.
