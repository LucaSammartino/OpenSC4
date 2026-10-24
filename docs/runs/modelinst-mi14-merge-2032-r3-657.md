tag: modelinst-mi14-merge-2032-r3-657
run: 657
date: 2026-10-24
lane: glue-modelinst-shadow12
dll: FreeCity4-glue-modelinst-shadow12 engine source a907efa2d; SHA256 8aa62a855f8d2d8f6994b976b46e2e4a509b5dafda1784f0896663b8c855607f
pacing: turbo
verdict: clean
switches: modelinst_shadow=1; city=Konradshohe; end_date=2032-01-01; seconds=600; timeout=1200; default replacements
result: modelinst-mi14-merge-2032-r3 145/145 exact shadow lines: modelinst 73/73 isolated-static-transform-bounds, modelinst-live 72/72 static-gettransform-destination. Every dated line has mismatches=0, resyncs=0. Reached 2032-01-10 in 27.953 real seconds, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Static fixtures: 144 fields/line, 10,512 comparisons. Live observer ON: 90,579 natural calls encountered, first 4,096 captured, 86,483 dropped at the city cap; 12,288 unique counter/padding bytes compared. First live line: 1,427 calls / 4,281 fields. Confirms additive merge of origin/main b5f3bb118 into prior READY tip; gate cargo 7408/0, clippy zero diagnostics, build 62/62, CTest 81/81. Source words/kind and other owner operations unmeasured; no whole-owner claim. Evidence: build-runs/game-results/modelinst-mi14-merge-2032-r3.verdict and .log; bounded reproducer: build-runs/modelinst-summary.py.
