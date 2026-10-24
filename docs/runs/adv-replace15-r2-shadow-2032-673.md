tag: adv-replace15-r2-shadow-2032-673
run: 673
date: 2026-10-24
lane: glue-advisors-replace15
dll: FreeCity4-glue-advisors-replace15 fb42fb2e8, sha256 prefix 8fd49163
pacing: turbo
verdict: clean
switches: Konradshohe; disable=advisors,mysims; advisors_shadow=1; simulator/budget default-on; turbo=1; end_date=2032-01-01; seconds=600; timeout=900; popup_dialogs=0; autotest_end=quit
result: adv-replace15-r2-shadow-2032 73/73 exact. Original advisors run separately from the replacement, record-only AD19 comparison; every sample mismatches=0, resynced lines=0. Final month 2032-01-01: records=861 bytes=57902 MySim_identities=6 fixups=18; mismatches=0. Shutdown: advisors shadow: city shutdown: 73 lines, 73 with mismatches=0. CLEAN through 2032-01-15, 29.406s simulation, 43s wall, 74.13 days/s; foreign_exceptions=0, ref_warnings=0, Rust panics=0, watchdog_stalls=0, rtc=0, overruns=0, structured exceptions=0, incomplete shadows=0, other warnings=0. Exact evidence: build-runs/game-results/adv-replace15-r2-shadow-2032.verdict and build-runs/game-results/adv-replace15-r2-shadow-2032.log. No coverage or regress tools files edited.
