# MySims (`cSC4MySimAgentSimulator`, `cSC4MySim`) — simulation-side behaviour spec

Status: round 3, written from the Windows 1.1.641 executable. The named Mac build was used for names and
intent only; every behaviour, constant and slot below was checked on Windows unless marked *[inferred]*.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs and strings (foundations §2), class store (§3), city
component table (§3.6), Init order (§3.7), messages (§5), random generators (§6), slot-order rule (§16),
x87 precision (§18); the advisor system, advisors, advices, the event queue, the Lua `game`/`mysim_*` functions
and the daily/monthly ticker (`docs/specs/advisors.md`, written separately); residential EQ/HQ, age and grade data
(`residential.md`, `residential-education-health.md`); grids (`simgrid.md`); the mayor rating (`aura.md`).

Window procs, the pick-sim and move-in input controls, balloon rendering, camera following, sounds and
U-Drive-It mission definitions belong to their UI/automata owners. The dispatch's simulation state,
destination queue, persistence and player-trip entry points belong here (§4.6.1, §6.2.1, §7.2.1).
The multimodal handoff rules are in §4.6.3–§4.6.5 (MS18); they do not permit immediate-arrival stubs.

---

## 1. Role and identity

**Summary.** A MySim is a named resident the player places in a house. It is an **advisor** (it derives from
`cSC4Advisor`) whose advices are the MySim's opinions ("thought balloons"), written in Lua. The engine side of a
MySim:

- keeps the MySim's home, job, age, wealth, generation and a few counters,
- copies values from the city simulators into the MySim's **Lua table** (§4.4–§4.6), which the advice scripts read,
- reacts to building, disaster and clock messages by pushing **advisor events** (§5.2),
- runs the life events that the advice scripts request through **script commands** (die, retire, spawn, …, §4.8).

**MySims do not feed back into any simulation result** *[compat]* (§9.1). Nothing they do changes population,
demand, budget, land value, the mayor rating or any grid. They are purely observational. Their only outputs are
Lua variables, advisor events, UI/audio messages and automata.

### 1.1 Classes

