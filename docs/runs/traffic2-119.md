tag: traffic2-119
run: 119
date: 2026-10-15
lane: traffic2
dll: FreeCity4-traffic2
pacing: turbo
verdict: PASS
switches: main `40d9eab` (traffic2: the traffic shadow `traffic_shadow = 1` (core `traffic_simulator/shadow.rs`, capi `traffic_shadow.rs`, plugin `FC4TrafficShadow`, seam op 51; switches `traffic_shadow_diffs`, `traffic_shadow_lines`, `traffic_shadow_hypotheses`), TS35-TS49 applied with vectors, the O(1) source iterator, the crime-lot ref warning fix, `path_info` wired; `traffic` and `traffic_shadow` off by default and in neither regress list; ABI 56; DLL `FreeCity4-traffic2`, sha256 `84039def`), `tools/regress.ps1 -Tag traffic2 -Queue` (turbo to 2032-01-01, lists as run 118)
result: **PASS, 0 problems, 264 s.** Both runs clean, `ref_warnings=0`. Replace 21.51 days/s (26 replacement keys, as run 118), shadows 9.23 days/s: zones 292/292, tract 145/145, zonedev 365/365, history 79/79, clock 73/73, demand 2237/2237, parks 72/72, budget 72/72, power 375/375, plumbing 144/144, development 913/913, protection 288/288, flora 72/72, residential 1137/1137, commercial 18/18, civic 71/71, flammability 54/54, every other shadow exact except land value 71/72 (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max
