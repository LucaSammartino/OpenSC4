tag: traffic-drift10-b28472-a-498
run: 498
date: 2026-10-24
lane: glue-traffic-drift10
dll: engine tree exactly 28472c808; SHA256 65670de3d51694f5c69d59fae5037c2fbc9c17fdef1c78942341f3a3366847b4
pacing: turbo to 2032-01-01, seconds=1200, timeout=1500
verdict: clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0
switches: house profile from main build-runs/regress-train188-shadows.ini, excluding autoload path; disable=budget,ordinance; automata_shadow=1; automata_owners_shadow=1; traffic_shadow=1; dbpf_shadow and gzmsg_shadow default off
result: Diagnostic construction-merge bisect, not a fix: traffic 71/71 exact, between-delivery resyncs=0. All dated mismatch-bearing shadow lines checked by build-runs/summarize-house.py exact; landvalue 72/72; automata and automataowners each shutdown 73/73 including load. Terrain shutdown queries 7/7 exact, pending=0. Reached 2032-01-01 in 805.234 real seconds; wall 834 seconds, slot 1. Native gate build 255/255 and CTest 80/80. Logs and verdict in build-runs/game-results/traffic-drift10-b28472-a.*. One clean run does not exclude intermittent regression at this revision.
