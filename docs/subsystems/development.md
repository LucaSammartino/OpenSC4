# Growth / development pipeline (zoned land to buildings)

Target: `SimCity 4.exe` 1.1.641 (GOG), image base `0x00400000`. Logic and names come from the named Mac Rev A Intel
pseudocode (written `mac:XXXXXXXX`). Windows addresses were matched to it through vtables, constants, string
switches and call-graph order. The full symbol map, with evidence for each entry, is in
[`re/symbols/development.tsv`](../../re/symbols/development.tsv). Scheduler conventions are described in
[`docs/sim-engine.md`](../sim-engine.md).

Legend: **[confirmed]** means the behaviour was read in code. Unless "Windows" is stated, that code is the Mac
pseudocode, and the matching Windows function has been identified. **[confirmed-W]** means it was also read in the
Windows decompilation or disassembly. **[inferred]** means it fits the evidence but has not been proven.

Field offsets are given for the **Mac** layout unless marked `W+0x..`. MSVC STL containers have different sizes from
GCC ones, so most offsets past the first container are different on Windows.

---

## 1. Overview

```text
             SimNewMonth / SimBegin                         SimIdle / SimNewDay(5 ms)
 cSC4BuildingDevelopmentSimulator ──StartMapUpdate──▶ cSC4GrowthDeveloper ×12   (desirability state machine,
   (BDS, CLSID 0x8990BE7A)                               (CLSID 0x0990C138)        growth list, decline list,
        │  SimNewDay: Grow()                                     ▲                  occupancy -> population)
        │  weighted-random developer pick by demand              │ ZoningChange -> UpdateMaxCapacity
        ▼                                                        │
 cSC4TractDeveloper::Grow(profile = {developer, tractX, tractZ, maxStage})
   (CLSID 0x2990C142)  list lots in tract -> reoccupy abandoned -> candidate lots (existing / aggregate / subdivide)
        │              -> score -> pick building (lot config + building family/style/wealth/capacity)
        ▼
 cSC4LotDeveloper::StartConstruction (CLSID 0xA990BFE0) -> lot state 2 + cSC4ConstructionOccupant -> state 3
 cSC4LotDeveloper daily: visits ~1/28 of all lots per day; tests desirability/power/water/travel -> abandon (state 4)
 cSC4ZoneDeveloper (0x498F9B01): parcellizes zones into lots (UI-time). cSC4ZoneManager (0x298F9B2D): zone grid.
```

### Key enums [confirmed]

- **PurposeType** (BuildingProfile+0): 1 Residence, 2 Services (Cs), 3 Office (Co), 5 Agriculture, 6 Processing (ID),
  7 Manufacturing (IM), 8 HighTech (IHT). 4 is presumably Tourism [inferred]. Purposes 5–8 are industrial.
- **WealthType**: 1 $, 2 $$, 3 $$$.
- **DeveloperType** (`GetDeveloperType_Static`, mac:000f6bfc, W `0x695790`): R 0x1010/0x1020/0x1030,
  Cs 0x3110/0x3120/0x3130, Co 0x3320/0x3330, I-agri 0x4100, I-dirty 0x4200, I-manuf 0x4300, I-hightech 0x4400.
- **TargetZoneType** (developer `GetType`, used by scoring): 1 R, 2 C, 3 I.
- **ZoneType**: 0 none, 1–3 R low/med/high, 4–6 C low/med/high, 7 I-agri, 8 I-med, 9 I-high, 0xA military,
  0xB airport, 0xC seaport, 0xD spaceport, 0xE landfill, 0xF plopped [inferred names from the switch shapes].
- **Lot state** (cISC4Lot vt+0x84 get, vt+0x88 set(state, ?, reason)): 2 under construction, 3 developed/occupied,
  4 abandoned. Values above 4 are treated as "lot not ready" [confirmed]. Values 0 and 1 are probably empty or zoned
  [inferred].
- **Abandon reason** (3rd argument of SetState(4,…)): 1 desirability, 2 power, 3 water, 4 travel/commute
  (all from `UpdateLot`), 5 kicked out by a higher-wealth takeover, 6 decline due to negative demand.
- **BuildingFitsInZone(buildingZone, cellZone)** (W `0x695390`) [confirmed-W]: the zones are equal, or
  2←1, 3←1|2, 5←4, 6←4|5, 9←8, and {0xB,0xC,0xF}←0 (higher-density zones accept lower-density buildings).
- **GrowthStageFitsInZone(stage, zone)** (W `0x695340`) [confirmed-W]: zones 1/4 (low density) need stage ≤ 3,
  zones 2/5 (medium) need stage ≤ 6, and zones 3/6/7/8/9 accept any stage.

### Outcome enum (cISC4TractDeveloper::Outcome) [confirmed-W]

Read from the debug string switch at W `0x004CDF80`. The debug-growth dump uses it, as do the per-function
histogram (13 functions × 28 outcomes) and `ExecuteGrowthPlan`.

