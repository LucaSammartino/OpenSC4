tag: run-93
run: 93
date: 2026-10-14
lane: run
dll: 83897a4b
pacing: unknown
verdict: clean
switches: main `393a529` (the `growprobe` merge; DLL `83897a4b`), `build-runs/growprobe-run.ps1`: **all original** (`disable = simulator, budget`), `growth_probe = 1`, Konradshohe, 900 s at High priority, CPU 96–100 % busy with nine parallel lanes
result: **clean, `ref_warnings=0`, hook installed and removed cleanly** (`0x00AB312C`: `0x0070D130` → forwarder → back). 2026-01 → 2065-03, **14,339 day lines, 48,106 Grow calls**, 159 µs per call (mean 574k cycles). Attempts per day 0/3/16 (min/median/max), qualifying 2/4/10; Σattempts / Σqualifying = **0.72**, steady across the run (0.65–0.78 per 2,000-day block, 0.48–0.88 per 28 days). Day lines alias (the posted day message lags, so 0 and 8 alternate around 4), which makes the probe's per-line `cut` flag meaningless (it reads 10,812 days). **The 3 ms timer could not have cut the loop**: a day's Grow calls total 0.5–1 ms, only 15 of 14,339 lines exceed 3 ms (two single calls did: 6.1 ms), and the 28 % gap is the picks without a call (qualifying developers whose growth list is empty or invalid), which the probe does not count. **Result: no cut days on the reference machine, so `growthAttemptsPerDay` stays unlimited (0)** (building-development.md §12). Caveat: one city of 626–742 lots; attempt cost grows with candidate lots. Day data: `build-runs/run93-days.txt`.
