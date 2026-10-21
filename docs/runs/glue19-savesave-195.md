tag: glue19-savesave-195
run: 189
date: 2026-10-20
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: ended
switches: glue19 `243f82ff` (the resume tip before the fix round's commits), `build-runs/game-queue/glue19-savesave2.req` (Konradshohe, `automata_shadow = 1`, `script` = one `save` at 2030-01-01, turbo, 300 s)
result: **ended, 0 problems, ref_warnings=0; a full SaveCity at 2030-01-13 inside the shadow run.** `script: line 1 2030-01-01 save -> ok=1 (SaveCity(full) in 10766 ms) (run on 2030-01-13)`; the run continued to 2064 (the game ended the city itself 217 s in, before the 300 s deadline) and the automata shadow was exact the whole way: 464 lines, 464 with mismatches=0, 465 at the city shutdown (2,790 components, 4,929,327 bytes, 0 mismatches). The save itself was not reloaded in this run (the queue mirrors the region before every run, so the slot's saved file does not survive to the next); the reload is run 197.
summary: glue19 savesave, ended, ref_warnings=0; SaveCity inside the exact shadow run, 464 lines
