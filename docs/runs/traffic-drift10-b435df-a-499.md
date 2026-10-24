tag: traffic-drift10-b435df-a-499
run: 499
date: 2026-10-24
lane: glue-traffic-drift10
dll: engine tree exactly 435df94b5; SHA256 c23f3f30e9bcdeee6fe17d1d2feca288ccc369c7dd93861a09dceaeb5b7d0d65
pacing: turbo to 2032-01-27, reached 2032-01-28; seconds=1200, timeout=1500
verdict: clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0
switches: house profile from main build-runs/regress-train188-shadows.ini, excluding autoload path; disable=budget,ordinance; automata_shadow=1; automata_owners_shadow=1; traffic_shadow=1; dbpf_shadow and gzmsg_shadow default off
result: Diagnostic GZMsg-merge bisect, not a fix: traffic 72/72 exact, between-delivery resyncs=0. Other house shadow totals exact except propmgr 219/220: queries on 2027-01-01 mismatches=1, resultsDiffering=0. Landvalue 73/73, dated automata and automataowners 72/72 each. Reached 2032-01-28 in 927.797 real seconds; wall 948 seconds, slot 3. Native gate build 48/48 and CTest 80/80. Logs/verdict build-runs/game-results/traffic-drift10-b435df-a.*. One clean traffic run does not exclude intermittent regression at this revision.
