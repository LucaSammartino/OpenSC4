tag: gwp6-both-2032-normal-capture-a-377
run: 377
date: 2026-10-24
lane: glue-weather-propdev6
dll: FreeCity4-glue-weather-propdev6 (d9230437; engine source 599207dca)
pacing: normal
verdict: clean
switches: weather_shadow=1, propdev_shadow=1, sim_turbo=0; turbo=0; Konradshohe to 2032-01-01
result: clean through 2032-01-01, 119.000 s, ref_warnings=0, rtc=0, no foreign exceptions or stalls. gwp6-both-2032-normal-capture-a 1/73 (weather); gwp6-both-2032-normal-capture-a 18/72 (propdev monthly projections only, not verified). Weather first difference unchanged at step 3 byte 65833 118/117; final cumulative mismatches=1613, steps=2558 (WE1). Genuine selector collector recorded 24138 calls including 297 zero returns, preserving original execution exactly once; native dispatch restoration/reuse also covered by CTest. Propdev still says replay=pending: selector results are captured after the original but not yet consumed by a replay against frozen pre-operation host inputs. Creation outcomes/types and removal identities remain unverified; matching saved projections do not establish exact operations under PV14 section 5.3. These are known implementation gaps, not spec questions. Both keys remain default-off; no coverage/profile credit requested.
