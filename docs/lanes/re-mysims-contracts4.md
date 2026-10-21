# re-mysims-contracts4 — MS8-MS10 contracts

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-contracts4 (answer MS8-MS10: job pathfinder setup, search shape, Sim template import) | `re-mysims-contracts4` / `.claude/worktrees/re-mysims-contracts4` | in progress | see Handoff |

## Handoff
- Goal: answer MS8, MS9, MS10 (docs/specs/questions/mysims.md) with byte-backed rules in docs/specs/mysims.md §4.7 / new pool section.
- Done: worktree created, lane file.
- Facts MS8/MS9 (byte-read, not yet in spec): FindNewJob 0x00425A30 -> SetCurrentCommuteDestination 0x004350D0(byCommute loc, notify=1) -> validateCommuteLocation 0x00432F50: resetPath; SetFlag(1),SetFlag(0x10); filter 0; SetDestinationFlag by GetWealth 1->1,2->2,3->4,else 1 (OR, never cleared); density 1; start rect=getLocationCellRect(+0x84 home); SetTravelTypeFlag(0x1CF,1); one FindPath; end cell in bounds -> GetLot->GetBuilding; else FindCityOnOtherSideOfConnection + clamp. Gate: dispatch flag 0x80 (valid default dest).
- Next: write spec; then read cSC4MySimDispatch SetCommuteDestination/by-commute pathfinder setup; cSC4MySimPool Import/LoadImports/FindTheSimsInstallation (0x0043B200..0x0043BC40).
