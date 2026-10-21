| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-models4 (RE spec: exact cSC43DRender owner beyond view3d.md §3/§4, docs/specs/render3d.md) | `re-models4` / (RE spec lane) | parked after FINAL round 3/3, partial owner spec (no whole-owner credit): render3d.md §1-6, §9-12 (dirty merge scoring, picking, view objects, backing store added), §7 remaining list non-empty; RND3 answered, RND1/2/4 open; no ABI change, no game runs | 1. View updates (DoTranslatedViewUpdate, DoViewZoomChange3, DoViewRotationChange2, AttemptTranslatedViewUpdate, UpdateVisibleTerrainMetrics). 2. Geometry helpers + ClearViewport/draw helpers. 3. PickTerrainAlways ray clip numerics (0x007C7E20, 0x007C6EC0). 4. DoSingleModelStaticDrawAddition, SetGraphicSystem/SetGDriver/StaticShutdown/dtor. 5. Then whole-owner credit. |

## Round 3 report (FINAL 3/3, parked partial)

- §6.1 merge step fully specified from 0x007C8F9A-0x007C9172: score = nodeArea + newArea - unionArea, minus
  min(newArea, nodeArea) when either mode is 0; strict best (first wins ties) from -limit; merge when count >= 16
  (global 0x00B0DC14) or float(best) >= -2500.0 (0x00B0DC10); growth accounting, recursive mode-6 re-add, area
  over-count. Rect filter 0x007C2C70 (overlap test + in-place clamp, no left<right test). Cases D1-D6.
- §5 queue item layout (0x007C2E50): model, kind, unused byte, unused 16 bytes, 6-float box; per-kind box source;
  dirty-rect mode values. RND3 ANSWERED (MATCHES), moved to answered/render3d.md, QUESTIONS.md reindexed.
- §11 view-object lists (4 sorted lists + terrain view +0x198), IsModelVisible x2, screen Pick/PickTerrain/
  PickTerrainAlways, ray PickTerrain (QI 0x6771477D, +4.0 retry, ground fallback), ray Pick (candidate collection,
  terrain distance limit, filter IID 0x26BB767E, model slot 11). Ray PickTerrainAlways only in outline. Cases P1-P6.
- §12 backing store: Create (probe), Destroy, Save (shake-offset clipping, +0x64 rule), Restore x2,
  ReevaluateBackingStoreObjectFlags. Cases B1-B4.
- Evidence: re_query asm for every numeric/branch claim cited; data constants read from bin/SimCity4.exe by VA
  (build-runs/va.py). REA unavailable this round ("No app is open"), fallback re_query + decomp/.
- §7 remainder (not empty, so no whole-owner credit): view updates, geometry helpers, ClearViewport and draw
  helpers, snapshot/stats, DoSingleModelStaticDrawAddition, SetGraphicSystem/SetGDriver/StaticSetGDriver/
  StaticShutdown/destructor, +0x1FD writer, PickTerrainAlways ray clip numerics.
- No ABI change, no game runs, no engine/tools edits.

## Round 2 report

- render3d.md §9: `Init` (14 ordered steps: once-per-process StaticShutdown registration, quad grid, 8 per-rotation
  margins = scale x float constants read from the binary, prefs AddRef, 11 message registrations, lighting manager,
  2D camera ortho with 32767.0f far, transposed projection copy to +0x98, draw-context wiring, +0x1F4 = 40000),
  `ReadProperties` (exemplar 0x6534284A/0x690F693F/0x4A639EF2, properties 0xEA639FB1..BB, exactly-4-values rule,
  draw-context +0x54 16-byte entries), `Shutdown` (12 ordered steps, 12 unregistrations incl. 0x4348B111, what it
  does not touch). Cases L1-L6.
- §10: shake node table, slot 42 prepends and returns the handle, slot 43 overload block ([a,0] amplitude, [0,60,0]
  rate, k 1, pattern 0), random table (seed 88, 64 pairs in [-10,10]) and sine table (10 sin(i pi/32)),
  `SetShakeOffsets` full per-frame rule (distance falloff R = cc+0x134 x 640, curve sampling, phase += F min(t,dt),
  0.6^(4-zoom), round half to even), `GetShakeOffsets`. Cases S1-S6.
- §6.3 corrected: node +0x08 is the start time; Stop pulls it back so the shake plays its fade.
- Evidence: re_query asm for every numeric path; data constants read from bin/SimCity4.exe by VA. REA not used.
- Questions: RND4 filed (overload never writes the fade length). RND1..3 still open (no byte proof this round;
  interfaces.tsv has no cIGZIStream/cIGZOStream slot 8 row).
- No ABI change, no game runs, no engine/tools edits.

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

**Questions.** docs/specs/questions/render3d.md: RND1 stream slot 8 name; RND2 writer of `+0x60`; RND3 queue item
layout and dirty-rect merge scoring / mode semantics.

**Not done.** Init/Shutdown/ReadProperties, picking, shake fill and `SetShakeOffsets`, backing-store create/save/restore,
translated/zoom/rotation view updates, terrain metrics, geometry helpers, `ClearViewport` and draw helpers (§7).
