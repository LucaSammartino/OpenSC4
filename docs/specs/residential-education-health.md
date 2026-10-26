# Residential simulator: education and health

Status: round-3 spec. Sources: `bin/SimCity4.exe` (Windows 1.1.641), disassembly checked function by function;
Ghidra pseudocode and the Mac symbols were used for names only. Every fact below holds for the Windows build.

This spec covers the **education and health half** of `cSC4ResidentialSimulator`. It covers schools and hospitals,
their coverage, grades, funding, strikes and auto-budget, and the EQ (education quotient), HQ (health quotient)
and life-expectancy model. The rest of the class is in **residential.md**: `Init`/`Shutdown`, the full tuning
loader and property table, `DoMessage` and agents, the state-machine overview, population, age, proximity,
workforce, the byte-exact save record, and the full slot tables. Where the two files overlap (the state frame,
the save record, `DoPopulationChange` ordering, the tuning loaders), residential.md is normative.

**No other class computes school or hospital coverage.** Checked:

- The civic building simulator does not read the education/health properties below and has no coverage grid.
- The Lua scripts (`SimCity_1.dat`) only *read* results, through the advisor and query glue.
- The coverage-radius display helper and the map view only *query* this class (§9).

Tags: *[compat]* observable behaviour a replacement must reproduce. *[quirk]* an original bug that is reproduced.
*[inferred]* not proven. *[choice]* FreeCity4 decision.

## 1. Role, class, interfaces, creation

| Item | Value |
|---|---|
| Class ID | `0x4990C013` (`cSC4ResidentialSimulator`) |
| City component | #36 (foundations §3.6). Created, initialised and saved by the city like every component (foundations §3.7). |
| `cISC4ResidentialSimulator` | IID `0x077AC1EE` (and `0x00000001`, IGZUnknown). Primary vtable `0x00AB2C10`, 60 slots, at offset 0. |
| `cIGZMessageTarget2` | IID `0x452294AA`. Vtable `0x00AB2BFC` (4 slots) at offset +4. |
| `cIGZSerializable` | IID `0xE4FDA3D4`. Vtable `0x00AB2BE4` (6 slots) at offset +8. |
| Global | `SL::OnCityInit` stores `city.GetResidentialSimulator()` (city slot 68 `+0x110`) in a global (`0x00B43DA4`). Most readers use that global (§9). |

What the education/health half does:

1. It **recognises service buildings** as they are added and removed: 7 school groups and 3 hospital groups
   (§4.1). It keeps them in per-group lists and registers them with the education and health departments of the
   budget.
2. Once per cycle it **rebuilds per-group coverage maps**: `uint8` "seat units" per tract (§4.4).
3. It **grades every building**. The grade is effective capacity divided by the population the building serves.
   It writes the grade, the staff ratio and the served population into the building's properties. It paints a
   shared grade map per service. It adds an EQ or HQ **boost** around the building into the per-wealth EQ/HQ grids
   (§4.5, §4.6).
4. It **ages EQ and HQ** daily, tract by tract, inside the population update. This covers decay, pollution, the
   newcomer blend, the generational reset and the death's-door effect (§4.7).
5. It **runs strikes**: a monthly roll per building, a city-level strike flag per service, and messages (§4.9).
6. It runs **auto-budget**: per-building ideal local funding (§4.10).
7. It answers **queries**: cell and global EQ/HQ/LE, average grids, ratings, totals, query-tool data, building
   lists and coverage geometry (§4.8, §4.11).
8. It **records history**: global EQ/HQ and school/hospital coverage per mille (§4.12).

Init order and timing are in residential.md §4.1 (`Init`), §4.3 (agent registration) and §5.1 (states). In
`Init` the class:

- reads the simulator tuning (§4.13);
- reads the ordinance effects (§4.3);
- creates the four department budgets when they are missing (§4.2);
- creates the coverage and grade grids when they are missing (§2).

## 2. State (education and health part)

Byte-exact layout and versions are in residential.md §9. The table gives meaning, initial value and whether the
item is part of the class's save record (residential.md §3 and §9.1 are normative).

| State | Initial | Saved | Notes |
|---|---|---|---|
| School coverage grids ×7 (`uint8`) | 0 | yes (grid refs) | Instance IDs, in group order: `0xAA8E1DC1`, `0x8A8DF4B6`, `0xCA8E449E`, `0xAA8DF4BD`, `0x8A8DF4C4`, `0xAA8DF4D4`, `0x69D5C30D`. Tract size = the population tract size S. |
| Hospital coverage grids ×3 (`uint8`) | 0 | yes | `0xAA8E1DE4`, `0x8A8DF6FC`, `0x69D5C317`. Tract size S. |
| School grade grid (`float`) | 0 | yes | `0x69D5C324`, tract size S. Returned by slot 39. |
| Hospital grade grid (`float`) | 0 | yes | `0x69D5C330`, tract size S. Returned by slot 40. |
| EQ grids ×3 (`float`, R$ / R$$ / R$$$) | 0 | yes | `0x69D5C2D0 + w` (w = 0, 1, 2). Tract size S_EQ. |
| HQ grids ×3 (`float`) | 0 | yes | `0x69D5C2E0 + w`. Tract size S_EQ. |
| Tract sizes S (population, age, coverage, grade) and S_EQ (EQ/HQ) | 4 and 4 (constructor) | yes | On a new city both are set to the tract size of the R$ developer's tract grid (growth developer `GetPopulationMap`, residential.md §2, §4.1). |
| School group lists ×7, hospital group lists ×3 | empty | as references | Buildings in group order (§4.1). Each holds a reference. |
| Pending school / hospital lists | empty | as references | Buildings added since the last state 0 (§4.1). |
| Per-service department record ×2 | see below | partly | Strike flag, strike start day, tuning values, curves, department budget pointers. |
| Strike flag and start day (per service) | false, 0 | saved, but **reset to false / 0 by the next `Init`** | City-level "the service is on strike" (§4.9). Part of the department block; `ReadSimulatorTunableValues` sets them unconditionally, so a city strike does not survive save and load *[quirk]* (residential.md §9.3; checked at `006FBCB0` / `006FBD33`). |
| Ordinance effect lists ×2 | empty | no | Rebuilt at `Init` and in every state 0 (§4.3). |
| Served counters ×2 | 0 | no | Population covered for the first time in the cycle (§4.4). They feed the history (§4.12). |
| Grade totals ×2 (`float`) | 0 | no | Members `+0x314` (schools) and `+0x318` (hospitals). Accumulated by the EQ/HQ boost (§4.6) in states 6 and 8 (ComputeOccupantGrade gets their addresses, `0x006FFED3`, `0x00700053`), and reset at the start of state 0 (`0x006FF567`). **Never read** otherwise: these four are the only references in the class (`0x006F6E50`–`0x00700E90`), no interface slot returns them, and `Read`/`Write` skip them (RS13). They belong to this half; residential.md's frame only triggers the reset with the rest of state 0. |
| Global auto-budget flags (schools, hospitals) | false, false | yes (version ≥ 7) | Slots 52–55 (§4.10). |
| `cRZRandom` member (`+0xB8`: `lcg` at `+0xB8`, `mul` at `+0xBC`) | wall-clock seed at construction | no | Strike rolls and death's-door effect (foundations §6.2). Every draw of this class is a double draw, which reads and advances only `mul` (RG6). **Seed** (RG21): the constructor (`0x007005AA`) calls `Seed(0xFFFFFFFF)` (`0x00913667`), so `lcg := mul := time(NULL) XOR ((c << 16) \| c)` with `c = clock()` read once (milliseconds since process start; `time` `0x009EFB59`, `clock` `0x009F361F`). This is **not** the `cRZTime` value (`time() + 2177452800`) that crime's generator uses. *[choice]* FreeCity4 seeds it with `derive(0x4990C013)`, the class ID (foundations §6.3). |
| Dirty flag | see residential.md §3 | yes (residential.md §9.1) | Set by the month message when the cycle is late, by every building add/remove and by `SetOccupantFundingPercentages`. **Never read** *[inferred: no reader in the class]*. A byte at `+0xA0`, 1 after construction (`0x007005B2`); the month message clears or sets it through the message-target sub-object (`0x007003A4`, `0x007003B9`), and every other access in the class (`0x006F834A`, `0x006FB354`, `0x006FB572`, `0x006FB910`, `0x006FB9C1`, `0x006FB9D4`) writes 1. No interface slot returns it. Its only trace outside the object is the saved byte, written as `dirty ∨ state ≠ 10` (residential.md §9.1), so a shadow can only compare it through a save; leaving it out is the implementer's choice (RG14). |

**Per-building state** is kept in the building's own property holder and saved with the building:

| Property | School | Hospital | Type | Written by |
|---|---|---|---|---|
| served population | `0x09220161` | `0xC922015C` | u32 | grading (§4.5); `0xFFFFFFFF` on add |
| effective capacity | `0xE9244B2E` | `0x49244B21` | u32 | grading; = capacity on add |
| grade | `0x292453C5` | `0x092453CB` | f32 | grading; 1.0 on add |
| staff ratio | `0x2A976C20` | `0xAA976BE7` | f32 | grading; 1.0 on add |
| strike start day (0 = not striking) | `0xEAA69F83` | `0xEAA69F83` | u32 | strikes (§4.9) |
| auto-budget | `0x6C05FC4D` | `0x6C05FC4D` | bool | slot 57 / UI |

All six are **scalars** *[compat]* (RG16). The class writes them with three helpers, u32 `0x005FE3C0`, float
`0x005FE4F0` and bool `0x005FE210`. Each looks the property up (holder slot 9), creates it when missing (a new
property object, vtable `0x00AA6988`, added with holder slot 15), then calls the variant's scalar setter:
`SetValUint32` (slot 26), `SetValFloat32` (slot 38) or `SetValBool` (slot 11). The setter (`0x0091587D` for u32)
clears any other content, an array included, and stores a single value of type 6 (u32), `0x0A` (f32) or 1 (bool).
So whatever a building's exemplar or an earlier write put there, after the class writes it the property is a
scalar of the type in the table.

## 3. Agents and cadence

Agent registration is described in residential.md §4.3, the state list in §5.1. The education/health work is done in these
states of the daily state machine. There is **one state per `DoDailySimulation` call**, and the call is driven
by SimNewDay `0x66956814`:

| State | Work |
|---|---|
| 0 | Reset both grade totals. Monthly strike check for schools, then hospitals (§4.9). Clear and re-read the ordinance effects (§4.3). Move pending schools, then pending hospitals, into their group lists (§4.1). Move pending health-affecting buildings (residential.md). Next state: 13. |
| 13, 1, 14, 2, 15, 3, 11, 4, 12 | Population, age and workforce (residential.md). State 4 (`DoPopulationChange`, per tract) also runs the EQ/HQ update of §4.7. |
| 5 | Reset the school served counter. `UpdateCoverageMap` for school groups 0..6 in order (§4.4). |
| 6 | School grade grid := 0 (`SetTractValues(0)`). For each school group 0..6 and each building in list order: auto-budget step (§4.10), then `ComputeOccupantGrade` (§4.5) with the EQ grids, max 200, isSchool = true. |
| 7 | Reset the hospital served counter. `UpdateCoverageMap` for hospital groups 0..2 (§4.4). |
| 8 | The same as state 6 for hospitals, with the HQ grids and isSchool = false. |
| 9 | History (§4.12). Next state: 10. |
| 10 | Idle. The month message SimNewMonth `0x66956816` sets state 0 when the state is 10. Otherwise it only sets the dirty flag: the running cycle continues, reaches state 10 and then waits for the next month message. |

Consequences *[compat]*:

- Day agents run before month agents on the 1st (simulator spec §5). State 0 therefore runs on the **2nd** of
  the month at the earliest. State 5 runs at least 10 days later, because states 13, 1, 14, 2, 15, 3, 11, 4
  and 12 each take at least one day.
- A cycle that is not back in state 10 by the next 1st skips that month's restart. It restarts on the 1st after
  it reaches state 10.
- The monthly strike check (state 0) uses grades and staff ratios from the **previous** cycle's states 6 and 8.
- An ordinance enacted during a month affects coverage and grades from the next state 0 on.
- Auto-budget calls `SetLocalFundingPercent` immediately, during state 6 or 8. The budget sees the new
  percentages at its next settlement (budget spec).
- Pollution used for the HQ decay (§4.7) is whatever the pollution simulator holds when state 4 reaches a
  tract.

`DoDailySimulation` returns true (AL = 1) in every state.

## 4. Behaviour and formulas

All float arithmetic is float32 (foundations §18): every add, sub, mul, div and sqrt rounds to float32, in the
order given. "trunc" is `_ftol` (toward zero). The result is used as a 32-bit integer and reinterpreted as u32
where noted. "u32→float" means `fild` plus 2^32 when the value is negative as int32. Response curves are
evaluated with `GetOutputValue(x)` (foundations §4). "%curve" means a curve whose output is a percentage and is
multiplied by **0.01** afterwards. Every `0.01` in this spec (including the ± 0.01 steps of §4.10) is the
**float32** constant `0.01f` = `0x3C23D70A` (`0x00A823A4`), never a double (verified in every residential-class
function that uses it; residential.md §7.1, RS1). So a shipped 80 % is 0.79999995 and 20 % is 0.19999999.

### 4.1 Service-building recognition and lists

**Messages.** The occupant-inserted message `0x99EF1142` and occupant-removed message `0x99EF1143` arrive with
data3 = `0x278128A0` (building occupant) and void1 = the occupant. The class asks the occupant for
`cISC4BuildingOccupant` (IID `0x87DDA3A9`) and calls `DoBuildingChanged(building, added)`, where added =
(type == `0x99EF1142`).