| # | name | # | name |
|---|---|---|---|
| 0 | unknown failure | 14 | stage too big |
| 1 | nothing to reoccupy | 15 | resident has different purpose |
| 2 | monopoly piece | 16 | resident has different wealth |
| 3 | no lot | 17 | not bigger than existing stage |
| 4 | lot not ready | 18 | historical |
| 5 | no lot configuration | 19 | just built recently |
| 6 | wrong zone | 20 | not enough construction resources |
| 7 | wrong purpose | 21 | poor roads |
| 8 | wrong wealth | 22 | wrong land value |
| 9 | wrong style | 23 | no fire coverage |
| 10 | too steep | 24 | no park |
| 11 | no building big enough | 25 | no water |
| 12 | stage too small | 26 | no power |
| 13 | all buildings too big | **27** | **success** |

Function columns in the debug dump are `_Grow_ _TOA__ _LPLC_ _LCLE_ _LCLA_ _LCLS_ _Bld__ _ILCS_ _ILS__ _IAL__ __PB__
_ILCC_ _IBC__`. They stand for Grow, TakeOverAbandoned, ListPossibleLotConfigurations,
ListCandidateLots_Existing/Aggregation/Subdivision, Build, IsLotConfigurationSuitable, IsLotSuitable,
IsAggregationLegal, PickBuilding, IsLotConfigurationCompatible and IsBuildingCompatible, which are histogram rows
0–12 [confirmed].

---

## 2. Agents and per-message behaviour

| Class | Agent levels (AddAgent) | Message handling [confirmed] |
|---|---|---|
| BDS | 0 once, 4 day, 6 month, 10 idle (flags 2) | **SimBegin and NewMonth** → `DoMonthlySimulation`. **NewDay** → `DoIdle(5 ms)` then `DoDailySimulation` [confirmed-W]. **SimIdle** → `DoIdle(ms)`. Always returns true. |
| GrowthDeveloper (one per developer exemplar) | 4 day, 10 idle | **NewDay** → `DoIdle(5 ms)`. Returns **false** (not finished) when a map update is running and `today - updateStartDay > state` (state is 1..0x14), so a day cannot advance until a slow desirability update catches up. **SimIdle** → `DoIdle(data3 ms)`. **ZoningChange 0x07699455** → `UpdateMaxCapacity(rect)`. **StateChanged 0xE8BDA2E7** → `UpdatePopulationCount`. |
| TractDeveloper | 6 month, 7 year | Only **NewYear** is handled: `yearsPassed++`. When `yearsPassed >= yearsBetweenStyles`, it resets to 0 and `currentStyleIndex = (index+1) % activeStyles.size()`. NewMonth is ignored. |
| LotDeveloper | 0 once, 4 day, 6 month | **NewDay** → `DoDailySimulation`, which can return false to continue on the next frame. SimBegin and NewMonth are ignored in `DoMessage`. Other messages: AnimationTick 0x86AD10EE (night lights), Insert/RemoveOccupant, TractPowerChanged 0x8A07AE00, TrafficMapChanged 0x69247DC7, which sets `canCheckCommute = demand(0x2000,0x20000).QueryDemandValue() > 0`, and CityEstablished 0x26D31EC4, which sets `noZotUntilDay = today + 31`. |
| ZoneManager | — | InsertOccupant: if the inserted building lacks the 0x80200090 ("zoned") occupant group, its footprint is dezoned with `PlaceZone(rect, 0, …)`. |

BDS also listens for TerrainHeightsChanged 0x278E6003 (`DoTerrainChanged`), Insert/RemoveOccupant (it keeps a list of
occupants with occupant type 0x150A/0x150B), StateChanged 0xE8BDA2E7 (`DoBuildingChanged` when a lot moves into or
out of states 2–3), command PlaceBuilding 0x0B99AE81 (`AddNewBuildingOccupant(type, pos, 2)`) and cheat 0x230E27AC.
The cheat codes are 0x491865E1 (ignore plumbing), 0x08DAD89A (ignore power) and 0x6AB6A2D7 (toggle coffee bar).

---

## 3. cSC4BuildingDevelopmentSimulator

Responsibilities: owns the GrowthDevelopers (a `map<DeveloperType, cISC4GrowthDeveloper*>`, Mac +0x6C/W+0x58), the
building/foundation/retaining-wall catalogues (`LoadBuildingProperties` etc.), and the **construction-resource
budget**. Each day it chooses which developer gets to grow and in which tract.

### Fields (Mac / Windows where verified)

| Mac | W | meaning |
|---|---|---|
| +0x6C/0x70/0x78 | +0x58/0x5C | developer map (header at +0x70 Mac) |
| +0x84 | — | cISC4TractDeveloper* |
| +0x88 | — | cISC4LotDeveloper* |
| +0x8C | **+0x70** | `cSC4Demand*` used as the **construction-resource counter** (supply = capacity, demand = in use) |
| +0x90 | **+0x74** | monthly/idle state (0 idle, 1 start, 2 waiting for developer maps, 3 plan growth, 4 plan decline) |
| +0x94 | +0x78 | developer iterator for state 2 |
| +0x98 / +0xA0 | +0x7C | growth plan / decline plan lists (dead code, see below) |
| +0xD8 | — | hash_map<GrowthProfile, lastFailDay> |
| +0xEC | +0xB0 | map<DeveloperType, dayOfLastSuccessfulBuild> |
| +0x104 | +0xBC | retry interval for failed plans (days) |
| +0x108/+0x10C | +0xC0/+0xC4 | construction resources min/max (0x47E2C380) |
| +0x110 | +0xC8 | construction resources per resident (0x47E2C381) |
| +0x114 | +0xCC | construction type selection bias (0x47E2C382) |
| +0x118 | +0xD0 | stage population multiplier[difficulty] (0x47E2C383) |
| +0x124 | +0xDC | "use growth plan" flag: **always 0** (constructor zeroes it and nothing sets it) [confirmed-W] |
| +0x125/126/129 | +0xDD/DE/E1 | cheat flags (ignore plumbing / power / coffee bar) |

