tag: run-71
run: 71
date: 2026-10-14
lane: run
dll: unknown
pacing: unknown
verdict: ended
switches: main `ae82e65` (demand capacity fix, verdict `ended`), soak: run 69's eight replacements (our power), `observe = 1`, quit, 900 s
result: **`ended`, 0 problems** (`ref_warnings=0`): the game ended the city itself in 2060-08 (julian 2473795) at funds about −100k. The new verdict and the quit after it worked (`PreCityShutdown` WARNING, `RequestQuit`, verdict at `PostSystemServiceShutdown`). The harness missed the verdict because of a read race, fixed in `e5fd34a`.
