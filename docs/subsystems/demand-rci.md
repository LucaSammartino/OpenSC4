# RCI demand and population (cSC4DemandSimulator, cSC4Demand, R/C/I simulators)

Target: `SimCity 4.exe` 1.1.641 (GOG), image base `0x00400000`. Read [`docs/sim-engine.md`](../sim-engine.md) first
for the scheduler (agent levels, `SendAgentMessage`, the "return false = not finished" convention).

Legend: **[confirmed]** read in code (named Mac Rev A x86 pseudocode and/or the Windows exe),
**[inferred]** consistent with evidence but not proven.
Windows addresses: [`re/symbols/demand-rci.tsv`](../../re/symbols/demand-rci.tsv) (84 checked functions). Mac x86
addresses are given in brackets `[Mac 0x…]`. The Windows code was compared against the Mac code for every formula
below; where only the Mac build was read, the claim says so.

| Class | CLSID | Windows vtables (primary / MessageTarget2 / Serializable) |
|---|---|---|
| cSC4DemandSimulator | `0x2990BFF2` | `00AB0E48` / `00AB0E38` / `00AB0E20` |
| cSC4Demand | `0x8990C12D` | `00AA2758` (cISC4Demand, 36 slots) / — / `00AA273C` |
| cSC4ResidentialSimulator | `0x4990C013` | `00AB2C10` / `00AB2BFC` / `00AB2BE4` |
| cSC4CommercialSimulator | `0x8990C080` | `00AB0CCC` / `00AB0CBC` / `00AB0CA4` |
| cSC4IndustrialSimulator | `0xA990C022` | `00AB1530` / `00AB1520` / `00AB1508` |

---

## 1. Big picture

RCI demand is **data-driven**. It is a graph of `cSC4Demand` nodes, one per *demand key*. Every node holds a
*supply* and a *demand* accumulator. The nodes and the links between them are created from **Demand exemplars**
(Drives / Satisfies / Contributes / Caps / Sums). Hard-coded C++ only does four things: **[confirmed]**

1. **Measures** the city. A monthly census walks every lot (`PerformCensus`) and counts residents by wealth and
   education level, plus jobs by C/I type. Building state changes add or remove capacity and "agent effects"
   (`UpdateCapacity`, `AddAgentEffect`). Each measurement is injected as a supply or demand *delta* on a node.
2. **Propagates** deltas through the exemplar links until nothing changes by more than 0.01 (`DoDemandIndexLinks`).
3. **Modifies** each R/C/I node with a tax modifier, an economy modifier and a cap (`UpdateTaxAndEconomyModifiers`,
   `EvaluateConnectionUtilization`, the "Caps" links).
4. **Publishes** each node's *active demand* = clamp(demand − supply, …), as `kSC4MessageActiveDemandChanged`
   (`0x426840A0`). It also exports unmet demand to neighbour cities (`ExportDemand`).

So the familiar "R needs jobs, C/I need workers and customers" logic is **not in the exe**. It lives in the Demand
exemplars (group `0xC7BB4816`). A reimplementation needs the exemplar data from `SimCity_1.dat`. That data is not
in this repo; see §9.

## 2. Demand keys

`key = (demandIndex << 16) | demandId` **[confirmed]** (`CalculateJobsPerUnitOfDemand` loads exemplar instance
`demandId + demandIndex`; `DoDailySimulation` reads keys `0x21010…0x24400`; regional code uses `id + 0x10000`).

| Index | Meaning |
|---|---|
| `0x00000` | local city node (census and building effects go here) **[confirmed]** |
| `0x10000` | regional node (sum over neighbour cities; exports to neighbours are added here) **[confirmed]** |
| `0x20000` | city total, the node shown in the RCI UI / history (prior art: "city totals") **[inferred]** |

The low 16 bits are the demand ID:

| ID | Meaning |
|---|---|
| `0x1010 / 0x1020 / 0x1030` | R$ / R$$ / R$$$ (= `DeveloperType`; BDS `GetDeveloperType(purpose, wealth)` returns these) **[confirmed]** |
| `0x1011 / 0x1021 / 0x1031` | census: residents by wealth (= devType + 1) **[confirmed]** |
| `0x1001, 0x3001, 0x4001` | totals of R population / C jobs / I jobs (summed nodes; used by `GetNeutralTaxRate`) **[confirmed use, summing inferred]** |
| `0x2010 / 0x2020 / 0x2030` | workforce (labour) by wealth; target of C/I "Drives" links used as jobs-per-unit **[confirmed use]** |
| `0x2111…0x2134` | census: workers by wealth (`0x21w0`) × education level 1–4 (`+1…+4`) **[confirmed]** |
| `0x2000` | node read for history `0x6A931184` and unemployment (`0x8A931172`) **[confirmed use; "total workforce" inferred]** |
| `0x3110 / 0x3120 / 0x3130` | CS$ / CS$$ / CS$$$ (commercial services) |
| `0x3320 / 0x3330` | CO$$ / CO$$$ (office) |
| `0x4100 / 0x4200 / 0x4300 / 0x4400` | I-R (agriculture) / I-D (dirty) / I-M (manufacturing) / I-HT |
| `devType + 1` (`0x3111…0x4401`) | census: occupied C/I capacity by type **[confirmed]** |
| `0x3800` | commercial cap-relief resource, credited by connections, airports and seaports **[confirmed]** |
| `0x4800`, `0x4900` | industrial cap-relief resources (both get the same delta) **[confirmed]** |

