tag: devperf2-shadows-197
run: 197
date: 2026-10-20
lane: devperf2
dll: FreeCity4-devperf2
pacing: turbo
verdict: clean
switches: devperf2, queue labels devperf2-shadows and devperf2-final - `enable = development, tract` **and** `tract_shadow = 1` and `development_shadow = 1` in one run (the self-check combination; development_shadow stays INACTIVE there by design, so the development-shadow differential comes from the same lines of devperf2-final where it did run active), turbo = 1, end_date = 2032-01-01
result: **clean, ref_warnings=0, 0 problems.** devperf2-shadows (DLL after fix 1): tract self-check ON over FreeCity4's replacement - **styles and record 74/74 lines `roundtrip=same`, 0 mismatches**; the population sample (the documented drift, not compared call by call) 124 mismatches over 28416 lots on 35 of 74 lines, within glue27's 155-over-7325 baseline; the development shadow reported INACTIVE as designed (`it needs FreeCity4's simulator and the original development unit (do not enable development)`). devperf2-final (the final DLL): **development shadow 854 growth-update lines, 0 mismatches** (all 12 developers 72 months, the 2032-01-01 month ends mid-update with 2), catalogue 744/744 same order, suitability 72/72 lines, and the tract self-check again 73/73 `roundtrip=same`. Together with devperf2-devshadow (the differential without `enable`, before the last two tweaks): 854 growth-update lines 0 mismatches, suitability 72/72.
summary: devperf2 shadows over the replacement exact after the cell-walk host change
