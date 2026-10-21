| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-models4 (RE spec: exact cSC43DRender owner beyond view3d.md §3/§4, docs/specs/render3d.md) | `re-models4` / (RE spec lane) | ready, partial owner spec (no whole-owner credit): render3d.md §1-6 + §7 remaining list + 12 conformance cases; questions R3D-1..R3D-3 filed; no ABI change, no game runs | 1. Next round: Init 0x007C6410 / Shutdown 0x007C7070 / ReadProperties 0x007C5F40. 2. Picking group (Pick, PickTerrain, PickTerrainAlways, IsModelVisible, view-object list). 3. Shake fill 0x007C63A0 + SetShakeOffsets 0x007C86D0. 4. Backing store create/save/restore, translated/zoom/rotation view updates. 5. Then claim whole-owner credit. |

## Report

**What changed.** New `docs/specs/render3d.md` (owner `cSC43DRender`, extends view3d.md without repeating §3.1–§4):

- §1 identity/lifetime: `QueryInterface` IID table (returns in AL; unknown IID leaves the out pointer untouched),
  `AddRef`/`Release` (count at `+0x10`, last release runs `Shutdown` then deletes), constructor defaults (fog 0,
  lighting 1, fog distances 10000/15000, map scale 1, dirty-area limit INT_MAX, band width 0x40, ...), `GetGZCLSID`.
- §2 serialised record (version 2): `Write`/`Read` run on `this+8`, so instruction offsets are base offsets − 8
  (Ghidra's struct view of those bodies is shifted by 8). Fields: viewport size, viewport rect, camera control
  (CLSID 0xC9C628EC), `+0x90`, `+0x94`, camera mode `+0xDC`.
- §3 accessors: camera/camera control refcounting, viewport, zoom (int→float) vs rotation (int unchanged), camera
  mode state machine (1 = 2D, 2 = 3D, 3 = UI) and `ReturnRenderSystem` restoring it, fog/lighting toggles (they differ
  in store-vs-redraw order), backing-store switch is a process global `0x00B0DC19`.
- §4 `ForceFullRedraw`/`DoFullViewUpdate`/`UpdateViewportRect`: collapse via `+0x109` until next `Draw`.
- §5 model change queue: 0x30-byte items, kinds 0–5, the grid-element flag word, `ModelShouldGoIntoBackingStore`,
  `ChangeModel` jump table read from the binary (0x007C94B0: 2→3, 3→4, 4→2 unreachable, 5→5), overflow at exactly 200.
- §6 dirty rectangles: node layout, append/area-limit rule, merge step outline, `ProcessDirtyRectangleQueue`,
  `ClearDirtyRectangleQueue`, `DoPartialViewUpdate`, `DoViewCleanup` 64-px banding; §6.3 `StopScreenShake`.
- §7 explicit list of the remaining owner methods; §8 twelve conformance cases.
- view3d.md §3.4: one cross-reference line to render3d.md.

**Sources.** decomp/cSC43DRender.c plus re_query asm for every numeric or return-channel claim (QI, Release,
SetZoom/SetRotation, fog/lighting, Write/Read offsets, ChangeModel table bytes, StopScreenShake compare). REA not
used.

**Questions.** docs/specs/questions/render3d.md: R3D-1 stream slot 8 name; R3D-2 writer of `+0x60`; R3D-3 queue item
layout and dirty-rect merge scoring / mode semantics.

**Not done.** Init/Shutdown/ReadProperties, picking, shake fill and `SetShakeOffsets`, backing-store create/save/restore,
translated/zoom/rotation view updates, terrain metrics, geometry helpers, `ClearViewport` and draw helpers (§7).