**DoBuildingChanged(b, added)** works on the building's property holder (`b.AsOccupant().AsPropertyHolder()`).

1. **Hospital test.** A building is a hospital if `budget.GetBudgetItemForPurpose(holder, 0xCA565486)` (health
   staff) succeeds. Otherwise (no item) it is a hospital if it is in occupant group `0x1507` or `0x151C`.
   For a hospital:
   - Budget registration (only when the staff item exists):
     - if the item's line is 0, `ChangeBudgetItemLine(holder, item.index, b.GetBuildingType())`, and the call's
       own copy of the item gets line := the building type (`0x006FB17F`; schools `0x006FB39A`). Every later step
       of this call (`GetLineItem`, `CreateLineItemForBuildingType`, the count, `RemoveLineItem`) uses that line;
       the item is not read again;
     - `dept := GetDepartmentBudget(item.department)`; null **aborts the whole call** (no list change);
     - when added: if `dept.GetLineItem(line)` is null, `CreateLineItemForBuildingType(line, 1)` then
       `line.SetDisplayFlag(2, true)`; `AddLocallyFundedObject(holder, 0xCA565486)`;
     - when removed: `RemoveLocallyFundedObject(holder, 0xCA565486)`;
     - the line item's 64-bit count (line slot 8 get / slot 9 set) is changed by +1 or −1; when removed and the
       count reaches 0, `RemoveLineItem(line)`;
     - *[quirk]* on removal the line item is fetched with `GetLineItem(line)` and used with **no null check**
       (`0x006FB1FD`–`0x006FB20C`; schools `0x006FB418`–`0x006FB427`). If the department has no line item for the
       line, the original calls through a null pointer and crashes. With consistent budget data this does not
       happen: the add created the line item and the count keeps it alive while a building uses it. *[choice]*
       FreeCity4 skips the count step and `RemoveLineItem` when the line item is missing;
     - then, if the ambulance purpose `0xEA56549E` has an item: its department (null aborts),
       `AddLocallyFundedObject` or `RemoveLocallyFundedObject` with that purpose.
   - **Added:** create and set served := `0xFFFFFFFF`, grade := 1.0, staff ratio := 1.0, effective capacity :=
     capacity (`0x69220415`). AddRef. Append to the pending hospital list. Set the dirty flag.
   - *[quirk]* **No capacity property.** The getter (`0x005FD480`) returns false and leaves its output alone, and
     the output is a stack slot this call used before. So the effective capacity written is: for a hospital with
     the staff item, the low 32 bits of the health department's object pointer (stored there at `0x006FB1A5`);
     for a school with the staff item, the school line item's pointer (`0x006FB41F`); for a building found
     through its groups only, whatever the slot held. The value is visible to the query data (§4.11 +0x18), the
     ratings and the totals until the next state 6 or 8 grades the building. No shipped building is affected:
     all 18 school and hospital buildings of `SimCity_1.dat`–`SimCity_5.dat` and `EP1.dat` have their capacity.
     *[choice]* FreeCity4 writes 0.
   - **Removed:** erase it from the first group list that contains it, else from the pending list. Release.
     Set the dirty flag.
2. **School test.** The same with staff purpose `0xEA5654B6`, bus purpose `0x4A5654BA`, groups `0x1503` or
   `0x151B`, capacity `0x8922041B`, and the pending school list. The school test runs for **every** building,
   after the hospital test. A building that passes both tests is put in both pending lists *[compat]*.
3. Health-affecting buildings (properties `0x6A19F6B5` and `0xAA19F6EA` both > 0) are handled in residential.md
   §5.8. They go to a pending list, which state 0 moves into the health-affecting list.
4. Finally, the building's slot 11 is called and the dirty flag set when the first u32 of the result is 1
   (`0x006FB9CC`–`0x006FB9D4`, RG2). Slot 11 is `cISC4Building::GetBuildingProfile()` (`0x00459250`): it
   returns the building development simulator's `GetBuildingProfile(type)` (slot 39) for the building's type
   (`+0x38`), or a static all-zero profile when there is no such simulator. The first u32 is the profile's
   purpose, and 1 is `Residence` (demand.md §6.3). So the flag is set for residential buildings. An
   early exit of the call (a null department, §4.1 step 1) skips this step. The flag is never read (§2).

**State 0 distribution** (§3). Each pending school is appended to the **first** group, in the order below, for
which `IsOccupantGroup(group)` is true. Hospitals are handled the same way. Then the pending list is cleared.

| Service | Group order (coverage-grid index 0..n−1) |
|---|---|
| School | `0x150F` (K-6 school), `0x1510` (high school), `0x1514`, `0x1504` (college), `0x1505`, `0x1506` (museum), `0x151B` |
| Hospital | `0x1512`, `0x1513`, `0x151C` |

Names come from the Lua constants and scripts where they exist. The others are unnamed here.

*[quirk]* A pending building that is in **none** of these groups is dropped from the pending list without
`Release`. It leaks one reference and never gets coverage or a grade.

**GetBuildings(lists, out, groups)** appends the buildings of each group list to `out` (AddRef each):

- `groups` null → all lists;
- otherwise only the lists whose group ID is in `groups`;
- an empty vector → none.

`GetSchoolBuildings` / `GetHospitalBuildings(out, groups)` do this and then **always append the pending list**.
**Counts** (slots 46/48) = Σ list sizes + pending list size.

**Building identity** *[compat]* (RG15). Every list (pending, group, health-affecting) stores the pointer returned
by the building's `AsOccupant()` (building slot 3), and removal finds the entry by comparing with
`AsOccupant()` of the removed building. That same list pointer is what `BeginStrike` puts in the strike message
(void1 and IGZUnknown, §5), what `GetBuildings` hands out, and the object each saved list reference names. So the
`cISC4Occupant` pointer is the building's identity throughout this class.

### 4.2 Departments and funding

**Departments** (created in `Init` when missing, budget group `0x6A357B7F`):

| Department | ID | Name (LTEXT group `0x6A3FF01C`) | Max | Initial funding |
|---|---|---|---|---|
| health | `0x09188F42` | `0xCA493A56` | health max | health initial |
| health secondary (ambulances) | `0xAA538CB3` | `0xCA538D98` | health max | 100 %; creates line item 1 (local) and clears its display flag 1 |
| education | `0x09188F4C` | `0x6A493A66` | school max | school initial |
| education secondary (buses) | `0x4A538CC6` | `0x8A538DAC` | school max | 100 %; line item 1 as above |

Calls: `CreateDepartmentBudget(id, 0x6A357B7F)`, AddRef, `SetDepartmentName(0x6A3FF01C, name)`,
`SetMaxAllowedFundingPercentage(&max)`, `SetFundingPercentage(&initial, 0)`.

**GetOccupantFundingPercentages(occ, &a, &b, isSchool)** (slot 20). Purposes: staff = `0xEA5654B6` (school) or
`0xCA565486` (health); vehicle = `0x4A5654BA` (bus) or `0xEA56549E` (ambulance).

- No staff item → a = b = 1.0.
- Staff item but its department is null → a and b are left unchanged.
- Otherwise: a = `item.localFundingPercentage` if `dept.IsLocallyFundedObjectInDepartment(holder, staff)`, else
  `*dept.GetFundingPercentage()`.
- b is found the same way with the vehicle purpose; b = 1.0 if there is no vehicle item.
- *[compat]* (RG10; `0x006F8360`, reached from slot 20 `0x006F84B0`) A vehicle item whose department is null
  returns at once: a has been written, **b is left unchanged**. In no case does the original read a funding
  value for a missing department, and it never substitutes 1.0 for one: 1.0 appears only when the **item** is
  missing. The outputs then keep what the caller put there; the class's own callers preset a = b = 0.0 (§4.2
  efficiency ratings, §4.10), so a missing department reads as 0 in the curves. A host asked for a missing
  department's funding must report "missing", and the core must then leave the output alone.
- (RG26) **How a host reports "missing"** has no original counterpart, because the original asks the budget
  simulator directly. The only original condition is `GetDepartmentBudget(item.department)` returning null for a
  purpose whose item exists. A missing **item** is not "missing" here: it gives 1.0, as above. The
  implementer's choice (a NaN or a null callback means missing, and the outputs are left alone) stands. It is
  safe because a real funding fraction is never NaN. The host must keep the two cases apart: no item gives 1.0,
  and an item without a department gives "missing".

**SetOccupantFundingPercentages(occ, &a, &b, isSchool)** (slot 19):

- For the staff purpose, then the vehicle purpose: `GetBudgetItemForPurpose`; if it succeeds,
  `GetDepartmentBudget(item.department)`, then `SetLocalFundingPercent(holder, x, purpose)`.
- A missing item ends the call **with** the dirty flag set. A null department ends it **without**.
- When both purposes succeed, the call also ends **with** the dirty flag set (`0x006F834A`, reached from the two
  "no item" exits and after the second `SetLocalFundingPercent`). So the flag is set in every case except a null
  department. The flag is never read (§2), so this is not observable.
- **What `SetLocalFundingPercent` stores** *[compat]* (RG13; budget spec §7.3). It does set the budget item's local
  funding, but as a **percentage**: the item's entry in the building's property `0xEA54D287` becomes
  float32(q × 100.0f) (`0x00692DBD`). Reading the item back (`GetBudgetItemForPurpose` / `GetBudgetItemInfo`)
  gives float32(pct × 0.01f) (`0x006939E6`; 0.01f is `0x3C23D70A`). The round trip is **not always exact**:

  | q written | stored pct | q read back |
  |---|---|---|
  | 0.43 (`0x3EDC28F6`) | 43.0 | 0.42999998 (`0x3EDC28F5`) |
  | 0.79999995 (`0x3F4CCCCC`) | 79.99999 (`0x429FFFFF`) | 0.7999999 (`0x3F4CCCCB`) |
  | 0.5 | 50.0 | 0.5 |
  | 1.2 (`0x3F99999A`) | 120.00001 (`0x42F00001`) | 1.2 |

  The budget ignores the call unless 0 ≤ q ≤ the department's maximum, the (holder, purpose) pair is registered
  and the item exists; the same call may also move the department's own percentage (budget spec §7.3). A shadow that compares the funding after an auto-budget step must compare the
  read-back value, not q.

**Efficiency ratings** `GetOccupantEfficiencyRatings(occ, &e1, &e2, isSchool, ignoreStrike)`:

```
e1 := e2 := 1.0
(a, b) := (0.0, 0.0); GetOccupantFundingPercentages(occ, &a, &b, isSchool)    (slot 20, always called)
if the building has no staff budget item: return
if not ignoreStrike and building strike day ≠ 0:  e1 := a × strikeEfficiency
else:                                              e1 := effCurve(a × 100) × 0.01
e2 := coverageCurve(b × 100) × 0.01
```

**Base staff efficiency** `GetBaseStaffEfficiency(occ, isSchool)` (`0x006F85F0`, result through a hidden pointer):
the same (a, b) := (0, 0) and slot-20 call, then `effCurve(a × 100) × 0.01` when the staff item exists, else a
(1.0, set by slot 20). It ignores strikes.

*[compat]* Both start a and b at **0.0** (`0x006F8749`–`0x006F8759`, `0x006F8611`–`0x006F8621`). Slot 20 leaves
them untouched when the staff item exists but its department is missing (§4.2 above), so then both read the curves
at 0. Example with the shipped school curves: e1 = effCurve(0) × 0.01 = **0**, e2 = coverageCurve(0) × 0.01 =
**0.5**, base = 0. The building then grades 0 (effective capacity 0), and its coverage radius is halved.

### 4.3 Ordinance effects

`GetOrdinanceEffects(healthList, schoolList)` runs at `Init` and in every state 0. State 0 clears both lists
first. `Init` does not, but the lists are empty then.

*[compat]* (RG7) Those are its **only** two callers (`0x006FED0B` in `Init`, `0x006FF5CA` in state 0; the
function `0x006FE0F0` has no other code or data reference). An ordinance enacted or repealed mid-cycle changes
nothing until the next state 0, and neither does a slot call: every reader of the effects (§4.4–§4.11) uses the
lists as state 0 left them.

**Before the first state 0** (RG28). Until then the readers use the lists that `Init` built. In a loaded city
those are the ordinances that were on in the save: the ordinance simulator's `Read` has run before any `Init`
(ordinance §9.1). In a new city every ordinance starts off, so the lists are empty. An ordinance enacted or
repealed after `Init` changes nothing until that first state 0.

The shadow is FreeCity4's own tool, with no original counterpart. "Reading them as they stand" gives the original's
lists as long as no ordinance has changed since `Init`, which holds for every shadow comparison made before the
player first changes an ordinance. *[choice]* To match in every case, the shadow should read the effects once at
city `Init` (or PostCityInit; nothing can change an ordinance between the two), and again only at each of the
original's state 0. The bracket is acceptable when the shadow starts with the city.

Example: a loaded city has the free clinics ordinance on. The player repeals it on day 3, and the first state 0
comes on day 10. From day 3 to day 10 the original still applies the clinics' health effect. A shadow that reads
the ordinances as they stand stops applying it on day 3, so for those days the two differ.

- The ordinance list comes from `GetOrdinanceOnCount` and `GetOrdinanceOnArray`, in ascending ID order
  (ordinance §4).
- For each ordinance, `GetOrdinanceByID(id)` is called; null skips it.
- The effect properties are read straight from `GetMiscProperties()`.
- Each service builds one **effect element**. The element is appended only if **at least one** of its six
  properties is present:

| Field | School property | Health property | Read | Default |
|---|---|---|---|---|
| radius factor | `0xA91B3AF4` | `0x491B3AD5` | f32 × 0.01 | 1.0 |
| distance curve | `0xA91B3AFA` | `0x891B3AE6` | curve, property-pointer loader | (0,100)(100,100) |
| effect factor | `0xA92D9D7A` | `0xE91B3AEE` | f32 × 0.01 | 1.0 |
| capacity factor | `0x892D9D02` | `0x092D909B` | f32 × 0.01 | 1.0 |
| age curve | `0xC91B3B02` | `0xE92D9DB4` | curve, property-pointer loader | (0,100)(100,100) |
| decay factor | `0x692EF65A` | `0xC92D9C7A` | f32 × 0.01 | 1.0 |
| group filter | — | — | — | **always empty** |

Both curves follow the foundations §4.1 **default** pattern. Each is rebuilt from the code default (0,100)(100,100)
for every ordinance, then loaded from the property when it is present.

Shipped users (ordinance §3.7): Pro-reading campaign (school effect factor 110 → 1.1), CPR Training and Free
Clinics (health effect factor 110 → 1.1 each).

**Which buildings an element affects.** `GetOrdinanceEffectListForOccupant(occ, kindProperty, list, &out)`
copies the elements for which `DoesOrdinanceAffectOccupant` is true:

- an element with an empty group filter affects every building;
- otherwise it affects a building whose u32-array property `kindProperty` contains one of the filter's values.

The filter is never filled (table above), so **every element affects every building of its service**
*[compat]*. The kind property is `0xA92D8E40` (school) or `0x292D8F9B` (hospital). It matters only for the
filter.

**Modifiers** `GetOrdinanceModifiers(list)`, products over the occupant's list in list order, starting at 1.0:
m1 = Π radius factor, m2 = Π capacity factor, m3 = Π effect factor.

**Decay factors** multiply the EQ and HQ decay (§4.7): EQdecay × Π school decay factors and HQdecay × Π health
decay factors. Each product starts at 1.0 and multiplies in list order.

### 4.4 Coverage maps

`UpdateCoverageMap(service, i)` for group i:

1. `coverage[i].SetTractValues(0)`. chunk := chunk size[i] (§4.13).
2. For each building of group i in list order:
   - e2 from `GetOccupantEfficiencyRatings(occ, isSchool, ignoreStrike = false)`;
   - the occupant's ordinance list (§4.3);
   - m := e2; then for each element, m := m × radius factor, in list order;
   - `MarkBuildingCoverage(i, occ, m, chunk, &served[service])`.

**MarkBuildingCoverage**:

1. radius := building property (school `0x691B42B3` "School Coverage Radius", hospital `0x091B42A2`, f32,
   metres). capacity := `0x8922041B` / `0x69220415` (u32). If either is missing, return.
2. r := radius × m. If r ≤ 0, return. (There are two dead calls before this test: a position read and a
   reciprocal tract width.)
3. units := capacity / chunk (u32 integer division). *[quirk]* chunk = 0 divides by zero (crash).
4. tracts := `GetCoveredTractList(occ, r, coverage[i])`.
5. For each listed tract (x, z), with old := `coverage[i](x, z)`:
   - `coverage[i](x, z) := (low 16 bits of old + units) > 255 ? 255 : low 8 bits of (old + units)`.
   - if old = 0 and no lower group j < i has a nonzero `coverage[j](x, z)`:
     `served += pop_R$(x, z) + pop_R$$(x, z) + pop_R$$$(x, z)` (u16 tract values).

*[quirk]* The served test does not look at units. A building with capacity < chunk (units = 0) adds 0 coverage
but still counts the tract's population. A second such building counts it again.

**GetCoveredTractList(occ, r, grid)**:

- Centre: the lot's centre (`lotManager.GetOccupantLot(occ)` → `lot.GetCenter`). If there is no lot, the
  occupant's `GetPosition`.
  - *[compat]* (RG5; `0x006F7E00`, called at `0x006F9CCF` and `0x006FDBA3`). The lot comes from the lot manager's
    slot 14 (`+0x38`) and the centre from lot slot 15 (`+0x3C`, `0x006BF560`), the same call crime makes
    (crime.md §13, CQ3). It writes x and z (8 · (x1 + x2 + 1), exact in float32) and y = the lot's altitude
    (`+0x24`), returns EAX = the output's address and leaves the **x87 stack empty**. A raw call that guards the
    x87 stack, as crime's glue does, therefore matches. Without a lot, the occupant's slot 8 (`+0x20`,
    `GetPosition`) fills the same vector.
- Calls `GetTractsInRadius(px, pz, r × slop, grid.GetTractWidthX(), countX, countZ, &list)`. slop = "coverage
  radius slop multiplier" (§4.13).

**GetTractsInRadius** (`0x005E5200`, cdecl, 7 arguments; new helper facts for simgrid §7):

- Inputs: centre (px, pz) in metres, radius r in metres, tract width w in metres, grid counts.
- inv := 1/w. x0 := trunc((px − r) × inv), x1 := trunc((px + r) × inv). z0 and z1 the same way.
- Clamp: x0 < 0 → 0. x1 ≥ countX → countX − 1, with an **unsigned** compare, so a negative x1 also becomes
  countX − 1. The same for z.
- The tract centre starts at cx := x0 × w + w × 0.5. It is advanced by `cx += w` in float32, not recomputed.
  The same for z.
- A tract is included iff `dz² + dx² ≤ r²`, with dx = cx − px and dz = cz − pz. dx² is stored to float32 first.
- Tracts are appended as (x, z) pairs, **x-major** (outer loop x, inner loop z).

### 4.5 Grading

`ComputeOccupantGrade(occ, coverage[i], chunk[i], gradeGrid, qGrids, max = 200, isSchool, &total)` runs for every
building of every group in state 6 or 8. Inputs:

- radius: as in §4.4. If it is missing: served := 0, effective capacity := 0, grade := 0.0. The staff ratio is
  not touched. Nothing else happens.
- capacity (u32, `0x8922041B` / `0x69220415`). *[quirk]* (corrected: not 0.) A missing capacity leaves the
  getter's output alone, and that stack slot holds the **coverage-radius property ID** stored there at the start
  (`0x006FD9D7` / `0x006FDA20`; read at `0x006FDBB7`). So a building with a radius but no capacity is graded with
  capacity `0x691B42B3` = 1 763 394 227 (school) or `0x091B42A2` = 152 781 474 (hospital). No shipped building
  has a radius without a capacity (§4.1). *[choice]* FreeCity4 uses 0.
- e1, e2 from `GetOccupantEfficiencyRatings(isSchool, ignoreStrike = false)`; base := base staff efficiency.
- The building's three curves: age (c1), wealth (c2), distance (c3). See the table below. Each is loaded with
  the property-pointer loader after a rebuild from the code default (0,100)(100,100), so the pattern is
  **default** (foundations §4.1).
- Ordinance list and m1, m2, m3 (§4.3).

| Curve | School | Hospital | Input |
|---|---|---|---|
| c1 age | `0xC9299662` | `0xA92AE3BD` | tract average age (u8) |
| c2 wealth | `0xC92D7EC0` | `0xA92D9FCD` | wealth index 0, 1, 2 |
| c3 distance | `0xE921A936` | `0x2921A9CC` | distance as a percentage of the radius |

Steps (float32 in this order):

```
r'        := (e2 × m1) × radius
tracts    := GetCoveredTractList(occ, r', population grid R$ geometry)        (same S as coverage)
served    := GetPopulationServed(occ, r', capacity, c1, c2, c3, ordinances, coverage[i], chunk, tracts)
staffCap  := trunc((float(capacity) × base) × m2)            (u32)
staff     := staffCap = 0 ? 0 : served = 0 ? 1.0 : min(1.0, float(staffCap) / float(served))
effCap    := trunc((float(capacity) × e1) × m2)              (u32)
grade     := effCap = 0 ? 0 : served = 0 ? 1.0 : min(1.0, float(effCap) / float(served))
write staff ratio, grade, effective capacity, served (properties of §2)
```

*[compat]* The Windows `_ftol` result is used as u32. A negative product would wrap. Capacities are never
negative in practice.

**Grade grid.** units := capacity / chunk (u32). t := float(units) × grade. For each listed tract with
n := `coverage[i](x, z)`:

```
v := (n > 1 ? gradeGrid(x, z) × float(n − 1) : 0) + t / float(n)
gradeGrid(x, z) := min(1.0, v)
```

- *[quirk]* This is not a weighted mean. The previous value is multiplied by n − 1 instead of being averaged. In
  a tract covered by several buildings, the earlier ones are over-weighted and the result reaches 1.0 quickly.
- *[quirk]* n = 0 can only happen when units = 0. Then t / 0 = NaN (t = 0), and the min keeps NaN *[inferred:
  min(1.0, NaN) stores NaN with the compare used]*.
- The grade grid is **shared by all groups of the service** and reset only at the start of state 6 or 8.

