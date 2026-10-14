# Spec: the pollution and garbage subsystem (`cISC4PollutionSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program (Windows `SimCity4.exe` 1.1.641) and its data
files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only.
They must **not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, class store §3, response curves §4, messages §5,
random numbers §6 and §12, month order §13, slot order §16), [simulator.md](simulator.md) (agents),
[simgrid.md](simgrid.md) (grid classes §2–§6, `AddRadiusEffect` §7.1, `ProcessTwoGridsWithCallback` §7.3,
iterators §7.4), [budget.md](budget.md) (departments, line items, budget items), [neighbors.md](neighbors.md)
(garbage deals), [ordinance.md](ordinance.md) (`GetCumulativeFactor`), [weather.md](weather.md) §6.7 (random
cell iterator).

Earlier write-up: `docs/subsystems/grid-sims.md` §3. Several of its statements are corrected here (§10.3).

---

## 1. Role

The pollution simulator owns four kinds of pollution and the city's garbage flow:

1. **Sources.** Every building whose exemplar has a pollution property stamps a linear cone into four
   **source grids** (air, water, garbage, radiation) when it is added to the city, and removes it when it
   leaves. Disasters add permanent radiation cones.
2. **Values.** Each day a number of 2×2-cell tracts are visited in a shuffled order. For each visited tract the
   simulator turns the source values, modified by ordinances, zoning and traffic, into a **target**, and moves the
   **air** and **water value grids** toward that target by at most a fixed step per visit.
3. **Garbage.** Each visited tract produces garbage. It is recycled, burned (incinerators, waste-to-energy),
   exported to neighbours, sent toward landfills (the "garbage supply"), left to decompose, or left lying in the
   cells of the **garbage value grid**.
4. **Statistics, budget and effects.** At the end of each full pass over the map it computes averages and
   records history; monthly it books the sanitation cost, rolls the monthly garbage counters, ages the sanitation
   buildings, and refreshes the smoke/sludge/garbage/radiation visual effects. It also asks the prop developer for
   garbage props.

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| `cSC4PollutionSimulator` | `0x8990C065` | `cISC4PollutionSimulator` `0x877181EC`, `cIGZUnknown` `1`, `cIGZMessageTarget2` `0x452294AA`, `cISC4PropRequester` `0x89C498BE`, `cIGZSerializable` `0xE4FDA3D4` |

- `QueryInterface` answers exactly these five IIDs; anything else fails and stores nothing *[compat]*.
- It is city component **#34** (foundations §3.6); on read the city asks for IID `0x877181EC`. Other code reaches
  it through `cISC4City::GetPollutionSimulator()` (city compiled slot 66, `+0x108`), and the game also caches the
  pointer in a global service slot at city init.
- City `Init` order (foundations §3.7): after **traffic**, before **residential**. Everything it reads at `Init`
  (city, zone manager, traffic, residential, neighbours, power, ordinance, effects manager, landfill developer,
  budget, prop developer, history warehouse) already exists.

## 2. Grids and coordinates

Units: a **cell** is a city cell (`cISC4City::CellCountX/Z`, 16 m in shipped cities). A **tract** is a grid
element; its size in cells is the grid's tract size (simgrid.md §3).

All eight grids are `cSC4SimGrid<int16>` (CLSID `0x49B9E605`, simgrid.md §2):

| Grid | Tract size | Instance ID | Meaning |
|---|---|---|---|
| air source | 2 | `0x89D5BBF0` | sum of building air cones |
| water source | 2 | `0x89D5BBF1` | sum of building water cones |
| garbage source | 2 | `0x89D5BBF2` | sum of building garbage production (rectangles, §4.2) |
| radiation source | 2 | `0x89D5BBF3` | building radiation cones, `SetIsRadioactive` flags and disaster radiation |
| air value | 2 | `0x49D5BC40` | relaxed air pollution (what the game shows and other systems read) |
| water value | 2 | `0x49D5BC41` | relaxed water pollution |
| garbage value | **1** | `0x49D5BC42` | uncollected garbage per **cell** (bits 0–14) and the **active-landfill flag** (bit 15) |
| radiation value | 2 | `0x49D5BC43` | created, saved and loaded, but **never read or written** *[inferred]* (radiation is read from the source grid) |

The garbage value element packs two fields *[compat]*: `amount = raw & 0x7FFF`, `landfill = raw bit 15`.

The **pollution type** index used by several methods is 0 air, 1 water, 2 garbage, 3 radiation.

## 3. State

"Persisted" refers to §6. Initial values are those of a new city (constructor, then `Init` on a city that was not
loaded).

### 3.1 Scalars

| Field | Initial | Persisted | Meaning |
|---|---|---|---|
| `initialized` | false | no | set by `Init`, cleared by `Shutdown` |
| `loaded` | false | no | set by `Read`; makes `Init` skip new-city setup |
| `cellX, cellZ` | first iterator position × 2 | yes | the **cell** coordinates of the next tract to visit (always even) |
| `tractsPerDay` | §5.1 | yes | tracts visited per day |
| `iterator` | §5.1 | yes | random cell iterator over the source tracts, **table mode** |
| `effectIterator` | prime-step mode | no | random cell iterator over 8×8-cell blocks, reset at each use (§4.10) |
| `recyclingCount` | 0 | yes | number of recycling centres in the recycling list |
| `avgAir, avgWater` | 0 | yes | end-of-cycle averages (int32) |
| `avgGarbage` | 0 | yes | end-of-cycle average uncollected garbage (uint16) |
| `airSum` | 0 | yes | Σ air target over tracts visited this cycle |
| `waterSumTreated` | 0 | yes | Σ water target after treatment, this cycle |
| `waterSum` | 0 | yes | Σ water target before treatment, this cycle |
| `treatedSum` | 0 | yes | Σ water removed by treatment plants, this cycle |
| `lastWaterSum, lastTreatedSum` | 0 | yes | the two values above from the previous cycle |
| `developedTracts` | 0 | yes | developed tracts visited this cycle |
| `radiationSum` | 0 | **no** | Σ radiation source value over tracts visited this cycle |
| `incineratorCapacity` | 0 | yes | Σ current incinerator capacity, recomputed daily (slot 31) |
| `wteCapacity` | 0 | yes | Σ current waste-to-energy capacity, recomputed daily (slot 32) |
| `recyclingEffect` | the default percentage (0.0 *[inferred]*) | yes | fraction of produced garbage that is recycled (§4.6.4) |
| `garbageUncollectedFlag` | false | yes | `totalUncollected ≠ 0` at the last end of cycle |
| `totalUncollected` | 0 | yes | running total of garbage lying in cells (uint32, §4.6) |
| `produced, imported, exported, incinerated, wte, landfilled` (this month) | 0 | yes | int64 counters |
| `recycled` (this month) | 0.0 | yes | float64 counter |
| the same seven, **last month** | 0 | yes | copies made by the monthly simulation |
| `exportCapacity` (this month) | 0 | yes | int32; export room bought from neighbours this month |
| `decomposed` (this month / last month) | 0 | yes | uint32; garbage that "disappeared" (SDK: *abducted*) |
| `activeLandfillCells` | 0 | yes | number of cells with the landfill flag |
| `unused64` | 0 | yes | an int64 that is saved and loaded but never written *[inferred]* |
| `lastMonthCost` | 0 | yes | int64; last garbage-disposal bill (§4.9) |
| `costPerUnit` | 0, then tuning | no | `0xAA1F8AB7` |
| `dirtyRadiation, dirtyAir, dirtyWater, dirtyGarbage` | false | yes | set by grid changes, cleared at end of cycle (§4.8) |
| `effectsVisible` | **true** | no | set by PreViewLevelChange `0xC6CDBF1A` (§5.3): true only in the surface view |
| `worstAir, worstWater, worstGarbage` | (0, 0, 0) | no | `(cellX, cellZ, value)` of the worst 8×8 block, refreshed by §4.10 |
| `efficiency` | **0.0** | yes | sanitation efficiency (§4.7) |
| `garbageSupply` | 0 | yes | int32 ≥ 0; garbage waiting for landfills ("valve") |
| `garbageCapacity` | 0 | yes | int32; set by the landfill developer |
| `department` | null, then created by `Init` | yes (ref) | the sanitation department budget |

### 3.2 Lists and effect slots

| Member | Persisted | Content |
|---|---|---|
| water-treatment list | yes (refs) | occupants with property `0xC8EBD131` |
| incinerator list | yes (refs) | occupants with property `0xA8EBCF0C` |
| waste-to-energy list | yes (refs) | occupants with property `0x48EBD0B1` |
| recycling list | yes (refs) | occupants with property `0x88ED1059` |
| four visual-effect arrays (water, air, radiation, garbage) | no | one optional effect object per 8×8-cell block, `(CellCountX/8) × (CellCountZ/8)` entries, index `bz × (CellCountX/8) + bx` |

Lists hold references (AddRef on insert, Release on remove). They allow **duplicates** (§4.2.3).

### 3.3 Static (process-wide) tuning

Tuning values live in process-wide statics *[compat: only matters if two cities existed at once, which the
game never does]*. They are re-read on `Init` and on message `0xCA1B151C` (§7.1).

## 4. Behaviour

### 4.1 Lifecycle

#### `Init()` → true

1. Once per process: create the six static response curves (prop request, four effect strengths, garbage
   transport) and set the update period constant to **30**.
2. If **not loaded** (a new city):
   1. Create each missing grid directly (simgrid.md §6.2): CLSID `0x49B9E605`, `AddRef`, `Init`,
      `SetTractSize` (§2), `SetInstanceID` (§2).
   2. Zero every statistic and counter of §3.1; `recyclingEffect` := default; recount the recycling list.
   3. `tractsPerDay := (tractCountX × tractCountZ) / 15` (integer division; 15 = 30 ÷ 2), counts taken from the
      air source grid.
   4. Iterator: table mode, rectangle `[0, tractCountX − 1] × [0, tractCountZ − 1]`, `Reset(shuffle)`, then one
      `GetNext` → `(tx, tz)`; `cellX := 2·tx`, `cellZ := 2·tz`.
   5. Clear the four dirty flags.
   6. `propDeveloper.AddPropRequester(prop-requester interface)` (compiled slot 5). This happens only for a new
      city. A loaded city gets the registration back from the prop developer's own save (§10.2 question 1), so
      the requester must **not** be added again there.
3. If **not initialized**:
   1. Set `initialized`; read tuning (§7.1).
   2. `simulator.AddAgent(self, 6, "cSC4PollutionSimulator", 2)` then `AddAgent(self, 4, "cSC4PollutionSimulator", 2)`
      (flags 2: runs only once the city is established, simulator.md §4).
   3. Subscribe to `0x99EF1142`, `0x99EF1143`, `0x26D31EC1`, `0xCA1B151C`, `0xC6CDBF1A`.
   4. Size the four effect arrays to `(CellCountX/8) × (CellCountZ/8)` (C integer division of the signed counts),
      all empty.
   5. Effect iterator: prime-step mode over `[0, CellCountX/8 − 1] × [0, CellCountZ/8 − 1]`.
   6. If there is no department: `budget.CreateDepartmentBudget(0x6A42A8F5, 0x4A357EAF)`;
      `SetDepartmentName(0x6A3FF01C, 0xCA4938F8)`; `SetMaxAllowedFundingPercentage(1.2)`; create line items
      **10**, **20**, **30** (not locally funded) named (`0x6A3FF01C`, `0xCA57FE96`), (`0x6A3FF01C`,
      `0x4A57FFF0`), (`0x6A3FF01C`, `0x0A580005`).
   7. `historyWarehouse.SetHistoryDataQueueLength(0xEA110138, 2)`.
4. Acquire the city's ordinance, zone manager, traffic, residential, neighbours and power simulators, effects
   manager and landfill developer (the landfill developer without `AddRef`).

#### `Shutdown()` → true

Clear `initialized`; release the simulators and the department; release each occupant list through the occupant
manager (compiled slot 48) and recount; for every effect object: if `IsStarted` then `Stop(1)`, then release it;
empty the arrays; release the effects manager and the city; `Shutdown` and release each grid; unsubscribe the
five messages; `simulator.RemoveAgent(self)` (all levels); `propDeveloper.RemovePropRequester(requester)`.

### 4.2 Building changes (`0x99EF1142` insert, `0x99EF1143` remove)

