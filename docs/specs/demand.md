# Spec: the demand subsystem (`cISC4DemandSimulator`, `cISC4Demand`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program. Implementers must work from this spec,
[simulator.md](simulator.md), the public LGPL gzcom-dll SDK headers, the game's data files and general knowledge
only. They must **not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game, its saves and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary,
undefined or crashes. *[inferred]* marks a statement the analysis team could not fully prove; see §15.

Scope: the demand simulator, the demand nodes, the monthly population/jobs census the demand simulator owns, and
the demand-related history it records. The residential, commercial and industrial simulators
(`cSC4ResidentialSimulator` `0x4990C013`, `cSC4CommercialSimulator` `0x8990C080`, `cSC4IndustrialSimulator`
`0xA990C022`) are separate specs; this one lists only the calls it makes into them (§14).

## 1. Role

RCI demand in SimCity 4 is **data-driven**. The demand simulator holds a graph of **demand nodes**. Each node has a
*supply* and a *demand* total. The nodes and the links between them come from **Demand exemplars** shipped with
the game (§4). The simulator's own logic only does five things:

1. **measures** the city: a monthly census of residents (by wealth and education) and of occupied commercial and
   industrial capacity (§6), plus immediate updates when a building becomes occupied or vacant (§7);
2. **injects** each measurement as a *delta* into a node;
3. **propagates** deltas along the exemplar links until nothing changes by more than 0.01 (§4.4);
4. **modifies** the 12 city-total R/C/I nodes once a month with a tax modifier and an economy modifier, and keeps
   capacity "caps" up to date, including cap relief from neighbour connections, airports and seaports (§8);
5. **publishes** each node's *active demand* as a broadcast message (§3.4), records history (§10.4), and exports
   unmet demand to neighbour cities (§9).

The familiar "residents need jobs, jobs need workers" relations are therefore not in the simulator. They are in the
exemplar data (§4.5), which FreeCity4 reads from the player's installed game.

Both classes are GZCOM objects:

| Class | CLSID | Interfaces (reachable through `QueryInterface`) |
|---|---|---|
| demand simulator | `0x2990BFF2` | `cISC4DemandSimulator` (IID `0x47967E16` *[inferred]*), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZMessageTarget2` (`0x452294AA`), a fourth message-target IID `0x452294AB` *[inferred: older message-target interface]*, `cIGZUnknown` |
| demand node | `0x8990C12D` | `cISC4Demand` (IID `0x0797A4E8` *[inferred]*), `cIGZSerializable`, `cIGZUnknown` |

- The city creates one demand simulator per city and returns it from `cISC4City::GetDemandSimulator()`.
- Demand nodes are created only through the GZCOM class factory: `GZCOM()->GetClassObject(0x8990C12D, 0x0797A4E8, &p)`.
  The save file stores nodes by class ID, so FreeCity4's DLL must register a factory for `0x8990C12D` *[compat]*.
- `GetGZCLSID()` returns `0x2990BFF2` for the simulator and `0x8990C12D` for a node *[compat]*.
- The interface method order of both classes must match the SDK headers, because plugins call them through the
  vtable *[compat]*. The SDK has three signature errors, noted in §3.2 and §5.


> **Replaceability note *[compat]*:** the original building-development simulator constructs demand-node objects
> (class `0x8990C12D`) **directly**, not through the GZCOM registry, and hands them to the demand simulator. A
> replacement demand simulator must therefore work with demand nodes it did not create, using only the
> `cISC4Demand` interface (no assumptions about their memory layout). It may still create its own nodes for the
> nodes it owns.

## 2. Demand keys

Every node is identified by a 32-bit **key**:

```
key = demandIndex + demandID        (the original adds; in practice demandIndex is a multiple of 0x10000)
```

| Index | Meaning |
|---|---|
| `0x00000` | **local** node: this city's census, buildings and capacity |
| `0x10000` | **regional** node: neighbour cities' contribution (§9) |
| `0x20000` | **city total** node: local + regional. The 12 R/C/I total nodes are the ones the RCI meter and the tax/economy modifiers use |

`demandID` (the low 16 bits) follows a fixed scheme in the shipped data. The simulator hard-codes the IDs marked **H**;
everything else is only a convention of the exemplar data.

| demandID | Meaning | Hard-coded use |
|---|---|---|
| `0x1010 / 0x1020 / 0x1030` | R$ / R$$ / R$$$: equals the *developer type* returned by `GetDeveloperType(Residence, wealth)` | **H** capacity (§7), tax (§8), export (§9) |
| `0x1011 / 0x1021 / 0x1031` | residents counted by the census, per wealth (= developer type + 1) | **H** census |
| `0x1001` | "R Census" sum node (in the shipped data: sum of `0x1010/20/30`) | **H** neutral tax rate, history |
| `0x2010 / 0x2020 / 0x2030` | jobs by wealth level of the worker | **H** jobs-per-unit lookup (§5) |
| `0x2111 … 0x2134` | census: workers, `0x2100 + 0x10·wealth + eqLevel` (wealth 1–3, EQ level 1–4) | **H** census, salary history |
| `0x2000` | "Jobs" sum node | **H** history |
| `0x3110 / 0x3120 / 0x3130` | CS$ / CS$$ / CS$$$ (commercial services) | **H** |
| `0x3320 / 0x3330` | CO$$ / CO$$$ (commercial office) | **H** |
| `0x4100 / 0x4200 / 0x4300 / 0x4400` | I-R (agriculture) / I-D (dirty) / I-M (manufacturing) / I-HT (high tech) | **H** |
| developer type + 1 (`0x3111 … 0x4401`) | census: occupied C/I capacity per type | **H** census |
| `0x3001 / 0x4001` | "C Census" / "I Census" sum nodes | **H** neutral tax rate |
| `0x3800` | commercial connection-relief resource | **H** cap relief (§8.3) |
| `0x4800`, `0x4900` | industrial connection-relief resource, I-R cap resource | **H** cap relief |

`PurposeType` values (SDK `cISC4BuildingOccupant`): 1 Residence, 2 Services (CS), 3 Office (CO), 4 Tourism,
5 Agriculture, 6 Processing (dirty), 7 Manufacturing, 8 HighTech. Wealth: 1 = $, 2 = $$, 3 = $$$.

## 3. Demand node (`cISC4Demand`, class `0x8990C12D`)

### 3.1 State

All values are 32-bit IEEE floats unless noted.

| State | Initial value | Saved (§12.2) |
|---|---|---|
| loadedFromSave | false (set to true by `Read`) | no |
| initialized | false | no |
| id (uint32) | 0 | yes |
| supply, demand | 0 | yes |
| newSupply, newDemand (pending deltas of the current cycle) | 0 | yes |
| debugLockValue, debugLockFlag | 0, 0 (never changed) | yes |
| activeMax | `+8.507059e37` (bit pattern `0x7E7FFFFF`) | yes |
| activeMin | `−8.507059e37` (bit pattern `0xFE7FFFFF`) | yes |
| activeDemand | 0 | yes |
| flag (byte) | 1, never changed; meaning unknown | yes |
| economyModifier, taxModifier | 0, 0 | yes |
| demandCap (`SC4Percentage`, a float, 1.0 = 100 %) | 0 | yes |
| regionUse (uint32): 0 none, 1 = quantity lives in *supply*, 2 = in *demand* | 0 | **no** (re-read from the exemplar at every load) |

### 3.2 Interface methods (SDK declaration order)

| Method | Behaviour |
|---|---|
| `Init()` | If not yet initialized: set initialized. If **not** loadedFromSave, reset every value in 3.1 to its initial value (including `id := 0`). Acquire the city and the message server; return false if either is unavailable. Return true. |
| `Shutdown()` | If initialized: clear the flag and release the city and message server. Return true. |
| `SimulationBegin()` | If not loadedFromSave, run **CalculateActiveDemand** (3.3), which posts. Then post **ActiveDemandChanged** again with `data2 = trunc(N)`, where N is defined in 3.4 (truncation toward zero here, rounding in 3.3). Return true. |
| `GetId()` / `SetId(id)` | accessor; `SetId` returns true |
| `QuerySupplyValue()` / `QueryDemandValue()` | committed `supply` / `demand` |
| `QueryNewSupply()` / `QueryNewDemand()` | pending `newSupply` / `newDemand` |
| `QueryActiveDemandValue()` | `activeDemand` |
| `QueryEconomyModifier()` | `economyModifier` (same as `GetEconomyModifier`) |
| `QueryActiveDemandMax()` / `QueryActiveDemandMin()` | `activeMax` / `activeMin` |
| `AddToSupplyValue(x)` / `AddToDemandValue(x)` | `newSupply += x` / `newDemand += x`. Nothing else happens until **EndOfCycle**. Return true. |
| `SetSupplyValue(x)` / `SetDemandValue(x)` | overwrite the **committed** `supply` / `demand` (no propagation). Return true. |
| `SetActiveDemandMax(x)` | `activeMax := x`, then if `activeMax < activeMin`, `activeMax := activeMin`. *[choice]* return true. |
| `SetActiveDemandMin(x)` | `activeMin := x`, then if `activeMin > activeMax`, `activeMin := activeMax`. *[choice]* return true. |
| `GetEconomyModifier()` / `SetEconomyModifier(x)` | accessor. Setting does **not** recalculate. *[choice]* setter returns true. |
| `GetTaxModifier()` / `SetTaxModifier(x)` | the same for `taxModifier` |
| `GetDemandCap()` | returns a **pointer** to the node's own cap value (`SC4Percentage*`, as the SDK says) |
| `SetDemandCap(const SC4Percentage&)` | stores the value, unclamped. Does not recalculate. |
| `GetRegionUse()` / `SetRegionUse(v)` | accessor |
| `EndOfCycle()` | `changed := (newSupply ≠ 0 or newDemand ≠ 0)`; `supply += newSupply`; `demand += newDemand`; zero both pending values; run **CalculateActiveDemand(smooth = false)**, which posts; return `changed`. *[compat]* The SDK declares the return type `float`. The original returns a **bool** (in the integer return register). FreeCity4 must return bool through the SDK slot, so that the original callers keep working. |
| `DebugLockValue(x)` / `DebugUnlockValue()` | no effect |
| `DebugIsValueLocked()` | false |

`CalculateActiveDemand(bool smooth)` is not part of the SDK interface. It is an internal operation. *[choice]* It
need not be a virtual function.

### 3.3 Active demand

```
base    := clamp(demand − supply, activeMin, activeMax)
mod     := economyModifier + taxModifier
bonus   := min(activeMax · mod, demand · mod)         -- a fraction of raw demand, bounded by the maximum
target  := clamp(base + bonus, activeMin, activeMax)
target  := max(activeMin, min(target, activeMax − activeMax · demandCap))
if smooth:                                            -- slew limiter, see below
    step := (activeMax − activeMin) · 0.07
    d    := target − activeDemand
    if |d| ≥ step:
        if d > 0: activeDemand := (target > 0 and activeDemand < 0 and activeDemand + step < 0) ? 0 : activeDemand + step
        else:     activeDemand := (target < 0 and activeDemand > 0 and activeDemand − step > 0) ? 0 : activeDemand − step
    else: activeDemand := target
