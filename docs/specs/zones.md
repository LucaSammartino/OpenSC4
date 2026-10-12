# Zones: zone manager (`cSC4ZoneManager`) and zone developer / lot manager (`cSC4ZoneDeveloper`) — behaviour spec

Status: M4 round 5, written from the Windows 1.1.641 executable (`bin/SimCity4.exe`). `docs/subsystems/development.md`
§7 (mostly Mac) was the starting point only; every fact below was checked on Windows. Scope: what growth depends on
(zone grid and queries, `PlaceZone`, the dezone on network insertion, parcellization into lots, the lot manager's
cell → lot map and lot lists, both save records, every slot of the interfaces). The zoning UI (drag colours, the
highlight preview, plopped lots) is described only as far as it shares state or random draws with growth.

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
| orientation result: flag, dominant side D, perpendicular side Q (§7.3) | 0, 0, 0 | no |
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

Exemplar "Zone Manager" `0xE9482490`, type `0xA52160F5`, group `0xE7E2C2DB`, read by `ReadSimulatorTunableValues`
(`0x004DC460`) at every `Init`. The two cost arrays are zeroed first. Each array property is read as **exactly 16**
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
if type ≥ 16 (unsigned): fail (code 0)
if !skipSizeCheck:
    w := x2 − x1 + 1;  h := z2 − z1 + 1
    if w < min[type] or h < min[type]: error 3, fail
    if w > max[type] or h > max[type]: error 4, fail
cost := 0 (int64);  accepted := [];  demolish := []        (lists, insertion order)
for x in x1..x2 (outer), z in z1..z2 (inner), only cells inside the city and set in the region:
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

`HighlightParcels(region, type, point*, SC4CellRegion** out)` (slot 5, `0x00733F30`, pop 16): UI preview. Sets the
real-mode flag false, stores type and point; if `IsRCIZone(type)`: clear the highlight, `OnZone(region, out)`,
update the highlight textures. Restores real mode. In highlight mode `CreateLotIfPossible` records **virtual** lots
instead of creating them, and `DeleteLot` records virtually deleted cells (§8.5, §8.6). Highlight mode still
**draws random numbers** (§9) *[compat]*.

### 7.2 `OnZone(region, out)` (`0x00733770`)

**Custom-size path** (option `customZoneSize`): the region becomes the restriction region for
`ExamineBoundaries` (§7.3) and is then cleared as restriction; the orientation list is `{D, Q, (D + 2) mod 4,
(Q + 2) mod 4}`; `CreateLotToFaceRoad(x1, z1, w, h, list)` (§7.7) on the whole region rect (no draw: the list is not
empty). On failure with `out` given, `*out` := a new all-false region over the rect (C).

**Normal path:**

1. Post DeferOccupantOperations (data1 = 1).
2. `CreateParcellizableRegion` (§7.4), cropped to its bounding box (`CompressRegion` `0x0072AF30`, new C region).
3. `DetermineLotSize` (§7.2.1).
4. If `placeStreets`: `ExtendAsStreets` (`0x00730BA0`) for network kinds 11, 3, 10, 0, 6 (dirt, street, one-way,
   road, avenue) in that order; if the cropped region is wider and deeper than 2 cells, also `LayInJogStreets`
   (`0x00730D30`) for the same kinds. Geometry not specified
   (§14.3). They draw streets into the region (through the network tools).
5. `SubdivideTheZone` (`0x0072EC60`): recompute the parcellizable region, split it into **4-connected components**
   (`ExtractConnectedCells` `0x0072CDA0`), each cropped and appended to a component list (B nodes; one empty region if
   there are none).
6. For each component in list order: it becomes the restriction region. In highlight mode with a highlight point,
   components not containing or 4-adjacent to the point are skipped (their cells are cleared from `*out`).
   Otherwise `ExamineBoundaries` (§7.3) and then by the current zone type: 1..6 → `Parcellize` (§7.6); 7 →
   `SimpleParcellize(16)`; any other → `SimpleParcellize(6)` (§7.5). The component region is freed.
7. In real mode, `0x0072B180(region)` (not traced; *[inferred: UpdateZoneType-like bookkeeping, no RNG]*).
8. Post DeferOccupantOperations (data1 = 0).

#### 7.2.1 `DetermineLotSize` (`0x00732BF0`)

Chooses the lot width/depth used by `Parcellize` from the developers that would build in this zone.

1. Centre `(cx, cz) := ((x1 + x2) / 2, (z1 + z2) / 2)` of the cropped region (C integer division).
2. Candidate developers (BDS slot 32 `GetDeveloper(type)`) by zone: 1–3 → `0x1010`, `0x1020`, `0x1030` (R$, R$$,
   R$$$); 4 → `0x3110`, `0x3120`, `0x3130`, `0x3320`, `0x3330` (CS$, CS$$, CS$$$, CO$$, CO$$$); 5–6 → the same five
   with the *shallow* flag set; other zones → none.
