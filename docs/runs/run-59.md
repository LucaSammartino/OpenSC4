tag: run-59
run: 59
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: `autotest_end = quit` (new), `clock_probe = 2`, our clock, plumbing + flammability, 150 s
result: **clean, `ref_warnings=0` through the whole Shutdown path.** At the deadline `cISC4App::RequestQuit(false, false) -> 1`; plumbing and flammability shut down and released their host grids (the `FC4_RELEASE_CREATED` path of `bbe9181`), the budget and simulator shut down, then PreAppShutdown → PostAppShutdown → PostSystemServiceShutdown (verdict) → process exit, all in 0.3 s. The verdict's julian read 2000-01-01 (our simulator publishes its reset date during Shutdown); fixed to the last day played.
