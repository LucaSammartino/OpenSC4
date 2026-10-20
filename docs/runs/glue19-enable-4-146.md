tag: glue19-enable-4-146
run: 184
date: 2026-10-16
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: corrupt
switches: glue19 (the GZCOM classes, after the QueryInterface fix and the empty-container stubs), `build-runs/game-queue/glue19-enable-4.req` (Konradshohe, `enable = automata`, turbo to 2032-01-01)
result: **corrupt (foreign_exceptions=1), a diagnostic run, not a gate run.** All six components now `Read -> 1` through their own `Write`-shaped records (script, cm, vehicle, pedestrian, aircraft, watercraft) and a second set of six is created in the Init phase; the game then faults at `SimCity 4.exe+0x0045BF1B` (ACCESS_VIOLATION reading 0, `eax=1 ecx=0 edx=0x30000 ebx=0x00A80810`) right after `AddAgent` for the terrain and surface water, before any further call of ours is logged. The failing slot is therefore one of the stubs the city's Init phase calls. With this on the branch the classes are **not registered** (glue19 question A4): every gate run stays clean.
summary: glue19 enable 4, corrupt; the Init-phase fault after the records Read, so the classes stay unregistered
