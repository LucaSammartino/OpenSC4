tag: histclock-113
run: 113
date: 2026-10-15
lane: histclock
dll: FreeCity4-histclock
pacing: turbo
verdict: PASS
switches: main `efac89a` (histclock: history warehouse `cFC4HistoryWarehouse` and 24-hour clock `cFC4Clock24`, `enable = history, clock24`, their shadows `history_shadow = 1` and `clock24_shadow = 1`, the save sweep record mode; all off by default and in neither regress list; ABI 52; DLL `FreeCity4-histclock`, sha256 `fe6ede72`), `tools/regress.ps1 -Tag histclock -Queue` (turbo to 2032-01-01)
result: **PASS, 0 problems, 194 s.** Both runs clean, `ref_warnings=0`; replace 26.65 days/s (17 replacement groups, as run 112). Shadows 13.08 days/s: development 913/913 (run 112's pacing line is gone), zones 292/292, tract 145/145, demand 2241/2241, power 359/359, plumbing 144/144, residential 1138/1138, protection 288/288, landfill 72/72, flammability 54/54, land value 71/72 (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max
