tag: run-42
run: 42
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: demand-shadow build `AC7D9017…` (+ the coordinator rustfmt commit), `demand_shadow = 1`, our simulator, Spandau
result: **clean; demand matches bit-for-bit.** All 160 NewDay updates had `mismatches=0`: 173 nodes, 12 active values and the neutral tax rate identical bitwise. 6 of 6 SimIdle modifier checks were exact. The graph built in 141 ms (173 exemplars, 353 links). `ref_warnings=1`: the per-call resource key list we own was released, a false positive (see 43).
