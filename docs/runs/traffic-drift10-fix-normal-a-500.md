tag: traffic-drift10-fix-normal-a-500
run: 500
date: 2026-10-24
lane: glue-traffic-drift10
dll: candidate 3d2a34205; SHA256 3bd60f90ba90c42cbfce9eb3715c723e2b6672bca78e0606f2e3abb6174a7754
pacing: normal speed to 2032-01-27; seconds=1800, timeout=2100
verdict: clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0
switches: house profile from main build-runs/regress-train188-shadows.ini excluding autoload path; turbo=0; automata_shadow=1; automata_owners_shadow=1; traffic_shadow=1; dbpf_shadow and gzmsg_shadow default off
result: traffic 72/72 exact, between-delivery resyncs=0. Other verdict totals exact except landvalue 72/73: 2026-06-14 pass-end sample, slope (5,13) 4.2510276/4.264569, settled=0. Reached 2032-01-27 in 952.782 real seconds; wall 969 seconds, slot 4. Rust 6997/0, clippy 0; corrective native build 154/154 and CTest 80/80 after stale native ABI object failure (preserved separately). Logs/verdict build-runs/game-results/traffic-drift10-fix-normal-a.*. First candidate traffic run only, turbo validation pending.
