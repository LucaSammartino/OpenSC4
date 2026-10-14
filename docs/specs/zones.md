# Zones: zone manager (`cSC4ZoneManager`) and zone developer / lot manager (`cSC4ZoneDeveloper`) — behaviour spec

Status: M4 round 5, written from the Windows 1.1.641 executable (`bin/SimCity4.exe`). `docs/subsystems/development.md`
§7 (mostly Mac) was the starting point only; every fact below was checked on Windows. Scope: what growth depends on
(zone grid and queries, `PlaceZone`, the dezone on network insertion, parcellization into lots, the lot manager's
cell → lot map and lot lists, both save records, every slot of the interfaces). The zoning UI (drag colours, the
highlight preview, plopped lots) is described only as far as it shares state or random draws with growth.
Round 6 added the exact parcellization geometry: `Parcellize` (§7.6), `DrawStreet` (§7.10), `ExtendAsStreets`
(§7.11), `LayInJogStreets` (§7.12), the connection test (§7.13) and conformance tests 19–29.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated: streams and references (foundations §2, §2.7), class store and city
component table (§3, §3.6, §3.7), messages (§5), random generators (§6), slot-order rule (§16), framework slot
tables (§17), x87 precision (§18); grid classes, geometry, persistence and the ownership rule (`simgrid.md` §2–§6);
allocator instances A/B/C (`docs/abi/allocators.md`). Specified in parallel and cited for their side of each
contract: `building-development.md` (BDS), `growth-developer.md`, `tract-developer.md`, `lot-developer.md`.

Notation: `r24(e)` = one rounding to float32 (foundations §18); `ftol(e)` = the CRT float → int64 truncation toward
zero (`0x009EEF04`, mislabelled `_lua_gettop`); `rhe(e)` = round half to even, computed by the original as
`bits(r24(e + 12582912.0f)) − 0x4B400000` (exact for |e| < 2^22). `U(n)` = `cRZRandom::RandomUint32Uniform(n)`
(`0x009136B1`; foundations §6.1). A *rect* is four `int32` `{x1, z1, x2, z2}`, inclusive, in cells. *Sides* and lot
*facings*: 0 = −z, 1 = +x, 2 = +z, 3 = −x.

---

## 1. Role and identity

**Zone manager.** Owns the zone grid (one `int8` zone type per cell), the per-type zone counts and the zoning
tuning (minimum/maximum zone size, costs, textures, colours, maximum slope). `PlaceZone` is the single entry point
that changes zones: it checks, charges, writes the grid, demolishes incompatible lots, calls the zone developer to
re-parcel the cells, and posts *ZoningChange*. It also dezones the cells under every newly inserted network
occupant (§5.1). It has no simulator agent.

**Zone developer.** One object implementing two interfaces: `cISC4ZoneDeveloper` (parcellization: turning zoned
cells into empty lots, and the draft "highlight" preview) and `cISC4LotManager` (the cell → lot map, the master lot
list, lot creation and deletion, lot queries). Growth (BDS, tract developer, lot developer, growth developers) sees
zones only through the lots this object creates and through the zone manager's queries. It has no simulator agent.

| Item | Zone manager | Zone developer / lot manager |
|---|---|---|
| Class ID | `0x298F9B2D` (`GetGZCLSID` `0x004DC1A0`) | `0x498F9B01` (`GetGZCLSID` `0x00729EB0`) |
| City component slot (foundations §3.6) | 6 (`0x67652433`) | 7 (`0xE818ED2B`) and 8 (`0x681905BD`, same object) |
| Global pointer used by other code | `0x00B43D14` | `0x00B43DB0` (zone developer), `0x00B43D08` (lot manager, the `+4` sub-object) |
| Constructor | `0x004DCE50` | `0x007348D0` |
| `QueryInterface` | `0x004DC1B0` | `0x00729EF0` |

Interfaces (IID → sub-object). Every recognised IID AddRefs and returns AL = 1; any other returns 0.

| Class | IID | Interface | Sub-object | vtable |
|---|---|---|---|---|
| zone manager | `0x00000001`, `0x67652433` | `cIGZUnknown`, `cISC4ZoneManager` | +0 | `0x00A92E00` (21 slots + deleting destructor `0x004DCF40` as slot 21) |
| | `0x452294AA` | `cIGZMessageTarget2` | +4 | `0x00A92DF0` (4) |
| | `0xE4FDA3D4` | `cIGZSerializable` | +8 | `0x00A92DD8` (6) |
| zone developer | `0x00000001`, `0xE818ED2B` | `cIGZUnknown`, `cISC4ZoneDeveloper` | +0 | `0x00AB3830` (10 slots + deleting destructor `0x00734AD0` as slot 10) |
| | `0x681905BD` | `cISC4LotManager` | +4 | `0x00AB37C0` (28) |
| | `0x452294AA` | `cIGZMessageTarget2` | +8 | `0x00AB37AC` (4) |
| | `0xE4FDA3D4` | `cIGZSerializable` | +0xC | `0x00AB3794` (6) |

`Release` of the zone manager (`0x004DC430`): count > 1 → decrement; otherwise `Shutdown` then the deleting
destructor.

### 1.1 Zone manager `Init` (slot 3, `0x004DE170`) → AL

Idempotent (returns 1 at once when already initialised). In order:

1. `ReadSimulatorTunableValues` (§4).
2. Take the services that are still null, each AddRef'd: city (`0x00B43CEC`), occupant manager (`0x00B43D0C`),
   terrain (`0x00B43CF4`), pollution simulator (`0x00B43D9C`), budget simulator (`0x00B43D64`). Take the message
   server and subscribe to `kSC4MessageInsertOccupant` `0x99EF1142`.
3. If the city was **not** loaded from a save: `N := city.CellCountX × city.CellCountZ` (city slots 93, 94). For each
   zone type t in 0..15: `zoneCount[t] := 0`, `undeveloped[t] := N`, `unnamedPerType[t] := N`. Then
   `zoneCount[0] := N` and `undevelopedTotal := N`.
4. If there is no zone grid: construct an **`int8` grid directly** (C, 0x44 bytes; game class `0x49B9E603`, vtables
   `0x00A87950` / `0x00A87934`; simgrid §6.2), AddRef, `Init`, `SetTractSize(1)`, `SetTractValues(0)`. Its instance ID
   stays 0. A loaded grid (§10.1) is kept with its saved geometry.
5. Set the initialised flag; return 1.

Init order: the city initialises the zone manager after the occupant manager and before the zone developer
(foundations §3.7 *[inferred]*); its notification is therefore registered before the zone developer's.

`Shutdown` (slot 4, `0x004DC230`): if initialised, clear the flag, unsubscribe, release the message server, budget,
pollution, terrain, city, occupant manager, then `Shutdown` and `Release` the grid. Returns 1.

### 1.2 Zone developer `Init` (slot 3, `0x00734030`; lot manager slot 3 `0x007348C0` tail-jumps here) → AL

Idempotent. In order:

1. Re-point its lot developer reference to `0x00B43D20` (AddRef new, release old). Clear the cached 3D-view value.
2. Take, in this order: the 3D-view global `0x00B43DD0` (set by `SL::OnView3DInit` `0x00602590`; its slot 10 result
   is cached), BDS (`0x00B43D68`), city (`0x00B43CEC`) and its cell counts, occupant manager (`0x00B43D0C`), terrain
   (`0x00B43CF4`), traffic simulator (`0x00B43DA8`), zone manager (`0x00B43D14`), message server (`0x00B43CCC`).
   **If any is null, return AL = 0 with the initialised flag unset.** A host without a 3D view therefore cannot
   initialise the original zone developer *[compat]* (§13).
3. If the cell → lot map is empty (new city), size it to `cellsX` rows of `cellsZ` nulls. Otherwise (loaded city)
   call `lot.Init()` (lot slot 5) for every lot at its origin cell, x outer, z inner.
4. If there is no maximum-capacity grid: construct a **`uint16` grid directly** (C, 0x44 bytes, constructor
   `0x0045F9B0`, game class `0x49B9E604`), AddRef, `Init`, `SetTractSize(4)`, `SetInstanceID(0x69D5C456)`. It is not
   saved by this class (§10.2) *[inferred: no writer found in this class]*.
5. `InitBoundaryRules` (`0x0072FF80`; lot-boundary texture rules, UI).
6. Subscribe to `0x99EF1142` (InsertOccupant), `0x26D31EC1` (PostCityInit), `0xA6B79602` (unnamed view message).
7. Register the cheats "PlopAllLots" (`0x8929E68E`) and "PlopAllBuildings" (`0x4AB569C9`) with the cheat manager
   (`0x00B43CAC`) when the app's slot 21 (`0x00B43C94`) is true, and subscribe to the cheat message `0x230E27AC`.
8. Set the initialised flag; return 1.

`Shutdown` (slot 4, `0x007335A0`): unregister the cheats, unsubscribe the three messages, clear the boundary-rule
map, release the lots of the map and free it, `Shutdown`/`Release` the network tools and the maximum-capacity grid,
release the services. Returns AL = 1.

---

## 2. Zone types and the zone grid *[compat]*

| Value | Zone (SDK `ZoneType`) | `IsRCIZone` | Parcellized by |
|---|---|---|---|
| 0 | none | no | (dezone, §7.9) |
| 1, 2, 3 | residential low / medium / high | yes | `Parcellize` (§7.6) |
| 4, 5, 6 | commercial low / medium / high | yes | `Parcellize` |
| 7 | agriculture | yes | `SimpleParcellize(16)` (§7.5) |
| 8, 9 | industrial medium / high | yes | `SimpleParcellize(6)` |
| 10 | military | no | — |
| 11, 12, 13 | airport, seaport, spaceport | no | — |
| 14 | landfill | no | — |
| 15 | plopped (lots placed by the user) | no | — |

The **zone grid** is a `cISC4SimGrid<int8>` with tract size 1 (one value per cell), city-sized, instance ID 0, values
0..15 (−1 never written by this class). `PlaceZone` is its only writer in this class; it writes cells raw through
`GetGridData` (simgrid §6.3). `GetZoneType(x, z)` reads it raw with **no bounds check** and returns the value
sign-extended to all of EAX.

`IsRCIZone(t)` is true for 1..9 and false for 0 and 10..16 (table lookup); for t > 16 only AL is defined.

**Zone counts.** `zoneCount[t]` (slot 6) is the number of cells of type t. `PlaceZone` keeps it exact (§6). The
arrays `undeveloped[t]`, `unnamedPerType[t]` and the four scalars (`undevelopedTotal`, slot 8; an unnamed one;
`abandonedTileCount`, slot 10; an unnamed one) are written only by `Init` (new city) and `Read`: **they are never
updated** after `Init` *[quirk]*. Slot 7 therefore returns N for every type in a new city, slot 8 returns N, and
slot 10 returns 0.

---

## 3. State

### 3.1 Zone manager

| State | Initial (constructor / Init) | Persisted |
|---|---|---|
| zone grid (`int8`, tract size 1) | created in `Init` | by reference |
| `zoneCount[16]` (u32) | Init: `[0] = N`, others 0 | yes |
| `undeveloped[16]`, `unnamedPerType[16]` (u32) | Init: N each | yes |
| `undevelopedTotal`, `unnamed1`, `abandonedTileCount`, `unnamed2` (u32) | constructor 0; Init: `undevelopedTotal = N` | yes |
| min / max zone size `[16]` (s32), create / destroy cost `[16]` (s64), texture instance `[16]`, drag colour `[16]` (u32) | from the exemplar (§4); costs zeroed first; sizes uninitialised if the exemplar is missing | yes (but overwritten by the exemplar at every `Init`) |
| max slope `[16]` (f32) | from the exemplar | **no** |
| display alpha, default display alpha (u8) | constructor 0xFF; exemplar value for both | alpha yes |
| development-failure map 1 and map 2: `{u8 zone, u8 result} → u32 count` | empty | yes |
| member `cRZRandom` | constructed with seed `0xFFFFFFFF` (clock) | no; **never drawn** (§9) |
| service references | Init | by reference (§10.1) |
| initialised flag, loaded flag | false | no (loaded is set by `Read`) |

The failure maps are written only by `Read` and read only by slot 9 (map 1); map 2 is only saved. Slot 9's lookup
inserts a zero entry (§11.1), which then appears in the next save *[quirk]*.

### 3.2 Zone developer / lot manager

