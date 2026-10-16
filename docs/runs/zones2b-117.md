tag: zones2b-117
run: 117
date: 2026-10-15
lane: zones2b
dll: FreeCity4-zones2b
pacing: turbo
verdict: PASS
switches: main `3cb9b43` (zones2 Stage B: the zone developer / lot manager `cFC4ZoneDeveloper` now creates game lots: `NewLot` from the game's lot pool and constructor (direct calls `0x006BF430`, `0x006C1E50`), slot 21 CanCreatePloppedLot, UpdateZoneType, the §5.2.1-§5.2.2 street joins, 17 new host callbacks; `enable = zonedev` off by default and not in the replace list; ABI 55; seam op 53; DLL `FreeCity4-zones2b`, sha256 `0b3737bd`), `tools/regress.ps1 -Tag zones2b -Queue` (turbo to 2032-01-01, lists as run 114)
result: **PASS, 0 problems, 234 s.** Both runs clean, `ref_warnings=0`. Replace 26.51 days/s (20 replacement keys, as run 116). Shadows 10.46 days/s: zones 292/292, tract 145/145, zonedev 365/365 (the Stage B core behind the shadow), history 79/79, clock 73/73, demand 2239/2239, power 372/372, plumbing 144/144, development 913/913, residential 1138/1138, protection 288/288, landfill 72/72, flammability 54/54, land value 71/72 (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max