else:
    activeDemand := target
post ActiveDemandChanged (3.4) with data2 = roundHalfEven(N)
```

- `clamp(v, lo, hi)` returns `hi` if `v > hi`, otherwise `lo` if `v < lo`, otherwise `v`.
- With a negative `mod`, `min(·)` picks the larger penalty. With `demandCap = 1.0`, the effective maximum is 0, so the
  node shows no positive demand. A cap of 0.5 halves the maximum.
- **The slew limiter is never used by the original**: every caller found passes `smooth = false` (§15 Q4). FreeCity4
  must implement `smooth = false` behaviour for every caller in this spec. *[choice]* The `smooth` branch may be
  kept for completeness.

### 3.4 ActiveDemandChanged message

Message type **`0x426840A0`** (named ActiveDemandChanged here; original name unknown). It is a
`cIGZMessage2Standard` posted (queued) through the message server, as defined in simulator.md §5.2.

| Field | Value |
|---|---|
| `data1` | the node's key |
| `data2` | normalised active demand N in −1000…+1000: `N = activeDemand / activeMax · 1000` if `activeDemand ≥ 0`, else `N = −(activeDemand / activeMin) · 1000` |

Rounding *[compat]*: `CalculateActiveDemand` rounds N to the nearest integer, ties to even. `SimulationBegin`
truncates toward zero. Nodes without a min/max (the ±8.5e37 defaults) always report 0.

The original posts once **per call** of CalculateActiveDemand, which means once per node on every propagation pass
(§4.4). Messages are queued, so subscribers see the last value last. *[choice]* FreeCity4 may skip posting when
`data2` equals the last value posted for that node, except in `SimulationBegin`, which always posts.

## 4. The demand graph

### 4.1 Demand exemplars (schema)

A demand exemplar is any resource of type `0x6534284A` (exemplar), **in any group**, whose property `0x10`
(ExemplarType) equals **3**. The shipped game has 173 of them, all in group `0xC7BB4816` in `SimCity_1.dat`. Their
instance ID is the node key.

| Property | Name (exemplar UI) | Type | Meaning | Shipped use |
|---|---|---|---|---|
| `0x00000010` | ExemplarType | uint32 | must be 3 | all 173 |
| `0x00000020` | ExemplarName | string | display name only; ignored | all 173 |
| `0x00000011` | — | uint32 | interface ID requested from the new node object; default `0x0797A4E8` | not used |
| `0x00000021` | — | uint32 | node key; default = exemplar instance ID | not used |
| `0x47BB3F10` | Minimum Value | sint32 | `SetActiveDemandMin(float(v))` | 54 |
| `0x47BB3F11` | Maximum Value | sint32 | `SetActiveDemandMax(float(v))`, applied after the minimum | 54 |
| `0x47BB3F20` | Initial Supply | sint32 | `SetSupplyValue(float(v))`, see 4.2 | 9 |
| `0x47BB3F21` | Initial Demand | sint32 | `SetDemandValue(float(v))`, see 4.2 | 8 |
| `0x47BB3F30` | Drives | uint32 pairs (targetKey, percent) | link type **Drives** | 24 |
| `0x47BB3F31` | Satisfies | uint32 pairs (targetKey, percent) | link type **Satisfies** | 32 |
| `0x47BB3F32` | Contributes | uint32 pairs (targetKey, percent) | link type **Contributes** | 23 |
| `0x47BB3F40` | Limits growth | — | **not read** by the demand simulator *[inferred: read by growth developers]* | 0 |
| `0x47BB3F41` | Caps demand | uint32 list (targetKey…) | link type **Caps** | 9 |
| `0x47BB3F50` | Sums | uint32 list (key…) | link type **Sums** | 69 |
| `0x47BB3F60` | Region Use | uint8 | `SetRegionUse(v)`; 0 if absent | 40 |

Link factors: `percent · 0.01` for Drives/Satisfies/Contributes, where `percent` is an unsigned integer that may
exceed 100 (the shipped data uses 0…1500). Caps and Sums carry factor 1.0, which is never used.

### 4.2 Building the graph (part of `Init`, §13)

For each demand exemplar, in the resource manager's enumeration order:

1. `key := property 0x21` if present, else the instance ID.
2. If no node with that key exists, create one through the class factory (§1), call its `Init()`, then
   `SetId(key)`. Add it to the node map. Mark it **new**.
   If a node with that key already exists (it was loaded from the save), call its `Init()`. That keeps its saved
   values, because the node is loadedFromSave.
3. If the node is new, or the simulator was not loaded from a save, apply Initial Supply / Initial Demand.
   *[compat]* A node restored from a save keeps its saved totals. A node that a new exemplar (mod) adds to an old
   save gets its initial values.
4. Always apply the minimum, then the maximum, then region use.
5. Append links, in this order: every Satisfies pair, every Drives pair, every Contributes pair, every Caps key,
   every Sums key (each list in property order). A link records `(sourceKey, targetKey, type, factor)`.

Initial values are written to the **committed** totals, so they do not propagate through links.

If two exemplars resolve to the same key, both are applied: later min/max win and the links accumulate
*[compat]*. Links whose target key has no node make the original crash during propagation. *[choice]* FreeCity4
ignores such links and logs a warning once.

### 4.3 Link semantics

Links belong to their **source** node. "Pending" means `newSupply`/`newDemand` (what changed this cycle);
"committed" means `supply`/`demand`.

| Type | ID | Effect when the source is processed |
|---|---|---|
| **Drives** | `0x47BB3F30` | `v := source.newSupply · factor`; if `|v| > 0.01`, `target.AddToDemandValue(v)` |
| **Satisfies** | `0x47BB3F31` | `v := source.newSupply · factor`; if `|v| > 0.01`, `target.AddToSupplyValue(v)` |
| **Contributes** | `0x47BB3F32` | `v := source.newDemand · factor`; if `|v| > 0.01`, `target.AddToDemandValue(v)` |
| **Caps** | `0x47BB3F41` | `s := max(0.01, source.supply)`, `d := max(0, source.demand)`; `target.SetDemandCap(d < s ? d / s : 1.0)`. The source acts as a *resource*: its demand is usage, its supply is capacity. |
| **Sums** | `0x47BB3F50` | the source becomes the sum of the listed nodes' **committed** values, see 4.4 |

A transfer of `|v| ≤ 0.01` is **dropped**, not deferred *[compat]*.

### 4.4 Propagation (`DoDemandIndexLinks`)

```
repeat:
    stable := true
    for each node N in ascending key order:
        sumS := 0;  sumD := 0;  hasSums := false
        for each link L of N, in creation order (4.2 step 5):
            Drives / Satisfies / Contributes / Caps: as in 4.3; any performed transfer sets stable := false
            Sums: sumS += target.supply;  sumD += target.demand;  hasSums := true
        N.EndOfCycle()                                  -- folds pending into committed, recalculates, posts
        if hasSums:
            if |sumS − N.supply| > 0.01: N.AddToSupplyValue(sumS − N.supply);  stable := false
            if |sumD − N.demand| > 0.01: N.AddToDemandValue(sumD − N.demand);  stable := false