### DoMonthlySimulation (W `0x696130`, mac:000f89d0) [confirmed]

```text
if state == 0:
    pop = Demand(0x1001).Supply + Demand(0x3001).Supply + Demand(0x4001).Supply   // R, C, I totals [inferred meaning]
    cap = clamp(uint(pop * resourcesPerResident), resMin, resMax)                // defaults 0.5, 100, 2000
    constructionDemand.SetSupplyValue(cap)
    for each developer: developer.StartMapUpdate()       // state 0 -> 1 in the developer
    state = 1
```

### DoIdle (W `0x699EA0`) [confirmed-W]

The states advance as follows. In state 2 it waits until every developer reports `IsMapUpdateFinished()` and returns
early if one has not. In state 3 it calls `PlanGrowth()`, but only if the plan flag is set. In state 4 it calls
`PlanDecline()` and returns to state 0. Because the plan flag is never set, the growth plan is never built.
`PlanDecline` always runs, but its list is executed by `ExecuteDeclinePlan` every day.

### Construction resources [confirmed-W]

`Free = max(0, supply − demand)`. `Claim(n)` succeeds when `demand + n ≤ supply`; it then runs
`AddToDemandValue(n); EndOfCycle()`. `Release(n)` adds −n. Capacity is claimed when a building is chosen (in
TractDeveloper `Build`). It is released by `cSC4Lot::SetState` (mac:001210a8) when the lot leaves state 2
(construction finished or cancelled). So at any time, the total **capacity under construction** is limited to
`clamp(0.5 × city population, 100, 2000)`.

### DoDailySimulation → Grow (W `0x69B6D0` → `0x69B0B0`) [confirmed-W]

```text
Grow():                                      // time-boxed: 3 ms (cSC4LimitTimer), adaptive check interval
  devs = copy of developer map
  while devs not empty and time remains:
      total = Σ over devs with ad = dev.SourceDemand.QueryActiveDemandValue() > 5 : (ad + bias)   // bias 0x47E2C382
      r = RandomUint32Uniform(total)
      pick the dev where r falls (weighted roulette)
      if dev.GrowthAndDeclineListsAreValid():
          list = dev.GetGrowthList()                // sorted by desirability, highest first
          item = first item whose tract is not marked for this developer type
          if none: ClearMarks(devType); item = list.front()
          if list not empty:
              MarkTract(devType, item.x, item.z)
              built = TractDeveloper.Grow({dev, item.x, item.z, item.maxStage}, outcomes, &flag)
              if built > 0: lastSuccessfulBuild[devType] = today
      devs.erase(dev)                              // each developer gets at most one tract per day
```

So per day each developer with active demand above 5 gets one attempt, in random order weighted by demand, at the
best-desirability tract it has not tried recently. The marks make it cycle through the growth list.

`ExecuteDeclinePlan` (W `0x696A50`) takes plans whose day has come. It drops a plan if the developer's active demand
is ≥ −19. Otherwise it calls `TractDeveloper.Decline(profile)` and stops after the first plan that removed something.
`PlanDecline` builds that list from developers with purpose ≠ 5 and active demand < −20, weighted by `bias − demand`.
Each plan takes the next tract from that developer's **decline list**. Plans are scheduled on consecutive days
(`plan.day = today + index`) [confirmed].

The dead-code `PlanGrowth`/`ExecuteGrowthPlan` path works the same way for growth. Its one extra rule is that
failures with outcomes {10, 11, 12, 14, 15, 16, 18, 22} (mask 0x47DC00) record the day, so the same profile is not
retried for `+0x104` days.

---

## 4. cSC4GrowthDeveloper: desirability, tract lists and occupancy

There is one per "developer" exemplar: group 0x6534284A, ExemplarType (0x10) == 4, keyed by property 0x21, which is
the DeveloperType. They are created in `BDS::InitDynamicDataFromExemplar` → `CreateDeveloper` [confirmed].

### Grids (all per developer, at the developer's own tract size 0x87CD6310)

| getter | Mac field | type | meaning |
|---|---|---|---|
| — | +0x128 | SimGrid<uint8> | number of cells in the tract whose zone fits this developer (`UpdateMaxCapacity`) |
| (scratch) | +0x12C | SimGrid<int16> | desirability being computed |
| GetDesirabilityMap | +0x130 | SimGrid<int16> | published desirability |
| GetPopulationMap | +0x134 | SimGrid<int16> | population |
| GetCapacityMap | +0x138 | SimGrid<uint16> | built capacity |
| GetMaxCapacityMap | +0x13C | SimGrid<uint16> | Σ max capacity of zoned cells |
| GetTotalCapacity | +0x124 | int | |
| growth list | +0x108 | vector<GrowthItem{x,z,maxStage}> | |
| decline list | +0x114 | vector<GrowthItem> | |