`PurposeType` (first field of `BuildingProfile`): 1 = residential, 2 = CS, 3 = CO, 5 = agriculture, 6 = dirty,
7 = manufacturing, 8 = high tech. Purpose 4 (probably tourism) is ignored by the census (mask `0x1EC`). **[confirmed mask]**

## 3. cSC4Demand (one node)

### Layout (identical offsets in both builds) **[confirmed]**

| Off | Type | Field |
|---|---|---|
| +0x00 / +0x04 | vptr | cISC4Demand / cIGZSerializable |
| +0x08 | bool | loaded-from-save |
| +0x09 | bool | initialized |
| +0x10 | u32 | id (key) |
| +0x14 | float | supply |
| +0x18 | float | demand |
| +0x1C | float | newSupply (pending delta this cycle) |
| +0x20 | float | newDemand (pending delta) |
| +0x24, +0x28 | float | (debug lock value/flag, unused here) |
| +0x2C | float | activeMax (default `0x7E7FFFFF` ≈ +8.5e37) |
| +0x30 | float | activeMin (default `0xFE7FFFFF` ≈ −8.5e37) |
| +0x34 | float | activeDemand |
| +0x38 | bool | (=1) |
| +0x3C | float | economyModifier |
| +0x40 | float | taxModifier |
| +0x44 | float | demandCap (SC4Percentage, 0..1) |
| +0x48 | u32 | regionUse (from `0x47BB3F60`; 1 = quantity lives in *supply*, 2 = in *demand*) [inferred meaning] |
| +0x4C / +0x50 | ptr | city / message server |

### Methods

- **AddToSupplyValue / AddToDemandValue(x)**: `newSupply += x` / `newDemand += x`. **[confirmed `005D4C60/70`]**
- **EndOfCycle** (slot 30, `005D4CF0`): **[confirmed]**
  ```
  changed = newSupply != 0 || newDemand != 0
  supply += newSupply; demand += newDemand; newSupply = newDemand = 0
  CalculateActiveDemand(false)        // vtable +0x8C
  return changed
  ```
- **CalculateActiveDemand(bool smooth)** (slot 35, `005D4E60`): **[confirmed, both builds]**
  ```
  base   = clamp(demand - supply, min, max)
  mod    = economyModifier + taxModifier
  bonus  = min(max * mod, demand * mod)        // modifier is a fraction of raw demand, bounded by max
  target = clamp(base + bonus, min, max)
  target = max(min, min(target, max - max * demandCap))   // cap 1.0 => active <= 0
  if smooth:
      step = (max - min) * 0.07
      d = target - active
      if |d| >= step:                           // slew at most 7% of range per call
          if d > 0: active = (target > 0 && active < 0 && active + step < 0) ? 0 : active + step
          else:     active = (target < 0 && active > 0 && active - step > 0) ? 0 : active - step
      else active = target
  else active = target
  post kSC4MessageActiveDemandChanged(0x426840A0, data1 = id,
       data2 = round(active >= 0 ? active / max * 1000 : -active / min * 1000))   // -1000..+1000
  ```
  The only callers pass `smooth = false` (EndOfCycle, and SimulationBegin when the node was not loaded from a
  save). No caller with `true` was found in either build, so **the 7% slew appears unused** and demand responds
  immediately. **[inferred from caller search]**
- **SimulationBegin** (slot 5, `005D4B90`): if not loaded, recalculate; then repost `0x426840A0` with the
  normalized value. **[confirmed]**
- **SetActiveDemandMax/Min** keep max ≥ min. **[confirmed Mac]**

## 4. cSC4DemandSimulator

### 4.1 Fields

Mac offsets (Windows offsets in parentheses where seen; MSVC/STL layout makes Windows ≈0x30–0x38 smaller after +0x34):