until stable
```

Consequences *[compat]*:
- **Every** node runs EndOfCycle (and posts) on **every** pass.
- A delta sent to a node with a higher key is handled in the same pass. A delta sent to a lower key waits for the
  next pass.
- A Sums correction is pending after the pass, so in the next pass the summing node's own Drives/Satisfies/Contributes
  links forward the *change* of the sum.
- Only transfers and sums corrections keep the loop going. Folding pending values does not.
- Every quantity is cumulative. A node's totals are the sum of every delta it ever received. That is why the census
  injects *differences* (§6.4).

The shipped graph is acyclic, so the loop ends. Mod data could make it cycle forever. *[choice]* FreeCity4 stops
after 1000 passes and logs a warning.

### 4.5 The shipped graph (summary)

The 173 exemplars form three parallel layers (local `0x0xxxx`, regional `0x1xxxx`, total `0x2xxxx`) with the same
ID scheme. Representative examples of how the data produces the familiar behaviour:

- **Residential capacity and demand.** `0x1010` "Residential $" (min/max ±6000, Region Use 1) Satisfies and
  Contributes 100 % into `0x21010` "Total R$" (min/max ±6000, initial demand 20). The local node's supply is the
  capacity of occupied R$ buildings (§7.2), so it becomes supply on the total node. Regional `0x11010` also
  Satisfies `0x21010`.
- **Jobs create residential demand.** The C/I census nodes Drive the jobs nodes (e.g. `0x3121` "CS$$ Census" Drives
  `0x2010` 68 %, `0x2020` 27 %, `0x2030` 5 %, i.e. jobs per unit of occupied capacity by worker wealth). `0x2010`
  "Jobs $" (Region Use 2) Satisfies and Contributes 100 % into `0x22010` "Total Jobs $", which Contributes **200 %**
  into `0x21010` (likewise `0x2020 → 0x22020 → 0x21020` and `0x2030 → 0x22030 → 0x21030`). Every new job for
  workers of wealth w makes two units of residential demand of wealth w.
- **Workers create C/I demand.** `0x22111` "Total WF$EQ1" Sums `0x2111` (local workers) and `0x12111` (regional) and
  Drives `0x23110` (37 %), `0x24100` (1500 %) and `0x24200` (112 %). The 12 Total WF nodes spread each worker group
  over the C/I total nodes according to education.
- **Caps.** `0x1810` "Amenities $" has initial supply 50000 and Caps `0x21010`. `0x1011` "R$ Census" Drives
  `0x1810` 100 %, so R$ demand is choked as R$ population approaches 50000 plus whatever buildings add to that
  supply through agent effects (§7.1). Likewise `0x1820`/`0x1830` (2000/1000), `0x3B20`/`0x3B30` "Co$$/Co$$$ Cap"
  (3000/2000, driven by the CO census, cap `0x23320/0x23330`), and `0x4A00/0x4B00/0x4C00` (14000/7000/4000) capping
  I-D/I-M/I-HT.
- **Cap relief.** `0x3800` "Commercial Connections" Satisfies `0x3B20`/`0x3B30`; `0x4800` "Industrial Connections"
  Satisfies `0x4A00/0x4B00/0x4C00`. The simulator feeds both from connections, airports and seaports (§8.3).
- **Agriculture shrinks with the city.** `0x4900` "I-R Cap" (initial supply 30000, plus industrial relief) Caps
  `0x24100`. Almost every census node Drives `0x4900` at 100 %, so I-R demand falls as the city grows.
- **Sum nodes** (`0x1000`, `0x2000`, `0x3000`, `0x21001`, …) carry no links of their own besides Sums. They exist for
  the UI and history. Note that `0x1001` "R Census" sums the R$ *capacity* nodes `0x1010/20/30`, not the census
  nodes `0x1011/21/31`.
- **Region Use** is set on the 40 local nodes that represent this city's population and jobs: 1 for residents,
  workers, R/C/I capacity and census nodes, and 2 for `0x2010/20/30`.

The full table is in the data files. FreeCity4 must not embed it.

## 5. Demand simulator: interface methods (`cISC4DemandSimulator`, SDK order)

| Method | Behaviour |
|---|---|
| `Init()` / `Shutdown()` | §13 |
| `GetSimulatorType()` | returns the constant `0x279681D7` *[compat]* |
| `GetDemand(demandID, demandIndex)` | If the simulator is not initialized, call `Init()` first *[compat]*. Look up key `demandID + demandIndex`; return the node or null. No reference is added. |
| `UpdateOccupantEffects(buildingType, oldPct, newPct)` | If `oldPct ≠ newPct`: **AddAgentEffect(buildingType, remove, oldPct)**, then **AddAgentEffect(buildingType, add, newPct)** (§7.1). Re-scales a building's demand effects, for example when its funding or effectiveness changes. *[compat]* The SDK declares the first parameter as `SC4Percentage` by value. It is really the `uint32` building type (same size and slot). |
| `CalculateJobsPerUnitOfDemand(float out[3], demandID, demandIndex)` | Set `out[0..2] := 0`. Load the exemplar `0x6534284A / 0xC7BB4816 / (demandID + demandIndex)` from the resource manager (the current data, not the link table). For each Drives pair `(target, percent)`: if `target = demandIndex + 0x2010` then `out[0] := percent / 100`; `+0x2020` → `out[1]`; `+0x2030` → `out[2]`. Missing exemplar or property: all zero. |
| `GetJobsBySensus(type)` | 0 if `type` ∈ {`0x1011`, `0x1021`, `0x1031`}. Otherwise `CalculateJobsPerUnitOfDemand(j, type, 0)`; `x := (j0 + j1 + j2) · node[type].supply`; return `x ≤ 0 ? 0 : trunc(x)` as uint32. *[choice]* A missing node returns 0 (the original crashes). |
| `GetNeutralTaxRate()` | `curveNeutral(node[0x1001].supply + node[0x3001].supply + node[0x4001].supply)` (§11.1), **unclamped**. *[choice]* If the curve is absent or a node is missing, its term is 0 and an absent curve yields 0 (the original crashes). |
| `GetLocalPopulationSummary(map<uint32,int32>&)` | Clear the map. For each node in key order: regionUse 1 → `map[key] := trunc(supply)`; regionUse 2 → `map[key] := trunc(demand)`; others skipped. This is the per-city summary the region view stores for neighbours *[inferred]*. |

"Curve" means a response curve (§11.1).

## 6. Census

### 6.1 State

| State | Initial | Saved |
|---|---|---|
| censusState: 0 start, 1 scanning, 2 record history, 3 done for this month | 3 for a new city, **0** for a city loaded from a save (§13) | no |
| scan cursor over the cell rectangle `(0, 0)…(CellCountX − 1, CellCountZ − 1)` | — | no |
| current census `map<key, float>` | — | no |
| previous census `map<key, float>` | 24 census keys = 0 (new city) | yes |

**Census keys** (24): `0x2111…0x2114, 0x2121…0x2124, 0x2131…0x2134, 0x1011, 0x1021, 0x1031, 0x3111, 0x3121,
0x3131, 0x3321, 0x3331, 0x4101, 0x4201, 0x4301, 0x4401`. "Zeroing" a census map sets these 24 entries to 0. Other
entries are left alone.

### 6.2 Time slicing (`DoIdle(ms)`)

The census is incremental. `DoIdle(ms)` advances it within roughly `ms` milliseconds of wall time:

| State | Work in one `DoIdle` call |
|---|---|
| 0 | Run the tax and economy update (§8.1). Zero the current census. Reset the cursor to `(0, 0)` (empty rectangle → treat as finished). Set state 1 and continue with state 1 in the same call. |
| 1 | Call **PerformCensus(x, z)** for each cell in row order (x increases fastest, then z) until time runs out or every cell is done. When done, **apply the census** (6.4), set state 2 and return. |
| 2 | Record census history (§10.4), set state 3. |
| 3 | nothing |

- `DoIdle(5)` runs at every NewDay and `DoIdle(data3)` at every SimIdle (§10.2). The idle level also runs while the
  game is paused (simulator.md §5.4), so **the census progresses while paused** *[compat]*.
- The original checks the deadline every 1…1024 cells, adapting the interval to the measured speed. *[choice]*
  FreeCity4 may check at any granularity. The result does not depend on the slicing, except through city changes
  that happen between slices.
- Each NewMonth sets state 3 → 0. If a census is still running (state 1 or 2) at month start, it is not restarted
  *[compat]*.

### 6.3 PerformCensus(x, z)

```
lot := LotManager.GetLot(x, z, true)
skip unless lot ≠ null and lot.HasCapacity() and (bt := lot.GetBuildingType(true)) ≠ 0
purpose := BDS.GetBuildingProfile(bt).purpose;  skip unless purpose ≤ 8
wealth  := lot.GetOccupantWealth()
devType := BDS.GetDeveloperType(purpose, wealth)
occ     := clamp(BDS.GetDeveloper(devType).GetDesirabilityMap().GetValue(x, z) / 100, 0, 1)
cap     := lot.GetCapacity(devType, false)                      -- uint16
if purpose = 1 (Residence):
    e     := uint8(trunc(ResidentialSim.GetCellEQ(x, z)))       -- low 8 bits of the integer part
    level := clamp(trunc(e / 50.1 + 1), 1, 4)                   -- e ≤ 50 → 1, 51…100 → 2, 101…150 → 3, ≥ 151 → 4
    census[0x2100 + 0x10·wealth + level] += ResidentialSim.GetCellWorkforcePercent(x, z) · occ · cap
    census[0x1001 + 0x10·wealth]         += occ · cap
