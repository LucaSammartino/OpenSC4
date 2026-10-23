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

The constructor (`0x006EF7B0`) clears every field above except ring, annual, the unused flag, changed, recompute,
agePlants and state. Those are set only by the new-city `Init` (§4.1 step 3), by `Read` (§9) or, for changed, by
state 0. This matters only for a failed load (§9.1).

**Counter types** *[compat]*. All counters are 32-bit. The table gives the signedness the instructions use and what
an underflow does. Every getter slot returns the stored 32 bits unchanged in EAX (`mov eax, [member]`, no
clamp). The SDK declares them `int32_t`, except `GetPowerPoleCount`, which is `uint32_t`.

| Counter | Changed by | Signedness in the code | Underflow |
|---|---|---|---|
| consumers | +1 at insertion (§5.1); −1 at removal (§5.2) | signed: after the decrement, a negative result (`jns` not taken) is replaced by 0 | cannot go below 0 |
| requirement | += / −= `consumed` (u32) | signed clamp, as for consumers: a result that is negative as `s32` becomes 0 | cannot go below 0. A sum above 2³¹ − 1 also reads as negative and is reset to 0 by the next removal *[quirk, unreachable with shipped values]* |
| powered count | += n per powered block (§6.3) | plain add | — |
| unpowered | `consumers − powered` at state 3 (§6.6) | signed: a negative result becomes 0 | never negative |
| plantCount, perType[t] | ±1 (§5.1, §5.2) | plain `inc`/`dec`, no clamp | plantCount is decremented only for a node found in the plant list, so it stays ≥ 0. `perType[t]` uses the type read **at removal time**: if a plant's `0x27812853` changed while it was in the city, the old type keeps its count and the new one can reach −1. Slot 23 then returns `0xFFFFFFFF` (−1) |
| poleCount | ±1 (§5.1, §5.2, §5.6) | unsigned: converted with `fild` plus 2³² when negative (§7.6) | decremented only for a pole found in the list, so it stays ≥ 0 |
| brokenCount | ±1 (§7.6) | unsigned compares with the target (`jbe`/`jae`) | not decremented when a distressed pole is removed (§5.2), so it can exceed the number of distressed poles. It never goes below 0 |
| overworked, aged, distressed | +1 (§7.4), zeroed before each ageing | plain add | — |
| consumption cell | `int16` add/sub (§5.1) | wraps | §5.1 |

The state-3 tests `consumers > 0` and `consumption > 0` (§6.6) are signed (`jle`). The building tests
`consumed > produced`, `consumed > 0` and `produced > consumed` are unsigned (`jbe`).

## 4. Lifecycle

### 4.1 `Init()` → true (AL = 1)

1. Take (and `AddRef`) the city (`SL::spCity`) and from it the occupant manager, zone manager, neighbours
   simulator and budget simulator; take the demolition utility (global). Each one is taken only while its member
   is still null, so a second `Init` takes no extra references. Then, only if the budget simulator is non-null
   **and no department is held yet**: look up the power department with `budget.GetDepartmentBudget(0x8910BC8A)`
   and `AddRef` it if found (`0x006F045C`–`0x006F047D`). *[compat]* After a load, `Read` has already set the
   department from its saved `ref` (§9), so `Init` does **not** look it up again and keeps the loaded object. The
   lookup runs after a load only when that ref was null or failed. If it then finds nothing, nobody creates the
   department, because step 3 is skipped after a load. It stays null, and state 1 later calls `GetLineItem(1)`
   through it without a null test (`0x006EF066`), which crashes *[quirk]*. *[choice]* FreeCity4 creates the
   department as in step 3 when it is still null at the end of `Init` after a load.
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

All three carry `data1` = the simulator's current date number, the julian day of `GetSimDateNumber` (`0x00703D40`,
the date object's slot 26) (PW2). The agent loop reads it once per `SimulateAgents` call (`0x007038E7`; the
message ID is `0x66956810 + level`, `data2` = 0), and the idle loop once per `SimulateIdleTime` call
(`0x00702E38`), so every idle agent of one frame gets the same date (simulator.md §5.3, §5.4). The power
simulator itself does not read `data1`.

## 5. Messages handled