| Mac off (Win) | Field |
|---|---|
| +0x0C | bool loadedFromSave |
| +0x0D | bool initialized |
| +0x14 (+0x14) | census state (0 = start, 1 = scanning, 2 = record history, 3 = done for the month) |
| +0x18 (+0x18) | `ScanningIterator*` over cell rect `(0,0)-(CellCountX-1, CellCountZ-1)` |
| +0x1C (+0x1C) | `map<key,float>` current census |
| +0x34 (+0x28) | `map<key,float>` previous census (deltas are taken against it) |
| +0x4C | city; +0x50 msg server; +0x54 (+0x3C) BuildingDevelopmentSim; +0x58 (+0x40) LotManager; +0x5C (+0x44) NeighborsSim; +0x60 (+0x48) OrdinanceSim; +0x64 (+0x4C) ResidentialSim |
| +0x68 (head ptr +0x50) | `map<key, cISC4Demand*>` demand nodes |
| +0x80 | `multimap<srcKey, EconomyLink>` with `EconomyLink {serial, srcKey, targetKey, linkType(prop ID), factor}` |
| +0x98…+0xC4 (+0x68…+0x94) | `u32 salary[12]` (`0x8A63F2A1`, default 100) |
| +0xC8, +0xD0 | lists of regional cities (previous / current connected) |
| +0xD8, +0xDC (+0xA0, +0xA4) | extra Co / I economy bonus (added in §4.5; only written by Read/Init=0 in Mac) |
| +0xE0 (+0xA8) | commercial cap relief (accumulated) |
| +0xE4 (+0xAC) | industrial cap relief |
| +0xE8 (+0xB0) | curve: neutral tax rate vs population (`0x4A1F38B5`) |
| +0xEC (+0xB4) | curve: tax modifier vs rate/neutral (`0x4A1F38B6`) |
| +0xF0 (+0xB8) | days of tax penalty (`0x4A1F38B7`, default 90) |
| +0xF4 (+0xBC) | `vector<float>[13]` connection cap relief by connection type (`0x4A1F38C5`, default 0) |
| +0x100…+0x110 (+0xC8…+0xD8) | relief coefficients `0x4A1F38C6…CA` (defaults 1.0, 2.0, 20.0, 0.1, 20.0) |

Agent-effect map (static): `map<buildingType, list<AgentEffect{demandKey, satisfies, creates}>>`.

### 4.2 Init (`006A8C20`) **[confirmed]**
1. `StaticInit`. Enumerate every exemplar of type `0x6534284A` and call `InitDynamicDataFromExemplar` on each.
   Exemplars with ExemplarType (`0x10`) == 3 become demand nodes through `CreateDemandFromExemplar` (§4.3).
2. Acquire city, LotManager, ResidentialSim, BDS, NeighborsSim, OrdinanceSim. Build the scanning iterator.
   If not loaded: zero relief fields and `ZeroCensusInfo(prevCensus)`; state = 0, else 3.
3. `ReadSimulatorTunableValues`, `ResetRegionalDemands(true)`.
4. Register 12 history series (`0x2A67A399 … 0x8A67A3CC`, one per R/C/I type).
5. Subscribe to messages `0xE8BDA2E7` (kMessageStateChanged), `0x8A0D15DA` and `0xEA0D163E` (neighbour connection
   created/deleted, posted by `cSC4NeighborsSimulator::CreateConnection`/`DeleteConnection` [Mac 0x51CF0/0x5211C]),
   `0x26C63343` (kSC4MessagePreSave) and `0xCA1B151C` (kSC4MessageTypeIngredChangesMade, i.e. tuning reload).
6. `AddAgent(…, level 0/4/6/10, "cSC4DemandSimulator", flags 2)`; then `DoDemandIndexLinks`.

### 4.3 Demand exemplars → nodes (`CreateDemandFromExemplar`, `006A5350`) **[confirmed]**

| Property | Use |
|---|---|
| `0x11` | serializable class for the node (default `0x0797A4E8`) |
| `0x21` | demand key (default = exemplar instance) |
| `0x47BB3F20` / `0x47BB3F21` | initial supply / demand (SetSupplyValue / SetDemandValue). Only applied when not loaded from a save. |
| `0x47BB3F10` / `0x47BB3F11` | active min / max (SetActiveDemandMin/Max) |
| `0x47BB3F31` Satisfies | pairs (targetKey, pct) → link, factor = pct × 0.01 |
| `0x47BB3F30` Drives | pairs (targetKey, pct) → link |
| `0x47BB3F32` Contributes | pairs (targetKey, pct) → link |
| `0x47BB3F41` Caps | list of targetKeys → link, factor 1.0 |
| `0x47BB3F50` Sums | list of keys → link, factor 1.0 |
| `0x47BB3F60` | regionUse (u8) |

`0x47BB3F40` "Limits growth" is **not** read here. It is probably read by the growth developers. **[inferred]**

### 4.4 Link propagation (`DoDemandIndexLinks`, `006A4E70`) **[confirmed]**
```
repeat:
  stable = true
  for src in nodes (sorted by key):
    s = 0; d = 0; hasSums = false
    for link in links[src.key]:
      tgt = nodes[link.target]
      switch link.type:
        Drives      (0x47BB3F30): v = src.newSupply * f; if |v| > .01: tgt.AddToDemand(v); stable = false
        Satisfies   (0x47BB3F31): v = src.newSupply * f; if |v| > .01: tgt.AddToSupply(v); stable = false
        Contributes (0x47BB3F32): v = src.newDemand * f; if |v| > .01: tgt.AddToDemand(v); stable = false
        Caps        (0x47BB3F41): sup = max(.01, src.supply); dem = max(0, src.demand)
                                  tgt.SetDemandCap(dem < sup ? dem / sup : 1.0)   // fraction of resource used
        Sums        (0x47BB3F50): s += tgt.supply; d += tgt.demand; hasSums = true
    src.EndOfCycle()                       // folds new* into totals and recalculates active demand
    if hasSums:                            // src becomes the sum of its listed nodes
      if |s - src.supply| > .01: src.AddToSupply(s - src.supply); stable = false
      if |d - src.demand| > .01: src.AddToDemand(d - src.demand); stable = false
until stable
```
Because every change is a delta, a node's totals are the sum of every census, building and link contribution ever
made to it. That is why the census injects *differences* from the previous census, not absolute values.

