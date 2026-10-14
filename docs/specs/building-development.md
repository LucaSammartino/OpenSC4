# Building development simulator (`cSC4BuildingDevelopmentSimulator`) — behaviour spec

Status: round 1 (M4), written from the Windows 1.1.641 executable. The earlier write-up in
`docs/subsystems/development.md` (largely from the Mac build) was the starting point. Every fact below was checked
against the Windows code; corrections to the earlier write-up and to other specs are in §10.3.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, references and the class store (foundations §2, §3), city
component table and Init order (foundations §3.6, §3.7), messages (§5), random generators (§6), month order (§13),
slot-order rule (§16), framework slot tables (§17), x87 precision (§18); scheduler levels, agent semantics and the
CPU-cycle limit timer (simulator spec §4, §5.5); grid classes and the grid ownership rule (simgrid spec §6.4); demand
nodes (demand spec §3). The growth developer, tract developer, lot developer and zone manager are specified in
`growth-developer.md`, `tract-developer.md`, `lot-developer.md` and `zones.md`; this spec only states the call
contracts it relies on.

Notation: `r24(·)` rounds to float32 (foundations §18); `trunc(·)` is the CRT `_ftol` (int64 truncation toward
zero; NaN gives low 32 bits 0, foundations §18.1). "Developer type" (`DeveloperType`) values are the hex codes of
§1.5 (`0x1010` … `0x4400`). Purposes and wealths are the `cISC4BuildingOccupant` enums (purpose 1 residence,
2 services, 3 office, 4 tourism, 5 agriculture, 6 processing, 7 manufacturing, 8 high tech; wealth 1–3 = $, $$,
$$$, 0 = none).

---

## 1. Role and identity

The building development simulator (BDS) is the hub of zoned growth:

- It owns the **growth developers** (one `cSC4GrowthDeveloper` per developer type, created from developer
  exemplars) and drives their monthly desirability-map rebuild.
- Every day it gives each developer with positive demand **one** growth attempt, in a demand-weighted random
  order, on the next not-yet-tried tract of the developer's growth list (§4.4). The attempt itself is
  `cISC4TractDeveloper::Grow` (`tract-developer.md`).
- Once a month it plans **decline** for developers with strongly negative demand (§4.3) and executes at most one
  successful decline per day (§4.5).
- It holds the **construction-resource budget**: a demand node whose supply is a clamped fraction of the city's
  R + C + I census and whose demand is the capacity of buildings currently under construction (§4.6).
- It holds the process-wide **building catalogue** (types, families, names, profiles, foundations, retaining
  walls, and the per-zone maximum density table) built once from the game's exemplars (§4.7).
- It keeps the **building map** (a `uint8` grid of building keys, read raw by the residential and commercial
  simulators), a per-developer **highest growth stage reached** (with a CityPlanning advisor event), the list of
  **landmark and reward buildings**, terrain-change handling for lots (§4.11), three cheats and a debug command.

### 1.1 Identity

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`) | `0x8990BE7A` |
| Object size, allocator | `0xE4` bytes, global heap C (factory `0x0069A7A0`, constructor `0x0069A000`) |
| City component | #21 "building development simulator", city slot 53 (`+0xD4`, `cSC4City::GetBuildingDevelopmentSimulator` `0x006197E0`), global `SL::spBuildingDevelopmentSim` (`0x00B43D68`) (foundations §3.6) |
| Interfaces (`QueryInterface` `0x00695290`) | `cIGZUnknown` `0x00000001` and `cISC4BuildingDevelopmentSimulator` `0x07813EAE` → primary (vtable `0x00AB0AF8`, 59 interface slots + scalar deleting destructor); `cIGZMessageTarget` `0xE98B2F57` → sub-object at +4 (vtable `0x00AB0AE8`, 4 slots; its `DoMessage` returns true and does nothing); `cIGZMessageTarget2` `0x452294AA` → sub-object at +8 (vtable `0x00AB0AD8`, 4 slots); `cIGZSerializable` `0xE4FDA3D4` → sub-object at +0xC (vtable `0x00AB0AC0`, 6 slots) |
| Unknown IID | `QueryInterface` returns false and leaves `*ppv` **unchanged** *[compat]* |
| `Release` (`0x006959B0`) | count > 1: decrement and return the new count. Count 1: call `Shutdown` (slot 4), then the scalar deleting destructor (slot 59) with 1 (frees with C), return 0 |

### 1.2 Init order

The city calls `Init` in the order of foundations §3.7: … budget, ordinance, power, plumbing, neighbours,
**building development**, tract developer, lot developer, demand … . The BDS `Init` therefore runs before the tract
and lot developers' and before the demand simulator's. It reads the simulator, land value, power, zone, lot,
occupant, terrain, lot developer and tract developer globals (`SL::OnCityInit` sets them all before any `Init`).

### 1.3 `Init()` → true (slot 3, `0x0069D240`) *[compat]*

Runs once (an `initialized` flag; a second call returns true at once).

1. **Static catalogue** (process-wide, guarded by a static flag, so it runs at the first `Init` of the process
   only): register a static-data cleanup callback (`0x0069B850`) with the application object (`0x00B43C94`, slot 69
   `+0x114`); enumerate every resource key of type `0x6534284A` (exemplars) through the resource manager (slot 18
   `+0x48`, then the key list's enumerate slot 7 `+0x1C`) and, for each exemplar, by its `ExemplarType` (property
   `0x10`): 2 → building (`AddBuildingData`), `0x11` → foundation (`0x00698F50`), `0x15` → lot retaining wall
   (`0x00698FC0`); other types are ignored. Then post `kSC4MessageBuildingTypesUpdated` (§5) and build the maximum
   density table (§4.7.4).
2. **Developers**: enumerate the type-`0x6534284A` keys again and call `CreateDeveloper` for every exemplar whose
   `ExemplarType` is **4** (§1.5).
3. **Tuning**: `ReadSimulatorTunableValues` (§4.13).
4. If **not loaded from a save**: for every developer in the developer map, `highestStage[devType] := 0`.
5. Acquire (AddRef) the resource manager, city, occupant manager (city slot 34), zone manager, lot manager, land
   value simulator, power simulator and terrain; cache the terrain's cell counts (slots 7 and 8).
6. If there is no building map yet (new city): create a `cISC4SimGrid<uint8>` (class `0x49B9E602`), `Init()` it
   (city size), tract size 1 (slot 9), instance ID `0x49D5B678` (slot 6); all cells 0. A loaded city brings its
   own grid (§6). *[choice]* FreeCity4 creates it through GZCOM (simgrid §6.4).
7. If there is no construction demand node yet (new city): create a `cSC4Demand` (0x54 bytes, C heap), AddRef,
   `Init()`, `SetSupplyValue(100.0)`, `SetDemandValue(0.0)`. A loaded node is only `Init()`ed (it keeps its saved
   values; demand spec §3.2).
8. Subscribe on the message server (slot 5 `+0x14`) to `0xE8BDA2E7` (StateChanged), `0x278E6003` (terrain
   changed), `0x99EF1142` (occupant inserted), `0x99EF1143` (occupant removed) and `0x0B99AE81` (PlaceBuilding
   command); register the command `0x0B99AE81` "PlaceBuilding" with the command server (slot 6 `+0x18`).
9. Register the agent at levels **0, 4, 6 and 10**, each with name `"cSC4BuildingDevelopmentSimulator"` and flags
   2 (enabled only when the city is established) (simulator slot 19 `+0x4C`).
10. Register and subscribe three cheats with the cheat code manager (slots 3 and 7): `0x491865E1` "HowDryIAm",
    `0x08DAD89A` "FightThePower", `0x6AB6A2D7` "Barstuck".
11. Return true.

### 1.4 `Shutdown()` → true (slot 4, `0x0069A210`)

*[compat]* If `initialized` is false (`Shutdown` before any `Init`, or a second `Shutdown`), it returns true at once
and does **nothing**: no service is released, no container is cleared, and the process-wide tried-tract marks are
kept (the flag test is the first instruction, `0x0069A213`). Otherwise, once (clears `initialized`): unregister the
three cheats (cheat manager slots 8 and 4); remove the agent from the
simulator (city slot 50, simulator slot 20 `+0x50`); unsubscribe the five messages; unregister the command (command
server slot 9); release the message server; for every developer: `Shutdown()` (slot 4) and `Release`; clear the
developer map; release every special building and clear the list; clear both plan lists, the planned-growth map,
the two unused maps and the highest-stage map; release the building map; `Shutdown` and release the construction
node; release the cached services; clear the process-wide **tried-tract marks** (§4.4). The static catalogue is
**not** cleared (it lives until process exit).

### 1.5 Developer map *[compat]*

`CreateDeveloper(holder)` (`0x006991A0`), for each developer exemplar (ExemplarType 4):

- `devType := ` property `0x21` (uint32, default 0). If 0: ignored.
- If the developer map has no entry for `devType`: create a `cSC4GrowthDeveloper` (`0x13C` bytes, C heap, constructor
  `0x006B7310`), AddRef, `Init(holder)` (slot 3), store it. If an entry exists (it was read from the save):
  call its `Init(holder)` again.
- `lastSuccessfulBuild[devType] := today` (simulator `GetSimDate()` → `cIGZDate` slot 26 day number), even when the
  developer came from a save (the day is not saved, §6).

The map is ordered by developer type (ascending). Shipped developer exemplars (group `0x67CD5FA1`, instance =
developer type, in `SimCity_1.dat`):

| Developer type | Exemplar name | Purpose, wealth |
|---|---|---|
| `0x1010`, `0x1020`, `0x1030` | Residential $ / $$ / $$$ | 1, 1–3 |
| `0x3110`, `0x3120`, `0x3130` | Commercial Service $ / $$ / $$$ | 2, 1–3 |
| `0x3320`, `0x3330` | Commercial Office $$ / $$$ | 3, 2–3 |
| `0x4100` | Industrial Resource | 5 (agriculture), 1 |
| `0x4200` | Industrial Dirty | 6, 2 |
| `0x4300` | Industrial Manufacturing | 7, 2 |
| `0x4400` | Industrial High Tech | 8, 3 |

Eight further type-4 exemplars (airports, seaports, landfill developer, parks manager) have no property `0x21` and
create nothing. Exemplars are visited in the resource key list's enumeration order; this changes only the order of
the developers' `Init` calls, never the map order.

---

## 2. State

| State | Initial | Saved | Notes |
|---|---|---|---|
| `loadedFromSave` | false | no | set by `Read`; Init then keeps the highest-stage map |
| `initialized` | false | no | |
| developer map: developer type → `cISC4GrowthDeveloper*` (holds one reference each) | empty | yes (refs) | §1.5. `GetDevelopers` exposes it raw (§9) |
| `highestStage`: developer type → uint8 | 0 for every developer (new city) | yes | §4.10 |
| special buildings: list of `cISC4Occupant*` (one reference each) | empty | yes (refs) | landmarks and rewards, §4.12 |
| building map: `cISC4SimGrid<uint8>` | created in Init (new city) | yes (ref) | building keys, §4.10 |
| construction node: `cISC4Demand` (`cSC4Demand`) | created in Init (new city) | yes (ref) | §4.6 |
| `state` (planning state machine, 0–4) | 0 | no | §4.2 |
| developer cursor (for state 2) | — | no | |
| decline plan: list of {day, x, z, developer, stage} | empty | no | §4.3 |
| growth plan list, planned-growth map {profile → day}, retry days | empty / empty / 0 | map **yes**, others no | used only by the dead growth-plan path (§10.1 Q1); always empty in the shipped game *[inferred]* |
| two further maps | empty | no | never filled *[inferred]* |
| `lastSuccessfulBuild`: developer type → day | today, per developer at Init | **no** | §4.4 |
| tuning: `crMin`, `crMax` (uint32), `crPerResident` (f32), `bias` (int32), `stageMultiplier` (vector of f32) | 100, 2000, 0.5, 10, empty | no | §4.13 |
| `useGrowthPlan` flag | false, **never set** | no | selects the dead path |
| cheats: ignore plumbing, ignore power, ignore caps | false | **yes** | §4.12 |
| cheats: debug growth, coffee bar | false | no | debug growth is never set by the BDS |
| **process-wide**: tried-tract marks: developer type → set of (x, z) | empty | **yes** (in this object's record) | §4.4; cleared at Shutdown |
| **process-wide**: the building catalogue (§4.7) | built at the first Init | no | |
| **process-wide**: the BDS random generator | clock-seeded at process start | no | §4.14 |
| **process-wide**: Grow's timer step and miss counter | 0x400, 0 | no | §4.4 |

---

## 3. Scheduling and messages handled

### 3.1 Agents *[compat]*

| Level | Message | Action |
|---|---|---|
| 0 OneShot (`0x66956810`, SimBegin) | | `DoMonthlySimulation` (§4.1) — so the first map rebuild starts when the simulation starts |
| 4 Day (`0x66956814`) | | `DoIdle(5)` (§4.2), then `DoDailySimulation` (§4.4, §4.5) |
| 6 Month (`0x66956816`) | | `DoMonthlySimulation` (§4.1) |
| 10 Idle (`0x6695681A`) | | `DoIdle(data3)` (the argument is ignored) |

`DoMessage` (`0x0069CCE0`, this = the `cIGZMessageTarget2` sub-object) **always returns true**: every interval
finishes in one call (simulator §4). Registration flags 2: the agent is disabled until the city is established.

Within the day level the BDS runs in agent registration order, i.e. city Init order (foundations §3.7, §13): before
the tract and lot developers' and the demand simulator's day agents. The demand values Grow reads are therefore the
ones left by the previous day's (and month's) demand passes; the growth lists are those of the developers' last
finished map rebuild.

### 3.2 Messages handled *[compat]*

| Message | Handling |
|---|---|
| `0x230E27AC` cheat (data1 = cheat ID) | `0x491865E1` → ignore plumbing := true; `0x08DAD89A` → ignore power := true; `0x6AB6A2D7` → coffee bar := not coffee bar. Others: nothing |
| `0x0B99AE81` PlaceBuilding command | §4.12 |
| `0x278E6003` terrain changed | data1 = `(x1 + 1) << 16 \| x2`, data2 = `(z1 + 1) << 16 \| z2` *[inferred: packing as read]*: the BDS takes `x1 := (data1 >> 16) − 1` clamped ≥ 0, `x2 := data1 & 0xFFFF` clamped ≤ cellCountX − 1 (same for z with data2), and calls `DoTerrainChanged(rect, data4)` (§4.11) |
| `0x99EF1142` occupant inserted / `0x99EF1143` removed | occupant = void1. Ignored if its property holder has property `0x029244DB` (a preview occupant). If the occupant is in occupant group `0x150A` or `0x150B` (`IsOccupantGroup`, slot 22): inserted → append to the special-building list and AddRef; removed → remove the first matching entry and Release it |
| `0xE8BDA2E7` StateChanged | lot = `GetIGZUnknown()` (a copy of the lot with its scalar fields, lot-developer.md §3.2); nothing if null or `lot.GetBuildingType(false)` (slot 46) is 0. `old := data1 ∈ {2, 3}`, `new := data2 ∈ {2, 3}` (Constructing or Occupied, demand spec §7). If they differ: `DoBuildingChanged(lot, wealth = data4, built = new)` (§4.10) |

---

## 4. Algorithms *[compat]*

### 4.1 `DoMonthlySimulation` (`0x00696130`)

Only when `state = 0` (otherwise nothing; a month that arrives while the previous rebuild is still running is
skipped):

```
R := demandSim.GetDemand(0x1001, 0).QuerySupplyValue()     -- "R Census" (demand spec §2)
C := demandSim.GetDemand(0x3001, 0).QuerySupplyValue()
I := demandSim.GetDemand(0x4001, 0).QuerySupplyValue()
x := r24( r24( r24(I + C) + R ) × crPerResident )
u := low 32 bits of trunc(x), as unsigned
u := max(u, crMin)  then  min(u, crMax)          -- unsigned compares
constructionNode.SetSupplyValue(float(int32 u))
for each developer, ascending type: developer.StartMapUpdate()   (slot 27)
state := 1
```

- The sum order is I + C first, then + R (each rounded). `crPerResident` is a float32 operand.
- *[quirk]* The clamps are unsigned: a negative x (impossible with census values) becomes a huge value and is clamped
  to `crMax`; NaN gives 0 → `crMin`.
- *[choice]* A missing census node crashes the original (no null test); FreeCity4 uses 0 for its term.
- With the shipped tuning (8192, 20480, 0.5) the supply is `clamp(trunc(0.5 · (R + C + I)), 8192, 20480)`.

### 4.2 `DoIdle` state machine (`0x00699EA0`)

| State | Action |
|---|---|
| 0 | nothing |
| 1 | cursor := first developer; state := 2; continue as state 2 in the same call |
| 2 | from the cursor, for each developer: if `IsMapUpdateFinished()` (slot 28) is false, **return** (the same developer is asked again at the next call); else advance. When all are finished: state := 3, then (only if `useGrowthPlan`, never) `PlanGrowth`; state := 4; return |
| 4 | `PlanDecline` (§4.3); state := 0 |

`DoIdle` runs at the start of every day (before the daily growth) and at every idle slice, so the map rebuild
progresses even when the game is paused. `IsMapUpdateFinished` (slot 28, `0x006B75D0`) is a **pure query**: it
returns `state = 0` and does no work (corrected). The growth developers do their incremental map work in their own
`DoIdle`, driven by their own idle and day agents (growth-developer.md §3.1–§3.2); this state machine only waits
for all of them. The decline plan is made on the `DoIdle` call **after** the one that saw every developer
finished.

### 4.3 `PlanDecline` (`0x00699890`)

```
clear the decline plan
total := 0
for each developer D, ascending type:
    if D.GetPurpose() ≠ 5 (agriculture) and ad(D) < −20:
        total += bias − ad(D);  cursor[D] := start of D.GetDeclineList()
                                   -- ad(D) := trunc(D.GetSourceDemand().QueryActiveDemandValue()), int32
