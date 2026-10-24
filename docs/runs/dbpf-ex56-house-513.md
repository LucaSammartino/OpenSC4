tag: dbpf-ex56-house-513
run: 513
date: 2026-10-24
lane: glue-dbpf-ex56apply11
dll: FreeCity4-glue-dbpf-ex56apply11 (ec17f769; engine source e73dbdebb)
pacing: turbo
verdict: clean
switches: main build-runs/regress-train193-shadows.ini house switches, dbpf_shadow=1, end_date=2032-01-01, seconds=1650, timeout=1800
result: dbpf 73/73; clean shutdown at 2032-01-03, real_seconds=916.234, wall=932 s, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Eleven packed files: 10 direct roots and 1 multi-packed child; registered city backing path explicitly discovered on every sample and compared 7 times (187 records each), no fallback used. Initial full sweep 111826 records; final cumulative 83 files/876020 records; all 11 files/111826 records covered in rotation cycle 6. Refusals=0, differences=0, max_slice_ms=828. Transient/unregistered files unobserved (EX57); no save in this run.
summary: EX56 DBPF house profile, dbpf 73/73 exact, clean to 2032 under 1800 s

Evidence: build-runs/game-results/dbpf-ex56-house-492.verdict and .log in the lane worktree. The registered backing path was C:/Users/tlori/FreeCity4/build-runs/sc4slots/slot2/Regions/FC4 Test/City - Konradshohe.sc4. The city was already among the eleven discovered files; this lane establishes explicit per-path comparison evidence rather than claiming a larger census.
