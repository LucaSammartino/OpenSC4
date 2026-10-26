# Spec: the regional city (`cSC4RegionalCity`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [foundations.md](foundations.md) §11 (the regional-city record, its read rules, the two deal lists,
the messages and TerrainReconciliation; this spec does not repeat them), [neighbors.md](neighbors.md) (§2.2 the cell
conversions, §4 connections, §5 deals, §8.2 `UpdateCommodityStats`, and the compiled slot notes at its end),
[demand.md](demand.md) (`CalculateJobsPerUnitOfDemand`, `GetLocalPopulationSummary`), [budget.md](budget.md)
(`GetTaxRate`), [aura.md](aura.md) (`GetMayorRating`).

## 0. Summary

One regional city object stands for each city tile of the region: its place and size, the statistics the region
view shows (population, jobs, mayor rating, tax rates, budget, the commodity balances), its names, its connections
to the neighbours and the deals that cross them. The current city's object is refreshed from the live city at
PostCityInit and PreSave; the others come from their city files.

## 1. Identity *[compat]*

- Class ID `0x6A037C24` (`GetGZCLSID`, `0x004ABA60`); factory `GZCOM_CreatecSC4RegionalCity` (`0x004AD9A0`, 0x1A0
  bytes, registered by the COM director at `0x00460399`). `SC4ReadRegionalCity`'s fallback (`0x005DC0EF`) and
  the region screen (`0x007AEAD4`) construct it directly (constructor `0x004AC040`).
- **`SC4ReadRegionalCity(package, &out)`** (`0x005DC070`): opens the record `0xCA027EDB / 0xCA027EE1 / 0` of the
  package (auto-load stream); without it, false. The object comes from `GZCOM()` (`CreateInstance(0x6A037C24,
  0x69D2BE1D)`), or is constructed directly when that fails; its serializable view reads the stream; on success
  `out` := the object (a reference added) and its `Init` runs. The result is the read's.
- Three views: `+0x00` the regional-city interface (vtable `0x00A8FB20`, 74 slots), `+0x04` the message target
  (`0x00A8FB10`, `DoMessage` `0x004ABD40`), `+0x08` the serializable (`0x00A8FAF8`: `Write` `0x004A7D10`, `Read`
  `0x004AC5B0`).
- **`QueryInterface`** (`0x004A74E0`): 1 and `0x69D2BE1D` → the object; `0x452294AA` → `+4`; `0xE4FDA3D4` → `+8`;
  each adds a reference; other IIDs false.
- **`Release`** (`0x004A7800`): a count above 1 is decremented and returned; otherwise `Shutdown` (slot 4) runs, then
  the deleting destructor (slot 73, `0x004AC590`; body `0x004AB7F0`), and 0 is returned.

## 2. Fields

| Offset | Field | Made as |
|---|---|---|
| `+0x0C` | initialised (byte) | 0 |
| `+0x10` | reference count | 0 |
| `+0x14`, `+0x18` | position x, z (region cells) | 0, 0 |
| `+0x1C`, `+0x20` | size x, z (region cells) | 1, 1 |
| `+0x24`, `+0x28`, `+0x2C` | population, commercial jobs, industrial jobs | 0 |
| `+0x30` | workforce fraction (f32) | 0.5 |
| `+0x34` | mayor rating (s8) | 0 |
| `+0x38` | difficulty | the application's default (`0x00B43CD8` `+0x46C`) |
| `+0x3C`, `+0x48`, `+0x54` | population, extrapolated and potential population maps (type → s32) | empty |
| `+0x60`–`+0x8C` | the 12 tax rates (f32, §4.2) | 7.0 each |
| `+0x90` | connection list | empty |
| `+0x94` | pending connection changes (16-byte records, §6.2) | empty |
| `+0xA0` | moved (byte) | 0 |
| `+0xA4` | serial number | 0 |
| `+0xA8`–`+0xB4` | original language, country; last language, country | 0 |
| `+0xB8` | birth date | 0 |
| `+0xBC` | save-file path (string) | empty |
| `+0xD0`, `+0xF8`, `+0x120`, `+0x154` | city name, mayor name, description, utility advisor name | empty |
| `+0xE4`, `+0x10C`, `+0x134` | the record's three unknown strings | empty |
| `+0x148`, `+0x14C`, `+0x150` | world longitude, latitude, base elevation (f32) | 0 |
| `+0x168` | established (byte) | 0 |
| `+0x16C` | tutorial ID | −1 |
| `+0x170`, `+0x178` | budget, income (f64) | 0 |
| `+0x180` | commodity map (commodity → five f64, §5) | commodities 1, 2, 3 at 0 |
| `+0x18C`, `+0x190` | the deal lists O and L (foundations.md §11.3) | empty |
| `+0x194` | points of interest (type → list of three-float positions) | empty |

