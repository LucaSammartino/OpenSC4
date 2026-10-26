# Small simulation and region owners: the regional flat terrain, terrain dialogs, record copies, helpers (behaviour spec)

Scope: 23 small owners as tools/coverage.py assigns them: `cSC4RegionalTerrainMap` (23 functions, 785 bytes),
`cSC4RegionalTerrainView` (4, 433), `cSTEChangeTerrainUI` (9, 701), `cSTEEdgeReconcileInformMismatch` (2, 418),
`cSTEEdgeReconcileInformNoMismatch` (4, 396), `cSTEEdgeReconcileInformDamage` (2, 306), `(anonymous_namespace)::cParser`
(4, 686), `cSC4FileTokenizer` (3, 495), `cSC4TileNeighborhoodCompact` (6, 474), `cSC4LotConfigurationFilterMonopoly`
(5, 460), `cS3DBoundingBox` (4, 458), `LinearRandomIterator` (1, 383), `FerryStationInformation` (3, 355), `cADVString`
(4, 312), `cSC4ScriptServiceResourceFactory` (7, 312), `cSC4PrimitivePool` (3, 307), `cS3DVertexArray` (2, 304),
`cSC4TrafficSimulator::TrafficSourceIterator` (2, 283), `cSC4LotConfigurationIdentityInfo` (3, 253),
`cSC4TrafficSimulator::TransitSwitch` (2, 213) and `cTablePropertyReader` (1, 188). Where another spec already
describes the main body this spec names it and adds the rest.

Related: region-model.md (the region's terrain stand-in and view), terrain.md §15.1 (the terrain interface slots the
stand-in mirrors), foundations.md §11.6 and modal-dialogs.md (edge reconciliation dialogs), network-rules.md (the rule
tokenizer and the tile neighbourhood words), lot-configuration.md and zones.md (lot filters), lot-developer.md (the
occupant iterators), traffic-simulator.md (traffic sources, transit switches, ferries), advisors.md (advice strings),
ui-script-service.md (the script resource factory), model-instances.md and render3d.md (bounding boxes, vertex arrays).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the regional terrain map's table `0x00AB8C00`
and its cell conversion, and the dialog bodies were read in the disassembly. Written 2026-10-24 (re-simmisc7).

## 1. The regional flat terrain (`cSC4RegionalTerrainMap`, IID `0xAB953253`) *[compat]*

A flat terrain the region view uses in place of a city's terrain (region-model.md). Constructor `(cellsX, cellsZ)`
(`0x007AACE0`): vertices per row `+0x1C` = cellsX + 1, cell size `+0x20` = 64.0, altitude `+0xC` = 270.0, water level
`+0x10` = 0, a lock byte `+0x24` = 0. Its table (`0x00AB8C00`) mirrors the first 34 slots of `cISTETerrain`
(terrain.md §15.1) with constant answers:

| Slots | Address | Behaviour |
|---|---|---|
| 0 | `0x007AA2A0` | `QueryInterface`: IIDs 1 and `0xAB953253` |
| 3 / 4 | `0x007AA2E0`, `0x007AA2F0` | lock byte := 1 / 0 (the second returns false) |
| 6 | `0x007AA300` | cell width (64.0) |
| 9 / 10 | `0x007AA310`, `0x007AA320` | vertex index `x + z · (cellsX + 1)`; and back (`%`, `/`) |
| 11 | `0x007AA350` | in bounds: `x, z ≥ 0` and `x / 64 < cellsX`, `z / 64 < cellsZ` |
| 12–14, 18, 19, 21, 22 | `0x007AA3B0`, `0x007AA4B0`, `0x007AA4D0` | the altitude (270.0) for any point or vertex |
| 15 / 16 | `0x007AA3C0`, `0x007AA430` | fill a rectangle of altitudes with the altitude / of water depths with 0 |
| 17 | `0x007AA490` | the four corner altitudes = the altitude |
| 20, 23, 30–33 | `0x007AA4C0`, `0x007AA4E0`, `0x007AA510` | 0.0 |
| 24 | `0x007AA4F0` | 1023.0 |
| 25 | `0x007AAD60` | the vertex index of `(floor(z / 64), floor(x / 64))` |
| 26–28 | `0x007AADF0` | the normal (0, 1, 0) |
| 29 | `0x007AA500` | the water level (0) |

Deleting destructor `0x007AAD30` (thunk `0x007AAAF0`). The view (`cSC4RegionalTerrainView`): constructor `0x007AC7D0`
and `Update` `0x007AB630` are region-model.md's; deleting destructor `0x007AB600` (thunk `0x007AAB00`).