The occupant is taken from the message. `sign = +1` on insert, −1 on remove.

#### 4.2.1 Skip

If the occupant has the bool property `0x029244DB` and it is true, **nothing** happens (no cones, no lists, no
budget) *[compat]*.

#### 4.2.2 Pollution cones

If the occupant has property `0x27812851` (*pollution at centre*, int32 array `[air, water, garbage, radiation]`):

- `outer` := property `0xAA5832F3` (int32 array, same layout), or all zeros.
- **Garbage rectangle**: take the occupant's bounding box (occupant compiled slot 10), convert both corners to
  cells (`cISC4City::PositionToCell`), clamp to the map, and for **every cell** of the inclusive rectangle add
  `sign × centre[2]` to the garbage **source** tract containing it (`cell >> 1`). A 2×2-cell tract covered by k
  cells of the footprint receives k additions *[compat]*. The new value is stored truncated to int16 (wrap, no
  clamp) *[quirk]*. Set `dirtyGarbage`.
- **Cones**: `radii` := property `0x68EE9764` (float array, in **tracts**), default `{2.5, 1.5, 0.5, 4.0}`. For
  each type i in 0..3 with `radii[i] > 0`: centre tract := source grid i's `PositionToTract(position.x,
  position.z)` (occupant position); apply `AddRadiusEffect` (simgrid.md §7.1, int16 variant) to source grid i
  with radius `radii[i]`, centre value `sign × centre[i]`, outer value `sign × outer[i]` (both converted to
  float). The garbage cone (i = 2) is applied **in addition** to the rectangle *[compat]*.
- The arrays are read without a length check *[compat]*: a shorter array reads beyond its end in the original.
  *[choice]* FreeCity4 treats missing elements as 0.
- The cone and the rectangle do **not** set the air/water/radiation dirty flags; only the garbage flag is set
  *[compat]*.

Removal applies the same arithmetic with negated values, so it restores the grid exactly unless a clamp or wrap
happened in between *[quirk]*.

#### 4.2.3 Building lists

Membership is decided by **presence** of a property; one occupant can be in several lists.

| Property present | List | On insert also | On remove also |
|---|---|---|---|
| `0xC8EBD131` (water treatment capacity) | water-treatment | set u32 `0x28ECF510` := 0 and `0x08FCDB69` := 0 | — |
| `0xA8EBCF0C` (incinerator capacity) | incinerator | set `0xE8ECF641` := 0, `0xA8FCDBA8` := 0 | — |
| `0x48EBD0B1` (waste-to-energy capacity) | waste-to-energy | set `0x88ECF76C` := 0, `0x28FCDBB0` := 0; if u32 `0x27812852` exists, copy it to `0x08FD1613` and remove `0x27812852` *[inferred: moves the exemplar's power output into a "max output" property, §4.9]*; `wteCapacity += capacity` | `wteCapacity −= capacity` |
| `0x88ED1059` (recycling percentage) | recycling | recount | recount |

`wteCapacity` is clamped at ≥ 0 after either change (it is recomputed daily anyway). Insert = `AddRef` + append
with **no duplicate check**; remove = remove the **first** match and `Release` it *[compat]*.

#### 4.2.4 Budget

`budget.GetBudgetItemForPurpose(holder, 0xCA58EDC7, &item)`. If found **and** `item.lineItem == 10`: insert →
`AddBudgetItemToBudget(holder, item.index)`, remove → `RemoveBudgetItemFromBudget(holder, item.index)`. If found
(any line), set the occupant's int64 property `0x2A58D350` := `item.cost` (on insert and on remove). This stores
the building's base maintenance cost for §4.9.

### 4.3 Radiation from disasters (`AddRadiationToCity(position, strength, radius)`)

If `radius > 0` and `strength ≠ 0`: centre := radiation source grid `PositionToTract(position.x, position.z)`;
`AddRadiusEffect` on the radiation **source** grid with radius `radius` (tracts), centre value `strength`, outer
value 0; then refresh the visual effects (§4.10) and post `0xDA6DD35D` (§5.2). There is no removal: disaster
radiation is permanent *[compat]*. Callers: the fire disaster (`ToastOccupant`, nuclear plant destruction) and
the power simulator (aged nuclear plant meltdown).

### 4.4 Daily simulation (level 4, SimNewDay `0x66956814`)

Order within one day *[compat]*:

#### 4.4.1 Disposal capacities

- `wtpUnused := Σ over the water-treatment list of max(0, current(0xC8EBD131) − used 0x28ECF510)`, where
  `current(p)` is `GetCurrentCapacity(p, occ)` (§8, slot 53) and an occupant whose query or property fails adds 0.
- `incineratorCapacity := Σ current(0xA8EBCF0C)`; `incUnused := Σ max(0, current − used 0xE8ECF641)`.
- `wteCapacity := Σ current(0x48EBD0B1)`; `wteUnused := Σ max(0, current − used 0x88ECF76C)`.
- The three "remaining" counters start equal to the unused totals and are consumed during the day.

#### 4.4.2 Neighbour deals (neighbors.md)

- **Import** (deal type `0x4773FFE5`, "sell garbage disposal": this city accepts trash): for each deal with
  `amount − amountUsed > 0`: `n := roundHalfEven(amount × 0.1f)` (not capped by the room left *[quirk]*);
  `importTotal += n`; `SetAmountUsed(amountUsed + n)`; `imported += n`.
- **Export** (deal type `0x4773FFE4`, "buy garbage disposal"): for each deal with `room := amount − amountUsed >
  0`: `n := min(roundHalfEven(amount × 0.1f), room)`; `SetAmountUsed(amountUsed + n)`; `exportCapacity += n`.
- **Rounding** *[compat]* (foundations §18): `0.1f` is a `float32` constant (0.100000001490116…). The int32 amount
  enters the multiply exactly, the product is rounded to `float32`, and that `float32` is rounded half to even
  with the 1.5·2²³ idiom (crime.md §6 notation; exact for products up to 2²², i.e. amounts up to 41,943,040).
  So amount 25 → 2.5 → **2** (a single rounding of the exact product 2.50000004 would give 3), 35 → 4, 15 → 2,
  1000 → 100.
- Then `eaten := Dispose(importTotal)` (§4.6.1) and `AddToGarbageSupply(trunc(importTotal − eaten))`.

#### 4.4.3 Factors

Read once per day from the ordinance simulator (`GetCumulativeFactor(id, index)`, ordinance.md §6):

| Factor | ID, index |
|---|---|
| `fTraffic` | `0x8A67E378`, 0 |
| `fAir` | `0x08F79B8E`, 0 |
| `fWater` | `0xE8F79C8B`, 0 |
| `fGarbage` | `0xE8F79C90`, 0 |
| zone multipliers for zone type z = 0..15 | air `0x8A67E373`, water `0x8A67E374`, garbage `0x8A67E376`, index z |

#### 4.4.4 One tract visit (repeated `tractsPerDay` times)

Let `(x, z) := (cellX, cellZ)` (cell coordinates of the tract's corner) and `t := (x/2, z/2)`.

1. **Sources**: `a, w, g, r :=` max(0, value) of the air, water, garbage, radiation **source** cell `(x, z)`.
2. **Ordinances**: `a := trunc(a × fAir)`, `w := trunc(w × fWater)`, `g := trunc(g × fGarbage)`, each then held as
   int16.
3. **Zoning**: for the four cells `(x + (k & 1), z + (k >> 1))`, k = 0..3, read the zone type (zone manager).
   For each cell whose zone type is **not 14** (landfill): add its three zone multipliers to running sums and
   `count += 1`; if its zone type is also **not 0**, the tract is **developed**. If developed:
   `developedTracts += 1`. If `count > 0`: `a := int16(trunc(a × Σair / count))`, same for `w` (water sum) and
   `g` (garbage sum) — the float product is truncated to 32 bits and then to 16 bits (wraps) *[quirk]*.
4. **Per cell** k = 0..3, cell `c = (x + (k & 1), z + (k >> 1))`:
   1. `cur := GetGarbageValue(c)` (own slot 9, landfill bit masked as in §8).
   2. **Traffic**: `A := trunc(A + traffic(c) × fTraffic × trafficAirFactor)`, where `A` starts as `a`,
      `traffic(c)` is the uint8 cell value of the traffic simulator's air-polluting traffic map
      (`GetAirPollutingTrafficMap`, slot 8; `GetCellValue(c)`), and `trafficAirFactor` is tuning `0x69501944`.
      Landfill cells contribute traffic too.
   3. If the cell's zone type is 14: next cell.
   4. **Garbage produced**: `gp := float(g) / count`; `produced += roundHalfEven(gp)` (sign-extended into the
      int64 counter).
   5. **Recycling**: if `recyclingEffect > default` (0.0) and `gp > 0`: `rec := min(gp × recyclingEffect, gp)`;
      `recycled += rec`; `gp −= rec`.
   6. `total := gp + cur`. Set the cell's garbage amount to 0 (`SetGarbageValue`, keeps the landfill bit). If
      `totalUncollected ≥ cur`: `totalUncollected −= cur`.
   7. `left := total − Dispose(total)` (§4.6.1).
   8. If `left > 0`:
      - `left > 32767`: amount := 32767; `totalUncollected += 32767`; if `left − 32767 > 0`:
        `AddToGarbageSupply(trunc(left − 32767))`.
      - else if `garbageCapacity − garbageSupply ≥ 0` or `left ≥ 32767`: amount := `trunc(left)`;
        `totalUncollected := trunc(totalUncollected + left)`.
      - else (**valve closed**, supply above capacity): `over := garbageSupply − garbageCapacity`;
        `share := roundHalfEven(g / count)`; `take := over` if `over ≤ share`, else `share` (or **1** when `g = 0`);
        if `take + left > 32767`: `take := trunc(32767 − left)`. `AddToGarbageSupply(−take)`; amount :=
        `trunc(take + left)`; `totalUncollected := trunc(totalUncollected + take + left)`. Garbage flows **back**
        from the supply into the streets *[compat]*.
5. `waterSum += W` where `W := w` (the zone-averaged water).
6. **Water treatment**: if `wtpRemaining > 0` and `W > 0`:
   - if `lastWaterSum > 0`, `lastTreatedSum > 0` and `wtpRemaining ≤ lastTreatedSum`:
     `t := roundHalfEven(lastTreatedSum / lastWaterSum × W)`, `t := 1` if that is 0. The arithmetic is
     `float(lastTreatedSum)`, divided by the int32 `lastWaterSum` and multiplied by the int32 `W` (integer
     operands enter exactly, each result rounded to `float32`, foundations §18);
   - otherwise `t := wtpRemaining`;
   - `t := min(t, W)` (signed), then `t := min(t, wtpRemaining)` (unsigned);
   - `treatedSum += t`, `W −= t`, `wtpRemaining −= t`.

   The previous cycle's treated fraction spreads treatment evenly over the map; without history (first cycle),
   treatment is greedy in visit order *[compat]*.
7. `waterSumTreated += W`; `airSum += A`; `radiationSum += r`.
8. Clamp `A` and `W` to [0, 32767].
9. **Air relaxation**: `v := airValue(t)`. If `v < A`: `v += min(A − v, maxAirDelta)`; if `v > A`:
   `v −= min(v − A, maxAirDelta)`; written with `SetAirValue` (marks `dirtyAir`) only when they differ.
10. **Water relaxation**: same with `maxWaterDelta` and `SetWaterValue`, **except** when a decrease would give
    exactly **0** (the *neighbour special case*, *[quirk]*): the value becomes **1** if any of these reads, done in
    order and stopping at the first hit, returns a water value **> 1** (`GetWaterValue`, cell coordinates):
    `(x − 1, z)` if `x > 0`; `(x, z − 1)` if `z > 0`; `(x + 1, z)` if `x < tractCountX − 1`; `(x, z + 1)` if
    `z < tractCountZ − 1`. Notes *[compat]*:
    - `x + 1` and `z + 1` are in the **same tract** (cell coordinates are even), so they read the tract's own
      current (pre-update) value: any tract with value ≥ 2 that decays to 0 stops at 1 instead.
    - The bounds compare **cell** coordinates with **tract** counts, so the own-tract reads only happen for
      `2·tx < tractCountX − 1` (resp. z), i.e. roughly the lower-x (lower-z) half of the map.
    - **The special case always writes** (PG14, `0x006E8B87`–`0x006E8C4B`). Once the decrease gives 0, the code
      calls `SetWaterValue` with 1 (a neighbour read > 1) or with 0 (none), without comparing with the current
      value. The case is entered only when `W = 0` (after the clamp of step 8) and `0 < v ≤ maxWaterDelta`. So
      with **v = 1** and a hit, `SetWaterValue(x, z, 1)` rewrites the 1 that is already there: no cell changes,
      but `dirtyWater` becomes **true** (`SetWaterValue` `0x006E30B0` sets it unconditionally, `0x006E30DC`).
      Every other water write, and every air write, has `v ≠ W` and a step of at least 1, so it changes the cell.

      *Example*: shipped `maxWaterDelta` 150, tract at cell (4, 6) with water value 1, `W = 0` this visit, and
      cell (3, 6) (the tract to the −x side) at 5. The decrease gives 0, the first read hits, and
      `SetWaterValue(4, 6, 1)` runs: the grid is unchanged, `dirtyWater` is true, and the next end of cycle
      (§4.8) posts "water changed" `0x5A6DD1AA` and clears it.

      This is the day's **only** `SetWaterValue` call (`0x006E8C4B`), and nothing else in the day writes
      `dirtyWater`: `DoDailySimulation` `0x006E7FD0`–`0x006E8E1F` has no other reference to it. The end of cycle
      that runs inside the day (step 11, `0x006E8C69`) clears it, so a later visit the same day can set it again.
      *[compat]* A reimplementation must mark `dirtyWater` whenever this case runs, even if the value stays 1.
    - *[compat]* With `maxWaterDelta` = 0 (or `maxAirDelta` = 0; not shipped), the step is 0. Then every visit with
      `v ≠ W` writes the unchanged value and marks the flag. The compare against the delta is unsigned
      (`0x006E8B61`, `0x006E8B7F`).
11. **Advance**: if the iterator is exhausted (simgrid.md §7.4): `Reset(shuffle)` and run the **end of cycle**
    (§4.8). Then `GetNext → (tx', tz')`; `cellX := 2·tx'`, `cellZ := 2·tz'`.

#### 4.4.5 After the visits

1. **Valve overflow**: if `garbageCapacity − garbageSupply < 0`: `excess := garbageSupply − garbageCapacity`;
   `e := trunc(Dispose(excess))`; if `excess − e > maxGarbageOnValve` (unsigned): `e += (excess − e) −
   maxGarbageOnValve`; `AddToGarbageSupply(−e)`. The supply therefore never stays more than `maxGarbageOnValve`
   above capacity after a day *[compat]*.
2. **Usage bookkeeping**: for water treatment, incinerators and waste-to-energy, `used := unused − remaining`;
   if non-zero, distribute it over the list (`SetOccupantListUsedCapacity`, §4.9.3).
3. **Maintenance costs**: recompute building costs (§4.9.4) for the water-treatment, incinerator and recycling
   lists (**not** waste-to-energy *[compat]*).

### 4.5 Iteration cadence *[compat]*

- One full pass ("cycle") visits every source tract exactly once in a shuffled order. A large city (256 cells →
  128 × 128 tracts) visits `16384 / 15 = 1092` tracts per day; a cycle completes 4 visits into the 16th day,
  and the pass continues with the new order in the same day.
- Each reset is `Reset(shuffle)` in **table mode**: the table of all `tractCountX × tractCountZ` entries is
  `random_shuffle`d with the C runtime `rand()` (foundations §12.3): **n − 1 `rand()` calls** per reset (16383 for a
  large city, 4095 for a medium one, 1023 for a small one). Resets happen in new-city `Init` and at every cycle end.
- Air and water value grids therefore change at most once per cycle per tract, by at most `maxAirDelta` /
  `maxWaterDelta`. With the shipped delta 150 a tract needs ⌈target / 150⌉ cycles to reach its target.

### 4.6 Garbage flow

#### 4.6.1 `Dispose(g, incRemaining, wteRemaining)` → amount disposed

Called with the day's running "remaining" capacities (§4.4.1). `g` is a float.

1. If `g ≤ 0`: return 0.
2. **Export**: `room := exportCapacity − exported` (32-bit, the low half of the int64 counter). If `room > 0`:
   `x := min(g, room)`; `exported := trunc(exported + x)`; `g −= x`.
3. **Waste-to-energy**: if `wteRemaining ≠ 0`: if `wteRemaining > g` (unsigned; compared as below):
   `wteRemaining −= floor(g)` (the 1.5·2²³ floor of §4.12, not `_ftol`), `x := g`,
   `g := 0`; **otherwise `wteRemaining := 0` and nothing is taken** *[quirk]* (a plant with less remaining
   capacity than the offered amount is emptied without burning anything). `wte := trunc(wte + x)`.
4. **Incinerators**: identical with `incRemaining` and the `incinerated` counter.
5. **Landfill**: `(usage, capacity) := landfillDeveloper.GetLandfillUsage()`; `room := capacity − usage`;
   `limit := roundHalfEven(efficiency × transportCurve(landfillDeveloper.GetLandfillTileCount())) −
   garbageSupply` (32-bit wrapping subtraction); `n := min(limit, room)` (signed). The rounding is the
   1.5·2²³ idiom and **does leave its exact range in play**: with the shipped curve (5000 per tile) and efficiency
   1.0, more than 838 landfill tiles give a product above 2²², so 839 tiles → 4,194,652 (not 4,195,000) and 1000
   tiles → 4,597,152 (not 5,000,000) *[compat]* (§4.12). If `n > 0`: if `g > n`: `AddToGarbageSupply(n)`, `g −= n`; else
   `AddToGarbageSupply(roundHalfEven(g))`, `g := 0`. (The landfill developer later moves the supply into landfill
   tiles and calls `GarbagePutInLandfill`, which feeds the `landfilled` counter.)
6. **Decomposition** ("abduction"): `room := decompositionAmount − decomposed`; if `room > 0`:
   `x := min(g, room)`; `decomposed := trunc(decomposed + x)`; `g −= x`.
7. Return the total taken from steps 2–6 (float).

The decomposition allowance (`0x09135479`, shipped 2000 per month) is consumed first-come in visit order.

**How `remaining > g` is compared in steps 3 and 4 (PG8) *[compat]*.** The remaining capacity is converted as
**unsigned** (waste-to-energy `0x006E38EA`–`0x006E3903`, incinerators `0x006E39A5`–`0x006E39BE`). The code loads
it as a signed 32-bit integer onto the x87 stack. If its top bit is set, it adds 2³² (`4294967296.0f`,
`0x00A80AA8`). It then compares the result with the `float32` `g`. The value is never stored as a `float32`, so:

- **Below 2³¹** the comparison is **exact**: the integer itself is compared with `g`.
- **From 2³¹ up**, the add of 2³² runs under the main thread's 24-bit precision (foundations §18.1). The value is
  therefore rounded to `float32` (nearest, ties to even) before the comparison.

A replacement gets the same result by converting the value to `f64` (exact for every `u32`) and comparing it with
`g as f64` when the value is below 2³¹, and by comparing `(value as f32) > g` otherwise.

| Remaining | `g` | Original (`remaining > g`) | Converting to `f32` first |
|---|---|---|---|
| 1,000 | 999.5 | true | true |
| 16,777,217 (2²⁴ + 1) | 16,777,216.0 | **true** (exact) | false (rounds to 16,777,216) |
| 2,147,483,649 (2³¹ + 1) | 2,147,483,648.0 | **false** (rounded to 2³¹) | false |
| 3,000,000,000 | 2,999,999,744.0 | true | true |

The remaining capacities are sums of the plants' unused capacities (§4.4.1). Every method agrees while they stay
below 2²⁴.

#### 4.6.2 Supply, capacity and landfill flags

- `garbageSupply` is changed only by `AddToGarbageSupply` (saturating int32 add, then clamped ≥ 0).
- `garbageCapacity` is set by the landfill developer (`SetGarbageCapacity`, monthly).
- The landfill flag of a cell is set/cleared by `SetIsActiveLandfill` (only on cells whose zone type is 14,
  §8 slot 50); it maintains `activeLandfillCells`.

#### 4.6.3 Recycling effect (`CalculateRecyclingCenterEffect`, at each end of cycle)

1. `P := residential.GetPopulation()` as float.
2. If `recyclingCount = 0`: `recyclingEffect := default`; stop.
3. Re-order the recycling list by **descending** current capacity (`GetRecyclingCenterCurrentCapacity`, slot 55;
   a new element is inserted before the first element whose capacity is ≤ its own; elements whose query fails
   are dropped from the list *[inferred: never happens for list members]*).
4. Walk the list while `served < P`: `pop :=` float property `0x68ED1736` (population served); if present:
   `served += pop`, and if the capacity query succeeds, `weighted += capacity × pop`.
5. If `served > 0`: `recyclingEffect := weighted / max(served, P)`; else `recyclingEffect := default`.

### 4.7 Sanitation efficiency

At each end of cycle (and in the unreachable SimBegin handler): load curve `0x6911E2DF` from the tuning exemplar
(fresh each time) and set `efficiency := curve(fundingFraction × 100) × 0.01f`, where `fundingFraction` is
`*department.GetFundingPercentage()` (a pointer return, budget.md). Shipped: funding 100 % → 1.0; 50 % → 0.25.
*[quirk]* The shipped curve with that ID is labelled "Power Funding To Efficiency Curve".

*[compat]* Arithmetic (foundations §18): three `float32` operations. `x := f32(fundingFraction × 100.0f)` (the
curve input is passed as a `float32`), `y := curve(x)`, `efficiency := f32(y × 0.01f)`. Both constants are
`float32` literals; `0.01f` = 0.00999999977648258…, slightly **below** 0.01, so the result is not always the
nearest `float32` to `y / 100`: `y` = 100 → 1.0, 50 → 0.25, 25 → 0.25, but **80** (shipped funding 90 %) →
**0.79999995** (`0x3F4CCCCC`, not 0.8f = `0x3F4CCCCD`). Because `GetCurrentCapacity` truncates (§8 slot 53), a
base capacity of 1000 at 90 % funding gives **799**, not 800.

*[quirk]* `efficiency` starts at **0.0** in a new city and stays 0 until the first end of cycle (about 15 days):
all `GetCurrentCapacity` results are 0 and no garbage goes to landfills until then.

### 4.8 End of cycle

1. Recompute `recyclingEffect` (§4.6.3).
2. `garbageUncollectedFlag := totalUncollected ≠ 0`.
3. If `developedTracts > 0` (unsigned divisions): `avgAir := airSum / developedTracts`; `avgWater :=
   waterSumTreated / developedTracts`; `avgGarbage := uint16(totalUncollected / developedTracts)`; `radPct :=
   radiationSum × 100 / developedTracts`. Else all three averages := 0 and `radPct := 0`. *[quirk]* The sums
   include **undeveloped** tracts but are divided by the developed count.
4. `lastWaterSum := waterSum`; `lastTreatedSum := treatedSum`.
5. History (`RecordHistoryData`): `0x0A1100E8` ← avgAir; `0xEA110130` ← avgWater; `0xEA110134` ← avgGarbage;
   `0xEA110137` ← radiationSum (raw); `0xEA110138` ← avgAir + radPct.
6. Zero `radiationSum, developedTracts, treatedSum, waterSum, waterSumTreated, airSum`.
7. Post the change message of each dirty flag (§5.2), then clear the four flags.
8. Recompute `efficiency` (§4.7).
9. Post `0xEA418D8F` with data1 = 8 (a neighbour-deal status report with no deal: asks the neighbours simulator to
   refresh its deal statistics, neighbors.md §11.1).

### 4.9 Monthly simulation (level 6, SimNewMonth `0x66956816`)

#### 4.9.1 Deals and cost

1. For every import deal (`0x4773FFE5`): `SetAmountUsed(0)`.
2. For every export deal (`0x4773FFE4`): `SetAmountUsed(0)`; `S += GetAmount()` (int64).
3. `cost := trunc((incinerated + wte + landfilled + S) × costPerUnit)` (int64; this month's counters).
   `department.GetLineItem(20).SetFullExpenses(cost)`; `lastMonthCost := cost`. *[quirk]* The export deals'
   **full amounts** are charged at the per-unit rate in addition to whatever the budget books for the deals.

#### 4.9.2 Counters and history

- Last month := this month for the seven counters and `decomposed`.
- History: `0xEA110158` produced, `0x8AB2F258` imported, `0x6AB2F260` exported, `0xEA11016F`
  trunc(recycled), `0xEA110197` incinerated, `0xEA1101A1` wte, `0xEA1101AA` landfilled (low 32 bits each).
- Zero the this-month counters, `decomposed` and `exportCapacity`.

#### 4.9.3 Building aging (`AgeOccupantList`) and usage

For the water-treatment (used `0x28ECF510`, capacity `0xC8EBD131`), incinerator (`0xE8ECF641`, `0xA8EBCF0C`) and
recycling (no properties) lists, for every occupant:

- `ratio := default`; with properties: `used` (u32) and `current := GetCurrentCapacity(capacityProp)`; if both
  are > 0: `ratio := used / current`. Recycling: if `recyclingCount > 0`: `ratio := recycled / recyclingCount` —
  but `recycled` was zeroed just before, so `ratio = 0` *[quirk]*.
- `condition` := float `0xC910B4CF` (default 0); `k` := `0xEA1CF220` (default 0); `t1 := 0xEA1CF221`,
  `t2 := 0xEA1CF222` (defaults 99).
- `condition += usageCurve(ratio) × fundingCurve(fundingFraction) × k` (curves `0x0911E2E9` and `0x0911E2E8`,
  §7.1); store it in `0xC910B4CF`.
- If `condition ≥ t2`: the lot at the occupant's cell gets `SetCondition(4)`; else if `condition ≥ t1`:
  `SetCondition(2)` (lot manager `GetLot(cellX, cellZ, false)`, compiled slot 6; lot compiled slot 36).

Then for water treatment, incinerators and waste-to-energy: `lastMonthUsed := used`; `used := 0`
(`0x28ECF510 → 0x08FCDB69`, `0xE8ECF641 → 0xA8FCDBA8`, `0x88ECF76C → 0x28FCDBB0`). Between the incinerator and
waste-to-energy resets, each waste-to-energy plant with properties `0x08FD1613`, `0x48EBD0B1`, `0x88ECF76C`
gets `0x27812852 := used × 0x08FD1613 / capacity` (unsigned integer arithmetic) *[inferred: its power output for
the power simulator]*.

**Usage distribution** (`SetOccupantListUsedCapacity`, daily §4.4.5) *[compat]*. Inputs: the list, its capacity
property `capProp` and used property `usedProp` (water treatment `0xC8EBD131`/`0x28ECF510`, incinerators
`0xA8EBCF0C`/`0xE8ECF641`, waste-to-energy `0x48EBD0B1`/`0x88ECF76C`), `total` := the day's unused total for that
list (§4.4.1), `used` := `total − remaining` (u32), and `n` := the list length (every node, duplicates and
occupants without the properties included). All arithmetic is unsigned 32-bit integer.

```
if total = 0 or n = 0 or used = 0: return
share := min(total, used) / n                    (integer division, remainder dropped)
k := n
for each occupant in list order, while used > 0 (checked before the first and after every occupant):
    k := k − 1                                   (also for occupants skipped below)
    if not GetCurrentCapacity(capProp, occ, cur): next      (slot 53)
    if the occupant has no property usedProp:      next
    already  := its u32 value of usedProp
    headroom := cur − already if cur > already, else 0
    if share ≤ headroom (signed 32-bit compare): take := share
    else: take := headroom; if k ≠ 0: share := used / k     (used BEFORE this take is subtracted)
    used := used − take                          (u32, may wrap)
    usedProp := already + take                   (written even when take = 0)