3. Best developer: the first candidate; a later candidate replaces it if its `EstimateDesirability(cx, cz)` (int16,
   `growth-developer.md` §4.6) is greater, or equal and `ftol(GetSourceDemand().QueryActiveDemandValue())` is greater
   (`demand.md` slot 12). Keep `bestDemand := ftol(best.demand value)`.
4. Stage := 1. If the best developer's growth/decline lists are valid, walk its growth list in order: stop at the
   first item whose desirability-map tract value at `(item.x, item.z)` is ≤ the best desirability; otherwise
   stage := `item.maxStage`.
5. Lot configurations: lot configuration manager (`0x00B43D18`) slot 9 with a filter (vtable `0x00AB36FC`) of
   `{purpose = best.GetPurpose(), wealth = best.GetWealth(), stage}` into an STLport `hash_set` (100 → 193 buckets);
   iterated in hash order.
6. For each configuration with a primary building object whose size `(sx, sz)` satisfies `2 ≤ sz ≤ 3`, `sx ≤ 3`
   and (not shallow or `sz ≤ 2`): over the building types of each family of the object (BDS slot 19, profiles by
   slot 39) with matching purpose and wealth, take `min` and `max` of the u16 capacity. If any was found:
   `k := (1 << s) << s` with s the desirability map's tract shift (cells per tract);
   `v := ftol(r24(r24(float(k) / float(sx · sz)) × float((min + max) / 2)))` (integer division; the product is
   unsigned); `score := |v − bestDemand|`. Keep the configuration with the smallest score (strict `<`: first wins).
7. Lot width/depth := the chosen `(sx, sz)`, else (2, 2). `minDepth := maxDepth := depth`; then over all
   configurations with `width mod sx = 0`, widen `[minDepth, maxDepth]` by `sz`. Every configuration is released.
   If `maxDepth − minDepth < 2`: `minDepth := (minDepth > 2) ? maxDepth − 2 : 1`; `maxDepth := minDepth + 2`.
8. Block depth by zone density (`0x00458960`: 1 low, 2 medium, 3 high) when the best developer's purpose is 1
   (residential): low `3 · depth`, medium `ftol(r24(depth × 2.5))`, high `2 · depth`; otherwise 7, 6, 5.

No random draws. Hash-set order depends only on the configuration IDs.

### 7.3 `ExamineBoundaries` (`0x00731C80`) and `ExamineCell` (`0x0072FAB0`)

Scores the four sides of the restriction region R to decide which side lots face.

- Scores `S[0..3] := 0`; conditions `C[0..3] := placeStreets ? 0 : 1`.
- The outside rect is R's rect grown by one cell, but **not past the city edge** (at the edge the "outside" coordinate
  stays on R's own edge).
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
   draw a street (`DrawStreet` `0x0072A2A0`, whole line across the rect) and reset the counter; otherwise
   counter += 1. So streets are separated by n lot lines.
2. For **every cell of the restriction rect**, x ascending (outer), z ascending (inner), regardless of the region's
   bits: `CreateLotToFaceRoad(x, z, 1, 1, emptyList)` (§7.7) → **one random draw per cell** (§9). The created lot's
   extra reference is released. Cells that cannot hold a lot (not zoned with the current zone, street, water, lot …)
   still consume their draw.
3. Free the restriction region.

Agricultural and industrial zones therefore always become 1×1 lots; growth later aggregates them
(`tract-developer.md` §4.15).

### 7.6 `Parcellize` (`0x0072EDD0`): residential and commercial

Uses the restriction region, D/Q and flag (§7.3) and the lot-size parameters (§7.2.1). Lots of `width × depth`
are laid in rows from the road side, with edge adjustments using `minDepth`, `maxDepth`, block depth and halving /
thirding of left-over strips; jog streets are drawn with `DrawStreet` when `placeStreets`. Each lot attempt calls
`CreateLotIfPossible` with an **explicit facing** (no random draw); a failed attempt falls back to
`CreateLotRemnants(rect, facing)` (§7.8; no draw for zones 1–6). In real mode every attempt first calls
`lotDeveloper.AttemptAreaLevelling(&clampedRect, 0, 1, 1, FLT_MAX, 0)` (slot 21). Created lots get
`SetVerticesAreLocked(1)` (lot slot 69) and are listed; at the end each is unlocked (0) and released.

The exact row geometry is **not specified here** (§14.3). The Windows structure matches the Mac function
(`000e365a`); a conforming implementation must reproduce the cell order of lot attempts, but no random draws
depend on it.

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
- otherwise: strips one cell deep, perpendicular to the facing side, with the list `{facing}` (no draw). Scan order
  by facing: 3 → rows of z, x ascending; 1 → rows of z, x descending; 0 → columns of x, z ascending; 2 → columns of
  x, z descending. A run of consecutive buildable cells (inside the city and the restriction region, not water, zoned
  in real mode, no `0x1F4F` network, not occupied) becomes one lot.

