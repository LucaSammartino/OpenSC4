tag: renderprops-shadow9-20261024-r1-487
run: 487
date: 2026-10-24
lane: glue-renderprops-shadow9
dll: FreeCity4-glue-renderprops-shadow9, source aa6a2508b, DLL SHA256 prefix 0775c3ab
pacing: turbo
verdict: clean
switches: regress-train187-shadows.ini house switches plus renderprops_shadow=1; Konradshohe; turbo=1; end_date=2032-01-01; seconds=600; timeout=900
result: renderprops-shadow9-20261024-r1 43/43. 542 independent registry fields per sample, 23306 total field comparisons, no duplicate months; samples 2026-01-27 through 2029-07-01. Clean shutdown at 2029-07-11 after 602.094 s, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. Incomplete date target: time budget expired before 2032. One unknown-key warning for renderprops_shadow despite active shadow; fixed in subsequent source. Values/setters/lifetime/separate view remain uncompared under RP7; no whole-owner credit. Evidence: build-runs/game-results/renderprops-shadow9-20261024-r1.{verdict,log}.
