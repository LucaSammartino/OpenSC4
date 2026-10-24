tag: gwp6-r3-creation-normal-2032-a-384
run: 384
date: 2026-10-24
lane: glue-weather-propdev6
dll: 113e5426
pacing: normal
verdict: clean
switches: weather_shadow=1, propdev_shadow=1, sim_turbo=0; Konradshohe to 2032-01-01
result: clean through 2032-01-01, ref_warnings=0, rtc=0, 256.688s. Weather 73/73 exact, zero resyncs, 2562 steps at 2032-01-01. Propdev 24237/24237 genuine selector events matched (303 zero returns), all 45465 streams complete, zero invalid streams/resyncs or creation-invalid diagnostic lines. 23682 eligible output matches and 21783 ineligible/nonmatching operations; full-operations-exact=0. Bounded remaining differences are eligibility notes and equal-count ordered night-list differences, first listed at 2029-07-01; missing native shuffle CRT input tracked by PV16, dated/nested effect contract by PV17. Counts do not prove equal membership or complete output exactness. Final source 48679b680; gate 6906/0, clippy clean, CTest 75/75. Both shadows default-off. Evidence remains in build-runs/game-results; FileSystemWatcher wait (no qwait), session 42355 collected to terminal.
summary: weather exact; propdev selector streams exact, output boundary remains open