### Desirability update (DoIdle state machine, mac:000ff9e8, W `0x6B5F20`) [confirmed]

Each state runs one step per DoIdle call. State 0x12 is time-sliced.

```text
1    wait for initialised; record start day
2    D[x,z] = baseline                                         (0x87CD6357, default 80)
3    D += curve_LV( LandValueSim total|intrinsic grid )        (0x87CD6340; source chosen by 0x87CD633F)
4    D += curve_slope( LandValueSim slope grid )               (0x87CD6398)
5    D += curve_R$/R$$/R$$$( ResidentialSim proximity[w] )    (0x87CD6343..45)
6    D += curve_Co$$/Co$$$( CommercialSim proximity[0x3320/0x3330] )   (0x87CD6391/92)
7    D += curve_park( AuraSim park map )                       (0x87CD6341)
8    D += curve_poll[i]( PollutionSim grid i ), i=0..3 air/water/garbage/radiation  (0x87CD634B..4E)
9    D += curve_school( ResidentialSim +0x9C grid )            (0x87CD6348)
10   D += curve_hospital( ResidentialSim +0xA0 grid )          (0x87CD6349)
11   D += curve_crime( CrimeSim grid )                         (0x87CD634A)
12   D += curve_trip( min non-zero trip-length cell in tract, 255 if none )   (0x87CD6347)
13   D += curve_traffic( TrafficSim +0x24 grid )               (0x87CD6346)
14   D += curve_landmark( AuraSim +0x3C grid )                 (0x87CD6399)
15   for each "special building" (BDS +0xB0 list) with prop 0x4A5F7C4B containing this developer type:
         AddRadiusEffect(D, pos, magnitude/tractSize, radius, falloff) from prop 0x4A5F7315 triples   [one per call]
16   D += curve_transient( AuraSim +0x34 grid )                (0x87CD6397)
17   publish: Desirability = D; growth list = all tracts with zonedCellCount > 0 (maxStage=1); start occupancy iterator
18   (time-sliced) for every tract: Pop[x,z] = Cap[x,z] * clamp(D/100, 0, 1);
         TractDeveloper.UpdatePopulation({dev,x,z})            // per-building occupancy (section 5.7)
19   phantom demand (0x87CD6356): Σ over growth list where D < growthThreshold of
         Pop * min(1, (growthThreshold − D)/100); SourceDemand.AddToDemandValue(Σ − lastΣ)
20   sort growth list by D descending (CompareDesirability)
     decline list = tracts with D < declineThreshold (0x87CD6354, default = growth threshold), lowest first;
         shuffled when purpose ∈ 5..8 (industry)
     growth list: remove tracts with D < growthThreshold (0x87CD6355, default 80)
     stage assignment (below); state = 0
```

Here `curve_X(v)` is `AddCurve<T>`, which adds `cSC4ResponseCurve::GetOutputValue(v)` from the curve stored in the
named property. Curves are piecewise-linear point lists, so the "effect weights" in the exemplar are really
x→y response curves [confirmed names: `ProcessTwoGridsWithCallback<short,T>`, `AddCurve<T>`].

**Stage assignment (maximum growth stage per tract)** [confirmed]:

```text
size = city "size" for this target zone:
   R (1): max( Demand(0x1001).Supply, Demand(0x3001).Supply + 0.5*(regionPop/regionCount?)*Demand(0x3301,0x10000).Supply )
   C (2): Demand(0x3001).Supply + 0.5 * ratio * Demand(0x3301,0x10000).Supply   (ratio from NeighborsSim lists)
   I (3): Demand(0x4001,0x20000).Supply
x = size * BDS.GetStagePopulationMultiplier()            // 0x47E2C383[difficulty]
n = stageCount (0x87CD6360); s = n; acc = curve_stage[s](x)      // "Stage s vs. Size", 0x87CD6360+s
for i, item in enumerate(growth list sorted by D desc):
    while acc <= 100*i/len and s > 1:  s -= 1; acc += curve_stage[s](x)
    item.maxStage = s
```

So the most desirable tracts get the highest stages, and the share of tracts at each stage comes from the
"Stage N vs. Size" curves, which are percentages as a function of city size. The exact meanings of the R/C size
formulas (the neighbour ratio and the regional demand 0x3301) are [inferred].

**UpdateMaxCapacity(rect)** (ZoningChange) [confirmed]: for each tract in the rect it counts the cells where
`BuildingFitsInZone(dev.zone, cellZone)`, then stores the count (+0x128) and
`Σ BDS.GetMaxCapacity(cellZone, purpose, wealth)` (+0x13C).

---

## 5. cSC4TractDeveloper: building inside one tract

`Grow(profile, outcomes&, &flag)` (vt+0x14, W `0x70D130`) returns the capacity it built.

### 5.1 Setup [confirmed]

```text
dev = profile.dev; cur = (x,z); maxStage = profile.maxStage
existingCap   = dev.CapacityMap[x,z]                      (+0xB4)
desired       = max(1, dev.SourceDemand.QueryActiveDemandValue())   (+0xB8)
desiredMax    = desired * overbuild                       (+0xBC, 0x47E2C500 default 1.5)
built = 0                                                 (+0xC0)
debug record: pop, maxCap, desirability, free construction resources
```

