tag: lotglue-116
run: 116
date: 2026-10-15
lane: lotglue
dll: FreeCity4-lotglue
pacing: turbo
verdict: PASS
switches: main `5212df2` (lotglue: lot developer `cFC4LotDeveloper`, `enable = lotdev`, host `FC4LotDevHost` with nine switches, the lot developer shadow `lotdev_shadow = 1`; off by default and in neither regress list; ABI 54; seam op 52; 219 switches; DLL `FreeCity4-lotglue`, sha256 `371958fb`), `tools/regress.ps1 -Tag lotglue -Queue` (turbo to 2032-01-01, lists as run 114)
result: **PASS, 0 problems, 218 s.** Both runs clean, `ref_warnings=0`. Replace 26.24 days/s (20 replacement keys, as run 115). Shadows 11.27 days/s: zones 292/292, tract 145/145, zonedev 365/365, history 79/79, clock 73/73, demand 2245/2245, power 369/369, plumbing 145/145, development 913/913 (run 115's pacing line is gone), residential 1143/1143, protection 288/288, landfill 72/72, flammability 54/54, land value 72/73 (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max
