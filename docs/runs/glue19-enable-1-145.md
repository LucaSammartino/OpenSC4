tag: glue19-enable-1-145
run: 145
date: 2026-10-16
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: corrupt
switches: glue19 (the GZCOM classes registered as `enable = automata`), `build-runs/game-queue/glue19-enable-1.req` (Konradshohe, `enable = automata` + `automata_shadow = 1`, turbo to 2032-01-01)
result: **corrupt (foreign_exceptions=1), a diagnostic run, not a gate run.** The shadow correctly stands down (`automata shadow: requested … but INACTIVE … and `automata` is enabled (ours)`) and all six classes are created, but the game faults at `SimCity 4.exe+0x00454676` (ACCESS_VIOLATION reading 0) during the budget's `DepartmentBudget::Read`. Cause found and fixed in this lane: `QueryInterface` answered `cIGZSerializable` (0xE4FDA3D4) with the **primary** interface pointer instead of the `cIGZSerializable` sub-object, and two reference-returning stubs dereferenced a null. Superseded by glue19-enable-2..4.
summary: glue19 enable 1, corrupt; QueryInterface returned the wrong sub-object