### 4.5 Tax and economy modifiers (`UpdateTaxAndEconomyModifiers`, `006A6070`) **[confirmed]**
Runs once a month, at the start of the census (DoIdle state 0):
```
totalPop = nodes[0x1001].supply + nodes[0x3001].supply + nodes[0x4001].supply
neutral  = max(0.1, curve_0x4A1F38B5(totalPop))                         // GetNeutralTaxRate (006A3DD0)
for i, key in enumerate([0x21010,0x21020,0x21030,0x23110,0x23120,0x23130,0x23320,0x23330,0x24100,0x24200,0x24300,0x24400]):
    maxRate = HistoryWarehouse.GetMinMaxAverage(0xAABAAA9F + i, today - taxPenaltyDays, today).max
    taxMod  = curve_0x4A1F38B6((maxRate * 0.1) / neutral)                 // history stores tax in tenths of a percent [inferred]
    node(key).SetTaxModifier(OrdinanceSim.GetCumulativeFactor(0x2A650000 | id, 0) - 1 + taxMod)
    econ = Nation.GetEconomyModifier(id)                                  // §4.5.1
    if id in (0x3320, 0x3330): econ += field_D8
    if id is industrial:       econ += field_DC
    node(key).SetEconomyModifier(econ)
```
The tax term uses the **highest** rate in the last `taxPenaltyDays` (default 90). That is why a tax cut takes up to 90
days to help. Ordinances contribute a multiplicative factor through property `0x2A65<id>`, for example
`0x2A651010`. The code adds `(factor − 1)` to the tax modifier.

#### 4.5.1 National economy (`cSC4Nation::GetEconomyModifier` [Mac 0x1E25AE]; tunables `00497B90`) **[confirmed]**
`econ = curve[demandId or default](monthsSinceCityStart) + difficultyMod[city.GetDifficultyLevel()]`.
The tuning exemplar is `0x6534284A/0xE7E2C2DB/0x4A1F6115`: ID list `0x4A1F629F`, curve *i* at property
`0x4A1F62A0 + i` (plain addition, so the 11th–13th curves are `0x4A1F62AA…AC`, **not** the `0x4A1F62B0…B2` the
community wiki lists), and difficulty vector `0x4A1F6300` (3 floats).

### 4.6 Census (DoIdle state machine, `006A7020`) **[confirmed]**
`DoIdle(ms)` is time-sliced with an adaptive batch size (`sTimerHistoryDoIdle`, starting at 1024 units). The batch
doubles or halves depending on how many iterations fit in `ms`. NewDay calls `DoIdle(5)` before the daily work;
SimIdle calls `DoIdle(data3)`.

| State | Work |
|---|---|
| 0 | `UpdateTaxAndEconomyModifiers`; `ZeroCensusInfo(current)`; reset the iterator → 1 |
| 1 | `PerformCensus(x, z)` for each cell until the iterator ends. Then, for each census key whose value changed since the previous census: `delta = cur − prev`. If the node's regionUse == 1, call `AddToSupply(delta)`; if 2, `AddToDemand(delta)`. Set `prev = cur` → 2 |
| 2 | Record history (below) → 3 |
| 3 | Idle. `DoMonthlySimulation` resets 3 → 0, so the census runs once per month |

**PerformCensus(x, z)** (`006A5A00`), per cell that has a lot with a building:
```
lot = LotManager.GetLot(x, z); require lot has building; bt = lot.GetBuildingType(true)
purpose = BDS.GetBuildingProfile(bt).purpose
wealth  = lot.GetOccupantWealth()
devType = BDS.GetDeveloperType(purpose, wealth)                 // e.g. 0x1020
occ     = clamp(BDS.GetDeveloper(devType).GetDesirabilityMap().GetCellValue(x, z) / 100, 0, 1)
cap     = lot.GetCapacity(devType, false)                        // u16
if purpose == 1 (residential):
    eqLvl = clamp(int(ResSim.GetCellEQ(x, z) * (1/50.1) + 1), 1, 4)   // 0.01996 constant in Windows
    census[0x2100 + wealth*0x10 + eqLvl] += ResSim.GetCellWorkforcePercent(x, z) * occ * cap
    census[0x1001 + wealth*0x10]         += occ * cap
elif purpose in {2,3,5,6,7,8}:
    census[devType + 1] += occ * cap
```
The census counts **every cell** of a lot, not one count per lot. Lot capacity is per lot, so a multi-cell lot is
counted once per cell. Either `GetCapacity` returns a per-cell share or this over-counts; **open question**. The
occupancy weight is the growth developer's **desirability** (0..100 %), not a vacancy rate. **[confirmed calls;
interpretation inferred]**

