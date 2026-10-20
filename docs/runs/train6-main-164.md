tag: train6-main-164
run: 164
date: 2026-10-20
lane: coordinator (merge-train batch train6 landed on main)
pacing: turbo
verdict: PASS
switches: main `34af2ae1` (train6 landed: glue17, apply15b, cores17, cores19, glue15, apply15terrain, glue12b1, glue12b2; effects core takes cores19's with the fc4_effects_* ABI at 73; observer PreCityShutdown unglued), `tools/regress.ps1 -Tag train6-main -Queue` (turbo to 2032-01-01; both runs)
result: **PASS, 0 problems, both runs clean, ref_warnings=0.** Replace 27.43 days/s (77 s), shadows 4.55 days/s (476 s, budget disabled). Every shadow exact except: development 912/913 (one tract population line, max diff 4, 2027-10-02), landvalue 71/72 (the known pass-2 early-settling cell, 2029-04-06), traffic 63/81 (record phase positions, the uncalibrated-shadow draws of train5's note). Terrain 7/7 query totals exact, unfollowed 0.
summary: regress train6-main, PASS, both runs clean to 2032; the train6 batch's post-merge gate on main (cargo 3400/0, clippy clean, ctest 62/62)
