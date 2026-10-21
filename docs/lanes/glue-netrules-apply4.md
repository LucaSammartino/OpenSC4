| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-apply4: apply solved piece id/orientation via NR14 slot30 before InsertOccupant | fc4-lane / .claude/worktrees/glue-netrules-apply4 | in progress | see Handoff |

## Handoff
Goal: in FC4NetToolHost.cpp InsertOccupantsForSolvedCells, call occupant slot30 SetPiece(id,u8 orient) (NR14) before InsertOccupant; prove in game with road11/street8 scripts, readback matches.
Done: e8cf6484 compat::NetworkOccupantSetPiece (table VA 0x0061B490, vtbl[30]==VA check, thiscall u32,u32) + host applies before PlaceOccupant; Pipe/Subway skipped -> NR15 filed. quick PASS.
Next: wait game nrapply4-g1 (qwait without --fresh), grep 'netrules insert' in log; then qbuild all; finish.
Facts: base main 530577d7. script build-runs/nrapply4.script (road 4,4-14,4; road 9,1-9,8; street 20,4-27,4), req label nrapply4-g1 queued.
