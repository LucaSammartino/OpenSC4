tag: viewutil12-r1-turbo-20261024a-636
run: 636
date: 2026-10-24
lane: glue-viewutil-apply12
dll: 9efd3ba0 (engine source b6672b1de)
pacing: turbo
verdict: clean
switches: viewutil_shadow=1; turbo=1; end_date=2032-01-01; Konradshohe
result: viewutil12-r1-turbo-20261024a 0/73 aggregate exact (36/50 bodies exercised); 33 bodies each 73/73 exact, three each 0/73 (object-angle VU19; camera-angles and city-width VU12). Clean at 2032-01-02; ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; 30.562 real seconds. Shadow lines: rotation, bounds, cross, min, max, ground-arithmetic, triangle-pick, ground-predicate, edge-vector, round-down, round-up, primitive-count, primitive-indices, vertex-copy, index-decode, clip2, units, zoom-rotation, view-viewport, fixed, view-box, city-corner, clip3, visible-edges, world-view, world-viewport, depth, world-viewport2, view-world, viewport-world, box-rectangle, view-point, viewport-fixed all 73/73 exact; object-angle, camera-angles, city-width all 0/73 exact. resyncs=0 throughout. Generic numeric decode outside its checked bias range remains VU13, not whole-body credit.
