tag: gwp6-r3-we1-propdiag-2032-turbo-a-383
run: 383
date: 2026-10-24
lane: glue-weather-propdev6
dll: 12479770
pacing: turbo
verdict: clean
switches: weather_shadow=1, propdev_shadow=1, sim_turbo=1; Konradshohe to 2032-01-01
result: clean through 2032-01-03, ref_warnings=0, rtc=0, 173s. Weather 73/73 exact, zero resyncs, 2566 steps at 2032-01-01. Propdev 23958/23968 selector events matched, 45055 complete/410 invalid streams, 41297 eligible output matches/4168 incomplete of 45465 operations. First 24 invalid streams were extra engine calls after every native event matched. This is source 1b9ed3f39, before genuine creation-return capture; it led to diagnostic fix 48679b680. First gate 6905/0, clippy clean, CTest 75/75. Both shadows default-off; no full-operation exactness claim. Full evidence in build-runs/game-results under the unique label; waited without qwait and collected session 68186 to terminal.
summary: WE1 weather exact; creation continuation capture diagnosed
