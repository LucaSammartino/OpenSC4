# Plumbing (water) simulator (`cSC4PlumbingSimulator`) — behaviour spec

Status: round 2. Written from the Windows 1.1.641 executable. `docs/subsystems/grid-sims.md` (partly from the Mac
build) was only a starting point. Every fact below was checked against the Windows code. Where the Mac
pseudocode differs, the Windows behaviour is the one specified (for example, the Mac build clears the pipe flags
on every `Init`, but Windows clears them only when the grids are created, §4.1).

Tags:

- *[compat]*: observable behaviour that must match.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced, or that a replacement must survive.

Shared material is cited, not repeated here:

- foundations: streams and refs (§2), the class store (§3), the city component table (§3.6), Init order (§3.7),
  response curves (§4), messages (§5), RNGs (§6), C `rand` and `random_shuffle` (§12), month order (§13), the
  slot-order rule (§16), framework slot tables (§17);
- the scheduler (simulator spec);
- grid classes, geometry, slots, persistence, lifetime and iterators (`docs/specs/simgrid.md` §2–§7);
- water pollution and water treatment (`docs/specs/pollution.md` §4.4.4, §4.9.3);
- neighbour deals and connections (`docs/specs/neighbors.md`);
- budget departments, line items and budget items (`docs/specs/budget.md`).

---

## 1. Role and identity

The plumbing simulator models the city's water supply. It does the following:

- It keeps a **per-cell flag map** of pipes, their connections, consuming buildings and water producers.
- It computes each **water producer's** monthly production from these inputs: funding, distance to water, water
  pollution, power and condition.
- In each **pass** it floods every pipe network from its producers or from an import connection. It then grows a
  **watered area** of `radius` rings around each network, until the network's supply is used up by the
  buildings it reaches.
- It books **imports and exports** against neighbour deals.
- It publishes a **watered grid** (`uint8`, one value per city cell) that the map views, the flammability
  simulator, query tools and others read.
