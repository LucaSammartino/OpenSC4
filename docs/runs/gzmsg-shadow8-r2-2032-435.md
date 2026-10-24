tag: gzmsg-shadow8-r2-2032-435
run: 435
date: 2026-10-24
lane: glue-gzmsg-shadow8
dll: FreeCity4-glue-gzmsg-shadow8 source 9545111e0, DLL sha256 prefix 647e4899
pacing: turbo
verdict: clean
switches: queue gzmsg-shadow8-r2-2032; Konradshohe to 2032-01-01; gzmsg_shadow=1; refaudit=1
result: clean through 2032-01-14, 35.125 simulated-run real seconds, 56 s wall; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0; gzmsg 0/73, all samples unavailable (captured=0, probes=0, baseline=0, mismatches=1), resyncs=0; acquisition diagnostic framework=1 lookup=0 pointer=0 service=64FA8434 iid=652294BC. Zero other WARNING lines after config-key registration fix. GZ12 blocks native acquisition, GZ11 blocks complete state observation; no verification credit. Final source gate Rust 6940/0, clippy clean, CTest 78/78. Evidence build-runs/game-results/gzmsg-shadow8-r2-2032.{verdict,log}.
