tag: glue19-shadow-1-142
run: 192
date: 2026-10-16
lane: glue19
dll: FreeCity4-glue19
pacing: turbo
verdict: clean
switches: glue19 `b772fef7` + the first automata shadow (glue19: the automata state-record corrections and the state stack's rules, and the automata shadow), `build-runs/game-queue/glue19-shadow-1.req` (Konradshohe, `automata_shadow = 1`, turbo to 2032-01-01), DLL `FreeCity4-glue19` sha256 `9e31c5ff`
result: **clean, 62.65 days/s, 34.8 s real, but ref_warnings=426 and the shadow not exact.** The shadow compared 3 of 6 components DIFFERENT at their first reference (`cm` byte 2 ours 0x89 original 0x01; `vehicle` / `pedestrian` byte 14 ours 0x18 / 0x08 original 0x01), `script`, `aircraft` and `watercraft` identical. Two shadow bugs, both in its streams: `fc4::MemOStream` has no `put_ref` / `put_str` callback (its error callback always answers 0, so the write silently dropped every reference and template name), and a reference's token must be the capture's object number rather than an object pointer. This run is superseded by glue19-shadow-2-143.
summary: glue19 shadow 1, clean but ref_warnings=426; found the shadow's reference-token bug