- Every month it ages pipes from department funding: pipes become **distressed** and can **burst** (the burst
  itself is the pipe-burst disaster). It also ages water producers.

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`, `0x006DA3B0`) | `0x0990C075` |
| Primary interface | `cISC4PlumbingSimulator`, IID `0x47782CCC`. QI also answers `0x00000001` (cIGZUnknown) with it |
| `cIGZMessageTarget2` | IID `0x452294AA` (sub-object at +4) |
| `cIGZSerializable` | IID `0xE4FDA3D4` (sub-object at +8) |
| Not answered | everything else: false, out pointer untouched |
| City component | row **#32** (foundations §3.6). The city creates it with `GetClassObject(0x0990C075, 0x47782CCC)` |
| City getter | `cISC4City` slot 64 `GetPlumbingSimulator` (pop 0, EAX). The game caches it in a global at city init |
| Init order | foundations §3.7: … budget, ordinance, power, **plumbing**, neighbours, building development … |

All three interfaces are one object with one reference count. `QueryInterface` AddRefs.

- `Release` with a count above 1 decrements and returns the new count.
- Otherwise it calls `Shutdown()`, destroys the object and returns 0 *[compat]*.

The scalar-deleting destructor is vtable slot 29 (pop 4). It is not part of the interface.

## 2. Grids, cells and the flag map

All coordinates are **city cells**: `0 ≤ x < countX = city.CellCountX()` (city slot 93) and
`0 ≤ z < countZ = city.CellCountZ()` (city slot 94).

**Two `uint8` grids**, both created by the simulator itself, tract size 1 (one tract = one cell):

| Grid | Instance ID at creation | Role |
|---|---|---|
| "working" | `0x49D5BB8C` | Rebuilt by each pass (states 2–6, §6) |
| "published" | `0x49D5BBA1` | Returned by `GetWateredGrid`, `GetSupplyValueMapPtr` and every query |

At the end of state 6 the two grid **pointers are swapped** (§6, state 6). The freshly built grid becomes the
published one, and the old published grid becomes the next working grid. The published grid's object and
instance ID therefore alternate from pass to pass *[compat]* *[quirk]*. **Callers must not cache the grid
pointer across passes.** The original's callers fetch it again on every update (§12).

**Published grid value bits** *[compat]*:

| Bit | Meaning | Set by |
|---|---|---|
| `0x01` | watered | the ring flood (state 4), then copied onto building lots (state 6) |
| `0x02` | building or producer lot cell | state 6 |
| `0x04` | producer-only cell (the anchor cell has the producer flag but no consumer flag) | state 6 |
| `0x08` | pipe in this cell | state 2 |

`AreWateredBuildingsInCell` tests `(v & 3) == 3`. `IsCellWatered` tests `v & 1`.

**The flag map**: one byte per cell, `countX × countZ`, indexed `z·countX + x`. It is not a grid object, and it
is persisted (§9).

| Bit | Meaning |
|---|---|
| `0x01` | the pipe in this cell connects to x−1 |
| `0x02` | connects to z−1 |
| `0x04` | connects to x+1 |
| `0x08` | connects to z+1 |
| `0x10` | a pipe occupies this cell |
| `0x20` | a water producer's lot covers this cell |
| `0x40` | a water consumer (a building with the water-consumption property) is anchored here |
| `0x80` | unused |

**The visited bitmap**: one bit per cell of a `side × side` square, `side = max(countX, countZ)`. It is
scratch space for the flood and is not persisted. It uses the shared 1-bit helpers of power §2.1: `side² / 32`
words (signed division, `0x006DF468`), word `(side·z + x) >> 5`, bit `x & 31`. This is the original layout
*[compat]*; the two bit rules agree because `side` is always a multiple of 32 (64, 128 or 256), so no cell shares a
bit with shipped city sizes. *[choice]* FreeCity4 may use `i & 31` with `ceil(side² / 32)` words.

## 3. State

| State | Initial (constructor) | Persisted | Notes |
|---|---|---|---|
| tuning scalars (§8) | code defaults | no | re-read at `Init` and on `0xCA1B151C` |
| tuning curves (§8) | empty | no | |
| working / published grid | null | yes (refs) | §2 |
| countX, countZ | 0 | yes | |
| flag map | none | yes | |
| visited bitmap, two flood queues (cell lists of `(u8 x, u8 z)`) | none | no | |
| scanning iterator (simgrid §7.4) over `[0, countX−1] × [0, countZ−1]` | none | yes | |
| producer list (§5.3) | empty | yes (partly) | |
| current producer (the network being flooded) | null | yes | |
| pipe list (references to pipe network occupants) | empty | yes | |
| numPipes, numDistressed, numBurst (u32) | 0 | yes | |
| pipe condition (f32) | 0 | **no** | §7.1. 0 after load *[quirk]* |
| demand (s64): Σ water-consumption property of all consuming buildings | 0 | yes | |
| department (`cISC4DepartmentBudget`) | null | yes (ref) | the water department `0xAA4014B4` |
| passCounter (s32) | 0 | yes | +1 at each pass start |
| averageWaterPollution (u32) | 0 | yes | §7.2 |
| totalWaterCost (s64) | 0 | yes | §6, state 8 |
| numPolluted (u32) | 0 | yes | producers shut down by pollution |
| last month totals: produced, consumed, imported, exported (s64) | 0 | yes | published at the end of a pass |
| pass totals: produced, consumed, imported, exported (s64) | 0 | no | |
| building cell counters: total, watered (u32), current and last | 0 | no | |
| consumption factor (f32) | 0 | no | read from the ordinances at pass start |
| network supply, consumed, importOwed (s64), ring counter, ring remaining | 0 | no | flood scratch |
| pipe count of the last pass (u32), unused counter (u32) | 0 | no | history only |
| state (0..9) | **9** (idle) | no | `Read` sets 0 |
| recompute flag | false | no | |
| ageing flag | false | no | |
| enabled flag | false | no | set by ConnectionsReady; never cleared |
| initialized flag, grids-initialized flag | false | no | `Shutdown` clears only the first (§4.2) |
| random generator (`cRZRandom`) | none | no | created in `Init` |

## 4. Lifecycle

### 4.1 `Init()` → true (AL = 1)

1. Set a process-wide flag (unknown purpose, §13).
2. Each of the following only if it is not already held: hold the city (from the game's city global); create a
   new `cRZRandom` with seed `0xFFFFFFFF` (wall clock, foundations §6.1); hold the occupant manager (city slot
   34), the pollution simulator (slot 66) and the neighbours simulator (slot 62), each AddRef'd.
3. Only on the **first** `Init` of the object (initialized flag false; set true here):
   - `ReadSimulatorTunableValues()` (§8).
   - Register **three agents** with `cISC4Simulator::AddAgent(target, level, "cSC4PlumbingSimulator", flags = 2)`
     (compiled slot 19, pop 16), in this order: level **6** (month), level **4** (day), level **10** (idle).
   - If the message server is not yet held: hold it and `AddNotification` (slot 5) for `0x99EF1142`,
     `0x99EF1143`, `0xCA1B151C`, `0xEA1F0DCD`, `0xEA418D8F` and `0x6AC284F3`.
   - If there is **no department** (new city):
     1. `budget.CreateDepartmentBudget(0xAA4014B4, 0x4A357EAF)` (budget slot 25).
     2. `dept.SetDepartmentName(0x6A3FF01C, 0x2A4937D3)` (department slot 5).
     3. `dept.SetMaxAllowedFundingPercentage(&maxFunding)` (slot 13).
     4. `li := dept.CreateLineItem(2, false)` (slot 17), then `li.SetName(0x6A3FF01C, 0x8A75F61E)` (line-item
        slot 7). Line item 2 holds the pipe maintenance cost.
   - Otherwise (the department came from the save): `dept.RemoveLineItem(1)` (department slot 19)
     *[compat]* *[inferred: removes a line item from older saves]*.
4. If the grids are not initialized (always on a new city; never after `Read`, which sets the flag):
   1. For each grid that is still null, construct a `cSC4SimGrid<uint8>` inline, **not through GZCOM**
      (CLSID field `0x49B9E602`, simgrid §2). Then AddRef, `Init`, `SetTractSize(1)`,
      `SetInstanceID(0x49D5BB8C / 0x49D5BBA1)` and `SetTractValues(0)`.
   2. countX, countZ := the city cell counts.
   3. Flag map: allocate if absent and **zero it**. This happens only in this branch, so a loaded city keeps its
      flags.
   4. Visited bitmap: allocate if absent and clear it.
   5. Scanning iterator: create it if absent, over `[0, countX−1] × [0, countZ−1]` (an empty range is widened to
      one cell). It is not reset.
   6. state := 0; recompute := true.

On a loaded city the grids, flag map, bitmap and iterator come from `Read` (§9).

### 4.2 `Shutdown()` → true (AL = 1)

If initialized (`0x006DA900`):

1. Clear the initialized flag.
2. Delete the random generator. Release the city, occupant manager, pollution and neighbours simulators and the
   department (all nulled).
3. `RemoveNotification` for the six messages, then release the message server (nulled).
4. `RemoveAgent` (simulator slot 20).
5. `Shutdown` and release both grids (nulled). Free the flag map and the bitmap (nulled). Release every
   producer's occupant and empty the producer list. Delete the scanning iterator (nulled). current producer :=
   null. Release every pipe and empty the pipe list.
6. Zero numPipes, numDistressed, numBurst and numPolluted.

It does **not** clear the **enabled** flag, the **grids-initialized** flag, countX/countZ, the state or the
recompute flag *[compat]*.

**A second `Init` on the same object** (QUESTIONS PL2, checked in `0x006DF0D0` and `0x006DA900`):

- **Without a `Shutdown` in between**: both flags are set, so steps 3 and 4 are skipped, and step 2 finds
  everything held. Only the process-wide flag of step 1 is touched. It is a no-op *[compat]*.
- **After `Shutdown`** *[compat]*: the initialized flag is clear, so step 2 re-acquires the city, a new random
  generator and the three simulators, and step 3 runs again in full. It re-reads the tuning (the "keep" curve
  pattern of foundations §4.1 applies), registers the three agents again, and subscribes the six messages
  again. The department is null, so `CreateDepartmentBudget(0xAA4014B4, …)` runs again (it returns the existing
  department unchanged, budget §6), followed by `SetDepartmentName`, `SetMaxAllowedFundingPercentage`,
  `CreateLineItem(2, false)` and `SetName`. The grids-initialized flag is **still set**, so step 4 is
  skipped. The grids, the flag map, the bitmap and the scanning iterator stay **null**, and the state and
  recompute flag keep their pre-`Shutdown` values.
- The original then dereferences null on the first use of any of them. That is the first pass through state 2
  (or at once if `Shutdown` interrupted a pass in states 2–7), the first building or pipe insertion (§5.3 calls
  the working grid, §5.4 writes the flag map), the first `IsCellWatered` / `GetSupplyValue` query, or a
  `Write`. `GetWateredGrid` returns null. It **crashes**; it neither recreates nor reuses the freed grids.
- `Shutdown`, then `Read`, then `Init` works: `Read` allocates the flag map and the bitmap, creates the
  iterator, takes the grids from the save and sets the grids-initialized flag (§9).

The game never re-`Init`s a shut-down simulator. `Release` at count ≤ 1 calls `Shutdown` and then destroys the
object (§1), and every load creates fresh objects (foundations §3.7) *[inferred: no other `Shutdown` caller
found]*.

*[choice]* FreeCity4's `Shutdown` also clears the grids-initialized flag. A second `Init` after `Shutdown` then
rebuilds the grids, flag map, bitmap and iterator exactly as on a new city (state := 0, recompute := true). This
is invisible to the game and removes the crash.

## 5. Events

### 5.1 Messages handled (`DoMessage`, compiled at `cIGZMessageTarget2` slot 3, pop 4)

`DoMessage` returns **AL = 1**, except for the day and idle messages, which return `DoIdle`'s result *[compat]*.

| Message | Action |
|---|---|
| SimNewMonth `0x66956816` (agent, level 6) | Monthly simulation (§7) |
| SimNewDay `0x66956814` (agent, level 4) | `DoIdle(5)`: 5 ms budget (§6) |
| SimIdle `0x6695681A` (agent, level 10) | `DoIdle(msg.GetData3())`: data3 ms (simulator §5.4) |
| ConnectionsReady `0x6AC284F3` (posted once per load by the regional city; when and when not: foundations.md §11.6) | enabled := true |
| InsertOccupant `0x99EF1142` / RemoveOccupant `0x99EF1143` | `occ := GetVoid1()`. Skip if `occ.AsPropertyHolder()` (occupant slot 6) **has** property `0x029244DB` (preview occupant, foundations §5; holder slot 3; existence only). Else `DoOccupantInserted / Removed(occ)` (§5.2) |
| `0xCA1B151C` (tuning reload) | `ReadSimulatorTunableValues()` |
| `0xEA1F0DCD` (pipe burst at a cell) | `x := GetData1()`, `z := GetData2()`. `flags[z·countX + x] &= 0xE0`, with **no bounds check**. recompute := true. Then the `0xEA418D8F` test below runs too (it fails, the type differs) |
| `0xEA418D8F` (neighbour deal status) | Only when `GetData1() == 0` ("deal changed", neighbors.md §5.3): QI `GetIGZUnknown()` for `cISC4NeighborDeal` (`0x0773FEF1`). If the deal's type (deal slot 8) is `0x4773FFE3` (water export) or `0x4773FFE2` (water import): **state := 9, recompute := true**. This aborts any pass in progress; the next `DoIdle` restarts at state 0. Release the deal |
| anything else | nothing |

Notes on `0xEA1F0DCD`:

- The pipe-burst disaster posts it (`cSC4DisasterManagerPipeBurst::CreateLeakEffectForPipe`) with
  `data1 = x` and `data2 = z` (city cells).
- `& 0xE0` clears the four connection bits **and the pipe bit `0x10`** *[compat]* *[quirk]*. The cell drops out
  of the network until the pipe occupant is removed and re-inserted, which re-sets the bits.
- The pipe stays in the pipe list.

Notes on `0xEA418D8F`:

- The simulator's own `0xEA418D8F` (data1 = 8, §6 state 8) is ignored.
- `SetFlags` on a water deal (§6, states 4 and 8) posts status 0 when the flags actually change (neighbors.md
  §5.2). That message later aborts and restarts the pass *[compat]* *[quirk]*. The restarted pass sets the same
  flags without a change, so this happens at most once per flag change.

### 5.2 Occupant changes

`DoOccupantInserted(occ)` and `DoOccupantRemoved(occ)` dispatch on `occ.GetType()` (occupant slot 7):

- `0x278128A0` (building) → `DoBuildingChanged(occ, insert?)` (§5.3);
- `0xC772BF98` (network occupant) → QI `0x49C05B9F` (`cISC4NetworkOccupant`). Unless
  `HasNetworkFlag(0x80000000)` (network occupant slot 8) is true, call `DoPipeChanged(netOcc, insert?)` (§5.4).
  Then release. *[inferred: flag `0x80000000` marks a non-final/preview network piece.]*
- other types are ignored.

### 5.3 Buildings (`DoBuildingChanged(occ, insert)`) *[compat]*

1. **Anchor cell**: `p := occ.GetPosition()` (occupant slot 8). `(x, z) := working grid.PositionToTract(p.x, p.z)`
   (grid slot 19; tract size 1, so this is the city cell, with no bounds check). `holder := occ.AsPropertyHolder()`.
2. **Consumer**: if property `0xC8ED2D84` (water consumption, read as **sint32**, getter `0x005FD450`) is
   present:
   - insert: `flags[x,z] |= 0x40`, `demand += c` (sign-extended);
   - remove: `flags[x,z] &= ~0x40`, `demand −= c`.

   On removal the bit is cleared even if another consumer is still anchored in the same cell. That consumer then
   no longer draws water and is never marked watered, until something re-inserts it *[quirk]*.
3. QI the occupant for `cISC4BuildingOccupant` (`0x87DDA3A9`). If that fails, stop here. recompute is **not**
   set.
4. **Budget** (the same scheme as pollution.md §4.2.4, purpose `0xCA58C9D5`):
   1. `item := budget.GetBudgetItemForPurpose(holder, 0xCA58C9D5)` (budget slot 51). If there is none, skip
      to step 5.
   2. `d := budget.GetDepartmentBudget(item.departmentID)` (slot 28).
   3. If `item.lineItem == 0`: `budget.ChangeBudgetItemLine(holder, item.index, building.GetBuildingType())`
      (budget slot 52, building-occupant slot 4), then re-read the item. If its line is still 0, skip to
      step 5.
   4. `li := d.GetLineItem(line)` (department slot 20). If null: `li := d.CreateLineItemForBuildingType(line,
      false)` (slot 18), then `li.SetDisplayFlag(2, true)` (line-item slot 18).
   5. `li.AddToFullExpenses(±item.cost)` (line-item slot 13, int64).
   6. `li.SetSecondaryInfoField(li.GetSecondaryInfoField() ± 1)` (line-item slots 9 / 8, int64). This counts
      the buildings on the line.
   7. Set the occupant's **int64** property `0x2A58D350 := item.cost`, on insert **and** on remove
      (setter `0x005FE450`).
5. **Producer**: `holder.GetProperty(0x88ED3303)` (water production capacity; holder slot 9).
   - **Insert**, if present:
     1. Create a record: seed := `(x, z)`; AddRef the building occupant.
     2. `lot := lotManager.GetLot(x, z, false)` (compiled slot 6, pop 12, foundations §16). If there is no lot,
        **return at once**: the record is leaked, the occupant reference is held forever, recompute is not set,
        and the building-interface reference is not released *[quirk]*.
     3. footprint := `lot.GetBoundingRect()` (lot slot 14, inclusive cells).
     4. maxCapacity := the property's value as uint32 (variant compiled slot 72).
     5. production := 0, efficiency := 0, polluted := false, visited := assigned := false.
     6. **push_front** the record onto the producer list.
     7. `flags |= 0x20` over the footprint.
     8. Set the float property `0xC910B4CF` (condition) := 0 (setter `0x005FE4F0`).
   - **Remove**: find the record by occupant. If found: clear `0x20` over its footprint, clear the current
     producer if it is this one, erase the record and release the occupant.
6. recompute := true. Release the building interface.

Producer records hold:

- the seed cell `(x, z)`;
- the footprint `(x1, z1, x2, z2)`;
- maxCapacity and production (u32);
- efficiency (f32) and polluted (bool), both not saved;
- visited and assigned (pass scratch);
- the occupant.

### 5.4 Pipes (`DoPipeChanged(netOcc, insert)`) *[compat]*

1. `occ := netOcc + 4`. The original converts `cISC4NetworkOccupant*` to `cISC4Occupant*` with a fixed pointer
   adjustment, not with a QI. See §12.2.
2. `(x1, z1, x2, z2) := occ.GetBoundingCityCells()` (occupant slot 11). For x from x1 to x2 (outer) and z from
   z1 to z2 (inner):
   - **insert**: `flags |= 0x10`, then for each direction `d` with `netOcc.ConnectsToNeighbor(d, 4)` (network
     occupant slot 47, network type 4 = pipe): d 2 → `|= 0x04`, d 1 → `|= 0x02`, d 0 → `|= 0x01`,
     d 3 → `|= 0x08` (called in that order). Old connection bits are **not** cleared first *[quirk]*.
   - **remove**: `flags &= 0xE0`. This also clears the bits of a pipe that shares the cell.
3. Pipe list:
   - **insert**: if the pipe is not in the list, AddRef it, append it and numPipes += 1.
   - **remove**: if it is in the list:
     1. If `netOcc.GetNetworkFlag() & 0x10000` (slot 12): numDistressed −= 1.
     2. If `& 0x20000`: numBurst −= 1.
     3. Erase it, release it, numPipes −= 1.
4. `occ.AsPropertyHolder()` is called and the result is ignored.
5. **Pipe cost**: `dept.GetLineItem(2).SetFullExpenses(round(float(numPipes) × costPerPipe))` (department
   slot 20, line-item slot 12, int64).
   - The product is formed in x87 arithmetic (numPipes as an **unsigned** 32-bit value).
   - It is rounded to nearest by adding `1.5·2^23` and storing as `float32`, so it is valid only for results
     below 2^22.
   - Precision (§13 question 5, resolved): the game runs the x87 at 24-bit precision (foundations §18), so the
     product is first rounded to a 24-bit significand and the add then rounds half to even. *[compat]* FreeCity4
     computes the product in `f64`, rounds it to `f32`, then rounds half to even.
6. recompute := true.

## 6. The pass (`DoIdle(budgetMs)`) *[compat]*

`DoIdle` returns "finished" (AL):

1. If **not enabled** → true. Nothing runs until ConnectionsReady has been received.
2. If state = 9 and recompute: recompute := false, state := 0.
3. Dispatch on the state. A state > 9, and state 9 itself, → true.

States 0–5 always finish in one call and return **true**. The exception is state 5, which falls through into
state 6 in the same call. States 6 and 7 are time-sliced: they use the CPU-cycle budget of simulator §5.5 with
an adaptive check stride of 1..1024 that starts at 1024, one static stride per state, as in landvalue §6. A
time-out returns **false** ("not finished"). On the day level this keeps the day open until the state completes
(simulator §5.3). *[choice]* FreeCity4 may slice deterministically (for example the whole state per call), as
landvalue §6 allows. The results must equal those of an uninterrupted pass.

A pass needs at least `2N + 7` calls, where `N` is the number of networks flooded.

| State | Work | Next |
|---|---|---|
| 0 | Start (§6.1) | 1 |
| 1 | `UpdateWaterProduction()` (§7.2) | 2 |
| 2 | Working grid: for every cell, `v := 8` if `flags & 0x10`, else 0. Clear every producer's visited and assigned. current producer := null | 3 |
| 3 | Seed one network and flood its pipes (§6.2). If there is nothing left to seed: reset the scanning iterator to (0, 0) | 4, or 5 |
| 4 | Exports, ring flood, imports, water used (§6.3). current := null | 3 |
| 5 | Pipes and connections (§6.4), then fall through into 6 in the same call | 6 |
| 6 | Copy the watered state onto lots (§6.5), time-sliced. At the end: reset the iterator, **swap the grid pointers** | 7 |
| 7 | Redisplay (§6.6), time-sliced | 8 |
| 8 | End of pass (§6.7) | 9 |
| 9 | idle | — |

### 6.1 State 0: start

1. passCounter += 1.
2. factor := `ordinanceSimulator.GetCumulativeFactor(0xA8F4EB0C, 0)` (ordinance slot 25, pop 8, float in ST0;
   from the game's ordinance global). *[inferred: the water-consumption factor of water ordinances.]*
3. For the deals of type `0x4773FFE2` (import), then `0x4773FFE3` (export) (`neighbours.FindDeals(type, list)`,
   slot 30): `deal.SetAmountUsed(0)` (deal slot 21).
4. `neighbours.GetConnectionsOfType(5, list)` (slot 17; type 5 = water). For each: `SetVisitedThisCycle(false)`
   (connection slot 12).
5. Zero the pass totals (produced, consumed, imported, exported) and the pass building-cell counters.

### 6.2 State 3: seed and pipe flood

importOwed := 0. If no network is in progress (current producer null), find a seed.

The current producer is **always null on entry** *[compat]* (QUESTIONS PL3). Its only writers are:

| Writer | Value |
|---|---|
| constructor | null |
| `Read` (§9) | null, or the matching producer record |
| `Shutdown` (§4.2) | null |
| `DoBuildingChanged` removal (§5.3) | null, when the removed record is current |
| state 2 (`0x006DD766`) | null |
| state 3, producer seed (`0x006DD92E`) | the producer. State 3 then moves to state 4 in the same call |
| end of state 4 (`0x006DE5E8`) | null |

The only writers of state 3 are the ends of state 2 (`0x006DD770`) and state 4 (`0x006DE5EE`), and each nulls
the current producer in the same call. `Read` sets state 0, so a restored current producer is nulled by state 2
before state 3 can see it. The original's non-null branch (`0x006DD79C`) is therefore dead code. It would skip
seeding and clearing, and run the pipe flood on the leftover queue A with the old supply. *[choice]* FreeCity4
always seeds.

To find a seed:

1. **Import seed.** For each deal of type `0x4773FFE2` in `FindDeals` order, with `used < amount` (deal slots
   20 and 10):
   1. `c := neighbours.FindUnvisitedConnection(5, deal.GetCityOnOtherSide())` (neighbours slot 15, deal slot 7).
      If c is null, try the next deal.
   2. `c.SetVisitedThisCycle(true)`.
   3. `o := neighbours.FindConnectionOnOtherSide(c)` (slot 13).
   4. If `o` exists and `o.GetIsActive()` (connection slot 13): use this deal.
      - seed := c's connection cell (`c.GetConnectionCell(&x, &z)`, connection slot 6), clamped to
        `[0, countX−1] × [0, countZ−1]`;
      - supply := importOwed := `amount − used`.

   Otherwise try the next deal.
2. **Producer seed.** Otherwise: the first producer in list order that is not visited. Mark it visited, make it
   current, seed := its seed cell, supply := its production.
3. **Nothing left**: reset the scanning iterator to (0, 0), state := 5, return true.

Then:

1. Clear both flood queues and the visited bitmap.
2. Push the seed onto queue A (to process) and queue B (all network cells), and mark it in the bitmap.
3. consumed := 0.

**Pipe flood** (breadth-first, until queue A is empty). Pop `(x, z)` from A, then let `f := flags[x, z]`:

1. If `f & 0x20`: find the **first** producer in list order whose seed cell is `(x, z)`. If that producer is not
   visited: supply += its production (when non-zero), and mark it visited.
2. For each direction bit in `f`, in the order `0x01` (x−1, if x > 0), `0x04` (x+1, if x < countX−1), `0x02`
   (z−1, if z > 0), `0x08` (z+1, if z < countZ−1): if the neighbour cell has the pipe bit `0x10` and is not in
   the bitmap, mark it and push it onto A and B.

The neighbour does **not** need the reciprocal connection bit. The seed cell itself does not need a pipe.

At the end: ringRemaining := |B|, ring := 0, state := 4, return true.

Seeding quirks *[compat]* *[quirk]*:

- Producers reached by a flood are visited and never seed again. Producers sharing a seed cell are an exception:
  only the first is found by the flood, and the second later seeds the same network again.
- An import deal can seed once per unvisited connection to its city, each time with its remaining amount.
- Each seed clears the bitmap. A network reached from two seeds is therefore flooded, and its consumers charged,
  once per seed.

### 6.3 State 4: exports, ring flood, imports, water used

All amounts are signed 64-bit.

1. ownProduction := supply − importOwed. pass produced += ownProduction.
2. **Exports**: for each deal of type `0x4773FFE3` in `FindDeals` order:
   1. take := min(amount − used, supply). If take ≤ 0, skip the deal.
   2. If `IsNeighborConnectedToCurrentNetwork(deal.GetCityOnOtherSide(), false)` (below):
      - pass exported += take;
      - supply −= take;
      - `deal.SetAmountUsed(used + take)`;
      - exportTaken += take.
   3. Otherwise: `deal.SetFlags(0x08 NoConnection, true)` (deal slot 37, call at `0x006DDE76`). This is per
      network: a deal that the current network cannot reach is flagged even if another network of the same
      pass (earlier or later) delivers it *[compat]* *[quirk]*. A deal skipped by step 1 (nothing owed, or a
      network with no supply) is not flagged.
3. **Ring flood**, only if supply > 0 or exportTaken > 0. It runs breadth-first over queue B, which starts as
   all network cells (ring 0). Repeat while B is not empty:
   1. If ringRemaining ≤ 0: ringRemaining := |B|, ring += 1. If ring ≥ radius, stop.
   2. ringRemaining −= 1. Pop `(x, z)` from B.
   3. If `flags[x, z] & 0x40`:
      - `c := low32(trunc(float(int32 BuildingConsumption(x, z)) × factor))`, sign-extended;
      - consumed += c;
      - if consumed > supply: consumed := supply and **stop**. This cell is not marked.
   4. Working grid `(x, z) |= 1`. The original writes it through the raw grid data.
   5. Push each in-bounds neighbour that is not in the bitmap (mark it), in this order: (x−1, z), (x+1, z),
      (x, z−1), (x, z+1).
   6. If ring < `radius >> 1`, also push the diagonals: (x−1, z−1), (x+1, z−1), (x−1, z+1), (x+1, z+1).

   Effect: around the network, distance is Chebyshev for the first `radius >> 1` rings and Manhattan after
   that. Cells in rings 0 … radius−1 are watered. Ring 0 is the pipe network itself.
4. pass consumed += consumed. consumed += exportTaken.
5. **Imports**, only if importOwed > 0. For each deal of type `0x4773FFE2`:
   1. take := min(amount − used, consumed). If take ≤ 0, skip the deal.
   2. If `IsNeighborConnectedToCurrentNetwork(deal's city, false)`:
      - pass imported += take;
      - consumed −= take;
      - `SetAmountUsed(used + take)`.

   No flag is set on failure.
6. ratio := ownProduction > 0 ? `f32(consumed / ownProduction)` : 0. The quotient is computed in x87 from the
   two 64-bit integers.
7. For each producer that is visited and not assigned:
   - set its uint32 property `0x08D9763D` (water used) := `trunc(u32 production × ratio)` (setter
     `0x005FE3C0`);
   - mark it assigned.
8. Clear queue B. current := null. state := 3. Return true.

**`IsNeighborConnectedToCurrentNetwork(city, b)`**:

1. `neighbours.GetConnectionsOfTypeToCity(5, city, list)` (slot 18).
2. For each connection, take its connection cell clamped to the city cell range.
3. Return true at the first connection whose cell is set in the **visited bitmap**.

For exports, the bitmap holds only the pipe network. For imports, it also holds the ring-flood cells, so an
import connection within `radius` of the network counts as connected *[compat]* *[quirk]*. The second argument
is unused *[inferred]*.

**`BuildingConsumption(x, z)`**: the occupant manager's typed cell query (compiled slot 41,
`GetOccupantsByStandardCityCells(list, &cell, &cell, 0x278128A0, −1)`, pop 20, foundations §16). It returns the
sum of the **uint32** property `0xC8ED2D84` over the buildings found.

