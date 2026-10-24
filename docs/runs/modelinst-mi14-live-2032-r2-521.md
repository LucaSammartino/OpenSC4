tag: modelinst-mi14-live-2032-r2-521
run: 521
date: 2026-10-24
lane: glue-modelinst-shadow12
dll: FreeCity4-glue-modelinst-shadow12 engine source 939c4bfa5; SHA256 0cc2798c7d79336bf890d7d88ca5eb6a9636bd71ea5a7539bb6907f3aef71d2a
pacing: turbo
verdict: clean
switches: modelinst_shadow=1; city=Konradshohe; end_date=2032-01-01; seconds=600; timeout=1200; default replacements
result: modelinst-mi14-live-2032-r2 145/145 exact shadow lines: modelinst 73/73 isolated-static-transform-bounds, modelinst-live 72/72 static-gettransform-destination. Every dated line has mismatches=0, resyncs=0. Reached 2032-01-01 in 30.125 real seconds, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Static fixtures: 144 fields/line, 10,512 comparisons. Live: 93,481 natural calls encountered, first 4,096 captured, 89,385 dropped at the city cap; 12,288 unique counter/padding bytes compared. Copies 56-byte before/after records synchronously, forwards original thunk exactly once, adds no native getter/mutator/refcount calls, retains no instance pointer. Source words/kind and other owner operations unmeasured; no whole-owner claim. Evidence: build-runs/game-results/modelinst-mi14-live-2032-r2.verdict and .log; bounded reproducer: build-runs/modelinst-summary.py.
