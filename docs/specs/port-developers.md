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

**`Init` and `Shutdown` in full** (PD10) *[compat]*. Both are guarded by `initialized` (`+0x0D`): a second `Init`
returns true at once (subclass `0x0068B013`, base `0x006EA956`), and `Shutdown` without `Init` does nothing.
- `Init` (subclass, then base `0x006EA950`): `initialized := 1`; load the tuning if not loaded (§6); `AddAgent` as
  above, **without** removing an earlier registration first (there is none: the flag guards it, and `Shutdown`
  removes the agent); hold the city, zone manager, lot manager, budget simulator and message server (each only if
  not held); subscribe to `0xE8BDA2E7`, `0x26D31EC1`, `0x99EF1142`. The subclass then gets or creates the
  department (§4.2) and subscribes to its growth messages (§3 table; the seaport also to `0x6A3B0AFD`).
- `Shutdown` (subclass: unsubscribe the growth messages, base, then release the department; base `0x006E9F60`):
  `initialized := 0`; unsubscribe the three messages and release the message server; **delete every port lot**
  (its lot pointer is resolved first when `needsLookup` is set, then the port lot's destructor releases it) and
  **empty the port list** (`0x006E9FB2`–`0x006EA03D`); delete every port type and empty the type vector
  (`0x006EA040`–`0x006EA07D`); release the budget simulator, lot manager, zone manager and city; remove the agent
  (`cISC4City` slot 50, then simulator slot 20, with target object + 8). `tuningLoaded` (`+0x0C`), `typeCount`
  (`+0x18`) and the instance list (`+0x54`) are **not** reset *[quirk]*: an `Init` or `Read` after a `Shutdown` on
  the same object would not reload the tuning and would find no port types. The city creates a new developer for
  every city, so the shipped game never re-uses one; *[choice]* FreeCity4 may clear `tuningLoaded` with the types.

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
- **`PortUsageValues` in the month** (PD8) is the internal `0x006E9770` (called at `0x006E9A7C`): find the port whose
  lot pointer is `lot` (resolving each port on the way), `usage := GetTripsForPort(p)`, `cap :=` the current stage's
  capacity, then `usage := min(usage, cap)` (unsigned). **No funding scaling**: the month's usage ratio is
  `float(min(trips, capacity)) / float(capacity)` (each operand converted from u32). The funded cap belongs only to
  the public query (slot 12, §4.7).
- **A port with a null lot** (PD7). The month reads `p.lot` and calls `GetState` on it with no null test
  (`0x006E9A2A`–`0x006E9A31`), so a port whose lot is null (after a failed swap, §4.5) makes the original fault at
  its first month. The removal lookup (`0x006E9400`, which calls `GetLocation` on each port's lot, `0x006E947A`) and
  the seaport tick fault the same way. *[choice]* FreeCity4 skips such a port in the month and in every walk over
  the ports, and does not stop.
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

   "If any" is over **ports** (PD9): the event's flag is set for each listed port whose lot has no access
   (`0x00701FDE`), before and whatever the prop switch does, so a port without access raises the event even when
   it has no `0xC98204B8` prop. Ports with access never raise it. Every listed port is tested, at any stage, and
   the test runs only in a month that follows a set `recheckAccess` (a new seaport, or the end of a terrain
   operation, §4.8).

`GetNumAirportTrips()` / `GetNumSeaportTrips()` = Σ `GetTripsForPort` over the ports (demand.md §8).

#### 4.4.1 Seaport prop switch: native boundary (PD12)

For step 3, the actual call is `SetLotPropsState(lot, 0xC98204B8, 1)` (`0x00701FD5–0x00701FEB`).
It uses the **occupant manager's spatial list**, with the lot's rectangle and a custom occupant filter.
It does not enumerate the lot configuration, the lot's stored objects, the lot manager's occupants, or either
prop-manager `GetLotProps` overload. The following slot numbers are zero-based; all calls use Windows x86
`__thiscall` (interface pointer in ECX, arguments on the stack, callee removes the stated argument bytes).

| Receiver | Slot / byte offset | Arguments in declaration order | Result / native evidence |
|---|---|---|---|
| `cISC4Lot` | 14 / `+0x38` | output integer rectangle reference | Writes inclusive cell bounds `(xmin,zmin,xmax,zmax)`; `0x006BF320–0x006BF341`; call `0x007011A1`. |
| `cISC4City` | 96 / `+0x180` | signed cell x, signed cell z, output float x reference, output float z reference | `CellCornerToPosition`, four words / 16 bytes; each integer coordinate is multiplied by its city's float cell size and stored as float; `0x00459410–0x00459432`. Called for `(xmin,zmin)` and `(xmax+1,zmax+1)` at `0x007011BD/0x007011DE`. |
| `cISC4OccupantManager` | 40 / `+0xA0` | list reference, pointer to minimum integer cell pair, pointer to maximum integer cell pair, filter pointer, maximum count | Five words / 20 bytes; boolean in AL. Original inclusive cell bounds, custom filter, count `0xFFFFFFFF`; `0x00701253–0x00701279`. Native entry `0x0049E250`, vtable `0x00A8F7F0`. |
| `cISC4OccupantManager` | 48 / `+0xC0` | list reference | Releases every listed occupant reference and frees its list nodes; retains the sentinel; `0x0049EF30–0x0049EF5A`. Call `0x00701B46`; no usable count result. |
| listed `cISC4Occupant` | 0 / `+0x00` | IID `0xE9793A65`, output prop pointer reference | `QueryInterface` for `cISC4Prop`, two words / 8 bytes. The caller initializes the output to null and tests that pointer, **not** AL (`0x00701AF0–0x00701B0D`). |
| acquired `cISC4Prop` | 11 / `+0x2C` | state word, here 1 | `SetState`, one 32-bit word / 4 bytes, **void**, no success/count return. Call `0x00701B21–0x00701B26`; ordinary prop entry `0x005ED290`, vtable `0x00AA4900`, return at `0x005ED3F1`. |

These identities are established by the native bodies and PE vtable words, rather than SDK method order.
In particular, `cSC4PropOccupant` constructs its prop interface at object `+0`, occupant interface at `+8`
and property holder at `+0x10` (`0x005EE06D–0x005EE087`). The occupant's QueryInterface thunk subtracts 8
(`0x005EE100`); the prop QueryInterface accepts `0xE9793A65`, returns the primary interface and AddRefs it
(`0x005EC960–0x005EC978`). Do not call the occupant's own slot 11 as though it were the prop interface.

**Selection.** `GetLotProps` (`0x00701190–0x00701290`) constructs a reference-counted filter whose native
vtable is `0x00AB2D34`; slot 3 points to `0x007010E0`. For each candidate, that filter tests in this order:

1. Occupant slot 7 (`+0x1C`) must return type `0xA823821E` (prop). The ordinary prop constructor installs
   this type at `0x005EE0C8`; the native getter returns the occupant's type word (`0x006BDD80`).
2. Occupant slot 8 (`+0x20`) supplies a three-float position. Its x and z must lie **inside or on** the
   world rectangle from the two city-corner conversions. Height is ignored. The four comparisons are single
   precision memory operands; lower bounds reject less-than/unordered, upper bounds reject greater-than/unordered
   (`0x0070111B–0x00701151`). Both upper bounds are inclusive; NaN coordinates fail. For ordinary props the
   reported x/z are bounding-box midpoints, not necessarily a placement anchor (`0x005ED1F0–0x005ED219`).
3. With the supplied property ID nonzero, occupant slot 6 (`+0x18`) returns its property holder; call that
   holder's slot 3 (`+0x0C`), `HasProperty(propertyID)`, and accept only true in AL
   (`0x00701153–0x0070116E`). A zero property ID would omit this test (`0x00701158`).

The property test is **presence**, not a Boolean/value read: a present property whose value is false, zero,
or an empty array still passes. It is performed on the **occupant holder**. For an ordinary prop, occupant
slot 6 points to `0x006C2040`, returning the holder at occupant `+8` (prop object `+0x10`); that holder's
slot 3 points to `0x005FEC20`. The latter first searches instance properties, then, if absent and an exemplar
is attached, calls the exemplar's property holder (`0x005FEC20–0x005FEC8A`). Thus an inherited exemplar-only
property qualifies, as does an instance-only property. The seaport helper never fetches an exemplar directly.

The manager applies its own spatial query as well as this filter. Slot 40 clamps the integer bounds to its
city-cell range and converts them to a world box, adding float `0.1` to its x/z minima and subtracting it
from its maxima (`0x0049E261–0x0049E353`, constant `0x00A8F768 = 0x3DCCCCCD`), with y bounds
`-FLT_MAX,+FLT_MAX`, then calls slot 36 / `+0x90` (`0x004A0670`). That query also requires bounding-box
intersection. Accordingly, passing the filter alone does not guarantee inclusion; for example, a zero-width
prop exactly on a lot corner can miss the manager's inset box. This is a spatial test, not a lot-ID membership
test: a qualifying nearby prop whose reported position lies in the rectangle can be selected regardless of
its nominal owning lot.

**Order, references and lifetime.** The helper creates an empty native list with one pointer to a 12-byte
sentinel; each 12-byte node contains next, previous and occupant pointers at `+0,+4,+8`
(`0x00701AB6–0x00701AD5`). It passes that list by reference, not a flat pointer array and not a list of props.
The manager appends accepted entries and AddRefs each listed occupant before appending
(`0x004A07F8–0x004A0809`, `0x004A093E–0x004A09AF`). The filter is released immediately after enumeration
(`0x0070127F–0x00701283`), before switching any state; the list's occupant references remain held.

The switch walks the finished list from sentinel.next to sentinel, following next pointers. It preserves
the provider's order and multiplicity; it does not sort, deduplicate, stop at the first prop, or re-enumerate
after a state change (`0x00701ADA–0x00701B36`). Native manager order is ascending outer grid index, then
ascending inner grid index, then the existing per-cell forward-list order; entries append at the tail.
In the single-grid-cell path, there is no identity deduplication. In the multi-grid-cell path, the manager
suppresses repeat pointers for nodes with tag bit 0 clear, using `0x004A03B0`; tag-bit-0-set nodes bypass
that check (`0x004A0883–0x004A08A5`). There is no second deduplication in the seaport helper.

For each entry: initialize the prop output to null; QueryInterface; if nonnull, AddRef that pointer, release
the QueryInterface output's reference, call prop slot 11 with 1, and release the remaining prop reference
(`0x00701AF0–0x00701B2D`). This is one acquired prop reference held across SetState, with a transient extra
AddRef/Release pair. Null output skips only that entry; later entries are still processed. A normal successful
QueryInterface already AddRefs its output. Finally manager slot 48 releases every list occupant, including
entries that yielded no prop interface, and clears/frees all nodes. The subsequent local clear is empty;
the list destructor frees the sentinel through the same native allocator
(`0x00701B39–0x00701BAB`, `0x00413E60–0x00413ED1`). Never release both through slot 48 and again per item.
State changes must not invalidate iteration: the list and acquired interfaces stay alive until their stated
release points, even if a state-change callback changes occupant-manager membership.

**Returns and failure.** Both the native lot-switch helper and SetState have **no defined count or success
result**. SetState reads a full 32-bit argument, compares it with its byte-sized current state, and, if
different, stores the low byte; even an already-1 prop still receives the call, whose body then exits early
(`0x005ED294–0x005ED2B1`, `0x005ED3EC–0x005ED3F1`). Incidental EAX after SetState or allocator cleanup is
not a count. The monthly caller ignores the helper's return registers (`0x00701FEB–0x00701FF0`).
If the replacement host exposes a numeric diagnostic result, define it as the **number of SetState calls
issued**, including duplicate entries and already-1 props, excluding null prop-interface outputs
*[choice: adapter instrumentation]*. It is not the number of actual state transitions or a native return
value; it must not control the advisor event. The event flag was set before enumeration and remains set
for an inaccessible port even when this diagnostic count is zero.

The manager's enumeration Boolean is ignored: any entries it supplied are processed, then released. An
uninitialized native manager returns false without appending (`0x0049E256–0x0049E25B/0x0049E367`), so the
initially empty list stays empty. QueryInterface's Boolean is also ignored; output nullness governs the switch.
A false result with nonnull output is therefore still used; true with null output is skipped. Null lot, city,
occupant-manager pointer, listed occupant, or property holder is **not** a supported empty-result case: these
are dereferenced without a guard (`0x00701196/0x007011A4/0x00701267`, `0x00701AF0`, `0x00701164`).
Filter allocation failure likewise reaches an unchecked dereference (`0x0070128D → 0x0070121A`). Missing
native providers are precondition failures, not a native return of 0; a defensive host's unavailable-provider
fallback must be identified separately from native conformance. The native helper also assumes its list
allocator exists (`0x00701A67–0x00701AB8`).

**Independent boundary vectors (constructed expectations; not new game-run results).** Use inclusive lot
cells `(2,3)–(3,4)` and float cell sizes `(16,16)`, giving filter world bounds x `[32,64]`, z `[48,80]`.
Unless stated otherwise, candidates have prop type `0xA823821E`, position `(40,0,56)`, a bounding box
intersecting the manager's inset box, property present, and a successful prop QueryInterface. `A:1` denotes
one SetState(1) call on A; C is the adapter diagnostic count defined above. Always release all references
and list storage, even for C = 0. An inaccessible port still raises the month's advisor event in all valid
zero-count cases.

| Vector | Independent input / provider behavior | Expected switch trace / result |
|---|---|---|
| PD12.1 zero | Initialized manager returns an empty list. | No prop query or switch; C = 0. |
| PD12.2 multiple/order | Candidate order B, A, C; all pass, A already has state 1. | `B:1, A:1, C:1`; C = 3; A's setter can exit without a state transition. |
| PD12.3 property presence | A has instance property value 0; B has only exemplar property value false; C has neither; D has instance property with an empty array value. | `A:1, B:1, D:1`; C = 3. No property value is fetched. |
| PD12.4 type/spatial | In order: A wrong type; B at x = 31; C at x = 64 with a box extending inside; D at x = 65; E at z = 80 with a box extending inside; F has NaN x. | `C:1, E:1`; C = 2. Wrong type fails before position/property queries; failed position fails before HasProperty. |
| PD12.5 ownership | A passes spatial/property tests but is nominally owned by another lot; B belongs to this lot but is outside its world rectangle. | `A:1`; C = 1. Ownership labels do not replace the spatial tests. |
| PD12.6 prop interface | Finished list `[A,B,C]`; A outputs null, B outputs nonnull with AL false, C outputs nonnull with AL true. | `B:1, C:1`; C = 2; all three occupant references released. Also true/null on A still skips A. |
| PD12.7 duplicate output | Provider supplies finished list `[A,A,B]`, one owned occupant reference per node. | `A:1, A:1, B:1`; C = 3; separate balanced prop acquisitions and occupant releases for all three nodes. |
| PD12.8 native grid duplicate | Two visited manager grid cells contain the same eligible A pointer with tag bit 0 clear. | Native manager lists A once; `A:1`; C = 1. If both nodes instead have tag bit 0 set, both bypass dedup and give `A:1, A:1`; C = 2. |
| PD12.9 enumeration failure | Nonnull, uninitialized native manager returns false with no entries. | Empty trace; C = 0; filter and sentinel still released. For an injected false result with `[B,A]` supplied, process `B:1, A:1`; C = 2. |
| PD12.10 missing provider | Occupant-manager pointer null (or city/lot pointer null); no enumeration result exists. | Native precondition failure at the unchecked call/dereference; no defined C, success or empty-list result. A safe-host skip/0 is a separately labeled fallback. |
| PD12.11 missing holder | Otherwise eligible occupant returns null from slot 6 while property ID is nonzero. | Native precondition failure before prop QueryInterface; no defined count. A missing property in a nonnull holder instead gives a normal rejection. |
| PD12.12 snapshot lifetime | Provider finishes `[A,B]`; A's SetState removes B from manager membership. | Still `A:1, B:1`, C = 2; B's list reference remains held until list cleanup. No second spatial query. |

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
   (slot 23). If that fails the swap fails (the old lot is already gone *[quirk]*; `p.lot` is left null,
   `0x006E9385`). Otherwise `p.lot := new`,
   `new.SetLotConfiguration(newConfig)`, `new.UpdateZoneType()`, and the result is the lot developer's
   `ConstructDefaultLot(new, newConfig, 0, 1)`.

   **A missing next-stage configuration is not tested** (PD5) *[compat]*. `newConfig` is used as returned
   (`0x006E9303`), with no null test anywhere in the swap, so a rule naming a configuration the manager does not
   have still demolishes the old lot and creates the new one. `SetLotConfiguration(null)` and `UpdateZoneType()` run,
   then `ConstructDefaultLot(new, null, 0, 1)` passes the null configuration and type 0 straight to
   `StartConstruction` (lot-developer.md §6.4, §6.1): the new lot gets no configuration, building type 0, state 2 and
   condition 1, `PickUnitToJoin` runs, and with type 0 it returns **true** at once; because `animate = 1`, no
   construction occupant is made and nothing ever sets state 3. So the swap **succeeds** and step 2 runs: the stage
   advances, the upgrade cost is withdrawn, `OnStageChange` and `UpdateBudget` run. The port is left on an empty lot
   that stays in state 2 with condition 1: the month skips it (condition 1) and it can never grow again (state 2)
   *[inferred: nothing else completes a state-2 lot without a construction occupant]*. Not reachable with shipped
   data (every rule's configuration exists).
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
| `Get{Airport,Seaport}UsageValues(lot, &usage, &cap)` | the port of that lot (`0x00700FE0`: the month's internal `0x006E9770` first, §4.3): `usage := min(trips, capacity)`, `cap := capacity`; then, if line 1 exists and `full > 0` (`0x00701040`), `cap := trunc(float(capacity) / full × float(current))` (operation order exact: the u32 capacity, divided by `full` converted from int64, times `current` converted from int64 and rounded to float; `0x00701027`–`0x00701063`), and `usage := min(usage, cap)` again; AL = found. Not found: 0, 0, false |
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

**New seaport** (`OnNewLotCreated`, `0x00701370`): `UpdateTripCapacity(p)`, `recheckAccess := 1`. Then the
terrain grid, if the port type has one (PD4, PD6; the step never runs with shipped data, see the end):

1. **Load** (`0x006EA5F0`–`0x006EA721`): rows `R` := u8 `0xCA271788`, columns `C` := u8 `0xCA271789`; both must be
   non-zero and `R × C` must equal the value count of `0xCA271800` (`0x006EA676`–`0x006EA690`), else no grid. The
   values fill `R` rows of `C` floats, row-major: `grid[r][c] := v[r·C + c]`.
2. **Use test** (`0x007013D7`–`0x0070141A`): with the lot's inclusive cell rectangle `{x1, z1, x2, z2}` (lot slot 14),
   the grid is used only when `R × C = (x2 − x1) × (z2 − z1)`. Only the **product** is compared, not each side.
3. **Walk**: `z` from `z1` while `z < z2` (outer), `x` from `x1` while `x < x2` (inner): `(R × C)` cells, the last
   row and column of the lot excluded. For each `(x, z)` the value is picked by the lot's **facing** (lot slot 23;
   jump table `0x007016F8`):

   | facing | row index | column index | code |
   |---|---|---|---|
   | 0 | `x − x1` | `z − z1` | `0x00701437` |
   | 1 | `z2 − 1 − z` | `x2 − 1 − x` | `0x00701591` |
   | 2 | `x2 − 1 − x` | `z2 − 1 − z` | `0x007014E7` |
   | 3 | `z − z1` | `x − x1` | `0x00701646` |
   | other | nothing is set | | |

   So at facing 0 the grid's **rows run along x** and its columns along z; facing 2 is facing 0 turned 180°; facings
   1 and 3 are the **transpose** (rows along z) and its 180° turn. Facings 1 and 3 are therefore mirror images of 0
   and 2, not quarter turns *[quirk]*. Indexes are not bounds-checked: when `R × C` matches but the sides do not
   (for example a 5 × 11 grid on an 11 × 5 walk), rows or columns past the grid's end are read *[quirk]*.
4. **The write**: `(px, pz) := city.CellCenterToPosition(x, z)` (`cISC4City` slot 97: `((x + 0.5) × 16, (z + 0.5)
   × 16)` in metres), then `terrain.SetAltitude(trunc(px), trunc(pz), value)` (`cISTETerrain` slot 51,
   `0x007014AE`). Slot 51 takes **vertex indices** (terrain.md §6: `i = (cellsX + 1) × z + x`, no bounds check), so
   cell `(x, z)` writes vertex `(16x + 8, 16z + 8)`, sixteen times too far out, past the map for most cells *[quirk:
   a bug that never runs]*.

*[quirk]* The shipped seaport type declares 11 × 5 (55) but ships 72 values, so the load rejects the grid and none of
this runs. Correction to PD4's answer: the use test counts `(x2 − x1) × (z2 − z1)`, which for a 12 × 6 lot (the
size the 72 values suggest) is **55**, the declared size. A grid of 72 values (12 × 6) would load but would pass the use test only on a 13 × 7
lot; a grid that loads **and** applies to the shipped lot needs 11 × 5 with 55 values. Either way the write of step 4
puts the heights in the wrong place. *[choice]* FreeCity4 reproduces steps 1–3 and, in step 4, skips a vertex index
outside the terrain instead of writing out of bounds.

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

Nothing after the two version checks can reject the record in the original (PD11): the port lots are read and
appended with no check of the stream or the index, and `Read` returns true (`0x006EAA91`–`0x006EAB9D`). The tuning is
loaded (`0x006EAAC8`) before the first port lot is read, so whenever FreeCity4's *[choice]* rejects a record for its
index, the tuning stays loaded, as it would in the original. The rest of a rejected `Read` should leave the port list
unchanged *[choice]*.

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
8. A failed rebuild in a swap loses the port's lot; the port lot stays with a null lot (`0x006E9385`), and the next
   month faults on it (§4.3, PD7; FreeCity4 skips such a port by *[choice]*). A missing next-stage configuration is
   not tested: the swap "succeeds" on an empty lot stuck in state 2 (§4.5, PD5).
10. The terrain grid's facings 1 and 3 transpose it, and its write uses metre coordinates as vertex indices (§4.8).
11. `Shutdown` keeps `tuningLoaded` (§3).
9. `Read` appends and does not bounds-check the type index.

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (PD1–PD4); all answered (PD3 by re-answers19, §4.3). PD5–PD11
(section "From the city detail, port developer and prop developer cores") are answered by re-answers22: §3, §4.3,
§4.4, §4.5, §4.7, §4.8, §7.

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
11. Funding at 50 % (`fr = 0.5`), one small airport (capacity 10000) with 4000 trips: the month's usage ratio is
    0.4 (plain capacity, §4.3); `GetAirportUsageValues` gives cap 5000 and usage 4000; with 6000 trips the month's
    ratio is 0.6 and the query gives 5000, 5000.
12. A next-stage rule naming a missing configuration: posting the growth message demolishes the old lot, creates
    an empty lot in state 2 with condition 1 and no configuration, advances the stage and withdraws the upgrade cost;
    later months skip the port (§4.5, PD5).
13. Two `Init` calls register one agent; `Shutdown` empties the port list and the type vector (§3, PD10).
