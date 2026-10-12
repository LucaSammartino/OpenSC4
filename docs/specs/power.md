# Spec: the power subsystem (`cISC4PowerSimulator`)

Status: **implementation spec**, clean-room input, round 2 (M3 grid simulators). This document describes
*behaviour*, not code. It was written by the analysis team from the original program (Windows build 1.1.641) and
its data files. The earlier write-up in `docs/subsystems/grid-sims.md` §4 was the starting point. Every fact
below was checked against the Windows executable; corrections to the earlier write-up are listed in §14.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* consistent with all evidence
but not proven; *[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must
survive).

Related specs, cited and not repeated: [foundations.md](foundations.md) (streams §2, references §2.7, class store
§3, city component table §3.6, Init order §3.7, response curves §4 and the four missing-property patterns §4.1,
messages §5, random generators §6, C `rand()` and `random_shuffle` §12, month order §13, slot-order rule §16,
framework slot tables §17); [simulator.md](simulator.md) (agents, levels, finished semantics);
[simgrid.md](simgrid.md) (grid classes §2, geometry §3, grid slots §4, grid persistence §5, lifetime and the
**grid ownership rule §6.4**, random cell iterator §7.4); [budget.md](budget.md) (departments, line items,
budget items, local funding); [neighbors.md](neighbors.md) (deals, connections, status reports);
[ordinance.md](ordinance.md) (cumulative factors); [pollution.md](pollution.md) (radiation, WTE capacity);
[weather.md](weather.md) §6.7 (random cell iterator).

---

## 1. Role and identity

The power simulator decides which cells of the city are **powered**. It keeps:

- a **wire** bitmap: cells that conduct power (building footprints, power poles and lines, some network pieces,
  and zoned cells, each widened by a radius R);
- a **consumption map**: the power requirement of every consuming building, stored at the building's centre cell;
- a list of **power plants** and their per-plant state (wear, power used), stored on the plant occupants as
  properties;
- a list of **power poles** (for infrastructure cost and pole decay);
- the **powered** result, as a bitmap and as a `uint8` grid (0 unwired, 1 wired but unpowered, 2 powered) that
  the map view and the tract developer read.

