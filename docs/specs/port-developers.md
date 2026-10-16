# Spec: the airport and seaport developers (`cSC4AirportDeveloper`, `cSC4SeaportDeveloper`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from observing the original program and its data files. Implementers must work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, read-before-init, §13.4 the month-agent tail),
[simulator.md](simulator.md) (§4 `AddAgent`), [budget.md](budget.md) (departments, line items, `WithdrawFunds`),
[demand.md](demand.md) (§7 `UpdateOccupantEffects`, §8 the port trips in the cap relief),
[traffic-simulator.md](traffic-simulator.md) (trip capacity and arrivals), [traffic-network.md](traffic-network.md)
(subnetworks), [history.md](history.md) (the record message), [advisors.md](advisors.md) (events, Lua
`event_feedback`), [lot-developer.md](lot-developer.md) (lot states and conditions, demolition, `ConstructDefaultLot`),
[zones.md](zones.md) (§6 `PlaceZone`; zone types 11 airport and 12 seaport).

## 0. Summary

The two developers are one class, `cSC4PortDeveloper`, with two thin subclasses. A port developer:

- **adopts** every phase-1 airport (seaport) lot that is built, as a *port lot* with a growth **stage** 0;
- runs a **month agent** that computes each port's trips, *efficiency* (spare funded capacity), usage ratio and
  lot condition, scales the port building's demand effects, offers an upgrade through the advisors, and records the
  average efficiency in the history warehouse;
- **upgrades** every port that is ready when the matching growth message arrives (the player answered *yes* to the
  advice): the lot is demolished and rebuilt with the next stage's lot configuration, and the upgrade cost is
  withdrawn;
- owns one **budget department** whose line 1 holds the summed monthly cost of all ports;
- forgets a port when its lot is destroyed and dezones the footprint.

The airport's trips come from the office jobs (developer types `0x3320` CO$$ and `0x3330` CO$$$) of the road
subnetworks it touches; the seaport's trips are the freight arrivals the traffic simulator counts at its lot, and
the seaport also keeps a sea-access map of the water that touches the map edges. Both feed the demand simulator's
cap relief through `GetNum*Trips` (demand.md §8).

The shipped data has three airport types (small, medium, large; three stages each) and one seaport type (three
stages). Only the airport can grow in the shipped game: there is no advice for the seaport growth event (§4.7).

Implementation size: about 600 lines for the shared core (tuning, month, growth, record) plus 150 for the airport
and 250 for the seaport (water map); no RNG.

## 1. Identity

| Item | Airport | Seaport |
|---|---|---|
| Class ID (`GetGZCLSID`) | `0xAA2049A1` (`0x0068AFD0`) | `0xEA230A28` (`0x00700E80`) |
| Component | city component **13**; `cISC4City` slot 77 (`+0x134`) `GetAirportDeveloper` | component **14**; slot 76 (`+0x130`) `GetSeaportDeveloper` |
| Factory | `0x0068B520`: global heap, **0x64** bytes, constructor `0x0068B4C0` | `0x007019F0`: **0x7C** bytes, constructor `0x007018F0` |
| `GetSimulatorType` | `0xEA3955C4` | `0xEA3957AD` |
| Interface at +0 | `cISC4AirportDeveloper` `0xEA20548D`, vtable `0x00AB0714` (15 slots) | `cISC4SeaportDeveloper` `0xCA22FE3E`, vtable `0x00AB2E10` (17 slots) |
| Port sub-object at +4 | vtable `0x00AB06C8` (19 slots, §8.3) | `0x00AB2DC0` |
| Message target at +8 | `0x00AB06B4` (`DoMessage` `0x0068B1F0`) | `0x00AB2DAC` (`DoMessage` `0x00701F20`) |
| Serializable at +0xC | `0x00AB069C` (`Write` `0x0068B3E0`, `Read` `0x007010B0`) | `0x00AB2D94` (same `Write` and `Read`) |

`QueryInterface`: the subclass IID → +0. Anything else goes to the port sub-object: `1` and `cISC4PortDeveloper`
(`0x4A2C4E69`) → **+4**; `0x452294AA` → +8; `0xE4FDA3D4` → +0xC; otherwise false. Note that `QueryInterface(1)`
returns +4, not +0 *[compat]*. `AsISC4PortDeveloper()` (slot 10) returns +4 as well; slots 0–9 of the +4 vtable are
the same methods as slots 0–9 of the +0 vtable, so +4 is a valid `cISC4PortDeveloper`.