| Class | CLSID | Interfaces (IID) | Role | Spec |
|---|---|---|---|---|
| `cSC4MySimAgentSimulator` | `0x29E37DEC` | `cISC4MySimAgentSimulator` `0xA9E37EBB` (also answers `1`), `cIGZMessageTarget2` `0x452294AA`, `cIGZSerializable` `0xE4FDA3D4` | per-city dispatcher of messages to the MySims | this spec |
| `cSC4MySim` | `0x4A1DBBBF` | `cISC4MySim` `0x0A1DA42F`, private class pointer `0x8BC01FFE` *[inferred: returns the object itself, AddRef'd]*, plus everything `cSC4Advisor` answers (`cISC4Advisor` `0xCA09F544`, `cIGZSerializable`, `cIGZMessageTarget2`, `1`) | one MySim | this spec |
| `cSC4MyStreetSim` | `0x0BF606A5` | answers `cISC4MySim` `0x0A1DA42F` | the "My Street" sim (advisor id 30) | out of scope; it shares the agent's sixth slot (§1.3) |
| `cSC4MySimDispatch` | `0xCBC14674` | `0xEB8B6F9A`, message target `0x452294AA`, serializable `0xE4FDA3D4` | per-MySim trip controller (path, automaton, commute destination) | this spec, §4.6.1, §4.7, §6.2.1, §7.2.1; remaining handoffs MS18 |
| `cSC4MySimPool` | — | — | imports Sim templates (names, portraits) for the pick-sim UI | **app/UI**; contract in §4.12 |
| `cMySimBuildingFilter` | — | `cISC4OccupantFilter` | building query filter | this spec, §4.9 |
| `cSC4AdviceSubject` | `0x6A26D049` | `0x6A26D050` | event subject | advisors.md |

Every interface of one object shares one reference count. `cSC4MySim`'s release path is the advisor's (advisors.md).
The agent's `Release` at count ≤ 1 calls `Shutdown()` and then the destructor; it returns 0 then, otherwise the
new count.

### 1.2 Creation, ownership and persistence owner

- **The agent simulator is a per-city component.** `cSC4City::CreateCityComponents` creates it with
  `GetClassObject(0x29E37DEC, 0xA9E37EBB)`. The city exposes it through `cISC4City` slot 71 (`+0x11C`)
  `GetMySimAgentSimulator`. It is **not** one of the city's saved components (foundations §3.6): the city's
  Read/Write never touch it, and nothing writes a reference to it *[compat]*.
- Init order: the agent is initialised after the advisor system, the city situation manager and the tutorial
  system, and before the city detail manager (foundations §3.7). The city releases it in its Shutdown.
- **The MySims are children of the advisor system.** The advisor system creates the MySim advisors from the Lua
  `advisors` table (ids 1–5, class `0x4A1DBBBF`) and the My Street sim (id 30, class `0x0BF606A5`), keeps them in
  its advisor map, and **saves them** (its record holds a ref plus the advisor id for each advisor; advisors.md
  §11). On load the class store recreates them from those refs. The agent only holds extra references.
- The agent's `Init` rebuilds its slot vector from the advisor system every time (§1.3). So a replacement agent
  needs no save data.

### 1.3 The agent's slot vector

The agent holds a vector of **6** `cISC4MySim` pointers (AddRef'd), all null at construction.

`Init` (once; a flag guards it):

1. Take the city (AddRef) and its advisor system (`+0x144`, AddRef).
2. Walk the advisor system's advisor map in its own order (by advisor id *[inferred]*). For each advisor that
   answers `QueryInterface(0x0A1DA42F)`:
   - advisor id (advisor slot 5) = **30** → store it at index **5**;
   - otherwise → store it at the next index, starting at 0.
3. Subscribe to the messages of §3.2 in the order listed there.
4. Register two Lua functions in the game data registry (service `0xEBE0E860`/`0xE898ED03`): `get_sim`
   (returns the MySim at the registry's current index) and `get_count` (returns the vector's size, 6). These are
   used by the Lua scripts; they are not simulation.

*[quirk]* The running index has no bound. With the shipped table it receives ids 1–5 → indices 0–4. A sixth
non-30 MySim advisor (a plugin) would overwrite index 5 and a seventh would write past the vector (undefined).
*[choice]* FreeCity4 ignores MySim advisors beyond index 4.

Most agent loops visit **indices 0–4 only**. Index 5 (My Street sim) is visited only by `TrackLocation` and
`HideLocation` and is reachable through `GetActiveUIPanelSim`.

---

## 2. State

### 2.1 Agent simulator

| Field | Initial | Persisted |
|---|---|---|
| initialised flag | false | no |
| slot vector (6 refs) | nulls | no (rebuilt in `Init`) |
| last disaster type | 0 | no |
| city, message server | null | no |
| active UI panel index | −1 | no |

### 2.2 One MySim

"Index" below means the MySim's own record, not memory layout.

| Field | Initial | Persisted | Notes |
|---|---|---|---|
| state | 0 | yes (u8) | 0 none (slot empty), 1 moving in, 2 at home, 3 at work, 4 travelling to work, 5 travelling home, 6 travelling elsewhere (a dispatch-tool pylon or other target). Set by `cSC4MySim::DoMessage` on dispatch notifications (§3.3): **trip start** (sub-id `0x4BB71A7E`, `0x0042D364`): destination = default destination (home) → 5 unless the state is 1 (a move-in keeps 1); else destination = commute destination → 4, otherwise 6. **Arrival** (`0x0042D5B3`): current location = home → state 2 (when it was 1, `AnnounceMoveIn` is called right after state := 2); = commute destination → 3; elsewhere unchanged. `DispatchToolLockPylon` sets 6 when the dispatch accepts the pylon `GoTo`; `QuitJob` sets 2 unless 5 |
| slot | 0 | yes | the UI slot passed to `MoveIn` |
| residence | null | ref | `cISC4BuildingOccupant` |
| home lot | null | no | looked up from the residence |
| home cell x, z | 0 | yes | from the home lot's location |
| home position x, y, z (metres) | 0 | yes (3 f32) | used by every distance (§4.10) |
| job | null | ref | `cISC4BuildingOccupant` in this city |
| neighbour job city | null | **no** | `cISC4RegionalCity`; re-resolved by `Init` (§2.2.1) from the saved neighbour city cell. "Has a neighbour job" everywhere in this spec means this reference is non-null; there is no separate flag |
| neighbour city cell x, z; commute destination cell x, z | 0 | yes (4 s32) | |
| jobless days | 0 | yes | |
| vehicle key | 0 | yes (res key) | |
| wealth | 0 | yes (u8) | 1..3 |
| generation | 0 | yes (u16) | 1 at move-in, +1 per Spawn |
| zodiac sign, is male, bio, name | — | sign u8, bool, bio GZStr; the name is the advisor's personal name (advisor record) | |
| age | 0 | yes (u8) | years |
| birth month | 0 | yes (u8) | 0..11 |
| portrait bitmap id and path | — | yes | UI |
| last commute time | 0 | yes | minutes, integer |
| retired | false | yes | |
| last local disaster | 0 | yes | disaster type ID that destroyed home or job, or 0 |
| EQ, HQ (current) | 0.0 | **no** | recomputed by `UpdateVariables` |
| EQ ring index, EQ 6-month delta, EQ ring[6] | 0 | yes | float32 |
| HQ ring index, HQ delta, HQ ring[6] | 0 | yes | float32 |
| car key, pedestrian key | 0 | yes (res keys) | automata |
| reserved s32 pair | 0 | yes (version ≥ 17) | **dead data**: zeroed by the constructor, written and read, never otherwise used by any MySim method (all 69 scanned; Windows ctor `0x00429A9E`) — carry the read values and write them back |
| dispatch | null | ref (`0xEB8B6F9A`) | automata |
| pylon timer, dispatch tool | — | yes | UI |
| pylon locked | false | yes (version ≥ 14) | set to the argument of `DispatchToolLockPylon` (`0x0042728E`); cleared by `DispatchToolDestroyPylon`, `LeaveCity`, `DispatchToolBegin`, `DeleteFromGame`, `Shutdown`; while false `is_pylon_location` is false, and `DispatchToolEnd` (`0x004264C0`) destroys the pylon of a still-active dispatch tool; the pylon timer (§3.3) and `UpdateTripVariables` (§4.6) only act on a locked pylon |
| last player trip day | 0 | yes (s32, version ≥ 16) | |
| commute check pending | false | no | set by neighbour-connection removal and by `Init` (§2.2.1) |
| last trip-update day | 0 | no | |
| local building distances (map occupant group → metres) | empty | no | §4.9 |
| per-MySim RNG | `cRZRandom`, seeded from the wall clock (`0xFFFFFFFF`) at construction | no | §8 |
| Lua table handle | 0 | no | created on first use |

#### 2.2.1 Neighbour job city at `Init` (`0x0042CCF0`, Windows `0x0042CEF3`–`0x0042CF44`)

Only when state ≠ 0, after the dispatch is created and the home lot looked up:

- If the MySim has **no job** and the saved neighbour city cell is not `(0, 0)` (x ≠ 0 **or** z ≠ 0): ask the
  region (`cISC4App` slot 35 `+0x8C`, then `cISC4Region::GetCity(x, z)` slot 11 `+0x2C`) for the city at that
  cell; if one is returned, it becomes the neighbour job city (a null result leaves the current reference as it is,
  which after a load is null). In both cases the **commute-check flag is set**, so the next daily step runs
  `CheckCommute` (§4.2 step 2, §4.7), which drops a neighbour job that no longer resolves or connects.
- Otherwise (the MySim has a job, or the cell is `(0, 0)`): the neighbour job city reference is released.

*[quirk]* A neighbour city at region cell `(0, 0)` is never re-resolved, so a MySim working there loses that
neighbour job on every load.

---

## 3. Scheduling and messages

### 3.1 No simulator agent

Neither class registers with the simulator scheduler (simulator.md): there are **no agent levels and no
finished/unfinished semantics**. MySims are driven by:

- the **advisor system's ticker** (advisors.md §5.1), which calls each advisor's `Simulate(kind)`: daily (kind 0)
  and, on day 28 of the ticker's date, monthly (kind 1) before the daily call. `cSC4Advisor::Simulate` dispatches to
  the class's `simulate_day` / `simulate_month` (§4.2, §4.3);
- **messages** forwarded by the agent (§3.2) and the MySim's own subscriptions (§3.3);
- **advices** whose `command` field asks for a life event (§4.8).

**Ordering.** The ticker is a self-posted advisor-system message, so MySim updates run **between** simulator
ticks and read whatever the simulators last published (residential EQ/HQ, grids, budget). There is no defined
order relative to a particular simulator's month *[compat]*: a replacement must not assume one.

### 3.2 Agent message handling (`cIGZMessageTarget2::DoMessage`, always returns true)

Subscriptions, in `Init` order: `0x99EF1142`, `0x99EF1145`, `0x99EF1143`, `0xA6B79602`, `0xA6B79612`, `0x44778230`,
`0x8A09FA6C`, `0xCA1B1E64`, `0xEA1F0DCD`, `0x0A36F1E9`, `0xEA0D163E`. `Shutdown` removes them.
"For each MySim" means indices 0–4, skipping nulls, in index order.

| Message | Fields read | Action |
|---|---|---|
| `0x99EF1142` occupant inserted | data3 = occupant type, void1 = occupant | type `0x278128A0` (building) → QI `0x87DDA3A9`, each MySim `DoBuildingAdded(b)`. Type `0x69D25C5F` (fire) → occupant position, each MySim `DoFireAdded(x, z)`. **Then** the changed handling below runs too (building → `DoBuildingChanged`). |
| `0x99EF1145` occupant changed | data3, void1 | building → each MySim `DoBuildingChanged(b)` |
| `0x99EF1143` occupant removed | data3, void1, data4 | building only. `cause := lastDisasterType` if `data4 = 4` **and** the city's disaster layer `IsADisasterOngoing()` is true; otherwise 0. Each MySim `DoBuildingRemoved(b, cause)` |
| `0xCA1B1E64` disaster period started | data2, data3, data4 | `lastDisasterType := data4`; each MySim `DoDisaster(data4, data2, data3)` |
| `0xEA1F0DCD` pipe burst | data1, data2 (cell) | `cISC4City::CellCenterToPosition(data1, data2)` → each MySim `DoPipeBurst(x, z)` |
| `0x0A36F1E9` tornado | data2, data3 (cell) | cell centre → each MySim `DoTornado(x, z)` |
| `0xEA0D163E` neighbour connection removed (posted by the neighbours simulator) | data2, data3 | each MySim `DoNeighborConnectionRemoved(data2, data3)` |
| `0x44778230` hour changed (24-hour clock) | data2 = hour | 8 → `LeaveForWork`; 17 → `ReturnHome`; 22 → `PlayNightSound` (automata/audio) |
| `0x8A09FA6C` AdviceCommand (advisors.md §6.4) | IGZUnknown = advisor, data1 = command | if the advisor answers `0x0A1DA42F`: that MySim's `DoScriptCommand(data1)` |
| `0xA6B79602`, `0xA6B79612` view changes | — | each MySim `UpdateResidence` (visual) |
| `0x230E27AC` cheat, data1 `0xEB6F3C97` | cheat text | debug only: posts `0x0B6F3E27` / `0xAB6F3E2D` / `0x4B6F3EBF` / `0x0BB04126` with data1 = a hex argument (print, new job, new house, dispatch) |

### 3.3 MySim's own messages (`cSC4MySim::DoMessage`)

Dispatch notifications are **synchronous owner calls**, not broadcasts or queued message-server posts.
They use type `0x2BA06B99`, data1 = notification ID, data2/data3 = old/new values for state or flag changes,
data4 = 0 in the transitions specified here. No owner means no delivery. State and flags have already
changed when the owner runs (`0x00431A30`, `0x00431BC0`, `0x00431C10`). Do not coalesce successive
flag changes: each changed mask is separately observable. Notification production is specified in §4.6.1.

The MySim subscribes (at its Init) to `0x0B6F3E27`, `0xAB6F3E2D`, `0x4B6F3EBF`, `0x0BB04126`, `0x4BC42C48`,
`0x26D31EC1`, and to the automata manager's broadcast. Simulation-relevant ones:

| Message | Action |
|---|---|
| `0x4BC42C48` MySim update, IGZUnknown = this MySim | `UpdateVariables` (§4.5); balloons (UI) |
| `0x2BA06B99` dispatch notification, data1 = sub-id | `0x4BB2D020` / `0x4BB2CFC0` → `UpdateTripVariables` with the new/old trip state (§4.6). `0xEBA06BF4` → **job found** (§4.7). `0xEBA06C12` → arrival: if the destination is still "by commute", `FailCommute` (§4.7); if the dispatch reports "home reached" and state is 1 → state 2 and announce move-in. `0xEBA06BD4` / `0x4BB71A7E` → state transitions (1/2/3/4/5/6) and posts `0x4BC42C48`. Others: visual. |
| `0x26D31EC1` | hide markers (UI) *[inferred: city shutdown notice]* |
| `0x86AD10EE` | balloons every > 2 s of wall time (UI) |
| others | `cSC4Advisor::DoMessage` |

---

## 4. Algorithms *[compat]*

### 4.1 Move-in

1. The UI calls the agent's `MoveIn(name, isMale, sign, bio, bitmapID, bitmapPath, slot)`. The agent picks the
   **first** MySim among indices 0–4 that is non-null and not active (`IsActive` = state ≠ 0) and calls its
   `MoveIn(slot, name, isMale, sign, bio, bitmapID, bitmapPath)`. If all five are active, nothing happens. The
   `slot` argument is only stored; it does not choose the MySim *[compat]*.
2. `MySim::MoveIn` stores the data, sets generation := 1, birth month := advisor month counter mod 12,
   last player trip day := 0, and starts the move-in input control (UI: the player clicks a house).
3. The input control accepts a building with `TestForResidence` (§4.7) and calls `SetResidence(b)` *[inferred:
   the control's exact arguments were not traced; UI]*.

**`SetResidence(b)`** (slot 13):

1. Store the residence; look up its lot (lot manager `GetOccupantLot`) and the lot's cell location.
2. If the state was 0 (a new MySim):
   - `ResetAdvisorState` (advisor Shutdown + Init with the same id, keeping the personal name; balloons rebuilt);
   - age := residential **average-age grid** value at the home cell (`GetAverageAgeMap` → `GetCellValue`), then
     `max(age, 18)` *[compat]*;
   - state := 1; wealth := the home lot's `GetOccupantWealth()`;
   - EQ := residential `GetCellEQByWealth(x, z, wealth)`, HQ := `GetCellHQByWealth(x, z, wealth)` (residential.md
     §10.1 slots 37/38, float32 in ST0); both 6-entry rings are filled with these values, deltas 0;
   - post `MySimsChanged` `0xAA25827C`;
   - `FindNewJob()` (§4.7); start the commute (dispatch); set the dispatch flag.
3. Otherwise: the "tracking" flag := dispatch `CheckFlags(0x100)`.
4. `UpdateVariables()` (§4.5).

The **first arrival home** (dispatch notification, state 1 → 2) calls `AnnounceMoveIn` (slot 40): if `RequestToPlaySound()` (slot 56, `0x00424DE0`) returns true, post `MySimMovedIn` `0xEA3ADA33` (void1 = home occupant,
data1 = is-male, data2 = wealth); then `UpdateVariables` and push the advisor event `MYSIM_MOVE_IN` `0xCA3E87A8`.
`RequestToPlaySound` is a sound throttle, **not** a reload test: it is true only when state ≠ 0 and the MySim's
wall-clock timer (`cRZTimer` in seconds, units 5, constructed with the MySim at `+0x2A0`) shows more than 4 whole seconds since the
last time it returned true (initially 0; unsigned difference); when true it stores the current reading. So a
move-in within 4 s of the previous announcement (or of the MySim's construction) posts no `MySimMovedIn`; the
`UpdateVariables` and the `MYSIM_MOVE_IN` event happen in every case (`0x0042BFF2`).

### 4.2 Daily step (`simulate_day`, advisor `Simulate(0)`)

Skipped entirely when state = 0, or while **a modal window is running**: the window manager (framework service
`0xA417445E`, `cIGZWinMgr` IID `0x5A4`) slot 42 (`+0xA8`, `0x009DB025`) returns true when its modal depth (the
counter `DoModalWin` increments around its loop) is not 0 — the same test as advisors.md §6 popup gate. The monthly
step (§4.3) makes the same check. Otherwise, in order:

1. **Job search.** If there is no job, no neighbour job, the MySim is **not retired**, and state = 2 (at home):
   `FindNewJob()`; if it returns false, jobless days += 1.
2. If the commute-check flag is set: `CheckCommute()` (§4.7), clear the flag.
3. `ActivateLuaContext()` (sets the Lua global `mysim` to this MySim's table, §4.4).
4. Balloons (UI). `cSC4Advisor::simulate_day` (advice evaluation, advisors.md §6).
5. If the advisor day counter is greater than the last trip-update day: store it and
   `UpdateTripVariables(−1, −1, −1, −1, false)` (§4.6).
6. Dispatch-tool pylon timeout (UI).

`simulate_day_events` (end of the advisor day pass) only re-queues balloon advices (UI) and then runs the base
event processing (advisors.md §6.3).

### 4.3 Monthly step (`simulate_month`, advisor `Simulate(1)`)

Same skip conditions as §4.2. Then:

1. **Ageing.** If `monthCounter mod 12 = birthMonth` (signed division; the advisor's month counter is incremented
   by `cSC4Advisor::Simulate(1)` before this call): age += 1 (u8, wraps at 256).
2. If the MySim has a job or a neighbour job: `CheckCommute()`.
3. `UpdateVariables()`.
4. **6-month EQ/HQ change** (float32, §18):
   `eqDelta := EQ − eqRing[i]; eqRing[i] := EQ; i := (i + 1) mod 6`, and the same for HQ with its own index.
   The ring holds the values of the last six monthly steps, so `delta_eq` is the change over six months.
5. `ActivateLuaContext()`. The base `simulate_month` is empty on Windows.

There is **no** death, retirement or wealth rule in the engine: those come from advice scripts (§4.8).

### 4.4 The Lua table

Each MySim has a table in the advisor system's Lua state, created on first use (`CreateLuaTable`, which also
forces a full `UpdateTripVariables(…, true)`). `ActivateLuaContext` (slot 68) pushes the table as the global
**`mysim`** and returns true (false if there is no Lua state). The advice scripts (`adv_mysim*.lua`, consumers)
read the fields below as `mysim.<name>`. Every value is stored as a Lua number unless noted; "int(x)" is the CRT
`_ftol` truncation toward zero. When a source is missing the field is **not written** and keeps its previous
value *[quirk]*.

### 4.5 `UpdateVariables` (full refresh) and `UpdateQuickVariables`

**Quick** (also called at the start of the full refresh):

| Field | Value |
|---|---|
| `state`, `age`, `wealth`, `generation`, `retired`, `zodiac`, `is_male`, `last_local_disaster` | the MySim's fields |
| `home_location`, `job_location` | building subjects (Lua helper `SetTableBuilding`) |
| `name` | the advisor's personal name |

**Full** (only when the Lua table exists; home fields only when there is a residence):

| Field | Source *[compat]* |
|---|---|
| `eq`, `hq` | `EQ := GetCellEQByWealth(homeX, homeZ, wealth)`, kept only if `> 0.0` (else the previous EQ stays); field = int(EQ). Same for HQ (`GetCellHQByWealth`). |
| `delta_eq`, `delta_hq` | int(eqDelta), int(hqDelta) (§4.3) |
| `le` | residential `GetCellLifeExpectancy(homeX, homeZ)` (uint8, residential.md slot 29) |
| `home_condition` | home lot `GetCondition()` (byte), 0 without a lot |
| `home_building` | the residence's building name (string) |
| `home_has_road` | home lot `HasRoadAccess()` as 0/1 |
| `home_has_power` | power simulator `IsOccupantPowered(home)` (slot 31) |
| `home_has_water` | plumbing simulator `IsBuildingWatered(home)` (slot 9) |
| `local_crime` | crime grid `GetCellValue(homeX, homeZ)` (uint8) |
| `home_police_coverage`, `home_fire_coverage` | police / fire coverage grid (`+0x14` of each simulator) `GetAverageValueInCellRect(x0, z0, x1 − x0 + 1, z1 − z0 + 1)` over the home lot's cell rectangle (uint8) |
| `local_air_pollution`, `local_water_pollution`, `local_garbage`, `local_radiation` | pollution simulator `GetPollutionGrid(0..3)` → `GetCellValue(homeX, homeZ)` (int16, signed) |
| `last_player_trip_day` | the MySim's field |
| `desirability` | wealth 1/2/3 → building development simulator `GetDeveloper(0x1010 / 0x1020 / 0x1030)` → its desirability grid (developer slot 13) → `GetCellValue(homeX, homeZ)` (int16). Other wealth or no developer → not written. |
| `home_occupant_wealth` | home lot `GetOccupantWealth()` |
| `local_police_funding`, `local_fire_funding` | the **nearest** police / fire station (police `+0x54` / fire `+0x58` station lists; nearest by §4.10 distance, strict `<`, list order) → that station's funding fraction (`+0x1C` / `+0x20`, `SC4Percentage`), field = int(f × 100). No station → 0 (`kSC4PercentageZero`). |
| `local_traffic_congestion` | the home lot's bounding rect (`cISC4Lot` slot 14, inclusive cells minX, minZ, maxX, maxZ) grown by 1 cell on each side; minX and minZ clamped up to 0, maxX **and maxZ** clamped down to `CellCountX − 1` (`cISC4City` slot 93 for both, *[quirk]*, invisible in square cities). Only the **border ring** of that rect is visited, not its interior: for x = minX..maxX the cells (x, maxZ) then (x, minZ); for z = minZ+1..maxZ−1 the cells (minX, z) then (maxX, z). A cell counts only if the traffic simulator's `GetCapacity(1, x, z)` (slot 23) is not 0.0; the value is the u8 maximum of the congestion map (slot 10, map slot 7 `Get(x, z)`) over counted cells, 0 if none (Windows `0x0042B3DE`–`0x0042B581`) |
| `job_title`, `job_building`, `job_city` | strings (`GetJobTitle`, the job building's name, the neighbour city's name) |
| `last_commute_time` | minutes (§4.7) |
| `employed`, `unemployed_days` | 1 if job or neighbour job; jobless days |
| `job_is_neighbor`, `job_condition`, `job_has_power`, `job_has_water` | for a city job: 0; the job lot's condition (`cISC4Lot` slot 35, 0 when the job has no lot); power as for home; water: **1 when the job building's profile purpose is 5 (agriculture)** (BDS `GetBuildingProfile`, `0x0042BA92`), else as for home. For a neighbour job: 1, 0, 1, 1 (constants, `0x0042BB54`–`0x0042BC0F`) |

`UpdateLocalBuildings` (§4.9) also runs inside the full refresh.

### 4.6 `UpdateTripVariables(newTripState, oldTripState, newFlags, oldFlags, force)`

Runs only when the Lua table exists, its handle is > 0 and **MySim state ≠ 0** (`IsActive`, cISC4MySim slot 20;
`0x00427779`–`0x004277B0`, implementation `0x00449550`, answers MS15 d). The guards run in that order,
before any Lua or dispatch sampling. It selects the
MySim's table and writes. Normal completion restores the saved Lua stack top (`0x0042850F`–`0x00428522`).
If the traffic/pollution/crime service guard below fails, the early exit releases the local location copy
but **skips that stack-restoration call** (`0x00427CBD`/`0x00427CC5`/`0x00427CD1` → `0x00428571`) *[quirk]*.

- If the trip state changed or `force`: `trip_in_transit` := (new = 2), `trip_at_location` := (new = 3). On
  arrival (new = 3) the MySim's state follows the dispatch: at the default destination (home) → state 2 (and
  `AnnounceMoveIn` if the state was 1); at the commute destination → state 3. Then `UpdateQuickVariables`.
  `destination_subject` uses the **new trip-state argument**, not the dispatch's current state (MS15 e):
  new = 2 → occupant at `GetCurrentDestination`; new = 3 → occupant at `GetCurrentLocation` when that location
  is neither home nor work, otherwise null. New states other than 2/3 **leave the previous subject untouched**,
  including a forced refresh with new = old = −1 (`0x00427980`–`0x00427A39`). The subject assignment follows
  arrival handling and its quick-variable refresh.
- If the flags changed beyond bit `0x100`, or `force`: `trip_at_transit_switch` (flag `0x10`), `trip_idle` (flag 1);
  if bit 15 of the flags went from clear to set, push event `MYSIM_APPROACHING_LOCATION` `0xEC107DF9`; the
  travel-type flags `trip_on_bus` (2), `trip_on_foot` (0), `trip_on_subway` (6), `trip_in_car` (1),
  `trip_on_train` (3), `trip_on_elevated_train` (7), `trip_on_monorail` (8), and `last_trip_type` (the matching
  number, 255 when none matches).
- **Local area.** Copy the dispatch's current location (holding its occupant reference); with `force`,
  `SetCellPosition(0, 0)` changes this **local copy's cells and world position** to zero before sampling
  (`0x00427C3E`–`0x00427C60`). The dispatch itself is unchanged. `half := LOCAL_AREA_SIZE / 2` (integer);
  the square is that copy's cell ± half.
- Only if the traffic, pollution and crime simulators all exist:
  - if the dispatch state is 2 (travelling) or `force`:
    - if not idle or `force`: `trip_traffic_congestion` (congestion map value, uint8), `trip_time_ratio`
      (traffic `GetTravelTimeRatio(cellX, cellZ, travelType)` as int; travel type 0xFE/0xFF → 1),
      `trip_wait_length` (dispatch transit-switch wait, minutes);
    - `trip_pollution_air/water/garbage` (pollution grids 0–2, int16), `trip_crime` (crime grid, uint8),
      **`trip_mayor_rating`** (aura simulator `GetAuraGrid()` value, **int8, signed**; aura.md), `trip_eq` /
      `trip_hq` (residential `GetAverageEQGrid` / `GetAverageHQGrid` values as int).
    **Congestion uses the local copy's world x/z**, not its cells: `PositionToTract` slot 19 `+0x4C`, then
    `GetTractValue` slot 22 `+0x58` (`0x00427D1C`–`0x00427D4A`, MS15 b). **Travel-time ratio uses that same
    copy's cells**, including (0, 0) with force (`0x00427D85`–`0x00427D96`, MS15 c). These are distinct units.
    All other grid reads also convert the local copy's world position with `PositionToTract` and read with `GetTractValue`
    (simgrid §4 slots 19/22, **no bounds check** *[quirk]*).
  - if `force`, dispatch state 3 (at a location), on foot, in a car, or idle: query the lots in the square
    (lot manager `GetLotsInCellRect(x0, z0, x1, z1, false)`). For every lot **with a building**, count it (n) and:
    - condition ≥ 4 → distress3; = 3 → distress2; = 2 → distress1; 0–1 → none;
    - lot occupant wealth ≥ 3 → high; = 2 → medium; = 1 → low;
    - the first building in **list order** that is in occupant group `0x150A` (landmark) is remembered.

    Then, float32 (§18): `r := 1.0 / (float(n) + 0.0001)` (the double constants enter at 24-bit precision);
    each percentage field = int(float(count) × r × 100.0): `trip_distress1_p`, `trip_distress2_p`,
    `trip_distress3_p`, `trip_wealth_low_p`, `trip_wealth_med_p`, `trip_wealth_high_p`.
    `trip_landmark_nearby` (0/1), `trip_landmark_name` (string, empty if none), `trip_landmark_funding_p` =
    int(funding × 100) of the landmark's budget item for purpose `0xAA59670C` (budget `GetBudgetItemForPurpose`,
    slot 51; 0 when no landmark; the funding field defaults to 0.0 if the item is missing).
    Disaster flags: walk the disaster layer's registered locations (`GetDisasterLocationCount`,
    `GetDisasterLocationByIndex`); a location inside the square sets `trip_any_disaster_nearby`; its manager type
    `0x621CDA33` sets `trip_fire_nearby`, `0xCA380B06` sets `trip_riot_nearby`; the walk stops early once both are
    set.
- Still inside the "simulators exist" block: if the trip state changed **and the flag block above did not run**,
  post `0x4BC42C48` (MySim update, IGZUnknown = this MySim's `cISC4MySim`). The flag block clears the "changed"
  marker *[quirk]* (Windows `0x00427C29`).

The argument names follow the dispatch notification that calls it: `0x4BB2CFC0` passes (data3, data2, −1, −1)
and `0x4BB2D020` passes (−1, −1, data3, data2) *[inferred: data3 = new, data2 = old]*.

### 4.6.1 Dispatch trip state, destinations and clock *[compat]* (MS17, partial)

The dispatch state is distinct from MySim state (§2.2): **−1** = unplaced/recovery, **2** = trip in progress,
**3** = at a location. Constructor state and previous state are both −1; flags and the simulation-minute
counter start at zero, travel type at `0xFF`. State 2 alone does not imply a moving visible automaton:
an idle or transit-switch wait remains a trip. Current location, home/default destination, active destination
and commute destination are four separate location values, at object offsets `+0x5C`, `+0x84`, `+0xAC`,
`+0xD4`. Getters expose these values, not copies manufactured from MySim home/job fields
(`0x00431290`–`0x004312B6`, `0x00698CD0`, constructor `0x00433A60`).

**Clock and delivery.** Once a dispatch pathfinder exists, each `0x44778231` message adds **5** to its
signed 32-bit minute counter; each animation message `0x86AD10EE` runs a state update. The minute message
does not itself run a state update. These are neither wall-clock dwell timers nor one trip per daily tick
(`0x0043793B`–`0x00437975`; message-target `this` is object+8).
The at-location deadline is `+0x80`; location duration/deadline are `+0x20/+0x24` within each location.
Use the strict comparison **now > deadline** for expiration; equality still waits where a tracked occupant
exists (`0x0043660C`–`0x0043662F`, `0x00435B2F`–`0x00435B61`, `0x00436146`–`0x0043614F`).

Each changed flag value sends `0x4BB2D020(oldFlags,newFlags)` and increments the change counter (`+0x34`).
Each changed state sends `0x4BB2CFC0(oldState,newState)` and increments it. Setting state 3 first installs
the new state, then sets flag 2 with its flag notification, then sends the state notification. Repeating
state 3 still sets flag 2 if needed, but sends no state notification when the state is unchanged
(`0x00431BC0`, `0x00431C10`). See §3.3 for the message envelope and owner reentrancy.

The following transitions are settled independently of the remaining transport handoffs (MS18).
Location equality here is §4.7.1's identity/cell comparison, not geometric proximity. The export's
`0x00431690` name is misleading: instructions implement **location inequality**, not deque mutation.

| State/input | Location, queue and timing effects | State/notifications, in order |
|---|---|---|
| −1, home flag `0x80` set, current differs from home | Clear flags with mask `0xFFFF01B2`, then clear `0x32`; copy home to current; travel type `0xFE`; deadline = now + current duration | `0xEBA06C12` trip-end/location notice; then enter 3 (flag-2 notification before state notification). Each preceding changed flag mask also notifies |
| −1, home flag set, current already equals home | Apply initial mask above; current remains home | enter 3; no teleport/location notice |
| −1, queued destinations remain | Queue is retained for at-location selection | enter 3 |
| 3, any update | Invalidate the active destination; detach the current location's automaton occupant if present | no state change merely for this cleanup |
| 3, now ≤ deadline, valid current location and an occupant to track | Stay at current location | no departure |
| 3, current invalid, by-commute location can be resolved | Resolve it using §4.7.1; copy resolved location to commute destination | `0xEBA06BF4` job-location notice before destination selection |
| 3, current invalid, resolution fails, valid home exists | Copy home to current | `0xEBA06C12` before destination selection |
| 3, eligible to select a destination | Discard invalid queue heads. Pop first valid queued location; if none, choose home only when home flag is set and current differs from home. Reset neighbour retry count to 0. A selected unresolved by-commute location may resolve and update commute destination with `0xEBA06BF4` | A valid selection differing from current becomes active destination, with its deadline = now + its duration; enter 2, then clear flags `0x5800`. State notice precedes that flag notice |
| 3, no valid selection, and no usable home return | Preserve current | enter −1 when home flag is absent or current differs from home; otherwise remain 3 at home |
| 2, route-ready flag `0x1000`, player-driving flag `0x10000` | Retain trip and locations | no automatic progression |
| 2, route-ready, stuck flag `0x40`, not player-driving | Leave recovery to the next update | enter −1 |
| 2, path search still suspended (result 2) | Preserve request and destinations | remain 2 |
| 2, path search fails, neighbour endpoint retries exhausted | Preserve location for recovery | enter −1, then `0xEBA06C12` |

Evidence: `updateState` `0x004365C0`–`0x004370EC`; the destination deadline store is
`0x004367CD`–`0x004367E0`; failure state/notice is `0x00436D61`–`0x00436D8A`.
This table does not yet specify all route-ready/transport-switch branches; see MS18.

**Destination queue.** `AddDestinationLocation(location, options)` copies the supplied location and always
returns true: option bit 4 first clears the queue, bit 1 selects front insertion, otherwise insert at the
back. Other option bits have no effect. This call does not force the dwell deadline to expire and does not
itself depart (`0x00434BA0`). A by-commute destination uses the validation/job-found rules of §4.7.1.

**Reported wait value.** The value sent to Lua as `trip_wait_length` is the dispatch's stored switch-start
timestamp (`+0x54`), **not now minus that timestamp**. Entering a located transit switch stores now there
and sets a deadline of now + minimum switch time (`0x00435F1F`–`0x00435F3D`). Its getter returns the stored
value unchanged while flag `0x10` is set. Without that flag it clears the value only when the signed
32-bit difference now − timestamp is **greater than 15**; the getter mutates that stored value
(`0x004312C0`–`0x004312DF`, Lua call `0x00427DC4`–`0x00427DD4`). Thus timestamp 100 at now 115 reports 100,
at now 116 without flag `0x10` reports 0, and at now 116 with the flag reports 100. Do not replace this
quirk with an elapsed-time computation. Serialization preserves the timestamp (§6.2.1).

### 4.6.2 Route-ready transport: geometry and stops *[compat]*

These contracts supplement §4.6.1; §4.6.3–§4.6.5 complete the dispatch-side handoffs.
Foreign transport services must
report real outcomes; route acceptance, an expired wait and a model allocation alone do
not constitute arrival.

**Proximity geometry.** The distance helper (`0x00431900`–`0x00431A29`) resolves
each location to a cell rectangle. With an occupant, look up its lot; without one,
look up the lot at the stored cell. A found lot supplies its cell bounds. With no
lot, use the degenerate rectangle (cellX,cellZ,cellX,cellZ). World x/z and neighbour
region coordinates are not geometric inputs (`0x00431880`–`0x004318F4`).

For rectangle arguments A then B, the helper measures the ordinary Euclidean
point-to-rectangle distance from **each of B's four corners to A**, takes the
minimum, then truncates toward zero to a signed integer. Each individual distance
is stored as float32 before minimum selection; integer rectangle coordinates are
converted to float32. Point-to-rectangle distance is zero for a point inside or
on the rectangle, the perpendicular gap alongside an edge, and the square root
of the sum of squared axis gaps beyond a corner. The square root is real: the
instructions contain `FSQRT` at `0x005D7324`; the final conversion is `_ftol2`
at `0x00431A21`. Use the main-thread floating-point environment of foundations
§18. This is neither a squared-distance helper nor a general symmetric
rectangle-to-rectangle minimum. In particular, one rectangle containing another
can give a different result when the arguments are swapped. Do not substitute
rectangle overlap or center-to-center distance.

Both arrival comparisons pass **active destination as A and current location as B**
(`0x00435B82`–`0x00435B8F`, `0x004361ED`–`0x004361FA`). Let D be that truncated
linear result, and P be `DESTINATION_PROXIMITY_MIN`. The threshold is the signed
low 32 bits of P×P. Idle-completion proximity is **D < threshold**; final-marker
proximity is **D ≤ threshold** (`0x00435B94`–`0x00435BA3`,
`0x004361FF`–`0x00436209`). Thus the published tuning is squared even though
the compared distance is linear. Negative tuning is not clamped; multiplication
wrap and signed comparisons are significant.

**Transport handoff waits.** Travel codes are 0 Walk, 1 Car, 2 Bus, 3 Rail,
6 Subway, 7 LightRail, 8 Monorail; FE and FF are controller sentinels.
For a switch from one of 0/1/2/3/6/7/8 to a different ordinary mode, the
target class determines the wait: 0/1 uses now + `IDLE_DURATION_MIN`; 2/3/6/7/8
uses now + `TRANSIT_SWITCH_WAIT`. Current's occupant is released and the held
automaton is cleared before this wait; this does not change the active destination
to current or emit arrival. Source FE/FF and same-mode continuation take different
branches, and must not be treated as this ordinary switch (`0x004362E6`–`0x00436365`).

During a transit-stop search (flag `0x10`), a missing current occupant triggers
an occupant-manager query over the rectangle spanning current cell and the
route marker's cell, extended by one cell on each side. Select the **first**
returned occupant matching the target stop purpose; this is not a nearest-stop
distance sort. The target also sets the automaton group used while boarding:

| Target mode | Stop purpose | Automaton group |
|---|---|---|
| 2 | `0x1301` | `0x4200` |
| 3 | `0x1300` | `0x4215` |
| 6 | `0x1302` | `0x4217` |
| 7 | `0x1303` | `0x422C` |
| 8 | `0x1307` | `0x422E` |

On selection, current holds that occupant and takes its world/cell position;
deadline = now + transit wait, and the wait getter's stored timestamp = now.
Emit `0xEBA0732E(purpose,0,0)` **before** clearing the held automaton
(`0x00435EF4`–`0x00435F46`). A missing stop takes the stuck path, not an arrival.
With a stop and deadline ≥ now, search for a matching-group automaton at the
current cell, retaining service enumeration order and requiring the automaton
interface. With deadline < now, attempt the next route leg without a supplied
automaton instead; FF means stuck. Successful dispatch of that leg sets flag
`0x200` and clears `0x436`, each through the existing changed-mask contract.
The expiry boundary is strict (`0x00435F79`–`0x00435FB9`).

**Final-marker observations and notice order.** For route marker FE, if either
current or active destination is a neighbour location, release current's occupant
and clear the held automaton before measuring proximity. At D > threshold, keep
the trip in progress, use travel FE, set deadline = now + idle minimum, and emit
`0x2BAC398F(0,0,0)` before the common state/flag notifications. At D ≤ threshold,
the near-marker path emits `0xEBA06BD4(0,0,0)` before that common tail; it can still
choose an idle wait when current differs from both home and commute and neither
neighbour/idle condition applies. The notice alone therefore does not promise
state 3 (`0x004361AD`–`0x004362E0`).

The common tail installs the selected travel type, calls the state setter (including
its flag-2-before-state order), invokes setStuck if selected, applies idle setup
`flags |= 0x2401` if selected, clears the selected mask, then ORs the selected
flags. Every changed flag operation remains separately observable; masks must
not be folded into one final notice (`0x004364E5`–`0x0043657F`). The exact selection
of those masks for the other branches is given next.

### 4.6.3 Route-ready decisions and recovery (MS18) *[compat]*

The route-ready controller consumes an ordered route of three-byte markers (unsigned
cell x, unsigned cell z, mode). FE is the end/walking marker; FF is a failed dispatch
result, not a successful ninth transport. The ordinary modes are 0, 1, 2, 3, 6, 7, 8.
Bits 1, 4, 10, 20 and 40 below are **hexadecimal masks**, respectively idle,
delay, stop wait, route switch ready and stuck. Several may coexist; priority matters.
This section describes the dispatch's decisions; routing, network snapping, model
creation and route acceptance are service inputs, not assumed successes.

**Observable flag operations.** The tables give a removal mask C and addition mask A.
After the indicated service calls/notices, install the selected travel type (one change
counter increment if different), run the state setter (§4.6.1), run setStuck if A has
40, perform idle setup `flags |= 0x2401` if A has 1, remove C, then add A.
Each of these operations sends its own changed-flags notice and counter increment.
An unchanged operation is silent. State is retained unless a row says otherwise.
These are ordered effects, not a license to combine masks. The common tail always
returns true, including recovery/failure decisions; a true return is not arrival.
Evidence: instructions `0x004364E5`–`0x0043658B`.

| Priority/input | Result before the common effects | C | A |
|---|---|---|---|
| Stuck 40 without idle 1 | State −1; retain travel and queue | 0 | 0 |
| Idle 1, including stuck+idle | Use the idle recovery rules below | as below | as below |
| Travel FE, no delay 4, current unequal to both home and commute | Deadline = now + idle minimum; retain travel/state/queue | 0 | 1 |
| Stop wait 10 | Use §4.6.2's stop search/boarding; missing stop or a failed leg selects stuck | 0 on failure; 436 on successful leg | 40 on failure; 200 on successful leg; 0 while awaiting a vehicle |
| Delay 4, now ≤ deadline | Wait; no leg attempt | 0 | 0 |
| Delay 4, now > deadline | Attempt leg without a supplied automaton; travel = result | 0 on FF; 436 otherwise | 40 on FF; 200 otherwise |
| No switch-ready 20 | Attempt leg without an automaton; success uses its travel result. On FF, skip to the next switch as described below | 436 on success; 2 on successful skip; 0 if no switch | 200 on success; 220 on successful skip; 0 if no switch |
| Switch-ready 20 | Use the mode/marker table below | as below | as below |

For a successful skip, current's cell becomes the marker immediately before the
new switch, and travel takes that marker's mode. This is a cell relocation, not a
destination arrival. If no switch remains, send `0xEBA06C12` and select state −1.
`jumpToNextSwitch` advances only while the leg-start index is below the leg-end index,
and rejects a resulting end-of-route/FE marker (`0x00434DF0`–`0x00434E23`). Leg
partitioning skips equal-mode markers, snaps the segment to the network, and advances
past an unsnappable zero-length non-FE segment; it is not random route selection
(`0x00434230`). The successful skip and failure masks are verified at
`0x00436459`–`0x004364DD`.

| Switch-ready marker/source | Effect and notices before common effects | C | A |
|---|---|---|---|
| Marker FE, D > P² | Travel FE; deadline = now + idle minimum; send `0x2BAC398F(0,0,0)` | 460 | 423 if both locations are local; 427 otherwise |
| Marker FE, D ≤ P², both locations local, no prior delay/idle, current differs from both home and commute | Travel FE; deadline = now + idle minimum; send `0xEBA06BD4(0,0,0)`; retain trip state | 460 | 423 |
| Other near FE | Travel FE; state 3; add now to current's **duration** (+7C), not its deadline (+80); copy active to current only if active is valid; send `0xEBA06BD4(0,0,0)` | 8460 | 2 if both locations local; 6 otherwise |
| Same ordinary source and marker mode | Reuse current automaton for next-leg dispatch. FF invokes setStuck immediately. Success separately adds 200, removes 400 and records the returned travel value | 20 | 0 before fallback below |
| Different ordinary modes, target 0/1 | Release current occupant and held automaton; deadline = now + idle minimum; retain source travel during wait | 20 | 1 |
| Different ordinary modes, target 2/3/6/7/8 | Release current occupant and held automaton; deadline = now + transit wait; retain source travel | 20 | 10 |
| Source FE/FF, marker not FE and not the same source | Select state −1; no ordinary-mode switch wait | 20 | 0 before fallback |
| Other mode (including 4/5 or unrecognized target) | No ordinary-mode wait; for an ordinary source, clear the held automaton | 20 | 0 before fallback |

Before measuring FE-marker proximity, if either current or active is a neighbour,
release current's occupant and clear the held automaton. Both-local testing uses
each location's occupant/local flag, not the by-commute flag. D and P² have the
width/rounding rules in §4.6.2. In the near-arrival row, copying active subsequently
replaces the just-modified duration/deadline with the copied values; if active is
invalid the modified duration remains. This odd duration write is literal
(`0x0043629B`–`0x004362C5`), not a corrected `now + duration` deadline.

**Switch fallback.** After a switch-ready decision, if A has none of bits in 53,
add bits 3 to A, force travel FE, clear the destination queue and set deadline to
now + idle minimum. This also happens after same-mode success or immediate
setStuck: do not silently suppress it because an automaton was dispatched or a
failure already notified. Immediate same-mode flag/notices precede the fallback
and common tail. This fallback is not run by the earlier stop/delay/idle rows
(`0x0043638F`–`0x00436454`).

**Idle recovery.** Its initial removal mask is 2002. Obtain the held automaton,
and its model information when available. If 2000 was set on entry, first send
`0xEBA06BD0(0,0,0)` and retrieve the stored offset for the current model type.
Only an existing model-type-2 automaton with model-info kind 7 and travel 0 or FE
can continue directly. Otherwise snapshot current, substitute the current route
marker's cell in that snapshot unless it is FE, and try road snapping first on
current and then, if that fails, on the snapshot. If both fail, send EBA06C12,
select state −1 and do not run the following expired-idle stage. On a successful
snap, current receives the snapshot, EBA06BD0 is sent, and a one-marker fallback
dispatch is attempted. Its FF result sends EBA06C12 and selects state −1. Its
non-FF result replaces selected travel unless the stored source travel is FE.
Thus two EBA06BD0 notices can occur in one update; do not deduplicate them.
Evidence: `0x0043588D`–`0x00435AF4`.

Only an idle entry with 2000 set continues into the expired-idle stage. Reacquire
the held automaton; while it exists and deadline ≥ now, retain the wait and apply
a stored model offset only when it is **strictly positive** (zero is not applied
here). When absent or strictly expired, first clear idle bit 1, separately
notifying, then measure D. Travel FE with switch-ready 20 completes at state 3,
deadline = now + current duration, removal A403; copy active only if valid and
D < P². Otherwise attempt a new leg. Success uses removal 2437/addition 200.
On FF, try skipping a switch: success uses removal 2002/addition 220 and the
preceding marker's cell/mode. If skipping fails but D < P², complete at state 3
with removal A403 and deadline = now + current duration, copying active only if
valid. Otherwise emit EBA06C12 and select state −1. These completions do not emit
EBA06BD4. Evidence: `0x00435AF4`–`0x00435CA7`; strict idle proximity is distinct
from the inclusive FE-marker test.

**Stuck is also a notice producer.** setStuck first adds 2401 when not already
idle and either flags have 12 or travel is 0/1; then always adds 440; then sends
EBA06C12 even if neither flags operation changed anything. It does not itself
change state, travel, current or active destination (`0x004320D0`). A delayed or
stop-wait FF reaches it after the common state setter; same-mode FF reaches it
before fallback. A subsequent stuck-without-idle update selects state −1.

### 4.6.4 Neighbour retries, models and foreign transport inputs (MS18) *[compat]*

**Neighbour retry quirk.** `updateNeighborLocation(index, location)`
(`0x00434C40`–`0x00434DE4`) returns false for the shipped binary's normal execution.
For a neighbour location it checks the regional connection, allocates an empty
occupant list, links its sentinel to itself, and immediately tests that empty
list. No service call populates it between initialization and test
(`0x00434CC8`–`0x00434CDE`). Local locations and missing/empty regional connections
also return false. Consequently no retry index selects a connection and no
location is changed by this helper. The unreachable selection body decrements
the supplied index once per matching occupant class, would select at zero
(one-based), and would clamp the selected cell to city bounds; implementers must
not invent an enumeration to make that body reachable. This rule settles the
retry outcome, however unhelpful the original behavior appears.

**Model choice is deterministic in this owner.** The mode-to-group mapping is
0/FE → 4107, 1 → 420B, 2 → 4200, 3 → 4215, 6 → 4217, 7 → 422C, 8 → 422E;
other modes map to group 0. Model type is 0 for car, 1 for ordinary walking, 2
for idle walking or FE, and −1 for the transit/unknown modes. A missing walking
model key substitutes group 4149 for male and 414A otherwise. The stored model
map supplies resource keys and offsets; absent offsets are −1, not a random
choice (`0x00432900`, `0x00432960`, `0x004329A0`).

Creating an automaton uses current's world x/z and y=0, or (0,0,0) when current
is invalid, the mapped group, stored key, creation flags 214042, dispatch's
message target and FF priority. A created occupant gains property 1BC42C49=1
(`0x00435170`–`0x004352A3`). A supplied automaton is retained only if its occupant
matches the mapped group and its cancellation/busy test permits reuse; otherwise
release it and request creation. Next-leg extraction must contain more than one
route marker before basic dispatch is attempted (`0x00435540`). Failure to
create, obtain the route interface or accept the route produces FF; do not
treat creation alone as departure or arrival. Success retains the automaton's
occupant, records model type and updates current from its reported position
(`0x004352B0`–`0x00435528`).

There is **no dispatch-owned random draw** in these decisions, recovery, model
lookups or neighbour retries. Automata creation/model realization and movement
are calls into foreign services. Their returned identity, group/model data,
route-acceptance result, positions and ordered callbacks are explicit inputs to
an independent dispatch model. This spec does not prescribe the foreign
automata owner's internal RNG stream, nor permit drawing from the MySim
residence generator as a substitute. No successful-trip evidence follows from
an accepted destination or creation request.

### 4.6.5 Automaton and mission callbacks (MS18) *[compat]*

`DoMessage`'s `this` is dispatch+8. The following contracts use base-object
locations/flags; the decompiler's apparent fields are displaced by eight bytes.
For automaton message 09D92FC3, the subtype is the integer returned by message
slot +38. Except subtype 5's state guard, matching the sender's occupant to the
held automaton occupant is the guard specified below. All other subtypes have
no dispatch effect. Evidence: `0x00437500`–`0x004378C4`.

| Subtype/input | Dispatch effects in order |
|---|---|
| 1, idle bit set | Add 2000 with changed-flags notice, then update state; repeated bit still runs update |
| 2, held occupant matches sender | If active-player 10000, call ExitUDriveIt; otherwise GoTo(active destination), then add 401 with changed-flags notice even when GoTo failed |
| 5, state 2 and not active-player | Query held automaton's route interface. With at most one remaining marker, sample its position into current, add 420, select next leg, update state. Otherwise submit its remaining route excluding the first marker, with acceptance arguments true/false; do not advance the dispatch leg or synthesize arrival |
| 9, held occupant matches sender | Release current's occupant if present and increment the change counter. If pending-player 20000, clear that bit with notice, then EnterUDriveIt. Sample held occupant position; if the sampled cell differs from current and not active-player, set approach flag 8000 iff squared integer cell distance to active is strictly below approach-cell tuning squared. Finally update current from the sampled world position |
| Message 0BB324BD, data1=1, active-player set | ExitUDriveIt; other data1 values do nothing |

The approach test uses signed 32-bit subtract/multiply/add and signed comparison
(`0x004377CF`–`0x00437834`), with the low 32 bits of arithmetic; equality clears
8000. This is a squared **cell** distance, unlike §4.6.2's truncated linear lot
distance. While active-player control is set, automatic trip updates pause;
subtype 9 can nevertheless process pending-entry and position input. Exit's
queue/home/path/flag behavior remains §7.2.1; neither a mission callback nor
Exit guarantees automatic arrival at the requested destination.

Message 8BBBD22A calls the message's +80 accessor, then stores the boolean
from +24 as the dispatch's tool/control latch. On A6B79601/A6B79611, a set
latch causes the mission service's +88(false) call and clears the latch only
when both its +8C and +34 queries are false (`0x0043797C`–`0x004379F6`).


### 4.7 Job, commute and residence changes

**`FindNewJob()`** asks the dispatch to set a commute destination "by commute": the dispatch runs a traffic
pathfinder (traffic simulator `CreatePathFinder`, slot 6) from the home lot towards job destinations and takes
the lot (building) or neighbour connection at the end. It returns whether a destination was set. The exact
contract (bytes `0x00425A30`, `0x004350D0`, `0x00432F50`; answers MS8/MS9) is in §4.7.1. When the dispatch later
reports the destination
(`0xEBA06BF4`):

- a city lot → job := the lot's building; neighbour city := null;
- no lot → neighbour city := the region's city at the neighbour cell (app → region → city at cell); job := null;
- in both cases store the cells, jobless days := 0, `UpdateVariables`, push event `MYSIM_NEW_JOB` `0xAA3EDBB1`.

#### 4.7.1 Job search (dispatch `SetCurrentCommuteDestination` / `validateCommuteLocation`) *[compat]*

`FindNewJob` (`0x00425A30`) builds a default dispatch location, marks it "by commute" (location byte `+0x1D` := 1)
and calls the dispatch's `SetCurrentCommuteDestination(loc, notify = true)` (`0x004350D0`), returning its result.
A default location is invalid (occupant flag set, cells −1), so:

1. If the location is valid it is stored as the commute destination (dispatch `+0xD4`) without any search.
2. Invalid and `notify` false → return false. Invalid, `notify` true and dispatch state (`+0x40`) = 2 → stored
   unvalidated (no search). Otherwise run **`validateCommuteLocation`** (below); false → return false.
3. Store the location; if `notify`, notify the MySim with `0xEBA06BF4` (§4.7 handler); return true.

The caller's location is copied first (the copy holds its own reference on the occupant); step 3 assigns the copy
to `+0xD4` (the old destination's occupant is released, the new one referenced) and the copy is released on every
exit. A valid location with `notify` true still notifies. (answers MS11)

**Dispatch location (`cLocation`, 0x28 bytes) contract** *[compat]*:

**Equality (MS19).** Two locations are equal exactly when all of these match:
the byte-valued in-this-city flag, occupant identity (including null), byte-valued
byCommute flag, signed cell x and z, and regional city x and z. Occupant identity
does not override any coordinate or flag difference. Regional coordinates participate
even for local locations. World coordinates, duration and deadline do not participate.
Thus two null-occupant locations can be equal, but a null/non-null pair cannot;
two locations referencing the same building with different cells are unequal.
This is exact stored-field equality, independent of validity or proximity.
Instructions `0x00431640`–`0x00431688`, with the two-word regional comparison at
`0x00424EC0`–`0x00424EDE`, establish the rule; the latter's exported deque name is wrong.

| Offset | Field | Default (`0x00431320`) |
|---|---|---|
| `+0x00` | occupant (referenced) | null |
| `+0x04`, `+0x08` | world x, z (float) | −16.0, −16.0 (= 16 × cell) |
| `+0x0C`, `+0x10` | cell x, z | −1, −1 |
| `+0x14`, `+0x18` | neighbour city's region position x, z | **−1, −1** |
| `+0x1C` | "in this city" flag (the "occupant flag" above) | 1 |
| `+0x1D` | byCommute | 0 |
| `+0x20`, `+0x24` | (copied, never read here) | 0, 0 |

- **`IsValid`** (`0x00434F90`): flag `+0x1C` set → valid iff `0 ≤ cellX < cellCountX` and `0 ≤ cellZ <
  cellCountZ` (city `+0x174` / `+0x178`); **the occupant is not consulted** (a location with an occupant and cells
  −1 is invalid; cells in range without an occupant are valid). Flag clear → valid iff the region (app slot 35,
  `GetCity(+0x14, +0x18)` slot 11) returns a city **and** the neighbours simulator (`0x00B43D8C`, slot 18
  `+0x48`, `GetConnectionsOfTypeToCity(type, city, list)`) returns a non-empty list for at least one of the types,
  tried in this order with a stop at the first hit: Highway 3, GroundHighway 12, Avenue 8, Road 2, DirtRoad 10,
  Rail 1, LightRail 7, Monorail 11, Subway 6 (table `0x00B07DD0`, terminator 13). Ferry, power and pipe
  connections never make a neighbour location valid.
- **`SetByOccupant(o)`** (`0x00431430`): first resets **every** field to the default above (so byCommute becomes 0
  and `+0x14`/`+0x18` −1), then flag `+0x1C` := 1, holds `o` (reference the new, release the old); if `o` is
  non-null: world x, z := `o.GetPosition()` (cISC4Occupant slot 8) x and z, and cell := `floor(world / 16)` per
  axis (single precision, `0x00431370`; exact rule below). A null `o` leaves cells −1 (invalid). Returns true. Callers that need
  byCommute set it again afterwards: `ChangeJob` and `validateCommuteLocation` (which restores the byCommute it had
  before the call, i.e. 1).
- **World → cell (`0x00431370`, per axis, answers MS13 a)** is not a truncating conversion but the
  "magic number" floor, under the main thread's 24-bit x87 precision (foundations §18). With `w` the float32
  world coordinate: `q := w × 0.0625` (`0x00A85798`, exact); `d := q − 0.5` (`0x00A84D2C`) rounded to 24 bits;
  `s := d + 12582912.0` (`0x00A8579C`, 1.5 × 2^23) rounded to 24 bits and stored as float32; `n :=` the bit
  pattern of `s` plus `0xB4C00000` (wrapping 32-bit add, read as int32); the result is the wrapping int32 `n + 1` if `q` compares
  **equal** to the integer `n + 1`, else `n` (an unordered compare, i.e. NaN, also gives `n`). For
  `|w| < 2^26` this is `floor(w / 16)` except one case: a negative `w` with `|w| ≤ 2^-21` gives **0**, not −1
  (`d` rounds to exactly −0.5 and `s` ties to even). Outside that range and for NaN/∞ the result is the bit
  arithmetic, **not** a saturation and not `0x80000000`; a NaN keeps its payload (quieted) into `n`.
  Vectors (input float32 bits → cell): `0x432FE666` (175.9) → 10; `0xC1000000` (−8) → −1; `0xC1800000` (−16) →
  −1; `0x80000000` (−0) → 0; `0x8DA24260` (−1e−30) → **0**; `0x477FFFFD` (65535.99) → 4095; `0x4C800000`
  (2^26) → 4194304; `0x4CBEBC20` (1e8) → **5222152**; `0xCCBEBC20` (−1e8) → **−8305697**; `0x7FC00000` (NaN) →
  **880803840**; `0xFFC00000` (−NaN) → **−1266679808**; `0x7F800001` (sNaN) → 880803841; `0x7F800000` (+∞) →
  876609536; `0xFF800000` (−∞) → −1270874112.
- **`SetCellPosition(x, z)` (`0x004311B0`, answers MS13 b)** writes the cells **and** the world fields:
  `+0x0C := x`, `+0x10 := z`, `+0x04 := float(x << 4)`, `+0x08 := float(z << 4)` (32-bit shift, wrapping, then
  signed int → float32, rounded to nearest-even at the store). It touches nothing else (occupant, flags, `+0x14`/`+0x18` unchanged). So the
  neighbour branch of `validateCommuteLocation` leaves world = 16 × the clamped cell (end cell (−1, 37) in a
  64×64 city → cell (0, 37), world (0.0, 592.0)), not the −16.0 of the reset; and §4.6's `force` reset to (0, 0)
  also moves the **local copy's** world position to (0.0, 0.0); it does not move the dispatch itself.
- **`+0x40` is the dispatch's trip state**: written only by dispatch `setState(s)` (`0x00431C10`): old := `+0x40`
  (kept at `+0x44`), `+0x40` := s; s = 3 also ORs `2` into dispatch flags `+0x48` (notification `0x4BB2D020`
  when they change); if old ≠ s, notification `0x4BB2CFC0(old, s)`, which is how §4.6 receives `newTripState` /
  `oldTripState`. So the value tested in step 2 (= 2) is the same trip state §4.6 sees, owned by the dispatch;
  the MySim only learns it through the notification.

**Callers of `SetCurrentCommuteDestination`** (all pass `notify = true`):

| Caller | Location passed | Effect |
|---|---|---|
| `FindNewJob` (`0x00425A30`) | default, byCommute 1 | invalid → search (or stored as is when trip state = 2) |
| `ChangeJob(b)` (`0x00425D70`) | `SetByOccupant(b.slot3())` (cISC4BuildingOccupant slot 3 `+0x0C`, an ICF-folded getter returning `b`'s cISC4Occupant), then byCommute 1 | `b` inside the city → valid → stored **without search**, `0xEBA06BF4` sent; null occupant → invalid → a normal job search runs instead |
| `FailCommute` (`0x0042C8E0`) | default, byCommute 1 (after `QuitJob` and the event) | the same as `FindNewJob`: a **new job search**, not a plain reset; only at trip state 2 is the default (empty) destination stored |
| `SetResidence` (`0x0042E020`, call `0x0042E28B`) | default, byCommute 1 | the same search, then `DispatchCommute` |

**`validateCommuteLocation(loc)`** (`0x00432F50`) uses the dispatch's own pathfinder (created once in dispatch
`Init` with the traffic simulator's `CreatePathFinder`, kept for the dispatch's lifetime). In this order:

| Step | Pathfinder call (pathfinder.md §4.2 / slot) | Value |
|---|---|---|
| 0 | `resetPath`: clear the route deque, dispatch flags `0x800|0x1000|0x4000`, `Reset` (22), `ClearAllTravelTypeFlags` (15), `SetRouteInfo(route deque)` (8) | — |
| 1 | reset `loc` to a default location, then `loc.byCommute := 1` | — |
| 2 | dispatch flag `0x80` (has a valid default destination = home, set by `SetDefaultDestination`) clear → **return false** | — |
| 3 | `SetFlag` (5) twice | `1` (no budget), then `0x10` (no ferries) |
| 4 | `SetTripDestinationFilter` (9) | **0** (standard destination) |
| 5 | `SetDestinationFlag` (10) by the MySim's `GetWealth()` (cISC4MySim slot 19) | wealth 1 → `0x1`, 2 → `0x2`, 3 → `0x4`, anything else → `0x1` |
| 6 | `SetTrafficDensity` (18) | 1 |
| 7 | `SetStartRect` (19) | cell rect of the home location (`+0x84`), `getLocationCellRect` (`0x00431880`): occupant non-null → lot manager slot 14 (`+0x38`, the occupant's lot); occupant null → `GetLot(cellX, cellZ, false)` (slot 6); a lot → its `GetBoundingRect` (lot slot 14); no lot → the single cell (cellX, cellZ, cellX, cellZ). An occupant whose lot is not found goes **straight to the single cell**, the cell lookup is not tried (answers MS13 c) |
| 8 | `SetTravelTypeFlag(mask, alsoStarting)` (13) | `0x1CF`, true (Walk, Car, Bus, Passenger train, Subway, Elevated train, Monorail — allowed **and** starting) |
| 9 | `FindPath` (23) | **one** call; result ≠ 1 → return false |

No end rect, no max trip length and no `ClearFlag`/`ClearAllDestinationFlags` call is made anywhere in the
dispatch. Flags and destination flags are OR-ed into the long-lived pathfinder, so they **accumulate** *[quirk]*:
a MySim whose wealth changed from 1 to 2 searches `$` and `$$` jobs (`0x3`); flags stay `0x11`. `ApplyTraffic` is
not called, so the found destination's trip capacity is not consumed.

After success, `GetEndCell(x, z)` (27), then:

- `x, z` in city bounds (`cISC4City::CellIsInBounds`, `+0x18C`): the lot manager's `GetLot(x, z, false)`; no lot,
  or the lot's `GetBuilding()` (lot `+0xC0`) null → return false. Else `loc` := that occupant (`SetByOccupant`),
  occupant flag 1, byCommute kept (1); return true.
- Out of bounds: `FindCityOnOtherSideOfConnection(x, z)` (neighbours simulator `+0x30`); null → false. Else clamp
  `x` to `[0, cellCountX − 1]` and `z` to `[0, cellCountZ − 1]` (city `+0x174` / `+0x178`), drop the occupant,
  occupant flag 0, `SetCellPosition(x, z)`, and the regional city's position (its slot 5, `+0x14`) goes into the
  location's neighbour-city fields (`+0x14`, `+0x18`); return true.

So the job search is **one** filter-0 search that picks the nearest reachable lot or neighbour pool with free
`$`/`$$`/`$$$` job capacity (pathfinder.md §4.10.1); there is no per-candidate loop and no filter 3 here (filter 3
is the dispatch's own trip-path setup, a separate configuration).

Worked examples (conformance):

| Case | Setup | Expected |
|---|---|---|
| wealth 2, first search | home lot rect (10,10)–(12,11) | flags `0x11`, filter 0, dest flags `0x2`, density 1, start rect (10,10,12,11), travel `0x1CF`/`0x1CF`, one FindPath |
| wealth 1 then 3 on the same dispatch | two searches | dest flags `0x1`, then `0x5` |
| wealth 0 | — | dest flags `0x1` |
| no home (flag `0x80` clear) | — | no pathfinder configuration beyond `resetPath`; `FindNewJob` false; no `0xEBA06BF4` |
| end cell (−1, 37), neighbour city found, city 64×64 | — | location cell (0, 37), no occupant; true |
| end cell in city, lot without building | — | false |
| `IsValid` of default location | — | false (cells −1) |
| `IsValid`, flag 1, no occupant, cell (3, 4), city 64×64 | — | true |
| `IsValid`, flag 1, cell (64, 0), city 64×64 | — | false |
| `IsValid`, flag 0, region city at (+0x14,+0x18) with only a Ferry connection | — | false; with a Subway one → true |
| `SetByOccupant(o)`, o at world (175.9, y, 32.0) | — | cell (10, 2), byCommute 0, `+0x14`/`+0x18` −1 |
| `ChangeJob(b)`, b in city, trip state any | — | stored, no pathfinder call, one `0xEBA06BF4`, true |
| home location: occupant set, lot manager has no lot for it, a lot exists at its cell (5, 6) | — | start rect (5, 6, 5, 6) (the cell lot is not used) |
| home location: no occupant, lot at cell (5, 6) with bounding rect (4, 6, 7, 8) | — | start rect (4, 6, 7, 8) |
| neighbour end cell (−1, 37), city 64×64 | — | `+0x04`/`+0x08` = 0.0 / 592.0 |

**`ChangeJob(b)`** (slot 8): set the dispatch commute destination to building `b`; result of the dispatch.

**`CheckCommute()`** (`0x0042D9E0`, answers MS15 a) has its **own** pathfinder request, distinct from job
search in §4.7.1. Native branch and callback order:

1. Obtain the traffic simulator via city slot 69 (`+0x114`) **on entry**, even if the following preconditions
   fail (`0x0042D9E7`–`0x0042D9F0`). A null city/traffic service is not guarded here.
2. Require a job building (`+0xE0`) **or** neighbour job city (`+0xE4`), and a residence building (`+0xC4`).
   Otherwise return false, preserving lastCommuteTime and without calling `FailCommute`.
3. Initialise the pathfinder output pointer to null and call traffic slot 6 (`+0x18`, `CreatePathFinder(&out)`).
   Only the **output pointer** is tested: null → false with no `FailCommute`; the factory's return register is
   not used (`0x0042DA1E`–`0x0042DA30`). The non-null output is held until step 7.
4. Obtain the residence's occupant via building slot 3 (`+0x0C`; the native building implementation returns
   its occupant sub-object, `0x005FD7E0`), then occupant slot 11 (`+0x2C`, `GetBoundingCityCells(&startRect)`).
   If the job building is non-null, obtain its rectangle the same way; **the local job takes precedence**
   when both job references exist. Otherwise endRect := `(commuteCellX, commuteCellZ, commuteCellX,
   commuteCellZ)` from MySim `+0xF0/+0xF4`, without validation or clamping (`0x0042DA3B`–`0x0042DA87`).
   These are **occupant bounding city cells, not lot rectangles**. No lot-manager query or dispatch destination
   getter runs. Missing home/job lots therefore do not cause a special failure or cell fallback. The occupant
   results are dereferenced without null guards; do not turn an invalid occupant interface into `FailCommute`.
5. Call the pathfinder slots in this exact order: 5 `SetFlag(1)`; 9 `SetTripDestinationFilter(3)`;
   18 `SetTrafficDensity(1)`; 19 `SetStartRect(startRect)`; 20 `SetEndRect(endRect)`; 23 `FindPath()`.
   Rectangles are passed as four int32 values by value (`0x0042DA8B`–`0x0042DB07`). There are **no calls** to
   clear flags, set/clear destination flags, set/clear travel-type masks, set starting costs, or set maximum
   trip length/work time. Preserve factory configuration for those settings; do not borrow job-search's
   wealth flag, travel masks or MAX_TRIP_LENGTH setup. The inherited values are specified in §4.7.2.
6. Only the **full integer result = 1** is success. Success: call slot 26 (`+0x68`,
   `GetTripLengthScaledForDisplay`, **not** slot 25 `GetTripLength`), convert the x87 result with `_ftol2`
   `0x009EEF04`, and store its **low 32 bits** in lastCommuteTime (`+0x154`, `0x0042DB14`–`0x0042DB1C`).
   For finite results within signed-64 conversion range this is truncation toward zero followed by reduction
   modulo 2^32: 12.75 → 12, −1.75 → `0xFFFFFFFF`, 4294967808 → 512. All three operands are representable
   with a 24-bit significand. There is **no nonnegative clamp**.
   The getter itself multiplies its stored trip length (`+0x60`) by traffic's slot-15 display scale
   (`0x006D6030`–`0x006D603B`), under the normal x87 precision rule (§18). Any FindPath result other than 1
   calls `FailCommute` **before releasing this pathfinder**, without calling the getter or storing a new time.
7. Release the original non-null pathfinder through slot 2 (`+0x08`) and return the success boolean
   (`0x0042DB2B`–`0x0042DB40`). No extra MySim-side AddRef or rectangle-result Release occurs.

**`FailCommute`** (`0x0042C8E0`): `QuitJob()` completes first (including `UpdateVariables`), then push event
`MYSIM_COMMUTE_FAILED` `0xCA4553B7`, then construct a default location, mark it **by commute**, and call
`SetCurrentCommuteDestination(location, notify = true)` (`0x0042C8E6`–`0x0042C91A`). Its normal validation,
search and owner-notification rules in §4.7.1 apply synchronously; a successful search can therefore notify
the owner and assign a replacement job before the original CheckCommute pathfinder is released. Do not
reuse the failed pathfinder or substitute a previously drawn RNG value. Failure returns false even if this
replacement search succeeds. The temporary location releases its held occupant on exit (`0x0042C91F`).
Pathfinder slot reference: §7.2.

**`QuitJob()`** (state ≠ 0): drop the job (or the neighbour city and its cells); if state ≠ 5, state := 2;
`UpdateVariables`.

**`DoNeighborConnectionRemoved(x, z)`**: if the MySim has a neighbour job, set the commute-check flag (the next
daily step runs `CheckCommute`). The arguments are ignored *[compat]*.

**`TestForResidence(b, filter, matchWealth)`** (slot 7) → bool:

1. false if `b` is the current residence;
2. if `matchWealth` and `b` has a lot: false unless the lot's occupant wealth equals the MySim's wealth;
3. false if another MySim lives there (agent `MySimLivesHere(b)` non-null; this includes the caller only if it
   already lives there, which step 1 excludes);
4. if `filter`: the result of the residential `cMySimBuildingFilter` (§4.9); else true.

**`ChangeResidence(b)`** (slot 6) → bool: if `TestForResidence(b, false, false)`: move out, `SetResidence(b)`;
then if `CommandWouldTrigger(MoveOut 0x6A47E79B)` (advisors.md, advisor slot 19: some move-out advice would fire
now) **revert**: move out and `SetResidence(old)`. Then location tracking (UI). If the residence actually
changed: wealth := new lot's occupant wealth, state := 1, return true. Else false.

**`FindNewResidence()`** (slot 5) → **always false** *[quirk]* (SDK: bool):

1. Query all occupants citywide with the residential `cMySimBuildingFilter` (occupant manager slot 38, no
   rectangle, filter, group −1).
2. If the query succeeds: `n :=` the list size; **one draw** `r := rng.RandomUint32Uniform(n)` on the MySim's
   own `cRZRandom` (§8). Start at list position `max(r − 1, 0)` (r = 0 walks to the end and wraps to the
   first element). Visit up to n buildings cyclically; the first one that answers `0x87DDA3A9`, passes
   `TestForResidence(b, false, false)` and `ChangeResidence(b)` wins.
3. If there is still no residence, or the residence is unchanged (then move out first): push event
   `MYSIM_CANT_FIND_HOME` `0x6A57DD94`.

**`MoveOut()`**: drop residence and home lot, hide markers and sounds; returns true.
**`LeaveCity()`** (state ≠ 0): `MoveOut`, hide, state := 0, dispatch Shutdown, clear the U-Drive-It state,
destroy the pylon, post `MySimsChanged`.
**`DeleteFromGame()`** (slot 9, from the UI "remove sim"): dispatch Shutdown, pylon, `MoveOut`, state := 0,
hide, `ResetAdvisorState`, post `MySimsChanged`. Returns true.

#### 4.7.2 Inherited commute pathfinder configuration (MS16) *[compat]*

Traffic slot 6 is `bool CreatePathFinder(cISC4PathFinder** out)` (`0x0070D860`–`0x0070D89F`).
On normal completion it publishes a **new** object with one owned reference and returns true in **AL**.
It allocates 0x3160 bytes, calls constructor `0x006D89C0`, slot 1 AddRef, then slot 3 Init, stores the
creating traffic simulator at pathfinder `+0x0C`, and finally writes `*out`. It does not fetch a cached
pathfinder, set any request parameter, call Reset, or test Init's return. AddRef increments the constructor's
zero reference count to 1 (`0x004BDF00`–`0x004BDF07`). The traffic pointer is borrowed: this sequence makes
no traffic AddRef. The caller owns the returned pathfinder reference; CheckCommute's existing release
order in §4.7 applies. This is separate from the dispatch's persistent job-search pathfinder (§4.7.1;
its factory call is `0x00433F3D`–`0x00433F42`, storing dispatch `+0x124`).

Factory values and the request immediately before CheckCommute's first FindPath are:

| Public setting | Factory result | CheckCommute request | Byte evidence (constructor / setter) |
|---|---|---|---|
| Flags, slot 5 | uint32 0 | uint32 1 | `0x006D89E3`; `0x006D6CF0`–`0x006D6CFE` ORs `+0x70` |
| Trip destination filter/strategy, slot 9 | ID **3** | ID **3** (explicitly set again) | `0x006D89EC`; `0x006D6D30`–`0x006D6D3C` writes `+0x80` |
| Destination flags, slots 10–12 | uint8 0 | uint8 0 | `0x006D89F6`; `0x006D6D40`–`0x006D6D54` operates on byte `+0x84` |
| Allowed travel types, slots 13–15 | uint16 **0x0002** | uint16 **0x0002** | `0x006D8A14`–`0x006D8A19`; `0x006D6D98` operates on word `+0x64` |
| Starting travel types, slots 13–15 | uint16 **0x0002** | uint16 **0x0002** | `0x006D8A1D`; `0x006D6DA0` operates on word `+0x66` |
| Starting-cost array, slot 16 | null pointer | null pointer | `0x006D8A02`; `0x006D6DE0`–`0x006D6DEC` writes `+0x90` |
| Maximum trip length, slot 17 | float32 **+0.0**, bits `0x00000000` | float32 **+0.0** | `0x006D89DD`; `0x006D6DF0`–`0x006D6DF9` writes `+0x60` |
| Traffic density, slot 18 | uint32 **0** | uint32 **1** | `0x006D89E0`; `0x006D6E00`–`0x006D6E09` writes `+0x68` |
| Maximum work time, slot 21 | uint32 **0** | uint32 **0** | `0x006D8A08`; `0x006D6E70`–`0x006D6E7C` writes `+0xC0` |

The two masks select only travel-type **ID 1** (Car), not ID 2 and not all travel types; mask values are
bits indexed by the travel-type ID (§4.6). In particular, do not substitute the job-search `0x1CF` masks,
wealth flags, strategy ID 0 or tuned MAX_TRIP_LENGTH. These are fresh-factory defaults, not state left
over from the most recent job search. Start/end rectangles come from the existing CheckCommute rule in
§4.7; no default rectangle contents are promised.

Init (`0x006D78B0`–`0x006D7951`) does **not** overwrite any setting in this table. On its first call it
sets the initialized byte before querying global city's slots 93/94 (`+0x174/+0x178`) for cell counts,
allocates zero-filled uint32 distance vectors of counts X+2 and Z+2, points each working table one element
past its vector's start, and returns true in AL. A repeated Init returns false in AL and leaves the existing
tables/settings intact. There is no null-city guard or recoverable allocation-failure return in this body.
The creating traffic pointer is assigned **after** Init; Init uses the global city, not that pointer.

The null cost pointer means **use the creating traffic simulator's starting-cost array**, not zero costs:
FindPath selects the nine float32 values at traffic `+0xC78` through `+0xC98`, and adds the request's float32
maximum trip length to each, storing nine float32 limits (`0x006D923C`–`0x006D92B3`). For CheckCommute the
added value is +0.0; the starting costs themselves remain traffic configuration, not new MySim constants.
CheckCommute's flag 1 bypasses AddTripNode's cost-budget rejection (`0x006D9007`–`0x006D905A`), so zero
maximum trip length does **not** imply a forced zero-length commute or immediate failure. Maximum work
time 0 disables the timer-expiry/suspension branch (`0x006D9C89`–`0x006D9CB8`); it is not an immediate
timeout. Strategy ID 3 runs SetupDistanceTables and copies traffic's float32 heuristic weight at `+0xCF8`;
ID 0 first runs FindNearestStandardDest (`0x006D93B9`–`0x006D9412`). Do not conflate these strategies.

Failure distinction: the native factory has no normal false/null-output branch. If its initial allocator
returns null, it sets its local object pointer to null and then dereferences it to call AddRef
(`0x0070D871`–`0x0070D886`); it does not publish a null output and return false. A null `out` is also
unguarded at the final write (`0x0070D897`). Do not translate these into CheckCommute's null-output branch:
that branch specifies how the caller handles a supplied service returning no object, not a native factory
allocation-failure result. No result register from Init or CreatePathFinder substitutes for the path search.

### 4.8 Life events: script commands *[compat]*

The advice scripts drive life events. When an advice with a non-zero `command` activates, the advisor system posts
`AdviceCommand` `0x8A09FA6C` (advisors.md §6.4); the agent routes it to that MySim's `DoScriptCommand(cmd)`
(slot 41), which does nothing when state = 0. Command IDs (SDK enum; names also in `adv_const.lua`
`game_commands.MYSIM_*`):

| Command | ID | Effect |
|---|---|---|
| MoveOut | `0x6A47E79B` | `FindNewResidence()` (slot 5), not a plain move-out *[compat]* |
| Retire | `0xAA4934B4` | `QuitJob`, retired := true, update location tracker |
| Die | `0xEA4D269C` | the city's automata controller manager (`cISC4City` slot 47) gets the trigger **`my_sim_died`** at the home position (spawns a hearse, game data `my_sim_died.lua`; visual only); ghost effect; `LeaveCity` |
| LeaveCity | `0x0A57DE1F` | `LeaveCity` |
| Spawn | `0x6A495F7F` | next generation: retired := false, generation += 1, bio := empty, age := residential `GetAverageNewAgeByWealth(wealth)` (residential.md §6.5; AL uint8), `QuitJob`, ghost effect, post `MySimsChanged` |
| IncreaseWealth | `0x6A4A5E9E` | if wealth < 3: wealth += 1, `QuitJob`, `FindNewResidence` |
| QuitJob | `0x6A5E6860` | `QuitJob` |
| other | — | ignored |

**Where randomness and thresholds live.** The engine draws no random number for death, retirement, spawn or
wealth. The shipped scripts use `tuning_constants.MYSIM_*` (ages, HQ/EQ thresholds, desirability bands; game data
in `adv_const.lua`) and the advice system's own frequency/once/random fields (advisors.md). So these rules are
**data**, not engine behaviour; FreeCity4 must run the same scripts to reproduce them.

### 4.9 Building filter and local building distances

**`cMySimBuildingFilter(residentialOnly)`** accepts an occupant when all hold:

- its type is building (`0x278128A0`) and it answers `0x87DDA3A9`;
- its lot (lot manager `GetOccupantLot`) is missing, or the lot's condition is ≤ 3 (not at the last abandonment
  stage; crime.md §6.7 condition values);
- `GetCompletionPercent()` ≥ 1.0 (fully built);
- if `residentialOnly`: the building profile's purpose is 1 (Residence).

**`UpdateLocalBuildings`** (part of the full refresh; only with a residence): clear the map; query the occupants
in the cell box `homeCell ± 20` (occupant manager slot 40, both corners inclusive *[inferred]*) with
`cMySimBuildingFilter(false)`; for each building, `d :=` distance (§4.10); for each occupant group of the
building's exemplar (building development simulator slot 14 → exemplar key → `GetOccupantGroups`):
`map[g] := d` if `map[g] = 0` or `d < map[g]`.

*[quirk]* A stored 0 counts as "unset": a building at distance 0 (the home itself, whose centre is the home
position) is overwritten by the next building of the same group. So `DistanceToClosest(residential group)` rarely
returns 0.

**`DistanceToClosest(g)`** (slot 50) → `map[g]`, or `0xFFFFFFFF` (−1) when state = 0, there is no map, or `g` is
absent. **`DistanceToClosestSpecial(type)`** (slot 51) → minimum distance over the building development
simulator's special-building list (`cISC4City` slot 53 → BDS slot 44 `GetSpecialBuildings`) of buildings whose
`GetBuildingType()` equals `type`; `0xFFFFFFFF` when none. That list holds exactly the occupants in occupant group
`0x150A` (landmark) or `0x150B` (reward), excluding previews (building-development.md §3, messages
`0x99EF1142`/`0x99EF1143`), in insertion order. Per entry (`0x00426AE0`): query `cISC4BuildingOccupant`
(`0x87DDA3A9`, entries that fail are skipped), compare its type, take its position, distance =
`sqrt(dx² + dz²)` (x and z only, home position minus building position, both float32; the products, sum and root
stay on the x87 stack, no float32 rounding before `__ftol`) truncated toward zero to an integer;
the result is the **unsigned** minimum, starting from `0xFFFFFFFF`.

**`LocalSchoolGrade(group)` / `LocalHospitalGrade(group)`** (slots 52/53): over the residential simulator's school
(slot 47) / hospital (slot 49) building list (all groups, including pending), take the building in occupant group
`group` (`IsOccupantGroup`) with the smallest distance (strict `<`, list order); return its query-data
**grade** (`GetSchoolQueryData` / `GetHospitalQueryData`, slots 26/27, field "grade" of
residential-education-health.md §4.11) as `SC4Percentage`. No building → 0.0. A failed query → 0.0 (the record
is zero-initialised).

### 4.10 Distance

`DistanceToOccupant(o)`: `o.GetPosition()`; `dx := homeX − px`, `dz := homeZ − pz` (float32, §18),
`d := sqrt(dx·dx + dz·dz)` computed on the x87 stack and truncated by `_ftol` to an unsigned integer (metres).
The home **position** (not the cell) is used. The disaster/fire/pipe/tornado handlers use the same formula with
their event position (clamped below at 0, converted to unsigned with the usual large-value handling).

### 4.11 Tuning (read in MySim Init, from Lua) *[compat]*

No exemplar and **no response curves** are loaded (so no foundations §4.1 pattern applies). The MySim reads the
Lua table `tuning_constants` (game data, `adv_const.lua`) with a table property reader; each value has a code
default that stays when the key is missing *[inferred]*:

| Key | Code default | Shipped | Used by |
|---|---|---|---|
| `MYSIM_BALLOON_FADE_DURATION` | 300 | 300 | balloons (UI) |
| `MYSIM_BALLOON_SHOWN_DURATION` | 1500 | 2500 | balloons (UI) |
| `MYSIM_BALLOON_HIDDEN_DURATION` | 300 | 300 | balloons (UI) |
| `MYSIM_TRIP_MIN_TIME_AT_LOCATION` | 1 | 30 | dispatch |
| `MYSIM_TRIP_MIN_TIME_AT_TRANSIT_SWITCH` | 1 | 1 | dispatch |
| `MYSIM_TRIP_MIN_TIME_IDLE` | 1 | 1 | dispatch |
| `MYSIM_TRIP_DESTINATION_PROXIMITY_MIN` | 10 | 4 | dispatch |
| **`MYSIM_TRIP_LOCAL_AREA_SIZE`** | 10 | 10 | the trip-variable square (§4.6) |
| `MYSIM_TRIP_APPROACH_DISTANCE` | 10 | 8 | dispatch |
| `MYSIM_TRIP_DEFAULT_CAR_MODEL` | 0 | `0x106A0000` | automata |

The five dispatch values have the following effects; they are signed 32-bit values with no added
nonnegative clamp. Object offsets below are interoperability facts, not required replacement layouts.

| Tuning suffix (`MYSIM_TRIP_`) | Object field | Effect |
|---|---|---|
| `MIN_TIME_AT_LOCATION` | `+0x20` | `GoTo` raises a copied destination's duration to this minimum **only when it has an occupant**; destination selection/arrival derives deadlines from location durations. `AddDestinationLocation` alone does not apply this clamp |
| `MIN_TIME_AT_TRANSIT_SWITCH` | `+0x24` | Transit waiting deadline = now + this value; switches to public modes 2/3/6/7/8 use this delay |
| `MIN_TIME_IDLE` | `+0x1C` | Idle/retry deadline = now + this value; switching to foot/car uses this delay |
| `DESTINATION_PROXIMITY_MIN` | `+0x18` | Signed low-32-bit square compared against the truncated **linear** cell-rectangle distance of §4.6.2. Idle completion tests distance < square; final route-marker handling accepts distance ≤ square. No square root of the tuning or square of the measured distance is substituted |
| `APPROACH_DISTANCE` | `+0x14` | On a changed automaton cell, when not player-driving: flag `0x8000` = ((x − destination.x)² + (z − destination.z)² < value²), using signed 32-bit products/sum. Equality clears the flag; §4.6 turns a rising edge into `MYSIM_APPROACHING_LOCATION` |

Evidence: the five named tuning assignments in MySim Init; duration clamp in `0x00437130`;
deadline additions `0x00435CE7`–`0x00435CF2`, `0x00435F1F`–`0x00435F3A`, `0x00436333`–`0x00436357`;
proximity comparisons `0x00435B94`–`0x00435BA3`, `0x004361FF`–`0x0043620F`; approach products and strict
comparison `0x004377E4`–`0x0043781B` (message `this` is object+8). Do not apply the approach test on every
minute message or substitute world coordinates for cells.

The other `tuning_constants.MYSIM_*` values (ages, EQ/HQ/desirability thresholds, radii) are read only by the
scripts.

---

### 4.12 Sim template pool (`cSC4MySimPool`, vtable `0x00A85B14`) *[compat]* (answers MS10)

The pool feeds the pick-sim UI; it never touches the simulation. Interface slots: 6 `GetMySimTemplates(vector&)`
`0x0043BBD0`, 7 `Import(uint32& count)` `0x0043BC40`, 8 `bool FindTheSimsInstallation()` `0x0043B200`.
A template (`cISC4MySimPool::tMySimTemplate`, 0x30 bytes; layout read from the stores in `EnumKeysCallback` and
`LoadImports` and the loads in `cSC4PickSimWinProc::PickHead` `0x007757F0`): portrait bitmap ID (u32, `+0x00`),
portrait file name (string, `+0x04`), name (string, `+0x18`), is-male (u8, `+0x2C`), zodiac sign (u8 0–11,
`+0x2D`).

**`GetMySimTemplates(out)`**: while the pool's loaded flag (`+0x08`) is clear: (1) if the resource manager exists,
query it for interface `0x6534284A` and enumerate its keys (`EnumKeys`, slot `+0x1C`) with `EnumKeysCallback`
(`0x0043B710`); no manager or no interface → no built-ins, step 2 still runs; (2) `LoadImports` (`0x0043B860`);
(3) set the flag. Then `out` is **replaced** by a copy of the cached vector (vector assignment `0x0043B580`, not an
append): built-ins first, in key-enumeration order, then imports in file order. The cache is the vector at pool
`+0x0C`.

`EnumKeysCallback(key)`: only keys of type `0x6A297266`. Load the property holder (`0xA52160F5`); keep it only if
property `0x10` (exemplar type) = `0x26`. Fields: bitmap ID := property `0xEA296F8D`; name := the localized string
named by property `0xCA416B3F` (`SC4LoadExemplarString` with the key's group/instance), else property
`0xEA296F8F`; is-male := property `0xEA296F91`; sign := property `0xEA296F90`. Appended.

Absent properties (answers MS12 a): every `cSCPropertyHelp::GetPropertyValue` (`0x005FD480` u32, `0x005FD570`
string, `0x005FD390` bool, `0x005FD3C0` u8) returns false and **writes nothing** when the property is missing, and
`SC4LoadExemplarString` writes the name only on success (crime.md §7). The template constructor (`0x008B5E70`)
initialises only the two strings (empty) and `+0x28`; bitmap ID `+0x00`, is-male `+0x2C` and sign `+0x2D` are
**not initialised** (stack bytes of the callback frame). So: name absent (no LTEXT and no `0xEA296F8F`) → empty;
portrait name of a built-in → always empty; bitmap ID / is-male / sign absent → indeterminate in the original
*[compat-undefined]*; a reimplementation uses 0 / false / 0 (shipped MySim exemplars carry all four).

**`LoadImports`**: open `<GetMySimDirectory()>MySims.dat` with access read, creation "open existing", share read (framework-rz-files.md §2.3). Missing file or first 4 bytes
≠ `0x4A32E4AA` (little-endian `AA E4 32 4A`) → nothing added. Then records until any read fails (a truncated
record is dropped):

| Field | Bytes |
|---|---|
| name length `n` | u8 |
| name | `n` bytes, no terminator (`"<first> <family>"`) |
| is male | u8 (≠ 0 → true) |
| zodiac sign | u8 |
| portrait length `m` | u8 |
| portrait name | `m` bytes; the template's portrait file name is these bytes with `.bmp` appended |

Imported templates have bitmap ID 0.

Path bytes (all joins are plain byte concatenation; nothing is inserted, case is never changed):

- Data file: the application's `GetMySimDirectory()` (app slot `+0xF4`) followed by the 10 bytes `MySims.dat`
  (a static string built at start-up from `0x00A85CF0`, held at `0x00B20E6C`; joined by `0x00408AE0`). The
  directory already ends in `\` (application.md §9: `<user dir>MySim\`), so the file is `<user dir>MySim\MySims.dat`.
  `Import` writes the same path.
- Template portrait (`+0x04`): only the record's `m` bytes followed by the 4 bytes `.bmp` (`2E 62 6D 70`,
  `0x00A85B38`, appended by `0x0090F053`). `LoadImports` stores **no directory**; the record bytes are taken as they
  are (no lower-casing, no separator check; an empty name gives `.bmp`).
- The pick-sim dialog resolves it (`PickHead` `0x007757F0`, also `SetupPickSims` `0x007751A0`): bitmap ID ≠ 0 → the
  image is the resource keyed by that ID (`0x00602B70` with `0x46A006B0`); bitmap ID = 0 → full path =
  `GetMySimDirectory()` followed by the portrait file name (`0x004081D0` append, no separator added), loaded as a
  file. A template's portrait directory is therefore the MySim directory at display time, not at load time.

**`FindTheSimsInstallation`**: read the registry string `HKEY_LOCAL_MACHINE\SOFTWARE\Maxis\The Sims\SIMS_DATA`; if
that fails, `...\The Sims\InstallPath` (the same `HKLM\...\value` reader as the SC4 install dir, application.md);
store it (pool `+0x18`), convert its encoding; if the directory does not exist the stored path is cleared and the
result is false. No value → false. Otherwise true.

**`Import(count)`**: `count := 0`; load the "male" text (localized string group `0xEA231E96`, instance
`0x6A7630C6`); `FindTheSimsInstallation` false → return. Ensure the MySim directory exists (create it). Create the
image converter (GZCOM class `0xA2FFB5D3`, interface `0x02FFBB60`); none → return. Open `MySims.dat` for **write,
create-always (truncate), share read** and write the 4 magic bytes; failure → return. Then for `i` = 1…99:

1. Directory = `Sprintf("%s%sUserData", path, "\")` for `i` = 1, `Sprintf("%s%sUserData%d", path, "\", i)`
   otherwise (`0x0043C04F` / `0x0043C072`; `path` = pool `+0x18`, `i` plain decimal): the `\` is **always**
   inserted, so a path already ending in `\` gives `...\\UserData` (double separator kept, passed as is to
   `DoesDirectoryExist`); missing → next `i`. Call this `<ud>` below. (answers MS12 f)
2. List `<ud>\Export\` (`<ud>` + `\` + `Export` + `\`) with filter `*.pub`. For each entry: `F.ini` = that
   directory + the entry name with its **last 4 bytes** dropped (`Left(len − 4)`, `0x0043C3CA`) + `.ini` — the
   `.ini` sits next to the `.pub` in the `Export` directory (answers MS12 d). For a listed name shorter than
   four bytes, the subtract wraps to a large unsigned count: `Left` compares **unsigned** with the string
   length and returns the **entire name**, not an empty stem (`0x0090F46D`–`0x0090F478`, answers MS14 c).
   Thus listed name `abc` gives `abc.ini`; an empty listed name gives `.ini`. It must exist (open read / open
   existing) and parse as an ini file, else skip the file.
3. `[WebPages] FamilyDescription` is required (else skip the file); its encoding is converted, then **one**
   trailing `/` is cut (`EndsWith("/")` → length − 1; `\` is not cut). The family's picture folder is, byte for
   byte, `<ud>` + `\` + `Web Pages` + `\` + value + `\` + `FamilyGFX` + `\` (`0x0043C5AD`–`0x0043C64E`): it
   lies under the **UserData directory**, not under the install path (answers MS12 c).
4. `[FamilyInfo]`: `sims_family_nummembers` (integer), `sims_family_name`. For member `k` = 1…n:
   `sims_familymember<k>_name`, `_gender`, `_zodiac` (lower-cased), `_guid` (integer; missing → skip member).
   Portrait: convert the source `cRZPath(<picture folder>)` with extension `.jpg` and file name
   `Sprintf("family%d_face", k)` (paths.md §2: `GetPath` = resolved folder + name + `.jpg`, e.g.
   `<ud>\Web Pages\<value>\FamilyGFX\family2_face.jpg`) to the destination `cRZPath(GetMySimDirectory())` with
   extension `.bmp` and file name `Sprintf("%s_%x", first, guid)` (`0x0043ABE0`); failure → skip member.
   Ini reads (answers MS12 b) use `cRZIniFile` `ReadEntry` (slot 6) and `ReadNumber` (slot 7) of
   framework-rz-files.md §3.5: section and key match ASCII case-insensitively; numbers are `sscanf("%ld")`
   (leading blanks, optional sign, decimal only, stops at the first non-digit; a miss writes nothing).
   `_guid` uses the `ReadNumber` result (false → skip member). `sims_family_nummembers`'s result is **not
   tested**; **n starts at 0 for each listed family** (answers MS14 a: zero store `0x0043C27E`, with 16 bytes of
   outstanding pushed arguments; read-output address `0x0043C761`; value loaded `0x0043C83E`). Missing or
   non-numeric `sims_family_nummembers` therefore leaves n = 0 and imports no members, even after a previous
   family with a valid positive n. The member loop runs `k = 1` while `k ≤ n` **unsigned**
   (`0x0043C84D`, `0x0043CE8A`); negative parsed n is reinterpreted as uint32. For n = `0xFFFFFFFF`, k wraps
   through zero and the loop never terminates by its bound *[quirk]*; do not simulate billions of iterations
   as a conformance test.
   The actual member-key formats are `sims_familymember%d_name`, `sims_familymember%d_gender`,
   `sims_familymember%d_zodiac`, `sims_familymember%d_guid`; the face format is `family%d_face` (answers MS14 b;
   calls `0x0043C9D3`, `0x0043CA4B`, `0x0043CB66`, `0x0043CBEA`, `0x0043CC92`). Although the bound is unsigned,
   `%d` interprets k as signed int32: k = `0x80000000` gives `sims_familymember-2147483648_name` and
   `family-2147483648_face.jpg`. This is a formatting vector, not a request to execute the import loop to k.
   **Every member text read is required** (answers MS14 d): `_name` failure skips before `_gender`;
   `_gender` failure skips before `_zodiac`; `_zodiac` failure skips before `_guid`; `_guid` failure skips
   before portrait conversion. A present empty value is different from a missing key: successful empty
   gender/zodiac values continue to the localized comparisons below. Branches: `0x0043CA2B`–`0x0043CA2F`,
   `0x0043CAA6`–`0x0043CB44`, `0x0043CBBE`–`0x0043CBC2`, `0x0043CC22`–`0x0043CC7B`.
5. Write one record (table above): name `"<first> <family>"`, is male := gender equals the "male" text
   (`cRZString::IsEqual` `0x0090EC57` called at `0x0043CDC2` with the case-sensitive flag = 0; see "Gender test"
   below), sign := zodiac index
   (below), portrait name `<first>_<guid>`; `count += 1`. Each string is written as a u8 length = **byte length
   mod 256** followed by that many bytes (`0x0043CD79`, `0x0043CE0C`): a 300-byte name is stored as length 44 and
   its first 44 bytes, not cut to 255 (answers MS12 e). The record stays readable by `LoadImports`.
6. Close and delete `F.ini`.

Zodiac index: compare the lower-cased value with the 12 localized names (group `0x6A231EAA`, instances
`0xEA822E90 0xCA822E93 0xCA822E97 0xAA822E9B 0xCA822EA4 0xAA822EA8 0xCA822EAC 0xCA822EAF 0xCA822EB3
0x6A822EB6 0x6A822EB9 0xEA822EBD`, table `0x00A85AA4`), each lower-cased; first exact match → its index 0–11; none
→ 0 (a debug text "Unknown zodiac symbol" is formatted, nothing shown).

Text that fails to load (answers MS12 g): the "male" text and the zodiac names are `cSLString` objects
(`0x00603580`, neighbors.md: a missing LTEXT leaves the string **empty**). An empty "male" text makes exactly the
members with a **present, successfully read** empty `_gender` male; an empty zodiac name matches a present empty `_zodiac` value (first such index
wins), and no longer matches anything else.

**Gender test** (`IsEqual(const cIGZString&, bool caseSensitive)` `0x0090EC57`; the flag pushed at `0x0043CDB2` is
`EBX`, zeroed at `0x0043BC48` and never written again in `Import`): lengths differ → false. Flag ≠ 0 would compare
the bytes with `strncmp`; flag = 0 (this call) copies both strings, lower-cases each with `cRZString::MakeLower`
(`0x0090EAC5`, graphics-formats.md §3.11: all bytes < `0x7F` → ASCII `A`–`Z` folded) and compares the bytes with
`memcmp`. So the gender value is male iff it equals the localized "male" text ignoring ASCII case, same length, no
trimming. The gender value itself is not lower-cased before the call (`Import`'s only `MakeLower` is
`0x0043CBDA`, the zodiac value).

**Cache after `Import`** (`0x0043CFEE`–`0x0043D006`): after the 99 directories, the output file and the converter
are released; then if `count > 0` **and** the loaded flag is set, the cached vector is cleared (`0x0043BB90`:
every template destroyed, end := begin, capacity kept) and the loaded flag is reset to 0. The next
`GetMySimTemplates` therefore re-enumerates the exemplars and re-reads the new `MySims.dat`. If `count = 0`, nothing
is reset: a cache already loaded keeps the old imports although `MySims.dat` was just rewritten to the magic alone
(quirk; they disappear at the next start). If the flag was clear, nothing to clear; the first `GetMySimTemplates`
reads the new file. The pick-sim dialog keeps its own copy (`cSC4PickSimWinProc` `+0x48`), which it clears with
the same `0x0043BB90` and refills with `GetMySimTemplates` (`0x0077694C`, `0x0077699E`).

Examples (conformance):

| Case | Input | Result |
|---|---|---|
| record | `MySims.dat` = `AA E4 32 4A 08 'Bob Newb' 01 05 09 'Bob_1a2b3'` | one template "Bob Newb", male, sign 5, bitmap 0, portrait `Bob_1a2b3.bmp` |
| magic only / wrong magic | `AA E4 32 4A` / `00 00 00 00 ...` | no imports |
| portrait bytes kept | portrait bytes `'BOB_FF'` | portrait `BOB_FF.bmp` (case kept) |
| display path | MySim dir `C:\U\SimCity 4\MySim\`, portrait `Bob_1a2b3.bmp`, bitmap 0 | file `C:\U\SimCity 4\MySim\Bob_1a2b3.bmp` |
| gender | "male" text `Male`; values `male`, `MALE`, `Male` / `Male ` , `m`, `female` | male / not male |
| cache, count 2, loaded | `GetMySimTemplates` called before `Import` | cache cleared, flag 0; next call returns built-ins + 2 new imports |
| cache, count 0, loaded | as above, no family found | flag stays 1; next call still returns the old imports |
| cache, not loaded | `Import` before any `GetMySimTemplates` | nothing cleared; first call reads the new file |
| UserData path | path `D:\Sims\`, i = 1 / 3 | `D:\Sims\\UserData` / `D:\Sims\\UserData3` (double `\` kept) |
| ini next to pub | `<ud>\Export\Smith.pub` | `<ud>\Export\Smith.ini` |
| picture folder | FamilyDescription `smith1/` | `<ud>\Web Pages\smith1\FamilyGFX\`, source `...\FamilyGFX\family1_face.jpg` |
| long name | first+family = 300 bytes | record length byte 44, 44 name bytes |
| ini number | `sims_familymember1_guid = 0x1F` / ` 42abc` / missing | 0 / 42 / member skipped |
| missing "male" LTEXT | gender `` / `Male` | male / not male |

## 5. Messages and events posted

### 5.1 Messages

| ID | Name | When | Fields | Consumers |
|---|---|---|---|---|
| `0xAA25827C` | MySimsChanged | `SetResidence` (new MySim), `LeaveCity`, `Spawn`, `DeleteFromGame` | none | MySim window, MySim input control (UI) |
| `0xEA3ADA33` | MySimMovedIn | `AnnounceMoveIn` | void1 = home occupant, data1 = is-male, data2 = wealth | audio event handler |
| `0x4BC42C48` | MySim update | trip state changes, arrival at a pylon | IGZUnknown = the MySim | the MySim itself (§3.3) |

No simulator subscribes to any of them (checked by scanning every use of the three IDs) *[compat]*.

### 5.2 Advisor events (`PushEvent(event, subject)`, advisors.md §6.3)

Subjects are new `cSC4AdviceSubject` objects (`0x6A26D049`/`0x6A26D050`). Names from `adv_const.lua`.

| Event | ID | Trigger | Subject |
|---|---|---|---|
| MYSIM_BUILDING_ADD | `0xCA4E8986` | `DoBuildingAdded(b)`, MySim has a residence | `SetObject(b)`, `SetData1(distance in metres)` (§4.10); read by `event_building_distance()` |
| MYSIM_FIRE_STARTED | `0xEA5E4E89` | `DoFireAdded(x, z)` | data1 = distance |
| MYSIM_PIPE_BURST | `0x6A5E598C` | `DoPipeBurst(x, z)` | data1 = distance |
| MYSIM_TORNADO | `0x0A5E8243` | `DoTornado(x, z)` | data1 = distance |
| MYSIM_RIOT | `0x8AC11A9B` | `DoDisaster(0xCA380B06 riot, cx, cz)` | `SetLocation(float(cx), float(cz))` — **cell indices, not metres** *[quirk]*; data1 = distance from the cell centre |
| MYSIM_VOLCANO | `0x2AC14492` | `DoDisaster(0xC4C90997 volcano, cx, cz)` | as riot |
| MYSIM_HOME_DESTROYED | `0xAA48F5C1` | home removed, cause 0 | null; then `FindNewResidence` |
| MYSIM_HOME_DESTROYED_BY_DISASTER | `0xAA4D2D64` | home removed, cause ≠ 0 | null; last local disaster := cause, `UpdateVariables` first, then `MoveOut` |
| MYSIM_JOB_DESTROYED | `0xCA48F879` | job removed, cause 0 | null; then `QuitJob` |
| MYSIM_JOB_DESTROYED_BY_DISASTER | `0xAA4D2D65` | job removed, cause ≠ 0 | null; last local disaster := cause, `UpdateVariables`, then `QuitJob` |
| MYSIM_MOVE_IN | `0xCA3E87A8` | `AnnounceMoveIn` | null |
| MYSIM_NEW_JOB | `0xAA3EDBB1` | job found (§4.7) | null |
| MYSIM_COMMUTE_FAILED | `0xCA4553B7` | `FailCommute` | null |
| MYSIM_CANT_FIND_HOME | `0x6A57DD94` | `FindNewResidence` without a new home | null |
| MYSIM_APPROACHING_LOCATION | `0xEC107DF9` | trip flag bit 15 set | null |

All handlers do nothing when state = 0. Other disaster types are ignored by `DoDisaster`. `DoBuildingChanged(b)`
only re-runs `UpdateVariables` when `b` is the residence or the job.

---

## 6. Persistence *[compat]*

### 6.1 Agent simulator

`Write` writes `u16 1` and nothing else; `Read` (a shared body) requires version 1. `GetGZCLSID` = `0x29E37DEC`.
Nothing in the shipped game writes a reference to the agent, so **no record exists in saves** *[compat]*.

### 6.2 MySim record (class `0x4A1DBBBF`, written by the class store because the advisor system holds a ref)

Version **17** (`0x11`). All integers little-endian; "ref" is foundations §2.7; "key" is three u32 (§2.5);
"bool" is a u8 0/1.

| # | Type | Field |
|---|---|---|
| 1 | u16 | version = 17 |
| 2 | u8 | state (the int truncated to a byte). **If 0, the record continues directly with the advisor part (row 32).** |
| 3 | u8 | slot |
| 4 | ref | residence (read as `cISC4BuildingOccupant` `0x87DDA3A9`) |
| 5 | u32, u32 | home cell x, z |
| 6 | ref | job (`0x87DDA3A9`) |
| 7 | s32 × 4 | neighbour city cell x, z; commute destination cell x, z |
| 8 | u32 | jobless days |
| 9 | key | vehicle key |
| 10 | u8 | wealth |
| 11 | u16 | generation |
| 12 | u8 | zodiac sign |
| 13 | bool | is male |
| 14 | GZStr | bio |
| 15 | u8, u8 | age, birth month |
| 16 | u32 | portrait bitmap id |
| 17 | GZStr | portrait path |
| 18 | u32 | last commute time |
| 19 | bool | retired |
| 20 | u32 | last local disaster |
| 21 | f32 × 3 | home position x, y, z |
| 22 | u8, f32, f32 × 6 | EQ ring index, EQ delta, EQ ring |
| 23 | u8, f32, f32 × 6 | HQ ring index, HQ delta, HQ ring |
| 24 | key, key | car key, pedestrian key |
| 25 | s32, s32 | reserved pair (dead data, §2.2) |
| 26 | ref | dispatch (`0xEB8B6F9A`) |
| 27 | u8, u64, u8 | pylon timer: units, elapsed, running |
| 28 | bool | has dispatch tool (tool exists and its slot `+0x40` is false) |
| 29 | ref | dispatch tool (`0xC9B6D7D6`), only if row 28 is true |
| 30 | bool | pylon locked (§2.2) |
| 31 | s32 | last player trip day |
| 32 | — | the `cSC4Advisor` record (advisors.md §11): advices, personal name, … |

**Read** accepts every version (no lower or upper check) *[quirk]*:

- rows 1–23 always;
- version 11–14: five s32 are read and discarded instead of row 24; version ≥ 15: row 24; version < 11: neither;
- version ≥ 17: row 25;
- version < 12: no rows 26–29; the existing dispatch is released (it is recreated when the MySim initialises);
- version 12: the dispatch is read **inline** (`cIGZSerializable::Read` on the existing dispatch), then rows 27–29;
- version ≥ 13: the existing dispatch is shut down and released, row 26 read as a ref, then rows 27–29;
- version ≥ 14: row 30; version ≥ 16: row 31;
- then the advisor part. The current EQ/HQ, the Lua table, the local building map and the neighbour city are
  **recomputed**, not read.

*[choice]* FreeCity4 writes version 17 only and reads versions 11–17; it rejects other versions in standalone mode.

---

### 6.2.1 Dispatch record (`0xCBC14674`) *[compat]*

This is the record referenced by MySim row 26, and the **same payload inline** in MySim version 12. It
starts with **u32 1**, not the MySim's u16 version. All scalars are little-endian; `ref`, `GZStr` and
resource keys use foundations §2. There is no raw pointer, C++ padding, RNG state or container capacity
in the payload. Write returns whether stream status is clear. Read consumes the version but has no
version-equality rejection (`0x00432440`–`0x0043285E`, `0x00434760`–`0x00434B9C`).

For the opaque scalars below, preserving their bits is required even though their runtime meaning has
not yet been settled. Offsets identify the **complete object**; the serializer receives object+`0x0C`.
This distinction matters: the first location is object+`0x5C`, serializer+`0x50`.

| Order | Encoded type | Value |
|---|---|---|
| 1 | u32 | version 1 |
| 2 | bool (u8) | object `+0x10` auxiliary flag |
| 3 | 18 × u32 | In ascending object-offset order `+0x14` through `+0x58`: approach distance, proximity, idle minimum, location minimum, switch minimum; opaque `+0x28`; path indices `+0x2C/+0x30`; change counters `+0x34/+0x38`; tracking value `+0x3C`; current/previous state; flags; minute clock; travel type; wait timestamp; neighbour retry count. Signed fields retain their two's-complement bits; travel type occupies a full u32 here |
| 4 | 4 × location payload | current, home/default, active destination, commute destination, in that order |
| 5 | ref | auxiliary occupant at `+0xFC`, read with IID `0x26BB767E` |
| 6 | u32, GZStr | auxiliary value `+0x100`, string `+0x104` |
| 7 | ref | automata occupant `+0x118`, IID `0x26BB767E` |
| 8 | ref | owner MySim `+0x11C`, IID `0x8BC01FFE`; writing passes its serializable sub-object at owner+`0xB8` (null stays null) |
| 9 | ref | simulation service `+0x120`, IID `0x681BD53A` (not the transient pathfinder at `+0x124`) |
| 10 | u32 count; count × (u8,u8,u8) | route deque, in deque order; each element is cell x, cell z, travel type. No per-element alignment |
| 11 | u32 count; count × (s32,key,s32) | travel-type/model map, ascending signed key order. Resource key = three u32; the final s32 is model offset |
| 12 | s32 | route/switch marker at `+0x15C` |
| 13 | u32 count; count × location payload | destination queue in front-to-back order |

Each **location payload** is: occupant ref; world x/z as two f32; cell x/z as two s32; regional city x/z
as two s32; in-city and by-commute as two u8 booleans; duration and deadline as two s32. It has no own
version or padding (`0x004314F0`–`0x00431587`, read `0x00431590`). The occupant IID on read is
`0x26BB767E`. Equality and validity use §4.7.1, not bytewise equality of this payload.

Byte checks: version bytes are `01 00 00 00`; the auxiliary flag is byte 4; the first tuning u32 begins
at byte 5; the first location begins at byte **77**, regardless of reference encoding, because no ref
precedes it. −1 trip state is `FF FF FF FF`, not `FF`; a 0xFE travel type is `FE 00 00 00`.
Record count arithmetic enumerates elements, not the implementation's deque blocks (`0x004325B8`–`0x004326BB`,
`0x00432763`–`0x0043283B`); map entries are key + resource key + offset (`0x00432724`–`0x0043274C`).

Read registers for `0x26D31EC1` and records whether the stream succeeded. Successful deferred setup on
that message calls Init, clears the pending flag, unregisters the message, then sends a flags notification
with old = 0 followed by a state notification with old = **−2**, using the restored current values
(`0x004378CE`–`0x0043792A`). Do not replace the restored trip with an immediate home arrival on loading.

## 7. Interfaces *[compat]*

Return channels were read from the epilogues; pops come from `abi_extract.py --check`. "AL" = bool in AL only.

### 7.1 `cISC4MySimAgentSimulator` (vtable `0x00A85544`, 13 interface slots + destructor)

| Slot | Off | Method (as the game expects it) | Pop | Ret | Address | Behaviour / SDK |
|---|---|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface(iid, void**)` | 8 | AL | `0x0042E310` | §1.1 |
| 1 | +0x04 | `AddRef()` | 0 | EAX | `0x004F7BA0` | |
| 2 | +0x08 | `Release()` | 0 | EAX | `0x0042E410` | §1.1 |
| 3 | +0x0C | `bool Init()` | 0 | AL | `0x0042EC30` | §1.3 |
| 4 | +0x10 | `bool Shutdown()` | 0 | AL | `0x0042E500` | unregister Lua functions, unsubscribe, release the city and the six MySims |
| 5 | +0x14 | `bool GetActiveMySims(SC4Vector<cISC4MySim*>&)` | 4 | AL | `0x0042F4A0` | appends (AddRef'd) every MySim among 0–4 with `IsActive`; true if any |
| 6 | +0x18 | `void MoveIn(name&, isMale, sign, bio&, bitmapID, path&, slot)` | 28 | — | `0x0042E990` | §4.1 |
| 7 | +0x1C | `void TrackLocation(cISC4MySim*)` | 4 | — | `0x0042E9F0` | UI, all six |
| 8 | +0x20 | `void HideLocation()` | 0 | — | `0x0042EA70` | UI |
| 9 | +0x24 | `cISC4MySim* MySimLivesHere(cISC4BuildingOccupant*)` | 4 | EAX | `0x0042EB60` | first of 0–4 whose `GetResidence()` equals the building; else null. Not AddRef'd. |
| 10 | +0x28 | `void CancelMoveIn()` | 0 | — | `0x0042E3A0` | removes the move-in input control (UI) |
| 11 | +0x2C | `void SetActiveUIPanelSim(int32 slot)` | 4 | — | `0x0042E910` | active index := the first index 0–4 whose MySim `GetSlot()` equals `slot`, else −1 |
| 12 | +0x30 | **`cISC4MySim* GetActiveUIPanelSim(int32& index)`** | 4 | **EAX pointer** | `0x0042E960` | if the active index is in [0, vector size): `index :=` the **vector index** (not the slot), return that MySim (not AddRef'd); else return null, `index` unchanged. **SDK error** (§7.4). |
| 13 | +0x34 | (destructor) | 4 | | `0x0042F450` | not part of the interface |

Other vtables of the object: message target `0x00A85534` (DoMessage `0x0042EFA0`, pop 4, AL = 1), serializable
`0x00A8551C` (Write `0x0042E440`, Read `0x006B8A70`, GetGZCLSID `0x0042E300`).

The advisor writer's claim (advisors.md §14.3) that CitySituations `UpdateMySimAdvices` calls agent `+0x30` and
then MySim `+0x110` is consistent with this table: slot 12 returns the MySim pointer, and `+0x110` is
`ActivateLuaContext` (§7.2 slot 68).

### 7.2 `cISC4MySim` (sub-object vtable `0x00A85080`, 69 slots)

The compiled order equals the SDK order except that the two `GetJobTitle` overloads are grouped at the position
of the first declaration, in reverse order (foundations §16): slot 27 is `GetJobTitle(occ, str&)` and slot 28 is
`GetJobTitle()`. Therefore **SDK slots 28–54 sit one slot (+4 bytes) later** in the compiled table, and slots
≥ 56 match. This is the MSVC rule, so the SDK header compiled with MSVC/Intel produces the right layout; it is
not an SDK error, but a non-MSVC replacement must lay the table out as below.

| Slot | Off | Method | Pop | Ret | Address | Notes |
|---|---|---|---|---|---|---|
| 0–2 | | QI / AddRef / Release | 8/0/0 | | `0x00429D60`… | thunks to the object |
| 3 | +0x0C | `cISC4Advisor* AsAdvisor()` | 0 | EAX | `0x00429C90` | |
| 4 | +0x10 | `void MoveIn(slot, name&, isMale, sign, bio&, bitmapID, path&)` | 28 | — | `0x0042D880` | §4.1 |
| 5 | +0x14 | `bool FindNewResidence()` | 0 | AL = **0 always** | `0x0042A700` | §4.7 |
| 6 | +0x18 | `bool ChangeResidence(b)` | 4 | AL | `0x00426C10` | §4.7 |
| 7 | +0x1C | `bool TestForResidence(b, bool filter, bool matchWealth)` | 12 | AL | `0x00425CA0` | §4.7 |
| 8 | +0x20 | `bool ChangeJob(b)` | 4 | AL | `0x00425D70` | §4.7 |
| 9 | +0x24 | `bool DeleteFromGame()` | 0 | AL = 1 | `0x0042DF50` | §4.7 |
| 10 | +0x28 | `uint8 GetSlot()` | 0 | AL | `0x0040CE50` | shared getter body |
| 11 | +0x2C | `cISC4BuildingOccupant* GetResidence()` | 0 | EAX | `0x00517F70` | shared getter body, no AddRef |
| 12 | +0x30 | `cISC4BuildingOccupant* GetJob()` | 0 | EAX | `0x0060FB80` | shared getter body, no AddRef |
| 13 | +0x34 | `bool SetResidence(b)` | 4 | AL | `0x0042E020` | §4.1 |
| 14 | +0x38 | `bool GetLocation(float[3])` | 4 | AL | `0x004265D0` | UI/automata |
| 15 | +0x3C | `void GetVehicleKey(key&)` | 4 | — | `0x00424C80` | |
| 16–19 | +0x40… | `GetSign`, `GetAge`, `IsMale`, `GetWealth` | 0 | AL/AL/AL/EAX | `0x00424CA0`, `0x00424CB0`, `0x00424D90`, `0x006AAEB0` | |
| 20 | +0x50 | `bool IsActive()` | 0 | AL | `0x00449550` | state ≠ 0 |
| 21–26 | +0x54… | `GetBio`, `GetBitmapID`, `GetBitmapFile`, `GetBitmapBuffer(cIGZBuffer**)` (pop 4), `GetHomeName`, `GetJobName` | | | `0x00698CD0`, `0x00407AC0`, `0x00424CC0`, `0x0042BDB0`, `0x00425DD0`, `0x00425DF0` | UI |
| 27 | +0x6C | `void GetJobTitle(cISC4BuildingOccupant*, cIGZString&)` | 8 | | `0x00424BD0` | reordered overload |
| 28 | +0x70 | `SC4String* GetJobTitle()` | 0 | EAX | `0x00425E30` | |
| 29 | +0x74 | `GetEQAsText(cIGZString&)` | 4 | AL = 1 | `0x00424CD0` | `"%d"` of int(EQ). SDK `void` (harmless) |
| 30 | +0x78 | `GetHQAsText(cIGZString&)` | 4 | AL | `0x00424D00` | text by HQ band < 50 / < 100 / else (UI) |
| 31–36 | +0x7C… | `Find`, `ShowResidence`, `HideResidence`, `ShowLocation`, `HideLocation`, `SetPylonVisibility(bool)` (pop 4) | | | `0x004293B0`, `0x00429590`, `0x00425E70`, `0x0042C820`, `0x004288D0`, `0x00425F00` | UI |
| 37 | +0x94 | `UpdateResidence()` | 0 | | `0x00425EA0` | visual |
| 38, 39 | +0x98, +0x9C | `LeaveForWork()`, `ReturnHome()` | 0 | | `0x00426EF0`, `0x00426F60` | dispatch trips (automata) |
| 40 | +0xA0 | `AnnounceMoveIn()` | 0 | | `0x0042BF70` | §4.1 |
| 41 | +0xA4 | `DoScriptCommand(uint32)` | 4 | | `0x0042DB50` | §4.8 |
| 42 | +0xA8 | `DoBuildingAdded(b)` | 4 | | `0x00426060` | §5.2 |
| 43 | +0xAC | `DoBuildingChanged(b)` | 4 | | `0x0042C020` | §5.2 |
| 44 | +0xB0 | `DoBuildingRemoved(b, uint32 cause)` | 8 | | `0x0042C940` | §5.2 |
| 45–47 | +0xB4… | `DoFireAdded(x, z)`, `DoPipeBurst(x, z)`, `DoTornado(x, z)` | 8 | | `0x00426120`, `0x004261D0`, `0x00426280` | §5.2 |
| 48 | +0xC0 | `DoDisaster(uint32 type, int32 cx, int32 cz)` | 12 | | `0x00426330` | §5.2 |
| 49 | +0xC4 | `DoNeighborConnectionRemoved(int32, int32)` | 8 | | `0x00426460` | §4.7 |
| 50 | +0xC8 | `uint32 DistanceToClosest(uint32 group)` | 4 | EAX | `0x00427600` | §4.9 (SDK `int32`: same bits) |
| 51 | +0xCC | `uint32 DistanceToClosestSpecial(uint32 type)` | 4 | EAX | `0x00426AE0` | §4.9 |
| 52 | +0xD0 | **`SC4Percentage LocalSchoolGrade(uint32 group)`** (hidden result pointer first) | 8 | EAX = result pointer | `0x00429FE0` | **SDK error** (§7.4) |
| 53 | +0xD4 | **`SC4Percentage LocalHospitalGrade(uint32 group)`** | 8 | EAX = result pointer | `0x0042A200` | **SDK error** |
| 54 | +0xD8 | `PlayNightSound()` | 0 | | `0x00426EC0` | audio |
| 55 | +0xDC | `bool GetCurrentScreenLocationXY(int32*, int32*)` | 8 | | `0x00425F20` | UI |
| 56 | +0xE0 | `RequestToPlaySound()` | 0 | | `0x00424DE0` | audio |
| 57, 58 | +0xE4, +0xE8 | `SetCar(exemplar)`, `SetPedestrian(exemplar)` | 4 | | `0x00426660`, `0x004272B0` | automata |
| 59 | +0xEC | `GetModelButtonBitmapID(bool)` | 4 | | `0x00429CB0` | UI |
| 60–63 | +0xF0… | balloon getters | 0 | | `0x004275E0`, `0x004275F0`, `0x00427410`, `0x00427440` | UI |
| 64–66 | +0x100… | `CanUDriveIt`, `EnterUDriveIt`, `ExitUDriveIt` | 0 | | `0x00426A00`, `0x00426A20`, `0x00426A40` | UI (tail jumps into the dispatch) |
| 67 | +0x10C | `bool CreateDispatchTool()` | 0 | | `0x00426830` | UI |
| 68 | +0x110 | `bool ActivateLuaContext()` | 0 | AL | `0x00429830` | §4.4 |

**Lua callers** (advisors.md §8.3, verified): the `mysim_*` functions take the context advisor, QI it to
`0x0A1DA42F`, read one number argument with `_ftol`, and call: `mysim_distance_to_closest_building` → `+0xC8`,
`mysim_distance_to_closest_landmark_or_reward` → `+0xCC`, `mysim_local_school_grade` → `+0xD0` with
(result pointer, group) and pushes `grade × 100.0`, `mysim_local_hospital_grade` → `+0xD4` likewise,
`mysim_dispatch_tool` → `+0x10C` (wrappers `0x0040A410`, `0x0040A4F0`, `0x0040A5D0`, `0x0040A6B0`,
`0x0040A790`). So the compiled `+0xC4..+0xD4` slots are indeed the SDK slot + 4 bytes.

External pathfinder calls made by `CheckCommute` (MS15; `cISC4PathFinder`, vtable `0x00AB2150`;
complete branch/order contract in §4.7):

| Slot | Offset | Native entry | Meaning at this call site |
|---|---|---|---|
| 5 | +0x14 | `0x006D6CF0` SetFlag | OR flag 1 |
| 9 | +0x24 | `0x006D6D30` SetTripDestinationFilter | filter 3 |
| 18 | +0x48 | `0x006D6E00` SetTrafficDensity | density 1 |
| 19 | +0x4C | `0x006D6E10` SetStartRect | home occupant's bounding city cells, four int32 by value |
| 20 | +0x50 | `0x006D6E40` SetEndRect | job occupant's bounding city cells, or MySim's stored commute cell |
| 23 | +0x5C | `0x006D91B0` FindPath | only integer 1 is success |
| 26 | +0x68 | `0x006D6030` GetTripLengthScaledForDisplay | x87 result; called only after success |
| 2 | +0x08 | Release | after the success store or the entire FailCommute handler |

The caller supplies no destination flags, travel masks or maximum trip length. Their inherited factory
configuration and the factory/Init ownership and failure contracts are specified in §4.7.2 (MS16).

### 7.3 Other vtables of `cSC4MySim`

| Vtable | Address | Overrides of the base |
|---|---|---|
| `cISC4Advisor` (at +8 of the object) | `0x00A851C0` (21 slots) | 3 Init `0x0042CCF0` (base Init, then dispatch creation, Lua table, tuning §4.11), 4 Shutdown `0x00428C20`, 8 GetPersonalName `0x0042D000`, 19 **CommandWouldTrigger** `0x00424DA0` (false when state = 0; otherwise `ActivateLuaContext` then the base). The rest are the base's (advisors.md §13). |
| `cIGZSerializable` | `0x00A851A4` | Write `0x004251A0`, Read `0x00425490`, GetGZCLSID `0x00424AF0` (= `0x4A1DBBBF`) |
| `cIGZMessageTarget2` | `0x00A85194` | DoMessage `0x0042D150` |
| class (non-interface) | `0x00A85218` (17 slots) | 6 simulate_day `0x0042DC90`, 7 simulate_day_events `0x004286C0`, 8 monthly hook `0x0042DDF0`, 9 update_variables `0x0042CB10`; 10–16 MySim-private |

### 7.4 SDK errors (checked)

1. **`cISC4MySimAgentSimulator::GetActiveUIPanelSim`** (slot 12, checked at `0x0042E960`, pop 4): returns the
   `cISC4MySim*` in EAX (epilogue loads the vector element), not `bool`. The out parameter receives the vector
   index, not the UI slot. A caller using the SDK `bool` reads AL, which is 0 for any 256-aligned MySim pointer.
   Replacement: return the pointer (null when none) and write the index.
2. **`cISC4MySim::LocalSchoolGrade` / `LocalHospitalGrade`** (slots 52/53, checked at `0x00429FE0` and
   `0x0042A200`, pop 8, `ret 8`, EAX = the hidden pointer): the signature is
   `SC4Percentage F(uint32 occupantGroup)` returned through a hidden result pointer, not
   `void F(SC4Percentage&)`. Replacement: write the grade (float32) to the result pointer and return it.
3. Not errors, noted: `FindNewResidence` always returns false; `GetEQAsText`/`GetHQAsText` return AL = 1 where
   the SDK says `void`; the `GetJobTitle` overload order (§7.2); `MySimLivesHere` already returns a pointer in the
   SDK.

---

### 7.2.1 Trip and player-control entry points *[compat]*

`LeaveForWork` / `ReturnHome` are cISC4MySim slots 38/39 (`0x00426EF0`, `0x00426F60`). Their shared
`DispatchCommute` (`0x00426CE0`) fails only when there is no residence; in the other cases below it reports
success without testing the return of `AddDestinationLocation`.

| Entry/current MySim state | Request and observable MySim result |
|---|---|
| LeaveForWork, 1/2/5 | Call DispatchCommute; success sets state 4; failure plays the existing home refusal sound |
| LeaveForWork, any other state | No action |
| ReturnHome, 3/4 | Call DispatchCommute; success sets state 5 |
| ReturnHome, any other state | No action |
| DispatchCommute, 2 | Copy commute destination if valid and by-commute; otherwise make a fresh by-commute placeholder. Duration is 5 before hour 8 or after hour 17; otherwise (17 − hour) × 60. Hour 17 gives **0**, not 5. Replace destination queue with this location (options 4) |
| DispatchCommute, 1 | TeleportToCommuteLocation, then replace destination queue with home (options 4) |
| DispatchCommute, 3 | Replace destination queue with home (options 4) |
| DispatchCommute, other nonzero/zero state with residence | Success without changing the queue. Thus LeaveForWork in state 5 and ReturnHome in state 4 can change MySim state without issuing a new destination |

Hour arithmetic is integer, verified `0x00426D56`–`0x00426D8D`. Queue insertion does not itself imply
immediate departure (§4.6.1). Do not turn these entry points into unconditional GoTo calls.
On a successful LeaveForWork or ReturnHome state change, copy the dispatch tracking value (`+0x3C`)
to the MySim's last-player-trip field (`+0x150`); this is not a fresh wall-clock sample
(`0x00426F1E`–`0x00426F27`, `0x00426F60`).

**`TeleportToCommuteLocation` (MS19, `0x00431C70`–`0x00431CF6`).** This void operation
copies the complete stored commute location to current, even if invalid. It makes no
validity check, search, queue edit, path reset, automaton release or arrival notification.
Home, commute and active destination are unchanged. Travel becomes `0xFE`, current's
deadline becomes now + its copied duration (wrapping 32-bit addition), previous state
becomes the old state, state becomes 3, and flags gain bit 2. All those state writes
precede callbacks. Changed flags emit `0x4BB2D020(oldFlags,newFlags,0)` first; changed
state then emits `0x4BB2CFC0(oldState,3,0)`, under §3.3's owner delivery gate. The change
counter gains one for a changed travel type, one unconditionally for the copied location,
and one after each changed flags/state notification attempt, even without an owner.
There is no boolean failure result. DispatchCommute in MySim state 1 subsequently queues
home with options 4 and returns success under its residence guard; an invalid stored
commute does not prevent that queue request or cause job validation here.

**`GoTo(location)`** (`0x00437130`) copies the argument and applies the occupant-only duration minimum of
§4.11. Outside dispatch state 2 it expires the current dwell by setting deadline = now − 1, clears the
queue, queues that copy, and returns true. In state 2 it clears the queue and replaces the active destination.
If the held automaton accepts cancellation (`0xA9B40F05`, slot `+0x7C`, argument true), it clears flags
`0x5800` and then `0x4000`, separately notifying changed masks, and returns true. Otherwise it clears
`0x5800`, sets `0x4000`, and runs an update. An empty resulting route or equal route indices invokes
setStuck and returns false; otherwise true. It does not overwrite current location with the new destination.

**U-Drive-It** (`0x00431D00`, `0x00437340`, `0x00434360`):

- CanUDriveIt is true in state 2 only with an automata occupant answering `0xA9B40F05` and travel type 1
  (Car); in state 3 it is true only when current equals home or commute destination. Otherwise false.
- Enter first applies that guard. At home it sets pending-player flag `0x20000` then GoTo(commute); at
  work it sets that flag then GoTo(home). These branches return true even if GoTo reports false. The
  later held-automaton update clears the pending flag and re-enters player control. In state 2 the
  existing automaton is handed to the player-drive/mission service, then active-player flag `0x10000`
  is set; no new destination is queued. The trip update pauses automatic progression while it is set.
- Exit with active-player flag set detaches player control, clears the queue, copies home to both current
  and active destination, sets travel type `0xFF`, enters state −1, resets the path and releases the
  automata occupant. Then, whether active or only pending, it clears **both** flags `0x30000` and returns
  true. State notification precedes that final flag notification. Exit does not directly emit an arrival
  notice; recovery runs on a subsequent state update.

Dispatch-tool calls use this same GoTo destination contract. The remainder of transport selection,
visual automaton construction, and mission-service callback details are MS18, not permission to report
arbitrary successful arrivals. Conformance inputs must distinguish accepted requests from arrival.

## 8. Random numbers *[compat]*

| Draw | Generator | Method | Draws |
|---|---|---|---|
| start position in `FindNewResidence` | the MySim's own `cRZRandom` member, seeded from the wall clock at construction (foundations §6.2, a new row: "MySim, per object") | `RandomUint32Uniform(n)` (foundations §6.1) | exactly 1 per call that reaches step 2 (n = 0 → no state change) |

No other simulation-side draw exists in the agent or MySim class. The dispatch entry points, queue edits,
clock/deadline arithmetic, wait getter, state/flag notification setters and record read/write specified above
make **no RNG draw** and must not consume the MySim residence-search generator. Its constructor does not
construct a cRZRandom member: its call to `0x008EC270` initializes deque storage (allocation
and block arithmetic), not a seed. No RNG bytes appear in §6.2.1. The transport/automata handoff's delegated
randomness remains MS18; do not infer a per-dispatch generator or replay a residence draw for it.
Balloon/idle-animation and advice-script randomness belongs to the corresponding UI/advisor owners.

---

## 9. Dependencies

### 9.1 Feedback into the simulation: none *[compat]*

- No simulator reads MySim state. The only engine readers are UI windows, input controls, the query tool's MySim
  icon, the audio handler, the CitySituations advisor (`UpdateMySimAdvices`, which activates MySim missions; UI),
  and the Lua scripts.
- MySims write nothing to the city: no population, jobs, demand, budget, ordinance, grid or lot change. A MySim's
  home and job are references only; the buildings do not know about them.
- `CheckCommute` (`0x0042D9E0`) and `FindNewJob` (via the dispatch's `validateCommuteLocation`, `0x00432F50`) only
  run pathfinder queries: neither calls `ApplyTraffic` (`cISC4PathFinder` slot 29, pathfinder.md §4.17) nor any
  other traffic-volume writer, so **no trip is recorded** and the traffic data is unchanged. `CheckCommute` creates
  its own pathfinder and releases it. The only shared state a query touches is the process-wide work-check
  interval / adaptation counter of pathfinder.md §4.11 (timing, not traffic data).
- Side outputs: advisor events and advices (news, balloons), the `my_sim_died` automata trigger, MySim cars and
  pedestrians, UI/audio messages (§5.1).

### 9.2 Interfaces called (Windows slots)

| Interface | Slot / offset | Method | Pop | Ret | Used in |
|---|---|---|---|---|---|
| `cISC4City` | 33 `+0x84` / 34 `+0x88` | GetLotManager / GetOccupantManager | 0 | EAX | §4.7, §4.9 |
| | 47 `+0xBC` | GetAutomataControllerManager | 0 | EAX | Die |
| | 53 `+0xD4` | GetBuildingDevelopmentSimulator | 0 | EAX | §4.5, §4.9 |
| | 71 `+0x11C` | GetMySimAgentSimulator | 0 | EAX | MySim → agent |
| | 72 `+0x120` | GetDisasterLayer | 0 | EAX | §3.2, §4.6 |
| | 81 `+0x144` | GetAdvisorSystem | 0 | EAX | agent Init |
| | 93/94 `+0x174/+0x178` | CellCountX / CellCountZ | 0 | EAX | §4.5 congestion |
| | 97 `+0x184` | CellCenterToPosition(cx, cz, &x, &z) | 16 | — | §3.2, §5.2 |
| `cISC4ResidentialSimulator` (`0x00AB2C10`) | 21/22 `+0x54/+0x58` | GetAverageEQGrid / GetAverageHQGrid | 12 | EAX | §4.6 |
| | 26/27 `+0x68/+0x6C` | GetSchoolQueryData / GetHospitalQueryData | 8 | AL | §4.9 |
| | 29 `+0x74` | GetCellLifeExpectancy(x, z) | 8 | AL | §4.5 |
| | 37/38 `+0x94/+0x98` | GetCellEQByWealth / GetCellHQByWealth(x, z, w) | 12 | ST0 | §4.1, §4.5 |
| | 41 `+0xA4` | GetAverageAgeMap | 0 | EAX | §4.1 |
| | 42 `+0xA8` | GetAverageNewAgeByWealth(w) | 4 | AL | Spawn |
| | 47/49 `+0xBC/+0xC4` | GetSchoolBuildings / GetHospitalBuildings(list&, groups*) | 8 | (garbage) | §4.9 |
| `cISC4CrimeSimulator` | 5 `+0x14` | GetCrimeGrid | 0 | EAX | §4.5, §4.6 |
| `cISC4PollutionSimulator` | 5 `+0x14` | GetPollutionGrid(type) | 4 | EAX | §4.5, §4.6 |
| `cISC4AuraSimulator` (`0x00AB07C8`) | 12 `+0x30` | GetAuraGrid | 0 | EAX | §4.6 |
| `cISC4TrafficSimulator` | 6 `+0x18` | CreatePathFinder | 4 | EAX | §4.7 |
| | 10 `+0x28` | GetCongestionMap | 0 | EAX | §4.5, §4.6 |
| | 24 `+0x60` | GetTravelTimeRatio(x, z, type) | 12 | ST0 | §4.6 |
| `cISC4PowerSimulator` | 31 `+0x7C` | IsOccupantPowered | 4 | AL | §4.5 |
| `cISC4PlumbingSimulator` | 9 `+0x24` | IsBuildingWatered | 4 | AL | §4.5 |
| `cISC4PoliceSimulator` / `cISC4FireProtectionSimulator` | 5 `+0x14`; 21 `+0x54` / 22 `+0x58`; `+0x1C` / `+0x20` | coverage grid; station list; station funding | | | §4.5 |
| `cISC4BuildingDevelopmentSimulator` | 14 `+0x38`, 32 `+0x80`, 44 `+0xB0` | exemplar key of a building type; GetDeveloper(type); special-building list | 8 / 4 / 0 | EAX | §4.5, §4.9 |
| `cISC4BudgetSimulator` | 51 `+0xCC` | GetBudgetItemForPurpose(holder, purpose, item&) | 12 | EAX | §4.6 |
| `cISC4LotManager` (IID `0x681905BD`) | 7 `+0x1C`; 14 `+0x38`; 5 `+0x14` | GetLotsInCellRect; GetOccupantLot; GetLot(x, z, …) | 24 / 4 / … | AL / EAX | §4.6, §4.7, §4.9 |
| `cISC4OccupantManager` | 38 `+0x98`; 40 `+0xA0`; 48 `+0xC0` | occupant queries (all / in box) with a filter; release list | 20 / 20 / 4 | AL | §4.7, §4.9 |
| `cISC4Lot` (`0x00AB19B0`) | 21 `+0x54`, 28 `+0x70`, 35 `+0x8C`, 41 `+0xA4`, 48 `+0xC0` | GetLocation, HasRoadAccess, GetCondition, GetOccupantWealth, GetBuilding | | | §4 |
| `cISC4BuildingOccupant` | 3, 4, 9, 11 | AsOccupant, GetBuildingType, GetCompletionPercent, GetBuildingProfile | 0 | EAX | §4.9 |
| `cISC4Occupant` | 8 `+0x20`, 22 `+0x58` | GetPosition, IsOccupantGroup | 4 | AL | §4.9, §4.10 |
| `cISC4DisasterLayer` (`0x00A9E390`) | 10 `+0x28`, 16 `+0x40`, 17 `+0x44` | IsADisasterOngoing, location count, location by index | 0/0/16 | AL/EAX | §3.2, §4.6 |
| `cISC4SimGrid<T>` | 7, 8, 19, 22 | GetCellValue, GetAverageValueInCellRect, PositionToTract, GetTractValue | 8/16/16/8 | T | §4.5, §4.6 |
| advisor system / `cSC4Advisor` | advisors.md | GetAdvisors map, `PushEvent` (advisor slot 18), `CommandWouldTrigger` (19), Lua state | | | throughout |

Shared grid helpers of simgrid §7 are **not** used. Grids are read only through `GetCellValue`,
`GetAverageValueInCellRect` and `PositionToTract` + `GetTractValue`, never through `GetGridData` (simgrid §6.4:
the MySims own no grid).

---

## 10. Replaceability

- **Agent simulator**: replaceable through the class registry (`0x29E37DEC`). It has no save data. A replacement
  must keep the slot table of §7.1 (CitySituations calls slot 12 and uses the returned pointer; the UI calls
  slots 5–12) and must forward the messages of §3.2 with the same field use. Nothing reaches into its memory.
- **MySim**: replaceable per class (`0x4A1DBBBF`) through the Lua `advisors` table's class id, but a replacement
  must also be a full `cSC4Advisor` (advisor slot table, record format, Lua context) because the advisor system
  owns, ticks and saves it. The UI windows and the Lua wrappers call `cISC4MySim` slots by offset (§7.2).
- **Blockers**: (1) the Lua state shared with the advisor system (advisors.md §14.2); (2) `cSC4MySimDispatch` and
  the input controls call non-virtual MySim members (the dispatch notifies the MySim by message, but the dispatch
  tool and U-Drive-It code reach the MySim directly *[inferred]*); (3) the record format of §6.2 if old saves
  must load.
- Nobody reads MySim or agent memory by offset outside these classes *[inferred: no raw offsets found]*.
- **Dispatch**: the replacement must retain separate current/default/active/commute locations, destination
  queue, time and state/flag notifications (§4.6.1), its own serialized record (§6.2.1), and request/arrival
  distinction (§7.2.1). An immediate-arrival stub is not conformant. MS18 identifies the remaining transport
  handoff work before the whole dispatch owner can be declared complete; balloon rendering may be provided
  by its UI owner independently.

---

## 11. Quirks (reproduced) and open questions

### 11.1 Quirks

1. MySims never change any simulation result (§9.1).
2. `FindNewResidence` always returns false (§4.7); the MoveOut script command finds a new home rather than
   leaving.
3. `ChangeResidence` reverts the move when a move-out advice would trigger at once (§4.7).
4. Missing sources leave stale Lua fields (EQ ≤ 0, desirability with wealth outside 1–3, §4.5).
5. Riot/volcano event subjects carry cell indices as their location (§5.2).
6. Grid reads in trip variables have no bounds check (§4.6); `force` resets the cell to (0, 0) before reading.
7. The local building map treats distance 0 as unset (§4.9).
8. `GetActiveUIPanelSim` returns a pointer and reports the vector index (§7.4).
9. The neighbour job city is not saved (§2.2).
10. The agent's slot index is unbounded (§1.3).
11. `MySim::Read` accepts any version (§6.2).
12. Ageing uses the advisor month counter and a birth month fixed at move-in, so a MySim ages once per 12 monthly
    steps regardless of the calendar.

### 11.2 Open questions

1. *Answered (MS1, 2026-10-21):* row 25 is dead data, row 30 is the pylon-locked flag (§2.2).
2. *Answered (MS2, 2026-10-21):* window-manager slot 42 = "a modal window is running" (§4.2).
3. *Answered (MS3, 2026-10-21):* yes, see §2.2.1.
4. *Answered (MS4, 2026-10-21):* groups `0x150A`/`0x150B` (landmarks and rewards), §4.9.
5. *Answered (MS5, 2026-10-21):* traced on Windows, §4.5 (border ring only; agriculture jobs count as watered).
6. *Answered (MS6, 2026-10-21):* no traffic-data side effect (§9).
7. *Answered (MS7, 2026-10-21):* the check is the `RequestToPlaySound` throttle (§4.1); states 4/5/6 in §2.2.

---

## 12. Conformance tests (must exist in `engine/conformance`)

The following MS17 vectors are normative **specification vectors**, not captured game traces or claims
of implemented tests. They may be implemented by the engine lane without access to RE material.

| Vector | Sections | Input | Expected |
|---|---|---|---|
| `ms17_01_dispatch_identity` | 1.1, 4.6.1 | Fresh dispatch, then set current/home/destination/commute to four different valid locations | Initial state/previous = −1, flags/clock = 0, travel = 255; all four getters retain their own location. GoTo while travelling changes destination, not current |
| `ms17_02_notification_order` | 3.3, 4.6.1 | State −1, flags 0; set state 3 twice | First owner calls are type `0x2BA06B99`: data1 `0x4BB2D020`, old/new 0/2; then data1 `0x4BB2CFC0`, old/new −1/3. Second call produces no notice. Owner callbacks see state 3 already installed |
| `ms17_03_clock_and_wait` | 4.6.1 | Pathfinder exists; now 100, deadline 105, tracked valid location; one minute message then animation message | Now 105; dwell has not expired. Another minute message makes now 110; next update may depart. Separately, timestamp 100, flag `0x10` clear: getter at now 115 returns 100; at 116 returns/stores 0. With flag set, now 116 still returns 100 |
| `ms17_04_tuning` | 4.11 | Minimum location time 30; GoTo duration 5 with occupant, then without occupant. Approach distance 8, changed current cell, no player flag, destination cell (0,0) | Copied durations 30 and 5 respectively. Approach cells (7,0), (8,0), (9,0) produce flag `0x8000` set, clear, clear; only a rising edge pushes approaching event. With player flag, preserve previous approach flag |
| `ms17_05_queue_and_deadline` | 4.6.1, 7.2.1 | Queue [A,B], at-location now 100 with deadline 200; AddDestination(C,4), then AddDestination(D,1) | Queue [D,C], deadline still 200; both calls true, no forced departure. GoTo(E) in state 3 instead replaces queue by [E] and deadline by 99 |
| `ms17_06_commute_requests` | 7.2.1 | Residence present, MySim state 2; LeaveForWork at hour 7/8/17/18 in separate cases | Replace queue with by-commute destination of duration 5/540/0/5; MySim state 4. No residence: no queued request or success state change. With residence, LeaveForWork in state 5 sets state 4 without changing queue; ReturnHome in state 4 sets 5 without changing queue |
| `ms17_07_dispatch_record` | 6.2.1 | Flag true; tuning words [8,4,1,30,1]; state −1; travel `0xFE`; route [(2,3,1),(4,5,0)]; map entries with signed keys 2 and 1 inserted in that order; queue [A,B] | Prefix `01 00 00 00 01 08 00 00 00 04 00 00 00 01 00 00 00 1E 00 00 00 01 00 00 00`; first location at byte 77; state encoded four FF bytes, travel as `FE 00 00 00`; route count 2 then `02 03 01 04 05 00`; map writes key 1 then 2; queue writes A then B. MySim v12 has this payload inline; v17 row 26 is a ref to it |
| `ms17_08_player_and_rng` | 7.2.1, 8, 10 | State 3 at home, EnterUDriveIt; then exit while only pending. Separate case: active-player exit from state 2 | First case sets `0x20000` and requests GoTo(commute), exit clears it without synthesizing arrival. Active exit copies home to current and destination, clears queue, travel 255, state −1, then clears `0x30000`. All these entry/queue/time/record operations leave the residence RNG unchanged; a subsequent recovery update is distinct from Exit |

1. **Ownership.** New city with the shipped advisors table: the agent's vector holds MySim advisors 1–5 at
   indices 0–4 and the My Street sim at index 5. `get_count()` = 6. The city save contains no record for
   `0x29E37DEC`.
2. **Move-in age.** Average-age grid value 12 at the home cell → age 18; value 40 → age 40.
3. **Move-in EQ ring.** `GetCellEQByWealth` = 87.5 at move-in (ring all 87.5, index 0). The next seven monthly
   steps see EQ 90.0, 95.0, 100.0, 100.0, 100.0, 110.0, 110.0. Step k (1-based) replaces ring entry
   (k − 1) mod 6: after step 6, eqDelta = 110.0 − 87.5 = 22.5 → `delta_eq` = 22; after step 7 (replaces entry 0,
   which holds 90.0), eqDelta = 20.0 → `delta_eq` = 20.
4. **EQ ≤ 0 is ignored.** EQ 50.0, then the residential call returns 0.0 → `eq` stays 50.
5. **Ageing.** Birth month 3; monthly steps with month counters 14, 15, 27 → age +1 at 15 and 27 only.
6. **Job search.** No job, state 2, `FindNewJob` false on 3 consecutive days → `unemployed_days` = 3; a job found
   → 0 and event `0xAA3EDBB1` pushed.
7. **Home destroyed (no disaster).** Remove the home building with data4 = 0 → event `0xAA48F5C1`, then one
   `RandomUint32Uniform(n)` draw on the MySim RNG (seed 1, n = 4 → first draw `(0x000041C6 × 4) >> 32` = 0 → start
   at the first candidate).
8. **Home destroyed by disaster.** Disaster period started with data4 = `0xCA380B06`; remove the home with data4 = 4
   while a disaster is ongoing → `last_local_disaster` = `0xCA380B06`, event `0xAA4D2D64`, residence null, state
   unchanged.
9. **Job destroyed.** Remove the job with data4 = 0 → event `0xCA48F879`, `employed` = 0, state 2.
10. **Script commands.** AdviceCommand `0xEA4D269C` for MySim 2 → its state 0, `MySimsChanged` posted, automata
    trigger `my_sim_died` requested once; other MySims unchanged. `0x6A4A5E9E` with wealth 3 → no change; with
    wealth 1 → wealth 2, job dropped, one `FindNewResidence`.
11. **Spawn.** Wealth 2, `GetAverageNewAgeByWealth(2)` = 30 → age 30, generation +1, retired 0, bio empty.
12. **Building added.** Home position (100, 0, 100), new building at (130, 0, 140) → event `0xCA4E8986`, subject
    data1 = 50.
13. **Riot subject.** `DoDisaster(0xCA380B06, 10, 20)` → subject location (10.0, 20.0), data1 = distance from the
    centre of cell (10, 20); `DoDisaster(0x621CDA33, …)` → no event.
14. **Trip percentages.** 3 lots with buildings in the square, conditions {2, 2, 4}, wealth {1, 2, 3}:
    r = float32(1 / 3.0001) → distress1 = int(2 × r × 100) = 66, distress2 = 0, distress3 = 33, each wealth band
    = 33.
15. **Local distances.** Buildings of group G at distances 0 (home) and 37 inside ±20 cells → `DistanceToClosest(G)`
    = 37; nothing of group H → `0xFFFFFFFF`.
16. **School grade.** Two schools of group G at 120 m and 80 m with grades 0.6 and 0.9 → `LocalSchoolGrade(G)`
    writes 0.9 to the result pointer and returns it; Lua `mysim_local_school_grade(G)` = 90.
17. **GetActiveUIPanelSim.** MySims at indices 0–4 with slots 4, 3, 2, 1, 0: `SetActiveUIPanelSim(1)` →
    `GetActiveUIPanelSim(i)` returns MySim index 3, i = 3. `SetActiveUIPanelSim(7)` → null, i unchanged.
18. **Persistence round trip.** A MySim with every field set writes a version-17 record whose bytes match §6.2;
    reading it back and writing again gives identical bytes. A state-0 MySim writes `11 00 00` followed by the
    advisor part.
19. **Old version.** A version-14 record (five discarded s32 instead of the keys, no rows 25 and 31) loads with
    car/pedestrian keys zero and last player trip day 0.
20. **No feedback.** Run 24 months with and without five MySims placed (same seeds for every simulator): every
    simulator record except the advisor system's is byte-identical.
21. **MS13 location arithmetic.** Use §4.7.1's bit/result pairs, including −1e−30 → 0,
    `0x7FC00000` → 880803840 and ±1e8, to distinguish the native algorithm from floor-plus-saturating-cast.
    These are instruction-derived values under 24-bit precision, not game measurements. Cells (3, 37) →
    world (48, 592); preserve occupant, flags and neighbour fields. Cell x = `0x10000001` stays unchanged
    but gives world x = 16.0 (wrapping shift). For job search, a home occupant with no lot uses a single-cell
    rect even when another lot exists at its cell; without an occupant, use that cell lot's rect.
22. **MS14 import branches.** Import a valid family, then families with missing and non-numeric counts:
    the latter import zero members, not the preceding family's count. For otherwise importable members,
    missing `_gender` skips before zodiac/guid reads and portrait conversion; missing `_zodiac` skips before
    guid and conversion. Present empty values continue (later guid/portrait steps must still succeed).
    Formatting-only k = `0x80000000` yields §4.12's negative decimal keys/face name; never iterate to it.
    Short listed name `abc` resolves `Export\abc.ini`, not `Export\.ini`.
23. **MS15 request trace.** Give home/job occupants, their lots and dispatch destinations different bounds:
    only occupant bounds supply CheckCommute's rectangles. Removing the lots must not change those calls.
    Neighbour end rect uses the MySim's stored commute cell, even when negative or different from the
    dispatch destination. With both job references, use the local job. Assert §4.7's exact setter order
    and the absence of destination-flag/travel-mask/max-length setters.
24. **MS15 outcomes and owner order.** Missing home or both job references: traffic lookup still occurs,
    but no factory, failure event or time store. Null factory output: no rectangles, FindPath, failure
    handler or time store. A captured native trace with FindPath result 1 must call slot 26, convert/store,
    then Release; every other result must run QuitJob refresh, failure event, by-commute destination request
    and its synchronous owner callbacks before releasing the original pathfinder. Return false even if
    that replacement search succeeds. Verify conversion separately with §4.7's arithmetic examples;
    do not invent host `commute_time` outputs, treat arbitrary nonzero results as success, or reuse RNG.
    These are required cases; this lane produced no native runtime traces or conformance implementation.
25. **MS15 trip sampling/subjects.** State 0 stops before Lua/dispatch sampling; absent table or nonpositive
    handle stops earlier still. With world (160, 320), cell (10, 20), congestion PositionToTract receives
    (160, 320), ratio receives (10, 20). Force makes both (0, 0), leaving the dispatch location unchanged.
    New trip state 2 with dispatch state 3 uses the destination occupant as subject; new = 3 with dispatch
    state 2 uses the arrival subject (null at home/work). Environmental sampling follows dispatch-state
    gates. New = old = −1 with force writes both state booleans as 0 but preserves the subject. Flag changes
    beyond bit `0x100` suppress the final state-change update post (§4.6).
26. **MS16 factory inheritance.** Compare a fresh factory object with the request at CheckCommute's first
    FindPath: the nine rows of §4.7.2 must match, including strategy 3, uint16 masks 2/2 and float32 +0.0
    maximum trip length. Only flags and density change (0 → 1); setting filter 3 repeats its default.
    Run a job search first on the dispatch's persistent pathfinder, then CheckCommute: the latter still
    receives a new object with no wealth destination flags and Car-only masks, not masks `0x1CF`.
    With traffic starting costs `[4, 8, 12, 16, 20, 24, 28, 32, 36]`, a null request cost pointer selects
    those values, not nine zeros; adding the inherited +0.0 preserves them. A positive-cost Car route is
    not rejected solely because maximum trip length is zero: flag 1 bypasses that budget rejection.
    A route that requires Walk/Bus cannot gain those travel types from job search. With maximum work
    time 0, elapsed timer expiry cannot take the timed-suspension branch. Distinguish strategy 3's direct
    distance-table setup from strategy 0's standard-destination pre-search.
    First Init returns true; second Init returns false without resetting a changed flag, mask or cost
    pointer. Successful factory publication transfers one reference; the final Release calls Shutdown
    then the deleting destructor (`0x006D61B0`–`0x006D61DE`). Factory allocation-null/null-out fault paths
    must not masquerade as a normal false factory return. For a supplied service, a false return with a
    non-null output still makes CheckCommute use and release that object; a true return with null output
    returns false without FailCommute (§4.7). These are contract cases, not captured native game traces;
    solve the configured path request rather than returning a saved native result.

### 12.27 MS19 location equality and named teleport *[compat]*

These are normative fixtures for the dispatch owner, without a native game-run claim.

| Vector | Input | Expected observation |
|---|---|---|
| MS19-01 exact identity | Two copies of a local location with occupant A, cell (4,5), region (−1,−1), flags (1,0) | Equal. Change only world x/z, duration or deadline: still equal. Change either cell, either region coordinate, either flag byte, or occupant: unequal. Test each independently |
| MS19-02 null and neighbour | Null occupant, flags (0,1), cell (4,5), region (2,3), copied twice | Equal even if invalid. Changing region to (2,4), byCommute to 0, or one occupant to A gives unequal; identical non-null occupants never override those differences |
| MS19-03 teleport ordering | State 2, flags 0, travel 1, counter 10, now 100; commute duration 7, deadline 999; all four locations distinct; queue contains Q | Current becomes the commute copy with deadline 107; active/home/commute and queue Q unchanged; travel FE, previous 2, state 3, flags 2. Counter 14. Owner sees flags (0,2) then state (2,3), both seeing new current/state/travel. No EBA06C12 or EBA06BD4 |
| MS19-04 repeated/invalid teleport | State 3, flags 2, travel FE, counter 10, now 100; invalid commute with duration −1 | Current copies invalid commute, deadline 99; counter 11; previous 3, state 3; no notices. No validity/pathfinder/automaton calls. Repeat without owner: same state and counter effects |
| MS19-05 state-1 commute request | MySim state 1 with residence and invalid stored commute; home duration 9, now 100 | Teleport effects of MS19-04 (or MS19-03 if old dispatch state differs), then queue replaced by home with options 4. Commute is not repaired. DispatchCommute succeeds; arrival is not synthesized |

### 12.28 MS18 verified geometry and stop cases *[compat]*

These cases cover only the settled portions of §4.6.2; they are specification
vectors, not implemented tests or captured game traces.

| Vector | Input | Expected observation |
|---|---|---|
| MS18-01 linear threshold | No lots; destination cell (0,0), current (3,3), P=2 | D=trunc(float32(sqrt(18)))=4. Idle proximity false, final-marker proximity true. A squared-distance replacement would incorrectly reject the final marker |
| MS18-02 equality boundary | Destination point (0,0); current points (3,0), (4,0), (5,0), P=2 | D=3/4/5: idle proximity true/false/false; final-marker proximity true/true/false |
| MS18-03 lot geometry direction | A point rectangle (0,0,0,0), B rectangle (−2,−2,2,2) | Helper(A,B)=2, helper(B,A)=0. Altering world positions without changing cell/lot resolution does not change these results |
| MS18-04 stop enumeration | Target mode 3, missing current occupant; two returned purpose-1300 stops A then B, B closer | Choose A, set group 4215, update current from A, deadline now + transit wait and wait timestamp now; EBA0732E data=(1300,0,0) precedes automaton clearing. Reversing enumeration reverses selection |
| MS18-05 wait boundary | Transit flag 10, selected stop, now 100, deadline 100 versus 99; foreign next-leg attempt returns FF | At 100 search for a matching-group automaton and remain waiting if none. At 99 attempt the next leg without a supplied automaton and take stuck on FF. No synthetic arrival |