**GetPopulationServed** (r' and the tract list from above):

- r' ≤ 0 → 0.
- (tx, tz) := the building's tract in the R$ population grid: `PositionToTract` of the occupant's **own**
  `GetPosition` (occupant slot 8, `0x006F8FB7`), with no lot lookup *[compat]*. The tract list (§4.4) and the
  boost centre (§4.6) use the lot centre instead, so the two can name different tracts. Example with 64 m tracts:
  a lot whose centre is at x = 128 m (tract 2) and a building whose position is x = 120 m (tract 1). The listed
  tract x = 2 then gets `dx = float(u32(1 − 2))` ≈ 4.29e9 (the +x quirk below) instead of 0, so the distance curve
  is read at its last point for that column.
- For each listed tract (x, z) with n := `coverage[i](x, z)` > 0:

```
dx   := float(u32(tx − x))          (*[quirk]* x > tx gives ≈ 4.29e9)
dz2  := float(u32((tz − z)²))
d    := sqrt(dz2 + dx × dx)          (tract units)
xd   := d / r' × 100                 (*[quirk]* r' is in metres: tracts divided by metres)
md   := c3(xd) × 0.01
ma   := c1(float(age(x, z))) × 0.01
for each ordinance element of the occupant: md := (element.distanceCurve(xd) × md) × 0.01
                                            ma := (element.ageCurve(age) × ma) × 0.01
for w in 0, 1, 2:
    share := (float(units) / float(n)) × float(pop_w(x, z))
    sum   := sum + (((c2(w) × 0.01) × share) × ma) × md
```

- Result: round-half-even(sum), computed as float32 `sum + 12582912.0` with the bias subtracted from the bits.
  It is valid for 0 ≤ sum < 2^22.
- The units mismatch makes xd small: for a 500 m radius and a 30-tract distance, xd = 6. Tracts on the +x side
  of the building get xd ≈ 8.6e11. So the distance curve is effectively read at its first point on the −x side
  and at its last point on the +x side *[compat]*.

### 4.6 EQ and HQ boost (AddRadiusEffectWithCurves)

At the end of `ComputeOccupantGrade`, when r' > 0:

- multP := building effect multiplier, school `0xA92AE446` / hospital `0xC9299659`, f32, default 1.0.
- Before this there is a dead call to `city.PositionToCell`.
- Centre: lot centre, else occupant position (as §4.4).
- Curve lists (`GetOrdinanceCurveListsForOccupant`):
  - listA (distance) = each matching ordinance element's distance curve, then c3;
  - listB (age) = each element's age curve, then c1.
  - *[quirk]* This function always filters with the **school** kind property `0xA92D8E40`, even for hospitals.
    The filter is empty (§4.3), so this has no effect.
- For w = 0, 1, 2:

```
value_w := ((grade × (c2(float(w)) × 0.01)) × multP) × m3
AddRadiusEffectWithCurves(ageGrid, qGrids[w], listB, px, pz, r' × slop, listA,
                          value_w, 200.0, units, coverage[i], popGrid[w], &total)
```

qGrids is the EQ grids (schools) or HQ grids (hospitals).

**AddRadiusEffectWithCurves** (`0x005E48F0`, cdecl, 13 arguments; new helper facts for simgrid §7). The
target is a float grid with tract size S_t and tract width w.

- Tract bounds: as in `GetTractsInRadius`, but the upper clamp uses a **signed** compare.
- It reads the coverage grid and the target **raw** through `GetGridData`.
- For each tract in the bounds, in x-major order:
  1. Skip the tract if pop_w(x, z) = 0 (u16). *[quirk]* On this path the z-centre accumulator is **not**
     advanced, so every later tract in that column uses a centre one tract too far back.
  2. d := sqrt(dz² + dx²) (float32). Skip the tract unless d ≤ r.
  3. mult := 1.0. xd := 1 − (r − d) / r.
  4. For each curve in listA: mult := (mult × curve(xd)) × 0.01.
  5. If the age grid is not null, for each curve in listB:
     mult := (mult × curve(float(ageGrid.GetCellValue(x × S_t, z × S_t)))) × 0.01.
  6. eff := mult × value.
  7. n := coverage(x, z) (raw u8). If n ≠ 0: k := float(units) / float(n); eff := eff × k;
     pop' := trunc(float(pop) × k). If n = 0, neither changes: eff stays as in step 6 and pop' := pop
     (verified, EH13: the compare with 0.0 at `0x005E4B5F`–`0x005E4B72` jumps straight to `0x005E4B96`).
  8. total := total + float(pop' & 0xFFFF) × eff.
  9. target(x, z) := min(max, target(x, z) + eff), written raw.
- **Which tract each grid is read at** (verified, EH13). One pair (x, z) drives every read and write of an
  iteration:
  - the population grid through `GetTractValue(x, z)` (slot 22), read twice with the same arguments: the step-1
    test (`0x005E4A41`–`0x005E4A49`) and pop in step 7 (`0x005E4B3F`–`0x005E4B45`);
  - the coverage grid raw at `data[x][z]` (`0x005E4B4E`–`0x005E4B57`);
  - the target raw at `data[x][z]` (`0x005E4BBA`–`0x005E4BC3`);
  - the age grid is the exception. It is read through **cell** coordinates (x × S_t, z × S_t) with
    `GetCellValue`, so it lands on its own tract ((x × S_t) >> its shift). That is the same tract only when its
    tract size equals S_t.

  The bounds, the tract centres and S_t all come from the target alone (counts slots 12/13 at `0x005E497D` and
  `0x005E49A6`, width slot 14 at `0x005E490C`, size slot 10 at `0x005E48FF`). No other grid's geometry is asked
  for, and nothing is bounds-checked against it.
  - In this class the target is an EQ or HQ grid (tract size E) and the other three grids use P. P = E = 4 on a
    new city, and each grid keeps its own tract size through a save (residential.md §2, simgrid §5), so all four
    name the same tract in every game-written city.
  - Only a hand-made save can make them differ. If the target then has more tracts than the population or
    coverage grid, the original reads past their arrays. *[choice]* FreeCity4 reads such a tract as 0 in those
    grids (population 0 skips it at step 1).
- Example for step 7 (eff = 1.5 after step 6, pop = 120, units = 3):

  | n | k | eff | pop' | total += | target += |
  |---|---|---|---|---|---|
  | 0 | not computed | 1.5 | 120 | 180 | 1.5 |
  | 3 | 1.0 | 1.5 | 120 | 180 | 1.5 |
  | 6 | 0.5 | 0.75 | 60 | 45 | 0.75 |

  With units = 0 (capacity below the chunk size, §4.4), n = 6 gives k = 0, so nothing is added. A tract that no
  building of the group covered (n = 0) still gets the full 1.5 and counts all 120 people.
- Notes:
  - Units: xd here is a 0..1 fraction of the radius. Curves with x in percent therefore see only their first
    percent *[compat]*.
  - EQ and HQ values can only rise here. The cap is 200 (`GetMaxEQ`/`GetMaxHQ`). The tuning values "Maximum EQ
    Boost" and "Maximum HQ Boost" are never used (§4.13).

### 4.7 Daily EQ/HQ update (inside DoPopulationChange)

`DoPopulationChange(tv, tx, tz)` runs per tract in state 4 (residential.md §5.5). tv holds the
population tunables read by `ReadTunablePopulationValues` (§4.13); that read multiplies the decays by the ordinance
decay factors.

**Precision context (VC29-PC):** the ordinary formulas assume PC 24. A simulation callback reached inside a
Lua guard instead uses live PC 53; it does not change the EQ/HQ grids' float32 storage. In the newcomer blend
below, intermediate arithmetic remains in registers until the final Q float store. See
[foundations §18.4](foundations.md#184-lua-to-native-re-entry-uses-the-live-precision-vc29-pc) for addresses,
storage boundaries and a 3-to-7 population case with distinct exact grid values in the two contexts.

The education/health steps, in order:

1. **Pollution.** For type 0 (air), 1 (water), 2 (garbage): avg := Σ `pollution.GetPollutionValue(type, x, z)`
   over the S × S cells of the tract / S² (u32 division). The sum is a wrapping u32 accumulator and S² a 32-bit
   product (residential.md §5.5 step 1, RS3). Only the **low byte** of avg is used *[quirk]*.
   GetPollutionValue returns the pollution **source** grid value (pollution spec, slot 6).
   ```
   HQdec := tv.HQdecay
   HQdec := airCurve(float(avg_air & 0xFF))     + HQdec
   HQdec := waterCurve(float(avg_water & 0xFF)) + HQdec
   HQdec := garbageCurve(float(avg_garb & 0xFF))+ HQdec
   ```
2. **Decay**, for w = 0, 1, 2:
   - EQ_w := EQ_w − tv.EQdecay; if the result is < 0 → 0. A NaN is kept.
   - HQ_w := HQ_w − HQdec; the same clamp.
3. **Death's door** (visual only), for the same w, right after its decay. It fires when all of these hold:
   - HQ_w < tv.deathsDoorHQ;
   - no effect has been started yet for this tract in this call;
   - pop_w(tx, tz) ≠ 0;
   - the draw r := `RandomDoubleRangeUniform(0.0, 100.0)` on the class's `cRZRandom` (1 draw) gives
     tv.deathsDoorChance > r.

   Then the class creates the visual effect "buzzards" (`effectsManager.CreateVisualEffect`):
   - x = tx × width + width × 0.5, and the same for z;
   - y = terrain.GetAltitude(x, z) + tv.height + `RandomDoubleRangeUniform(−delta, +delta)` (a 2nd draw);
   - the effect is positioned, started and released;
   - only after a successful create is the per-tract flag set.

   **Who creates the effect** *[compat]* (RG1; `0x006FA649`–`0x006FA83E`). The game itself does, through the
   city's effects manager (city slot 85 `+0x154`, cached in `0x00B43D1C`). It calls the manager's slot 7 (`+0x1C`,
   `0x005939B0`, pop 8): `bool CreateVisualEffect(const char* name, cISC4VisualEffect** out)`, with the name
   `"buzzards"` (`0x00AB2BD8`).
   - The manager lowercases the name and looks it up in its two effect tables. An unknown name returns AL = 0 and
     leaves `*out` alone. A known one creates the effect with one reference, stores it in `*out` and returns
     AL = 1. The caller tests `out`, not AL.
   - The shipped effect directory (`SimCity_1.dat`, type `0xEA5118B0`, group `0xEA5118B1`, instance 1) defines
     `buzzards`, so the create always succeeds with shipped data.
   - Geometry, all float32: width := the w-th EQ grid's `GetTractWidthX()` (slot 14) and `GetTractWidthZ()`
     (slot 15) for z; x := (width × tx) + width × 0.5, z likewise with tz. y := float32(double(altitude +
     tv.height) + `RandomDoubleRangeUniform(−delta, +delta)`). The altitude is the terrain's slot 13 (`+0x34`)
     at (x, z).
   - The transform record (0x38 bytes) has bytes +0 and +1 set to 1, an identity 3 × 3 matrix at +0x04..+0x24,
     the translation (x, y, z) at +0x28/+0x2C/+0x30 and the scale 1.0 at +0x34. The effect then gets
     `SetTransform` (slot 8, `+0x20`), `Start(0)` (slot 3, `+0x0C`) and `Release` (slot 2), and the tract's
     flag is set.
   - **Bytes +2 and +3** *[compat]* (RG25; `0x006FA814`–`0x006FA819`, `0x005BEC20`). The original never writes
     them: the record is a stack buffer, and only bytes +0 and +1 are stored before the matrix. They are the
     alignment gap after the two flag bytes. The effect's `SetTransform` (`0x005BE880`, which forwards to
     `0x005BEC20`) copies the dwords +0x04 to +0x34 and byte +1. It reads neither +0, +2 nor +3 (it counts its own
     changes instead of copying +0). So any value there gives the same effect, and 0 is equivalent.
   - **If the create fails**, only the first draw has been made, the flag stays clear, and the next wealth level
     of the same tract may draw again. The shared generator therefore advances by a different number of draws,
     which shifts every later strike roll (§4.9).

   *[choice]* FreeCity4 asks the host to create the effect. When the host cannot show effects (standalone, tests),
   the core treats the create as **successful**: it makes both draws and sets the flag, as the original does with
   shipped data. A host that reports a real failure gets the original's one-draw path.

   **"Cannot show" counts as created** (RG27). In the original, the steps after a successful create, in order, are:
   the two tract widths, the terrain altitude (slot 13), the second draw, the transform, `Start(0)`, `Release`,
   and the flag. Of these, only the second draw and the flag change simulation state. The width and altitude
   queries are pure reads, and `Start` and `Release` act only on the effect object. So a core that makes the
   second draw and sets the flag, and skips the altitude read, the start and the release, leaves the generator,
   the flag and every grid exactly as the original does with shipped data. The original has no "cannot show"
   state of its own: with shipped data its create always succeeds (above). **The bracket matches.** One
   condition applies: a host that skips the effect must not also skip the second draw, or every later strike
   roll shifts (§4.9).
4. **New population** (residential.md). If new > old for wealth w, with delta := int16(new − old):
   ```
   m      := EQ_old > 0 ? 2 : 1
   EQ_new := min(200, ((((EQ_old + newEQ_w) / m) × delta) + old × EQ_old) / new)
   ```
   The same for HQ with newHQ_w. If new = 0: EQ_w := HQ_w := 0.
5. **Generational reset.** If the tract's average age > `u8(GetCellLifeExpectancy(S × tx, S × tz))`:
   - age := tv.ageReset;
   - for each w: EQ_w := EQ_w × tv.EQreset and HQ_w := HQ_w × tv.HQreset.
6. **Accumulators**:
   - a 64-bit sum += LE × total new population;
   - the age histogram: bucket population += pop and bucket EQ sum += trunc(EQ) × pop, with the bucket rule of
     residential.md §5.5 step 11 (ages 18..22 → bucket 0; ages 0..17 and ≥ 77 → bucket 19).
   - Their readers are in residential.md (§5.6, §5.7).

The exact order of these steps relative to the population and age steps (decay before the population read; LE
after the EQ/HQ updates) is normative in residential.md §5.5.

### 4.8 EQ, HQ and LE queries

Precision qualifications are as in §4.7: statements below about rounding every operation to float32 apply to
ordinary PC 24. At PC 53 the global queries still spill the first wealth product, second wealth subtotal and
per-tract running sum to float32, but use 53-bit arithmetic between those stores. Their final quotient is
returned in the x87 register; the caller may subsequently store it as float32 (foundations §18.4, VC29-PC).

**GetTractQuotient(q[3], p[3], x, z)** (internal):

- sum16 := pop_0 + pop_1 + pop_2 as **uint16** (wraps, *[quirk]*). If sum16 = 0 → 0.0.
- s := q2 × p2; s := q1 × p1 + s; result := (q0 × p0 + s) / float(sum16).

| Method | Behaviour |
|---|---|
| `GetCellEQ(x, z)` / `GetCellHQ(x, z)` (slots 35/36) | GetTractQuotient(EQ or HQ, pop, x / T, z / T), unsigned division, where T is the R$ EQ grid's own `GetTractSize()`, asked on every call (slot 35, `0x006F79C0`; AQ2), and for slot 36 the R$ **HQ** grid's (`0x006F7A10`; AQ3). ST0. |
| `GetCellEQByWealth(x, z, wealth)` (37/38) | `EQ[wealth − 1].GetTractValue(x / T, z / T)`, with T from the **R$** EQ grid as in slot 35, whatever the wealth (`0x006F7A60`; AQ2). Slot 38 reads `HQ[wealth − 1]` with T from the R$ HQ grid (`0x006F7AB0`; AQ3). The wealth byte is decremented as u8, so **wealth 0 or > 3 reads out of bounds** (crash) *[quirk]*. ST0. |
| `GetGlobalEQ()` / `GetGlobalHQ()` (32/33) | pop := `GetPopulation()`; 0 → 0.0. Otherwise, over the EQ grid's tract counts: a := Q0 × p0; b := Q1 × p1 + a; c := Q2 × p2 + b; sum := c + sum; result := sum / float(pop). ST0. **Traversal (RS4, from the exe, `0x006F7720` / `0x006F7870`):** x outer from 0 to `GetTractCountX() − 1`, z inner from 0 to `GetTractCountZ() − 1`. The counts come from the R$ EQ grid (HQ: the R$ HQ grid). Every value is read through `GetTractValue(x, z)`: Q_w as a float, p_w as a uint16 converted with `float()`. No raw grid memory is read. Each operation is one float32 rounding, and `sum` is stored as float32 after every tract. On a 2 × 2 grid the order is (0,0), (0,1), (1,0), (1,1). |
| `GetAverageEQGrid(&g, min*, max*)` / `HQ` (21/22) | Creates a **new** float grid (refcount 1, owned by the caller), tract size E, the member itself for both (`0x006FCF8B`, `0x006FD02B`; AQ2, AQ3), ID `0x69D5C351` / `0x69D5C360`. *[compat]* (RG8) The grid is built **directly**, not through GZCOM: 0x44 bytes from the global heap (`0x006FCF36`) with the float grid's constructor inlined (vtable `0x00A87C20`, class ID `0x49B9E60A` at `+0x10`), then `AddRef`, `Init`, `SetTractSize`, `SetInstanceID`. A failed allocation is not handled (null is dereferenced). The callers found are UI code (main thread *[inferred]*); the original has no rule for other threads, so how a replacement creates the grid off the main thread is the implementer's choice. The walk covers the new grid's tract counts, so E sets them, and reads the three EQ grids (slot 22: HQ) and the three population grids at the same tract (x, z) with `GetTractValue`, whatever their own sizes (`0x006F70E0`). Each tract := float(trunc(quotient)), or 0 when sum16 = 0. min starts at 255.0 and max at 0.0. Each is updated with every tract, populated or not (verified, RS6: `0x006F70E0`; strict compares), and written only when its pointer is not null. |
| `GetEQMinAndMaxCellCoords(&minX, &minZ, &maxX, &maxZ, &min, &max)` / `HQ` (43/44) | If `GetPopulation()` = 0: all six outputs := 0, AL = 0. Otherwise min := 200.0, max := −200.0. For each tract (x outer, z inner) with any pop_w ≠ 0: q := quotient; q < min → min := q and remember (x, z); q > max likewise (strict). Coordinates out := T/2 + T × tract (signed /2), T being the R$ EQ grid's own `GetTractSize()`, asked twice for each of the four outputs (`0x006F74A7`–`0x006F7535`; AQ2); slot 44 asks the R$ HQ grid (`0x006F75A6`; AQ3). AL = 1. *[quirk]* If no tract has population (the population counter disagrees with the grids), the coordinates are uninitialised stack values. |
| `GetCellLifeExpectancy(x, z)` (29) | Loads the residential tuning exemplar **on every call**; missing → 0. Curve "Health Quotient to Life Expectancy Curve" `0x6941A67F`, **default** pattern, code default (0,0)(200,200). LE := trunc(curve(GetCellHQ(x, z))). Returns **AL only** (§7). |
| `GetGlobalLE()` (34) | The same with `GetGlobalHQ()`. **AL only**, not a float. |
| `GetMaxEQ()` / `GetMaxHQ()` (50/51) | 200 (EAX). |

**Which tract size** (AQ2, from the exe). The member E (`+0x78`) is read only by slots 21/22, as the tract size of the
grid they create, and by `Init` and `Write` (residential.md §2, §9.2). Slots 35, 37 and 43 ask the R$ EQ grid for its
own `GetTractSize()` on every call, and the grids keep their saved geometry through a load (simgrid §5), so these
queries follow the grids, not E. In a game-written city E and every EQ and HQ grid are 4 and the two rules agree; only
a hand-made save with other sizes tells them apart. *[choice]* Using E for every query matches the original whenever
the grids have size E; asking the R$ grid, as the original does, matches it always and is recommended.

The HQ variants follow the same rule with the R$ **HQ** grid (AQ3): slots 36, 38 and 44 ask it, never an EQ grid, and
slot 22 uses E like slot 21.

**With no R$ grid** (AQ11, from the exe). The original never tests the R$ EQ grid pointer (`+0x7C`) or the R$ HQ
grid pointer (`+0x88`) before it uses them, so a missing grid is a crash, not a fallback:

- Slots 35/36 and 37/38 call the R$ grid's `GetTractSize` at once (`0x006F79C4`, `0x006F7A14`, `0x006F7A63`). The
  wealth variants never ask the R$$ or R$$$ grid for a size, whatever the wealth.
- Slots 43/44 first test `GetPopulation()`. With population 0 they write the six zeros and return false without
  touching any grid (`0x006F7389`). Otherwise they take the **tract counts** of the walk from the R$ grid too
  (slots 12/13, `0x006F73D8`, `0x006F73E5`; `0x006F75AE`, `0x006F75BB`), then its tract size for the coordinates.
- The quotient helper (`0x006F6FC0`) also reads the three population grids and the three EQ (or HQ) grids without a
  test, so a missing R$$ or R$$$ grid crashes there too.

The grids exist from `Init` on, so this happens only after a failed `Init` or in a call before it. *[choice]* There
is no original behaviour to match; the implementer's choice (member E for the size, and E's tract counts for the
walk) is accepted.

### 4.9 Strikes

Per service there is a department record holding: strike flag, start day, strike efficiency, strike duration,
"min grade to end", strike curve, max funding and the message type. The message types are `0x077ACCD0`
(school) and `0x077ACCD2` (health). "today" is `city.GetSimulator().GetSimDate().DayNumber()`.

**DoMonthlyStrikeCheck(dept, groupLists, staffProp, gradeProp, strikeGroup)** (state 0). strikeGroup is
`0x1A03` for schools and `0x1A02` for health. changed := false, count := 0. For every building of every group
list, in order:

1. Skip it unless `IsOccupantGroup(strikeGroup)`.
2. changed := changed | `UpdateStrikingState(occ, dept, staffProp, gradeProp)`.
3. If the building's strike day is > 0: count += 1, and if changed, `BeginStrike(dept, occ)`.
   - *[quirk]* changed is sticky. After the first change, every later striking building calls BeginStrike. That
     call is a no-op while the flag is set.

At the end: if changed and the flag is set and count = 0 → `EndStrike(dept, today − startDay)`.

**UpdateStrikingState(occ, dept, staffProp, gradeProp)**. s := the building's strike day (`0xEAA69F83`, u32,
default 0).

