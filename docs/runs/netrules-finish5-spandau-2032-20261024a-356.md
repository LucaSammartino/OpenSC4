tag: netrules-finish5-spandau-2032-20261024a-356
run: 356
date: 2026-10-24
lane: glue-netrules-finish5
dll: engine/build-x86/FreeCity4.dll; SHA256 3c724b5ed15cac3d5a6d3bc0a8238b1a54efbb3e3f3bcaaaa526dfc4c61365f5
pacing: turbo
verdict: clean
switches: Spandau; enable=simulator,budget; netrules_shadow=1; script=build-runs/netrules-finish-actions.txt; seconds=600; timeout=750; end_date=2032-02-01; popup_dialogs=0; autotest_end=quit.
result: Clean/ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; timeout after 600.969 seconds at 2019-03-29, not 2032. Emergency pauses repeatedly held the clock; final pause=1 hidden=0 emergency=1. Last monthly funds=62094. All scripted actions dated 2026 or later remained pending. netrules-finish5-spandau-2032-20261024a 212/212 mismatch-clean; plugin shutdown 212 lines, 101 exact, evaluations=24, tables=24. No incomplete/unavailable/mismatch counts. Many startup/month lines preceded the first natural Evaluate; no full exactness or 2032 credit. Verdict and log retained under build-runs/game-results/. Watcher timeout and cleanup failures are helper failures, separate from this valid game verdict.
summary: Spandau control clean but short of 2032; emergency pauses and pre-observation samples prevent full shadow verification; key stays out of shadows profile.
