# Traffic simulator (cSC4TrafficSimulator + cSC4PathFinder)

Target: SimCity 4 1.1.641 (GOG) Windows exe, image base `0x400000`. Reference: named Mac Rev A Intel pseudocode
(`external/extracted/sc4-mac-symbol-analysis`, "Mac" addresses below). Symbol map with per-function evidence:
[`re/symbols/traffic.tsv`](../../re/symbols/traffic.tsv) (185 Windows entries: 184 functions + the next-state table). Scheduler background: [`docs/sim-engine.md`](../sim-engine.md).

Legend: **[confirmed]** read in code (Mac pseudocode, and Windows disassembly where a Win address is given) ·
**[inferred]** consistent with the evidence but not proven.

## 1. Responsibilities (summary)

The traffic simulator is a **commute / desirability generator**, not the visible car animation (that is
`cSC4TrafficAutomataManager` / `cSC4TrafficNetworkMap`, which only *reads* edge densities). [confirmed]

1. Keeps a per-cell model of every transport network (`NetworkInfo` lists + `TrafficSimCellConnections`), rebuilt on
   occupant insert/remove (`DoConnectionsChanged`). [confirmed]
2. Each lot (and neighbor connection) is a **traffic source** that owns a list of `cSC4TripData` (groups of commuters).
   Residential lots generate commuter trips per wealth level; industrial lots generate freight trips. [confirmed]
3. A modified **A\*** (`cSC4PathFinder::FindPath`) routes each trip group to the nearest destination with free job
   capacity, over a multi-modal graph (walk/car/bus/rail/subway/el/monorail/freight), with congestion-dependent edge
   costs. [confirmed]
4. Routes are stored (compressed) in the trip data and re-applied every cycle without re-searching while still valid. [confirmed]
5. Applied routes accumulate **traffic volume** per network per cell; volume/capacity gives **congestion**, which feeds
   back into the next cycle's edge costs (congestion → speed curve). [confirmed]
6. Outputs: trip-length (commute) map, congestion map, air-polluting-traffic map, commercial-traffic map, per-edge
   densities (automata/query), budgets/income, statistics, accidents. [confirmed]

## 2. Class layout and data structures

### 2.1 Interfaces / vtables

| Object | Windows | Mac | Notes |
|---|---|---|---|
| cSC4TrafficSimulator primary (cISC4TrafficSimulator) | `0xAB33D0` (44 slots) | `__ZTV20cSC4TrafficSimulator` `0xBAFD80` | slot order identical; Mac additionally lists dtors/Write/Read/GetGZCLSID/DoMessage (GCC puts secondary-base overriders in the primary vtable) [confirmed] |
| cIGZSerializable | `0xAB33A4` (Write `0x712400`, Read `0x721990`, GetGZCLSID `0x70D7D0`) | | [confirmed] |
| cIGZMessageTarget2 | `0xAB33BC` (DoMessage `0x7241B0`) | | DoMessage is entered with `this = object+4` (it calls `lea ecx,[ebp-4]` before DoDailySimulation); offsets *inside* DoMessage are therefore +4-biased [confirmed] |
| cSC4PathFinder (cISC4PathFinder) | `0xAB2150` (34 slots), ctor `0x6D89C0`, object size `0x3160` | `__ZTV14cSC4PathFinder` `0xBB1840` | created by `CreatePathFinder` `0x70D860`; pathfinder field offsets identical on both builds [confirmed] |

cISC4TrafficSimulator slot order (interface offsets relative to the vptr; Win slot = Mac slot−2):
`0x00 QI, 04 AddRef, 08 Release, 0C Init, 10 Shutdown, 14 GetSimulatorType (returns 0x88027F0D), 18 CreatePathFinder,
1C SetupPathFinderForLot, 20 GetAirPollutingTrafficMap, 24 GetCommercialTrafficMap, 28 GetCongestionMap,
2C GetBackgroundTraffic(x,z), 30 GetTrafficEdgeDensity(x,z,travelType,evening), 34 GetTripLengthMap, 38 GetTripScale,
3C GetTripScaleForDisplay, 40 IsRoadDamaged, 44 CheckRailAccident, 48 GetLastMonthlyIncome, 4C SetMaxTripCapacity,
50 GetMaxTripCapacity, 54 GetTrafficArrived, 58 GetDesiredLotInsertionPoint, 5C GetCapacity, 60 GetTravelTimeRatio,
64 GetConnectionCount, 68 GetTravelStrategyPercentages, 6C GetFreightScalingFactor, 70 GetTransitSwitches,
74 GetFerryRouteBetweenTiles, 78 GetAllFerryRoutes, 7C GetFerryRoutesInUse, 80 GetFerryTerminalCount, 84 GetWaterRoute,
88 GetTrafficStats, 8C GetTransitSwitchQueryData, 90 AreLotsConnected, 94 GetConnectedOccupantCount,
98 GetConnectedDestinationCount, 9C GetSubnetworksForLot, A0 GetSubnetworksInRectangle, A4 GetSubnetworksInRegion,
A8 GetOccupantCountForAllSubnetworks, AC dtor`. [confirmed]

