tag: adv-replace15-r2-only-2032-654
run: 654
date: 2026-10-24
lane: glue-advisors-replace15
dll: FreeCity4-glue-advisors-replace15 fb42fb2e8, sha256 prefix 8fd49163
pacing: turbo
verdict: clean
switches: Konradshohe; enable=advisors; simulator/budget default-on; turbo=1; end_date=2032-01-01; seconds=600; timeout=900; popup_dialogs=0; autotest_end=quit
result: Clean through 2032-01-05, 23.781s simulation, 38s wall, 91.24 days/s. foreign_exceptions=0, ref_warnings=0, Rust panics=0, watchdog_stalls=0, rtc=0, overruns=0, structured exceptions=0, incomplete shadows=0, other warnings=0. advisors Init: ok=1, advisors=9, Lua acquired. Corrected advisor object payload and callback reference tracking. Exact evidence: build-runs/game-results/adv-replace15-r2-only-2032.verdict and build-runs/game-results/adv-replace15-r2-only-2032.log. Source 8beddc753 subsequently restores only the return-channel review comment, without changing runtime behavior.
