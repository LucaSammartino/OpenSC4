tag: gwp6-weather-2032-normal-a-416
run: 416
date: 2026-10-24
lane: glue-weather-propdev6
dll: FreeCity4-glue-weather-propdev6 (308c2d83; weather source 4d5f5b227)
pacing: normal
verdict: clean
switches: weather_shadow=1, propdev_shadow=0; turbo=0; Konradshohe to 2032-01-01
result: clean through 2032-01-01, 146.078 s, ref_warnings=0, rtc=0, no foreign exceptions or stalls. gwp6-weather-2032-normal-a 1/73 (weather). Same first difference step=3 byte=65833 original/ours=118/117 as turbo. Last monthly line 2032-01-01 mismatches=1611, steps=2554. No model resynchronization. WE1 occurs at normal speed too; no verified coverage claimed.