```

Consequences *[compat, quirk]*:

- **Remainders are dropped.** `used = 10` over three plants with room gives 3 + 3 + 3; 1 unit is never recorded.
  If `used < n` the share is 0 and nothing is recorded at all.
- **A capped occupant over-allocates the next ones.** The new share divides the `used` value from before the
  capped occupant's take, so the following occupants can be given more than is left. `used` then wraps to a huge
  unsigned value, which keeps the walk going to the end of the list. Example: `used = 100`, two plants with
  headroom 10 and 1000 → the first takes 10, `share := 100 / 1 = 100`, the second takes 100: 110 recorded.
- Occupants failing the capacity query or lacking `usedProp` take nothing but still count down `k`.
- Whatever is left in `used` is discarded by the caller.

#### 4.9.4 Maintenance cost (daily, §4.4.5)

For each occupant: `cost := CalcOccupantMaintenanceCost(occ)` (slot 65: base `0x2A58D350` × curve property
`0x0911E2E7` at `condition`, truncated). If it succeeds and the budget item for purpose `0xCA58EDC7` has a
different cost: `GetLineItemFromBudgetItem(item).AddToFullExpenses(cost − item.cost)` and
`ChangeBudgetItemCost(holder, item.index, cost)`.

#### 4.9.5 End of month

Refresh the visual effects (§4.10); `neighbours.MarkDealsDirty()`.

### 4.10 Visual effects and worst tracts (`UpdatePollutionVisualEffect`)

Runs monthly, on PostCityInit `0x26D31EC1`, when `effectsVisible` changes, and after `AddRadiationToCity`.

1. `effectIterator.Reset(shuffle)`: **one `rand()` call** (prime-step mode). Reset the three worst values to 0.
2. For every 8×8-cell block (bx, bz) in iterator order, with origin cell `(8bx, 8bz)`:
   - averages over the 64 cells (sum, then unsigned `>> 6`): air value, water value, radiation **source**, garbage
     amount (`& 0x7FFF`);
   - worst tracts: if an average is **strictly greater** than the stored worst value, store
     `(8bx + 4, 8bz + 4, average)` (cell coordinates of the block centre);
   - four effects, each driven by `s := strengthCurve(average × k)`, `k` = the `float32` constant 1/32767 (a multiply, rounded to
     `float32`, foundations §18; not a division) (curves `0x0A10C772` water, `0x0A10C773`
     air, `0x0A10C774` radiation, `0x0A10C775` garbage):

     | Effect | Shown when | Position (world) | Effect id, kind | Float parameter |
     |---|---|---|---|---|
     | water | `s > 0.05` | `(8bx·cw + U(−2cw, 2cw), 0, 8bz·cw + U(−2cw, 2cw))` | `0x0A78747A`, 1 | param 3 := s |
     | air | `s > 0.05` and `effectsVisible` | block centre, terrain altitude | `0x0A78747A`, 2 | param 2 := s |
     | radiation | same | block centre, altitude + 3.0 | `0x0A78747A`, 3 | param 2 := s |
     | garbage | same | block centre, terrain altitude | `0x0A78747A`, 4 | param 2 := s |

     `cw` is the cell width; "block centre" is `8bx·cw + 4cw` (same for z). If shown and the slot is empty: create
     the effect (`CreateVisualEffect(0x0A78747A, kind, &e)`), set its transform (identity with that
     translation), set the float parameter, `Start(0)`. If shown and present: only update the parameter. If not
     shown and present: `Stop(0)` if started, release, empty the slot.
3. Random draws: the water effect's position uses the pollution file's own `cRZRandomSuper` (foundations §6.2)
   with `RandomDoubleRangeUniform` — **2 draws per newly created water effect** (x then z), none otherwise.

**Calls and layouts** *[compat]* (PG4). The air path's calls are at `0x006E6836`–`0x006E693D`; the water path's
are the same.

| Object | Slot | Call | Pop | Return |
|---|---|---|---|---|
| `cISC4EffectsManager` (`0x00A9F264`, simulator `+0x38`) | 6 `+0x18` | `CreateVisualEffect(uint32 effect, uint32 kind, cISC4VisualEffect** out)` (`0x00593AE0`). It returns false and leaves `*out` alone for an unknown effect; otherwise it stores a new effect in `*out` with one reference. The original tests `*out`, not AL | 12 | AL |
| visual effect (vtable `0x00AA1170`) | 3 `+0x0C` | `Start(bool)`, always with 0 here | 4 | — |
| | 4 `+0x10` | `Stop(bool)`, always with 0 here | 4 | — |
| | 5 `+0x14` | `IsStarted()`: the byte at `+0x24` (`0x007D6680`, a folded getter) | 0 | AL |
| | 8 `+0x20` | `SetTransform(const cS3DTransform*)` (`0x005BE880`) | 4 | — |
| | 12 `+0x30` | `SetParameter(uint32 index, const float* values, uint32 count)`, here `(2 or 3, &s, 1)` | 12 | — |
| | 2 `+0x08` | `Release` | 0 | — |

`cS3DTransform` is 0x38 bytes. The copy it is read by (`0x005BEC20`) takes byte `+0x01` and the 13 floats at
`+0x04`–`+0x37`:

| Offset | Field | Set by the pollution code |
|---|---|---|
| `+0x00` | byte, a revision counter (the receiver keeps its own) | 0, then incremented to 1 |
| `+0x01` | byte, flags | bit 0 set *[meaning not traced]* |
| `+0x04`–`+0x27` | 3 × 3 rotation matrix, 9 floats, row by row | identity: 1.0 at `+0x04`, `+0x14`, `+0x24`; 0 elsewhere |
| `+0x28`–`+0x33` | translation x, y, z | the world position of the table above |
| `+0x34` | uniform scale | 1.0 |

**Not drawing the effects.** That is the implementer's choice, but two parts of this pass are not visual and
must still run:

- **The `rand()` call.** The effect iterator's reset makes one C `rand()` call (step 1). That stream is shared
  with every other main-thread `rand()` user, such as iterator shuffles (foundations §12.1, §12.3). Skipping the
  pass would shift every later draw.
- **The worst tracts.** Slots 17–19 return them (§8.1).

The two water-position draws are safe to drop. The `cRZRandomSuper` at `0x00B49BD0` is used only at `0x006E668C`
and `0x006E66DB` (and its static constructor `0x00A74E41`).

### 4.11 Prop requests (`cISC4PropRequester`)

`GetRequestedShare(rect)`: sum `GetGarbageValue` (own slot 9) over every cell of the inclusive cell rectangle,
divide (unsigned) by the cell count, multiply by 1/32767, evaluate curve `0x69A437F6` and return
`roundHalfEven` of it. The satisfaction id is `0xE9A4300B`; it uses no custom selector and `SelectProp` returns 0.
*[quirk]* Active landfill cells return a large negative value from `GetGarbageValue` (§8), which makes the
unsigned average enormous and the request maximal.

### 4.12 Rounding sites *[compat]*

Every rounding to the nearest integer in the pollution code uses the 1.5·2²³ idiom: `fadd dword 12582912.0f`,
store the sum as a `float32`, then subtract `0x4B400000` from its bit pattern as an integer (written
`add 0xB4C00000`). In crime.md §6 notation that is `rhe(x)`. It is exact (round half to even of the `float32`
`x`) for −2²² ≤ x ≤ 2²²; outside that range the result follows the formula there (for example
`rhe(5,000,000) = 4,597,152`), and a replacement must use the formula. The pollution code (`0x006E2CC0`–`0x006E8F09`,
`GetAverageAirValue` `0x007624C0`, `SelectProp` `0x0088B410`) contains **no** `fistp` / `fist` / `frndint`:
every other float → integer conversion is the CRT `_ftol` (truncation over the full int64 range, foundations
§18.1), written `trunc` in this spec.

| Site (spec) | Address | Operand of `rhe` (each step one `float32` operation) | Range in play |
|---|---|---|---|
| Import deal `n` (§4.4.2) | `0x006E82AE` | `float(amount) × 0.1f` | exact up to amount 41,943,040; deal amounts are far below |
| Export deal `n` (§4.4.2) | `0x006E8366` | same | same |
| Garbage produced (§4.4.4 step 4) | `0x006E877A` | `float(g) / float(count)` (added as `12582912f + gp`, same result) | \|g\| ≤ 32767: always exact |
| Water treated `t` (§4.4.4 step 6) | `0x006E8907` | `float(lastTreatedSum) ÷ lastWaterSum × W` (`fidiv`, `fimul`) | ≤ W ≤ 32767 (the treated sum never exceeds the water sum): exact |
| Valve share (§4.4.4 step 8) | `0x006E895F` | `float(g) / float(count)` | \|g\| ≤ 32767: exact |
| Landfill limit (§4.6.1 step 5) | `0x006E3AB7` | `transportCurve(float(tiles)) × efficiency`, then `− garbageSupply` (int32) | **leaves the range** above 838 tiles at efficiency 1.0 (shipped curve) |
| Landfill amount (§4.6.1 step 5) | `0x006E3B03` | `g` (only when `g ≤ n`) | exact while `n ≤ 2²²` |
| Prop share (§4.11) | `0x006E3C63` | `curve(float(avg) × (1/32767)f)` | the curve output, at most 10 shipped: exact |
| Current capacity (§8 slot 53) | `0x006E43C7` | `float(uint32 r) × curve(condition)` | exact for capacities × multiplier ≤ 2²² |

Two more sites use the same constant for a **floor**, in `Dispose` (§4.6.1 steps 3 and 4: waste-to-energy
`0x006E390F`, incinerator `0x006E39CA`): `n := rhe(f32(g − 0.5f))`, then `n := n + 1` if `float(n + 1) = g`
(`fild`, exact compare). This equals `floor(g)` (= `trunc(g)`, since `g > 0` there) up to 2²² and a little
beyond; for larger `g` it follows the `rhe` formula (`g` = 5,000,000 → 4,597,152). The step runs only when the
plant's remaining capacity exceeds `g`.

## 5. Agents and messages

### 5.1 Agents

| Level | Message | Handler | Returns |
|---|---|---|---|
| 4 Day | `0x66956814` | §4.4 | true (finished) |
| 6 Month | `0x66956816` | §4.9 | true |
| (0 SimBegin) | `0x66956810` | sets efficiency (§4.7) | **unreachable**: no level-0 agent is registered *[compat]* |

`DoMessage` returns true for every message. The day agent reads, in the same day: the ordinance simulator's
cumulative factors, the traffic simulator's air-polluting traffic map, zone types, neighbour deals, landfill
usage and tile count, and power status. No ordering requirement among these was found: each value is read as it
stands *[inferred]*. On the 1st of a month the day agent runs before the month agent (simulator.md §5).

### 5.2 Messages posted

All are posted (queued) standard messages with data1..4 = 0 unless stated.

| Type | When |
|---|---|
| `0xDA6DD35D` | radiation changed: end of cycle if `dirtyRadiation`; after `AddRadiationToCity` |
| `0xFA6DD0BB` | air changed (end of cycle, `dirtyAir`) |
| `0x5A6DD1AA` | water changed (end of cycle, `dirtyWater`) |
| `0xDA6DD2CB` | garbage changed (end of cycle, `dirtyGarbage`) |
| `0xEA418D8F` | end of cycle, data1 = 8 (§4.8) |

### 5.3 Messages handled

| Type | Action |
|---|---|
| `0x66956814`, `0x66956816` | §5.1 |
| `0x99EF1142` / `0x99EF1143` | occupant inserted / removed (§4.2) |
| `0x26D31EC1` PostCityInit | §4.10 |
| `0xC6CDBF1A` PreViewLevelChange (foundations §5.2) | `b := (new view level == 1)`: the u32 that void2 (standard-message compiled slot 15, `GetVoid2`) points at; level 1 is the surface view, 2 subway, 3 water pipes. If `b ≠ effectsVisible`: store it and run §4.10 |
| `0xCA1B151C` | re-read tuning (§7.1) |

## 6. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x8990C065`) *[compat]*