cISC4PathFinder slot order: `00 QI, 04 AddRef, 08 Release, 0C Init, 10 Shutdown, 14 SetFlag, 18 ClearFlag,
1C ClearAllFlags, 20 SetRouteInfo, 24 SetTripDestinationFilter, 28 SetDestinationFlag, 2C ClearDestinationFlag,
30 ClearAllDestinationFlags, 34 SetTravelTypeFlag(mask, alsoStarting), 38 ClearTravelTypeFlag, 3C ClearAllTravelTypeFlags,
40 SetStartingCostArray, 44 SetMaxTripLength, 48 SetTrafficDensity, 4C SetStartRect, 50 SetEndRect, 54 SetMaxWorkTime,
58 Reset, 5C FindPath, 60 GetTrafficArrived, 64 GetTripLength, 68 GetTripLengthScaledForDisplay, 6C GetEndCell,
70 GetLastTravelType, 74 ApplyTraffic, 78 WasTripReversible, 7C ReleasePathNodes, 80 GetCompressedRoute, 84 dtor`. [confirmed]

### 2.2 Enumerations

- **TravelType** (9): 0 Walk, 1 Car, 2 Bus, 3 PassengerTrain, 4 FreightTruck, 5 FreightTrain, 6 Subway, 7 ElevatedTrain(LightRail), 8 Monorail. [confirmed via prior art + masks below]
- **Transit network** (6, `kTravelModeToTransitNetwork`): 0 Pedestrian, 1 Car(road), 2 Train, 3 Subway, 4 LightRail, 5 Monorail. Six float volume grids, one per transit network. [confirmed]
- **Network type / subtype** (13, index into speed & capacity tables): rd, rail, hwy, street, x, x, avenue, subway, lightrail, monorail, one-way, dirt road, ground highway (property descriptions). [confirmed order from tuning comments]
- Masks used by `SetupTrip`: passenger travel types `0x1CF` = {0,1,2,3,6,7,8}; starting types for commuters `0x3` = {walk, car}; freight `0x30` = {truck, freight train}; mass-transit strike restricts allowed types to `0x33` = {walk, car, truck, freight train}. [confirmed]
- **Trip type** (`cSC4TripData+0x1C`): 0/1/2 = residential commuters of wealth $/$$/$$$, 3 = freight. **Strategy** (`+0x1D`): 0 MassTransit-preferred, 1 Car-preferred, 2 Fastest. [confirmed]
- **Destination filter** (pathfinder `+0x80`): 0 standard destination (nearest lot with free job capacity of the wanted type), 2 occupant filter (query tool), 3 explicit end rectangle (re-route / return leg). [confirmed]
- **TrafficApplicationStyle**: 0 outbound (morning), 1 return (evening), 2 both (reversible route counted in both directions). [confirmed]

### 2.3 cSC4TrafficSimulator fields

| Mac | Windows | Field | |
|---|---|---|---|
| `+0x14` | | `cISC4City*` | [confirmed] |
| `+0x18` | | message server | [confirmed] |
| `+0x20/0x24` | | city cell count X/Z | [confirmed] |
| `+0x30` | `+0x30` | `SimGrid<u8>` air-polluting traffic map (id `0x89D5C3A2`) | [confirmed] |
| `+0x34` | | `SimGrid<float>` max traffic volume per cell (`0x69D5C3AC`) | [confirmed] |
| `+0x38` | `+0x38` | `SimGrid<u8>` commercial traffic map, tract 4 (`0xCABD2925`) | [confirmed] |
| `+0x3C` | | `SimGrid<u8>` commercial traffic work buffer, tract 4 (`0xCABD2926`), copied into +0x38 | [confirmed] |
| `+0x40` | | `SimGrid<u8>` (`0x69D5C3B5`) | purpose unknown |
| `+0x44` | `+0x44` | `SimGrid<u8>` **trip-length (commute) map** (`0x69D5C3BF`) | [confirmed] |
| `+0x48` | `+0x48` | `SimGrid<u8>` congestion map (`0x69D5C402`) | [confirmed] |
| `+0x4C` | | `vector<vector<u8>>` (W+2)×(H+2) **destination bitmask map** (bit i = lot has free capacity for destination type i) | [confirmed] |
| `+0x58` | `+0x58` | `TrafficSimCellConnections[256*256]` (0x2E bytes each, index `(x<<8|z)`) | [confirmed] |
| `+0x7C` | | `vector<vector<NetworkInfo*>>` per-cell linked list | [confirmed] |
| `+0x170` | | `vector<SimGrid<float>*>`[6] **traffic volume per transit network** (`0x69D5C3E0+n`) | [confirmed] |
| `+0x180` | | `SimGrid<float>` max congestion ratio (`0x69D5C40E`) | [confirmed] |
| `+0x184` | | `hash_map<u32, TrafficEdgeDensity>` key `evening<<24 | travelType<<16 | x<<8 | z` | [confirmed] |
| `+0x1C8` | | `u32[4]` number of destinations with free capacity per destination type | [confirmed] |
| `+0x1FC` | `+0x1E4` | `TrafficStats` (GetTrafficStats) | [confirmed] |
| `+0x718/0x71C` | | `SimGrid<u8>`/`SimGrid<u32>` transit-switch maps (`0x69D5C424`, `0x69D5C42D`; u32 = switch id) | [confirmed] |
| `+0x724` | | `hash_map<u32, TransitSwitch>` | [confirmed] |
| `+0x738` | `+0x720`* | transit-switch cost multiplier = curve `0xCA5F7821`(mass transit funding %) | [confirmed] |
| `+0x73C/0x740` | `+0x724/0x728`* | roads / mass-transit budget departments | [confirmed] |
| `+0x7AC` | `+0x7A4` | mass-transit strike flag (restricts travel types to `0x33`) | [confirmed] |
| `+0x794` | `+0x788` | last monthly income | [confirmed] |
| `+0x81C` | `+0x814` | work-cycle **state** (0..19) | [confirmed] |
| `+0x838/0x83C` | `+0x830/0x834` | pathfinders: outbound, return | [confirmed] |
| `+0x844` | | `StandardTripParams` (current source, coords, pathfinders, search state `+0x48`) | [confirmed] |
| `+0x910` | `+0x908` | `TrafficSourceIterator*` | [confirmed] |
| `+0x91C/0x920/0x924/0x934` | `+0x914/0x918/0x91C/0x92C` | daily pacing: processed, day index, total, days | [confirmed] |
| `+0x928` | `+0x920`* | "SimBegin seen" byte | [confirmed] |
| `+0x930` | `+0x928` | trip-parameter reselection bucket | [confirmed] |
| `+0x944/0x948/0x94C` | `+0x93C/0x940/0x944` | counts of trips to confirm / reroute / full-route | [confirmed] |
| `+0x958` | | 2-bit rotation counter (which quarter of sources gets a forced full recalculation) | [confirmed] |
| `+0x97C..+0xD5C` | `+0x974..+0xD54` | tuning values (Win = Mac−8 for this range), §6 | [confirmed] |