Reference count at port +0x10 (object +0x14); the release of the last reference calls `Shutdown` and the
destructor.

## 2. State

Offsets are relative to the **port sub-object** (object + 4).

| Offset | Field | Initial (airport / seaport) | Saved |
|---|---|---|---|
| `+0x0C` | `tuningLoaded` (byte) | 0 | no |
| `+0x0D` | `initialized` (byte) | 0 | no |
| `+0x14` | tuning exemplar instance | `0xCA270E9D` / `0xEA29AD92` | **yes** |
| `+0x18` | number of port types listed in the tuning exemplar | 0 | no |
| `+0x1C`…`+0x2C` | message server, zone manager, lot manager, city, budget simulator (held at `Init`) | null | no |
| `+0x30` | the budget department | null | no |
| `+0x34` | zone type: **11** / **12** | | **yes** (u16) |
| `+0x38` | history key: `0x6A3E9032` / `0xEA3E904A` | | no |
| `+0x3C` | department ID: `0x6A4119F0` / `0x4A4155AB` | | no |
| `+0x40` | building occupant group: `0x1508` (airport) / `0x1509` (seaport) | `0xFFFFFFFF` in the base | no |
| `+0x44` | **port lots**: list of `cPortLot*` (insertion order) | empty | **yes** |
| `+0x48` | port types: vector of `cPortTypeConfig*` (load order) | empty | no |
| `+0x54` | port type exemplar instances (property `0x8A270FC3`) | empty | no |

Seaport only (object offsets): `+0x64` `recheckAccess` (byte), `+0x65` `rebuildMap` (byte, never set to 1 by any
shipped code), `+0x66` `cityHasSea` (byte), `+0x68` sea-access map (vector of bytes, row-major, `width × height`
cells), `+0x74` width, `+0x78` height. None is saved.

**Port lot** (`cPortLot`, 0x28 bytes): `+4` stage (u32, 0-based); `+8` port type; `+0xC` `needsLookup` (byte: the
lot pointer must be fetched again from the lot manager at (x, z)); `+0xE` x, `+0x10` z (s16, the lot's location);
`+0x14` efficiency (float, initially 1.0); `+0x18` `swapping` (byte); `+0x1C` usage ratio (float, initially 0);
`+0x20` the lot (held); `+0x24` trips (u32, airport only). Getting the lot resolves it first if `needsLookup`.

**Port type** (`cPortTypeConfig`, 0x30 bytes): index; stage count; *efficiency threshold*; size class (u8, default
2); one **growth rule** per stage: lot configuration ID, monthly cost (s64), capacity (u32), upgrade cost (s64);
optional terrain grid (rows × columns of floats). See §6.

## 3. Agents, messages and cadence

**AO1 answer** *[compat]*: both developers register **one month agent each with the same name,
`cSC4PortDeveloper`** (the shared base `Init`; there is no airport- or seaport-specific name): `AddAgent(target =
object + 8, level 6, "cSC4PortDeveloper", flags 2)`. Flags 2 means the agent is enabled only while the city is
established (simulator.md §4). `GetAgentEnabled`/`SetAgentEnabled("cSC4PortDeveloper")` therefore see and switch
**both** developers. In the month level they are two registrations; in FreeCity4 they keep the fixed tail position
of foundations §13.4 (after the landfill developer, before the neighbours); *[choice]* airport before seaport
(component order; the logs do not separate the two).
`Shutdown` removes the agent from every level.

Month message `0x66956816` (the agent): the subclass's `DoMonthlyTick` (§4.4), then the shared month (§4.3).
Returns true.

Subscriptions:

| Message | Who | Handling |
|---|---|---|
| `0x99EF1142` occupant inserted | both | void1 = occupant (held for the call). If its property holder lacks the preview marker `0x029244DB`, its type is building `0x278128A0` and it is in occupant group `+0x40` (occupant slot 22): `lot := lotManager.GetOccupantLot(occupant)` (slot 14); if non-null, **adopt** (§4.1) |
| `0xE8BDA2E7` lot state changed | both | lot = IGZUnknown queried for `cISC4Lot` (`0xE818FCED`), old = data1, new = data2: §4.6 |
| `0x26D31EC1` PostCityInit | both | resolve every port lot's lot pointer (§2) |
| `0xEA394B74`, `0xCB3BE815`, `0xEB3BE415`, `0x2B3BE44A`, `0x6B3BE6D8`, `0x0B3BE81A` | airport | **grow** (§4.5) |
| `0x8A394DC4` | seaport | **grow** (§4.5) |
| `0x6A3B0AFD` (end of a long terrain operation) | seaport | rebuild the sea-access map (§4.8), `recheckAccess := 1` |

Every handler returns true.

Outgoing: history record `0x89EFA536` (sent synchronously, history.md); advisor events through the advisor system
(`cISC4City` slot 81, then system `PushEvent(event, null)`, advisors.md §6.3); budget calls (§4.2, §4.5); demand
`UpdateOccupantEffects`; traffic `SetMaxTripCapacity` (seaport); lot-developer demolition and construction; zone
manager `PlaceZone`.

## 4. Behaviour

### 4.1 Adoption (`DoLotInserted`, `0x006EA0E0`) *[compat]*

1. `config := lot.GetLotConfiguration()` (lot slot 9); null → stop. If `config.GetGrowthStage()` (lot configuration
   slot 11, property `0x27812837`) is not **1**, stop: only phase-1 lots start a port. (A phase-2 lot built by an
   upgrade is inserted too and is ignored here.)
2. Make a port lot: stage 0, efficiency 1.0, usage 0, location from `lot.GetLocation()` (slot 21), the lot held.
3. Find the port type whose **stage-0** rule has `config.GetID()` as its lot configuration (types in load order,
   `i < typeCount`). None → discard the port lot (releasing the lot) and stop.
4. Append it to the list, `UpdateBudget()` (§4.2), `OnNewLotCreated(port)` (airport: nothing; seaport: §4.8).

No check for a duplicate: a lot adopted twice is listed twice *[quirk]* (not reachable with shipped data).

### 4.2 Budget

At `Init`: `dept := budget.GetDepartmentBudget(departmentID)` (slot 28); if none, create it
(`CreateDepartmentBudget(departmentID, 0xAA369059)`), `SetDepartmentName(0x6A3FF01C, 0xEA493564 airport /
0xCA493AF3 seaport)`, `SetMaxAllowedFundingPercentage(1.2)`, create line item 1 (not locally funded) and clear its
display flag 1. The department is held.

- `UpdateBudget()`: line 1 `SetFullExpenses(RequiredBudget())`, where `RequiredBudget()` = Σ over port lots of the
  current stage's monthly cost. Called on adoption, removal and growth only (not at load: the budget's saved line
  keeps the value).