## 2. Terrain change and edge reconciliation dialogs *[compat]*

Three small dialog objects share one layout (vtable, count `+4`; `Release` frees at a count of 1): the terrain-change
confirmation (`cSTEChangeTerrainUI`) and the edge-reconciliation notices of foundations.md §11.6.

- `cSTEChangeTerrainUI`: constructor `0x00734D20`; `QueryInterface` `0x00734B60` (IIDs 1, `0xC6AE7085`); `Release`
  `0x00734D00` (shared bodies `0x0048C1C0`, `0x00734DF0`, `0x00735100`). `DoDialog(&choice)` (`0x00734B90`): without
  the game's main window (`0x00B43CC0`) false; otherwise the scripted dialog is created and this object registered
  with it, the dialog runs modally (window manager slot 41) between two calls to the view host `0x00B43CE0` (its slots
  10 and 6); when it ends
  with 11 (OK) the selected item of its list (`0xA1336CC0`) is matched against the choice table `0x00AB3864`
  (one entry) and stored; the dialog is removed and released; true for OK. The procedure (`0x00734B10`) ends the
  dialog (window manager slot 9, code 15) with 11 when the button `0xB` of window `0x10000001` is pressed; the second
  procedure (`0x00734D90`) does the same for buttons `0x6A4D0A6B` and `0x6A4D0A71` of window `0x6A4D0A59`.
- `cSTEEdgeReconcileInformNoMismatch` (constructor `0x00734FB0`, `Release` `0x00734FC0`): `DoDialog`
  (`0x00734FE0`) shows the script dialog `{0, 0x96A006B0, 0x0A4D0C43}` modally the same way (modal-dialogs.md).
- `cSTEEdgeReconcileInformMismatch` (constructor `0x00735250`) and `cSTEEdgeReconcileInformDamage` (constructor
  `0x007350F0`): their `DoDialog` bodies `0x00734E10` and `0x00735120` are foundations.md §11.6's.

## 3. Text and rule helpers *[compat]*

