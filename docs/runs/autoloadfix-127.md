tag: autoloadfix-127
run: 127
date: 2026-10-16
lane: autoloadfix
dll: FreeCity4-autoloadfix
pacing: turbo
verdict: PASS
switches: main `4c92365` (autoloadfix: the autoload ignores the return value of region slot 23 GetCityLocations, whose return channel is eax-or-void in `docs/abi/interfaces.tsv`, and goes by the list itself; a truly empty list is retried on the wall clock, about once a second, giving up 120 s after the first empty read (`FC4AutoloadLogic.h`, `diag_logic_test`); plugin only, ABI 60 unchanged; DLL `FreeCity4-autoloadfix`, sha256 `b0ada1c6`), `tools/regress.ps1 -Tag autoloadfix -Queue` (turbo to 2032-01-01, lists as run 121; baseline picked: rl4a)
result: **PASS, 0 problems, 469 s.** Both runs clean, `ref_warnings=0`. Both games log `autoload: the region lists 85 cities (GetCityLocations -> true; 0 ms and 0 empty checks before)` and `LoadCity -> true` (this build happens to return true; the lane's runs of its own DLL, sha256 `B94203A2`, logged `-> false` with the same 85 cities and loaded, which is the run 123 no-load: the old code required true). Replace 24.19 days/s (27 keys). Shadows 4.88 days/s, 444 s: zones 292/292, tract 145/145, zonedev 365/365, history 79/79, clock 73/73, demand 2237/2237, protection 288/288, parks 72/72, budget 72/72, power 389/389, plumbing 143/143, development 912/912, flora 71/71, pollution 71/71, civic 71/71, landfill 71/71, neighbours 71/71, land value 72/72, residential 1136/1136, crime 71/71, commercial 32/32, flammability 54/54, **traffic 49/94** (known difference, hypotheses 7), **aura 70/71** (cycle 2, 2026-03-11, the AU14 case of runs 120, 122, 126). The delta's "3 worse" against rl4a is `lotdev_shadow` missing (in rl4a's lists, not yet in main's), traffic (50/94 -> 49/94, moves with the pacing) and that aura line. Gate autoloadfix3 at lane `94c1d38` (main `558fe40` merged in): cargo 2182/0, clippy clean, ctest 55/55; no .rs files touched.
summary: regress autoloadfix, PASS, lists as run 121
