# Spec: the civic building simulator (`cISC4CivicBuildingSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, references §2.7, components §3.6, messages §5,
shared IDs including the preview-occupant property §5.2, random numbers §6), [budget.md](budget.md) (departments, line items and
budget items, §6–§8, §11), [lot-developer.md](lot-developer.md) (the lot-state message, §3.2, §6.5),
[advisors.md](advisors.md) (reward advice and the Lua reward functions, §6.6),
[ordinance.md](ordinance.md) (legalised gambling), [building-development.md](building-development.md) (building
keys, hash-container order), [pollution.md](pollution.md) §6 (the occupant-list record format),
[parks.md](parks.md) (the zoo-escape sibling).

## 1. Role

The civic building simulator keeps the bookkeeping of **reward and business-deal buildings** (city halls, statues,
the stock exchange, the casino, the army base and so on):

1. a set of **exclusion IDs**: one ID per unique building that already stands in the city. The building menu greys
   out a building whose ID is in the set ("already exists in this city");
2. a map of **conditional building statuses**, one per reward building type: whether it is hidden, available, and
   how many stand in the city, plus the tooltip text that lists the missing requirements. The advisors and the Lua
   reward functions write it; the building menu reads it;
3. the **maintenance and income** of these buildings: when a building's lot becomes occupied it charges the
   building's maintenance to its department's line item, or credits a business deal's income; when the building
   is removed it reverses this;
4. a **monthly** pass: buildings whose maintenance department is funded below 50 % look run down (lot condition
   2), and recover at 50 % or more;
5. a 1 % monthly chance of an **ape escape** at a Disease Research Center (an automata effect only).

| Class | CLSID | Interfaces (IID) | City component | City getter |
|---|---|---|---|---|
| civic building simulator | `0x0A413481` | `cISC4CivicBuildingSimulator` (`0x8A41322F`) and `cIGZUnknown` (`1`) at +0, `cIGZMessageTarget2` (`0x452294AA`) at +4, `cIGZSerializable` (`0xE4FDA3D4`) at +8 | 39 | `cISC4City` slot 73 (`+0x124`) |

`QueryInterface` for any other IID returns false. `SL::OnCityInit` caches the object in the global `0x00B43DB4`.

Factory `0x0069E800` (`GlobalHeapAlloc(0x48)` + constructor `0x0069E670`). Vtables: primary `0x00AB0C7C`
(9 interface slots + deleting destructor `0x0069E700`), message target `0x00AB0C6C` (`DoMessage` `0x0069E1A0`),
serializable `0x00AB0C54` (Write `0x0069D920`, Read `0x0069E820`, GetGZCLSID `0x0069D6B0`).

