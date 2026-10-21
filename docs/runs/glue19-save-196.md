tag: glue19-save-196
run: 190
date: 2026-10-20
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `a28c7282` (the fix-round tip), `build-runs/glue19-save-reload.ps1 -Tag g19sr3` run (a) (a direct `tools/ingame_run.ps1` run, not the queue: the save has to survive to the reload run, which the queue's region mirror would undo; `-KeepSaves` keeps the saved file), Konradshohe, `automata_shadow = 1`, `script` = one `save` at 2026-01-30, turbo, 240 s
result: **clean, ref_warnings=0; SaveCity(full) at 2026-02-13.** `script: line 1 2026-01-30 save -> ok=1 (SaveCity(full) in 10969 ms) (run on 2026-02-13)`; the automata shadow exact across the save (358 lines, 358 with mismatches=0, 2,148 components, 3,837,340 bytes, 0 mismatches at the shutdown). The city file's hash changed 864E0A88 -> 9783ACA2 (kept by `-KeepSaves`). The first pair (g19sr2) ran the same save without `-KeepSaves` and the two runs without `autoload_city` did not load: those are the diagnostic runs behind this run's shape.
summary: glue19 save, clean, ref_warnings=0; SaveCity inside the exact shadow run, file kept for the reload