### 6.4 State 5: pipes and connections

1. lastPassPipeCount := 0, unusedCounter := 0.
2. For each pipe in list order:
   1. `(x, z) := netOcc.GetOccupiedCell()` (network occupant slot 59).
   2. **Direct, non-virtual call** to `cSC4PipeOccupant::SetWateredState(bool)` (`0x00648540`, pop 4) with
      `working.GetCellValue(x, z) & 1`. That function stores the state in the pipe and, when it changes,
      tail-calls pipe vtable slot 100 (§12.2).
   3. lastPassPipeCount += 1.
3. For each water connection (`GetConnectionsOfType(5)`): `SetIsActive(working(clamped cell) & 1)`
   (connection slot 14).
4. state := 6 and fall through.

### 6.5 State 6: lot fill (time-sliced)

Iterate the scanning iterator (x inner, z outer). For each cell with `flags & 0x60`:

1. `v := (working(x, z) & 1) | 2`.
2. If `flags & 0x40`: building cells += 1, and if `v & 1`, watered building cells += 1. Otherwise `v |= 4`.
3. For each building found with `GetOccupantsByStandardCityCells(list, &cell, &cell, 0x278128A0, −1)`:
   `FillGridAtOccupant(working, occ, v)`:
   1. min corner := `occ.GetBoundingBox()` (occupant slot 10) min, converted with `PositionToTract`.
   2. If `lotManager.GetLot(that cell, false)` exists: OR `v` over the lot's **inclusive** bounding rect (through
      the raw grid data).
   3. Otherwise OR `v` over `[tract(min), tract(max))`, half-open.