**History recorded in state 2** (HistoryWarehouse IDs): R pop by wealth from census supply `0x1001/11/21/31` →
`0x2A4E2003, 0x4A5E9485, 0xEA5E948A, 0x2A5E948E`. Jobs by type via `GetJobsBySensus` → `0x8A5E950E…0xEA5E9557` (C),
with total C `0x0A4E2056`; `0x2A5E95A1…0xAA5E95B1` (I), with total I `0x4A4E206B`. `nodes[0x2000].demand` →
`0x6A931184`. `max(0, that − Cjobs − Ijobs)` → `0x8A931172` (unemployment [inferred]). Average salary =
`Σ census[0x21wE]·salary[i] / Σ census[0x21wE]` → `0x8A63F30C`, where the 12 keys `0x2111…0x2134` are the table at
Windows `0x00B0D248`.

**GetJobsBySensus(key)** (`006A3E60`): 0 for R keys; otherwise
`(j$ + j$$ + j$$$) × nodes[key].supply`, where `CalculateJobsPerUnitOfDemand` (`006A23A0`) reads the *Drives* list of
the demand exemplar `0x6534284A/0xC7BB4816/key` and takes the percentages for targets `0x2010/0x2020/0x2030` × 0.01.
**[confirmed]**

### 4.7 Building state changes (DoMessage `0xE8BDA2E7`) **[confirmed]**
Message: `IGZUnknown` = lot, data1 = old HabitationState, data2 = new state, data3 = building type (0 → use
`lot.GetBuildingType(true)`), data4 = wealth. A lot counts as *occupied* when its state is 2 or 3 (Windows uses
helper `00695260`). On a change of occupancy, unless the building exemplar has bool `0x27812844` set:
- `AddAgentEffect(bt, occupied, 1.0)` (`006A5920`): for each effect of the building type,
  `node.AddToSupply(±satisfies·pct)` and `node.AddToDemand(±creates·pct)`.
  The effects come from the building exemplar (`AddToAgentEffectMap`, `006A8420`, run for every building type at
  tuning load): `0x27812840` "demand satisfied" (key, amount) pairs, with optional float overrides in `0x27812842`;
  and `0x27812841` "demand created" pairs, with overrides in `0x27812843`.
  `UpdateOccupantEffects(bt, old%, new%)` re-applies them when a percentage (for example funding) changes.
- `UpdateCapacity(bt, wealth, occupied)` (`006A2C80`): sums the building profile's capacity for
  `devType = GetDeveloperType(purpose, wealth)` and calls `nodes[devType].AddToSupply(±capacity)`. So the local
  R$/CS$/… node's **supply = capacity of occupied buildings**.

### 4.8 Cap relief from connections (`EvaluateConnectionUtilization`, monthly, `006A3ED0`) **[confirmed]**
```
oldC = relief_C; oldI = relief_I; relief_C = relief_I = 0
for conn in Neighbors.GetAllConnections() with type in {1,2,3,8,10,12}:
    relief_I += TrafficSim.vt54(conn.vt0C(), 3)   // GetTrafficArrived(...) per SDK: trips arriving via that connection [inferred]
relief_I *= coef_C8                                                  // 0x4A1F38C8 "I cap relief for trips", def 20
n_k = max(1, count for connection class k)   // classes: types {3,12},{8},{2},{10},{1},{6,7,11}; count = distinct neighbour
                                             // cities (+ connections) of that class not already counted [exact count approximate]
relief_C += Σ_k reliefVec[idx_k] * log2(n_k)                         // reliefVec = 0x4A1F38C5, idx = 2,6,0,11,1,7
A = city.GetAirportDeveloper().vt34(); S = city.GetSeaportDeveloper().vt34()     // capacity/usage [inferred]
relief_C += A * coef_C6 (1.0) + S * coef_CA (20.0)
relief_I += A * coef_C9 (0.1) + S * coef_C7 (2.0)
nodes[0x4800].AddToSupply(relief_I - oldI); nodes[0x4900].AddToSupply(relief_I - oldI)
nodes[0x3800].AddToSupply(relief_C - oldC)
```
These resource nodes are presumably referenced by "Caps demand" (`0x47BB3F41`) links in the C/I exemplars. Then
`cap = used/supply` of the resource, and active demand ≤ max·(1 − cap). This is the familiar "C/I demand capped
until you build an airport, seaport or neighbour connection". **[link data inferred]** Note that the seaport
coefficients are crossed relative to their wiki names (`0x4A1F38C7` "Commercial … Seaports" feeds *industrial*
relief, and `0x4A1F38CA` "Industrial … Seaports" feeds *commercial*). This holds in both builds, assuming
`cISC4City` slot 0x130 = Seaport and 0x134 = Airport.