- **Striking (s ≠ 0).** The strike ends (s := 0) if `today − s > duration` (u32 compare, **days**). Otherwise it
  ends if staffRatio ≥ minGradeToEnd; staffRatio defaults to 1.0 when missing. A NaN ratio keeps the strike.
- **Not striking.**
  - g := grade (default 0.0); p := strikeCurve(g × 100). There is no 0.01 factor: p is a percentage.
  - r := `RandomDoubleRangeUniform(0.0, 100.0)` on the class's `cRZRandom`, **exactly 1 draw per eligible
    non-striking building per monthly check**, even when p = 0.
  - If r < p, the strike starts and s := today.
- **On a change:**
  - write s;
  - `city.GetAutomataControllerManager()`: on start, `AddControllersFromOccupantGroup(name, occ, 0)`; on end,
    `RemoveControllers(name, occ)`;
  - name = "striking_health" when the department's message type is `0x077ACCD2`, otherwise
    "striking_education";
  - AL = 1. With no change, AL = 0.

**Differential shadows** (RG6). The rolls come from the member generator, seeded from the clock (§2). There is no
original behaviour to follow here; a shadow that does not reproduce them leaves the strike days, the city strike
flags and anything that follows from them out of its comparison *[choice]*. A shadow that wants them can read the
original object's `mul` word (`+0xBC`) just before state 0 and run the same double draws from it, since no other
part of the generator is used.

**BeginStrike(dept, occ)**: if the flag is set, AL = 0. Otherwise flag := true, start := today, post the strike
message (§5), AL = 1.

**EndStrike(dept, length)**: if the flag is clear, AL = 0. Otherwise flag := false, start := 0, post the
end-of-strike message, AL = 1.

**EndSchoolStrike / EndHealthStrike** (slots 8/9): if the service is on strike, `EndStrike(dept, today − start)`.
Otherwise AL = 0. The aura simulator calls these when a disaster ends (aura spec). The per-building strike days
are **not** cleared, so the next monthly check re-evaluates them.

So AL is the city strike flag as it was **before** the call (1 = a strike was ended and its end message posted,
0 = nothing happened). The original runs these, the setters of §4.2/§4.10 and the per-building setters on the
caller's thread with no deferral; its callers (the aura simulator's agent, the UI) are on the main thread
*[inferred]*. Deferring calls from another thread and returning the flag that was visible at the call is the
implementer's choice (RG9): no original behaviour covers it.

**SchoolIsOnStrike / HealthIsOnStrike** (6/7): AL = flag.

**ChanceOfSchoolStrike / ChanceOfHealthStrike** (10/11), returned through a hidden pointer as SC4Percentage:

- On strike → 1.0.
- Otherwise p := 0. For each group list: c := `GetChanceOfStrike(list)`, then p := (c + p) − c × p.
- `GetChanceOfStrike` (one list): q := 0, g := 0. For **every** building in the list:
  - if it is in the strike group: g := grade if the property is present (otherwise g keeps its previous value);
    c := strikeCurve(g × 100) × 0.01;
  - then q := (q + c) − q × c.
  - *[quirk]* A building outside the strike group reuses the previous c (0 before the first member).

### 4.10 Auto-budget and ideal funding

In states 6 and 8, a building is auto-budgeted if the global flag for its service is set (slots 52–55) **or** its
own property `0x6C05FC4D` is true. For such a building:

```
(a, b) := (0, 0); GetOccupantFundingPercentages(occ, &a, &b, isSchool)   (slot 20)
if EstimateIdealFunding(occ, &a):                                        (slot 58)
    SetOccupantFundingPercentages(occ, a, b, isSchool)                   (slot 19)
```

Only the staff percentage changes; the vehicle percentage is written back unchanged. The auto-budget step runs
**before** the building is graded, so it uses last cycle's served population.

**EstimateIdealFunding(occ, &f)** (slot 58, `0x006FD680`):

- **Service** *[compat]*: school if `GetSchoolQueryData` (slot 26) succeeds; otherwise hospital if
  `GetHospitalQueryData` (slot 27) succeeds; otherwise AL = 0. The school is tried first.
- Fails (AL = 0) if the building's query data fails (§4.11) or served = `0xFFFFFFFF` (not graded yet). The test
  reads the served **property** itself (`0x09220161` / `0xC922015C`) into a value preset to `0xFFFFFFFF`
  (`0x006FD701`–`0x006FD72B`), not the query data's served field (which maps `0xFFFFFFFF` to 0). A missing
  property therefore also fails, though the query has already failed in that case.
