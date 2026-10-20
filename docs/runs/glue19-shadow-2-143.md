tag: glue19-shadow-2-143
run: 143
date: 2026-10-16
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `2206f09b` (glue19: the shadow's reference token is the capture's object number), `build-runs/game-queue/glue19-shadow-2.req` (Konradshohe, `automata_shadow = 1`, turbo to 2032-01-01), DLL `FreeCity4-glue19`
result: **clean, ref_warnings=0; the automata shadow is exact.** 73 shadow lines, 73 with mismatches=0; 438 components compared, 778,963 bytes compared, 0 mismatches. Every month line reads all six components byte identical (at 2032-01: script 2, cm 4942, vehicle 1296, pedestrian 4088, aircraft 146, watercraft 126 bytes). The load line is exact too (script 2, cm 5026, vehicle 1312, pedestrian 4200, aircraft 146, watercraft 126). The 2 remaining log warnings are the pre-existing x87 precision notes at director AddRef, not this lane's. 42.52 days/s, 51.2 s real.
summary: glue19 shadow 2, clean, ref_warnings=0; the automata shadow exact 73/73
