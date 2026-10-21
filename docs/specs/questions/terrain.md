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
- **TR33 - §15.5.1 step 7:** what `0x00743B60` and `0x007490B0` do with `&terrain+0x40` (the whole-city vertex
  rect) right after `+0xEC` is set. The replacement does a whole-city `RedisplayTerrain(1, 1, rect, 0)` instead
  [choice], because §15.5.8 makes the two redisplay posts the view's only channel for heights and water. Is that
  the observable effect of the pair (and with which last argument), or do they also refresh textures/lighting?
  What settles it: a description of the two functions' effects.
- **TR34 - §15.5.1 step 6:** what object terrain `+0x100` holds and what its slot 5 (`+0x14`) does. §2.3's field
  table does not list `+0x100`, so the replacement does not call it [choice]. What settles it: the writer of
  terrain `+0x100`.
- **TR35 - §15.5.8 / §15.5.11 item 15:** who calls the view's `Shutdown` (`0x007560F0`, which unsubscribes the six
  messages) and when: the terrain's own `Shutdown` / `PreCityShutdown`, the view's last `Release`, or the app.
  The replacement only `Release`s its factory reference in the terrain's destructor [choice]. What settles it: the
  call sites of `0x007560F0`.
- **TR36 - §15.5.11 item 12:** the source of the vertex record's `+0x16` surface texture id that
  `ComputeTextureSpansForRow` and `DrawTerrainSurfaceRows` read. §2.2 gives the record layout but no writer of
  `+0x16`; the replacement leaves it 0 [choice]. Which terrain function computes it (from altitude/slope, the
  terrain type, or the brush texture accumulation `+0xC0`) and when? What settles it: the writers of record `+0x16`.
- **TR37 - §15.5.8:** the terrain's hide/flip slots call the view's `SetTriangleIndicesForCell` (`0x00752AF0`)
  directly for the touched cell. Its calling convention and arguments (view base or `+0x0C`; `(x, z)` order) are
  not given, so the replacement does not call it [choice] and the view's index buffers only follow those slots
  at its next whole-grid rebuild. What settles it: the call at `0x007411E1` and the prologue of `0x00752AF0`.
<!-- from: glue-terrain3 (game run gt3-repl-4, enable = terrain, 2026-10-21) -->