- `AllocatedFunding()` = line 1 `GetCurrentExpenses()`.
- **Funding ratio** `fr` (airport and seaport tick, usage values, maintenance cost): `current / full` from line 1
  as floats when `full > 0`, else **1.0**. It can exceed 1 (funding above 100 %).

### 4.3 The month (`0x006E9980`) *[compat]*

Nothing happens without port lots. Otherwise:

```
funded := float(AllocatedFunding());  ratio := funded / float(RequiredBudget())
offered := false
for each port p (list order):
    lot := p.lot;  state := lot.GetState()
    if lot.GetCondition() = 1: continue                       -- unpowered or building (lot-developer.md §5)
    trips := GetTripsForPort(p)                               -- §4.4
    p.efficiency := Efficiency(capacity(p) × ratio, trips)
    (usage, cap) := PortUsageValues(p)                        -- usage = min(trips, capacity), cap = capacity
    new := float(usage) / float(cap)
    demand.UpdateOccupantEffects(lot.GetBuildingType(0), &p.usage, &new);  p.usage := new   -- PD2, below
    stressed := (p.efficiency ≤ type.threshold or funded ≤ 0) and lot.GetDaysSinceLastStateChange() > 90
    lot.SetCondition(stressed ? 2 : 0)
    if state ∈ {1, 3, 4} and GrowthTriggered(p) and not offered:
        SendAdvisorUpgradeMessage(p);  offered := true
record history (key +0x38, round(AverageEfficiency() × 100))
```

- `capacity(p)` = the current stage's capacity; `Efficiency(c, t)` = 1.0 if `t ≤ 0`; 0 if `c ≤ 0` or `t > c`;
  otherwise `1 − t / c`. It is the **spare** fraction of the funded capacity: high means idle.
- `ratio` is `funded / required` (not `fr`): with no funding it is 0, every port with trips gets efficiency 0.
  With `RequiredBudget() = 0` (impossible with shipped data) it is a division by zero *[quirk]*.