now := today's day number;  k := 0
loop:
    r := RNG.RandomUint32Uniform(total)                         -- one draw (none if total = 0)
    walk the qualifying developers, ascending type:
        w := bias − ad(D)          (re-queried; same value)
        if r < w (signed): pick D, stop;  else r −= w
    if a developer D was picked and cursor[D] is not at its list's end:
        append {day = k, x, z of the item, developer D, stage 0} to the plan;  k += 1;  cursor[D] += 1
    if no qualifying developer has items left: stop
for each plan entry in order: day := trunc(r24(float(now) + float(day)))
```

- Qualification needs `ad < −20`; the weight is `bias − ad` (so ≥ `bias + 21`). `GrowthAndDeclineListsAreValid` is
  **not** checked here.
- The plan interleaves the developers' decline lists in weighted random order, each list in its own order. A pick
  of a developer whose list is used up is **wasted** but still consumed a draw *[compat]*: the number of draws is
  the number of loop passes, not the number of entries.
- Entry *k* is scheduled for day `now + k` (one entry per day). The addition is done in float32 (`fild` of both
  unsigned values, `fadd`, `_ftol`); exact while `now + k < 2^24`.
- The decline item's stage byte is not used: the entry's stage is always 0.
- If `total = 0` there is no draw, nothing is picked and the loop ends at once.

### 4.4 Daily growth: `DoDailySimulation` → `Grow` (`0x0069B6D0`, `0x0069B0B0`)

`DoDailySimulation` runs `Grow` (or the dead `ExecuteGrowthPlan` when `useGrowthPlan`), then `ExecuteDeclinePlan`
(§4.5).

**`Grow`**:

```
start a "3 ms" CPU-cycle limit timer (simulator §5.5; 0x0068BF50 / 0x0068BF80)
pending := copy of the developer map (type → developer)
counter := 0
while pending is not empty:
    counter += step;  if counter mod 1024 = 0 and the timer has expired: stop
    total := Σ over D in pending (ascending type) with ad(D) > 5 of (ad(D) + bias)
    r := RNG.RandomUint32Uniform(total)                        -- one draw (none if total = 0)
    walk the same developers: if r < ad(D) + bias (signed): pick D, stop;  else r −= ad(D) + bias
    if nothing was picked: continue
    t := D.GetSubType()                                         -- slot 6, the developer type
    if D.GrowthAndDeclineListsAreValid():                        -- slot 10
        L := D.GetGrowthList()                                   -- slot 11, items {x, z, stage}
        item := the first item of L whose (x, z) is not in marks[t]
        if none: clear marks[t]; item := first item of L (if L is empty: no attempt)
        if there is an item:
            insert (item.x, item.z) into marks[t]
            built := tractDeveloper.Grow({D, item.x, item.z, item.stage}, outcomes, flag)   -- §8, slot 5
            if built > 0 (signed): lastSuccessfulBuild[t] := today
    remove t from pending
```

Consequences *[compat]*:

- Every developer with `ad > 5` makes **exactly one attempt per day** (unless the timer stops the loop first); the
  weighted draw only decides the **order** of the attempts. Order matters because the attempts compete for
  construction resources (§4.6) and land.
- Draws per day: one per qualifying developer, in pick order. `RandomUint32Uniform(0)` (no qualifying developer
  left) does not advance the generator.
- `marks[t]` makes the developer walk its growth list round-robin across days: each day it tries the next tract it
  has not tried since the last reset, whether or not the attempt built anything. Marks are keyed by (x, z) only.
  They are process-wide, saved in this object's record (§6), and cleared at `Shutdown`.
- *[compat]* The marks lookup **creates** the developer's entry: `marks[t]` is an insert-if-absent lookup
  (`0x0069AB40`), done by the "not marked" test for each item examined (predicate `0x0069B070`) and again when no
  unmarked item is found (before the clear). So every pick of a developer whose lists are valid creates
  `marks[t]` if it is absent, even when its growth list is **empty**; in that case the entry stays empty, because no
  item is inserted. With a non-empty list an item is always inserted at once, so an empty entry exists only for a
  developer whose last attempt saw an empty growth list (or that came from a save with 0 cells, §6). Developers never
  picked, or picked with invalid lists, get no entry. Every entry, empty or not, is saved (§6).
- *[quirk]* Developers with `ad ≤ 5` stay in `pending` forever, so once the qualifying developers are done the loop
  **spins** (re-summing demand, drawing nothing) until the 3 ms timer expires. Every day therefore costs about
  3 ms whenever any developer lacks demand.
- *[quirk]* The timer is checked only when `counter mod 1024 = 0`, with `counter` advancing by a process-wide
  `step` (initially 1024). After the loop, with `n := counter >> 10`: if `n > 20`, `miss += 1`, and when it reaches
  16, `miss := 0` and `step := step / 2` (if > 1); if `1 ≤ n < 5`, `miss −= 1`, and when it reaches −4, `miss := 0`
  and `step := step · 2` (if < 1024). This only tunes how often the clock is read.
- If the attempts take longer than 3 ms in total, the remaining qualifying developers get **no** attempt that day.
  This depends on the machine's speed and cannot be reproduced.
- *[choice]* FreeCity4 replaces the 3 ms timer with a deterministic **daily growth budget** (§4.4.1). There is
  no spin, no timer and no step adaptation.
- The outcome list passed to `Grow` is created empty by the BDS, filled by the tract developer and freed by the
  BDS (allocator B, §9.2); its content is not used. `flag` starts false and is not read afterwards.

#### 4.4.1 Daily growth budget *[choice]*

- **Unit of work**: one call of `tractDeveloper.Grow` (the call in the loop above; original call site
  `0x0069B394`). Picks that end without a call (lists invalid, growth list empty) are not units.
- **Budget** `growthAttemptsPerDay` (tuning value, unsigned; `0` means unlimited). `Grow` keeps a count of the
  units done today. Before each loop iteration (before the sum and the draw), if `count = growthAttemptsPerDay` the
  loop stops and `Grow` returns. Otherwise the loop runs as above until every qualifying developer has been picked
  once, and then returns (developers with `ad ≤ 5` are skipped, not spun on).
- Draw sequence: identical to the original's for the iterations that run. A budget stop leaves the generator
  untouched for the rest of the day (no draw for an iteration that does not run).
- **Default**: to be calibrated in the original (§12), not guessed. Until then the default is unlimited.
- **Observable effect of the budget** (for the M4 exit tests):
  - At or above the number of qualifying developers (at most 13 with the shipped exemplars) the budget has **no
    effect**: every qualifying developer attempts once per day, as in the original on a fast enough machine.
  - Below it, fewer attempts happen per day, so **growth is slower** (fewer buildings per game month), and the
    attempts are biased towards high-demand developers, because they are picked first. Low-demand developer types
    are starved first, which changes the R/C/I mix of new buildings.
  - Budget-sensitive statistics: buildings constructed per month by developer type, the time for a zoned area to
    fill, and how the population curve depends on demand. Budget-insensitive (given the same attempts):
    what is built where, and the draw-for-draw choices of each attempt.
- *[compat]* `GetSubType()` must return the developer's map key (property `0x21`); otherwise the developer is never
  removed from `pending`.

### 4.5 `ExecuteDeclinePlan` (`0x00696A50`)

Every day, after `Grow`:

```
for each plan entry E in plan order:
    if E.day > today: keep it, continue
    if ad(E.developer) > −20: drop E, continue
    result := tractDeveloper.Decline({E.developer, E.x, E.z, E.stage = 0})        -- slot 6
    drop E
    if result > 0 (signed): stop for today
