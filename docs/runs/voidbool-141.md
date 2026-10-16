tag: voidbool-141
run: 141
date: 2026-10-16
lane: voidbool
dll: FreeCity4-voidbool
pacing: turbo
verdict: PASS
switches: main `2501158` (voidbool: the return-channel audit; `engine/tests/check_ret_channels.py` (ctest `ret_channel_check`) matches every use of a game method's return value in `engine/plugin` to the slot's `ret` column and to cited spec facts, and 10 call sites in 7 groups that read a bool the spec does not give were fixed: budget slots 29/31 GetAllGroups / GetDepartmentBudgetsInGroup (FC4BudgetShadow SumDepartments, FC4Observer; F1/F2), lot manager slot 24 DeleteLot (FC4LotDevHost, FC4TractHost; F3), exemplar holder slot 3 SetDefaultExemplar (cFC4Development, now judged by GetDefaultExemplar; F4), dispatch slot 13 RegisterDispatchSource (cFC4Protection; F5), lot configuration slot 18 (FC4TractHost; F6), lot developer slot 21 AttemptLevelling read as int32 (cFC4Landfill; F7); plugin and test only, ABI stays 68; DLL `FreeCity4-voidbool`, sha256 `61c0dbc5`), `tools/regress.ps1 -Tag voidbool -Queue` (turbo to 2032-01-01, lists as run 138; baseline picked: abitidy2)
result: **PASS, 0 problems, 519 s.** Both runs clean, `ref_warnings=0`, rtc=0. Replace 27.12 days/s (27 keys). Shadows 4.42 days/s: zones 292/292, tract 145/145, lotdev 73/73, zonedev 365/365, history 79/79, clock 73/73, terrain 93/93 (totals 7/7 exact; 0 unfollowed, 0 late, 0 pending), demand 2238/2238, protection 288/288, parks 72/72, budget 72/72, power 392/392, plumbing 144/144, development 913/913, flora 72/72, pollution 71/71, civic 71/71, landfill 71/71, neighbours 71/71, residential 1137/1137, crime 71/71, commercial 26/26, flammability 54/54, **traffic 61/83** (known difference, hypotheses 7), **land value 71/72** (2027-07-06 pass 19, `settled=0`: the original's record was dirty, 8 of 4096 intrinsic cells, max \
summary: regress voidbool, PASS, lists as run 138