## 2. State

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C` | reference count | 0 | no |
| `+0x10` | `initialized` (u8) | 0 | no |
| `+0x14` | Landmarks department (`0x8A416368`), held | null | yes (reference) |
| `+0x18` | `exclusions`: hash set of u32 (STLport, 193 buckets) | empty | yes |
| `+0x2C` | `statuses`: hash map u32 key → `ConditionalBuildingStatus` (193 buckets) | empty | yes |
| `+0x40` | `diseaseCenters`: list of occupants in occupant group `0x1917`, each held | empty | yes |
| `+0x44` | `maintained`: list of occupants whose maintenance this simulator charged, each held | empty | yes |

`ConditionalBuildingStatus` (SDK `cISC4CivicBuildingSimulator::ConditionalBuildingStatus`) is
`{u8 status; u16 buildingCount; cRZString itemToolTip}`. Status bits (SDK enum): **1** hidden, **2** available,
**4** advisor event data set. Bit 4 is the reward advice's "already announced" mark (CB3): the reward advice
(advisors.md §6.6) sets the status to 6 (available and announced) when its condition first returns 1, and fires
the advice only while bit 4 is clear. A later condition result keeps the bit: a string clears bits 1 and 2 (4),
number 0 sets hidden and clears available (5). That accounts for the corpus values 4, 5 and 6. The key is the
building exemplar's instance ID (for example `0x032B0000` for the
Opera House).

Both hash containers are STLport hash tables with the identity hash, created with a size hint that gives **193
buckets**, new entries at the head of their chain, and iteration bucket by bucket (building-development.md
§4.7.1.2). The iteration order is visible in the save (§7).

## 3. Agents, messages and cadence

`Init` registers **one** agent: `AddAgent(+4, level 6 (month), "cSC4CivicBuildingSimulator" (string `0x00A89938`),
flags 2)` (simulator.md §4: enabled only when the city is established).

It subscribes, in this order, to `0xE8BDA2E7` (lot state changed), `0x99EF1142` (occupant inserted) and
`0x99EF1143` (occupant removed). `DoMessage` returns true for every message.

| Message | Handling |
|---|---|
| `0x66956816` SimNewMonth | `DoMonthlySimulation` (§4.6) |
| `0x99EF1142` / `0x99EF1143` | §4.3 |
| `0xE8BDA2E7` | IGZUnknown → `QueryInterface(0xE818FCED)` (`cISC4Lot`; the message carries a **copy** of the lot, lot-developer.md §3.2). If data2 (the new state) is **3** (occupied): `DoLotActivated(lot)` (§4.4). Release the lot. |

Nothing is posted.

## 4. Behaviour

### 4.1 `Init()`

If `initialized`, return true. Otherwise set `initialized := true` first, then:

1. If no Landmarks department is held (none was loaded): `CreateDepartmentBudget(0x8A416368, group 0x6A357B96)`,
   hold it, `SetDepartmentName(0x6A3FF01C, 0xCA491DD2)` ("Landmarks") and `SetMaxAllowedFundingPercentage(1.0)`.
2. If the budget has no department `0xEA59717A`: create it with group `0xEA597195`, name (`0x6A3FF01C`,
   `0x4A59726D`) ("Building Maintenance"), maximum funding 1.0. It is not held.
3. If the budget has no department `0x2A5A723F`: create it with group `0x0A5A72D1`, name (`0x6A3FF01C`,
   `0x8A5A7330`) ("Business Deals"), maximum funding 1.0. Not held.
4. Subscribe (§3).
5. `AddAgent` (§3).
6. Register the Lua table `civic` with one function, `get_landmark_count()`, which returns `GetLandmarkCount()` as a
   number.
7. Return true.

A department that cannot be created *[choice]* (QUESTIONS CB10): the original sets the Landmarks name and maximum on
the result of `CreateDepartmentBudget` without a null test, so a failure there crashes it (`0x0069DC69`). Steps 2
and 3 test the result (`0x0069DCAB`, `0x0069DCF1`) and set nothing on a null one. FreeCity4 treats all three alike:
no name or maximum is set and nothing is held (a later `Init` will not retry Landmarks, because `initialized` is
already set).

Departments are budget.md §3.2 objects (record §11.2); the department IDs are listed in budget.md §12. The maximum
funding fraction is passed as a pointer to a `float` 1.0 (budget.md §7). This is the shared service-simulator
pattern of budget.md §12.

### 4.2 `Shutdown()`

If `initialized`, in this order: unregister the Lua functions; release every entry of `diseaseCenters` and clear it;
the same for `maintained`; `RemoveAgent(+4)` on the city's simulator; release the Landmarks department and set the
field to null; unsubscribe the three messages; clear `initialized`. Return true.

*[compat]* The two hash containers are **not** cleared. The object is destroyed after a city shutdown, so this is
not visible in the shipped game.

### 4.3 Occupant inserted / removed *[compat]*

`occ := void1`. If `occ.GetType()` (occupant slot 7) is not `0x278128A0` (building), ignore. `holder :=
occ.AsPropertyHolder()`. Then, in this order:

1. **Count.** If the bool property `0xEA3209F8` (conditional building) is present and true, and `occ` answers
   `QueryInterface(0x87DDA3A9)` (`cISC4BuildingOccupant`): `key := BDS.GetBuildingKeyFromType(building type)`
   (building development simulator slot 14). If that succeeds and `GetConditionalBuildingStatus(key.instance)`
   finds an entry, its `buildingCount` is incremented (insert) or decremented (remove), as a 16-bit wrap.
2. **Exclusion.** If the u32 property `0xEA2E078B` (exclusion ID) is present: on insert, add the value to
   `exclusions` and **stop** (step 3 is for removals only); on remove, erase the value.
3. **Removal.** On remove, if `occ` is a building: `DoBuildingRemoved(building)` (§4.5).

*[quirk]* There is no preview-occupant test (foundations §5.2, property `0x029244DB`). If a preview occupant of a
building with an exclusion ID passes through the occupant manager, its insert adds the ID and its removal erases
it. Whether that removal can erase the ID of the building just placed depends on the order of the two messages
(QUESTIONS CB1). Counts are symmetric (+1, then −1).

*[quirk]* Erasing removes the ID even if a second building with the same exclusion ID still stands (possible only
through plugins or cheats).

### 4.4 `DoLotActivated(lot)` (`0x0069DE20`) *[compat]*

1. `b := lot.GetBuilding()` (lot slot 48) on the lot the message carries. If null, return. `holder := b.AsOccupant().AsPropertyHolder()`.
2. **Maintenance.** If `budget.GetBudgetItemForPurpose(holder, 0xAA59670C, &item)`:
   - if `item.line = 0`: `budget.ChangeBudgetItemLine(holder, item.index, b.GetBuildingType())` and use that type as
     the line;
   - `dept := budget.GetDepartmentBudget(item.department)`. If it exists: `line := dept.GetLineItem(item.line)`, or
     if absent `dept.CreateLineItemForBuildingType(item.line, isLocal = false)`; if a line was found or made,
     `line.AddToFullExpenses(item.cost)` (int64);
   - in every case (even without a department): `AddRef` the building and append `b.AsOccupant()` to `maintained`.
     *[quirk]* There is no duplicate check: a lot that returns to state 3 a second time is charged and listed
     twice.
3. **Income.** If `budget.GetBudgetItemForPurpose(holder, 0x4A5A495E, &item)` and its department exists:
   `type := b.GetBuildingType()`; `line := dept.GetLineItem(type)` (**the building type, not `item.line`**); if
   absent, `line := dept.CreateLineItemForBuildingType(type, false)` and `line.SetType(1)` (income);
   `line.AddToIncome(item.cost)`.
4. **Disease Research Center.** If `b.AsOccupant().IsOccupantGroup(0x1917)` and that occupant is not already in
   `diseaseCenters`: append it and `AddRef` the building.

*[choice]* An income line that cannot be created (QUESTIONS CB9): the original calls `SetType(1)` on the result of
`CreateLineItemForBuildingType` without a null test and crashes (`0x0069DF6D`–`0x0069DF78`); FreeCity4 skips the
income step then. The maintenance step already tests its line (a line neither found nor made adds nothing) *[compat]*.

### 4.5 `DoBuildingRemoved(b)` (`0x0069DAA0`) *[compat]*

1. **Maintenance.** If the holder has a budget item for `0xAA59670C`: `budget.RemoveBudgetItemFromBudget(holder,
   item.index)` (budget.md §6: subtracts the cost from the line). Then find the **first** entry of `maintained`
   equal to `b.AsOccupant()`; if found, release it and erase that entry.
2. **Income.** If the holder has a budget item for `0x4A5A495E` and its department exists: `type :=
   b.GetBuildingType()`; if `dept.GetLineItem(type)` exists: `line.SetIncome(line.GetIncome() − item.cost)`
   (SetIncome clamps at 0, budget.md §8); then if `line.GetIncome() < 1`: `dept.RemoveLineItem(type)`.
3. **Disease Research Center.** If in group `0x1917`: erase the first matching entry of `diseaseCenters` and release
   it. The release happens only when a matching entry was found and erased (`0x0069DBEB`–`0x0069DC02`), as in step
   1 (QUESTIONS CB12).

### 4.6 `DoMonthlySimulation()` (`0x0069E010`) *[compat]*

1. **Ape escape.** If `diseaseCenters` is not empty: create a temporary `cRZRandom` seeded from the clock
   (seed −1, foundations §6.2). If `RandomDoubleUniform() ≤ 0.009999999776482582` (the double constant at
   `0x00A9C328`, float 0.01 widened): `i := RandomUint32Uniform(n)` with n the list length; `occ :=` the i-th entry;
   if `city.GetAutomataControllerManager()` (city slot 47, `+0xBC`) exists:
   `AddControllersFromOccupantGroup("ape_escape", occ, 0)` (automata controller manager slot 10; the name is passed as
   a `cRZString`). No message is posted and no news or advice is produced here.
2. **Run-down look.** For each entry `occ` of `maintained`, in list order:
   - `item := GetBudgetItemForPurpose(occ.AsPropertyHolder(), 0xAA59670C)`; skip if none;
   - `dept := GetDepartmentBudget(item.department)`; skip if none;
   - `lot := lotManager.GetOccupantLot(occ)` (lot manager slot 14); skip if none;
   - `f := *dept.GetFundingPercentage()` (a fraction, 1.0 = 100 %);
   - if `f < 0.5`: if `lot.GetCondition() = 0`, `lot.SetCondition(2)`;
   - otherwise (including NaN): if `lot.GetCondition() = 2`, `lot.SetCondition(0)`.

Null pointers *[choice]* (QUESTIONS CB8, CB11). The original tests none of these:

- a null entry of `maintained` (an unresolved save reference, §7) is dereferenced at once (`0x0069E0F0`–`0x0069E10B`)
  and crashes it; FreeCity4 skips the entry;
- a null funding pointer from `GetFundingPercentage` is dereferenced at `0x0069E146`; FreeCity4 skips the entry;
- a null entry drawn by the ape escape is passed on to `AddControllersFromOccupantGroup` (`0x00517150`), which hands
  it to each matching controller template (`0x005163D0`) without a test; what a controller does with a null target
  was not followed. FreeCity4 does nothing for a null draw (the generator has still made both draws).

The comparison uses the department's percentage, not the building's local funding. A building listed twice (§4.4)
is visited twice; the second visit changes nothing.

### 4.7 Interface methods

| Method | Behaviour |
|---|---|
| `IsBuildingExcluded(id)` | true if `id` is in `exclusions` |
| `GetConditionalBuildingStatus(key)` | pointer to the stored status, or null. Not a copy: callers modify it in place (the advisors set bits through it) |
| `UpdateConditionalBuildingStatus(key, const ConditionalBuildingStatus&)` | find or insert `key` (a new entry starts as `{0, 0, ""}`), then copy status, count and text into it. The return value is not meaningful (callers ignore it); *[choice]* FreeCity4 returns true |
| `GetLandmarkCount()` | the number of entries of `maintained` (every maintained civic building, not only landmarks) *[quirk: the name]* |

### 4.8 Worked example

The player builds the Mayor's House (`0x031F0000`: maintenance purpose `0xAA59670C`, department `0xEA59717A`,
line 0, cost 30; exclusion ID `0xAA9EE012`; conditional building).

1. Placement: the occupant manager inserts the building. `0x99EF1142`: its status entry's count goes 0 → 1; its
   exclusion ID is added to the set (the menu now greys it out).
2. The lot developer finishes the lot and posts `0xE8BDA2E7` with data2 = 3. `DoLotActivated`: the budget item's
   line is 0, so it is changed to the building type; the Building Maintenance department gets (or creates) that
   line and adds 30 to its full expenses; the building joins `maintained` (GetLandmarkCount = 1).
3. The player lowers Building Maintenance funding to 40 %. At the next SimNewMonth the lot's condition 0 becomes 2.
   At 60 % the month after, 2 becomes 0.
4. Bulldozing: `0x99EF1143`: count 1 → 0, the ID leaves the set, the budget item's cost is removed from the line,
   and the entry leaves `maintained`.

A business deal (the casino, income purpose `0x4A5A495E`, department `0x2A5A723F`, 300): at activation the
Business Deals department gets an income line keyed by the casino's building type with income 300; at removal the
income drops to 0 and the line is removed.

## 5. Random numbers

Only the ape escape (§4.6 step 1) draws: a **temporary** `cRZRandom` constructed with seed −1 (wall clock) each
month that has at least one Disease Research Center; one `RandomDoubleUniform()`, and on success one
`RandomUint32Uniform(n)`. No generator state is saved (foundations §6.2, which lists this temporary). *[choice]*
FreeCity4 seeds each temporary with `derive(0x0A413481 XOR n)`, `n` counting earlier creations in the session
(foundations §6.3, "temporaries seeded from the clock"). `n` counts the generators **this simulator object** has made,
from 0 when it is constructed, and is not saved (QUESTIONS CB14): loading a city builds a new object, so `n` starts
again at 0 with each city.

## 6. Tuning

The simulator reads no exemplar of its own. The data it uses lives on the buildings:

| Property | Type | Meaning | Shipped data |
|---|---|---|---|
| `0xEA3209F8` | bool | conditional building (counted in its status entry) | true on 62 exemplars, false on 8 parks |
| `0xEA2E078B` | u32 | exclusion ID (one per unique building) | unique per reward, e.g. zoo `0x8A9AE664`, Disease Research Center `0x8A9AE357` |
| budget items (budget.md §2) | | maintenance purpose `0xAA59670C`, income purpose `0x4A5A495E` | maintenance on 17 rewards in Building Maintenance `0xEA59717A` (costs 30–470), the cruise ship in Parks and Recreation `0x8A4260A6`, the Great Pyramid in Landmarks `0x8A416368`; income on 6 business deals in `0x2A5A723F`: toxic waste dump 400, prison 250, army base 350, casino 300, area control tower 380, missile range 450 |
| occupant group `0x1917` | | Disease Research Center (ape escape) | only `RW36x50_DiseaseCtr_033F` |

Constants: the 0.5 funding threshold (float at `0x00A84D2C`), the 0.01 escape chance, the automata group name
`"ape_escape"`, the department IDs, groups and names of §4.1.

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x0A413481`) *[compat]*

