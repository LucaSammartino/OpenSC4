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

"Failure flags" are `0x38`. Totals and pricing ignore deals with any of `0x3D` (§10.1).

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
| `UpdateCost()` | Unless Grace or Probation: `cost := trunc_s64(neighbours.GetDealCost(type, amount, GetCityOnOtherSide()))` through the **current city's** neighbours simulator; if that is 0, `cost := 1`; report status 0 (posted). |
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

Then find the occupant's cell `(cx, cz)`. If `cx == 0` delete the connection `(type, −1, cz)`; else if
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

Only when `city` is the current city. With `R` = the current regional city:

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
| `0xEA46934F` | 1 | 5 in proposals, 0 in the deal simulation, unset in `UpdateCommodityStats` (*[choice]* 0 → treat the free-landfill term as 0) | landfill horizon and maximum deal term (years) |
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

## 14. Lifecycle

### 14.1 `Init()` → true

If not initialized:
1. Set initialized. Load the tuning exemplar if not yet held; load the scarcity curve from `0x0A469353`.
2. Register agents at levels **6 (month), 4 (day) and 10 (idle)**, name `"cSC4NeighborsSimulator"`, flags 2.
3. Hold the message server; subscribe to `0xAA7A014D`, `0x278E6003`, `0xEA418D8F`, `0x6AC284F3`, `0x99EF1143`,
   `0x26D31EC5`, `0x26D31EC4`. Hold the city.
4. `Repair(R)` every connection with the current regional city `R`.
5. For each adjacent established city: `region.ReloadCity` it (refreshes its record from disk).
6. `R.UpdateLocalDeals()`; clear the deal list.
7. For each adjacent established city `C ≠ R` (in reverse list order): `C.UpdateLocalDeals()`; for each deal `d`
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