The cell value is read **live**, so an earlier fill can make a later anchor cell watered *[compat]*.

- A time-out returns **false**; the iterator keeps its position.
- At the end: reset the iterator, swap the working and published pointers, state := 7, return true.

### 6.6 State 7: redisplay (time-sliced)

Iterate the cells again *[compat]* (QUESTIONS PL4). State 7 does **not** reset the scanning iterator; state 6
left it fresh at (0, 0). Each call continues with `GetNext` (simgrid §7.4, `0x0060C8D0`):

- rectangle `[0, countX−1] × [0, countZ−1]` (set by `Init` on a new city, or taken from the save, §9);
- step 1, x inner, z outer: (0, 0), (1, 0) … (countX−1, 0), (0, 1) … (countX−1, countZ−1);
- end: the `GetNext` after the last cell sets x := x0 = 0 and z := countZ, finds z > z1, sets **done** and
  returns false.

The iterator stays **done at (0, countZ)** (for example (0, 64) on a 64 × 64 city, not (0, 63)), with step 1
and the rectangle unchanged. It is not reset at the end of state 7. It keeps that position through states 8
and 9, and that is what a save between passes writes (§9 field 14: state 2, x 0, z countZ). State 2 of the
next pass resets it to (0, 0) (`0x0060C860`). State 2 itself scans the whole grid with it and leaves it done
at the same spot. The nothing-left branch of state 3 then resets it for state 6.

For each cell:

1. Compare the new published grid with the old one (now the working grid), cell by cell.
2. Track the bounding box of the cells that differ. The minimum starts at (countX, countZ) and the maximum at
   (0, 0), per call.
3. When minX ≤ maxX: `city.GetTerrain()` (city slot 84) `.RedisplayTerrain(0, 0, &min, &max, 0)` (terrain slot
   69, pop 20).
   - On time-out this redisplays the box found so far and returns false.
   - At the end it redisplays the box, sets state := 8 and returns true.

### 6.7 State 8: end of pass (one call, returns true)

1. For each producer:
   1. totalWaterCost += `trunc(int32 used(0x08D9763D) × float property 0xEA1E3C53)` (s64).
   2. If the producer has the curve property `0x0911E2E7`: cost := `trunc(curve(condition 0xC910B4CF) ×
      s64 property 0x2A58D350)`.
   3. If its budget item (purpose `0xCA58C9D5`) has `item.cost ≠ cost`:
      - `budget.GetLineItemFromBudgetItem(item)` (slot 57) `.AddToFullExpenses(cost − item.cost)`;
      - `budget.ChangeBudgetItemCost(holder, item.index, cost)` (slot 53, pop 16).
2. last building-cell counts := pass counts.
3. If the ageing flag is set: `AgeWaterProducer` for each producer (§7.3), then ageing := false.
4. Post `0x6946DC34`, a new `cRZMessage2Standard` with `void1 = this` and `IGZUnknown = this`
   (`MessagePost(msg, false)`, server slot 4).
5. last month totals := pass totals (produced, consumed, imported, exported).
6. History (`cISC4HistoryWarehouse::RecordHistoryData(id, u32)`, slot 12, pop 8):

   | ID | Value |
   |---|---|
   | `0x0A14EBB4` | produced |
   | `0xCA14EBD0` | consumed |
   | `0xCAB2F27B` | imported |
   | `0x8AB2F280` | exported |
   | `0xCA14EBFE` | lastPassPipeCount |
   | `0xCA14EC19` | unusedCounter (always 0) |
   | `0xCA78A955` | watered building cells |
   | `0x8A78AB06` | building cells − watered |

   The values are the low 32 bits.
7. Each export deal (`FindDeals(0x4773FFE3)`) with `used < amount` (signed 32-bit, deal slots 20 and 10):
   `SetFlags(0x20, true)` (call at `0x006DEFEF`). neighbors.md calls this flag "NoLandfill"; for water it
   means the export was not fully delivered.

   **Export flags** *[compat]* (QUESTIONS PL7): the plumbing simulator has exactly two `SetFlags` calls, both
   with `on = true`, and no `SetFlags(0x28)`. It never clears a deal flag. An export deal with amount > 0 that
   no network reaches therefore gains `0x08` in state 4 (from the first network with supply > 0) and `0x20`
   here, so it ends the pass with **`0x28`** set. If no network has supply, only `0x20` is set. A deal that is
   fully delivered gets no `0x20`, but it can still carry `0x08` from another network (§6.3). `SetFlags` itself
   drops `0x38` while the deal has Grace (neighbors.md §5.2).
8. `neighbours.MarkDealsDirty()` (slot 31).
9. state := 9. Post `0xEA418D8F` with data1 = 8 (`SC4PostMessage2`).

## 7. Monthly simulation and production

### 7.1 Month (`DoMonthlySimulation`, level 6) *[compat]*

It runs even when the simulator is not enabled.

1. recompute := true; ageing := true.
2. `ApplyPipeFunding(f)`, with f := `dept.GetFundingPercentage()` (department slot 10, which returns a pointer to
   the fraction):
   1. cond := cond − `curve 0xCA7138C4 (f × 100)`. Clamp to [0, 1]. If numPipes = 0: cond := 0.
   2. target := `trunc(float(u32 numPipes) × cond)`.
   3. bursts := `random.RandomSint32RangeGaussianFast(0, trunc(2 × float(int32 target) × burstProbability))`.
      This uses the simulator's own `cRZRandom`, **exactly 4 `NextRaw` draws every month**, including when the
      upper bound is 0. The method:
      - `a := (r1 + r2 + r3 + r4 + 2) >> 2`, the sum taken in 64 bits;
      - result = `lo + ((a × (u32)(hi − lo)) >> 32)`, in `[lo, hi)`;
      - it is `lo` when `hi ≤ lo`.
   4. If target ≠ numDistressed or bursts ≠ 0:
      1. Copy the pipe list, in list order, into a vector and `std::random_shuffle` it with the CRT `rand`
         (foundations §12.3, §12.5).
      2. If target > numDistressed: walk forward, and for each pipe without `0x10000`:
         `SetNetworkFlag(0x10000)` (slot 10), numDistressed += 1, until equal.
      3. If target < numDistressed: walk forward, and for each pipe with `0x10000`: clear it (slot 11),
         numDistressed −= 1, until equal. Its burst flag `0x20000` is **not** cleared.
      4. Walk **backward** from the end while bursts > 0. For each pipe with `0x10000` and without `0x20000`:
         - `dm := city.GetDisasterLayer()` (slot 72) `.GetDisasterManager(0x4A1332E8)` (slot 7);
         - if `dm` exists: `dm.AttemptDisasterStart(x, z)` (pipe-burst manager slot 6, `0x0057D180`, pop 8),
           with `(x, z)` from `GetOccupiedCell`;
         - `SetNetworkFlag(0x20000)`, numBurst += 1, bursts −= 1. This happens even when `dm` is null.

Because cond is not saved (§3), after a load the pipes start from cond 0. Distressed and burst flags live in the
pipe occupants. numDistressed and numBurst are saved, so the counters persist.

### 7.2 `UpdateWaterProduction` (state 1) *[compat]*

1. Reset: averageWaterPollution := 0, numPolluted := 0, sum := 0, count := 0.
2. eff0 := `curve 0x6A410BF2 (f × 100) × 0.01`, with f the department funding fraction (as in §7.1).

For each producer, in list order:

1. cap := maxCapacity; eff := eff0.
2. `v := 0; pollution.GetWaterValue(seedX, seedZ, &v)` (pollution slot 8, pop 12, city cells; pollution.md §8.1).
   This is the cell's **current water pollution**, after the water treatment of pollution.md §4.4.4 step 6. Then
   sum += v (u32), count += 1.
3. `lot := lotManager.GetOccupantLot(occ)` (slot 14). If the lot exists, `lot.GetNeedsPower()` (slot 65) is true
   and `lot.GetHasPower()` (slot 66) is false: production := 0, efficiency := 0, polluted unchanged, and go to the
   next producer.