The record (foundations.md §11.2) stores these fields; its "moved" byte is `+0xA0`.

**Read edge cases (RY7):** these are constructor defaults, not replacement-map rules. The native object's
in-place `Read` retains absent commodity keys (including 1–3), overwrites duplicate scalar-map entries and
appends duplicate POI lists. Failure can leave a partly modified receiver. See foundations.md §11.2 for the
unsigned count guards, nonzero deal loops, wrapper publication rule and worked cases.

**Opaque strings (RY2).** Treat `+0xE4`, `+0x10C`, `+0x134` as independent stored strings, called
`s1`, `s2`, `s3` in foundations.md §11.2. Their wire positions are respectively after city name, after
mayor name, and after description (utility advisor name precedes description on the wire). They use the
same string encoding as the named strings. Construction makes them empty; Read fills them and Write
preserves them. There is no public getter/setter for these three strings in slots 0–73, and neither cache
overload assigns a meaning to them. Do not alias them to the adjacent names, discard nonempty values or
populate them from locale guesses. Read's string calls are `0x004AC73B`–`0x004AC795` (serializable-view
offsets are eight bytes below the object offsets in the table).

Worked preservation case: in an established version-13 record, set `s1="alpha"`, `s2="beta"`, `s3="gamma"`
with distinct city/mayor/description strings. Read then Write must preserve those three distinct strings
in the same wire positions; changing city or mayor name through its public setter must not change them.
The semantic labels remain unproven: a runtime observation that would settle them is a nonempty value
created by the original with the **writer call stack** captured. Watch the three string objects while
founding, naming, renaming, importing and localizing a city; distinguish Read restoring supplied test
values from an original producer. Empty shipped samples alone cannot supply a semantic name.

## 3. Slots *[compat]*

The compiled order is the SDK's, except where neighbors.md's slot notes say otherwise (overloads grouped).

- 3, 4: `Init`, `Shutdown` (§4.1; `0x004A7830`, `0x004A8470`).
- 5–8: `GetPosition` (`0x004ABA70`), `SetPosition` (§6.1, `0x004A99F0`), `GetCitySize` (`0x004ABA90`),
  `SetCitySize` (`0x004ABAB0`).
- 9, 10: `GetPopulation(type)` (`0x004A8500`), `GetPopulation()` (`0x006BDD80`).
- 11–15: `GetCommercialJobs`, `GetIndustrialJobs`, `GetWorkforcePercentage`, `GetMayorRating`,
  `GetDifficultyLevel` (getters shared with other classes).
- 16: `GetTaxRate(type)` (§4.2, `0x004A7580`).
- 17–19: `GetExtrapolatedPopulation`, `GetAllowableExtrapolation`, `ExtrapolateGrowth` (§4.3; `0x004A8530`,
  `0x004A9820`, `0x004A9860`).
- 20–22: `FindConnection`, `GetAllConnections`, `ChangeSymmetricConnection` (§6.2; `0x004A7B40`, `0x004A8DB0`,
  `0x004A8DF0`).