```
u16  version = 6
u32  n;  n × u32 exclusion ID                    (hash-set iteration order)
u32  m;  m × { u32 key; u8 status; u16 buildingCount; GZStr itemToolTip }   (hash-map iteration order)
ref  Landmarks department                        (foundations §2.7; class 0xE990BFFC)
list diseaseCenters                              (pollution.md §6: u32 count, then count refs; class 0xA9BD882D)
list maintained
```

GZStr is foundations §2.3. The iteration order is the STLport order of §2: buckets `key mod 193` ascending, and
within a bucket newest first. *[compat]* A writer must reproduce it to write byte-identical records; a reader must
accept any order.

`Read`: fail unless the version is exactly 6. Insert every exclusion ID and every status into the existing
containers (they are not cleared first; on a fresh object they are empty). Release the held department, then read
the reference with IID `0x223E2B67` (`cISC4DepartmentBudget`). Read both lists with the shared list reader
(`0x005DB3C0`): the list is cleared (without releasing the old entries), a `u32` count is read, and a count of
10,000,000 or more reads nothing further; otherwise each entry is a reference read with IID `0x26BB767E`
(`cISC4Occupant`), and an unresolvable reference appends null (foundations §2.7). Return true unless the
stream reports an error.

**Insertion order** *[compat]* (QUESTIONS CB6). The original inserts the file's entries **in file order**, each at
the head of its chain (set insert `0x004F0940` → `0x00624DD0`; map insert `0x0069E720` → `0x0069E5F0`; the table is
grown with `resize(n + 1)` before each insert, which does nothing below 194 entries). Since the file lists each
chain newest first, reading **reverses every chain of two or more entries**, and the next `Write` lists those
chains oldest first. A second round trip restores the original order. Duplicates: an exclusion ID already present
or repeated in the file is not inserted again (a set); a status key already present or repeated **overwrites** the
stored status, count and text, so the **last** value in the file wins (`0x0069E944`–`0x0069E96D`), and the entry
keeps its position in the chain.