4. type := uint8 property `0x48F23A7E` (variant compiled slot 64), 0 when missing. *[inferred: 0 = fresh-water
   pump, 1 = salt-water/desalination pump, others = no water needed.]*
   - type 0: `r := NearestWater(x, z, 0, freshReach)`. If r = −1: cap := 0. Else if r > 1:
     `cap := lossPct × maxCapacity / 100` (unsigned 32-bit).
   - type 1: `r := NearestWater(x, z, 1, saltReach)`. If r = −1: cap := 0.
5. polluted := false. If `v > maxPumpedPollution` (unsigned) and maxCapacity > 0 (signed): polluted := true,
   numPolluted += 1, eff := 0.
6. If the producer has the curve property `0x8A1E07EE`: eff ×= `curve(condition 0xC910B4CF, default 0)`. The
   curve is built from the property for each call (loader `0x005F4620`).
7. production := low32 of `trunc(float(int32 cap) × eff)`; efficiency := eff.

At the end, if count > 0: averageWaterPollution := sum / count (unsigned).

**`NearestWater(x, z, type, reach)`** (`0x006DB2B0`) *[compat]*:

1. type 2 → 0. If the city has no terrain (city slot 84) → −1.
2. The box is `[max(0, x − reach), min(x + reach, terrainCellsX − 1)]` × the same in z (terrain slots 7 and 8;
   int32 arithmetic). An upper bound below the lower bound is raised to it.
3. If `(x, z)` lies outside the box (an anchor outside the terrain) → −1, with no cell tested.
4. Walk the spiral iterator (`SpiralIterator`, constructor `0x0043FE10`, `GetNext` `0x00440850`) from `(x, z)`
   over the box, clockwise. The order is (x, z), then ring 1: (x+1, z), (x+1, z+1), (x, z+1),
   (x−1, z+1), (x−1, z), (x−1, z−1), (x, z−1), (x+1, z−1). Ring r starts at (x+r, z−r+1) and ends at
   (x+r, z−r).
5. Return the iterator's ring counter at the first cell where `terrain.CellHasWater(cx, cz)` (terrain slot 34,
   pop 8) is true. If there is none, return −1.

**Clipping** (QUESTIONS PL1, proven by an exact model of `0x0043FE10`, `0x00440850`, `0x00440420` and
`0x0043FE80`, checked over every anchor of 16 × 16 and 64 × 64 maps for reaches 0, 1, 2, 3, 5, 7, 20 and
256):

- Out-of-box cells are skipped **individually**. A leg that runs past the box jumps to its endpoint, and a leg
  lying wholly outside is skipped, but the ring counter still advances. The walk returns **every box cell
  exactly once** and never an outside cell. `CellHasWater` is never called outside the box.
- The ring counter equals the Chebyshev distance `max(|cx − x|, |cz − z|)` for every cell returned, including
  rings cut by the box. Rings come in increasing order, so the result is the **minimum Chebyshev distance** from
  `(x, z)` to a water cell inside the box, or −1.
- The walk ends when the spiral has passed all four box edges, after ring `max(x − x0, x1 − x, z − z0,
  z1 − z)`, which is at most `reach`. There is no other radius limit.

Types 0 and 1 use the same test: the code does not tell fresh from salt water.

### 7.3 Producer ageing (`AgeWaterProducer`, state 8 after a month) *[compat]*

This is the pollution simulator's §4.9.3 rule, with two differences:

- usage := `int32 used(0x08D9763D) / int32 production` (stored as `f32`) when **both are > 0**, otherwise 0.
  The divisor is the producer's **current production**, not a max-capacity property.
- the funding input of curve `0x0911E2E8` is the water department's funding fraction.

The rule itself:

- `condition += c0911E2E9(usage) × c0911E2E8(f) × k(0xEA1CF220, default 0)`, stored in `0xC910B4CF`.
- condition ≥ `t2` (`0xEA1CF222`, default 99) → `lot.SetCondition(4)`.
- else condition ≥ `t1` (`0xEA1CF221`, default 99) → `lot.SetCondition(2)`.

Lot slot 36. The lot is found with `city.PositionToCell` (slot 95) and `lotManager.GetLot(x, z, false)`.

## 8. Tuning *[compat]*

Exemplar: type `0x6534284A`, group `0xE7E2C2DB`, instance `0xC911E35B` ("Utilities", re/tuning/simulators.md).
It is fetched through the resource manager (slot 3, IID `0xA52160F5`) and released after reading.

`ReadSimulatorTunableValues`:

1. Always first, set the code defaults (built with `0x005F4480`):
   - `0x0911E2E8` := (0,1)(1,1);
   - `0x0911E2E9` := (0,1)(1,1);
   - `0xCA7138C4` := (0,−0.02)(120,0.01).
2. If the exemplar is missing, stop. The scalars keep their current values, and curve `0x6A410BF2` is untouched
   (empty on a fresh object).
3. Otherwise read the scalars from the exemplar's property holder (exemplar slot 7), then the curves with the
   key-based loader `0x005F4700`.

| Property | Meaning (name from the exemplar) | Type / getter | Code default | Shipped |
|---|---|---|---|---|
| `0x68F5436A` | water pipe effect radius (rings) | uint32 (`0x005FD480`) | 7 | 7 |
| `0x68F5436B` | fresh water pump reach (cells) | uint32 | 2 | 256 |
| `0x68F5436C` | salt water pump reach (cells) | uint32 | 1 | 256 |
| `0x68F5436D` | fresh water distance loss (% of capacity **kept**) | uint32 | 20 | 100 |
| `0xAA161F81` | max acceptable pumped water pollution | uint32 | 100 | 512 |
| `0x8A235C56` | pipe burst probability | float (`0x005FD4F0`) | 0.3 | 0.03 |
| `0xEA4025A9` | water department max funding | **SC4Percentage** getter `0x005FDAC0` (value × the `float32` 0.01f, `fmul dword [0x00A823A4]`, rounded to 24 bits) | 1.2 (`0x3F99999A`) | 120 → **1.1999999** (`0x3F999999`) *[compat]*; the budget clamps the department's funding to it (budget §7.1), so 120 % funding is stored as 1.1999999 |
| `0x8A84778F` | monthly cost per pipe | float | 0 | 0.1 |

Notes:

- A missing scalar property keeps the current value.
- "Distance loss" is applied as `cap × value / 100`. The shipped 100 means no loss, and the code default 20
  keeps 20 % *[compat]*.

| Curve | Use | Pattern (foundations §4.1) | Code default | Shipped |
|---|---|---|---|---|
| `0x6A410BF2` | funding % → efficiency % (§7.2) | **keep**, then **default if empty**: after the load, a curve with 0 points becomes (0,0)(100,100). With no exemplar it stays as it was (empty on a fresh object → evaluates to 0, foundations §4.3) | (0,0)(100,100) | (0,0)(50,30)(90,80)(100,100)(120,110) |
| `0x0911E2E8` | funding fraction → decay multiplier (§7.3) | **default** (step 1). After the load, an empty curve becomes (0,0)(100,100), which only happens when the property exists with no points | (0,1)(1,1) | (0,1.1)(1,1)(1.2,0.9) |
| `0x0911E2E9` | usage ratio → decay multiplier (§7.3) | **default**, then the same "empty → (0,0)(100,100)" rule | (0,1)(1,1) | (0,0.1)(0.25,0.2)(0.5,0.45)(0.75,0.85)(0.9,1.25)(1,1.55)(1.1,2) |
| `0xCA7138C4` | funding % → pipe decay (−) / repair (+) per month (§7.1) | **default** | (0,−0.02)(120,0.01) | (0,−0.01)(20,−0.005)(50,0)(100,0.02)(110,0.05) |

*[choice]* FreeCity4 evaluates an empty `0x6A410BF2` as 0, so every producer produces 0 when the exemplar is
missing.

**Building properties read** (per occupant; the names marked *[inferred]* are not in our tables):

| Property | Type | Use |
|---|---|---|
| `0xC8ED2D84` | sint32 / uint32 | water consumption: demand (sint32), flood (uint32 sum) |
| `0x88ED3303` | uint32 | water production capacity (presence makes a producer) |
| `0x48F23A7E` | uint8 | water source type *[inferred]* (§7.2) |
| `0x029244DB` | any | preview occupant, existence only (foundations §5) |
| `0xC910B4CF` | float | condition (aging accumulator, shared with pollution.md §7.2) |
| `0x8A1E07EE` | curve | condition → capacity multiplier |
| `0x0911E2E7` | curve | condition → maintenance-cost multiplier |
| `0x2A58D350` | sint64 | base maintenance cost (written by §5.3) |
| `0xEA1CF220/221/222` | float | aging rate, thresholds 1 and 2 |
| `0x08D9763D` | uint32 | water used (written in §6.3) |
| `0xEA1E3C53` | float | water cost per unit used *[inferred]* |

**Ordinance**: `GetCumulativeFactor(0xA8F4EB0C, 0)` multiplies building consumption (§6.1).

## 9. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x0990C075`) *[compat]*

**`Write`** (`0x006DB090`, pop 4, AL = 1). Stream methods: Uint8, Uint16, Sint32, Uint32, Sint64 (foundations
§2).