elif purpose ∈ {2, 3, 5, 6, 7, 8}:
    census[devType + 1] += occ · cap
-- purpose 4 (Tourism) and others: ignored
```

Notes *[compat]*:
- The weight `occ` is the growth developer's **desirability** at that cell (0–100 %), not a vacancy rate.
- The census visits **every cell** and asks for the lot's capacity at each one. A multi-cell lot therefore
  contributes once per cell it covers, unless `GetCapacity` itself returns a per-cell share (§15 Q3). FreeCity4 must
  reproduce the per-cell visit and use whatever the lot interface returns.
- `GetCellWorkforcePercent` returns a fraction (0…1) despite its name *[inferred from the residential spec notes]*.

### 6.4 Applying the census

For each census key `k` in ascending order, with `cur := current[k]` and `prev := previous[k]`:

```
delta := cur − prev
if delta ≠ 0 and node k exists:
    if node.regionUse = 1: node.AddToSupplyValue(delta)
    if node.regionUse = 2: node.AddToDemandValue(delta)
    previous[k] := cur
```

The deltas stay pending until the next propagation (NewDay or PreSave). *[choice]* FreeCity4 pairs the two maps
by key. The original walks both maps in parallel and assumes they hold the same key set. That holds as long as
`GetDeveloperType` only returns the developer types listed in §2.

## 7. Building occupancy (`kMessageStateChanged`, `0xE8BDA2E7`)

Message fields: `IGZUnknown` = the lot (`cISC4Lot`), `data1` = old `HabitationState`, `data2` = new state, `data3` =
building type (0 = unknown), `data4` = occupant wealth. A lot counts as **occupied** in state 2 (Constructing) or
3 (Occupied).

```
oldOcc := data1 ∈ {2, 3};  newOcc := data2 ∈ {2, 3}
if oldOcc = newOcc: done
bt := data3, or lot.GetBuildingType(true) if data3 = 0;  if bt = 0: done
if the lot's building has bool property 0x27812844 = true: done          -- "no demand effects"
AddAgentEffect(bt, newOcc ? add : remove, 1.0)
UpdateCapacity(bt, data4, newOcc)
```

### 7.1 Agent effects

When the tuning is read (§13, and again on tuning reload), the simulator walks every building type known to the
BDS (`GetAllBuildingTypes`), loads its exemplar (`GetBuildingKeyFromType`) and builds a list of
**effects** `(nodeKey, satisfies, creates)`:

| Property | Type | Meaning |
|---|---|---|
| `0x27812841` | uint32 pairs (key, amount) | demand **created**: `creates := float(amount)` |
| `0x27812843` | float list | optional: overrides the amount of the i-th pair of `0x27812841` |
| `0x27812840` | uint32 pairs (key, amount) | demand **satisfied**: `satisfies := float(amount)` |
| `0x27812842` | float list | optional: overrides the amount of the i-th pair of `0x27812840` |
| `0x27812844` | bool | exclude this building from all demand effects and capacity counting (§7) |

The "created" entries are listed first, then the "satisfied" ones. A pair whose key is not an existing node is
dropped. A non-empty list replaces the building type's previous list; an empty one leaves the previous list in
place *[compat]*. (The building exemplar data is not summarised here; it lives in the players' game files.)

`AddAgentEffect(bt, add, pct)`: for each effect of `bt`, with `sign := add ? +1 : −1`:
`node.AddToSupplyValue(sign · satisfies · pct)` if `satisfies ≠ 0`, and `node.AddToDemandValue(sign · creates · pct)`
if `creates ≠ 0`. A building type without effects does nothing.

### 7.2 UpdateCapacity(bt, wealth, occupied)

```
profile := BDS.GetBuildingProfile(bt);  if profile.purpose = 0: done
devType := BDS.GetDeveloperType(profile.purpose, wealth)
c := Σ over the profile's capacity list (vector of map<developerType, uint16>) of entry[devType] where present
if node devType exists: node.AddToSupplyValue(occupied ? +c : −c)
```

So the supply of a local R$/CS$/… node is the total capacity of occupied (or constructing) buildings of that type.

## 8. Monthly modifiers and cap relief

### 8.1 Tax and economy modifiers (census state 0)

```
neutral := max(0.1, GetNeutralTaxRate())
for i, (key, id) in enumerate([(0x21010,0x1010), (0x21020,0x1020), (0x21030,0x1030),
                               (0x23110,0x3110), (0x23120,0x3120), (0x23130,0x3130),
                               (0x23320,0x3320), (0x23330,0x3330),
                               (0x24100,0x4100), (0x24200,0x4200), (0x24300,0x4300), (0x24400,0x4400)]):
    maxRate := HistoryWarehouse.GetMinMaxAverage(0xAABAAA9F + i, today − taxPenaltyDays, today).max
    taxMod  := curveTax(maxRate · 0.1 / neutral)
    node[key].SetTaxModifier(OrdinanceSim.GetCumulativeFactor(0x2A650000 + id, 0) − 1 + taxMod)
    econ := Nation.GetEconomyModifier(id)
    if id ∈ {0x3320, 0x3330}: econ += coEconomyBonus
    if id ≥ 0x4100:           econ += iEconomyBonus
    node[key].SetEconomyModifier(econ)