`Read` is **not atomic** *[compat facts]* (QUESTIONS CB13). A wrong version returns false with nothing changed.
Otherwise the containers are filled entry by entry, the held Landmarks department is released (`0x0069E9A1`–
`0x0069E9AB`) before its reference is read, the two lists are cleared without releasing their old entries
(`0x005DB3C0`), and the stream error is tested only at the end. *[choice]* FreeCity4 reads into temporaries and
changes nothing on failure; with the shipped game's one `Read` on a fresh object the two agree on every valid save.

### 7.1 Corpus validation

All 421 distinct saves parse exactly (every byte consumed):

| (exclusions, statuses, diseaseCenters, maintained) | Saves |
|---|---|
| (0, 46, 0, 0) | 405 (new and unplayed cities) |
| (0, 49–63, 0, 0) | 10 |
| (0, 63, 0, 1) | 1 (a maintained building without an exclusion ID: the lighthouse or the Great Pyramid) |
| (1, 59–60, 0, 0–1) | 2 |
| (3, 60, 0, 1) | 2 |
| (5, 61, 0, 3) | 1 |

- The department reference is always class `0xE990BFFC`, the list entries class `0xA9BD882D` (7 entries in all).
- Every save keeps the hash order: exclusion IDs and status keys ascend by `key mod 193`. (`mod 53`, `97`, `389`
  and `769` all fail.)
