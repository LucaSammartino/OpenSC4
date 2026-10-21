| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-apply4: answer NR14 (how a solved piece id/orientation reaches an occupant) | fc4-lane / .claude/worktrees/re-netrules-apply4 | in progress | see Handoff |

## Handoff
- Goal: answer NR14 in docs/specs/network-rules.md §11; static only (REA: no app open).
- Done: worktree.
- Next: read InsertOccupantsForSolvedCells 0x006258C0 / CreateNetworkOccupant 0x00624210 in decomp/cSC4NetworkTool.c.
