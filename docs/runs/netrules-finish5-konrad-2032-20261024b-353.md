tag: netrules-finish5-konrad-2032-20261024b-353
run: 353
date: 2026-10-24
lane: glue-netrules-finish5
dll: engine/build-x86/FreeCity4.dll; SHA256 3c724b5ed15cac3d5a6d3bc0a8238b1a54efbb3e3f3bcaaaa526dfc4c61365f5
pacing: turbo
verdict: clean
switches: Konradshohe; run270 profile minus netmgr/netlot/nettool, 39 actual providers; netrules_shadow=1; script=build-runs/netrules-finish-actions.txt; seconds=600; timeout=750; end_date=2032-02-01; popup_dialogs=0; close_dialogs=1; autotest_end=quit. Full requested provider list retained in build-runs/game-queue/netrules-finish5-konrad-2032-20261024b.req.
result: Headline clean to 2032-02-01 in 163.015 seconds, ref_warnings=0, dialogs_closed=0, rtc=0, overruns=0. NOT ACCEPTED as clean evidence: full log has 14 caught structured exceptions during script PlaceNetworkOccupants plus one later foreign first-chance exception at shutdown, after headline verdict counters were printed. PDB resolution of our DLL RVA DC1FF1/DC1551 gives anonymous RegionToC / cFC4Demolition::DemolishRegion; not the shadow. netrules-finish5-konrad-2032-20261024b 75/75 mismatch-clean, plugin shutdown 75 lines/74 exact, evaluations=54, tables=24, incomplete=0, unavailable=0. One load sample precedes readiness; intersections unobserved. January 2032 script marker completed. All evidence retained in build-runs/game-results/; diagnostic helper build-runs/resolve-plugin.py reads only our DLL/PDB. Retry leaves demolition native, changing game inputs only.
summary: Reached 2032 with mismatch-clean shadow, but caught demolition faults and later shutdown exception invalidate clean-run acceptance; key stays out of shadows profile.
