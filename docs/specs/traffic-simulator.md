# cSC4TrafficSimulator: commute trips, traffic volume and the traffic maps (behaviour spec)

Status: M5 behaviour spec, verified on the Windows executable `bin/SimCity4.exe` (1.1.641, image base `0x00400000`).
Every address below is a Windows address. `docs/subsystems/traffic.md` and `re/symbols/traffic.tsv` (mostly Mac
build) were used as a starting point for names only; the differences found are listed in the final section of
§10.

Related specs:

* [traffic-network.md](traffic-network.md): the per-cell network model this class owns and keeps up to date (network
  info per cell and transit network, cell connection words, transit switches and their rules, tunnels, ferries and
  water routes, neighbour connections, subnetworks and the connectivity queries, the lot insertion point,
  `DoConnectionsChanged`). This spec only says **when** those structures are read or rebuilt.
* [pathfinder.md](pathfinder.md): `cSC4PathFinder`, the A\* search, the compressed route format (§4.15), the goal test
  with destination capacity (§4.10) and the class statics the search reads. This spec says how the simulator
  configures each search and what it does with the result.
* [foundations.md](foundations.md) §4 (response curves), §6 and §12 (random numbers), §18 (x87 precision);
  [simulator.md](simulator.md) §4–§5 (agents, levels, slicing); [simgrid.md](simgrid.md) (grids; §6.4 raw readers);
  [budget.md](budget.md) §7–§8 (departments and line items); [ordinance.md](ordinance.md) §6.3;
  [neighbors.md](neighbors.md) (neighbour connections; their edge records and pools are traffic-network.md §2.7);
  [growth-developer.md](growth-developer.md) (the commute layer reads the trip-length map);
  [docs/abi/allocators.md](../abi/allocators.md).

Notation. *[compat]* = observable behaviour a replacement must reproduce. *[quirk]* = surprising original behaviour
that is reproduced. *[choice]* = FreeCity4 decision where the original is undefined or non-deterministic.
*[inferred]* = not proven. `r24(e)` = `e` rounded to float32 (foundations §18: every x87 add, subtract, multiply and
divide on the simulation thread rounds to a 24-bit significand, also when an operand is a double or an integer).
`round(v)` = the `1.5·2^23` idiom: `r24(v + 12582912.0f)` (constant `0x00AB32D8`, a **dword**), bit pattern minus
`0x4B400000` = round half to even, exact for `|v| < 2^22`; outside that range the result is garbage *[compat]* (all
sites below stay inside it or clamp first, unless stated). `ftol(v)` = the runtime's `_ftol` (`0x009EEF04`):
truncation toward zero to **int64**, of which the callers keep the low 32 (or 64) bits.

---

## 1. Role, identity, creation, Init

The traffic simulator turns the city's population and jobs into commute and freight **trips**, routes them over the
transport networks with the pathfinder, accumulates the resulting **traffic volume** per cell and transit network,
and publishes the derived maps: congestion, air-polluting traffic, commercial traffic and commute **trip length**.
It also collects mass-transit fares and tolls, drives the roads and mass-transit departments' monthly costs,
records history, runs the accident check and the mass-transit strike, and owns the network model of
traffic-network.md.

| Fact | Value |
|---|---|
| Class ID | `0x6990C1AA` (city component) |
| Interfaces (QueryInterface `0x0070D7E0`) | `cIGZUnknown` (`1`) and `cISC4TrafficSimulator` (`0x68027F37`) → the object; `cIGZMessageTarget2` (`0x452294AA`) → object + 4; `cIGZSerializable` (`0xE4FDA3D4`) → object + 8. Any other IID: false. |
| Primary vtable | `0x00AB33D0`, 44 slots (§7) |
| Message-target vtable | `0x00AB33BC`; `DoMessage` `0x007241B0` (runs with `this` = object + 4) |
| Serializable vtable | `0x00AB33A4`: slot 3 `Write` `0x00712400`, slot 4 `Read` `0x00721990`, slot 5 `GetGZCLSID` (all with `this` = object + 8) |
| `GetSimulatorType` (slot 5) | returns `0x88027F0D` |
| City accessor | `cISC4City` slot 69 (`+0x114`) `GetTrafficSimulator` |

**Release** (slot 2, `0x0070E2F0`): above 1 it decrements and returns the new count; at 0 or 1 it calls `Shutdown`
(slot 4) and the scalar deleting destructor (slot 43, flag 1: free with allocator C `0x005E5620`) and returns 0.

**Init** (slot 3, `0x0071E7E0`), only when not yet initialised, in this order *[compat]*:

1. `ReadSimulatorTunableValues` (§4.14).
2. Registers its message target (object + 4) as a simulator agent at level 0 (one-shot), level 6 (month) and level
   4 (day), name `"cSC4TrafficSimulator"`, flags 2 (simulator slot `+0x4C`, simulator.md §4).
3. Subscribes to `0x99EF1142`, `0x99EF1143` (occupant inserted / removed), `0x26D31EC1` (post city init),
   `0xE8BDA2E7` (lot (de)activated), `0xC53D10AA` (tuning reload), `0x8A0D15DA`, `0xEA0D163E` (neighbour connection
   added / removed), `0x08DAC3E7` (budget change), `0xC2C09554` (`OrdinanceUpdated`).
4. Reads the city size `W × H` cells (city slots 93, 94) and creates the grids, each with `SetTractSize` and `SetID`:

| Grid | Element | Tract | ID | Saved | Meaning |
|---|---|---|---|---|---|
| air-polluting traffic | u8 | `tractSize` (code default 1, not read from tuning) | `0x89D5C3A2` | yes | slot 8 (§4.10) |
| peak volume | f32 | `tractSize` | `0x69D5C3AC` | yes | per-cell maximum volume (§4.9) |
| commercial traffic | u8 | **4** | `0xCABD2925` | yes | slot 9 (§4.10) |
| commercial traffic (work copy) | u8 | 4 | `0xCABD2926` | yes | built during the map pass, copied into the published grid |
| (unused) | u8 | 1 | `0x69D5C3B5` | no | created, never written *[inferred]* |
| trip length | u8 | 1 | `0x69D5C3BF` | yes | slot 13 (§4.6) |
| traffic volume, one per transit network `n` = 0…5 | f32 | 1 | `0x69D5C3E0 + n` | yes | §4.5 |
| (float) | f32 | 1 | `0x69D5C3FA` | yes | not written by the code in this spec *[inferred]* |
| congestion | u8 | 1 | `0x69D5C402` | yes | slot 10 (§4.10) |
| peak volume/capacity ratio | f32 | 1 | `0x69D5C40E` | yes | §4.9 |
| transit-switch mode-mask map (corrected: not a "wormhole" map; TN13) | u8 | 1 | `0x69D5C424` | yes | traffic-network.md §2.3 |
| transit-switch ID map | u32 | 1 | `0x69D5C42D` | yes | traffic-network.md |

   Grid element types are the framework's grid classes (u8 `0x49B9E602`, f32 `0x49B9E60A`, u32 `0x49B9E606`;
   simgrid.md). The destination bitmask map is a plain `(W+2) × (H+2)` byte array indexed `[x+1][z+1]` (pathfinder.md
   §4.1), zero-filled.
5. Allocates the cell connection array (traffic-network.md), the generator `cRZRandomSuper` seeded from the wall
   clock (§4.15), the scanning iterator over the city rect (simulator.md §5.5 slicing), and the department budgets:
   roads (`0xC921CEEA`) and mass transit (`0xC921CEEB`) with line items 1…6 (budget.md §7–§8).
6. Creates two pathfinders through its own slot 6 (`CreatePathFinder`): **pf** (outbound) and **pf2** (return trip).
7. Creates the lot iterator (lot manager slot 15), the traffic-source iterator (§4.2), its property holder, the
   ferry `cSC4WaterPathFinder` (traffic-network.md) and the accident timer.

The constructor (`0x0071F830`; tuning defaults in §4.14) sets the cycle state to 0 (StartRecalc), the last
seen day of month to −1, the strike state to "no strike" (2) and the transit-switch cost multiplier to 1.0.

**Shutdown** (slot 4, `0x00719290`) releases the pathfinders, grids, departments and iterators *[inferred: order not
part of the contract]*.

## 2. State

The data model, by meaning (offsets are deliberately not part of it):

| State | Notes |
|---|---|
| Tuning values (§4.14) | read at Init and on the reload message |
| The grids of §1 and the destination bitmask map | §4.5–§4.11 |
| Free destination counts `freeDest[k]`, k = 0…3 | number of lots that set destination bit `k`; written by the pathfinder too (pathfinder.md §4.10.1) |
| Destination-type capacity totals `jobCapacity[k]`, k = 0…2 | §4.11 |
| Commuter statistics per trip type k = 0…2: `attempted[k]`, `routed[k]` | §4.6 |
| Edge-density map: key → record of 33 u16 | §4.8 |
| Transit switches (hash: switch ID → record with usage, previous usage, fee, ratio, rules) | traffic-network.md; usage and fee accumulation in §4.5 |
| Accident-prone list (cells with probability), and the copy used by the accident check | §4.9, §4.13 |
| Per-network-type statistics (13 types, current and last) and the traffic stats block (slot 34) | §4.9 |
| Income accumulators (two doubles: income, fees), monthly income floats (transit, tolls, total) | §4.5, §4.12 |
| Commute accumulators: car-commute length sum and count, freight length sum and count | §4.6, §4.12 |
| Per-travel-type trip counts (two vectors of 9 u32: building, published) | §4.6 |
| Cycle state machine: state, phase-entry flags, slicing step state, iterators | §3 |
| Day pacing: day index, days for the phase, total, processed counters, the 20 pass counters | §3.3 |
| Re-route context (current source, search state, flags, work budget) | §4.4 |
| Re-route bucket `rerouteBucket` (0…3) | §4.3 |
| Strategy bucket counter, strategy bucket multimap (bucket → lot cell) | §4.1.3 |
| Travel strategy percentages `pct[w][s]` (4 wealth rows × 3 strategies, u8) | §4.1.2 |
| Strike state, damaged-cell set | §4.13, traffic-network.md |
| The network model | traffic-network.md |

**Trip data** (`cSC4TripData`, `0x2C` bytes) lives in each traffic source's list (§4.1):

| Field | Type | Meaning |
|---|---|---|
| route | `vector<u8>` | compressed route of the outbound leg (pathfinder.md §4.15) |
| returnRoute | `vector<u8>` | compressed route of the return leg (empty for reversible and freight trips) |
| count | u32 | number of trips in this batch |
| type | u8 | trip type = destination type: 0, 1, 2 = commuters of wealth $, $$, $$$ (jobs of that wealth), 3 = freight (§4.1) |
| strategy | u8 | lots: travel strategy 0 = mass-transit preference, 1 = car preference, 2 = neutral; neighbour connections: a travel type 0…8 (§4.1.4) |
| flags | u8 | bit 0 *reversible* (routed one way, counted both ways), bit 1 *probe*, bit 2 *forced start type* (`1 << strategy`), bit 3 *routed in the last full pass* |
| status | u8 | 0 needs a full search, 1 needs a re-route to its known destination, 3 routed, 4 new (lot side), 5 failed |
| destX, destZ | s16 | destination cell of the last successful search |
| tripLength | f32 | trip length of the last successful search |
| travelTypesUsed | u32 | mask `1 << t` of the travel types on the route(s) |

## 3. Agents, the daily work cycle and the state machine

### 3.1 Messages handled (`DoMessage` `0x007241B0`)

| Message | Condition | Action |
|---|---|---|
| `0x66956810` (SimBegin, the one-shot agent) | | sets the **network-changed byte** := 1 and returns true. The byte is cleared by the constructor and saved and loaded (§6.1 row 14: the `bool` after the intersection-dirty rectangle). Nothing else reads it, in 1.1.641 (`0x00724558`, its only writer besides the constructor `0x0071FC1A`) or in the Mac build. No rebuild follows: the network model is kept current by the occupant and neighbour handlers below (traffic-network.md §3) and rebuilt only by DoPostCityInit (§3.5). *[compat: keep the byte for the round trip; it drives nothing]* (TS5) |
| `0x66956814` (day agent) | | `DoDailySimulation` (§3.3); its result is the agent's "finished" answer |
| `0x66956816` (month agent) | | `DoMonthlySimulation` (§4.12) |
| `0x26D31EC1` | | `DoPostCityInit` (`0x00720430`, §3.5: builds the network model, traffic-network.md, then resumes the saved cycle) |
| `0x08DAC3E7` | data2 = `0xC921CEEB` (mass transit) | `transitMultiplier` (the transit-switch entry cost multiplier, traffic-network.md §4.7.3) := curve `0xCA5F7821`(`r24(f × 100.0f)`), `f` = the department's funding fraction (slot 10 `+0x28` returns a pointer to it); also set the same way in DoPostCityInit; saved |
| `0x8A0D15DA` / `0xEA0D163E` | | `AddNeighborConnection(data1 type, data2 x, data3 z)` (`0x007153C0`) / `RemoveNeighborConnection(type, x, z)` (`0x00713AB0`): they maintain the off-map cell's edge record (traffic-network.md §2.7.2–§2.7.3). Add sets the type bit, raises the trip cost and sets the pool. Remove clears the bit; once the mask is empty it also clears the pool, runs the pool sweep and frees the destination byte. Neither touches the network-changed byte (TS6) |
| `0xC53D10AA` | data1 = `0xE7E2C2DB` and data2 = `0xC9133286` | re-read tuning (§4.14) |
| `0xC2C09554` | | `UpdateTravelStrategies` (§4.1.2) |
| `0x99EF1142` / `0x99EF1143` | data3 = `0xC772BF98` | network occupant inserted / removed: the cell-count and pothole-set bookkeeping (§3.6), then the network part (traffic-network.md); on remove the cell's edge-density records are erased. Any such message (even a failed cast) sets the network-changed byte (see `0x66956810`; never read) |
| `0xE8BDA2E7` | lot (`IGZUnknown` → a **copy** of the lot with an empty trip list, lot-developer.md §3.2; §4.1.3 finds the real lot again at its corner) | data1 = 3: `DoLotDeactivated` (`0x0071E2F0`); else data2 = 3: `DoLotActivated` (`0x00723AF0`, §4.1.3) |

All handlers return true except the day handler.

### 3.2 WorkOnSimCycle (`0x00720F00`) and the next-state table

The traffic computation is one long **cycle** of phases, spread over several days. `WorkOnSimCycle(deadline)` runs
the current phase until it finishes or the deadline passes. When a phase finishes, the state moves to
`next[state]` (table `0x00AB3480`, u32[20]) and the new state's entry action runs. Before any phase, if the
intersection data are marked dirty, `UpdateIntersectionData` runs (traffic-network.md).

`next` = {16, 14, 4, 7, 5, 11, 10, 6, 0, 0, 12, 3, 13, 19, 15, 2, 17, 18, 15, 8}; state 12 is skipped (13 instead)
when the subnetwork data need no rebuild *[inferred: flag meaning, traffic-network.md]*.