- Status bytes over the corpus: 0 (always with text) 11,024; 1 (hidden, without text) 8,466; 4: 15; 5: 8; 6: 32.
  Counts above 0 occur in 13 entries of 5 played cities. No save has a Disease Research Center.
- Example: `Berlin\City - New City (10).sc4` (3,821 bytes): 46 statuses, the first `0x032B0000` with status 0 and
  the text "Opera House is not available.\nRequirements: …". `Berlin\City - Konradshohe.sc4`: exclusions
  `0x4A9AE483, 0xAA9EE012, 0xAA9AE59F, 0xEA9ADE07, 0x8A9ADCB0`, three maintained buildings.

## 8. Interface slot table *[compat]*

`cISC4CivicBuildingSimulator` (vtable `0x00AB0C7C`):

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x0069D6C0` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x004664A0` (shared) | 0 | 0 | EAX |
| 2 | `+0x08` | `Release()` | `0x0069D7A0` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init()` | `0x0069DC10` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x0069D7C0` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `IsBuildingExcluded(uint32 id)` | `0x0069E5B0` | 1 | 4 | AL |
| 6 | `+0x18` | `GetConditionalBuildingStatus(uint32 key)` → `ConditionalBuildingStatus*` | `0x0069D8D0` | 1 | 4 | EAX (null if absent) |
| 7 | `+0x1C` | `UpdateConditionalBuildingStatus(uint32 key, ConditionalBuildingStatus const*)` | `0x0069E9F0` | 2 | 8 | none meaningful |
| 8 | `+0x20` | `GetLandmarkCount()` | `0x0069E5D0` | 0 | 0 | EAX |
| 9 | `+0x24` | deleting destructor | `0x0069E700` | 1 | 4 | EAX |

