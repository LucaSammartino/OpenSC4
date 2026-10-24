tag: lotconf-sim-shadow6-native-2032-20261024b-387
run: 387
date: 2026-10-24
lane: glue-lotconf-sim-shadow6
dll: FreeCity4-glue-lotconf-sim-shadow6 sha256-prefix b5bb8553
pacing: normal
verdict: clean
switches: disable = simulator; lotconf_shadow = 1; simulator_shadow = 1; turbo = 0; end_date = 2032-01-01; label lotconf-sim-shadow6-native-2032-20261024b
result: CLEAN through 2032-01-01 in 126.328s, ref_warnings=0, watchdog_stalls=0, foreign_exceptions=0, rtc=0, overruns=0, panics=0, structured_exceptions=0, incomplete=0, other_warnings=0. lotconf 73/73; simulator 73/73; all dated lines mismatches=0, both final lines dated 2032-01-01. Every lotconf line: exact=1909/1909, pairs=97752, objects=25210, detail=1, refused=0, raw_foundation_tags=1908 (native unspecified tags, PL23; defined fields exact, no raw-array identity claim). Occupancy and cost unmeasured pending GLUE22. Every simulator line: pairs=89, bytes=79, tick_counters=11, record_bootstrap=1, independent_evolution=0, inTick ignored on Read per spec. Evidence build-runs/game-results/lotconf-sim-shadow6-native-2032-20261024b.{verdict,log}.
summary: lotconf 73/73 and simulator 73/73 exact on measured fields through 2032, clean native owners
