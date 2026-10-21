| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-construction4 (construction occupant record + lifecycle, lot-records §9) | `core-construction4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: `cSC4ConstructionOccupant` in fc4-core `lot_records` (lr §9/§9.1/§9.2, lot-developer §6.7): record Write/Read
  with version quirks, state, Init/SetLot/DoDailySim via typed host callbacks. Pure Rust, game=false.
- Done: nothing yet.
- Next: read lot-developer.md §6.7 (lines 936-1165), write engine/core/src/lot_records/construction.rs.
