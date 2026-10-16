tag: zones2-112
run: 112
date: 2026-10-15
lane: zones2
dll: FreeCity4-zones2
pacing: turbo
verdict: PASS
switches: main `9a9b0df` (zones2 Stage A: zone developer / lot manager `cFC4ZoneDeveloper`, `enable = zonedev` and `zonedev_shadow = 1`, both off by default and in neither regress list; lot creation stubbed until Stage B; ABI 51; DLL `FreeCity4-zones2`, sha256 `4136cab2`), `tools/regress.ps1 -Tag zones2 -Queue` (turbo to 2032-01-01)
result: **PASS, 0 problems, 334 s.** Both runs clean, `ref_warnings=0`; replace 25.79 days/s (17 replacement groups, as run 111). Shadows 6.99 days/s (about half run 111's pace: lane games ran in the other slots at the same time): zones 292/292, tract 145/145, demand 2237/2237, power 359/359, plumbing 145/145, residential 1137/1137, protection 288/288, landfill 71/71, flammability 54/54, land value 71/72 (the usual 2026-02-07 pass 2 early-settling line), every other shadow exact except **development 912/913**: growth 0x3320 on 2026-06-01 has population at tracts (4,26) and (4,27) 0 in the original and 14/46 in ours, with the total equal (1193/1193) and the next month exact. The shadows run loads none of this merge's code (zonedev off; the lane's shared-file changes are the lot manager slot 8-10/17/19/27 return types, whose only other callers discard the result, and an index in WriteCapture with the same output), so this is read as a pacing effect (DG31), not this merge's. The lane's own runs: `zonedev-shadow-1`, `-final`, `-final3` and `-tip` clean to 2028-01, 130 zonedev shadow lines each, all exact (741 lots at load); `zonedev-enable-1` and `-2` corrupt (refused CreateLot in the original tract developer's Grow, 2 lot over-releases, then an AV; Stage A limit, ZG34).
