| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-regionview7: the region playfield `cSC4WinRegionView`, its tile canvas `cSC4TiledScrollingCanvas` and the signpost animator `cSC4RegionSignpostAnimatorWinProc`, new spec region-view.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-graphs7) | **ready** (docs only). region-view.md §1–§11: the view object, `Init`/`Shutdown`/`SetArea`, transparency, the city settings and selection, positions and hit testing, the city entry, the canvas (tiles, invalidation, wrap-around scrolling, rendering, the renderer callback), `RenderTile` (background, composed city images, labels, airport/seaport icons, selection), the four image helpers (runs, span blit, coverage fill, copy-and-tint), `UpdateCity` (points of interest, the isometric projection, effects), signposts and their contents, the bobbing animator, quirks, six conformance cases; questions RGV1–RGV6; owners claimed: `cSC4WinRegionView`, `cSC4TiledScrollingCanvas`, `cSC4RegionSignpostAnimatorWinProc`: specified +15,215 bytes (+58 functions) | RGV1–RGV6 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 58 claimed functions were disassembled, plus the view's constructor (counted under the region screen) and its
three unattributed functions. The decompile loses the register arguments of the image helpers, so every body was
read from the disassembly. The projection constants, the bob constants, the rating scale and the icon table were
read from the data section. The price resource was looked up in `SimCity_1.dat`…`SimCity_5.dat` with
`tools/dbpf.py`. Written on 2026-10-23. No upstream lane covers these owners (checked on `c11352c3`).

### What changed

- New docs/specs/region-view.md.
- Read on the bytes:
  - the view layout and the city entry as the view reads it;
  - the canvas: tile count `(W − 1) / t + 2`, floor-division invalidation, wrap-around scrolling with re-dirtying
    of the wrapped tiles, the render loop and the renderer callback;
  - the two list formats (sorted run pairs, used by the odd-index hit test; coverage spans);
  - the premultiplied span blit, the coverage fill, the half-strength tint and the one-pixel edge spread of
    `BuildRuns`;
  - the label rules (tutorial, established, selected, the two label switches), fonts `0x8A8CC984/5 − 2·size`, the
    4/3 width box;
  - the isometric projection of points of interest, the rounding trick, the `region_airport`/`region_seaport`
    effects;
  - the signpost rules, scripts and fields (name, mayor, difficulty, budget with the negative variant,
    population and jobs, rating `× 0.005 + 0.5`, the unclaimed-land price);
  - the animator's `1 − |sin|` bob with a 1.5 s period.
- docs/specs/questions/region-view.md: RGV1–RGV6 (new prefix RGV); QUESTIONS.md regenerated.

### Quirks found

- A failed canvas `Init` leaves the view initialised; `SetArea` ignores its re-init.
- `UpdateCity` redraws every city.
- `CopyAndTint` takes both sides of its size minimum from the source and never unlocks the source.
- `BuildRuns` changes the composed image in place.
- An unused colour in `RenderTile`.
- The unclaimed-land price is read from the Crime Simulator exemplar's key, so it is always 10000 in shipped
  data.

### Coverage

- tools/coverage-map.tsv:
  - `cSC4RegionSignpostAnimatorWinProc` → region-view.md, before `cSC4Region*`;
  - `cSC4WinRegionView` → region-view.md, before `cSC4WinRegion*`;
  - the existing `cSC4TiledScrollingCanvas` row now cites region-view.md.
- Specified bytes 4,467,400 → 4,482,615 (+15,215, 80.0% → 80.3%, `cov_local.py`, on `5c0ff782` + the seven lanes
  before).

### Not covered

- The region screen's image building (RGV5).
- The GZ surface, font and blitter interfaces (RGV3).
- The effects themselves.
- The three unattributed functions, which have no owner to claim.
