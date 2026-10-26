| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-render3d7: the rest of the 3D render core `cSC43DRender`, folded into render3d.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7) | **ready** (docs only). render3d.md now covers the whole owner: §11.6 the ray `PickTerrainAlways` clip, §13 the scroll copy `DoTranslatedViewUpdate` (the `+0x1FD` writer), zoom and rotation changes, single-model additions, `ClearViewport` and the viewport rectangles, background pictures, the driver, snapshots, clipping, statistics, `UpdateVisibleTerrainMetrics`; §14 the container-rectangle geometry and `RebuildQuadGrid`; §15 the draw-context fog and lighting, the lifecycle remainder and the helpers; §16 the 153-function inventory; RND2 answered; owner claimed: `cSC43DRender`: specified +40,210 bytes (+153 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every body that render3d.md §7 listed as open, and every summary of §13, was disassembled; the float and double
constants were read from `.rdata`. Written on 2026-10-23. Upstream's lane re-models4 is parked after its third
round; this lane finishes the owner on top of its §1–§12.

### What changed

- render3d.md:
  - the completeness line now claims the whole owner;
  - §1.3 names `+0x10C`/`+0x110` (the 4096 m city extent), `+0x60` (ground tiling) and `+0x63`/`+0x64`;
  - §7 becomes a pointer list;
  - §11.6 gives the exact clip: the plane order, the float rounding, the insert and the early stop;
  - §13 is rewritten from summaries into steps;
  - new §14 (container rectangles, the quad-grid rebuild), §15 (fog and lighting, `SetGraphicSystem`,
    `StaticShutdown`, the destructor, sorts and vector helpers) and §16 (the inventory).
- Corrections on the bytes:
  - the viewport rectangles draw in the **2D** camera (slot 25), not the UI camera;
  - `UnsetClipRect` is a camera method;
  - `TurnOnLighting` and `TurnOnFog` run on the draw context;
  - the zoom and rotation changes take (old zoom, old rotation, new zoom, new rotation).
- RND2 answered (both `SetBackgroundPicture` overloads write `+0x60`); the entry moved to
  questions/answered/render3d.md. QUESTIONS.md regenerated.

### Quirks found

- The city extent `+0x10C`/`+0x110` is always 4096 m.
- `DoTranslatedViewUpdate(0, 0)` passes two uninitialised rectangles.
- `PickTerrainAlways` stops a plane's clip after an outside → inside crossing.
- `WorldToContainerRect` turns a box wider than half the grid into a 1-unit sliver.
- A failed `SetBackgroundPicture(image)` leaks the image reference; the texture overload keeps the old size for
  the same texture.
- `SetGraphicSystem(null)` calls slot 29 on the released interface.
- The statistics counters are only ever zeroed.

### Coverage

- tools/coverage-map.tsv: new row `cSC43DRender` → render3d.md, before the `cSC43D*` catch-all.
- Specified bytes 4,553,560 → 4,593,770 (+40,210, 81.5% → 82.3%, `cov_local.py`, on `bb641878` + re-batch7).

### Not covered

- The `SC4QuadGrid` and `SC43DGridElement` internals, and the readers of the visible-terrain fields.