| # | Field | Type |
|---|---|---|
| 1 | version = **4** | u16 |
| 2 | working grid | ref (foundations §2.7; the grid is saved as its own object, simgrid §5) |
| 3 | published grid | ref |
| 4 | countX | u32 |
| 5 | countZ | u32 |
| 6 | flag map, `countX·countZ` bytes, in index order `z·countX + x` | u8 × n |
| 7 | passCounter | s32 |
| 8 | averageWaterPollution | u32 |
| 9 | totalWaterCost | s64 |
| 10 | producer count P | u32 |
| 11 | numPolluted | u32 |
| 12 | P producer records, in list order | below |
| 13 | the current producer's record, or an all-zero record (null occupant) when there is none. It is non-null only in a save taken while state 4 is pending (§6.2) | below |
| 14 | scanning iterator: state (0 fresh, 1 running, 2 done) u8; x, z s32; step u32; x0, z0, x1, z1 s32. From the end of state 7 until state 2 of the next pass (so in every save taken between passes): 2, 0, countZ, 1, 0, 0, countX−1, countZ−1 (§6.6) | 29 bytes |
| 15 | demand | s64 |
| 16 | numPipes, numDistressed, numBurst | u32 × 3 |
| 17 | numPipes pipe refs, in list order (the pipe's `cIGZSerializable`) | ref × numPipes |
| 18 | department | ref |
| 19 | last month produced, consumed, imported, exported | s64 × 4 |

A **producer record** is: seedX, seedZ, x1, z1, x2, z2 (s32 × 6), maxCapacity (u32), production (u32), occupant
(ref). That is 36 bytes for a null ref and 40 bytes otherwise.

**`Read`** (`0x006DB620`, pop 4):

- version ≠ 4 → **AL = 0** immediately.
- Grid refs are read as IID `0xC9BD939A`, producer occupants as `0x26BB767E`, pipes as `0x49C05B9F` and the
  department as `0x223E2B67`.
- The flag map and the bitmap are allocated if absent; the bitmap side is max(countX, countZ).
- Producers are appended in file order. efficiency := 0, polluted := false.
- The current producer is matched by occupant against the list (first match in list order); the record read
  for it is then released. A saved all-zero record (null occupant) matches a producer whose own occupant ref
  read as null *[quirk]*. This only matters for a damaged save, and the match is cleared by state 2 anyway
  (§6.2).
- A new scanning iterator is created with the saved fields, rectangle and step included. `Init` does not
  change them on a loaded city (§4.1 step 4 is skipped).
- It sets grids-initialized := true and state := 0, and returns **AL = 1** whatever the stream state.

Not saved, and recomputed by the first pass after load:

- tuning (re-read by `Init`);
- state (Read sets 0, so a pass starts immediately; recompute is false);
- the pass totals and building counts;
- producer efficiency and polluted;
- pipe condition (0 after load *[quirk]*);
- the enabled flag (it needs ConnectionsReady again).

Size example: countX = countZ = 64, 2 producers, no current producer, 3 pipes, all refs non-null:
2 + 16 + 8 + 4096 + 4 + 4 + 8 + 4 + 4 + 80 + 36 + 29 + 8 + 12 + 24 + 8 + 32 = **4375 bytes**.

## 10. Interface slot tables *[compat]*

### 10.1 `cISC4PlumbingSimulator` (vtable `0x00AB2228`, 29 interface slots + destructor)

There are no overloads, and the SDK order and argument sizes are correct. **Return types differ** in the rows
marked SDK.

| Slot | Off | Method | Pop | Return | Behaviour | SDK |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` (`0x006DA3C0`) | 8 | **AL only** (upper EAX garbage on false) | §1 | |
| 1 | `+0x04` | `AddRef` (`0x004F7BA0`) | 0 | EAX | | |
| 2 | `+0x08` | `Release` (`0x006DA610`) | 0 | EAX | §1 | |
| 3 | `+0x0C` | `Init` | 0 | AL = 1 | §4.1 | |
| 4 | `+0x10` | `Shutdown` | 0 | AL = 1 | §4.2 | |
| 5 | `+0x14` | `uint8 GetSupplyValue(x, z)` (`0x006DA440`) | 8 | AL | tail jump to the published grid's `GetTractValue` (grid slot 22): `data[x][z]`, no bounds check | |
| 6 | `+0x18` | `GetSupplyValueMapPtr(cISC4SimGrid<uint8>*& g)` (`0x006DA450`) | 4 | **none meaningful** (EAX = the pointer) | `g :=` the published grid, no AddRef | **SDK `bool`**: replacement returns true |
| 7 | `+0x1C` | `GetWateredGrid()` (`0x00A6CAB0`, shared getter) | 0 | EAX | the published grid, no AddRef | |
| 8 | `+0x20` | `AreWateredBuildingsInCell(x, z)` (`0x006DAD50`) | 8 | **full EAX** 0/1 | `(raw data[x][z] & 3) == 3` (GetGridData, no bounds check) | |
| 9 | `+0x24` | `IsBuildingWatered(occ)` (`0x006DA6A0`) | 4 | EAX (from slot 8) | `occ.GetPosition()` → published grid `PositionToTract` → **this object's own slot 8, called virtually** | |
| 10 | `+0x28` | `uint32 GetWaterProduction(occ)` (`0x006DAD80`) | 4 | EAX | the production of the producer record with that occupant; 0 if none | |
| 11 | `+0x2C` | `IsCellWatered(x, z)` (`0x006DA460`) | 8 | **AL only** | `published.GetCellValue(x, z) & 1` | |
| 12 | `+0x30` | `GetNumWaterSources()` (`0x006DC710`) | 0 | EAX | producer count | |
| 13 | `+0x34` | `GetNumPollutedWaterSources()` (`0x00423850`) | 0 | EAX | numPolluted | |
| 14 | `+0x38` | `GetWaterProductionCapacity()` (`0x00423700`) | 0 | **EAX only** | low 32 bits of last month produced | **SDK `int64`**: wrong; EDX garbage |
| 15 | `+0x3C` | `GetWaterConsumed()` (`0x007C1910`) | 0 | **EAX only** | last consumed (low 32) | **SDK `int64`** |
| 16 | `+0x40` | `GetWaterImported()` (`0x007E56F0`) | 0 | **EAX only** | last imported (low 32) | **SDK `int64`** |
| 17 | `+0x44` | `GetWaterExported()` (`0x006DC680`) | 0 | **EAX only** | last exported (low 32) | **SDK `int64`** |
| 18 | `+0x48` | `GetNumPipes()` (`0x006DC690`) | 0 | EAX | numPipes | |
| 19 | `+0x4C` | `GetNumDistressedPipes()` (`0x006DC6A0`) | 0 | EAX | | |
| 20 | `+0x50` | `GetNumBurstPipes()` (`0x007FF120`) | 0 | EAX | | |
| 21 | `+0x54` | `GetNumWateredBuildings()` (`0x006DC6B0`) | 0 | EAX | last watered building **cells** | |
| 22 | `+0x58` | `GetNumUnwateredBuildings()` (`0x006DC6C0`) | 0 | EAX | last building cells − watered | |
| 23 | `+0x5C` | `GetWaterDemand()` (`0x00798370`) | 0 | **EAX only** | low 32 bits of demand | **SDK `int64`** |
| 24 | `+0x60` | `GetWaterProducerInfo(occ, u32* cur, u32* max, SC4Percentage* eff, bool* polluted, u32* used)` (`0x006DADB0`) | 24 | **AL only** | find the record. If missing: AL = 0. Each non-null pointer receives production, maxCapacity, efficiency, polluted, and property `0x08D9763D` (0 if missing). AL = 1 | |
| 25 | `+0x64` | `GetWaterProducerConditionPercent(occ, SC4Percentage&)` (`0x006DBFC0`) | 8 | **AL only** | condition absent or curve `0x8A1E07EE` absent → AL = 0, output untouched. Else output := curve(condition), AL = 1 | |
| 26 | `+0x68` | `GetOccupantMonthlyUpkeepCost(occ, int64&)` (`0x006DA640`) | 8 | **AL only** | budget item for purpose `0xCA58C9D5` → output := item.cost, AL = 1; else AL = 0 | |
| 27 | `+0x6C` | `FindWaterBuildings(occ** lowestFunded, SC4Percentage& funding, occ** worstCondition, long& pct)` (`0x006DAB60`) | 16 | **none meaningful** | below | **SDK `bool`**: replacement returns true |
| 28 | `+0x70` | `GetWaterProducers(ilist<cISC4Occupant*>&)` (`0x006DC7D0`) | 4 | **none meaningful** | appends every producer occupant in list order (AddRef) | **SDK `bool`**: replacement returns true |
| 29 | `+0x74` | scalar deleting destructor (class) | 4 | — | | |

**`FindWaterBuildings`** (used by the advisors):

- **lowestFunded**: among producers that the department funds locally (`dept.IsLocallyFundedObjectInDepartment`,
  department slot 28), the one with the lowest local funding (strict `<`, starting at 2.0). funding := that value.
- **worstCondition**: the producer with the highest `condition / max(t1, t2)` (strict `>`, starting at 0).
  `pct := trunc(ratio × 100)`.
- Outputs are written only when found and when the pointer is non-null. Occupants are AddRef'd.
- `GetWaterProducers` allocates list nodes through the game's list-node allocator (framework service
  `0x3AE4BEA3 / 0x3AE4BEAB`).

### 10.2 `cIGZSerializable` (vtable `0x00AB21FC`, 6 slots)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0 | QueryInterface thunk (`0x006DC6E0`, −8) | 8 | AL |
| 1 | AddRef thunk (`0x0068D250`) | 0 | EAX |
| 2 | Release thunk (`0x006DC700`) | 0 | EAX |
| 3 | `Write(cIGZOStream&)` (`0x006DB090`) | 4 | AL = 1 |
| 4 | `Read(cIGZIStream&)` (`0x006DB620`) | 4 | AL (0 only for a wrong version) |
| 5 | `GetGZCLSID()` (`0x006DA3B0`) | 0 | EAX `0x0990C075` |

### 10.3 `cIGZMessageTarget2` (vtable `0x00AB2214`, 4 slots)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0 | QueryInterface thunk (`0x006DC6D0`, −4) | 8 | AL |
| 1 | AddRef thunk (`0x0068D230`) | 0 | EAX |
| 2 | Release thunk (`0x006DC6F0`) | 0 | EAX |
| 3 | `DoMessage(cIGZMessage2*)` (`0x006DF570`) | 4 | AL (§5.1) |

### 10.4 SDK errors (checked addresses)

| SDK declaration | Game | Checked |
|---|---|---|
| `int64_t GetWaterProductionCapacity()` | returns 32 bits in EAX; EDX is not set | `0x00423700` |
| `int64_t GetWaterConsumed()` | same | `0x007C1910` |
| `int64_t GetWaterImported()` | same | `0x007E56F0` |
| `int64_t GetWaterExported()` | same | `0x006DC680` |
| `int64_t GetWaterDemand()` | same | `0x00798370` |
| `bool GetSupplyValueMapPtr(...)` | no meaningful return (EAX = the grid pointer) | `0x006DA450` |
| `bool FindWaterBuildings(...)` | no meaningful return (EAX garbage) | `0x006DAB60` |
| `bool GetWaterProducers(...)` | no meaningful return (EAX garbage) | `0x006DC7D0` |

For a plug-in calling the original, the low 32 bits of the five `int64` getters are valid and the high 32 bits
are garbage. A replacement returns the 64-bit value; callers in the game read only EAX.

Bool returns: slots 0, 11, 24, 25 and 26 set **AL only**. Slot 8 sets **full EAX**. Slot 9 forwards slot 8's
EAX.

## 11. Dependencies (methods the original calls) *[compat]*

Slot numbers are compiled slots; pops and return channels come from `abi_extract --check`.

| Interface (vtable) | Slot | Method | Pop | Ret |
|---|---|---|---|---|
| `cISC4City` | 34, 62, 66 | `GetOccupantManager`, `GetNeighborsSimulator`, `GetPollutionSimulator` | 0 | EAX |
| | 72 | `GetDisasterLayer` | 0 | EAX |
| | 84 | `GetTerrain` | 0 | EAX |
| | 93, 94 | `CellCountX`, `CellCountZ` | 0 | EAX |
| | 95 | `PositionToCell` | 16 | — |
| `cISC4Simulator` | 19 / 20 | `AddAgent(target, level, name, flags)` / `RemoveAgent(target)` | 16 / 4 | — |
| `cIGZMessageServer2` | 4 / 5 / 6 | `MessagePost` / `AddNotification` / `RemoveNotification` | 8 | AL |
| `cISC4BudgetSimulator` | 25, 28, 51, 52, 53, 57 | `CreateDepartmentBudget`, `GetDepartmentBudget`, `GetBudgetItemForPurpose`, `ChangeBudgetItemLine`, `ChangeBudgetItemCost`, `GetLineItemFromBudgetItem` | 8, 4, 12, 12, 16, 4 | EAX |
| `cISC4DepartmentBudget` | 5, 10, 13, 17, 18, 19, 20, 28 | `SetDepartmentName`, `GetFundingPercentage` (pointer), `SetMaxAllowedFundingPercentage`, `CreateLineItem`, `CreateLineItemForBuildingType`, `RemoveLineItem`, `GetLineItem`, `IsLocallyFundedObjectInDepartment` | 8, 0, 4, 8, 8, 4, 4, 8 | EAX |
| `cISC4LineItem` | 7, 8, 9, 12, 13, 18 | `SetName`, `GetSecondaryInfoField` (EDX:EAX), `SetSecondaryInfoField(int64)`, `SetFullExpenses(int64)`, `AddToFullExpenses(int64)`, `SetDisplayFlag` | 8, 0, 8, 8, 8, 8 | |
| `cISC4OrdinanceSimulator` | 25 | `GetCumulativeFactor(id, 0)` | 8 | ST0 float |
| `cISC4PollutionSimulator` (`0x00AB2630`) | 8 | `GetWaterValue(x, z, int32&)` | 12 | none (pollution.md §8.1) |
| `cISC4NeighborsSimulator` (`0x00AB1EE0`) | 13, 15, 17, 18, 30, 31 | `FindConnectionOnOtherSide(conn)`, `FindUnvisitedConnection(type, city)`, `GetConnectionsOfType`, `GetConnectionsOfTypeToCity`, `FindDeals(type, list)`, `MarkDealsDirty` | 4, 8, 8, 12, 8, 0 | |
| `cISC4NeighborConnection` (`0x00AB2034`) | 6, 12, 13, 14 | `GetConnectionCell(&x,&z)`, `SetVisitedThisCycle`, `GetIsActive`, `SetIsActive` | 8, 4, 0, 4 | AL for 13 |
| `cISC4NeighborDeal` (`0x00AB1DD8`) | 7, 8, 10, 20, 21, 37 | `GetCityOnOtherSideOfConnection`, `GetType`, `GetAmount`, `GetAmountUsed`, `SetAmountUsed`, `SetFlags(f, on)` | 0, 0, 0, 0, 4, 8 | EAX |
| `cISC4NetworkOccupant` (`0x00AA8080`, 81 slots) | 8, 10, 11, 12, 47, 59 | `HasNetworkFlag`, `SetNetworkFlag`, `ClearNetworkFlag`, `GetNetworkFlag`, `ConnectsToNeighbor(dir, type)`, `GetOccupiedCell(&x,&z)` | 4, 4, 4, 0, 8, 8 | |
| `cSC4PipeOccupant` (non-virtual) | — | `SetWateredState(bool)` `0x00648540` | 4 | — |
| `cISC4Occupant` | 6, 7, 8, 10, 11 | `AsPropertyHolder`, `GetType`, `GetPosition`, `GetBoundingBox`, `GetBoundingCityCells` | 0, 0, 4, 8, 4 | |
| `cISC4BuildingOccupant` (QI `0x87DDA3A9`) | 4 | `GetBuildingType` | 0 | EAX |
| `cISC4OccupantManager` | 41 | `GetOccupantsByStandardCityCells(list, &c1, &c2, type, −1)` (the typed overload) | 20 | |
| `cISC4LotManager` | 6, 14 | `GetLot(x, z, bool)`, `GetOccupantLot(occ)` | 12, 4 | EAX |
| `cISC4Lot` | 14, 36, 65, 66 | `GetBoundingRect`, `SetCondition`, `GetNeedsPower`, `GetHasPower` | 4, 4, 0, 0 | |
| `cISTETerrain` | 7, 8, 34, 69 | cell counts X/Z, `CellHasWater(x, z)`, `RedisplayTerrain(0, 0, &min, &max, 0)` | 0, 0, 8, 20 | |
| `cISC4DisasterLayer` | 7 | `GetDisasterManager(0x4A1332E8)` | 4 | EAX |
| `cISC4DisasterManager` pipe burst (`0x00A9E66C`, 12 slots) | 6 | `AttemptDisasterStart(x, z)` (`0x0057D180`) | 8 | |
| `cISC4HistoryWarehouse` | 12 | `RecordHistoryData(id, u32)` | 8 | |
| `cISCPropertyHolder` | 3, 9 | `HasProperty`, `GetProperty` | 4 | |
| `cIGZVariant` | 64, 72 | uint8 / uint32 value getters (reversed overload order, foundations §16) | | |
| `cISC4SimGrid<uint8>` | simgrid §4 | `Init`, `SetTractSize`, `SetInstanceID`, `SetTractValues`, `GetCellValue`, `PositionToTract`, `GetTractValue`, `GetGridData` (24, 25) | | |
| `cRZRandom` | — | `RandomSint32RangeGaussianFast` (`0x009136DF`) | 8 | EAX |

Property helpers (`cSCPropertyHelp`):

- getters: `0x005FD450` sint32, `0x005FD480` uint32, `0x005FD4F0` float, `0x005FDAC0` SC4Percentage;
- setters: `0x005FE3C0` uint32, `0x005FE450` sint64, `0x005FE4F0` float.

**Order relative to other simulators** (foundations §13):

- The month agent only sets flags and applies pipe funding. It reads the department funding. The budget
  registers first, so this is the funding the player set before this month's settlement.
- The pass reads water pollution (pollution's daily visits), lot power (power simulator) and the deals' amounts
  (neighbours).
- It writes `SetAmountUsed`, deal flags and connection active/visited bits, which the neighbours simulator reads
  at its next `simulate_deals`. MarkDealsDirty requests that run.
- Same-day reads by others: flammability's `IsCellWatered` (flammability.md §5) sees the grid published by the
  last completed pass.

## 12. Consumers and replaceability

### 12.1 Consumers (through the global set from city slot 64 at city init, or the city getter)

| Caller | Uses |
|---|---|
| `cSTETerrainView3D::DrawPlumbingInfo` | `GetWateredGrid` + `GetCellValue` (bits 8, 1, 2), `AreWateredBuildingsInCell`, `IsCellWatered` |
| `cSC4WinMapView::Update` (water map) | `GetWateredGrid` → **`GetGridData` raw** (grid slot 25) → `DisplayStandardUint8Grid` |
| `cSC4WinMapView::UpdateHighlights` | `GetWaterProducers` |
| `cSC4WinMapView::HookUpdateMessages` | subscribes to `0x6946DC34` (§6.7) |
| neighbours `UpdateCommodityStats` | `GetWaterProductionCapacity`, `GetWaterDemand` (EAX only; neighbors.md §8.2) |
| query tools | slots 9, 11, 18, 24 (null pointers allowed), 25, 26 |
| advisors, MySim | counts, totals, `FindWaterBuildings` |
| flammability | `IsCellWatered` (flammability.md §5, §6.5) |
| lot developer, tract developer | hold the pointer at `Init` |

### 12.2 Replaceability findings

The class is created through the GZCOM class registry (CLSID `0x0990C075`), and every external caller uses the
vtable. A GZCOM replacement is **possible**. It must preserve the following:

1. **Raw grid reads** (simgrid §6.4 applies): the published grid is read raw.
   - `cSC4WinMapView::Update` calls `GetGridData` on it and walks the `vector<vector<uint8>>` directly.
   - The original's own `AreWateredBuildingsInCell` also reads it raw.

   A replacement must hand out a real `cSC4SimGrid<uint8>` (CLSID `0x49B9E602`) created **through GZCOM**, per
   §6.4. The original constructs its grids inline, so a replacement must not copy that. Dimensions: city cells,
   tract size 1.
2. **Grid identity**: the map view and the terrain view fetch `GetWateredGrid` on each update, so swapping
   grids is safe for them. Any code that caches the pointer sees a stale grid after a swap. A replacement may
   keep one fixed published grid *[choice]*: that is safer and invisible through the interface. The save
   format, however, holds both refs (§9).
3. **Non-virtual call into another class**: the pass calls `cSC4PipeOccupant::SetWateredState` (`0x00648540`)
   directly. This is the only caller, and it drives the pipe's watered display through pipe vtable slot 100. A
   replacement cannot call it through any interface. It must either call that address on the game's pipe
   objects (a plug-in shim) or accept that pipes no longer show their watered state. **Blocker for a pure
   GZCOM replacement** *[compat]*.
4. **Pointer adjustment**: the original turns a network occupant into an occupant with `+4`, not with QI. A
   replacement should QI for `cISC4Occupant` (`0x87254E2F`) instead.
5. **Globals**: the game caches the simulator pointer (city init) and the ordinance simulator in globals. The
   replacement is reached through `GetPlumbingSimulator`.
6. **Messages**: keep posting `0x6946DC34` (map view refresh) and `0xEA418D8F`/8. Keep handling `0xEA1F0DCD`
   from the pipe-burst disaster.
7. **Budget**: keep department `0xAA4014B4` with line item 2 (pipes), and the building budget-item scheme with
   purpose `0xCA58C9D5`, so that the budget window and saves agree.
8. No code outside the class reads its memory, constructs it, or calls its non-virtual members (xrefs to
   `0x006DA3B0`–`0x006DF750` come only from its vtables and its own code).

## 13. Quirks (reproduced) and open questions

Quirks *[compat]*:

- `0xEA1F0DCD` clears the pipe bit as well as the connections (§5.1).
- Removing one consumer clears the consumer bit for the whole cell (§5.3).
- A producer without a lot leaks its record and occupant reference (§5.3).
- Pipe insertion ORs connection bits without clearing old ones (§5.4).
- Seeding: shared seed cells and multiple import connections re-flood a network (§6.2).
- Import connectivity is tested against the ring-flood area; export connectivity against the pipes only (§6.3).
- A water deal status change (data1 = 0) aborts the current pass (§5.1). The simulator's own `SetFlags` can
  trigger this once per change.
- Pipe condition is not saved (§7.1).
- Ageing divides by current production, not capacity (§7.3).
- `RandomSint32RangeGaussianFast` draws 4 values even when the range is empty (§7.1).
- The published grid alternates between two grid objects (§2).
- An export deal is flagged NoConnection by any network that cannot reach it, even when another network
  delivers it (§6.3).
- `Init` after `Shutdown` leaves the grids, flag map, bitmap and iterator null, and the original crashes on
  first use (§4.2). FreeCity4 rebuilds them *[choice]*.
- The scanning iterator is left done at (0, countZ) after state 7, and saves taken between passes hold that
  position (§6.6).

Open questions:

1. The meaning of the unused counter recorded as history `0xCA14EC19`. It is zeroed in state 5 and never
   incremented, so it is always 0.
2. What pipe vtable slot 100 does (called by `SetWateredState` on change). Probably a model or texture swap
   *[inferred]*.
3. *(resolved, PL1)* The spiral's ring number at clipped box edges is the Chebyshev distance, and out-of-box
   cells are skipped one by one (§7.2).
4. The name of message `0x6946DC34` (a "water map changed" notification *[inferred]*). The process-wide flag
   that `Init` sets is unexplained.
5. *(resolved)* The game runs the x87 at 24-bit precision (foundations §18). Each operation rounds like one
   `float32` operation. Example: `trunc(100 × 0.01f)` is **1** (the exact product 0.99999997… rounds to 1.0 at 24
   bits), whereas extended precision would give 0. Pipe-cost ties (§5.4) follow the same rule.
6. Whether a single import deal seeding several networks (one per connection) is intended.
7. *(resolved, PL2)* A second `Init` after `Shutdown` re-runs the first-`Init` work but leaves the grids null,
   and the original crashes (§4.2).
8. *(resolved, PL3)* State 3 never sees a current producer: its non-null branch is dead (§6.2).
9. *(resolved, PL4)* State 7 ends with the iterator done at (0, countZ) (§6.6).
10. *(resolved, PL7)* An unreachable export deal gets `0x08` (state 4) and `0x20` (state 8), so `0x28` (§6.7).

## 14. Conformance tests (must exist in `engine/conformance`)

Unless stated, tuning is the shipped exemplar, funding is 100 % and the factor is 1.0.

1. **Defaults** (no exemplar):
   - radius 7, reaches 2 / 1, loss 20, max pumped pollution 100, burst probability 0.3, max funding 1.2, cost per
     pipe 0;
   - `0x0911E2E8` = `0x0911E2E9` = (0,1)(1,1); `0xCA7138C4` = (0,−0.02)(120,0.01);
   - `0x6A410BF2` empty.
2. **Shipped tuning**: radius 7, reaches 256 / 256, loss 100, max 512, burst 0.03, max funding 1.1999999 (`0x3F999999`, from 120 × 0.01f),
   cost per pipe 0.1. `0x6A410BF2(90) = 80`.
3. **Pipe flags**:
   - insert a pipe at (10, 20) connecting in directions 0 and 2 → flags 0x15;
   - message `0xEA1F0DCD` (10, 20) → 0x00;
   - a cell with 0x55 and the same message → 0x40.
4. **Pipe cost**, cost per pipe 0.1: 12 pipes → `SetFullExpenses(1)`; 18 pipes → 2; 0 pipes → 0.
5. **Pump capacity**, fresh pump (type 0), maxCapacity 3000:
   - water at distance 1 → production 3000;
   - funding 50 % → curve 30 → `eff0 = f32(30 × 0.01f) = 0.29999998` → production **899**, not 900 (float
     rounding *[compat]*);
   - loss 20 and reach 2 (other tuning shipped), water at distance 2 → production 600; no water within the
     reach → production 0;
   - no exemplar at all → the funding curve is empty → production 0 (FreeCity4 choice, §8).
6. **Pollution shutdown**:
   - water value 600 (> 512) → production 0, polluted true, numPolluted 1;
   - water value 512 → not polluted;
   - averageWaterPollution of two producers with 600 and 100 → 350.
7. **Power**: a producer whose lot needs power and has none → production 0, efficiency 0, polluted unchanged.
8. **Condition curve**: `0x8A1E07EE` = (0,1)(100,0.5), condition 50 → eff 0.75 → production 2250.
9. **Basic flood**:
   - setup: pump at (5,5) with production 1000; pipes at x = 5..9, z = 5, connected along x; a building anchored
     at (12,5) with consumption 300;
   - expected: the building's cell is watered (ring 3); pass consumed 300; water used of the pump
     `0x08D9763D` = 300; history produced 1000, consumed 300.
10. **Shortage**: the same with consumption 1200 → consumed 1000, the building cell **not** watered, and the
    flood stops there (cells of ring 4 and later are not watered), water used 1000.
11. **Ring geometry**, radius 7, single pipe cell network at (50,50) with its own pump:
    - (53,53) watered (ring 3, diagonals for rings 0–2);
    - (54,54) watered (ring 5);
    - (55,55) not watered (ring 7);
    - (50,56) watered (ring 6); (50,57) not.
12. **Export**: pump 1000, a consumer of 100 within the radius, and an export deal of 400 (used 0) whose
    connection cell is a pipe cell of the network:
    - exported 400; deal used 400; ratio (100 + 400) / 1000 = 0.5 → water used 500.
    - Connection not on the network → flag 0x08 set in state 4, exported 0, water used 100. At state 8,
      used 0 < 400 → flag 0x20 set too: the deal's flags gain **0x28** (starting without Grace).
    - Two networks, each with pump 1000: the first seeded cannot reach the connection and the second can →
      exported 400 and used 400, flag 0x08 set (by the first network), flag 0x20 not set.
    - No producer and no import (no network) → no 0x08; flag 0x20 set at state 8.
    - Export amount 1500: take 1000, used 1000, supply 0. The ring flood still runs (exportTaken > 0), but the
      first consumer stops it (consumed := 0). Ratio 1000 / 1000 → water used 1000. At state 8, used < amount →
      flag 0x20 set.
13. **Import**:
    - setup: no producers; an import deal of 500, used 0; an active connection at (0,10); pipes from (0,10) to
      (5,10); a consumer of 200 at (7,10);
    - expected: watered; imported 200; deal used 200; produced 0.
14. **Pipe funding**, curve `0xCA7138C4` replaced by (0,−0.25)(120,0.25), burst probability 0.5, 100 pipes,
    funding 0, generator seeded with 1:
    - cond 0.25 → target 25 distressed;
    - bursts = `RandomSint32RangeGaussianFast(0, 25)` = **10** (raw draws 16838, 1507104382, 3204589441,
      2509948011);
    - the next month at funding 0: cond 0.5, target 50, `hi = 50` → draws give 0 + ((avg × 50) >> 32) with the
      next four raws (2043738443, 1622775291, 117692898, 1481547003) → **15**;
    - a month with target unchanged and `hi = 0` still consumes 4 raws.
15. **GaussianFast**: seed 1, `(0, 25)` → 10, then 7 (draws 5–8). `(0, 0)` → 0.
16. **Burst bookkeeping**: removing a pipe that has flags 0x10000 and 0x20000 → numPipes, numDistressed and
    numBurst each decrease by 1.
17. **Deal message**: during state 4, message `0xEA418D8F` with data1 = 0 for an import deal → state 9 and
    recompute set. The next `DoIdle` → state 0, passCounter + 1. data1 = 8 → no change.
18. **Disabled**: before ConnectionsReady, `DoIdle` returns true and the state does not change. The month
    handler still runs (pipe funding happens).
19. **Grid values**:
    - a watered building lot of 2×2 cells → value 3 on all four cells (0x0B where a pipe lies);
    - a producer-only lot (no consumer flag) → 6, or 7 if watered;
    - `AreWateredBuildingsInCell` true only for `(v & 3) == 3`;
    - `GetSupplyValue(x, z)` = v.
20. **Grid swap**: after one complete pass, `GetWateredGrid()` returns the grid created with instance
    `0x49D5BB8C`; after the next pass, `0x49D5BBA1`.
21. **NearestWater**: water at (x+2, z+1) → 2; only water at (x+3, z) with reach 2 → −1; type 2 → 0.
    Clipped edges, 64 × 64 terrain:
    - anchor (0, 0), reach 256, water only at (5, 3) → 5;
    - anchor (62, 10), reach 256, water only at (55, 10) → 7;
    - anchor (1, 1), reach 2, water only at (0, 3) → 2; water only at (4, 1) → −1 (outside the box
      [0, 3] × [0, 3]);
    - anchor (0, 0), reach 2: the cells are tested in the order (0,0); (1,0), (1,1), (0,1); (2,0), (2,1),
      (2,2), (1,2), (0,2). Exactly 9 `CellHasWater` calls when there is no water;
    - anchor (64, 10) (outside the terrain), water everywhere → −1; anchor (63, 63), reach 0, water there → 0.
22. **Ageing** (pollution §4.9.3 rule): production 1000, used 500, `k` 1, shipped curves, funding 1.0 →
    condition 0 → 0.45. With production 0 → usage 0 → condition += 0.1. Condition 99 with default thresholds →
    `SetCondition(4)`.
23. **Persistence**: the §9 size example writes 4375 bytes; write → read → write is byte-identical; version 3 →
    `Read` returns false.
24. **SDK return channels**: with last month produced = 2^32 + 5, `GetWaterProductionCapacity` returns EAX = 5.
    `IsCellWatered` sets only AL.
25. **Iterator after a pass** (PL4): 64 × 64 city, one complete pass → the iterator is done at (0, 64), step 1,
    rectangle [0, 63] × [0, 63]. A save written now holds field 14 = `02`, x 0, z 64, step 1, 0, 0, 63, 63.
    Write → read → write is byte-identical, and the next pass runs normally (state 2 resets it to (0, 0)).
26. **Current producer** (PL3): a save taken while state 4 is pending holds the seeding producer's record in
    field 13. After loading it, the first pass gives the same grid, totals and water-used values as a load of
    the same save with an all-zero field 13.
27. **Re-Init** (PL2, FreeCity4 *[choice]*): `Init`, `Shutdown`, `Init` on one object for a new 64 × 64 city →
    `GetWateredGrid()` is a fresh grid (instance `0x49D5BBA1`) with all values 0, the flag map is all 0, state 0,
    recompute true. Three agents and six subscriptions are registered once each (not twice). A pass then runs
    without a fault. `Init`, `Init` without `Shutdown` → the second `Init` changes nothing.