```

At most one successful decline per day; failed and no-longer-needed entries are dropped. A new plan (§4.3) replaces
the old one once per month, so overdue entries never accumulate beyond a month. (The planner requires `ad < −20`,
the executor `ad ≤ −20`.)

### 4.6 Construction resources *[compat]*

The construction node is a plain `cSC4Demand` (demand spec §3) owned by the BDS: **supply** = the month's budget
(§4.1), **demand** = the capacity currently claimed. Its key is never set (0) *[inferred]*; every `EndOfCycle`
below posts ActiveDemandChanged (`0x426840A0`, data1 = 0) like any node (demand spec §3.4).

| Method | Behaviour |
|---|---|
| `GetTotalConstructionResources()` (49) | `trunc(supply)`, low 32 bits in EAX |
| `GetFreeConstructionResources()` (48) | `v := trunc(r24(supply − demand))`, low 32 bits; negative → 0; EAX |
| `ClaimConstructionResources(n)` (50) | `t := r24(demand + float(n))`. If `supply < t` or unordered (NaN): return false, nothing changes. Else `AddToDemandValue(float(n))`, `EndOfCycle()`, return true |
| `ReleaseConstructionResources(n)` (51) | `AddToDemandValue(float(−n))` (n zero-extended from uint16), `EndOfCycle()`, return `EndOfCycle`'s bool. No lower bound: demand can go negative |

- Units are building capacity units (residents or jobs). `cSC4Lot::SetState` (`0x006C3100`) releases
  `GetBuildingProfile(type).capacity` when a lot leaves state 2 (Constructing) and the building's purpose is not 0;
  the tract developer claims `GetBuildingProfile(building).capacity` in `Build` before construction starts, and a
  farm claims `n + 30`, releases 30 and keeps `n` for its fields (`tract-developer.md` §4.15).
- The supply is reset monthly, the demand only by claims and releases; both are saved with the node.

### 4.7 Building catalogue (process-wide) *[compat]*

#### 4.7.1 Buildings

For each building exemplar (ExemplarType 2) the **building type** is the exemplar's instance ID.
`AddBuildingData` stores type → exemplar key (a later exemplar with the same instance replaces the key) and calls
`LoadBuildingProperties(holder, type)` (`0x0069B910`), which fills:

| Field | Source |
|---|---|
| exemplar name (`GetExemplarName`) | property `0x20` (exemplar name), else "" |
| building name (`GetBuildingName`) | localized string `0x8A416A99` (`SC4LoadExemplarString`); if absent, property `0x899AFBAD`; else "" |
| profile.wealth | property `0x27812832` (uint8); outside 0–3 → 0 (with a debug message) |
| profile.purpose | property `0x27812833` (uint8), default 0 |
| profile capacities satisfied | property `0x27812834`: pairs (developer type, amount) → a map developer type → uint16 |
| profile.capacity | the amount of that map for `GetDeveloperType(purpose, wealth)` (0 if absent) |
| profile.categoryFlags | property `0xAA1DD396` (occupant groups, uint32 list): §4.7.1.1 |
| further profile fields | `0x27812836` (bool), `0x499AFA38` (uint8, default 30), occupant size `0x27812810` (3 floats), plop cost `0x49CAC341` (int64), budget item cost (budget simulator); not used by growth *[inferred: field mapping to the SDK's BuildingProfile]* |
| families | property `0x27812870` (uint32 list): the type is appended to each family's member list |

The profile that `GetBuildingProfile` inserts for an unknown type (§4.7.2) is all zero, including the
`0x499AFA38` byte (the default 30 applies only in `LoadBuildingProperties`).

##### 4.7.1.1 Building category flags *[compat]*

`SetBuildingCategoryFlags(holder, profile)` (`0x00696B80`) sets `categoryFlags := 0`, reads property `0xAA1DD396`
as a uint32 list (`0x005FF510`; missing property → flags stay 0) and, for every value v of the list and every
index i in 0..30, sets bit i when `table[i] = v`. The table (31 uint32 entries at `0x00AB0A40`, data) maps bit
index to occupant group ID; values not in the table set nothing, a repeated value sets the same bit, and bit 31 is
never set. Names are the SDK's `BuildingCategoryFlag…` names (community names; the game's own scripts confirm
`0x150A` landmark, `0x1301` bus and `0x1508` airport).

| Bit | Mask | Occupant group | SDK name |
|---|---|---|---|
| 0 | `0x00000001` | `0x1000` | Residential |
| 1 | `0x00000002` | `0x1001` | Commercial |
| 2 | `0x00000004` | `0x1002` | Industrial |
| 3 | `0x00000008` | `0x1003` | Transportation |
| 4 | `0x00000010` | `0x1004` | Utility |
| 5 | `0x00000020` | `0x1005` | Civic |
| 6 | `0x00000040` | `0x1006` | Park |
| 7 | `0x00000080` | `0x1300` | Rail |
| 8 | `0x00000100` | `0x1301` | Bus |
| 9 | `0x00000200` | `0x1302` | Subway |
| 10 | `0x00000400` | `0x1400` | Power |
| 11 | `0x00000800` | `0x1401` | Water |
| 12 | `0x00001000` | `0x1402` | Landfill |
| 13 | `0x00002000` | `0x1500` | Police |
| 14 | `0x00004000` | `0x1501` | Jail |
| 15 | `0x00008000` | `0x1502` | Fire |
| 16 | `0x00010000` | `0x1503` | School |
| 17 | `0x00020000` | `0x1504` | College |
| 18 | `0x00040000` | `0x1505` | Library |
| 19 | `0x00080000` | `0x1506` | Museum |
| 20 | `0x00100000` | `0x1507` | Health |
| 21 | `0x00200000` | `0x1508` | Airport |
| 22 | `0x00400000` | `0x1509` | Seaport |
| 23 | `0x00800000` | `0x150A` | Landmark |
| 24 | `0x01000000` | `0x150B` | Reward |
| 25 | `0x02000000` | `0x2000` | StyleChicago |
| 26 | `0x04000000` | `0x2001` | StyleNewYork |
| 27 | `0x08000000` | `0x2002` | StyleHouston |
| 28 | `0x10000000` | `0x3000` | IndustryAgriculture |
| 29 | `0x20000000` | `0x3001` | IndustryDirty |
| 30 | `0x40000000` | `0x3002` | IndustryManufacturing |

The BDS itself never reads the flags; they are part of the `BuildingProfile` returned by slots 39/40.

##### 4.7.1.2 Catalogue maps and their order *[compat]*

The catalogue is five process-wide STLport `hash_map`s keyed by uint32 with the identity hash (bucket = key mod n,
unsigned): **key map** (building type → exemplar key, `0x00B48548`), **profile map** (`0x00B48584`), the two name
maps, and the **family map** (family → member vector, `0x00B48598`). Each is constructed at process start with a
bucket hint of 100, so n = 193. They share the STLport hash table of allocators.md (*hash_set*): the 28-entry
prime table at `0x00A829B0`, chains singly linked from the bucket.

- **Insert** (`operator[]`, e.g. `0x004A3470` → `0x004A2930` for the key map, `0x0069B720` → `0x0069B650` for
  the profile map; the family map does the same inline): the key is **looked up first**; if present, nothing
  changes (no growth). If absent: `resize(count + 1)` (`0x004F0940` / `0x00696540`), then the new node goes to the
  **head** of chain `key mod n` and count += 1. (This differs from the `hash_set::insert` callers of allocators.md,
  which resize before the lookup.)
- **Resize(h)**: nothing when h ≤ n. Else n' := the first prime of the table ≥ h (4294967291 beyond it); nothing
  if n' ≤ n. Else a new zeroed bucket vector of n' entries; the old buckets are walked in ascending order and each
  chain from its head: every node is unlinked and pushed onto the **head** of chain `key mod n'`; the old vector is
  freed. With the shipped counts the growth sequence is 193 → 389 (on the 194th distinct key) → 769 (on the
  390th) → 1543 …
- **Enumeration**: buckets 0 to n − 1, each chain from its head. Consequences: keys in the same bucket enumerate
  newest first; a rehash reverses the relative order of keys that share both their old and their new bucket.
- The key map receives one insertion per building exemplar, in the resource manager's key enumeration order
  (`AddBuildingData` inserts only when the exemplar resource loads); a second exemplar with the same instance
  only replaces the stored key. So `GetAllBuildingTypes` order is a function of that exemplar order alone.
- **Which map answers what** (§4.7.2): the set of building types is the **key map**. `GetBuildingProfile` and
  `GetBuildingMapKey(type)` insert into the profile map only, so an unknown type queried there does **not** become
  valid, enumerated or counted.

#### 4.7.2 Queries the developers use

"t is a building type" means: t is a key of the **key map** (§4.7.1.2). This holds for `IsBuildingTypeValid`
(`0x00697E30`), the type fallback of `IsBuildingFamilyTypeValid` (`0x00697E70`) and of both
`GetBuildingTypesByFamily` overloads (`0x00697EF0`, `0x00699D20`), `GetBuildingKeyFromType` (`0x00697DD0`), both
`GetAllBuildingTypes` overloads (`0x00697C50`, `0x00697B80`; typeCount = the key map's count),
`GetBuildingTypesByFilter` (`0x00699C50`), the count posted in `kSC4MessageBuildingTypesUpdated` (§5) and the
expansion of §4.7.4 *[compat]*. "Hash order" below is the key map's enumeration order. Profiles inserted by
`GetBuildingProfile(unknown)` change none of these.

| Query | Behaviour |
|---|---|
| `IsBuildingTypeValid(t)` (15) | t is a building type |
| `IsBuildingFamilyTypeValid(f)` (16) | f is a family **or** a building type |
| `IsBuildingInFamily(b, f)` (20) | `b = f`, or b is a member of family f |
| `GetBuildingTypesByFamily(vector&, f)` (18) | f is a family: the vector **becomes** a copy of the member list (assignment, B); returns the member count. Else f is a building type: the vector is cleared and holds f; **returns 0** *[quirk]*. Else: unchanged, returns 0 |
| `GetBuildingTypesByFamily(hash_set&, f)` (19) | same, but **inserts** into the set (no clear); returns the member count, 0 for a single type |
| `GetBuildingTypesByFilter(hash_set&, filter)` (17) | for each building type in hash order: if `filter` is null or `filter.slot3(type)` is true, insert; returns the number of types that passed |
| `GetBuildingProfile(t)` (39) | reference to the stored profile; **inserts a zero profile** for an unknown type *[compat]* (the catalogue grows) |
| `GetBuildingProfilePtr(t)` (40) | pointer, null for an unknown type |
| `GetBuildingKeyFromType(t, key&)` (14) | true and the key, or false and key unchanged |
| `GetExemplarName(t)` / `GetBuildingName(t)` (37/38) | pointer to the stored `cRZString` (inserting "" for an unknown type); the caller must not release or free it |
| `GetAllBuildingTypes(vector&)` (11), `GetAllBuildingFamilyTypes` (13), `GetAllFoundationTypes` (21), `GetAllLotRetainingWallTypes` (27) | clear the vector (end = begin, keeps the buffer), then append every key in hash order (B) |
| `GetAllBuildingTypes(uint32* array, uint32& n)` (12) | `n := min(n, typeCount)`; fills the first n types in hash order (nothing if array is null); returns typeCount |

#### 4.7.3 Developer types, keys and zone compatibility

`GetDeveloperType(p, w)` (`0x00695790`, slot 30): p = 1: w 1/2/3 → `0x1010/0x1020/0x1030`; p = 2: → `0x3110/0x3120/0x3130`;
p = 3: w 2 → `0x3320`, w 3 → `0x3330`; p = 5 → `0x4100`, 6 → `0x4200`, 7 → `0x4300`, 8 → `0x4400` (any wealth);
everything else (p = 4, p = 3 with w 1, wealth 0 for p 1–3) → 0.

`GetWealth(devType)` (`0x00695840`, slot 31): `0x1010/0x3110/0x4100` → 1; `0x1020/0x3120/0x3320/0x4200/0x4300` → 2;
`0x1030/0x3130/0x3330/0x4400` → 3; else 0.

`GetBuildingMapKey(p, w)` (42) = `uint8(4p + w + 1)`; `GetBuildingMapKey(t)` (43) the same from t's profile (inserting a
zero profile, key 1, for an unknown type). Key 0 means "no building".