Two passes are made: pass 0 is "fussy" (+0xC8 = 0) and pass 1 is "not fussy". The number of passes is bounded by
`tractSize² + 1`.

```text
for pass in (fussy, not fussy):
    ListExistingLots()                     // all lots in tract cells, random order (RandomIterator)
    r = TakeOverAbandoned();               if r == success: done
    if ListPossibleLotConfigurations() == success:
        ListCandidateLots_Existing(); ListCandidateLots_Aggregation();
        if purpose != 5: ListCandidateLots_Subdivision()
        if candidates: ScoreCandidateLots(); SortCandidateLots()      // ascending score
            for c in candidates:
                r = (purpose == 5) ? BuildFarm(c) : Build(c)
                if r == 20 (no construction resources): stop
                if built >= desired: stop (success)
                if r == success and purpose in 5..8: stop (success)   // industry builds once per attempt
        else outcome = (best failure or "no lot")
    if not fussy-pass and outcome != success: stop
```

### 5.2 TakeOverAbandoned (W `0x70A3D0`) [confirmed]

The candidates are lots of this developer's zone type that are either **abandoned (state 4)**, or are occupied
(state 3) by a lower wealth while the building's own wealth is at least the developer's wealth. The second case
needs `kickOutLowerWealth` (0x47E2C540, default true). Each candidate must pass `IsLotSuitable` (success or
"historical") and have capacity for this developer type. **Utility caps**: for a building stage above the developer's
power cap (0x87CD6373) without power → "no power". Above the water cap without water → "no water". Above the fire
cap without fire coverage → "no fire coverage". Above the park cap with no park aura → reject ("no park").

```text
if Σ abandonedCapacity > (desired − built) * reoccupyStart (0x47E2C501[0], default 0):
    score + sort candidates
    for c in sorted:
        if c.lot.state == 3: LotDeveloper.AbandonBuilding(c.lot, 5)     // kick out lower wealth
        LotDeveloper.ReoccupyBuilding(c.lot, devType)
        built += c.capacity
        if Σtaken > (desiredMax − built) * reoccupyStop (0x47E2C501[1], default 1.2): break
    return success
```

### 5.3 Lot configurations (ListPossibleLotConfigurations / IsLotConfigurationSuitable, W `0x709650`/`0x704C00`) [confirmed-W]

`LotConfigurationManager.GetLotConfigurations(set, filter)` is called with a cached `cLotSizesFilter` per
(wealth, purpose). Each configuration is then checked in this order:

1. Purpose matches (7), then wealth matches (8).
2. Config growth stage ≤ `maxStage` from the growth list (14 "stage too big").
3. For R/Cs (purpose 1–3), when a style list exists, the config must be in the current style
   (`activeStyles[currentStyleIndex]`). When "use all styles at once" is set, any active style is accepted (9).
4. The config has buildings (0 if not).
5. Fussy pass: config max capacity (vt+0x34) ≥ desired, otherwise 11 "no building big enough".
6. Config min capacity (vt+0x30, ignored at stage 1) ≤ desiredMax + existingCap, otherwise 13 "all buildings too big".

### 5.4 IsLotSuitable(lot, redevelop) (W `0x707A60`; inlined in `ListCandidateLots_Existing`) [confirmed-W]

The checks run in this order:

1. `BuildingFitsInZone` (6).
2. When redeveloping, a developed lot younger than **180 days** (lot+0xA0 < 0xB4) gives 19 "just built recently".
3. The land value at the lot is outside the developer's range 0x87CD6333 (min, max), giving 22.
4. Lot state 2 or > 4 gives 4 "lot not ready".
5. The lot-manager footprint check (vt+0x50) must give 7 or 8. Otherwise a value of 2 gives 6, 3 gives 3 and 1
   gives 4.
6. For a developed lot: a farm, or IM replacing ID (purpose 7 vs 6), gives 15. A wealth or purpose mismatch, when
   kick-out rules don't allow it, gives 16. Historical gives 18. A building stage not below `maxStage` (equal wealth),
   or above it (different wealth), gives 17 "not bigger than existing stage".
7. **Road access**: R checks `TrafficSim tripLengthMap[x,z] != 0`. I purposes (6–8) check lot vt+0x74(purpose).
   Others check lot vt+0x70. A failure gives 21 "poor roads".

The result is cached per lot (+0x124).

### 5.5 Candidate lots and scoring [confirmed]

The candidate types (CandidateLot+0) are: 1 = reoccupy/refurbish an abandoned building, 2 = build on an existing lot
(rebuild or redevelop), 3 = aggregate several lots, 4 = subdivide. The debug strings are "Refurbish", "Build new",
"Aggregate" and "Subdivide".

`FindNetworks(c)` (W `0x705040`) counts the network cells along each of the 4 sides of the candidate rect into 4
vectors, each indexed by side 0..3. The network types (enum) are:

| vector | Mac off | W off | networks counted |
|---|---|---|---|
| v0 | +0x7C | +0x70 | street (3), dirt road (11) |
| v1 | +0x88 | +0x7C | road (0), one-way road (10) |
| v2 | +0x94 | +0x88 | avenue (6) |
| v3 | +0xA0 | +0x94 | rail (1) |