| Message | Action |
|---|---|
| `0x99EF1142` occupant inserted | `DoOccupantInserted(void1)` (§5.1), only if `initialized` (the handler's first test, `0x006ECC56`) |
| `0x99EF1143` occupant removed | `DoOccupantRemoved(void1)` (§5.2), only if `initialized` (the handler's first test, `0x006EBF66`) |
| `0x07699455` zoning changed | rect = the four `int32` at `void2`; `CellChanged(rect)` (§5.3); then the city's network manager `GetNetworkTool(5, false)` (the power-line tool): `AddRef`, `Init`, `UpdateOnZoneChange(rect by value, data3)`, `Shutdown`, `Release` |
| `0xCA1B151C` tuning reload | re-read the tuning (§8) |
| `0x278E6003` terrain heights changed | ignored if data4 is 4 or 5; else rect `x1 = (data1 >> 16) − 1`, `x2 = data1 & 0xFFFF`, `z1 = (data2 >> 16) − 1`, `z2 = data2 & 0xFFFF`, clamped to the city; `DoTerrainHeightsChanged` (§5.4) |
| `0xC2C09554` OrdinanceUpdated | if data2 = `0xE0D0722E` (Nuclear Free Zone) and data3 ≠ 0: §5.5 |
| `0xE8BDA2E7` kMessageStateChanged | void1/IGZUnknown QI `cISC4Lot` (`0xE818FCED`; a copy of the lot with its scalar fields, lot-developer.md §3.2); if `lot.GetState() == 1`: centre = `((x1 + x2) / 2, (z1 + z2) / 2)` of `lot.GetBoundingRect` (signed integer division); if `IsPowered(centre)` and `lotManager.GetLot(cx, cz, false)` is not null: that lot's `SetHasPower(true)` |
| `0xEA418D8F` deal status | if data1 = 0 and the IGZUnknown is a deal (QI `0x0773FEF1`) of type `0x4773FFE0` or `0x4773FFE1`: state := 4, recompute := true |
| `0x6AC284F3` ConnectionsReady (posted once per load by the regional city; when and when not: foundations.md §11.6) | enabled := true |
| `0x26D31EC1` PostCityInit | §5.6 |

### 5.1 `DoOccupantInserted(occ)` *[compat]*

Dispatch on `occ.GetType()` (occupant slot 7):

- **Power pole** `0x2890D4DE`: QI `cISC4PowerPoleOccupant` (`0xA8C86142`). If pole slot 13 returns true, ignore
  the pole entirely (slot 13 is `IsPreview`, network-occupants.md §4.2). Else: cell := `pole.GetCell` (slot 8); set the wire
  bits of the cell's neighbourhood; keep the reference in the pole list; poleCount += 1; costBase += int64
  property `0x09132DE8` of the pole; recompute := true.
- **Power line** `0x0990B2EA`: QI `cISC4PowerLineOccupant` (`0xC98F3B24`). If `IsPreview()` ignore. Else wire the
  neighbourhood of `GetCell()`; recompute := true. Lines are not kept.
- **Network occupant** `0xC772BF98`: QI `cISC4NetworkOccupant` (`0xA821EF94`). Skip it if
  `HasNetworkFlag(0x80000000)`. It conducts only if `HasNetworkFlag(0x40000)`: then wire its single
  `GetOccupiedCell()` (**not** widened by R); recompute := true. Flag meanings unknown *[inferred]*.
- **Building** `0x278128A0`:
  - centre cell = `trunc(((min + max) × 0.5) / CellWidth)` per axis from `GetBoundingBox` (world units). The
    half-sum is **stored as a float32** before the division (`0x006ECEBB`, `0x006ECECD`; removal `0x006EC20F`,
    `0x006EC221`), which decides cells at an edge (test 10);
  - wire bits: `GetBoundingCityCells` expanded by R;
  - `consumed` = u32 `0x27812854`, `produced` = u32 `0x27812852` (both default 0);
  - if `consumed > produced` (a **consumer**): `consumption[cx][cz] += (int16)consumed` (16-bit wrap);
    consumers += 1; requirement += consumed; if `IsPowered(centre)` and the building has a lot: `lot.SetHasPower(true)`;
  - if `produced > consumed` (a **plant**): `AddRef`; append a node `{visited 0, occ}`; plantCount += 1;
    `perType[type] += 1` with `type` = s32 `0x27812853` (default 0; **no bounds check** *[quirk]*);
    recompute := true. Then the budget hookup (`0x006ED020`–`0x006ED10E`):
    1. `GetBudgetItemForPurpose(holder, 0xCA58E540)`; not found → stop. QI `cISC4BuildingOccupant`
       (`0x87DDA3A9`); fails → stop.
    2. `dept := budget.GetDepartmentBudget(item.department)`: the **item's** department, not the simulator's own
       department member (null not checked).
    3. `line := item.line`. If it is 0: `line := building.GetBuildingType()` and
       `budget.ChangeBudgetItemLine(holder, item.index, line)`.
    4. If `line` is (still) 0, stop. **All** of the following are inside this `line ≠ 0` branch, in this order:
       `li := dept.GetLineItem(line)`, or if none `li := dept.CreateLineItemForBuildingType(line, true)` (result
       not null-checked) followed by `li.SetDisplayFlag(2, true)`; `dept.AddLocallyFundedObject(holder,
       0xCA58E540)`; `li.SetSecondaryInfoField(li.GetSecondaryInfoField() + 1)` (int64; the plant count shown in
       the budget window); set the int64 property `0x2A58D350 := item.cost` (the base maintenance, §7.3).
    *[compat]* A plant whose item has line 0 and whose building type is 0 is therefore not locally funded, not
    counted in a line item and has no `0x2A58D350`, so §7.3 computes its maintenance from a base of 0.
  - Equal consumed and produced (including both 0): only the wire bits.
  - *[quirk]* There is no preview-marker (`0x029244DB`) test, unlike the other simulators.

### 5.2 `DoOccupantRemoved(occ)` *[compat]*

- Pole: the same slot-13 test. Mark the neighbourhood dirty (§5.3, without the zone test). Search the pole list:
  if the pole is there, release it, erase it, poleCount −= 1. Then, **whether or not it was found**: costBase −=
  int64 `0x09132DE8` of the pole (default 0) and recompute := true (`0x006EC04B`–`0x006EC08C`). *[compat]* So a
  pole that is not in the list still lowers costBase, for example one whose slot 13 was true at insertion and is
  false at removal. costBase can then drop below the sum over the list, even below 0. Only PostCityInit (§5.6)
  recomputes it. brokenCount is **not** decremented when the removed pole was distressed (§3, counter types)
  *[quirk]*.
- Power line / network occupant: the same filters as insertion; mark dirty (line: neighbourhood; network: the
  single cell); recompute := true.
- Building: if `consumed > produced` and `consumed > 0` (unsigned): `consumption[cx][cz] −= consumed` (int16);
  consumers −= 1 (a negative result becomes 0); requirement −= consumed (a result negative as `s32` becomes 0).
  Always mark `GetBoundingCityCells` expanded by R dirty. If the budget item is found: `dept :=
  budget.GetDepartmentBudget(item.department)` (the item's department), and if that is non-null `li :=
  dept.GetLineItem(item.line)`. Only if both are non-null: `dept.RemoveLocallyFundedObject(holder, 0xCA58E540)`
  and `li.SetSecondaryInfoField(Get − 1)` (int64). If the occupant is in the plant list: release, unlink,
  perType[type] −= 1 (type read now), plantCount −= 1, recompute := true.

**Running counts against a rebuild** (PW8) *[compat]*. consumers, requirement and the consumption map are running
sums. Insertion adds and removal subtracts what the building's properties say **when the message is delivered**.
Nothing recomputes them from the occupants: `Read` restores them from the save (§9.1), and `Init`, PostCityInit
(§5.6) and the pass only read them. So applying §5.1 to every live occupant (a rebuild) can give other numbers,
in three ways:

1. **Inherited from the save.** The counts are whatever the saving game had. The shipped Berlin cities do not
   match their own buildings. Konradshohe saves consumers 725 and requirement 6478, but its 737 saved buildings
   give 723 and 6476. The two extra consumers are cells (41,102) and (55,111), 1 each. They are the centre
   cells of the mayor's statue (exemplar `0x8A3858D8`/`0x03730000`, `RW4x4_MayorStatueMediumLow`) and the
   cemetery (`0x8A3858D8`/`0x03470000`, `RW32x32_Cemetery_0347`). Both have Power Consumed `0x27812854` = 0 in
   the shipped exemplars, and no other building is centred there. Tegel has the same +1 at its cemetery (74,37).
   The other base-game saves with buildings (Spandau, Fulham, Kensington and the three Timbuktu tutorials) match
   exactly. *[inferred]* These cities were built while the two rewards consumed 1. The extra 1 never goes away:
   removing the building reads 0, which is not a consumer, so the cell keeps 1 with nothing on it. This is the
   whole persistent part of the shadow's bookkeeping difference: consumers +2, requirement +2 and those two
   cells, in every bookkeeping line of runs 80, 83 and 86 (94, 95 and 56 months). The 1s are real consumption:
   when such a cell is in a network, the pass adds it to its block's need and powered count (§6.3 step 6).
2. **Messages in flight.** The occupant manager posts InsertOccupant and RemoveOccupant (queued; flora.md §9.1).
   At a month boundary the original may not yet have seen an occupant that a rebuild already sees, and it may
   still count one that is gone. Example (run 80, 2028-01-01): orig (31,43) 14 and (31,44) 14, ours 0 and 76.
   Two buildings of 14 are being replaced by one of 76. Consumers 685/682 = 2 old − 1 new + 2 inherited, and
   requirement 8923/8969 = 28 − 76 + 2. The next month only the inherited part is left.
3. **Properties that change while a building is in the city** (not seen in these runs). The values are read
   twice. A building whose `0x27812854` or `0x27812852` changes in between adds one value and subtracts another,
   or nothing if it is no longer a consumer at removal. No game code writes `0x27812854`. The only writer of
   `0x27812852` is pollution's waste-to-energy handling (pollution.md §4.2.3, §4.9.3), and those buildings are
   not consumers. Plug-ins or exemplars changed between saving and loading act like case 1.

*[compat]* FreeCity4 loads the three counts from the save and keeps them as running sums. It never rebuilds them,
so the inherited 1s stay. *[choice]* For the shadow: a cell or count whose difference is the same as at the first
bookkeeping line after the load is inherited (case 1). A difference that is gone a month later is case 2.

**Telling case 2 apart without guessing** (PW9, PW11, from the exe). The message server has a queue count:
`cIGZMessageServer2` slot 10 (`+0x28`, `0x0092D0F4`) takes the queue lock and returns the number of messages still
queued (it walks the queue's list, `0x0092F821`). The tick (`0x0092E040`) takes each message out of the queue before
it sends it, so inside a handler the count excludes the message being delivered. The count is no use here:

- It counts every posted message of every type, and there is no per-type count or way to look inside the queue.
- At a month line it is never 0. The day's own SimNewDay, and SimNewMonth on the first, are posted in the same
  simulator tick before the agents run (simulator §5.2), and they wait for the next message-server tick.

The reliable signal is the message stream itself. A posted message is delivered by `MessageSend`, which calls every
subscriber of its type in the same call (foundations §5.1). So a shadow subscribed to `0x99EF1142` and `0x99EF1143`
receives exactly the occupant messages that the original power simulator has handled, in the same order, by the
time of any line. The occupant manager lists an occupant and sets its flag 1 before it posts InsertOccupant, and
unlists it before it posts RemoveOccupant (flora §9.1, flammability §15 item 3). Therefore, at any line:

- the occupants **in flight** are those whose state in the occupant manager's lists differs from what the delivered
  messages say (inserted but not yet delivered, or removed but not yet delivered);
- the original's counts equal the saved counts plus the effects of every delivered message (PW8), each read with
  the properties as they were at delivery.

*[choice]* (recommended) Build the expected counts from the delivered messages, applying §5.1 and §5.2 to each in the
shadow's own handler. That removes case 2 instead of detecting it. Failing that, take the baseline (PW9) at the first
line where no occupant is in flight by the rule above. The bracket's rule (the first line equal to its predecessor,
never edited afterwards, with its mismatch tests) is a heuristic for the same thing and is accepted. Under a clock
that opens several days per frame it can settle late or settle on a line with messages in flight, as PW11 observed.

**Wire bits at a bookkeeping line** (PW10). The original's wire bits depend on history, not only on the live
occupants:

- insertion sets bits directly, with the filters of §5.1;
- a rebuild clears and re-wires only the dirty rect, with no filters, a 2R margin for occupants and no margin for
  zoned cells (§5.3);
- bits set outside a rebuilt rect only accumulate.

A rebuild from the live occupants can therefore differ in both directions with no error on either side, and no
counted comparison of the wire bits against a rebuild is exact. The bracket (listed as "wire bits, not counted",
outside the baseline and the mismatch count, with the cycle lines comparing each rebuild exactly) is right. The bits
that §5.1 sets directly are covered exactly only by a mirror that replays the same sequence: the insertion bits for
each delivered message, the same merged dirty rect, and the rebuild at the original's next enabled `DoIdle`.

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

The bits set in steps 1 and 2 are not clipped to the rect (`SetWireBits` `0x006EAFD0`); outside it they can only be
added. Step 1 also has none of the insertion filters of §5.1: a network occupant with flag `0x80000000` is wired
when it has `0x40000`, and a preview line is wired (`0x006ED859`–`0x006ED9A8`).

**The rectangle matters** (PW5). The occupant query has a margin of 2R, so it finds every occupant whose wiring can
reach the rect, and step 1 does not depend on the rect's exact size. Step 2 has **no margin**: only zoned cells
inside the rect re-wire their neighbourhoods. A cell of the rect whose only wire source is a zoned cell outside the
rect, at most R away, loses its bit in the rebuild *[quirk]*. Two rects A ⊂ B therefore rebuild differently
exactly where a cell of A is wired only by zoned cells in B but not in A.

Example (R = 2). A power line runs along x = 10, so it wires x 8…12 in every row. Cell (11, 15) is zoned, so it wires
x 9…13 × z 13…17. Nothing else is near. A building with footprint x 12…18 × z 12…18 is removed, and the dirty rect
B is the footprint expanded by 2, x 10…20 × z 10…20. Rebuilding B leaves x 10…12 wired (the line) and (13, 13…17)
wired (the zoned cell); every other cell of B changes from 1 to 0. The bounding box of the changed bits is
therefore A = x 13…20 × z 10…20, which does not hold (11, 15). A rebuild over A clears (13, 15) and does not set it
again, while the original keeps it.

*[choice]* for the shadow: use the dirty rect that our own simulator merges from the same messages, which is the
original's rect, rather than one inferred from the changed bits.

### 5.4 `DoTerrainHeightsChanged(rect)`

*[compat]* (`0x006EDDA0`.) The candidates come from **one occupant-manager query**, not from the simulator's pole
list: `occupantManager.GetOccupantsByStandardCityCells(list, &(x1, z1), &(x2, z2), 0, −1)` (the global occupant
manager, slot 40, all types) on the rect of §5. If the query returns false, pass 1 is skipped. Passes 2 and 3
still run over whatever the list holds.

1. **Poles**, in list order: each occupant that answers QI `cISC4PowerPoleOccupant` (`0xA8C86142`) and whose
   own `GetCell` (slot 8) lies inside the rect (inclusive, signed compares) gets `BreakAllConnections(true)`
   (slot 6), then `occupantManager.RemoveOccupant(occ, true, 0)`. There is no slot-13 test. Poles that the
   query returns but whose cell is outside the rect are kept.
2. **Lines**: each occupant of the same list that answers QI `cISC4PowerLineOccupant` (`0xC98F3B24`) and whose
   `GetConnectedPoles` (slot 3) returns two non-null poles is collected, unless a line already collected has
   the same two poles in the same order. Lines are distinct by pole pair, not by object. This pass runs after
   the pole removals, so a line whose pole was just removed may no longer report it.
3. For each collected line: `GetConnectedPoles` again, then `firstPole.RemoveConnection(secondPole, 0)`
   through the pole class's non-virtual `RemoveConnection` (§11.2).

The removals then come back as `0x99EF1143` (§5.2).

### 5.5 Nuclear Free Zone *[compat]*

Collect the lots (`lotManager.GetOccupantLot`) of every plant in the plant list whose `0x27812853` exists and
equals 5 (nuclear; read as u32), and demolish them: `demolition.DemolishLots(true, list, 0, 1, false, null,
null, 0)`. **The call is made unconditionally** (`0x006EFED0`), also when the list is empty (a Nuclear Free Zone
enacted in a city without nuclear plants). The demolition utility (`0x004670C0`) returns true at once for an empty
list and does nothing, so with the game's demolition utility an empty call has no observable effect.
*[choice]* FreeCity4 makes the call anyway, so a replaced demolition utility sees the same calls.
Then, in every case, if the 3D view's current input control has ID `0x88F154FB` (control slot 5),
`RemoveCurrentViewInputControl(false)` *[inferred: the placement tool for a nuclear plant]*.

*[quirk]* A nuclear plant without a lot adds a **null** entry to the list (no null test). `DemolishLots`
dereferences the first entry unconditionally, so a null first entry crashes. *[choice]* FreeCity4 skips null lots.

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

1. Clear mark; set bit (x, z); selection grow (§2.1) with mask = wire, `rowMin = z`, `rowMax = z + 1`. Then,
   in this order (`0x006EE1B6`–`0x006EE1EA`): if `rowMin > rowMax` (signed), reset both to the **seed values**
   `rowMin := z`, `rowMax := z + 1` (not the empty range and not the full map); then if `rowMax ≥ CellCountZ`
   (signed), `rowMax := CellCountZ − 1`. The reset cannot trigger: the grow only lowers rowMin and raises rowMax
   from `z ≤ z + 1` *[inferred]*.
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

**Block count** (PW6). The block rectangle is at most `CellCountX / 8` wide and as many high, so a 256-cell city has
at most 32 × 32 = 1,024 blocks (64: 64; 128: 256). The iterator's start is `rand() mod count` with `rand()` in
0…32,767 (foundations §12), so every start is reachable, and a count above 32,768 cannot occur. If it did, the
starts from 32,768 up could never be drawn.

### 6.4 Helpers of §6.3

**`IsNeighborConnected(city, markVisited, light)`** → bool: for each PowerPole connection to `city`
(`GetConnectionsOfTypeToCity(4, city)`), take its connection cell clamped to the map; if that cell is marked:
if markVisited, `SetVisitedThisCycle(true)`; if light, set the cell's powered-next bit; return true. None → false.

**`MarkPowerPlantsAsVisited(a, b)`** (`0x006EBC10`): `ratio := production(pass) > 0 (signed) ? f32(u32 a /
u32 b) : 0`. Both operands are loaded as unsigned (`fild` plus 2³² when negative), divided at 24 bits and stored
as f32. For each plant with visited = 1: `used := _ftol(f32(s32 GetCurrentPowerProduced) × ratio)` (low 32
bits; `0x006EBCA7`); set the s32 property `0x08D9763D := used`; `ring[ringIndex][type] += used`; visited := 2.
*[compat]* When `a = b = 0` while the pass production is positive (this network produced and used nothing, another
one produced), the ratio is 0/0 = NaN. This network's plants all produce 0, `0 × NaN` is NaN, and `_ftol` turns NaN
into `0x8000000000000000`, whose low 32 bits are **0** (foundations §18.1). Those plants therefore get `used =
0`, as they would with a ratio of 0. *(Corrected: an earlier version said `0x80000000`, the result of an
inline 32-bit `fistp`. This function converts with `_ftol`.)*

### 6.5 State 2 — neighbours and publishing

For each buy deal with `used < amount`: repeatedly take `conn := FindUnvisitedConnection(4, deal.city)` until null;
`SetVisitedThisCycle(true)`; if `FindConnectionOnOtherSide(conn)` exists and its `GetIsActive()` is true:
`UpdatePowerGrid(connection cell, clamped)`. Then swap powered-current and powered-next and run
**UpdatePoweredLots**.

`FindUnvisitedConnection(t, city)` (neighbours slot 15, `0x006CF720`; PW1) walks the neighbours simulator's
connection list from the start, in append order (neighbors.md §6.2), and returns the first connection whose type
is `t` (connection slot 5), whose visited bit is clear (slot 11) and whose other-side city is `city` (slot 9),
without `AddRef`; null when none is left. Each call starts again from the head. Because the caller sets the
visited bit before the next call, the loop above visits the matching connections once each, in list order: the
same order as `GetConnectionsOfTypeToCity(t, city)` filtered to the unvisited ones.

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
5. `0xAA11FB59` ← consumers > 0 (signed) ? `_ftol(r24(r24(1.0 − r24(powered / consumers)) × 100.0))` (no
   clamp) : 0. *[compat]* The x87 sequence (`0x006EF39D`–`0x006EF3B2`) is `fild dword powered` (s32), `fidiv
   dword consumers` (s32), `fsubr qword 1.0` (`0x00A80AB0`), `fmul qword 100.0` (`0x00A80AC0`), `_ftol`. The two
   constants are doubles, but both are exact, and under the 24-bit precision control each of the three
   operations rounds to a float32 significand (foundations §18). The computation is therefore **24-bit, not
   double**, and the truncated result differs from a double evaluation for many ratios: consumers 5, powered 3 →
   3/5 = 0.60000002, 1 − that = 0.39999998, × 100 = 39.999996 → **39** (double: 40); consumers 10, powered 9 →
   **10** (double: 9); consumers 20, powered 11 → **45** (double: 44); consumers 5, powered 6 → **−20** (double:
   −19).
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
  curve0x0911E2E7(wear))`. If the budget item is found and `maint ≠ item.cost` (int64): `d :=
  budget.GetDepartmentBudget(item.department)`. This is the **item's** department, looked up again here, not the
  simulator's own department member. If `d` is null, nothing more happens. Otherwise `d.SetLocalFullFunding(holder,
  maint, 0xCA58E540)` and then `budget.ChangeBudgetItemCost(holder, item.index, maint)` (`0x006EEFE0`–`0x006EF022`;
  redundant, budget §7.4 already changes it). `0x2A58D350` keeps the base cost, so the multiplier does not
  compound.
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

After a load (PW4): only the constructor (`0x006EF8A7`, 0) and `ApplyPoleFunding` write brokenFraction; `Read`,
`Write` and `Init` never touch it. The saved brokenCount is kept, so the first month after a load moves the broken
poles towards the target of a fraction that starts again from 0. Example: 1,000 poles, 300 broken when saved.
At 100 % funding the shipped curve gives 0.025, so the fraction is `clamp(0 − 0.025) = 0`, the target is 0, and the
first month shuffles the poles and repairs all 300. At 0 % funding the curve gives −0.02, the fraction becomes 0.02,
the target is 20, and 280 poles are repaired.

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
| `0x49122036` | Utility Min Funding to End Strike | f32 × 0.01 | 0.9 | 90 (→ 0.9 after × 0.01) | dead |
| `0x0911E2E6` | Utility Strike Response Curve | curve | (0,20)(100,100) | (0,0)(20,45)(50,75)(100,100) | dead |

The ordinance factor is refreshed at every tuning read and every month: `GetCumulativeFactor(0x0911E117, 0)`
(Power Conservation 0.9 when enacted, ordinance.md).

**When the factor is read** (PW7). Only three places write it: the constructor (1.0, `0x006EF889`), the tuning read
(`0x006ED6B2`) and the start of `DoMonthlySimulation` (`0x006EC580`). The tuning read runs in the first `Init`, so
after a load too, and on `0xCA1B151C`. `Read` does not touch the factor, and `OrdinanceUpdated` (`0xC2C09554`) is
handled only for the Nuclear Free Zone (§5.5). An ordinance switches on at once (ordinance.md, `IsOn`), but power
keeps the old factor until the next month. Power runs first at month level (§4.3), so it reads the factor before
the ordinance simulator's own month work. Example: Power Conservation enacted on 10 March. Power uses 1.0 until 1
April, then 0.9. A shadow that starts on 15 March and reads the live value gets 0.9 while the original holds 1.0.
*[choice]* for the shadow: take the live value only when it starts at a load (the original's `Init` read) or at a
month start; otherwise treat the factor as unknown until the first month.

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
- `Read` (`0x006F0880`) sets `running` first; a version > 6 returns false after the version only (`running`
  stays set). Older versions read the deficit as an integer percentage (`s32 × 0.01f`). It reads straight into
  the members, except where the table below names a local. The plant nodes are **appended** to the list, which is
  not cleared. The pole vector is resized to `poleCount` null entries and then filled. The four bitmaps are
  allocated anew with `words × 4` bytes each, without freeing old arrays and without clearing. It resets the strike
  flag to 0 and **always returns true** (AL = 1), ignoring stream errors.

**Read after a stream error** (foundations §2.1, §2.9; the loader ignores the result, foundations §3.5). The
original never calls `GetError`. Once the version is accepted it reads every field in order and returns **true**
with `running` set, even if the stream failed part-way (for example on the ref of a plant or pole whose class
cannot be created). Fields read before the failing call hold the loaded values. After it every read fails. A
field read directly into a member keeps the member's previous value: on the fresh object the class store
reads into, that is the constructor value (0 for most fields, §3). The exceptions:

| Field | State after a failed read |
|---|---|
| version (failed already, e.g. an empty record) | read into the argument slot that holds the stream pointer, so the "version" is the low 16 bits of the stream object's address. Above 6 → return false. Otherwise the load continues as that version, with every later read failing |
| `department`, powered grid | **null** (the ref helper clears the destination before reading, without releasing what it held) |
| deficit | copied from an uninitialised local: garbage (version < 6: garbage × 0.01f) |
| `nx`, `nz` | uninitialised locals: the consumption map is resized to garbage dimensions (new cells 0), usually a huge allocation or an out-of-memory failure |
| ring, annual, unused flag, recompute, agePlants | keep what the fresh object held. The constructor does not set these (§3), so they are whatever the allocation contained *[inferred: the game allocator does not clear memory]* |
| plant nodes | `plantCount` nodes are appended. Each failed occupant ref gives a node with a **null occupant**, and a failed `visited` keeps the previous loop value (or an uninitialised local). If `plantCount` itself failed it stays 0 and no node is read. A null plant crashes state 1 (`GetCurrentPowerProduced` and the property reads dereference it) |
| poles | if `poleCount` was read, the vector holds that many entries, and every failed ref is **null** (crash in §5.6 and §7.6). If `poleCount` failed it stays 0 |
| bitmaps | if `words` was read, four new arrays of that size whose unread words are uninitialised heap memory. If `words` failed it stays 0 (zero-size arrays, and every later bit access is out of bounds) |
| state | copied from an uninitialised local: garbage. `DoIdle` dispatches only states 0–4 (unsigned compare), so any other value stops the recompute cycle for good |
| strike flag | 0 (set after the reads) |

The result is unusable: `Init` sees `running` and skips the new-city setup. The null powered grid then crashes its
first user: the map view, the tract developer, the wire rebuild (§5.3 step 3) or UpdatePoweredLots (§6.5). *[choice]* FreeCity4: on a stream error `Read` returns **false**, does **not** set
`running`, releases any refs it acquired during the call (department, grid, plants, poles) and leaves every
member as it was before the call. The class store calls `Read` on a fresh object, so the following `Init` runs the
new-city path. A rejected version (> 6) keeps the original behaviour above (consumes only the version, returns
false, `running` set) *[compat]*.
- Recomputed on load: nothing. The tuning, the ordinance factor, enabled, wireDirty and brokenFraction are reset by
  construction/`Init`.
- **The unsaved pass values after a load** (PW3). The class store reads into a fresh object, and neither `Read`
  nor `Init` writes the pass import, export or demand, so all three are the constructor's **0**
  (`0x006EF805`–`0x006EF80B`) until state 0 zeroes them again. The saved state then continues (§6):
  - saved in state 4, 0 or 1: nothing changes. In state 4 and 0 the old pass values are zeroed by state 0 before
    anything adds to them, and a state 1 save comes right after state 0, when they were 0 already;
  - saved in state 2: the imports, exports and demand that state 1 added are lost, so state 3 publishes only state
    2's share (the networks reached from buy-deal connections);
  - saved in state 3: state 3 publishes 0 for all three.

  Example: a city saved in state 3 with pass import 500, export 200 and demand 6,000 publishes last import, export
  and demand 0, records 0 in histories `0x0AB2F224`, `0x4AB2F236` and `0xAA11F7D5`, and records `consumption − 0`
  (times the factor) in `0xAA11F7D2`, 200 more than without the load at factor 1.0. The next cycle is normal again. Production and consumption are
  saved, so they and everything derived from them alone are unaffected.
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

**FindPowerPlants** (`0x006EBA30`) *[compat]*, one pass over the plant list in order, two independent searches:

- **Lowest funded**: only plants whose budget item exists **and** for which `IsLocallyFundedObjectInDepartment(holder,
  0xCA58E540)` is true. That test is asked of the **simulator's own power department** (the member, §3), not of
  the item's department (`0x006EBA93`). Keeps the lowest `item.localFunding` (start 2.0, strict `<`).
- **Most worn**: **every** plant, including those that fail the funding filter. Keeps the largest `wear / d`
  (start 0, strict `>`). `wear` = f32 `0xC910B4CF` (default 0), `d` = t1 if `t2 < t1` else t2 (defaults 99). The
  quotient is stored as f32.

Each output pair is written only if a plant was found **and** its first out pointer is non-null: `*lowestFunded
:= occ` (`AddRef`) and `funding := its localFunding`; `*mostWorn := occ` (`AddRef`) and `wearPct :=
_ftol(ratio × 100)`. Otherwise the outputs are untouched.

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
literal (§6.3); shortfall consumes all remaining availability (§6.3); NaN ratio, harmless through `_ftol`
(§6.4); unpowered percentage at 24 bits (§6.6); overworked count practically 0 (§7.4); double ring advance (§7.5);
deficit values (§6.6, §7.5); history `0xAA11F7DC` always 0 (§6.6); brokenFraction not saved (§7.6); plant without
a condition curve produces nothing (§7.1); redundant `ChangeBudgetItemCost` (§7.3); a removed pole lowers
costBase even when it is not in the list, and brokenCount ignores removed distressed poles (§5.2); null lot in
the Nuclear Free Zone list (§5.5) and a department missing after a load (§4.1), both crashes and not
reproduced; `Read` ignores stream errors (§9; FreeCity4 fails cleanly instead).

## 13. Open questions

1. ~~Meaning of pole slot 13 (a byte flag that makes power ignore the pole) and of network flags `0x40000` and
   `0x80000000`.~~ **Answered 2026-10-21 (re-powerpoles):** slot 13 is `IsPreview` (the byte `+0x118` that slot
   14 sets; network-occupants.md §4.2); `0x40000` is the bridge-step flag and `0x80000000` the draft flag
   (network-manager.md §5).
2. ~~Who posts ConnectionsReady `0x6AC284F3`, and whether it always arrives after a load (otherwise a loaded city
   never recomputes power).~~ **Answered 2026-10-14 in foundations.md §11.6.**
   - The poster is `cSC4RegionalCity::CreateSymmetricConnections`, on TerrainReconciliation.
   - The trigger is the city terrain's second animation tick after its PostCityInit, so it arrives once per load.
   - The message is queued and carries no data.
   - With the default preference it **never arrives while popup dialogs are disabled**
     (`cISC4App::SetPopupDialogsEnabled(false)`), and it waits while a modal window is open.
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
   (0.9f = 0.89999998, so the exact product 500 × 0.9f is 449.99998808, which rounds to 450.0 at 24 bits; the earlier 449 was a 53-bit result), `0xAA11F7D8` = 500.
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
    −25536 at that cell and contributes no need. Float store: box x 48.04999924 .. 79.94999695 (float bits
    `0x42403333`, `0x429FE666`; Konradshohe's `ID32x16_3DirtyOut7`) → half-sum 63.99999809 in double, stored as
    the float32 64.0 (a tie, rounded to even) → cx = **4**, where the saved map has its 59. Without the float
    store it would be 3.
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
17. **Unpowered %** (24-bit, §6.6 step 5): consumers 4, powered 3 → 25; consumers 3, powered 2 → 33; consumers 0
    → 0; consumers 4, powered 5 → −25. Cases where a double evaluation differs: consumers 5, powered 3 → **39**
    (0.6f = 0.60000002, 1 − that = 0.39999998, × 100 rounds to 39.999996); consumers 10, powered 9 → **10**
    (10.0000019); consumers 20, powered 11 → **45** (45.0); consumers 5, powered 6 → **−20** (−20.000004).
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
    With no nuclear plant, `DemolishLots` is still called once, with an empty list, and nothing is demolished. The
    view input control check runs in both cases.
23. **Budget hookup** (§5.1): a plant whose item has line 0 and whose `GetBuildingType()` is 0 →
    `ChangeBudgetItemLine(holder, index, 0)`, then no line item lookup or creation, no `AddLocallyFundedObject`,
    no secondary-info change, no `0x2A58D350`. With building type `T ≠ 0` and no line item `T` →
    `CreateLineItemForBuildingType(T, true)`, `SetDisplayFlag(2, true)`, `AddLocallyFundedObject`, secondary info
    0 → 1, `0x2A58D350 := item.cost`. All calls go to the department named by the item.
24. **Departments** (§7.3, §10.1): a plant whose budget item belongs to department X (≠ the power department) and
    whose maintenance changes → `X.SetLocalFullFunding(...)`. If X does not exist (`GetDepartmentBudget` null), neither
    `SetLocalFullFunding` nor `ChangeBudgetItemCost` is called. `FindPowerPlants`: P1 funded 50 % and locally
    funded in the power department, wear 0.2; P2 funded 30 % but not in the power department, wear 0.5 (t1 0.9,
    t2 1.0) → lowest funded P1 (0.5), most worn **P2** with `wearPct` 50. Null out pointers are skipped without a
    fault.
25. **Counters** (§3): consumers 0, remove a consumer → consumers stays 0. Requirement 100, remove a consumer of
    300 → requirement 0. A coal plant (type 1) whose `0x27812853` is set to 3 before removal → perType[1] stays 1,
    perType[3] = −1, `GetPowerPlantCount(3)` returns `0xFFFFFFFF`, plantCount back to its previous value.
26. **Pole removal** (§5.2): costBase 10, poleCount 10; removal of a pole that is not in the list → costBase 9,
    recompute true, poleCount 10. Removing a distressed pole leaves brokenCount unchanged.
27. **Terrain change** (§5.4): the occupant query returns poles A (cell inside the rect) and B (cell outside) →
    only A gets `BreakAllConnections(true)` and `RemoveOccupant(A, true, 0)`, also when A is not in the pole list.
    Two lines with the same pole pair → one `RemoveConnection` call.
28. **NaN ratio** (§6.4): plant list order: network 1 with a 1000 plant and need 500, then network 2 with one plant
    without `0x8A1E07EE` (output 0) and no consumer → network 2's plant gets `0x08D9763D` = **0** and its ring
    entry gains 0.
29. **Init after load** (§4.1): with the department loaded by `Read`, `Init` makes no `GetDepartmentBudget` call
    and keeps the loaded object. With a null department ref and an existing department `0x8910BC8A` in the budget,
    `Init` looks it up and `AddRef`s it.
30. **Truncated save** *[choice]* (§9.1): a record cut off after the powered-grid ref → `Read` returns false,
    `running` stays clear, the department and grid refs read during the call are released, every member is as
    before. The next `Init` runs the new-city path (bitmaps, grid, department, state 4). A record of version 7 →
    false with `running` set.
31. **Running counts** (§5.2, PW8): insert a building with `0x27812854` = 1 centred at (5,5), then set the
    property to 0 and remove the building → consumers, requirement and cell (5,5) stay at 1. Load Konradshohe →
    consumers 725, requirement 6478, cells (41,102) and (55,111) = 1, and they stay so while no occupant changes.
