# re-mysims-contracts4 — MS8-MS10 contracts

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-contracts4 (answer MS8-MS10: job pathfinder setup, search shape, Sim template pool) | `re-mysims-contracts4` / `.claude/worktrees/re-mysims-contracts4` | ready: MS8, MS9, MS10 answered (all DIFFERS), mysims.md §4.7.1 and §4.12 added | engine: rework dispatch job search and pool cores to §4.7.1/§4.12; add conformance vectors from the worked examples |

## Report

- **MS8 (DIFFERS)**: `validateCommuteLocation` `0x00432F50` (via `FindNewJob` `0x00425A30` ->
  `SetCurrentCommuteDestination` `0x004350D0`): resetPath, flags 1 then 0x10, filter **0**, destination flags by
  MySim wealth (1->0x1, 2->0x2, 3->0x4, else 0x1; OR-accumulated on the long-lived pathfinder), density **1**,
  start rect = home location lot rect, travel mask 0x1CF allowed+starting, one FindPath. Gate: dispatch flag 0x80.
  End cell in bounds -> GetLot/GetBuilding; else FindCityOnOtherSideOfConnection, clamp, neighbour fields.
- **MS9 (DIFFERS)**: one filter-0 search, no per-candidate loop.
- **MS10 (DIFFERS)**: pool vtable `0x00A85B14`; templates = exemplars (type 0x6A297266, prop 0x10 = 0x26) +
  `MySim\MySims.dat` (magic 0x4A32E4AA, u8-len name, u8 male, u8 sign, u8-len portrait + .bmp); registry
  `HKLM\SOFTWARE\Maxis\The Sims\SIMS_DATA` then `InstallPath`; Import walks UserData, UserData2..99 \Export\*.pub
  with matching .ini ([WebPages] FamilyDescription, [FamilyInfo] keys), zodiac via 12 LTEXT (group 0x6A231EAA).
- Inferred and marked: IsEqual flag meaning in the gender test; portrait path prefix for imported templates is the
  file name with `.bmp` (directory joining not traced); the post-import cache clear `0x0043BB90`.
- All byte-read with re_query.py asm and a PE string reader (build-runs/peread.py); REA not used.
- No engine/, tools/ or docs/abi edits; no ABI rows.
