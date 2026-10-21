# re-mysims-contracts4 — MS8-MS10 contracts

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-contracts4 (answer MS8-MS10: job pathfinder setup, search shape, Sim template pool) | `re-mysims-contracts4` / `.claude/worktrees/re-mysims-contracts4` | ready: MS8, MS9, MS10 answered (all DIFFERS); round 2 closed the 3 MS10 inferences byte-read; mysims.md §4.7.1 and §4.12 | engine: rework dispatch job search and pool cores to §4.7.1/§4.12; add conformance vectors from the worked examples |

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
- Round 2 (MS10 closeout, no inference left in §4.12):
  - Gender: `IsEqual` 0x0090EC57 called at 0x0043CDC2 with cs flag = EBX = 0 (zeroed 0x0043BC48, never rewritten):
    length check, then MakeLower both + memcmp -> ASCII case-insensitive, no trim.
  - Portrait: LoadImports stores record bytes + `.bmp` (0x00A85B38) at template +0x04, no directory; PickHead
    0x007757F0 joins GetMySimDirectory() + name for bitmap ID 0. Data file = GetMySimDirectory() + static
    `MySims.dat` (0x00A85CF0 -> 0x00B20E6C).
  - Cache: Import 0x0043CFF5..0x0043D006: count > 0 and loaded -> vector clear 0x0043BB90 + loaded flag := 0;
    count 0 keeps stale imports (quirk). GetMySimTemplates assigns (operator= 0x0043B580), not appends.
  - Corrected template offsets: +0x00 bitmap, +0x04 portrait, +0x18 name, +0x2C male, +0x2D sign.
  - Conformance example table at the end of §4.12.
  file name with `.bmp` (directory joining not traced); the post-import cache clear `0x0043BB90`.
- All byte-read with re_query.py asm and a PE string reader (build-runs/peread.py); REA not used.
- No engine/, tools/ or docs/abi edits; no ABI rows.