`*` = derived from the +4-biased offset seen inside DoMessage.

`static float mfInverseMaximumSpeed[9][13]` (minutes per cell), `static bool mbTravelTypeAffectedByTraffic[9]`,
`static cSC4ResponseCurve mCongestionToInverseSpeedCurve` are class statics. [confirmed]

### 2.4 Per-cell network model

- **NetworkInfo** (0x18, linked per cell, one per transit network): `+0 float invCapacity` (=1/`Network Traffic Capacity`
  of the cell's network type), `+4 u8` connection bits, `+5 u8 networkType` (0..12, index into speed table), `+6 u8
  transitNetwork`, `+7 u8 flags` (bit0 from occupant flag 0x40000; excluded from the per-type statistics),
  `+8 float extraCost` (= `Damaged road extra step cost` when `IsRoadDamaged(x,z)`), `+0x14 next`. [confirmed]
- **TrafficSimCellConnections** (0x2E): `u16 conn[9]` (per travel type, 4 bits per *entry* side = allowed exit sides),
  `u16 switchType[9]` at +0x12 (per travel type, a nibble per exit side = travel type you become when leaving that way,
  0xF = none), `u16 priorCar[4]` at +0x24 (car turn restrictions by entry side), `u8 wormhole` at +0x2C (1 subway
  station, 2 tunnel, 4 ferry). [confirmed]
- **TrafficEdgeDensity** (0x42): `u16 current[4][4]` (entry×exit), `u16 total` at +0x20 (what queries/automata read),
  `u16 pending[4][4]` at +0x22 (written by ApplyRoute during the cycle). [confirmed]

### 2.5 cSC4TripData (0x2C, stored in each lot's list at lot+0x9C, saved in the lot subfile)

`+0x00 vector<u8> route (outbound, compressed)`, `+0x0C vector<u8> returnRoute`, `+0x18 u32 count` (commuters/freight units),
`+0x1C u8 tripType`, `+0x1D u8 strategy`, `+0x1E u8 flags` (1 = route reversible, 2 = probe trip (no traffic, no stats),
4 = split trip, 8 = counted as success), `+0x1F u8 status` (0 needs full search, 1 has destination → re-route,
3 routed OK, 4 surplus / pending, 5 failed), `+0x20/+0x22 s16 destX/destZ` (off-map coordinates = neighbor connection),
`+0x24 float tripLength` (minutes), `+0x28 u32 travelTypesUsed` mask. [confirmed]

### 2.6 cSC4PathFinder fields (same on both builds)

`+0x0C trafficSim, +0x10 goal/current TripNode*, +0x14 float heuristicWeight, +0x18 SC4Rect start, +0x28 SC4Rect end,
+0x38 search-rect (start expanded by 4), +0x48 SC4Rect originatingEdge, +0x58/0x5C end cell, +0x60 float maxTripLength in /
tripLength out, +0x64 u16 allowedTravelTypes, +0x66 u16 startingTravelTypes, +0x68 u32 trafficDensity (trip count),
+0x6C u32 trafficArrived, +0x70 u32 flags (bit0 unlimited cost, bit1 accept nearest on failure, bit2 resume/continue,
bit3 must pass originating car cell, bit4 no ferries), +0x74 route deque*, +0x80 destinationFilter, +0x84 u8 destinationFlags,
+0x88 occupant filter, +0x8C destination type reached, +0x90 float* startingCost[9], +0x94 float maxCost[9],
+0xB8/0xBC city size, +0xC0 maxWorkTime ms, +0xC4 suspended flag, +0xC8 u32 overBudgetCount[9], +0xEC u8 disabledTravelTypes,
+0xEE/0xF0/0xF2 park-and-ride cell & side, +0x10C/+0x110 distance tables X/Z, +0x114 priority queue, +0x128 node hash map`. [confirmed]

TripNode: `+0 u32 key, +4 parent, +8 float g, +0xC float f, +0x10 open, +0x11 closed, +0x12 entrySide, +0x13 travelType,
+0x14 s16 x, +0x16 s16 z`. Key = `parentSide<<26 | side<<24 | travelType<<16 | x<<8 | z` (bit 31 = off-map exit node). [confirmed]

## 3. Registration and messages

`Init` (Win `0x71E7E0`, Mac `00058684`): ReadSimulatorTunableValues; `AddAgent(this+4, level 0/6/4, "cSC4TrafficSimulator",
flags 2)`; subscribes to InsertOccupant `0x99EF1142`, RemoveOccupant `0x99EF1143`, PostCityInit `0x26D31EC1`, StateChanged
`0xE8BDA2E7`, ReloadTunableValues `0xC53D10AA`, AddNeighborConnection `0x8A0D15DA`, RemoveNeighborConnection `0xEA0D163E`,
FundingPercentageChanged `0x08DAC3E7`, OrdinanceUpdated `0xC2C09554`; creates the grids of §2.3, a 0x2E0000-byte connection
array, and a `cRZRandomSuper`. [confirmed on both builds]

`DoMessage` (Win `0x7241B0`, Mac `00068b38`) [confirmed]:

| Message | Behaviour |
|---|---|
| SimBegin `0x66956810` | set "begun" byte; return true |
| SimNewDay `0x66956814` | `return DoDailySimulation()` (the only time-sliced handler) |
| SimNewMonth `0x66956816` | `DoMonthlySimulation()`; return true |
| PostCityInit | `DoPostCityInit()` (builds network model, sources, subnetworks) |
| StateChanged (lot) | old state 3 → `DoLotDeactivated`; new state 3 → `DoLotActivated` (also `SelectNewTripParameters`) |
| Insert/RemoveOccupant (network occupant, data3 = `0xC772BF98`) | `DoRoadChanged` for network types 6, 0, then 10 or 3/2/12; `DoRailChanged` for 1; subway (7) count + mass transit budget (Win `DoSubwayChanged 0x70EA70`); if the type maps to a transit network: `DoTunnelChanged`, erase edge-density entries on remove, `DoConnectionsChanged(x,z,x,z)` |
| Add/RemoveNeighborConnection | `AddNeighborConnection` / `RemoveNeighborConnection(type,x,z)` |
| FundingPercentageChanged for dept `0xC921CEEB` (mass transit) | transit-switch multiplier = curve `0xCA5F7821`(funding×100) |
| ReloadTunableValues for (`0xE7E2C2DB`,`0xC9133286`) | `ReadSimulatorTunableValues()` |
| OrdinanceUpdated | `UpdateTravelStrategies()` |

## 4. Time slicing and the work cycle

### 4.1 DoDailySimulation (Win `0x7215C0`, Mac `00068714`) [confirmed]

```text
deadline = now + 4 ms                                   // sMillisecsToCyclesCoeff * 4
if day-of-month changed:
    if new day == 1: post last month's road/transit income into budget line items
    if dayCounter % (period[0xA92356B6] / buckets[0xA92356B7]) == 0:
        for lot in lotsInBucket[bucket]: SelectNewTripParameters(lot)   // lot prop 0x4A611773 = rand(100)
        bucket = (bucket+1) % buckets
loop:
    if now > deadline: return false                       // scheduler calls again next frame
    finished = WorkOnSimCycle(deadline)
    if !finished:                                          // ran out of time inside a phase
        if state not in {16,17,18}: return false           // other phases must finish before the day ends
        if day < days and processed >= (day+1)*total/days: day++; return true   // daily quota met
        return false
    // a phase completed; state already advanced
    if state == 17 (ReRoute):   total = rerouteCount;  days = max(1,total/18); reset pacing; return true
    if state == 18 (FullTrips): total = fullCount;     days = max(1,total/18); reset; return true
    if state == 16 (Confirm):   total = confirmCount;  days = max(1,total/20); reset counters
    if state == 2 or state == 11: return true             // day boundary after these phases
```

So routing is paced at a **minimum of ~20 (confirm) / 18 (route) trip sources per game day**, and the game day cannot
advance until that quota is met, but more is done when frame time allows. All other phases run to completion inside the
day (4 ms per frame, returning false until done). Each phase also self-tunes how often it polls `rdtsc` (a static
"timer history" starting at 1024 iterations, doubled/halved depending on how many polls a slice took). [confirmed]

### 4.2 WorkOnSimCycle state machine (Win `0x720F00`, jump table `0x7214EC`, next-state table `0xAB3480`) [confirmed]

`kNextState = {16,14,4,7,5,11,10,6,0,0,12,3,13,19,15,2,17,18,15,8}`; state 12 is skipped (→13) unless the subnetworks are
dirty (`this+0x62`). If an intersection update is pending, `UpdateIntersectionData(rect)` runs first. The effective cycle is:

| # | State | Function (Win / Mac) | Work |
|---|---|---|---|
| 1 | 0 | WorkOnStartRecalc `0x717C90` / `0005b5ec` | zero commute stats, `rotation = (rotation+1)&3`, `RecalculateNeighborConnectionCapacities()` |
| 2 | 16 | WorkOnConfirmTrips `0x71E1B0` / `00064b9a` | per source: `source->UpdateTrafficVolume()` (trip generation, §5.1) then `ConfirmTrips` (§5.2) |
| 3 | 17 | WorkOnReRouteTrips `0x71D020` / `00064dce` | status-1 trips: path search to the *known* destination (filter 3) |
| 4 | 18 | WorkOnFullTrips `0x71D440` / `00065282` | status-0 trips: full search for the nearest job (filter 0); `CompleteTripSourceCalculations` per source |
| 5 | 15 | WorkOnFinishTripMap `0x70FEF0` / `00065750` | clear lot flag 0x80; fill trip-length map for zone-type 8/9 lots without own trips from same-type neighbors [inferred purpose] |
| 6 | 2 | WorkOnCreateMaps `0x71A0C0` / `00065b7e` | congestion, stats, accidents, volume decay (§5.5); then UpdateRoadsBudget/UpdateMassTransitBudget |
| 7 | 4 | WorkOnCreateFinalMap `0x70F460` / `0006664e` | derive u8 maps; broadcast `kMsgTrafficMapChanged 0x69247DC7` |
| 8 | 5 | WorkOnClearTripDestinations `0x70E4F0` / `00066a78` | per lot: arrived = used; used = 0; total jobs per wealth |
| 9 | 11 | WorkOnUpdateNeighborConnections `0x714BE0` / `00059e0e` | neighbor connection bookkeeping |
| 10 | 3 | WorkOnReduceTransitSwitches `0x713750` / `000543f2` | every transit switch: usage-ish value `*= monthlyDensityReduction` |
| 11 | 7 | WorkOnDumpEdgeDensities `0x7137D0` / `0005437a` | edge densities: current = pending, total = Σ (saturating u16) |
| 12 | 6 | WorkOnReduceEdgeDensities `0x7110B0` / `00066ce8` | zero `pending` for every network/switch/neighbor cell |
| 13 | 10 | WorkOnRefreshCommuteMap `0x70F6E0` / `0006705a` | rebuild destination bitmask map and per-type destination counts from lot max capacities |
| 14 | 12/13 | UpdateSubnetworks `0x7183B0`, WorkOnUpdateSubnetworkInfo `0x71DC50` | connectivity components (flood fill) for AreLotsConnected etc. |
| 15 | 19 | WorkOnFerryRoutes `0x71DC90` / `00067c1e` | roll ferry usage counters (current = pending) |
| 16 | 8 | WorkOnFinalCalculations `0x70E7D0` / `000526be` | monthly income (roads, transit switches), average commute, freight trip length byte |

States 1, 9, 14 are unused (legacy `WorkOnGenerateTrips` / `WorkOnGenerateNeighborTrips` exist only as stubs in the PPC
symbol list). One full cycle therefore takes many game days (routing quota-bound), which matches the community
observation that commutes are recomputed "about every four months". [confirmed structure; duration inferred]

## 5. Algorithms

### 5.1 Trip generation — `cSC4Lot::UpdateTrafficVolume` (Mac `0011ec6a`, via cISC4TrafficSource vtable +0x10) [confirmed]

```text
zone = lot.GetZoneType()
if zone in 1..3 (residential) and lot functional:
    scale = ResidentialSim->vt+0x78(lotX, lotZ)          // per-tile commuter factor (exact meaning TBD)
    pop, wealth = first nonzero of lot.capacity(R$ 0x1010 → w0, R$$ 0x1020 → w1, R$$$ 0x1030 → w2)
    commuters = max(1, round(pop * scale))
    pct[3] = TrafficSim.GetTravelStrategyPercentages(wealth+1)   // MassTransit, Car, Fastest
    if commuters < 50: pick one strategy s by cumulative pct vs lot.randomByte (lot+0xA4, 0..99); want[s] = commuters
    else: want[s] = round(commuters * pct[s]) for each s
    tripType = wealth
elif zone in 7..9 (industrial) and functional and not ...:
    want[Fastest] = round((jobs I-R+I-D+I-M+I-HT) * FreightScalingFactor); tripType = 3 (freight)
else: return                                              // commercial lots are only destinations
reconcile lot.trips with want[]:
    total == 0 → single probe trip {count 1, flags 2 (no traffic)} so the lot still gets a trip-length value
    otherwise drop probe trips and trips of another type; per strategy add a new status-0 trip for a deficit
    (flag 4 if a trip of that strategy already exists), or shrink/erase (status-4 trips first) for a surplus
```

Neighbor connections also act as traffic sources (`cSC4NeighborConnection::UpdateTrafficVolume`). `SelectNewTripParameters`
re-rolls `0x4A611773` in rotating buckets so small lots occasionally change strategy.

### 5.2 Confirm phase — `ConfirmTrips` (Win `0x71D920`, Mac `00060df4`) [confirmed]

```text
forcedFull = (++sourceCounter & 3) == rotation           // a quarter of the sources each cycle
if forcedFull: merge routed trips with identical (type,strategy) into one status-0 trip
for trip in source.trips:
    status 3: if !CapacityStillAvailable(trip)            // count <= maxCap - used at destination
                  status = 0                                // needs a new job
              else if ConfirmRoute(route)                   // every cell/connection still exists
                  re-ApplyRoute(route[, returnRoute]) with style 2 (reversible) or 0+1; status stays 3
              else status = (type != freight) ? 1 : 0       // re-route to same destination
    status 5: status = 0
    count status 1 → rerouteCount, status 0 → fullCount
```

### 5.3 Trip routing — `SetupTrip` + `PerformPathSearch` (Win `0x70EC40`/`0x71C6C0`) [confirmed]

```text
SetupTrip(params, trip):
    pf.SetTrafficDensity(trip.count)
    if trip.status == 1: filter = 3; endRect = bounds of lot at trip.dest (or the off-map cell)
    else: filter = 0; destFlags = (flags&2 && type==0) ? 7 : 1<<type
    both pfs: SetMaxWorkTime(remaining ms of the 4 ms slice)
    pf.SetStartRect(source bounds); ClearAllFlags; ClearAllTravelTypeFlags; SetMaxTripLength(CommuteTripMaxTime)
    if type < 3 (commuter): roundTrip = true; allowed = 0x1CF; starting = {walk, car}
         startingCost = strategy==Car ? CarPrefCosts : strategy==MassTransit ? MassTransitPrefCosts : DefaultCosts
    if type == 3 (freight): oneWay; SetFlag(1 = no cost limit); allowed = starting = {truck, freight train}

PerformPathSearch:
    r = pfOut.FindPath()                    // 0 fail, 1 found, 2 out of time (resumed next call)
    if r == 1:
        trip.dest = pfOut.GetEndCell(); trip.route = pfOut.GetCompressedRoute()
        if freight: ApplyRoute(style 0)
        elif pfOut.WasTripReversible(): ApplyRoute(style 2)                  // same path counted morning+evening
        else: configure pfRet: start = destination, end = home (highway-return tile if off-map),
              filter 3, starting type = last travel type, must pass the park-and-ride cell (flag 8); search later,
              then ApplyRoute(route, style 0) + ApplyRoute(returnRoute, style 1)
    if found and status==0 trips: if arrived < trip.count, split: remainder becomes a new status-0 trip
    trip.status = found ? 3 : 5; trip.tripLength = pf.GetTripLength()
```

### 5.4 Pathfinder — `cSC4PathFinder::FindPath` (Win `0x6D91B0`, Mac `0009b804`) [confirmed]

A\* over states `(x, z, entrySide, travelType)` on the city's cell grid (plus off-map neighbor cells):

```text
maxCost[t] = maxTripLength + startCost[t]                 // start cost biases, but does not consume, the budget
if filter == 0: w = NearestDestinationAttractiveness; FindNearestStandardDest() → endRect   // ring search on the
               destination bitmask map, nearest by Manhattan distance; fail if no destination of the wanted type
elif filter == 3: w = NearestDestinationAttractiveness else w = 1.0
SetupDistanceTables(): dX[x] = cells from x to endRect's x-range, dZ likewise   // Manhattan, integer
CreateStartNodes(): rings d = 1..MaxDistanceOriginToNetwork around the start rect (only cells orthogonally aligned
    with the lot); for every transit network present (legal starting subtype if the start is a lot) and every allowed
    starting travel type on it and every valid entry side: node.g = startCost[t]; push.
    Off-map start (neighbor connection): seed from that connection's entry cells with startCost + link cost.
h(x,z) = (dX[x] + dZ[z]) * w, but 0 inside the start rect expanded by 4 cells.   f = g + h
loop until queue empty or maxWorkTime elapsed (then suspend, return 2):
    n = pop min f
    canEnd = TravelTypeCanReachDestination[t] && (in city ⇒ kLegalEndingSubtype[networkType])
    if AtGoal(n, canEnd): goal = n; DoSuccessfulTrip(); return 1
    exits = conn[cell].conn[t] >> (n.entrySide*4) & 0xF; forbid U-turn if kProhibitSharpTurns[t]
    for side in 0..3:
        if exit bit set: neighbour cell c' (entry side computed by EdgeTo)
            if c' off-map: AddTripNode(c', cost = n.f - n.g, exitNode=true)
            elif info = NetworkInfo(c', network(t)):
                if car: check priorCar turn bits
                cost = invMaxSpeed[t][info.type] * (AffectedByTraffic[t] ? CongestionCurve(volume[c'] * info.invCapacity) : 1)
                       + info.extraCost
                AddTripNode(c', side', t, cost)
            elif c' is a transit switch: AddTripNode(..., TransitSwitchEntryCost(switch))
        t2 = switchType nibble for this side; if t2 != 0xF and allowed: same as above with travel type t2
    wormholes: tunnel exits (hash +0xD4, per-type cost); subway station: switch walk<->subway in place, cost 0;
               ferry terminal links (hash +0xE8) unless flag bit4
    park-and-ride constraint (flag bit3): from cell (0xEE,0xF0) add the car continuation
    close n
failure (queue empty): if flag bit1 and filter 3 and a route deque is wanted, take the closed node nearest to the end
    rect (DistanceToRectangle) as the goal; else return 0
```

`AddTripNode(x,z,side,t,cost,parent,maxCost,exit)`: skip if travel type disabled; ignore immediate backtracking; `g =
parent.g + cost`; if `g > maxCost[t]` (and not flag bit0) count an overflow, and after **64 overflows the travel type is
disabled** for the rest of this search; otherwise insert/relax in the node hash and priority queue. [confirmed]

`AtGoal` (filter 0): checks the 4 neighbours of the node for lots whose destination bits match; for each matching type
`CheckDestinationCapacity` (`free = GetMaxTripCapacity - GetUsedTripCapacity`; `arrived = min(free, tripCount)`); on success
the end cell is that lot. A lot with no remaining capacity has its bits cleared from the destination map and the
per-type counter decremented, so later searches skip it. Off-map cells use the neighbor city's job pool
(`GetNeighborPoolPropertyHolder`). Filter 3: node inside (or adjacent to) the end rect. Filter 2: occupant filter. [confirmed]

`DoSuccessfulTrip`: `tripLength = goal.g - root.g` (start cost excluded, clamped ≥ 0); plus the neighbor connection's
extra cost if the goal is off-map; records the last car→non-car transition as the park-and-ride cell; optionally fills
the route deque. Trip length units are **minutes** (`speed km/h × 1000/60 / cellSizeMeters` → cells/min). [confirmed]

`TransitSwitchEntryCost = baseCost × CongestionCurve(usage/capacity) × fundingMultiplier`, infinite when usage ≥ 5×capacity. [confirmed]

### 5.5 Applying traffic and congestion feedback [confirmed]

`ApplyRoute(route, count, startRect, destX, destZ, style, tripType)` (Win `0x719910`):
- car count is multiplied by 0.8 (ceil) when ordinance `0xA2BF1DDC` is active [inferred: carpool ordinance];
- for each cell: if `TravelTypeGeneratesTraffic[t]`: `volume[network(t)][cell] += count` (×2 for style 2);
  transit-switch usage and fare income; `edgeDensity[evening=style==1][t][cell].pending[entry][exit] += count`
  (style 2 also adds the reversed movement to the evening entry); ferry counters;
- income `+= count × IncomePerTile[t]` (`0xA92356B1`) and ferry fare; at the end (not for the return leg)
  `ApplyTrafficToDestination` adds `count` to the destination's used capacity and, if now full, removes it from the
  destination map. Neighbor destinations also record per-travel-type counts in property `0xAA9EA81A+wealth`.

`WorkOnCreateMaps`, per cell (Win `0x71A0C0`):
```text
for each transit network n present in cell:
    v = volume[n][cell]
    maxVolume[cell] = max(v) over non-rail networks; commercialTract[cell/4] = max(min(255, v*128/RoadCapacity)) (+edge spread)
    ratio = v * info.invCapacity;  congestionRatio[cell] = max(ratio)   (also transit switch usage/capacity)
    accident-prone if car/rail with type > 3: CongestionToAccident(ratio) * CapacityToAccident(capacity) > 0
    per-network-type stats: sum/avg ratio, max + location, #cells over capacity (ratio > 1)
    volume[n][cell] = v * MonthlyTrafficDensityReduction (+ GetBackgroundTraffic(x,z) for the road network)
```
`GetBackgroundTraffic(x,z) = Σ_{R,C,I} PopulationBackgroundTraffic[k] × population_k(x,z)` from the growth-developer
population grids (R$..R$$$, Cs/Co, I-R..I-HT). The volume grid is thus an exponentially decayed accumulation of applied
routes plus background traffic, and it is what the next routing pass reads. [confirmed]

`WorkOnCreateFinalMap`: `airPollutingTraffic[cell] = min(255, maxVolume × 0.1)`; `congestion[cell] = m <= 1 ? 1 :
clamp((m-1) × 255/(lastCurveY-1), 1, 255)` with `m = CongestionCurve(congestionRatio)`; cells without traffic get 0. [confirmed]

### 5.6 Commute reporting — `CompleteTripSourceCalculations` (Win `0x70F0A0`) [confirmed]

```text
for trip in source.trips:
    routed (3): sum += count*tripLength; n += count; stats per wealth (commuters, successful); usage per travel type
    failed (5): sum += count*CommuteTripMaxTime; n += count; anyFailed = true
avg = sum/n
in city:   tripLengthMap[lot cells] = anyFailed ? 0 : clamp(round(avg/CommuteTripMaxTime*255), 1, 255)
off map:   neighbor connection property 0xCA947077 = avg
```
Consumers: `cSC4GrowthDeveloper::DoIdle/EstimateDesirability` take, per tract, the minimum non-zero trip-length value
(255 if none) through a growth-developer response curve into desirability, and also read the commercial-traffic map;
`cSC4AuraSimulator::DoIdle` reads congestion and trip-length maps; the query tool shows `value × TripLengthDisplayMultiplier`.
The average commute (`+0x438`) goes to the history warehouse monthly (`0x8A5D4116`, ×25). [confirmed reads; exact
desirability weighting lives in the growth developer]

### 5.7 Monthly — `DoMonthlySimulation` (Win `0x718AC0`, Mac `0005f5d2`) [confirmed]

`ApplyRoadFunding(roads%)`, `ApplyMassTransitFunding(transit%)`, `UpdateStrikes()`, `CheckForAccident()`, remember the
funding percentages, history-warehouse entries: `0xEA5BA01D` (traffic stat ×100), `0x8A5D4116` (avg commute ×25),
`0xABB8FAAC+t` (trips per travel type, t=0..8), `0xEC0BDBF8` (ferry passengers).

## 6. Tuning exemplar (TGI `0x6534284A / 0xE7E2C2DB / 0xC9133286`) — `ReadSimulatorTunableValues` (Win `0x716AE0`)

Read in this order on both builds; Mac offset (Win = Mac−8). Defaults shown only where the code hard-codes one. [confirmed]

| Property | Name | Use |
|---|---|---|
| `0x491332E6` | Traffic Volume Per Population | `+0x97C` (consumer not seen) |
| `0x491332E7..EF` | Max speed by network for travel type 0..8 (km/h, 13 values) | `mfInverseMaximumSpeed[t][n] = 1/(km/h·1000/60/cellSize)`; ≤0 → impassable (FLT_MAX) |
| `0x29136788` | Monthly Traffic Density Reduction | volume & switch decay factor per cycle |
| `0x29136789` | (unnamed) | `+0x980` |
| `0x6A84493E` | Monthly cost per network tile (13) | budgets |
| `0xA92356AE/AF` | Max roads / mass transit funding % | |
| `0xA92356B0` | Damaged road extra step cost | NetworkInfo.extraCost |
| `0xA92356B6/B7` | (unnamed) trip-parameter reselection period (days) / bucket count | DoDailySimulation [inferred names] |
| `0xA92356B8` | Maximum distance from origin to network | CreateStartNodes ring count |
| `0xA92356B1` | Income per tile by travel type (9) | ApplyRoute income |
| `0xA92356B3` | Network Traffic Capacity (13) | `invCapacity = 1/value` (1.0 if ~0) |
| `0xA92356B4` | Travel type generates traffic (9 bools) | ApplyRoute |
| `0xA92356BF` | Travel type affected by traffic (9 bools) | congestion term in edge cost |
| `0xA92356B5` | Travel type can reach destination (9 bools) | AtGoal `canEnd` |
| `0xA92356B9` | Congestion vs Speed (curve) | stored inverted (y → 1/y) = congestion → time multiplier |
| `0xA92356BA` | Commute trip max time (minutes) | maxTripLength, trip-length map scale |
| `0xA92356BB` | Intersection and Turn Capacity Effect | stored as reciprocals at `+0xC74` |
| `0xA92356BC` | Trip Starting Cost by travel type (Fastest/default) | startCost / default table |
| `0xEA8C3CDB` | Trip Starting Cost, Mass-Transit preferred | |
| `0xCAD64136` | Trip Starting Cost, Car preferred | |
| `0xA92356BD` | Job Scaling Constant | |
| `0x49A2E8BE` | Freight traffic scaling factor | freight trips per industrial job |
| `0x4A678060` | Nearest Destination Attractiveness | A\* heuristic weight w |
| `0xCA76013B` | Trip Length to Minutes Display Multiplier | |
| `0xEA7B5F06` | Max Mass Transit Strategy Trip Length | |
| `0xA92356BE` | Population Background Traffic (R, C, I; default 1.0 each if short) | GetBackgroundTraffic |
| `0xCA5F7821` | Transit Switch Entry Cost vs Budget (curve; default (0,0)-(1,1)) | |
| `0x4BE09EC9` | Congestion to Accident Probability (default (0,0),(0.25,0),(1,0.05),(2,0.25)) | |
| `0x4BE09ECA` | Capacity to Accident Probability (default (0,1),(1000,1)) | |
| `0x4BE09ECB` | Accident Duration, s (default 10) | |
| `0x4BE09ECC` | Accident Check Period, s (default 60) | |
| `0x0BD86FD4` | Funding to Damage Acceleration (default (0,0.02),(100,-0.01),(120,-0.04)) | |
| `0x0BD86FD5` | Rail Damage Accident Factor (default 1.0) | |
| `0x8C3CAEF5` | Ferry Fare | |
| `0x4953E8A3..A6` | Travel strategy % for wealth none/$/$$/$$$ (MassTransit, Car, Fastest) | read by UpdateTravelStrategies; ordinances add deltas from their property `0x8A612FEE` (clamped 0..100) |

Other properties: lot trip capacity vector `0x8A0B4100` (`[0..3]` max, `[4..7]` used, `[8..11]` arrived per destination
type); lot random byte `0x4A611773`; neighbor commute `0xCA947077`; neighbor per-travel-type trips `0xAA9EA81A+wealth`. [confirmed]

## 7. Interactions

- **Reads**: Residential sim (commuter scale), growth-developer population grids (background traffic), lot capacities /
  trip capacity properties, budget departments (funding), ordinance sim (strategy deltas, carpool), neighbors sim
  (connections, job pools), city cell size. [confirmed]
- **Writes / is read by**: growth developers (trip-length, commercial traffic → desirability), aura (congestion,
  trip length), pollution (air-polluting traffic map, via `GetAirPollutingTrafficMap`) [inferred consumer], demand
  (`EvaluateConnectionUtilization`), automata and query UI (edge densities, travel time ratio), budget (income /
  expenses), history warehouse. Broadcasts `0x69247DC7` after each map rebuild. [confirmed unless noted]

## 8. Helpers: cSC4TrafficNetworkMap and cSC4WaterPathFinder

- **cSC4TrafficNetworkMap** (CLSID `0x6A0F82B2`, Win primary vtable `0xA8FED8`, DoMessage `0x4B0C10`) is the
  *visual* side: per-tile network info for automata, intersections and stoplight timing (`TickIntersections`,
  `SwitchStoplightStates`), train junctions, lane ticks / autos-per-second, and constraint of positions to network tiles.
  It is not part of the commute model; it consumes the traffic simulator's edge densities (`GetTrafficEdgeDensity`,
  `GetTravelTimeRatio` = congestion-curve(Σ current+evening usage × invCapacity)). [confirmed slot map; behaviour from names]
- **cSC4WaterPathFinder** (CLSID `0xABDF0343`, Win FindPath `0x725170`) is a separate grid search over water depth
  (`UpdateDepthMap`) used only for ferry routes: `GetWaterRoute`, `RouteAndLinkTerminals` (terminal↔terminal links) and
  `RouteTerminalToNeighbor` (terminal↔neighbor edge). Ferry links then appear in the land pathfinder as wormhole edges
  (connection flag 4, hash at sim `+0xE8`). [confirmed call graph; search internals not analysed]

## 9. Open questions

- Exact semantics of `ResidentialSim vt+0x78` (commuter scale) and the zone-type 8/9 handling in `WorkOnFinishTripMap`.
- `0x491332E6` and `0x29136789` consumers; names of `0xA92356B6/B7`.
- `PathFinder::ApplyTraffic` (Win `0x6DA060`) callers (probably `SetupPathFinderForLot` / UI); subnetwork flood fill details.
- Neighbor connection capacity model (`RecalculateNeighborConnectionCapacities`) and ferry routing were not analysed.
- Maxis default exemplar values (not in this repo; need `SimCity_1.dat`).