- 23–26: `SetupPreferences` (`0x004A78A0`), `UpdateCityCache(city)` (`0x004AD290`), `UpdateCityCache(preferences)`
  (`0x004AA4F0`), `SetupCity` (`0x004A8960`) (§7).
- 27–30: the serial number's getter and setter (`0x006F6F70`, `0x004ABAE0`); the original and the last language and
  country (`0x004ABAF0`, `0x004ABB10`).
- 31–40: getters and setters of the save-file path, the city name, the mayor name, the utility advisor name and the
  description (`0x004ABB30`–`0x004ABC10`).
- 41–49: the birth date, the established flag, the world position and base elevation, `GetWorldHemisphere`
  (`0x004ABC20`–`0x004ABCC0`).
- 50–53: `GetBudget`, `SetBudget`, `GetIncome`, `SetIncome`, doubles (`0x004A7640`–`0x004A7670`).
- 54–63: getters and setters of exported, imported, produced, demanded and cost per unit (§5;
  `0x004A9EE0`–`0x004AA080`).
- 64: `GetCommodityBalance` (§5, `0x004A7680`).
- 65–67: `GetTutorialGUID`, `SetTutorialGUID`, `IsTutorial` (`0x004ABCE0`, `0x004ABCF0`, `0x004A76E0`).
- 68–71: `UpdateLocalDeals`, `SetLocalDeals`, `GetLocalDeals`, `UpdateImportExport` (foundations.md §11.3;
  `0x004A9110`, `0x004A90D0`, `0x004AA0C0`, `0x004AA200`).
- 72: `GetPointsOfInterest(type, list)` (§7.3, `0x004A8A40`).
- 73: the deleting destructor (`0x004AC590`).

Notes: the string getters copy into the caller's string (its `operator=`), the setters copy from it; `IsTutorial` is
`GetTutorialGUID() ≠ −1`; `GetWorldHemisphere` is 1 when the latitude is above 0, otherwise 2 (also for a NaN)
*[inferred: 1 northern, 2 southern]*; `GetOriginalLanguageAndCountry` and `GetLastLanguageAndCountry` return true.

## 4. Life and statistics *[compat]*

### 4.1 Init and Shutdown

- **`Init`** (once): subscribes the message view to PostLoad `0x26C63342`, PreSave `0x26C63343`, PostCityInit
  `0x26D31EC1`, PostCityShutdown `0x26D31EC3` and TerrainReconciliation `0xCA63D517` (foundations.md §11.3). True.
- **`Shutdown`** (once Init has run): the five subscriptions are removed, and the connection list and both deal
  lists are emptied (their references released). True.

### 4.2 Tax rates

`GetTaxRate(type)` maps the twelve tax groups to `+0x60`–`+0x8C` in this order: `0x1010`, `0x1020`, `0x1030`,
`0x3110`, `0x3120`, `0x3130`, `0x3320`, `0x3330`, `0x4100`, `0x4200`, `0x4300`, `0x4400`. Any other type returns
7.0. `UpdateCityCache(city)` refreshes them by index 0–11 from the budget simulator's `GetTaxRate` (budget.md).

### 4.3 Population maps

- `GetPopulation(type)`: the population map's value, 0 when absent. `GetExtrapolatedPopulation(type)`: the
  extrapolated map's value; when absent, `GetPopulation(type)`.
- `GetAllowableExtrapolation(type)`: `max(potential[type] − extrapolated[type], 0)`.
- `ExtrapolateGrowth(type, add)`: `extrapolated[type] := trunc(float(extrapolated[type]) + add)`.
- These two read both maps with insertion: a missing type gets a 0 entry, which the record then saves; so
  `ExtrapolateGrowth` on a type without an extrapolated entry starts from 0, not from the population *[quirk]*.
- **PreSave** on the current city's record (foundations.md §11.3) runs `UpdateCityCache(city)`, then:
  - **potential**: for each type `t` of the population map with `p := population[t]`, `potential[t] := 0` when
    `p ≤ 0`, else `trunc(max(1.1 · p, 1000))` (float, `0x004A9890`; a product below 1000 gives 1000);
  - **extrapolated**: `extrapolated[t] := population[t]` for every type of the population map (`0x004A9970`).