`GrowthStageFitsInZone(stage, zone)` (`0x00695340`, static): zones 1, 4 (low density) → stage ≤ 3; zones 2, 5
(medium) → stage ≤ 6; zones 3, 6, 7, 8, 9 → true; other zones → false.

`BuildingFitsInZone(bz, cz)` (`0x00695390`, static; bz = the building's zone type, cz = the cell's): `bz = cz` → true;
cz 2 → bz 1; cz 3 → bz 1 or 2; cz 5 → bz 4; cz 6 → bz 4 or 5; cz 9 → bz 8; cz `0x0B`, `0x0C` or `0x0F` → bz 0;
otherwise false. (A denser zone accepts lower-density buildings of the same family; zone IDs per `zones.md`.)

#### 4.7.4 Maximum density table (`CalculateMaxCapacity` `0x0069C370`, `GetMaxCapacity` slot 45)

Built once at the static initialisation, from every lot configuration (lot configuration manager `0x00B43D18`, slot
20 `GetLotConfigurationMap`):

```
(w, h) := config.GetSize();  area := w · h (int)
sizeX := r24(float(w) × CellWidthX);  sizeZ := r24(float(h) × CellWidthZ)     -- city slots 91/92; w, h unsigned
obj := config.GetPrimaryBuildingConfigurationObject()        -- slot 24
for each ID in obj's ID list:
  S := {} (a fresh 193-bucket hash_set<uint32>)
  if ID is a family: insert every member into S
  else if ID is a building type (key map): insert ID
  else: nothing (S stays empty; no catalogue map gains an entry)
  for each building type T in S (set hash order):
    if T's exemplar loads (resource manager slot 3):
        (dx, dz) := occupant size 0x27812810 [0] and [2], swapped when obj's orientation is 1 or 3
        left := r24(objX − r24(dx × 0.5)), right := r24(r24(dx × 0.5) + objX), same for z with objZ
        skip T unless r24(left + 0.001) ≥ 0, r24(right − 0.001) ≤ sizeX, r24(top + 0.001) ≥ 0,
                      r24(bottom − 0.001) ≤ sizeZ                 -- 0.5, 0.001 are float32 constants
    prof := GetBuildingProfile(T);  skip if prof.purpose = 0
    d := r24(r24(float(prof.capacity) / float(area)) + 0.5)
    if d < 1.0 and prof.capacity > 0: d := 1.0
    for zone z in 0..15 with config.IsCompatibleWithZoneType(z):        -- slot 15
        key := (z, uint8(4·purpose + wealth + 1));  table[key] := max(table[key], d)   -- inserted if absent
```

*[compat]* An ID that is neither a family nor a key-map building type contributes nothing and inserts nothing
anywhere (`0x0069C4CA`–`0x0069C59C`: family-map lookup, then key-map lookup, both without insertion). The family
test comes first, so an ID that is both a family and a building type is expanded as the family only. For each T in
S the code then looks T up in the **key map with insert-if-absent** (a missing T would gain a zero key and become a
"valid" building type) and fetches the exemplar by that key; with exemplar-built families every member is already a
key-map type (§4.7.1), so the insert cannot happen with game data *[inferred: unreachable]*; a replacement need not
reproduce it. When the exemplar does not load, the fit test is skipped and T is still rated. `GetBuildingProfile(T)`
inserts a zero profile for a T without one (purpose 0: skipped). The order of S does not matter (the table keeps
maxima).

`GetMaxCapacity(zone, p, w)` returns `float(trunc(table[(zone, GetBuildingMapKey(p, w))]))` in **ST0**, or 0.0 if the
key is absent: the largest "capacity per cell, rounded half up, at least 1" of any building of that purpose and
wealth that fits a lot configuration allowed in that zone. *[compat]* SDK error, §7.1.

### 4.8 `IsSuitableForConstruction(profile, reason&)` (slot 46, `0x00695430`)

`profile` is a `ConstructionProfile`, a 0x24-byte struct passed by reference (ABI layout, as both slot 46 and slot
47 read it; SDK names in brackets):

| Offset | Type | Field |
|---|---|---|
| +0x00 | int32 | building zone type (`unknown1`) |
| +0x04 | uint32 | unused by the BDS (`unknown2`) |
| +0x08 | int32 | wealth |
| +0x0C | int32 | purpose (not read by the BDS) |
| +0x10 | uint8 | land value min |
| +0x11 | uint8 | land value max (+0x12, +0x13 padding) |
| +0x14 | uint32 | building type |
| +0x18 | int32 | **orientation** (`unknown3`, "facing?"): read only by `PlopBuilding` (§4.9) |
| +0x1C | int32 | cell x |
| +0x20 | int32 | cell z |

Checks, in order; the first failure sets `reason` and returns false:

| # | Check | reason |
|---|---|---|
| 1 | `BuildingFitsInZone(profile.zone, zoneManager.GetZoneType(x, z))` (zone type sign-extended from int8) | 0 |
| 2 | `lotManager.GetLot(x, z, true)` is not null | 3 |
| 3 | ignore-power cheat set, or `powerSim` slot 19 (powered at x, z) | 5 |
| 4 | if lvMin = lvMax = 0: unless profile.zone ∈ {7, 8, 9}, `landValueSim.GetLandValueType(x, z) = profile.wealth`. Else: `lvMin ≤ landValueSim.GetLandValue(x, z) ≤ lvMax` | 2 / 1 |
| 5 | `lot.GetState() = 1` | 4 |

All pass → true (`reason` untouched).

### 4.9 Placing buildings

- `AddNewBuildingOccupant(type, float pos[3], orientation)` (slot 7, `0x00698DE0`): false/null unless
  `IsBuildingTypeValid(type)` and `LocationIsOKForNewBuilding(type, pos[0], pos[2])`. Creates a building occupant
  (`0x5C` bytes, `0x00458A60` / `0x00458E90`), sets its type, loads its properties (`0x00698490`; failure → release,
  return null), takes its bounding box, swaps the box's x and z extents for orientation 1 or 3 and stores it back.
  If `pos[1] < 0` (strictly; NaN does not count) the altitude is replaced by
  `lotDeveloper.GetLowestOrHighestAreaAltitude({min.x, min.z, max.x, max.z}, false, true)` (slot 16) — with the
  **box's** local coordinates, not offset by the position *[quirk]*. Then sets the position and orientation, initialises
  the occupant and inserts it (`AddNewBuildingOccupant(occupant)`, slot 8, through the vtable). Returns the
  occupant (borrowed: the occupant manager holds the reference) or **null**.
  **References** *[compat]*: the BDS AddRefs the new occupant once (its creation reference, count 1) and releases
  that reference on **every** exit after creation (`0x00698F1B`): property-load failure, insert failure (slot 8
  false) and success alike. On insert failure it therefore returns null and holds nothing; the occupant is destroyed
  unless the occupant manager kept a reference of its own. On success the return value is the pointer after that
  release, kept alive only by the occupant manager. A caller never releases the result.
- `AddNewBuildingOccupant(type, cx, cz, orientation)` (slot 6) = slot 7 with
  `pos = (centreX, terrain.GetAltitude(centreX, centreZ), centreZ)` of the cell (city `CellCenterToPosition`).
- `PlopBuilding(profile)` (slot 47, `0x00695570`) = slot 6 (through the vtable) with
  `(profile.buildingType, profile.cellX, profile.cellZ, profile.orientation)`: the orientation is the int32 at +0x18
  between the building type and cell x (SDK `unknown3`, §4.8), **not** a land value field *[compat]*.
- `LocationIsOKForNewBuilding(type, x, z)` (slot 5): false if the type is unknown; else true when the occupant
  manager finds **no** occupant of type `0x278128A0` (buildings) at the cell containing (x, z) (slot 33).
- `AddNewBuildingOccupant(occupant)` (8) / `AddNewFoundationOccupant(occupant)` (24): `occupantManager.InsertOccupant(
  occupant.AsOccupant(), false)` (slot 20); false if there is no occupant manager.
- `DemolishFoundationOccupant(occupant, u)` (25): `occupantManager` slot 21 `(occupant, true, false)`.
- `GetCostOfOccupantDemolition(building, u)` (10): int64 property `0x099AFACD` of the occupant, default 0 (u unused).
  The foundation overload (9) returns 0.

### 4.10 `DoBuildingChanged(lot, wealth, built)` (`0x0069CA80`)

```
T := lot.GetBuildingType(false);  P := GetBuildingProfile(T)        -- inserting if unknown
key := built ? uint8(4·P.purpose + wealth + 1) : 0                  -- wealth = message data4, not P.wealth
write key into every cell of lot.GetBoundingRect() of the building map (raw grid data; no notification)
if built and P.purpose ≠ 0 and P.wealth = wealth:
    stage := lot.GetLotConfiguration().GetGrowthStage();  B := lot.GetBuilding()
    if both exist:
        t := GetDeveloperType(P.purpose, P.wealth)
        if stage > highestStage[t]:                              -- inserted as 0 if absent
            highestStage[t] := stage
            subject := new cSC4AdviceSubject (0x6A26D049 / 0x6A26D050)
            subject.SetLocation(r24(pos.x / CellWidthX), r24(pos.z / CellWidthZ))   -- B's position, unrounded cells
            subject.SetObject(lot);  subject.SetData1(stage << 24 | t)
            advisorSystem.GetAdvisor(0x17).PushEvent(0x4A64A9CC, null)              -- city slot 81 → slot 7; slot 18
            release the subject
```

*[quirk]* The event is pushed with a **null** subject; the filled subject is discarded. The advisor is not null
checked. The building map cell value is `4·purpose + wealth + 1`, so residential keys are 6, 7, 8 (residential spec
§4); it is cleared to 0 when the lot leaves states 2/3.

### 4.11 `DoTerrainChanged(rect, kind)` (`0x00698610`)

Nothing for an empty rectangle. Lots = `lotManager.GetLotsInCellRect(rect, false)` (B list, allocators.md).

- `kind ∉ {1, 2, 3}`: `lotDeveloper.UpdateLotElements(lot, kind)` (slot 37) for each lot.
- *[compat]* Both call sites (`0x00698690`, `0x00698898`) pass `kind` as the **whole 32-bit** data4 value. Slot 37's
  second parameter is a `uint32` kind, not a bool: the lot developer only tests `kind ≠ 5` (full 32-bit compare)
  to decide whether a lot without a foundation is re-checked for one (`lot-developer.md` §6.4). A replacement must
  pass the kind unchanged; reducing it to `kind & 0xFF ≠ 0` would turn kind 5 into "true" and re-check the slope.
