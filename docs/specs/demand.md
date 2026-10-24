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
| `AddToSupplyValue(x)` / `AddToDemandValue(x)` | `newSupply += x` / `newDemand += x` (one float add). Nothing else happens until **EndOfCycle**. Return true **as a bool in AL; nothing is left on the x87 stack** (slots 16/17, pop 4; corrected 2026-10-14). |
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
exceed 100 (the shipped data uses 0…1500). *[compat]* The integer is loaded with `FILD`, which is exact. When its
sign bit is set, the original then adds 2^32 (the float 4294967296.0 at `0x00A80AA8`). It multiplies by
the **float32** constant `0.01f` (`0x00A823A4`, bytes `0A D7 23 3C` = `0x3C23D70A`) and stores a float32. On the
main thread every `FADD` and `FMUL` rounds to 24 bits (foundations §18), so (QUESTIONS DQ12):

| `percent` | Factor | Roundings |
|---|---|---|
| below 2^31 | `r24(percent × 0.01f)` | one: the exact integer times the constant |
| 2^31 and above | `r24(r24(percent) × 0.01f)` | two: the `FADD` of 2^32 rounds the integer to 24 bits first |

Below 2^24 the integer is exact as a float, so the factor equals `f32(percent) × 0.01f` and the shipped data
cannot tell the forms apart. Above 2^24 they can differ in the last bit:
- 16777217 (2^24 + 1, first row) gives 167772.171875 (`0x4823D70B`); rounding the integer to float first would give
  167772.15625 (`0x4823D70A`);
- 3242996796 (second row) gives 32429966.0 (`0x4BF76BC7`); a single rounding would give 32429968.0 (`0x4BF76BC8`).