| State | Phase | Spec | Entry action when entered |
|---|---|---|---|
| 0 | StartRecalc (`0x00717C90`) | §4.3 | |
| 16 | ConfirmTrips (`0x0071E1B0`) | §4.3 | traffic-source iterator reset; pass counters reset; re-route context idle |
| 17 | ReRouteTrips (`0x0071D020`) | §4.4 | same |
| 18 | FullTrips (`0x0071D440`) | §4.4 | same; the per-travel-type count vector being built := 9 zeros |
| 15 | FinishTripMap (`0x0070FEF0`) | §4.7 | lot iterator reset |
| 2 | CreateMaps (`0x0071A0C0`) | §4.9 | scanning iterator reset; per-network-type statistics reset; commercial work grid := 0 |
| 4 | CreateFinalMap (`0x0070F460`) | §4.10 | scanning iterator reset; commercial traffic grid := commercial work grid (copy of the grid data) |
| 5 | ClearTripDestinations (`0x0070E4F0`) | §4.11 | scanning iterator reset |
| 11 | UpdateNeighborConnections (`0x00714BE0`) | traffic-network.md §2.7.5 (not a no-op: on every neighbour connection it rolls the per-travel-type arrival arrays `0xAA9EA81A + i` into `0xAA9EA81D + i` and the arrival counts of `0x8A0B4100`, elements 4–7 into 8–11; TS2) | |
| 3 | ReduceTransitSwitches (`0x00713750`) | §4.8 | |
| 7 | DumpEdgeDensities (`0x007137D0`) | §4.8 | |
| 6 | ReduceEdgeDensities (`0x007110B0`) | §4.8 | scanning iterator reset |
| 10 | RefreshCommuteMap (`0x0070F6E0`) | §4.11 | `freeDest[0…3]` := 0; scanning iterator reset |
| 12 | UpdateSubnetworks (`0x007183B0`) | traffic-network.md §4.11 | runs in one call |
| 13 | UpdateSubnetworkInfo (`0x0071DC50`) | traffic-network.md §4.12 | working subnetwork info reset; when the phase finishes the working vector is copied to the published one (`vector::operator=` `0x0071CD90`, also called by DoPostCityInit) |
| 19 | FerryRoutes (`0x0071DC90`) | traffic-network.md §4.9.3 | ferry counters roll (last := current, current := 0; the riders are added to the ferry total used by history) and the pass restarts; posts `0x2C06BA82` when done |
| 8 | FinalCalculations (`0x0070E7D0`) | §4.12 | |
| 1 | (lot sweep) | | entry: lot iterator reset. **No `next` entry leads to 1**, so the cycle never enters it after construction (state 0); `GetDesiredLotInsertionPoint` (slot 22), which answers only in state 1, therefore returns null in practice *[inferred]* |
| 9, 14 | (none) | | finish immediately, never entered either (14 resets a further iterator) |

The cycle is therefore *[compat]*:

`0 → 16 → 17 → 18 → 15 → 2 → 4 → 5 → 11 → 3 → 7 → 6 → 10 → (12) → 13 → 19 → 8 → 0`.

The cell-scanning phases (2, 4, 5, 6, 10, …) use the adaptive slicing helper of simulator.md §5.5 (each with its
own static step state); slicing only splits the work, the results do not depend on it.

### 3.3 DoDailySimulation (`0x007215C0`): the day agent

1. Deadline := now + **4 ms** (`0x0068BF50`).
2. Day of month from the simulator (slot 6 `GetSimDate`). If it differs from the last seen value: store it, and
   * on day **1**: mass-transit line item 2 `SetIncome(int64 ftol(transitIncome))`, roads line item 5
     `SetIncome(int64 ftol(tollIncome))` (line item slot 15 `+0x3C`, pop 8; budget.md §8);
     `lastMonthlyIncome := r24(tollIncome + transitIncome)` (slot 18);
   * **strategy re-draw**: if `GetSimDateNumber() mod (strategyPeriod / strategyBuckets) = 0` (unsigned integer
     divisions; code defaults 365 and 128, so every 2nd day), for each lot cell registered under bucket
     `strategyBucket` (§4.1.3), in multimap order: lot := lot manager `GetLot(x, z, false)`; if present, draw
     `RandomUint32Uniform(100)` (§4.15) and store it as property `0x4A611773` (u8) on the lot. Then
     `strategyBucket := (strategyBucket + 1) mod strategyBuckets`.
3. Loop:
   * deadline passed → return **false** (not done for today; the simulator calls again within the same day,
     simulator.md §4);
   * `finished := WorkOnSimCycle(deadline)`;
   * if finished, by the **new** state `s`:
     * `s = 16`: `total := passTotal` (the number of trips counted in the last cycle, §4.6), `days := max(1, total
       div 20)`; day index and processed := 0; `toReRoute`, `toSearch` := 0; of the ten pass counters that follow
       them (§6.1 row 15 and two unsaved ones; §10 Q4) all are zeroed **except** counter 1 (the re-route counter,
       incremented by ReRouteTrips; zeroed on entering 17 instead) and counter 2 = `rerouteBucket` (kept: only
       StartRecalc advances it). Counter 3, the source counter of §4.3, is zeroed here, so it restarts at 0 every
       cycle (TS1); continue;
     * `s = 17`: `total := toReRoute`, `days := max(1, total div 18)`, reset (day index, processed and pass
       counter 1 := 0), return **true**;
     * `s = 18`: `total := toSearch`, `days := max(1, total div 18)`, reset, `passTotal := 0`, return **true**;
     * `s = 2` or `s = 11`: return **true**;
     * otherwise continue;
   * if not finished: if `16 ≤ state ≤ 18`, `dayIndex < days` and `processed ≥ total · (dayIndex + 1) / days`
     (all unsigned, `0x0072191E`), then `dayIndex += 1` and return **true**; else return false. *(Corrected: the
     `dayIndex < days` guard was missing.)* Once `dayIndex` has reached `days` (a cycle with more trips than the
     last one, or `days = 0` after a load, §3.5 step 13) no share ends the day: the phase runs on, slice after
     slice, until it finishes.

So the trip phases are spread over `days` days (at least the share `total/days` per day), and a day also ends after
FinishTripMap (on entering 2) and after ClearTripDestinations (on entering 11).

*[compat]* The amount of work done on a given day in the original depends on wall-clock time: the per-day share is
tested only when a 4 ms slice ends without finishing the phase, so the last slice of a day overshoots the share by
a timing-dependent amount, and a fast machine may finish a whole phase inside one slice. *[choice]* FreeCity4 makes
this deterministic with a work budget per slice (§3.4). Differential tests compare per-cycle results, not per-day
progress (§11).

### 3.4 Traffic work budget *[choice]*

The original tests its 4 ms deadline once per **unit**: before each traffic source in ConfirmTrips (`0x0071E222`)
and before each trip in ReRouteTrips (`0x0071D1D9`) and FullTrips (`0x0071D607`). FreeCity4 replaces the deadline
with a count of those units.

- **Unit of work**: in ConfirmTrips, one traffic source (`UpdateTrafficVolume` + `ConfirmTrips(source)`); in
  ReRouteTrips and FullTrips, one trip examined (with `SetupTrip` + `PerformPathSearch` when it is searched). A
  search never suspends (pathfinder max work time 0), so a trip is atomic.