Primitives as in foundations §2 (little-endian). `ref` = `SC4WriteGZSerializable` / `SC4ReadGZSerializable`
(foundations §2.7). Grids are saved as their own objects (simgrid.md §5).

```
u16  version = 4
ref  airSource, waterSource, garbageSource, radiationSource          (grids)
ref  airValue, waterValue, garbageValue, radiationValue              (grids)
u32  cellX, cellZ
list waterTreatment, incinerators, wasteToEnergy, recycling          (each: u32 n, then n refs)
u32  recyclingCount, avgAir, avgWater
s32  airSum, waterSumTreated, waterSum, treatedSum, lastWaterSum, lastTreatedSum
u32  developedTracts
u16  avgGarbage
u32  incineratorCapacity, wteCapacity
f32  recyclingEffect
u8   garbageUncollectedFlag
u32  totalUncollected
s64  unused64, lastMonthCost
u32  decomposedLastMonth                                             (version ≥ 4 only)
s64  producedLast, importedLast, exportedLast
f64  recycledLast
s64  incineratedLast, wteLast, landfilledLast
s64  produced, imported, exported
f64  recycled
s64  incinerated, wte, landfilled
u32  decomposed
s32  exportCapacity
u32  activeLandfillCells
u32  tractsPerDay
iterator                                                             (below)
u8   dirtyRadiation, dirtyAir, dirtyWater, dirtyGarbage
f32  efficiency
s32  garbageSupply, garbageCapacity
ref  department
```