```

- `today` is the simulator's current date. The tax history IDs `0xAABAAA9F…0xAABAAAAA` are the tax-rate series,
  one per type, in the order above. They are recorded elsewhere (budget). The tax term uses the **highest** rate in
  the window, so a tax cut takes up to `taxPenaltyDays` (shipped: 90) to help *[compat]*. The `· 0.1` means the
  history stores the rate in tenths of a percent *[inferred]*.
- Ordinances contribute through a multiplicative factor per demand type (`0x2A651010`, …, `0x2A654400`), applied
  as `factor − 1`.
- The new modifiers take effect at the node's next recalculation (next propagation pass).
- `coEconomyBonus` and `iEconomyBonus` are saved fields. Nothing in the original ever sets them to anything but 0
  (§15 Q5). *[compat]* Keep, save and apply them.
- *[choice]* If the history has no data in the window, use `maxRate := neutral · 10` (ratio 1). If the tax curve is
  absent, `taxMod := 0`. If a total node is missing, skip it.

### 8.2 National economy (`cISC4Nation::GetEconomyModifier(demandID)`)

The nation object belongs to the application (`cISC4App::GetNation()`), not to this subsystem. The demand
simulator depends on this contract:

```
months := (simDate.year − startDate.year) · 12 + (simDate.month − startDate.month)    -- startDate = Simulator.GetSimStartDate
curve  := nationCurve[demandID] if present, else nationCurve[0]
econ   := curve(months) + difficultyModifier[City.GetDifficultyLevel()]
```

Tuning: §11.2. With the shipped data the curve is constantly 0, so the economy modifier is 0.06 / 0.03 / 0.01 for
difficulty 0 / 1 / 2.

### 8.3 Connection cap relief (`EvaluateConnectionUtilization`, NewMonth)

State: `commercialRelief`, `industrialRelief` (saved). Coefficients and the 13-entry vector `reliefByNetwork` are
tuning (§11.1).

```
oldC := commercialRelief;  oldI := industrialRelief;  commercialRelief := industrialRelief := 0

-- freight trips through road/rail connections
for each conn in NeighborsSim.GetAllConnections() with conn.GetType() ∈ {Rail 1, Road 2, Highway 3, Avenue 8, DirtRoad 10, GroundHighway 12}:
    industrialRelief += TrafficSim.GetTrafficArrived(conn.AsISCPropertyHolder(), 3)
industrialRelief *= reliefPerTrip

-- connection classes; cities(T) = distinct conn.GetCityOnOtherSideOfConnection() over GetConnectionsOfType(T)
A := cities(Highway) ∪ cities(GroundHighway);   nA := max(1, |A| + count(Highway))
B := cities(Avenue) \ A;                        nB := max(1, |B| + count(Avenue))
C := cities(Road);                              nC := max(1, |C| + count(Road))
D := cities(DirtRoad);                          nD := max(1, |D| + count(DirtRoad))
E := cities(Rail);                              nE := max(1, |E| + count(Rail))
F := cities(LightRail) ∪ cities(Monorail) ∪ cities(Subway);  nF := max(1, |F| + count(LightRail) + count(Subway))
commercialRelief += reliefByNetwork[2]·log2 nA + reliefByNetwork[6]·log2 nB + reliefByNetwork[0]·log2 nC
                  + reliefByNetwork[11]·log2 nD + reliefByNetwork[1]·log2 nE + reliefByNetwork[7]·log2 nF

-- ports
air := City.GetAirportDeveloper().GetNumAirportTrips();  sea := City.GetSeaportDeveloper().GetNumSeaportTrips()
commercialRelief += air · reliefAirC + sea · reliefSeaC
industrialRelief += air · reliefAirI + sea · reliefSeaI

node[0x4800].AddToSupplyValue(industrialRelief − oldI)
node[0x4900].AddToSupplyValue(industrialRelief − oldI)
node[0x3800].AddToSupplyValue(commercialRelief − oldC)
```

- `reliefByNetwork` is indexed by the **network type** (0 Road, 1 Rail, 2 Highway, 3 Street, 4 Pipe, 5 Power,
  6 Avenue, 7 Subway, 8 LightRail, 9 Monorail, 10 OneWayRoad, 11 DirtRoad, 12 GroundHighway), not by the connection
  type. Only six entries are read *[compat]*.
- The class counts reproduce the original exactly, including its quirks *[compat]*. Road and DirtRoad do not
  exclude cities already counted by another class. Each class adds its *connection* count to its *city* count. The
  highway class counts only Highway connections, and the transit class does not count Monorail connections.
- The relief is applied as a delta against last month's value, so these three nodes' supply always contains exactly
  the current relief on top of their initial supply.
- Which port coefficient feeds which side is crossed relative to the exemplar labels: the property named
  "Commercial … Seaports" (`0x4A1F38C7`) feeds **industrial** relief, and "Industrial … Seaports" (`0x4A1F38CA`)
  feeds **commercial** relief. The airport ones are not crossed *[compat]*.
- The meaning of argument 3 to `GetTrafficArrived` (probably the freight trip class) and the identity of the port
  trip methods are inferred from SDK slot order (§15 Q6).

## 9. Region

### 9.1 Regional demands (`ResetRegionalDemands(initial)`)

Runs at `Init` with `initial = true`, and with `initial = false` whenever a neighbour connection is created
(`0x8A0D15DA`) or deleted (`0xEA0D163E`).

```
newList    := NeighborsSim.GetAllConnectedCities(list, false)
exportList := NeighborsSim.GetConnectedCities(list, false)          -- kept for ExportDemand (9.2)
self       := App.GetRegionalCity()                                 -- this city's own region entry, excluded below
for each node N (key order) with N.regionUse ∈ {1, 2}:
    R := node[N.key + 0x10000];  skip if missing
    newSum := Σ_{c ∈ newList,  c ≠ self} c.GetExtrapolatedPopulation(N.key)
    oldSum := Σ_{c ∈ prevList, c ≠ self} c.GetExtrapolatedPopulation(N.key)   -- re-queried now, not remembered
    if regionUse = 1: delta := initial ? newSum − (R.supply + R.newSupply) : newSum − oldSum;  R.AddToSupplyValue(delta)
    if regionUse = 2: delta := initial ? newSum − (R.demand + R.newDemand) : newSum − oldSum;  R.AddToDemandValue(delta)