## 5. Commodities *[compat]*

The commodity map holds, per commodity (1 power, 2 water, 3 garbage, neighbors.md §3), five doubles: produced,
demanded, exported, imported, cost per unit. The getters and setters of slots 54–63 work on the entry of the given
commodity, **creating a zero entry** for an unknown one *[quirk]*; every setter stores 0 for a negative value
(a NaN is stored).

`GetCommodityBalance(c)` (slot 64) is `(produced + imported) − (demanded + exported)`, computed in double precision
through the slots (produced, then imported, then demanded, then exported).

`UpdateImportExport` and the deal lists are foundations.md §11.3; neighbors.md §8.2 sets budget, income and the
commodities through these setters.

## 6. Position and connections *[compat]*

### 6.1 Moving

`SetPosition(x, z, rearrange)`: the position is stored. With `rearrange`:

1. `moved := 1`;
2. every own connection is repaired with this city (connection slot 10, `Repair`, neighbors.md §4.2), so its other
   region cell follows the move (`0x004A9720`);
3. with `(dx, dz)` the change of position, every deal of O and then of L moves **both** its region cells, the city
   side (deal slots 4 and 3) and the other side (deal slots 6 and 5), by `(dx, dz)` *[quirk]*.

The TerrainReconciliation that follows a load of the moved city rebuilds the connections (§6.2) and clears `moved`.

### 6.2 Connections

- **`FindConnection(type, x, z)`**: the first connection of the list with that type (connection slot 5) and cell
  (slot 6), without a reference; null otherwise. **`GetAllConnections(list)`** empties `list` and copies the
  connection list into it.