`Release` (`0x0069D7A0`) decrements when the count is above 1 and returns the new count. At 1 or less it calls the
deleting destructor (`0x0069E700`, destructor `0x0069E500`) **without** calling `Shutdown` first, and returns 0. The
destructor releases the Landmarks department but not the entries of the two lists (QUESTIONS CB7). The city always
calls `Shutdown` before its last release, so in the shipped game the object is already shut down. *[choice]*
FreeCity4 follows the sibling rule (commercial §2.3): `Shutdown`, then destroy. On a shut-down object the
`Shutdown` does nothing, so the two agree.

The SDK declares slot 7's second parameter as a non-const pointer; the original copies from it and never writes
it. The SDK's `bool` return of slot 7 is not honoured by the original.

## 9. Who reaches in

| Caller | Uses |
|---|---|
| Building menu (`cSC4View3DWin::AddBuildingsToItemList` `0x007EFF40`) | slot 5 with the building's `0xEA2E078B` value: excluded → disabled with LTEXT `0x6A231EAA`/`0x0A765619` ("This building cannot be built because it already exists in this city."); slot 6: status bit 2 → available, otherwise the item's tooltip is the status text |
| Reward advice and the advisor base (`0x0043EC30` `GetTrigger`, `0x00411570` `DoMessage`) | slots 6 and 7: create and update statuses, set bit 4 (advisors.md §6.6) |
| Lua `reward_instance_count` (`0x0040A8E0`), `unlock_reward_building` (`0x0040B760`), `get_reward_building_state` | slots 6 and 7 (advisors.md §6.6) |
| Legalise Gambling ordinance (`0x0080C3A0`) | sets the casino `0x033A0000` status to hidden (ordinance.md §3.6) |
| City situation manager (`0x0052CC50`) | reads statuses for its dialog |
| Lua `civic.get_landmark_count()` | slot 8 |
| `SL::OnCityInit` / `OnCityShutdown` | the global `0x00B43DB4` |