prevList := newList
```

So at every load the regional nodes are reset to exactly the neighbours' current values. `prevList` and
`exportList` are not saved (both start empty). The regional-city method is the 17th slot of `cISC4RegionalCity`,
named `GetExtrapolatedPopulation(type)` in the SDK *[inferred]* (§15 Q2).

### 9.2 Exporting unmet demand (`ExportDemand`, NewMonth)

```
totals := {R: 0, C: 0, I: 0}
for each growth developer G in BDS.GetDevelopers() (map order):
    devType := G.GetSubType()
    last := BDS.GetDayOfLastSuccessfulBuild(devType);  today := current Julian day
    skip unless last < today and today − last > 65                       -- ≥ 66 days without building
    src := G.GetSourceDemand();  unmet := src.demand − src.supply         -- committed values
    skip unless unmet ≥ 10
    order exportList by 3 · c.GetTaxRate(devType), ascending              -- cheapest neighbour first
    for each city c in that order:
        amt := min(int(c.GetAllowableExtrapolation(devType)), 1000)
        if amt > unmet: amt := trunc(unmet)
        if amt ≥ 10:
            c.ExtrapolateGrowth(devType, amt)
            node[devType + 0x10000].AddToSupplyValue(amt)
            unmet −= amt;  totals[G.GetType()] += amt                       -- GetType: 1 R, 2 C, 3 I
            ExtrapolateResidentialCensus(c, devType, amt)
            ExtrapolateBusinessCensus(c, devType, amt)
        if unmet < 10: stop with this developer
record history 0xAA772F1D := totals.R, 0x6A774F64 := totals.C, 0xCA774F69 := totals.I   (also when 0)
```

*[choice]* Ties in the sort keep the neighbour list order. The original's sort is unstable.

**ExtrapolateResidentialCensus(c, devType, amt)** (only for `0x1010/0x1020/0x1030`):

```
p := c.GetPopulation(devType);  p1 := c.GetPopulation(devType + 1)
r := p ≥ 1 ? p1 / p : 0.8
workers := r · c.GetWorkforcePercentage() · amt
e_k := c.GetPopulation(devType + 0x1100 + k) for k = 1..4;  T := Σ e_k
if T = 0: e_1 := 1, T := 1
for k = 1..4: node[devType + 0x11100 + k].AddToSupplyValue(workers · e_k / T)
for k = 1..4: c.ExtrapolateGrowth(devType + 0x1100 + k, workers · e_k / T)
```

**ExtrapolateBusinessCensus(c, devType, amt)** (only for the nine C/I developer types of §2):

```
r := (c.GetPopulation(devType) ≥ 1) ? c.GetPopulation(devType + 1) / c.GetPopulation(devType) : 0.8
n := r · amt
node[devType + 0x10001].AddToSupplyValue(n);  c.ExtrapolateGrowth(devType + 1, n)
CalculateJobsPerUnitOfDemand(j, devType + 1, 0)
for w = 0..2: node[0x12010 + 0x10·w].AddToDemandValue(n · j[w]);  c.ExtrapolateGrowth(0x2010 + 0x10·w, n · j[w])
```

Missing nodes make the original crash. *[choice]* FreeCity4 skips those calls.

## 10. Scheduling and messages

### 10.1 Agent registration

In `Init`: `AddAgent(self, L, "cSC4DemandSimulator", flags = 2)` for L = 0 (SimBegin), 4 (Day), 6 (Month) and
10 (Idle). Flag 2 means "enabled only when the city is established" (simulator.md §4). Consequences *[compat]*:
in an unestablished city no day, month or idle work happens (no census, no propagation), and the level-0
registration counts as finished without running and is removed, so `SimulatorBegin` does not run.

### 10.2 Per-message behaviour

All handlers return **true** ("finished"). Long work (the census) keeps its own state and does not use the
scheduler's resume mechanism.

| Message | Action |
|---|---|
| SimBegin `0x66956810` | **SimulatorBegin**: `SimulationBegin()` on every node in key order |
| NewDay `0x66956814` | `DoIdle(5)` (§6.2), then **daily**: `DoDemandIndexLinks()` (§4.4), then record the 12 active-demand series (§10.4) |
| NewMonth `0x66956816` | `EvaluateConnectionUtilization()` (§8.3), `ExportDemand()` (§9.2), then if censusState = 3, set it to 0 |
| SimIdle `0x6695681A` | `DoIdle(data3)` |
| `0x26C63343` kSC4MessagePreSave | `DoDemandIndexLinks()` **twice** (so the save holds folded totals) |
| `0xE8BDA2E7` kMessageStateChanged | §7 |
| `0x8A0D15DA` / `0xEA0D163E` | neighbour connection created / deleted: `ResetRegionalDemands(false)` |
| `0xCA1B151C` kSC4MessageTypeIngredChangesMade (tuning reload) | re-read tuning and agent effects (§11, §7.1) |
| anything else | ignored |

On the first day of a month the scheduler delivers NewDay before NewMonth (simulator.md §5.3). So that day's
propagation runs before the month's relief, export and census restart.

### 10.3 Messages posted

The only message this subsystem posts is **ActiveDemandChanged `0x426840A0`** (§3.4), from nodes. Neighbour
effects are direct interface calls (§9), not messages.

### 10.4 History

Values go through `cISC4HistoryWarehouse::RecordHistoryData(id, int32)`. "trunc" means conversion toward zero.

| When | History ID | Value |
|---|---|---|
| `Init` | the 12 IDs below | `SetHistoryDataQueueLength(id, 1)` |
| every NewDay | `0x2A67A399, 0xAA67A3A0, 0x8A67A3A4` | trunc(active demand) of `0x21010, 0x21020, 0x21030` |
| | `0x0A67A3A9, 0xAA67A3AD, 0xCA67A3B2` | … of `0x23110, 0x23120, 0x23130` |
| | `0xAA67A3B7, 0x2A67A3BC` | … of `0x23320, 0x23330` |
| | `0xEA67A3BF, 0x0A67A3C4, 0xEA67A3C8, 0x8A67A3CC` | … of `0x24100, 0x24200, 0x24300, 0x24400` |
| census state 2 | `0x2A4E2003` | trunc(`node[0x1001].supply`) (residential total) |
| | `0x4A5E9485, 0xEA5E948A, 0x2A5E948E` | trunc(supply) of `0x1011, 0x1021, 0x1031` |
| | `0x8A5E950E, 0x0A5E9515, 0x8A5E954A, 0x8A5E9552, 0xEA5E9557` | `GetJobsBySensus` of `0x3111, 0x3121, 0x3131, 0x3321, 0x3331` |
| | `0x0A4E2056` | sum of those five (C jobs, uint32) |
| | `0x2A5E95A1, 0x6A5E95A7, 0x0A5E95AC, 0xAA5E95B1` | `GetJobsBySensus` of `0x4101, 0x4201, 0x4301, 0x4401` |
| | `0x4A4E206B` | sum of those four (I jobs) |
| | `0x6A931184` | trunc(`node[0x2000].demand`) |
| | `0x8A931172` | trunc(max(0, `node[0x2000].demand − Cjobs − Ijobs`)), computed in float |
| | `0x8A63F30C` | average salary: with `n_i := trunc(node[k_i].supply)` for the 12 keys `k = 0x2111, 0x2112, 0x2113, 0x2114, 0x2121, …, 0x2134` (in that order) and `salary[i]` (§11.1): `Σ n_i·salary[i] / Σ n_i` with 64-bit integer arithmetic and truncating division; 0 if `Σ n_i = 0` |
| every NewMonth | `0xAA772F1D, 0x6A774F64, 0xCA774F69` | exported R / C / I totals (§9.2) |

## 11. Tuning

### 11.1 Demand Simulator exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F3CE6`

Read at `Init` and on tuning reload. If the exemplar is missing, the original keeps whatever values it had.
*[choice]* FreeCity4 uses the defaults below.