The three sites in `CreateDemandFromExemplar` are `0x006A5514`, `0x006A5604` and `0x006A56F4`. Slot 8
`CalculateJobsPerUnitOfDemand` (§5) uses the same instructions. Caps and Sums carry factor 1.0, which is never used.

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
| `CalculateJobsPerUnitOfDemand(float jobs[4], demandID, demandIndex)` | **Returns nothing, and leaves the x87 stack as it found it** (corrected 2026-10-14; slot 8, `+0x20`, pops 12). `jobs` points to **four** floats (the lot's `GetJobs` record): the three outputs are `jobs[1]`, `jobs[2]`, `jobs[3]` (byte offsets `+4`, `+8`, `+0xC`); **`jobs[0]` is never touched**. Set `jobs[1..3] := 0`. Load the exemplar `0x6534284A / 0xC7BB4816 / (demandID + demandIndex)` from the resource manager (the current data, not the link table). For each Drives pair `(target, percent)`: if `target = demandIndex + 0x2010` then `jobs[1] := factor(percent)`; `+0x2020` → `jobs[2]`; `+0x2030` → `jobs[3]`, where `factor` is the link factor of §4.1 (the same instructions: exact `FILD`, `+2^32` when the sign bit is set, one `FMUL` by the float constant 0.01f, `0x3C23D70A`, 0.0099999998). `percent` is read as an **unsigned** 32-bit integer, and the scale is a multiply, **not** a division by 100; the last bit differs, e.g. 33 gives 0.32999998 (`0x3EA8F5C2`), not 0.33000001. Missing exemplar or property: all zero. |
| `GetJobsBySensus(type)` | 0 (EAX) if `type` ∈ {`0x1011`, `0x1021`, `0x1031`}. Otherwise slot 8 `CalculateJobsPerUnitOfDemand(j, type, 0)` into a local four-float record; `s := f32(f32(j[3] + j[2]) + j[1])`; `x := f32(node[type].supply × s)`; return the low 32 bits of `x` truncated toward zero to 64 bits (`_ftol`), with **no clamp** (corrected 2026-10-14; arithmetic below). `j[0]` is neither written nor read. *[choice]* A missing node returns 0 (the original inserts a null map entry and crashes). |
| `GetNeutralTaxRate()` | `curveNeutral(s)` (§11.1), **unclamped**, with `s := f32(f32(I + C) + R)`, where `I`, `C`, `R` are `node[0x4001]`, `node[0x3001]`, `node[0x1001]` `.supply`. The sum is evaluated **I + C first, then + R**, each addition rounded to `float32` (below). *[choice]* If the curve is absent or a node is missing, its term is 0 and an absent curve yields 0 (the original crashes). |
| `GetLocalPopulationSummary(map<uint32,int32>&)` | Clear the map. For each node in key order: regionUse 1 → `map[key] := trunc(supply)`; regionUse 2 → `map[key] := trunc(demand)`; others skipped. This is the per-city summary the region view stores for neighbours *[inferred]*. The game's map layout, the original's insert and a worked example: [docs/abi/allocators.md](../abi/allocators.md) *map and set*, *GetLocalPopulationSummary*. |

"Curve" means a response curve (§11.1).

**`GetNeutralTaxRate` arithmetic** *[compat]* (`0x006A3DD0`). The original looks up the three nodes in the order
`0x1001`, `0x3001`, `0x4001`, and stores the first two supplies to `float32` locals. `QuerySupplyValue`
(`0x005D4C40`) loads the `float32` member, so each value is an exact `float32`. The third supply stays in ST0. The
original then adds the `0x3001` local and then the `0x1001` local (`FADD dword`, `0x006A3E39`, `0x006A3E3D`), and
stores the sum to `float32`. On the main thread's 24-bit x87 precision (foundations §18), each `FADD` rounds to a
24-bit significand. So the sum is two `float32` additions, in the order **(I + C) + R**. It is neither R + C + I
nor one rounding of the exact sum. The curve then evaluates as three `float32` operations (foundations §4.3,
`0x005F4250`) and returns in ST0. The caller stores it to `float32` (§8.1).

Worked example (run 73, the demand shadow's one mismatch): R = 27207 (`0x46D48E00`), C = 2569.47998
(`0x452097AE`), I = 10583.1006 (`0x46255C67`).

| Step | Exact | `float32` result |
|---|---|---|
| I + C | 13152.58056640625, an exact tie between two `float32`s | 13152.580078125 (`0x464D8252`), to even |
| + R | 40359.580078125, again an exact tie | **40359.578125** (`0x471DA794`), to even |
| curve segment 0: (0, 9.0) to (150000, 8.9f), slope `0xB532F529` (−6.6666922e−7) | `(s − 0) · slope` = `0xBCDC6AFF`, then `9.0 +` | **8.97309399** (`0x410F91CB`) |

Summing R + C + I instead gives 29776.48046875, then 40359.58203125 (`0x471DA795`), and the result is 8.97309303
(`0x410F91CA`), 1 ulp lower. One rounding of the exact sum (40359.58056640625, as an extended-precision x87 would
give) rounds to the same `0x471DA795`. Only the original's order with two `float32` roundings gives
`0x410F91CB`. This also confirms that the main thread was at 24-bit precision in that run.

**`GetJobsBySensus` arithmetic** *[compat]* (QUESTIONS DQ10, DQ11; `0x006A3E60`). The original passes slot 8 the
address of a 16-byte local, so slot 8 writes the local's `j[1]`, `j[2]`, `j[3]`. `j[0]` keeps whatever was on the
stack, and nothing reads it: the old "j0 + j1 + j2" came from the slot-8 ABI before its correction. The sum is
`FLD j[3]`, `FADD j[2]`, `FADD j[1]` (`0x006A3E8C`–`0x006A3E9C`), each `FADD` rounded to 24 bits on the main
thread (foundations §18). The sum is stored to a `float32` local, so it is **(j[3] + j[2]) + j[1]**, not
(j[1] + j[2]) + j[3]. The node's committed supply (`QuerySupplyValue`, node slot 8, returned in ST0) is then
multiplied by that local (one `FMUL`, one 24-bit rounding). The product goes straight to `_ftol` (`0x009EEF04`), and
the function returns. So:
- there is **no `x ≤ 0` test**. A negative product returns its truncation as a negative `int32` (−3.7 → −3,
  `0xFFFFFFFD`);
- NaN, ±∞ and |x| ≥ 2^63 return 0, and 2^31 ≤ |x| < 2^63 wraps (the `_ftol` table under `GetLocalPopulationSummary`
  below);
- the value is in EAX. EDX holds the high half of `_ftol`'s 64-bit result, or garbage on the early-0 path.
  Callers read EAX only (the history recorder `0x006A7126`).

The grouping changes the result. Worked example: a census whose Drives percents are 1, 3 and 96 (into `0x2010`,
`0x2020`, `0x2030`), supply 1000:

| Step | Original: (j[3] + j[2]) + j[1] | Other grouping: (j[1] + j[2]) + j[3] |
|---|---|---|
| factors | j[1] = 0.0099999998 (`0x3C23D70A`), j[2] = 0.029999999 (`0x3CF5C28F`), j[3] = 0.95999998 (`0x3F75C28F`) | same |
| first sum | j[3] + j[2] = 0.98999995 (`0x3F7D70A3`) | j[1] + j[2] = 0.039999999 (`0x3D23D70A`) |
| second sum | + j[1] = **0.99999994** (`0x3F7FFFFF`) | + j[3] = 1.0 (`0x3F800000`) |
| × supply 1000 | 999.999939 | 1000.0 |
| result | **999** | 1000 |

Among the 5151 splits of 100 % into three whole percents, 1198 give a different `float32` sum under the two
groupings.

**`GetLocalPopulationSummary` conversion** *[compat]* (DQ3). Both region uses share one conversion: the CRT's
`_ftol` at `0x009EEF04`, called at `0x006A6013`. The original stores **EAX** at `0x006A6018`.

- **How `_ftol` converts.** It turns the value into a **64-bit** integer, rounding toward zero (foundations
  §18.1), so EAX holds the low 32 bits of that `int64`.
- **In range** (|v| < 2^31): plain truncation toward zero.
- **2^31 ≤ |v| < 2^63**: the value **wraps** modulo 2^32. It does not saturate.
- **NaN, ±∞ and |v| ≥ 2^63**: the 64-bit conversion produces the integer indefinite `0x8000000000000000`, and its
  low 32 bits are **0**. So the result is 0, not `0x80000000`.

Worked examples, for the supply or demand value as `float32`:

| `v` | `float32` bits | Original | Saturating (`as i32`) |
|---|---|---|---|
| 1234.7 | `0x449A5666` | 1234 | 1234 |
| −56.9 | `0xC263999A` | −56 | −56 |
| 2147483648 (2^31) | `0x4F000000` | **−2147483648** | 2147483647 |
| 3000000000 | `0x4F32D05E` | **−1294967296** (`0xB2D05E00`) | 2147483647 |
| −3000000000 | `0xCF32D05E` | **1294967296** (`0x4D2FA200`) | −2147483648 |
| 1e19 (above 2^63) | `0x5F0AC723` | **0** | 2147483647 |
| +∞ | `0x7F800000` | **0** | 2147483647 |
| NaN | — | 0 | 0 |

*[choice]* Reproduce it exactly: 0 when `v` is NaN or |v| ≥ 2^63, otherwise the low 32 bits of `v` truncated to
`i64` (in Rust, `(v as i64) as i32` after that check). It differs from saturation only for |v| ≥ 2^31 or ±∞. The
shipped graph does not produce such values, but the result must match whatever the original stores.

**A summary map whose count is 0 but whose tree is not empty** (DQ4). The game never builds one. The caller's map
(regional city `+0x3C`) is only written by its constructor (`0x004AC09E`), by this method and by the regional
city's reader. That reader inserts each saved pair with `operator[]` (`0x004AC841`, `0x004AC8A1`). Every one of
these keeps the count equal to the number of nodes.

If such a map did arrive, the original would behave like this:

1. It **skips the clear** (step 1 tests only the count), so the stale nodes stay.
2. `operator[]` searches the stale tree. A key that is already there gets its value overwritten without a new
   node or a count increment. A new key gets a node linked into the stale tree.
3. The final count is the number of new nodes only. The stale entries for keys that were not rewritten stay
   visible to iteration.

That is a corrupt map, not a contract. Refusing it is the implementer's choice, and that choice stands.

**Who calls it, and on which thread** (DQ8). Its only caller is the regional city's `UpdateCityCache` (slot 24,
`+0x60`, `0x004AD290`). The regional city's own message handler (`DoMessage`, `0x004ABD40`) calls that for two
messages, and only when the message's city is non-null and belongs to this regional city:

| Message | Call site | Sent by (the only senders) |
|---|---|---|
| `kSC4MessagePostCityInit` `0x26D31EC1` | `0x004ABE76` | `cSC4App::NewCity` (`0x00454963`) and `cSC4App::LoadCity` (`0x004558C5`) |
| `kSC4MessagePreSave` `0x26C63343` | `0x004ABE9B` | `cSC4App::SaveCity` (`0x00456102`) |

Every other reference to these two IDs in the exe is an `AddNotification` or `RemoveNotification`. The app
sends or posts them through `cSC4App::PostCityMessage` (`0x00449AF0`) or the message server directly. Either way,
there is no dispatch thread (foundations §19.1), so the summary is built on the thread that creates, loads or
saves the city: the **main thread**. That happens once after a city is created or loaded, and
once before each save. *[inferred: static; no thread trace]* The glue's log of its first call settles it at run
time.


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
- The profile fields used here (`purpose` at +0x00) and the whole profile: [docs/abi/layouts.md](../abi/layouts.md) *Building profile*.
- **How the original reads the profile** (DQ5). At every cell that passes the lot tests, it calls BDS slot 39
  (`+0x9C`, `0x006A5A53`).
  - Slot 39 returns a **reference** into the process-wide profile table. There is no copy and no walk of the
    capacity maps.
  - It reads only `purpose` (+0x00, `0x006A5A5D`) and accepts 1…8 (`dec`, `cmp 7`, `ja`).
  - Cells without a lot, without capacity or with building type 0 never reach the call.
  - **Side effect.** Slot 39 inserts a zero profile for an unknown type. A lot's building type is normally known,
    so this does not fire in practice.

  Walking the capacity maps at each cell therefore has no original counterpart. It is harmless as long as it is
  read-only. *[choice]* Read `purpose` through the reference (or slot 40, which does not insert) and walk the
  capacity maps only in `UpdateCapacity` (§7.2).
- **Host shape** (DQ13). The census needs exactly one value per cell: the purpose, or "skip". Any host route that
  yields it is faithful, including a default `building_purpose` built on `building_profile`, provided an unknown
  type gives purpose 0 (skipped), which is what slot 39's inserted zero profile gives. A profile visitor that is
  null, so that nothing is walked, is the closest match to the original, which walks nothing here. No original
  behaviour goes further; the rest is the implementer's choice.

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
3 (Occupied). (`lot-developer.md` §3.2 gives the exact data4 rule of `cSC4Lot::SetState`: on leaving states 2/3 it
is the wealth the lot had before the call; the vacate reason is never sent. The `IGZUnknown` is a copy of the lot
made at post time.)

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

The profile and its capacity list (offsets, element type, who fills it, a worked example): [docs/abi/layouts.md](../abi/layouts.md) *Building profile*, *How `UpdateCapacity` reads it*.

**Header colour of the capacity maps** *[compat]* (DQ6). Every capacity map in a profile has a header colour of
**0**:

- **Copied maps.** The profile copy constructor `0x004D1230` copies a profile into its table node. Its vector copy
  constructor `0x004CDF00` makes each map with the map copy constructor `0x004C9470` (`0x004CDF5F`). That constructor
  allocates a new B header and writes colour 0 into it (`0x004C9499`) before it copies the nodes.
- **Other maps.** The other constructors also write colour 0.
- **Afterwards.** Nothing changes it (allocators.md *map and set*, *The header and the empty map*). The insert
  rebalance, for one, stops at the root, which it colours black.

A header with colour 1 therefore never occurs in a game map, and refusing one is the implementer's choice. With
colour 1, the original's iterator decrement would no longer recognise the header (allocators.md *map and set*).

**Sizes** (DQ7). The original has no fixed limit on any of these sizes:

| Quantity | Original | Shipped data (all packages scanned) | What can exceed it |
|---|---|---|---|
| capacity maps per profile | `LoadBuildingProperties` pushes **at most one** map per load (`0x0069BC59` / `0x0069BC7E`), and nothing else builds a catalogue profile's vector | 0 or 1 | nothing |
| entries in one capacity map | one per distinct key in property `0x27812834`, a `uint32` list of (key, amount) pairs of any length; a repeated key overwrites | **1**: each of the 558 exemplars with the property has exactly one pair | a plug-in exemplar with more distinct keys (only the 13 developer types of §2 have any effect) |
| nodes in the population summary | one per demand node with region use 1 or 2; demand exemplars may come from any group (§4.1) | 40 | plug-in demand exemplars with region use |

Limits of 16 maps, 128 entries and 4,096 summary nodes therefore hold for every shipped game. A larger value
needs plug-in data well beyond any normal mod. Refusing above those limits is the implementer's choice, and that
choice stands.

## 8. Monthly modifiers and cap relief

### 8.1 Tax and economy modifiers (census state 0)

```
neutral := max(0.1f, GetNeutralTaxRate())                    -- float32
recip   := r24(1.0f / neutral)                               -- once, before the first type; stored as float32
for i, (key, id) in enumerate([(0x21010,0x1010), (0x21020,0x1020), (0x21030,0x1030),
                               (0x23110,0x3110), (0x23120,0x3120), (0x23130,0x3130),
                               (0x23320,0x3320), (0x23330,0x3330),
                               (0x24100,0x4100), (0x24200,0x4200), (0x24300,0x4300), (0x24400,0x4400)]):
    maxRate := HistoryWarehouse.GetMinMaxAverage(0xAABAAA9F + i, today − taxPenaltyDays, today).max   -- float32
    ratio   := r24(r24(maxRate · 0.1f) · recip)
    taxMod  := curveTax(ratio)                                -- float32
    node[key].SetTaxModifier(r24(r24(OrdinanceSim.GetCumulativeFactor(0x2A650000 + id, 0) − 1.0f) + taxMod))
    econ := Nation.GetEconomyModifier(id)
    if id ∈ {0x3320, 0x3330}: econ += coEconomyBonus
    if id ≥ 0x4100:           econ += iEconomyBonus
    node[key].SetEconomyModifier(econ)
```

- **Ratio arithmetic** *[compat]* (`UpdateTaxAndEconomyModifiers` `0x006A6070`; foundations §18). `r24` is one
  rounding to a 24-bit significand (one `float32` operation). The 12 types are not a loop in the original: the code is
  unrolled into 12 copies, the first starting at `0x006A60EF` (`push 0xAABAAA9F`) and the last at `0x006A683A`
  (`push 0xAABAAAAA`). The ratio is a **multiplication by a rounded reciprocal**, not a division by `neutral`:
  1. `neutral` is the float32 return of `GetNeutralTaxRate()`, stored to a float32 local (`0x006A6096`). If it is
     below `0.1f`, the local is set to `0.1f` (`0x3DCCCCCD`, `0x006A60A7`). A NaN passes the test and is kept.
  2. Type 0 (`0x006A60F9`–`0x006A611A`): load `maxRate` (float32), multiply by the **float32** constant `0.1f` at
     `0x00A8C950` (bytes `CD CC CC 3D`). Then load `1.0f` (`0x00A81228`, bytes `00 00 80 3F`) and **divide** it by the
     `neutral` local. This gives `recip`, which is written back over the `neutral` local as a float32 (`FST dword`).
     Multiply `recip · (maxRate · 0.1f)` and store the product as the curve's float32 argument.
  3. Types 1–11 (for example `0x006A61A7`–`0x006A61BE`): `maxRate · 0.1f` as before, then multiply by the **stored
     float32 `recip`** reloaded from that local. There is no further division.
  4. Each product (`maxRate · 0.1f`, `1.0f / neutral`, the ratio) is a separate x87 result, so each is rounded once
     to 24 bits: three roundings in total. Type 0 uses `recip` from the register and types 1–11 reload it from memory.
     At 24-bit precision these are the same value, so all 12 types follow the formula above. *[compat]* Implement the
     reciprocal as float32 and use it for all 12 types.
  5. The curve is `cSC4ResponseCurve::GetOutputValue` (`0x005F4250`, foundations §4.3: three float32 operations).
     Its result is stored as float32. Then `GetCumulativeFactor(...)` (ST0) `− 1.0f` (`0x00A81228`) `+ taxMod` gives
     the tax modifier, stored as float32.

  The literal `maxRate · 0.1 / neutral` has one rounding fewer. It differs from the original by 1 ulp in the ratio
  for some inputs, and that becomes several ulp after the curve.

  **Worked example** (game date 2001-10-01, run 43). `neutral = 8.99755573` = `0x410FF5FD`, `maxRate = 70`, ordinance
  factor 1, shipped curve (0.2, 0.3) (1, 0) (5, −3.5). The segment-1 slope is `0xBEC00000` = −0.375.

  | step | original | literal `maxRate·0.1/neutral` |
  |---|---|---|
  | `maxRate · 0.1f` | `0x40E00000` (7.0 exactly after rounding) | same |
  | `1.0f / neutral` | `0x3DE39E0C` | — |
  | ratio | `0x3F472A4A` (0.77798903) | `0x3F472A4B` (0.777989089) |
  | taxMod = tax modifier | **`0x3DAA8124`** (0.0832541287) | `0x3DAA8120` (0.0832540989) |

  The original showed `0x3DAA8124` for the 11 types whose `maxRate` was 70. I-R had `maxRate = 90`: the ratio is
  `0x3F8008E7` in both orders, and the modifier is `0xB9794400` (−0.000237718). At 2001-09-09 (`neutral = 8.99767494`
  = `0x410FF67A`, `1.0f / neutral` = `0x3DE39D47`, `maxRate = 70`), both orders give ratio `0x3F47299E` and modifier
  `0x3DAA8328` (0.0832579732). That is why the differential found only the one mismatching month.
- `today` is the simulator's current date. The tax history IDs `0xAABAAA9F…0xAABAAAAA` are the tax-rate series,
  one per type, in the order above. They are recorded elsewhere (budget). The tax term uses the **highest** rate in
  the window, so a tax cut takes up to `taxPenaltyDays` (shipped: 90) to help *[compat]*. The `· 0.1` means the
  history stores the rate in tenths of a percent *[inferred]*.
- Ordinances contribute through a multiplicative factor per demand type (`0x2A651010`, …, `0x2A654400`), applied
  as `factor − 1`.
- The new modifiers take effect at the node's next recalculation (next propagation pass).
- `coEconomyBonus` (`+0xA0`) and `iEconomyBonus` (`+0xA4`) are saved fields that **no code can set**, except the
  save loader (§15 Q5, settled by a vtable scan). A new city has 0, and every save the original writes for it keeps
  0. *[compat]* Keep, save and apply them, so a save edited by hand behaves the same.
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

Two details from the bytes (nation.md §2): `months` is converted to float as an **unsigned** value (a negative count
becomes `months + 2^32`), and the difficulty term is added only while a city exists.

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
CalculateJobsPerUnitOfDemand(j, devType + 1, 0)           (writes j[1], j[2], j[3]; j[0] unused)
for w = 0..2:  v := f32(j[w + 1] · n)
               node[0x12010 + 0x10·w].AddToDemandValue(v);  c.ExtrapolateGrowth(0x2010 + 0x10·w, v)
```

**Which output each wealth reads** *[compat]* (QUESTIONS DQ10; `0x006A6BD8`–`0x006A6C83`). Each wealth reads the
output slot 8 computed for its own jobs node: $ (`0x12010`, `0x2010`) reads `j[1]`, $$ (`0x12020`, `0x2020`) reads
`j[2]` and $$$ (`0x12030`, `0x2030`) reads `j[3]`. The three products `j[k] × n` are each one 24-bit rounding, all
formed before the first `AddToDemandValue`. Here `n = f32(r × amt)` with `r = f32(p1 / p)` (`FIDIV`, one rounding)
or 0.8f. `amt` arrives as a `float32`.

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

**"Population" versus "jobs" for C and I** *[compat]* (how the original's other subsystems read these numbers):

- **C / I population** is the **supply** of the census sum nodes: C = `node[0x3001].supply`, I =
  `node[0x4001].supply` (R = `node[0x1001].supply`); region-wide C / I / R are the supplies of `0x23001`,
  `0x24001`, `0x21001`. This is what the city-planning advisor's population variables use. Never the demand.
- **C / I jobs** are the `GetJobsBySensus` sums above (`0x0A4E2056`, `0x4A4E206B`). The regional-city record's
  commercial and industrial jobs are computed the same way (per census key, trunc(trunc(supply) × the key's
  summed jobs-per-unit factors), summed over the five C keys and the four I keys).
- The two agree for commercial (every C census node's job factors sum to 1, up to float truncation). They differ
  for industry: `0x4001` sums only `0x4201`, `0x4301`, `0x4401` (agriculture excluded), while I jobs count
  agriculture `0x4101` at its job factor (shipped 0.2). The city's total population figure is the residential
  simulator's `GetPopulation`, not a demand node.
- A diagnostic that wants the original's "C and I population" should therefore read the supply of `0x3001` and
  `0x4001` (local index), not a sum over the developer-type nodes `0x3110 … 0x3330` / `0x4100 … 0x4400`, which
  are the capacity/demand nodes of §2, not census counts.

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

**Missing exemplar or properties** *[compat]*: the original has **no usable code defaults**. The nation reads this
exemplar on each `Init` after `Shutdown` (normally once in an application lifetime; its message handler does
not handle the tuning-reload message; nation.md §6.3 describes reinitialization).
Before reading, it sizes the difficulty vector to 3 (new elements 0.0). Then:

| Situation | Result in the original |
|---|---|
| exemplar missing, or `0x4A1F629F` missing or empty | no curves; difficulty [0, 0, 0] (`0x4A1F6300` is read only inside the per-ID loop, so it is never read). `GetEconomyModifier` finds no curve for the demand ID and none for ID 0, and then uses an uninitialised pointer: **undefined behaviour** (most likely a crash). |
| an ID listed but its curve property missing | an **empty** curve for that ID; evaluating it dereferences null data (crash) when that ID, or ID 0 as the fallback, is used. A one-point curve returns its constant. |
| `0x4A1F6300` missing | difficulty stays [0, 0, 0]: adds 0 |
| `0x4A1F6300` with fewer than 3 values | the vector takes that length; the difficulty index is not bounds-checked (reads past the end) |

Neither property is type-checked; the curve and difficulty data are taken from whatever type is stored. An ID
listed twice rebuilds the same curve with the later index's data. The difficulty term is added only while a city
exists. *[choice]* FreeCity4 defaults, used only when the exemplar or a property is missing: one curve for ID 0
equal to the constant 0, and difficulty [0.06, 0.03, 0.01] (the shipped values); an empty curve evaluates to 0.

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
nodeCount × { uint32 key;  ref node }        ascending key; ref = foundations §2.7 (u32 objectID, u32 classID)
uint32   censusCount
censusCount × { uint32 key;  float32 value }                                         previous census, ascending key
```

- `Write` returns false without writing if the stream is already in error. Otherwise it returns "no stream error".
- `Read` fails for a stream error or `version ≥ 3`. Otherwise it sets **loadedFromSave** (after the version
  check, before reading the rest), reads each key and resolves the reference with a delayed read (foundations
  §2.7, §3.5), requesting interface `0x0797A4E8`, stores the result under `key`, and fills the previous census.
  Returns "no stream error".
- **Node references** *[compat]* (answers the former open question 1): each node is an ordinary object reference,
  not an embedded record. An entry is `u32 key, u32 objectID, u32 classID` (12 bytes; classID is always
  `0x8990C12D`), or `u32 key, u32 0` (8 bytes) for a null reference; the original writes objectID 0 only if the
  node does not answer `cIGZSerializable`, which never happens. The node's own data (§12.2, 55 bytes for version
  2) is written once by the class-object store into the class record `0x8990C12D` (foundations §3.3); nodes that
  other subsystems created (for example the building development simulator's own node) share that record but are
  not referenced from this payload.
- *[quirk]* `Read` does not clear the existing map, does not check the reference read's result (a null or failed
  reference stores a **null** node under the key, and a later use of that key crashes), overwrites a duplicate key
  without releasing the old node, and does not check that the key equals the node's id or that keys ascend.
  *[choice]* FreeCity4 skips null or failed references and rejects duplicate keys.