## 10. Original quirks (reproduced)

- No preview-occupant test (§4.3).
- No duplicate check in `maintained`; a re-activated lot is charged twice (§4.4).
- Income lines are keyed by building type while maintenance lines use the item's line (§4.4).
- `GetLandmarkCount` counts every maintained building (§4.7).
- An exclusion ID is erased when any building carrying it is removed (§4.3).
- The ape escape has no message, advice or news of its own (§4.6).

## 11. Open questions

See QUESTIONS.md, section "From the commercial, industrial, civic, park, history and clock specs" (CB1–CB5).

## 12. Conformance tests (must exist in `engine/conformance`)

1. **Init**: on an empty budget, `Init` creates the three departments with the IDs, groups, names and maximum 1.0 of
   §4.1, holds only Landmarks, subscribes `0xE8BDA2E7`, `0x99EF1142`, `0x99EF1143` and registers one month agent
   with flags 2; a loaded Landmarks department is reused, not recreated.
2. **Exclusion**: inserting a building with `0xEA2E078B = 0x8A9AE664` makes `IsBuildingExcluded(0x8A9AE664)` true;
   removing it makes it false; a non-building occupant with the property changes nothing.
3. **Counts**: a conditional building with a status entry: insert → count 1, remove → 0; a building without a
   status entry changes nothing; `0xEA3209F8 = false` changes nothing.
4. **Activation**: lot-state message with data2 = 3 for a building with a maintenance item (line 0, cost 30, department
   `0xEA59717A`) → the item's line becomes the building type, the line's full expenses rise by 30, `maintained`
   has one entry; data2 = 2 does nothing; a second data2 = 3 message gives two entries and +60.
5. **Income**: activation of a 300 business deal creates an income line (type 1) keyed by the building type with
   income 300; removal deletes the line; a line with income 500 drops to 200 and stays.
6. **Monthly condition**: department funding 0.49 → condition 0 becomes 2, condition 1 stays 1; funding 0.5 →
   condition 2 becomes 0; NaN behaves like ≥ 0.5.
7. **Ape escape**: a stub generator returning 0.009999999776482582 gives an escape; one returning the double 0.01
   (slightly larger) does not. The index draw is `RandomUint32Uniform(n)`; nothing is drawn without a Disease
   Research Center; without an automata controller manager nothing happens after the draws.
8. **Status API**: `UpdateConditionalBuildingStatus(k, {6, 2, "x"})` then `GetConditionalBuildingStatus(k)` returns
   `{6, 2, "x"}`; an unknown key returns null; writes through the returned pointer persist.
9. **Save**: write → read → write keeps every byte except the order **within** a bucket chain of two or more
   entries, which is reversed (§7, CB6): keys 1 then 194 inserted (both bucket 1) write 194, 1; after a read the
   record writes 1, 194; a second read → write gives 194, 1 again. The bucket order (`key mod 193` ascending) is
   kept. A status key repeated in the record keeps its last value. Version 5 is rejected.
10. **Corpus**: every civic record of the 421-save corpus reads; its re-write equals the record with each multi-entry
    bucket chain reversed (byte-identical when no bucket holds two entries), and a second read → write gives the
    record back byte-identically.