- `kind ∈ {1, 2, 3}`: for each lot:
  `d := r24(terrain.GetAltitude(cx, cz) − cy)` with `(cx, cy, cz) := lot.GetCenter()`;
  `A := atan(r24(lotDeveloper.GetLotSlope(lot, false) × r24(1 / CellWidthX)))`,
  `B := atan(r24(lot.GetSlope() × r24(1 / CellWidthX)))` (each arctangent rounded to float32).
  If `−1 ≤ d ≤ 2` (NaN counts as inside) and `r24(A − B) ≤ 0.08726647` (5°, float32 `0x3DB2B8C3`):
  `UpdateLotElements(lot, kind)`. Otherwise the change is too big: a lot that `IsInDemolishableState()` is queued for
  demolition; any other lot's rectangle is dezoned at once (`zoneManager.PlaceZone(region, zone 0, true,
  kind ∈ {1, 2}, false, true, true, null, null, null)`).
  Then `demolition.DemolishLots(true, queued, 0, 1, true, null, &cost, null)` (global `0x00B43D00`, slot 7); if
  `kind ∈ {1, 2}` and `cost > 0`: `budget.SetTotalFunds(budget.GetTotalFunds() − cost)` (int64).
  *[compat]* `DemolishLots` is called **unconditionally** on this path (call `0x006988D1`), also when nothing was
  queued and when the rectangle holds no lot. `cost` is set to 0 before the call; with an empty list
  `cSC4Demolition::DemolishLots` (`0x004670C0`) returns true at once without touching `cost`, so the funds are
  unchanged and nothing observable happens. A replacement may skip the call for an empty list.

The meaning of the kinds (1–3: terraforming tools *[inferred]*) is the terrain's (open question §10.2 Q3).

### 4.12 Special buildings, cheats, PlaceBuilding

- **Special buildings** (§3.2): occupants of groups `0x150A` / `0x150B` (landmarks, rewards *[inferred names]*),
  in insertion order; `GetSpecialBuildings` (44) returns the list itself.
- **Cheats**: "HowDryIAm" → `CheatIgnorePlumbing` (54) true; "FightThePower" → `CheatIgnorePower` (55) true (used by
  §4.8 check 3); "Barstuck" toggles `CheatCoffeeBar` (58). `CheatIgnoreCaps` (56) is set only by `Read`;
  `CheatDebugGrowth` (57) is never set. The BDS itself reads only ignore-power; the others are for its callers.
- **PlaceBuilding** command (`0x0B99AE81`): parameters from the command object (void1, slot 5 `GetParameter(i)`):
  type = parameter 0 as uint32 (variant slot 24), a = parameter 1 and b = parameter 2 as float (slot 36). Calls
  `AddNewBuildingOccupant(type, {a, b, −1.0}, 2)` (slot 7, through the vtable). *[quirk]* b lands in `pos[1]` (y) and
  z is −1.0; the command appears unusable as shipped. Returns true.

### 4.13 Tuning *[compat]*

`ReadSimulatorTunableValues` (`0x006959E0`) runs at every `Init`. Exemplar `{type 0x6534284A, group 0xE7E2C2DB,
instance 0xE8DA7677}` (the "simulators" exemplar, `re/tuning/simulators.md`), fetched as `cISCResExemplar`
(`0xA52160F5`). Code defaults are assigned **before** each read; a missing exemplar leaves them.

| Property | Name (re/tuning) | Read as | Code default | Shipped |
|---|---|---|---|---|
| `0x47E2C380` | Construction Resources Min/Max | the first two uint32 of the property's data (`RefUint32`, variant slot 103), **no type or count check** | 100, 2000 | 8192, 20480 |
| `0x47E2C381` | (unnamed; per-resident factor) | float (`0x005FD4F0`) | 0.5 | absent → 0.5 |
| `0x47E2C382` | (unnamed; bias) | uint32 (`0x005FD480`), stored as int32 | 10 | absent → 10 |
| `0x47E2C383` | Stage Population Multiplier | float vector (`0x005FFCC0`: replaces the vector; missing → unchanged) | empty | 1.2, 1.1, 1.0 |

**`0x47E2C380` in detail.** The code takes the property's value variant (property slot 5) and its slot 103
(`0x0060CE80`), which returns the variant's data word as a pointer whatever the type is, and reads two uint32 at
that pointer: `crMin := word[0]`, `crMax := word[1]`. A game variant keeps an array (type with bit `0x8000`) as a
pointer to a global-heap block of count × element-size bytes, and a single value **inline** in the same word.
Behaviour of the original by property shape:

| Property shape | Original |
|---|---|
| absent | both defaults kept (100, 2000) |
| array of ≥ 8 data bytes (shipped: uint32 × 2) | the first 8 bytes as two little-endian uint32, whatever the element type (a float array gives the bit patterns) *[compat]* |
| array of 4–7 data bytes (e.g. one uint32) | `crMin` from bytes 0–3; `crMax` from the 4 bytes **after** the property's data block (heap garbage) |
| array of 0–3 data bytes | both words read past or outside the data (garbage, or a fault for a null pointer) |
| single (non-array) value | the value itself is used as an address and read: a fault for any small value (e.g. 8192) |

*[choice]* A replacement reads the property's actual data bytes (an array's count × element size; a single value's
own bytes, little-endian) and takes `crMin` from bytes 0–3 and `crMax` from bytes 4–7, **each only when all four of
its bytes exist**; a word without four bytes behind it keeps its code default (100 or 2000). This matches the
original wherever the original reads defined data, and never faults. The clamp of §4.1 uses the values as read,
even when `crMin > crMax` (then the result is always `crMax`, since the `min` comes last).

`GetStagePopulationMultiplier()` (53) returns `stageMultiplier[city.GetDifficultyLevel()]` (0 easy, 1 medium,
2 hard) in ST0, **without bounds check** *[quirk]*. *[choice]* FreeCity4 returns 1.0 when the index is outside the
vector. The BDS loads **no response curves** (foundations §4.1 does not apply).

### 4.14 Random draws *[compat]*

**Generator**: one file-local `cRZRandom` (foundations §6.1, **not** a `cRZRandomSuper`), static object
`0x00B485F0`, constructed at process start (`0x00A74B40`) with seed `cRZTime() XOR 0x0C09C0B8` (local wall-clock
time; `cRZRandom` constructor `0x00913A5C` → `Seed`). Never re-seeded; not saved. Only the integer method is used.
*[choice]* FreeCity4 seeds it from the host (foundations §6.3); the shim uses the wall clock like the original.

| # | Operation | Call | Argument | Position | Draws |
|---|---|---|---|---|---|
| 1 | `Grow` (daily, §4.4) | `RandomUint32Uniform(n)` (`0x009136B1`) | n = Σ (ad + bias) over the pending developers with ad > 5 | before each pick; picks interleave with the tract developer's `Grow` calls | one per qualifying developer per day; 0 when none qualifies |
| 2 | `PlanDecline` (monthly, §4.3) | `RandomUint32Uniform(n)` | n = Σ (bias − ad) over the qualifying developers (constant during the plan) | once per loop pass | one per pass until every qualifying decline list is used up (wasted passes included); 0 when total = 0 |
| 3 | `PlanGrowth` (dead, `0x00699596`) | `RandomUint32Uniform` | — | never runs | 0 |

`RandomUint32Uniform(n)` = `(NextRaw() × n) >> 32` (one LCG step), and 0 without a step when n = 0. The pick walks
the developers in ascending type with a signed `r < w`. Helpers called by the BDS draw from their own generators
(tract developer, lot developer; their specs); the BDS calls no `rand()`, shuffle or random iterator.

---

## 5. Messages posted

| Message | When | Fields |
|---|---|---|
| `0xA783EBC8` kSC4MessageBuildingTypesUpdated | once per process, at the end of the catalogue load (§1.3 step 1) | `cRZMessage2Standard`; `SetVoid1(null)`, `SetData2(building type count)`; posted (`MessagePost`, slot 4 `+0x10`, queued) |
| `0x426840A0` ActiveDemandChanged | every `ClaimConstructionResources` that succeeds and every `Release…` (the construction node's `EndOfCycle`) | from the node (demand spec §3.4), data1 = 0 |

The CityPlanning advisor event of §4.10 is pushed through the advisor system, not the message server.

---

## 6. Persistence *[compat]*

`cIGZSerializable` (sub-object +0xC): `Write` slot 3 (`0x00697800`), `Read` slot 4 (`0x0069ACB0`), `GetGZCLSID`
slot 5 → `0x8990BE7A`. Both return false at once (writing/reading nothing) if the stream is already in error.
The record (one class-store entry, foundations §3.3), **version 4**:

```
u16   version = 4
ref   building map                        read with IID 0xC9BD939A (cISC4SimGrid<uint8>)
ref   construction node                   read with IID 0x0797A4E8 (cISC4Demand)
u32   nDev;    nDev × { u32 developer type ; ref developer (IID 0xE7CDD662) }      -- ascending type
u32   nSpec;   nSpec × ref occupant (IID 0x26BB767E)                               -- list order
u32   nStage;  nStage × { u32 developer type ; u8 highest stage }                  -- ascending type
u32   nMark;   nMark × { u32 developer type ; u32 nCell ; nCell × { s32 x ; s32 z } }
                                           -- ascending type; cells ascending by (x, z), signed
u32   nPlan;   nPlan × { ref developer (IID 0xE7CDD662) ; s32 x ; s32 z ; u8 stage ; u32 day }
                                           -- the planned-growth map, hash order; 0 in practice
