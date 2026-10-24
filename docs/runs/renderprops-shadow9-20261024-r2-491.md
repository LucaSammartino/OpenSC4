tag: renderprops-shadow9-20261024-r2-491
run: 491
date: 2026-10-24
lane: glue-renderprops-shadow9
dll: FreeCity4-glue-renderprops-shadow9, final engine source e1937e01f, DLL SHA256 prefix f179d2f8
pacing: turbo
verdict: clean
switches: regress-train187-shadows.ini house switches plus renderprops_shadow=1; Konradshohe; turbo=1; end_date=2032-01-01; seconds=1800; timeout=2100
result: renderprops-shadow9-20261024-r2 73/73. 542 independent registry fields per sample, 39566 total field comparisons, no duplicate months; samples 2026-01-27 through 2032-01-01. Clean shutdown at 2032-01-01 after 939.625 s (957 s queue wall), foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0, Rust panics=0, structured exceptions=0, other WARNING lines=0. Config key recognized. Values/setters/lifetime/separate view remain uncompared under RP7; no whole-owner credit. Evidence: build-runs/game-results/renderprops-shadow9-20261024-r2.{verdict,log}.
summary: renderprops registry 73/73 exact to 2032, clean; RP7 leaves whole-owner verification open, run 491
