tag: glue19-riot-1-193
run: 193
date: 2026-10-20
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `9837af8f` (the fire bridge and the U-Drive-It pause / resume on top of the wrappers), `build-runs/game-queue/glue19-riot-1.req` (Konradshohe, `automata_shadow = 1`, `script` = disaster survey then a `disaster riot auto`, turbo, end 2026-06-01 but the riot's own emergency pause held the date)
result: **clean, ref_warnings=0; the automata shadow exact.** 1 shadow line (the load), 1 with mismatches=0. `disaster riot auto -> AttemptDisasterStart(40, 33) -> instance 00000000; active=1 ongoing=1`: the original layer started a riot and the game then held the city in the disaster's own emergency pause (pause=1 hidden=1 emergency=1 for the whole run), so 600 s of wall time covered no further game date. The driver's counts were not logged yet (added for run 194).
summary: glue19 riot 1, clean, ref_warnings=0; the shadow exact through a scripted riot's load