A recompute (§6) flood-fills each connected wire network from its plants (and from neighbour connections that
import power), sums the plants' output and the imports, sells exports, then powers 8 × 8-cell blocks in random
order while the supply lasts. Each month it also ages the plants (§7.4), applies the plants' maintenance cost
curve (§7.3) and breaks or repairs poles according to department funding (§7.6).

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`) | `0x0990C005` (verified: the factory and `GetGZCLSID` both use it) |
| Primary interface | `cISC4PowerSimulator`, IID `0xE9E7BFF5`; QI also answers `0x00000001` (cIGZUnknown) with it |
| `cIGZMessageTarget2` | IID `0x452294AA` (separate sub-object of the same object) |
| `cIGZSerializable` | IID `0xE4FDA3D4` (separate sub-object of the same object) |
| Not answered | everything else → false, out pointer untouched |
| City component | slot **#35** (foundations §3.6); the city gets it with `GetClassObject(0x0990C005, 0xE9E7BFF5)` |
| City getter | `cISC4City` slot 67 (`+0x10C`) `GetPowerSimulator`, cached by the game as `SL::spPowerSim` |
| Init order | after the ordinance simulator, before plumbing and neighbours (foundations §3.7) |
| Object | one object, one reference count. `Release` with count ≤ 1 calls `Shutdown()`, destroys the object and returns 0; otherwise it decrements and returns the new count *[compat]* |

## 2. Grids, maps and coordinates

The power simulator works in **cells** (tract size 1). Its slots 5–8 convert world positions to cells with the
city's cell widths (§10.1).

| Map | Representation | Saved | Outside readers |
|---|---|---|---|
| powered grid | `cSC4SimGrid<uint8>` (class `0x49B9E602`), tract size 1, instance ID `0x49D5BC86` | **ref** (§9) | map view (`GetGridData`, raw), tract developer (`GetCellValue`), anyone through slots 20/35/36 |
| consumption map | `vector<vector<int16>>`, `[x][z]`, `CellCountX` rows of `CellCountZ` | inline (§9) | exposed raw by slot 18 (no outside caller found) |
| mark, wire, powered-current, powered-next | 1-bit bitmaps (§2.1) | inline (§9) | none (slots 37/38 read them) |

**Powered grid values** *[compat]*: for each cell, `wired ? (poweredCurrent ? 2 : 1) : 0`. A cell that is
powered but not wired reads 0. The tract developer treats any value `< 2` as "no power" (§11.1).

### 2.1 Bitmaps *[compat]*

A bitmap holds one bit per cell in 32-bit words. Bit index `i = width · z + x` with `width = CellCountX`; word
`i >> 5`, bit `x & 31` (LSB first). The word count is `(CellCountX · CellCountZ) / 32` (signed division). The
"clear" and "xor" helpers process `width²` bits, i.e. they assume a **square** city *[quirk]*; every shipped city
size is square.

*Layout confirmed* *[compat]*: this is the original's layout, not a transcription error. The shared 1-bit helpers
(`Bit1_PixelTest` `0x00603DD0`, `Bit1_Clear` `0x00603E00`, rect set `0x00604180`, rect clear `0x00604260`,
rect test `0x00604350`, xor `0x00603E20`; used by power and plumbing) compute the word as `(width·z + x) >> 5`
but the bit as `1 << (x & 31)`, not `i & 31`; `InitMarkAndWireBits` (`0x006EAED0`) allocates
`(CellCountZ · CellCountX) / 32` words (signed division) and clears `CellCountX²` bits. The two bit rules agree
whenever `width` is a multiple of 32, which every SC4 city size is (64, 128, 256 cells). For other widths cells
would share bits and the last partial word would not be allocated; this cannot happen with real cities.
*[choice]* FreeCity4 may use `i & 31` and `ceil(w·h / 32)` words, which is identical for all reachable sizes.

Rectangles are inclusive `{x1, z1, x2, z2}`. The helpers used (their exact semantics are part of
§6 and the report to simgrid §7):

| Helper | Behaviour |
|---|---|
| clear | all bits 0 |
| pixel test / set | one bit |
| rect set / rect clear | all bits in the rectangle |
| rect test any | true if any bit in the rectangle is set |
| rect or | `dst |= src` inside the rectangle |
| xor | `dst ^= src` over the whole bitmap |
| selection grow | flood fill, below |

**Selection grow** `(mark, mask, width, maxIterations = 600, &rowMin, &rowMax)` *[compat]*: repeats, at most 600
times, a *horizontal* pass then a *vertical* pass over the rows of an **active band**:

- horizontal pass: for each row of the band, every cell whose left or right neighbour (same row, no wrap across
  rows) is marked becomes marked if it is set in `mask` (Jacobi update: the pass reads the bits as they were
  before the pass);
- vertical pass: every cell of the band whose neighbour in the row above or below is marked becomes marked if
  set in `mask` (Jacobi update).

Connectivity is therefore **4-neighbour**. The first band is `[rowMin − 1 (only if rowMin > 0), rowMax + 1
(only if rowMax < width − 1)]`. After each iteration the band becomes `[lowest changed row − 1 (if > 0), highest
changed row + 1 (if < width)]`, and while something changed, `rowMin := min(rowMin, band low)`, `rowMax :=
max(rowMax, band high)`. It stops when an iteration changes nothing or after 600 iterations. The row bound uses
`width` (square-city assumption again). A seed bit outside `mask` stays marked; only grown bits need `mask`.
*[quirk]* A wire path longer than about 600 horizontal-plus-vertical steps is cut off; in a 256-cell city no
4-connected path needs more iterations than that in practice *[inferred]*.

### 2.2 Radius and neighbourhoods

`R` = tuning `0xA911E2F1` (§8). **Expand**`(rect)`: `x1 := x1 > R ? x1 − R : 0` (unsigned), `x2 := min(x2 + R,
CellCountX − 1)`, the same for z with `CellCountZ`. The **neighbourhood of a cell** is the expanded one-cell
rectangle.

## 3. State

| Field | Initial (constructor / new-city `Init`) | Persisted | Meaning |
|---|---|---|---|
| initialized | false | no | first `Init` part done (§4.1) |
| running | false | no (but `Read` sets it) | per-city part of `Init` done; set by `Read` so a loaded city skips it |
| enabled | false | no | set by ConnectionsReady `0x6AC284F3`; `DoIdle` does nothing until then |
| services | null | no | city, budget simulator, power department, occupant manager, zone manager, neighbours simulator, demolition (§4.1) |
| power department | found or created (§4.1) | **ref** | department `0x8910BC8A` |
| ringIndex | 0 | yes | 0..11 |
| production, consumption (pass) | 0 | yes | totals of the current recompute cycle |
| import, export, demand (pass) | 0 | no | totals of the current recompute cycle |
| last production, consumption, import, export, demand | 0 | yes | copied at the end of each cycle (§6.6); getters 12–15, 17 |
| deficit (`SC4Percentage`, float) | 0 | yes | §6.6, §7.5; no getter |
| consumers | 0 | yes | number of consuming buildings (§5.1) |
| powered count | 0 | yes | consuming cells powered in the last cycle (§6.4) |
| unpowered count | 0 | yes | `max(0, consumers − powered)` at cycle end |
| requirement (u32) | 0 | yes | sum of the consuming buildings' requirements |
| costBase (int64) | 0 | yes | pole cost sum (§7.3) |
| powered grid | created in `Init` | ref | §2 |
| consumption map | `Init` sizes it | yes | §2 |
| ring[12][10] (int32), annual[10] | 0 | yes | per plant type power used per cycle; `annual = Σ ring` (§7.5) |
| plant list (visited state + occupant) | empty | yes | §5.1; `visited` ∈ {0, 1, 2} |
| plantCount, perType[10] | 0 | yes | |
| pole list (references), poleCount, brokenCount | empty, 0, 0 | yes | §5.1, §7.6 |
| mark, wire, powered-current, powered-next bitmaps | `Init` allocates and clears | yes | §2.1 |
| unused flag (u8) | 0 (new-city `Init`) | yes | never read *[inferred]* |
| strike flag (u8), strike start (s32) | 0 | yes (flag reset by `Read`) | strikes are disabled (§10.1) |
| recompute, agePlants (u8) | 0 (new-city `Init`) | yes | requests (§6) |
| wireDirty, dirty rectangle | false | no | pending wire rebuild (§5.3) |
| changed | false | no | set when the powered set changed (§6.5) |
| state (u32) | 4 (new-city `Init`) | yes | `DoIdle` step (§6) |
| overworked, aged, distressed counts | 0 | no | plant statistics of the last ageing (§7.4) |
| brokenFraction (f32) | 0 | **no** | pole decay accumulator (§7.6) |
| ordinance factor (f32) | 1.0 | no | `GetCumulativeFactor(0x0911E117, 0)` (§8) |
| tuning scalars, 5 curves | §8 | no | |
| a `cRZRandom` member | wall-clock seed | no | **never used** |

## 4. Lifecycle

### 4.1 `Init()` → true (AL = 1)

1. Take (and `AddRef`) the city (`SL::spCity`) and from it the occupant manager, zone manager, neighbours
   simulator and budget simulator; take the demolition utility (global). Look up the power department with
   `budget.GetDepartmentBudget(0x8910BC8A)` and `AddRef` it if found.
2. If not `initialized`: set it; read the tuning (§8); register the agent (§4.3); set the history queue length to
   2 for the nine per-type history IDs (§9.2); subscribe (in this order) to `0x99EF1142`, `0x99EF1143`,
   `0x07699455`, `0xCA1B151C`, `0x278E6003`, `0xC2C09554`, `0xE8BDA2E7`, `0xEA418D8F`, `0x6AC284F3`,
   `0x26D31EC1`; if the consumption map does not have `CellCountX` rows, resize it to `CellCountX` rows of
   `CellCountZ` zeros.
3. If not `running` (a **new** city; `Read` sets `running`): set it; clear the unused flag, recompute, agePlants
   and wireDirty; allocate and clear the four bitmaps (§2.1); if there is no powered grid, create one (class
   `0x49B9E602`), `AddRef`, `Init`, `SetTractSize(1)`, `SetInstanceID(0x49D5BC86)`, `SetTractValues(0)`; zero
   ring and annual; state := 4; if no department was found: `budget.CreateDepartmentBudget(0x8910BC8A,
   0x4A357EAF)`, `AddRef`, `SetDepartmentName(0x6A3FF01C, 0x4A493980)`, `SetMaxAllowedFundingPercentage(maxFunding)`
   (§8), `CreateLineItem(1, false)` and on it `SetName(0x6A3FF01C, 0x2A75F757)`.
4. Return true.

*[compat]* After a load, step 3 is skipped: the bitmaps (including the wire bitmap), the grid, the state and the
flags come from the save; nothing is rebuilt. *[choice]* FreeCity4 creates the grid through GZCOM per simgrid
§6.4 (the original constructs it directly).

### 4.2 `Shutdown()` → true (AL = 1)

If `running`: clear it; release the city, occupant manager, demolition, zone manager, department and budget;
unsubscribe the ten messages; `RemoveAgent(target)` on the simulator. If `initialized`: clear it; release every
plant and free the list; for every pole call `BreakAllConnections(false)` (pole slot 6) and release it; clear the
pole list; poleCount := brokenCount := 0; free the four bitmaps; `Shutdown` and release the grid; release the
neighbours simulator; clear the consumption map.

### 4.3 Agent registration and cadence *[compat]*

`AddAgent(target, L, "cSC4PowerSimulator", flags 2)` for L = **6** (month), **4** (day), **10** (idle), in that
order. The power simulator is the 8th month-level registration and runs **first** at month level in the
original's pointer order (foundations §13.2); power runs before the neighbours simulator in both orders. It reads
the ordinance factor (refreshed in its own month handler) and the current neighbour deals; nothing it reads at
month time is produced by a later month agent in the same month *[inferred]*.

| Message (level) | Action |
|---|---|
| `0x66956816` SimNewMonth (6) | `DoMonthlySimulation` (§7.5) |
| `0x66956814` SimNewDay (4) | `DoIdle` — one step |
| `0x6695681A` SimIdle (10) | `DoIdle` — one step (the time budget in data3 is ignored) |

`DoMessage` always returns **true** (finished) for every message *[compat]*.

## 5. Messages handled

| Message | Action |
|---|---|
| `0x99EF1142` occupant inserted | `DoOccupantInserted(void1)` (§5.1), only if `initialized` |
| `0x99EF1143` occupant removed | `DoOccupantRemoved(void1)` (§5.2) |
| `0x07699455` zoning changed | rect = the four `int32` at `void2`; `CellChanged(rect)` (§5.3); then the city's network manager `GetNetworkTool(5, false)` (the power-line tool): `AddRef`, `Init`, `UpdateOnZoneChange(rect by value, data3)`, `Shutdown`, `Release` |
| `0xCA1B151C` tuning reload | re-read the tuning (§8) |
| `0x278E6003` terrain heights changed | ignored if data4 is 4 or 5; else rect `x1 = (data1 >> 16) − 1`, `x2 = data1 & 0xFFFF`, `z1 = (data2 >> 16) − 1`, `z2 = data2 & 0xFFFF`, clamped to the city; `DoTerrainHeightsChanged` (§5.4) |
| `0xC2C09554` OrdinanceUpdated | if data2 = `0xE0D0722E` (Nuclear Free Zone) and data3 ≠ 0: §5.5 |
| `0xE8BDA2E7` kMessageStateChanged | void1/IGZUnknown QI `cISC4Lot` (`0xE818FCED`); if `lot.GetState() == 1`: centre = `((x1 + x2) / 2, (z1 + z2) / 2)` of `lot.GetBoundingRect` (signed integer division); if `IsPowered(centre)` and `lotManager.GetLot(cx, cz, false)` is not null: that lot's `SetHasPower(true)` |
| `0xEA418D8F` deal status | if data1 = 0 and the IGZUnknown is a deal (QI `0x0773FEF1`) of type `0x4773FFE0` or `0x4773FFE1`: state := 4, recompute := true |
| `0x6AC284F3` ConnectionsReady | enabled := true |
| `0x26D31EC1` PostCityInit | §5.6 |

### 5.1 `DoOccupantInserted(occ)` *[compat]*

Dispatch on `occ.GetType()` (occupant slot 7):

- **Power pole** `0x2890D4DE`: QI `cISC4PowerPoleOccupant` (`0xA8C86142`). If pole slot 13 returns true, ignore
  the pole entirely *[inferred: a preview or temporary pole]*. Else: cell := `pole.GetCell` (slot 8); set the wire
  bits of the cell's neighbourhood; keep the reference in the pole list; poleCount += 1; costBase += int64
  property `0x09132DE8` of the pole; recompute := true.
- **Power line** `0x0990B2EA`: QI `cISC4PowerLineOccupant` (`0xC98F3B24`). If `IsPreview()` ignore. Else wire the
  neighbourhood of `GetCell()`; recompute := true. Lines are not kept.
- **Network occupant** `0xC772BF98`: QI `cISC4NetworkOccupant` (`0xA821EF94`). Skip it if
  `HasNetworkFlag(0x80000000)`. It conducts only if `HasNetworkFlag(0x40000)`: then wire its single
  `GetOccupiedCell()` (**not** widened by R); recompute := true. Flag meanings unknown *[inferred]*.
- **Building** `0x278128A0`:
  - centre cell = `trunc(((min + max) × 0.5) / CellWidth)` per axis from `GetBoundingBox` (world units);
  - wire bits: `GetBoundingCityCells` expanded by R;
  - `consumed` = u32 `0x27812854`, `produced` = u32 `0x27812852` (both default 0);
  - if `consumed > produced` (a **consumer**): `consumption[cx][cz] += (int16)consumed` (16-bit wrap);
    consumers += 1; requirement += consumed; if `IsPowered(centre)` and the building has a lot: `lot.SetHasPower(true)`;
  - if `produced > consumed` (a **plant**): `AddRef`; append a node `{visited 0, occ}`; plantCount += 1;
    `perType[type] += 1` with `type` = s32 `0x27812853` (default 0; **no bounds check** *[quirk]*);
    recompute := true. Then the budget hookup: `GetBudgetItemForPurpose(holder, 0xCA58E540)`; if found and the
    occupant is a `cISC4BuildingOccupant` (QI `0x87DDA3A9`): `dept := budget.GetDepartmentBudget(item.department)`
    (null not checked); if `item.line == 0`: `line := building.GetBuildingType()` and
    `budget.ChangeBudgetItemLine(holder, item.index, line)`; if `line ≠ 0`: `li := dept.GetLineItem(line)`, or if
    none `dept.CreateLineItemForBuildingType(line, true)` followed by `li.SetDisplayFlag(2, true)`;
    `dept.AddLocallyFundedObject(holder, 0xCA58E540)`; `li.SetSecondaryInfoField(li.GetSecondaryInfoField() + 1)`
    (the plant count shown in the budget window); set int64 property `0x2A58D350 := item.cost` (the base
    maintenance, §7.3).
  - Equal consumed and produced (including both 0): only the wire bits.
  - *[quirk]* There is no preview-marker (`0x029244DB`) test, unlike the other simulators.

### 5.2 `DoOccupantRemoved(occ)` *[compat]*

- Pole: the same slot-13 test; mark the neighbourhood dirty (§5.3, without the zone test); find it in the pole
  list: release, erase, poleCount −= 1; costBase −= `0x09132DE8`; recompute := true.
- Power line / network occupant: the same filters as insertion; mark dirty (line: neighbourhood; network: the
  single cell); recompute := true.
- Building: if `consumed > produced` and `consumed > 0`: `consumption[cx][cz] −= consumed`; consumers −= 1
  (clamped at 0); requirement −= consumed (clamped at 0). Always mark `GetBoundingCityCells` expanded by R dirty.
  If the budget item is found: `dept.GetLineItem(item.line)`, `dept.RemoveLocallyFundedObject(holder, 0xCA58E540)`
  and `li.SetSecondaryInfoField(Get − 1)`. If the occupant is in the plant list: release, unlink, perType[type]
  −= 1, plantCount −= 1, recompute := true.

### 5.3 Dirty wiring (`CellChanged`, `UpdateWireBits`) *[compat]*

`CellChanged(rect)`: expand the rect by R; if `zoneManager.GetZoneType(x1, z1) ≠ 0` (the rect's first corner
only *[quirk]*) set its wire bits; merge it into the dirty rectangle (union if wireDirty, else replace), set
wireDirty and recompute. Removals use the same merge without the zone test.

At the start of every enabled `DoIdle`, if wireDirty: clear the wire bits in the dirty rectangle, then rebuild
them (`UpdateWireBits(rect)`), then clear wireDirty:

1. Ask the occupant manager for the occupants in the rect expanded by 2R (`GetOccupantsByStandardCityCells`, all
   types). For each: building → its bounding cells expanded by R; network occupant whose `GetNetworkFlag()` has
   `0x40000` → its cell; pole → its neighbourhood (**no** slot-13 test here *[quirk]*); line → its neighbourhood.
2. For every cell of the rect (x outer, z inner) with `GetZoneType ≠ 0`: wire its neighbourhood.
3. Rewrite the powered grid for every cell of the rect (`SetTractValue`, values of §2).

### 5.4 `DoTerrainHeightsChanged(rect)`

Poles in the rect: `BreakAllConnections(true)` (slot 6), then `occupantManager.RemoveOccupant(pole, true, 0)`.
Each distinct power line in the rect whose two ends are connected is detached from its poles through the pole
class's non-virtual `RemoveConnection` (§11.2). The removals then come back as `0x99EF1143` (§5.2).

### 5.5 Nuclear Free Zone *[compat]*

Collect the lots (`lotManager.GetOccupantLot`) of every plant with type `0x27812853 = 5` (nuclear) and demolish
them: `demolition.DemolishLots(true, list, 0, 1, false, null, null, 0)`. Then, if the 3D view's current input
control has ID `0x88F154FB` (control slot 5), `RemoveCurrentViewInputControl(false)` *[inferred: the placement
tool for a nuclear plant]*.

### 5.6 PostCityInit

Every pole whose occupant (`QI 0x26BB767E`) answers `IsFlagSet(1)` = false (not in the occupant manager) is
released and erased, poleCount −= 1. Then `RecomputeInfrastructureCosts`: costBase := Σ over the pole list of the
**low 32 bits** of `0x09132DE8` (high dword 0); if costBase changed, recompute := true.

## 6. The recompute cycle (`DoIdle`) *[compat]*

`DoIdle` does nothing until enabled. After the dirty-wiring step (§5.3) it performs **one** state per call:

| State | Step | Next |
|---|---|---|
| 4 | idle; if recompute: recompute := false | 0 (else stay) |
| 0 | reset (§6.1) | 1 |
| 1 | power the plants' networks; costs; ageing (§6.2, §7.3, §7.4) | 2 |
| 2 | power from neighbours; publish (§6.5) | 3 |
| 3 | statistics, history, deal flags (§6.6) | 4 |

A new city starts in state 4 with recompute false, so nothing runs until the first change or the first month
(which sets recompute). After a load the saved state continues.

### 6.1 State 0 — reset

`ringIndex := (ringIndex + 1) mod 12`; for t in 0..9: `annual[t] −= ring[ringIndex][t]; ring[ringIndex][t] := 0`.
For every buy-power (`0x4773FFE0`) and sell-power (`0x4773FFE1`) deal: `SetAmountUsed(0)`. Every PowerPole
connection (`GetConnectionsOfType(4)`): `SetVisitedThisCycle(false)`. Zero the pass counters (production,
consumption, export, import, demand) and the powered and unpowered counts. Every plant: visited := 0. Clear
powered-next; changed := false.

### 6.2 State 1 — plants

`total := costBase`. For each plant in list order: if visited = 0, `UpdatePowerGrid(PositionToCell(occ.GetPosition()))`
(§6.3). Then the plant's costs (§7.3). After the loop: `dept.GetLineItem(1).SetFullExpenses(total)`. If
agePlants: zero the overworked, aged and distressed counts, agePlants := false, and age every plant (§7.4).

### 6.3 `UpdatePowerGrid(x, z)` — one connected network

1. Clear mark; set bit (x, z); selection grow (§2.1) with mask = wire, `rowMin = z`, `rowMax = z + 1`; clamp
   rowMax to `CellCountZ − 1` (and reset an inverted range).
2. **Supply**: for every plant with visited = 0 whose cell rectangle (`GetBoundingBox` → `PositionToCell` of min
   and max) has any marked bit: `supply += GetCurrentPowerProduced(plant)` (§7.1, u32), visited := 1.
   production += supply.
3. **Import capacity**: `imp := Σ (amount − used)` over buy deals with `used < amount` whose other city is
   connected to this network (`IsNeighborConnected(city, markVisited = true, light = false)`, §6.4).
4. `total := imp − trunc(supply_u32 × (−1.1f))` (so supply counts at 110 %, the hard-coded production ratio).
   `avail := trunc(total_u32 / factor)` with `factor` = the ordinance factor (§8).
5. **Exports**: for each sell deal with `used < amount` and `IsNeighborConnected(city, false, light = avail > 0)`:
   `give := min(amount − used, avail)` (signed); if `give > 0`: exports += give, avail −= give,
   `SetAmountUsed(used + give)`; otherwise `deal.SetFlags(0x20, true)`.
6. **Blocks**: a random cell iterator (weather §6.7; **prime-step mode, one `Reset` with shuffling → exactly one
   C `rand()` call**, foundations §12.4) over the block rectangle `bx ∈ [0, max(0, CellCountX/8 − 1)]`, `bz ∈
   [rowMin >> 3, max(rowMin >> 3, rowMax >> 3)]`. For each block, over its 8 × 8 cells (x outer, z inner) that
   are marked: `need := Σ consumption[x][z]` over cells with consumption > 0, `n` := their count; demand +=
   need. If `avail ≠ 0` and `need ≤ avail` (unsigned): powered-next |= mark ∩ block, poweredCount += n, avail −=
   need. Else shortfall := true (later, smaller blocks may still be powered).
7. If shortfall: avail := 0. `leftover := trunc(avail_u32 × factor)`; `used := total − leftover`; consumption +=
   used.
8. **Imports**: for each buy deal with `used_d < amount` and `IsNeighborConnected(city, false, false)`:
   `take := min(amount − used_d, remaining)` (signed, `remaining` starts at `used`); imports += take;
   remaining −= take; `SetAmountUsed(used_d + take)`.
9. `MarkPowerPlantsAsVisited(remaining, max(supply, remaining))` (§6.4).

*[quirk]* On a shortfall the whole remaining availability counts as consumed. A block with zero need is powered
only while `avail ≠ 0`.

### 6.4 Helpers of §6.3

**`IsNeighborConnected(city, markVisited, light)`** → bool: for each PowerPole connection to `city`
(`GetConnectionsOfTypeToCity(4, city)`), take its connection cell clamped to the map; if that cell is marked:
if markVisited, `SetVisitedThisCycle(true)`; if light, set the cell's powered-next bit; return true. None → false.

**`MarkPowerPlantsAsVisited(a, b)`**: `ratio := production(pass) > 0 ? f32(u32 a / u32 b) : 0` (stored as f32).
For each plant with visited = 1: `used := trunc(GetCurrentPowerProduced × ratio)`; set s32 property
`0x08D9763D := used`; `ring[ringIndex][type] += used`; visited := 2. *[quirk]* With `b = 0` and a positive pass
production (another network produced), the ratio is NaN and a zero-output plant gets `used = 0x80000000`.

### 6.5 State 2 — neighbours and publishing

For each buy deal with `used < amount`: repeatedly take `conn := FindUnvisitedConnection(4, deal.city)` until null;
`SetVisitedThisCycle(true)`; if `FindConnectionOnOtherSide(conn)` exists and its `GetIsActive()` is true:
`UpdatePowerGrid(connection cell, clamped)`. Then swap powered-current and powered-next and run
**UpdatePoweredLots**:

- `next ^= current` (the changed cells);
- for each 8 × 8 block (bz outer, bx inner, both over `0 .. (CellCountX >> 3) − 1`) with any changed bit: for
  each changed cell (x outer, z inner) rewrite its grid value (§2); post `0x8A07AE00` with data1..data4 = the
  block's `x1, z1, x2, z2`; changed := true.

If changed: post `0x6A07C2EE` (no data). *[quirk]* Cells whose wiring changed but whose power did not are not
rewritten here (only §5.3 does that).

### 6.6 State 3 — statistics

1. last production/consumption/import/export/demand := the pass values.
2. History (`RecordHistoryData`): `0xAA11F7CE` ← production; `0x0AB2F224` ← import; `0x4AB2F236` ← export;
   `0xAA11F7D2` ← `trunc(r24(int32(consumption − export) × factor))`; `0xAA11F7D5` ← `trunc(r24(int32(demand) ×
   factor))` (one x87 multiply by the `float32` ordinance factor, rounded to 24 bits per foundations §18, then
   `_ftol`; `0x006EF293`–`0x006EF2B8`);
   `0xAA11F7D8` ← production − consumption.
3. Sell deals with `used < amount`: `SetFlags(0x20, true)`. Then `neighbours.MarkDealsDirty()`.
4. If consumption > 0: `0xAA11F7DC` ← `budget.GetFunding(dept)` (int64, slot 35) / consumption (int64 division).
   Per budget.md that getter always returns 0, so the value is 0 *[quirk]*.
5. `0xAA11FB59` ← consumers > 0 ? `trunc((1.0 − powered / consumers) × 100.0)` (double, no clamp) : 0.
6. Per type t = 1..9: history `0xAA120D5E, 0xAA120D61, 0xAA120D63, 0xAA120D65, 0xAA120D66, 0xAA120D68,
   0xAA120D6A, 0xAA120D6B, 0xAA120D6D` ← `ring[ringIndex][t]`. Then `annual[t] += ring[ringIndex][t]` for
   t = 0..9.
7. deficit := production > consumption ? `f32(int((production − consumption) / production))` (integer
   division) : 0. *[quirk]* This is 1.0 only when consumption = 0, else 0.
8. unpowered := max(0, consumers − powered).
9. Every PowerPole connection: `SetIsActive(IsPowered(its clamped connection cell))`.
10. state := 4; post `0xEA418D8F` with data1 = 8 (neighbors.md §5.3).

## 7. Plants and poles *[compat]*

### 7.1 Output

`GetCurrentPowerProduced(occ)` = `trunc(f32(u32 0x27812852) × efficiency(occ))`; 0 if the occupant is null or has
no `0x27812852`. No upper clamp: funding above 100 % gives more than the nominal capacity.

**Efficiency** (slot 45) = `(item found ? curve0x6911E2DF(item.localFunding × 100) × 0.01 : 0) × condition(occ)`
where `item = GetBudgetItemForPurpose(holder, 0xCA58E540)`. **Condition** (slot 46) = if the occupant has curve
property `0x8A1E07EE`: that curve (built fresh per call) at x = f32 `0xC910B4CF` (the plant's **wear**, default
0); else 0. Null occupant → 0. *[quirk]* A plant without `0x8A1E07EE` produces nothing.

### 7.2 Per-occupant queries

| Query | Result |
|---|---|
| `GetOccupantPowerConsumed(occ, &v)` | needs a non-null occupant (not checked); if `0x27812854` exists: `v := its u32`, true; else false |
| `GetOccupantPowerProduced(occ, &prod, &used)` | if `0x27812852` exists: `prod := GetCurrentPowerProduced`, `used := s32 0x08D9763D` (0 if absent), true; else false, nothing written |
| `GetOccupantMaxPowerProduced(occ, &v)` | the property helper's result for `0x27812852` |
| `IsOccupantPowered(occ)` | `lot := lotManager.GetOccupantLot(occ)`; none → false; `lot.GetHasPower()` → true; else `!lot.GetNeedsPower()` |
| `GetMonthlyPowerGenerationCost(occ, &c)` | budget item found → `c := item.cost` (int64), true; else false |
| `GetPowerPlantFundingPercentage(occ, &p)` | item found → `p := item.localFunding`, true |
| `SetPowerPlantFundingPercentage(occ, p)` | item found → `budget.GetDepartmentBudget(item.department).SetLocalFundingPercent(holder, p, 0xCA58E540)`; returns its result |

### 7.3 Costs (state 1, per plant)

- Fuel: `total += trunc(s32 0x08D9763D × f32 0xC910B4CE)` (int64). `0xC910B4CE` is not shipped, so 0.
- Maintenance, only if the plant has curve property `0x0911E2E7`: `maint := trunc(int64 0x2A58D350 ×
  curve0x0911E2E7(wear))`. If the budget item is found and `maint ≠ item.cost`: `dept.SetLocalFullFunding(holder,
  maint, 0xCA58E540)` and then `budget.ChangeBudgetItemCost(holder, item.index, maint)` (redundant, budget §7.4
  already changes it). `0x2A58D350` keeps the base cost, so the multiplier does not compound.
- Poles: `costBase` = Σ int64 `0x09132DE8` per pole (shipped 1 each). The line item 1 gets `costBase + fuel` as
  full expenses.

### 7.4 Ageing `AgePowerPlant(occ)` (state 1 when agePlants)

Inputs: wear = f32 `0xC910B4CF` (default 0), rate = f32 `0xEA1CF220` (default 0), capacity = s32 `0x27812852`,
used = s32 `0x08D9763D`, t1 = f32 `0xEA1CF221` (default 99), t2 = f32 `0xEA1CF222` (default 99), destroy = u32
`0x6A43150F` (default 0).

1. `effCap := trunc(capacity × efficiency)`; `usage := (effCap > 0 and used > 0) ? used / effCap : 0`.
2. `wear += curve0x0911E2E9(usage) × curve0x0911E2E8(departmentFunding) × rate`, written back to `0xC910B4CF`
   (`departmentFunding` = the department's `GetFundingPercentage()` as a fraction).
3. usage > 1.0 → overworked += 1. wear ≥ 0.8 → aged += 1.
4. If wear ≥ t2 or wear ≥ t1: `lot := lotManager.GetLot(PositionToCell(position), false)`; no lot → stop.
5. If wear ≥ t2 and destroy > 0: `DemolishLots(true, [lot], 0, destroy | 0x300, false, null, null, 0)`; if f32
   `0xAA5C04C8` > 0: `pollution.AddRadiationToCity(&position, strength = 0xAA5C04C5, radius = 0xAA5C04C8)`; if the
   plant type is 5: post `0xAA552F77` (data 0, IGZUnknown = the plant; the safety advisor listens).
6. Else if wear ≥ t1: distressed += 1; `lot.SetCondition(2)`.

*[quirk]* `used` comes from §6.4 with `ratio ≤ 1`, so `usage ≤ 1` and the overworked count is practically always 0.

### 7.5 Month (`DoMonthlySimulation`)

1. factor := `ordinance.GetCumulativeFactor(0x0911E117, 0)`.
2. `ringIndex := (ringIndex + 1) mod 12`; `annual[t] −= ring[ringIndex][t]`, `ring[ringIndex][t] := 0`, then
   `annual[t] += ring[ringIndex][t]` (adds 0). *[quirk]* With the advance in state 0 the index moves twice per
   monthly cycle, so `annual` covers about six cycles, not twelve months.
3. deficit := (consumption > production > 0) ? `f32(int((production − consumption) / production))` : 0 (pass
   values; non-zero only when consumption ≥ 2 × production).
4. recompute := agePlants := true.
5. Pole funding (§7.6) with the department's funding percentage.

### 7.6 Pole decay `ApplyPoleFunding(pct)`

`brokenFraction := clamp(brokenFraction − curve0xAA713782(pct × 100), 0, 1)`; `target := trunc(u32 poleCount ×
brokenFraction)`. If target ≠ brokenCount: copy the pole list, **`random_shuffle`** it (C `rand()`, foundations
§12.3: poleCount − 1 calls), then for each pole while brokenCount ≠ target: if `IsDistressed()` and brokenCount >
target: `SetDistressed(false)`, brokenCount −= 1; else if not distressed and brokenCount < target:
`SetDistressed(true)`, brokenCount += 1. Distressed poles still conduct (§5.3 ignores the state) *[inferred:
visual only]*. *[quirk]* brokenFraction is not saved, so after a load it restarts at 0.

## 8. Tuning *[compat]*

**"f32 × 0.01" properties** *[compat]*: the value is read with the float getter and multiplied by the `float32`
constant 0.01f (`0x00A823A4`, `fmul dword`, e.g. `0x006ED5E8`), rounded to 24 bits (foundations §18). 0.01f is
slightly below 0.01, so several shipped percentages land one unit below the decimal value: 120 → 1.1999999
(`0x3F999999`), not 1.2. The budget clamps the department's funding to this maximum (budget §7.1, `SetPct`:
`q := clamp(q, 0, pMax)`), so the highest power funding a player can set is 1.1999999. Code defaults are the `float32` literals
shown (1.2 = `0x3F99999A`).

Exemplar type `0x6534284A`, group `0xE7E2C2DB`, instance **`0xC911E35B`** ("Utilities", SimCity_1.dat). Read at
`Init` and on `0xCA1B151C`. Every read first restores the code defaults; curves are rebuilt from code-default
points and then loaded by exemplar key → foundations §4.1 pattern **default** for all five curves.

| Property | Name (re/tuning) | Type | Code default | Shipped | Use |
|---|---|---|---|---|---|
| `0xA911E2F1` | Power radiation radius | u32 | 2 | 2 | R (§2.2) |
| `0x6911E2DF` | Power Funding To Efficiency Curve | curve | (0,0)(100,100)(120,110) | (0,0)(50,25)(90,80)(100,100)(120,110) | §7.1 |
| `0x0911E2E8` | Funding % → decay rate multiplier | curve | (0,1)(1,1) | (0,1.1)(1,1)(1.2,0.9) | §7.4 |
| `0x0911E2E9` | Usage % → decay rate multiplier | curve | (0,1)(1,1) | (0,0.1)(0.25,0.2)(0.5,0.45)(0.75,0.85)(0.9,1.25)(1,1.55)(1.1,2) | §7.4 |
| `0xAA713782` | Funding % → pole decay/repair | curve | (0,−0.02)(120,0.01) | (0,−0.02)(25,−0.005)(50,0)(100,0.025)(110,0.05) | §7.6 |
| `0xE911E2D1` | Power max allowed funding | f32 × 0.01 | 1.2 | 120 → **1.1999999** (`0x3F999999`) | `SetMaxAllowedFundingPercentage` (new city) |
| `0x09122027` | Utility Strike Efficiency % | f32 × 0.01 | 0.5 | 20 → 0.19999999 (`0x3E4CCCCC`) | only exposed by slot 47 |
| `0xE911E2FC` | Power plant max production ratio | f32 × 0.01 | 1.1 | 110 → 1.1 | **dead** (§6.3 uses a literal 1.1) |
| `0x29121E8B` | Power Min Acceptable Funding % | f32 × 0.01 | 0.6 | 60 → 0.59999996 | dead |
| `0x2912202E` | Utility Strike Duration | s32 | 10 | 12 | dead |
| `0x49122036` | Utility Min Funding to End Strike | f32 × 0.01 | 0.9 | 0.9 | dead |
| `0x0911E2E6` | Utility Strike Response Curve | curve | (0,20)(100,100) | (0,0)(20,45)(50,75)(100,100) | dead |

The ordinance factor is refreshed at every tuning read and every month: `GetCumulativeFactor(0x0911E117, 0)`
(Power Conservation 0.9 when enacted, ordinance.md).

**Per-building properties** (roles from the code; names from the community property list `new_properties.xml`
where it has one; "—" = not listed there, role name ours *[inferred]*). The list's `Default` column is the
editor default, **not** the code default used here:

| Property | Name | Type | Role / code default | Shipped examples |
|---|---|---|---|---|
| `0x27812852` | Power Generated | u32 | capacity; 0 | coal 6000, H2 50000, gas 3000, nuclear 16000, oil 7000, solar 5000, WTE 5000, wind 200; civic buildings 1 |
| `0x27812854` | Power Consumed | u32 | requirement; 0 | |
| `0x27812853` | Power Plant Type | read as s32 | plant type index 0..9; 0 | 1 coal, 2 hydrogen, 3 gas, 5 nuclear, 6 oil, 7 solar, 8 WTE, 9 wind |
| `0x08D9763D` | — | s32 | power used last cycle (written) | |
| `0xC910B4CF` | — | f32 | wear / age (written; "condition" in pollution.md); 0 | starts 0 |
| `0xC910B4CE` | Power plant energy conversion cost | f32 | fuel cost per unit used; 0 | not shipped |
| `0xEA1CF220` | Age degradation rate | f32 | rate; 0 | ≈ 0.000833–0.001667 |
| `0xEA1CF221` | Soft failure threshold | f32 | t1; 99 | 0.9 |
| `0xEA1CF222` | Hard failure threshold | f32 | t2; 99 | 1.0 |
| `0x6A43150F` | Hard Failure Type | u32 | demolition flags at t2; 0 | 3 coal/gas/oil/H2, 4 nuclear, 0 solar/wind, absent WTE |
| `0xAA5C04C5` / `0xAA5C04C8` | Radiation Spew Intensity / Radius | f32 | radiation at t2; 0 | nuclear 3200 / 15 |
| `0x8A1E07EE` | Age to output level response curve | curve | wear → condition | (0,1)(0.4,0.95)(0.5,0.92)(0.6,0.86)(0.7,0.77)(0.8,0.6)(0.9,0.35)(1,0); gas 0.88 at 0.6 |
| `0x0911E2E7` | Age to maintenance cost multiplier response curve | curve | wear → maintenance multiplier | (0,1)(0.4,1.03)(0.5,1.05)(0.65,1.15)(0.75,1.3)(0.85,1.55)(0.9,1.85)(1,3) |
| `0x2A58D350` | — | int64 | base maintenance (written at insertion) | coal 250, WTE 1000, solar 1000, H2 10000, gas 400, wind 50, oil 600, nuclear 3000 |
| `0x09132DE8` | Power pole monthly upkeep | int64 | pole infrastructure cost | 1 |

The two per-building curves are loaded into a fresh temporary curve from the property only when the property
exists; otherwise the caller takes its "no curve" path (condition 0; no maintenance update): foundations §4.1
pattern **delete** (per call).

**Random numbers** (foundations §6.2): no `cRZRandom` draw. C `rand()`: one call per `UpdatePowerGrid` (the
block iterator reset) and `poleCount − 1` calls per pole-funding change (`random_shuffle`).

## 9. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x0990C005`) *[compat]*

