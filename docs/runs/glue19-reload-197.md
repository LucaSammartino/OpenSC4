tag: glue19-reload-197
run: 191
date: 2026-10-20
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `a28c7282` (the fix-round tip), `build-runs/glue19-save-reload.ps1 -Tag g19sr3` run (b) (a direct `tools/ingame_run.ps1` run loading the city run 196 saved), Konradshohe (the saved 2026-02-13 state), `automata_shadow = 1`, turbo, 240 s
result: **clean, ref_warnings=0; the saved city reloads and the shadow is exact on it.** `Read: version 4, 79 bytes, result 0, julian 2461085` (the saved 2026-02-13) and `budget Read -> 1 (funds 935, departments 18, bonds 0)`; the automata shadow's load line exact (`cm 5194 / vehicle 1304 / pedestrian 4088 / aircraft 146 / watercraft 126 bytes identical`) and 117 lines, 117 with mismatches=0 (702 components, 1,257,036 bytes, 0 mismatches) to the shutdown at 2035-10-04. Save -> reload round-trips under the shadow (the fix round's step 4). The region was restored to `864E0A88` afterwards (the pair's restore had taken the new save as its backup; fixed by hand from `city-backups`).
summary: glue19 reload, clean, ref_warnings=0; the saved city round-trips under the exact shadow
