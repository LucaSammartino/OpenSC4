tag: lotconf-sim-shadow6-native-2032-20261024a-374
run: 374
date: 2026-10-24
lane: glue-lotconf-sim-shadow6
dll: FreeCity4-glue-lotconf-sim-shadow6 sha256-prefix 8701949c
pacing: normal
verdict: clean
switches: disable = simulator; lotconf_shadow = 1; simulator_shadow = 1; turbo = 0; end_date = 2032-01-01; label lotconf-sim-shadow6-native-2032-20261024a
result: clean at 2032-01-01, 127.671s, ref_warnings=0, rtc=0, foreign_exceptions=0. lotconf 73/73; simulator 73/73, every dated line mismatches=0. Each lotconf line compares 1909/1909 configurations, 97752 pairs, 25210 objects at detail 1; 1908 unspecified appended-foundation tags differ (PL23), separately recorded. Occupancy and cost unmeasured pending GLUE22. Each simulator line compares 89 pairs, 79-byte record, all 11 level counter/fired pairs plus date/speed/time/pause getters; record bootstrap, independent_evolution=0, inTick ignored on Read per spec. Six unknown-key warnings diagnosed and subsequently fixed by registering the two keys. One watchdog sample at 02:56:59.515 after clean verdict/end-date at 02:56:54; shutdown outside FreeCity4, no simulation stall or foreign exception. Evidence build-runs/game-results/lotconf-sim-shadow6-native-2032-20261024a.{verdict,log}.
summary: lot configuration and simulator native shadows exact on specified measured fields through 2032