### 7.9 `OnDezone(region, flag)` (`0x0072E6A0`)

1. Post DeferOccupantOperations (data1 = 1).
2. For each set cell, z outer, x inner, with a lot: current zone := the lot's zone; if `IsRCIZone` or 11 or 12:
   `DeleteLot`, then `CreateLotRemnants(lotRect, lotFacing)` (§7.8). Since the grid already says 0, no lot is
   recreated on dezoned cells, but zones 7–9 still consume one draw per cell of the deleted lot *[compat]*.
3. If `flag`: collect the set cells that have a `0x808` network (street, dirt) and no `0x1747` network into a region;
   if any, `demolition.DemolishRegion(1, &region, 1, 1, 0, 0, 0, 0, 0, 0, 0)` (demolition slot 6, 11 arguments).
4. Post DeferOccupantOperations (data1 = 0).

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
| 10 | `GetLotsSurroundingRect(rect, list, flags)` → count | walks the ring one cell outside the rect (`0x0060CEE0`); side bits 1 = row z1−1, 2 = column x2+1, 4 = row z2+1, 8 = column x1−1; bits for sides outside the city are cleared. `flags = 0xF` → every ring cell; otherwise a ring cell needs its side's bit (a corner needs both). Appends each lot **that does not intersect the rect**, once (linear search of the whole list, including old elements), one AddRef each. Does not clear. Returns the number appended in this call |
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
argument is ignored**. If code = 8: in highlight mode record a virtual lot `{x, z, facing}` for every cell and return
AL = 1 (upper EAX undefined); else if `facing = −1`, `CreateLotToFaceRoad` with a fresh empty list (**one draw**,
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

`DeleteLot(lot)` (slot 24, `0x0072F8E0`, pop 4; EAX undefined). Highlight mode: record the lot's cells as
virtually deleted `{x1, z1, facing}`; nothing else. Real mode, in order:

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
  `HasAnyNetworkFlag(flags)`. In highlight mode it also accepts the street tool's draft (if `flags & 8`) or the
  dirt-road tool's draft (if `flags & 0x800`) at the cell (network tool slot 9).
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
dezone under new networks, §5.1) and the zone developer's InsertOccupant handler (§5.2). `Parcellize` (zones 1–6) and
`CreateLotRemnants` for zones 1–6 never draw.

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
| `cISC4ZoneManager::PlaceZone` (11), argument 10 `outLots` | `std::set<cRZAutoRefCount<cISC4Lot>>*` (MSVC tree; SDK `intptr_t`) | B (nodes) | **no** | does not clear; inserts each lot to be demolished (unique); AddRef only on a new insertion. The caller releases each element and frees the nodes with B | `0x004DDCBB` → `0x00467950` → `0x00467720` |
| `cISC4ZoneManager::PlaceZone` (11), argument 1 `region` | `SC4CellRegion<long>&` (in/out) | C (row array and rows) | n/a | the game only clears the bits of rejected cells; it never reallocates or frees the region | `0x004DD780` |
| `cISC4ZoneDeveloper::HighlightParcels` (5), argument 4 `out` | `SC4CellRegion<long>*` stored into `*out` | C (object 0x24 bytes, row array, rows) | n/a | the game allocates a new region; when it narrows the result to the highlighted component it frees its first region and stores a new one. The caller owns the final region and frees rows, row array and object with C *[inferred: caller not traced]* | `0x00733A1C`, `0x00733E89`, `0x00733891` |
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
5. `ExamineBoundaries` examines the region's own edge cells at the city border (§7.3).
6. The master list order changes on save/load (§10.2).
7. `GetLotSavvyTerrainAltitude` rounds x/16 up above a fraction of 0.999 (§8.7).
8. DeferOccupantOperations is posted, not sent (§5.3).
9. `CreateLot` does not count a lot when the traffic simulator's iterator has no list (§8.5).
10. Re-zoning a cell to its own type posts ZoningChange and re-parcels it (§6).

### 14.2 Corrections to earlier notes

- `development.md` §2/§7: the dezone under a new network depends on the **network flags** `0x80200090`
  (`HasAnyNetworkFlag`), not an occupant group; it calls `PlaceZone` with type 0 free of charge.
- foundations §6.2: the zone manager's member `cRZRandom` is never drawn; add the zone developer's file-local
  `cRZRandom` `0x00B4B398` (XOR `0x8C09C68A`).
- `re/abi_interfaces.tsv`: the `cISC4LotManager` row (`00ab37c0`) belongs to class `cSC4ZoneDeveloper`.

### 14.3 Open questions

1. Exact geometry of `Parcellize`, `ExtendAsStreets`, `LayInJogStreets`, `DrawStreet` (cell order of lot attempts
   and street cells). No random draws depend on them.
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
