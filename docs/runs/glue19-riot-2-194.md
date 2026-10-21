tag: glue19-riot-2-194
run: 188
date: 2026-10-20
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `1dc7cfbd` (the driver's shutdown counts line on top of run 193), `build-runs/game-queue/glue19-riot-2.req` (Konradshohe, `automata_shadow = 1`, the same riot script, turbo, end 2026-04-01)
result: **clean, ref_warnings=0; the automata shadow exact.** 1 shadow line, 1 with mismatches=0. `disaster riot auto -> AttemptDisasterStart(40, 33) -> instance 00000000; active=1 ongoing=1` and the game's disaster emergency pause again held the date. `automata driver: off: leaders created 0, refused 0, SetState calls 0, stage-rise calls 0, fire controller calls 0` — the expected empty count: the disaster layer is the original's on this branch (there is no `enable = disasters`), so its `create_riot_leader` never goes through the FC4DisasterHost table the driver serves; the live path fires when a disaster replacement exists.
summary: glue19 riot 2, clean, ref_warnings=0; the shadow exact, the driver idle as expected with the original layer