- r := float(served) / float(trunc(float(capacity) × m2)) (u32 operands; served and capacity are the query
  data's +0x24 and +0x14, `0x006FD7EE`–`0x006FD82B`). A zero divisor gives inf or NaN.
- effCurve is the service's "Funding To Efficiency" curve and L its max allowed funding.

```
f := r
v := effCurve(f × 100) × 0.01
while v > r:                       # step down
    f := f − 0.01;  v := effCurve(f × 100) × 0.01
    if f ≤ 0: f := 0; break
while v < r:                       # step up
    f := f + 0.01;  v := effCurve(f × 100) × 0.01
    if f > L: f := L; break
```

f is accumulated in float32. The result is the first point of the grid r ± k × 0.01 where the efficiency
reaches r, clamped to [0, L]. AL = 1.

### 4.11 Query data, ratings, totals, capacity and coverage geometry

**Query data** (`GetSchoolQueryData` / `GetHospitalQueryData(occ, tQueryData*)`, slots 26/27). The record
begins with a `cRZString` (vtable `0x00A80810`) at offset 0, which these methods do not fill *[inferred]*. The
fields below follow it and are u32 unless marked:

| Offset | School | Hospital |
|---|---|---|
| +0x14 | capacity | capacity |
| +0x18 | effective capacity (property) | effective capacity |
| +0x1C | capacity / school divisor (floor) | capacity / hospital divisor (floor) |
| +0x20 | ceil(effCap / school divisor) | **floor**(effCap / hospital divisor) |
| +0x24 | served (`0xFFFFFFFF` → 0) | served (same) |
| +0x28 | grade (f32) | grade (f32) |
| +0x2C / +0x30 | staff item: funding percentage (f32), cost; without an item 1.0 / 0 | the same, staff item **required** (AL = 0 without it) |
| +0x34 / +0x38 | bus item: funding, cost; else 1.0 / 0 | ambulance item; else 1.0 / 0 |

The school query fails (AL = 0) only if one of capacity, effective capacity, served or grade is missing. A
divisor of 0 divides by zero *[quirk]*.

**Cost fields** *[compat]* (RG3). The cost at +0x30 and +0x38 is the **low 32 bits** of the budget item's 64-bit
cost (the item's `+0x10`); the high half is dropped (school `0x006F92B7` and `0x006F92F6`, hospital
`0x006F7F76`–`0x006F7FAC`). The funding percentage next to it is the item's local funding as a float. Without the
item the pair is 1.0 (`0x00B0D440`) and 0. A core that keeps the cost in 64 bits writes back its low 32 bits.
Reading a record back into 64 bits, sign extension would recover a negative cost and zero extension would not;
shipped costs are positive and below 2^31, so both give the same value.

**Order of the tests and partial writes** *[compat]* (school `0x006F91C0`, hospital `0x006F7E80`). Both read, in
this order: capacity (a probe, then again straight into +0x14), effective capacity (straight into +0x18), served
(straight into +0x24, still raw), grade (into a local). The hospital query then requires the staff item
(`0xCA565486`) **last**; the school query does not. The first missing one returns AL = 0 at once. So a failed query
leaves the fields read before the failure in the record (+0x14, +0x18, +0x24 with a raw `0xFFFFFFFF`), and every
other field as the caller left it. Only a successful query writes +0x1C to +0x38 and maps served `0xFFFFFFFF` to
0. Example: a hospital with every property but no staff item → AL = 0 with +0x14, +0x18 and +0x24 already
written.

**Ratings** `GetSchoolSystemRating` / `GetHealthSystemRating` (12/13), returned through a hidden pointer as
SC4Percentage:

- sum := Σ effective capacity (+0x18) of the query data of every building in the group lists.
- *[quirk]* The record is not reset between buildings. A failed query adds the previous building's value again.
- **Starting record** *[compat]* (`0x006F9F27`–`0x006F9F79`, health `0x006F9DD7`–`0x006F9E29`): one record per
  call, built on the stack. Only +0x10, +0x28, +0x2C and +0x34 are set to 0 (and the string at +0). +0x14, +0x18,
  +0x1C, +0x20, +0x24, +0x30 and +0x38 are **not initialised**. So if the first building's query fails before it
  reads the effective capacity (no capacity or no effective capacity property), the sum starts with a stack
  leftover. A failure later in the query (served, grade or, for hospitals, the staff item) comes after +0x18 is
  written (above), so the sum still gets that building's own value. Every building in a group list has an
  effective-capacity property (created on add, §4.1), so only a building without a capacity reaches the leftover,
  and none ships. *[choice]* FreeCity4 starts the record at 0.
- rating := sum = 0 ? 0 : pop = 0 ? 1.0 : min(1.0, float(sum) / float(GetPopulation())).

**Totals** `GetSchoolSystemTotals` / `GetHospitalSystemTotals(totals*, groups*)` (14/15; both call one helper,
`0x006FA070`, with the service's group lists):

- **Reset first** *[compat]*: the caller's +0x14, +0x18, +0x1C, +0x20, +0x24 := 0 and +0x28 := 0.0
  (`0x006FA0A6`–`0x006FA0BB`). Nothing is added onto earlier contents; the string and +0x2C..+0x38 are not
  touched.
- **Which buildings** (corrected): every building of every group list, in list order (pending buildings are not
  in these lists), that is in **at least one** of the groups in `groups` (`IsOccupantGroup`, tested per building,
  so a building filed under another group still counts when it also has one of these groups).
- For each such building: run the service's query (slot 26 or 27) into one stack record, **ignore its result**,
  then add the record's +0x14..+0x24 (u32 sums) and +0x28 (float32 grade sum) to the totals and count the
  building (`0x006FA167`–`0x006FA1CF`).
- Return the **number of buildings counted** in EAX: every matching building, whether its query succeeded or not.
- **The stack record** (AQ1, `0x006FA0D5`–`0x006FA0E9`) is set up once per call, before the walk, like the ratings'
  record: only +0x10, +0x28, +0x2C and +0x34 are set to 0. +0x14..+0x24, +0x30 and +0x38 are stack leftovers, and
  nothing resets the record between buildings. So a failed query leaves what it read plus the previous building's
  other fields or, for the first building, the leftovers (§4.11 partial writes). Shipped data reaches this: when the
  farmer's market below is the first building the hospital totals count, its +0x1C and +0x20 are two stack words.
  *[choice]* FreeCity4 starts +0x14..+0x24 at 0, so that case adds 0 there.
- *[quirk]* `groups` must not be null. Null is dereferenced and crashes; an empty vector counts nothing.
  The dereference (`0x006FA11C`) happens per building, inside the list walk, so a null `groups` crashes only
  once some group list holds a building. With every group list empty, the original returns 0 after the reset
  (RG4). *[choice]* FreeCity4 treats null as an empty vector: the reset, then 0, which is the original's result
  whenever it does not crash.

*[compat]* This is reached by shipped data. The farmer's market (`0x8A3858D8 / 0x032C0000`) is a hospital through
its groups only and has no health staff item, so its hospital query always fails at the staff test. The hospital
totals still count it, with its own capacity, effective capacity and served (+0x24 raw: `0xFFFFFFFF` until its
first grading, which adds −1 modulo 2^32), and the previous building's +0x1C, +0x20 and +0x28.

**EstimateCurrentOccupantCapacity(occ, &cap, &staff)** (28, `0x006FD510`):

- **Service** *[compat]*: school if the building has the school capacity `0x8922041B`, otherwise hospital (with
  no test that the hospital capacity exists; a missing one reads as 0, so cap = staff = 0).

- e1 from the efficiency ratings with **ignoreStrike = true**.
- cap := trunc((float(capacity) × e1) × m2).
- staff := ceil(cap / **school** divisor) for both services *[quirk]*.

**GetOccupantCoverage(occ, const SC4Percentage& level, float& dist, float& radius)** (45):

- Kind: school if the building has `0x8922041B`; hospital if it has `0x69220415` (hospital wins if both).
  Neither → AL = 0 and no output is written (RG12: the two property probes at `0x006FD28B` and `0x006FD2C6`
  write only to a local; with neither found the code jumps to the epilogue `0x006FD502`, which returns the
  "kind found" byte, still 0, before any write to `dist` or `radius`).
- *[quirk]* "Hospital wins" covers only the radius property, the kind property and the ordinance list. The
  school/hospital flag passed to the efficiency ratings and to the building-curve read is set by the school test
  and not cleared by the hospital test (`0x006FD2A2`, `0x006FD2D8`). So a building with both capacities uses the
  school curves and funding with the hospital radius. No shipped building has both (§4.1).
- radius := (e2 × m1) × coverage radius.
- dist := the distance at which the building's distance curve (c3) falls to level × 100:
  - walk the points from the second one on, keeping the last point P_prev with y > level × 100;
  - at the first point P with y ≤ level × 100:
    dist := (((y_prev − L) / (y_prev − y)) × ((x − x_prev) × 0.01)) × radius + x_prev × radius,
    with L = level × 100;
  - if there is no such point, dist := 0.
  - *[quirk]* The x_prev term has no 0.01 factor.
  - With the default flat curve (0,100)(100,100): level < 1 gives dist = 0, and level = 1 gives NaN.
- AL = 1 when found.
- **Edge cases** *[compat]* (`0x006FD260`): AL is the "kind found" flag alone, so it is 1 in all of these.
  - No radius property: the radius local is set to 0 before the read (`0x006FD308`) and the read's result is not
    tested, so radius := (e2 × m1) × 0 = **0**. dist is computed as usual and multiplied by 0.
  - No point at or below the level: dist := **0** (written before the walk, `0x006FD435`).
  - A distance curve with one point: dist := 0. A curve with **no** points (a property with fewer than two values,
    foundations §4.1) skips the whole step, so dist is **not written** and keeps the caller's value *[quirk]*.
    *[choice]* FreeCity4 writes 0.

**Which service the three methods pick** (summary): slot 45 tests school then hospital (hospital properties win,
flag above); slot 28 tests only the school capacity (school first); slot 58 tries the school query, then the
hospital query (school first). They differ only for a building with both capacities.

### 4.12 History

State 9 records through a synchronous **message**, not the history-warehouse interface:

- `messageServer.MessageSend(m)` with type `0x89EFA536`, data1 = history ID, data2 = value (int32);
- this type is the history warehouse's class ID *[inferred: the warehouse handles it]*.

| History ID | Value |
|---|---|
| `0xAA1A2864` | trunc(GetGlobalEQ()) |
| `0xAA1A2866` | trunc(GetGlobalHQ()) |
| `0xAA1A2CCA` | total population (residential.md) |
| `0x4A8A3FE5` | school coverage ‰: round-half-even((float(servedSchool) / float(pop)) × 1000); 0 if pop ≤ 0 |
| `0x4A8A3FE2` | health coverage ‰: the same with servedHealth |

Population and served values are loaded as **signed** int32. The last two IDs are registered with the history
warehouse in `Init` (residential.md).

### 4.13 Tuning (education/health properties)

Residential tuning exemplar (type, group, instance) = (`0x6534284A`, `0xE7E2C2DB`, `0x69189C84`). It is read by
`ReadSimulatorTunableValues` at `Init`, by `ReadTunablePopulationValues` before every state-4 pass, and by the LE
queries on every call. Names come from re/tuning/simulators.md. Percent values are read as f32 × 0.01
("pct"). Read patterns (foundations §4.1; residential.md §7.1–§7.3 is normative for the loaders):

- **Department values and the funding/strike curves** (`ReadSimulatorTunableValues`): the code defaults are set
  unconditionally (including strike flag := false and strike start := 0, §6), then the properties are read only
  if the exemplar exists. Curves: **default** pattern (rebuilt from the code point list, then loaded by key). A
  missing property keeps the code default.
- **Staff divisors, slop multiplier, maximum boosts and chunk sizes** are set only when the exemplar exists; the
  "code default" column gives the value set in that branch. Without the exemplar they stay uninitialised
  (residential.md §7.2, *[choice]* there).
- **Pollution curves** (`ReadTunablePopulationValues`): **default** when the exemplar exists, **keep** when it
  is missing (residential.md §7.1).
- **HQ → LE curve**: a fresh temporary curve per call, **default** pattern; a missing exemplar makes the LE query
  return 0 (residential.md §7.3).

| ID | Name | Read as | Code default | Shipped |
|---|---|---|---|---|
| `0xE918946F` / `0xC91894A8` | (OBS) Health / School Min Acceptable Funding % | pct | 0.6 | absent from the shipped exemplar; loaded, but the value is **never used** *[inferred]* (name from re/prior_art_exemplar_props.md) |
| `0xA9189479` / `0x891894AD` | Health / School Init/Max Allowed Funding | f32[2] × 0.01 → initial, max | 0.25, 1.2 | 25, 120 |
| `0x89189482` / `0xE91894B3` | Health / School Strike Efficiency % | pct | 0.2 | 20 |
| `0x8918948B` / `0xA91894BC` | Health / School Strike Duration | s32 | 10 | 500 (*[compat]* compared in **days**; the name says months) |
| `0x89189495` / `0x691894C0` | Health / School Min Grade to End Strike | SC4Percentage getter (× 0.01) | 0.9 | 90 → 0.9 (compared with the **staff ratio**) |
| `0x2918949E` / `0x691894C6` | Health / School Funding To Efficiency Curve | curve | (0,0)(100,100)(120,110) | same |
| `0x0A53B2F9` / `0xEA53B30F` | Health / School Funding to Coverage Curve | curve | (0,20)(100,100) | health (0,50)(100,100); school (0,50)(100,100)(120,110) |
| `0x691894A4` / `0xC91894CB` | Health / School Strike Response Curve | curve | (0,20)(100,100) | (0,100)(80,0) |
| `0x8A891421` | School Capacity To Staff Divisor | u32 | 30 | 30 |
| `0x8A89142A` | Hospital Capacity To Staff Divisor | u32 | 30 | 30 |
| `0x6A9F6EDD` | Coverage radius slop multiplier | f32 | 1.1 | 1.1 |
| `0x4AA84161` / `0x2AA84164` | Maximum EQ / HQ Boost | f32 | 10.0 / 10.0 | 1.3 / 1.1; **loaded, never read** |
| `0xEC43826B` | School Capacity Chunk Size | u32 array | none (empty) *[quirk: then indexed out of range]* | 500, 750, 1000, 1000, 10000, 300, 1200 |
| `0xAC43826D` | Hospital Capacity Chunk Size | u32 array | none | 500, 500, 2000 |
| `0xC929C433` / `0x0929C439` | EQ / HQ Decay | f32 | 1.0 / 1.0 | 0.2 / 0.2 |
| `0xC929CC58` / `0x0929CC65` / `0x2929CC6B` | Low / Medium / High Wealth EQ (newcomers) | float getter (converts) | 4 / 6 / 8 | u8 20 / 40 / 60 |
| `0xC929CC8D` / `0x6929CC92` / `0x4929CC97` | Low / Medium / High Wealth HQ | float getter | **none: uninitialised** *[quirk]* | u8 20 / 40 / 60 |
| `0xCA027325` / `0xCA027328` | EQ / HQ reset percent | SC4Percentage getter (× 0.01f) | 0.7 / 0.7 | 80 → 0.79999995 (`0x3F4CCCCC`; RS1) |
| `0xCA027307` | Average Age reset value | u8 | 40 | 40 |
| `0x6AB70DA7` | Death's door HQ | f32 | 0 | 2 |
| `0x4AB70DAC` | Death's door effect chance (%) | f32 | 0 | 1.5 |
| `0x6AB71654` / `0x4AB71659` | Death's door effect height / height delta | f32 | 0 / 0 | 50 / 10 |
| `0x6941B022` | Air Pollution to HQ Decay Curve | curve | (0,0)(32000,100) | (0,0)(512,0.1)(1024,50) |
| `0xA941B028` | Water Pollution to HQ Decay Curve | curve | (0,0)(32000,100) | (0,0)(512,0.1)(1024,50) |
| `0xC941B02E` | Garbage Pollution to HQ Decay Curve | curve | (0,0)(32000,100) | (0,0)(512,0.2)(1024,20) |
| `0x6941A67F` | Health Quotient to Life Expectancy Curve | curve (per LE call) | (0,0)(200,200) | (0,50)(20,55)(40,60)(60,65)(200,100) |

Notes:

- The ages (`0xC929C652`, `0x0929CC30`, `0x0929CC36`) and the nine age-group curves are in residential.md.
- *[compat]* The newcomer EQ/HQ values ship as u8 and are read with the converting float getter.
  docs/subsystems/demand-rci.md quotes the code defaults (4/6/8, reset 0.7). The shipped values are 20/40/60 and
  80 %, which the getter turns into 0.79999995 (`0x3F4CCCCC`), not 0.8f.
- If the exemplar is missing, `ReadTunablePopulationValues` leaves the newcomer HQ values uninitialised. A
  replacement should use the shipped values *[choice]*.

**Building properties** (from building exemplars, read per call):

| Property | School | Hospital | Default when missing |
|---|---|---|---|
| coverage radius (m) | `0x691B42B3` | `0x091B42A2` | building skipped |
| capacity | `0x8922041B` | `0x69220415` | skipped (coverage) / 0 |
| effect multiplier | `0xA92AE446` | `0xC9299659` | 1.0 |
| age / wealth / distance curves | `0xC9299662` / `0xC92D7EC0` / `0xE921A936` | `0xA92AE3BD` / `0xA92D9FCD` / `0x2921A9CC` | (0,100)(100,100) |
| ordinance kind (u32 array) | `0xA92D8E40` | `0x292D8F9B` | — (filter unused) |

## 5. Messages posted

| Type | When | Fields | How |
|---|---|---|---|
| `0x077ACCD0` (school) / `0x077ACCD2` (health) **strike begins** | `BeginStrike` | void1 = the building's occupant; IGZUnknown = the same occupant; data2 = 1 | `MessagePost(m, 0)` |
| same types, **strike ends** | `EndStrike` | data2 = 0; data3 = length in days (today − start) | `MessagePost(m, 0)` |
| `0x89EFA536` history record | state 9 | data1 = history ID; data2 = value | `MessageSend(m)` (synchronous) |

Each message is a new `cRZMessage2Standard` released after posting.

Every field the table does not name is **0 or null** *[compat]* (RG19): the message constructor (`0x009134D6`)
zeroes `+0x04` to `+0x28`, and neither strike call sets data1 (so data1 = 0 in both), data3 on begin, or
void1/IGZUnknown on end.

## 6. Persistence

The layout is in residential.md §9. Meaning only:

- **Saved:** the coverage, grade, EQ and HQ grids (as grid references); the per-service department blocks
  (including the strike flag and start day); the global auto-budget flags (version ≥ 7); the group and pending
  building lists (as references).
- *[quirk]* The loaded strike flags and start days are overwritten by the next `Init` (the tuning read resets
  them), so a city-level strike ends on load without an end-of-strike message. The per-building strike start
  day (property `0xEAA69F83`) is kept. The city flag is set again only when a later monthly check (§4.9) sees
  some building change state while a building is striking (BeginStrike is called only after a change).
- **Not saved:**
  - the ordinance effect lists (rebuilt in `Init`);
  - the tuning (re-read);
  - served counters and grade totals (recomputed in the next cycle);
  - the RNG (foundations §6.2).
- Per-building values (served, grade, staff ratio, effective capacity, strike day, auto-budget) live in the
  buildings' property holders and are saved with the buildings.
- *[inferred]* After a load, the grades shown before the first state 6/8 are the saved building properties.

## 7. Interface methods (education/health slots of `cISC4ResidentialSimulator`, vtable `0x00AB2C10`)

Callee-pop values were checked with `abi_extract.py --check`; return channels by reading the epilogues.

| Slot | Off | Signature as the game expects it | Pop | Return | Notes / SDK |
|---|---|---|---|---|---|
| 6 | +0x18 | `bool SchoolIsOnStrike()` | 0 | AL | — |
| 7 | +0x1C | `bool HealthIsOnStrike()` | 0 | AL | — |
| 8 | +0x20 | `bool EndSchoolStrike()` | 0 | AL | — |
| 9 | +0x24 | `bool EndHealthStrike()` | 0 | AL | — |
| 10 | +0x28 | `SC4Percentage* ChanceOfSchoolStrike(SC4Percentage* out)` | 4 | EAX = out | **SDK error**: SDK `float ChanceOfSchoolStrike()` (checked `006F9670`). |
| 11 | +0x2C | `SC4Percentage* ChanceOfHealthStrike(SC4Percentage* out)` | 4 | EAX = out | **SDK error** (`006F9730`). |
| 12 | +0x30 | `SC4Percentage* GetSchoolSystemRating(SC4Percentage* out)` | 4 | EAX = out | **SDK error**: SDK float (`006F9F20`). |
| 13 | +0x34 | `SC4Percentage* GetHealthSystemRating(SC4Percentage* out)` | 4 | EAX = out | **SDK error** (`006F9DD0`). |
| 14 | +0x38 | `uint32 GetSchoolSystemTotals(tQueryData* totals, SC4Vector<uint32>* groups)` | 8 | EAX = count | **SDK error**: SDK `bool`; groups must be non-null (`006FA240` → `006FA070`). |
| 15 | +0x3C | `uint32 GetHospitalSystemTotals(...)` | 8 | EAX = count | **SDK error** (`006FA260`). |
| 17 | +0x44 | `int32 GetTotalCityEducationUpkeepCost()` | 0 | EAX = 0 | `004E13E0`: always 0. |
| 18 | +0x48 | `int32 GetTotalCityHealthUpkeepCost()` | 0 | EAX = 0 | same body. |
| 19 | +0x4C | `? SetOccupantFundingPercentages(cISC4Occupant*, const SC4Percentage& staff, const SC4Percentage& vehicle, bool isSchool)` | 16 | garbage | `006F8290`: no return value is set. A replacement should return true *[choice]*. The SDK names the arguments "school/health"; they are staff/vehicle of one service. |
| 20 | +0x50 | `? GetOccupantFundingPercentages(cISC4Occupant*, SC4Percentage& staff, SC4Percentage& vehicle, bool isSchool)` | 16 | garbage | `006F84B0` → `006F8360`. Same remarks. |
| 21 | +0x54 | `? GetAverageEQGrid(cISC4SimGrid<float>*& out, float* min, float* max)` | 12 | garbage (the `max` pointer) | `006FCF30`. The caller owns `out` (refcount 1). Replacement: return true. |
| 22 | +0x58 | `GetAverageHQGrid` (same) | 12 | garbage | `006FCFD0`. |
| 23 | +0x5C | `GetEQGrids(cISC4SimGrid<float>*&, cISC4SimGrid<float>*&, cISC4SimGrid<float>*&)` | 12 | garbage | All three are outputs and **AddRef'd** (caller releases). **SDK error** (2nd/3rd are not references), already reported by crime.md (`006F72C0`). |
| 24 | +0x60 | `GetHQGrids` (same) | 12 | garbage | `006F7300`. |
| 26 | +0x68 | `bool GetSchoolQueryData(cISC4Occupant*, tQueryData*)` | 8 | AL | `006F91C0`. |
| 27 | +0x6C | `bool GetHospitalQueryData(cISC4Occupant*, tQueryData*)` | 8 | AL | `006F7E80`. |
| 28 | +0x70 | `? EstimateCurrentOccupantCapacity(cISC4Occupant*, uint32& cap, uint32& staff)` | 12 | garbage | `006FD510`. **SDK error** (bool). Replacement: return true. |
| 29 | +0x74 | `uint8 GetCellLifeExpectancy(uint32 x, uint32 z)` | 8 | **AL only** | **SDK error**: SDK int32; bits 8–31 of EAX are garbage (`006FC570`). |
| 32 | +0x80 | `float GetGlobalEQ()` | 0 | ST0 | `006F7720`. |
| 33 | +0x84 | `float GetGlobalHQ()` | 0 | ST0 | `006F7870`. |
| 34 | +0x88 | `uint8 GetGlobalLE()` | 0 | **AL only** | **SDK error**: SDK `float`; nothing is left on the x87 stack (`006FC9E0`). |
| 35 | +0x8C | `float GetCellEQ(uint32 x, uint32 z)` | 8 | ST0 | `006F79C0`. |
| 36 | +0x90 | `float GetCellHQ(uint32 x, uint32 z)` | 8 | ST0 | `006F7A10`. |
| 37 | +0x94 | `float GetCellEQByWealth(uint32 x, uint32 z, uint8 wealth)` | 12 | ST0 (from `GetTractValue`) | `006F7A60`. wealth 1..3. Checked (RG22): x and z are divided (unsigned) by the R$ EQ grid's `GetTractSize`, the grid is `EQ[wealth − 1]` (`+0x7C`; HQ `+0x88`), and the last call is the float grid's slot 22 `GetTractValue` (`0x0045F210`, which ends with `fld`); the method returns with `ret 0xC` and leaves that ST0 in place. So the return channel is x87 with high confidence. |
| 38 | +0x98 | `float GetCellHQByWealth(...)` | 12 | ST0 | `006F7AB0`. |
| 39 | +0x9C | `cISC4SimGrid<float>* GetSchoolAverageGradeMap()` | 0 | EAX ptr, **no AddRef** | `00809240`. SDK error already reported by aura.md. |
| 40 | +0xA0 | `cISC4SimGrid<float>* GetHospitalAverageGradeMap()` | 0 | EAX ptr | `00407AC0`. |
| 41 | +0xA4 | `cISC4SimGrid<uint8>* GetAverageAgeMap()` | 0 | EAX ptr, no AddRef | **SDK error**: SDK int32 (`006AAEB0`). Age grid of residential.md. |
| 43 | +0xAC | `bool GetEQMinAndMaxCellCoords(uint32&, uint32&, uint32&, uint32&, float&, float&)` | 24 | AL | `006F7380`. |
| 44 | +0xB0 | `bool GetHQMinAndMaxCellCoords(...)` | 24 | AL | `006F7550`. |
| 45 | +0xB4 | `bool GetOccupantCoverage(cISC4Occupant*, const SC4Percentage& level, float& distance, float& radius)` | 16 | AL | `006FD260`. **SDK naming error**: the SDK's "fRangeX, fRangeZ" are (distance at `level`, radius). |
| 46 | +0xB8 | `int32 GetSchoolBuildingCount()` | 0 | EAX | `006F9590`. |
| 47 | +0xBC | `? GetSchoolBuildings(SC4List<cISC4Occupant*>&, SC4Vector<uint32>* groups)` | 8 | garbage | `006FADE0`. **SDK error** (bool). Appends with AddRef; the caller releases. Replacement: return true. |
| 48 | +0xC0 | `int32 GetHospitalBuildingCount()` | 0 | EAX | `006F9600`. |
| 49 | +0xC4 | `? GetHospitalBuildings(...)` | 8 | garbage | `006FAE50`. |
| 50 | +0xC8 | `int32 GetMaxEQ()` | 0 | EAX = 200 | `006FEA40`. |
| 51 | +0xCC | `int32 GetMaxHQ()` | 0 | EAX = 200 | same body. |
| 52 | +0xD0 | `bool GetGlobalAutoBudgetForSchools()` | 0 | AL | `006FEA50`. |
| 53 | +0xD4 | `bool SetGlobalAutoBudgetForSchools(bool)` | 4 | AL = argument | `006FEA60`. |
| 54 | +0xD8 | `bool GetGlobalAutoBudgetForHospitals()` | 0 | AL | `006FEA70`. |
| 55 | +0xDC | `bool SetGlobalAutoBudgetForHospitals(bool)` | 4 | AL = argument | `006FEA80`. |
| 56 | +0xE0 | `bool GetAutoBudget(cISC4Occupant*)` | 4 | AL | **SDK error**: SDK `GetAutoBudget(void)` (`006F7D70`). Reads `0x6C05FC4D`, default false. |
| 57 | +0xE4 | `bool SetAutoBudget(cISC4Occupant*, bool)` | 8 | AL (property setter result) | **SDK error**: SDK `SetAutoBudget(bool)` (`006F7DA0`). |
| 58 | +0xE8 | `bool EstimateIdealFunding(cISC4Occupant*, SC4Percentage& out)` | 8 | AL | `006FD680`. |
| 59 | +0xEC | `void ToggleTractTracking(int32, int32)` | 8 | none | `006F7DC0`: empty (`ret 8`). |

"garbage" means the original leaves an unrelated value in EAX. No caller found in the game tests it
*[inferred]*. Slots 39–41 return borrowed pointers.

**Callers of the "garbage" slots** (RG11). A scan for calls to these slots through the residential simulator global
(`0x00B43DA4`) found 14 sites (slots 19, 21, 22, 23, 25, 28, 47 and 49), and none tests EAX or AL afterwards: `0x004C7B62`, `0x004C7C95`, `0x004C7CFA`,
`0x004C7FF1`, `0x004C8114`, `0x004C8179` (query and budget UI), `0x0068DAD2` (aura), `0x0069FB24` and
`0x0069FB3E` (crime), `0x0077BE7B` (sub-budget window), `0x007A1B53` and `0x007A1BC1` (map-view highlights),
`0x007A32CC` and `0x007A32D1` (map-view update). So a replacement may return true from slots 19, 20, 21–25, 28,
47 and 49. Callers that keep the simulator pointer in a member of their own were not all traced, so this stays
*[inferred]* for them.

## 8. Dependencies

| Interface (vtable) | Slot | Off | Method | Pop | Return |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 47 | +0xBC | GetAutomataControllerManager | 0 | EAX |
| | 50 | +0xC8 | GetSimulator | 0 | EAX |
| | 52 | +0xD0 | GetBudgetSimulator | 0 | EAX |
| | 95 | +0x17C | PositionToCell (dead call in grading; result unused) | 16 | — |
| `cISC4Simulator` | 7 | +0x1C | GetSimDate | 0 | EAX |
| `cIGZDate` | 26 | +0x68 | DayNumber | 0 | EAX |
| `cIGZMessageServer2` (`0x00AD1BF4`) | 3 | +0x0C | MessageSend(m) | 4 | unused |
| | 4 | +0x10 | MessagePost(m, 0) | 8 | unused |
| `cISC4BudgetSimulator` (`0x00AB0908`) | 25 | +0x64 | CreateDepartmentBudget(id, group) | 8 | EAX |
| | 28 | +0x70 | GetDepartmentBudget(id) | 4 | EAX |
| | 51 | +0xCC | GetBudgetItemForPurpose(holder, purpose, item&) | 12 | AL |
| | 52 | +0xD0 | ChangeBudgetItemLine(holder, index, line) | 12 | — |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 5 | +0x14 | SetDepartmentName(group, instance) | 8 | — |
| | 10 | +0x28 | GetFundingPercentage() | 0 | EAX ptr |
| | 11 | +0x2C | SetFundingPercentage(const SC4Percentage&, arg) | 8 | — |
| | 13 | +0x34 | SetMaxAllowedFundingPercentage(const SC4Percentage&) | 4 | — |
| | 17 | +0x44 | CreateLineItem(id, isLocal) | 8 | EAX |
| | 18 | +0x48 | CreateLineItemForBuildingType(id, isLocal) | 8 | EAX |
| | 19 | +0x4C | RemoveLineItem(id) | 4 | — |
| | 20 | +0x50 | GetLineItem(id) | 4 | EAX |
| | 22 | +0x58 | AddLocallyFundedObject(holder, purpose) | 8 | — |
| | 23 | +0x5C | RemoveLocallyFundedObject(holder, purpose) | 8 | — |
| | 24 | +0x60 | SetLocalFundingPercent(holder, const SC4Percentage&, purpose) | 12 | — |
| | 28 | +0x70 | IsLocallyFundedObjectInDepartment(holder, purpose) | 8 | AL |
| `cISC4LineItem` (`0x00AB0F58`) | 8 / 9 | +0x20 / +0x24 | count get (int64, EDX:EAX) / set (int64) | 0 / 8 | — |
| | 18 | +0x48 | SetDisplayFlag(flag, on) | 8 | — |
| `cISC4AutomataControllerManager` (`0x00A96058`) | 10 | +0x28 | AddControllersFromOccupantGroup(name, occ, 0) | 12 | — |
| | 12 | +0x30 | RemoveControllers(name, occ) | 8 | — |
| `cISC4OrdinanceSimulator` (`0x00ABEAC0`) | 14 | +0x38 | GetOrdinanceOnCount | 0 | EAX |
| | 15 | +0x3C | GetOrdinanceOnArray(out, &n) | 8 | EAX |
| | 20 | +0x50 | GetOrdinanceByID(id) | 4 | EAX (no AddRef) |
| `cISC4Ordinance` (`0x00ABDD38`) | 16 | +0x40 | GetMiscProperties | 0 | EAX |
| `cISC4PollutionSimulator` (`0x00AB2630`) | 6 | +0x18 | GetPollutionValue(type, x, z) | 12 | AX only |
| `cISC4EffectsManager` (`0x00A9F264`) | 7 | +0x1C | CreateVisualEffect(const char* name, cISC4VisualEffect** out) | 8 | unused; the class tests `out` |
| `cISTETerrain` (`0x00AB3D18`) | 13 | +0x34 | GetAltitude(x, z) | 8 | ST0 |
| `cISC4BuildingOccupant` (`0x00A872C0`) | 3 / 4 | +0x0C / +0x10 | AsOccupant / GetBuildingType | 0 | EAX |
| `cISC4Occupant` (`0x00AA41D8`) | 6 | +0x18 | AsPropertyHolder | 0 | EAX |
| | 8 | +0x20 | GetPosition(float*) | 4 | — |
| | 22 | +0x58 | IsOccupantGroup(id) | 4 | AL |
| `cISC4LotManager` (`0x00AB37C0`) | 14 | +0x38 | GetOccupantLot(occ) | 4 | EAX |
| `cISC4Lot` (`0x00AB19B0`) | 15 | +0x3C | GetCenter(float*) | 4 | — |
| `cISC4SimGrid<T>` (simgrid §5) | | +0x18, +0x1C, +0x24, +0x28, +0x30, +0x34, +0x38, +0x4C, +0x58, +0x60, +0x68, +0x6C | SetInstanceID, GetCellValue, SetTractSize, GetTractSize, counts, GetTractWidthX, PositionToTract, GetTractValue, GetGridData, SetTractValue, SetTractValues | | |
| `cRZRandom` | — | — | `RandomDoubleRangeUniform` `0x00913761` (named `cSC4ParticlesEffect::RandomRange` in names_windows) | thiscall, 16 | ST0 |
| Response curves | — | — | ctor `0x007D0410`, point list `0x005F4480`, key loader `0x005F4700`, property-pointer loader `0x005F4620`, `GetOutputValue` `0x005F4250` (pop 4, ST0) | | |
| Property helpers | — | — | getters `0x005FD480` u32, `0x005FD4F0` f32 (converting), `0x005FD450` s32, `0x005FDAC0` pct, `0x005FD390` bool, `0x005FF510` u32 vector; setters `0x005FE3C0` u32, `0x005FE4F0` f32, `0x005FE210` bool | | |
| Shared map helpers | — | — | `GetTractsInRadius` `0x005E5200`, `AddRadiusEffectWithCurves` `0x005E48F0` (cdecl, §4.4, §4.6) | caller | — |

## 9. Replaceability

**Readers outside the class.** The survey covered every use of the residential-simulator global and every
`GetResidentialSimulator()` call next to a city access.

| Reader | What it uses | Raw access? |
|---|---|---|
| Aura simulator (`DoIdle`) | slots 39/40 (grade maps) | **Yes**: `ProcessTwoGridsWithCallback` reads the grids through `GetGridData` (aura spec, simgrid §7.3). |
| Aura (`BustStrike`) | slots 8/9 | no |
| Crime simulator (`DoCensus`) | slots 23/25 (EQ and population grids) | **Yes**, through `GetGridData` (crime spec). |
| Map view (`cSC4WinMapView::Update`) | slots 21/22 (average EQ/HQ grids, read through `GetGridData`); slot 41 (age grid, through `GetGridData`) | **Yes** |
| Map view highlights, coverage-radius effect helper | slots 47/49, 45 | no |
| Growth developer (`EstimateDesirability`, `DoIdle`) | slots 39/40 via `GetCellValue` | no |
| Query tool, sub-budget window | slots 39/40 (`GetCellValue`), 35/36, 26/27, 19, 28, 59 | no |
| Advisors (HQ, Safety), MySim, Lua glue, budget window, tutorial | strike, rating, LE, EQ/HQ, query-data, funding slots | no |

**Grid ownership.**

- Every grid in §2 is created by the class with the game's own `cSC4SimGrid<T>` vtables (class IDs `0x49B9E602`
  u8 and `0x49B9E60A` float). The original constructs them inline, not through GZCOM.
- The grade, EQ, average-EQ/HQ and age grids are read raw by the aura, crime and map view.
- A replacement must therefore follow **simgrid §6.4**: create the grids through GZCOM with those class IDs and
  keep the tract sizes and instance IDs of §2.
- `GetAverageEQGrid` / `GetAverageHQGrid` hand the caller a new grid with refcount 1. A replacement must do the
  same.

**Non-virtual access.**

- No code outside the class calls its non-virtual members or reads its memory directly. Every access in the
  survey is a virtual call.
- The strike messages (§5) and the per-building properties (§2) are a contract with the budget window, advisors
  and query tool. A replacement must keep the property IDs and value conventions: served = `0xFFFFFFFF` until
  graded, and strike day = start day number.

**GZCOM.** The class is created by the city through GZCOM (class ID `0x4990C013`, foundations §3.6). It is
replaceable as a whole. This half cannot be replaced without the population half (residential.md), because
both share the grids and the state machine.

## 10. Quirks and open questions

Quirks reproduced *[compat]*. Each is described in the section given:

1. Coverage units of 0 still count served population, possibly twice (§4.4).
2. Coverage saturation tests the low 16 bits of the sum (§4.4).
3. The grade grid blend over-weights earlier buildings (§4.5).
4. Served-population distance: the unsigned dx and the tracts-per-metres units (§4.5).
5. `AddRadiusEffectWithCurves` skips the z advance on tracts with no population (§4.6).
6. `AddRadiusEffectWithCurves` uses a 0..1 distance fraction, unlike the 0..100 used elsewhere (§4.6).
7. The pollution average for HQ decay uses only its low byte, and reads pollution **sources** (§4.7).
8. Strike duration is compared in days, although the tuning name says months (§4.9).
9. "Min grade to end strike" is compared with the staff ratio, not the grade (§4.9).
10. `GetChanceOfStrike` reuses c for non-members, and g carries over between buildings (§4.9).
11. The sticky `changed` flag in the monthly strike check (§4.9).
12. Ratings add stale query data (§4.11).
13. The capacity estimate uses the school divisor for hospitals; the hospital query floors where the school
    query rounds up (§4.11).
14. A null groups pointer crashes the system totals (§4.11).
15. `GetCellEQByWealth` with wealth 0 reads out of bounds (§4.8).
16. Unmatched pending buildings are leaked (§4.1).
17. Maximum EQ/HQ Boost and Min Acceptable Funding are loaded but never used (§4.13).
18. The newcomer HQ values are uninitialised without the exemplar (§4.13).
19. `GetOccupantCoverage` distance is missing a 0.01 factor (§4.11).
20. LE and global LE are byte-valued (§7).
21. City-level strikes end silently on load: the next `Init` resets the strike flags and start days (§2, §6).

Open questions:

1. Is the dirty flag read anywhere? No reader was found in the class *[inferred: write-only]* (also residential.md
   §13).
2. What are the group names of `0x1514`, `0x1505`, `0x151B`, `0x1512`, `0x1513`, `0x151C`, `0x1A02`, `0x1A03`?
   *[inferred: `0x1A02`/`0x1A03` are "can strike" groups for health/education]*.
3. ~~Who reads the grade totals (§4.6)?~~ **Nobody** (RS13; §3 table): the class only resets and accumulates
   them, and nothing outside it can reach them. (The age histogram is read by residential.md §5.6.)
4. What does the building's slot-11 value (§4.1 step 4) mean? (The LE-weighted population sum is the average
   life expectancy history, residential.md §5.6.)
