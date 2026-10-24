tag: gzmsg-shadow8-r1-2032-454
run: 454
date: 2026-10-24
lane: glue-gzmsg-shadow8
dll: FreeCity4-glue-gzmsg-shadow8 source 488be884c, DLL sha256 prefix 43f4ea1c
pacing: turbo
verdict: clean
switches: queue gzmsg-shadow8-r1-2032; Konradshohe to 2032-01-01; gzmsg_shadow=1; refaudit=1
result: clean through 2032-01-08, 35.610 simulated-run real seconds, 46 s wall; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0; gzmsg 0/73, every sample unavailable (captured=0, probes=0, baseline=0, mismatches=1), resyncs=0. GZ12 blocks native manager acquisition; no exact-state or whole-owner verification credit. Three unknown-key warnings exposed missing config registration, corrected after this run. Evidence build-runs/game-results/gzmsg-shadow8-r1-2032.{verdict,log}.