### 9.1 Record (version 6)

```
u16   version = 6
ref   department                      (read IID 0x223E2B67)
s32   ringIndex
s32   production (pass)
s32   consumption (pass)
f32   deficit                         (version < 6: s32, value × 0.01)
s32   consumers
s32   powered
s32   unpowered
s32   requirement
s64   costBase
s32   last production, consumption, import, export, demand
ref   powered grid                    (read IID 0xC9BD939A)
u32   nx, u32 nz
nx × nz × s16  consumption map        (x outer, z inner)
120 × s32      ring                   (month-major: ring[0][0..9], ring[1][0..9], …)
10 × s32       annual
s32   plantCount
10 × s32       perType
per plant node in list order: u32 visited, ref occupant   (read IID 0x26BB767E; plantCount nodes are read)
u32   poleCount
u32   brokenCount
poleCount × ref pole                  (read IID 0xA8C86142)
u32   words
words × u32  mark, then wire, then powered-current, then powered-next
u8    unused flag
u8    strike flag
s32   strike start
u8    recompute
u8    agePlants
u32   state
```

- `Write` always returns true (AL = 1); it does not check the stream error. It writes as many plant nodes as the
  list holds, with the separate `plantCount` before them *[quirk: they agree in practice]*.
- `Read` sets `running` first; a version > 6 returns false after the version only (`running` stays set). Older
  versions read the deficit as an integer percentage. It allocates the bitmaps with the saved word count, resets
  the strike flag to 0 and **always returns true** (AL = 1), ignoring stream errors.