| Property | Name | Type | Default | Shipped value |
|---|---|---|---|---|
| `0x4A1F38B5` | Neutral Tax Rate vs. Population | float pairs (curve) | none | 41 points from (0, 9.0) falling to (6 000 000, 4.0), e.g. (1.5e6, 8.3), (3e6, 7.5), (4.5e6, 6.5), (5.4e6, 5.6) |
| `0x4A1F38B6` | Tax Modifier vs. Rate Variance | float pairs (curve) | none | (0.2, 0.3), (1, 0), (5, −3.5) |
| `0x4A1F38B7` | Days of Tax Penalty (`taxPenaltyDays`) | uint32 | 90 | 90 |
| `0x4A1F38C5` | Commercial Cap Relief for Connections (`reliefByNetwork`) | float[13] | 13 × 0 | 10000, 10000, 100000, 0, 0, 0, 20000, 20000, 20000, 20000, 0, 1000, 100000 |
| `0x4A1F38C6` | Commercial Cap Relief for Airports (`reliefAirC`) | float | 1.0 | 1 |
| `0x4A1F38C7` | "Commercial" Cap Relief for Seaports → **`reliefSeaI`** | float | 2.0 | 2 |
| `0x4A1F38C8` | Industrial Cap Relief for Trips (`reliefPerTrip`) | float | 20.0 | 20 |
| `0x4A1F38C9` | Industrial Cap Relief for Airports (`reliefAirI`) | float | 0.1 | 0.1 |
| `0x4A1F38CA` | "Industrial" Cap Relief for Seaports → **`reliefSeaC`** | float | 20.0 | 20 |
| `0x8A63F2A1` | Average Salary for Demographic (`salary[12]`) | uint32[12] | 12 × 100 | 17000, 19250, 21500, 26000, 25000, 31250, 37500, 50000, 45000, 108750, 172500, 300000 |

- A **response curve** is a list of `(x, y)` float pairs. The output is linear interpolation between neighbouring
  points, and the first/last `y` outside the x range. A point whose x does not increase over the previous point is
  dropped (unless its y also equals the previous y). An absent property removes the curve.
- The original copies exactly 13 / 12 entries even if the property is shorter (reading past its end). *[choice]*
  Missing entries take the default.

### 11.2 Nation exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F6115` (read by the nation, §8.2)

| Property | Type | Meaning | Shipped |
|---|---|---|---|
| `0x4A1F629F` | uint32 list | demand IDs that have their own curve; ID 0 = default | 0 |
| `0x4A1F62A0 + i` | float pairs (curve) | economy modifier vs. months since city start, for the i-th listed ID (plain addition: the 11th–13th curves are `0x4A1F62AA…AC`) | `0x4A1F62A0` = (0, 0) |
| `0x4A1F6300` | float[3] | difficulty modifier, indexed by `cISC4City::GetDifficultyLevel()` | 0.06, 0.03, 0.01 |

### 11.3 Other data

Demand exemplars: §4.1. Building exemplar properties: §7.1.

## 12. Persistence (`cIGZSerializable`)

Records are little-endian, written in the order shown *[compat]*: existing saves must load, and saves we write
must load in the original game.

### 12.1 Demand simulator (`GetGZCLSID() = 0x2990BFF2`)

```
uint16   version = 2
[version < 2 only: 4 × uint32, read and ignored]
float32  coEconomyBonus
float32  iEconomyBonus
float32  commercialRelief
float32  industrialRelief
uint32   nodeCount
nodeCount × { uint32 key;  <embedded serializable object: the cSC4Demand node> }     ascending key
uint32   censusCount
censusCount × { uint32 key;  float32 value }                                         previous census, ascending key
```

- `Write` returns false without writing if the stream is already in error. Otherwise it returns "no stream error".
- `Read` fails for a stream error or `version ≥ 3`. Otherwise it sets **loadedFromSave** (before reading the rest),
  creates each node from the embedded object (through the class factory of the class ID the object carries
  *[inferred]*, requesting interface `0x0797A4E8`), stores it under `key`, and fills the previous census. Returns
  "no stream error".
- The embedded object goes through the save segment's serializable-object facility (the same one other savable
  objects use). Its exact byte encoding belongs to the save-file spec (§15 Q1).
- Not saved: census state (derived in `Init`), the current census, links, curves, tuning, agent effects, city lists.

### 12.2 Demand node (`GetGZCLSID() = 0x8990C12D`)

```
uint16   version = 2
uint32   id
float32  supply
float32  demand
float32  newSupply
float32  newDemand
float32  debugLockValue          (0)
float32  debugLockFlag           (0)
float32  activeMin               ← note: min before max
float32  activeMax
float32  activeDemand
uint8    flag                    (1)
float32  economyModifier
float32  taxModifier
version 2: float32 demandCap      version 1: uint8 percent, demandCap := percent · 0.01
```

- `Read` sets loadedFromSave first, then fails (returns false) for `version ≥ 3`. It returns "no stream error".
- `regionUse` is not saved. It comes from the exemplar (4.2).

## 13. Lifecycle

`Init()` (does nothing and returns true if already initialized):

1. Build the graph from the demand exemplars (§4.2).
2. Acquire the city. Create the scan cursor over the whole city (`CellCountX` × `CellCountZ`).
3. `censusState := loadedFromSave ? 0 : 3`. A loaded city starts a census right away. A new city waits for the
   first NewMonth *[compat]*. If not loaded: set the two economy bonuses and the two relief values to 0, and zero
   the previous census.
4. Acquire the lot manager, residential simulator, BDS, neighbours simulator and ordinance simulator.
5. Read tuning (§11.1) and agent effects (§7.1).
6. `ResetRegionalDemands(true)` (§9.1).
7. Set the queue length of the 12 active-demand history series to 1 (§10.4).
8. Subscribe to `0xE8BDA2E7`, `0x8A0D15DA`, `0xEA0D163E`, `0x26C63343`, `0xCA1B151C`.
9. Register the four agents (§10.1).
10. Set initialized, then run `DoDemandIndexLinks()` once.
11. Return true.

`Shutdown()` (does nothing if not initialized; returns true):
clear initialized; `RemoveAgent(self)` on the city's simulator; unsubscribe the five messages; release the
services; `Shutdown()` and release every node and empty the node map; drop the scan cursor, the current and
previous census and both curves. The original does not clear the link table. *[choice]* FreeCity4 clears it, so
that a later `Init` does not duplicate links.

Some state is per process in the original (the agent-effect table survives between cities). *[choice]* FreeCity4
rebuilds it at each `Init`.

## 14. Interactions (exact calls)

