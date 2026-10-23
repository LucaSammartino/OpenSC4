| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-render3d: the 3D render core's quad grid, passes and the methods render3d.md left open, folded into render3d.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-view3dwin) | **ready** (docs and RE tables only). render3d.md §5.1 (the quad grid), §5.2 (the two passes and their rounds), §13 (view updates, single-model additions, clearing, backgrounds, the driver, snapshots, clipping, terrain metrics) added, §5 flag table and limits completed, §7 shortened, §12 `+0x64`; view3d.md V3D-2/3/4 and terrain-view TV1 answered; coverage rows for the render core (`cSC43DRender`, `SC4QuadGrid`, its quadrant, `SC43DGridElement`); 152 names (17 new, 135 renamed). Specified coverage +47,399 B / +193 functions | — |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`) and the
Ghidra export. Written on 2026-10-20 as a separate `render-core.md` and held back when `main` gained render3d.md; this
is that work folded into render3d.md, adding what render3d.md §7 listed as not specified. Where the two disagreed,
upstream was right that `SetMapScaleFactor` stores its argument at `+0x264` (`0x007C9FA4`; this lane had said it
ignores it); the lane's quad grid numbers were re-read (8192 s and 32 s at `0x00ABA9E8`/`0x00ABA9EC`, the cell
rounding by the `1.5 × 2²³` trick at `0x005F2A9A`).

### What changed

- **render3d.md** §5: the grid-element flags `UpdateGridElementObjectFlags` sets from the properties and draw
  attributes, the pick and collect bits, and the render-property names of the three backing-store limits.
- **render3d.md** §5.1 (new): `SC4QuadGrid`: container space and the rotation offsets, `CreateQuadGrid`, `SetSize`,
  the cell rule and its round-to-even quirk, quadrant flags, `AddElement`, `RemoveElement`, `MoveElement`,
  `SetQuadrantFlagsForElement`, `GetAllElements`, `RemoveAllElements`, the grid element's layout.
- **render3d.md** §5.2 (new): `DrawStaticView` and `DrawDynamicView`: what each collects, the sort keys, the list
  order, the 12-round rule and its shared counter in the dynamic pass, the highlighted elements.
- **render3d.md** §13 (new): `AttemptTranslatedViewUpdate`, `DoTranslatedViewUpdate`, `DoViewZoomChange3` with
  `RebuildQuadGrid`, `DoViewRotationChange2`, `DoSingleModelStaticDrawAddition`, `ClearViewport` and the rectangle
  helpers, the background picture, `SetGDriver` / `StaticSetGDriver`, `TakeSnapshot`, `SetClipRect` /
  `UnsetClipRect`, `UpdateVisibleTerrainMetrics`, the statistics strings. §7 lists only what is still open; §12
  says what sets `+0x64`; conformance cases 13–16.
- **view3d.md**: V3D-2 (the brackets set the pass word and draw the layer's view objects), V3D-3 (the render
  properties' boolean table), V3D-4 (the backing-store switch) answered in its question list.
- **terrain-view.md**: the pass word is a fact, not an inference; TV1 answered (moved to
  questions/answered/terrain-view.md).

### Names and coverage

- re/symbols/render-core.tsv: 152 names (17 new; 135 replace auto names). The `names_windows.tsv` delta (−135 +152)
  is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: `cSC43DRender` (render3d.md, view3d.md), `SC4QuadGrid`, `SC4QuadGridQuadrant`,
  `SC43DGridElement` (render3d.md), as a "3D render core" subsystem ahead of the `cSC43D*` catch-all; the renderer
  had no spec in the map before.

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-view3dwin 3,306,464 bytes in 14,852
functions; with this lane 3,353,863 bytes in 15,045 functions (+47,399 B, +193 functions; denominator unchanged).
`docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered: view3d.md V3D-2, V3D-3, V3D-4 (in place); terrain-view TV1 (archived).
- The lane's own RC1 (the record's fields) and RC2 (the merge trigger, the shake overloads) are not filed: render3d.md
  §2, §6.1 and §10 now answer them.