- The usage update (PD2) is **on top of** the building's own demand effects. `UpdateOccupantEffects(bt, old, new)`
  removes the effects at `old` and adds them at `new` (demand.md §7.1), and the port's first call has `old = 0`. The
  lot's occupancy change (state 1 → 3) has already added the effects at full strength (demand.md §7.1), so an
  occupied port contributes (1 + usage) × its effects, and the monthly call moves only the usage part *[inferred:
  port lots take the common occupancy path]*.
- `GrowthTriggered(p)`: there is a next stage, the lot is not constructing (state ≠ 2) and `float(trips) ≥
  float(capacity(p)) × 0.96`.
- Only the first triggered port in list order is offered each month.
- `AverageEfficiency()` = the mean of the port efficiencies, **1.0** with no ports. The value is rounded to the
  nearest integer, ties to even (the original adds 1.5 × 2²³ in float) and sent as data2 of `0x89EFA536` with data1
  = the history key. Nothing is recorded in a month without ports.
- A port that is skipped (condition 1) keeps last month's efficiency and usage.
- **What condition 2 does on a port lot** (PD3). The same as on any lot; nothing reads it as "port stressed". The
  lot developer's civic branch (lot-developer.md §4.2, purpose 0) changes only conditions 0 → 1 (unpowered), 1 → 0
  (powered) and 5, 6 → 0, and `TestPower` and the power-changed handler likewise only 0 ↔ 1 (§5.2, §9 there), so condition 2 stays until this
  month sets 0 again. Two consequences *[compat]*: a stressed port that then loses power is **not** set to 1, so the
  month above does not skip it and it keeps updating its efficiency and usage; and when it stops being stressed it
  gets 0, and an unpowered port then becomes 1 at the lot developer's next visit. The readers of condition 2 are the
  generic ones: the base texture is re-picked with decay variant 1 (lot-developer.md §6.5, `0xE99B3D4D` and
  `PickLotBaseTextureForLot`; only when a matching texture resource exists), the crime simulator's lot condition bit `0x2`
  (crime.md §6.7, `GetLotBits`), and the MySim `home_condition` value (mysims.md).

### 4.4 Trips

**Airport** (`DoMonthlyTick`, `0x0068B630`; `GetTripsForPort` returns `p.trips`):

```
for each port p: p.trips := 0
a := traffic.GetOccupantCountForAllSubnetworks(0x3320);  b := …(0x3330)     -- one entry per subnetwork id
bucket[s] := ports whose lot is in subnetwork s (traffic.GetSubnetworksForLot; a lot can be in several)
for s := 0 … size(a) − 1 with bucket[s] non-empty:
    D := trunc(float(Σ_{q ∈ bucket[s]} capacity(q)) × fr)
    for each p in bucket[s]:
        p.trips += trunc( float(trunc(float(capacity(p)) × fr)) / float(D) × float(a[s] + b[s]) )
```

`trunc` is the CRT truncation toward zero, low 32 bits (foundations §18.1; NaN → 0). With `fr = 0` every share is
0/0 → NaN → 0 *[compat]*.

**Seaport** (`GetTripsForPort` `0x00700F60`): `traffic.GetTrafficArrived(lot.AsPropertyHolder(), 3)` — the freight
arrivals of the last traffic cycle. `DoMonthlyTick` (`0x00701F70`):

1. if `rebuildMap`, rebuild the map (§4.8; never true in the shipped game);
2. for each port: `UpdateTripCapacity(p)` = `traffic.SetMaxTripCapacity(lot.AsPropertyHolder(), 3,
   trunc(capacity(p) × fr))`;
3. if `recheckAccess`: clear it; every port whose lot has **no** sea access (§4.8) gets its props with property
   `0xC98204B8` switched to state 1 (prop `SetState(1)`, presentation); if any, push advisor event `0x4A490D33`
   (BAD_SEAPORT_LOCATION).

`GetNumAirportTrips()` / `GetNumSeaportTrips()` = Σ `GetTripsForPort` over the ports (demand.md §8).

### 4.5 Growth *[compat]*

`SendAdvisorUpgradeMessage(p)` (port slot 15):

- **Airport**: event by the port type's size class and whether the port is at stage 0:

  | Size | Stage 0 | Later stage |
  |---|---|---|
  | 1 small | `0xEA394B74` GROW_SMALL_AIRPORT1 | `0xCB3BE815` …2 |
  | 2 medium | `0xEB3BE415` | `0x2B3BE44A` |
  | 3 large | `0x6B3BE6D8` | `0x0B3BE81A` |
  | other | 0 | 0 |

- **Seaport**: always `0x8A394DC4` (GROW_SEAPORT).