### 4.9 Region (`ResetRegionalDemands`, `ExportDemand`, `Extrapolate*Census`) **[confirmed]**
- **ResetRegionalDemands(initial)** (`006A5CB0`; on Init with `true`, on connection created/deleted with `false`):
  for each node with regionUse 1 or 2, the regional node `id + 0x10000` receives
  `Σ_{connected cities except self} regionalCity.vt44(id)` (regional population/jobs for that ID) minus the previous
  sum. On `initial` it is set absolutely: `sum − (supply + newSupply)` (or the demand equivalent).
- **ExportDemand** (monthly, `006A8FF0`): for each growth developer (BDS `GetDevelopers`), if
  `today − GetDayOfLastSuccessfulBuild(devType) > 65` days and `unmet = src.demand − src.supply ≥ 10`, sort the
  connected cities by `GetTaxRate(devType) × 3` ascending (cheapest first; position and size are fetched but unused).
  Then for each city: `amt = min(1000, city.GetAllowableExtrapolation(devType), unmet)`. If `amt ≥ 10`:
  `city.ExtrapolateGrowth(devType, amt)`; `nodes[devType + 0x10000].AddToSupply(amt)`; `unmet −= amt`;
  `Extrapolate{Residential,Business}Census`. Stop when `unmet < 10`. Totals exported per R/C/I go to history
  `0xAA772F1D / 0x6A774F64 / 0xCA774F69`.
- **ExtrapolateResidentialCensus** (`006A68E0`): ratio `r = city.pop(id+1)/city.pop(id)` (0.8 if 0);
  `workers = r · city.workforcePct · amt`, split by that city's EQ-level shares `id+0x1101…0x1104`. These are
  added as supply to `id+0x11101…` and pushed to the city.
- **ExtrapolateBusinessCensus** (`006A6AF0`, C/I IDs only): `n = r·amt` → supply `id+0x10001`; workforce demand on
  `0x12010/20/30 += n·jobsPerUnit[w]`, mirrored to the neighbour.

### 4.10 Per-message summary (DoMessage `006A9470`) **[confirmed]**

| Message | Action |
|---|---|
| SimBegin `0x66956810` (level 0, once) | `SimulatorBegin`: `SimulationBegin()` on every node, which reposts its active demand |
| NewDay `0x66956814` | `DoIdle(5)`, then `DoDailySimulation`: `DoDemandIndexLinks()`, then record the active demand of the 12 city-total nodes `0x2xxxx` to history `0x2A67A399…` |
| NewMonth `0x66956816` | `DoMonthlySimulation`: `EvaluateConnectionUtilization`, `ExportDemand`, and if census state == 3 → 0 (start a new census) |
| SimIdle `0x6695681A` | `DoIdle(data3 ms)`, continuing the census |
| `0x26C63343` PreSave | `DoDemandIndexLinks()` twice |
| `0xE8BDA2E7` StateChanged | §4.7 |
| `0x8A0D15DA` / `0xEA0D163E` | `ResetRegionalDemands(false)` |
| `0xCA1B151C` | `ReadSimulatorTunableValues` |

All handlers return 1 ("finished"). The census progress is kept in the object's own state, not through the
scheduler's resume mechanism.

## 5. cSC4ResidentialSimulator (population, EQ/HQ, age)

> **Superseded by `docs/specs/residential.md`** (Windows-verified, §5.5–§5.6 and §14). This section is kept as
> background; the corrections listed in residential.md §14 have been applied below.

Registered at levels 0, 10, 6, 4 with flags 2 [Mac Init]. Of those it **handles only NewMonth and NewDay**
(`00700350`). SimBegin and SimIdle are ignored. It also handles `0x99EF1142/0x99EF1143` (building added/removed,
data3 must equal `0x278128A0`) → `DoBuildingChanged(occupant, added)`. That keeps the school/hospital lists.
**[confirmed]**

- NewMonth: `if state == 10: state = 0, overdue = 0 else overdue = 1`. The state is at the object's offset
  `+0x94` on Windows (`+0x90` is the same field relative to the message-target sub-object).
- NewDay: `DoDailySimulation(5)` (`006FF520`), a state machine that does one month of work spread over days.
  Order:
  `0` strike checks (`DoMonthlyStrikeCheck` for schools/hospitals), ordinance effects, sort new school/hospital
  buildings → `0xD/1`, `0xE/2`, `0xF/3`: proximity maps for R$, R$$, R$$$. Each is a flood-fill distance from
  residential buildings of that wealth (BDS building map key `(1,w)`), computed incrementally with
  `ExpandFastOnePass` and copied into grids +0x38/+0x3C/+0x40 (`GetProximityMap`) → `0xB`: zero the age histograms
  → `4`: **DoPopulationChange** over every tract (time-sliced) → `0xC`: **PostPopulationChange** → `5…8`: school and
  hospital coverage maps and grading → `9`: history (global EQ/HQ etc.) → `10`: wait for the month.

