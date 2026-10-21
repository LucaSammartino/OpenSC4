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
<!-- from: glue-terrain3 (enable = terrain over the TR21 view contract, 2026-10-21) -->
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
- **TR38 - §15.5.8 / TR30:** the precondition of the edge-layer refresh `FUN_00755B60` that a replacement terrain
  breaks. Runtime evidence (gt3-repl-4): the first queued `0x278E6003` after `DoPostCityInit` (whole-city rect
  `(0, 0, 128, 128)`, data1 = data2 = `0x80`, a flat 300 m terrain, sea 250, 128 x 128 cells, `T+0x6C` a live
  `0x24`-byte array) faults in the view at `0x00755C2B` (ACCESS_VIOLATION writing `0x16B23FF8`; caller
  `0x00756AAC` in `FUN_007569F0`, under the view `DoMessage` `0x007575DE`). Registers: ecx `0x16B84C24`, edx
  `0xFFF9F3D0`, esi `0xFFF9FFE8` (edx - esi = -24 x 129), edi `0x4102`, ebx 2, eax 6, ebp `0x437A0000` (250.0f).
  The game itself posts whole-city heights redisplays (`SetSeaLevel`, `0x470BD214`, the grid toggle), so a
  whole-city rect alone is not the fault. Which input does `FUN_00755B60` index with (terrain altitudes, slot 29,
  terrain fields beyond §2.3, the edge records `0x64 + side`, or state set by §15.5.1 step 7's pair `0x00743B60` /
  `0x007490B0`, TR33), and what range must it be in? The replacement keeps the TR33 whole-city redisplay; it is
  the fault's trigger [choice: unchanged until answered]. What settles it: a byte walk of `0x00755B60`-`0x00755C47`
  naming the array written at `0x00755C2B` and the source of its index.
- **TR39 - §13.1 slot 96:** who calls the terrain's `Load` (slot 96, `0x00747800`) on a city load, with which
  segment, and when relative to `Read`, `Init` and `PostCityInit`. In gt3-repl-4 the replacement's slot 96 was
  never called with a non-null segment (no Load log line; `Read` ran, then `Init`), so the replaced city ran on the
  §2 flat default instead of its saved altitudes. Is slot 96 called by `cSC4City` (which function, which
  interface pointer), or does the original terrain call `Load` itself from `Init` with a segment it obtains (from
  where)? [choice: the replacement only answers slot 96] What settles it: the call sites of `0x00747800`.
- **TR40 - §15.5.5 slot 67 / §15.5.9:** who initialises the view's city-edge layer state (`0x00B4C760`,
  `0x00B4C790`, `InitializeCityEdgeLayerHeights` `0x00757300`) for a city, and what the terrain's slot 67
  `DoLighting` (`0x00742930`, called once from `DrawTerrainLayers` with `V+0x34`) must do. Runtime evidence
  (gt3-repl-5, the TR33 whole-city post switched off, slot 67 a no-op, no other change): the first draw faults at
  `0x007540A5` (ACCESS_VIOLATION reading `0x00000008`; edx 8, esi 0, ebx `0x81`; `0x00B4C760` twice on the
  stack; called from `0x0075BFA3` in `DrawTerrainLayers` under `Update` `0x0075C07E`). Together with TR38 this
  says the edge-layer state is not set up when a replacement terrain is in place: is it set by §15.5.1 step 7's
  `0x00743B60` / `0x007490B0` (TR33), by the original slot 67, or by a terrain field the view reads that §2.3 does
  not list? [choice: slot 67 a no-op, as §15 calls it render] What settles it: the callers of `0x00757300` and a
  description of `0x00742930`, `0x00743B60` and `0x007490B0`.
