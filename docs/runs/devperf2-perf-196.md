tag: devperf2-perf-196
run: 196
date: 2026-10-20
lane: devperf2
dll: FreeCity4-devperf2
pacing: turbo
verdict: clean
switches: devperf2, queue labels devperf2-base-ours (baseline, before the changes) and devperf2-fix1/fix1b/fix2/fix3/fix4 (after) - `enable = development, tract` with `perf_profile = 1` (the perf month sections), turbo = 1, end_date = 2032-01-01
result: **clean, ref_warnings=0, 0 problems, all runs.** The metric glue27 used (the perf month sections' totals over 72 turbo months to 2032-01-01, same city, same pacing): **growth:tract-UpdatePopulation 3115 ms / 884736 calls = 3.52 us/call before, 656-995 ms / 884736 = 0.74-1.05 us/call after** (run-to-run spread on an otherwise loaded machine; the same DLL measured three times gave 0.95 / 1.13 / 1.05 total, and a rebuild of the first fix gave 0.95, so the residual delta between the fix builds is noise). The original's UpdatePopulation is 0.36 us/call (glue27), so the gap closed from 7.7-9.8x to about 2.1-2.9x; what is left is the ABI hop chain and the per-lot reads through it, which the original does in-process (a host slot batching the per-lot reads would need a C ABI change and was out of scope). **bds:tract-Grow 3188 ms / 9132 = 349 us/call before, 2702-4047 ms / 9205-10116 = 267-413 us/call after** (the counts differ because a faster unit makes more Grow calls in the same budget; per-pass Vec clones of the lot and configuration sets are gone and the listing memo now covers ListExistingLots). days/s at equal pacing: baseline 51.7, fixes 46.2-65.5 (the spread tracks machine load, two of the runs shared the box with other work). Exactness in the same runs: development shadow 854 growth-update lines 0 mismatches, suitability 72/72, tract styles/record 73-74 lines roundtrip=same, tract population sample drift only.
summary: devperf2 UpdatePopulation 3.52 -> 0.74-1.05 us/call, Grow 349 -> 267-413 us/call, shadows exact
