tag: devperf2-upcomp-195
run: 195
date: 2026-10-20
lane: devperf2
dll: FreeCity4-devperf2
pacing: turbo
verdict: clean
switches: devperf2, queue labels devperf2-upcomp (before the last two tweaks) and devperf2-upcomp2 (the final DLL) - `tract_shadow = 1` with no `enable`: the differential where the original tract developer is the live class and the slot 7 UpdatePopulation forwarder compares every call against ours; turbo = 1, end_date = 2032-01-01
result: **clean, ref_warnings=0, rtc=0, overruns=0, both runs.** The UpdatePopulation comparison the forwarder makes against the original, after the cell-walk host change: devperf2-upcomp (DLL b2d3497a) **884736 calls, 883872 compared, 0 failures, 42461 lots, 0 mismatches**; devperf2-upcomp2 (the final DLL ed60a09c) **909312 calls, 908424 compared (the 888 others are tract (0, 0), skipped as the spec's clock-seed quirk says), 0 failures, 43638 lots, 0 mismatches** - the cell-map walk answers the original's GetLotsInCellRect(originOnly = true) exactly, lot by lot, in the same order. Tract styles and record both runs: 74 lines, `roundtrip=same` on all 74, 0 mismatches.
summary: devperf2 UpdatePopulation forwarder comparison, clean, 0 mismatches over 883872 + 908424 compared calls
