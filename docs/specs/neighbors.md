# Spec: the neighbours subsystem (`cISC4NeighborsSimulator`, `cISC4NeighborConnection`, `cISC4NeighborDeal`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose. "The original returns no meaningful value" means the SDK declares a return value that the
original never sets; callers in the shipped game ignore it.

Related specs: [foundations.md](foundations.md) (streams, references, response curves §4, messages §5, random
numbers §6), [simulator.md](simulator.md) (agents, calendar), [budget.md](budget.md) (books deal totals),
[demand.md](demand.md) (connection cap relief, §8.3).

## 1. Role

A city sits in a region next to other cities. This subsystem:

1. keeps the city's **neighbour connections**: the edge cells where a road, rail, pipe, power line, etc. leaves
   the city towards a neighbour (§4);
2. keeps the city's **neighbour deals**: monthly contracts to buy or sell power, water or garbage capacity with an
   adjacent city (§5), runs their life cycle (§7) and prices them (§8);
3. publishes the city's commodity statistics and deal list to the **region** so that neighbours see them (§9);
4. reports the month's deal income and expense to the budget (§10).

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| neighbours simulator | `0xC990C0A4` | `cISC4NeighborsSimulator` (`0x0773FEF0`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |
| neighbour connection | `0x0A134F24` | `cISC4NeighborConnection` (`0x0773FEF2`), `cIGZSerializable`, `cIGZUnknown`; plus a property holder (`AsISCPropertyHolder`) and a traffic source (`AsTrafficSource`) |
| neighbour deal | `0x2A134F42` | `cISC4NeighborDeal` (`0x0773FEF1`), `cIGZSerializable`, `cIGZUnknown`; plus a property holder |

The city creates the simulator with `GZCOM()->GetClassObject(0xC990C0A4, 0x0773FEF0, &p)` and stores it in
reference slot 30 of its save record (foundations §3.6). Others reach it through
`cISC4City::GetNeighborsSimulator()`.

**List parameters** *[compat]*: every SDK method that fills an `SC4List<…>` stores **references** (each element
is AddRef'd; the caller's list releases them when cleared). The SDK types the deal lists of `ProposeDeals`,
`FindDealsAll` and both `FindDeals` as `SC4List<cISC4NeighborConnection*>`; their elements are really
`cISC4NeighborDeal*`.

## 2. Geometry

### 2.1 Cells

- City cells (tiles) are indexed `0 .. countX − 1` / `0 .. countZ − 1` (`cISC4City::CellCountX/Z`).
- A **connection cell** lies one cell **outside** the city: `x = −1` (west), `x = countX` (east), `z = −1`
  (north), or `z = countZ` (south), with the other coordinate inside. Corners are not valid connection cells.
- The **side** of a connection cell is decided in this order: `x < 0` → West; else `z < 0` → North; else
  `x < countX` → (`z ≥ countZ` → South, else interior); else East.
- The **inner cell** of a connection is the city cell next to it (x clamped to `0 .. countX − 1`, z likewise).
- **Region cells**: one region cell = 64 city cells. A regional city has a position `(px, pz)` and a size
  `(sx, sz)` in region cells (`cISC4RegionalCity::GetPosition`, `GetCitySize`).

### 2.2 Conversions (static helpers; no object state) *[compat]*

`/` and `mod` are C integer operations (truncation toward zero).

**Neighbouring region cell** of connection cell `(x, z)` for regional city `R` (default: the current city's
regional record): start from `R`'s position `(px, pz)`, then West: `(px − 1, pz + z/64)`; East:
`(px + sx, pz + z/64)`; North: `(px + x/64, pz − 1)`; South: `(px + x/64, pz + sz)`; interior: unchanged.

**Neighbour's connection cell**: with `(rx, rz)` as above and `N = region.GetCity(rx, rz)` (fails if there is
none) at position `(qx, qz)` and size `(tx, tz)`:

| Our side | Neighbour's connection cell |
|---|---|
| East | `(−1, z mod 64 + (rz − qz)·64)` |
| West | `(tx·64, z mod 64 + (rz − qz)·64)` |
| North | `(x mod 64 + (rx − qx)·64, tz·64)` |
| South | `(x mod 64 + (rx − qx)·64, −1)` |
| interior (none of the above) | `(0, 0)`, and success |

The helper (`0x006CE920`) first sets its three outputs (neighbour city, cell x, cell z) to null, 0 and 0. It then
fails (returns false) when there is no current regional record or no city at `(rx, rz)`. An **interior** cell
(NB6) leaves `(rx, rz)` at our own position `(px, pz)`. The city found there is **our own** regional city, so the
helper returns **true** with that city and the zeroed cell `(0, 0)` (the side switch has no interior case). Example:
a 1 × 1 city at region (3, 3), with the cell (10, 20), gives true, our own regional city, and `(0, 0)`.

The only in-simulator user, `FindConnectionOnOtherSide`, then asks our own regional record for a connection at
`(0, 0)`. No connection lies inside the city, so it gets null, as for a failed conversion. *[compat]* The helper
itself must return the row above, because the original regional city calls this conversion directly (§15.2).

**Valid connection cell** for regional city `R` of size `(sx, sz)`: exactly one of `x ∈ {−1, 64·sx}` and
`z ∈ {−1, 64·sz}` holds. The inverse conversion (neighbour cell → our connection cell) mirrors the table. Other
original code (the query tool, the connection info writer, the regional city) calls these helpers directly
(§15.2).

## 3. Commodities and deal types

| Deal type | Meaning | Commodity index | Base amount property | Connection types searched (§6.2) |
|---|---|---|---|---|
| `0x4773FFE0` | buy power (this city pays) | 1 | `0xEA46932C` | PowerPole (4), all must be checked |
| `0x4773FFE1` | sell power (the neighbour pays) | 1 | `0xEA46932C` | PowerPole (4) |
| `0x4773FFE2` | buy water | 2 | `0x6A469337` | Pipe (5) |
| `0x4773FFE3` | sell water | 2 | `0x6A469337` | Pipe (5) |
| `0x4773FFE4` | buy garbage disposal (this city exports trash, pays) | 3 | `0x8A46933D` | Highway (3), GroundHighway (12), Avenue (8), Road (2), DirtRoad (10): the first type with any connection, first connection |
| `0x4773FFE5` | sell garbage disposal (accept trash, paid) | 3 | `0x8A46933D` | as above |

**Even** types: this city is the buyer and pays; **odd**: the neighbour pays. Swapping the perspective of a deal
toggles the low bit (`E0 ↔ E1`, …). Connection types are the SDK's `ConnectionType` enum (None 0, Rail 1,
Road 2, Highway 3, PowerPole 4, Pipe 5, Subway 6, LightRail 7, Avenue 8, Ferry 9, DirtRoad 10, Monorail 11,
GroundHighway 12). **Transport** types are all except 0, 4 and 5 (bit mask `0x1FCE`).

## 4. Connection object (`cISC4NeighborConnection`)

### 4.1 State

| Field | Initial | Saved |
|---|---|---|
| type | 0 | yes |
| cell (x, z) — the connection cell | 0 | yes |
| otherRegionCell (x, z) | 0 | yes |
| flags byte: bit 0 = visitedThisCycle, bit 1 = **inactive** | 0 (active, unvisited) | yes |
| property holder | empty | yes |
| trip data list (traffic source) | empty | no |
| needsTrafficSync | true | no |

### 4.2 Methods (SDK order)

| Method | Behaviour |
|---|---|
| `AsISCPropertyHolder()` / `AsTrafficSource()` | the embedded holder / traffic-source object (no AddRef). |
| `GetType()` | type. |
| `GetConnectionCell(&x, &z)` | the connection cell. |
| `GetConnectionInnerCell(&x, &z)` | False if the cell is inside the city or outside on both axes (a corner). Else the inner cell (§2.1) and true. |
| `GetRegionCellOnOtherSideOfConnection(&x, &z)` | otherRegionCell. |
| `GetCityOnOtherSideOfConnection()` | `region.GetCity(otherRegionCell)`'s regional city, or null; no AddRef. |
| `Repair(R)` | recompute otherRegionCell from the cell with regional city `R` (§2.2). |
| `GetVisitedThisCycle()` / `SetVisitedThisCycle(b)` | bit 0. |
| `GetIsActive()` / `SetIsActive(b)` | `GetIsActive` = bit 1 clear; `SetIsActive(true)` clears bit 1, `(false)` sets it. Set by the utility simulators *[inferred]*. |

**Traffic volume sync** (`UpdateTrafficVolume`, a concrete method used by the traffic simulator): the first time
after creation it reads, on the matching connection on the other side (`FindConnectionOnOtherSide`), the 9-value
trip-count arrays `0xAA9EA81D + w` (w = 0..2) and adds or removes trip-data entries in its own list so that the
counts match. Details belong to the traffic spec *[summary only]*.

### 4.3 Persistence (`GetGZCLSID() = 0x0A134F24`) *[compat]*

```
u16  version = 1
u32  type
s32  cellX, cellZ
s32  otherRegionX, otherRegionZ
u8   flags
...  property holder record (ordinance spec §9.3, plain-holder part)
```

`Read` fails unless `version == 1`. The same record is stored inline in regional-city records (§9.3).

## 5. Deal object (`cISC4NeighborDeal`)

### 5.1 State

| Field | Initial | Notes |
|---|---|---|
| type | 0 | §3 |
| regionCellOtherSide (x, z) | 0 | the neighbour |
| regionCellCitySide (x, z) | 0 | "our" city in this deal's perspective |
| amount (s32) | 0 | commodity units per month |
| incAmount, incAmount2, lastAmount, lastIncAmount, amountUsed (s32) | 0 | maintained by the utility simulators *[inferred]*; only incAmount, lastAmount and amountUsed are saved |
| cost (s64) | 0 | monthly price (§8.1) |
| startDate, expirationDate, currentDate, suspensionDate | the PC's local date at construction (see below); *[choice]* 0 | |
| flags | `0x81` (Proposed + Silent) | §5.2 |
| property holder | empty | |

**Initial dates** (NB1). The four dates are date objects whose default constructor (`0x0091307C`) reads the
**wall clock**. It calls the C runtime's `time` and `localtime`, then stores the Julian day of that local calendar
date (`Jday(month, day, year)`, `0x00912B5F`; 2000-01-01 = 2451545). So a deal constructed on 2026-10-14 starts
with all four dates = 2461328. There is no fixed default date.

Where the construction date survives:

- `suspensionDate` of every deal that was never suspended. The region writes it into the version-5 deal record
  (§5.6). Nothing reads it before the first strike overwrites it (§7.3), so the simulation does not see it.
- All four dates of a deal from `ProposeDeal` that nobody dated afterwards. `ProposeDeals` dates its own proposals
  (§7.1).
- `suspensionDate` of a deal read from a version ≤ 4 record (§5.6 reads it only for version > 4). The regional
  city reads such a record into a new deal object, so the date is the day of that read. `Init` step 7 then shifts
  it with the other three dates (§14.1). It matters only if the record also has Grace or Probation set: the grace
  arithmetic of §7.3 then starts from a wall-clock date.

*[choice]* FreeCity4 initializes the four dates to **0**, deterministically. The original value depends on the
PC's clock and cannot be reproduced. A saved 0 is harmless to the original game, for the reasons above.

### 5.2 Flags *[compat]*