u8    ignore plumbing ; u8 ignore power ; u8 ignore caps                          -- bools
```

`Write` returns true iff the stream has no error afterwards.

`Read`:

- Version ≠ 4 → return false (only the 2 bytes consumed; `loadedFromSave` stays false).
- Sets `loadedFromSave := true`, then reads in the order above with the given IIDs (a failed reference leaves
  null, foundations §2.7). Developers go into the map by `operator[]` (holding the read reference); special
  buildings are **appended** (holding the reference); stages by `operator[]`; marks are **merged** into the
  process-wide set (which `Shutdown` emptied); planned-growth entries are inserted or overwritten, and the developer
  reference read for each is **released** (the map holds no reference).
- Marks: each saved type's entry is created by the insert-if-absent lookup (`0x0069AEC6`) before its cells are read,
  so a saved entry with `nCell = 0` comes back as an **empty entry** and is written again as such (§4.4). `Write`
  writes every entry of the process-wide map (`0x00697968`), empty ones included.
- Developers *[compat]*: each pair is stored as `map[type] := ref` (`0x0069AD70`), an overwrite. If the map already
  holds `type`, the old pointer is overwritten **without** `Release`: its reference **leaks** (the object is never
  released). On the city-load path the BDS is freshly constructed, so its map is empty and this only happens when one
  record lists the same type twice, which `Write` never produces (map keys are unique). *[choice]* A replacement
  releases the replaced reference; nothing observable depends on the leak.
- A developer reference that fails to resolve is stored as a **null** entry. The original then crashes: `Init`'s
  `CreateDeveloper` finds the entry and calls `Init` on it (`0x006991E9`), and `DoMonthlySimulation` calls
  `StartMapUpdate` on every entry. *[choice]* A replacement drops null developer entries after reading, so
  `CreateDeveloper` creates a fresh developer for that type.
- Returns true iff the stream has no error at the end.

Recomputed on load: the static catalogue (if not yet built), the tuning, `lastSuccessfulBuild` (today, per developer,
at `Init`), the developers' `Init(holder)`, the planning state (0) and an empty decline plan. Because the state is 0,
the level-0 SimBegin call rebuilds the maps at once. Not saved: coffee bar, debug growth, the RNG, the timer step.

---

## 7. Interfaces *[compat]*

All methods `__thiscall`; "pop" = bytes removed by the callee (`abi_extract --check`); returns: AL = bool in AL only,
EAX = 32-bit, EDX:EAX = 64-bit, ST0 = x87.

### 7.1 `cISC4BuildingDevelopmentSimulator` (vtable `0x00AB0AF8`, 59 interface slots)

| Slot | Off | Signature as compiled | Pop | Return | Behaviour / notes | Address |
|---|---|---|---|---|---|---|
| 0 | +0x00 | `bool QueryInterface(uint32 iid, void** ppv)` | 8 | AL | §1.1 | `0x00695290` |
| 1 | +0x04 | `uint32 AddRef()` | 0 | EAX | | `0x00695330` |
| 2 | +0x08 | `uint32 Release()` | 0 | EAX | §1.1 | `0x006959B0` |
| 3 | +0x0C | `bool Init()` | 0 | AL | §1.3 | `0x0069D240` |
| 4 | +0x10 | `bool Shutdown()` | 0 | AL | §1.4 | `0x0069A210` |
| 5 | +0x14 | `bool LocationIsOKForNewBuilding(uint32 type, f32 x, f32 z)` | 12 | AL | §4.9 | `0x00695590` |
| 6 | +0x18 | `cISC4BuildingOccupant* AddNewBuildingOccupant(uint32 type, int32 cx, int32 cz, int32 orientation)` | 16 | EAX | §4.9. **SDK error**: returns the occupant pointer (or null), not bool | `0x006956A0` |
| 7 | +0x1C | `cISC4BuildingOccupant* AddNewBuildingOccupant(uint32 type, f32* pos, int32 orientation)` | 12 | EAX | §4.9. **SDK error**: pointer, not bool | `0x00698DE0` |
| 8 | +0x20 | `bool AddNewBuildingOccupant(cISC4BuildingOccupant*)` | 4 | AL | §4.9 | `0x00695740` |
| 9 | +0x24 | `int64 GetCostOfOccupantDemolition(cISC4FoundationOccupant*, uint32)` | 8 | EDX:EAX | returns 0. **SDK error**: the SDK has `(void)` and declares it later (non-adjacent overload, foundations §17.8) | `0x006AFC80` |
| 10 | +0x28 | `int64 GetCostOfOccupantDemolition(cISC4BuildingOccupant*, uint32)` | 8 | EDX:EAX | §4.9 | `0x00695700` |
| 11 | +0x2C | `size_t GetAllBuildingTypes(vector<uint32>&)` | 4 | EAX **garbage** | §4.7.2. Replacement: return the vector's new size | `0x00697C50` |
| 12 | +0x30 | `size_t GetAllBuildingTypes(uint32* array, uint32& n)` | 8 | EAX | returns the total type count | `0x00697B80` |
| 13 | +0x34 | `size_t GetAllBuildingFamilyTypes(vector<uint32>&)` | 4 | EAX garbage | as 11 | `0x00697D10` |
| 14 | +0x38 | `bool GetBuildingKeyFromType(uint32, cGZPersistResourceKey&)` | 8 | AL | | `0x00697DD0` |
| 15 | +0x3C | `bool IsBuildingTypeValid(uint32)` | 4 | AL (EAX 0/1) | | `0x00697E30` |
| 16 | +0x40 | `bool IsBuildingFamilyTypeValid(uint32)` | 4 | AL | §4.7.2 | `0x00697E70` |
| 17 | +0x44 | `uint32 GetBuildingTypesByFilter(hash_set<uint32>&, cISC4BuildingTypeFilter*)` | 8 | EAX count | **SDK error**: returns a count, not bool (a count that is a multiple of 256 reads as false through AL); the set is an STLport `hash_set` (B), not `std::unordered_set` | `0x00699C50` |
| 18 | +0x48 | `uint32 GetBuildingTypesByFamily(vector<uint32>&, uint32 family)` | 8 | EAX count | **SDK error**: count, not bool; declared after the hash-set overload in the SDK (reverse-order rule, foundations §16) | `0x00697EF0` |
| 19 | +0x4C | `uint32 GetBuildingTypesByFamily(hash_set<uint32>&, uint32 family)` | 8 | EAX count | **SDK error**: count, not bool; STLport `hash_set` | `0x00699D20` |
| 20 | +0x50 | `bool IsBuildingInFamily(uint32 type, uint32 family)` | 8 | AL | | `0x00697FE0` |
| 21 | +0x54 | `uint32 GetAllFoundationTypes(vector<uint32>&)` | 4 | EAX count | **SDK error**: count, not bool | `0x00698050` |
| 22 | +0x58 | `bool GetFoundationKeyFromType(uint32, cGZPersistResourceKey&)` | 8 | AL | | `0x00698120` |
| 23 | +0x5C | `bool AddNewFoundationOccupant(uint32, int32, f32*, f32, f32, int32, f32)` | 28 | AL | foundation placement (not used by growth) | `0x00695AF0` |
| 24 | +0x60 | `bool AddNewFoundationOccupant(cISC4FoundationOccupant*)` | 4 | AL | §4.9 (same body as 8) | `0x00695740` |
| 25 | +0x64 | `bool DemolishFoundationOccupant(cISC4FoundationOccupant*, uint32)` | 8 | AL (via call) | §4.9 | `0x00695770` |
| 26 | +0x68 | `bool LoadFoundationProperties(cISC4FoundationOccupant*, uint32)` | 8 | AL | | `0x006982B0` |
| 27 | +0x6C | `uint32 GetAllLotRetainingWallTypes(vector<uint32>&)` | 4 | EAX count | **SDK error**: count, not bool | `0x00698180` |
| 28 | +0x70 | `bool GetLotRetainingWallKeyFromType(uint32, cGZPersistResourceKey&)` | 8 | AL | | `0x00698250` |
| 29 | +0x74 | `bool LoadLotRetainingWallProperties(cISC4LotRetainingWallOccupant*, uint32)` | 8 | AL | | `0x006983C0` |
| 30 | +0x78 | `DeveloperType GetDeveloperType(PurposeType, WealthType)` | 8 | EAX | §4.7.3 | `0x00698D60` |
| 31 | +0x7C | `WealthType GetWealth(DeveloperType)` | 4 | EAX | §4.7.3 | `0x00698D80` |
| 32 | +0x80 | `cISC4GrowthDeveloper* GetDeveloper(DeveloperType)` | 4 | EAX | map lookup, null if absent, **no AddRef** | `0x00696C80` |
| 33 | +0x84 | `cISC4GrowthDeveloper* GetDeveloper(PurposeType, WealthType)` | 8 | EAX | slot 32 of slot 30, both through the vtable | `0x006958B0` |
| 34 | +0x88 | `std::map<uint32, cISC4GrowthDeveloper*>* GetDevelopers()` | 0 | EAX | the internal map itself (BDS + 0x5C; the STLport map of [docs/abi/allocators.md](../abi/allocators.md) *map and set*, B header and 0x18-byte B nodes, one reference held per developer); SDK `intptr_t` | `0x00698CD0` |
| 35 | +0x8C | `cISC4TractDeveloper* GetTractDeveloper()` | 0 | EAX | cached, no AddRef | `0x0068D1D0` |
| 36 | +0x90 | `cISC4LotDeveloper* GetLotDeveloper()` | 0 | EAX | cached, no AddRef | `0x00698CE0` |
| 37 | +0x94 | `cIGZString* GetExemplarName(uint32)` | 4 | EAX | §4.7.2 | `0x0069AB20` |
| 38 | +0x98 | `cIGZString* GetBuildingName(uint32)` | 4 | EAX | §4.7.2 | `0x0069AB00` |
| 39 | +0x9C | `BuildingProfile& GetBuildingProfile(uint32)` | 4 | EAX | inserts if absent | `0x0069CA60` |
| 40 | +0xA0 | `BuildingProfile* GetBuildingProfilePtr(uint32)` | 4 | EAX | | `0x006985C0` |
| 41 | +0xA4 | `cISC4SimGrid<uint8>* GetBuildingMap()` | 0 | EAX | no AddRef; SDK `intptr_t` | `0x00698CF0` |
| 42 | +0xA8 | `uint8 GetBuildingMapKey(PurposeType, WealthType)` | 8 | **AL** | **SDK error**: the SDK's `intptr_t` reads EAX; only AL is set (upper bits garbage) | `0x00698D90` |
| 43 | +0xAC | `uint8 GetBuildingMapKey(uint32 type)` | 4 | **AL** | same SDK error | `0x0069CCB0` |
| 44 | +0xB0 | `ilist<cISC4Occupant*>* GetSpecialBuildings()` | 0 | EAX | the internal list (A); SDK `intptr_t` | `0x00520080` |
| 45 | +0xB4 | `f32 GetMaxCapacity(ZoneType, PurposeType, WealthType)` | 12 | **ST0** | §4.7.4. **SDK error**: the SDK says `int64` (EDX:EAX hold garbage) | `0x00696C20` |
| 46 | +0xB8 | `bool IsSuitableForConstruction(ConstructionProfile const&, int32& reason)` | 8 | AL | §4.8 | `0x00695430` |
| 47 | +0xBC | `cISC4BuildingOccupant* PlopBuilding(ConstructionProfile const&)` | 4 | EAX | §4.9. **SDK error**: pointer, not bool | `0x00695570` |
| 48 | +0xC0 | `int32 GetFreeConstructionResources()` | 0 | EAX | §4.6; SDK `int16` reads AX: equal for values < 32768 (shipped max 20480) | `0x006958E0` |
| 49 | +0xC4 | `int32 GetTotalConstructionResources()` | 0 | EAX (EDX = high half of the `_ftol`) | as 48 | `0x00695910` |
| 50 | +0xC8 | `bool ClaimConstructionResources(uint16)` | 4 | AL | §4.6 | `0x00695920` |
| 51 | +0xCC | `bool ReleaseConstructionResources(uint16)` | 4 | AL | §4.6 (returns `EndOfCycle`'s result) | `0x00695980` |
| 52 | +0xD0 | `int32 GetDayOfLastSuccessfulBuild(DeveloperType)` | 4 | EAX | 0 if absent | `0x00696CC0` |
| 53 | +0xD4 | `f32 GetStagePopulationMultiplier()` | 0 | ST0 | §4.13 | `0x00696040` |
| 54–58 | +0xD8 … +0xE8 | `bool CheatIgnorePlumbing / CheatIgnorePower / CheatIgnoreCaps / CheatDebugGrowth / CheatCoffeeBar()` | 0 | AL | §4.12 | `0x00741740`, `0x00698D00`, `0x00698D10`, `0x0098AD60`, `0x00698D20` |
| (59) | +0xEC | scalar deleting destructor | 4 | EAX | not part of the interface | `0x00699180` |

Compiled order vs the SDK header (MSVC overload grouping and reversal, foundations §16): 6/7/8 are the header's
`(uint32,int32,int32,int32)` / `(uint32,float*,int32)` / `(occupant*)`; 11/12 vector / array; 18/19 vector / hash
set; 23/24 seven-argument / occupant; 32/33 by type / by purpose and wealth; 42/43 by purpose and wealth / by type;
slot 9 is the SDK's later `GetCostOfOccupantDemolition(void)`.

**What a replacement returns where the original returns garbage**: slots 11 and 13 → the vector's size; slot 45 →
the float in ST0 (game callers read ST0); slots 42/43 → the key zero-extended in EAX; slots 6/7/47 → the occupant
pointer or null. A replacement written against the SDK must declare these return types as compiled (§9.3).

### 7.2 `cIGZMessageTarget2` (vtable `0x00AB0AD8`, 4 slots, sub-object +8)

| Slot | Method | Pop | Return | Address |
|---|---|---|---|---|
| 0–2 | QueryInterface / AddRef / Release (thunks, this − 8) | 8 / 0 / 0 | | `0x00698D40`, `0x005200F0`, `0x00698DC0` |
| 3 | `bool DoMessage(cIGZMessage2*)` | 4 | AL = 1 always | `0x0069CCE0` |

### 7.3 `cIGZMessageTarget` (vtable `0x00AB0AE8`, 4 slots, sub-object +4)

Slots 0–2 thunks (this − 4: `0x00698D30`, `0x005200D0`, `0x00698DB0`); slot 3 `bool DoMessage(cIGZMessage*)`
(`0x004FC900`, pop 4) returns true and does nothing.

### 7.4 `cIGZSerializable` (vtable `0x00AB0AC0`, 6 slots, sub-object +0xC)

| Slot | Method | Pop | Return | Address |
|---|---|---|---|---|
| 0–2 | thunks (this − 0xC) | 8 / 0 / 0 | | `0x00698D50`, `0x00520110`, `0x00698DD0` |
| 3 | `bool Write(cIGZOStream&)` | 4 | AL | `0x00697800` |
| 4 | `bool Read(cIGZIStream&)` | 4 | AL | `0x0069ACB0` |
| 5 | `uint32 GetGZCLSID()` | 0 | EAX `0x8990BE7A` | `0x00695280` |

---

## 8. Dependencies *[compat]*

| Interface | Slot / offset | Method | Pop | Return | Used in |
|---|---|---|---|---|---|
| `cISC4GrowthDeveloper` | 3 `+0x0C` | `Init(cISCPropertyHolder*)` | 4 | AL | §1.5 |
| | 4 `+0x10` | `Shutdown()` | 0 | AL | §1.4 |
| | 6 `+0x18` | `GetSubType()` (= developer type) | 0 | EAX | Grow |
| | 7 `+0x1C` | `GetPurpose()` | 0 | EAX | PlanDecline |
| | 10 `+0x28` | `GrowthAndDeclineListsAreValid()` | 0 | AL | Grow |
| | 11 `+0x2C` / 12 `+0x30` | `GetGrowthList()` / `GetDeclineList()`: reference to a vector of 12-byte `{int32 x, int32 z, uint8 stage}`; the BDS only reads `begin`/`end` | 0 | EAX | Grow, PlanDecline |
| | 18 `+0x48` | `GetSourceDemand()` | 0 | EAX | ad(D) |
| | 27 `+0x6C` / 28 `+0x70` | `StartMapUpdate()` / `IsMapUpdateFinished()` | 0 | — / AL | §4.1, §4.2 |
| `cISC4Demand` (growth developers' source nodes and the construction node) | 3 `+0x0C`, 4 `+0x10`, 8 `+0x20`, 9 `+0x24`, 12 `+0x30`, 17 `+0x44`, 18 `+0x48`, 19 `+0x4C`, 30 `+0x78` | Init, Shutdown, QuerySupplyValue (ST0), QueryDemandValue (ST0), QueryActiveDemandValue (ST0), AddToDemandValue(f32), SetSupplyValue(f32), SetDemandValue(f32), EndOfCycle (AL) | 0/0/0/0/0/4/4/4/0 | | §4.1–§4.6 (demand spec §3.2) |
| `cISC4DemandSimulator` (`0x00B43D74`) | 6 `+0x18` | `GetDemand(uint32 id, uint32 index)` | 8 | EAX | §4.1 |
| `cISC4TractDeveloper` (`0x00B43D24`) | 5 `+0x14` | `int32 Grow(GrowthProfile const&, std::list<Outcome>&, bool&)` — **returns int** (built capacity; tested `> 0`), not bool | 12 | EAX | §4.4 |
| | 6 `+0x18` | `int32 Decline(GrowthProfile const&)` (tested `> 0`) | 4 | EAX | §4.5 |
| `cISC4Simulator` (`0x00B43D5C`) | 7 `+0x1C` | `GetSimDate()` → `cIGZDate*`, then `cIGZDate` slot 26 `+0x68` day number (EAX) | 0 | EAX | dates |
| | 19 `+0x4C` / 20 `+0x50` | AddAgent / RemoveAgent | | | §1.3, §1.4 |
| `cISC4City` (`0x00B43CEC`) | 24 `+0x60`, 34 `+0x88`, 50 `+0xC8`, 81 `+0x144`, 91/92 `+0x16C`/`+0x170` (ST0), 95 `+0x17C` (pop 16), 97 `+0x184` (pop 16) | GetDifficultyLevel, GetOccupantManager, GetSimulator, GetAdvisorSystem, CellWidthX/Z, PositionToCell, CellCenterToPosition | | | §4.7–§4.13 |
| `cISC4ZoneManager` (`0x00B43D14`) | 12 `+0x30` (pop 8, EAX = int8 sign-extended, `0x004DC740`) / 11 `+0x2C` (pop 40, AL) | `GetZoneType(x, z)` / `PlaceZone(...)` | | | §4.8, §4.11 (zones.md) |
| `cISC4LotManager` (`0x00B43D08`) | 6 `+0x18` (pop 12) / 7 `+0x1C` (pop 24) | `GetLot(x, z, bool)` / `GetLotsInCellRect(list, x1, z1, x2, z2, bool)` | | EAX / **EAX count** (`0x0072E080`, zones.md §8.3 and §11.3; the BDS ignores it) | §4.8, §4.11 |
| `cISC4Lot` | 9 `+0x24`, 14 `+0x38`, 15 `+0x3C`, 19 `+0x4C` (ST0), 33 `+0x84`, 39 `+0x9C`, 46 `+0xB8` (pop 4), 48 `+0xC0` | GetLotConfiguration, GetBoundingRect, GetCenter, GetSlope, GetState, IsInDemolishableState, GetBuildingType(bool), GetBuilding | | | §3.2, §4.8–§4.11 |
| `cISC4LotConfiguration` | 7 `+0x1C`, 11 `+0x2C` (AL), 15 `+0x3C`, 24 `+0x60` | GetSize, GetGrowthStage, IsCompatibleWithZoneType, GetPrimaryBuildingConfigurationObject | | | §4.7.4, §4.10 |
| `cISC4LotConfigurationManager` (`0x00B43D18`) | 20 `+0x50` | GetLotConfigurationMap | 0 | EAX | §4.7.4 |
| `cISC4LotDeveloper` (`0x00B43D20`) | 16 `+0x40` (pop 12, ST0), 30 `+0x78` (pop 8, ST0), 37 `+0x94` (pop 8) | GetLowestOrHighestAreaAltitude, GetLotSlope, UpdateLotElements | | | §4.9, §4.11 (lot-developer.md) |
| `cISC4LandValueSimulator` (`0x00B43D88`) | 5 `+0x14` / 6 `+0x18` | GetLandValueType (EAX 1–3) / GetLandValue (AL) | 8 | | §4.8 (landvalue spec §11) |
| `cISC4PowerSimulator` (`0x00B43DA0`) | 19 `+0x4C` | powered at (x, z) | 8 | AL | §4.8 (power spec) |
| `cISC4OccupantManager` | 20 `+0x50` (pop 8), 21 `+0x54` (pop 12), 33 `+0x84` (pop 16) | InsertOccupant, RemoveOccupant, occupant-at-cell query | | AL | §4.9 |
| `cISTETerrain` (`0x00B43CF4`) | 7 `+0x1C`, 8 `+0x20`, 13 `+0x34` (pop 8, ST0) | cell counts, GetAltitude | | | §1.3, §4.9, §4.11 |
| `cISC4Demolition` (`0x00B43D00`) | 7 `+0x1C` | DemolishLots (8 args) | 32 | AL | §4.11 |
| `cISC4BudgetSimulator` (`0x00B43D64`) | 5 `+0x14` / 6 `+0x18` | SetTotalFunds(int64) / GetTotalFunds (EDX:EAX) | 8 / 0 | | §4.11 |
| `cISC4AdvisorSystem` → `cISC4Advisor` | 7 `+0x1C` / 18 `+0x48` | GetAdvisor(0x17) / PushEvent(id, subject) | 4 / 8 | EAX / AL | §4.10 (advisors spec) |
| `cISC4AdviceSubject` (`0x6A26D049`) | 3, 7, 10 | SetLocation(f32, f32), SetObject, SetData1 | | | §4.10 |
| resource manager (`0x00B43CA8`) | 3 `+0x0C`, 18 `+0x48` | GetResource(key, iid, &out, 0, 0), key list of a type | 20 / 8 | | §1.3, §4.13 |
| message server, command server, cheat code manager | 4/5/6; 6/9; 3/4/7/8 | post / subscribe / unsubscribe; register / unregister; register, unregister, subscribe, unsubscribe | | | §1.3, §1.4, §5 |
| `cIGZVariant` (property values) | 4 `+0x10`, 24 `+0x60`, 36 `+0x90`, 103 `+0x19C` | count, GetValUint32, GetValFloat32, RefUint32 | | | §4.12, §4.13 |

No new interface vtables are needed for the dependencies beyond those already in `docs/abi`.

---

## 9. Replaceability

**Replacement unit** *[choice]*: this simulator's developer creation and the growth developer (growth-developer.md
§12) are one unit; replacing either alone leaves new and loaded cities with different developer classes.

### 9.1 Who reaches into the BDS

- **Raw containers through the interface** (a replacement must provide the game's layouts): the developer map
  (slot 34, an STLport `std::map<uint32, cISC4GrowthDeveloper*>` with B nodes, [docs/abi/allocators.md](../abi/allocators.md) *map and set*; iterated by the demand simulator,
  demand spec §8, and others *[callers not enumerated]*); the special-building list (slot 44, `ilist<cISC4Occupant*>`,
  allocator A); the building profiles (slots 39/40: the game's `BuildingProfile` with a vector of STLport `std::map<uint32, uint16>`
  capacity tables, B; layout in [docs/abi/layouts.md](../abi/layouts.md) *Building profile*); names as `cRZString` objects inside the static maps (slots 37/38).
- **Raw grid**: the building map (slot 41) is read raw by the residential simulator (residential spec §4, cells
  through `GetGridData`) and the commercial proximity maps *[inferred: commercial]*. The BDS itself writes it raw
  (§4.10). Simgrid §6.4 applies: a replacement creates the grid through GZCOM (class `0x49B9E602`).
- **Global**: `SL::spBuildingDevelopmentSim` (`0x00B43D68`) is read by many callers (advisors, MySims, lot,
  lot configuration manager, residential, commercial, demand, aura, tract and growth developers, query tool,
  zone manager, demolition, lot-configuration editor …); all calls found go through the vtable.
- **Non-virtual calls into the BDS** from outside: none found. Statics used only inside: `GetDeveloperType_Static`,
  `BuildingFitsInZone`, `GrowthStageFitsInZone` (each called by other classes only through slots 30/31 *[inferred]*).
- **Internal containers** (never cross an interface; any representation): the decline plan, the tried-tract marks,
  `lastSuccessfulBuild`, `highestStage`, the pending copy in `Grow`, PlanDecline's cursors, the planned-growth map,
  the static name/key/family/foundation/retaining-wall/density tables (except through the slots above).

### 9.2 Containers crossing an interface (allocators.md format)

See the allocator rows in the final report of this round; summary: the outcome list given to the tract developer's
`Grow` is a B `std::list` (head and nodes freed by the BDS with B); vectors filled by slots 11/13/18/21/27 and the
hash sets of 17/19 are B (not SDK containers); `GetDevelopers` returns a B map; `GetSpecialBuildings` returns an A
`ilist`; strings returned by 37/38 are owned by the BDS.

### 9.3 Replacing the class

The class is created by the city through the GZCOM registry (class `0x8990BE7A`), so it can be replaced. A
replacement must keep: the 59-slot layout with the return types of §7.1 (not the SDK's), the `cIGZSerializable`
record of §6 if old saves are to load, the raw containers of §9.1 (or no game code may call slots 34/39/40/44), the
building map as a game `cISC4SimGrid<uint8>` with keys `4p + w + 1`, the construction node as a `cISC4Demand`
(the lot calls `ReleaseConstructionResources` with the building capacity), and the advisor event if the
CityPlanning advisor is to fire. The static catalogue can be rebuilt by the replacement from the same exemplars.

---

## 10. Quirks, open questions, corrections

### 10.1 Quirks (reproduced unless marked *[choice]*)

1. The growth-plan path (`PlanGrowth` `0x00699440`, `ExecuteGrowthPlan` `0x00699270`, the planned-growth map and
   retry days) is dead: its flag is never set. Its map is still saved (always empty).
2. Grow's 3 ms spin and timer-based cut-off (§4.4); *[choice]* FreeCity4 uses the daily growth budget of
   §4.4.1 and does not spin.
3. Wasted decline draws for used-up developers (§4.3).
4. Planner `ad < −20` vs executor `ad ≤ −20` (§4.5).
5. `GetBuildingProfile` and `GetBuildingMapKey(type)` insert a zero profile for unknown types.
6. `GetBuildingTypesByFamily` returns 0 for a single building type although it filled the container.
7. The highest-stage advisor event carries no subject (§4.10).
8. `AddNewBuildingOccupant` auto-altitude uses local box coordinates; the PlaceBuilding command builds a position
   with z = −1 (§4.9, §4.12).
9. Tuning min/max are read without type or count checks (*[choice]* per-word fallback to the defaults, §4.13); the
   stage-multiplier index is not bounds-checked (*[choice]* 1.0 outside).
10. `lastSuccessfulBuild` is not saved: after a load every developer's last build is "today".
11. The monthly clamp is unsigned (§4.1).
12. `Read` overwrites an existing developer entry without releasing it (a leak, *[choice]* released), and keeps a
    null entry for an unresolved developer reference, which crashes `Init` (*[choice]* dropped) (§6).
13. Empty tried-tract mark entries are created by lookups and saved (§4.4, §6).

### 10.2 Open questions

1. Q1. *Answered*: the tract developer claims (see §4 construction resources and `tract-developer.md` §4.15).
2. Q2. Complete list of raw consumers of slots 34, 41 and 44 (most callers cache the BDS pointer; a vtable-offset
   scan with data flow is needed).
3. Q3. Meaning of the terrain-change kinds 1–3 in `0x278E6003` data4, and the exact packing of data1/data2.
4. Q4. *Partly answered*: `0x150A` is the landmark group (SDK `BuildingCategoryFlagLandmark`; the game's own scripts
   call it "landmark") and `0x150B` the reward group (SDK `BuildingCategoryFlagReward`), §4.7.1.1. Still open: the
   name of property `0x029244DB` (preview flag *[inferred]*).
5. Q5. The BuildingProfile fields after `capacity` (properties `0x27812836`, `0x499AFA38`) — not needed by growth.
6. Q6. Whether `cISC4Lot::SetState` also claims (it only releases in the code read).

### 10.3 Corrections to other documents

- `docs/subsystems/development.md`: the generator is a file-local `cRZRandom` seeded `cRZTime XOR 0x0C09C0B8`
  (§4.14); each qualifying developer grows once per day (the weighted pick orders the attempts); decline planning is
  monthly and execution at most one success per day; the construction supply is `0.5 × (R + C + I)` clamped
  (shipped 8192–20480), not a per-resident count; Windows field offsets differ from the Mac ones.
- `docs/specs/foundations.md` §6.2: the table lists file-local generators only as `cRZRandomSuper`; the BDS's
  file-local generator is a `cRZRandom` (seed `cRZTime() XOR 0x0C09C0B8`, `0x00A74B40`).
- `docs/specs/foundations.md` §17.8: add the SDK return-type errors of §7.1 (slots 6/7/47 pointer, 17/18/19/21/27
  counts, 42/43 AL, 45 ST0 float).

---

## 11. Conformance tests (must exist in `engine/conformance`)

1. **Developer types**: `GetDeveloperType(1, 2)` = `0x1020`; `(3, 1)` = 0; `(3, 3)` = `0x3330`; `(5, 0)` = `0x4100`;
   `(4, 2)` = 0. `GetWealth(0x4300)` = 2; `GetWealth(0x3320)` = 2; `GetWealth(0x1234)` = 0.
2. **Map keys**: `GetBuildingMapKey(1, 1)` = 6; `(3, 3)` = 16; `(8, 3)` = 36.
3. **Zone fits**: `BuildingFitsInZone(1, 3)` true; `(3, 1)` false; `(0, 0x0B)` true; `(8, 9)` true; `(4, 2)` false.
   `GrowthStageFitsInZone(4, 1)` false; `(6, 5)` true; `(15, 3)` true; `(1, 0)` false.
4. **Monthly supply**: R = 30000, C = 12000.5, I = 9000.25, shipped tuning → x = 25500.375, clamped to 20480 →
   `SetSupplyValue(20480.0)`. R = 1000, C = 200, I = 50 → x = 625 → 8192.0. Code-default tuning (no exemplar),
   R = 3000, C = I = 0 → 1500.0.
5. **Claim/Release**: supply 20480, demand 20400: `Claim(100)` false (t = 20500), state unchanged; `Claim(80)` true,
   demand 20480; `GetFreeConstructionResources()` 0; `Release(80)` → demand 20400, returns true;
   `GetFree` = 80. Supply 8192, demand 9000 → `GetFree` = 0, `GetTotal` = 8192.
6. **Grow order (seeded)**: generator seed 12345 (`0x3039`; NextRaw = `0x0C63D3DC`, `0xFED6A704`, `0x6E80D665`),
   bias 10, developers ascending with active demand: `0x1010` 100.0, `0x1020` 3.0, `0x3110` 40.0, `0x4300` 250.0,
   all lists valid and non-empty. Day: draw 1 `RandomUint32Uniform(420)` = 20 → `0x1010`; draw 2
   `RandomUint32Uniform(310)` = 308 → (308 − 50 = 258 < 260) `0x4300`; draw 3 `RandomUint32Uniform(50)` = 21 →
   `0x3110`. Tract-developer `Grow` call order: `0x1010`, `0x4300`, `0x3110`; `0x1020` gets no call; exactly 3
   generator steps. With seed 1 the same day gives draws 0 (of 420), 108 (of 310), 37 (of 50) and the same order.
   With `growthAttemptsPerDay` = 2 (seed 12345): calls `0x1010`, `0x4300` only; exactly 2 generator steps; the
   next NextRaw is `0x6E80D665`. With budget 0 (unlimited) or 3 or more: as above.
7. **Tried-tract marks**: growth list [(4, 4), (8, 4), (12, 4)], one qualifying developer: days 1–3 try (4, 4),
   (8, 4), (12, 4); day 4 clears the marks and tries (4, 4) again. Save after day 2 and load in a fresh process:
   day 3 tries (12, 4).
8. **Decline plan (seeded)**: seed 99 (NextRaw `0x00196FB0`, `0x565A18BD`, `0x5B7457A4`, `0x9064E2E8`,
   `0xBD542D09`), bias 10, developers: `0x1010` ad −50 (purpose 1, decline list [(4, 8), (12, 8)]), `0x3110` ad −30
   (purpose 2, [(20, 4)]), `0x4100` ad −90 (purpose 5, [(0, 0)]), `0x4200` ad −10 (purpose 6, [(1, 1)]). Total 100.
   Draws: 0 → `0x1010` (4, 8); 33 → `0x1010` (12, 8); 35 → `0x1010` (used up, wasted); 56 → `0x1010` (wasted);
   73 → (73 − 60 = 13 < 40) `0x3110` (20, 4). Plan: (4, 8) day now, (12, 8) now + 1, (20, 4) now + 2; 5 steps.
   `0x4100` (agriculture) and `0x4200` (ad > −20) are excluded.
9. **Decline execution**: plan of test 8, now = D. Day D: `Decline((4, 8))` returns 0 → dropped; (12, 8) is due on
   D + 1 → kept; stop. Day D + 1: `Decline((12, 8))` returns 50 → dropped, stop. Day D + 2 with `0x3110` ad now
   −15 → (20, 4) dropped without a call.
10. **Max density**: a 2 × 3 lot (area 6) allowing zone 1 with one residential $ building (purpose 1, wealth 1)
    of capacity 20 that fits: table[(1, 6)] = 3.8333333 (`0x40755555`); `GetMaxCapacity(1, 1, 1)` = 3.0 in ST0.
    Capacity 2 → 0.8333334 → 1.0 → 1.0. Capacity 0 → 0.5 (kept) → 0.0. Capacity 5 on a 1 × 2 lot → 3.0. A second
    configuration giving 2.0 for the same key leaves 3.8333333. Unknown key → 0.0.
11. **Suitability**: building zone 1 on a cell of zone 3, lot present in state 1, powered, lvMin = lvMax = 0,
    land value type = wealth → true. Cell zone 4 → false, reason 0. No lot → reason 3. Unpowered → reason 5; with
    "FightThePower" → passes. lvMin 10, lvMax 20, land value 25 → reason 1. Zone 7 profile with land value type ≠
    wealth and lv range 0/0 → passes. Lot state 2 → reason 4.
12. **Building map**: StateChanged for a 2 × 2 lot at (10, 10)–(11, 11), building purpose 3, data4 = 2, old 1 →
    new 3: the 4 cells become 15. Old 3 → new 4: cells 0. Old 2 → new 3: no change.
13. **Highest stage**: first occupied stage-3 lot of `0x1020` → highestStage 3, one PushEvent(`0x4A64A9CC`, null)
    to advisor `0x17`; a later stage-3 lot → no event; stage 4 → event, value 4. Building wealth ≠ data4 → no event.
14. **Record**: a BDS with developers {`0x1010`: id 5}, no special buildings, highestStage {`0x1010`: 2}, marks
    {`0x1010`: {(4, 8)}}, cheats (0, 1, 0), grid id 7, node id 9, writes (ids per foundations §2.7 with class IDs):
    `04 00` · `07 00 00 00` + grid class · `09 00 00 00` + node class · `01 00 00 00 10 10 00 00 05 00 00 00 38 C1 90 09` ·
    `00 00 00 00` · `01 00 00 00 10 10 00 00 02` · `01 00 00 00 10 10 00 00 01 00 00 00 04 00 00 00 08 00 00 00` ·
    `00 00 00 00` · `00 01 00`. Reading it back reproduces the state; version 3 → `Read` false after 2 bytes.
15. **Stage multiplier**: shipped tuning, difficulty 1 → 1.1 (`0x3F8CCCCD`); difficulty 3 → 1.0 *[choice]*.
16. **Catalogue queries**: family F = {A, B}: `GetBuildingTypesByFamily(vector, F)` → [A, B], returns 2;
    `(vector, A)` → [A], returns 0; `(vector, X unknown)` → vector unchanged, 0. `IsBuildingInFamily(A, A)` true.
    `GetAllBuildingTypes(array, n = 1)` with 3 types → n = 1, returns 3.
17. **PlopBuilding orientation**: profile {zone 1, wealth 1, purpose 1, lvMin 0, lvMax 200, type T, orientation 3,
    x 10, z 12} → slot 6 is called with `(T, 10, 12, 3)` (not 200); with orientation 0 and lvMax 2 → `(T, 10, 12, 0)`.
18. **Construction-resource tuning shapes** (§4.13): property `0x47E2C380` as uint32 array [8192, 20480] → 8192,
    20480; as uint32 array [5000] → `crMin` 5000, `crMax` 2000 *[choice]*; as an empty array or absent → 100, 2000;
    as uint8 array [0x00, 0x20, 0, 0, 0x00, 0x50, 0, 0] → 8192, 20480; as a single uint32 8192 → `crMin` 8192,
    `crMax` 2000 *[choice]*. With [3000, 1000] and R + C + I = 10000 (factor 0.5) the monthly supply is 1000.0.
19. **Category flags**: occupant groups [`0x1000`, `0x150A`, `0x1000`, `0x9999`] → flags `0x00800001`; [`0x3002`]
    → `0x40000000`; no property → 0; [`0x1303`] → 0.
20. **Key-map order**: building exemplars with instances 5, 198, 391 (in that enumeration order) →
    `GetAllBuildingTypes` = [391, 198, 5]. Instances 1, 75078, then 2, 3, …, 192 (193 types; 193 buckets) →
    [75078, 1, 2, …, 192]; one more instance 1000 resizes to 389 buckets before its insertion → [1, 75078, 2, …,
    192, 1000]. Loading instance 5 twice (second exemplar key K2) keeps one entry with key K2 and does not grow.
    `GetBuildingProfile(77)` for an unknown 77 → zero profile; afterwards `IsBuildingTypeValid(77)` is false and
    `GetAllBuildingTypes` is unchanged.
21. **Max density, unknown ID**: a lot configuration whose object lists only ID `0xDEAD` (neither a family nor a
    building type) adds nothing to the table; `IsBuildingTypeValid(0xDEAD)` stays false and no profile is created.
22. **Empty marks entry**: one qualifying developer `0x1010` with valid lists and an **empty** growth list: after
    one day no `Grow` call happens, and the record's marks section is `01 00 00 00 10 10 00 00 00 00 00 00` (one
    entry, 0 cells). A developer with invalid lists (or `ad ≤ 5`) creates no entry.
23. **Shutdown before Init**: a constructed BDS whose `Shutdown` is called first returns true, releases nothing and
    leaves the process-wide marks of a previous city intact; a following `Init` then works normally.
24. **AddNewBuildingOccupant insert failure**: valid type and location, occupant manager `InsertOccupant` returns
    false → slot 7 returns null; the created occupant got exactly one AddRef and one Release from the BDS.
25. **Terrain change with no lots**: kind 1 on a rectangle without lots → `DemolishLots` is called once with an empty
    list (or skipped, *[choice]*); funds unchanged.

## 12. Calibration (counters for the observer in the original)

Purpose: set the default of `growthAttemptsPerDay` (§4.4.1) from the original's behaviour on the reference machine.
The observer logs, **per game day** (one `DoDailySimulation` call, `0x0069B6D0`; `Grow` is called once from it at
`0x0069B6FD` and has no other caller):

| Counter | Where to count in the original | Meaning |
|---|---|---|
| `attempts` | calls to the tract developer's `Grow` (slot 5): the virtual call at `0x0069B394` inside `Grow` `0x0069B0B0` (equivalently, entries of `cSC4TractDeveloper::Grow` `0x0070D130`, which has no other caller) | units of §4.4.1 done that day |
| `draws` | calls to `RandomUint32Uniform` at `0x0069B1C7` | picks that day (attempts + picks without a call) |
| `qualifying` | at entry of `Grow`: the number of developers in the developer map (slot 34) with `trunc(active demand) > 5` (§4.3) | attempts the day would have with an unlimited budget |
| `cut` | `attempts + (picks without a call) < qualifying` at the exit of `Grow` | the 3 ms timer stopped the loop |
| `elapsed` | rdtsc at `Grow` entry and exit, and at each `0x0069B394` call and return | time per attempt (the observer can compute how many attempts fit in 3 ms) |

The default is the distribution of `attempts` on days with `cut` = true (when no day is cut on the reference
machine, the budget is unlimited). Log the city size and population with each day; attempt cost grows with the
number of candidate lots.