5. ~~The population tract size and the EQ tract size are equal on a new city. Does a loaded city keep them equal?
   This affects aura question 4.~~ **Yes** in every game-written save (EH13, EH14): P, E and each grid's own tract
   size are saved and stay 4. The simulation asks each grid for its own size; P and E only size a grid that `Init`
   recreates (residential.md §9.2). `AddRadiusEffectWithCurves` takes its geometry from the EQ/HQ grid alone
   (§4.6).

## 11. Conformance tests

These must exist in `engine/conformance`. Unless a test says otherwise:

- tunables are the shipped values;
- the tract size is S = 4 cells of 16 m (tract width 64 m);
- there are no ordinances;
- funding is 100 %.

1. **Coverage units.** K-6 school (chunk 500), capacity 1500 → every covered tract of coverage grid 0 rises by
   3. Two such schools over the same tract → 6. A tract at 250 plus units 10 → 255.
2. **Units 0 still serve.** School capacity 400 (chunk 500), tract population (10, 20, 0), tract not covered by a
   lower group → coverage stays 0, served counter += 30. A second identical school → += 30 again (total 60).
3. **Tract list** (`GetTractsInRadius`, width 64 m, 8 × 8 tracts, slop 1.0, centre (32, 32) m).
   - Radius 60 m: the bounds are x, z ∈ 0..1. (0,1) has d² = 4096 > 3600 and (1,1) has 8192, so the list is
     (0,0) only.
   - Radius 100 m: the bounds are 0..2. The list is (0,0), (0,1), (1,0), (1,1), in that order (x-major);
     (0,2) has d² = 16384 > 10000.
   - Centre (−10, 32), radius 100: x0 = trunc(−1.71875) = −1, clamped to 0.