- **Pending changes.** Each record is `{type, x, z, flag}` (u32, s32, s32, u8; flag 1 to create, 0 to remove
  *[inferred]*), saved with the record (foundations.md §11.2). They are added by:
  - `ChangeSymmetricConnection(conn, flag)`: the connection's cell turned into the neighbour's cell
    (`ConvertConnectionCellToNeighborCell`, neighbors.md §2.2) and its type, with `flag`;
  - **dropping** (`0x004A8570`): the pending list is emptied;
  - **own connections to remove** (`MarkConnectionsForRemoval`, `0x004A9580`, after a move): every own connection
    whose cell has a city on the other side (the neighbours simulator's `FindCityOnOtherSideOfConnection`, its slot
    12) that is established (that city's slot 43) adds `{type, its cell, 0}`;
  - **incoming as new** (`TreatAllIncomingConnectionsAsNew`, `0x004A9220`): for every adjacent city, established or
    not (the neighbours simulator's `GetAdjacentCities(list, true)`, slot 20), each of its connections whose
    other-side city is this one adds `{type, this side's cell, 1}`, the cell from the inverse conversion of
    neighbors.md §2.2 (`0x006CEBA0`).
- **TerrainReconciliation** (foundations.md §11.6), on the current city's record: when the city is not
  established, the message's data3 is set or `moved` is set, the dropping, the own removals (only when moved) and
  the incoming-as-new step run in that order; then `CreateSymmetricConnections`; then `moved := 0`.
- **`CreateSymmetricConnections`** (`0x004AA6D0`) *[summary; RY1]*: duplicate pending records (same type and cell) are
  removed; the rest are sorted by edge (west `x < 0`, north `z < 0`, east, south), then x, then z (`0x004A78E0`); for
  the flagged records of types 3, 8 and 12 a running parity, flipped on the north and east edges and by the drive-side
  setting (`0x00B21CC1`), gives each a direction; each record then finds or creates its connection through the
  neighbours simulator (`FindConnection`, `CreateConnection`) and builds or removes the network, pole or power-line
  pieces at the edge cell (types `0xC772BF98`, `0x2890D4DE`, `0x0990B2EA`), demolishing and dezoning what is in the way
  (demolition utility slot 6, zone manager), and updates the edge network occupants' flags. It always ends by posting
  ConnectionsReady `0x6AC284F3` (foundations.md §11.6).

### 6.2.1 Border-strip demolition host call (RY9) *[compat]*

The add path that actually places a piece (`pairFlag` true) uses the strip and cell exclusions in
[regional-city-records.md](regional-city-records.md) §4.6 step 1. Let **R** be that masked cell region. Before
creating the connection or placing its network piece, it calls demolition utility slot 6 (`+0x18`,
`DemolishRegion`) with these **eleven arguments, in interface order**:

| Argument | Value |
|---|---|
| `doIt` | **true** (actual demolition, not a preview) |
| `R` | the three-cell strip's region, with the already-occupied cells excluded as above |
| `privilege` | **0** |
| `flags` | **`0x00000001`** |
| `dezone` | **false** |
| `callerFilter` | a **view-level occupant filter**, with the level below |
| `cost` | null |
| `demolished` | null |
| `clicked` | null |
| `sx` | 0 |
| `sy` | 0 |

The argument values and dispatch are verified at `0x004AAE73`–`0x004AAE96`. The result is ignored: it does not
gate the following connection creation or placement (`0x004AAE99` onward). Removal entries make **no** such
demolition call (regional-city-records §4.7). PowerPole adds use the same arguments, including `dezone=false`;
their **separate subsequent** zone-manager call dezones R (§4.6 step 3, `0x004AAEA2`–`0x004AAECD`).

| Incoming connection | Caller filter's level |
|---|---|
| Pipe (type 5) | **3**, pipe layer |
| Subway (type 6) | **2**, subway layer |
| Every other supported network connection, including PowerPole (type 4) | **1**, surface |

Level selection is verified at `0x004AAA56`, `0x004AACCF`–`0x004AACF6`; the filter receives it at
`0x004AAE55`–`0x004AAE69`. This is `cSC4ViewLevelOccupantFilter`, whose level is at `+0x0C`, **not** a filter
restricted to network occupants. Its occupant query (slot 3, `+0x0C`) accepts exactly those occupants for which
`SC4GetOccupantViewLevel` equals the selected level (`0x005E9E40`–`0x005E9E5B`). Its type-only query (slot 4,
`+0x10`) accepts **every occupant-type ID**; its property-holder query (slot 5, `+0x14`) also always accepts
(`0x004FC900`, entries at `0x00A8FAE4` and `0x00A8FAE8`). Neither query substitutes for the occupant query.
The full layer classifier, including interface, group and property precedence, is specified in
[occupant-manager.md](occupant-manager.md) §13.2.4. In particular, the caller filter does not read the current
3D view's selected level. These filter-only examples assume no further classifier overrides:

| Occupant instance | Classified level | Surface filter (1) | Subway filter (2) | Pipe filter (3) |
|---|---|---|---|---|
| Ordinary surface building, flora, real prop, power pole or surface network | 1 | admit | refuse | refuse |
| Pipe or pipe connection | 3 | refuse | refuse | admit |
| Subway or subway connection | 2 | refuse | admit | refuse |
| Prop with `0xCAA45670` and `0xCAA45672` true, `0xCAA45671` false | 3 | refuse | refuse | admit |
| Prop with `0xCAA45670` and `0xCAA45671` true | 2 | refuse | admit | refuse |
| Prop with only `0xCAA45671` true | 1 | admit | refuse | refuse |

These admissions are still subject to [demolition.md](demolition.md) §§4.2–4.4. In particular:

- Privilege **0** selects removal level **1**, so the **system lock** blocks removal; the player lock alone
  does not. The utility's occupant filter does **not** apply the current view filter, despite `0x2000` being
  clear (`0x0046847D`–`0x004684AA`, `0x00468515`–`0x00468524`).
- The utility still applies realism, cell bounds and its own type gate: building `0x278128A0`, power pole
  `0x2890D4DE`, construction `0x096E763C`, prop `0xA823821E`, network `0xC772BF98`, flora `0x74758926`, and
  foundation `0x68FD0C69` only when its foundation-interface slot 9 returns 1. Other types are refused even if
  the caller filter admits their layer (`0x004667E7`–`0x0046688C`).
- `flags=1` enables demolition effects, with the utility's usual **current view filter** gate on those effects.
  Its empty-lot type test also still consults that view filter. Independently, when the current view level is
  not 1, its network-only check still refuses occupants with any network flag in `0x1F4F`. Privilege 0 does
  not bypass these tests (demolition §§4.2, 4.4, DM12; `0x004668AE`–`0x004668ED`).
- There is no confirmation request or budget charge, and no cost is returned to this caller. `dezone=false`
  does not suppress the utility's normal special-zone handling (demolition §4.4 step 7).

**Worked border request.** In a 64-by-64 city, add a Road connection at west-edge cell `(-1, 10)`, with no
existing connection. The strip is `(0, 10)`, `(1, 10)`, `(2, 10)`. If the traffic network map already reports
occupation at `(1, 10)`, R contains only `(0, 10)` and `(2, 10)`. Expect the eleven arguments above, with a
**level-1** caller filter. Place an ordinary flora occupant at the centre of each selected cell, neither on a
lot: at `(0, 10)` set only the player lock; at `(2, 10)` set the system lock. Let the current view filter reject
both flora occupants, with current view level 1 and a working flora simulator. The first occupant passes the
caller and removal filters and is demolished **without an effect**; the second is refused by `CanRemove(1)`
and survives. Nothing at the excluded cell `(1, 10)` is selected by this demolition request. As isolated
predicate checks, the same level-1 caller filter refuses a pipe and a subway, but admits a surface occupant of
an unsupported type; the utility then refuses that unsupported type. Repeating those predicate checks with
levels 2 and 3 gives the admissions in the table. Later network placement has its own outcome; it does not
change these demolition inputs.

If allocation of the 16-byte caller filter fails, the original still makes the same demolition call with a
**null caller filter**, losing only this additional layer restriction (`0x004AAE3A`–`0x004AAE48`,
`0x004AB0FC`–`0x004AB0FE`); the utility's own filters remain in force.

## 7. The city cache *[compat]*

### 7.1 New-city preferences

- **`SetupPreferences(prefs)`**: the city name into `prefs + 0x000` and the mayor name into `prefs + 0x200` (C
  strings), the difficulty into `prefs + 0x400`.
- **`UpdateCityCache(prefs)`**: the reverse: city name, mayor name and difficulty from `prefs`; the save-file path
  := the drive and directory of the current path, the city name and `.sc4` (`0x00A87054`).
- **`SetupCity(city)`**: the city's mayor name (city slot 17), city name (slot 13) and difficulty (slot 25) from this
  record; its save-file path (slot 11) from `SC4CreateSavePathForCity(city name, 0, &path)`; its size (slot 88) :=
  `(sizeX · 1024, sizeZ · 1024)` metres.

**Caller boundary and path lifetime (RR8).** Slots 23 (`+0x5C`, `0x004A78A0`) and 25 (`+0x64`,
`0x004AA4F0`) have no direct callers in the executable export. Virtual calls cannot be excluded from
that count; no positively resolved native preference caller is established by this analysis of the
region screen, region and application paths. The public interface itself permits clients to call them.
Do not make the core depend on a native dialog calling them in a particular order.

Slot 25's new path is stored immediately and is observable by `GetFilePath` (slot 31); it is not a file
rename, file creation or package write. It uses the old drive/directory, the new city name and literal
`.sc4`, without `City - `. This value remains until another path assignment, including the live-city
cache copy in §7.2; `SetupCity` obtains its live path separately through `SC4CreateSavePathForCity`.
Thus it is not justified to assume that the next save is the first possible user of the temporary path.

Worked case: starting with `C:/Regions/R/City - Old.sc4`, preferences `New`, `Mayor`, difficulty 1 give
names `New`/`Mayor`, difficulty 1 and path `C:/Regions/R/New.sc4` (native separator spelling follows the
path helper). Reading preferences back through slot 23 reproduces those names/difficulty. The operation
alone neither renames `City - Old.sc4` nor creates `New.sc4`.
To settle actual stock callers and intervening disk use, break on both method addresses and on slot-31
path reads and slot-32 writes for the same object while opening, accepting and cancelling founding/rename
dialogs and importing a city. From each slot-25 return until the next path assignment, log filesystem
opens and correlate their paths with the stored value, including return-to-region before saving.
This observation is required before claiming the prefix-free path is unused.

### 7.2 From a live city (`UpdateCityCache(city)`)

In this order: the mayor name (city slot 16), the city name (slot 12), the difficulty (slot 24), the save-file path
(slot 10); the serial number and established flag through this record's own setters (slots 28 and 44) from city
slots 5 and 22; the mayor rating from the aura simulator (city slot 51, its slot 8); the 12 tax rates (§4.2); the
neighbours simulator (city slot 62) copies its connections into the connection list (its slot 16) and runs
`UpdateCommodityStats(city)` (its slot 34, neighbors.md §8.2); when the tutorial system (slot 82) exists and the
tutorial ID is −1, the tutorial ID := its `GetTutorialGUID`; when the residential simulator (slot 68) exists, the
population := its `GetPopulation` and the workforce fraction := its `GetGlobalWorkforcePercent`.

Then the demand simulator (city slot 56) fills the population map (`GetLocalPopulationSummary`, demand.md) and the
jobs are computed: for each type `t`, `jobs(t) := trunc(population[t] · s)` with `s` the sum
`(j[3] + j[2]) + j[1]` of its `CalculateJobsPerUnitOfDemand(j, t, 0)` (the arithmetic of demand.md's
`GetJobsBySensus`); commercial jobs := the sum over `0x3111`, `0x3121`, `0x3131`, `0x3321`, `0x3331`, industrial jobs
:= the sum over `0x4101`, `0x4201`, `0x4301`, `0x4401`.

### 7.3 Points of interest

The same call empties the points of interest and lists the lots of the airport (`0x00B43D2C`, type 0) and seaport
(`0x00B43D30`, type 1) developers (their slot 8): each lot in a livable state (lot slot 37) adds its centre (lot slot
15). `GetPointsOfInterest(type, list)` copies the type's list into `list`, leaving it unchanged for an unknown type.

## 8. Quirks

1. Reading the extrapolation or a commodity creates zero entries that are saved (§4.3, §5).
2. A moved city shifts both region cells of its deals (§6.1).
3. Unknown tax types read as 7.0 (§4.2).

## 9. Open questions

See [questions/regional-city.md](questions/regional-city.md): RY1 (the per-record rules of
`CreateSymmetricConnections`), RY2 (the three unknown strings of the record).

## 10. Conformance tests

1. **Allowable extrapolation:** potential 1500 and extrapolated 1800 give 0; potential 1500 and extrapolated 1200
   give 300.
2. **Potential:** a population of 500 gives 1000; 2000 gives 2200; 0 gives 0.
3. **Balance:** produced 10, imported 5, demanded 8, exported 2 give 5.
4. **Clamp:** `SetExported(1, −3)` stores 0.
5. **Move:** `SetPosition(5, 4, true)` from (3, 4) moves a deal's cells (3, 4) and (2, 4) to (5, 4) and (4, 4) and
   sets `moved`.
6. **Tax rate:** `GetTaxRate(0x3320)` reads the seventh rate; `GetTaxRate(0x9999)` returns 7.0.
7. **Border demolition host:** the masked west-edge Road request and the three caller-filter policies in
   §6.2.1 use privilege 0, flags 1 and no demolition dezone; a player-locked occupant with no system lock is
   removed, a system-locked occupant survives, and hidden eligible flora is removed without a visual effect.