| Bit | Name | Meaning |
|---|---|---|
| `0x001` | Proposed | offered, not yet in force |
| `0x002` | Active | in force |
| `0x004` | Cancelled | ended by the player or by the system |
| `0x008` | NoConnection | no suitable connection to the other city |
| `0x010` | CannotAfford | the payer's budget is below the cost |
| `0x020` | NoLandfill | not enough disposal capacity for a garbage-sell deal; the plumbing and power simulators also set it on export deals they could not fully deliver (plumbing spec, power §6.3) |
| `0x040` | Expired | term ended; renewal pending |
| `0x080` | Silent | no status messages; not simulated (deals stored in the region, clones, new objects) |
| `0x100` | Grace | first failure; further failure flags are ignored |
| `0x200` | Probation | after the grace period; a failure now ends the deal |

"Failure flags" are `0x38`. The totals ignore deals with any of `0x3D` (§10.1). **Pricing does not** (NB9):
`GetDealCost` and `GetDealMaxAmount` take `existing` from the first `FindDeals` match, whatever its flags (§8.1).
`FindDeals` (both overloads, `0x006D0760` and `0x006D0840`) matches on type, and on the other-side city where one
is given, and on nothing else.

### 5.3 Status reports *[compat]*

A **status report** is message `0xEA418D8F` with data1 = status and IGZUnknown = the deal. It is not sent for
Silent deals. It is **posted**, except that `SetAmount` **sends** it synchronously.

| data1 | Meaning |
|---|---|
| 0 | the deal changed (flags, amount or cost) |
| 1 | activated |
| 2 | cancelled and removed |
| 3 / 4 / 5 | removed for NoConnection / CannotAfford / NoLandfill |
| 6 | expired (renewal pending) |
| 7 | renewed |
| 8 | (posted by the simulator itself, no deal) retry a busy deal simulation |

### 5.4 Methods (SDK order)

| Method | Behaviour |
|---|---|
| `Set/GetRegionCellOnCitySideOfConnection`, `Set/GetRegionCellOnOtherSideOfConnection` | plain accessors. |
| `GetCityOnOtherSideOfConnection()` | `region.GetCity(regionCellOtherSide)`'s regional city or null; no AddRef. |
| `GetType()` / `SetType(t)` | plain. |
| `GetAmount()` | amount. |
| `SetAmount(a)` | Only if the current city exists, has a neighbours simulator, and flags have neither Grace nor Probation: `amount := a`; report status 0 (**sent**); `UpdateCost()`. Otherwise ignored *[compat]*. |
| `Get/SetLastAmount`, `Get/SetLastIncAmount`, `Get/SetIncAmount`, `Get/SetIncAmount2`, `Get/SetAmountUsed` | plain. |
| `GetCost()` | cost. |
| `UpdateCost()` | Unless Grace or Probation: `cost := trunc_s64(neighbours.GetDealCost(type, amount, GetCityOnOtherSide()))` through the **current city's** neighbours simulator; if that is 0, `cost := 1`; report status 0 (posted). The amount is passed as a double. *Without a current city or neighbours simulator* (NB5), the original crashes: it checks neither the global city pointer (`0x006CF9D4`) nor `GetNeighborsSimulator()`'s result (`0x006CF9E8`). `SetAmount` checks both before it calls `UpdateCost`, so only direct callers can reach this. *[choice]* FreeCity4 does nothing in that case: the cost stays as it was and no report is sent. |
| `Get/SetStartDate`, `Get/SetExpirationDate`, `Get/SetCurrentDate`, `Get/SetSuspensionDate` | copy the Julian day to / from the caller's `cIGZDate`. |
| `Copy(src)` | copies type, flags, amount, amountUsed, cost, incAmount, lastAmount, both region cells and the four dates from `src` (through its interface). |
| `UpdateState()` | §7.3 step 1. |
| `Cancel()` | `SetFlags(Cancelled, true)`, `SetFlags(Active, false)`, `SetAmount(0)`. |
| `Activate()` | If Proposed is set and none of `0x38`, it makes three calls **through `SetFlags`** (its own virtual slot 37; `0x006CE640`): `SetFlags(Active, true)`, `SetFlags(Proposed, false)`, `SetFlags(Cancelled, false)`, in that order, and returns true. Otherwise it returns false. Each call that changes the flags of a non-Silent deal posts a status 0 report. A Proposed, non-Cancelled, non-Silent deal therefore posts **two** status-0 reports (NB2). Cancelled (`0x04`) is not in `0x38`, so `Activate` also activates a cancelled proposal and clears Cancelled. |
| `GetFlags()` | flags. |
| `SetFlags(f, on)` | If `on`: if Grace is set, drop `0x38` from `f`; `flags |= f`. Else `flags &= ~f`. If the flags changed and the deal is not Silent, report status 0 (posted). Returns true. |
| `CheckFlagsAnd(f)` | all bits of `f` set. |
| `CheckFlagsOr(f)` | any bit of `f` set. |
| `Clone(&out, silent)` | §5.5. Returns true. |

### 5.5 `Clone(&out, silent)` *[compat]*