The event is pushed to the advisor system. The shipped advisor scripts answer the six airport events with a YES/NO
advice whose YES link calls `game.event_feedback()`, which **posts a message whose type is the event ID**
(advisors.md §8.3). That message, received by the developer, runs **GrowPortsToNextStage**. There is no advice for
`0x8A394DC4`, so the seaport never grows *[compat]*. (Each subclass also checks a debug flag, `0x00B47B5C` for the
airport and `0x00B4AFBC` for the seaport, that would post the message itself or grow at once; both are zero and
never written in the shipped program.)

**GrowPortsToNextStage** (`0x006E9BD0`): for **every** port lot (not only the one the advice was about) with
`GrowthTriggered(p)`:

1. `rule := type.rules[p.stage + 1]`. **Swap** the lot (`0x006E92A0`): `newConfig :=
   lotConfigurationManager.GetLotConfiguration(rule.lotConfig)`; `p.swapping := 1`; keep the old lot's facing and
   bounding rectangle; lot developer (building development simulator slot 36) `StartDemolishLot(old)` then
   `EndDemolishLot(old)`; drop the old lot; `lotManager.CreateLot(x0, z0, x1 − x0 + 1, z1 − z0 + 1, facing, &new)`
   (slot 23). If that fails the swap fails (the old lot is already gone *[quirk]*). Otherwise `p.lot := new`,
   `new.SetLotConfiguration(newConfig)`, `new.UpdateZoneType()`, and the result is the lot developer's
   `ConstructDefaultLot(new, newConfig, 0, 1)`.
2. If the swap succeeded: `p.stage += 1`; `budget.WithdrawFunds(rule.upgradeCost)` — **the result is ignored**, so
   an upgrade the city cannot afford is free *[quirk]*; `OnStageChange(p)` (airport nothing, seaport
   `UpdateTripCapacity`); `UpdateBudget()`.

The demolition in step 1 posts the state change to 5 for the old lot; §4.6 sees `swapping` and only clears it.

### 4.6 Removal (`0x006E9EA0`, `0x006E9D70`)

On a lot state change to **5** (destructing): if the lot's configuration is compatible with the zone type `+0x34`
(lot configuration slot 15) and a port lot has the **same location** (the match is by location, not by lot
pointer): if `swapping`, clear it; otherwise remove the port:

1. `demand.UpdateOccupantEffects(lot.GetBuildingType(0), &p.usage, &0)`; `p.usage := 0`;
2. dezone the lot's bounding rectangle for free: `PlaceZone(region, 0, commit, skipFunds, no slope check, skip size,
   lot need not be inside, null, null, null)` (zones.md §6);
3. erase and delete the port lot; `UpdateBudget()`.

### 4.7 Queries

| Method | Result |
|---|---|
| `GetAvgEfficiency()` | `AverageEfficiency()` (§4.3), returned through a hidden pointer |
| `GetLeastEfficientPort(&pct, &point)` | no ports → false; one port → its efficiency and location, true. **Two or more ports: the original loops forever** (the search loop never advances) *[quirk]*. No shipped caller was found, and no Lua binding exists: the executable's only port names for scripts are the `g_*airport*` and `g_seaport_*` count and volume globals (strings at file offset `0x682C0E`–`0x683090`), and no string contains `LeastEfficient` (PD1). *[choice]* FreeCity4 returns the minimum efficiency (first port on ties) and its location |
| `GetAllPortLots(vector&)` | appends every port's lot (resolving it), one reference each |
| `GetNumPorts(sizeFlags, stageFlags)` | ports with `sizeFlags & (1 << sizeClass)` and `stageFlags & (1 << stage)`. **The size bit is `1 << size class`** (small 2, medium 4, large 8); the SDK's `PortSizeEnumToFlag` (1, 2, 4) is wrong. The stage bits are 1, 2, 4 for stages 0, 1, 2 |
| `GetNumAirports(size)` | `size = 0`: number of ports; otherwise `GetNumPorts(1 << size, 0x1F)` |
| `GetNumSeaports()` | number of ports. **Takes no argument** (`ret` without pop); the SDK declares one |
| `Get{Airport,Seaport}UsageValues(lot, &usage, &cap)` | the port of that lot: `cap := trunc(capacity × fr)` if line 1 exists and `full > 0`, else capacity; `usage := min(trips, cap)`; AL = found. Not found: 0, 0, false |
| `GetMaintenanceCost(lot)` | the stage's monthly cost × `fr` (int64; 0 if the lot is not a port) |
| `CityHasSea()` | seaport: `cityHasSea` |
| `GetCellSeaAccess(x, z)` | seaport: the map byte (no bounds check) |

### 4.8 Seaport water access

**Map** (`BuildWaterAccessMap`, `0x00701BB0`; at `Init` and on `0x6A3B0AFD`): size from the terrain (slots 7 and
8, cells); each cell `0x80` if `terrain.CellHasWater(x, z)` (slot 34), else 0. `cityHasSea := false`. Then four edge
scans, each starting a **flood fill** at the first water cell of every run of water along that edge, with an edge
bit: row z = 0 → bit 1; row z = height − 1 → bit 8; column x = 0 → bit 2; column x = width − 1 → bit 4. Any fill sets
`cityHasSea`. The fill (`cSC4MapUtilities::FloodFill`, `0x005E3AC0`) is a 4-connected scanline seed fill over
cells whose byte has `0x80` and not yet the bit; it ORs the bit in. Its segment stack holds 10 000 entries; a push
beyond that is dropped *[compat]*. So a water cell's low four bits say which map edges its water body reaches.
`rebuildMap := 0`.

**Access** of a port (`0x00701710`): true if any cell of its lot's bounding rectangle has a non-zero low nibble.

**New seaport** (`OnNewLotCreated`, `0x00701370`): `UpdateTripCapacity(p)`, `recheckAccess := 1`. If the port type
has a terrain grid and its size equals the lot's cell area, each lot cell's terrain altitude is set
(`SetAltitudeAtVertex`) from the grid, read in an orientation chosen by the lot's facing. *[quirk]* The shipped
seaport type declares 11 × 5 but ships 72 values (a 12 × 6 lot), so the grid is rejected at load and this step
never runs. The code counts **cells** in both places (PD4): the load keeps the grid only when rows × columns equals
the value count (`0x006EA676`–`0x006EA690`), and the use only when rows × columns equals the lot rectangle's cell
area, then sets one vertex per cell, the far edge excluded (`0x007013D7`–`0x0070141A`). So the declared size is the
data error; a 12 × 6 declaration would load, and would apply to a 12 × 6 lot.

## 5. Random numbers

None.

## 6. Tuning *[compat]*

Read once (`0x006EA840`), at `Read` or `Init` whichever comes first, only if the instance is non-zero, from
exemplar `0x6534284A` / `0x67CD5FA1` / instance `+0x14`:

| Property | Use | Shipped airport `0xCA270E9D` | Shipped seaport `0xEA29AD92` |
|---|---|---|---|
| `0x8A270FC3` | port type exemplar instances (u32 list); `typeCount` = its length | `0x0A271653`, `0x0A271647`, `0x2A27163F` | `0xCA2D8CE6` |

Each port type (`0x006EA3F0`), same type and group, the listed instance; a type whose exemplar is missing is not
appended, so later types shift down *[quirk]* (`typeCount` is not reduced; *[choice]* FreeCity4 bounds every scan by
the loaded count):

| Property | Meaning | Small | Medium | Large | Seaport |
|---|---|---|---|---|---|
| `0xCA271780` u32 | stage count (required) | 3 | 3 | 3 | 3 |
| `0xCA271781` u32[] | lot configuration per stage | `0xCA2436C0`–`C2` | `0x6A5126D0`–`D2` | `0x0A512810`–`12` | `0x4A52917D`, `0x4A63F3C1`, `0x8A63F610` |
| `0xCA271783` s64[] | monthly cost | 700, 800, 1000 | 1200, 1400, 1700 | 2000, 2400, 2900 | 500, 600, 700 |
| `0xCA271784` u32[] | capacity (trips) | 10000, 17500, 30600 | 53600, 93800, 164200 | 287400, 503000, 880300 | 650, 900, 1200 |
| `0xCA271786` s64[] | upgrade cost into the stage | 0, 5700, 9700 | 0, 16500, 28100 | 0, 47800, 81300 | 0, 6000, 6000 |
| `0xCA271785` (×0.01) | efficiency threshold | 0.25 | 0.25 | 0.25 | 0.25 |
| `0xCA271787` u8 | size class (default 2) | 1 | 2 | 3 | 1 |
| `0xCA271788`, `0xCA271789` u8 | terrain grid rows, columns | 5, 15 | 6, 18 | 11, 22 | 11, 5 |
| `0xCA271800` f32[] | terrain grid (rows × columns) | absent | absent | absent | 72 values (rejected) |
| `0xCA271782` f32[] | not read | | | | |

The four per-stage lists are read only if all four exist; they must hold at least `stage count` entries
(*[choice]* missing entries read as 0). Shipped airport exemplar `0xCA270E9D` also has `0xAA60E745` (`0, 100, 100,
0`), which no code reads.

Other constants: maximum funding 1.2 (`0x00B0D148`, `0x00B0D454`); growth threshold 0.96 (`0x00AB2764`);
condition age 90 days.

## 7. Persistence *[compat]*

| Field | Encoding |
|---|---|
| subclass version | u16, **2** |
| port version | u16, **1** |
| tuning instance | u32 |
| zone type | u16 (11 / 12) |
| port count n | u16 |
| n port lots | `stage` u32; port type index u16 (load order); x s16; z s16 |

`Read` (`0x007010B0`, then `0x006EAA60`): a subclass version other than 2, or a port version other than 1, returns
false. The instance and zone type are read, the tuning is loaded if not yet (with that instance), and each port lot
is **appended** with `needsLookup = 1`, efficiency 1.0, usage 0, `type := types[index]` (**no bounds check**;
*[choice]* reject the record when the index is out of range). `Write` always returns true.

Corpus: all 421 saves hold the empty forms, 12 bytes each: airport `02 00 01 00 9D 0E 27 CA 0B 00 00 00`, seaport
`02 00 01 00 92 AD 29 EA 0C 00 00 00`. No save has a port.

Example: one small airport at stage 1 on (40, 52): `02 00 01 00 9D 0E 27 CA 0B 00 01 00 | 01 00 00 00 00 00 28 00
34 00`.

## 8. Interface slot tables *[compat]*

### 8.1 `cISC4AirportDeveloper` (`0x00AB0714`) and `cISC4SeaportDeveloper` (`0x00AB2E10`)

| Slot | Offset | Method | Airport | Seaport | Pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x0068B1A0` | `0x00700EE0` | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x0068B1D0` | same | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x0068B1E0` | same | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x0068B010` | `0x00701E60` | 0 | AL |
| 4 | `+0x10` | `Shutdown` | `0x0068B110` | `0x00700E90` | 0 | AL |
| 5 | `+0x14` | `GetAvgEfficiency(SC4Percentage* out)` | `0x00700F40` | same | 4 | EAX = out |
| 6 | `+0x18` | `GetSimulatorType` | `0x0068AFE0` | `0x00701970` | 0 | EAX |
| 7 | `+0x1C` | `GetLeastEfficientPort(pct&, point&)` | `0x0068B260` | same | 8 | AL |
| 8 | `+0x20` | `GetAllPortLots(vector&)` | `0x0068B000` | same | 4 | none |
| 9 | `+0x24` | `GetNumPorts(sizeFlags, stageFlags)` | `0x0068AFF0` | same | 8 | EAX |
| 10 | `+0x28` | `AsISC4PortDeveloper` | `0x005FD7E0` | same | 0 | EAX (+4) |
| 11 | `+0x2C` | `GetNumAirports(size)` / `GetNumSeaports()` | `0x0068B540` | `0x007019E0` | 4 / **0** | EAX |
| 12 | `+0x30` | `Get…UsageValues(lot, &usage, &cap)` | `0x00700FE0` | same | 12 | AL |
| 13 | `+0x34` | `GetNum…Trips` | `0x00701800` | same | 0 | EAX |
| 14 | `+0x38` | `GetMaintenanceCost(lot)` | `0x00701830` | same | 4 | EDX:EAX |
| 15 | `+0x3C` | `CityHasSea` | | `0x00701090` | 0 | AL |
| 16 | `+0x40` | `GetCellSeaAccess(x, z)` | | `0x00701780` | 8 | AL |

### 8.2 Message target and serializable

Message target: 0–2 thunks, 3 `DoMessage` (pop 4, AL true). Serializable: 0–2 thunks, 3 `Write` `0x0068B3E0`
(pop 4, AL true), 4 `Read` `0x007010B0` (pop 4, AL), 5 `GetGZCLSID`.

### 8.3 The port sub-object vtable (internal; +4)

Slots 0–9 are thunks to the +0 slots 0–9 (a `cISC4PortDeveloper`). Then: 10 deleting destructor, 11 final
release, 12 `GetTripsForPort(p)` (airport `0x0068B250`, seaport `0x00700F60`; pop 4, EAX), 13 `OnStageChange(p)`
(airport no-op `0x00648F10`, seaport `0x00701360`; pop 4), 14 `OnNewLotCreated(p)` (airport no-op, seaport
`0x00701370`; pop 4), 15 `SendAdvisorUpgradeMessage(p)` (airport `0x0068B400`, seaport `0x00700F20`; pop 4), 16
`DoMonthlyTick` (`0x0068B630`, `0x00701F70`), 17 `UpdateBudget` (`0x00700FA0`), 18 `AllocatedFunding`
(`0x0068B270`, EDX:EAX). Plugins cannot reach these through an IID other than via `AsISC4PortDeveloper`'s object.

## 9. Who reaches in

| Caller | Use |
|---|---|
| City | `Read`/`Write` (before `Init`), `Init`, `Shutdown`; slots 76/77 hand them out |
| Demand simulator (demand.md §8) | `GetNumAirportTrips`, `GetNumSeaportTrips` |
| Advisors (advisors.md §7.3, §7.7) | `GetAvgEfficiency`, `GetAllPortLots`, `GetNumPorts`, `AsISC4PortDeveloper`, `GetNumSeaportTrips`, `CityHasSea`; the `ga_airport_efficiency`, `ga_seaport_efficiency`, `g_airport_count`, `g_seaport_count`, `g_seaport_volume` and `g_city_has_sea` variables |
| Advisor scripts | `event_feedback()` posts the growth messages |

## 10. Quirks (reproduced)

1. One agent name for both developers (§3).
2. An upgrade is free when the city cannot pay (§4.5).
3. Growth upgrades every ready port, not only the advised one; the advice is offered for one port per month.
4. The seaport cannot grow (no advice) and its terrain grid is rejected (§4.8).
5. "Efficiency" is spare capacity; a port using ≥ 75 % of its funded capacity whose lot has not changed state for
   more than 90 days is set to condition 2 (§4.3), and back to 0 otherwise.
6. `GetLeastEfficientPort` hangs with two or more ports (§4.7; FreeCity4 deviates by *[choice]*).
7. `GetNumPorts` size bits are `1 << sizeClass`; `GetNumSeaports` takes no argument (SDK errors).
8. A failed rebuild in a swap loses the port's lot; the port lot stays with a null lot *[inferred]*.
9. `Read` appends and does not bounds-check the type index.

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (PD1–PD4); all answered (PD3 by re-answers19, §4.3).

## 12. Conformance tests

1. Empty records: a fresh airport writes `02 00 01 00 9D 0E 27 CA 0B 00 00 00`; a seaport `02 00 01 00 92 AD 29 EA
   0C 00 00 00`. Version 1 or 3 is rejected.
2. Round trip of the §7 example: one port at stage 1, type 0, location (40, 52); `GetNumAirports(0) = 1`,
   `GetNumPorts(0x1F, 2) = 1`, `GetNumPorts(2, 0x1F) = 1`, `GetNumPorts(1, 0x1F) = 0`.
3. Adoption: inserting a building of group `0x1508` on a phase-1 small-airport lot adds a port at stage 0 and sets
   line 1's full expenses to 700; a phase-2 lot or a preview occupant adds nothing.
4. Month, one small airport, `fr = 1`, offices 4000 + 1200 in its only subnetwork: trips 5200, efficiency 0.48,
   usage ratio 0.52, condition 0, no advisor event; history value 48.
5. Two airports (capacities 10000 and 53600) in one subnetwork with 31800 office jobs: trips 5000 and 26800.
6. Trips 9700 on a 10000 small airport at stage 0, state 3: event `0xEA394B74` once; efficiency 0.03, condition 2
   after 91 days without a state change.
7. Then posting `0xEA394B74`: the port is rebuilt with `0xCA2436C1`, stage 1, 5700 withdrawn (and still upgraded with
   funds below 5700), full expenses 800; the old lot's state-5 message does not remove the port.
8. A port lot destroyed (state 5, not swapping): the port is removed, its rectangle dezoned, full expenses drop, the
   demand effect at its usage ratio is removed.
9. Seaport map on a 4 × 3 map with water cells (0,0), (1,0), (1,1) and (3,2): (0,0), (1,0) and (1,1) are `0x83`
   (one body touching the z = 0 row and the x = 0 column), (3,2) is `0x8C` (the z = 2 row and the x = 3 column),
   every other cell 0; `CityHasSea` true; a seaport whose lot covers only (2,1) has no access.
10. With no ports: `GetAvgEfficiency` = 1.0, nothing recorded, `GetLeastEfficientPort` false.