`ScoreCandidateLots` (W `0x705750`). **Lower score is better.** The sort is ascending.

```text
(wRoad, wAve, wRail, wRand) =  R: (1, 2, 10, 0)   C: (-2, -5, 0, 0)   I: (-1, -1, -5, 20 [Windows only])
for c in candidates:
    c.score = 0
    if c.type != 1:  FindExistingCapacity(c)
                     c.score = c.nonVacantCap * demolitionCost[0] + c.abandonedCap * demolitionCost[1]   // 0x47E2C510: 1.0, 0.2
    if (Mac or zone != I):
        if c.type == 3: c.score += (c.lotCount − 1) * newLotCost[0]     // 0x47E2C511: 10
        if c.type == 4: c.score += newLotCost[1]                         //              5
    if wRail != 0:                                   // BUG: C has wRail = 0, so C network weights are never applied
        for side in 0..3: c.score += v1[side]*wRoad + v2[side]*wAve + v3[side]*wRail
    if wRand != 0: c.score += RandomFloat01() * wRand     // Windows 1.1.641: industry gets random jitter (0..20)
```

The Windows build differs from Mac here [confirmed-W]. For industry, Windows skips the aggregation and subdivision
lot costs and adds `rand*20`. Streets (v0) are never weighted on either platform. The effect is that R prefers lots
on streets away from roads, avenues and rail, while I prefers roads, avenues and especially rail. The C weights are
dead because of the guard.

### 5.6 Build and PickBuilding [confirmed]

`PickBuilding(c)` (W `0x709BA0`):

```text
FindExistingCapacity(c)
style = 0x3000 if BDS.LotDeveloper.NeedsAnchorBuilding(c.rect, purpose)          // anchor/"0x3000" occupant group
        else weighted-random over dev.StylePercentages (0x87CD63A0 pairs groupID,percent)  // 0 if none
lotConfigs = { cfg in possibleConfigs : (style==0 or cfg.IsInGroup(style)) and
                                        IsLotConfigurationCompatible(cfg, c) == success }
chosenCfg = uniform random from lotConfigs                                      (+0x108)
minCap = c.existingCap + 1 (not fussy) or c.existingCap + (desired − built) [+ c.vacant for type 4]
         (÷ lotCount for subdivision); minCap >= c.existingCap
maxCap = max(minCap + 1, c.existingCap + desiredMax − built)                       (+0x114/+0x118)
for family in chosenCfg.buildingFamilies:   for b in BDS.GetBuildingTypesByFamily(family):
    r = IsBuildingCompatible(b)       // purpose(7), wealth(8), style(9), cap<minCap(11), cap>maxCap(13), monopoly(2)
    if r == success: good.add(b)
    elif r == monopoly and dev.MonopolyUse == 1: monopolyOnly.add(b)
if good empty: good = monopolyOnly
chosenBuilding = uniform random from good                                       (+0x10C)
```

Monopoly use (0x87CD6381, default 1). Property 0x2781283F on the building marks a "monopoly piece". With modes 0
and 1 monopoly pieces are rejected, but mode 1 still lets them be used as a fallback. With mode 2 only monopoly
pieces are allowed.

`Build(c)` (W `0x70B440`):

```text
r = PickBuilding(c); if r != success: return r
cap = BuildingProfile(chosenBuilding).capacity
if !BDS.ClaimConstructionResources(cap): return 20
switch c.type:
   3: PerformAggregation(c, &lot)       // merge lots into one
   4: BDS.ReleaseConstructionResources(cap); PerformSubdivision(c); return success   // subdivision re-picks per sub-lot
   2: demolish existing lot (Demolition service, flag 0x800); built −= c.nonVacantCap
LotDeveloper.StartConstruction(lot, chosenCfg, chosenBuilding, animate=true)
built += cap; return success
```

`BuildFarm` (W `0x70B640`, purpose 5) [confirmed, partially read]:

1. It collects field cells into a list (n entries). If n < 15 it fails.
2. It claims `n + 30` construction resources, then releases the 30 again and calls `Build` (the farmhouse).
3. On success it picks a random field lot configuration from building property 0x29B55F73 and creates field lots on
   the collected cells.
4. On failure it releases the n.

### 5.7 UpdatePopulation (per-building occupancy, W `0x706BC0`) [confirmed]

This is called for every tract from the developer's occupancy step (state 18):

```text
d = dev.Desirability[x,z]; rng seeded with x*0x100000 + z*0x10       // deterministic per tract
for lot in lots in tract with lot.IsDeveloped and lot.wealth == dev.wealth and maxCap(devType) > 0:
    occ = (d/100) * (1 + rng.Uniform(−variation, +variation))          // 0x47E2C530, default 0.2
    occ = clamp(occ, 0, ...); if occ >= 1: occ = 1 − rng.Uniform(minVac[0], minVac[1])   // 0x47E2C531: 0.01..0.1
    lot.SetPopulation(devType, round(occ * maxCap))
```

**Occupancy (and so population/jobs) equals tract desirability ÷ 100**, with a ±20% jitter, and it is capped just
below 100% [confirmed].

### 5.8 Decline (W `0x709200`) [confirmed]

