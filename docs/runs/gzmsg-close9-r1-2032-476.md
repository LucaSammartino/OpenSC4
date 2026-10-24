tag: gzmsg-close9-r1-2032-476
run: 476
date: 2026-10-24
lane: glue-gzmsg-close9
dll: FreeCity4-glue-gzmsg-close9 source 6c10a0bc0; sha256 4bd22766e5411e819e4c7db5604e7c587ddf1a8a4be85f424f780650b63ca638
pacing: turbo
verdict: clean
switches: queue gzmsg-close9-r1-2032; Konradshohe to 2032-01-01; enable empty; gzmsg_shadow=1; refaudit=1; focused diagnostic, not house-shadows profile
result: gzmsg-close9-r1-2032 0/73; gzmsg 0/73; all 73 dated samples unavailable, captured=0, probes=0, baseline=0, mismatches=1, resyncs=0. Clean shutdown at 2032-01-01; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0, other WARNING lines=0. 28.797 real seconds, 40 s wall, slot 3. framework=1 lookup=0 pointer=0 service=64FA8434 iid=652294BC. GZ12 specifies optional absence; no factory fallback and no native manager verification credit. No normal-speed repeat because acquisition fails before turbo. Gate: 6992 Rust tests passed, clippy clean, CTest 79/79. Evidence: build-runs/game-results/gzmsg-close9-r1-2032.{verdict,log}.
