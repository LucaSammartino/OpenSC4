# terrain-view: answered questions (archive)
<!-- Archive for [terrain-view.md](../../terrain-view.md). Lanes do not read this file. Open questions are in ../terrain-view.md. -->

<!-- from: re-terrainview (2026-10-20) -->
- ~~**TV1 - §4.1:** what the draw context's `+0x50` holds when `Update` runs (0 selects the land layers, anything
  else the water layers) and who sets it; the view registers itself twice with the draw registry, at layers 0
  and 2 (terrain.md §15.5.4). [the draw registry's current pass, inferred]~~
  **ANSWERED 2026-10-21 (re-render3d):** the pass word: the terrain view is registered at view-object layers 0 and 2,
  so its `Update` runs with `+0x50` = 0 (the land, before the static pass) and 2 (the water, after it) (render3d.md
  §11.1; view3d.md V3D-2).
