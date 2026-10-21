| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-apply4: apply solved piece id/orientation via NR14 slot30 before InsertOccupant | fc4-lane / .claude/worktrees/glue-netrules-apply4 | in progress | see Handoff |

## Handoff
Goal: in FC4NetToolHost.cpp InsertOccupantsForSolvedCells, call occupant slot30 SetPiece(id,u8 orient) (NR14) before InsertOccupant; prove in game with road11/street8 scripts, readback matches.
Done: -
Next: read NR14 answer (docs/specs/questions/network-rules.md), network-rules.md 11/11.2/15; implement.
Facts: base main 530577d7.
