tag: modelinst-mi14-loadfix-2032-r1-655
run: 655
date: 2026-10-24
lane: glue-modelinst-shadow12
dll: FreeCity4-glue-modelinst-shadow12 e8703c22d (DLL sha256 prefix fa30f030)
pacing: turbo
verdict: clean
switches: modelinst_shadow=1; city=Konradshohe; end_date=2032-01-01; seconds=600; timeout=1200; default replacements
result: modelinst-mi14-loadfix-2032-r1 73/73 static shadow lines exact, 144 fields/line, mismatches=0 throughout. Reached 2032-01-02 in 32.015 real seconds; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. City loaded on the first corrected-DLL request (slot 2). MI14 constructor flag 1 and standalone +0xA0 instance receiver applied. Isolated static fixture only; no live or whole-owner credit. Evidence: build-runs/game-results/modelinst-mi14-loadfix-2032-r1.verdict and .log.
