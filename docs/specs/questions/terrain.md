# terrain: open questions
<!-- Questions about [terrain.md](../terrain.md). Answered entries move to answered/terrain.md. -->

<!-- from: the terrain 3D view contract lane (re-terrain3d, 2026-10-20) -->
- **TR28 - §15.5.10 slot 0:** the extra IID `0xC989F960` that `cSTETerrainView3D::QueryInterface` answers with the
  object base (`0x0075273B`), which `cSC43DRender::DoMessage` asks for at `0x007CB1C1` and caches at `+0x194`.
  No table in `re/class_id_table.tsv`, `docs/abi/*` or any spec names it. What interface is it — a
  `cISC4TerrainView`-style SDK declaration, or an internal render hookup? [unnamed] What settles it: the
  gzcom-dll/SDK headers or a symbol-table sweep of the Mac build for the matching GUID.

<!-- from: the terrain 3D view contract lane (re-terrain3d, 2026-10-20) -->
- **TR29 - §15.5.8:** the identity of message `0xC9DA96EA`. The view subscribes to it (`0x0075C860`), unsubscribes
  in `Shutdown` (`0x0075620B`), and its handler calls the view's own slot 13 with a null rect, then stores the
  message's `GetData2()` into `view+0x44` (`0x00757613`–`0x00757633`). It is not in `re/class_id_table.tsv` and no
  spec names it. Who posts it and what does `data2` mean (a lighting-band index? a quality level?) [unnamed]
  What settles it: an xref sweep of `0xC9DA96EA` posts across the image, or the Mac symbols.

<!-- from: the terrain 3D view contract lane (re-terrain3d, 2026-10-20) -->
- **TR30 - §15.5.8:** the semantics of the six `0x278E6003`/`0x678E6E3A` edge-layer refresh calls
  (`FUN_00755B60`, called from `FUN_007569F0` at `0x00756A6C`/`0x00756A8C`/`0x00756ABA`/`0x00756ABF` with row and
  column arguments). They rewrite the three city-edge layer-height arrays (`0x00B4C760`, `0x00B4C790`) that
  `InitializeCityEdgeLayerHeights` (`0x00757300`) fills from ini property `0x29AB15C5` at init. The contract for a
  replacement view needs the exact per-row/column rule (which of the three arrays each call refreshes, and what
  the four int arguments mean). *[inferred: a border-row/column refresh]* What settles it: a byte-level walk of
  `FUN_00755B60` (`0x00755B60`, 231 bytes) against the array layouts.

<!-- from: the terrain 3D view contract lane (re-terrain3d, 2026-10-20) -->
- **TR31 - §15.5.10 slot 16:** `LevelCell`'s return channel. `docs/abi/vtables.tsv` records pop 12 / ST0, the body
  (`0x00759750`–`0x00759891`) posts no value, and the only call site found is the cheat router. Is ST0 a real
  return the callers read (the x87 stack would leak a value if not), or a `vtables.tsv` artefact of the body's
  unrelated FLD/FSTP pairs? [ST0, unverified] What settles it: the caller of slot 16 outside the view (if any) or
  a stack-balance check on `0x00759891`.

<!-- from: the terrain 3D view contract lane (re-terrain3d, 2026-10-20) -->
- **TR32 - §15.5.4 step 7:** the identity of the two `StaticInit` functor objects: the pair
  (`0xC9EFB727`, `0x69EFB6F8`) queried through the image-codec factory `0xA2FFB5D3` (`0x0075C2AC`–`0x0075C2FA`)
  into `0x00B4C7A0`/`0x00B4C7A8`, whose slot 21 (`+0x54`) the water redraw uses as a level→colour lookup
  (`0x00756C39`). Neither id is in `re/class_id_table.tsv`. Are they an `SC4DevInsight`-style tunable pair or
  compiled-in LUT classes? [unnamed] What settles it: a resource sweep for the two GUIDs in the DATs, or the Mac
  symbol table.
<!-- from: glue-terrain3 (game run gt3-repl-4, enable = terrain, 2026-10-21) -->

<!-- from: glue-terrain3 final run4 round, coordinator transcribed committed Handoff 97b1c237 (2026-10-21) -->
- **TR42 - section 12.1 reconciliation preference:** what is the exact width, signedness and access contract of the preference at `cISC4App::GetPreferences()+0x4EC` (default 1)? The final diagnostic round found the animation-tick reconciliation path missing but did not establish this field's width. Do not infer a byte or dword read from its default value.
- **TR43 - section 12.1 / slot 89 ReconcileEdgesWithNeighboringCities:** specify `ComputeEdgeVerticesToReconcile`: arguments, neighboring-city inputs, comparison and tolerance, output edge vertices, ownership and side effects. How does its result control reconciliation and the queued `TerrainReconciliation` message `0xCA63D517`? Final diagnostics show that missing reconciliation leaves `ConnectionsReady` absent and dialogs enabled; treating the unknown comparison as "no mismatch" is unverified.

<!-- from: glue-terrain-replace14 (2026-10-24) -->
- **TR44 - section 15.1 slot 54 and the reported replacement crash:** specify the interface identity, ownership and initialization lifecycle of the render object at terrain `+0xE4`. The slot table says only "render object"; the current plugin initially stores the city there and later replaces it with the terrain-view object base. The task reports a virtual call to `0x41800000` from `0x004918E0` after view creation (callers `0x0050CF71`, `0x00491A1D`, `0x00494495`, `0x00494A18`). Which interface and slot does that caller require, and is its receiver obtained from terrain slot 54, slot 66, surface water, or a direct field? [unresolved; no native signature inferred; slot 66 corrected separately per section 15.5.6]

<!-- from: replace-terrainview29 (2026-10-26) -->
- **TR46 - section 15.5 exact frame submission:** specify the ordered draw-context calls, arguments, vertex/index contents, render-state/texture bindings and frustum rules for one terrain-view `Update`, including `DrawTerrainLayers`, `DrawTerrainSurfaceRows`, `DrawWaterLayers`, `DrawSideCliffs` and the software-cache rebuilds. Sections 15.5.5 and 15.5.9 identify readers, buffer sizes and producers, and section 15.5.10 specifies the two single-quad helpers, but do not define the full frame stream. `draw-context.md` section 9 defines what context calls do, not which calls the terrain view submits. What are the six indices for each combination of the flags in section 15.5.13.4? The lane requires equal context-call frames with the key off/on; omitted passes or invented indices would be unexplained differences. Frame implementation/comparison is stopped pending these rules; only the specified height-cache copy is implemented.