**Iterator** (as weather.md §10): `s8 state, s8 mode, u8 dirty, s32 x0, z0, x1, z1, s32 count, u32 n, n × u16
table, s32 index, s32 current, s32 step`. Pollution uses **mode 0 (table)**, so `n = tractCountX × tractCountZ`
and the table holds the shuffled packed `(tx << 8) | tz` entries (32 KB for a large city).

**Write** returns true.

**`Write` changes nothing (PG6).** `Write` (`0x006E3380`–`0x006E371A`) only reads the simulator. It stores to no
member, clears no dirty flag and allocates nothing. The iterator's writer (`0x005DAB70`) only reads the iterator.
Calling `Write` outside a save, for example to capture the state, is therefore safe for the simulator. Its other
effects all go through the stream it is given:

- **Refs.** Each `ref` goes through `SC4WriteGZSerializable` (`0x005D99A0`), which first asks the stream for
  `cISC4DBSegmentOStream` (`0x6999A8FB`).
  - **If the stream refuses,** the ref writes **nothing**, not even a null ID. The helper returns false, which
    `Write` ignores, and the referenced object is not touched.
  - **If the stream answers,** the helper asks the object for `cIGZSerializable` (`0xE4FDA3D4`) and passes it to
    the stream's `WriteGZSerializable` (slot 26, `+0x68`). It then releases both pointers, so the object's
    reference count ends where it started. On the game's save stream, that call also registers the object with
    the class-object store (foundations §2.7). A capture must therefore never use a save's stream.
- **Lists.** The list writer (`0x005DAFF0`) counts the nodes by walking the list, writes the count as `u32`, and
  then writes each entry as a ref, as above.

A capture through a stream that refuses `0x6999A8FB` therefore gets every field of the layout above in order,
except the 8 grid refs, the list entries (the four counts are still there) and the department.

**Read**:
- `version > 4` → fail. Versions 0–4 are accepted.
- Grid refs are read with IID `0xC9BD939A`, list refs with IID `0x26BB767E` (occupant), the department with IID
  `0x223E2B67` (after releasing any current department).
- Each list is cleared first; a count of 0 or ≥ 10,000,000 reads no refs.
- `version < 3`: an extra string (`GetGZStr`) follows `developedTracts` and is discarded; an extra `u32` follows the
  four dirty flags and is discarded.
- `version < 4`: `decomposedLastMonth` is absent (left unchanged).
- Sets `loaded`, returns true.

**Read after a stream error** (foundations §2.1, §2.9; the loader ignores the result, foundations §3.5). The
original never calls `GetError`: once the version is accepted it reads every field in order and returns **true**
and sets `loaded` even if the stream failed part-way (for example on a list ref to an occupant whose class cannot
be created). Fields read before the failing call hold the loaded values. After it every read fails and leaves its
destination unchanged, with these exceptions:

| Field | State after a failed read |
|---|---|
| each grid ref | **null** (the ref helper clears the destination before reading; the previous grid, if any, is neither released nor kept) |
| each occupant list | cleared first; each failed ref is still **appended as a null entry**; if the count itself fails, the count is an uninitialised local and that many null entries are appended (garbage) |
| `recyclingEffect`, `efficiency` | copied from an uninitialised (or stale) local (garbage) |
| iterator | the table length is left in the slot that held the stream pointer, so the table is resized to *(the stream object's address)* entries: a multi-hundred-MB allocation or an out-of-memory failure |
| `department` | the current department is **released and cleared** just before its ref is read, so after an earlier error it is null (Init then creates a new one) |

The result is unusable: `Init` sees `loaded`, skips the new-city setup, and the null grids crash the first day.
*[choice]* FreeCity4: on a stream error `Read` returns **false**, sets nothing (`loaded` stays clear), releases
any refs it acquired during the call and leaves every member, including the current department, as it was before
the call. The class store calls `Read` on a fresh object, so the following `Init` runs the new-city path. A
rejected version (> 4) consumes only the version and also leaves everything unchanged *[compat]*.

**Not saved** (recomputed or reset after load): `radiationSum` (restarts at 0 mid-cycle *[quirk]*), tuning and
`costPerUnit`, `effectsVisible` (back to true), effect objects and the effect iterator, worst tracts (refreshed by
PostCityInit), `initialized`.

**`radiationSum` across a load (PG7) *[quirk]*.** Reproduce this; it is the original's behaviour. Whether it was
intended cannot be read from the exe.
- `Write` skips the field: it writes `developedTracts` and then `avgGarbage`, with nothing between them.
- `Read` does not touch it.
- The constructor sets it to 0 (`0x006E206A`, from a zero global).
- `Init`'s new-city setup, which a loaded city skips, also zeroes it (`0x006E7288`).

After a load, the field therefore holds 0, while the iterator position, `developedTracts` and the other sums
continue from the save. The first end of cycle after the load uses only the radiation of the tracts visited since
the load. It zeroes the sum again (§4.8 step 6), so the divergence lasts **one** cycle: one entry in each of
`0xEA110137` and `0xEA110138`.

*Example.* The save falls mid-cycle with `radiationSum` = 30. The rest of the cycle adds 10, `developedTracts` is
50 at the end, and `avgAir` = 500.

| | `radiationSum` at end | `radPct` | `0xEA110137` | `0xEA110138` |
|---|---|---|---|---|
| No save and load | 40 | 40 × 100 / 50 = 80 | 40 | 580 |
| Saved and loaded | 10 | 10 × 100 / 50 = 20 | 10 | 520 |

The next cycle is the same in both runs.

## 7. Tuning

### 7.1 Exemplar

Type `0x6534284A`, group `0xE7E2C2DB`, instance **`0xC911E35B`** ("Utilities", confirmed). Before reading, every
value is reset to its code default and every curve emptied (`0x0911E2E8` and `0x0911E2E9` default to the constant
1: points (0, 1), (1, 1)). If the exemplar is missing the defaults stay.

| Property | Meaning | Type | Code default | Shipped |
|---|---|---|---|---|
| `0x69501944` | traffic air pollution factor | f32 | 8.0 | **0.25** |
| `0x49134BDD` | max air value for UI (slot 67) | u32 | 27000 | 1024 |
| `0x09134BE2` | max water value for UI (slot 68) | u32 | 32000 | 1024 |
| `0x49134BD4` | max garbage value for UI (slot 69) | u32 | 27000 | 32000 |
| `0x89135044` | max radiation value for UI (slot 70) | u32 | 32000 | 1024 |
| `0x6A406201` | `maxAirDelta` per visit | u32 | 50 | 150 |
| `0x8A40620B` | `maxWaterDelta` per visit | u32 | 50 | 150 |
| `0xA91351A6` | air threshold (slot 13) | u32 | 16000 | 512 |
| `0x091351AE` | water threshold (slot 14) | u32 | 16000 | 512 |
| `0x0913519B` | garbage threshold (slot 15) | u32 | 16000 | 1500 |
| `0x291351B5` | radiation threshold | u32 | 1 | 1 (read, never used *[inferred]*) |
| `0x09135479` | `decompositionAmount` per month | u32 | 5000 | 2000 |
| `0x09135482` | garbage UI scaling factor (slot 48) | f32 | 0.125 | 0.01 |
| `0xAA1F8AB7` | `costPerUnit` | f32 | (unchanged; 0 in a new object) | 0.005 |
| `0x4A652679` | `maxGarbageOnValve` | u32 | 655340 | 65536 |
| `0x4A65267A` | landfill tile capacity (slot 21) | u32 | 32767 | 30000 |
| `0x69A437F6` | prop request curve | curve | empty | (0,0) (0.0625,5) (0.125,10) |
| `0x0A10C772` | water effect strength | curve | empty | (0,0) (0.002,0.05) (0.005,0.15) (0.01,0.4) (0.02,1) |
| `0x0A10C773` | air effect strength | curve | empty | (0,0) (0.005,0.05) (0.01,0.25) (0.02,0.5) (0.03,0.6) |
| `0x0A10C774` | radiation effect strength | curve | empty | (0,0) (0.001,1) (0.01,1) |
| `0x0A10C775` | garbage effect strength | curve | empty | (0,0) (0.02,0.02) (0.03,0.06) (0.04,0.2) (0.05,0.4) (0.06,0.53) (0.085,0.75) (0.13,1) |
| `0x8C19C3E9` | garbage transport vs landfill tiles | curve | empty | (0,0) (65536, 3.2768e8): 5000 per tile |
| `0x0911E2E8` | funding fraction → decay multiplier | curve | (0,1)(1,1) | (0,1.1) (1,1) (1.2,0.9) |
| `0x0911E2E9` | usage ratio → decay multiplier | curve | (0,1)(1,1) | (0,0.1) (0.25,0.2) (0.5,0.45) (0.75,0.85) (0.9,1.25) (1,1.55) (1.1,2) |
| `0x6911E2DF` | funding % → efficiency % (read at each end of cycle, §4.7) | curve | — | (0,0) (50,25) (90,80) (100,100) (120,110) |

Curves are evaluated as in foundations §4.3 (clamped at both ends; empty → 0.0 by FreeCity4 choice).

### 7.2 Building properties read

| Property | Use |
|---|---|
| `0x029244DB` bool | preview occupant (foundations §5.2): skip all pollution handling |
| `0x27812851` s32[4], `0xAA5832F3` s32[4], `0x68EE9764` f32[4] | centre values, outer values, radii (tracts) |
| `0xC8EBD131`, `0xA8EBCF0C`, `0x48EBD0B1` u32 | base capacity: water treatment, incinerator, waste-to-energy |
| `0x88ED1059` | recycling percentage (list membership; capacity via slots 54/55) |
| `0x28ECF510`, `0xE8ECF641`, `0x88ECF76C` u32 | used this month (written by §4.9.3) |
| `0x08FCDB69`, `0xA8FCDBA8`, `0x28FCDBB0` u32 | used last month |
| `0x27812852` / `0x08FD1613` u32 | waste-to-energy output / max output *[inferred]* |
| `0x68ED1736` f32 | recycling centre: population served |
| `0xC910B4CF` f32 | condition (aging accumulator) |
| `0xEA1CF220` f32, `0xEA1CF221` f32, `0xEA1CF222` f32 | aging rate, threshold 1, threshold 2 |
| `0x8A1E07EE` curve | condition → capacity/efficiency multiplier |
| `0x0911E2E7` curve | condition → maintenance cost multiplier |
| `0x2A58D350` s64 | base maintenance cost (written by §4.2.4) |
| `0x48EE7A44` u32 | building garbage capacity (slot 47) |

Budget purpose `0xCA58EDC7`; line item 10 = building maintenance, 20 = garbage disposal, 30 = **landfill
maintenance**, created here but booked by the landfill developer (`GetLineItem(30)`, `SetFullExpenses` /
`GetCurrentExpenses`; landfill.md §4.9). *(Corrected: the earlier text guessed neighbour deal booking.)*

## 8. Interface slots

Calling convention: `thiscall`, callee pops its stack arguments. "Pop" is in bytes. Return channel: **AL/EAX**,
**ST0**, **EDX:EAX** or **none** (the register holds an unrelated value). "SDK" flags differences from gzcom-dll.

### 8.1 `cISC4PollutionSimulator` (original vtable `0x00AB2630`, 71 slots, SDK declaration order)

| Slot | Off | Method (as the game expects it) | Pop | Return | Behaviour | SDK |
|---|---|---|---|---|---|---|
| 0 | +0x0 | `QueryInterface(iid, void**)` | 8 | AL | §1 | |
| 1 | +0x4 | `AddRef()` | 0 | EAX | | |
| 2 | +0x8 | `Release()` | 0 | EAX | | |
| 3 | +0xC | `Init()` | 0 | AL true | §4.1 | |
| 4 | +0x10 | `Shutdown()` | 0 | AL true | §4.1 | |
| 5 | +0x14 | `cISC4SimGrid<int16>* GetPollutionGrid(type)` | 4 | EAX | 0 → air **value**, 1 → water value, 2 → garbage value, **3 → radiation source**; no `AddRef`. Other types read the member at `valueGrids[type]` (mod 2³²) *[quirk]*: 4 and 5 return `cellX` / `cellZ` as a pointer, 6–9 the four occupant lists' internal head pointers, higher values later members; **no type ≥ 4 ever yields a grid**. (Only the wrapped values `0xFFFFFFFC`–`0xFFFFFFFF` reach the air/water/garbage/radiation **source** grids.) No original caller passes a type outside 0..3 *[inferred: aura, MySim and map view use 0..3]*. *[choice]* Return null for every type other than 0..3. | semantics undocumented |
| 6 | +0x18 | `uint16 GetPollutionValue(type, cellX, cellZ)` | 12 | **AX only** | max(0, **source** grid `type` value). Upper 16 bits of EAX are garbage. Types 4–7 read the value grids *[quirk]*. | type OK; reads sources, not values |
| 7 | +0x1C | `GetAirValue(cellX, cellZ, int32& v)` | 12 | none | `v :=` air value (sign-extended) | **SDK `bool`**: EAX = v. Replacement returns true. |
| 8 | +0x20 | `GetWaterValue(...)` | 12 | none | water value | same |
| 9 | +0x24 | `GetGarbageValue(...)` | 12 | none | `v := signExtend16(raw) & 0xFFFF7FFF`: amount for normal cells, **`amount − 65536`** for active landfill cells *[quirk]* | same |
| 10 | +0x28 | `int32 GetAverageAirValue()` | 0 | EAX | `avgAir` | |
| 11 | +0x2C | `int32 GetAverageWaterValue()` | 0 | EAX | `avgWater` | |
| 12 | +0x30 | `int32 GetAverageGarbageValue()` | 0 | EAX | `avgGarbage` zero-extended | |
| 13 | +0x34 | `bool IsAirPolluted(cellX, cellZ)` | 8 | AL | slot 7, then **unsigned** `v ≥ airThreshold` (negative counts as polluted) *[quirk]* | |
| 14 | +0x38 | `bool IsWaterPolluted` | 8 | AL | slot 8, water threshold | |
| 15 | +0x3C | `bool IsGarbageAccumulated` | 8 | AL | slot 9, garbage threshold (landfill cells always true) | |
| 16 | +0x40 | `bool IsRadioactive(cellX, cellZ)` | 8 | AL | radiation **source** value ≠ 0 | |
| 17–19 | +0x44..+0x4C | `GetWorstAir/Water/GarbageTract(u32& x, u32& z, int32& v)` | 12 | none | copy §4.10 values (x, z are cell coordinates) | **SDK `bool`**: EAX = v. Return true. |
| 20 | +0x50 | `uint32 GetGarbageSupply()` | 0 | EAX | `max(0, garbageSupply)` (also stores the clamp) | |
| 21 | +0x54 | `int32 GetLandfillTileCapacity()` | 0 | EAX | tuning `0x4A65267A` | |
| 22 | +0x58 | `int32 GetGarbageCapacity()` | 0 | EAX | | |
| 23 | +0x5C | `bool IsGarbageUncollected()` | 0 | AL | flag of §4.8 | |
| 24 | +0x60 | `int32 GetTotalGarbageUncollected()` | 0 | EAX | `totalUncollected` | |
| 25 | +0x64 | `SetIsRadioactive(cellX, cellZ, bool)` | 12 | none | radiation **source** tract := 1 or 0 (overwrites cone values); `dirtyRadiation` | **SDK `bool`**: return true |
| 26 | +0x68 | `SetAirValue(cellX, cellZ, int16)` | 12 | none | air value tract := v; `dirtyAir` | same |
| 27 | +0x6C | `SetWaterValue(...)` | 12 | none | `dirtyWater` | same |
| 28 | +0x70 | `SetGarbageValue(cellX, cellZ, int16)` | 12 | none | `raw := (raw & 0x8000) | v` (keeps the landfill bit; v is not masked); `dirtyGarbage` | same |
| 29 | +0x74 | `AddToGarbageSupply(int32 n)` | 4 | none | §4.6.2; EAX = the applied delta | **SDK `bool`** |
| 30 | +0x78 | `SetGarbageCapacity(int32)` | 4 | none | EAX = argument | **SDK `bool`** |
| 31 | +0x7C | `int32 GetMonthlyIncineratorCapacity()` | 0 | EAX | `incineratorCapacity` | |
| 32 | +0x80 | `int32 GetMonthlyWasteToEnergyCapacity()` | 0 | EAX | `wteCapacity` | |
| 33 | +0x84 | `SC4Percentage* GetRecyclingCenterEffect()` | 0 | EAX **pointer** | address of `recyclingEffect` (float) | **SDK ERROR: `int32`** |
| 34 | +0x88 | `int64 GetGarbageAbductedPrevMonth()` | 0 | EDX:EAX | `decomposedLastMonth` zero-extended | **SDK ERROR: `int32`** |
| 35 | +0x8C | `int64 GetGarbageProducedLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 36 | +0x90 | `int64 GetGarbageImportedLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 37 | +0x94 | `int64 GetGarbageExportedLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 38 | +0x98 | `int64 GetGarbageRecycledLastMonth()` | 0 | EDX:EAX | trunc(recycledLast) | **SDK ERROR** |
| 39 | +0x9C | `int64 GetGarbageIncineratedLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 40 | +0xA0 | `int64 GetGarbageConvertedToEnergyLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 41 | +0xA4 | `int64 GetGarbageLandfilledLastMonth()` | 0 | EDX:EAX | | **SDK ERROR** |
| 42–45 | +0xA8..+0xB4 | `uint32 GetIncineratorCount / GetWasteToEnergyBuildingCount / GetRecyclingCenterCount / GetWaterTreatmentPlantCount()` | 0 | EAX | list lengths (duplicates count) | |
| 46 | +0xB8 | `GetGarbageBuildings(SC4List<cISC4Occupant*>&)` | 4 | none | appends (AddRef) incinerators, then waste-to-energy, then recycling (not water treatment) | **SDK `bool`**: return true |
| 47 | +0xBC | `uint32 GetBuildingGarbageCapacity(occ)` | 4 | EAX | u32 `0x48EE7A44`, else 0 | |
| 48 | +0xC0 | `float GetGarbageScalingFactor()` | 0 | ST0 | tuning `0x09135482` | |
| 49 | +0xC4 | `bool IsActiveLandfill(cellX, cellZ)` | 8 | AL | bit 15 of the garbage cell | |
| 50 | +0xC8 | `SetIsActiveLandfill(cellX, cellZ, bool)` | 12 | none | only if the cell's zone type is 14 **and** the flag differs: set/clear bit 15, `activeLandfillCells ± 1` (decrement only if > 0), `dirtyGarbage` | **SDK `bool`** |
| 51 | +0xCC | `GarbagePutInLandfill(int32 n)` | 4 | none | `landfilled += n` (int64; the overflow guard is unreachable in practice). EDX:EAX = (sign of n, new high dword) | **SDK `int64`: value meaningless**. Return the new total. PG2: the registers at `ret 4` (`0x006E32CE`) are EAX = the new **high** dword of `landfilled` (the `adc` at `0x006E32C4`) and EDX = the sign extension of n (`cdq` at `0x006E32B4`). That is neither the total nor void. Its known caller, the landfill month agent (landfill.md §4.7.2), ignores it. Declare it int64 in EDX:EAX (SDK) with pop 4. The value is free; FreeCity4 returns the new total. |
| 52 | +0xD0 | `bool GetMaxCapacity(prop, occ, u32& r)` | 12 | AL | `r := 0`, then u32 property `prop`; returns found | |
| 53 | +0xD4 | `bool GetCurrentCapacity(prop, occ, u32& r)` | 12 | AL | missing → `r := 0`, false. Else `r := base`; 0 if not powered (`IsOccupantPowered`); `r := trunc(r × efficiency)`; if curve property `0x8A1E07EE` exists, `r := roundHalfEven(r × curve(condition))`; true | |
| 54 | +0xD8 | `bool GetRecyclingCenterMaxCapacity(occ, SC4Percentage&)` | 8 | AL | `p := default` (`0x00B49BC4`), then property `0x88ED1059` through the shared percentage getter `0x005FDAC0` (value × **0.01f**, one float32 rounding, as in residential.md §7.1 RS1); returns found | **AL only, not a float** (PG3, `0x006E2D70`–`0x006E2D94`): AL is the getter's result, and each arm of the getter stores the float through the reference (`fstp`) and returns its bool in AL, so the x87 stack is empty. `docs/abi`'s "x87\|eax" is a heuristic. |
| 55 | +0xDC | `bool GetRecyclingCenterCurrentCapacity(occ, SC4Percentage&)` | 8 | AL | slot 53 with `0x88ED1059`: success → `p := f32(float(uint32 r) × 0.01f)` (§4.7: `0.01f` is a `float32` literal, so r = 30 → 0.29999998 `0x3E999999`, not 0.3f), true; else `p := default`, false | **AL only** (PG1): AL is slot 53's result, left untouched on both paths (`0x006E3737`–`0x006E3768`). The float goes through the reference (`fstp` `0x006E3757`), so the x87 stack is **empty** on return. `docs/abi`'s "x87" is a heuristic misreading of the `fild`/`fmul` (the `fadd` `0x4F800000` = 2³² makes r unsigned). |
| 56 | +0xE0 | `bool GetCurrentOccupantCondition(occ, SC4Percentage&)` | 8 | AL | `p := 1.0`; with curve `0x8A1E07EE`: `p := curve(condition) × 1.0`, true; else false | |
| 57 | +0xE4 | `bool GetCurrentOccupantEfficiency(occ, SC4Percentage&)` | 8 | AL | powered and slot 56 succeeds → `p := efficiency × condition`, true; else `p := default`, false | |
| 58 | +0xE8 | `bool GetIncineratorUsedCapacity(occ, u32&)` | 8 | AL | `r := 0`, then `0xE8ECF641` | |
| 59 | +0xEC | `bool GetWasteToEnergyUsedCapacity(occ, u32&)` | 8 | AL | `0x88ECF76C` | |
| 60 | +0xF0 | `bool GetIncineratorCurrentCapacity(occ, u32&)` | 8 | AL | `r := 0`; slot 53 with `0xA8EBCF0C` | |
| 61 | +0xF4 | `bool GetWasteToEnergyCurrentCapacity(occ, u32&)` | 8 | AL | slot 53 with `0x48EBD0B1` | |
| 62 | +0xF8 | `bool GetWaterTreatmentMaxCapacity(occ, u32&)` | 8 | AL | slot 52 with `0xC8EBD131` | |
| 63 | +0xFC | `bool GetWaterTreatmentCurrentCapacity(occ, u32&)` | 8 | AL | slot 53 with `0xC8EBD131` | |
| 64 | +0x100 | `bool GetWaterTreatmentUsedCapacity(occ, u32&)` | 8 | AL | slot 52 with `0x28ECF510` | |
| 65 | +0x104 | `bool CalcOccupantMaintenanceCost(occ, int64& c)` | 8 | AL | `c := 0`; base `0x2A58D350` (missing → false); needs `0xC910B4CF` and curve `0x0911E2E7` (missing → false, `c` = base); `c := trunc(base × curve(condition))`, true | SDK `uint64&`: signed in practice |
| 66 | +0x108 | `void AddRadiationToCity(cS3DVector3 const& pos, float strength, float radius)` | 12 | none | §4.3 | **SDK names wrong**: 2nd = centre strength, 3rd = radius in tracts (not radiusX/radiusZ) |
| 67–70 | +0x10C..+0x118 | `int32 GetMax{Air,Water,Garbage,Radiation}PollutionLevelForUI()` | 0 | EAX | tuning (§7.1) | |

Slots 31 and 32 share their bodies with unrelated getters in the original (identical code folding); this has no
behavioural effect.

### 8.2 `cIGZSerializable` (vtable `0x00AB25E8`)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0–2 | `QueryInterface`, `AddRef`, `Release` (forward to the object) | 8 / 0 / 0 | |
| 3 | `bool Write(cIGZOStream&)` | 4 | AL true |
| 4 | `bool Read(cIGZIStream&)` | 4 | AL |
| 5 | `uint32 GetGZCLSID()` | 0 | EAX `0x8990C065` |

### 8.3 `cIGZMessageTarget2` (vtable `0x00AB261C`)

Slots 0–2 forward; slot 3 `bool DoMessage(cIGZMessage2*)` (pop 4) → always true (§5.3).

### 8.4 `cISC4PropRequester` (IID `0x89C498BE`, vtable `0x00AB2600`)

No SDK header exists for this interface.

| Slot | Off | Method | Pop | Return | Behaviour |
|---|---|---|---|---|---|
| 0–2 | | `QueryInterface`, `AddRef`, `Release` (forward) | 8/0/0 | | |
| 3 | +0xC | `int32 GetRequestedShare(SC4Rect<int32> const& cells)` | 4 | EAX | §4.11 (rect = x0, z0, x1, z1, inclusive) |
| 4 | +0x10 | `uint32 GetRequesterSatisfactionId()` | 0 | EAX | `0xE9A4300B` |
| 5 | +0x14 | `bool UsesCustomPropSelector()` | 0 | AL | false |
| 6 | +0x18 | `uint32 SelectProp(vector<uint32> const&, int32, int32)` | 12 | EAX | 0 |

## 9. Dependencies

Calls the original makes on other subsystems (compiled slots; pop in bytes):

| Interface (vtable) | Slot | Method | Pop | Return |
|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 66 / 91 / 92 / 93 / 94 / 95 | GetPollutionSimulator / CellWidthX / CellWidthZ / CellCountX / CellCountZ / PositionToCell | 0/0/0/0/0/16 | EAX/ST0/ST0/EAX/EAX/none |
| `cISC4City` | 33, 62, 84, other getters | GetLotManager, GetNeighborsSimulator, GetTerrain, … | 0 | EAX |
| `cISC4Simulator` (`0x00AB2FB8`) | 19 / 20 | AddAgent(t, level, name, flags) / RemoveAgent(t) | 16 / 4 | AL |
| `cISC4ZoneManager` (`0x00A92E00`) | 12 | GetZoneType(x, z) | 8 | EAX |
| `cISC4TrafficSimulator` (`0x00AB33D0`) | 8 | GetAirPollutingTrafficMap (uint8 grid) | 0 | EAX |
| `cISC4OrdinanceSimulator` (`0x00ABEAC0`) | 25 | GetCumulativeFactor(id, index) | 8 | ST0 |
| `cISC4NeighborsSimulator` (`0x00AB1EE0`) | 30 / 31 | FindDeals(type, list&) / MarkDealsDirty | 8 / 0 | none |
| `cISC4NeighborDeal` (`0x00AB1DD8`) | 10 / 20 / 21 | GetAmount / GetAmountUsed / SetAmountUsed | 0 / 0 / 4 | EAX / EAX / none |
| `cISC4PowerSimulator` (`0x00AB28B8`) | 31 | IsOccupantPowered(occ) | 4 | AL |
| `cISC4ResidentialSimulator` (`0x00AB2C10`) | 16 | GetPopulation | 0 | EAX |
| `cISC4LandfillDeveloper` (`0x00AB157C`) | 10 / 11 | GetLandfillUsage(u32&, u32&) / GetLandfillTileCount | 8 / 0 | none / EAX |
| `cISC4PropDeveloper` (`0x00AB29F8`) | 5 / 6 | AddPropRequester / RemovePropRequester | 4 / 4 | none |
| `cISC4EffectsManager` (`0x00A9F264`) | 6 | CreateVisualEffect(id, kind, cISC4VisualEffect**) | 12 | AL |
| `cISC4VisualEffect` | 3 / 4 / 5 / 8 / 12 | Start(t) / Stop(t) / IsStarted / SetSourceTransform / SetFloatParams(type, float*, n) | 4/4/0/4/12 | none/none/AL/none/AL |
| `cISTETerrain` (`0x00AB3D18`) | 13 | altitude at (x, z) (SDK: misnamed `SetAltitudeAtVertex`) | 8 | ST0 |
| `cISC4LotManager` (`0x00AB37C0`) | 6 | GetLot(cellX, cellZ, bool) | 12 | EAX |
| `cISC4Lot` (`0x00AB19B0`) | 36 | SetCondition(u32) | 4 | AL |
| `cISC4Occupant` (`0x00AA41D8`) | 6 / 8 / 10 | AsPropertyHolder / GetPosition(vec3&) / GetBoundingBox(box&) | 0 / 4 / 8 | EAX |
| `cISCPropertyHolder` (`0x00AA6CD8`) | 9 / 13 | GetProperty(id) / AddProperty(id, value, …) | 4 / 12 | EAX |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 48 | release an occupant list | 4 | none |
| `cISC4BudgetSimulator` (`0x00AB0908`) | 25 / 51 / 53 / 55 / 56 / 57 | CreateDepartmentBudget / GetBudgetItemForPurpose / ChangeBudgetItemCost / AddBudgetItemToBudget / RemoveBudgetItemFromBudget / GetLineItemFromBudgetItem | 8/12/16/8/8/4 | EAX/AL/AL/AL/AL/EAX |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 5 / 10 / 13 / 17 / 20 | SetDepartmentName / GetFundingPercentage (**pointer**) / SetMaxAllowedFundingPercentage / CreateLineItem / GetLineItem | 8/0/4/8/4 | EAX |
| `cISC4LineItem` (`0x00AB0F58`) | 7 / 12 / 13 | SetName / SetFullExpenses(int64) / AddToFullExpenses(int64) | 8/8/8 | AL |
| `cISC4HistoryWarehouse` (`0x00A8CCB4`) | 12 / 13 | RecordHistoryData(type, int32) / SetHistoryDataQueueLength | 8 / 8 | AL |
| `cISC4SimGrid<int16>` (simgrid.md §4) | 3, 4, 6, 7, 9, 12, 13, 19, 24, 26 | Init, Shutdown, SetInstanceID, GetCellValue, SetTractSize, GetTractCountX/Z, PositionToTract, GetGridData, SetTractValue | | |

Helpers (simgrid.md §7): `AddRadiusEffect` int16 variant; the random cell iterator (table mode for the tract
pass, prime-step for effect blocks); the property helpers; the standard-message post helper.

## 10. Replaceability, quirks and open questions

### 10.1 External callers and replaceability

Original code outside the class reaches the simulator through `cISC4City::GetPollutionSimulator()` and the cached
global service pointer, always **through the interface**. Observed callers (by slot):

| Slot | Callers | Use of the result |
|---|---|---|
| 5 `GetPollutionGrid` | aura simulator (each type with a non-zero aura weight), growth developer (desirability, idle), map view (four layers), query tooltips, desirability equation, MySim | aura and growth developer pass the grid to `ProcessTwoGridsWithCallback` (simgrid.md §7.3) and similar helpers, which read it through **`GetGridData`** (raw `vector<vector<int16>>`, simgrid.md §6.3); the map view reads `GetTractWidthX/Z` and **`GetGridData` (const)**; tooltips and MySim call `GetCellValue` (some treat the result as unsigned) |
| 6 `GetPollutionValue` | residential: every tract in the monthly population pass (air, water, garbage; residential.md, `DoPopulationChange`) and the hospital query data | value |
| 7 `GetAirValue` | cell query | value |
| 21 | landfill zones (heap capacity) | |
| 20, 24, 29, 30, 50, 51 | landfill developer / landfill zones | supply, uncollected total, capacity, landfill flags, landfilled counter |
| 33, 35, 38, 41, 44, 48, 53–60 | query tool, performance meters, building descriptions | statistics and capacities (int64 and pointer returns, §8.1) |
| 38 | neighbours simulator (commodity statistics) | |
| 46 | map view (highlights) | |
| 66 | fire disaster (meltdown), power simulator (plant aging) | |

Findings:

1. No original code constructs this class, calls its non-virtual functions or reads its members directly. It is
   **replaceable through the GZCOM class registry** (CLSID `0x8990C065`).
2. The grids returned by slot 5 are read as **raw memory** by the aura simulator, growth developer and map view.
   A replacement must return genuine game `cSC4SimGrid<int16>` objects (create them through GZCOM with CLSID
   `0x49B9E605`, simgrid.md §6.3 recommendation) with the original tract sizes (2, garbage value 1) and must
   keep the radiation slot returning the **source** grid.
3. The garbage value grid's bit 15 is part of the observable data (landfill flag) and must be kept.
4. Slots 33–41 must be compiled with the real return types (§8.1); an SDK-compiled replacement would leave EDX or
   the pointer wrong for the query tool.
5. Grids, lists and the department are saved as refs: a replacement that keeps the record of §6 loads original
   saves unchanged.

### 10.2 Open questions

1. *(resolved, PG5)* **Prop requester after load**: `AddPropRequester` is only called for a new city. A loaded
   city keeps its garbage props because the prop developer saves its requesters itself:
   - **`Write`** (`0x006F1970`–`0x006F19B6`) writes the requester count, then each requester by ref. The
     requesters are the vector at prop developer `+0x9FC`.
   - **`Read`** (`0x006F3680`–`0x006F36C8`) resizes that vector and reads each ref with IID `0x89C498BE`.
   - So after a load the vector again points at the loaded pollution simulator's requester sub-object.
   - **Do not add it again.** `AddPropRequester` (`0x006F21C0`) appends with no duplicate check, so a second call
     would register the requester twice.
   - **For a replacement.** It must answer IID `0x89C498BE` and be saved by ref under its class ID, so that this
     ref resolves to it.
2. *(resolved)* `0xC6CDBF1A` is PreViewLevelChange, sent synchronously by the 3D view; effects are visible only in
   the surface view (foundations §5.2).
3. The water effect is positioned at height 0 at a jittered block **corner**; whether that is intentional (sea
   level / water surface) is unknown *[inferred]*.
4. *(resolved)* `SetOccupantListUsedCapacity` is specified exactly in §4.9.3 (remainders dropped, the share after
   a capped building recomputed from the pre-take `used`).
5. `unused64` (saved int64) is never written in the Windows build; its meaning is unknown.
6. *(resolved)* Line item 30 is the landfill maintenance line, booked by the landfill developer (landfill.md §4.9).
7. Callers that load the service pointer and call it later (advisor utility, MySim trip variables, street sim,
   ordinance "trash presort") were not followed to the slot they call.
8. Property names in §7.2 are inferred from use; no Maxis names were available for them.
9. *(resolved)* Every round-half-even site in the pollution code (and the two `Dispose` floors) uses the
   1.5·2²³ idiom and shares its range limit; there is no `fistp`/`frndint` rounding. Only the landfill limit
   leaves the exact range in normal play (§4.12).

### 10.3 Corrections to grid-sims.md §3

- `0xA8EBCF0C` is the **incinerator** capacity and `0xC8EBD131` the **water treatment** capacity (they were
  swapped).
- `GetPollutionValue` reads the **source** grids; `GetPollutionGrid(3)` returns the radiation **source** grid.
- The SimBegin handler is unreachable; the tract iterator uses **table** mode; the cadence is tracts/15 per day.
- The return types of §8.1 (pointer, int64, garbage bools).

### 10.4 Quirks reproduced (summary)

Wrapping int16 arithmetic in the garbage rectangle and zone averaging; garbage cone applied in addition to the
rectangle; uncapped import deal usage; `Dispose` emptying a waste-to-energy or incinerator remainder that is
smaller than the offer; garbage flowing back from the supply when the valve is closed; the water "stop at 1"
special case with its own-tract read and cell-vs-tract bounds; averages divided by the developed count only;
efficiency 0 for the first cycle; recycling aging ratio always 0; export deal amounts charged per unit; landfill
cells' negative `GetGarbageValue`; unsigned threshold tests; radiation sum not saved; usage distribution that
drops remainders and over-allocates after a capped building (§4.9.3); `float32` constants 0.1f and 0.01f
(§4.4.2, §4.7); the 1.5·2²³ rounding of the landfill limit beyond 2²² (§4.12).

## 11. Conformance tests (must exist in `engine/conformance`)

Unless stated: shipped tuning, all ordinance factors and zone multipliers 1.0, no traffic, efficiency 1.0.

1. **Cone**: a building with centre `[100, 0, 0, 0]`, outer 0, radius 2.5 at the centre of tract (10, 10) of a
   64×64-tract grid → air source: (10,10) = 100; the four at distance 1 = 60; the four diagonals (1.414) = 43;
   the four at distance 2 = 20; the eight at distance 2.236 = 11; (12,12) (2.828) and all others = 0; 21 tracts
   changed. Removing it restores all zeros.
2. **Default radii**: centre `[10, 10, 10, 10]`, outer 0, without `0x68EE9764` → radii 2.5, 1.5, 0.5, 4.0.
   Water cone (r = 1.5): the centre (10), the 4 tracts at d = 1 (10 − 6.667 = 3.33 → 3) and the 4 diagonals
   (d = 1.414: 10 − 9.43 = 0.57 → 1), 9 tracts. Garbage cone (r = 0.5): only the centre tract (+10, on top of the
   rectangle of §4.2.2). Radiation cone (r = 4): every tract with d < 4, 45 tracts.
3. **Garbage rectangle**: a 2×2-cell footprint aligned on tract (5,5) with garbage centre 30 and radius 0 →
   garbage source (5,5) = 4 × 30 = 120; with a footprint covering cells (11..12, 10..10) → tracts (5,5) += 30 and
   (6,5) += 30.
4. **Skip**: the same building with `0x029244DB = true` → no grid, list or budget change.
5. **Relaxation**: air source 1000 on a developed tract, value 0 → successive visits give 150, 300, 450, 600,
   750, 900, 1000. With code-default delta 50 → 50, 100, …
6. **Water stop at 1**: 64 × 64 tracts, water value 100 at tract (1,1) (cell (2,2)), neighbours 0, water target 0
   → visits give 1 (own-tract read), then 0. The same at tract (40,40) → 0 directly.
7. **Traffic**: air target 0, the four cells' traffic map values 40, fTraffic 1, factor 0.25 → A = 40 (10 per
   cell). With code default factor 8.0 → 1280.
8. **Zone averaging**: air source 100, cells' zone multipliers 1.0, 1.0, 0.5 and landfill → count 3, A =
   trunc(100 × 2.5 / 3) = 83.
9. **Garbage production**: g = 400, four residential cells, no capacities, no deals, decomposition room 2000 →
   produced += 4 × 100; each cell's 100 is decomposed; cells stay 0; decomposed = 400. After 20 such cells,
   `decomposed = 2000` and the next cells keep 100 each (`totalUncollected += 100`).
10. **Dispose WTE quirk**: wteRemaining 50, offer 100 → wteRemaining 0, nothing burned; wteRemaining 150, offer
    100 → 100 burned, remaining 50.
11. **Valve**: supply 700,000, capacity 0, no disposal → after the day supply = 65,536 (shipped
    `maxGarbageOnValve`).
12. **Valve closed in a cell**: supply 1100, capacity 1000, g = 400, count 4, a cell with left = 50 → share 100,
    over 100 → take 100; supply 1000; cell amount 150.
13. **Water treatment, first cycle**: wtpRemaining 30, W = 50 (no history) → t = 30, W = 20. With history
    `lastTreated 300, lastWater 1000`, remaining 200, W = 50 → t = roundHalfEven(15.0) = 15.
14. **Efficiency**: funding 100 % → 1.0; 50 % → 0.25; 90 % → curve 80 → **0.79999995** (`0x3F4CCCCC`), and a
    powered incinerator with base 1000 and no condition curve then reports `GetIncineratorCurrentCapacity` =
    **799**; a new city before its first end of cycle → 0.0 and `GetIncineratorCurrentCapacity` = 0.
    `GetRecyclingCenterCurrentCapacity` with a current capacity of 30 → 0.29999998 (`0x3E999999`); 25 → 0.25.
15. **Current capacity**: base 1000, powered, efficiency 0.8, condition curve (0,1)(100,0.5), condition 50 →
    trunc(800) then roundHalfEven(800 × 0.75) = 600.
16. **Monthly cost**: incinerated 1000, wte 0, landfilled 3000, one export deal of amount 1000, costPerUnit 0.005
    → line item 20 full expenses = 25.
17. **Cadence**: 128 × 128 tracts → `tractsPerDay` = 1092; the 16384-entry table is reset with 16383 `rand()`
    calls; the end of cycle happens during day 16.
18. **End of cycle**: airSum 50,000 over a cycle with 100 developed tracts → avgAir 500, history `0x0A1100E8` =
    500; radiationSum 2 → radPct 2, `0xEA110138` = 502.
19. **Slots**: `GetPollutionGrid(3)` is the radiation source grid; `GetPollutionValue(0, x, z)` with a source
    value −5 → 0; `GetGarbageValue` on a landfill cell with raw `0x8064` → −65,436; `IsAirPolluted` with air
    value −1 → true; `GetRecyclingCenterEffect` returns a pointer to 0.25 after `recyclingEffect = 0.25`;
    `GetGarbageProducedLastMonth` returns 5,000,000,000 in EDX:EAX. FreeCity4 `GetPollutionGrid(4)` and
    `GetPollutionGrid(0xFFFFFFFF)` return null *[choice]*.
20. **Prop share**: a 4 × 4-cell rect with garbage 2048 everywhere → 2048 × k = 0.0625019 → curve 5.0001526 → 5.
21. **Effects**: a block with average water 655 → 655 × k = 0.019989625 → s = **0.9993775** (three `float32`
    operations, foundations §4.3; 0.99939 was the unrounded value) → a water effect is created (2
    `cRZRandomSuper` draws); average 33 → s = 0.0251778 → no effect. Air effects are removed when `0xC6CDBF1A`
    reports a new view level other than 1 (for example 3, the water-pipe view), water effects are kept.
22. **Persistence**: write → read round trip is byte-identical; a version-3 record (no `decomposedLastMonth`) loads;
    a version-5 record is rejected; a version-2 record skips the extra string and the extra `u32`.
23. **Deal rounding**: import deals with amount 25, 35, 15 and 1000 (amountUsed 0) → n = 2, 4, 2, 100
    (`importTotal` 108, `imported` += 108, each deal's `amountUsed` = n). An export deal of amount 25 with
    amountUsed 24 → n = min(2, 1) = 1.
24. **Usage distribution** (efficiency 1.0, all powered, no condition curves, `0x28ECF510` = 0 unless stated):
    - three water-treatment plants of capacity 1000, `total` 3000, `used` 10 → `0x28ECF510` = 3, 3, 3;
    - five plants, `used` 3 → all stay 0 (share 0), each property still written;
    - two plants of capacity 10 and 1000, `total` 1010, `used` 100 → 10 and **100** (110 recorded);
    - two plants of capacity 1000, the first with `0x28ECF510` = 1000 already, `used` 200 → share 100, the first
      takes 0 and the share becomes 200 / 1 = 200, the second takes 200 → 1000 and 200.
25. **Read with a stream error** (FreeCity4 *[choice]*): a version-4 record truncated inside the incinerator list
    → `Read` returns false, no grid, list or department is replaced, `loaded` stays clear, and the next `Init`
    creates new grids and a new department as for a new city.
26. **Landfill limit rounding** (§4.6.1 step 5, §4.12): efficiency 1.0, shipped transport curve, garbage supply 0,
    landfill room 10,000,000, offer `g` = 6,000,000 (no export, plants or decomposition room) → 1000 tiles:
    curve 5,000,000 → `limit` = **4,597,152**, `AddToGarbageSupply(4597152)`, `g` left 1,402,848; 839 tiles →
    limit 4,194,652; 838 tiles → 4,190,000 (exact). With supply 100,000 and 1000 tiles → limit 4,497,152.
27. **Deal rounding out of range**: an import deal of amount 50,000,000 (amountUsed 0) → `float32` product
    5,000,000 → n = **4,597,152**; amount 41,943,040 → 4,194,304 (exact).
28. **Dispose floor** (§4.12): wteRemaining 10, offer 9.99 → wteRemaining 1 (floor 9), the full 9.99 burned;
    wteRemaining 10, offer 3.0 → 7; incinerator remaining 8,000,000, offer 5,000,000 → remaining
    8,000,000 − 4,597,152 = 3,402,848.