### 5.1 DoPopulationChange(tract x, z) (`006FA450`) **[confirmed Mac, Windows matches by constants]**
Grids: `pop[w]` (u16, +0x48…), `EQ[w]` (float, +0x7C…), `HQ[w]` (float, +0x88…), `age` (u8, +0x54). w = $, $$, $$$.
```
p_q = (average of pollution.GetPollutionValue(q, ·) over the tract's cells) mod 256  // SOURCE grids, low byte only
pollHQ = curve_air(p_0) + curve_water(p_1) + curve_garbage(p_2)   // 0x6941B022 / 0xA941B028 / 0xC941B02E
for w:
    EQ[w] = max(0, EQ[w] - eqDecay)                   // eqDecay = 0xC929C433 × ordinance decay modifiers
    HQ[w] = max(0, HQ[w] - (hqDecay + pollHQ))        // hqDecay = 0x0929C439 × ordinance modifiers
    if HQ[w] < deathsDoorHQ (0x6AB70DA7) and pop[w] > 0 and rand(0,100) < chance (0x4AB70DAC):
        spawn effect "buzzards" at tract centre, height 0x6AB71654 ± 0x4AB71659
for w:
    new = growthDeveloper[w].GetPopulationMap()(x, z); old = pop[w]; pop[w] = new; cityPop += new
    if new > old:                                     // newcomers bring the wealth's default EQ/HQ
        k = EQ[w] > 0 ? 2 : 1
        EQ[w] = min(200, (EQ[w]*old + ((EQ[w] + newEQ[w]) / k) * (new-old)) / new)   // newEQ = 0xC929CC58/0x0929CC65/0x2929CC6B (shipped 20/40/60)
        HQ[w] = min(200, same with newHQ[w])                                           // 0xC929CC8D/0x6929CC92/0x4929CC97 (shipped 20/40/60)
    if new == 0: EQ[w] = HQ[w] = 0
age = (Σnew == 0) ? 0 : age + (month == 12 ? 1 : 0)
if anyone moved in: age = (Σ_w (new-old)_w * newAge[w] + Σ_w old_w * age) / Σ_w new_w   // ages 0xC929C652/0x0929CC30/0x0929CC36 (25/30/35)
LE = low byte of GetCellLifeExpectancy(4x, 4z)        // cell coordinates, evaluated after the EQ/HQ updates
if LE < age:                                          // generational reset
    EQ[w] *= 0xCA027325; HQ[w] *= 0xCA027328          // code default 0.7, shipped 80 % → 0.8
    age = 0xCA027307 (40)
accumulate Σ LE·pop (not age·pop), and per age bucket b = (age-20)/3 truncated, any b outside 0..19 → 19
(so ages 0..17 land in the oldest bucket): pop and trunc(EQ)·pop
```
**Residential population is not computed by this class.** It copies the growth developers' population maps.
Those maps are written by the tract and lot developers when buildings grow and fill; see the development
subsystem. `GetPopulation()` returns `Σ pop` (+0xA4). **[confirmed]**

- `GetCellLifeExpectancy = curve_0x6941A67F(GetCellHQ)` (default line (0,0)–(200,200)); `GetCellWorkforcePercent =
  curve_0x4941A92B(LE) / 100` (default (0,0)–(200,100)). Tuning exemplar `0x6534284A/0xE7E2C2DB/0x69189C84`.
  **[confirmed]**
- **PostPopulationChange** (`006F8070`): history `0x2A5E7009` = population-weighted **average life expectancy**
  (Σ LE·pop ÷ pop), not the average age. Then, for 9 age
  groups g, `count_g = Σ_b curve_{0xEA540621+g}(21+3b)/100 · pop_b` → history `0xCA545CAB+g`, and the average EQ of
  that group → `0xCA545CCA+g`. **[confirmed Mac]**

## 6. cSC4CommercialSimulator and cSC4IndustrialSimulator **[confirmed]**
- **Commercial** (`0069EB80`…): it does **no demand math**. It maintains two proximity maps (SimGrid<u8>, IDs
  `0x89D5B788` for CO$$ and `0x89D5B6EF` for CO$$$). Each is a flood-fill distance from existing office buildings
  (BDS building map key `(3,2)` / `(3,3)`), returned by `GetProximityMap(0x3320/0x3330)`. Registered at levels 0, 10,
  4, 6. NewDay → `DoIdle(5)`, SimIdle → `DoIdle(ms)`. The state machine (+0x24 Mac) computes CO$$ then CO$$$, then
  stays idle until a building with purpose 3 (office) is added or removed (messages `0x99EF1142/43`). That sets a
  dirty flag (+0x20 Mac / +0x1C Win) which restarts it. SimBegin and NewMonth are accepted but do nothing.
- **Industrial**: a stub. `Init` sets flags, registers no agents, and `DoMessage` returns 1 without doing anything.
  Industrial demand logic lives entirely in the demand graph and exemplars.

## 7. Tuning properties