4. **Staff ratio and grade.** capacity 1000, efficiency curve at 100 → 1.0, m2 = 1.
   - served 1250 → staff ratio 0.8, effCap 1000, grade 0.8.
   - served 0 → both 1.0.
   - served 500 → both 1.0 (clamp).
5. **Grade grid blend.**
   - One building, units 3, grade 0.5, n = 3, previous value 0 → 0.5.
   - The same tract with a previous value of 0.5 → 1.5, clamped to 1.0.
6. **Served rounding.** sum 2.5 → 2; 3.5 → 4; 2.4999 → 2.
7. **Strike roll.** grade 0.4 → p = curve(40) = 50.
   - draw r = 49.9 → strike starts; the property = today; message `0x077ACCD0` posted with data2 = 1.
   - draw r = 50.0 → no strike.
   - Exactly one `cRZRandom` double draw per eligible non-striking building.
8. **Strike end.**
   - Start day 1000, today 1501, duration 500 → ends (501 > 500).
   - Today 1500, staff ratio 0.9 → ends (0.9 ≥ 0.9).
   - Staff ratio 0.89 → continues.
   - City-level end with no striking building left: message with data2 = 0, data3 = today − start.
9. **Chance of strike.** Two groups with chances 0.5 and 0.5 → 0.75. Service on strike → 1.0.
10. **System rating.** Σ effCap 3000, population 4000 → 0.75. Population 0 → 1.0. Σ effCap 0 → 0.
11. **History ‰.** served 1234, population 5000 → 246.8 → 247. Population 0 → 0.
12. **Tract quotient.** EQ (100, 50, 0), population (10, 20, 0) → 2000 / 30 = 66.666664 (float32). The
    average grid stores 66.0.
13. **Life expectancy.** GetCellHQ = 30 → curve 57.5 → AL = 57. HQ = 100 → 75. HQ = 0 → 50.
14. **EQ decay.** EQ 10 → 9.8. EQ 0.1 → 0.
15. **HQ pollution decay byte.** Average air source 600, no water or garbage pollution → index 88 →
    HQdec = 0.2 + 0.0171875 = 0.2171875. With average 256 → index 0 → 0.2.
16. **Newcomer blend.** old 100, EQ_old 50, new 150, newcomer EQ 20 → ((35 × 50) + 5000) / 150 = 45.0. With
    EQ_old 0 → (20 × 50 + 0) / 150 = 6.6666665.
17. **Generational reset.** Average age 60 > LE 57 → age 40, EQ 50 → 40, HQ 30 → 24.
18. **Ideal funding.** School efficiency curve (0,0)(100,100)(120,110), max 1.2.
    - served 500, capacity 1000 → r = 0.5 → result 0.5.
    - served 1500 → r = 1.5 → climbs past 1.2 → result 1.2.
19. **Ordinance.** Pro-reading enacted → every school's m3 = 1.1. EQ boost value_w scales by 1.1 from the next
    state 0.
20. **Capacity estimate.**
    - capacity 1000, school divisor 30 → cap 1000, staff 34.
    - A hospital with the school divisor set to 25 and the hospital divisor 30 → staff 40.
21. **Query data.** Hospital capacity 1000, effCap 990, divisor 30 → +0x1C = 33, +0x20 = 33. School with the
    same numbers → +0x1C = 33, +0x20 = 33 (ceil(990 / 30) = 33). effCap 991 → school 34, hospital 33.
22. **Min/max coordinates.** S = 4; tract (2,3) quotient 10 and tract (5,1) quotient 80; every other tract empty
    → min (10, 14) value 10, max (22, 6) value 80, AL = 1. Population 0 → all outputs 0, AL = 0.
23. **Death's door.** HQ after decay 1.0 < 2, population > 0, draw 1.2 < 1.5 → "buzzards" created, with a second
    draw for the height. Draw 1.5 → nothing, and a second wealth can still draw.
24. **ABI.**
    - Slot 34 returns its value in AL with the x87 stack unchanged.
    - Slots 10–13 write through the hidden pointer and return it.
    - Slot 14 returns the number of buildings counted.
    - Slot 56 pops 4 and slot 57 pops 8.
