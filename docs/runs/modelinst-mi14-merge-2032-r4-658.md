tag: modelinst-mi14-merge-2032-r4-658
run: 658
date: 2026-10-24
lane: glue-modelinst-shadow12
dll: FreeCity4-glue-modelinst-shadow12 engine source 95e87ebc1; SHA256 0e7a5d9eb4af327798a12637112d5af9a7303134dffe93a97bea285702169e28
pacing: turbo
verdict: clean
switches: modelinst_shadow=1; city=Konradshohe; end_date=2032-01-01; seconds=600; timeout=1200; default replacements
result: modelinst-mi14-merge-2032-r4 145/145 exact shadow lines: modelinst 73/73 isolated-static-transform-bounds, modelinst-live 72/72 static-gettransform-destination. Every dated line has mismatches=0, resyncs=0. Reached 2032-01-19 in 29.156 real seconds, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Static fixtures: 144 fields/line, 10,512 comparisons. Live observer ON: 94,037 natural calls encountered, first 4,096 captured, 89,941 dropped at the city cap; 12,288 unique counter/padding bytes compared. First live line: 1,426 calls / 4,278 fields. Confirms additive merge of origin/main 625733fc6 preserving both viewutil and modelinst registrations; final gate cargo 7466/0, clippy zero diagnostics, build 110/110, CTest 81/81. Source words/kind and other owner operations unmeasured; no whole-owner claim. Evidence: build-runs/game-results/modelinst-mi14-merge-2032-r4.verdict and .log; bounded reproducer: build-runs/modelinst-summary.py. Run number checked free against current origin/main and worktree after concurrent low-number collisions.
