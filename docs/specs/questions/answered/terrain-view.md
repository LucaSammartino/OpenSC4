# terrain-view: answered questions (archive)
<!-- Archive for [terrain-view.md](../../terrain-view.md). Lanes do not read this file. Open questions are in ../terrain-view.md. -->

<!-- from: re-terrainview (2026-10-20) -->
- ~~**TV1 - §4.1:** what the draw context's `+0x50` holds when `Update` runs (0 selects the land layers, anything
  else the water layers) and who sets it; the view registers itself twice with the draw registry, at layers 0
  and 2 (terrain.md §15.5.4). [the draw registry's current pass, inferred]~~
  **ANSWERED 2026-10-21 (re-render3d):** MATCHES — the pass word: the terrain view is registered at view-object layers 0 and 2,
  so its `Update` runs with `+0x50` = 0 (the land, before the static pass) and 2 (the water, after it) (render3d.md
  §11.1; view3d.md V3D-2).

<!-- from: re-terrainview (2026-10-20) -->
- ~~**TV2 - §1, §8.2:** which pipe-coverage state each plumbing colour (`0x497F42A4`, `0xE97F42B0`, `0xA97F42CE`,
  `0x097F4C4E`) stands for, what the flag `0x297F42B7` switches, and what `0x29AB15C7` (0.9) weighs in the cliff
  colours. [the values as shipped]~~
  **ANSWERED 2026-10-27 (re-x-net):** MATCHES for the values, with the meanings now fixed — the four colours are chosen by (building or producer lot cell, watered): built dry `0x497F42A4`, built watered `0xE97F42B0`, empty watered `0xA97F42CE`, empty dry `0x097F4C4E`; `0x297F42B7` switches the tint of that last group on (shipped) or off; `0x29AB15C7` is the weight of the cliff shade (terrain-view.md §1, §7.1, §8.2).

<!-- from: re-terrainview (2026-10-20) -->
- ~~**TV3 - §9.3:** what the object at `+0x74` is: it is registered at draw layer 5 next to the ripples, listens to
  `0x89171496` and sits on the animation tick manager (its shutdown is `0x00762300`). [a second animated water
  effect, inferred]~~
  **ANSWERED 2026-10-27 (re-x-net):** DIFFERS — nothing in the executable ever stores anything but null at `+0x74`: it is a vestigial hook (registered at draw layer 5 and torn down only when non-null) for a second water effect the shipped game never creates (terrain-view.md §9.3).