| Dependency | Calls |
|---|---|
| `cISC4City` | `GetSimulator()`, `CellCountX()`, `CellCountZ()`, `GetAirportDeveloper()`, `GetSeaportDeveloper()`, `GetDifficultyLevel()` (via the nation) |
| `cISC4Simulator` | `AddAgent`, `RemoveAgent(target)`, `GetSimDate()`, `GetSimStartDate()` (nation) |
| `cISC4LotManager` | `GetLot(x, z, true)` |
| `cISC4Lot` | `HasCapacity()`, `GetBuildingType(true)`, `GetOccupantWealth()`, `GetCapacity(devType, false)`, `GetBuilding()` (then the occupant's property holder for `0x27812844`) |
| `cISC4BuildingDevelopmentSimulator` | `GetAllBuildingTypes(array, count)`, `GetBuildingKeyFromType`, `GetBuildingProfile`, `GetDeveloperType`, `GetDeveloper(devType)`, `GetDevelopers()`, `GetDayOfLastSuccessfulBuild` |
| `cISC4GrowthDeveloper` | `GetDesirabilityMap()->GetValue(x, z)` (int16), `GetSubType()`, `GetType()`, `GetSourceDemand()` |
| `cISC4ResidentialSimulator` | `GetCellEQ(x, z)`, `GetCellWorkforcePercent(x, z)` |
| `cISC4OrdinanceSimulator` | `GetCumulativeFactor(0x2A65xxxx, 0)` |
| `cISC4HistoryWarehouse` | `GetMinMaxAverage`, `RecordHistoryData`, `SetHistoryDataQueueLength` |
| `cISC4App` | `GetNation()`, `GetRegionalCity()` |
| `cISC4Nation` | `GetEconomyModifier(demandID)` |
| `cISC4NeighborsSimulator` | `GetAllConnections`, `GetConnectionsOfType`, `GetAllConnectedCities(list, false)`, `GetConnectedCities(list, false)` |
| `cISC4NeighborConnection` | `GetType()`, `AsISCPropertyHolder()`, `GetCityOnOtherSideOfConnection()` |
| `cISC4TrafficSimulator` | `GetTrafficArrived(holder, 3)` |
| `cISC4AirportDeveloper` / `cISC4SeaportDeveloper` | `GetNumAirportTrips()` / `GetNumSeaportTrips()` *[inferred]* |
| `cISC4RegionalCity` (neighbours) | `GetTaxRate`, `GetPopulation(type)`, `GetExtrapolatedPopulation(type)` *[inferred]*, `GetAllowableExtrapolation`, `ExtrapolateGrowth`, `GetWorkforcePercentage` |
| resource manager | enumerate exemplars of type `0x6534284A`; load single exemplars by TGI |

Known readers of this subsystem (for orientation): the RCI meter UI reads the total nodes. Growth developers read
their source node through `GetSourceDemand` (the development spec). The BDS reads `0x1001`/`0x3001` supply
monthly. The traffic simulator reads regional jobs (`GetDemand(0x2010, 0x10000)`). Lots use
`CalculateJobsPerUnitOfDemand`. Lot developers and port developers call `UpdateOccupantEffects`.

Arithmetic *[choice]*: 32-bit float throughout. The original runs the x87 at 24-bit precision (foundations §18), so
each operation already rounds like a `float32` operation; differences remain only where an operand is a `double`
constant or an integer above 2^24. Tests compare floats with a relative tolerance of 1e-5, except where this spec
defines integer rounding.

## 15. Open questions

1. **Embedded node encoding in the save** (§12.1): the byte format the save segment uses for an embedded
   serializable object (prior art suggests a class-ID/instance reference with the node stored as its own record of
   type `0x8990C12D`). It must be pinned down from a sample save before saves can be written.
2. **Regional sum method** (§9.1): slot 17 of `cISC4RegionalCity` is `GetExtrapolatedPopulation(type)` if the SDK
   order is right. The extrapolation code uses slot 16 (`GetPopulation(type)`), so the two are distinct. Confirm
   with a region test that regional nodes equal the neighbours' extrapolated values, not their real population.
3. **Census per cell vs per lot** (§6.3): does `cISC4Lot::GetCapacity(devType, false)` return the whole lot's
   capacity (then multi-cell lots are over-counted by their cell count) or a per-cell share?
4. **Slew limiter** (§3.3): no caller with `smooth = true` was found in either build. A full scan of indirect
   calls would settle it.
5. **CO/I economy bonuses** (§8.1): saved and applied, but never written except by load and zero-init. They may be
   dead, or written by an unidentified caller.
6. **Relief inputs** (§8.3): the meaning of argument 3 to `GetTrafficArrived`, and whether the port methods (slot
   13 of the port developers) are really `GetNum*Trips` rather than capacity. The class-count quirks (Road/DirtRoad
   not excluding cities, connection counts added to city counts) were read in both builds but should be confirmed
   by a test against the original.
7. **Tax history units** (§8.1): the `· 0.1` assumes the tax history stores tenths of a percent. Also, the
   original's behaviour when `GetMinMaxAverage` finds no data (it uses an uninitialised value).
8. **Names**: the original names of messages `0x426840A0`, `0x8A0D15DA`, `0xEA0D163E`; the identity of IID
   `0x452294AB`; confirmation that `0x47967E16` / `0x0797A4E8` are the `cISC4DemandSimulator` / `cISC4Demand` IIDs.
9. **Node flag byte** (§3.1): always 1, saved, never read by any code found.
10. **Unusual developer types**: what `GetDeveloperType` returns for unusual purpose/wealth pairs (e.g. Office, $).
    If it returns a type outside §2, the original's census application misaligns its two maps (§6.4).

## 16. Conformance tests (must exist in `engine/tests`)

1. **Node arithmetic.** `AddToSupplyValue`/`AddToDemandValue` change only pending values. `EndOfCycle` folds them,
   returns true iff a pending value was non-zero, and posts one `0x426840A0` with `data1 = key`.
2. **Active demand formula.** With min/max ±6000: demand 1000, supply 400 → 600 → `data2 = 100`; add
   `economyModifier = 0.03` → bonus `min(180, 30)` = 30 → 630 → `data2 = 105`; cap 0.95 → capped at
   `6000 − 5700 = 300`; negative values normalise by `activeMin`; ties round to even (`N = 0.5 → 0`, `1.5 → 2`).
   `SimulationBegin` truncates.
3. **Min/max ordering.** `SetActiveDemandMin` above max clamps to max, and vice versa.
4. **Propagation.** A small graph with one link of each type: Drives/Satisfies/Contributes move `pending · factor`,
   drop transfers `≤ 0.01`, Caps sets `min(1, demand / max(0.01, supply))`, Sums converges to the sum of committed
   values. Lower-key targets settle on a later pass. Every node posts once per pass (or, with the coalescing
   choice, at least once with its final value).
5. **Shipped graph smoke test** (needs the player's data files; skipped if absent): build all 173 nodes. Inject
   +1000 supply on `0x1011` and +1000 on `0x3121`, propagate, and check `0x1810.demand = 1000`,
   `0x2010.demand = 680`, `0x2020.demand = 270`, `0x2000.demand = 1000`, `0x4900.demand = 2000`,
   `0x21010.demand = 20 + 2·680 = 1380`, `0x21020.demand = 540`, and the cap on `0x21010 = 1000 / 50000`.
6. **Census.** A fake city (lot manager, BDS, developers, residential sim) with a 2×1 residential lot: per-cell
   contributions, the EQ level buckets (50 → 1, 51 → 2, 150 → 3, 151 → 4), purpose 4 ignored, delta application by
   regionUse, time slicing across several `DoIdle` calls giving the same result as one call, state 3 → 0 only at
   NewMonth.
7. **Occupancy messages.** State 1→3 adds capacity and agent effects, 3→2 does nothing, 3→4 removes them,
   `0x27812844` suppresses both, and `UpdateOccupantEffects(bt, 1.0, 0.5)` nets −50 %.
8. **Tax and economy.** With a stub history (max rate 90), neutral 9.0, shipped tax curve and ordinance factor 1.0:
   tax modifier 0. With max rate 180: modifier `curve(2.0) = −0.875`. Economy modifier = nation value + bonus, for
   the right nodes only.
9. **Cap relief.** One highway connection to one city → `commercialRelief = 100000` with shipped data; month-to-month
   deltas leave `0x3800.supply` equal to the current relief; the seaport coefficient crossing.
10. **Region.** `ResetRegionalDemands(true)` sets regional nodes absolutely; `false` applies only differences.
    `ExportDemand` respects the 66-day, ≥ 10, 1000-per-city limits and the cheapest-first order, and records the
    three monthly totals.
11. **History.** The daily 12 series and the census-state-2 series, with the documented IDs and truncation; the
    average salary integer division.
12. **Save round-trip.** Write → read → write gives identical bytes for both record types. Version 1 node records
    (byte cap) load. Version ≥ 3 is rejected. A loaded simulator starts its census immediately, and loaded nodes
    keep their totals when the exemplars are re-applied.

## Compiled slot tables (round 2) *[compat]* (see foundations §16, §17)

Verified slot by slot against the original Windows vtables (each slot's `ret` operand = stack-argument bytes,
compared with the SDK header compiled under the MSVC overload rule):

| Interface | Original vtable | Interface slots | Result |
|---|---|---|---|
| `cISC4DemandSimulator` | `0x00AB0E48` | 12 (3 + 9) | no overloads; order and argument sizes match the SDK |
| `cISC4Demand` (demand node) | `0x00AA2758` | 34 (3 + 31) | no overloads; order and argument sizes match the SDK |

The slot after the interface is the class's own destructor in both cases. A FreeCity4 implementation may take
the SDK declaration order as is for these two interfaces.
