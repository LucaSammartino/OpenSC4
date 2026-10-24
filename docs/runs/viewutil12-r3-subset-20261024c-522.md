tag: viewutil12-r3-subset-20261024c-522
run: 522
date: 2026-10-24
lane: glue-viewutil-apply12
dll: 4a19a8f0 (engine source f1b647211)
pacing: turbo
verdict: clean
switches: viewutil_shadow=1; turbo=1; end_date=2032-01-01; Konradshohe
result: viewutil12-r3-subset-20261024c 73/73 (viewutil-subset, 33/50 bodies); all 73 subset shadow lines have mismatches=0. Clean at 2032-01-13; ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; 27.906 real seconds. Each family 73/73 exact: rotation, bounds, cross, min, max, ground-arithmetic, triangle-pick, ground-predicate, edge-vector, round-down, round-up, primitive-count, primitive-indices, vertex-copy, index-decode, clip2, units, zoom-rotation, view-viewport, fixed, view-box, city-corner, clip3, visible-edges, world-view, world-viewport, depth, world-viewport2, view-world, viewport-world, box-rectangle, view-point, viewport-fixed. Object-angle (VU19), camera-angles and city-width (VU12) each 0/73; full viewutil aggregate remains 0/73 (36/50 exercised). resyncs=0 throughout. Fourteen bodies remain unmeasured; generic numeric decode outside the checked bias range remains VU13. Observer stays default-off; no whole-owner verification claimed.
summary: measured partial view utilities: 33-family subset 73/73 exact through 2032; complete aggregate 0/73
