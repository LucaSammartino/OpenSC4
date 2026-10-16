tag: run-68
run: 68
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: same DLL, `pollution_shadow = 1` (the original pollution simulator, our core replayed on its record each day and month), 150 s
result: **clean; all eight grids identical to the original, cell by cell, for 94 months** (2026-02 → 2033-11, 0 mismatches in 1.4 M cells a month), with the end-of-cycle replay in the original's tract order. Every saved field matches except `dirtyWater`: on 104 days the original's is true after the day and ours false, with identical grids (PG14, sent to the RE lane). Not compared: radiation sum, posts, history, budget values, effects.
