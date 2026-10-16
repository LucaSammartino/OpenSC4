tag: run-24
run: 24
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: same switches as 23, DLL with the panic-hook work (checkpointed 8129d67), lane 1
result: **clean** — `AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0)`, no panic, 4378 log lines, budget settled (funds 69986 → 70154, income 964, expense 796, tax 919). Still **no `power recompute` line**, because power was off by default in this run's config (`subsystem 'power': off by default`), so power production is still unverified. The x87 warning fired again: 53-bit from PostAppInit through tick 1, city load, month observation and both budget settlements.
