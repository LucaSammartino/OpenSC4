tag: train8-main-182
run: 182
date: 2026-10-20
lane: coordinator (merge-train batch train8 landed on main)
pacing: turbo
verdict: PASS
switches: main `89eb8ac4` (train8: re-answers32, glue27, cores23, cores20, glue22, glue18; ABI 74), `tools/regress.ps1 -Tag train8 -Queue` (turbo to 2032-01-01; both runs)
result: **PASS, 0 problems, both runs clean, ref_warnings=0.** Replace and shadows both clean to 2032. Every shadow exact except one line worse vs train6 (aura 70/71 -> 71/71 elsewhere; the regress output's delta column). Terrain 7/7 query totals exact, unfollowed 0.
summary: regress train8, PASS, both runs clean to 2032 (the batch's own regress, recorded after the land)