- Recomputed on load: nothing. The tuning, the ordinance factor, enabled, wireDirty and brokenFraction are reset by
  construction/`Init`.
- The powered grid is saved by `ref` as its own object (simgrid §5).

### 9.2 History IDs

`0xAA11F7CE` production, `0x0AB2F224` import, `0x4AB2F236` export, `0xAA11F7D2` consumption net of export,
`0xAA11F7D5` demand, `0xAA11F7D8` surplus, `0xAA11F7DC` cost per unit, `0xAA11FB59` unpowered %, and the nine
per-type IDs of §6.6 (queue length set to 2 at `Init`).

## 10. Interface slot tables *[compat]*

All methods are `__thiscall`; "pop" is the number of argument bytes the callee removes (checked with
`tools/abi_extract.py --check`); return channels read from the epilogues.

### 10.1 `cISC4PowerSimulator` (vtable `0x00AB28B8`, 52 interface slots + destructor)

| Slot | Off | Method (as compiled) | Pop | Return | Behaviour |
|---|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface(iid, void**) | 8 | AL | §1 |
| 1 | `+0x04` | AddRef | 0 | EAX count | |
| 2 | `+0x08` | Release | 0 | EAX count | §1 |
| 3 | `+0x0C` | Init | 0 | AL = 1 | §4.1 |
| 4 | `+0x10` | Shutdown | 0 | AL = 1 | §4.2 |
| 5 | `+0x14` | PositionToCell(f32 x, f32 z, s32& cx, s32& cz) | 16 | none (garbage) | `trunc(x × (1 / CellWidthX))`, same for z; no bounds check |
| 6 | `+0x18` | CellToPosition(s32 cx, s32 cz, f32& x, f32& z) | 16 | none (`fstp`) | `x = CellWidthX × cx` (cell corner) |
| 7 | `+0x1C` | CellToStandardCityCell(s32, s32, s32&, s32&) | 16 | none | identity copy |
| 8 | `+0x20` | StandardCityCellToCell(s32, s32, s32&, s32&) | 16 | none | identity copy (same body as 7) |
| 9 | `+0x24` | GetAnnualConsumption | 0 | EAX u32 | Σ annual[0..9] |
| 10 | `+0x28` | GetAnnualConsumptionByPowerType(s32 t) | 4 | EAX | annual[t], no bounds check |
| 11 | `+0x2C` | GetMonthlyRequirement | 0 | EAX | requirement |
| 12 | `+0x30` | GetMonthlyProduction | 0 | EAX | last production |
| 13 | `+0x34` | GetMonthlyImport | 0 | EAX | last import |
| 14 | `+0x38` | GetMonthlyExport | 0 | EAX | last export |
| 15 | `+0x3C` | GetMonthlyDemand | 0 | EAX | last demand |
| 16 | `+0x40` | **GetMonthlyConsumption(s32 x, s32 z)** | 8 | EAX (sign-extended int16) | consumption[x][z], no bounds check |
| 17 | `+0x44` | **GetMonthlyConsumption()** | 0 | EAX | last consumption |
| 18 | `+0x48` | GetMonthlyConsumptionMapPtr(vector*&) | 4 | none | writes the address of the internal map (raw) |
| 19 | `+0x4C` | GetSupplyValue(s32 x, s32 z) | 8 | EAX 0/1 | same as slot 37 |
| 20 | `+0x50` | GetSupplyValueMapPtr(grid*&) | 4 | none | writes the powered grid (no `AddRef`) |
| 21 | `+0x54` | GetUnpoweredConsumerCount | 0 | EAX | |
| 22 | `+0x58` | GetPoweredConsumerCount | 0 | EAX | |
| 23 | `+0x5C` | **GetPowerPlantCount(s32 type)** | 4 | EAX | perType[type], no bounds check |
| 24 | `+0x60` | **GetPowerPlantCount()** | 0 | EAX | plantCount |
| 25 | `+0x64` | GetPowerPlants(ilist<cISC4Occupant*>&) | 4 | none | appends every plant, `AddRef`ed; does not clear the list |
| 26 | `+0x68` | GetPowerPoleCount | 0 | EAX | poleCount |
| 27 | `+0x6C` | GetOverworkedPowerPlantCount | 0 | EAX | §7.4 |
| 28 | `+0x70` | GetAgedPowerPlantCount | 0 | EAX | |
| 29 | `+0x74` | GetDistressedPowerPlantCount | 0 | EAX | |
| 30 | `+0x78` | FindPowerPlants(occ** lowestFunded, SC4Percentage& funding, occ** mostWorn, s32& wearPct) | 16 | none | below |
| 31 | `+0x7C` | IsOccupantPowered(occ) | 4 | EAX 0/1 | §7.2 |
| 32 | `+0x80` | GetOccupantPowerConsumed(occ, u32&) | 8 | **AL bool** | §7.2 |
| 33 | `+0x84` | GetOccupantPowerProduced(occ, u32& prod, u32& used) | 12 | **AL bool** | §7.2 |
| 34 | `+0x88` | GetOccupantMaxPowerProduced(occ, u32&) | 8 | **AL bool** | §7.2 |
| 35 | `+0x8C` | GetPoweredGrid() const | 0 | EAX ptr | no `AddRef` |
| 36 | `+0x90` | GetPoweredGrid() | 0 | EAX ptr | same body |
| 37 | `+0x94` | IsPowered(s32 x, s32 z) | 8 | EAX 0/1 | powered-current bit |
| 38 | `+0x98` | IsWired(s32 x, s32 z) | 8 | EAX 0/1 | wire bit |
| 39 | `+0x9C` | IsOnStrike | 0 | AL = 0 (upper EAX garbage) | |
| 40 | `+0xA0` | GetChanceOfStrike(SC4Percentage&) | 4 | none meaningful (EAX = the argument) | writes 0.0 |
| 41 | `+0xA4` | BeginStrike | 0 | AL = 0 | shared body with 39 |
| 42 | `+0xA8` | EndStrike | 0 | AL = 0 | shared body |
| 43 | `+0xAC` | GetStrikeStartDate | 0 | EAX | saved strike start |
| 44 | `+0xB0` | GetStrikeLength | 0 | EAX | start ≠ 0 ? `simulator.GetSimTime() − start` : −1 |
| 45 | `+0xB4` | GetCurrentPlantEfficiencyPercent(occ) | **8** | **hidden pointer** | `SC4Percentage` returned by value, §7.1 |
| 46 | `+0xB8` | GetCurrentPlantConditionPercent(occ) | **8** | **hidden pointer** | same convention, §7.1 |
| 47 | `+0xBC` | GetStrikeEfficiencyPercent | 0 | EAX ptr | address of the strike-efficiency member |
| 48 | `+0xC0` | GetFundingPercent | 0 | EAX ptr | tail call of `department.GetFundingPercentage()` |
| 49 | `+0xC4` | GetMonthlyPowerGenerationCost(occ, int64&) | 8 | AL bool | §7.2 |
| 50 | `+0xC8` | GetPowerPlantFundingPercentage(occ, SC4Percentage&) | 8 | AL bool | §7.2 |
| 51 | `+0xCC` | SetPowerPlantFundingPercentage(occ, const SC4Percentage&) | 8 | AL bool | §7.2 |
| 52 | `+0xD0` | (scalar deleting destructor, class) | 4 | EAX this | |