- Checked against two sample saves (version 2, 173 references, all class `0x8990C12D`, keys ascending, every
  node's id equal to its key; the node class record holds 174 nodes of 55 bytes).
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
  Version 0 is read like version 2 (float cap); version 1 reads the cap as a byte × 0.01f. The flag byte is read
  as a boolean (any non-zero value becomes 1).
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

1. *(resolved)* Node encoding in the save: object references, with the node data in class record `0x8990C12D`
   (§12.1).
2. **Regional sum method** (§9.1): slot 17 of `cISC4RegionalCity` is `GetExtrapolatedPopulation(type)` if the SDK
   order is right. The extrapolation code uses slot 16 (`GetPopulation(type)`), so the two are distinct. Confirm
   with a region test that regional nodes equal the neighbours' extrapolated values, not their real population.
3. **Census per cell vs per lot** (§6.3): does `cISC4Lot::GetCapacity(devType, false)` return the whole lot's
   capacity (then multi-cell lots are over-counted by their cell count) or a per-cell share?
4. **Slew limiter** (§3.3): no caller with `smooth = true` was found in either build. A full scan of indirect
   calls would settle it.
5. *(resolved 2026-10-13, DQ2)* **CO/I economy bonuses** (§8.1): **dead, apart from the save record.** The
   simulator object has three vtables: `cISC4DemandSimulator` `0x00AB0E48` at `+0` (14 slots), `cIGZMessageTarget2`
   `0x00AB0E38` at `+4` (4 slots), and `cIGZSerializable` `0x00AB0E20` at `+8` (6 slots). Every target was
   disassembled, along with every function of the class (`0x006A2300`–`0x006A97D0`). The scan looked for memory
   operands at the field offsets, corrected for each interface's `this` adjustment (`+0xA0`/`+0xA4` from `+0`,
   `+0x9C`/`+0xA0` from `+4`, `+0x98`/`+0x9C` from `+8`). The fields are touched only by:
   - the constructor (zero, `0x006A36F3`/`0x006A36F9`);
   - `Init` when not loaded from a save (zero, `0x006A8D11`/`0x006A8D17`);
   - `Write` (`0x006A26A4`/`0x006A26B2`);
   - `Read` (`0x006A3CCC`/`0x006A3CDA`);
   - the six `FADD`s in `UpdateTaxAndEconomyModifiers` (§8.1).

   No slot returns their value or their address. The only addresses taken are in `Read`, which passes them to the
   stream's float reader. Slot 13 (`+0x34`, `0x006A52D0`, after the destructor) is a class virtual outside the
   interface. It stores a node in the node map (`+0x50`) and does not touch the bonuses. `DoMessage` does not
   touch them either.

   Outside the class, other code reaches the simulator only through its interfaces. As a cross-check, the 358
   `call [reg+0xE0]` sites (`cISC4City::GetDemandSimulator`) were followed for 60 instructions, and none reads or
   writes the result at `+0xA0`/`+0xA4`.

   So the only way in or out is the save record (§12.1: bytes 2–9 of the record, after the `uint16` version):
   - a tool can edit a save;
   - a plugin could QI `cIGZSerializable` (`0xE4FDA3D4`) and call `Write` to read them *[untested: the stream must
     accept the node references]*;
   - a plugin cannot safely call `Read`, because it re-reads the node map with the quirks of §12.1.
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