Make a new deal, `Copy` this one into it, then `SetFlags(Silent, silent)`. Look up the regional cities at both
region cells. If either is missing, `SetFlags(Cancelled, true)` on the clone. Otherwise, if the city-side city is
**not** the current city's regional record, **swap** the two region cells and toggle the type between buy and
sell (§3), so that the clone is seen from the current city. `out :=` the clone (AddRef'd).

### 5.6 Persistence (`GetGZCLSID() = 0x2A134F42`) *[compat]*

Deals are **not** in the neighbours simulator's record; they live in regional-city records (§9.3). Format:

```
u16  version = 5
u32  type
s32  otherX, otherZ                 (regionCellOtherSide)
s32  amount
s32  amountUsed
s64  cost
s32  flags
s32  cityX, cityZ                   (regionCellCitySide)
s32  incAmount
s32  lastAmount
s32  amountUsed                     (written twice; the second value is the one kept)
u32  currentDate, startDate, expirationDate     (Julian days)
...  property holder record
u32  suspensionDate                 (present when version ≥ 5)
```

`Read` accepts any version (no check) and reads `suspensionDate` only for `version > 4`.

## 6. The neighbours simulator

### 6.1 State

| Field | Initial | Saved |
|---|---|---|
| initialized, ready (ConnectionsReady seen) | false | no |
| connections: ordered list (append order) | empty | yes (§13) |
| deals: ordered list | empty | no (§9) |
| tuning exemplar, deal-price curve (§8.3) | none | no |
| dealsDirty | **true** | no |
| statsDirty, uiDisabled, busy | false | no |

### 6.2 Methods (SDK order)

| Method | Behaviour |
|---|---|
| `Init()` / `Shutdown()` | §14. |
| `RequestCreateConnection(type, x, z, flag)` | If `HasConnection(type, x, z)` → false. Otherwise show a yes/no confirmation dialog (title LTEXT `0x6A231EAA/0xAA8E151B`; text: `sprintf(LTEXT 0x0A8E152D, neighbourName)`, where the name is the established neighbour city's name or LTEXT `0x0A8E15F9`) and return the answer. `flag` is unused. *[choice]* FreeCity4's core returns "needs confirmation" and lets the shim show the dialog. |
| `CreateConnection(type, x, z, flag)` | §6.3. The original returns no meaningful value; *[choice]* return the new connection (no AddRef) or null. |
| `DeleteConnection(type, x, z)` | `FindConnection`; if found, delete it (§6.4) and remove it from the list. |
| `DeleteOccupantConnection(occ)` | §6.5. |
| `DeleteAllConnectionsOfType(t)` | delete (§6.4) every connection of type `t`, then remove them all from the list. |
| `HasConnection(t, x, z)` | `FindConnection(t, x, z) ≠ null`. |
| `FindConnection(t, x, z)` | the first connection with that type and cell; no AddRef. |
| `FindCityOnOtherSideOfConnection(x, z)` | the regional city at the neighbouring region cell of `(x, z)` (§2.2), or null. |
| `FindConnectionOnOtherSide(t, x, z)` | convert to the neighbour's connection cell (§2.2); `neighbour.FindConnection(t, nx, nz)` (`cISC4RegionalCity`), or null. |
| `FindConnectionOnOtherSide(conn)` | the same with `conn`'s type and cell. |
| `FindUnvisitedConnection(t, city)` | the first connection of type `t`, not visited, whose other-side city is `city`. |
| `GetAllConnections(list)` | clear `list`, append all connections. |
| `GetConnectionsOfType(t, list)` / `GetConnectionsOfTypeToCity(t, city, list)` / `GetConnectionsToCity(city, list)` | clear and append the matching connections in list order. |
| `GetAdjacentCities(list, includeUnestablished)` | §9.1. |
| `GetConnectedCities(list, includeUnestablished)` | Clear `list`. For each connection of a **transport** type whose other-side city exists (and is established unless the flag is set), append that city if not yet present. |
| `GetAllConnectedCities(list, includeUnestablished)` | Breadth-first over the region: start with the current regional city; for each city in the growing list take its connections (ours for the current city, `cISC4RegionalCity::GetAllConnections` for others) and append every other-side city of a transport connection not yet present. Finally remove the current city; if the flag is false, remove unestablished cities. |
| `FindConnectionSuitableForDeal(type, city)` | §6.6. |
| `ProposeDeal(type, city, &out)` | §7.1. |
| `ProposeDeals(list, addToSimulator)` | §7.1. Returns true. |
| `AddDeal(deal)` | `deal.SetFlags(Silent, false)`; append (AddRef); push to the region (§9.2, both flags). Returns true. |
| `SetUIDisableFlag(b)` | `uiDisabled := b` (§7.3). |
| `FindDealsAll(list)` | `list :=` a copy of the deal list. |
| `FindDeals(type, list)` / `FindDeals(type, city, list)` | clear and append deals with that type (and other-side city). |
| `MarkDealsDirty()` | `dealsDirty := true`. |
| `GetTotalMonthlyNeighborDealIncome()` / `…Expense()` | §10.1. |
| `UpdateCommodityStats(city)` | §8.2. |
| `GetDealCost(type, amount, city)` | §8.1. |
| `GetDealMaxAmount(type, city)` | §8.1. |
| `GetTuningExemplar()` | the tuning exemplar (§12), no AddRef. |

**Return values at the interface** (NB17):

- `RequestCreateConnection` (`0x006D15E0`) returns a real `bool`: true only when the dialog was confirmed; false
  when the connection already exists or the player declined. The shim must return exactly that.
- `CreateConnection` (`0x006D4860`, `ret 0x10`) leaves whatever is in `EAX`. On the early return that is
  `HasConnection`'s result (true). Otherwise it is the return value of the final `Release` of the posted message.
  Neither is a pointer, and shipped callers ignore it.

The core's internal types (a three-way `ConnectionRequest`, and the new connection or null) are the
implementer's choice; no original behaviour constrains them beyond the `bool` above.

### 6.3 `CreateConnection(type, x, z, flag)` *[compat]*

1. If `HasConnection(type, x, z)`, do nothing.
2. Create a connection, append it, set type, cell and otherRegionCell (§2.2). If the other-side regional city
   exists, call its `ChangeSymmetricConnection(conn, true)`.
3. Unless `type` is Ferry (9), place a **marker prop** `0x29F10000` at the centre of the inner cell, at height
   `terrain.GetAltitude(px, pz)` (+15.5 for Highway, LightRail and Monorail).

   Here `(px, pz)` is the inner cell's centre, from `city.CellCenterToPosition` (slot 97), which is the same point
   the prop is placed at (NB3; `0x006D494A`–`0x006D4A4F`). The connection cell itself is never used for the
   height.
   - The inner cell is found from the side, in this order: `x < 0` → `(x + 1, z)`; else `z < 0` → `(x, z + 1)`;
     else `x ≥ countX` → `(x − 1, z)`; else `(x, z − 1)`.
   - The +15.5 (`0x00AB1CA8`) is added in float32.

   Example: the connection Road at (−1, 5) in a 64 × 64 city puts its marker at the centre of cell (0, 5), at that
   point's terrain altitude. A Highway at the same cell puts it 15.5 m higher. Orientation by side: with
   `flag = false` West 3, North 0, East 1, South 2; with `flag = true` the opposite (1, 2, 3, 0). Give the prop the
   bool property `0xCAA46595 = true` (marks it as a connection marker); for Pipe also `0xCAA45670` and
   `0xCAA45672`; for Subway `0xCAA45670` and `0xCAA45671` (all true). Insert it through the prop manager.
4. Post NeighborConnectionCreated `0x8A0D15DA` (data1 = type, data2 = x, data3 = z).

### 6.4 Deleting a connection *[compat]*

Call the other-side city's `ChangeSymmetricConnection(conn, false)` if it exists; remove every prop with
`0xCAA46595 = true` in the inner cell; post NeighborConnectionDeleted `0xEA0D163E` (data1 = type, data2 = x,
data3 = z of the connection cell).

### 6.5 `DeleteOccupantConnection(occ)` *[compat]*

- A **network occupant** (occupant type `0xC772BF98`): get its `cISC4NetworkOccupant` and map the networks it
  carries to a connection type; if several apply the later one in this list wins: pipe → Pipe, dirt road →
  DirtRoad, road → Road, avenue → Avenue, ground highway → GroundHighway, highway → Highway, subway → Subway,
  light rail → LightRail, monorail → Monorail, rail → Rail (network type numbers 4, 11, 0, 6, 12, 2, 7, 8, 9, 1).
  No match → nothing.
- Occupant type `0x2890D4DE` (power pole *[inferred]*) → PowerPole.
- Otherwise nothing.

Then find the occupant's cell `(cx, cz)` from its position: `GetPosition` (occupant slot 8), then the city's
`PositionToCell` (slot 95), for networks and poles alike (NG17, §15.4). If `cx == 0` delete the connection `(type, −1, cz)`; else if
`cx == countX − 1` delete `(type, countX, cz)`. Independently, if `cz == 0` delete `(type, cx, −1)`; else if
`cz == countZ − 1` delete `(type, cx, countZ)`.

### 6.6 `FindConnectionSuitableForDeal(type, city)` *[compat]*

- Garbage types: the connection list is that of the first transport type in §3's order that has any connection
  to `city`; return its **first** connection without further checks.
- Power and water types: take all PowerPole (resp. Pipe) connections to `city` in order and return the first
  that qualifies: for a **sell** type, the connection itself must be active; for a **buy** type, the matching
  connection on the other side (`FindConnectionOnOtherSide(conn)`) must exist and be active.
- Any other type: null.

## 7. Deal life cycle

### 7.1 Proposals *[compat]*

**`ProposeDeal(type, city, &out)`**: create a deal (Proposed + Silent). `conn := FindConnectionSuitableForDeal(type,
city)`. If found: other-side region cell := `conn`'s; city-side region cell := the current regional city's
position; `SetType(type)`; `SetAmount(0)` (so `cost = 1`); `out :=` the deal (AddRef). Returns whether `conn` was
found.

**`ProposeDeals(list, add)`** (only when `ready`; otherwise it just returns true):

```
UpdateCommodityStats(city);  clear list
minBudget := tuning 0xEA4AE9EF (default 0);  costFactor := tuning 0xCA46931F (default 1)
for C in GetAdjacentCities(established only):
    for (buy, sell, base) in [(E0, E1, basePower), (E2, E3, baseWater), (E4, E5, baseGarbage)]:
        for (type, payer) in [(buy, our regional city), (sell, C)]:
            if FindConnectionSuitableForDeal(type, C) and FindDeals(type, C) is empty:
                if min(GetDealMaxAmount(type, C), base) ≥ base:                  # i.e. max ≥ base
                    cost := GetDealCost(type, base, C);  budget := payer.GetBudget()
                    if minBudget < budget and cost < budget and costFactor × cost < budget:
                        Propose(type, C)
if list is not empty and add: dealsDirty := true; push to the region (both flags)
```

`Propose(type, C)`: `ProposeDeal(type, C, &d)`; if `add`, `d.SetFlags(Silent, false)` and append `d` to the
deal list; `d.SetAmount(0)` (the proposal's amount is **0** *[compat]*); term in years
`y := floor(rng.RandomDoubleRangeUniform(lo, max(lo, hi) + 0.001))` with `lo = 0xAA46934B` (default 1) and
`hi = 0xEA46934F` (default 5); `currentDate := startDate := today`; `expirationDate := today + 365·y`; append `d`
to `list`. The generator is a process-wide `cRZRandom` created on first use and seeded from the wall clock
(foundations §6.2), double path. With the shipped tuning (1 and 1) `y = 1`.

**Re-entry with `add = true`** *[compat]* (NB10; `create_deal_proposition` `0x006D0520`). The order inside
`Propose` is:

1. `ProposeDeal`: `d` is still Silent, so its `SetAmount(0)` reports nothing.
2. `SetFlags(Silent, false)`, which **posts** status 0.
3. Append `d` to the deal list.
4. `SetAmount(0)`. `d` is now listed and not Silent, so this **sends** status 0 synchronously (§5.3). The
   simulator's handler sets `statsDirty` and calls `simulate_deals` at once. `ready` is true here, because
   `ProposeDeals` runs only when ready, and `busy` is false, because `ProposeDeals` does not set it. So a full run
   (§7.3) happens **inside** `Propose`:
   - unless `uiDisabled`, `d` normally passes `UpdateState` and the garbage check, because the proposal tests just
     checked its connection and the payer's budget. It then has no flag of `0x3C` and is Proposed, so `Activate()`
     makes it **Active** (posting 0, 0, NB2), `currentDate := today`, and status 1 is posted. A garbage-sell
     proposal can instead get NoLandfill here, which starts Grace (§7.3);
   - every other listed deal is simulated as well;
   - `UpdateCommodityStats` runs and the region push follows.
5. Back in `SetAmount`: `UpdateCost()` (cost 1, status 0 posted).
6. Back in `Propose`: the term, then `currentDate := startDate := today` and `expirationDate := today + 365·y`
   overwrite the dates. During the nested run the start and expiration dates were still the construction dates
   (§5.1). The nested run does not read them for a Proposed deal.

So `ProposeDeals(list, true)` returns deals that are already **Active**, unless `uiDisabled` was set, in which case
they stay Proposed. This is the original's behaviour and must be kept. The queued reports (0 from step 2, then 0,
0, 1 from step 4, then 0 from step 5) each trigger a further run when delivered (§7.2).

Who calls `ProposeDeals` and sets amounts afterwards is the UI's business (open question 3).

### 7.2 When the deal simulation runs

`simulate_deals` (§7.3) runs:
- on SimNewMonth (month agent);
- on **every** status report `0xEA418D8F` delivered to the simulator (§11.1), including the ones its own runs post.
  A run therefore usually triggers one or two follow-up runs in the next message-server ticks, until no deal
  changes *[compat]*.

  Example: one run activates one non-Silent proposal. It posts, in this order: status 0 (Active set), status 0
  (Proposed cleared), both from `Activate`'s `SetFlags` calls (§5.4), then status 1. Each report, when delivered,
  sets its dirty flag and runs `simulate_deals` again.

It does nothing until ConnectionsReady has been seen. If called while a run is in progress (a synchronous status
report from `SetAmount`), it posts `0xEA418D8F` with data1 = 8 and returns.

### 7.3 `simulate_deals` *[compat]*

```
busy := true
UpdateCommodityStats(city)                                   # §8.2 (also overwrites the price curve, §8.3)
if not uiDisabled:
    months := 12 × tuning 0xEA46934F;  k := months + 0.000107
    free := landfill capacity − landfill usage                 (GetLandfillUsage outputs, unsigned)
    R := our regional city;  exportBudget := R.GetExported(3)
    for d in deals (list order), skipping Silent deals:
        1. d.UpdateState():
             if d is a buy type and R.GetBudget() < d.cost: SetFlags(CannotAfford)
             elif d's other-side city is missing or FindConnectionSuitableForDeal(type, it) is null: SetFlags(NoConnection)
        2. if type == E5 (sell garbage) and
              R.GetImported(3) + R.GetProduced(3) + free × (1 − 1/k) < exportBudget:
                 exportBudget −= d.amount;  SetFlags(NoLandfill)
        3. if no flag of 0x3C:
             if Proposed:   if d.Activate(): currentDate := today; report 1; statsDirty := true
             elif Active:   currentDate := today
                            if Grace:      if today ≥ suspension + grace: clear Grace, set Probation
                            elif Probation: if today ≥ suspension + grace + probation: clear Probation
                            elif Expired:  clear Expired; payer := (sell type) ? other-side city : R
                                           d.UpdateCost()
                                           if payer.GetBudget() < d.cost: SetFlags(CannotAfford)
                                           else: renew: span := expiration − start; currentDate := startDate := today;
                                                 expirationDate := today + span; report 7
                            elif today ≥ expiration: set Expired; report 6
             else:          d.Cancel()
           else:
             if Cancelled:          mark for removal, report 2
             elif not Probation:    set Grace; clear 0x38; suspensionDate := today          # first strike
             elif NoConnection:     mark for removal, report 3
             elif CannotAfford:     mark for removal, report 4
             elif NoLandfill:       mark for removal, report 5
             on removal: d.SetAmount(0) after the report
    dealsDirty := dealsDirty or any removal
    remove the marked deals from the list
    UpdateCommodityStats(city)
push to the region (dealsDirty, statsDirty)                     # §9.2
busy := false
```

`grace` and `probation` are the tuning values `0x6B588FAD` and `0x8B58C59E` in days (90 and 90). Dates compare
as Julian days; "today" is the simulator date.

**`UpdateState` in detail** (`0x006CF360`):

- `R` comes from the region (its current regional record).
- For a **buy** type it compares `cost` (int64, loaded exactly) with `R.GetBudget()`. It sets CannotAfford when
  `cost > budget`; equal, or a NaN budget, does not set it. It then returns **without** the connection test.
- Otherwise (a sell type, or an affordable buy) it sets NoConnection when the other-side city is missing or when
  the neighbours simulator (the global pointer, not `this`) finds no suitable connection.

*No current regional city* (NB15): `R` is used without a null check. A **buy** deal crashes the original at
`0x006CF386`. A sell deal never touches `R`. *[choice]* FreeCity4 skips the budget test and goes on to the
connection test.

**Renewal** (`0x006D1366`–`0x006D13C4`). The run clears Expired and picks the payer: `R` for a buy type, the
other-side city for a sell type. It then calls `UpdateCost()` and compares the payer's `GetBudget()` with `cost`
(as double). It sets CannotAfford when `budget < cost`, or when the budget is a NaN, and renews otherwise, equality
included.

*Missing payer* (NB16): the original calls `GetBudget` on the payer without a null check (`0x006D13A5`). The case is
unreachable in the original:
- A sell deal whose neighbour is missing gets NoConnection from step 1 of the same pass, which sends it to the
  failure branch before renewal. Grace or Probation would also bypass renewal.
- A buy deal without `R` has already crashed in step 1 (NB15).

With FreeCity4's NB15 choice a buy deal can reach renewal without `R`. *[choice]* A missing payer counts as
**cannot afford**: CannotAfford is set and the deal is not renewed.

Consequences: a failing deal survives one grace period (90 days) during which failures are ignored, then a
probation period (90 days) during which any failure ends it. Expired deals renew automatically for the same span
if the payer can afford the updated cost. The checks in step 1 run every time; for **sell** deals the
neighbour's ability to pay is checked only at renewal.

## 8. Pricing

### 8.1 `GetDealCost(type, amount, C)` and `GetDealMaxAmount(type, C)` *[compat]*

Both use the regional-city commodity statistics (`GetDemanded`, `GetExported`, `GetProduced`, `GetImported`,
`GetCostPerUnit` with the commodity index of §3) and the tuning of §12. Let `buyer`/`seller` be (us, `C`) for
buy types and (`C`, us) for sell types, `base` the type's base amount, `scale := 0xEA4AD2FF` (default 1) and
`existing` the amount of the **first** deal returned by `FindDeals(type, C)` (0 if none).

About `existing` (NB8, NB9; `0x006D33A5`–`0x006D33E1`, and the same in `GetDealMaxAmount`):

- It is this simulator's own `FindDeals(type, C)` (slot 29): the deal list in order, matched on type and
  other-side city only. **No flag filter** is applied: a Cancelled, Silent or failing deal counts like any other.
- Only the first match counts.
- So the deal being priced is its own `existing` when it is in this simulator's list and is the first deal of its
  type with `C`. That is the usual case for `UpdateCost` on a listed deal, and `SetAmount` has already stored the
  new amount by then.
- When another deal of the same type and city comes first, that deal's amount is used instead.
- A deal outside the list, such as a fresh `ProposeDeal` result or a deal held by a regional city, is never its own
  `existing`.

Example (test 3's data): the priced power-buy deal of amount 100 is the only listed deal, so `existing = 100`.
Then `x1 = (1 − (500 − 200)) / 100 = −2.99` and `x2 = ((2000 − 100) − (1000 − 100)) / 2000 = 0.5`, so the scarcity
curve gives 1 and 0.0714286. `unit = 2.2 + (1 + 0.0714286) / 2 × 0.8 = 2.6285714`, and the cost is trunc(262.857) =
**262**. With `existing = 0`, test 3 gives 263.

```
GetDealCost:
    if amount < 1e−8: return 0
    need  := max(base/scale + 1, buyer.Demanded + buyer.Exported)
    have  := max(1e−8, seller.Imported + seller.Produced)
    x1    := (1 − (need − (buyer.Produced + buyer.Imported − existing))) / amount
    x2    := ((have − amount) − (seller.Demanded + seller.Exported − existing)) / have
    pMin  := seller.CostPerUnit × (1 + 0x6A469342)          (default markup 1 → ×2)
    pMax  := seller.CostPerUnit × (1 + 0xCA469346)          (default 1 → ×2)
    unit  := clamp(pMin + (curve(x1) + curve(x2)) / 2 × (pMax − pMin), pMin, pMax)
    return unit × amount                                       (double; callers truncate)

GetDealMaxAmount (0 if C is null):
    need      := max(base/scale + 1, buyer.Demanded + buyer.Exported)
    shortfall := scale × need − (buyer.Produced + buyer.Imported − existing)
    surplus   := (seller.Produced + seller.Imported) − (seller.Demanded + seller.Exported − existing)
    return max(0, min(shortfall, surplus))
```

`curve` is the simulator's single response curve (§8.3), evaluated with a `float32` input and output
(foundations §4). The clamp is applied as `max(pMin, …)` then `min(pMax, …)`. *[quirk]* An unknown deal type uses
an undefined commodity index in the original; *[choice]* FreeCity4 returns 0 for unknown types.

### 8.2 `UpdateCommodityStats(city)` *[compat]*

Only when `city` is the current city.

The subject (NB12; `0x006D2A2C`–`0x006D2A46`) is the **city object**, a `cISC4City*` as the SDK declares it, not
a regional city. It is compared by identity with the current city (the game's global city pointer), and a null or
any other pointer returns at once with nothing done. All shipped callers pass the current city, including the
simulator itself in §7.1, §7.3 and §14. The regional-city object of our own city is a different object: passed
here, it would do nothing. FreeCity4 must therefore key this call on the city: a handle that is either the current
city or not.

**Which pointer is "the current city"** (NB18). The test is raw pointer equality with the global at `0x00B43CEC`
(`0x006D2A40`). The only writes to that global are:
- the city's constructor (`cSC4City::cSC4City`, through the setter `0x00601C10` at `0x0045A34A`), which stores the
  object's own address, its primary `cISC4City` interface;
- `SL::OnCityShutdown` (`0x0060210E`), which stores null.

So the global holds the city object's `cISC4City*` from construction until city shutdown, and a host method that
returns exactly that pointer gives the subject the original compares against. The match is by address. Any other
interface pointer of the same object, or a wrapper around it, does not match, and the call then does nothing. How
a test host represents "the current city" is glue: no original behaviour, and the implementer's choice of a token
distinct from every regional-city ID stands.

`R` is then fetched from the region without a null check (`0x006D2A6C`). With no current regional city the original
crashes; *[choice]* FreeCity4 returns at once in that case.

With `R` = the current regional city:

1. `R.SetBudget(budget.GetTotalFunds())`, `R.SetIncome(budget.GetTotalMonthlyIncome())`.
2. Sum `cost` over **all** deals (any flags) of types E0 → `P`, E2 → `W`, E4 → `G`.
3. Push to the region (both flags, §9.2).
4. For each commodity `c`, with `imp := R.GetImported(c)`, `exp := R.GetExported(c)`:

| c | produced | demanded | cost per unit (`1e−5` if the department is missing) |
|---|---|---|---|
| 1 power | `power.GetMonthlyProduction()` | `power.GetMonthlyRequirement()` | `(P + dept(0x8910BC8A).GetTotalExpenses()) / (produced + imp + 1)` |
| 2 water | `plumbing.GetWaterProductionCapacity()` | `plumbing.GetWaterDemand()` | `(W + dept(0xAA4014B4).GetTotalExpenses()) / (produced + imp + 1)` |
| 3 garbage | `recycled + wasteToEnergyCap + incineratorCap + free / (12 × years) + abducted` | `pollution.GetGarbageProducedLastMonth()` | `curve((wte + G + dept(0x6A42A8F5).GetTotalExpenses()) / (produced + imp + 1))` |

   For garbage: the pollution simulator's `GetGarbageRecycledLastMonth`, `GetMonthlyWasteToEnergyCapacity`,
   `GetMonthlyIncineratorCapacity`, `GetGarbageAbductedPrevMonth`; `free` = landfill capacity − usage; `years` =
   `0xEA46934F`; `wte` = full expenses of line item `0x1F470000` of the power department. **Before** evaluating the
   garbage curve, the simulator loads property `0x4B61FFC9` into its curve (§8.3).

   The `free / (12 × years)` term (NB13) works as follows:
   - `years` is read as f32 into a local that is given **no default** first (`0x006D2AE3`–`0x006D2B0A`).
   - `12 × years` is stored as float32. The u32 `free` is converted and divided by it on the x87 (`0x006D309C`).
   - A **present** `years = 0` is divided by literally. With `free > 0` the term is +∞, so `produced` is +∞: `R`
     stores +∞ (`SetProduced` takes a double), the garbage cost-per-unit input `x / ∞` is 0, and the `0x4B54F333`
     history entry is 0 (`_ftol2`'s low dword).
   - With `free = 0`, a present 0 gives 0/0 = NaN instead, and `produced` is NaN.
   - A **missing** property leaves whatever was on the stack: the original's result is undefined. *[choice]*
     FreeCity4 treats a missing `years` as making the term 0.

   The shipped exemplar has `years = 1`, so only mods reach either case.

   Then `R.SetProduced(c, produced)`, `SetDemanded(c, demanded)`, `SetExported(c, exp)`, `SetImported(c, imp)`,
   `SetCostPerUnit(c, costPerUnit)`, and record history: power `0xAA11F837` ← imported, `0xAA11F83A` ←
   exported; water `0x0A14EBB5` ← imported, `0xCA14EBC6` ← exported; garbage `0xEA11017B` ← exported,
   `0xEA11017E` ← imported, `0xEB54F32E` ← demanded, `0x4B54F333` ← produced (all truncated to `int`).

### 8.3 The deal-price curve bug *[compat]*

The original has **one** curve object. `Init` loads the deal-price scarcity curve `0x0A469353`
`[(0,1) (0.1,0.3) (0.3,0.1) (1,0)]` into it. `UpdateCommodityStats` loads the garbage cost curve `0x4B61FFC9`
`[(0,1e−5) (0.0075,0.0075) (1,0.075)]` into the **same** object whenever the sanitation department exists and the
property is present. `UpdateCommodityStats` runs at the start and end of every `simulate_deals`, at the start of
`ProposeDeals`, and whenever another subsystem calls it. Therefore, from the first deal simulation after a city
is loaded or created, `GetDealCost` uses the **garbage curve**: prices sit just above `pMin` (the curve's output is
at most 0.075). Before that first run (right after a load, until ConnectionsReady and the first month), prices
use the scarcity curve.

FreeCity4 must reproduce this by default. *[choice]* A configuration switch may keep two curves (the intended
behaviour); it is off by default and must not change saves.

## 9. Region interaction

### 9.1 `GetAdjacentCities(list, includeUnestablished)`

The regional cities in the region cells that touch the current city's footprint: for each region column
`px .. px+sx−1` the cells just north (`pz − 1`) and south (`pz + sz`), and for each row the cells just west and
east. Each city once; established only unless the flag is set. The original orders them by object address;
*[choice]* FreeCity4 orders them by region position (x, then z).

How the original orders them (NB14; `0x006D01E0`):

- It collects the cities into a `std::set<cISC4RegionalCity*>` with `std::less`, then copies the set into the list.
  The order is therefore ascending **heap address** of the regional-city objects, which depends on allocation
  history and cannot be reproduced.
- The north and south rows are scanned for each column, then the west and east cells for each row. This decides
  only which cities are found, not their order.

The order shows in four places:
- the order of `ProposeDeals`' results;
- the order of the per-city pushes in §9.2;
- the order of `ReloadCity` in `Init` step 5;
- the order of the cloned deals in `Init` step 7.

Mostly only the order of messages and of list entries changes. There is one exception: with
`ProposeDeals(…, true)`, the re-entrant runs (§7.1) refresh `R`'s statistics between cities. An earlier buy
proposal's cost of 1 then enters `P`, `W` or `G`, and through them our cost per unit, which prices a later sell
proposal's budget test. Because the original's order is not reproducible, the *[choice]* stands.

### 9.2 Pushing to the region (`update_deals_in_regional_cities(deals, stats)`) *[compat]*

If either flag is set: for each adjacent established city, then for the current regional city:
`if deals: city.SetLocalDeals(dealList)`; `if stats: city.UpdateImportExport()`. Then clear the pushed flags.
`SetLocalDeals` keeps **references** to the simulator's deal objects (only those involving that city) and later
writes them with their own `Write` (§5.6) into the region records; FreeCity4 deal objects must therefore produce
the exact format of §5.6.

Pushes happen: in `simulate_deals` (§7.3), `UpdateCommodityStats`, `AddDeal`, `ProposeDeals`, `Init`, on
SimNewDay and SimIdle (with the current dirty flags) and on CityEstablished / CityObliterated.

### 9.3 Where deals are stored

Deals are not in the city's own neighbours record. Each regional-city record in the save holds its connections
(the inline connection record, §4.3) and its local deals (the inline deal record, §5.6), read back by the
regional city, which marks them Silent. On `Init` the simulator rebuilds its own list from the neighbours'
records (§14.1). The regional-city record format belongs to a region spec (open question 1).

## 10. Effects on other subsystems

### 10.1 Budget

```
GetTotalMonthlyNeighborDealExpense = Σ cost over deals with a buy type (E0, E2, E4) and none of the flags 0x3D
GetTotalMonthlyNeighborDealIncome  = Σ cost over deals with a sell type (E1, E3, E5) and none of the flags 0x3D
```

Expired (0x40), Grace, Probation and Silent deals still count. The budget books these monthly (budget spec
§5.5); whether it sees this month's prices depends on month-agent order (budget spec §14 question 2).

### 10.2 Utilities

The power, plumbing and pollution simulators read the deals of their commodity (`FindDeals`) and the regional
import/export figures, count bought amounts as capacity and sold amounts as load, maintain the deal's amount
fields, set connections active or inactive, and call `MarkDealsDirty` when their capacities change (see the
grid-sims notes; their specs own the details).

### 10.3 Demand and traffic

- Demand: connection counts and traffic through transport connections relieve demand caps (demand spec §8.3);
  it uses `GetConnectionsToCity`, `GetConnectionsOfTypeToCity`, `FindConnectionOnOtherSide` and subscribes to
  `0x8A0D15DA` / `0xEA0D163E`.
- Traffic: every connection is a traffic source; the traffic simulator uses `GetAllConnections`,
  `GetConnectionsOfType`, `GetConnectedCities`, `FindConnection`, `FindConnectionOnOtherSide`, the connection's
  property holder and `UpdateTrafficVolume`, and subscribes to the created/deleted messages.

## 11. Messages

### 11.1 Handled (subscriptions made by `Init`, plus agents)

| Message | Action |
|---|---|
| SimNewMonth `0x66956816` | `simulate_deals` |
| SimNewDay `0x66956814`, SimIdle `0x6695681A` | push to the region with the current dirty flags |
| `0xEA418D8F` deal status | data1 ∈ {0, 8} → `statsDirty := true`, else `dealsDirty := true`; then `simulate_deals` |
| ConnectionsReady `0x6AC284F3` (posted once per load by the regional city; when and when not: foundations.md §11.6) | `ready := true` |
| TerrainHeightsChanged `0x278E6003` | rectangle `x0 = (data1 >> 16) − 1`, `x1 = data1 & 0xFFFF`, `z0 = (data2 >> 16) − 1`, `z1 = data2 & 0xFFFF`, clamped to the city: for each connection whose inner cell lies in it, move its marker props to the cell centre at `terrain.GetMaxSurfaceLevel(x, z)` (+15.5 for elevated types) and update them in the occupant manager |
| PreDemolishNetworkOccupant `0xAA7A014D` | `DeleteOccupantConnection(void1)` |
| RemoveOccupant `0x99EF1143` | if the occupant's type is `0x2890D4DE` and its interface `0xA8C86142` (`cISC4PowerPoleOccupant`) reports false from its 14th method (compiled slot 13, a byte flag of unknown meaning; power spec open question 1): `DeleteOccupantConnection` |
| CityEstablished `0x26D31EC4`, CityObliterated `0x26D31EC5` | clear the deal list; push to the region (both flags) |

### 11.2 Posted

`0x8A0D15DA` NeighborConnectionCreated, `0xEA0D163E` NeighborConnectionDeleted (§6.3–6.4); status reports
`0xEA418D8F` (§5.3).

## 12. Tuning

Exemplar `0x6534284A / 0x7A4A8458 / 0x6A469021` "Neighbor Deals" (`SimCity_1.dat`). All values are `f32`.

| Property | Shipped | Default when missing | Meaning |
|---|---|---|---|
| `0x0A469353` | [0,1, 0.1,0.3, 0.3,0.1, 1,0] | no curve | deal-price scarcity curve (§8.3) |
| `0x4B61FFC9` | [0,1e−5, 0.0075,0.0075, 1,0.075] | curve unchanged | garbage cost-per-unit curve |
| `0xEA46932C` / `0x6A469337` / `0x8A46933D` | 1000 / 1000 / 1000 | 0 | base amount for power / water / garbage |
| `0xEA4AD2FF` | 2.5 | 1 | amount scale |
| `0x6A469342` | 0.1 | 1 | minimum markup over the seller's cost per unit |
| `0xCA469346` | 0.5 | 1 | maximum markup |
| `0x6B588FAD` | 90 | 0 | grace period (days) |
| `0x8B58C59E` | 90 | 0 | probation period (days) |
| `0xEA46934F` | 1 | 5 in proposals, 0 in the deal simulation; in `UpdateCommodityStats` an **uninitialized** stack slot (§8.2; *[choice]* missing → treat the free-landfill term as 0) | landfill horizon and maximum deal term (years) |
| `0xAA46934B` | 1 | 1 | minimum deal term (years) |
| `0xCA46931F` | 60 | 1 | proposal: budget must exceed this × the deal cost |
| `0xEA4AE9EF` | 5000 | 0 | proposal: minimum payer budget |

Read widths (data-formats.md §7): `GetDealCost` reads `0x6A469342` and `0xCA469346` as **f64** (`0x006D3512`,
`0x006D3530`), so 0.1 and 0.5 enter its arithmetic unrounded; `ProposeDeals` reads `0xCA469346` as f32
(`0x006D22B4`), i.e. 0.5 exactly. Corrected: not all values are f32.

That f32 read in `ProposeDeals` is **dead** (NB4; `0x006D22A1`–`0x006D22CE`). The value, default 1.0, plus 1.0 is
stored in a local that nothing reads. The proposal checks of §7.1 use `0xEA4AE9EF` (f32, default 0) and
`0xCA46931F` (f32, default 1) only. So `0xCA469346` has one effect, the maximum markup in `GetDealCost` (§8.1).

## 13. Persistence (`GetGZCLSID() = 0xC990C0A4`) *[compat]*

```
u16  version = 2
u32  count
count × ref connection           (foundations §2.7, list order)
```

`Read` fails unless `version == 2`; each reference is resolved as `cISC4NeighborConnection` (`0x0773FEF2`) and
appended (the list is not cleared first).

Each reference is read by the game's helper `SC4ReadGZSerializable` (`0x005D9730`; foundations §2.7). There are
three cases (NB7; `0x006CFE10`–`0x006CFEDD`):

| Reference | Helper result | Original | FreeCity4 |
|---|---|---|---|
| a connection | success, the object | appended (AddRef'd) | same |
| non-null, failing: unknown class, failed creation, or an object without `0x0773FEF2` | failure, null | skipped (not appended); the loop goes on | same |
| null (`objectID = 0`) | success, null | the null is **appended**, and then the local is released **without a null check** (`0x006CFEC5`): a null dereference, so the original crashes inside `Read` | *[choice]* skipped |

At the end, `Read` returns true unless the stream is in the error state (an unknown class sets it, foundations
§2.7).

**Wrong version** (NB11). With `version ≠ 2`, `Read` returns false straight after the `u16` (`0x006CFDDB`) and
reads nothing else. Reading the `u16` succeeded, so the stream stays good. The class-object store ignores the
result and re-synchronises at the next entry (foundations §3.5). The simulator object was created empty when its
reference was resolved, so the load continues with **no connections**: for example, a version-3 record loads
exactly like a version-2 record with `count = 0`. `Init` (§14.1) then runs as usual on the empty list.

## 14. Lifecycle

### 14.1 `Init()` → true

If not initialized:
1. Set initialized. Load the tuning exemplar if not yet held; load the scarcity curve from `0x0A469353`.
2. Register agents at levels **6 (month), 4 (day) and 10 (idle)**, name `"cSC4NeighborsSimulator"`, flags 2.
3. Hold the message server; subscribe to `0xAA7A014D`, `0x278E6003`, `0xEA418D8F`, `0x6AC284F3`, `0x99EF1143`,
   `0x26D31EC5`, `0x26D31EC4`. Hold the city.
4. `Repair(R)` every connection with the current regional city `R`.
5. For each adjacent established city: `region.ReloadCity` it (refreshes its record from disk).
   *[compat]* `ReloadCity` **replaces** the region's regional city object at that position: the region drops its
   reference to the old object and holds a new one (run r50e2: `141D1414 → 17F41C14`). The list from
   `GetAdjacentCities` holds a reference to each entry. The original releases each entry right after its
   `ReloadCity`, which frees the old object, and then **calls `GetAdjacentCities` again** for steps 6–7. No
   reference to an object from before the reload is used after step 5. A last-reference release here is expected;
   the old object is no longer the region's.
6. `R.UpdateLocalDeals()`, with `R` the current regional city asked for again after step 5; clear the deal list.
7. For each adjacent established city `C ≠ R` of the **new** list (in reverse list order): `C.UpdateLocalDeals()`; for each deal `d`
   in `C.GetLocalDeals()` (from the back): `d.Clone(&e, false)`; if `e`'s other-side city exists:
   `e.SetFlags(Silent, false)`, append `e`, and shift its four dates by `today − e.currentDate` (so
   `currentDate = today` and the remaining term is kept).
8. Push to the region (both flags).

The connection list itself comes from the save (§13) or is empty for a new city; connections are created by the
network tools and the regional city as the player builds.

### 14.2 `Shutdown()` → true

If initialized: clear it; release the city; unsubscribe all seven messages and release the message server;
remove the agents (`RemoveAgent(this)`); clear the connection and deal lists; release the tuning exemplar.

## 15. Interactions and replaceability

### 15.1 Callers (call-site survey)

| Caller | Uses |
|---|---|
| City | creates through GZCOM, `Init`/`Shutdown`, save/load, `GetNeighborsSimulator()`. |
| Budget | `GetTotalMonthlyNeighborDealIncome/Expense`. |
| Power, plumbing, pollution | `FindDeals`, `MarkDealsDirty`, connections (§10.2). |
| Demand, traffic, traffic automata, MySim dispatch, growth developer, watercraft manager | connection queries (§10.3). |
| Network tool / network tool UI, surface water (ferries), regional city (`CreateSymmetricConnections`, `TreatAllIncomingConnectionsAsNew`, `MarkConnectionsForRemoval`, `ExtractLocalDeals`) | `RequestCreateConnection`, `CreateConnection`, `DeleteConnection`, `DeleteAllConnectionsOfType`, `HasConnection`, `FindConnection`, `FindCityOnOtherSideOfConnection`, `GetAdjacentCities`. |
| Demolition | `DeleteOccupantConnection`. |
| Query tool, advisors (utility, finance, traffic, city planning), budget window, city situation manager, `SaveCity` | deal and connection queries, totals, `GetDealMaxAmount`. |
| Deals themselves | `UpdateCost` → `GetDealCost`; `UpdateState` → `FindConnectionSuitableForDeal`. |

### 15.2 Replaceability findings

From the Windows caller data (direct calls) and the call-site survey:

- **Neighbours simulator**: created only through GZCOM (`GetClassObject(0xC990C0A4, 0x0773FEF0)`), reached only
  through `cISC4NeighborsSimulator`. Other original code calls the **static** helpers of §2.2 directly
  (valid-cell test from the connection-info writer and the query tool; neighbour-cell conversion from the
  regional city). They use no simulator state, so the simulator is **replaceable** through the registry; our
  geometry must match §2.2 exactly because both sides compute it.
- **Connections** are constructed **directly** (not through GZCOM) by the original simulator's
  `CreateConnection` and by the regional city when it reads its record; through GZCOM (CLSID `0x0A134F24`) only
  when the class store loads a city's own neighbours record. Consequences: a FreeCity4 simulator creates its own
  connection objects, but regional cities always hold **original** connection objects, and `FindConnectionOnOtherSide`
  returns them. FreeCity4 must use them only through `cISC4NeighborConnection` (and the traffic interfaces) and
  must accept original objects in its own list after a load if the connection class is not replaced.
- **Deals** are constructed **directly** by `ProposeDeal`, by `Clone` and by the regional city's reader; the
  GZCOM factory (CLSID `0x2A134F42`) is effectively unused because city records contain no deals. Consequences:
  original deal objects reach a FreeCity4 simulator through `GetLocalDeals` (and their `Clone` produces original
  objects), and FreeCity4 deal objects reach regional cities through `SetLocalDeals` and are written by the
  region with their own `Write`. FreeCity4 must therefore (a) handle foreign deals through `cISC4NeighborDeal`
  only, (b) implement `Clone` on its own deals, and (c) write the exact §5.6 format.

### 15.3 How original code reaches a FreeCity4 connection object (NG3) *[compat]*

**No original code outside the connection class reads a connection's fields or calls its code directly.** With the
neighbours simulator replaced, every access to one of our connection objects goes through a virtual slot of
`cISC4NeighborConnection` (§4.2, vtable `0x00AB2034`), of the traffic source (`0x00AB1FA0`, traffic-simulator.md
§4.1), of the property holder (§17.2 of foundations) or of `cIGZSerializable`. Our objects may have any layout behind
those interfaces. How this was checked:

- **Direct calls.** A scan of the whole image for `call`/`jmp` rel32 targets and for stored pointers to every
  method of the class (`0x006CDFF0`–`0x006D4C10` and the thunks `0x006D47A0`–`0x006D4853`) finds only the vtables,
  the class's own thunks, the GZCOM class-factory registration (`0x00460A72`, the CLSID getter) and two calls to the
  **constructor**: the original simulator's `CreateConnection` (`0x006D489B`) and the regional city's `Read`
  (`0x004AC994`), which builds the regional city's own connection objects. The Mac build agrees: outside the class,
  only the factory, `cSC4NeighborsSimulator::CreateConnection` and `cSC4RegionalCity::Read` call or name the
  concrete class.
- **Class identification.** The vtable constants (`0x00AB2034`, `0x00AB201C`, `0x00AB1FB8`, `0x00AB1FA0`) appear in
  code only in the constructor (`0x006D4680`) and the destructor (`0x006D4BB0`). Nothing tests the vtable to
  recognise an original connection.
- **Field reads.** The 66 functions that use connections (Mac symbol names mapped to the Windows image) and the 55
  that read the simulator global `0x00B43D8C` were scanned for reads at the class's field offsets (`+0x20`…`+0x43`)
  through a pointer taken from a list node or from a call's result. None was found on a connection. The hits were
  other objects: the traffic statistics block (`0x0041C80D`), new allocations (`0x0052B012`, the traffic simulator's
  `Init` and `Read`) and the budget's own records.

Where original code holds **our** objects, and what it calls:

| Holder | How it gets them | What it calls |
|---|---|---|
| the current city's regional record (`cSC4RegionalCity`, list at `+0x90`) | `UpdateCityCache(city)` (`0x004AD290`; on PostCityInit and PreSave, foundations §11.3) calls our `GetAllConnections(&list)` at `0x004AD34E`, so the list then holds references to our objects | `Write` (`0x004A7D10`): `QueryInterface(0xE4FDA3D4)` and the serializable's `Write` (slot 3), so the current city's record contains our §4.3 records; `FindConnection` (slot 20, `0x004A7B40`): slots 5 and 6; `GetAllConnections` (slot 21): AddRef only |
| `ChangeSymmetricConnection(conn, flag)` (`0x004A8DF0`) | our simulator passes the connection (§6.3, §6.4) | slots 6 (`GetConnectionCell`) and 5 (`GetType`); the pointer is not kept |
| the traffic simulator | `GetAllConnections`, `GetConnectionsOfType`, `FindConnection`, the traffic-source iterator | `AsTrafficSource` (slot 4) and `AsISCPropertyHolder` (slot 3), then the traffic-source and holder slots |
| demand, MySim, the query tool, advisors, network and power-line tools, surface water, watercraft, plumbing, power | the simulator's connection queries | the `cISC4NeighborConnection` slots only |

The one layout contract behind an interface is the traffic source's slot 3 `GetTripDataList`: it returns a pointer
to the source's own STL `std::list<cSC4TripData>` (allocator B), and the traffic simulator edits that list in place
(traffic-simulator.md §4.1, §7). A FreeCity4 connection must therefore own a list of that exact STL layout, as it
already does. Everything else about the object's memory is free.

### 15.4 Original behaviour behind the glue's choices (NG2–NG34) *[compat]*

The neighbours glue (`cFC4NeighborsSimulator`, `FC4NeighboursShadow`) made a choice wherever the spec was silent.
Each item below gives what the original does, then whether that choice matches. Items marked *glue* have no
original behaviour behind them; the implementer's choice stands.

**NG2. Object-valued properties on a connection.** The original connection's holder is the plain holder of
foundations §10.2. `AddProperty(id, cIGZUnknown*, bool)` (holder slot 11, `0x005FF950`) stores the object in the
property's variant and keeps a reference to it. `GetProperty(id, riid, &ppv)` (slot 6) and `GetProperty(id)` (slot 9)
return it like any other value. On save the holder writes it as a reference (marker `0xA9BEE9AC`, foundations §9.2,
§10.2), so the object also gets its own record in the save. On load the holder drops it. No shipped code adds an
object-valued property to a connection. The properties seen on connections in the sample saves are the traffic
arrays `0x8A0B4100` and `0xAA9EA81A`–`0xAA9EA81F` and the float `0xCA947077` (foundations §10.6 (b)), all plain
values.

- While the city runs, the game-facing holder must keep such a property like any other (a mod may add one and read
  it back). Dropping it there would be visible.
- Leaving it out of the core mirror and out of the save is invisible to any reader: the original drops it on load.
  Only the bytes of the save differ, and only when a mod has added one.
- So the bracket "dropped from the core sync" matches the original for every shipped path, provided the holder
  that game code calls still keeps the object until the property is removed or the connection dies.

**NG4. `AddDeal` with a foreign deal.** `AddDeal(d)` (slot 26, `0x006D2870`) keeps **the caller's object**:
`d.SetFlags(Silent, false)`, append `d` itself to the deal list (AddRef), push to the region (§9.2, both flags),
return true. From then on the simulation, `FindDeals`, the status reports and the region's `SetLocalDeals` all hand
out that same pointer, and the caller's later calls on `d` act on the simulated deal. No shipped code calls
`AddDeal`, so only a mod can see the difference.

- The bracket (copy into a deal of ours) differs: the caller's object stays outside the simulation, its later changes
  are lost, and `FindDeals` returns a different pointer from the one that was added.
- *[choice]* Keep the caller's object as the listed deal and drive it through `cISC4NeighborDeal`, as for the foreign
  deals of §15.2. If the core must own its deals, the copy may stay as a deviation that only mods can observe.

**NG5. Lifetime of a deal object.** An original deal is a plain counted object. It is destroyed at its last
`Release`, never earlier, and it is never reused. These hold references to it:

- the simulator's deal list (§6.1);
- the regional cities, after `SetLocalDeals` (§9.2, foundations §11.3), until the next push replaces their lists;
- every queued status report (§5.3), until it is delivered. That can be after the deal left the list (statuses 2–5);
- the lists filled by `FindDeals`, `FindDealsAll` and `ProposeDeals` (each entry AddRef'd), including the budget
  window's neighbour-tab list while the tab is open (NG7);
- the utility advisor's advice made from a status 3–6 report, which is given the deal (NG31).

So a deal that has left the simulation must keep answering every `cISC4NeighborDeal` call with its last state while
any of these holds it, and the same deal must keep the same pointer for as long as any reference exists. Creating the
wrapper lazily cannot be seen by the game. Neither can keeping a wrapper at zero references for reuse ("parked"),
because no correct caller can reach an object it holds no reference to. The bracket stands under those two rules.

**NG6 (glue). Getters called from other threads.** The original simulator has no lock and no thread check: every
method runs on the caller's thread against the live fields. The 20 ms try-lock and its fallback are the
implementer's choice.

**NG7. Simulator state the shadow cannot see through the interface.** The original keeps it in byte fields of the
simulator object (offsets from the `cISC4NeighborsSimulator` pointer, build 1.1.641):

| Offset | Field | Written by |
|---|---|---|
| `+0x0C` | ready | 0 at construction; 1 by the ConnectionsReady handler. Never cleared, not even by `Shutdown` |
| `+0x0D` | initialized | `Init` sets it, `Shutdown` clears it |
| `+0x3C` / `+0x3D` | dealsDirty / statsDirty | §6.1, §11.1; cleared after each region push |
| `+0x3E` | uiDisabled | `SetUIDisableFlag` (`0x006CDFE0`) only |
| `+0x3F` | busy | set at the start of `simulate_deals`, cleared at its end (`0x006D0E99`, `0x006D15D2`) |

The bracket (ready = 1, uiDisabled = 0) is wrong in two windows:

- **Before ConnectionsReady.** `ready` is 0 from `Init` until the regional city posts ConnectionsReady (or for the
  whole session if it never does, foundations §11.6). Meanwhile `simulate_deals` and `ProposeDeals` do nothing.
- **While the budget window's neighbour tab is open.** `initNeighborBudget` calls `SetUIDisableFlag(true)`
  (`0x0077E6C5`) and then `ProposeDeals(list, true)` (`0x0077E735`). `DestroySubBudgetWindow` calls
  `SetUIDisableFlag(false)` (`0x0077A216`) and then `MarkDealsDirty` (`0x0077A21E`). While the tab is open, runs skip
  the state machine (§7.3), and the tab's proposals stay Proposed instead of being activated (§7.1).

*[choice]* The shadow holds the original object, so it should read these bytes at the offsets above rather than
assume them.

**NG8. The volume-dirty flag.** The flag (a byte at connection `+0x40`) is set by the connection's constructor and
by nothing else: not by `Read`, `Repair` or the simulator. `UpdateTrafficVolume` clears it as its first action when
it is set (`0x006D4C2B`), before it looks for the other side. So each connection object syncs once: a new connection
after creation, a loaded one after the load (the load constructs it). If the other side has no matching connection
at that moment, the trip list is cleared (NG19) and that object never syncs again. The bracket matches.

**NG9. Setters on foreign connections.** `cISC4NeighborConnection` has no setter for type or cell (§4.2). In the
original, type, cell and otherRegionCell are written only when the object is made: by the original simulator's
`CreateConnection` and by `Read`. After that only `Repair(R)` rewrites otherRegionCell. So ignoring a type or cell
change on a foreign connection matches the original. The state that does change after creation must go through the
interface on a foreign connection: `Repair`, `SetVisitedThisCycle`, `SetIsActive` and the property holder. The
bracket stands.

**NG10. The return of `CreateConnection`.** Not meaningful in the original (§6.2, NB17), and every shipped caller
ignores it. Any value works; the bracket stands.

**NG11. Slots 32 and 33 return 64 bits.** `GetTotalMonthlyNeighborDealIncome` (`0x006CF860`) and
`GetTotalMonthlyNeighborDealExpense` (`0x006CF8D0`) return a signed 64-bit sum in EDX:EAX. Every caller reads both
halves:

| Caller | Sites |
|---|---|
| finance advisor `update_variables` | `0x00413CA8`, `0x00413CD0` |
| budget simulator `GetTotalMonthlyExpense`, `GetTotalMonthlyIncome`, `DoMonthlySimulation` | `0x006909AA`, `0x00690A67`, `0x00692601` |
| budget window `CalculateValues` | `0x00767520`, `0x00767554` |
| budget sub-window `initNeighborBudget`, `updateNeighborBudget` | `0x007806EA`, `0x00781836`, `0x00783DCD`, `0x0078549F` |

The bracket (the low 32 bits) is wrong: EDX then holds whatever the shim's code left there, and the budget books it as
the high half. Return the full `int64`.

Example: two sell deals costing 2,000,000 and 3,000,000 per month. The income is 5,000,000 = `0x00000000004C4B40`:
EAX = `0x004C4B40`, EDX = 0. If EDX were left at, say, `0x0012F8A0` (a stack address), the budget would book
`0x0012F8A0004C4B40`, about 5.3 × 10^15.

**NG12. Regional-city getters 50–64.** They return a `double` on the x87 stack (compiled slot table at the end of
this spec). The bracket matches, and `docs/abi` already has these rows right.

**NG13. The return of deal `Copy`.** The original's `Copy` (`0x006CE2E0`) leaves no meaningful value, and its only
caller, `Clone` (`0x006CF3E0`), ignores it. `void` matches.

**NG14. `RequestCreateConnection` asks the player.** When the connection does not exist yet, the original builds the
title and text of §6.2 and calls `nSC4UI::GenericConfirmationDialog` (`0x0078DFF0`), a modal yes/no dialog, and
returns its answer. With popups disabled (`cISC4App::SetPopupDialogsEnabled(false)`, advisors.md) that dialog
returns false without asking, so the request is **refused**.

- The bracket (confirm without the dialog) differs in two ways: the player cannot decline a connection, and with
  popups disabled the original refuses where the glue accepts.
- *[choice]* Show the dialog through the game, as §6.2 already says (the core returns "needs confirmation", the shim
  asks). When popups are disabled, decline. Confirming without asking may stay only as a temporary deviation.

**NG15. `GetCity` with cells outside the region.** The region's `GetCity(x, z)` (slot 11, `0x004A4930`) looks the
pair up in its hash map, keyed by `SC4Point<long>`, and returns the map's holder pointer or null. Signed and unsigned
32-bit values have the same bits, so a negative cell passed as `uint32` finds the same entry as the signed value,
which is none: the map has entries only for cells covered by a city. The bracket is fine.

**NG16. Creating the marker prop.** The original (`0x006D4A53`–`0x006D4B15`) uses the city's prop manager:

1. `CreateProp(0x29F10000, &prop)` (prop manager slot 15). If it fails, there is no marker; the created message
   (§6.3 step 4) is still posted.
2. Position the prop at the inner cell's centre and height (§6.3), set its orientation, and add the bool
   properties of §6.3 through its holder (holder slot 13, `AddProperty(id, 1, false)`).
3. `AddCityProp` with the prop's occupant interface (prop manager slot 19, the one-argument overload).
4. Release its own reference to the prop.

The bracket (`CreateProp` + `AddCityProp`, then our reference released) matches.

**NG17. The occupant's cell in `DeleteOccupantConnection`.** The original asks neither the network nor the pole for
a cell. For both kinds it takes the occupant's position (`cISC4Occupant::GetPosition`, slot 8, `0x006CEF62`) and
converts it with the city's `PositionToCell(x, z, &cx, &cz)` (slot 95, `0x006CEF7E`): 16 m cells from the city's
(0, 0) corner, rounded down. §6.5 then uses `(cx, cz)`.

The bracket (the network's `GetOccupiedCell`, else the pole's `GetCell`) gives the same cell for any occupant whose
position lies inside its own cell, which is the normal case. *[choice]* Use the original's rule; it costs nothing and
removes the doubt for odd occupants.

**NG18. A deal's property holder.** The original deal's holder is unreachable too. `cSC4NeighborDeal::QueryInterface`
(`0x006CE260`) answers only `1` (IGZUnknown), `0x0773FEF1` (`cISC4NeighborDeal`) and `0xE4FDA3D4`
(`cIGZSerializable`); the deal interface has no `AsISCPropertyHolder`; and the holder's vtable (`0x00AB1D58`) appears
only in the deal's constructor and destructor. The holder exists only to be written and read with the deal record
(§5.6). Nothing can add a property to it, so in practice it is always empty, except for properties read from a
record. A FreeCity4 deal therefore needs no reachable holder: it must only keep the properties of a record it read
and write them back.

**NG19. No matching connection on the other side.** When `UpdateTrafficVolume` runs (the flag of NG8 is set) and
`FindConnectionOnOtherSide(conn)` (simulator slot 13) returns null, whether there is no city there or the city has
no matching connection, the original **clears the whole trip list** (`0x006D5030`, every node freed) and returns.

The bracket (targets 0) erases every trip of types 0–2 through the excess rule of traffic-simulator.md §4.1.4, but
keeps trips of type > 2, which that rule never touches. This sync creates only types 0–2, so the two agree unless
something else puts a trip of another type on a connection. *[choice]* Clear the list, which is exact and simpler.

**NG20. `GetSourceCells`.** Traffic-source slot 5 (`0x006D4820`) fills the rectangle with the connection cell twice:
`(x, z, x, z)`. The cell is the connection cell itself, outside the city (for example `(−1, 5)`), not the inner cell.
The bracket matches.

**NG21. The return of `UpdateTrafficVolume`.** The original's `AL` is not a clean bool: 0 when the flag of NG8 was
already clear, 3 (the loop counter) after a sync, and whatever the list clear left behind when there was no other
side (NG19). The one caller that reaches a connection's slot 4, the ConfirmTrips step of the traffic simulator
(`0x0071E28E`), ignores it. `SetupPathFinderForLot` calls slot 4 only on lots, and looks at the trip list rather
than the result. So "true" is safe; the bracket stands.

**NG22. `cISCProperty::Write` and `Read`.** The original property (`cSCProperty`, vtable `0x00AA6988`) writes and
reads foundations §10.1 on the stream it is given:

- `Write(stream)` (slot 8, `0x005FD870`): `stream.WriteUint32(propertyID)`, then the variant (foundations §9.2), then
  returns "stream not in error".
- `Read(stream)` (slot 9, `0x005FD8B0`): the same fields into the property, same result.

Nothing calls these two directly; every call is virtual. The caller that matters is the original holder's record
writer (`0x005FD990`): for each stored property it writes the key, then calls **that property's own** `Write` with
the save's stream. A holder stores the object it is given (foundations §10.2), so a FreeCity4 property that game code
puts into an original holder is written by its own `Write`, on the game's stream, in the middle of the game's
record. It must produce exactly the bytes of foundations §10.1. The bracket (through the segment stream) matches.

**NG23. `GetProperty(id, void* buffer, uint32& length)` (holder slot 5).** There is no size query. The original
(`0x005FD290`) finds the property (slot 9) and asks its variant for raw bytes (variant slot 57, `0x00915714`):

1. No property → false.
2. The variant's type is not exactly `0x8000` (raw bytes, foundations §9) → false; `length` is unchanged.
3. Otherwise `length := min(length, count)`, copy `length` bytes into `buffer`, return true.

A caller therefore cannot learn the size: `length = 0` returns true with 0, and a `length` below the count is left as
it was. A null `buffer` with a non-zero `length` and count makes the original copy to address 0 and crash.

Example: a property holding the 5 bytes `01 02 03 04 05`. With `length = 8` → true, `length = 5`, 5 bytes copied.
With `length = 3` → true, `length = 3`, `01 02 03` copied. A `Uint32` property → false.

*[choice]* For a null `buffer`, return false and leave `length` unchanged.

**NG24 (glue). Regional cities held across calls.** The original returns regional cities without AddRef (§4.2,
§6.2) and holds them only inside one call (the list of `GetAdjacentCities` in `Init` and `ProposeDeals`). How long
the glue holds its own references is the implementer's choice.

**NG25. Finding the marker props.** The original never names `0xA823821E` itself. It asks the prop manager for the
**city props** in the inner cell, `GetCityProps(list, rect)` (prop manager slot 18, `0x004A1DB0`), and keeps those
whose bool property `0xCAA46595` is true:

- `GetCityProps` runs the occupant manager's rectangle query with the prop manager's filter. The filter passes an
  occupant only if its type is `0xA823821E` (prop) **and** its prop usage is 1 (city prop, set by `AddCityProp`).
- Deleting a connection (§6.4) removes each match with the prop manager's `RemovePropA` (slot 29). The terrain
  handler (§11.1) moves each match and updates it in the occupant manager.

The bracket's type, `0xA823821E`, is right, but the original also requires usage 1. A lot or street prop carrying
`0xCAA46595` would be ignored by the original and found by a type-only query. Shipped data puts that property only
on the markers, which are city props, so the two agree in practice. *[choice]* Call `GetCityProps`, or add the usage
test.

**NG26. `QueryInterface` on the traffic source.** Slot 0 of the connection's traffic-source part (`0x006D47F0`) is
a thunk to the connection's `QueryInterface` (`0x006CE000`), so the bracket (forwarded to the connection) matches.
That `QueryInterface` answers only `1` and `0x0773FEF2` (the connection's main interface) and `0xE4FDA3D4`
(`cIGZSerializable`). It does **not** answer the traffic-source or property-holder IIDs: those parts are reachable
only through `AsTrafficSource` and `AsISCPropertyHolder`. So `QueryInterface(1)` on the traffic source returns the
connection's main interface (AddRef'd), not the traffic source.

**NG27. `GetProperty(id, uint32&)` (holder slot 8) on an array.** The original (`0x005FD230`) asks the variant for a
`Uint32` (variant slot 25, `0x009176C8`). A scalar `Uint32` is returned as is. Any other type goes through the
variant's generic conversion: the value is turned into text and the text is parsed back as a `Uint32`. For an array
the text is its **first element** only (`%u` for unsigned, `%d` for signed integers, `"1"`/`"0"` for bools), or `"0"`
for an empty array. So a `Uint32` or `Uint16` array gives its first element, and an empty one gives 0; the result is
true in both cases.

Example: the trip-capacity property `0x8A0B4100` as 12 × u16 `{30, 0, …}` reads as 30; promoted to u32 it still
reads as 30.

The bracket (the first element) matches for unsigned integer arrays, which is what connections carry. For float,
64-bit, negative and raw-byte values the result depends on the text parser (`0x009167C5`), which was not traced.

**NG28 (glue). The shadow's statistics tolerance.** The commodity statistics are `double` arithmetic in the original
(§8.2); the tolerance the shadow allows is the implementer's choice.

**NG29. Network types and network flag bits.** The numbers in §6.5 (4, 11, 0, 6, 12, 2, 7, 8, 9, 1) are network
**type** numbers (the SDK's network types: Road 0, Rail 1, Highway 2, Street 3, Pipe 4, PowerLine 5, Avenue 6,
Subway 7, LightRail 8, Monorail 9, OneWayRoad 10, DirtRoad 11, GroundHighway 12). The original passes each to
`cISC4NetworkOccupant` slot 22 (`0x00619520`). That slot turns the type into its flag bit through a table
(`0x00AA7CE8`: `1 << type` for types 0–12) and tail-calls slot 8, which returns whether all bits of the mask are set
in the occupant's network flags.

So the test is `flags & (1 << type) ≠ 0`. A glue that reads the occupant's flag word must shift; one that calls slot
22 must pass the type number, not the bit. Example: an occupant carrying road and rail has flags `0x0003`; slot 22
answers true for 0 and 1, so Road then Rail match, and the later one, Rail, wins (§6.5).

**NG30. `ReloadCity` through `cISC4Region`.** The SDK's `cISC4Region` matches the game's vtable (`0x00A8FA40`) slot
for slot; the slots used here are `GetCity(x, z)` (slot 11, pops 8) and `ReloadCity(holder*&)` (slot 15, pops 4).

What the original `Init` does (step 5 of §14.1; `0x006D3B0C`–`0x006D3BEC`), for each entry `C` of
`GetAdjacentCities(list, false)`, taking entries from the front of the list:

1. AddRef `C`; read its position (regional-city slot 5).
2. `h := region.GetCity(x, z)`, with the region from the app (`cISC4App` slot 35, `+0x8C`). `h` is the region's own
   holder pointer for that cell (a `cRZAutoRefCount<cISC4RegionalCity>*` shared by all cells of the city).
3. If `h` and the city in it are non-null: `region.ReloadCity(h)`, passing the address of the local that holds `h`.
   The result is not read.
4. Remove the entry from the list, release it, then release `C`.

There is no gate and no identity check. Because the list was just built from the region's cities, `h` always holds
`C` itself, so an identity check always passes and is harmless; but a gate that is off skips the reload, which the
original always does.

What `ReloadCity(h)` (`0x004A5030`) does:

1. Take the city's file name; `RemoveCity(h)` (slot 13): erase every cell of the city from the map, release the
   region's reference and **free the holder** `h` points to.
2. Read the file into a new regional city (`SC4ReadRegionalCity`).
3. On success: `InsertCity(new)` (slot 12), then `h := GetCity(new position)` (the new holder).
4. On failure: the city is no longer in the region, and `h` still points to the freed holder.
5. The return value is never set (`AL` is garbage).

So a caller must not use `h` after a failed reload, and must not test the result. The original `Init` does neither.

**NG31. Status reports from a deal with no object.** In the original every deal is an object, so every report of
statuses 0–7 carries it (the message's IGZUnknown, AddRef'd by the message). Only status 8 has none: the simulator
(§7.2), the plumbing simulator and the pollution simulator post it with no deal. Three subscribers read the deal:

| Subscriber | Statuses | What it does with the deal |
|---|---|---|
| power simulator | 0 | if the deal's type is `0x4773FFE0`/`E1` (power), it schedules a power recheck |
| plumbing simulator | 0 | the same for `0x4773FFE2`/`E3` (water) |
| utility advisor | 3, 4, 5, 6 | it creates an advice item and gives it the deal (the message's IGZUnknown), for the advice text |

With a null deal, power and plumbing skip the recheck after a deal change, and the advisor's advice gets no deal.
*[choice]* Create the deal's object (the wrapper) when the report is posted, and put it in the message for statuses
0–7; the message's reference keeps it alive until delivery (NG5). Status 8 stays without a deal.

## 16. Original quirks (reproduced)

1. Deal pricing uses the garbage curve after the first update (§8.3).
2. `simulate_deals` re-runs on its own status reports (§7.2).
3. `SetAmount` is ignored during Grace and Probation (§5.4); `Cancel` during them therefore leaves the amount.
4. Proposals carry amount 0 (cost 1) (§7.1).
5. `amountUsed` is written twice in the deal record; `incAmount2` and `lastIncAmount` are not saved (§5.6).
6. Marker props are placed at `GetAltitude` height but moved to `GetMaxSurfaceLevel` height on terrain changes.
7. `CreateConnection`'s SDK return value is not meaningful.

## 17. Open questions

1. The regional-city record format (connections, local deals, commodity statistics); needed for standalone mode
   and to verify what the region writes for our deals.
2. Exactly which deals `cISC4RegionalCity::SetLocalDeals` keeps (the analysis suggests: deals between that city
   and the pushing city, plus all deals for the current city's own record), and what `UpdateImportExport`
   computes from them.
3. Which UI calls `ProposeDeals`, `SetUIDisableFlag` and `AddDeal`, and how the deal amount is chosen.
4. Who sets `SetIsActive` / `SetVisitedThisCycle` on connections and the `inc`/`last`/`used` amounts on deals
   (utility simulators, per their specs).
   *Answered for water* (plumbing spec): the plumbing simulator sets the connection's active and visited bits and
   the deal's `AmountUsed`. It also sets deal flag `0x08` on deals it cannot connect, and flag `0x20` on export
   deals it could not fully deliver. *Answered for power* (power §6.3–§6.5): the power simulator sets
   `SetIsActive` / `SetVisitedThisCycle` on PowerPole connections (type 4) and the deals' `AmountUsed`, and sets
   flag `0x20` on sell deals it cannot supply.
5. *(partly answered)* `0xA8C86142` is `cISC4PowerPoleOccupant` (power spec); the meaning of its 14th method
   (compiled slot 13, a byte flag that also makes power ignore the pole) is still unknown.
6. The full rule of `UpdateTrafficVolume` (traffic spec).
7. Order of month agents (budget vs. neighbours), shared with the budget spec.

## 18. Conformance tests (must exist in `engine/conformance`)

1. **Geometry**: side classification for (−1, 5), (64, 5), (5, −1), (5, 64), (5, 5) in a 64×64 city; neighbouring
   region cell and neighbour connection cell for each side with a 1×1 city at (3, 3) and 2×2 neighbours; corners
   are invalid.
2. **Connections**: `CreateConnection` twice creates one object, posts `0x8A0D15DA` once, places one marker prop
   with the documented orientation for both flag values and none for Ferry; `DeleteOccupantConnection` for a
   network occupant carrying road and rail deletes the Rail connection; corner cells delete two; save
   round-trip of the simulator and connection records; version 3 rejected.
3. **Price** (scarcity curve, power buy, amount 100): buyer demanded 500, exported 0, produced 300, imported 0;
   seller produced 2000, imported 0, demanded 1000, exported 0, cost per unit 2.0; base 1000, scale 2.5, markups
   0.1/0.5 → unit 2.6314286, cost trunc 263. The same with the garbage curve → unit 2.2150418, cost 221.
4. **Max amount** for the same data → 950 (shortfall 950, surplus 1000); with an existing deal of amount 100 →
   1050 (shortfall 1050, surplus 1100); with the seller's demanded raised to 1900 → 100.
5. **Curve bug**: after one `UpdateCommodityStats` with a sanitation department, test 3 yields 221; without the
   department or the property it still yields 263.
6. **Life cycle** (grace 90, probation 90): a proposed deal activates on the first run, posting status 0, 0,
   then 1 (§7.2); losing the connection sets Grace and suspension = today (no removal); failures during Grace are ignored; after 90 days
   Probation; a failure during Probation removes it with status 3; after 180 days Probation clears; a cancelled
   deal is removed with status 2; expiry posts 6, and the next run renews with the same span (status 7) or flags
   CannotAfford.
7. **Garbage capacity**: two garbage-sell deals of 60 with exported 100 and capacity 50: the first gets
   NoLandfill and reduces the export budget to 40, the second is then unflagged.
8. **Totals**: buy deals count as expense, sell deals as income; flags `0x01`, `0x04`, `0x08`, `0x10`, `0x20` exclude
   a deal; `0x40`, `0x80`, `0x100`, `0x200` do not.
9. **Re-entrancy**: a status message delivered during a run posts `0xEA418D8F` with data1 = 8 and changes nothing;
   status 0/8 set statsDirty, others dealsDirty.
10. **Proposals**: with a 1-year term the expiration is today + 365; amount 0 and cost 1; no proposal when an
    existing deal of that type exists, when the payer's budget ≤ 5000 or ≤ 60 × cost.
11. **Clone**: a deal whose city side is the neighbour comes back swapped with E0 ↔ E1; missing region cells give
    a Cancelled clone; the Silent flag follows the argument.
12. **Deal record**: write → read → write is byte-identical for version 5; a version-4 record reads without the
    suspension date.

## Compiled slot tables (round 2) *[compat]* (see foundations §16, §17)

Verified slot by slot against the original Windows vtables (`ret` operand = stack-argument bytes, SDK header
compiled under the MSVC overload rule). Equal-size pairs were told apart by what the body does.

### `cISC4NeighborsSimulator` (original vtable `0x00AB1EE0`, 38 interface slots)

| Slot | Offset | Method (compiled layout) |
|---|---|---|
| 7 | `+0x1C` | `void DeleteConnection(ConnectionType, int32 x, int32 z)` |
| 8 | `+0x20` | `void DeleteOccupantConnection(cISC4Occupant*)` |
| 13 | `+0x34` | `cISC4NeighborConnection* FindConnectionOnOtherSide(cISC4NeighborConnection*)` |
| 14 | `+0x38` | `cISC4NeighborConnection* FindConnectionOnOtherSide(ConnectionType, int32 x, int32 z)` |
| 28 | `+0x70` | `void FindDealsAll(SC4List<…>& deals)` |
| 29 | `+0x74` | `void FindDeals(int32 dealType, cISC4RegionalCity*, SC4List<…>& deals)` |
| 30 | `+0x78` | `void FindDeals(int32 dealType, SC4List<…>& deals)` |

In the original source, `DeleteOccupantConnection` and `FindDealsAll` were overloads of `DeleteConnection` and
`FindDeals`, under different groupings. The SDK gives them unique names, and the compiled SDK reproduces the
slots above exactly. Keep these slots when renaming. All other slots match the SDK order and argument sizes.

### `cISC4NeighborConnection` (`0x00AB2034`, 15 slots) and `cISC4NeighborDeal` (`0x00AB1DD8`, 41 slots)

No overloads. Order and argument sizes match the SDK. The deal's date getters (`GetStartDate`, …) fill a caller's
`cIGZDate&` through that date's `Set(uint32 dayNumber)` (`cIGZDate` slot 4, foundations §17.4).

### `cISC4RegionalCity` (the game's regional-city object, vtable `0x00A8FB20`, 73 slots)

The neighbours simulator **calls** this interface, so its compiled layout and true signatures matter:

| Slot | Offset | Method (compiled layout) | Note |
|---|---|---|---|
| 9 | `+0x24` | `int32 GetPopulation(uint32 populationType) const` | **non-adjacent** overload (the SDK declares it 9 lines later) |
| 10 | `+0x28` | `int32 GetPopulation() const` | non-adjacent; `GetCommercialJobs` … `GetTaxRate` follow at 11–16 |
| 24 | `+0x60` | `void UpdateCityCache(cISC4City*)` | **non-adjacent** (checked by body: it calls city methods) |
| 25 | `+0x64` | `void UpdateCityCache(SC4NewCityPreferences*)` | non-adjacent; `SetupCity` follows at 26 |
| 47 | `+0xBC` | `float GetWorldBaseElevation() const` | **SDK error**: the SDK declares a `GetWorldBaseElevation(float)` overload, which MSVC would put **first** (slot 47). In the game, 47 is the getter |
| 48 | `+0xC0` | `void SetWorldBaseElevation(float)` | the SDK's `GetWorldBaseElevation(float)` is this setter |
| 50 / 51 | `+0xC8` / `+0xCC` | `double GetBudget()` / `void SetBudget(double)` | **SDK error**: `double`, not `float` (the setter pops 8 bytes) |
| 52 / 53 | `+0xD0` / `+0xD4` | `double GetIncome()` / `void SetIncome(double)` | same |
| 54–63 | `+0xD8`–`+0xFC` | `double GetX(int32 commodity)` / `void SetX(int32 commodity, double)` for X = Exported, Imported, Produced, Demanded, CostPerUnit | **SDK error**: `double` values (setters pop 12 bytes) |
| 64 | `+0x100` | `double GetCommodityBalance(int32 commodity)` | computed in double precision |

The getters return the value on the x87 stack, so a caller that declares `float` still reads a correct value,
rounded. The setters must be called with `double` arguments, or the stack is corrupted. §8.2
(`UpdateCommodityStats`) calls `SetBudget`, `SetIncome`, `SetProduced` and the other setters. The stored values are the `f64` fields of the
regional-city record (foundations §11.2).