**Hidden-pointer convention of slots 45/46** *[compat]*: the caller passes a pointer to a 4-byte `SC4Percentage`
as the first stack argument and the occupant as the second; the callee pops 8, stores the float there, returns
that pointer in EAX and leaves nothing on the x87 stack.

**FindPowerPlants**: over the plants whose budget item exists and that `dept.IsLocallyFundedObjectInDepartment`:
the lowest `item.localFunding` (start 2.0, strict `<`) and the largest `wear / max(t1, t2)` (start 0, strict `>`).
Outputs are `AddRef`ed (or untouched if none); `wearPct := trunc(ratio × 100)`.

**SDK check** (`cISC4PowerSimulator.h`), errors flagged:

1. **Slots 45/46**: declared `float f(cISC4Occupant*)` (pop 4, ST0). Compiled: `SC4Percentage f(occ)` returned
   by value through a hidden pointer, pop **8**, EAX = pointer, ST0 empty. Checked at `0x006EB5F0` and
   `0x006EC820`. A caller using the SDK declaration corrupts its stack.
2. **Slots 32/33/34**: declared `void`; they return **bool in AL** (upper EAX bits undefined; slot 34 returns the
   property helper's AL). Checked at `0x006EB0D0`, `0x006EB420`, `0x006EB110`.
3. **Overload order** (foundations §16): the SDK declares `GetMonthlyConsumption()` before `(x, z)` and
   `GetPowerPlantCount()` before `(type)`; the compiled order is reversed (slot 16 = `(x, z)`, slot 23 =
   `(type)`). Checked by reading the bodies `0x006EBA10` (pop 8), `0x006AAEB0` (pop 0), `0x006EFA00` (pop 4),
   `0x006EF9F0` (pop 0).
4. Slot 40 is declared `void GetChanceOfStrike(SC4Percentage&)`: ABI-compatible (pop 4), but **the return value
   is used**: the Utility advisor (`0x0042220F`, advisors.md) loads the float through the returned EAX. *[compat]*
   A replacement must return the argument's address in EAX (declare it returning `SC4Percentage*`); a `void`
   replacement makes that caller read through a garbage pointer. *(Corrected: the earlier text said "not an
   error".)*
5. All other slots match (argument sizes, pointer returns of 35/36/47/48, `FindPowerPlants`).

### 10.2 `cIGZSerializable` (vtable `0x00AB288C`)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0–2 | QueryInterface / AddRef / Release (adjust to the main object) | 8 / 0 / 0 | as above |
| 3 | `Write(cIGZOStream&)` | 4 | AL = 1 always |
| 4 | `Read(cIGZIStream&)` | 4 | AL (false only for version > 6) |
| 5 | `GetGZCLSID()` | 0 | EAX `0x0990C005` |

### 10.3 `cIGZMessageTarget2` (vtable `0x00AB28A4`)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0–2 | QueryInterface / AddRef / Release | 8 / 0 / 0 | as above |
| 3 | `DoMessage(cIGZMessage2*)` | 4 | AL = 1 always |

## 11. Dependencies and replaceability

### 11.1 Methods the original calls *[compat]*

| Interface (vtable) | Slot | Off | Method | Pop | Return used |
|---|---|---|---|---|---|
| `cISC4City` | 34 / 36 / 38 | `+0x88/+0x90/+0x98` | GetOccupantManager / GetZoneManager / GetNetworkManager | 0 | EAX ptr |
| | 50 / 52 / 62 | `+0xC8/+0xD0/+0xF8` | GetSimulator / GetBudgetSimulator / GetNeighborsSimulator | 0 | EAX ptr |
| | 91 / 92 | `+0x16C/+0x170` | CellWidthX / CellWidthZ | 0 | ST0 |
| | 93 / 94 | `+0x174/+0x178` | CellCountX / CellCountZ | 0 | EAX |
| `cISC4Simulator` | 19 / 20 | `+0x4C/+0x50` | AddAgent(target, level, name, flags) / RemoveAgent(target) | 16 / 4 | AL |
| | 29 | `+0x74` | GetSimTime | 0 | EAX |
| `cIGZMessageServer2` | 4 / 5 / 6 | `+0x10/+0x14/+0x18` | MessagePost(msg, false) / AddNotification / RemoveNotification | 8 / 8 / 8 | — |
| `cISC4BudgetSimulator` | 25 | `+0x64` | CreateDepartmentBudget(id, group) | 8 | EAX ptr |
| | 28 | `+0x70` | GetDepartmentBudget(id) | 4 | EAX ptr |
| | 35 | `+0x8C` | GetFunding(dept) (folded with 34) | 4 | EDX:EAX |
| | 51 / 52 / 53 | `+0xCC/+0xD0/+0xD4` | GetBudgetItemForPurpose / ChangeBudgetItemLine / ChangeBudgetItemCost | 12 / 12 / 16 | AL |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 5 / 10 / 13 | `+0x14/+0x28/+0x34` | SetDepartmentName / GetFundingPercentage / SetMaxAllowedFundingPercentage | 8 / 0 / 4 | EAX ptr (10) |
| | 17 / 18 / 20 | `+0x44/+0x48/+0x50` | CreateLineItem / CreateLineItemForBuildingType / GetLineItem | 8 / 8 / 4 | EAX ptr |
| | 22 / 23 / 24 / 25 / 28 | `+0x58…+0x70` | AddLocallyFundedObject / RemoveLocallyFundedObject / SetLocalFundingPercent / SetLocalFullFunding / IsLocallyFundedObjectInDepartment | 8 / 8 / 12 / 16 / 8 | AL |
| `cISC4LineItem` | 7 / 8 / 9 / 12 / 18 | `+0x1C/+0x20/+0x24/+0x30/+0x48` | SetName / GetSecondaryInfoField / SetSecondaryInfoField / SetFullExpenses(int64) / SetDisplayFlag | 8 / 0 / 8 / 8 / 8 | EDX:EAX (8) |
| `cISC4OrdinanceSimulator` | 25 | `+0x64` | GetCumulativeFactor(id, 0) | 8 | ST0 |
| `cISC4HistoryWarehouse` | 12 / 13 | `+0x30/+0x34` | RecordHistoryData(id, value) / SetHistoryDataQueueLength(id, n) | 8 / 8 | — |
| `cISC4NeighborsSimulator` (`0x00AB1EE0`) | 13 | `+0x34` | FindConnectionOnOtherSide(conn) | 4 | EAX ptr |
| | 15 / 17 / 18 | `+0x3C/+0x44/+0x48` | FindUnvisitedConnection(type, city) / GetConnectionsOfType(type, list&) / GetConnectionsOfTypeToCity(type, city, list&) | 8 / 8 / 12 | EAX |
| | 30 / 31 | `+0x78/+0x7C` | FindDeals(type, list&) / MarkDealsDirty | 8 / 0 | — |
| `cISC4NeighborDeal` (`0x00AB1DD8`) | 7 / 8 / 10 | `+0x1C/+0x20/+0x28` | GetCityOnOtherSideOfConnection / GetType / GetAmount | 0 | EAX |
| | 20 / 21 / 37 | `+0x50/+0x54/+0x94` | GetAmountUsed / SetAmountUsed / SetFlags(flags, bool) | 0 / 4 / 8 | EAX (20) |
| `cISC4NeighborConnection` (`0x00AB2034`) | 6 / 12 / 13 / 14 | `+0x18/+0x30/+0x34/+0x38` | GetConnectionCell / SetVisitedThisCycle / GetIsActive / SetIsActive | 8 / 4 / 0 / 4 | AL (13) |
| `cISC4OccupantManager` | 21 / 40 | `+0x54/+0xA0` | RemoveOccupant(occ, true, 0) / GetOccupantsByStandardCityCells(list, &min, &max, 0, −1) | 12 / 20 | — |
| `cISC4Occupant` | 6 / 7 / 8 / 10 / 11 / 32 | `+0x18…+0x80` | AsPropertyHolder / GetType / GetPosition / GetBoundingBox / GetBoundingCityCells / IsFlagSet | 0/0/4/8/4/4 | EAX / AL |
| `cISCPropertyHolder` | 9 | `+0x24` | GetProperty(id) | 4 | EAX ptr (plus the shared property helpers) |
| `cISC4ZoneManager` | 12 | `+0x30` | GetZoneType(x, z) | 8 | EAX |
| `cISC4LotManager` | 6 / 14 | `+0x18/+0x38` | GetLot(x, z, false) / GetOccupantLot(occ) | 12 / 4 | EAX ptr |
| `cISC4Lot` | 14 / 33 / 36 / 65 / 66 / 67 | `+0x38/+0x84/+0x90/+0x104/+0x108/+0x10C` | GetBoundingRect / GetState / SetCondition / GetNeedsPower / GetHasPower / SetHasPower | 4/0/4/0/0/4 | AL / EAX |
| `cISC4PollutionSimulator` | 66 | `+0x108` | AddRadiationToCity(&pos, strength, radius) | 12 | — |
| `cISC4Demolition` (`0x00A88004`) | 7 | `+0x1C` | DemolishLots(8 args) | 32 | AL (ignored) |
| `cISC4NetworkManager` (`0x00AA7AD0`) | 5 | `+0x14` | GetNetworkTool(5, false) | 8 | EAX ptr |
| `cISC4NetworkTool` (power-line tool `0x00AA9F30`) | 3 / 4 / 10 | `+0x0C/+0x10/+0x28` | Init / Shutdown / UpdateOnZoneChange(SC4Rect by value, u32) | 0 / 0 / **20** | — |
| `cISC4View3DWin` (`0x00ABCE40`) | 24 / 26 | `+0x60/+0x68` | GetCurrentViewInputControl / RemoveCurrentViewInputControl(false) | 0 / 4 | EAX ptr |
| `cISC4PowerPoleOccupant` (`0x00AA9EB0`) | 3 / 6 / 8 / 11 / 12 / 13 | `+0x0C…+0x34` | AsOccupant / BreakAllConnections(bool) / GetCell(&x, &z) / IsDistressed / SetDistressed(bool) / unnamed flag | 0/4/8/0/4/0 | EAX 0/1 (11), AL (13) |
| `cISC4PowerLineOccupant` (`0x00AA9C80`) | 3 / 5 / 6 | `+0x0C/+0x14/+0x18` | GetConnectedPoles(3 outs) / GetCell / IsPreview | 12 / 8 / 0 | AL |
| `cISC4NetworkOccupant` (`0x00AA8080`) | 8 / 12 / 59 | `+0x20/+0x30/+0xEC` | HasNetworkFlag / GetNetworkFlag / GetOccupiedCell | 4 / 0 / 8 | AL / EAX |
| `cISC4BuildingOccupant` (`0x00A872C0`) | 4 | `+0x10` | GetBuildingType | 0 | EAX |
| non-virtual | — | — | `cSC4PowerPoleOccupant::RemoveConnection` `0x0064DA00` | — | §11.2 |

### 11.2 Consumers and replaceability

Callers through `SL::spPowerSim` (or `cISC4City +0x10C`), from a call-site survey:

| Consumer | Uses |
|---|---|
| Map view `Update` (power layer) | slot 35 `GetPoweredGrid` → `GetGridData` (**raw rows**) → standard uint8 display |
| Tract developer | slot 35 → `GetCellValue(x, z) < 2` ⇒ "no power" failure reason |
| Lot developer, prop developer | slot 37 `IsPowered`; lot developer handles `0x8A07AE00` (`DoPowerChanged` on the block rect) |
| Building development simulator | slot 19 `GetSupplyValue` |
| Pollution simulator | slot 31 `IsOccupantPowered` |
| Query tool | Windows call sites: slots 19, 32 (result read as AL), 33, 45 (hidden pointer), 49; the Mac build also shows 31, 34, 46, 50, 51 |
| Advisors / advisor utility | slots 24 and 48 (`g_power_plant_count`, `g_power_funding_p`); per the Mac build also 12, `GetMonthlyConsumption()` (Windows slot 17), 21, 27, 39, 40 |
| Budget sub-window | slot 51 |
| Neighbours simulator | slot 12 `GetMonthlyProduction` (`UpdateCommodityStats`) |
| MySim, animation coordinator, landfill-gas ordinance, building development, lot/tract/prop developers | cache the pointer |
| Safety advisor | listens to `0xAA552F77` |

No caller of `GetMonthlyConsumptionMapPtr` was found *[inferred]*. No code outside the class reads its members.

Findings:

- **Replaceable** through the GZCOM class registry (CLSID `0x0990C005`): the city and the class store create it
  only through GZCOM.
- **The powered grid must be a game `cSC4SimGrid<uint8>`** (simgrid §6.4): the map view reads it raw through
  `GetGridData`, and the tract developer reads cell values. Values must be exactly 0/1/2 (§2). The original
  constructs this grid directly; a replacement creates it through GZCOM per §6.4 and saves it by `ref`.
- `GetMonthlyConsumptionMapPtr` (slot 18) exposes the raw `vector<vector<int16>>`. A replacement that cannot
  provide the STLport layout should keep such a vector *[choice: keep it, since plugins may call the slot]*.
- **Non-virtual call** *[compat, blocker for a pure-interface port]*: terrain changes detach power lines with
  `cSC4PowerPoleOccupant::RemoveConnection` (`0x0064DA00`), which no interface exposes. A replacement must call
  that function (or reproduce its effect on the game's pole objects).
- Lots get `SetHasPower(true)` from this simulator; the lot developer updates lots from `0x8A07AE00` and
  `IsPowered`. A replacement must post `0x8A07AE00` per changed 8 × 8 block with the same rect and keep
  `IsPowered` consistent with the grid.
- Budget hookup (§5.1) and pole decay (§7.6) modify game objects (department, line items, poles) through
  interfaces only.

## 12. Original quirks (reproduced)

Square-city assumptions in the bitmaps and blocks (§2.1, §6.3, §6.5); 600-iteration flood-fill cap (§2.1);
zone test on the first rect corner only (§5.3); no preview test for buildings (§5.1); pole slot-13 test missing
in the rebuild (§5.3); int16 consumption wrap (§5.1); no bounds checks on plant type (§5.1, slots 10/23); 110 %
literal (§6.3); shortfall consumes all remaining availability (§6.3); NaN ratio (§6.4); overworked count
practically 0 (§7.4); double ring advance (§7.5); deficit values (§6.6, §7.5); history `0xAA11F7DC` always 0
(§6.6); brokenFraction not saved (§7.6); plant without a condition curve produces nothing (§7.1); redundant
`ChangeBudgetItemCost` (§7.3); `Read` ignores stream errors (§9).

## 13. Open questions

1. Meaning of pole slot 13 (a byte flag that makes power ignore the pole) and of network flags `0x40000` and
   `0x80000000`.
2. Who posts ConnectionsReady `0x6AC284F3`, and whether it always arrives after a load (otherwise a loaded city
   never recomputes power).
3. Meaning of lot state 1 in the `0xE8BDA2E7` handler (demand.md §7 describes the message).
4. The meaning of the "Hard Failure Type" values 3 and 4 once passed to `DemolishLots` as flags `0x303`/`0x304`
   (§7.4).
5. View input control `0x88F154FB` (§5.5): assumed to be the nuclear-plant placement tool.
6. The exact effect of `RemoveConnection` on the pole and line objects (§5.4).
7. Whether `0x6A07C2EE` has any subscriber (none found).

## 14. Corrections to earlier documents

- `docs/subsystems/grid-sims.md` §4: the radius tuning `0xA911E2F1` is the wire radius R; `0xE911E2FC` is dead (the
  code uses a literal 1.1); the curve `0x0911E2E7` is a per-building property, not tuning; `avail` is divided by
  the conservation factor; the `DoIdle` time budget is ignored; `0xEA418D8F` is both handled and posted.
- foundations §4.1 listed power under the **keep** pattern; the tuning curves are **default** (code defaults are
  rebuilt before every load, §8). *(applied: foundations Errata M3 item 4)*
- neighbors.md names deal flag `0x20` "NoLandfill"; the power simulator also sets it on sell-power deals it cannot
  supply (§6.3, §6.6). *(applied in neighbors §5.2)*

## 15. Conformance tests (must exist in `engine/conformance`)

Unless stated: factor 1.0, R = 2, shipped curves, 256 × 256 city, cell width 16.

1. **Output**: capacity 6000, local funding 100 %, wear 0 → efficiency 1.0, condition 1.0 → 6000. Funding 50 %
   → 1500; 90 % → **4799** (curve 80 × 0.01f = 0.79999998 exactly, a tie that rounds to even, 0.79999995; × 6000
   rounds to 4799.9995); 120 % → 6600 (no clamp). Funding 100 %, wear 0.5 → condition 0.92 → 5520. No
   `0x8A1E07EE` → 0. No budget item → 0. Wind (200) at 50 % → 50.
2. **Slot 45/46 ABI**: calling slot 45 with (out*, occ) pops 8, returns out in EAX, writes 1.0f for case 1.
3. **Single network**: one plant (supply 1000), one block need 500 → total 1100, avail 1100 → 600, leftover
   600, used 500; plant `0x08D9763D` = 500; production 1000, consumption 500, demand 500; powered-next has the
   marked cells of that block.
4. **Shortfall**: supply 1000 (total 1100), blocks A need 700 and B need 500. Order A, B: A powered, B not;
   used = 1100; plant used 1000; demand 1200. Order B, A: B powered, A not; same used.
5. **Conservation**: factor 0.9f, supply 1000, need 555 → avail 1222, then 667; leftover 600; used 500; plant
   used 500; history `0xAA11F7D5` = 499 (555 × 0.9f = 499.49998… rounds to 499.5, truncated), `0xAA11F7D2` = **450**
   (500 × 0.9f = 450.0000067 rounds to 450.0 at 24 bits; the earlier 449 was a 53-bit result), `0xAA11F7D8` = 500.
6. **Import**: no plant; buy deal amount 300 used 0, a PowerPole connection to that city whose cell is wired and
   whose other side is active; block need 200 → powered; deal used 200; import 200.
7. **Export**: supply 1000, sell deal amount 400 connected; need 100 → exports 400, deal used 400, avail 600,
   used 500, plant used 500, history `0xAA11F7D2` = 100. A sell deal that cannot be served (avail 0) gets flag
   `0x20`.
8. **Flood fill**: wire bits at (0,0), (1,0), (1,1), (2,2); seed (0,0) → mark = {(0,0), (1,0), (1,1)}; (2,2) is
   not reached (diagonal). A seed on an unwired cell stays marked.
9. **Wire radius**: building cells (10,10)–(11,11) → wired (8,8)–(13,13); building at (0,0)–(1,1) → (0,0)–(3,3);
   at (254,254)–(255,255) → (252,252)–(255,255). A conducting network piece wires only its own cell.
10. **Centre cell**: bounding box x 32..64, z 16..48 → consumption stored at (3, 2). A consumer of 40000 stores
    −25536 at that cell and contributes no need.
11. **Grid values**: wired + powered → 2; wired only → 1; unwired → 0 (even if powered). After a power change in
    block (1, 2), `0x8A07AE00` is posted with (8, 16, 15, 23) and `0x6A07C2EE` once.
12. **Positions**: `PositionToCell(100.0, 250.0)` → (6, 15); `CellToPosition(6, 15)` → (96.0, 240.0).
13. **Maintenance**: coal base 250: wear 0 → 250; wear 0.5 → 262 (262.49999 rounds to 262.5); wear 0.75 → **325**
    (250 × 1.3f = 324.99999 rounds to 325.0 at 24 bits; the curve result stays on the FPU stack, `fmulp`). 300 poles → line item 1
    full expenses 300 (no fuel cost shipped).
14. **Ageing**: rate 0.001, used/effCap 0.5, department funding 100 % → wear += 0.45 × 1.0 × 0.001 = 0.00045.
    Wear reaching 0.9 (t1 0.9, t2 1.0) → distressed += 1 and `SetCondition(2)`; reaching 1.0 with flags 3 →
    `DemolishLots` with flags `0x303`; nuclear (flags 4, radiation 3200/15) also adds radiation and posts
    `0xAA552F77`. Wear ≥ 0.8 → aged += 1.
15. **Pole decay**: 100 poles, brokenFraction 0, funding 0 % → fraction 0.02f → target `trunc(100 × 0.02f)` = **2**
    (the exact product 1.99999996 rounds to 2.0 at 24 bits, foundations §18); 99 `rand()` calls. 1000 poles →
    **20**; 50 poles → 1. Next month at 100 % → fraction 0 → all repaired.
16. **Deficit**: state 3 with production 1000, consumption 500 → 0.0; consumption 0 → 1.0. Month with
    production 1000, consumption 2000 → −1.0; consumption 1500 → 0.0.
17. **Unpowered %**: consumers 4, powered 3 → 25; consumers 3, powered 2 → 33; consumers 0 → 0; consumers 4,
    powered 5 → −25.
18. **Ring**: one cycle with coal used 500 → ring[idx][1] = 500, annual[1] = 500, `GetAnnualConsumption` = 500.
    After 12 more cycles without plants → 0.
19. **Strikes**: `IsOnStrike`/`BeginStrike`/`EndStrike` → AL 0; `GetChanceOfStrike` writes 0.0;
    `GetStrikeLength` → −1 with start 0.
20. **Save**: write → read → write is byte-identical (with 0 and 2 plants, 0 and 3 poles). Version 7 → `Read`
    false. Version 5 with deficit s32 50 → 0.5. After load: the strike flag is 0, state and bitmaps as saved, no
    recompute until a change or the month.
21. **Lifecycle**: a new city: `Init` creates the department `0x8910BC8A` with line item 1, the grid (instance
    `0x49D5BC86`, tract size 1, all 0) and state 4. `Init` twice → one set of agents and subscriptions.
    `DoIdle` before ConnectionsReady does nothing.
22. **Nuclear Free Zone**: enacting `0xE0D0722E` demolishes every type-5 plant lot with one `DemolishLots` call.
