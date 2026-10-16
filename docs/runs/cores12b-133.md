tag: cores12b-133
run: 133
date: 2026-10-16
lane: cores12b
dll: FreeCity4-cores12b
pacing: turbo
verdict: PASS
switches: main `3eb6d22` (cores12b: city detail manager, airport / seaport developers and prop developer cores (`engine/core/src/{city_detail,port_developer,prop_developer}/`), capi `fc4_city_detail`, `fc4_port_developer`, `fc4_prop_developer` and the list sinks, `cd8_*` / `pd12_*` / `pv12_*` vectors, the re-specs10 interfaces.tsv rows; **no plugin code**; ABI 65, taken as 61 on the lane; DLL `FreeCity4-cores12b`, sha256 `6c42151f`), `tools/regress.ps1 -Tag cores12b -Queue` (turbo to 2032-01-01, lists as run 132; baseline picked: masterlist)
result: **PASS, 0 problems, 927 s.** Both runs clean, `ref_warnings=0`, rtc=0; the test city loads in both (slots 2 and 3). Replace 16.60 days/s. Shadows 2.41 days/s: zones 292/292, tract 145/145, lotdev 73/73, zonedev 365/365, history 79/79, clock 73/73, demand 2238/2238, parks 72/72, budget 72/72, power 395/395, protection 288/288, plumbing 144/144, development 913/913, flora 72/72, pollution 71/71, civic 71/71, landfill 71/71, neighbours 71/71, residential 1136/1136, crime 71/71, aura 71/71, commercial 32/32, flammability 54/54, **land value 70/72** (2026-02-10 pass 2 and 2026-07-09 pass 7, both `settled=0`: an input changed after the original's pass), **traffic 46/98** (known difference, hypotheses 7). Delta against masterlist: land value 72/72 -> 70/72 and traffic 60/84 -> 46/98 (worse; both timing / pacing lines, and this merge adds no plugin code), aura better, `lotdev` new 73/73, the rest same.
summary: regress cores12b, PASS, lists as run 132
