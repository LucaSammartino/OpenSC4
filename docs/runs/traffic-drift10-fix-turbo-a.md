tag: traffic-drift10-fix-turbo-a
run: 492
date: 2026-10-24
lane: glue-traffic-drift10
dll: candidate 3d2a34205; SHA256 3bd60f90ba90c42cbfce9eb3715c723e2b6672bca78e0606f2e3abb6174a7754
pacing: turbo to 2032-01-27; seconds=1800, timeout=2100
verdict: clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0
switches: train188 house profile; traffic_shadow=1, traffic_shadow_hypotheses=7; propmgr_shadow=1, propmgr_shadow_cells=32; dbpf_shadow default off
result: traffic 72/72 exact, between-delivery resyncs=0. Propmgr 219/220: queries 2027-01-01 mismatches=1, resultsDiffering=0; every other house verdict total exact, including landvalue 73/73. Reached 2032-01-27 in 951.438 real seconds, wall 966 seconds, slot 2. Existing unique request collected without resubmission. Logs/verdict build-runs/game-results/traffic-drift10-fix-turbo-a.*. Together with normal run 480 establishes two exact traffic runs; subsequent DBPF-enabled run 493 has every house total exact. Renumbered from provisional 491 because main recorded train193 as 491 during the gate.
