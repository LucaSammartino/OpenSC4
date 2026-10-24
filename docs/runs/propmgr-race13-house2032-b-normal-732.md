tag: propmgr-race13-house2032-b-normal-732
run: 732
date: 2026-10-24
lane: glue-propmgr-race13
dll: FreeCity4-glue-propmgr-race13 513fde7b (source 88730ab68)
pacing: normal
verdict: clean
switches: Konradshohe; propmgr_shadow=1; propmgr_shadow_cells=32; disable=budget,ordinance; turbo=0; end_date=2032-01-01
result: propmgr-race13-house2032-b-normal 219/220. Clean to 2032-01-01 in 111.781 s, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Zero input-error lines and zero propmgr resync lines. Sole difference is PL24: 2026-03-01 lot(2,58), captured rect(2,58,3,58), native slot-8 Z=944 at the upper bound, native list 1 / core 0. Reproduces bounds-contract contradiction at normal speed with identical native/core position inputs. Evidence build-runs/game-results/propmgr-race13-house2032-b-normal.log and .verdict.