- **Budget** `trafficUnitsPerSlice` = N (tuning value, ≥ 1). One `WorkOnSimCycle` call in states 16–18 is a
  slice: it ends unfinished after N units (where the original's deadline would fire). Step 3 of §3.3 then applies
  unchanged: the share test runs at each unfinished slice end; when the share is not reached the day continues
  with another slice (the original's "return false, called again within the same day"). A slice that finishes its
  phase and continues into the next one (state 16) starts the next phase with a fresh N *[choice: the original
  shares one deadline]*. The other phases run to completion in one call (their slicing does not change results).
- Consequently the work of one trip-phase day is the shortest run of whole slices whose end reaches the share
  `total · (dayIndex + 1) / days`: at least the share, and at most N − 1 units more.
- **Default**: to be calibrated in the original (§12). N = 1 is the minimum (the day ends exactly at the share).
- **Observable effect of the budget** (for the M5 exit tests):
  - Larger N means more work per day than the share, so the trip phases finish in fewer days and the **traffic
    cycle is shorter** (in game days). The commute, congestion, air-polluting and commercial maps are then
    republished more often and converge faster after a change to the network or the zoning. Monthly sums that
    accumulate per applied route (toll and transit income, edge-density counters, accident draws per entry) grow
    with the number of cycles per month.
  - Smaller N has the opposite effect. The cycle length tends to `days(16) + days(17) + days(18)` plus the fixed
    day ends after phases 15 and 5 (§3.3).
  - Budget-sensitive statistics: cycle length, maps sampled at a fixed date, convergence time after a change, and
    monthly income and accident counts. Budget-insensitive: the result of one cycle given the same inputs (routes,
    volumes and the maps it publishes), and the draw sequence of each operation.

### 3.5 DoPostCityInit (`0x00720430`, message `0x26D31EC1`) *[compat]*

Runs once per city load (and for a new city), after every lot and occupant of the save exists and after `Read`. The
whole order (TS7):

1. – 7. **Network part**, traffic-network.md §1 steps 1–7: re-read every transit switch's rules; refresh the subway
   access points; `DoConnectionsChanged` over the whole city; **register the neighbour connections** (an Add for each
   connection of the neighbours simulator, slot 16 `GetAllConnections` in list order, then one pool sweep;
   traffic-network.md §2.7.2–§2.7.3); the **destination-map boundary pass** (each ring cell around the map whose
   `HasMapExit` is false gets destination byte 0, `freeDest` untouched; traffic-network.md §1 step 5, §2.7); clear
   the subnetwork labels and run `UpdateSubnetworks`; build the subnetwork info with no time limit and publish it.
8. If the loaded cycle state is 12 or 13, it becomes **8** (FinalCalculations): the subnetwork work was just done in
   full, so the cycle skips the rest of 12/13 and also state 19 (`0x00720859`).
9. `transitMultiplier := curve 0xCA5F7821(r24(f × 100.0f))`, `f` = the mass-transit department's funding fraction
   (as §3.1 for message `0x08DAC3E7`).
10. A rebuild of the per-network-type cell counts from the network occupants of the city, guarded by a flag that
    the constructor clears and nothing in 1.1.641 sets (`0x00720895`): **never runs**. The counts come from the save
    and from the occupant messages *[compat: do nothing]*.
11. **Resume the interrupted phase**, by the loaded cycle state:
    * 16, 17, 18: the re-route context's search state := idle; the traffic-source iterator is reset (lots, then
      neighbour connections, §4.1) and advanced until it has returned the source whose cell is the saved source cell
      (the cell the phase stored when it took its current source, §6.1 row 14). That source becomes the held current
      source, so the phase continues after it. If no source has that cell, the iterator is reset again and no source
      is held (the phase restarts from the first source). *[inferred: the position inside a partly processed source's
      trip list is the saved context's, not traced]*
    * 15: the lot iterator is reset; when the lot manager has a lot at the saved cell (`GetLot(x, z, false)`), the
      iterator is advanced past that lot; if the walk reaches the end without meeting it, the iterator restarts from
      the first lot.
    * 19: every ferry terminal's re-route flag := 1 and the ferry pass restarts at the first terminal, its sub-step
      0 (traffic-network.md §4.9.3). **Only in this state**: a city saved in any other state keeps the loaded ferry
      links until the next state-19 pass.
    * 1 or 14 (never saved by 1.1.641, §3.2): `freeDest[0…3] := 0`, then the ClearTripDestinations work (§4.11) and
      the RefreshCommuteMap work over the whole city, each run to completion, then state := 0.
    * any other state: nothing.
12. **Strategy buckets**: the bucket multimap (§4.1.3) is cleared and rebuilt from the lots, in the lot manager's
    list order (slot 15): each lot whose property `0x691C863C` is present is registered under that value with its
    location (lot slot 21). Equal keys keep list order. (So the daily re-draw of §3.3 does reach loaded lots; the
    bucket counter and the current bucket come from the save.)
13. `days := lotCount div 18` (lot manager slot 16 `GetLotCount`, unsigned; **no** `max(1, …)`, so 0 with fewer than
    18 lots). `total`, `dayIndex` and `processed` keep their loaded values; §3.3's share test is skipped while
    `dayIndex ≥ days`.
14. `UpdateMassTransitBudget`, then `UpdateRoadsBudget` (§4.9): the departments' monthly costs from the loaded cell
    counts.

### 3.6 Network occupant bookkeeping: DoRoadChanged, DoRailChanged, DoSubwayChanged *[compat]*

On `0x99EF1142` (inserted, `ins` = true) and `0x99EF1143` (removed, `ins` = false) for a network occupant (§3.1),
DoMessage first does this bookkeeping (`0x00724334`–`0x0072442E`), then the network part (traffic-network.md §3). Let
`has(t)` mean that the occupant's network flag word contains bit `1 << t` (cISC4NetworkOccupant slot 22; network
types as traffic-network.md §2.2). The calls, in this order (TS4):

1. `has(6)` Avenue → `DoRoadChanged(occ, ins, 6)`.
2. `has(0)` Road → `DoRoadChanged(occ, ins, 0)`.
3. The first of OneWayRoad 10, Street 3, Highway 2, GroundHighway 12 that `has` holds for →
   `DoRoadChanged(occ, ins, t)`. At most one call comes from this step.
4. `has(1)` Rail → `DoRailChanged(occ, ins)`.
5. `has(7)` Subway, and the flag word does **not** contain bit 31 (slot 8, all-bits test of `0x80000000`) →
   `DoSubwayChanged(ins)`.

An occupant with several of these types gets several calls (Avenue and Road: steps 1 and 2). DirtRoad 11, LightRail 8
and Monorail 9 get no bookkeeping.

`cells[t]` (t = 0…12, u32, saved in §6.1 row 18) are the per-network-type counts that `UpdateRoadsBudget` and
`UpdateMassTransitBudget` (§4.9) price. `bump(t)` is `cells[t] += 1` on insert. On remove it is `cells[t] −= 1` only
if `cells[t] > 0`; at 0 it stays 0.

**DoRoadChanged(occ, ins, t)** (`0x007155D0`):

1. Cell `(x, z)` := the city's `PositionToCell` (slot 95) of the occupant's position (occupant slot 8, its x and z).
2. `bump(t)`, then `UpdateRoadsBudget`.
3. If `t` is Highway 2 or GroundHighway 12, stop: highways are not in the pothole sets.
4. `key := x << 8 | z` (u16, as §4.12).
5. **Insert**: add `key` to the intact road set; nothing happens if it is already there. The damaged set is not
   looked at, so a key can end up in both sets. Then `roadCellCount += 1` in **every** case, including a key that
   was already present.
6. **Remove**, in this order:
   * If `key` is in the intact set, erase it.
   * Otherwise, if `key` is in the damaged set, it is repaired by removal. Erase it, then `damagedRoadCount −= 1`.
     Then decrement the published **pothole count** (the u16 at offset `0x134` of the slot-34 statistics block, read
     by the traffic advisor as `g_pothole_count`; it wraps below 0). Then post `0x69247DC8` (message-server slot 4,
     queued) with data1 = x, data2 = z, data3 = 0.
   * In **every** case, including a key in neither set: `roadCellCount −= 1` (u32, no clamp).

**DoRailChanged(occ, ins)** (`0x00715790`):

1. `bump(1)`.
2. If the occupant also has Road or Street (slot 9, any-bit test of mask 9: a level crossing), skip to step 5.
3. Cell `(x, z)` := the network occupant's own cell (slot 59). `key := x << 8 | z`.
4. Insert and remove as DoRoadChanged steps 5–6, with the rail sets, `railCellCount` and `damagedRailCount`. On
   insert the rail set is first grown to hold `count + 1` (the same rehash rule as the road insert). *[quirk]*
   Removing a damaged rail cell also decrements the **road** pothole count. It posts `0x69247DC8` with data3 = 0, like
   a road.
5. `UpdateMassTransitBudget`, also when step 2 skipped.

**DoSubwayChanged(ins)** (`0x0070EA70`): `bump(7)`, then `UpdateMassTransitBudget`. It has no cell set.

The bracket of TS4 holds for the sets. What it misses is the count, the damaged set, the highway and level-crossing
exclusions, and the per-type counts with the budget refresh.

* **The cell counts count calls, not cells.** Both cases below add 2 to the count but only one key to the set:
  * an occupant re-inserted at a cell it already holds;
  * an occupant with two counted road types (steps 1 and 2).

  The monthly pothole target is `cellCount × d` (§4.12), so a rebuild that counts distinct cells drifts from the
  original. A replacement keeps the running count, updated by these rules. A rebuild from the live occupants is
  exact only when it repeats the same per-call rules. To do so, count each occupant once per DoRoadChanged call
  (steps 1–3, excluding Highway and GroundHighway), and once per rail occupant without Road or Street.
* The pothole count is only re-set by `ApplyRoadFunding`, and only in a month when the road target changes
  (`damagedRoadCount := target`, then the pothole count := its low 16 bits; `0x00717DF2`). In the months between,
  the drops above stay, including the rail drops. `ApplyMassTransitFunding` writes no pothole count.
* *[quirk]* The cell counts have no floor: with more removes than inserts (an occupant removed twice), `roadCellCount`
  wraps from 0 to 2³² − 1. The next month's target is then `trunc(r24(4294967295.0 × d))`, and `ApplyPotholeChanges`
  caps the move at the intact set's size. Keep the wrap *[compat]*.

Worked example. 100 road calls so far, `cells[0]` = 100, `roadCellCount` = 100, `damagedRoadCount` = 3, pothole
count 3, and cell (12, 34) is damaged (key `0x0C22`). Bulldoze its road piece (flags: Road only):

1. Message `0x99EF1143`. Step 2 calls `DoRoadChanged(occ, false, 0)`.
2. `cells[0]` = 99, and the roads budget is recomputed from it.
3. Key `0x0C22` is not intact but is damaged. It is erased, `damagedRoadCount` = 2, the pothole count = 2,
   `0x69247DC8 (12, 34, 0)` is posted, and `roadCellCount` = 99.

Then place a level-crossing piece at (20, 5) with flags Road | Rail:

1. Step 2 inserts key `0x1405` into the intact road set: `cells[0]` = 100, `roadCellCount` = 100.
2. Step 4: `cells[1] += 1`. The rail sets and `railCellCount` are untouched (Road is set), and the mass-transit budget
   is recomputed.

## 4. Behaviour

### 4.1 Trip generation (traffic sources)

A **traffic source** is a lot or a neighbour connection. Its interface `cISC4TrafficSource` (vtable `0x00AB1914` on
lots, `0x00AB1FA0` on neighbour connections) has: slot 3 `GetTripDataList` (the source's `std::list<cSC4TripData>`),
slot 4 `UpdateTrafficVolume`, slot 5 `GetSourceCells(SC4Rect<u32>&)` (the lot's cell rect, or the connection's
cell).

**Traffic-source iterator** (`Reset` `0x007107A0`, `GetNext` `0x007107F0`): all lots in the lot manager's list
order (lot manager slot 15), then all neighbour connections in the neighbour simulator's list order
(`GetAllConnections`, slot 16). Each step yields the source's location and its traffic-source interface
(`AsTrafficSource`, slot 4; AddRef'd).

#### 4.1.1 Lot: `cSC4Lot::UpdateTrafficVolume` (`0x006C27E0`)

The target per strategy `w[0…2]` (u16) starts at 0, `type` at 0.

* **Residential** (zone 1–3):
  * not in a livable state (lot slot 37) → probe (below);
  * `scale := residential simulator GetCellWorkforcePercent(x, z)` (slot 30; float);
  * `pop := GetTravelPopulation(0x1010)` (lot slot 56) with `wealth = 0`; if 0, `0x1020` with `wealth = 1`; if 0,
    `0x1030` with `wealth = 2`; all 0 → probe. `type := wealth`.
  * `n := r24(float(pop) × scale)`; `commuters := max(1, floor(n))` where floor is computed as
    `r := round(r24(n − 0.5))`, then `r + 1` if `float(r + 1) = n` (u32 arithmetic). *[compat]*
  * `pct := traffic simulator GetTravelStrategyPercentages(wealth + 1)` (slot 26, §4.1.2, a 3-float vector).
  * If `commuters < 50`: all commuters use **one** strategy. `c := 1`; for `s` = 0, 1, 2:
    `c := c + (round(r24(pct[s] × 100.0f)) & 0xFF)`; the first `s` with `strategyByte < c` gets
    `w[s] := commuters`. `strategyByte` is the lot's own byte (below). *[quirk]* Byte 0 always selects strategy 0,
    even when `pct[0] = 0`.
  * Else: `w[s] := round(r24(float(commuters) × pct[s])) & 0xFFFF` for each `s`. The parts need not add up to
    `commuters` (test 8). *[compat]*
* **Industrial** (zone 7–9): only if the lot has road access (slot 28) and slot 59 returns 0, else return without
  change. `type := 3`. Not livable → probe. `jobs := Σ GetTravelPopulation(t)` for `0x4400, 0x4300, 0x4200, 0x4100`
  (in this order); `n := r24(freightScaling × float(jobs))` (slot 27 value); `w[2] := ceil(n)` computed as
  `r := round(r24(n + 0.5))`, then `r − 1` if `float(r − 1) = n`.
* **Other zones**: return without change (no trips).

**Probe**: a single trip that only tests reachability. **Reconcile** the trip list with the targets
(`total = w[0] + w[1] + w[2]`):

* `total = 0`: if the list's first trip is a probe, keep the list unchanged; otherwise clear the list and append one
  probe `{count 1, type, strategy 0, flags 2 (probe), status 0, tripLength 0}` (destination and travel-type fields
  undefined in the original *[quirk]*; FreeCity4: 0 *[choice]*).
* otherwise: erase all probe trips; then for `s` = 0, 1, 2: walk the list, **erasing trips of another type** and
  summing `have` = the counts of strategy-`s` trips;
  * `have < w[s]` (deficit `d`): the first strategy-`s` trip with status 4 or 5 gets `count += d`; if none, append
    `{count d, type, s, flags 0, status = (any strategy-s trip exists ? 4 : 0), tripLength 0}`;
  * `have > w[s]` (surplus): reduce, in list order, first the status-4 trips of strategy `s`, then any trips of
    strategy `s` (a trip whose count would reach 0 is erased).

**Strategy byte.** Each lot has a byte, `0xFF` at construction; `cSC4Lot::Init` replaces `0xFF` by
`RandomUint32Uniform(100)` from the lot file's `cRZRandomSuper` (`0x00B49120`, foundations §6.2); it is saved with
the lot (§6.3). The value the traffic simulator writes into property `0x4A611773` (§3.3, §4.1.3) is **never read**
by the executable (the only references are the two writers `0x007216F6`, `0x00723FD0`) *[compat: the property is
saved with the lot's properties]*.

#### 4.1.2 Travel strategy percentages

`pct[w][s]` (u8, w = 0…3 = no wealth, $, $$, $$$) comes from tuning properties `0x4953E8A3 + w` (first 3 bytes,
no length check *[quirk]*). `UpdateTravelStrategies` (`0x00711810`): re-reads them from the exemplar, then for every
ordinance that is on (ordinance simulator slots 14/15, the ID array allocated with allocator C) adds its misc
property `0x8A612FEE` (s32[9], read without a size check *[quirk]*; ordinance.md §6.3) element `3(w−1) + s` to
`pct[w][s]` for w = 1…3, clamping each sum to 0…100. Runs at Init and on `OrdinanceUpdated`. *[quirk]* When the
exemplar is missing, the re-read is skipped and the ordinance offsets accumulate on every update.

`GetTravelStrategyPercentages(w, vector<float>* out)` (slot 26) resizes `*out` to 3 (allocator B) and writes
`out[s] := r24(float(pct[w][s]) × 0.01f)` (dword constant).

#### 4.1.3 Lot activation and deactivation

`DoLotActivated` (`0x00723AF0`):
- building := lot slot 48 (none → return);
- lot := the lot manager's `GetLot` at the lot rectangle's corner (slot 6, bool false), which may be null;
- the transit-switch and ferry-terminal steps from the building's properties are traffic-network.md §4.7.1.

If the building's `0xE90E25A1` has a byte size that is not a multiple of 4, DoLotActivated returns inside the switch
step, and **none of the steps below runs**, not even the draw (TN1, traffic-network.md §4.7.1 step 1). Otherwise
(the property is missing or well formed), in order:

1. Only if the lot was found: `b := strategyBucket counter`; counter := `(counter + 1) mod strategyBuckets`; lot
   property `0x691C863C` := `b`; the lot cell is added to the bucket multimap under `b`.
2. Only if the lot was found: `RandomUint32Uniform(100)` (`0x00723FBE`) → lot property `0x4A611773` (u8).
3. Budget items. Budget simulator slot 51 is asked with the building's property holder for `0xCA550301` first, and
   for `0x0C3BF549` only when that fails. If one succeeds and the lot was found, slots 58 and 55 follow. Only on the
   `0xCA550301` path is the int64 at `+0x10` of the record that slot 51 filled added to the total `E` (`+0x778`,
   saved, §6).
4. Unless the cycle is in state 10, and whether or not the lot was found: `bits := 0`; for `k` = 0…3, if
   `GetMaxTripCapacity(holder of the message's lot, k) > 0`, set bit `k` and `freeDest[k] += 1`. Then **store**
   `bits` into the destination byte of every cell of the lot rectangle (`0x00724167`). *(Corrected: the byte is
   replaced, not ORed into.)*

`DoLotDeactivated` (`0x0071E2F0`) removes what activation added (destination bits, bucket entry, network rules)
*[inferred: not traced line by line]*.

#### 4.1.4 Neighbour connection: `UpdateTrafficVolume` (`0x006D4C10`)

Runs only when the connection's "volume dirty" flag is set (then cleared). Holder := the connection's property
holder (neighbour simulator slot 13). For trip type `w` = 0…2 and index `s` = 0…8: `target := ` element `s` of the
u32 vector property `0xAA9EA81D + w` (padded with zeros to 9; set by the neighbour simulation, neighbors.md);
`have :=` the sum of counts of the list's trips with type `w` and strategy `s` (trips of type > 2 are ignored and
never erased).

* `have < target`: `d := (target − have) & 0xFFFF` *[quirk: 16-bit]*; the first type-`w`, strategy-`s` trip with
  status 5 absorbs `d`; else append `{count d, type w, strategy s, flags 4 (forced start type), status 0,
  tripLength 0}` (destination and travel-type fields uninitialised in the original *[quirk]*; FreeCity4: 0
  *[choice]*).
* `have > target`: remove the excess from status-5 trips of `(w, s)` first, then from any trips of `(w, s)`, in list
  order (erasing trips that reach 0).

So for neighbour connections the strategy field is a **travel type** and the trip must start in it (§4.4).

### 4.2 Capacity of destinations

Each lot and neighbour pool carries property `0x8A0B4100`, an array of 12 u16 (promoted to u32 elements once a value
reaches `0xFFFF`): elements 0–3 = **max trip capacity** per destination type `k`, 4–7 = trips **arrived this
cycle**, 8–11 = trips **arrived last cycle**.

| Function | Result |
|---|---|
| `GetMaxTripCapacity(holder, k)` (slot 20) | element `k`, 0 if the property is missing |
| arrived this cycle (`0x0070DFF0`, non-virtual; the pathfinder's "used capacity") | element `4 + k` |
| `GetTrafficArrived(holder, k)` (slot 21) | element `8 + k` |
| `SetMaxTripCapacity(holder, k, value)` (slot 19) | creates the property if missing; sets element `k` |

The capacities are set by the building development and civic code from jobs and freight acceptance (callers of
slot 19; building-development.md) *[inferred: caller list not part of this spec]*.

`CapacityStillAvailable(trip)` (`0x007136B0`): holder := the lot at the trip's destination cell (in city) or the
neighbour pool holder of that off-map cell (`GetNeighborPoolPropertyHolder(−1, x, z)`, `0x007114E0`); result :=
`max(k) − arrivedThisCycle(k) ≥ count` (unsigned subtraction) with `k = type`; with no holder `0 ≥ count`.

`ApplyTrafficToDestination(count, x, z, lastTravelType, type)` (`0x00714740`): holder as above, except that an
off-map cell asks `GetNeighborPoolPropertyHolder(lastTravelType, x, z)` (traffic-network.md §2.7.1), which is null
when no connection type at the cell carries that travel type and the cell has no ferry connection. For an off-map
cell and `type < 3` it also adds `count` to element `lastTravelType` of the u32[9] property `0xAA9EA81A + type`
(nine zeros if missing) on the **neighbour connection's own** holder, the connection that
`FindNeighborConnection(lastTravelType, x, z)` finds (`0x007147C9`). This is not the pool. If there is no such
connection, nothing is added. *(Corrected: an earlier text said the pool's property.)* Then
`arrivedThisCycle(type) += count`; if now `arrived ≥ max`, clear destination bit `type` over the destination lot's
rect and decrement `freeDest[type]` *[quirk: decremented on every such call, also when the bit was already
clear]*.

### 4.3 StartRecalc and ConfirmTrips (states 0 and 16)

**StartRecalc** (§4.12) advances `rerouteBucket` (0…3) and resets the cycle accumulators. It then recomputes the
neighbour pool capacities and sets the destination bits of the off-map edge cells (traffic-network.md §2.7.4,
`0x00714E60`, called at the end of `0x00717C90`).

**ConfirmTrips** walks all traffic sources (§4.1). For each source: `source.UpdateTrafficVolume()` (§4.1.1 /
§4.1.4), then `ConfirmTrips(source)` (`0x0071D920`):

* `sourceCounter += 1`; `rerouteSource := ((sourceCounter & 3) = rerouteBucket)` — one source in four takes part in
  this cycle's re-balancing. `sourceCounter` is 0 on entering state 16 (§3.3), so the first source has counter 1.
* If `rerouteSource`: **merge** — for each routed (status 3) trip, every later trip of the same type and strategy is
  merged into it (counts added, the other erased) and the trip's status becomes 0 (search again).
* Then for each trip (`processed += 1`):
  * **status 3** (routed):
    * if `CapacityStillAvailable`: status := 1 (re-route to the known destination), but 0 for freight. If not
      `rerouteSource`, the old route is **confirmed** without a search: `ConfirmRoute(route)` (`0x0070E320`; below);
      on success:
      * freight: `ApplyRoute(route, mode 0)` (§4.5) unless probe; status := 3;
      * reversible (flag 0): `ApplyRoute(route, mode 2)` unless probe; status := 3;
      * otherwise `ConfirmRoute(returnRoute)` must succeed too; then `ApplyRoute(route, mode 0)` and
        `ApplyRoute(returnRoute, mode 1)` unless probe; status := 3.
    * else (destination full): status := 0.
  * **status 5** (failed last time): status := 0.
  * afterwards status 1 counts towards `toReRoute`, status 0 towards `toSearch`.

**ConfirmRoute(route)** (`0x0070E320`, one argument) *[compat]* (TS14). It replays the compressed route
(pathfinder.md §4.15) with the route reader and tests each cell against the cell connection records
(traffic-network.md §2.1). The reader turns the route into one record per visited cell, in route order:

* the cell (x, z) and its travel type `t` (the type of the header or token that brought the route there);
* the next type `t'` (the type of the following run or jump; `t' = t` at the last cell);
* the **exit** side `x`: the direction of the following unit step; for the following **jump** to cell `(qx, qz)`,
  with `dx = qx − cx` and `dz = qz − cz`: when both are non-zero, the one smaller in absolute value is set to 0
  (`dx` when the magnitudes are equal); then `|dx| ≥ 2` → `+x` or `−x` by its sign; `dx = ±1` (so `dz = 0`) →
  **none**, and the cell's own entry side also becomes none; `dx = 0` → `dz ≤ −2` gives `−z` and **every other `dz`
  gives `+z`**, including `dz = −1` and `dz = 0` *[quirk]* (`0x006D65E0`–`0x006D666A`). The last cell's exit is
  none.
* the **entry** side `e`: none at the first cell; after a unit step or a jump with an exit, the side facing back
  (`(x + 2) mod 4`); after a jump whose exit is none, the previous cell's entry side carries over unchanged.

The test, with `strike := (strike state = 1)` (§4.12) read once at the start:

1. An empty route: **false**.
2. **First cell**, if inside the city and its exit is not none: when `t = t'`, some entry side `e' ∈ 0…3` must have
   bit `x + 4e'` set in `conn[t]` (any entry may leave through `x`); when `t ≠ t'`, nibble `x` of `switch[t]` must
   equal `t'`. No strike test and no wormhole test here.
3. **Every later cell**, if inside the city (cells off the map are skipped) and its exit is not none (the last cell
   and a cell before a jump with no exit are skipped):
   * `t = t'`: pass when bit `x + 4e` of `conn[t]` is set (never when `e` is none). Otherwise pass when the cell's
     wormhole byte has the tunnel bit (2); otherwise pass when it has the ferry bit (4) **and** there is no strike;
     otherwise **false**.
   * `t ≠ t'` (a type change): during a strike **false**; otherwise nibble `x` of `switch[t]` must equal `t'`, else
     **false**.
4. **true**.

So a strike fails every confirmed route that changes travel type after its first cell or that crosses a ferry
terminal's water link, whatever the types; a route that keeps one type (a car route, or a neighbour trip forced to
start on the bus or a train and staying on it) still confirms. The last cell is never tested. *[choice]* A type
byte above 8 (only in a damaged save) fails the test; the original reads unrelated words of the record.

**Example** (§11 test 10's route `01 02 03 41 02 0A`): cells (2,3) type 1, exit `+x`, entry none; (3,3) type 1,
entry `−x` side 0, exit `+x` side 2; (4,3), exit none. ConfirmRoute is true when (2,3)'s `conn[1]` has bit 2, 6, 10
or 14 and (3,3)'s `conn[1]` has bit `2 + 4·0 = 2`; (4,3) is not looked at. The same with a walk-to-bus change at
(3,3) (`t = 0`, `t' = 2`): during a strike false; otherwise nibble 2 of (3,3)'s `switch[0]` must be 2.

### 4.4 Path searches: ReRouteTrips and FullTrips (states 17 and 18)

**ReRouteTrips** (`0x0071D020`) takes, for each source, the status-1 trips: if the destination has no capacity left
(`CapacityStillAvailable` false) the trip goes back to status 0; otherwise `SetupTrip` then `PerformPathSearch`;
result found → status 3 and `tripLength := pf.GetTripLength()`; not found → status 5. Then, for the same source, a
merge of status-0 trips (each absorbs the later ones of the same type and strategy whose status is not 3). A
network change (`DoConnectionsChanged`) aborts a suspended search: both pathfinders `Reset` and the context restarts.

**FullTrips** (`0x0071D440`) takes, for each source, the status-0 trips: `SetupTrip`, `PerformPathSearch`;

* found: status 3, `tripLength := pf.GetTripLength()`, `arrived := pf.GetTrafficArrived()`; if `arrived < count` the
  trip is **split**: a copy with `count − arrived` and status 0 is inserted right after it (allocator B node) and the
  original keeps `arrived`; the copy is searched next with the *continue* flag (pathfinder flag 4) so the search
  resumes from the previous open set (`continueSearch := true`); otherwise `continueSearch := false`;
* not found: status 5, `continueSearch := false`.

After each source's list: `CompleteTripSourceCalculations(source)` (§4.6). At the end of the phase the per-travel-type
trip counts built in this pass are published (slot-34 stats and history, §4.12).

**SetupTrip(trip)** (`0x0070EC40`) configures **pf** (pathfinder.md §4.2 for the meaning of every setting):

1. `SetTrafficDensity(count)`.
2. Status 1 (re-route): `SetTripDestinationFilter(3)`, `SetEndRect(` the destination lot's bounding rect if the
   destination cell is in the city and holds a lot, else the single cell `(destX, destZ))`.
   Otherwise: `SetTripDestinationFilter(0)`, `ClearAllDestinationFlags`, `SetDestinationFlag(probe and type 0 ? 7 :
   1 << type)`.
3. `SetMaxWorkTime(remaining slice ms)` on pf and pf2; `SetStartRect(source.GetSourceCells())`.
4. `ClearAllFlags`, `ClearAllTravelTypeFlags`, `SetStartingCostArray(null)` (= the simulator's default array),
   `SetMaxTripLength(commuteMaxTime)`.
5. Commuters (type ≤ 2): if `continueSearch`, `SetFlag(4)`. Forced start (flags bit 2):
   `SetTravelTypeFlag(0x1CF, false)` then `SetTravelTypeFlag(1 << strategy, true)`. Otherwise
   `SetTravelTypeFlag(0x1CF, false)`, `SetTravelTypeFlag(0x3, true)` (may start walking or by car), and
   `SetStartingCostArray` by strategy: 0 → mass-transit array (`0xEA8C3CDB`), 1 → car array (`0xCAD64136`), 2 →
   neutral array (`0xA92356BC`). `0x1CF` = walk, car, bus, passenger train, subway, el-train, monorail.
6. Freight (type 3): `SetFlag(1)` (no budget), `SetTravelTypeFlag(0x30, true)` (truck, freight train).

**PerformPathSearch(trip)** (`0x0071C6C0`):

1. Outbound: `r := pf.FindPath()` (unless resuming the return leg). 2 → suspended (resume later); 0 → not found.
2. Found: `(destX, destZ) := pf.GetEndCell()`.
   * Freight: `pf.GetCompressedRoute(route, travelTypesUsed)`; `ApplyRoute(route, pf.arrived, mode 0)` unless probe;
     done.
   * `pf.WasTripReversible()`: flags |= 1; route as above; `ApplyRoute(…, mode 2)` unless probe; done.
   * Otherwise flags &= ~1 and the **return leg** is searched with **pf2**, configured by writing its fields
     directly *[compat: the pathfinder's memory layout is shared with the simulator; pathfinder.md §9]*:
     start rect = the destination lot's rect (or the destination cell; for an off-map neighbour connection the
     highway-return adjustment of traffic-network.md applies), end rect = pf's start rect (or the highway return
     tile), flags := 1 (no budget), traffic density := pf's traffic arrived, filter := 3, allowed types := pf's
     allowed types, starting types := `1 << ` the travel type of pf's result node, starting cost array := pf's;
     if pf recorded a park-and-ride cell, it is copied and flag 8 is set.
     *[quirk]* pf2's max trip length is never set: it is the shared max-length/trip-length float (pathfinder.md §4.3),
     so it holds pf2's previous trip length; with flag 1 it is not used as a budget.
   * `pf2.FindPath()`: 2 → suspended; 0 → not found (the whole trip fails). Found: both compressed routes are
     fetched (`travelTypesUsed |=` pf2's mask), `ApplyRoute(route, mode 0)`, `ApplyRoute(returnRoute, mode 1)` unless
     probe.

`SetupPathFinderForLot(pf, lot)` (slot 7, `0x00711610`) is the same configuration for outside callers (traffic
automata, query tool) *[inferred]*.

### 4.5 Applying a route: ApplyRoute (`0x00719910`)

`ApplyRoute(route, count, sourceRect, destX, destZ, mode, type)` replays a compressed route (reader `0x006D64B0` /
`0x006D6510`, pathfinder.md §4.15) and adds its traffic. `mode`: 0 = outbound, 1 = return leg, 2 = both directions
(reversible trip).

* `N := float(count)`; `Nc := N`, or with the Carpool ordinance (`0xA2BF1DDC`) on,
  `Nc := float32(ceil(double(r24(N × 0.8f))))`. Car (travel type 1) uses `Nc`, all others `N`. `Q` = that value,
  `Qi := round(Q)`.
* For each route cell `(x, z, t)` in city:
  1. **Volume**: if the cell has network info for transit network `network(t)` (pathfinder.md §4.1) and
     `generatesTraffic[t]`: `vol := r24(vol + Q)`; mode 2 adds a second time, `vol := r24(vol + Q)`.
  2. **Transit switch**: when the cell's switch ID (switch map) differs from the last one seen on this route and is
     non-zero, except when both this and the previous step are subway (type 6): `usage := r24(usage + Q)` (once, also
     in mode 2) and `fees := r24(fees + r24(Q × switchFee))`.
  3. **Edge density** (§4.8): entry side `e` = direction from the cell to the previous route cell (for the first cell:
     towards the source rect, `x < x0 → step +x`, `x > x1 → −x`, else `z < z0 → +z`, else `−z`); exit side `o` =
     direction from the cell to the next route cell, or at the last cell towards `(destX, destZ)` (mode 0, 2) or
     towards the source rect (mode 1); a direction is only defined for a one-cell step (else no update). With both
     defined: record key `(mode = 1) << 24 | t << 16 | x << 8 | z`, `current[4e + o] := min(0xFFFF, current + Qi)`.
     **Mode 2 also adds `Qi` to the record with key `1 << 24 | t << 16 | x << 8 | z` at index `4o + e`** (the mirrored
     direction) *[compat]*. The pathfinder's `ApplyTraffic` (pathfinder.md §4.17, no known caller) mirrors the same
     way (corrected by PF2).
  4. **Ferry legs**: a step across a ferry link adds `round(Q)` to the link's counters (by mode) and
     `r24(Q × ferryFare)` to the fares *[inferred: counter meaning, traffic-network.md]*.
  5. **Income**: `I := r24(I + r24(Q × incomePerTile[t]))`.
* At the last cell, unless mode 1: `ApplyTrafficToDestination(count, destX, destZ, lastTravelType, type)` (§4.2).
* Finally `incomeAcc := incomeAcc + I` and `feeAcc := feeAcc + fees` (doubles, the additions rounded to 24 bits).

### 4.6 Daily results: trip length per source (CompleteTripSourceCalculations, `0x0070F0A0`)

For each trip of the source (`passTotal += 1` per trip):

* **status 3**: flags |= 8; `t := r24(float(count) × tripLength)`; `acc := r24(acc + t)`; `n += count`. Unless probe:
  commuters (type < 3): `attempted[type] += count`, `routed[type] += count`, `carLenSum := r24(r24(float(count) ×
  tripLength) + carLenSum)`, `carLenCount += count`; freight: `freightLenSum := r24(freightLenSum + t)`,
  `freightCount += count`; for each bit `i` of `travelTypesUsed` (0…8): `typeCount[i] += count`.
* **status 5**: flags &= ~8; `failed := true`; `t := r24(float(count) × commuteMaxTime)`; `acc`, `n` as above;
  commuters (not probe): `attempted[type] += count`; freight (probe or not) *[quirk]*: `freightLenSum`,
  `freightCount` as above.
* other status: flags &= ~8.

If `n ≠ 0`: `avg := r24(acc / float(int32 n))`. If the source rect's first corner is in the city, the trip-length map
is filled over the whole source rect with

`v := failed ? 0 : clamp(round(r24(r24(avg / commuteMaxTime) × 255.0f)))`, clamp: `> 255` (unsigned) → 255,
`0` → 1.

(`255.0f` = `0x00A9D8B0`, dword.) A lot with any failed trip therefore shows **0** ("no commute"; the growth
developer's commute layer ignores 0, growth-developer.md §4.3) *[compat]*. Otherwise (off-map source) the first
neighbour connection found at that point (neighbour simulator slot 11 `FindConnection(type, x, z)` for type 0…12)
gets property `0xCA947077 := avg` (float).

### 4.7 FinishTripMap (state 15, `0x0070FEF0`)

For each lot (lot iterator): clear the lot's flag `0x80` (slots 12/13). For residential lots that are not
"traffic sources of their own" (zone 8 or 9 *[inferred: these are the zone values tested]* and slot 28 false), the
trip-length value is borrowed: from the cell of the lot named by slot 59 if any; else by searching outward along
the four directions (−x, −z, +x, +z, distance 1, 2, …; each direction stops when it leaves the city, finds no lot or
a lot of another zone) for a lot of the same zone with slot 28 true; its trip-length value is written over the
lot's rect.

### 4.8 Edge densities and transit switches (states 3, 7, 6; slot 12)

`TrafficEdgeDensity` = 33 u16: `last[16]` (index `4·entry + exit`), `usage`, `current[16]` (the SDK's layout of
`0x42` bytes is correct). Keys `evening << 24 | t << 16 | x << 8 | z` (`evening` = the mode-1 / mode-2-mirror
record).

* **Dump** (state 7): for every record `last := current`, `usage := min(0xFFFF, Σ current)`.
* **Reduce** (state 6): `current := 0` for all 9 travel types and both evening values at every cell with network
  info or a transit switch, and at the ferry terminal cells. Records are erased only when the network occupant is
  removed (§3.1).
* **GetTrafficEdgeDensity(x, z, t, evening)** (slot 12): the record, or a shared all-zero record (`0x00AB31D0`) when
  missing. The returned pointer must not be written by callers.
* **ReduceTransitSwitches** (state 3, `0x00713750`): for every transit switch, in table order, `load :=
  r24(densityReduction × load)` (`0x29136788`, the same factor as the volume reset of §4.9 step 3). Nothing
  else: the two saved counters keep what CreateMaps (state 2, earlier in the same cycle) wrote, `prevUsage := load`
  and `ratio := min(load / capacity, 1)` (§4.9 step 4). Finishes in one call. With the shipped factor 0 the load
  restarts from 0 for the coming trip phases (`0 × NaN` and `0 × ±∞` stay NaN; a negative load would become −0.0).
  With a non-zero factor the load decays instead *[compat]* (TS3).
* **GetTravelTimeRatio(x, z, t)** (slot 24, `0x007103B0`): 1.0 if `affectedByTraffic[t]` is false. Else
  `u := Σ` over travel types `i` with `generatesTraffic[i]` of `usage(x, z, i, 0) + usage(x, z, i, 1)` (u32), and with
  the cell's network info for `network(t)` (none → 1.0): `congestionCurve(r24(float(u) × invCapacity))`.

### 4.9 CreateMaps (state 2, `0x0071A0C0`)

Per cell (x innermost), with `B` = peak volume grid, `D` = peak ratio grid:

1. `B := 0`, `D := 0`.
2. If `x mod 4 = 0` and `z mod 4 = 0`: `commercialWork[x/4][z/4] := min(255, round(r24(float(v) × commercialScale)))`,
   where `v` is the commercial traffic window value of block `(bx, bz) = (x/4, z/4)` (`0x00714AA0`, TS8). Rounding is
   to nearest, ties to even: the f32 magic-number trick `+ 12582912.0f`, `0x0071A271`. Definitions:
   * **Cell value** `cv(x, z, k)` (`0x00714940`), `k` = the `evening` byte of §4.8 (0 or 1), with `usage(x, z, t, k)`
     = the `usage` field of the edge-density record (§4.8; 0 when there is no record). The cell value is the maximum
     (u16) of three per-network sums:
     * road: `usage(t = 1 Car) + usage(2 Bus) + usage(4 FreightTruck)`, summed in u16 (mod 2¹⁶);
     * el-train: `usage(7)`;
     * monorail: `usage(8)`.

     Travel types whose network (traffic-network.md §2.1, table `0x00AB31A8`) is pedestrian, rail or subway add
     nothing.
   * **Window** of block `(bx, bz)`, with `X0 = 4·bx` and `Z0 = 4·bz`: a plus shape, the 4 × 4 block widened by 2
     cells on each side in each direction, without the corners:
     * `z` runs from `max(0, Z0 − 2)` to `min(H, Z0 + 6) − 1`;
     * on the block's own rows (`Z0 ≤ z < Z0 + 4`), `x` runs from `max(0, X0 − 2)` to `X0 + 5`;
     * on the other rows, `x` runs from `X0` to `X0 + 3`.

     That is at most 48 cells.
   * `v := (maxM + maxE) mod 2¹⁶`, where `maxM` = the maximum over the window of `cv(·, ·, 0)` and `maxE` = the
     maximum of `cv(·, ·, 1)`. The two maxima are taken independently and may come from different cells. The sum
     is truncated to u16 by the caller.
   * *[quirk]* The `x` range on the block rows has **no upper clamp**, and the record key uses the low byte of `x`.
     So in a 256-wide city the east-most block column (`X0 = 252`) also reads cells `x = 0` and `x = 1` on the same
     rows: the west edge. In 64- and 128-wide cities, `x = W` and `W + 1` have no records and give 0. Keep it
     *[compat]*.

   Worked example, block (2, 3): `X0 = 8`, `Z0 = 12`. The window is rows 12–15 with x = 6–13, and rows 10, 11, 16, 17
   with x = 8–11 (48 cells).
   * Morning: at (13, 14), Car 300 + Bus 40 + FreightTruck 100 = road 440. At (9, 10), el-train 500. So `maxM` =
     500.
   * Evening: the largest cell value is 380, at (6, 12). So `maxE` = 380.
   * `v` = 880. With `commercialScale` 0.128: `r24(880 × 0.128)` = 112.64, so `commercialWork[2][3]` = 113.
3. For transit network `n` = 0…5 with network info `e` at the cell (`vol := volume[n]`):
   * `n ∉ {2, 3}` (not rail, not subway): `B := max(B, vol)`;
   * `r := r24(vol × e.invCapacity)`; `D := max(D, r)`;
   * accident list (road and rail, `n ∈ {1, 2}`, and the cell's connection count `> 3`):
     `p := accidentCongestionCurve(r)`; if `p > 0`, `p2 := r24(accidentCapacityCurve(r24(1 / e.invCapacity)) × p)`;
     if `p2 > 0`, the cell is added to the accident-prone list with probability `p2` (`0x00716950`);
   * per network type statistics (sum of `r`, count, maximum with its cell, count with `r > 1`) when the network
     info is not excluded *[inferred: flag meaning]*;
   * `volume[n] := r24(densityReduction × vol)` (shipped `0x29136788` = 0: volumes restart from 0 every cycle);
   * road (`n = 1`): `volume[1] := r24(volume[1] + GetBackgroundTraffic(x, z))` — background traffic therefore
     appears in the **next** cycle's maps.
4. Transit switch at the cell: `prevUsage := usage`; `ratio := usage / capacity`; `D := max(D, ratio)`;
   `switch.ratio := min(ratio, 1)`.
5. If any network info exists at the cell: `B := max(B, 1.0)`.

At the end: the network-type averages become the "last" statistics (slot 34), the cell counts per network type are
kept, `UpdateRoadsBudget` (`0x0070D8A0`) and `UpdateMassTransitBudget` (`0x0070DA40`) set the departments' monthly
costs from the cell counts and the per-tile costs `0x6A84493E` (budget.md), and the accident-prone list is copied
for the accident check.

**GetBackgroundTraffic(x, z)** (slot 11, `0x0070E870`): with `R`, `C`, `I` = the sums of the BDS developers'
population grid values (developer slot 15 `GetCellValue`, s16) at the cell for residential `0x1010, 0x1020,
0x1030`, commercial `0x3110, 0x3120, 0x3130, 0x3320, 0x3330`, industrial `0x4100, 0x4200, 0x4300, 0x4400`:
`r24(r24(r24(R × bg[0]) + r24(C × bg[1])) + r24(I × bg[2]))`, returned in ST0.

### 4.10 CreateFinalMap (state 4, `0x0070F460`)

Per cell with `b := B`:

* `b = 0`: air-polluting := 0, congestion := 0.
* else: air-polluting := `min(255, round(r24(b × 0.1f)))` (unsigned compare; `0.1f` = `0x00A8C950`);
  `c := congestionCurve(D)`; congestion := 1 if `c ≤ 1`; else `g := round(r24(r24(c − 1) × congestionScale))`,
  congestion := 255 if `g > 255` (unsigned), 1 if `g = 0`, else `g`.

`congestionScale := 255 / (lastY − 1)` of the (inverted) congestion curve (§4.14). Writes address `[x][z]` of the grid
data directly, assuming tract size 1 for both u8 grids *[quirk]*. Then posts `0x69247DC7` (TrafficMapChanged, §5).

### 4.11 ClearTripDestinations (state 5) and RefreshCommuteMap (state 10)

**ClearTripDestinations** (`0x0070E4F0`): per cell with a lot (`GetLot(x, z, true)`): `arrivedLast[k] :=
arrivedThisCycle[k]`, `arrivedThisCycle[k] := 0` (k = 0…3); `jobCapacity[k] += GetMaxTripCapacity(holder, k)` for
k = 0…2 *[inferred: once per lot cell, so multi-cell lots count once per cell]*.

**RefreshCommuteMap** (`0x0070F6E0`, despite the name it refreshes the **destination map**): per cell, the
destination byte `&= 0xF8` (bits 0–2 cleared, *[quirk]* bit 3 = freight destination is never cleared); with the lot
(`GetLot(x, z, true)`): bit `k` := `GetMaxTripCapacity(holder, k) ≠ 0` and `freeDest[k] += 1` for each set bit
(k = 0…3); else with `GetLot(x, z, false)`: bits only.

### 4.12 FinalCalculations (state 8), DoMonthlySimulation and history

**FinalCalculations** (`0x0070E7D0`): `transitIncome := float(incomeAcc)`, `tollIncome := float(feeAcc)`;
`avgCarCommute := r24(carLenSum / float(u32 carLenCount))` *[quirk: no zero check; 0/0 = NaN]*; if
`freightCount ≠ 0`, `freightByte := low8(ftol(r24(r24(r24(freightLenSum / float(freightCount)) / commuteMaxTime)
× 255.0f)))` (truncation, no clamp *[quirk]*), else 0. The next state is 0:

**StartRecalc** (state 0, `0x00717C90`): remembers the cycle start date (`GetSimDateNumber`); income and fee
accumulators := 0.0; `jobCapacity[k]`, `attempted[k]`, `routed[k]` := 0; `rerouteBucket := (rerouteBucket + 1) & 3`;
car and freight length sums and counts := 0; then `RecalculateNeighborConnectionCapacities` (`0x00714E60`,
traffic-network.md §2.7.4: regional job capacities per established neighbour pool, the default pool's freight
counter, and destination bits plus `freeDest` on the off-map edge cells; not a no-op, TS12). Finishes in one call.

**DoMonthlySimulation** (`0x00718AC0`), in this order:

1. `ApplyRoadFunding(f)` (`0x00717D30`), `f` = the roads department's funding fraction (department slot 10 returns a
   pointer to it), then `ApplyMassTransitFunding(f')` (`0x00717E00`) with the mass-transit fraction. Each keeps its
   own **damage rate** `d` (road, rail; saved):
   * `d := r24(damageCurve(r24(f × 100.0f)) + d)`, then `d := max(d, 0)`, `d := min(d, 1)` (NaN passes both);
     `damageCurve` = `0x0BD86FD4` for both kinds (shipped: +0.02 per month at 0 % funding, −0.01 at 100 %, −0.04 at
     120 %);
   * `target := low32(ftol(r24(float(u32 cellCount) × d)))` with `cellCount` = the number of road (resp. rail)
     cells kept by the network part;
   * if `target ≠ damagedCount`: `ApplyPotholeChanges(kind, target − damagedCount)` and `damagedCount := target`.
     Road only: then the published pothole count (u16 at offset `0x134` of the slot-34 block, §3.6) := the low 16
     bits of `damagedCount`. Rail writes no pothole count, and sets `damagedCount` before the call rather than after
     (the outcome is the same).

   **ApplyPotholeChanges(kind, delta)** (`0x00715FB0`; kind 1 road, 2 rail): the network part keeps, per kind, a
   set of intact cells and a set of damaged cells (SGI hash tables keyed by the u16 cell key `x << 8 | z`;
   traffic-network.md). For `delta > 0` cells move from intact to damaged, for `delta < 0` back; `|delta|` is capped
   at the source set's size. For each move: `r := RandomUint32Uniform(sourceCount)` on the simulator's generator;
   the `r`-th element of the source set **in its iteration order** (bucket index ascending, then chain order;
   SGI inserts at the head of a chain and grows the bucket array to the next prime of its prime list when the count
   would exceed it, re-linking old chains in order, each node at the head of its new chain) *[compat]*; if the cell
   has network info for transit network `kind`, its extra step cost := `damagedRoadExtraCost` (damaging) or 0
   (repairing) and message `0x69247DC8` is sent (data1 = x, data2 = z, data3 = 1 when damaged *[inferred: slot
   names, foundations §5.1]*); the key moves to the other set. `IsRoadDamaged` (slot 16) and `CheckRailAccident`
   (slot 17) test the damaged sets.
2. `UpdateStrikes` (`0x0070DBA0`), with `f` = the mass-transit funding fraction, `E` = the int64 mass-transit
   building expense total maintained by lot activation (§4.1.3; non-zero when the city has such buildings
   *[inferred]*), and the code-only constants `strikeStart` 0.7, `strikeKeep` 1.0, `strikeMaxMonths` 24,
   `strikeScale` 1.0 (no tuning property):

   | State | Condition | Result |
   |---|---|---|
   | 2 normal | `f < strikeStart` and `E > 0` (signed) | `p := r24(r24(strikeStart − f) × strikeScale)` (kept for the stats); `u := RandomDoubleUniform()`; if `p > u`: state 0, message `0x69247DC6` (strike starts) |
   | 2 normal | otherwise | nothing (no draw) |
   | 0 strike started | `f < strikeKeep` and `E ≠ 0` | state 1, month counter 0, "strike" flag := 1, message `0x69247DC4` |
   | 0 | otherwise | state 2, message `0x69247DC3` (strike over) |
   | 1 strike | `f < strikeKeep`, counter `< strikeMaxMonths`, `E ≠ 0` | counter += 1 |
   | 1 | otherwise | state 2, flag := 0, message `0x69247DC5` (strike over) |

   The pathfinder and `ConfirmRoute` treat state 1 as "on strike" (pathfinder.md, traffic-network.md).
3. `CheckForAccident` (`0x007184E0`, §4.13).
4. History (history warehouse slot 12 `+0x30`, `(id, u32)`): `0xEA5BA01D ← low32(ftol(r24(stats[0] × 100)))`,
   `0x8A5D4116 ← low32(ftol(r24(avgCarCommute × 25.0f)))` (25 hard-coded, `0x00AB32C0`), `0xABB8FAAC + i ←
   typeCount[i]` (i = 0…8), `0xEC0BDBF8 ←` the ferry total.

### 4.13 Accidents

**CheckForAccident** runs only when the accident timer (wall clock, ms) has passed the next check time; next check
:= now + `ftol(accidentCheckPeriod)` *[compat: wall-clock driven]*. It computes the visible cell rect from the view
(`0x00B43DD8` + `0x3EC`, world units / 16, rounded with the idiom and adjusted to the outer cells) and walks the
accident-prone list in the order of a **linear random iterator** (foundations §12.4, prime-step mode: one `rand()`
at reset). For each entry: draw `u := RandomDoubleRangeUniform(0.0, 1.0)` on the simulator's generator; if `u < p`
and the cell is in the visible rect and has an occupant (network-occupant query, `+0x1C` grid slot 14): create an
accident there (`cSC4TrafficAutomataManager::CreateAccident`, duration `ftol(r24(accidentDuration × 1000))` ms),
post the accident message `0x0C05E4C3` (§5; data1 = x, data2 = z of the entry, queued) and stop. The result of
`CreateAccident` is not checked: the message is posted even if the automata manager made no accident. *[choice]* FreeCity4 replaces the wall-clock period by the simulation clock
(the check runs once per `accidentCheckPeriod` seconds of game time at the current speed) and keeps the draw order.

**CheckRailAccident(x, z)** (slot 17, `0x00711590`): if the cell is in the damaged-cell set (hash at +0x804, key
`(x << 8 | z)` truncated to 16 bits, `x` the first argument): `u := RandomDoubleUniform()` on the simulator's
generator (+0x6FC, `cRZRandomSuper::RandomDoubleUniform` `0x00913A29`) and the result is
`railDamageAccidentFactor (+0xD04) > u`; else false, no draw. It posts **no** message; its result goes back to the
caller (train automata *[not traced]*).

**IsRoadDamaged(x, z)** (slot 16, `0x00720070`): membership of `(x << 8 | z)` in the damaged-road list
(traffic-network.md).

### 4.14 Tuning (`ReadSimulatorTunableValues`, `0x00716AE0`)

Exemplar `0x6534284A / 0xE7E2C2DB / 0xC9133286` from the resource manager; absent → nothing is read (constructor
defaults stay). Shipped values: `re/tuning/simulators.md` "Traffic Simulator". Order of reads *[compat]* below. Float
scalars use the f32 helper (unchanged if missing); "vector" helpers leave the target unchanged if the property is
missing.

| Property | Name | Code default | Shipped | Use |
|---|---|---|---|---|
| `0x491332E6` | Traffic Volume Per Population | — | 2 | read, unused |
| `0x491332E7 + t` | Max speed by network for travel type t (13 floats) | `FLT_MAX` inverse | see tuning | `invMaxSpeed[t][type] := 1 / r24(r24(v × 16.666666f) / cellWidth)` for `v > 0`, else `FLT_MAX` (class static `0x00B4B110`, pathfinder.md §4.1); only the first min(count, 13) |
| `0x29136788` | Monthly Traffic Density Reduction | — (uninitialised *[quirk]*; FreeCity4 0) | 0 | `densityReduction` (§4.9) |
| `0x29136789` | — | 0.03125 | — | read, unused |
| `0x6A84493E` | Monthly cost for network tile [13] | 0 | see tuning | department costs (§4.9) |
| `0xA92356AE`, `0xA92356AF` | Max roads / mass transit funding percent | 1.2, 1.2 | 120, 120 (→ 1.2) | department maximum funding |
| `0xA92356B0` | Damaged road extra step cost | 32.0 | 0.1 | traffic-network.md |
| `0xA92356B6` | strategy period (u32) | 365 | — | §3.3 |
| `0xA92356B7` | strategy buckets (u32) | 128 | — | §3.3 |
| `0xA92356B8` | Maximum distance from origin to network (u32) | 1 | 1 | pathfinder.md |
| `0xA92356B1` | Income per tile by travel type [9] | 0 | see tuning | §4.5 |
| `0xA92356B3` | Network Traffic Capacity [13] | 0 | see tuning | capacity and `invCapacity = 1/cap` (1.0 when `|cap| ≤ 1e-10`, double constant), traffic-network.md |
| `0xA92356B4` | Travel type generates traffic [9] | true | see tuning | §4.5 |
| `0xA92356BF` | Travel type affected by traffic [9] | true | see tuning | class static `0x00B4B100` |
| `0xA92356B5` | Travel type can reach destination [9] | true | see tuning | pathfinder.md |
| `0xA92356B9` | Congestion vs Speed (curve) | — | (0,1)(1,1)(2,0.65)(3,0.3) | class static curve `0x00B4B2F0` (below) |
| `0xA92356BA` | Commute trip max time | 90.0 | 6 | `commuteMaxTime` (max trip length, trip-length scale; slot 14) |
| `0xA92356BB` | Intersection and Turn Capacity Effect [n] | — | 0.7, 0.8, 0.9 | reciprocals stored; traffic-network.md |
| `0xA92356BC` | Trip Starting Cost by travel type [9] | 0 | 0, 0.4, 0… | neutral start costs, also the default array |
| `0xEA8C3CDB` | … for Mass Transit [9] | 0 | 0, 1.95, 0… | strategy 0 |
| `0xCAD64136` | … for Car Pref [9] | 0 | 0.1, 0… | strategy 1 |
| `0xA92356BD` | Job Scaling Constant | 1.0 | 1.2 | traffic-network.md *[inferred]* |
| `0x49A2E8BE` | Freight traffic scaling factor | — (uninitialised; FreeCity4 0) | 0.05 | slot 27, §4.1.1 |
| `0x4A678060` | Nearest Destination Attractiveness | 0.75 | 0.09 | pathfinder.md |
| `0xCA76013B` | Trip Length to Minutes Display Multiplier | 0.5 | 25 | slot 15 |
| `0xEA7B5F06` | Max Mass Transit Strategy Trip Length | 4.0 | 4 | *[inferred: not found in a read path]* |
| `0xA92356BE` | Population Background Traffic [3] | — (missing entries 1.0) | 0.05, 0.2, 0.2 | `bg[0…2]`, §4.9 |
| `0xCA5F7821` | Transit Switch Entry Cost vs. Budget (curve) | (0,0)(1,1) | (0,2)(100,1) | multiplier, §3.1 |
| `0x4BE09EC9` | Congestion to Accident Probability (curve) | (0,0)(0.25,0)(1,0.05)(2,0.25) | same | §4.9 |
| `0x4BE09ECA` | Capacity to Accident Probability (curve) | (0,1)(1000,1) | same | §4.9 |
| `0x4BE09ECB` | Accident Duration | 10 | 20 | §4.13 |
| `0x4BE09ECC` | Accident Check Period | 60 | 60 | §4.13 |
| `0x0BD86FD4` | Funding to Damage Acceleration Curve | (0,0.02)(100,−0.01)(120,−0.04) | same | traffic-network.md |
| `0x0BD86FD5` | Rail Damage Accident Factor | 1.0 | 1 | §4.13 |
| `0x8C3CAEF5` | Ferry Fare | 0.001 | 0.01 | §4.5 |
| `0x0CAAAEB8` | (commercial traffic scale) | 0.128 | 0.128 | `commercialScale`, §4.9 |

Then `UpdateTravelStrategies` (§4.1.2). `0xA92356B2` (Mass Transit Usage Chance) is not read by this function.

**Shipped "Max speed by Network"** (`0x491332E7 + t`, 13 floats each). Source: the Traffic Simulator exemplar
`0xC9133286` in `SimCity_1.dat` (dump `re/tuning/simulators.md`). Columns are the network types in index order. A
blank cell is 0, which gives `invMaxSpeed = FLT_MAX`: the type cannot use that network, and a tunnel's cost is +∞.

| t | Travel type (property) | Road | Rail | Highway | Street | Avenue | Subway | El | Monorail | OneWay | GroundHwy |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Walk (`…E7`) | 3.5 | | | 3.5 | 3.5 | | | | 3.5 | |
| 1 | Car (`…E8`) | 31 | | 82 | 21 | 40 | | | | 31 | 82 |
| 2 | Bus (`…E9`) | 46 | | 100 | 31 | 60 | | | | 46 | 100 |
| 3 | Passenger train (`…EA`) | | 110 | | | | | | | | |
| 4 | Truck (`…EB`) | 31 | | 82 | 21 | 40 | | | | 31 | 82 |
| 5 | Freight train (`…EC`) | | 150 | | | | | | | | |
| 6 | Subway (`…ED`) | | | | | | 150 | | | | |
| 7 | El train (`…EE`) | | | | | | | 150 | | | |
| 8 | Monorail (`…EF`) | | | | | | | | 200 | | |

Pipe (4), PowerPole (5) and **DirtRoad (11)** are 0 for every travel type, so they have no columns here. With
CellWidthX 16, `invMaxSpeed` is:

| v | 3.5 | 21 | 31 | 40 | 46 | 60 | 82 | 100 | 110 | 150 | 200 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bits | `0x3E8C6F2E` | `0x3D3B3EE7` | `0x3CFDB011` | `0x3CC49BA7` | `0x3CAAF6A7` | `0x3C83126F` | `0x3C3FD00C` | `0x3C1D4952` | `0x3C0EFCD6` | `0x3BD1B717` | `0x3B9D4952` |

For example, Walk on Road (3.5) gives 0.27428573, and Car on Road (31) gives 0.030967744. The constant is
`16.666666f` = `0x41855555`.

**Curves** (foundations §4.1 patterns):

| Curve | Pattern | Notes |
|---|---|---|
| Congestion vs Speed `0xA92356B9` | **keep** (`0x005F4700`), then transformed | after the load each point's y is replaced by `r24(1 / y)` and the curve is rebuilt from the new points (`0x005F4480`); `congestionScale := r24(255 / r24(lastY − 1))`. *[quirk]* A reload with the property missing inverts the kept curve again; an empty curve reads before its point vector (crash). FreeCity4: keep the previous curve unchanged when the property is missing, and treat an empty curve as "speed factor 1" with `congestionScale` 255 *[choice]*. |
| `0xCA5F7821`, `0x4BE09EC9`, `0x4BE09ECA`, `0x0BD86FD4` | **default** | rebuilt from the code default above, then loaded when present (`0x005F4620`) |

Shipped congestion curve after inversion: (0, 1), (1, 1), (2, 1.5384616 `0x3FC4EC4F`), (3, 3.3333333 `0x40555555`);
`congestionScale` = 109.28572 (`0x42DA924A`).

### 4.15 Random draws *[compat]*

The simulator owns one `cRZRandomSuper` (MT19937 with the non-standard seeding of foundations §6.1), seeded at Init
from the wall clock. *[choice]* FreeCity4 seeds it from the host-supplied seed for "traffic simulator" (foundations
§6.3).

| # | Operation | Generator | Method (advances) | Arguments | Position |
|---|---|---|---|---|---|
| 1 | strategy re-draw (§3.3) | simulator's `cRZRandomSuper` | `RandomUint32Uniform(100)` (1 word) | — | once per lot of the day's bucket, in multimap order, before any cycle work of that day |
| 2 | `DoLotActivated` (§4.1.3) | same | `RandomUint32Uniform(100)` (1) | — | after the bucket registration; only when the lot is found again and the building's `0xE90E25A1`, if present, has a byte size that is a multiple of 4 |
| 3 | `CheckForAccident` (§4.13) | C `rand()` (one call at iterator reset), then the simulator's generator | `RandomDoubleRangeUniform(0.0, 1.0)` (1 word per visited entry) | doubles 0.0, 1.0 | per accident-prone entry visited, until the first accident |
| 4 | `CheckRailAccident` (slot 17) | same generator | `RandomDoubleUniform()` (1) | — | only for damaged cells; caller-driven (train automata) |
| 6 | `ApplyPotholeChanges` (§4.12) | same generator | `RandomUint32Uniform(sourceCount)` (1 per moved cell) | current size of the source set | monthly: road moves first, then rail |
| 7 | `UpdateStrikes` (§4.12) | same generator | `RandomDoubleUniform()` (1) | — | monthly, only in the normal state with `f < 0.7` and `E > 0`; after the pothole draws, before the accident check |
| 5 | `cSC4Lot::Init` strategy byte | the lot file's `cRZRandomSuper` (`0x00B49120`) | `RandomUint32Uniform(100)` (1) | — | only when the byte is `0xFF` (new lot) |

Draws 1, 2 and 5 only write values nobody reads (§4.1.1) but 1 and 2 advance the shared generator, so they matter
for the sequence of draws 3, 4, 6 and 7. The month order is fixed: pothole draws (road, rail), strike draw, accident
draws. No other draw exists in this class: trip generation, routing and the maps are deterministic, and the network
part draws nothing (traffic-network.md §12).

### 4.16 Station query: GetTransitSwitchQueryData(x, z, out) (slot 35, `0x0071CB80`) *[compat]*

`out` is five 32-bit fields, all written on success (TS11). The names are those of the query window's readers in
the Mac build (`cSC4ViewInputControlQuery::SetOrGetTransitStation…`):

| Field | Meaning (query reader) | Transit switch at the cell | Ferry terminal in the lot at the cell |
|---|---|---|---|
| +0x00 u32 | available capacity (service quality numerator) | `low32(ftol(capacity))`; 0 during a strike | riders last period; 0 during a strike |
| +0x04 u32 | maximum capacity | `low32(ftol(capacity))`, **also during a strike** | riders last period; 0 during a strike |
| +0x08 f32 | load fraction (no query reader) | the switch's `ratio = min(load / capacity, 1)` written by the last CreateMaps (§4.9 step 4); 0.0 during a strike | 1.0; 0.0 during a strike |
| +0x0C u32 | actual usage | `low32(ftol(prevUsage))` (the load seen by the last CreateMaps); 0 during a strike | riders last period; 0 during a strike |
| +0x10 u32 | monthly maintenance cost | cost (below), whatever the strike | cost (below), whatever the strike |

"During a strike" is the mass-transit strike state 1 (§4.12), for both kinds. `capacity` is the switch's building
property `0xE90E25A3`; "riders last period" is the terminal's u16 rolled on entering state 19 (traffic-network.md
§2.6). `ftol` truncates toward zero.

1. `id :=` the switch-ID grid at `(x, z)` (grid slot 7). If `id ≠ 0`: the switch is looked up with insert-if-missing
   (as traffic-network.md §4.7.3), the fields are filled from it, and the cost is taken from its building. Return
   **true**.
2. Else `lot := GetLot(x, z, false)`; none → return **false** (`out` untouched). Walk the ferry terminals in list
   order; the first whose lot cell (x, z) lies inside the lot's bounding rect (lot slot 14, bounds inclusive) is
   used, the cost taken from the lot's building (lot slot 48; no building → the cost stays 0). Return **true**. No
   such terminal → return **false** (`out` untouched).

**Cost**: `holder :=` the building's property holder; the budget simulator's `GetBudgetItemForPurpose(holder,
0xCA550301, item)` (slot 51) gives the building's first budget item with that purpose (budget.md §2); then
`dept := GetDepartmentBudget(item.department)` (slot 28). When both exist, cost `:= low32(ftol(r24(float(item.cost)
× f)))`, `item.cost` the item's int64 monthly cost at 100 % and `f` the department's funding fraction (department slot
10, a pointer to the fraction). Otherwise 0.

The query window then shows: service quality `0` if the call fails or +0x00 is 0, `1` if +0x0C is 0, else
`min(+0x00 / +0x0C, 1)`; usage percent `+0x0C / +0x04 × 100` (0 when +0x04 is 0); maximum capacity +0x04; usage
+0x0C; maintenance +0x10 *[inferred: from the Mac readers; the Windows readers were not traced]*.

*[choice]* A switch ID in the grid with no switch behind it (only possible from a damaged save) makes the original
insert a default switch with no building and crash on the cost step. FreeCity4 returns false and leaves `out`
untouched.

**Example.** A subway station with capacity 2500.0, last cycle's load 3100.0 (so ratio 1.0), no strike, budget item
cost 150 for purpose `0xCA550301` in a department funded at 0.9: `out = {2500, 2500, 1.0f, 3100, 135}`
(`r24(150 × 0.9) = 135.0`, truncated 135). The window shows service quality `min(2500/3100, 1) = 0.80645`, usage
124 %. During a strike: `{0, 2500, 0.0f, 0, 135}`.

## 5. Messages posted

| Message | When | Data |
|---|---|---|
| `0x69247DC7` TrafficMapChanged | end of CreateFinalMap (state 4) | `cRZMessage2Standard`: type, void1 and IGZUnknown = the simulator; **posted, queued** (message server `+0x10` = `MessagePost(msg, false)`, `0x0070F6C0`) |
| `0x0C05E4C3` traffic accident *[inferred name]* | `CheckForAccident` (§4.13), right after `CreateAccident` (`0x0071877E`) | new `cRZMessage2Standard` (`0x2C` bytes); `SetType(0x0C05E4C3)`, `SetData1(x)`, `SetData2(z)` (the accident-prone entry's cell, words +0 and +4); data3, data4, void1 and IGZUnknown are not set (0). **Posted, queued, normal priority** (`MessagePost(msg, false)` on `0x00B43CCC`, call at `0x007187D7`; skipped when the server is null), then released. Receivers: **none compiled**: the constant appears once in the image (`0x007187B2`, also only at the poster in the Mac build), so only wildcard or plug-in subscribers see it. Rail accidents post nothing (`CheckRailAccident` returns a bool). The advisor events CAR_CRASH `0xABF4B4B2` / TRAIN_DERAIL `0xABF4B57F` have no code reference to this path *[not traced]* |
| `0x69247DC3`, `0x69247DC4`, `0x69247DC5`, `0x69247DC6` | strike transitions (§4.12) | type, data1 (`+0x30` void1) and IGZUnknown = the simulator; posted, queued, through the simulator's message server (`+0x10` = `MessagePost(msg, false)`, `0x0070DD18`) |
| `0x69247DC8` | a cell becomes damaged or repaired (§4.12) | x, z, damaged flag |
| `0x2C06BA82` | the ferry pass of state 19 has finished all terminals | traffic-network.md §5 |

## 6. Persistence

### 6.1 Record (`Write` `0x00712400`, `Read` `0x00721990`) *[compat]*

Version **0x29** (41). Read accepts versions ≥ `0x13` and returns false below that; the version gates are listed
in §6.4. Primitive encodings are foundations §2.2.

**Corrected (M6):** the authoritative byte layout, with every Read version gate, is save-records.md §7.1.1; it
parses all 417 corpus records. The table below is kept as a field overview; where it differs, save-records.md
wins. The differences (§7.1.2 there): the grids (rows 3, 5, 6, 9), the switch building (row 10) and the
departments (row 11) are **refs** (ref writer `0x005D99A0`, 14 calls in `Write`), not inline objects; row 14 holds
a fixed `4 × u32` and a `3 × D` u32 table with no count fields; row 17 is the **tunnel table**, not the
neighbour-connection records; in row 18 the `u32 4` is a count; in row 19 the usage table is a map from u16 to a
property holder, the two per-travel-type vectors are interleaved pairs, and the link counters come after the route
vectors.

| # | Field | Encoding |
|---|---|---|
| 1 | version | u16 = `0x29` |
| 2 | dimension counts | s32 13, s32 1, s32 13, s32 9 (network types, —, network types, travel types *[inferred: names]*) |
| 3 | grids | air-polluting, peak volume, commercial traffic, commercial work, trip length, congestion (6 grids) |
| 4 | destination map | u32 `W + 2`, u32 `H + 2`, then `(W+2)(H+2)` u8 in `[x][z]` order (x outer) |
| 5 | volume grids | u32 count (6), then each grid |
| 6 | float grids | `0x69D5C3FA` grid, peak ratio grid |
| 7 | edge densities | u32 count; per record: u32 key, `last[16]` u16 (index order 0…15), u16 usage, `current[16]` u16 |
| 8 | per trip type k = 0…2 | u32 `jobCapacity[k]`, u32 `attempted[k]`, u32 `routed[k]` |
| 9 | network grids | transit-switch mode-mask grid (`0x69D5C424`), transit-switch ID grid (`0x69D5C42D`) |
| 10 | switch block (traffic-network.md §2.4, §6) | u32 next switch ID, u32 count, then per switch in table iteration order: u32 ID, building reference (`SetGZSerializable`), f32 load, f32 time factor, f32 capacity, bool has-access-point, u8 access x, u8 access z, f32, f32 (the two counters), u32 rule count, per rule: u8 kind, 4 × bool side, u16 from, u16 to; then f32 (the `0x0BFC0A4C` value *[inferred: pairing]*), bool counts-as-intersection |
| 11 | departments | roads, mass transit (serializables, budget.md) |
| 12 | income | s64 budget-item total (§4.1.3), f32 transit income, f32 toll income, f32 last monthly income, f64 income accumulator, f64 fee accumulator |
| 13 | cycle date, strike, damage | u32 cycle start date (§4.12 StartRecalc), u8 strike state, s32 strike month counter; road: u32 cell count, u32 damaged count, f32 damage rate; u32 n + n × u16 intact road cell keys, u32 n + n × u16 damaged road cell keys (hash iteration order); rail: the same five fields and two key lists |
| 14 | cycle position | u8 cycle state, the scanning iterator (helper `0x005D9FD0`/`operator<<`), bool intersection-dirty flag, s32 × 4 intersection dirty rect, bool network-changed byte (§3.1; never read), u32 strategy bucket counter, u32 strategy bucket, s16 × 2 phase positions, the lot iterator position (helper `0x005DA390`), bool, bool, f32, s32 x, s32 z (re-route context), u32 n + n × u32, u8, u32, s32 × 4, u32 n + n × u32 (per entry), u8 × 2 (context flags) |
| 15 | pacing and counters | s32 processed, s32, s32 total, u32 days, u32 day index, u32, u32, s32 `passTotal`, s32 `toReRoute`, s32 `toSearch`, then u32 × 10 pass counters (routed, re-routed, `rerouteBucket`, source counter, …; §10 Q4) |
| 16 | network-type statistics, two sets (published "last" set = slot 34 block, then the working set) | per set: 13 × (f32 sum, u32 count, s32 max x, s32 max z, f32 max), the aggregate (same 5 fields), u16 × 13 (over-capacity counts), u16, u16, s32 12, 12 × (u16, f32, s32, s32, f32) per-switch-type records, f32, f32, bool, f32, u8 |
| 17 | neighbour-connection records | u32 count, per record (traffic-network.md §2.7): u16, u8, u8, u16, u16, then its f32 values |
| 18 | commute | f32 `avgCarCommute`, f32 `carLenSum`, u32 `carLenCount`, f32 `freightLenSum`, u32 `freightCount`, u32 × 13 cells per network type, u32 4, u32 × 4 `freeDest` |
| 19 | tail | u32 count + records of the neighbour pool map (per pool: u16 ID, then its property-holder record; traffic-network.md §2.7.3), the per-travel-type count vectors (u32 × 9 each) and the ferry total, then the **ferry terminals**: u32 count, per terminal u8 × 6, u8, u16 × 3, u8 × 2 (kind, cell, counters), u32 link count + per link (f32 cost, s16 x, s16 z, u16, u16 × 4 counters, route bytes `vector<u8>`), u32 neighbour-link count + per link (f32, s16 × 4, u16 × 4, two route vectors); Read rebuilds the terminal tables from them (`0x00713DF0`) *[inferred: field grouping]*; traffic-network.md §6 agrees |

Rows 14–19 are partly given by type sequence; the meaning of each unnamed scalar is either traffic-network.md's or an
internal counter that FreeCity4 writes back unchanged. The full write order with addresses is in the evidence file named in
§10 Q5. FreeCity4 writes version `0x29` only.

### 6.2 What must survive a save

The six published grids (consumers read them right after load), the volume grids (the next CreateMaps uses them),
the trip-data lists (in the lots, §6.3), edge densities (traffic automata and the query tool), income accumulators
and monthly income, `freeDest`, the destination map, damage sets and strike state, the transit switches, the ferry
terminals, and the cycle position. Everything else is rebuilt within one
cycle.

### 6.3 Trip data in the lot record

Each lot record (`cSC4Lot::Write` `0x006C0080`) ends with: u32 trip count, the trips, then u8 strategy byte. Each
trip (`cSC4TripData` Write `0x0070E0B0` / Read `0x0070E160`): u32 version (2), u32 n + n × u8 route, u32 n + n × u8
returnRoute, u32 count, u8 type, u8 strategy, u8 flags, u8 status, s16 destX, s16 destZ, f32 tripLength, u32
travelTypesUsed (only when version > 1; 0 otherwise). Read accepts any version; for a vector count ≥ 1,000,000
nothing more is read and the vector keeps its previous contents (corrected, `0x005DB520`) *[compat]*. The lot's
Read **appends** the trips to its existing list, and its Write stops before the strategy byte when a trip Write
fails (`0x006C35F0`, `0x006C0334`; lot-records.md). Fields the original never initialises are saved as they are:
`travelTypesUsed` is `0xCCCCCCCC` in 126 of 8,109 saved trips and destZ holds −13,108 (`0xCCCC`) in some, so a
writer must copy loaded values back unchanged to round-trip a city. Neighbour connections save their lists the same
way (neighbors.md).

### 6.4 Version gates (Read)

The reader branches on the version at these points (`m5sim/read_trace.txt` evidence): `0x18`, `0x25`, `0x1C`, `0x1C`
(the dimension counts, defaulting to 8, 1, 4, 7 for older saves), `0x14`, `0x1D`, `0x17`, `0x1C`, `0x20`, `0x15`,
`0x1A`, `0x19`, `0x27`, `0x23` (×2), `0x25` (×2), `0x22`, `0x14` (×2), `0x1A`, `0x26`, `0x1D`, `0x20`, `0x25`,
`0x23`, `0x1E`, `0x23`. FreeCity4 reads **every version from `0x13` to `0x29`** with these gates (corrected: 403 of
the 417 corpus saves hold version 19; the gate-by-gate layout is save-records.md §7.1.1) *[compat]*. Skipping an
older record is not an option: the six published grids are separate objects that would be left unreferenced, and
the destination map, damage and switch state would be lost. An old record is written back as version `0x29`; a
byte-exact round trip is expected only for versions 40 and 41 (save-records.md §7.1.4).

## 7. Interface: cISC4TrafficSimulator (vtable `0x00AB33D0`, 44 slots)

All slots `__thiscall`. "pop" = bytes popped by the callee (checked with `tools/abi_extract.py`, docs/abi
interfaces.tsv); return channel read from the epilogues. AL = bool/byte (upper EAX garbage); x87 = ST0.

| Slot | Off | Address | Signature (as compiled) | pop | Return | Notes / SDK |
|---|---|---|---|---|---|---|
| 0 | +0x00 | `0x0070D7E0` | `bool QueryInterface(uint32, void**)` | 8 | AL | §1 |
| 1 | +0x04 | `0x004F7BA0` | `uint32 AddRef()` | 0 | EAX | shared |
| 2 | +0x08 | `0x0070E2F0` | `uint32 Release()` | 0 | EAX | §1 |
| 3 | +0x0C | `0x0071E7E0` | `bool Init()` | 0 | AL | |
| 4 | +0x10 | `0x00719290` | `bool Shutdown()` | 0 | AL | |
| 5 | +0x14 | `0x0071FFA0` | `uint32 GetSimulatorType()` | 0 | EAX | `0x88027F0D` |
| 6 | +0x18 | `0x0070D860` | `bool CreatePathFinder(cISC4PathFinder**)` | 4 | AL | pathfinder.md §1 |
| 7 | +0x1C | `0x00711610` | `bool SetupPathFinderForLot(cISC4PathFinder*, cISC4Lot*)` | 8 | AL | §4.4 |
| 8 | +0x20 | `0x006AAC50` | `cISC4SimGrid<uint8>* GetAirPollutingTrafficMap()` | 0 | EAX | folded getter |
| 9 | +0x24 | `0x0080BC00` | `cISC4SimGrid<uint8>* GetCommercialTrafficMap()` | 0 | EAX | tract 4 |
| 10 | +0x28 | `0x00698CF0` | `cISC4SimGrid<uint8>* GetCongestionMap()` | 0 | EAX | |
| 11 | +0x2C | `0x0070E870` | `float GetBackgroundTraffic(int32 x, int32 z)` | 8 | x87 | **SDK error**: declared `intptr_t` |
| 12 | +0x30 | `0x00711460` | `TrafficEdgeDensity* GetTrafficEdgeDensity(uint8 x, uint8 z, uint32 travelType, bool evening)` | 16 | EAX | §4.8; never null |
| 13 | +0x34 | `0x00820250` | `cISC4SimGrid<uint8>* GetTripLengthMap()` | 0 | EAX | |
| 14 | +0x38 | `0x0071FFB0` | `float GetTripScale()` | 0 | x87 | = `commuteMaxTime` |
| 15 | +0x3C | `0x0071FFC0` | `float GetTripScaleForDisplay()` | 0 | x87 | = display multiplier (25 shipped) |
| 16 | +0x40 | `0x00720070` | `bool IsRoadDamaged(int32 x, int32 z)` | 8 | AL | §4.13 |
| 17 | +0x44 | `0x00711590` | `bool CheckRailAccident(int32 x, int32 z)` | 8 | AL | RNG draw (§4.15) |
| 18 | +0x48 | `0x0071FFD0` | `float GetLastMonthlyIncome()` | 0 | x87 | tolls + transit (§3.3) |
| 19 | +0x4C | `0x0070DF70` | `void SetMaxTripCapacity(cISCPropertyHolder*, uint32 k, uint32 value)` | 12 | none | **SDK error**: declared `bool`; EAX is garbage |
| 20 | +0x50 | `0x0070DF90` | `uint32 GetMaxTripCapacity(cISCPropertyHolder*, uint32 k)` | 8 | EAX | §4.2 |
| 21 | +0x54 | `0x0070E050` | `uint32 GetTrafficArrived(cISCPropertyHolder*, uint32 k)` | 8 | EAX | arrived **last** cycle |
| 22 | +0x58 | `0x00720010` | `cISC4Lot* GetDesiredLotInsertionPoint()` | 0 | EAX | traffic-network.md; **SDK**: `uint32` is a pointer |
| 23 | +0x5C | `0x00710320` | `float GetCapacity(uint32 transitNetwork, int32 x, int32 z)` | 12 | x87 | `1 / invCapacity` or 0.0; **SDK error**: declared `uint32_t` (and the first argument is the transit network 0…5, not a network type) |
| 24 | +0x60 | `0x007103B0` | `float GetTravelTimeRatio(int32 x, int32 z, uint32 travelType)` | 12 | x87 | §4.8 |
| 25 | +0x64 | `0x00710370` | `uint32 GetConnectionCount(uint32 transitNetwork, int32 x, int32 z)` | 12 | EAX | byte value; traffic-network.md |
| 26 | +0x68 | `0x00718BB0` | `void GetTravelStrategyPercentages(uint32 wealth, std::vector<float>* out)` | 8 | none | **SDK error**: declared `bool` with `SC4Vector<SC4Percentage>` **by value**; the game passes a pointer to an STL vector (allocator B) and EAX/ST0 are garbage |
| 27 | +0x6C | `0x0071FFE0` | `float GetFreightScalingFactor()` | 0 | x87 | |
| 28 | +0x70 | `0x00714460` | `bool GetTransitSwitches(list&)` | 4 | AL | traffic-network.md |
| 29 | +0x74 | `0x00713B70` | `GetFerryRouteBetweenTiles(…)` | 16 | EAX | traffic-network.md |
| 30 | +0x78 | `0x00718C40` | `GetAllFerryRoutes(list&)` | 4 | AL | traffic-network.md |
| 31 | +0x7C | `0x0071C250` | `GetFerryRoutesInUse(list&)` | 4 | AL | traffic-network.md |
| 32 | +0x80 | `0x0071FFF0` | `uint32 GetFerryTerminalCount(uint32 type)` | 4 | EAX | |
| 33 | +0x84 | `0x0070EC10` | `GetWaterRoute(…, vector<uint8>&)` | 20 | AL | traffic-network.md |
| 34 | +0x88 | `0x00720000` | `const TrafficStats* GetTrafficStats()` | 0 | EAX | pointer to the statistics block (§4.9); **SDK**: `intptr_t` is a pointer |
| 35 | +0x8C | `0x0071CB80` | `bool GetTransitSwitchQueryData(uint32 x, uint32 z, TransitSwitchQueryData&)` | 12 | AL | §4.16 (five 32-bit fields); target lookup traffic-network.md |
| 36–41 | +0x90…+0xA4 | `0x00710540`, `0x00711A70`, `0x007106C0`, `0x00711B50`, `0x00718D20`, `0x00718F90` | AreLotsConnected, GetConnectedOccupantCount, GetConnectedDestinationCount, GetSubnetworksForLot, GetSubnetworksInRectangle, GetSubnetworksInRegion | 8 each | AL / EAX | traffic-network.md |
| 42 | +0xA8 | `0x007143B0` | GetOccupantCountForAllSubnetworks | 8 | | traffic-network.md |
| 43 | +0xAC | `0x007215A0` | scalar deleting destructor `(uint32 flags)` | 4 | EAX = this | not in the SDK; bit 0 frees with allocator C |

Other SDK remarks: `GetTrafficEdgeDensity`'s `TrafficEdgeDensity` (0x42 bytes, usage at `+0x20`) is correct; the
travel type enum is 32-bit. `GetTripDataList` on `cISC4TrafficSource` returns an STL `std::list<cSC4TripData>`
(allocator B), not an `SC4List`.

## 8. Dependencies

Virtual calls (slot, offset, pop, return):

| Interface | Slot (off) | Method | pop | Ret | Used for |
|---|---|---|---|---|---|
| `cISC4City` | 93, 94 (+0x174, +0x178) | cell counts | 0 | EAX | Init |
| `cISC4City` | (+0x16C) | cell width (float) | 0 | x87 | speed tables (§4.14) |
| `cISC4LotManager` | 6 (+0x18) | `GetLot(x, z, bool)` | 12 | EAX | everywhere |
| `cISC4LotManager` | 15 (+0x3C) | lot list / iterator | | | source iterator, FinishTripMap |
| `cISC4Lot` | 3 (+0x0C), 12/13 (+0x30/+0x34), 14 (+0x38), 21 (+0x54), 28 (+0x70), 31 (+0x7C), 37 (+0x94), 48 (+0xC0), 56 (+0xE0), 59 (+0xEC) | property holder, flags, bounding rect, location, road access, zone type, livable, building, travel population, lot link | | | §4.1, §4.7 |
| `cISC4TrafficSource` | 3, 4, 5 | §4.1 | 0, 0, 4 | | |
| `cISC4NeighborsSimulator` | 11 (+0x2C), 13 (+0x34), 16 (+0x40) | FindConnection, property holder, GetAllConnections | | | §4.1.4, §4.6 |
| `cISC4Simulator` | 6 (+0x18), 8 (+0x20), +0x4C | GetSimDate (5 pointers), GetSimDateNumber, AddAgent | | | §3.3 |
| `cISC4BuildingDevelopmentSimulator` | 32 (+0x80) | GetDeveloper(group) → developer slot 15 grid → GetCellValue (s16) | | | §4.9 |
| `cISC4ResidentialSimulator` | 30 (+0x78) | GetCellWorkforcePercent(x, z) | | x87/hidden | §4.1.1 |
| `cISC4OrdinanceSimulator` | 8 (+0x20), 14, 15, 20 | IsOrdinanceOn, count, array, by ID (+ ordinance slot 16 misc properties) | | | §4.1.2, §4.5 |
| `cISC4DepartmentBudget` | 10 (+0x28), (+0x50) | funding fraction, GetLineItem | | | §3.3 |
| `cISC4LineItem` | 15 (+0x3C) | SetIncome(int64) | 8 | none | §3.3 |
| history warehouse | 12 (+0x30) | add point (id, u32) | 8 | | §4.12 |
| message server | (+0x10) | send message (msg, 0) | | | §5 |
| `cISCPropertyHolder` | (+0x24) | GetProperty(id) | | | §4.2 |

Globals: city `0x00B43CEC`, lot manager `0x00B43D08`, simulator `0x00B43D5C`, BDS `0x00B43D68`, neighbours
`0x00B43D8C`, ordinance `0x00B43D90`, budget `0x00B43D64`, history `0x00B43DC0`, resource manager `0x00B43CA8`.

Pathfinder: every slot listed in pathfinder.md §7 that §4.4 names, plus the direct field writes of §4.4.

## 9. Replaceability

**Replacement unit** *[choice]*: the traffic simulator and the pathfinder (pathfinder.md) are one unit; each reads and
writes the other's members directly.

* **Raw readers of this class's memory** (simgrid.md §6.4): the pathfinder reads the simulator's members directly
  through its owner pointer (cell connection array, destination map and `freeDest`, volume-grid vector, transit
  switch grid, strike state, per-travel-type tables, start-cost arrays, `MaxDistanceOriginToNetwork`,
  `NearestDestinationAttractiveness`, the class statics) and writes the destination map, `freeDest` and the income
  double (pathfinder.md §8). The simulator in turn writes pf2's fields directly (§4.4). The two classes are one unit:
  **neither can be replaced alone** (pathfinder.md §9).
* Other modules use only the interface: grids through slots 8–10, 13 (growth developer, pollution, demand, aura, land
  value, map views), edge densities through slot 12 (traffic automata, query tool; the records are read as the
  33-u16 layout), capacities through slots 19–21, strategy percentages through slot 26 (lots), the pathfinder
  through slot 6.
* Lots (`cSC4Lot::UpdateTrafficVolume`) and neighbour connections are the other half of trip generation: a
  replacement simulator relies on their trip lists (allocator B) and must leave their format intact.
* Internal-only containers: the bucket multimap, accident-prone lists, edge-density and transit-switch hashes (but
  the record layout of slot 12 is public), per-type vectors, the source iterator's list (allocator A).

## 10. Quirks and open questions

Quirks reproduced: strategy byte 0 always picks strategy 0; percentage splits can lose or gain commuters; the
16-bit deficit for neighbour trips; uninitialised fields in new trips (FreeCity4 0); the free-destination count is
decremented even when the bit was already clear; bit 3 of the destination byte is never cleared by the refresh;
pf2's max trip length is a leftover; mode 2 mirrors edge densities (unlike `ApplyTraffic`); background traffic
lands one cycle late; a lot with one failed trip shows trip length 0; `avgCarCommute` divides by zero when nothing
was routed; the freight byte is not clamped; the final maps assume tract size 1; the congestion curve is inverted
again on a reload without the property; ordinance offsets accumulate without the exemplar; the 0x4A611773 value is
never read.

Open questions:

* **Q1** `UpdateStrikes`, `ApplyRoad/MassTransitFunding`, `UpdateRoads/MassTransitBudget` and
  `DoLotDeactivated` were not traced line by line *[inferred]*; their observable outputs (strike state, department
  costs) need differential tests.
* **Q2** The meaning of the slot-59 lot link and of zones 8/9 in FinishTripMap (§4.7).
* **Q3** *Answered* (§5): `0x0C05E4C3`, data1 = x, data2 = z, posted queued, no compiled receiver; rail accidents
  post nothing. Open: who calls `CheckRailAccident` and what the caller does with a true result.
* **Q4** Names of the 20 pass counters (sources, merges, confirmed, re-routed, failed, split, …) saved in §6.1 row 15;
  only `toReRoute`, `toSearch`, `passTotal`, `processed` and `rerouteBucket` drive behaviour.
* **Q5** Full save layout: rows 14–19 of §6.1 are given by type sequence (write trace of `0x00712400`, kept in the
  helper's scratch folder `m5sim/write_trace.txt`); traffic-network.md must name the network-owned fields.
* **Q6** `0x0CAAAEB8` has no name in the tuning file; its only use is the commercial traffic scale.
* **Q7** Lot activation (`0xE8BDA2E7`, posted only by the virtual `cSC4Lot::SetState` `0x006C3100`) drives the
  strategy-bucket registration, the destination bits, the transit switches and the ferry terminals. Whether loading a
  city re-sends it for every lot is the lot spec's question (traffic-network.md Q2). The bucket multimap is not in
  the save record, but DoPostCityInit rebuilds it from the lots' saved `0x691C863C` values (§3.5 step 12, TS7); the
  destination bits come back with the next RefreshCommuteMap.
* **Q8** The int64 total `E` that gates strikes (§4.12) comes from budget items of activated lots (properties
  `0xCA550301` / `0x0C3BF549`); its exact meaning needs the budget items' owner.

Corrections to `docs/subsystems/traffic.md` (Mac-derived): the lot's trip list is at a different place on Windows
(the interface, not the offset, is the contract); new trips are distinguished by **status 4**, not a flag; property
`0x4A611773` is never read; the inverse speed table is a class static filled from tuning (the member copy holds the
speeds); `GetBackgroundTraffic`, `GetCapacity` and `GetTravelStrategyPercentages` have the signatures of §7.

## 11. Conformance tests

Shipped tuning unless stated. Exact values are float32 bit patterns where given.

1. **Cycle order.** From state 0 with an empty city, successive finished phases visit
   `0, 16, 17, 18, 15, 2, 4, 5, 11, 3, 7, 6, 10, 12, 13, 19, 8, 0` (12 skipped when no subnetwork rebuild is
   needed). `DoDailySimulation` returns true on entering 17, 18, 2 and 11.
2. **Day pacing.** `passTotal = 100` at the start of ConfirmTrips: `days = 5`; with `trafficUnitsPerSlice` = 1 the
   first day ends after the first source that brings `processed ≥ 20`. `toReRoute = 17` → `days = 1`. With 10
   sources of 4 trips each (same `passTotal = 100`, shares 20, 40, …) and N = 3: day 1 runs slices of sources 1–3
   (processed 12 < 20, continue) and 4–6 (24 ≥ 20, day ends); day 2 runs sources 7–9 (36 < 40, continue), then
   source 10, after which the phase finishes and the day ends on entering 17. With N = 1 day 1 ends after source 5
   (processed 20).
3. **Congestion scale.** Shipped curve: inverted points `(2, 0x3FC4EC4F)`, `(3, 0x40555555)`; `congestionScale =
   0x42DA924A` (109.28572).
4. **Congestion map.** Peak ratio `D` → congestion: 0.5 → 1; 1.0 → 1; 1.5 → `c = 0x3FA27628` (1.2692308) → 29;
   2.0 → `c = 0x3FC4EC4F` → 59; 2.5 → `c = 0x401BE5BE` (2.4358974) → 157; 3.0 → 255; 4.0 → 255. Peak volume 0 → 0.
5. **Air-polluting map.** Peak volume 150 → 15; 15.0 → 2 (1.5 rounds to even 2); 25.0 → 2 (2.5 rounds to even 2);
   2.0 → 0; 3000 → 255.
6. **Trip-length map** (`commuteMaxTime` 6): average 3.0 → 128 (127.5 rounds to even); 0.6 → 26; 1.5 → 64; 6.0 →
   255; 7.0 → 255 (298 clamped); a lot with any failed trip → 0.
7. **Strategy percentages.** `pct = (80, 20, 0)` → `GetTravelStrategyPercentages` = `0x3F4CCCCC`, `0x3E4CCCCC`, 0.
   Lot with 30 commuters: thresholds 81, 101, 101; strategy byte 80 → strategy 0, 81 → strategy 1, 0 → strategy 0
   even with `pct = (0, 50, 50)`.
8. **Large lot split.** 200 commuters, `(80, 20, 0)` → 160, 40, 0. 75 commuters with `$$` `(30, 0, 70)` → 22, 0, 52
   (total 74: 22.499998 rounds down, 52.5 rounds to even 52).
9. **Carpool.** Count 10 → car `Q = 8.0`; 7 → 6.0 (`r24(5.6) → ceil`); 3 → 3.0; 1 → 1.0.
10. **Hand-checkable network** (pathfinder.md §11 tests 2–3 city: 8 × 8, street `(0,3)…(4,3)` with road network info
    `invCapacity 0.0625`, connection count ≤ 3, lot A at `(2,2)` = source, end cell `(4,4)` = job lot B with
    `max[0] = 100`; route bytes `01 02 03 41 02 0A`). Trip `{count 40, type 0, status 3}` confirmed with
    `ApplyRoute(mode 0)`:
    * road volume `(2,3)`, `(3,3)`, `(4,3)` = 40.0 (`0x42200000`);
    * edge densities (key bit 24 = 0, type 1): `(2,3)` `current[4·1 + 2] = 40`, `(3,3)` `current[4·0 + 2] = 40`,
      `(4,3)` `current[4·0 + 3] = 40`;
    * lot B `arrivedThisCycle[0] = 40`; income 0 (car income per tile 0);
    * with mode 2 instead: volumes 80.0, the same counts in the bit-24-clear records and `current[4·2 + 1]`,
      `current[4·2 + 0]`, `current[4·3 + 0]` = 40 in the bit-24-set records of the same cells;
    * CreateMaps on `(3,3)`: `r = 2.5`, peak ratio 2.5, peak volume 40 → congestion 157, air-polluting 4
      (`round(4.0000001)`); volumes reset to 0 (`densityReduction` 0);
    * CompleteTripSourceCalculations for A with `tripLength 3.0`: average 3.0 → trip-length 128 at `(2,2)`;
      `attempted[0] = routed[0] = 40`; `carLenSum = 120.0`, `carLenCount = 40` → `avgCarCommute = 3.0`, history
      `0x8A5D4116 ← 75`.
11. **Background traffic.** `R = 100`, `C = 10`, `I = 0` at a road cell: `r24(r24(100 × 0.05f) + r24(10 × 0.2f)) =
    7.0` (`0x40E00000`), added to the road volume after the reset, so the next CreateMaps sees 7.0.
12. **Seeded draws.** Simulator generator seeded with 1 (foundations §6.1 test 14: first words `0xD3F6B9E5`,
    `0xEF284157`, `0x0D3639C7`): strategy re-draw for a bucket with lots L1, L2, L3 writes `0x4A611773` = 82, 93, 5;
    `RandomUint32Uniform(100)` continues 13, 78, 34. With seed 12345: 81, 12, 25, 77, 2, 59. Seed 1,
    `RandomDoubleUniform` on the first three words (`r24(r24(i · 2^-32) + 0.5)`, `i` signed): 0.32798350
    (`0x3EA7ED74`), 0.43420801 (`0x3EDE5083`), 0.55160868 (`0x3F0D363A`); with rail damage accident factor 1 every
    `CheckRailAccident` on a damaged cell returns true for these draws, with factor 0.5 only the first two do.
13. **Capacity.** `max[1] = 30`, arrived 25, trip count 5 → `CapacityStillAvailable` true; count 6 → false; after
    `ApplyTrafficToDestination(5, …, type 1)` arrived 30 → destination bit 1 cleared over the lot and `freeDest[1]`
    decremented.
14. **Neighbour reconcile.** Property `0xAA9EA81D` = `{0, 50, 0, …}` and an empty list → one trip `{count 50, type 0,
    strategy 1, flags 4, status 0}`; SetupTrip then allows only start type car (`SetTravelTypeFlag(0x1CF, false)`,
    `SetTravelTypeFlag(0x2, true)`).
15. **Save round trip.** Write then Read of a fresh simulator gives the same bytes; the header is
    `29 00 0D 00 00 00 01 00 00 00 0D 00 00 00 09 00 00 00`; a trip record with empty routes is
    `02 00 00 00 | 00 00 00 00 | 00 00 00 00 | count | type strategy flags status | destX destZ | length | mask`.

## 12. Calibration (counters for the observer in the original)

Purpose: set the default of `trafficUnitsPerSlice` (§3.4) from the original's behaviour on the reference machine.
The observer logs, for every `WorkOnSimCycle` call (`0x00720F00`) made from `DoDailySimulation` (`0x007215C0`):

| Counter | Where to count in the original | Meaning |
|---|---|---|
| `state` | the simulator's current work-cycle state at the call's entry and exit | which phase the slice ran |
| `sources` | calls to `ConfirmTrips(source)` `0x0071D920` from ConfirmTrips `0x0071E1B0` (call at `0x0071E294`) | units of state 16 in the slice |
| `trips` | trips examined by ReRouteTrips `0x0071D020` and FullTrips `0x0071D440`: each passage through the deadline checks at `0x0071D1D9` and `0x0071D607`; searches: calls to `PerformPathSearch` `0x0071C6C0` (at `0x0071D257`, `0x0071D672`) | units of states 17 and 18 in the slice |
| `finished` | the return value of `WorkOnSimCycle` | the slice finished its phase |
| `suspended` | `PerformPathSearch` returning with the search incomplete (pathfinder work-time limit) | a trip split across slices (FreeCity4 has none) |
| `elapsed` | rdtsc at the call's entry and exit | slice length |

and, per game day (all `DoDailySimulation` calls with the same simulation date until one returns true): the
number of slices, `processed`, `total`, `days` and `dayIndex` before and after (§6, save row 15). The default is
the distribution of units per unfinished slice in states 16–18; log the city population and trip count with it,
because the cost of a unit grows with the network size.