This runs for one tract, in random order. It considers every lot in state 3 that is **older than 59 days**
(lot+0xA0 > 0x3B) and whose building purpose and lot wealth match the developer. Each such lot gets
`AbandonBuilding(lot, 6)`. The return value is the summed capacity. There is no per-lot limit, so every matching
building in the tract is abandoned.

---

## 6. cSC4LotDeveloper: per-lot upkeep, abandonment and construction

### DoDailySimulation (W `0x6CD770`) [confirmed-W]

This uses a safe lot iterator that persists across days (+0xB0). Each day it visits `lotCount/28 + 1` lots within a
**5 ms** budget. It returns false while unfinished, so the scheduler resumes it. Every lot is therefore visited about
once per 28 days. At the start of a cycle it latches the "fix facings" and "commute check" flags. It also sets the
"empty-lot checks" flag when `noZotUntilDay < today`.

### UpdateLot (W `0x6CD3A0`) [confirmed]

```text
if lot has building b with purpose != 0:
    newState = lot.state
    if !TestDesirability: newState = 4, reason 1
    if !TestPower:        newState = 4, reason 2
    if !TestWater:        newState = 4, reason 3
    if !TestTravel:       newState = 4, reason 4
    if newState != lot.state: lot.SetState(newState, 0, reason)
    lot.SetCondition(TestCondition(...))      // distressed / renewed etc.
    if coffee-bar cheat and purpose == 2: TestCoffeeBar
else (empty or plopped): zot checks (TestTravel for R/C zones when commute data exists, TestPower)
business-unit bookkeeping (UpdateTravelDesignates/Populations, PickUnitToJoin)
```

**TestDesirability** (W `0x6C47C0`) [confirmed-W]:

```text
dev = BDS.GetDeveloper(GetDeveloperType(purpose, lot.wealth)); if farm(5): pass
if dev.Desirability[lotCell] * 0.01 < abandonThreshold (0x47E2C401[0], default 0.2):
    n = lot.prop(0xC999E206) ; lot.prop = n+1
    r = Random(seed = x<<16 + z).Sint32RangeUniform(0, 7)       // fixed per lot position
    return (n+1) <= r+2           // fail once count exceeds 2..9 consecutive low checks (~3..10 months)
else lot.prop(0xC999E206) = 0; pass
```

**TestCondition** (W `0x6C4630`) [confirmed, details partly inferred]. For a developed lot, the condition step is
`buildingWealth − lotWealth` (0, 1 or 2). It is incremented when the lot is older than the stay-new time
(0x47E2C400 days) and the lot's value (vt+0xF4) is below distress 0.4, or below renew 0.6 when wealth is mismatched.
The result is ≥3 distressed, 0 normal (1 if prop 0xE999B11F), and so on. Abandoned lots step their condition up over
time.

**StartConstruction** (W `0x6CC490`) [confirmed]: picks a business unit and clears zots and blocking objects. It
then sets the lot state to **2** and optionally levels the terrain (0xE900EA24..2A). With animation it creates a
`cSC4ConstructionOccupant` (class 0xA97F909E) that later completes the building (state 3, releasing construction
resources in `cSC4Lot::SetState`). Without animation the state goes straight to **3**. The construction **duration**
lives in cSC4ConstructionOccupant, which has not been analysed.

`AbandonBuilding(lot, reason)` = `lot.SetState(4, 0, reason)`. `ReoccupyBuilding(lot, devType)` sets the condition
from `buildingWealth − devWealth`, then `SetState(3, devWealth)` and `SetPopulation(devType, maxCap(devType))`.
[confirmed-W]

---

## 7. Zoning: cSC4ZoneManager / cSC4ZoneDeveloper (brief)

- ZoneManager owns the zone grid (`SimGrid<int8>`), `PlaceZone(region, type, …)` (vt+0x2C) and the per-cell
  development failure counters (`GetDevelopmentFailureCount`, map keyed by (x<16, z<19)) [confirmed]. Its only
  message behaviour is dezoning under non-zoned buildings (section 2).
- ZoneDeveloper (vtable W `0x00AB3830`) turns zone drags into lots. It does this through `DoParcellization` →
  `Parcellize`/`SimpleParcellize`, `LayInJogStreets` (W `0x730D30`, prior art), `DetermineLotSize` (W `0x732BF0`,
  prior art) and `CreateLotToFaceRoad`. Lots are pre-subdivided at zoning time. Growth then works only on existing
  lots, aggregating or subdividing them as needed. This class was not analysed further.

---

## 8. Tuning properties

BDS tuning exemplar: group/instance `0x6534284A/0xE7E2C2DB/0xE8DA7677`, type 0xA52160F5.

