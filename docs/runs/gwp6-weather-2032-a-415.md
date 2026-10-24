tag: gwp6-weather-2032-a-415
run: 415
date: 2026-10-24
lane: glue-weather-propdev6
dll: FreeCity4-glue-weather-propdev6 (308c2d83; weather source 4d5f5b227)
pacing: turbo
verdict: clean
switches: weather_shadow=1, propdev_shadow=0; turbo=1; Konradshohe to 2032-01-01
result: clean through 2032-01-05, 138.797 s, ref_warnings=0, rtc=0, no foreign exceptions or stalls. gwp6-weather-2032-a 1/73 (weather). Bootstrap exact; first difference step=3 byte=65833 original/ours=118/117, both records=65889 bytes. Last monthly line 2032-01-01 mismatches=1622, steps=2565. No model resynchronization. Differences tracked by WE1; no verified coverage claimed.