| Exemplar (T/G/I) | Property | Meaning | Default in code |
|---|---|---|---|
| Demand sim `0x6534284A/0xE7E2C2DB/0x4A1F3CE6` | `0x4A1F38B5` | curve: neutral tax rate vs (R pop + C jobs + I jobs) | none (curve absent → no curve) |
| | `0x4A1F38B6` | curve: tax modifier vs (max recent rate / neutral) | none |
| | `0x4A1F38B7` | days of tax penalty (look-back window for max tax) | 90 |
| | `0x4A1F38C5` | float[13] commercial cap relief per connection class, × log2(#cities) | 0 |
| | `0x4A1F38C6` | commercial relief per airport unit | 1.0 |
| | `0x4A1F38C7` | **industrial** relief per seaport unit (named "Commercial … Seaports") | 2.0 |
| | `0x4A1F38C8` | industrial relief per freight trip through neighbour connections | 20.0 |
| | `0x4A1F38C9` | industrial relief per airport unit | 0.1 |
| | `0x4A1F38CA` | **commercial** relief per seaport unit (named "Industrial … Seaports") | 20.0 |
| | `0x8A63F2A1` | u32[12] salary per (wealth × EQ level), for average salary history | 100 each |
| Nation `0x6534284A/0xE7E2C2DB/0x4A1F6115` | `0x4A1F629F` / `0x4A1F62A0+i` / `0x4A1F6300` | economy modifier curves vs months since city start / difficulty | — |
| Demand exemplars `0x6534284A/0xC7BB4816/<key>` (ExemplarType 3) | `0x11, 0x21, 0x47BB3F10…60` | §4.3 | — |
| Building exemplars | `0x27812840…43`, `0x27812844` | agent demand effects / exclude flag | — |
| Residential sim `0x6534284A/0xE7E2C2DB/0x69189C84` | `0xC929C433`, `0x0929C439` | EQ / HQ decay per month | 1.0, 1.0 |
| | `0xC929C652, 0x0929CC30, 0x0929CC36` | new-resident age $/$$/$$$ | 25, 30, 35 |
| | `0xC929CC58, 0x0929CC65, 0x2929CC6B` | new-resident EQ (read as float) | 4, 6, 8 |
| | `0xC929CC8D, 0x6929CC92, 0x4929CC97` | new-resident HQ | (not defaulted in ReadTunablePopulationValues) |
| | `0xCA027307 / 0xCA027325 / 0xCA027328` | age reset value / EQ, HQ reset factor | — / 0.7 / 0.7 |
| | `0x6941B022 / 0xA941B028 / 0xC941B02E` | air / water / garbage pollution → HQ decay curves | (0,0)–(32000,100) |
| | `0x6941A67F`, `0x4941A92B` | HQ → life expectancy, LE → workforce % | (0,0)–(200,200), (0,0)–(200,100) |
| | `0x6AB70DA7, 0x4AB70DAC, 0x6AB71654, 0x4AB71659` | death's-door HQ, chance, effect height ± | 0 |
| | `0xEA540621…29` | avg age → age-group share curves (history only) | — |

## 8. Interactions
- **Inputs:** LotManager (lots, capacity, wealth, building type); BuildingDevelopmentSim (building profiles,
  `GetDeveloperType`, growth developers' desirability and population maps, last successful build day);
  ResidentialSim (cell EQ, workforce %); HistoryWarehouse (tax history `0xAABAAA9F…`); OrdinanceSim
  (`GetCumulativeFactor(0x2A65xxxx)`); cSC4Nation (economy curves, difficulty); NeighborsSim and TrafficSim
  (connections, freight trips); City (airport and seaport developers); regional cities (populations, tax rates,
  extrapolation).
- **Outputs:** `kSC4MessageActiveDemandChanged 0x426840A0 (id, −1000..1000)` for the UI. Growth developers read
  their source node (`cISC4GrowthDeveloper::GetSourceDemand`, slot 0x48) to decide how much to grow; ExportDemand
  uses the same accessor. **[confirmed accessor; growth-side consumption to be documented in the development
  subsystem]** Also HistoryWarehouse series (graphs) and regional extrapolation into neighbour cities.

## 9. Open questions
1. The **Demand exemplar data** (group `0xC7BB4816`): the exact Drives/Satisfies/Caps/Sums graph is what produces
   R/C/I behaviour, but the game `.dat` files are not in this repo. Dump them (e.g. with sebamarynissen/sc4) to
   finish the model, in particular which nodes cap on `0x3800/0x4800/0x4900` and how `0x2xxxx` totals are summed.
2. `CalculateActiveDemand(true)`: is the 7% slew really unused? A full scan of Windows indirect calls to slot
   `+0x8C` on cISC4Demand objects outside this subsystem would settle it.
3. Fields +0xD8/+0xDC (Mac; Windows +0xA0/+0xA4): added to the Co/I economy modifier, but no writer other than
   Read/Init was found. They may be always 0 (dead code) or written by another class.
4. Census per cell vs per lot (§4.6): check whether `cSC4Lot::GetCapacity(devType, false)` is per cell.
5. `cISC4RegionalCity` slot `0x44` (regional sum) vs the overloaded `GetPopulation`: confirm the MSVC slot order.
6. The meaning of airport and seaport developer slot `0x34` (capacity vs usage).
7. Exact Windows field offsets of cSC4DemandSimulator beyond those noted. A full ctor read (`006A3xxx`) is pending.
