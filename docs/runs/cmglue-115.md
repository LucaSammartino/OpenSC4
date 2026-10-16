tag: cmglue-115
run: 115
date: 2026-10-15
lane: cmglue
dll: FreeCity4-cmglue
pacing: turbo
verdict: PASS
switches: main `58b6d11` (cmglue: commercial and industrial `cFC4CommercialSimulator` / `cFC4IndustrialSimulator`, civic `cFC4CivicBuildingSimulator`, parks `cFC4ParkManager`, `enable = commercial, industrial, civic, parks`, their shadows `commercial_shadow = 1`, `civic_shadow = 1`, `parks_shadow = 1`, the capi save sweep for the four records, CB6 applied to civic `Read`; all off by default and in neither regress list; ABI 53; DLL `FreeCity4-cmglue`, sha256 `11936e07`), `tools/regress.ps1 -Tag cmglue -Queue` (turbo to 2032-01-01, lists as run 114)
result: **PASS, 0 problems, 203 s.** Both runs clean, `ref_warnings=0`. Replace 29.05 days/s (20 replacement keys, as run 114). Shadows 12.02 days/s: zones 292/292, tract 145/145, zonedev 365/365, history 79/79, clock 73/73, demand 2241/2241, power 360/360, plumbing 144/144, residential 1139/1139, protection 288/288, landfill 72/72, flammability 54/54, land value 71/72 (the usual 2026-02-07 pass 2 early-settling line), every other shadow exact except **development 912/913**: growth 0x1020 on 2029-11-02 has population at 2 tracts differing (max
