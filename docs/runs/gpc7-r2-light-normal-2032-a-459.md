tag: gpc7-r2-light-normal-2032-a-459
run: 459
date: 2026-10-24
lane: glue-propdev-close7
dll: 0a9789a1
pacing: normal
verdict: clean
switches: source 9600542f6; enable=simulator; propdev_shadow=1; sim_turbo=0; script=build-runs/propdev-lighting-r2-actions.txt; end_date=2032-01-01
result: clean to 2032-01-01 in 151.563s; ref_warnings=0, foreign_exceptions=0, stalls=0, rtc=0, overruns=0. Exact coverage propdev 0/72, resyncs=0. Eligible operations 42537/45465, output matches 42537/42537, incomplete=2928, mismatch total=2928 (PV18 only), full-operations-exact=0. PV16 exercised: night callbacks=6, exact=6, CRT draws=975. Selectors 24200/24200, complete streams 45465/45465, replay-invalid=0, requester-tag projections 562/562. Six genuine native lighting SetTimeOfDay actions, SDK slot 6, borrowed PV15 acquisition. Shadow remains default-off; successful creation and dated effects remain ineligible under PV18.
