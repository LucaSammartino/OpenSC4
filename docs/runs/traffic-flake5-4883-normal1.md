tag: traffic-flake5-4883-normal1
run: 331
date: 2026-10-23
lane: glue-traffic-flake5
dll: 9fce084f (source 4883e8c4, glue-traffic-flake5)
pacing: normal
verdict: clean
switches: Konradshohe; enable = simulator; traffic_shadow = 1; turbo = 0; end_date = 2032-01-01
result: Reached 2032-01-01 in 427.984 s (443 s wall), ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0. SHADOW FAILED: traffic-flake5-4883-normal1 70/73; 1 resynced line, INCOMPLETE 0. At 2028-03-28 day state 18 -> 2 (43 units), mismatches=37: 45 of 479920 edge-density bytes, car length sum/count, 2 lot trip lists, and 33 volume-grid tracts differ. Next day records the drift; April month line is not exact. This preserves the train152 failure class after the sentinel fix; do not credit traffic as verified. Original-vs-copy differential. Evidence: build-runs/game-results/traffic-flake5-4883-normal1.verdict and .log in the lane worktree. DLL SHA256 9FCE084FAEFEC1AFAE36352DBC1B7B189363F2A76499675877FF00A2A1F01DED.