| ID | owner | meaning | default (code) |
|---|---|---|---|
| 0x47E2C380 | BDS | construction resources min,max | 100, 2000 |
| 0x47E2C381 | BDS | construction resources per resident | 0.5 |
| 0x47E2C382 | BDS | type-selection bias added to each developer's active demand in the roulette | 10 |
| 0x47E2C383 | BDS | stage population multiplier [difficulty] | (vector) |
| 0x47E2C400 | LotDev | stay-new time (days before distress), uint16 at +0xC0 | constructor default not read |
| 0x47E2C401 | LotDev | thresholds abandon, distress, renew | 0.2, 0.4, 0.6 |
| 0xE900EA24..2C | LotDev | flatness/levelling, foundations, night-light update interval/count | — |
| 0x6916516B, 0x2A2D7824 | LotDev | (uint, response curve) unknown | — |
| 0x47E2C500 | TractDev | overbuild factor | 1.5 |
| 0x47E2C501 | TractDev | reoccupy start, stop | 0.0, 1.2 |
| 0x47E2C502 | TractDev | redevelop size threshold | 1.5 (read; use not traced) |
| 0x47E2C510 | TractDev | demolition cost occupied, abandoned | 1.0, 0.2 |
| 0x47E2C511 | TractDev | new-lot cost aggregate (per extra lot), subdivide | 10, 5 |
| 0x47E2C530 | TractDev | occupancy variation | 0.2 |
| 0x47E2C531 | TractDev | min vacancy range | 0.01, 0.1 |
| 0x47E2C540 | TractDev | kick out lower wealth | true |
| 0x47E2C550 | TractDev | available styles (R/C) | — |
| 0x47E2C551 | TractDev | years between style changes | 40 |

Per-developer exemplars (ExemplarType 4) store their fields at the Mac GrowthDeveloper offsets below.

| ID | meaning | default / field |
|---|---|---|
| 0x21 | developer type (0x1010…) | +0xE4 |
| 0x87CD6300 | target zone (1 R, 2 C, 3 I) | +0xE8 |
| 0x87CD6301 | demand source ID → `DemandSim.GetDemand(id, 0x20000)` | +0x34 |
| 0x87CD6310 | tract size (power of 2) | SetTractSize |
| 0x87CD6330 / 6332 | wealth / purpose | +0x3C / +0x38 |
| 0x87CD6333 | land value range (min,max), 0,0 disables | +0x45/+0x46 |
| 0x87CD633F | use total vs intrinsic land value | +0x47 (false) |
| 0x87CD6340..634E, 6391..6399 | desirability response curves (section 4) | +0x48..+0x8C |
| 0x87CD6354 | decline threshold | = growth threshold |
| 0x87CD6355 | growth threshold | 80 |
| 0x87CD6356 | phantom demand | false |
| 0x87CD6357 | baseline desirability | 80 |
| 0x87CD6360 / 6361..6368 | stage count / "Stage N vs size" curves | +0xA0 |
| 0x87CD6370..6373 | water / fire / park / power caps (max stage without the service) | +0xAD/+0xAE/+0xAF/+0xAC |
| 0x87CD6380 | (obsolete) abandonment thresholds per density | 0.2,0.2,0.2 |
| 0x87CD6381 | monopoly-piece use mode | 1 |
| 0x87CD63A0 | style percentages (groupID, percent) pairs | +0xCC |
| 0xCAB1CE3A / 0xCAB1CE3C | two uint vectors, unknown | +0x140/+0x14C |

Building and lot properties read: 0x2781283F (monopoly piece flag), 0x4A5F7C4B/0x4A5F7315 (special building
desirability radius effects per developer type), lot sim properties 0xC999E206 (low-desirability counter) and
0xE999B11F (condition flag).

---

## 9. Reimplementation summary

1. Monthly: compute the construction budget. Each developer recomputes desirability = baseline + Σ curve(input
   maps), plus radius effects. It then builds a growth list (zoned tracts, sorted by desirability, filtered by the
   growth threshold, with a max stage taken from the city-size stage curves) and a decline list.
2. While that runs: population = capacity × clamp(desirability/100), per building with ±20% jitter.
3. Daily: each developer with active demand > 5 gets one weighted-random turn. It takes its next unmarked
   best-desirability tract and tries to build `demand … 1.5×demand` capacity there. In order it tries: reoccupy
   abandoned or lower-wealth buildings; then score existing, aggregate and subdivide lot options (demolition and new-lot
   costs, network adjacency); then pick a random compatible lot config in the current style and a random building whose
   capacity fits; then claim construction resources and start construction.
4. Daily: developers with active demand < −20 abandon every developed building older than 59 days in the next
   decline-list tract.
5. Per lot about monthly: abandon when tract desirability < 20% for a lot-specific 3–10 consecutive checks, or when
   power, water or commute fail.

---

## 10. Open questions

- Exact semantics of Demand IDs 0x1001/0x3001/0x3301/0x4001 and the NeighborsSim ratio in the stage-size formula.
  These need the demand subsystem write-up.
- What `TestPower`, `TestWater` and `TestTravel` require in detail, including the commute-time threshold and the use
  of the `canCheckCommute` flag. The Windows addresses are known.
- `IsLotConfigurationCompatible` (W `0x706500`): slope ("too steep"), corner-lot and facing rules were not read in
  detail.
- `PerformAggregation`/`PerformSubdivision` geometry and `IsAggregationLegal` rules.
- Construction duration (cSC4ConstructionOccupant) and how the 0x47E2C502 redevelop threshold is used.
- `LocationIsOKForNewBuilding` and BDS slot order 5–29 are mapped by interface order only, because overloads may be
  reordered. Of the overloads, only the `GetDeveloper` pair was verified (Windows +0x80 = by DeveloperType).
- Field offsets for the Windows GrowthDeveloper and TractDeveloper beyond those listed were not catalogued.