| State | Initial | Persisted |
|---|---|---|
| cell → lot map: `cellsX` rows × `cellsZ` lot pointers (one pointer per cell covered by a lot; no reference per cell) | Init | yes (a reference per cell) |
| master lot list (list of lot pointers, B nodes; holds the manager's one reference per lot) | empty | no (rebuilt by `Read`) |
| lot count | 0 | no (rebuilt by `Read`) |
| registered safe lot iterators (list, B nodes) | empty | no |
| maximum-capacity grid (`uint16`, tract size 4, instance `0x69D5C456`) | Init | no |
| options: `alternateOrientation`, `placeStreets`, `customZoneSize` | false, **true**, false | no |
| lot-size parameters: lot width, lot depth, min depth, max depth, block depth | 2, 2, 1, 3, 6 (overwritten by `DetermineLotSize` before every use) | no |
| orientation result: flag, facing side D, start side Q (§7.3), side scores S[4], side conditions C[4] | 0, 0, 0, —, — | no |
| current zone type (set during parcellization and dezoning) | 0 | no |
| real-mode flag (false only inside `HighlightParcels`) | true | no |
| restriction region (borrowed or owned pointer, §7) | null | no |
| highlight point, virtual-lot maps (created / deleted cells → `{x, z, facing}`) | empty | no |
| street, dirt-road and road network tools (from the network manager at PostCityInit) | null | no |
| boundary-rule map, cached view value (UI) | — | no |
| file-local `cRZRandom` `0x00B4B398` (shared by every instance) | static init (§9) | no |
| initialised flag, loaded flag | false | no |

---

## 4. Tuning *[compat]*

Exemplar "Zone Manager" `0xE9482490`, type `0x6534284A`, group `0xE7E2C2DB`, read by `ReadSimulatorTunableValues`
(`0x004DC460`) at every `Init`. The key is built as (type `0x6534284A`, group `0xE7E2C2DB`, instance `0xE9482490`)
(`0x004DC49B`–`0x004DC4AB`); `0xA52160F5` is the interface ID passed with it to the resource manager (slot 3,
`0x004DC4B6`), not a type (ZG1). The two cost arrays are zeroed first. Each array property is read as **exactly 16**
values through the variant's `Ref*` accessor, with no count or type check; a missing exemplar or property leaves the
array unchanged. Names from `re/tuning/simulators.md`.

| Property | Name | Type | Code default | Shipped (index = zone type 0..15) |
|---|---|---|---|---|
| `0xE94825B2` | kZM_mZoneMinCellSize | s32[16] | uninitialised | 0,1,1,1,1,1,1,4,1,1,1,1,1,1,2,1 |
| `0xE94825B3` | kZM_mZoneMaxCellSize | s32[16] | uninitialised | 32768,32,32,32,32,32,32,48,32,32,32,32,32,32,32,32 |
| `0xE94825B4` | kZM_mZoneCreationCostArray | s64[16] | 0 | 0,10,20,50,10,20,50,10,20,50,1,1,1,1,50,1 |
| `0xE94825B5` | kZM_mZoneDestructionCostArray | s64[16] | 0 | 0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1 |
| `0xE94825B6` | kZM_mZoneTextureResourceInstanceArray | u32[16] | — | 4121,4117,4118,4116,4099,4100,4098,4112,4113,4111,4114,4096,4120,4123,4101,4122 |
| `0xE94825B7` | Zone Display Alpha | u8 | 0xFF | 170 (copied to the default alpha) |
| `0xE94825B8` | Zone Drag Color | u32[16] | — | 4278190335,12845311,10092799,7471359,1333264383,542375935,337699583,4292555519,4289868799,3465551871,0,0,0,0,0,0 |
| `0xE94825B9` | Zone Max Slope | f32[16] | — | 256, then 16 × 15 |

No response curves are loaded by either class.

The zone developer reads no exemplar. Its lot-size parameters come from building and lot-configuration data
(§7.2).

---

## 5. Messages and scheduling

Neither class registers a simulator agent; both act only from method calls and message handlers.
`DoMessage` of both classes always returns AL = 1.

### 5.1 Zone manager `DoMessage` (`0x004DCA70`): dezone under new networks *[compat]*

Only `kSC4MessageInsertOccupant` `0x99EF1142` whose data3 is the network occupant type `0xC772BF98`:

1. occupant := void1, queried for `cISC4NetworkOccupant` (`0xA821EF94`).
2. If it has **none** of the network flags `0x80200090` (network occupant slot 9 `HasAnyNetworkFlag`; bits 4, 7,
   21, 31), take its bounding cells (occupant slot 11 `GetBoundingCityCells`), build a full region over that rect,
   and call `PlaceZone(region, 0, commit = 1, skipFunds = 1, checkSlope = 0, skipSize = 1, lotInside = 1, null, null,
   null)` (§6).

So every ordinary road, street, rail etc. dezones its bounding rectangle, free of charge, and the zone developer
re-parcels the remains (§7.9). This corrects `development.md` §2 (“a building lacking occupant group 0x80200090”).

### 5.2 Zone developer `DoMessage` (`0x007328D0`)

| Message | Action |
|---|---|
| `0x26D31EC1` PostCityInit | Take the terrain view (`SL::TerrainView`). From the network manager (`SL::NetworkManager` slot 5 `GetNetworkTool(type, 1)`) take the **network tools** of types 3 (street), 11 (dirt road) and 0 (road), each AddRef'd and its slot 3 called. If the city was loaded, call `lot.PostCityInit()` (lot slot 8) for every lot at its origin cell, x outer, z inner. |
| `0x230E27AC` cheat (ID = data1) | `0x8929E68E` → `PlopAllLotTypes` (`0x0072C8E0`); `0x4AB569C9` → `PlopAllBuildingTypes` (`0x0072BE50`). Debug only; not specified. |
| `0xA6B79602` | stores the u32 at `*void2` (used for highlight textures). |
| `0x99EF1142` InsertOccupant, data3 = `0xC772BF98` | see below |

InsertOccupant of a network occupant:

1. If the occupant is a `cISC4NetworkOccupant` with any flag of `0x80200090`, stop.
2. cell := `city.PositionToCell(occupant.GetPosition())` (occupant slot 8, city slot 95). If a lot covers that
   **one** cell: remember its rect (lot slot 14), facing (slot 23) and zone (slot 31); set the current zone type to
   that zone; `DeleteLot(lot)` (§8.6); if `IsRCIZone(zone)`, `CreateLotRemnants(rect, facing)` (§7.8); reset the
   current zone type.
3. Then by the message's data4 (message slot 24 *[inferred: GetData4]*): 1 or 2 → `0x00731F60(occupant)`; 3 →
   `0x00732450(occupant)`. Names *[inferred]*: street-joining helpers (connect a new road to adjacent streets /
   streets across rail). Not traced; they draw no random numbers (no RNG call in the class outside §9).

**Order with the zone manager.** Both classes handle the same synchronous InsertOccupant. The zone manager
subscribed first (§1.1), so its dezone (and the re-parcelling it triggers) runs before step 2 here *[inferred:
subscription order follows Init order, foundations §5.1, §3.7]*. Step 2 then usually finds no lot (it was deleted
by the dezone) unless the lot's zone is not dezoned by `OnDezone` (§7.9).

### 5.3 Messages posted

| Message | By | When | Fields | Delivery |
|---|---|---|---|---|
| ZoningChange `0x07699455` | zone manager `PlaceZone` | after a committed change with ≥ 1 accepted cell (§6) | void1 = IGZUnknown = zone manager; **void2 = a new `SC4CellRegion<long>`** (C, 0x24 bytes) whose rect is the request rect and whose bits are the changed cells; data3 = new zone type; data4 = `checkSlope` argument | `MessagePost(m, 0)` (queued) |
| DeferOccupantOperations `0x49D968C4` | zone developer `OnZone` (normal path) and `OnDezone` | at start (data1 = 1) and end (data1 = 0) | data1 | `MessagePost(m, 0)` |

The ZoningChange message is a subclass of the standard message (vtable `0x00A92D40`) whose destructor
(`0x004DCC30`) frees the void2 region with C. Consumers read the rect from the first 16 bytes of void2
(growth developers' `UpdateMaxCapacity`, `growth-developer.md` §6.1). Because it is **posted**, consumers see it on a
later message tick, after the zone developer has already re-parcelled (synchronously, inside `PlaceZone`).

DeferOccupantOperations is posted, not sent, so both halves arrive after the operation they bracket *[quirk;
inferred: the deferral window is empty]*.

---

## 6. `PlaceZone` (zone manager slot 11, `0x004DD780`) *[compat]*

Signature as compiled (pop 40, AL):
`bool PlaceZone(SC4CellRegion<long>& region, int32 type, bool commit, bool skipFunds, bool checkSlope,
bool skipSizeCheck, bool checkLotInside, int64* outCost, int32* outError, std::set<cISC4Lot*>* outLots)`.

`region` is a rect plus a bit per cell (§11.4). Error codes: 0 ok, 1 insufficient funds, 2 landfill on water,
3 zone too small, 4 zone too large, 5 blocked.

```
funds := budget.GetTotalFunds()                         (slot 6, int64)
cost := 0 (int64);  accepted := [];  demolish := []        (lists, insertion order)
if type ≥ 16 (unsigned): type := 0                        (the argument itself is replaced; ZG17)
if !skipSizeCheck:
    w := x2 − x1 + 1;  h := z2 − z1 + 1
    if w < min[type] or h < min[type]: error 3, ABORT before the first cell
    if w > max[type] or h > max[type]: error 4, ABORT before the first cell
for x in x1..x2 (outer), z in z1..z2 (inner), only cells inside the region's bitmap and set (ZG2):
    old := grid(x, z);  if old ≥ 16 unsigned: old := 0
    tile := city.GetTrafficNetwork().FindNetworkTile(x, z, 0x1F4F, 1)
    if tile and tile.AsOccupant().IsFlagSet(1) and type ≠ 14:
        if type ≠ 0: reject
        elif tile.IsOfType(3 /*street*/): cost += tile.GetCostOfDemolition()
    if terrain.CellHasWater(x, z):
        if type = 14: error 2, ABORT
        if type ∉ {0, 12}: reject
    if checkSlope and type ∉ {0, 15}:
        s := lotDeveloper.GetAreaSlope({x, z, x, z}, 1)   (float, ST0)
        if not (s ≤ maxSlope[type]): reject                (NaN rejects)
    lot := lotManager.GetLot(x, z, false)
    if no lot:
        lf := occupantManager slot 33 (cell (x, z), type 0x6A11BD2B landfill)
        if lf: if lf.CanRemove(2) accept else error 5, ABORT
        elif type = 14 and tile: error 5, ABORT
        else accept
    else:
        if checkLotInside and lot.GetBoundingRect() is not inside the request rect:
            if type = 14: error 5, ABORT else reject
        zt := lot.GetZoneType()
        if zt ∈ {11, 12, 15}: if type = 14: error 5, ABORT else reject
        cfg := lot.GetLotConfiguration()
        if cfg and (not cfg.IsCompatibleWithZoneType(type) or type = 0):
            b := lot.GetBuilding()
            if checkSlope and b and not b.CanRemove(2): reject
            add lot to demolish (unique)
        accept
    reject  = clear the cell's bit in the CALLER's region; continue
    accept  = append (old, x, z) to accepted; if old ≠ type: cost += create[type] + destroy[old]
after the loop (also after ABORT):
    if demolish ≠ []: cost += demolition.DemolishLots(false, demolish, checkSlope, 1, 0, 0, &c, 0) → c
    if outLots: insert every demolish lot into *outLots
    if type = 14: cost += city.GetLandfillDeveloper().GetZoningCost(region)       (int64)
    if skipFunds: cost := 0  elif cost > funds (signed int64): error 1, fail
if ok and commit:
    if not budget.WithdrawFunds(cost): error 1, fail
    if accepted ≠ []:
        changed := new region (C) with the request rect, all bits false
        for (old, x, z) in accepted order: set bit; grid(x, z) := type;
                                           zoneCount[old] −= 1; zoneCount[type] += 1   (also when old = type)
        demolition.DemolishLots(true, demolish, checkSlope, 0x2000, 0, 0, 0, 0)
        zoneDeveloper.DoParcellization(changed, type, checkSlope)                  (§7)
        post ZoningChange (void2 = changed, data3 = type, data4 = checkSlope)       (§5.3)
*outCost := cost (if given); *outError := code (if given); return AL = ok
```

Notes *[compat]*:

- "ABORT" stops the cell loop with the error set; the post-loop block still runs (cost, outLots), then the call
  fails. Cells accepted before the abort stay set in the caller's region; rejected cells are already cleared.
- **A failed size check is an ABORT too** (ZG17; `0x004DD82A` and `0x004DD846` jump to `0x004DDC31`, the abort
  path). The cell loop does not run, so `accepted` and `demolish` are empty and the region is untouched, but the
  post-loop block runs in full:
  - for type 14 the cost is `landfillDeveloper.GetZoningCost(region)` (`0x004DDCC8`–`0x004DDCF5`, landfill §4.3)
    over the whole request region; for every other type it stays 0;
  - with `skipFunds` the cost is set to 0;
  - otherwise a cost above the funds (signed int64, `0x004DDE34`) **replaces** error 3 or 4 with error 1
    (`0x004DE023`);
  - `*outCost := cost` and `*outError := code` are written as on every other exit (`0x004DE02B`–`0x004DE052`).

  The same holds after the other aborts (errors 2 and 5): the type-14 landfill cost of the whole region is added,
  and error 1 can replace the abort's code.

  Worked example (regress rzones): a 1 × 1 landfill (type 14) dry run with the size check on, `checkSlope` on,
  `commit = false`, `skipFunds = false`. `min[14] = 2`, so w = 1 < 2: error 3. No cell is examined. The cost is
  `GetZoningCost` of the 1 × 1 region (landfill §4.3, shipped `kCostPerCellHeightChange` = 4):
  - the cell lies in the rect of an existing landfill zone with level altitude 100 m, and the cell's altitude is
    91 m: `round(4 × |100 − 91|)` = **36**. A 52 m difference gives 208. This is the in-game range; it varies with the
    place and, month by month, with the zone's level and the terrain;
  - the cell touches no landfill zone: one cell has variance 0 ≤ 5 (`kLevelToleranceThreshold`), so the cost is 0.

  With the cost ≤ the funds, the call returns false with `*outError = 3` and `*outCost` = that cost; above the
  funds, `*outError = 1`. Other types write 0: a 1 × 1 agriculture zone (type 7, `min = 4`) fails with error 3 and
  `*outCost = 0`; a 1 × 1 residential zone (type 1, `min = 1`) passes the size check.
- **Type out of range** (ZG17, `0x004DD7F9`–`0x004DD807`): a type ≥ 16 is not a failure. The type argument is
  replaced by 0 and the call proceeds as a dezone, with the size limits of type 0 (corrects the earlier "fail
  (code 0)").
- **No city-bounds test** (ZG2, `0x004DD8B0`–`0x004DD913`). The loop skips a cell only when it lies outside the
  region's own bitmap (its width and height, unsigned offsets) or its bit is clear. It then reads the zone grid with
  no bounds check, and a commit writes it the same way. The size check limits the rect's size, not its position: any
  part of the rect outside the city is read, and on a commit written, outside the grid. A bitmap smaller than its
  rect skips the cells it does not cover; a bitmap with bad row pointers is dereferenced. *[choice]* reject such a
  call with error 0 before touching anything; the original has no such failure.
- The 5th argument (`checkAreaSlope` in the SDK) is used four ways: slope check, building-removal check,
  `DemolishLots` argument, `DoParcellization`'s street-removal flag (§7.9) and ZoningChange data4. It behaves as
  "this is a user action" *[inferred]*.
- Re-zoning a cell to its own type costs nothing and leaves the counts unchanged, but the cell still counts as
  accepted: a ZoningChange is posted and the developer re-parcels it.
- The demolition utility (`0x00B43D00`) is called twice; the first call (`false`) only prices the lots.
- The input region is modified (rejected bits cleared). `outLots` is a `std::set` of lots (B nodes); inserted
  elements are AddRef'd (allocator rows: §13.2).

Callers relevant to growth: the zone manager's own dezone (§5.1); the BDS `DoTerrainChanged`
(`building-development.md` §4.11: `PlaceZone(region, 0, true, kind ∈ {1, 2}, false, true, true, null, null, null)`).

---

## 7. Parcellization (zone developer) *[compat]*

### 7.1 Entry points

`DoParcellization(region, type, flag)` (slot 6, `0x00733FA0`, pop 12): set the current zone type := type. If
`zoneManager.IsRCIZone(type)`: clear the highlight state, then `OnZone(region, null)`. Otherwise
`OnDezone(region, flag)` (§7.9). Reset the current zone type to 0.

`HighlightParcels(region, type, point*, SC4CellRegion** out)` (slot 5, `0x00733F30`, pop 16): UI preview. It stores
the point, sets the real-mode flag false and sets the current zone type := type. If `IsRCIZone(type)`: clear the
highlight (below), `OnZone(region, out)` (§7.2), then `UpdateHighlightTextures(region)` (`0x0072DB60`). That last
call is UI: it walks the virtual-lot map and draws each entry through the terrain view, *[inferred]* changing no
simulation state. In every case it then sets real mode back to true, the type to 0 and the point to null. A non-RCI
type does nothing else: the highlight is not cleared and `*out` is not written. The virtual maps and the street
tool's drafts are **not** cleared at the end. They stay until the next clear.

**Clear the highlight** (`ClearHighlight`, `0x0072C0E0`). `DoParcellization` calls it too, in real mode.
- Empty the virtual-lot map (`+0xD8`) and the virtually-deleted map (`+0xE4`).
- Call `Reset()` (network tool slot 5) on the street tool (`+0x48`) and the road tool (`+0x4C`). The dirt-road tool
  (`+0x44`) is **not** reset.
- Call the terrain view's slot 9 (UI).

Both maps are keyed by cell. Each value is `{x1, z1, facing}` of the lot that put it there, and a later entry for the
same cell overwrites the earlier one.

**Highlight mode** (real-mode flag false) changes only these (ZQ4; test 33):

| Where | Real mode | Highlight mode |
|---|---|---|
| `CreateLotIfPossible` with code 8 (§8.5) | creates the lot | records a virtual lot on every cell, leaves `*out` null, AL = 1 |
| `DeleteLot` (§8.6) | deletes the lot | records its cells as virtually deleted; the lot stays |
| `CanCreateLot` (§8.4) | code 2 applies; code 7 = a real lot | no code 2; code 7 = a real lot not virtually deleted, or a virtual lot |
| `CreateParcellizableRegion` (§7.4) | lot test on every cell | lot test only where `IsCellOccupied` holds |
| `ExistsNetworkOfType`, `ExistsConnectionOfType`, `LayInJogStreets`' parallel test (§8.8, §7.13, §7.12) | real networks | also the street and dirt-road tools' drafts |
| `DrawStreet` (§7.10) | a street | a draft (`PlaceNetworkOccupants` with realMode false) |
| `Parcellize` (§7.6.2) | levels every rect, locks created lots | neither (no lot is returned) |
| `OnZone` step 7 (§7.2) | `0x0072B180` | skipped |

Everything else runs as in real mode. `ExamineCell`'s lot test sees only real lots (§7.3), both DeferOccupantOperations
messages are posted (§7.2), and random numbers are still **drawn** (§9) *[compat]*.

### 7.2 `OnZone(region, out)` (`0x00733770`)

**Custom-size path** (option `customZoneSize`): the region becomes the restriction region for
`ExamineBoundaries` (§7.3) and is then cleared as restriction; the orientation list is `{D, Q, (D + 2) mod 4,
(Q + 2) mod 4}`; `CreateLotToFaceRoad(x1, z1, w, h, list)` (§7.7) on the whole region rect (no draw: the list is not
empty). On failure with `out` given, `*out` := a new all-false region over the rect (C). On success `*out` is not
written. This path never looks at the highlight point.

**Normal path:**

1. Post DeferOccupantOperations (data1 = 1).
2. `CreateParcellizableRegion` (§7.4), cropped to its bounding box (`CompressRegion` `0x0072AF30`, new C region).
3. `DetermineLotSize` (§7.2.1).
4. If `placeStreets`: `ExtendAsStreets` (§7.11) for network kinds 11, 3, 10, 0, 6 (dirt, street, one-way,
   road, avenue) in that order; then, if the cropped region is wider and deeper than 2 cells, `LayInJogStreets`
   (§7.12) for the same kinds in the same order. Both receive the cropped region's rect, and the cropped region
   is the restriction region while they run, so their streets (§7.10) land only on its cells.
5. `SubdivideTheZone` (`0x0072EC60`): recompute the parcellizable region, split it into **4-connected components**
   (`ExtractConnectedCells` `0x0072CDA0`), each cropped and appended to a component list (B nodes; one empty region if
   there are none).
6. If `out` is given: `*out` := a new C region with the input region's rect and bits (`0x00733A1C`). That is the
   region as passed in, so cells that were not parcellizable (networks, water, kept lots) stay set.

   Then for each component in list order: it becomes the restriction region. With a highlight point P (set only by
   `HighlightParcels`), a component is processed only if it contains P or one of P's four neighbours. Each neighbour
   is tested only inside the city (x > 0, x < cellsX − 1, z > 0, z < cellsZ − 1). For a skipped component, its cells
   are cleared from `*out` (when given) and nothing else happens. A processed component gets `ExamineBoundaries`
   (§7.3) and then, by the current zone type:
   - 1..6 → `Parcellize` (§7.6);
   - 7 → `SimpleParcellize(16)`;
   - any other → `SimpleParcellize(6)` (§7.5).

   The component region is freed.

   After the loop, with both P and `out` given:
   - if P's bit in `*out` is set, `*out` := the 4-connected component of `*out` containing P, cropped
     (`ExtractConnectedCells`, `CompressRegion`);
   - otherwise `*out` := null.

   The previous `*out` is freed either way (`0x00733E02`–`0x00733E95`). That bit is read **without a bounds check**
   (`0x00733E1C`): for a P outside `*out`'s rect the original reads outside the bit array, giving garbage or a crash.
   *[choice]* FreeCity4 treats such a P as a clear bit, so `*out` := null.
7. In real mode, `0x0072B180(region)` (not traced; *[inferred: UpdateZoneType-like bookkeeping, no RNG]*).
8. Post DeferOccupantOperations (data1 = 0).

#### 7.2.1 `DetermineLotSize` (`0x00732BF0`)

Chooses the lot width/depth used by `Parcellize` from the developers that would build in this zone.

1. Centre `(cx, cz) := ((x1 + x2) / 2, (z1 + z2) / 2)` of the cropped region (C integer division).
2. Candidate developers (BDS slot 32 `GetDeveloper(type)`) by zone: 1–3 → `0x1010`, `0x1020`, `0x1030` (R$, R$$,
   R$$$); 4 → `0x3110`, `0x3120`, `0x3130`, `0x3320`, `0x3330` (CS$, CS$$, CS$$$, CO$$, CO$$$); 5–6 → the same five
   with the *shallow* flag set; other zones → none. The original does not null-check the results: each one is
   dereferenced at once (its vtable is read at `0x00732C72` for `0x1010`, and the same way for every other
   candidate), so a missing developer is a null read and the game crashes. The shipped BDS has all eight.
   *[choice]* (ZQ7) FreeCity4 skips a missing candidate. The first candidate that exists becomes the best, and later
   ones compete with it as step 3 says. If none exists, the zone takes the no-best path: stage 1, no query, an
   empty set, the (2, 2) default of step 7 and the no-developer row of step 8.
3. Best developer: the first candidate; a later candidate replaces it if its `EstimateDesirability(cx, cz)` (int16,
   `growth-developer.md` §4.6) is greater, or equal and `ftol(GetSourceDemand().QueryActiveDemandValue())` is greater
   (`demand.md` slot 12). Keep `bestDemand := ftol(best.demand value)`.
4. Stage := 1. If the best developer's growth/decline lists are valid (growth developer slot 10), walk its growth
   list (slot 11) in order: stop at the first item whose desirability-map tract value (slot 13, then
   `GetTractValue(item.x, item.z)`, int16, signed compare) is ≤ the best desirability; otherwise
   stage := `item.maxStage`. A walk that reaches the end keeps the last item's `maxStage`.
5. Lot configurations: lot configuration manager (`0x00B43D18`) slot 9 `GetLotConfigurationsByFilter` with a filter
   on the stack (vtable `0x00AB36FC`) holding `{purpose = best.GetPurpose(), wealth = best.GetWealth(), stage}`
   (growth developer slots 7, 8). Its `IsLotConfigurationIncluded(cfg)` (`0x0072A080`) is
   `cfg.IsCompatibleWithPurposeType(purpose)` (cfg slot 17) **and** `cfg.IsCompatibleWithWealthType(wealth)`
   (slot 16) **and** `cfg.GetGrowthStage()` (slot 11, u8) ≤ stage. That compare is signed, with the u8
   zero-extended (`0x0072A0B7`–`0x0072A0BD`, ZQ8). It differs from an unsigned compare only when stage is negative,
   which needs a negative `maxStage` in step 4. Then no configuration passes: with stage −1, a growth stage of 0
   fails (0 ≤ −1 is false), where an unsigned compare would accept all of them. The filter is never registered, so
   the manager takes its uncached path (`0x0048B2E7`): it walks all its configurations and inserts each one the
   filter accepts into a fresh STLport `hash_set` (hint 100 → 193 buckets), with one AddRef. No best developer → no
   query and an empty set.
6. For each configuration of the set (order below) that has a primary building object (cfg slot 24; null →
   skipped) and whose size `(sx, sz)` (cfg slot 7) satisfies `2 ≤ sz ≤ 3`, `sx ≤ 3` (unsigned) and (not shallow or
   `sz ≤ 2`): `min := 0x7FFFFFFF`, `max := 0x80000000`. For each ID of the object's ID list, in list order, a fresh
   hash set is filled by BDS slot 19 `GetBuildingTypesByFamily(set, ID)` (a family → its members, a building type
   → itself, anything else → nothing). For each type T in it, `P := GetBuildingProfile(T)` (slot 39); if
   `P.purpose = best.GetPurpose()` and `P.wealth = best.GetWealth()`, `min` and `max` take the u16 `P.capacity`
   (signed compares). If any was found:
   `k := (1 << s) << s` with s the desirability map's tract shift (slot 11; k = cells per tract);
   `v := ftol(r24(r24(float(k) / float(sx · sz)) × float((min + max) / 2)))` (integer division; the product is
   unsigned); `score := |v − bestDemand|`. Keep the configuration with the smallest score (strict `<`: first wins).
   The score is int32 arithmetic (ZQ9). The difference wraps mod 2³² (`0x007331AA`), and the absolute value is the
   two's-complement one (`0x007331AC`–`0x007331AF`), so |−2³¹| stays −2³¹. The best score starts at `0x7FFFFFFF`
   (`0x00732F77`), and a configuration is taken when its score is less, compared signed (`0x007331B1`). So a score of
   exactly `0x7FFFFFFF` is never taken: if every scored configuration scores that, none is chosen and step 7 uses
   (2, 2). A score that wraps to −2³¹ is taken and is never beaten. Example: bestDemand = −2147483643 and v = 5 give
   5 + 2147483643 = 2³¹, which wraps to −2³¹; that configuration is kept. bestDemand = −2147483647 and v = 0 give
   2147483647, which is never taken. Taking the first scored configuration always and skipping the wrap agrees
   with this whenever every |v − bestDemand| < 2³¹ − 1, which covers all real data. *[compat]* Implement the int32
   rule anyway, since it costs nothing.
   *[compat]* Implement the chain as written. For k ≤ 64 (tract size ≤ 8; the shipped 4 gives k = 16) it equals
   `⌊k · avg / (sx · sz)⌋` for every u16 `avg` (checked exhaustively), but not for larger k: with k = 256,
   `sx · sz = 3` and avg 49154 it gives 4194475 where the floor is 4194474, and k = 4096 (tract size 64, the
   developer's default when its tuning lacks `0x87CD6310`) already differs at avg 3074 (test 30g).
7. Lot width/depth := the chosen configuration's size `(sx, sz)` (lot configuration slot 7, first out-parameter =
   width), else (2, 2). `minDepth := maxDepth := depth`; then over **all** configurations of the hash set (not only
   those that passed the size filter of step 6), in hash order, with `width mod sx = 0` (unsigned), widen
   `[minDepth, maxDepth]` to include `sz` (unsigned compares; cfg slot 7 again). This walk releases every
   configuration of the set once. `width mod sx` is an unsigned `div` (`0x007332AA`), so a configuration in the set
   with `sx = 0` is a divide fault and crashes the original. The shipped configurations all have `sx ≥ 1`.
   *[choice]* FreeCity4 treats a configuration with `sx = 0` as failing step 6's size filter and skips it in this
   walk.
   If `maxDepth − minDepth < 2` (unsigned): `minDepth := (minDepth > 2) ? maxDepth − 2 : 1`; `maxDepth := minDepth + 2`.
   `minDepth > 2` is unsigned too (`cmp`, then `jbe` at `0x00733342`, ZQ10); with real depths (1–3) the signedness
   does not matter.
8. Block depth by zone density (`0x00458960`: zones 1, 4 → 1; 2, 5 → 2; 3, 6, 7, 8, 9 → 3; any other → 0) when a
   best developer exists and its purpose (growth developer slot 7) is 1 (residential): density 1 → `3 · depth`,
   2 → `ftol(r24(float(depth) × 2.5))` (depth converted as unsigned; 2 → 5, 3 → 7, 4 → 10), 3 → `2 · depth`.
   Otherwise (no best developer, or another purpose): 1 → 7, 2 → 6, 3 → 5. Density 0 leaves the block depth
   unchanged (unreachable for zones 1–9).

No random draws.

**Configuration order.** The configuration set is keyed by the configuration **pointer**. An element goes into bucket
`(ptr >> 2) mod 193`, at the head of its chain (`0x0072BA50`), and iteration runs through the buckets in ascending
order, each chain from its head. The original's order is therefore heap-address order and cannot be reproduced from
the data. (An earlier version of this section said it depended only on the configuration IDs; that was wrong.) It
is observable only through step 6's tie, where two configurations with the same score meet and the first wins. Steps
7 and 8 do not depend on it, and neither do the min and max over each inner type set. *[choice]* FreeCity4 visits the
configurations in **ascending configuration ID** (cfg slot 3 `GetID`); test 30e pins it.

### 7.3 `ExamineBoundaries` (`0x00731C80`) and `ExamineCell` (`0x0072FAB0`)

Scores the four sides of the restriction region R to decide which side lots face.

- Scores `S[0..3] := 0`; conditions `C[0..3] := placeStreets ? 0 : 1`.
- The outside rect is R's rect grown by one cell, but **not past the city edge** (at the edge the "outside" coordinate
  stays on R's own edge). Checked on the bytes (`0x00731C8D`–`0x00731CC8`): `x1' := x1 > 0 ? x1 − 1 : x1`,
  `x2' := x2 < cellsX − 1 ? x2 + 1 : x2`, and the same for z.
  At the border the cell examined is therefore R's own edge cell. In real mode that is a cell of R, so it is zoned
  and has no network, lot or water. It only gets the `C = 0` bonus: +1 with `placeStreets`, nothing without, exactly
  like an empty cell outside. A border side is neither skipped nor scored as an obstacle (test 31).
- For x in x1..x2: if `(x, z1) ∈ R` → `ExamineCell(x, z1', side 0)`; if `(x, z2) ∈ R` → `ExamineCell(x, z2', side 2)`.
  Then for z in z1..z2: if `(x1, z) ∈ R` → `ExamineCell(x1', z, side 3)`; if `(x2, z) ∈ R` → `ExamineCell(x2', z,
  side 1)`. (Primes are the grown coordinates.)
- `ExamineCell(x, z, side)`:

| Cell content (first match; network tests with `ExistsNetworkOfType`) | C[side] | S[side] |
|---|---|---|
| network of `0x104` (highway, light rail) | := 2 | += 5 |
| network of `0x808` (street, dirt road) | := max(C, 1) | += 30 |
| network of `0x401` (road, one-way) | := max(C, 1) | += 100 |
| network of `0x40` (avenue) | := max(C, 1) | += 400 |
| network of `0x2` (rail) | := 2 | += 5 |
| other network of `0x1F4F` | — | — |
| no network: a lot on the cell, or water | := 2 | += 5 |
| no network: each occupant in the cell (occupant manager slot 41, all types) of type `0x278128A0` (building) or `0x2890D4DE` (unnamed) | := 2 | += 5 each |
| then, in every case: if C[side] = 0 | | += 1 |

- In highlight mode the network tests also see drafts (§8.8), but in practice only street drafts. Every test whose
  flags include the dirt-road bit (`0x1F4F`, `0x808`) also includes the street bit, so it asks only the street tool.
  A street draft therefore scores as a street (+30). The lot test uses the **real** lot only (`GetLot(x, z, false)`,
  `0x0072FBAC`), so a virtual lot is not an obstacle and a virtually deleted real lot still is.
- **D** := argmax S (first of equal maxima). **Q** := for D ∈ {0, 2}: `S[1] ≤ S[3] ? 3 : 1`; for D ∈ {1, 3}:
  `S[0] > S[2] ? 0 : 2`. flag := 1.
- If `placeStreets` and the current zone is 1 or 2 and R is wider and deeper than 2: flag := 0 and swap D ↔ Q.
- If `alternateOrientation`: flag := not flag, and swap D ↔ Q.

### 7.4 `CreateParcellizableRegion` (`0x0072CB90`)

New C region over the input rect, all false. For each set input cell:
skip if (current zone ≠ 0 and `ExistsNetworkOfType(x, z, 0x1F4F)`) or `IsCellWater(x, z)`. Otherwise keep := true;
if (real mode or the cell is occupied per §8.4) and a lot is on the cell: with the lot's configuration present and
`cfg.IsCompatibleWithZoneType(current zone)` → keep := false (the lot survives); without a configuration, if the
current zone and the lot's zone are both in 1..6 → keep := false; otherwise **`DeleteLot(lot)`** and keep := true.
Set the bit if keep.

So zoning over an existing compatible lot keeps it; an incompatible or empty-configuration lot is deleted and its
cells are re-parcelled.

**The "occupied" predicate (ZQ2, from the exe).** The parenthetical is not a `CanCreateLot` code. It is the zone
developer's own per-cell test `IsCellOccupied(x, z)` (`0x0072C050`, pop 8, AL), called only in highlight mode
(`0x0072CCB1`). In real mode the lot test runs directly. The test, in order:

1. **Highlight mode**, with the cell in the virtually-deleted map (`+0xE4`): go to step 3.
2. A **real** lot on the cell (lot manager slot 6, `GetLot(x, z, false)`) → true.
3. **Highlight mode**, with the cell in the virtual-lot map (`+0xD8`) → true. Otherwise false.

In real mode the result is "a real lot is on the cell". In highlight mode it is "a real lot that is not virtually
deleted, or a virtual lot". This is exactly the cell condition of code 7 in §8.4, **without** the earlier codes 0–3.
So `CanCreateLot(x, z, 1, 1, …) == 7` is not equivalent. In highlight mode, the only mode where the test is called,
it also fails for cells that code 0 catches first (outside the city, or outside a set restriction region), and for
code-3 network cells when the current zone is 0. Code 2 applies only in real mode. Code 1 (water) cannot arise here,
because water cells are skipped first.

The lot that the cell test then examines is always the **real** lot: `GetLot(x, z, false)`, which ignores both
highlight maps. Consequences in highlight mode:

| Cell | `IsCellOccupied` | Real lot | Result |
|---|---|---|---|
| nothing | false | — | keep, bit set |
| real lot, not virtually deleted | true | the lot | the configuration test above. A failing lot is "deleted", which only records a virtual deletion (§8.6) |
| real lot, virtually deleted, no virtual lot | false | — | keep, bit set (the lot is not examined) |
| virtual lot only | true | null | keep, bit set |
| virtually deleted real lot **and** a virtual lot | true | the real lot | the real lot is examined again, and may clear the bit or be virtually deleted a second time *[quirk]* |

### 7.5 `SimpleParcellize(n)` (`0x0072E9B0`): agriculture (n = 16) and industry (n = 6)

1. If `placeStreets`: street lines across the restriction region's rect. The first pass walks x-lines, the second
   z-lines, each from a start edge in a direction chosen by (Q, D):

| Q | x lines (start, direction) | z lines (start, direction) |
|---|---|---|
| 0 | D = 3: x1, + ; else x2, − | z1, + |
| 1 | x2, − | D = 0: z1, + ; else z2, − |
| 2 | D = 3: x1, + ; else x2, − | z2, − |
| 3 | x1, + | D = 0: z1, + ; else z2, − |

   A counter starts at 0 per pass. On each line: if counter ≥ n and the line is ≥ 2 cells from both rect edges,
   draw a street (`DrawStreet` §7.10, whole line across the rect, perCell = 0) and reset the counter; otherwise
   counter += 1. So streets are separated by n lot lines.
   The table was checked on the bytes: Q is `+0xCC` and D is `+0xD0`, as `ExamineBoundaries` stores them
   (`0x00731ECE`), and the jump table is at `0x0072EC44`. An x-line street is `DrawStreet(x, z1, x, z2, 0)` and a
   z-line street is `DrawStreet(x1, z, x2, z, 0)`, so both are drawn from the low edge whatever the pass direction;
   only the positions of the lines show the direction (test 32).
2. For **every cell of the restriction rect**, x ascending (outer), z ascending (inner), regardless of the region's
   bits: `CreateLotToFaceRoad(x, z, 1, 1, emptyList)` (§7.7) → **one random draw per cell** (§9). The created lot's
   extra reference is released. Cells that cannot hold a lot (not zoned with the current zone, street, water, lot …)
   still consume their draw.
3. Free the restriction region.

Agricultural and industrial zones therefore always become 1×1 lots; growth later aggregates them
(`tract-developer.md` §4.15).

### 7.6 `Parcellize` (`0x0072EDD0`): residential and commercial

Inputs: the restriction region R (one component, §7.2 step 6) with rect `{x1, z1, x2, z2}`; D, Q, flag and C[4]
from `ExamineBoundaries` (§7.3; D and Q are always perpendicular); the lot-size parameters of §7.2.1, written here
`Lw` (lot width), `Ld` (lot depth), `minDepth`, `maxDepth`, `block` (block depth); `ps` := `placeStreets` as 0/1.
All arithmetic is `int32`; comparisons marked *u* are unsigned 32-bit. No random draws (§9).

#### 7.6.1 Frame

Lots face side D (or its opposite) and are laid **back to back along the depth axis**, starting at side D; one
such strip of lots is a **row**, `Lw` cells wide. Rows are laid side by side along the row axis, starting at side
Q. The switch is on Q:

| | Q ∈ {0, 2} | Q ∈ {1, 3} |
|---|---|---|
| depth axis (coordinate p), range `[a1, a2]` | x, `[x1, x2]` | z, `[z1, z2]` |
| row axis (coordinate o), range `[b1, b2]` | z, `[z1, z2]` | x, `[x1, x2]` |
| lot rect at (p, o) with depth size n | x `p..p+n−1`, z `o..o+Lw−1` | x `o..o+Lw−1`, z `p..p+n−1` |

Depth walk, by the position of D on the depth axis:

| | D ∈ {0, 3} (low side) | D ∈ {1, 2} (high side) |
|---|---|---|
| start `p0` | `a1` | `a2 − Ld + 1`, **replaced by 0 when negative** *[quirk]* |
| step `sp` | `+Ld` | `−Ld` |
| keep while | `a1 ≤ p ≤ a2` | `a1 − Ld + 1 ≤ p ≤ p0` |
| back-street offset `j` | `+Ld` | `−1` |
| remaining depth `rem` at p | `a2 − p + 1` | `p − a1 + Ld` |

Row walk, by the position of Q on the row axis:

| | Q ∈ {0, 3} (low side) | Q ∈ {1, 2} (high side) |
|---|---|---|
| start `o0` | `b1` | `b2 − Lw + 1` |
| step `so` | `+Lw` | `−Lw` |
| keep while | `b1 ≤ o ≤ b2` | `b1 − Lw + 1 ≤ o ≤ o0` |
| cross-street offset `k` | `+Lw` | `−1` |

Further constants: front facing `A := D`, back facing `B := (D + 2) mod 4`; `first := (C[D] = 1) ? A : B`;
`farRoad := (C[B] = 1)`; `Lrow := b2 − b1 + 1`; `Ldepth := a2 − a1 + 1`.

**Middle row.** Let `n` be the number of row positions `o0, o0 + so, o0 + 2·so, …` that satisfy the row keep
condition (counted until the first that does not; 0 if `o0` itself fails). `mid := o0 + (⌊n/2⌋ − 1)·so` when
`⌊n/2⌋ ≥ 1`, else `mid := o0`. (Example: `o0 = 10`, `so = +2`: range 10..15 gives n = 3, mid = 10; 10..17 gives
n = 4, mid = 12; 10..21 gives n = 6, mid = 14.)

#### 7.6.2 Lot loop

```
acc := 0;  backStreet := false;  locked := empty list
o := o0
while o satisfies the row keep condition:
    f := first;  p := p0
    while p satisfies the depth keep condition:
        n := Ld;  δ := 0
        if rem(p) > 0:                                             (always true in practice)
            n' := DepthRule(rem(p), f, farRoad)                    (§7.6.3)
            if Ldepth ≤u maxDepth and Lrow <u ps + 2·Lw:  n' := Ldepth
            if n' ≠ Ld:  δ := n' − Ld;  n := n';  if sp < 0: p := p − δ
        rect := lot rect at (p, o), depth size n                    (§7.6.1)
        if not lotManager.CreateLotIfPossible(rect, facing f):     (§8.5)
            CreateLotRemnants(rect, f)                              (§7.8)
        if δ ≠ 0 and sp > 0:  p := p + δ
        if f = A:  f := B
        else:      f := A
                   if ps:
                       s := p + j
                       if a1 ≤ s ≤ a2:
                           if o = o0:                               (drawn during the first row only)
                               DrawStreet(row-axis line at depth s, from b1 to b2,
                                          perCell = (s = a1 or s = a2))                   (§7.10)
                               backStreet := true
                           p := p + 1 if sp > 0 else p − 1          (every row skips the street line)
        if real mode:  lotDeveloper.AttemptAreaLevelling(rect clamped to the city)
        if a lot was created:  lock its vertices;  append it to locked
        p := p + sp
    if ps:
        acc := acc + Lw
        force := flag and backStreet and Lrow ≤u block + Lw + 1 and o = mid
        if acc ≥u block or force:
            t := o + k
            if force or (b1 + Lw ≤u t and t ≤u b2 − Lw):
                acc := 0
                DrawStreet(depth-axis line at row coordinate t, from a1 to a2,
                           perCell = (t = b1 or t = b2))
                o := o + 1 if so > 0 else o − 1                      (the next row starts after the street)
    o := o + so
for each lot in locked, in order:  unlock its vertices;  release the reference
```

Details *[compat]*:

- The keep conditions are re-tested after every step, so the extra ±1 for a street only shifts the walk; a lot
  position that fails the keep condition ends the row.
- When the depth size changes on a high-side walk (`sp < 0`), p moves by −δ first, so the lot keeps its far edge
  at `p + Ld − 1` (the edge nearer to side D) and grows or shrinks towards the interior. On a low-side walk the lot
  starts at p and p is advanced by δ afterwards. Either way the next lot starts right after this one.
- `CreateLotIfPossible` is lot manager slot 22 with the attempted rect, facing f, out lot, and the remaining
  arguments 0, 0, null (§8.5); it draws no random numbers. On failure `CreateLotRemnants` (§7.8) covers the
  buildable cells of the rect with 1-cell-wide strips facing f; for zones 1–6 it passes the explicit facing list
  `{f}`, so `CreateLotToFaceRoad` makes **no** `U(4)` draw here.
- The levelling call (lot developer slot 21, `lot-developer.md`) runs **after** the attempt, its remnants and the
  back street, for every attempt including failed ones, on the attempted rect clamped to
  `[0, cellsX − 1] × [0, cellsZ − 1]` (city slots 93, 94), with arguments `(rect, 0.0f, true, true,
  FLT_MAX (0x7F7FFFFF), null lot)`. The clamped rect can include cells outside the zone. Not called in highlight
  mode.
- Created lots get `SetVerticesAreLocked(1)` (lot slot 69) right after their levelling and keep the lock until the
  whole region is done, so later levelling in the same region does not move their corners; then each is unlocked
  (0) in creation order and released.
- The lot's row-axis size is always `Lw`: a row that overhangs the region (`Lrow` not a multiple of `Lw`, or the
  narrow arm of an L) fails with code 0 (§8.4) and falls back to remnants. The depth size is adapted (§7.6.3) so
  that the last lot of a row ends at the region edge or one cell short of it (case (c), leaving room for a street).
- Lots are created in row order and, within a row, in depth order; remnant lots appear at the point of the attempt
  that produced them. This is the order of the `CreateLot` calls; where each lot lands in the master list is
  decided by `CreateLot` (§8.5).
- The back street is drawn once, during the first row, across the whole row range `b1..b2`; later rows only skip
  its line. The depth positions of all rows are identical because `rem`, `f` and the adjustments do not depend on
  o.
- New streets are visible to `CanCreateLot` at once (code 3), so a later attempt overlapping a street fails and
  falls back to remnants *[inferred: `PlaceNetworkOccupants` inserts the street occupants before it returns; in
  highlight mode they are drafts, seen through §8.8]*. Cells under new streets stay zoned until the posted
  InsertOccupant message of each street occupant reaches the zone manager, which dezones them (§5.1) later.
- `flag` (§7.3) matters only for the forced middle cross street. It is 0 for zones 1–2 with `placeStreets` and a
  region wider and deeper than 2 cells (§7.3), so low and medium density residential gets cross streets only every
  `block` cells of rows.
- The cross-street range test is unsigned, so when `b2 < Lw` the upper bound wraps to a huge value and the test
  can pass where a signed test would fail, and a negative t (`k = −1` at `o = 0`) fails *[quirk]*. The street
  itself is still clipped to R by `DrawStreet`.
- The start clamp `p0 := max(0, a2 − Ld + 1)` for a high-side D changes the layout of a region whose depth range
  lies within the first `Ld − 1` cells of the map: the walk then starts at 0 instead of a negative position, and
  the extra lot positions it produces are tried (test 26) *[quirk]*.
- The forced override `n' := Ldepth` applies when the whole depth range fits in one lot (`Ldepth ≤ maxDepth`) and
  the row range is narrower than `ps + 2·Lw`; then every lot of the row spans the full depth.

#### 7.6.3 `DepthRule(rem, f, farRoad)`

`m := minDepth`, `M := maxDepth`, `P := Ld`, `s := ps`. The first matching line wins; all compares *u*; `/` is
integer division (all operands positive).

| Case | Result |
|---|---|
| **(a)** `farRoad` and `f = A` | `rem ≤ M` → rem; `rem < 2m` → rem; `rem < 2M − 1` → `(rem + 1)/2`; `rem < s + 3m` → `(rem + 1)/2`; `rem ≥ s + 3P` → P; else `(rem + 1)/3` |
| **(b)** `farRoad` and `f = B`, or not `farRoad` and `f = A` | `rem < s + 2m` → rem; `rem ≥ s + 2P` → P; `rem ≤ M` → rem; else `rem/2` |
| **(c)** not `farRoad` and `f = B` | `rem < s + 2m` → (`rem > 1` ? `rem − 1` : P); `rem ≥ s + 2P` → P; `rem − 1 > M` → `rem/2`; else (`rem > 1` ? `rem − 1` : P) |

Case (c) leaves one cell for the back street when no road lies behind the block. A result equal to P means "no
change". Test 27 tabulates the function.

### 7.7 `CreateLotToFaceRoad(x, z, w, h, list, out)` (`0x0072D200`)

1. If the orientation list is empty: **r := U(4)** on the file-local generator (§9), then the list :=
   `r, r+1, r+2, r+3` (mod 4) (B nodes). This happens on entry, before any test.
2. For each orientation o in list order, test the cells just outside side o for a network of `0xC49` (road,
   street, avenue, one-way, dirt road) with `ExistsNetworkOfType`; the side must lie inside the city:

| o | requires | cells tested |
|---|---|---|
| 0 | z > 0 | (x..x+w−1, z−1) |
| 1 | x+w−1 < cellsX−1 | (x+w, z..z+h−1) |
| 2 | z+h−1 < cellsZ−1 | (x..x+w−1, z+h) |
| 3 | x > 0 | (x−1, z..z+h−1) |

   At the first side with any such cell: `CreateLotIfPossible(x, z, w, h, o, out, 0, 0, null)` and return its result.
3. No side has a road: `CreateLotIfPossible(x, z, w, h, list.front(), out, 0, 0, null)`.

### 7.8 `CreateLotRemnants(x, z, w, h, facing)` (`0x0072D440`)

Re-creates lots on the cells of a deleted lot. By the **current zone type**:

- 7, 8, 9: every cell, z outer, x inner: `CreateLotToFaceRoad(cell, 1, 1, emptyList)` → **one draw per cell**.
  Returns 1.
- otherwise: strips one cell wide that run along the facing direction (facing 1 or 3: a strip is `run × 1`
  cells on one z row; facing 0 or 2: `1 × run` cells on one x column), each made with
  `CreateLotToFaceRoad(strip, {facing})` (no draw; the strip faces `facing` whether or not that side has a road,
  §7.7). Scan order by facing: 3 → rows of z, x ascending; 1 → rows of z, x descending; 0 → columns of x, z
  ascending; 2 → columns of x, z descending. A run of consecutive buildable cells (inside the city and the
  restriction region, not water, zoned in real mode, no `0x1F4F` network, not occupied) becomes one lot.

### 7.9 `OnDezone(region, flag)` (`0x0072E6A0`)

1. Post DeferOccupantOperations (data1 = 1).
2. For each set cell, z outer, x inner, with a lot: current zone := the lot's zone; if `IsRCIZone` or 11 or 12:
   `DeleteLot`, then `CreateLotRemnants(lotRect, lotFacing)` (§7.8). Since the grid already says 0, no lot is
   recreated on dezoned cells, but zones 7–9 still consume one draw per cell of the deleted lot *[compat]*.
3. If `flag`: collect the set cells that have a `0x808` network (street, dirt) and no `0x1747` network into a region;
   if any, `demolition.DemolishRegion(1, &region, 1, 1, 0, 0, 0, 0, 0, 0, 0)` (demolition slot 6, 11 arguments).
4. Post DeferOccupantOperations (data1 = 0).

### 7.10 `DrawStreet(x1, z1, x2, z2, perCell)` (`0x0072A2A0`, pop 20)

Draws an automatic street along one cell line from `(x1, z1)` to `(x2, z2)` (the callers always pass a horizontal or
vertical line; the end may be below or above the start), but only on cells of the current restriction region R.
`u` := the unit step from start to end (per axis: sign of the difference). "Place(a, b)" is the street tool's
`PlaceNetworkOccupants(a.x, a.z, b.x, b.z, 0xC49, callback = the street tool, realMode, −1)` (network tool slot 7,
pop 32, bool in AL; §12): one street segment from a to b. In highlight mode the street is a draft.

1. **Single cell** (start = end): if the cell is in R, Place(cell, cell). Done.
2. Otherwise c := start, and repeat:
   - Skip cells not in R (advance c by u while `c ∉ R` and c ≠ end).
   - **c = end**: if `end ∉ R` stop. Else try, in order, until one succeeds: Place(end, end); Place(end + u, end);
     Place(end, end − u); Place(end + u, end − u). Stop.
   - Else the **run** is the maximal set of consecutive in-R cells from c towards the end: `[c, r]`.
   - Unless `perCell` and the run is a single cell: try Place(c, r); on failure Place(c + u, **end**); on failure
     Place(c, **end − u**); on failure Place(c + u, end − u) *[quirk: the fallbacks use the line's end, not the
     run's end, so they can lay street on cells outside R]*.
   - If `perCell`: Place(cell, cell) for every cell of the run, in order (after the segment, so a segment and the
     single-cell placements overlap; their only purpose is to add the cells one by one when the segment fails or
     is a draft) *[inferred purpose]*.
   - If r = end stop; else c := r + u and repeat.

Return values of Place are only used for the fallback chain; `DrawStreet` returns nothing. The callers' choice of
`perCell`: `Parcellize` sets it when the line lies on the region's outer edge (§7.6.2), `LayInJogStreets` always,
`ExtendAsStreets` and `SimpleParcellize` never.

### 7.11 `ExtendAsStreets(rect, k)` (`0x00730BA0`, pop 8)

Prolongs every dead end of network kind k that lies inside the region's rect across the region. `flags := 1 << k`
(table `0x00AB3680`: kind k ↦ network flag bit k). For every cell (x, z) of the rect, x ascending (outer), z ascending
(inner), including cells not in R:

1. Skip unless `ExistsNetworkOfType(x, z, flags)`.
2. Count the connected sides: for side s = 0, 1, 2, 3 (facing sides: 0 = −z, 1 = +x, 2 = +z, 3 = −x), the side
   counts once if `ExistsConnectionOfType(x, z, flags, s, t)` (§7.13) holds for **any** t of road (0), one-way (10),
   street (3), dirt road (11), or avenue (6) — avenue only when `x + z` is odd *[quirk]*. Stop counting at 2.
3. If exactly one side d connects, the cell is a dead end; draw a street from the cell to the
   rect edge in that direction, `perCell = 0`:

| d | street | skipped when |
|---|---|---|
| 0 (−z) | (x, z) → (x, z2) | z = z2 |
| 1 (+x) | (x, z) → (x1, z) | x = x1 |
| 2 (+z) | (x, z) → (x, z1) | z = z1 |
| 3 (−x) | (x, z) → (x2, z) | x = x2 |

The dead-end cell itself is not in R (it holds a network), so `DrawStreet` starts at the next cell. Cells scanned
later see streets drawn earlier in the same scan *[inferred: synchronous insertion, §7.6.2]*; a fully extended
street ends on the rect edge, where the table above skips it.

### 7.12 `LayInJogStreets(rect, k)` (`0x00730D30`, pop 8)

Connects dead ends of kind k that stop just **outside** the region by laying a street along the region's edge.
`flags := 1 << k`. The four outside lines are processed in this order, each only when it lies inside the city:

| # | line (outside cells) | condition | wanted dead-end side | parallel edges | street drawn |
|---|---|---|---|---|---|
| 1 | z = z1 − 1, x = x1..x2 | z1 > 0 | 0 (−z) | −x, +x | (x1, z1) → (x2, z1) |
| 2 | z = z2 + 1, x = x1..x2 | z2 < cellsZ − 1 | 2 (+z) | −x, +x | (x1, z2) → (x2, z2) |
| 3 | x = x1 − 1, z = z1..z2 | x1 > 0 | 3 (−x) | −z, +z | (x1, z1) → (x1, z2) |
| 4 | x = x2 + 1, z = z1..z2 | x2 < cellsX − 1 | 1 (+x) | −z, +z | (x2, z1) → (x2, z2) |

Per line: `dead := false`, `parallel := false`. For each outside cell in ascending order that has a network of kind
k (`ExistsNetworkOfType(flags)`):

- Count its sides s = 0..3 with `ExistsConnectionOfType(x, z, flags, s, k)` (§7.13), stopping at 2. If the count is
  1 and that side is the wanted side (the network comes from outside and stops at the region): `dead := true`
  (sticky for the line).
- Parallel test: for each kind in the order avenue (flag `0x40`, type 6), road (`0x1`, 0), one-way (`0x400`, 10),
  street (`0x8`, 3), dirt road (`0x800`, 11): if the cell has that network, take it (`GetNetwork`, §7.13) and ask
  `ConnectsToNeighbor` for either of the two parallel edges with that type. Any yes: `parallel := true` and the
  scan of this line stops. "Has that network" is `ExistsNetworkOfType` (§8.8), so in highlight mode a street or
  dirt-road draft passes it. The edge test differs by kind:
  - avenue, road and one-way: real network only;
  - street and dirt road: §7.13's `ExistsConnectionOfType` written out inline, highlight-mode branch included
    (for example `0x00730F25`–`0x00730FD6` for the street and `0x0073101A`–`0x007310C6` for the dirt road in the
    first line's scan). When the real occupant does not connect on the edge, ask the street tool, then the
    dirt-road tool, and test byte `edge` of the mask. The type is ignored.

  An earlier version of this section said the parallel test used the real network only. That was wrong.

After the scan, if `dead` and not `parallel`: `DrawStreet(street drawn, perCell = 1)`. So a jog street is laid when
some dead end points at the region and no network cell on that line runs along the region edge.

### 7.13 `ExistsConnectionOfType(x, z, flags, side, type)` (`0x0072FE80`, pop 20, AL) and `GetNetwork` (`0x0072FCA0`, pop 12)

`GetNetwork(x, z, flags)`: the first occupant in the cell (occupant manager slot 41, type `0xC772BF98` = network
occupant, list order) whose `HasAnyNetworkFlag(flags)` is true; null if none. No AddRef (the list is released, the
pointer stays valid because the occupant manager keeps a reference).

`ExistsConnectionOfType`: map the facing side to a network edge `e := (side + 1) mod 4` (network edges: 0 = −x,
1 = −z, 2 = +x, 3 = +z; a side outside 0..3 gives e = −1). If `GetNetwork(x, z, flags)` is non-null and its
`ConnectsToNeighbor(e, type)` (network occupant slot 47, `+0xBC`, pop 8, AL: true when the occupant is of network
type `type` and has a connection on edge e for that type) is true → true. Otherwise, **only in highlight mode** and
only when `flags & 0x808` (street or dirt road): ask the street tool `OccupantExists(x, z, 0x8, &mask)` (network tool
slot 9, pop 16, AL); if it reports a draft, the result is `byte e of mask ≠ 0`; else ask the dirt tool
`OccupantExists(x, z, 0x800, &mask)` the same way; else false. (The mask is 4 bytes, one per network edge; the SDK
calls it `outOccupantId`; the contract is in §8.8.) In real mode the second branch is not taken. Details of the
draft branch:
- It asks the street tool first and the dirt-road tool second, whichever bits of `0x808` are set.
- It ignores `type`, so any draft connection on edge e counts for every type.
- It takes the byte by shifting the mask right by 8·e. For e = −1 the x86 shift count wraps to 24, so a side
  outside 0..3 reads byte 3 (+z) *[quirk]* (`0x0072FF14`, `0x0072FF45`).

### 7.14 Worked example

Zone 1 (low density residential), `SetOptions(0, 1, 0)` (place streets), lot 2 × 2 with `minDepth` 1, `maxDepth` 3,
block 6; a road on (8..17, 9); zone x 10..15, z 10..13 (6 × 4), nothing else.

- `ExtendAsStreets`/`LayInJogStreets`: the road row z = 9 is outside the rect; its cells (10..15, 9) connect on both
  ±x sides, so no dead end, no street.
- `ExamineBoundaries`: S = {600, 4, 6, 4}, C = {1, 0, 0, 0}: D = 0 (the road), Q = 3 (S[1] ≤ S[3]); zone 1 with
  `placeStreets` and a 6 × 4 region → flag 0, swap: **D = 3, Q = 0**.
- Frame (§7.6.1): Q = 0 → depth axis x (a = 10..15), row axis z (b = 10..13). D = 3 → low side: p0 = 10, sp = +2,
  j = +2. Q = 0 → o0 = 10, so = +2, k = +2. A = 3, B = 1; C[3] = 0 → first = B = 1; farRoad = (C[1] = 1) = false.
  Lrow = 4, Ldepth = 6.
- Row o = 10, p = 10: rem = 6, f = B, case (c): 6 ≥ 1 + 2·2 → P (no change): lot **(10, 10)–(11, 11) facing 1**.
  f was B → f := A, and s = p + j = 12 ∈ [10, 15]: first row → `DrawStreet((12, 10) → (12, 13), perCell 0)`, one
  street segment (12, 10)–(12, 13); p := 11. Levelling; p := 11 + 2 = 13.
- p = 13: rem = 3, f = A, case (b): 3 ≤ maxDepth → n = 3, δ = 1: lot **(13, 10)–(15, 11) facing 3**; p := 14;
  f := B; p := 16 > 15, the row ends.
- acc = 2 < 6 and flag = 0: no cross street. Row o = 12: same depth positions: lots **(10, 12)–(11, 13) facing 1**
  and **(13, 12)–(15, 13) facing 3**; the back-street line is skipped without drawing. acc = 4. Row o = 14 > 13 ends.

Result: four lots, two west of the new street facing it (facing 1 = +x) and two east of it facing it (facing
3 = −x); the road on z = 9 is only a scoring input (the lots at z = 10 touch it on their −z side but face the
street). Conformance test 19.

---

## 8. Lot manager contract *[compat]*

All lot-manager methods are called on the `+4` sub-object (global `0x00B43D08`).

### 8.1 The cell → lot map and the master list

- Every cell holds the lot covering it or null. A lot's cells are its bounding rect; a lot holds **one** reference
  owned by the manager, not one per cell.
- The master lot list holds each lot once. New lots are inserted at the traffic simulator's desired insertion point
  (§8.5 step 4); in practice that point is null, so new lots are appended *[inferred]*. After a load it is rebuilt in first-cell order,
  x outer, z inner (§10.2).
- `GetLotCount` (slot 16) = list size.

### 8.2 Lookups

| Slot | Method | Behaviour |
|---|---|---|
| 5 | `GetLot(const cS3DVector3&)` | `city.LocationIsInBounds` (slot 98), `PositionToCell` (slot 95), then slot 6 with false |
| 6 | `GetLot(x, z, bool originOnly)` | null if outside `0 ≤ x < cellsX`, `0 ≤ z < cellsZ`; else the cell's lot; with `originOnly`, only if `lot.GetLocation()` (slot 21) is `(x, z)`. **No AddRef** |
| 13 | `GetRandomLot()` | up to 999 tries of `x := U(cellsX)`, `z := U(cellsZ)` (x first); first non-null cell lot, no AddRef; else null (§9) |
| 14 | `GetOccupantLot(occupant)` | `p := occupant.GetPosition()`; per axis `a := r24(p × 0.0625f)`, `c := rhe(r24(a − 0.5f))`, `c := c + 1` if `a = c + 1` exactly (so c = ⌊p / 16⌋); then slot 6 `(cx, cz, false)` |

### 8.3 Lot lists (caller-supplied `std::list<cISC4Lot*>`, B nodes)

| Slot | Method | Behaviour |
|---|---|---|
| 7 | `GetLotsInCellRect(list, x1, z1, x2, z2, bool originOnly)` → count (EAX) | **Clears the list first: releases every element and frees every node with B** (keeps the head). Clamps the rect to the city (empty → 0). `false`: x outer, z inner; a lot is appended when it differs from the lot at `(x, z−1)` (within the rect) and from the lot at `(x−1, z)`. `true`: z outer, x inner, lots whose origin is the cell. One AddRef per appended lot. Returns the number appended |
| 8 | `GetLotsSurroundingLot(x, z, list, flags)` → count | lot at `(x, z)` via slot 6 (false); none → 0; else slot 9 |
| 9 | `GetLotsSurroundingLot(lot, list, flags)` → count *[inferred: EAX of slot 10]* | `lot.GetBoundingRect()` then slot 10 |
| 10 | `GetLotsSurroundingRect(rect, list, flags)` → count | walks the ring one cell outside the rect (`0x0060CEE0`); side bits 1 = row z1−1, 2 = column x2+1, 4 = row z2+1, 8 = column x1−1; bits for sides outside the city are cleared. `flags = 0xF` → every ring cell; otherwise a ring cell needs its side's bit (a corner needs both). Appends each lot **that does not intersect the rect**, once (linear search of the whole list, including old elements), one AddRef each. Does not clear. Returns the number appended in this call. **Walk order — verified from the exe, was `[inferred]`:** row by row, ascending z. The iterator is `{state, value=x, index=z, step, start=x1−1, limit=x2+1, count=z2+1}`; `index` (z) runs z1−1 → z2+1, and within each row `value` (x) runs from `start` with `step`, where `step = 1` for the first and last index but `step = max(limit − start, 1)` for every interior index (`0x0060CF4C`–`0x0060CF67`: `EDX = limit; EDX −= start; step = (EDX < 1) ? 1 : EDX`). So the first row z1−1 walks **every** x ascending, each interior row z1..z2 visits **only its two side cells** (x1−1 then x2+1), and the last row z2+1 again walks every x ascending. Assembly: at `0x0060CF4A` `JGE 0x0060CF6C` takes the `step = EAX (=1)` arm on the final index, otherwise `0x0060CF4C` computes the jump. The side-mask filter in the caller `0x0072E2B0` matches the four bits exactly (`iVar4 != local_3c \|\| bit1`, `iVar5 != local_38 \|\| bit2`, `iVar4 != local_34 \|\| bit4`, `iVar5 != iVar8 \|\| bit8`), confirming the side-to-bit mapping |
| 15 | `GetSafeLotIterator()` | an 8-byte C object `{node, master list}` registered with the manager, so lot removal advances it. Freeing not traced (§14.3) |

### 8.4 `CanCreateLot(x, z, w, h, zoneNotNeeded, waterAllowed, cfg)` (slot 20, `0x0072C2D0`, pop 28) → int32

`waterOK := waterAllowed ? true : (cfg ? cfg.IsCompatibleWithWater() : false)` (lot configuration slot 14). The
result is the **minimum** cell code over the rect (x outer, z inner), starting from 8:

| Code | Cell condition (first match) |
|---|---|
| 0 | outside the city, or a restriction region is set and does not include the cell |
| 1 | not `waterOK` and `IsCellWater` |
| 2 | real mode, not `zoneNotNeeded`, and the cell is not zoned (zone = the current zone type if set, else `IsRCIZone(zone)`) |
| 3 | `ExistsNetworkOfType(x, z, 0x1F4F)` |
| 7 | the cell is occupied: a lot (in highlight mode: a real lot not virtually deleted, or a virtual lot) |
| 8 | free |

Code 7 means "free except for existing lots"; the tract developer treats 7 and 8 as success
(`tract-developer.md` §4.15).

`CanCreatePloppedLot` (slot 21, `0x007306C0`, pop 24) is the plop path; not specified.

### 8.5 Creating lots

`CreateLotIfPossible(x, z, w, h, facing, cISC4Lot** out, zoneNotNeeded, waterAllowed, cfg)` (slot 22, `0x00730A80`,
pop 36): `*out := null`; `code := CanCreateLot(x, z, w, h, zoneNotNeeded, waterAllowed, null)` — **the `cfg`
argument is ignored**. If code = 8: in highlight mode record a virtual lot `{x, z, facing}` for every cell of the
rect, x outer and z inner. A facing of −1 is recorded as is, with no draw. `*out` stays null and the result is AL = 1
(upper EAX undefined). Else, if `facing = −1`, `CreateLotToFaceRoad` with a fresh empty list (**one draw**,
§9); else `CreateLot`. Returns EAX = (`*out` ≠ null). Code 7 (an existing lot) fails: the caller must delete it first.

`CreateLot(x, z, w, h, facing, cISC4Lot*& out)` (slot 23, `0x0072E520`, pop 24) → EAX 0/1. No checks:

1. Allocate and construct a lot (`0x006BF430` 0x8C bytes, constructor `0x006C1E50(x, z, w, h, facing)`).
2. `out := lot` with one AddRef **for the caller**; `lot.Init()` (slot 5); `lot.SetState(1, 0, 0)` (slot 34);
   one more AddRef for the manager.
3. Write the lot into every cell of the rect (overwriting).
4. `it := trafficSimulator.GetDesiredLotInsertionPoint()` (traffic simulator slot 22). If `it` is non-null: when its
   list is set, insert the lot after the iterator's node (append if the iterator is at the end), move the iterator to
   its node's predecessor, and count it; when its list is null nothing is inserted and the count is unchanged
   *[quirk]*. If `it` is null: append to the master list and count it.
   The traffic simulator answers only in its work-cycle state 1, which the cycle never enters (traffic-simulator.md
   §3), so `it` is null in practice and new lots are appended *[inferred]*.
5. `bds.GetLotDeveloper().SetEmptyLotBaseTexture(lot)` (BDS slot 36, lot developer slot 43).

### 8.6 Deleting lots

`DeleteLot(lot)` (slot 24, `0x0072F8E0`, pop 4; EAX undefined). Highlight mode: record every cell of the lot's
bounding rect as virtually deleted, with `{x1, z1, facing}` (lot slots 14 and 23). The lot itself is untouched: it
stays in the cell map and the master list, and it is not released. Real mode, in order:

1. `lotDeveloper.RemoveBuildingZotProp(lot)` (slot 11);
2. `lotDeveloper.GetLotBaseTextureOccupant(lot, &occ, 0)` (slot 41); if found, `occupantManager.RemoveOccupant(occ's
   occupant, 1, 0)` (slot 21) and release it;
3. null every cell of the lot;
4. remove it from the master list (B free; registered safe iterators are advanced) and decrement the count;
5. if the lot has a business unit (lot slot 25), `lotDeveloper.RemoveLotFromItsBusinessUnit(lot)` (slot 33);
6. `lot.Shutdown()` (slot 7) and release the manager's reference.

`DeleteLots(x1, z1, x2, z2)` (slot 25, `0x0072CAE0`, pop 16; EAX undefined): `GetLotsInCellRect(…, false)` into a
temporary list, `DeleteLot` each, release and free.

### 8.7 Geometry and altitude helpers

| Slot | Method | Behaviour |
|---|---|---|
| 11 | `GetVerticesAtEdgeOfLot(rect, flags, SC4Rect& out)` | lowest set flag bit only: 1 → `(x1, z1)–(x2+1, z1)`; 2 → `(x2+1, z1)–(x2+1, z2+1)`; 4 → `(x1, z2+1)–(x2+1, z2+1)`; 8 → `(x1, z1)–(x1, z2+1)`; no bit → out untouched. EAX undefined |
| 12 | `GetNeighborCellsAtEdgeOfLot(rect, flags, SC4Rect& out)` → AL | out := `{0, 0, −1, −1}`; lowest bit: 1 → if z1 > 0: `{x1, z1−1, x2, z1−1}`; 2 → if x2 < cellsX−1: `{x2+1, z1, x2+1, z2}`; 4 → if z2 < cellsZ−1: `{x1, z2+1, x2, z2+1}`; 8 → if x1 > 0: `{x1−1, z1, x1−1, z2}`. AL = (out.x2 ≥ out.x1) |
| 17 | `GetLotSavvyTerrainVertexAltitude(vx, vz, cISC4Lot* exclude, bool* ok)` → float (ST0) | `*ok := 1`. For cells `(vx−1, vz−1)`, `(vx, vz−1)`, `(vx−1, vz)`, `(vx, vz)` in that order (in-city only): the first lot that is not `exclude` and has flag bit 0 (lot slot 12) gives `lot.GetAltitude()` (slot 16). Otherwise `*ok := 0`, `terrain.GetAltitudeAtVertex(vx, vz)` (slot 14) |
| 18 | `GetLotSavvyTerrainAltitude(float x, float z, bool* ok)` → float | per axis `c := rhe(r24(r24(x × 0.0625f) − 0.499f))` (constant `0x3EFF7CEE` = 0.49900001); clamp c < 0 → 0, c ≥ count → count − 1. Lot with flag bit 0 → `*ok := 1`, lot altitude; else `*ok := 0`, `terrain.GetAltitude(x, z)` (slot 13). `ok` may be null |
| 19 | `GetLotSavvyCellAltitude(cx, cz, bool* ok)` → float | **no bounds check**. Lot with flag bit 0 → lot altitude; else terrain altitude at `(r24(r24(cx + 0.5f) × 16), r24(r24(cz + 0.5f) × 16))` |
| 26 | `GetLotVertexAltitudeAdjustmentRange(vx, vz, float& a, float& b)` | the distinct lots in cells `vx−1..vx × vz−1..vz` (clamped): `a := b := FLT_MAX`, or 32.0 for both if any has `GetState()` (slot 33) > 1. EAX undefined |
| 27 | `GetMaximumCapacityGrid()` | the `uint16` grid of §1.2, no AddRef |

Slot 18's cell is ⌊x/16⌋ except that x/16 with fraction > 0.999 rounds up (x = 15.99 → cell 1) *[quirk]*.

### 8.8 `cISC4ZoneDeveloper` helpers used by growth

- `ExistsNetworkOfType(x, z, flags)` (slot 8, `0x0072FE10`, pop 12, AL): the first occupant of the cell of network
  type `0xC772BF98` (occupant manager slot 41, list from A) that is a `cISC4NetworkOccupant` with
  `HasAnyNetworkFlag(flags)`. When no real network matches, in highlight mode only (`0x0072FE33`–`0x0072FE70`):
  - if `flags & 8`, the result is the street tool's `OccupantExists(x, z, 8, &mask)`, and the dirt-road tool is
    **not** asked even when `flags & 0x800` is set too *[quirk]*;
  - else, if `flags & 0x800`, the result is the dirt-road tool's `OccupantExists(x, z, 0x800, &mask)`;
  - else false.

  The road tool is never asked.
- **Network tool `OccupantExists(x, z, flags, &mask)`** (slot 9, `0x006248D0`, pop 16, AL). This is the draft query
  of §7.12, §7.13 and the item above. The tool looks up its cell record for (x, z) (key `(z << 16) + x`,
  `0x00633220`). On a miss it builds the record from the existing network, or none if the cell has no network.
  - It returns true only when a record exists, its draft byte (`+0x51`) is set, and its network flags (`+8`)
    intersect `flags`. Then `*mask` := the record's 4-byte edge mask for the tool's own network type
    (`+0x18 + 4·type`, type at tool `+0x35C`); byte i ≠ 0 means a connection on network edge i.
  - Otherwise it returns false and leaves `*mask` alone.

  A replacement needs only this contract: for each tool, its set of draft cells with their network flags and edge
  masks. How the set grows (ZQ11, ZQ12; `PlaceNetworkOccupants`, `0x00629210`) *[compat]*:
  - **Lifetime.** The records live in the tool's cell cache (`+0x1C`). Only `Reset` (slot 5, `0x00626F90`) and a
    **real-mode** Place (`0x0062932F`) empty it. A draft Place does not, so drafts accumulate until the next
    `Reset`, which for the street tool is the zone manager's clear (§7.1).
  - **The draft byte.** A record built from the existing network has it clear (`0x00633030`), so an existing network
    alone never answers true. When a Place's solve succeeds, every cell the solver re-solved gets it
    (`0x0062591E`, `0x00625BFB`). Those are the covered cells and the cells of existing networks around them that
    the solver includes (`0x006355C0`). A real-mode Place sets it too (`0x00625D20`), but the zone manager asks the
    tools only in highlight mode.
  - **Covering a cell.** A Place takes the cell's cached record, or builds one (`0x00633250`). It ORs its network
    flag into the record's flags. It then merges its edge directions into the record's edges for its type
    (`0x00638160` → `0x00620F00`; `0x00626EB0` ORs the neighbour connections in the same way). The merge keeps an
    edge that already has a direction and gives the new direction to an edge that has none.
  - **A second draft Place on a drafted cell** (ZQ11) keeps the record: flags := old OR new. Each edge byte is
    ≠ 0 afterwards when it was ≠ 0 before or the new draft connects that edge, so for this contract the masks are
    ORed. The byte values themselves are not an OR, so a replacement promises only "≠ 0":
    - the low 3 bits are a direction code, and opposite one-way directions merge to two-way;
    - the upper bits are minor types, which the solver reassigns.

    *[inferred]* A second Place that fails can still leave its merged edges in a record that an earlier Place
    marked as a draft, because the merge runs before the solve.
  - **A draft Place over a cell that has a network** (ZQ12) **does** make it a draft once the solve succeeds:
    - its flags are the existing network's flags OR the tool's (8 for the street tool);
    - its mask is the existing edges of the tool's type merged with the new ones.

    A cell of an existing network next to the draft that the solver includes also gets the draft byte, but keeps
    its own flags (only covered cells receive the tool's flag). When the draft connects to it, its edges for the
    tool's type gain the edge towards the draft. So it answers only when its own network matches the query: an
    existing street next to a street draft answers `OccupantExists(x, z, 8)` with that new edge, and an existing
    road does not.
  - **Ends next to earlier drafts.**
    - A segment Place (a ≠ b) looks at the cell just beyond each end. In draft mode, if that cell has no real
      network matching the Place's flags but has a cached record whose flags intersect them, the segment is
      lengthened by that cell (`0x006296D0`–`0x006296F5`, `0x00629782`–`0x006297A7`), so the two drafts connect.
    - A real network of the tool's own type beyond an end lengthens the segment in both modes.
    - A single-cell Place looks at its four neighbours only when one of them has a real network matching the
      flags. It then extends along that axis in the same way.
- `IsCellWater(x, z)` (slot 9, `0x0072A030`, pop 8): tail jump to `terrain.CellHasWater` (terrain slot 34).
- `SetOptions(alternateOrientation, placeStreets, customZoneSize)` (slot 7, `0x007347C0`, pop 12).

---

## 9. Random draws *[compat]*

**Zone manager: none.** Its member `cRZRandom` (constructed with `0xFFFFFFFF`) is never drawn (no RNG call in
`0x004DC1A0`–`0x004DE430`; the Mac build also only constructs it). A replacement need not create it.

**Zone developer: one file-local `cRZRandom` at `0x00B4B398`**, shared by all instances, seeded once at static
initialisation (`0x00A74F00`, constructor `0x00913A5C`) with `cRZTime()` local wall clock XOR **`0x8C09C68A`**, never
re-seeded, not saved (foundations §6.2). FreeCity4 seeds it from the host's per-instance seed (foundations §6.3).
Only `RandomUint32Uniform(n)` is used (one `NextRaw` advance per call when n > 0). No `rand()`, `random_shuffle`,
random iterator or `cRZRandomSuper` is used by either class.

| # | Operation | Generator / method | Arguments, order | Draws |
|---|---|---|---|---|
| Z1 | `GetRandomLot` (lot manager slot 13) | `0x00B4B398` `U(n)` | per try: `U(cellsX)` then `U(cellsZ)` | 2 per try, until a lot is found, at most 999 tries (1998 draws) |
| Z2 | `CreateLotToFaceRoad` with an empty list | `0x00B4B398` `U(4)` | on entry, before any test | 1 |
| Z2a | … from `SimpleParcellize` (zones 7, 8, 9 and any non-1..6 zone reaching `OnZone`), also in highlight mode | | x ascending outer, z ascending inner, every cell of each component's rect; components in `SubdivideTheZone` order | 1 per rect cell |
| Z2b | … from `CreateLotRemnants` when the current zone is 7, 8 or 9 | | z outer, x inner, every cell of the deleted lot | 1 per cell |
| Z2c | … from `CreateLotIfPossible` with facing −1, real mode, code 8 | | tract developer aggregation (`tract-developer.md` §4.15.3, purposes 5–8 with a square configuration) | 1 per call |

Paths that reach Z2b: `OnDezone` (zoning to 0 over agricultural or industrial lots, including the zone manager's
dezone under new networks, §5.1) and the zone developer's InsertOccupant handler (§5.2). `Parcellize` (zones 1–6),
`CreateLotRemnants` for zones 1–6, `DrawStreet`, `ExtendAsStreets`, `LayInJogStreets` and the connection tests
(§7.6, §7.10–§7.13) never draw from this generator. Whether the street tool's `PlaceNetworkOccupants` draws from
any generator while placing automatic streets is not traced (§14.3).

The draw result fixes only the **orientation order** tried by `CreateLotToFaceRoad`; it decides a lot's facing when
two or more sides have roads (the first in `r, r+1, …` order wins) and when none has (facing r).

**Seeded test** (`cRZRandom` of foundations §6.1 with `lcg := 0x8C09C68A`, i.e. clock value 0): `NextRaw` gives
`0xF5DF3363`, `0x23EC5CA1`, `0xC51FC91C`, `0x1EACEFC5`, …; `U(4)` gives 3, 0, 3, 0, … Conformance test 14 applies it.

---

## 10. Persistence *[compat]*

### 10.1 Zone manager (`cIGZSerializable` at +8; `Write` `0x004DC7A0`, `Read` `0x004DD400`)

Record (little-endian; references as in foundations §2.7):

| # | Field |
|---|---|
| 1 | `uint16` version = 1 |
| 2 | reference: zone grid |
| 3 | 16 × u32 `zoneCount` |
| 4 | 16 × u32 `undeveloped` |
| 5 | 16 × u32 `unnamedPerType` |
| 6 | 4 × u32: `undevelopedTotal`, `unnamed1`, `abandonedTileCount`, `unnamed2` |
| 7 | 16 × s32 min sizes; 16 × s32 max sizes |
| 8 | 16 × s64 creation costs; 16 × s64 destruction costs |
| 9 | 16 × u32 texture instances; 16 × u32 drag colours |
| 10 | u8 display alpha |
| 11 | u32 n1, then n1 × `{u8 zone, u8 result, u32 count}` of map 1 in map order (ordered by result, then zone) |
| 12 | u32 n2, then map 2 the same way |
| 13 | references: city, occupant manager, terrain, pollution simulator, budget simulator |

`Write` returns `GetError() = 0`. Max slopes and the default alpha are not saved.

`Read`: sets the loaded flag; version ≠ 1 → false (nothing more read). Grid reference with IID `0xC9BD939A`; the
fields in the same order; each map is cleared first if non-empty, entries inserted uniquely (duplicates dropped);
references with IIDs `0x66BA4DB5` (city), `0xC6BB8334` (occupant manager), `0xC70BD37E` (terrain), `0x877181EC`
(pollution), `0x6772BD23` (budget). Returns `GetError() = 0`. The tuning arrays read here are overwritten by
`Init` (§4); `Init` skips the count reset because the loaded flag is set and keeps the loaded grid.

The display alpha read here (`0x004DD58E`, the byte at +0x360) lasts only until `Init` (ZG3). `Init` calls
`ReadSimulatorTunableValues` (`0x004DE192`), which overwrites it with the exemplar's Zone Display Alpha (shipped 170)
and copies that to the default alpha (`0x004DC65E`–`0x004DC675`). The saved alpha survives only when the exemplar or
the property is missing.

### 10.2 Zone developer (`cIGZSerializable` at +0xC; `Write` `0x0072AB40`, `Read` `0x00734380`)

| # | Field |
|---|---|
| 1 | `uint16` version = 1 |
| 2 | u32 countX (rows of the cell map) |
| 3 | u32 countZ (size of row 0; 0 if there are no rows) |
| 4 | countX × countZ references to the cell's lot (null reference for empty cells), x outer, z inner |

`Read`: sets the loaded flag; version must be 1; resizes the map; reads each cell's lot with IID `0xE818FCED`
(`cISC4Lot`). A temporary `hash_set` (100 buckets) dedupes: the first sighting of a lot appends it to the master list
(no extra reference) and increments the count; later sightings release the extra reference. Each lot ends with
exactly the manager's one reference. The master list order after load is first-cell order, x outer, z inner, which
generally differs from the order before saving *[quirk]* (it changes e.g. the budget's lot iteration order,
`budget.md` §5.2). The options, lot-size parameters, highlight state, network tools and maximum-capacity grid are not
saved; `Init` and PostCityInit rebuild what is needed (§1.2, §5.2).

No RNG state is saved (foundations §6.2).

---

## 11. Interface slots *[compat]* (foundations §16, §17)

Pops from `tools/abi_extract.py --check`; return channels from the epilogues. "SDK" = gzcom-dll header.

### 11.1 `cISC4ZoneManager`, vtable `0x00A92E00`, 21 interface slots (+ deleting destructor `0x004DCF40` as slot 21)

| Slot | Offset | Method (as compiled) | Address | Pop | Return | Notes |
|---|---|---|---|---|---|---|
| 0–2 | +0x0–+0x8 | QueryInterface, AddRef, Release | `0x004DC1B0`, —, `0x004DC430` | 8, 0, 0 | AL / EAX | |
| 3 | +0xC | `Init()` | `0x004DE170` | 0 | AL | §1.1 |
| 4 | +0x10 | `Shutdown()` | `0x004DC230` | 0 | AL = 1 | |
| 5 | +0x14 | `GetZoneGrid()` | `0x00698CF0` (shared getter) | 0 | EAX | no AddRef |
| 6 | +0x18 | `GetZoneCount(type)` | `0x004DC2D0` | 4 | EAX | no bounds check |
| 7 | +0x1C | `GetUndevelopedTileCount(type)` | `0x004DC2F0` | 4 | EAX | overloads reversed vs SDK (allowed by §16); never updated (§2) |
| 8 | +0x20 | `GetUndevelopedTileCount()` | `0x004DC2E0` | 0 | EAX | never updated |
| 9 | +0x24 | `GetDevelopmentFailureCount(int32 zone, int32 result)` | `0x004DE130` | 8 | EAX | **SDK error**: the parameters are (zone, result), not (x, z). Returns 0 unless `zone < 16` and `result < 19` (signed); otherwise map 1's count, **inserting a 0 entry** when absent |
| 10 | +0x28 | `GetAbandonedTileCount()` | `0x007C9FB0` (shared getter) | 0 | EAX | never updated |
| 11 | +0x2C | `PlaceZone(…)` (§6) | `0x004DD780` | 40 | AL | SDK: argument 8 is the **int64 cost**, not a zoned-cell count; argument 10 is a `std::set<cISC4Lot*>*` (SDK `intptr_t`); argument 7 is "lot must lie inside the rect" |
| 12 | +0x30 | `GetZoneType(x, z)` | `0x004DC740` | 8 | EAX, sign-extended `int8` | no bounds check |
| 13 | +0x34 | `IsRCIZone(type)` | `0x004DC300` | 4 | AL only | |
| 14 | +0x38 | `GetTextureForZone(type, cGZPersistResourceKey&)` | `0x004DC340` | 8 | AL | type < 16: key `{0x7AB50E44, 0x1ABE787D, texture[type]}` |
| 15 | +0x3C | `GetZoneDragColor(type)` | `0x004DCD20` | 4 | EAX | |
| 16 | +0x40 | `GetZoneDisplayAlpha()` | `0x004DCD10` | 0 | AL | |
| 17 | +0x44 | `GetDefaultZoneDisplayAlpha()` | `0x004DC370` | 0 | AL | |
| 18 | +0x48 | `SetZoneDisplayAlpha(uint8)` | `0x004DCF60` | 4 | — | when changed: every lot of the city (lot manager slot 7) in state 1 gets lot developer slot 43 `SetEmptyLotBaseTexture` |
| 19 | +0x4C | `GetMinZoneSize(type)` | `0x004DCD30` | 4 | EAX | |
| 20 | +0x50 | `GetMaxZoneSize(type)` | `0x004DCD40` | 4 | EAX | |

### 11.2 `cISC4ZoneDeveloper`, vtable `0x00AB3830`, 10 interface slots (+ deleting destructor `0x00734AD0` as slot 10)

| Slot | Offset | Method | Address | Pop | Return |
|---|---|---|---|---|---|
| 0–2 | | QueryInterface, AddRef, Release | `0x00729EF0`, —, `0x0072A270` | 8, 0, 0 | |
| 3 | +0xC | `Init()` | `0x00734030` | 0 | AL |
| 4 | +0x10 | `Shutdown()` | `0x007335A0` | 0 | AL = 1 |
| 5 | +0x14 | `HighlightParcels(const SC4CellRegion<long>&, int32 type, SC4Point<long>*, SC4CellRegion<long>** out)` | `0x00733F30` | 16 | void |
| 6 | +0x18 | `DoParcellization(const SC4CellRegion<long>&, int32 type, bool removeStreets)` | `0x00733FA0` | 12 | void |
| 7 | +0x1C | `SetOptions(bool alternateOrientation, bool placeStreets, bool customZoneSize)` | `0x007347C0` | 12 | void |
| 8 | +0x20 | `ExistsNetworkOfType(x, z, uint32 flags)` | `0x0072FE10` | 12 | AL |
| 9 | +0x24 | `IsCellWater(x, z)` | `0x0072A030` | 8 | AL (tail jump to terrain slot 34) |

Matches the SDK. `HighlightParcels`' `*out` receives a new C region owned by the caller (§13.2).

### 11.3 `cISC4LotManager`, vtable `0x00AB37C0` (on the +4 sub-object), 28 slots

Slots 0–2 are thunks (`0x007347F0`, `0x005200D0`, `0x00734820`). Overloads compile in reverse declaration order
(§16): slot 5 is the vector overload, slot 8 the `(x, z)` overload.

| Slot | Offset | Method (as compiled) | Address | Pop | Return | SDK |
|---|---|---|---|---|---|---|
| 3 | +0xC | `Init()` | `0x007348C0` | 0 | AL | |
| 4 | +0x10 | `Shutdown()` | `0x007348B0` | 0 | AL | |
| 5 | +0x14 | `GetLot(const cS3DVector3&)` | `0x0072A1C0` | 4 | EAX | |
| 6 | +0x18 | `GetLot(x, z, bool originOnly)` | `0x0072B410` | 12 | EAX | |
| 7 | +0x1C | `GetLotsInCellRect(list&, x1, z1, x2, z2, bool)` | `0x0072E080` | 24 | EAX count | **error**: returns a count, not bool (a multiple of 256 reads false in AL) |
| 8 | +0x20 | `GetLotsSurroundingLot(x, z, list&, flags)` | `0x0072A230` | 16 | EAX count | **error**: count |
| 9 | +0x24 | `GetLotsSurroundingLot(const cISC4Lot*, list&, flags)` | `0x0072A750` | 12 | EAX count *[inferred]* | **error**: count |
| 10 | +0x28 | `GetLotsSurroundingRect(const SC4Rect<long>&, list&, flags)` | `0x0072E2B0` | 12 | EAX count | **error**: count |
| 11 | +0x2C | `GetVerticesAtEdgeOfLot(const SC4Rect&, flags, SC4Rect&)` | `0x0072A790` | 12 | undefined | **error**: no bool result |
| 12 | +0x30 | `GetNeighborCellsAtEdgeOfLot(const SC4Rect&, flags, SC4Rect&)` | `0x0072A820` | 12 | AL | |
| 13 | +0x34 | `GetRandomLot()` | `0x0072B3C0` | 0 | EAX | |
| 14 | +0x38 | `GetOccupantLot(cISC4Occupant*)` | `0x0072A110` | 4 | EAX | |
| 15 | +0x3C | `GetSafeLotIterator()` | `0x00734850` | 0 | EAX (C object) | |
| 16 | +0x40 | `GetLotCount()` | `0x00809250` (shared getter) | 0 | EAX | |
| 17 | +0x44 | `GetLotSavvyTerrainVertexAltitude(vx, vz, cISC4Lot*, bool*)` | `0x0072A8F0` | 16 | **float, ST0** | **error**: SDK says `int32` |
| 18 | +0x48 | `GetLotSavvyTerrainAltitude(float, float, bool*)` | `0x0072A9F0` | 12 | **float, ST0** | **error**: SDK says `int32` |
| 19 | +0x4C | `GetLotSavvyCellAltitude(x, z, bool*)` | `0x0072AAC0` | 12 | **float, ST0** | **error**: SDK says `int32` |
| 20 | +0x50 | `CanCreateLot(x, z, w, h, bool zoneNotNeeded, bool waterAllowed, cISC4LotConfiguration*)` | `0x0072C2D0` | 28 | EAX code (§8.4) | SDK names the 6th `bIgnoreConfig` (it only bypasses the water test) |
| 21 | +0x54 | `CanCreatePloppedLot(SC4Rect<long> by value, int32, cISC4LotConfiguration*)` | `0x007306C0` | 24 | EAX | |
| 22 | +0x58 | `CreateLotIfPossible(x, z, w, h, facing, cISC4Lot*& out, bool, bool, cISC4LotConfiguration*)` | `0x00730A80` | 36 | EAX 0/1 (AL = 1 in highlight mode) | **error**: the out parameter is missing in the SDK (foundations §17.8); `cfg` is ignored |
| 23 | +0x5C | `CreateLot(x, z, w, h, facing, cISC4Lot*& out)` | `0x0072E520` | 24 | EAX 0/1 | |
| 24 | +0x60 | `DeleteLot(cISC4Lot*)` | `0x0072F8E0` | 4 | undefined | **error**: no bool result |
| 25 | +0x64 | `DeleteLots(x1, z1, x2, z2)` | `0x0072CAE0` | 16 | undefined | **error**: no bool result |
| 26 | +0x68 | `GetLotVertexAltitudeAdjustmentRange(vx, vz, float&, float&)` | `0x0072AE20` | 16 | undefined | **error**: no bool result |
| 27 | +0x6C | `GetMaximumCapacityGrid()` | `0x00809240` (shared getter) | 0 | EAX | **error**: the grid is `cISC4SimGrid<uint16_t>` (class `0x49B9E604`, constructor `0x0045F9B0`), not `<short>` |

Where the original leaves EAX undefined, a replacement should return true (1) *[choice]*.

### 11.4 Other vtables and shared types

| Interface | Class | vtable | Slots | Methods |
|---|---|---|---|---|
| `cIGZMessageTarget2` | zone manager (+4) | `0x00A92DF0` | 4 | QI, AddRef, Release (thunks), `DoMessage` `0x004DCA70` |
| `cIGZSerializable` | zone manager (+8) | `0x00A92DD8` | 6 | QI, AddRef, Release, `Write` `0x004DC7A0`, `Read` `0x004DD400`, `GetGZCLSID` `0x004DC1A0` |
| `cIGZMessageTarget2` | zone developer (+8) | `0x00AB37AC` | 4 | …, `DoMessage` `0x007328D0` |
| `cIGZSerializable` | zone developer (+0xC) | `0x00AB3794` | 6 | …, `Write` `0x0072AB40`, `Read` `0x00734380`, `GetGZCLSID` `0x00729EB0` |

`SC4CellRegion<long>` (0x24 bytes, crosses the interface in `PlaceZone`, `DoParcellization`, `HighlightParcels` and
ZoningChange void2): `int32 x1, z1, x2, z2` (inclusive) then a `cRZCellMap<bool>` (vtable `0x00A86364`): width
(x count), height (z count), words per row = ⌈height / 32⌉, `uint32** rows`; bit (dx, dz) is bit `dz mod 32` of
`rows[dx][dz / 32]`. Rows and the row array come from C. Constructors `0x004422A0(w, h, fill)` and
`0x00442640(rect, fill)`, destructor `0x00441A60`, `IncludesCell` `0x00466610` (unsigned offset test).

---

## 12. Dependencies *[compat]*

Pops are bytes (from `docs/abi/interfaces.tsv` argument counts or `--check`).

| Interface (global) | Slot (offset) | Method | Pop | Return | Used by |
|---|---|---|---|---|---|
| `cISC4City` (`0x00B43CEC`) | 40 (+0xA0) | `GetTrafficNetwork` | 0 | EAX | PlaceZone |
| | 78 (+0x138) | `GetLandfillDeveloper` | 0 | EAX | PlaceZone (type 14) |
| | 93, 94 (+0x174, +0x178) | cell counts X, Z | 0 | EAX | both Inits, OnZone |
| | 95 (+0x17C) | `PositionToCell` | 16 | AL | lot manager slot 5, InsertOccupant |
| | 98 (+0x188) | `LocationIsInBounds` | 8 | AL | lot manager slot 5 |
| traffic network | 8 | `FindNetworkTile(x, z, 0x1F4F, 1)` | 16 | EAX | PlaceZone |
| network tile | 22, 48, 63 | `IsOfType(3)`, `AsOccupant`, `GetCostOfDemolition` (int64 EDX:EAX) | | | PlaceZone |
| `cISC4Occupant` | 7, 8, 11, 14, 32 | `GetType`, `GetPosition`, `GetBoundingCityCells`, `CanRemove(2)` *[inferred name]*, `IsFlagSet(1)` | | | PlaceZone, DoMessage, ExamineCell |
| `cISC4NetworkOccupant` (IID `0xA821EF94`) | 9 (+0x24) | `HasAnyNetworkFlag(flags)` | 4 | AL | DoMessage, ExistsNetworkOfType |
| `cISTETerrain` (`0x00B43CF4`) | 13, 14, 34 | `GetAltitude(x, z)`, `GetAltitudeAtVertex`, `CellHasWater` | 8 each | ST0, ST0, AL | lot manager 17–19, PlaceZone, IsCellWater |
| `cISC4OccupantManager` (`0x00B43D0C`) | 21 (+0x54) | `RemoveOccupant(occ, 1, 0)` | 12 | | DeleteLot |
| | 33 (+0x84) | occupant at cell by type (`0x6A11BD2B`) | 16 | | PlaceZone |
| | 41 (+0xA4) | `GetOccupantsByStandardCityCells(list, &cell, &cell, type, −1)` (list from A) | 20 | | ExistsNetworkOfType, ExamineCell |
| | 48 (+0xC0) | `ReleaseOccupantList` | 4 | | same |
| `cISC4BudgetSimulator` (`0x00B43D64`) | 6, 8 | `GetTotalFunds` (EDX:EAX), `WithdrawFunds(int64)` (pop 8, AL) | 0, 8 | | PlaceZone |
| `cISC4Demolition` (`0x00B43D00`) | 6 (+0x18) | `DemolishRegion` (11 dwords) | 44 | AL | OnDezone |
| | 7 (+0x1C) | `DemolishLots` (8 dwords) | 32 | AL | PlaceZone |
| `cISC4BuildingDevelopmentSimulator` (`0x00B43D68`) | 19, 32, 36, 39 | `GetBuildingTypesByFamily(set)`, `GetDeveloper(type)`, `GetLotDeveloper`, `GetBuildingProfile` | 8, 4, 0, 4 | EAX | DetermineLotSize, PlaceZone, CreateLot |
| `cISC4LotDeveloper` | 11, 21, 29, 33, 41, 43 | `RemoveBuildingZotProp`, `AttemptAreaLevelling` (24), `GetAreaSlope` (ST0), `RemoveLotFromItsBusinessUnit`, `GetLotBaseTextureOccupant` (12), `SetEmptyLotBaseTexture` | 4, 24, 8, 4, 12, 4 | | DeleteLot, Parcellize, PlaceZone, CreateLot |
| `cISC4Lot` | 5, 7, 8, 9, 12, 14, 16, 21, 23, 25, 31, 33, 34, 48, 69 | Init, Shutdown, PostCityInit, GetLotConfiguration, GetFlags, GetBoundingRect, GetAltitude (ST0), GetLocation, GetFacing, business unit, GetZoneType, GetState, SetState(1, 0, 0), GetBuilding, SetVerticesAreLocked | see `interfaces.tsv` | | throughout |
| `cISC4LotConfiguration` | 14, 15 | `IsCompatibleWithWater`, `IsCompatibleWithZoneType(type)` | 0, 4 | AL | CanCreateLot, PlaceZone, CreateParcellizableRegion |
| `cISC4LotConfigurationManager` (`0x00B43D18`) | 9 | `GetLotConfigurationsByFilter` | | | DetermineLotSize |
| `cISC4GrowthDeveloper` | 29 and others | `EstimateDesirability`, purpose, wealth, source demand, growth list | | | DetermineLotSize |
| `cISC4TrafficSimulator` (`0x00B43DA8`) | 22 (+0x58) | `GetDesiredLotInsertionPoint` | 0 | EAX | CreateLot |
| `cISC4LandfillDeveloper` | 9 | `GetZoningCost(region*)` (int64) | 4 | EDX:EAX | PlaceZone |
| `cIGZMessageServer2` (`0x00B43CCC`) | 4 (+0x10) | `MessagePost(m, 0)` | 8 | | ZoningChange, Defer |
| 3D view global (`0x00B43DD0`) | 10 (+0x28) | cached value (UI) | 0 | EAX | zone developer Init |
| `cISC4NetworkManager` | 5 (+0x14) | `GetNetworkTool(type, 1)` | 8 | EAX | PostCityInit |
| `cISC4NetworkTool` (street tool, type 3; dirt tool, type 11; vtable `0x00AA8748`) | 7 (+0x1C) | `PlaceNetworkOccupants(x1, z1, x2, z2, 0xC49, callback = the tool, realMode, −1)` (`0x00629210`) | 32 | **AL only** (SDK: `uint32_t`; upper EAX bytes are garbage) | DrawStreet (street tool) |
| | 9 (+0x24) | `OccupantExists(x, z, networkFlags, &mask)` (`0x006248D0`, §8.8); `mask` = 4 bytes, one per network edge | 16 | **AL only** (SDK: `uint32_t`) | ExistsNetworkOfType, ExistsConnectionOfType, LayInJogStreets; highlight mode only |
| | 5 (+0x14) | `Reset()` | 0 | | ClearHighlight (street and road tools only) |
| `cISC4NetworkOccupant` | 47 (+0xBC) | `ConnectsToNeighbor(edge, type)` | 8 | AL | ExistsConnectionOfType, LayInJogStreets |
| `cISC4OccupantManager` | 41 (+0xA4) | `GetOccupantsByStandardCityCells(list, &cell, &cell, 0xC772BF98, −1)` | 20 | | GetNetwork |
| `cISC4LotDeveloper` | 21 (+0x54) | `AttemptAreaLevelling(&rect, 0.0f, 1, 1, FLT_MAX, null)` | 24 | ignored | Parcellize, real mode only |
| `cISC4Lot` | 69 | `SetVerticesAreLocked(1 / 0)` | 4 | | Parcellize |

---

## 13. Replaceability

**Headless limit**: the original zone developer's `Init` fails without the 3D view (global `0x00B43DD0`), so it can
only be exercised in the running game; FreeCity4's zone developer must not depend on the view.

### 13.1 Who reaches in

- **Zone grid**: built directly (not through GZCOM) and written raw by `PlaceZone`; read raw by `GetZoneType` and,
  through `GetZoneGrid`, by the map view, land value and flammability *[inferred from their specs]*. A replacement
  zone manager follows simgrid §6.4 (game `int8` grid `0x49B9E603` created through GZCOM, tract size 1, instance ID
  0, saved by reference).
- **Maximum-capacity grid** (`uint16`, `0x49B9E604`): same rule; handed out by slot 27 without AddRef.
- **ZoningChange void2** is a game `SC4CellRegion<long>` freed by the game's message destructor with C: a
  replacement must allocate it with C (`0x005E55E0`) in this layout, or post its own message class whose destructor
  frees it.
- **Lists returned to callers** (slots 7–10) use B nodes and must keep that allocator; `PlaceZone`'s `outLots` set
  likewise (§13.2).
- **Lot objects** are created by the zone developer directly (`0x006BF430` + `0x006C1E50`), not through GZCOM. A
  replacement must create the game's own lot objects so that the original lot developer and the save format keep
  working *[inferred]*.
- **Global pointers**: the BDS, tract developer, lot developer, budget and UI read the zone manager and lot manager
  through `0x00B43D14` / `0x00B43D08`, which the city fills from components 6 and 8; replacing the classes through
  the GZCOM class registry (CLSIDs `0x298F9B2D`, `0x498F9B01`) is enough for those *[inferred]*.
- **Init blocker**: the original zone developer's `Init` fails without a 3D view (`0x00B43DD0`); a headless
  replacement must not have that dependency.
- **Traffic simulator coupling**: `CreateLot` asks the traffic simulator where to insert the lot; a replacement of
  either side must keep `GetDesiredLotInsertionPoint` semantics (§8.5).

Internal containers (no allocator row needed): the zone manager's changed-cell vector and demolish list, its two
failure maps; the zone developer's master list, iterator registry, virtual-lot maps, boundary-rule map,
component list, orientation lists, temporary lot lists and the configuration `hash_set` in `DetermineLotSize`.

### 13.2 Allocator rows (proposed for `docs/abi/allocators.md`)

Section (1), the game fills or hands out, the caller frees. Columns: Method (slot) | Container | Game allocator |
SDK container compatible? | Notes | Evidence.

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence |
|---|---|---|---|---|---|
| `cISC4LotManager::GetLotsInCellRect` (7) | list of lots | B | **no** | (correction) the initial clear **releases every element** (slot 2) before freeing its node with B, keeping the head; returns the number appended | `0x0072E080` → `0x00473EC0` |
| `cISC4LotManager::GetLotsSurroundingRect` (10) | list of lots | B | **no** | (addition) only lots that do **not intersect** the rect; uniqueness is checked against the whole list, including elements present before the call; returns the number appended in this call | `0x0072E2B0` |
| `cISC4ZoneManager::PlaceZone` (11), argument 10 `outLots` | `std::set<cRZAutoRefCount<cISC4Lot>>*` (the STLport tree of [docs/abi/allocators.md](../abi/allocators.md) *map and set*: 0x14-byte nodes, lot pointer at +0x10; SDK `intptr_t`) | B (nodes) | **no** | does not clear; inserts each lot to be demolished (unique); AddRef only on a new insertion. The caller releases each element and frees the nodes with B | `0x004DDCBB` → `0x00467950` → `0x00467720` |
| `cISC4ZoneManager::PlaceZone` (11), argument 1 `region` | `SC4CellRegion<long>&` (in/out) | C (row array and rows) | n/a | the game only clears the bits of rejected cells; it never reallocates or frees the region | `0x004DD780` |
| `cISC4ZoneDeveloper::HighlightParcels` (5), argument 4 `out` | `SC4CellRegion<long>*` stored into `*out` | C (object 0x24 bytes, row array, rows) | n/a | the game allocates a new region; with a highlight point it frees that region at the end and stores either the point's component (a new region) or null (§7.2 step 6). The caller owns the final region and frees rows, row array and object with C *[inferred: caller not traced]* | `0x00733A1C`, `0x00733E89`, `0x00733E95`, `0x00733891` |
| `cISC4LotManager::GetSafeLotIterator` (15) | 8-byte iterator object `{node, list}` | C | n/a | registered in the manager's iterator list (B node), advanced by lot removal; freeing not traced | `0x00734850` → `0x00734860`, `0x0072BA10` |
| `cISC4LotManager::CreateLot` (23), `CreateLotIfPossible` (22), argument `out` | `cISC4Lot*&` | — | n/a | one reference for the caller, who releases it; the manager keeps its own | `0x0072E520` |
| `cISC4LotManager::GetLot` (5, 6), `GetRandomLot` (13), `GetOccupantLot` (14) | `cISC4Lot*` | — | n/a | **no AddRef**; valid until the lot is deleted | `0x0072B410`, `0x0072B3C0`, `0x0072A110` |
| ZoningChange `0x07699455` void2 (posted by `PlaceZone`) | `SC4CellRegion<long>*` | C | n/a | owned by the message; its destructor `0x004DCC30` frees it with C; receivers must not free or keep it | `0x004DD780`, `0x004DCC30` |

Section (2), the implementer is called by the game. Columns as section (2) of `allocators.md`.

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence (game callers; original implementation) |
|---|---|---|---|---|---|
| `cISC4ZoneDeveloper::DoParcellization` (6), argument 1 | `const SC4CellRegion<long>&` | C | n/a | the zone manager passes the region it then attaches to ZoningChange (freed by the message with C); an implementation must neither free nor keep it | caller `0x004DD780`; original `0x00733FA0` |
| `cISC4ZoneManager::PlaceZone` (11) | ZoningChange void2 region it posts | C | n/a | a replacement zone manager must allocate the region with C in the §11.4 layout (the game's message destructor frees it), or post its own message class | consumers: growth developers `0x006B4B90`; original `0x004DD780` |

Section (3), returned by pointer, no AddRef: `cISC4ZoneManager::GetZoneGrid` (5, `0x00698CF0`) → the `int8` zone
grid; `cISC4LotManager::GetMaximumCapacityGrid` (27, `0x00809240`) → the `uint16` grid. Both are game grid objects
(simgrid §6.4, rule 6).

---

## 14. Quirks and open questions

### 14.1 Quirks (reproduced)

1. Undeveloped / abandoned counters are never updated after `Init` (§2).
2. `GetDevelopmentFailureCount` inserts zero entries that are then saved (§11.1).
3. `CreateLotIfPossible` ignores its configuration argument (§8.5).
4. `SimpleParcellize` and zone 7–9 remnants draw one random number per cell even where no lot can be made, and in
   the highlight preview (§9).
5. `ExamineBoundaries` examines the region's own edge cells at the city border (§7.3, test 31).
6. The master list order changes on save/load (§10.2).
7. `GetLotSavvyTerrainAltitude` rounds x/16 up above a fraction of 0.999 (§8.7).
8. DeferOccupantOperations is posted, not sent (§5.3).
9. `CreateLot` does not count a lot when the traffic simulator's iterator has no list (§8.5).
10. Re-zoning a cell to its own type posts ZoningChange and re-parcels it (§6).
11. `Parcellize` clamps a high-side depth start to 0, which changes the layout of regions in the first `Ld − 1`
    cells of the map (§7.6.1, test 26).
12. `Parcellize`'s cross-street range test is unsigned (§7.6.2).
13. `DrawStreet`'s fallback placements run to the line's end, not the run's end, so they can lay street outside
    the restriction region (§7.10).
14. `ExtendAsStreets` counts an avenue connection only on cells with `x + z` odd (§7.11).
15. `Parcellize` levels every attempted rect, also failed attempts, clamped to the city but not to the zone (§7.6.2).
16. In highlight mode `ExistsNetworkOfType` asks only the street tool when the flags include the street bit, so a
    dirt-road draft is invisible to `0x808` and `0x1F4F` tests (§8.8, test 33d).
17. `ExistsConnectionOfType`'s draft branch ignores the type, asks both tools whatever the flag bits, and reads byte 3
    for a side outside 0..3 (§7.13, test 33d).
18. `ClearHighlight` resets the street and road tools but not the dirt-road tool (§7.1).

### 14.2 Corrections to earlier notes

- `development.md` §2/§7: the dezone under a new network depends on the **network flags** `0x80200090`
  (`HasAnyNetworkFlag`), not an occupant group; it calls `PlaceZone` with type 0 free of charge.
- foundations §6.2: the zone manager's member `cRZRandom` is never drawn; add the zone developer's file-local
  `cRZRandom` `0x00B4B398` (XOR `0x8C09C68A`).
- `re/abi_interfaces.tsv`: the `cISC4LotManager` row (`00ab37c0`) belongs to class `cSC4ZoneDeveloper`.
- This spec, round 5 §7.6: the levelling call does not run "first" in each lot step; it runs after the attempt,
  its remnants and the back street (fixed in §7.6.2). §7.8: remnant strips are one cell **wide** along the facing
  direction, not one cell deep (fixed).
- This spec, §7.12: `LayInJogStreets`' parallel test does have a highlight-mode draft branch, for street and dirt road
  (fixed, ZQ4). §8.8: in highlight mode `ExistsNetworkOfType` asks the dirt-road tool only when the street bit is
  clear (fixed).

### 14.3 Open questions

1. *Answered in round 6* (§7.6, §7.10–§7.14, tests 19–29). Remaining unknowns of the street path:
   (a) the internals of the street tool's `PlaceNetworkOccupants` (when it fails, what it costs, whether it draws
   random numbers, whether it inserts occupants before returning *[inferred: yes]* and whether it auto-connects
   neighbouring networks; its draft records and how it lengthens a Place into a neighbouring network or draft are
   now in §8.8, ZQ11–ZQ12); the conformance tests below replace it by a stub; (b) the exact semantics of the network
   occupant's `ConnectsToNeighbor` (slot 47) for mixed network types in one cell.
2. `CanCreatePloppedLot` (slot 21) and the highlight texture code (UI).
3. Who frees the `GetSafeLotIterator` object, and with what.
4. Identity of `0x00731F60` / `0x00732450` (InsertOccupant data4 1/2 and 3) and of `0x0072B180` (end of `OnZone`).
5. Occupant type `0x2890D4DE` (counted as an obstacle by `ExamineCell`).
6. Message `0xA6B79602` name; the 3D-view global's slot 10 value.
7. Writers and readers of the maximum-capacity grid outside this class.
8. Dispatch order of InsertOccupant between the zone manager and the zone developer (inferred from Init order).

---

## 15. Conformance tests (must exist in `engine/conformance`)

City 64 × 64 unless stated; shipped tuning (§4); options `SetOptions(0, 0, 0)` unless stated.

1. **Zone types.** `IsRCIZone(t)` is true exactly for t = 1..9; `GetZoneType` of a cell zoned 9 returns EAX = 9;
   of a cell holding −1 (written raw) returns `0xFFFFFFFF`.
2. **Size errors.** `PlaceZone` of a 33 × 1 rect, type 1, `skipSizeCheck = 0` → false, error 4, no grid change. A 3 × 3
   rect of type 7 → error 3 (min 4). A 1 × 1 rect of type 14 → error 3 (min 2). Type 16 → false.
3. **Cost.** On 6 unzoned dry flat cells (3 × 2), `PlaceZone(type 1, commit, skipFunds = 0, checkSlope = 0)` with funds
   ≥ 60 → cost 60 (6 × (10 + 0)), funds −60, `zoneCount[1] = 6`, `zoneCount[0] = 4090`. Re-zoning them to 4 → cost
   66 (6 × (10 + 1)). Re-zoning them to 4 again → cost 0, counts unchanged, one ZoningChange posted. With funds 59
   and cost 60 → false, error 1, grid unchanged.
4. **Water.** A cell with water: type 1 → the cell's bit is cleared in the caller's region, others proceed; type 12
   → accepted; type 14 → error 2.
5. **ZoningChange.** After test 3's first call: exactly one queued message `0x07699455`, void1 = IGZUnknown = zone
   manager, data3 = 1, data4 = 0, void2 rect = the request rect with the 6 accepted bits set.
6. **Dezone under a network.** InsertOccupant of a network occupant without flags `0x80200090` whose bounding cells
   are `{10, 10, 12, 10}` over zone-1 cells without lots → those three cells become 0 at no cost; with flag bit 4
    set → no change. A cell of a 2 × 2 lot that extends beyond the rect is rejected (stays zoned) because
    `checkLotInside` is set.
7. **Failure count.** `GetDevelopmentFailureCount(16, 0)` = 0 and the map stays empty; `(0, 0)` = 0 and map 1 then
   holds one entry `{0, 0, 0}` (saved as count 1).
8. **GetOccupantLot cells.** Positions (x) 112.0 → cell 7; 104.0 → 6; 15.99 → 0; −0.01 → −1 (null).
9. **GetLotSavvyTerrainAltitude cells.** x = 15.98 → 0; 15.99 → 1; 16.0 → 1; −8.0 → −1 clamped to 0;
   127.99 → 8.
10. **CanCreateLot.** In real mode with current zone 0, on a 2 × 2 rect at (5, 5) zoned 1 with a lot on (6, 6) and a
    road on (5, 6): `CanCreateLot(5, 5, 2, 2, 0, 0, null)` = 3. Without the road = 7. Without the lot = 8. Unzoned
    cell inside = 2; `zoneNotNeeded = 1` → 8. A water cell with `waterAllowed = 0` = 1. A rect reaching x = 64 = 0.
11. **GetLotsInCellRect.** Lots A (1 × 1 at (2, 2)) and B (2 × 2 at (3, 2)); list pre-filled with one referenced
    lot C. `GetLotsInCellRect(list, 0, 0, 5, 5, false)` → returns 2, list = [A, B], C released once, A and B one
    extra reference each. With `true` → [A, B] (z outer, x inner: A's origin (2, 2) before B's (3, 2)).
12. **GetLotsSurroundingRect.** Rect `{3, 2, 4, 3}` (lot B) with A at (2, 2): flags `0xF` → [A] (count 1); flags `2`
    → [] (count 0); flags `8` → [A]. A call with a list already holding A → appends nothing, returns 0.
13. **ExamineBoundaries.** Region 3 × 3 at x 10..12, z 10..12, zone 4, `SetOptions(0, 1, 0)`; roads (bit 0) on
    (10..12, 9), streets (bit 3) on (13, 10..12), nothing else → S = {300, 90, 3, 3}, D = 0, Q = 1, flag = 1. Same
    with zone 1 → flag = 0, D = 1, Q = 0. Same with zone 4 and `alternateOrientation` → flag = 0, D = 1, Q = 0.
14. **Seeded parcellization (RNG).** Zone developer generator `lcg := 0x8C09C68A`. Region 2 × 2 at x 10..11,
    z 20..21, zoned 7 by `PlaceZone` (commit, skipFunds, skipSizeCheck); roads on (9..12, 19) and (12, 19..22); `SetOptions(0, 0, 0)`.
    `SimpleParcellize(16)` draws in cell order (10, 20), (10, 21), (11, 20), (11, 21): `U(4)` = 3, 0, 3, 0.
    Orders: 3,0,1,2 → side 0 has a road → facing 0; 0,1,2,3 → no road (the neighbours are lots) → facing 0 (front);
    3,0,1,2 → side 0 road → facing 0; 0,1,2,3 → side 1 (12, 21) road → facing 1. Result: four 1 × 1 lots with
    facings 0, 0, 0, 1; generator advanced exactly 4 times (next `NextRaw` = `0xF27EB69F`).
15. **GetRandomLot (RNG).** Generator `lcg := 1`, one 1 × 1 lot at (47, 37): draws (x, z) = (0, 22), (47, 37) →
    returns that lot after 4 draws. With no lots: returns null after 1998 draws.
16. **Remnant draws.** Dezoning (`PlaceZone` type 0, skipFunds) a cell under an empty (no configuration) 1 × 1
    agricultural lot consumes one draw and leaves no lot; dezoning a cell under an empty 2 × 2 residential lot
    consumes none.
17. **CreateLotIfPossible with facing −1.** Free 2 × 2 zoned rect, generator `lcg := 1`: one draw (`U(4)` = 0),
    no road → facing 0, returns 1 and `*out` holds one reference for the caller.
18. **Save round trip.** Zone manager record of a new 64 × 64 city: version 1, grid reference, `zoneCount[0] = 4096`,
    undeveloped 16 × 4096, unnamed 16 × 4096, `{4096, 0, 0, 0}`, tuning arrays as §4, alpha 170, `0`, `0`, five
    references. Zone developer record: version 1, 64, 64, 4096 references. After reload, the master list order is
    first-cell order.

**Parcellization geometry (tests 19–29).** Common setup unless stated: city 64 × 64, flat and dry, real mode, no
lots; roads (type 0) and streets (type 3) are laid as straight network runs whose cells connect to the neighbouring
cells of the same run; zones are written with `PlaceZone(commit, skipFunds, skipSizeCheck)` and parcelled by
`DoParcellization(zoned cells, type, 0)`. The lot-size parameters are forced to **width 2, depth 2, minDepth 1,
maxDepth 3, block 6** (the harness bypasses `DetermineLotSize`, §7.2.1). The street tool is a **stub**: Place(a, b)
always succeeds and immediately puts a street on every cell of a..b that has no network (cells keep their zone; the
posted dezone is not run). The expected output is the ordered log of lot creations (rect `x1, z1, x2, z2`, facing)
and Place calls; levelling calls are not logged. Lots are written `(x1, z1)–(x2, z2) fN`.

19. **6 × 4 low-density residential along a road** (worked example §7.14). Road (8..17, 9); zone 1 on x 10..15,
    z 10..13; `SetOptions(0, 1, 0)`. S = {600, 4, 6, 4}, C = {1, 0, 0, 0}, D = 3, Q = 0, flag 0. Log:
    (10, 10)–(11, 11) f1; Place(12, 10, 12, 13); (13, 10)–(15, 11) f3; (10, 12)–(11, 13) f1; (13, 12)–(15, 13) f3.
20. **Same with zone 4 (CS$): back street and forced middle cross street.** S, C as test 19; D = 0, Q = 3, flag 1
    (no swap). Middle row = 10. Log: (10, 10)–(11, 11) f0; (10, 12)–(11, 12) f2 (case (c): depth 1);
    Place(10, 13, 15, 13), then Place(x, 13, x, 13) for x = 10..15 (back street on the region edge, perCell);
    Place(12, 10, 12, 13) (forced cross street at row 10 + 2); (13, 10)–(14, 11) f0; (13, 12)–(14, 12) f2;
    attempt (15, 10)–(16, 11) fails (x = 16 not zoned) → remnant (15, 10)–(15, 11) f0; attempt (15, 12)–(16, 12)
    fails → remnant (15, 12)–(15, 12) f2. 6 lots, 8 Place calls.
21. **No automatic streets.** Test 19 with `SetOptions(0, 0, 0)`: C = {1, 1, 1, 1}, S = {600, 0, 0, 0}, D = 0,
    Q = 3, flag 1, farRoad. Log: (10, 10)–(11, 11) f0; (10, 12)–(11, 13) f2; (12, 10)–(13, 11) f0;
    (12, 12)–(13, 13) f2; (14, 10)–(15, 11) f0; (14, 12)–(15, 13) f2. No Place.
22. **L-shaped region.** Road (8..17, 9); zone 1 on {x 10..15, z 10..11} ∪ {x 10..11, z 12..15};
    `SetOptions(0, 1, 0)`. S = {600, 2, 2, 6}, D = 3, Q = 0, flag 0. Log: (10, 10)–(11, 11) f1;
    Place(12, 10, 12, 11) (the back street `DrawStreet((12, 10) → (12, 15))` clipped to the region);
    (13, 10)–(15, 11) f3; (10, 12)–(11, 13) f1; attempt (13, 12)–(15, 13) f3 fails with no remnant;
    (10, 14)–(11, 15) f1; attempt (13, 14)–(15, 15) fails with no remnant. No cross street (after the third row
    acc = 6 but t = 16 > b2 − 2 = 13). 5 lots.
23. **Jog street.** Street (12, 5..9) ending one cell north of the zone; zone 4 on x 10..15, z 10..13;
    `SetOptions(0, 1, 0)`. `ExtendAsStreets` finds nothing (no network inside the rect); `LayInJogStreets(3)`
    line 1 (z = 9): cell (12, 9) connects only on side 0 → dead end, no parallel connection → Place(10, 10, 15, 10)
    and Place(x, 10, x, 10) for x = 10..15. Component: x 10..15, z 11..13 (18 cells); S = {180, 3, 6, 3},
    C = {1, 0, 0, 0}, D = 0, Q = 3, flag 1. Lots: (10, 11)–(11, 13) f0; (12, 11)–(13, 13) f0; (14, 11)–(15, 13) f0
    (depth 3 by case (b): rem 3 ≤ maxDepth). No further Place.
24. **Extended street.** Street (12, 5..10) ending inside the zone rect; zone 4 on x 10..15, z 10..13 except
    (12, 10); `SetOptions(0, 1, 0)`. `ExtendAsStreets(3)`: (12, 10) connects only on side 0 → `DrawStreet((12, 10)
    → (12, 13), 0)` → Place(12, 11, 12, 13). `LayInJogStreets` draws nothing ((12, 9) connects on two sides).
    Components in first-cell order: x 10..11, z 10..13 then x 13..15, z 10..13. First: S = {2, 120, 2, 4},
    D = 1, Q = 2, flag 1; lots (10, 12)–(11, 13) f1, (10, 10)–(11, 11) f1 (rows from the high side, full-depth
    override). Second: S = {3, 4, 3, 120}, D = 3, Q = 2; lots (13, 12)–(15, 13) f3, (13, 10)–(15, 11) f3.
25. **Large block with back and cross streets.** Road (4..29, 9); zone 1 on x 10..21, z 10..19 (12 × 10);
    `SetOptions(0, 1, 0)`. S = {1200, 10, 12, 10}, D = 3, Q = 0, flag 0. Back streets (first row only):
    Place(12, 10, 12, 19); Place(17, 10, 17, 19); Place(21, 10, 21, 19) followed by Place(21, z, 21, z) for
    z = 10..19 (on the region edge). Cross street after the third row: Place(10, 16, 21, 16). Rows start at
    z = 10, 12, 14, 17 and 19. In each of the rows 10, 12, 14, 17 (o = row start): (10, o)–(11, o+1) f1;
    (13, o)–(14, o+1) f3; (15, o)–(16, o+1) f1; (18, o)–(19, o+1) f3 (case (b): rem 4 → 2); (20, o)–(20, o+1) f1
    (case (c): rem 2 → 1). In row 19 every attempt overhangs (z = 20 not zoned) and leaves a remnant one cell deep
    in z: (10, 19)–(11, 19) f1; (13, 19)–(14, 19) f3; (15, 19)–(16, 19) f1; (18, 19)–(19, 19) f3;
    (20, 19)–(20, 19) f1. Order: row 10's lots interleaved with its streets (lot (10,10), street x = 12, lot
    (13,10), lot (15,10), street x = 17, lot (18,10), lot (20,10), street x = 21), then rows 12 and 14, the cross
    street, rows 17 and 19. 25 lots.
26. **Map edge (start clamp).** Lot width 1, depth 3, minDepth 1, maxDepth 3; `SetOptions(0, 0, 0)`; road
    (2, 3..7); zone 4 on (0..1, 5). S = {0, 100, 0, 0}, C = {1, 1, 1, 1}, D = 1, Q = 2, flag 1. p0 = max(0, −1) = 0;
    full-depth override (Ldepth 2 ≤ 3, Lrow 1 < 2) → depth 2. Attempt (1, 5)–(2, 5) f1 fails (road) → remnant
    (1, 5)–(1, 5) f1; attempt (−1, 5)–(0, 5) f3 fails (outside) → remnant (0, 5)–(0, 5) f3. Two 1 × 1 lots (without
    the clamp it would be one (0, 5)–(1, 5) f1). Control: road (3, 3..7), zone on (1..2, 5) → p0 = 0 needs no clamp
    → one lot (1, 5)–(2, 5) f1.
27. **DepthRule** (§7.6.3), results for rem = 1..12, as (a) far/front, (b) far/back = (b) near/front, (c) near/back:
    - Ld 2, min 1, max 3, ps 1: (a) 1,2,3,2,2,2,2,2,2,2,2,2; (b) same; (c) 2,1,2,3,2,2,2,2,2,2,2,2.
      ps 0: (c) 2,1,2,2,2,2,2,2,2,2,2,2.
    - Ld 3, min 2, max 4, ps 1: (a) 1,2,3,4,3,3,2,3,3,3,3,3; (b) 1,2,3,4,2,3,3,3,3,3,3,3; (c) 3,1,2,3,4,3,3,3,3,3,3,3.
    - Ld 4, min 3, max 5, ps 1: (a) 1,2,3,4,5,3,4,4,5,3,4,4; (b) 1,2,3,4,5,6,3,4,4,4,4,4;
      (c) 4,1,2,3,4,5,3,4,4,4,4,4. ps 0: (a) 1,2,3,4,5,3,4,4,3,3,4,4; (b) 1,2,3,4,5,3,3,4,4,4,4,4; (c) as ps 1.
28. **DrawStreet.** Restriction region = x 10..15 on z = 5 minus (12, 5). `DrawStreet(9, 5, 16, 5, 1)` →
    Place(10, 5, 11, 5), Place(10,5,10,5), Place(11,5,11,5), Place(13, 5, 15, 5), Place(13,5,13,5), Place(14,5,14,5),
    Place(15,5,15,5); with perCell 0 → Place(10, 5, 11, 5), Place(13, 5, 15, 5). Region {10, 11, 13} on z = 5:
    `DrawStreet(10, 5, 13, 5, 1)` → Place(10, 5, 11, 5), (10,5,10,5), (11,5,11,5), Place(13, 5, 13, 5) (end cell);
    `DrawStreet(13, 5, 10, 5, 0)` → Place(13, 5, 13, 5), Place(11, 5, 10, 5); `DrawStreet(11, 5, 11, 5, x)` →
    Place(11, 5, 11, 5); `DrawStreet(12, 5, 12, 5, x)` → nothing. Fallback: region x 10..15 on z = 5, stub fails
    every segment containing (15, 5): `DrawStreet(10, 5, 15, 5, 0)` → Place(10, 5, 15, 5) fails, Place(11, 5, 15, 5)
    fails, Place(10, 5, 14, 5) succeeds, stop.
29. **Middle row.** `o0 = 10`, row step +2: row range 10..15 → mid 10; 10..17 → 12; 10..21 → 14; 10..11 → 10.
    In test 20 the forced cross street therefore follows the first row.

**`DetermineLotSize` (test 30, §7.2.1).** The harness calls `DetermineLotSize` directly. The current zone type is set,
the cropped region's rect is `{10, 10, 15, 13}` (centre (12, 11)), and the BDS, the growth developers and the lot
configuration manager are scripted. Every developer asserts that it is asked `EstimateDesirability(12, 11)`, and
every desirability map has tract shift 2 (k = 16). Building types B1…B9 are `0xB001`…`0xB009`; families
F1 = `0xF001` = {B1, B2, B3} and F2 = `0xF002` = {B5, B6, B7}. Profiles (purpose / wealth / capacity): B1 1/2/40,
B2 1/2/60, B3 1/1/500, B4 1/2/150, B5 1/2/300, B6 1/2/371, B7 3/2/90, B8 1/2/356, B9 1/2/237. Configurations
(compatible purposes and wealths, `GetGrowthStage`, the primary building object's ID list):

| ID | sx × sz | Purposes | Wealths | Stage | Object IDs |
|---|---|---|---|---|---|
| `0x101` | 2 × 2 | 1 | 2 | 1 | F1 |
| `0x102` | 2 × 3 | 1, 3 | 2 | 3 | B4, B7 |
| `0x103` | 3 × 3 | 1, 3 | 2 | 2 | F2 |
| `0x104` | 1 × 4 | 1 | 2 | 1 | B1 |
| `0x105` | 2 × 3 | 1 | 2 | 4 | B8 |
| `0x106` | 2 × 2 | 1 | 1 | 1 | B9 |
| `0x107` | 3 × 1 | 1, 3 | 2 | 1 | B5 |
| `0x108` | 3 × 2 | 1, 3 | 2 | 1 | none (slot 24 returns null) |
| `0x109` | 2 × 2 | 3 | 2 | 1 | B7 |

Developers (type: purpose / wealth, `EstimateDesirability`, `QueryActiveDemandValue`; lists valid and empty unless
stated): `0x1010` 1/1, 40, 812.9; `0x1020` 1/2, 40, 950.5, growth list [(3, 2, 4), (1, 1, 3), (0, 0, 1)] over
desirability tract values 55, 41, 40; `0x1030` 1/3, 39, 5000.0; `0x3110` 2/1, 10, 100.0; `0x3120` 2/2, 12, 300.9;
`0x3130` 2/3, 12, 400.0; `0x3320` 3/2, 13, 250.0, growth list [(0, 0, 2)] over tract value 20; `0x3330` 3/3, 13,
250.7. Results are written (width, depth, minDepth, maxDepth, block).

30. **`DetermineLotSize`.**
    - (a) Zone 1. R$ is taken first (40, 812). R$$ replaces it (equal desirability, 950 > 812); R$$$ does not (39 <
      40, whatever its demand). bestDemand 950. Stage walk: 55 > 40 → 4, 41 > 40 → 3, 40 ≤ 40 stops: stage 3. The
      set is {0x101, 0x102, 0x103, 0x104, 0x107, 0x108}: 0x105 (stage 4) and 0x106 (wealth 1) are filtered out, and
      either would win if admitted (scores 1 and 2). Step 6: 0x101 counts B1 and B2 (not B3, wealth 1): avg 50,
      `r24(16 / 4)` = 4.0, v 200, score 750. 0x102 counts B4 (not B7, purpose 3): avg 150, `r24(16 / 6)` =
      2.6666667 (`0x402AAAAB`), product 400.0, v 400, score 550. 0x103 counts B5 and B6: avg 671 / 2 = 335,
      `r24(16 / 9)` = 1.7777778 (`0x3FE38E39`), product 595.55554 (`0x4414E38E`), v 595, score 355. 0x104 (sz 4)
      and 0x107 (sz 1) fail the size test; 0x108 has no object. Chosen 0x103 → (3, 3). Widening over the whole set
      with width 3: 0x104 (sx 1) → maxDepth 4, 0x107 (sx 3) → minDepth 1, 0x103 and 0x108 inside, 0x101 and 0x102
      skipped (3 mod 2 = 1); [1, 4] needs no adjustment. Block 3 · 3. **(3, 3, 1, 4, 9).** The manager AddRefs each
      of the six configurations once, and each is released once.
    - (b) Zone 2, otherwise (a): block `ftol(r24(3 × 2.5))` = 7. **(3, 3, 1, 4, 7).**
    - (c) Zone 5 (shallow). CS$ (10, 100) is taken first; CS$$ replaces it (12 > 10); CS$$$ replaces it (12, 400 >
      300); CO$$ replaces it (13 > 12); CO$$$ does not (13, `ftol(250.7)` = 250, not > 250). Stage: 20 > 13 → 2 and
      the list ends: stage 2. Set {0x103, 0x107, 0x108, 0x109}. 0x103 fails the shallow test (sz 3); 0x109 counts
      B7: v 4 · 90 = 360, score 110 → (2, 2). Widening with width 2 keeps [2, 2] (0x103, 0x107 and 0x108 have
      sx 3), adjusted to [1, 3]. Purpose 3: block by density 2 → 6. **(2, 2, 1, 3, 6).**
    - (d) Zone 4, otherwise (c). 0x103 counts B7 only (B5 and B6 have purpose 1): product `r24(1.7777778 × 90)` =
      160.0, v 160, score 90 < 110 → (3, 3). Widening with width 3: 0x107 → minDepth 1, 0x108 inside, 0x109 skipped:
      [1, 3]. Block by density 1 → 7. **(3, 3, 1, 3, 7).** With CO$$'s lists invalid: stage 1, 0x103 leaves the
      set and 0x109 is chosen: **(2, 2, 1, 3, 7).**
    - (e) *[choice]* Tie. (a) with demands 99.9 (R$) and 300.0 (R$$): best R$$, bestDemand 300; scores 100 (0x101),
      100 (0x102), 295 (0x103). FreeCity4 takes the lower ID, 0x101 → (2, 2). Widening with width 2: 0x102 → 3,
      0x104 → 4, and 0x103, 0x107, 0x108 skipped (2 mod 3 = 2): [2, 4]. Block 6. **(2, 2, 2, 4, 6).** The original
      takes whichever its address-ordered set visits first; 0x102 first gives (2, 3, 2, 4, 9).
    - (f) Zone 7: no candidates, no BDS or manager call. **(2, 2, 1, 3, 5).**
    - (g) Pure functions. `v(k, n, avg) = ftol(r24(r24(k / n) × avg))`: (16, 4, 50) = 200; (16, 6, 150) = 400;
      (16, 9, 335) = 595; (256, 3, 49154) = 4194475; (4096, 3, 3074) = 4197035; (4096, 6, 6145) = 4194987;
      (4096, 9, 4616) = 2100793 (the last four are one above the exact floor). Depth adjustment of
      `[minDepth, maxDepth]`: [2, 2], [3, 3], [1, 2], [2, 3], [1, 1] → [1, 3]; [3, 4], [4, 4] → [2, 4];
      [5, 6] → [4, 6]; [1, 3] and [1, 4] unchanged. Residential density-2 block: depth 1, 2, 3, 4 → 2, 5, 7, 10.
31. **`ExamineBoundaries` at the city border** (§7.3, quirk 5). City 64 × 64, zone 4, `SetOptions(0, 1, 0)`, real
    mode; R is set directly as the restriction region (its rect is its bounding box). Results are S, C, D, Q, flag.
    - (a) Low edges. R = {x 0..2, z 0..2} without (0, 2) (8 cells); road (bit 0) on (3, 0..2); nothing else.
      - Side 0 (z1 = 0) examines R's own cells (0..2, 0): +1 each.
      - Side 3 (x1 = 0) examines its own cells (0, 0) and (0, 1): +1 each. (0, 2) is not in R, so it is not examined.
      - Side 2 examines (1, 3) and (2, 3).
      - Side 1 examines the road on (3, 0..2).

      **S = {3, 300, 2, 2}, C = {0, 1, 0, 0}, D = 1, Q = 0** (S[0] = 3 > S[2] = 2), **flag 1**. Skipping the
      off-map sides would give S[0] = 0 and Q = 2. Scoring them as obstacles would give C[0] = C[3] = 2.
    - (b) High edges. R = {x 61..63, z 61..63} without (61, 63); road on (61..63, 60).
      - Side 1 (x2 = 63) examines its own cells (63, 61..63): 3.
      - Side 2 (z2 = 63) examines its own cells (62, 63) and (63, 63): 2.
      - Side 3 examines (60, 61) and (60, 62): 2.

      **S = {300, 3, 2, 2}, C = {1, 0, 0, 0}, D = 0, Q = 1** (S[1] = 3 > S[3] = 2), **flag 1**. With zone 1
      instead, the 3 × 3 rect allows the swap: **flag 0, D = 1, Q = 0**.
    - (c) (a) with `SetOptions(0, 0, 0)`. C starts at 1, so no cell gets the +1: **S = {0, 300, 0, 0},
      C = {1, 1, 1, 1}, D = 1, Q = 2, flag 1**.
32. **`SimpleParcellize` street lines** (§7.5 step 1, every arm of the table). R = the full rect x 10..25,
    z 20..35 (16 × 16), n = 6, `placeStreets` set. The harness sets D and Q directly and logs the Place calls of step
    1, with the street stub of tests 19–29. A pass from the low edge lays streets at offsets 6 and 13 (x 16, 23;
    z 26, 33). A pass from the high edge lays them at x 19, 12 and z 29, 22, in that order. Every x-pass street is
    Place(x, 20, x, 35) and every z-pass street is Place(10, z, 25, z). All x-pass streets come first:

    | Q | D | x streets | z streets |
    |---|---|---|---|
    | 0 | 1 | 19, 12 | 26, 33 |
    | 0 | 3 | 16, 23 | 26, 33 |
    | 1 | 0 | 19, 12 | 26, 33 |
    | 1 | 2 | 19, 12 | 29, 22 |
    | 2 | 1 | 19, 12 | 29, 22 |
    | 2 | 3 | 16, 23 | 29, 22 |
    | 3 | 0 | 16, 23 | 26, 33 |
    | 3 | 2 | 16, 23 | 29, 22 |

    Edge rule: with x 10..17 (8 wide), neither direction lays an x street, because offset 6 is one cell from the far
    edge. With x 10..18, the low edge gives 16 and the high edge gives 12.
33. **Highlight mode** (§7.1, §7.2, §7.13, §8.8; ZQ4). The setup of tests 19–29, with the street stub extended for
    drafts:
    - A Place with realMode false records each cell of a..b that has no network as a **street draft**. Its network
      flags are `0x8`, and its mask has a nonzero byte for each edge towards a neighbouring cell of the same segment.
    - The stub answers `OccupantExists` from these drafts (§8.8), and `Reset` clears them.
    - A later draft Place on a drafted cell ORs its mask bytes into the cell's, as the original does (§8.8, ZQ11).
    - The dirt-road tool is a stub with no drafts.
    - The stub is simpler than the original in two ways (§8.8, ZQ12): it never drafts a cell that already has a
      network, and it never lengthens a Place into a neighbouring draft or network. No checked result below depends
      on either:
      - In (a), the cell beyond Place(12, 10, 12, 13) is the road at (12, 9). It is not a street, so the original
        does not lengthen the Place into it either, and its record never matches a street query.
      - In (e), the original lengthens the single-cell Place(12, 10, 12, 10) to (12, 9), the end of the real street.
        That cell becomes a draft with an edge towards (12, 10). Only `ExistsConnectionOfType` could see that edge.
        Every call to it (`ExtendAsStreets`, `LayInJogStreets`) comes before that Place, or finds no cell with a
        network of its kind.

    The results checked are:
    - the virtual-lot map and the virtually-deleted map (cell → `{x1, z1, facing}`);
    - the Place log;
    - `*out`;
    - the state after the call.

    These hold in every case of (a)–(c) and (e):
    - no real lot is created;
    - neither `AttemptAreaLevelling` nor `SetVerticesAreLocked` is called;
    - both DeferOccupantOperations messages are posted;
    - on return, real mode is true, the zone type is 0 and the point is null;
    - the maps and the street drafts remain.
    - (a) Test 19 as a preview: `HighlightParcels(x 10..15 × z 10..13, all set; type 1; null; &out)`, with or without
      the zone written (code 2 is real-mode only). S, C, D, Q and flag are as in test 19. The Place log is
      Place(12, 10, 12, 13) with realMode false. The virtual-lot map has 20 entries:
      - (10..11, 10..11) → {10, 10, 1};
      - (13..15, 10..11) → {13, 10, 3};
      - (10..11, 12..13) → {10, 12, 1};
      - (13..15, 12..13) → {13, 12, 3}.

      The virtually-deleted map is empty. `*out` is a new region equal to the input: rect {10, 10, 15, 13}, 24
      cells. Calling it again gives the same result, because the clear at the start empties the maps and resets the
      street stub.
    - (b) (a) with a real lot L, 2 × 2 at (10, 10), facing 0, whose configuration is not compatible with zone 1.
      - `CreateParcellizableRegion` reaches L at the first of its cells that it visits and calls `DeleteLot(L)`,
        once. The virtually-deleted map then holds (10..11, 10..11) → {10, 10, 0}.
      - At L's other cells, and in the second pass in `SubdivideTheZone`, `IsCellOccupied` is false.
      - None of the real-mode steps of §8.6 run: L stays in the cell map and in the master list.
      - `CanCreateLot` ignores the virtually deleted L (code 8). The virtual lots are exactly as in (a), including
        (10..11, 10..11) → {10, 10, 1}.
    - (c) Point filter. Test 21 (`SetOptions(0, 0, 0)`, road (8..17, 9)) with a second block. The input region is
      x 10..15 ∪ x 20..23, z 10..13 (rect {10, 10, 23, 13}), type 1, which gives two components.
      - P = (12, 11), inside block 1. Block 1 is parcelled as in test 21, giving 24 entries:
        - (10..11, 10..11) → {10, 10, 0};
        - (10..11, 12..13) → {10, 12, 2};
        - (12..13, 10..11) → {12, 10, 0};
        - (12..13, 12..13) → {12, 12, 2};
        - (14..15, 10..11) → {14, 10, 0};
        - (14..15, 12..13) → {14, 12, 2}.

        Block 2 is skipped (no `ExamineBoundaries`, no lots), and its 16 cells are cleared from `*out`. Then
        `*out` := block 1, cropped: rect {10, 10, 15, 13}, 24 cells.
      - P = (16, 11), outside both blocks but 4-adjacent to block 1's cell (15, 11). The same 24 entries, and
        block 2 is skipped. P's bit is clear, so `*out` := null.
      - P = (18, 11), next to neither block. Both blocks are skipped: there are no entries, and `*out` := null.
      - *[choice]* P = (5, 5), outside `*out`'s rect: as for (18, 11). The original reads outside the bit array here.
    - (d) Draft queries. The harness clears the real-mode flag directly, as `HighlightParcels` does. There are no
      real networks.
      - The street stub has a draft at (5, 5) with mask bytes (edges −x, −z, +x, +z) {1, 0, 1, 0}.
      - The dirt-road stub has a draft at (6, 6) with mask {0, 1, 0, 1}.
      - Both stubs log their calls.

      `ExistsNetworkOfType` gives:
      - (5, 5, `0x808`) → true, asking only the street stub;
      - (6, 6, `0x808`) → **false**, asking only the street stub, because bit 3 is set *[quirk]*;
      - (6, 6, `0x800`) → true, asking the dirt-road stub;
      - (5, 5, `0x1F4F`) → true, and (6, 6, `0x1F4F`) → false;
      - (5, 5, `0x1`) → false, with no stub call.

      `ExistsConnectionOfType` gives:
      - (5, 5, `0x8`, side s, 3) for s = 0, 1, 2, 3 → false, true, false, true (edges 1, 2, 3, 0), asking only the
        street stub;
      - (6, 6, `0x8`, 0, 3) → true: the street stub says no, so the dirt-road stub is asked with `0x800`, and its
        byte 1 is set;
      - (5, 5, `0x800`, 1, 0) → true: the street stub is asked first, and the type is ignored;
      - (6, 6, `0x808`, −1, 11) → true, because e = −1 reads byte 3; side 4 gives the same;
      - (5, 5, `0x808`, −1, 3) → false, because byte 3 = 0;
      - (5, 5, `0x1`, 1, 0) → false, with no stub call.

      With the real-mode flag set, every query above is false and no stub is called.
    - (e) Test 23 as a preview (`HighlightParcels`, type 4, no point). The Place calls are test 23's, all with
      realMode false: Place(10, 10, 15, 10), then Place(x, 10, x, 10) for x = 10..15. The parcellizable region and
      `ExamineCell` see the drafts on z = 10 through `ExistsNetworkOfType`. So the component and the scores are test
      23's: x 10..15 × z 11..13; S = {180, 3, 6, 3}, C = {1, 0, 0, 0}, D = 0, Q = 3, flag 1. There are 18 entries:
      - (10..11, 11..13) → {10, 11, 0};
      - (12..13, 11..13) → {12, 11, 0};
      - (14..15, 11..13) → {14, 11, 0}.

      No real street is laid. `*out` is the input region (rect {10, 10, 15, 13}).