- `cParser` (network-rules.md, the rule value tokenizer `0x0063DDF0`): its character classes `0x0063BF30` (the C
  runtime's class test), `0x0063BF50` (letter, digit or `_`), `0x0063BF70` (letter).
- `cSC4FileTokenizer`: constructor `0x00621C30` (two empty strings `+0xC`, `+0x20`, counters 0); reading is
  network-rules.md's (`0x006210B0`, `0x00622620`).
- `cADVString` (advisors.md): constructors `0x00603000` (empty) and `0x00417A30` (from a C string).
- `cTablePropertyReader(holder, id, row)` (`0x004084E0`): remembers the holder, its type and the row; reads property
  `id` (holder slot 37) and keeps the row only when it is within the property's count (slot 64).

## 4. Lot, network and traffic records *[compat]*

- `cSC4TileNeighborhoodCompact` (one dword, a byte per edge; network-rules.md): constructor `0x00620DD0`;
  `HasConnectionOnEdge(e)` (`0x00620EC0`): the edge byte's low two bits non-zero; `SetEdgeCondition(e, v)`
  (`0x00620FC0`): replaces the edge byte; `SetEdgeConnectionDirection(e, d)` (`0x00620FF0`): the edge byte := the
  direction code from the table `{2, 3, 0, 1, 2, 3, 0, 1}` at `(d + 2e) & 7`, keeping bit 2.
- `cSC4LotConfigurationFilterMonopoly(flag)` (`0x004BE0C0`): a lot-configuration filter holding the flag and three
  city services; `0x004BE130` is a forwarding body (slot 17 of the object at `+0xC` with `(argument, +8, 0)`);
  deleting destructor `0x0072A730` (lot-configuration.md has the test).
- `cSC4LotConfigurationIdentityInfo`: constructor `0x0047D8F0` (zero fields, an empty string `+0x10`).
- `FerryStationInformation`: copy constructor `0x0056D8B0` and assignment `0x0056D920` (a counted reference, a byte
  vector and the scalar fields); traffic-simulator.md has its use.
- `cSC4TrafficSimulator::TransitSwitch`: copy constructor `0x006D6C30` (the fields and a vector of 10-byte entries,
  copied by `0x006D68B0`). `TrafficSourceIterator` (`Reset` `0x007107A0`, `GetNext` `0x007107F0`) is
  traffic-simulator.md's.
- `LinearRandomIterator` (`0x005CF610`): the occupant iterator's `Reset` of lot-developer.md.

## 5. Rendering helpers *[compat]*

- `cS3DBoundingBox::Translate(p)` (`0x005EDE00`): moves the box so its x and z centre and its bottom are at `p`;
  `0x00676CC0` is a vector-of-pointers insert (grows to `size + max(size, n)`). The other bodies are
  model-instances.md's and render3d.md's.
- `cS3DVertexArray`: `0x00800E80` allocates through the vertex allocator (`0x00B4E670`, under its lock).
- `cSC4PrimitivePool`: `RestartArrays` (`0x005EC4A0`) empties the vertex and index counts; `Draw(context)`
  (`0x005EC5D0`) sets the vertex buffer and draws in batches of at most 1,536 primitives (indexed when there are
  indices); `GetPrimIndices(n)` (`0x005EC850`) reserves `n` primitives of indices, growing the index vector and
  calling the pool's growth callback.
- `cSC4ScriptServiceResourceFactory` (IID `0x8AA22237`, constructor `0x005F4860`, `QueryInterface` `0x005F47C0`,
  `Release` `0x005F4800`, destructor `0x005F4820`): loads UI-script resources: strings with `SC4LoadString`
  (`0x005F4840` into a given string, `0x005F48B0` into a new one, null on failure) and buffers through `cSLBuffer`
  (`0x005F4870`, sl-library.md).

## 6. Function inventory

`cSC4RegionalTerrainMap`: `0x007AA2A0`, `0x007AA2E0`, `0x007AA2F0`, `0x007AA300`, `0x007AA310`, `0x007AA320`,
`0x007AA350`, `0x007AA3B0`, `0x007AA3C0`, `0x007AA430`, `0x007AA490`, `0x007AA4B0`, `0x007AA4C0`, `0x007AA4D0`,
`0x007AA4E0`, `0x007AA4F0`, `0x007AA500`, `0x007AA510`, `0x007AAAF0`, `0x007AACE0`, `0x007AAD30`, `0x007AAD60`,
`0x007AADF0`. `cSC4RegionalTerrainView`: `0x007AAB00`, `0x007AB600`, `0x007AB630`, `0x007AC7D0`.
`cSTEChangeTerrainUI`: `0x0048C1C0`, `0x00734B10`, `0x00734B60`, `0x00734B90`, `0x00734D00`, `0x00734D20`,
`0x00734D90`, `0x00734DF0`, `0x00735100`. `cSTEEdgeReconcileInformMismatch`: `0x00734E10`, `0x00735250`.
`cSTEEdgeReconcileInformNoMismatch`: `0x00734D30`, `0x00734FB0`, `0x00734FC0`, `0x00734FE0`.
`cSTEEdgeReconcileInformDamage`: `0x007350F0`, `0x00735120`. `cParser`: `0x0063BF30`, `0x0063BF50`, `0x0063BF70`,
`0x0063DDF0`. `cSC4FileTokenizer`: `0x006210B0`, `0x00621C30`, `0x00622620`. `cSC4TileNeighborhoodCompact`:
`0x00620DD0`, `0x00620EC0`, `0x00620F00`, `0x00620FC0`, `0x00620FF0`, `0x00621440`.
`cSC4LotConfigurationFilterMonopoly`: `0x004BE0C0`, `0x004BE130`, `0x004BEDD0`, `0x0072A080`, `0x0072A730`.
`cS3DBoundingBox`: `0x0046FFC0`, `0x005EDE00`, `0x00676CC0`, `0x007C2E50`. `LinearRandomIterator`: `0x005CF610`.
`FerryStationInformation`: `0x0056D770`, `0x0056D8B0`, `0x0056D920`. `cADVString`: `0x004178F0`, `0x00417A00`,
`0x00417A30`, `0x00603000`. `cSC4ScriptServiceResourceFactory`: `0x005F47C0`, `0x005F4800`, `0x005F4820`,
`0x005F4840`, `0x005F4860`, `0x005F4870`, `0x005F48B0`. `cSC4PrimitivePool`: `0x005EC4A0`, `0x005EC5D0`,
`0x005EC850`. `cS3DVertexArray`: `0x00800E80`, `0x00801040`. `cSC4TrafficSimulator::TrafficSourceIterator`:
`0x007107A0`, `0x007107F0`. `cSC4LotConfigurationIdentityInfo`: `0x0047D8F0`, `0x0047E5F0`, `0x0047F050`.
`cSC4TrafficSimulator::TransitSwitch`: `0x006D68B0`, `0x006D6C30`. `cTablePropertyReader`: `0x004084E0`.
