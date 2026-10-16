# Spec: the disaster layer and the disasters (`cSC4DisasterLayer`, fire, riot and the rest)

Status: **behaviour spec**. Fire and riot are specified in full; the earthquake, pipe burst, toxic spill, volcano
and the effect disasters are specified in short form (§7–§11 say what that means). Clean-room input. This
document describes *behaviour*, not code. It was written by the analysis team from observing the original program
and its data files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general
knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, `Init` order §3.7, response curves §4, messages
§5, random numbers §6, the disaster-manager slot order §17), [simulator.md](simulator.md) (emergency pause),
[flammability.md](flammability.md) (spontaneous ignition, the adjusted grid the fire reads), [crime.md](crime.md)
(arson), [aura.md](aura.md) (riots), [plumbing.md](plumbing.md) (pipe bursts), [budget.md](budget.md) (§4.3
RiskyMoney earthquakes), [demolition.md](demolition.md) (`DemolishRegion` flags §4.9), [advisors.md](advisors.md)
(§7.6 disaster variables), [dispatch.md](dispatch.md) (fire and police dispatch), [history.md](history.md) (the
fire and riot counters), [mysims.md](mysims.md) (`DoDisaster`), [protection.md](protection.md) (fire and police
coverage grids), [weather.md](weather.md) (wind), [INVENTORY.md](INVENTORY.md) (why this spec is first).

## 0. Summary for the harness and for planning

- **Every disaster pauses the simulator and disables saving while it runs** *[compat]*. The first disaster that
  begins (any type) calls the simulator's `EmergencyPause` (simulator slot 11) and the city's `DisableSave` (city
  slot 104). When the last one ends, the layer calls `EnableSave` (slot 103) and `EmergencyResume` (simulator
  slot 14). The layer counts ongoing disasters; nothing else ends the pause (§2.4).
- **Disasters happen without the player.** Four sources run in unattended play:
  - the flammability simulator lights spontaneous fires (flammability.md);
  - the crime simulator commits arson (crime.md);
  - the aura simulator starts riots (aura.md);
  - the plumbing simulator bursts pipes when water funding stays below 50 % (plumbing.md).

  The budget's RiskyMoney earthquake (budget.md §4.3) is a fifth. An unattended run of a grown city therefore sees
  emergency pauses, and the harness must expect them. The layer's disable counter (§2.3) does **not** stop any of
  these sources; nothing in the game reads it. To avoid the pauses, a test city needs fire
  coverage above 50 everywhere flammable, police coverage, and water funding of at least 50 %.
- **Disasters run on the wall clock, not on game days** *[compat]*. The fire advances once per **1000 ms of real
  time** through its own timer and the animation tick. The riot advances once per real second. The earthquake,
  pipe burst, volcano and toxic spill use the animation tick's real microseconds. A turbo clock does not speed
  them up. While the simulator is in the emergency pause, game days do not pass at all. A long fire therefore
  costs real seconds, not game days. In the harness, an unattended fire with no fire station burns down its
  building at its 55th update, about 54 real seconds after the ignition (§5.12, test 3).
- **The layer has no save record.** It is not among the 53 city components (foundations §3.6). Fires, riots and
  spills in progress are not saved. A save cannot even be made while one runs (above). After a load, the layer
  starts empty.
- **The damage reaches the rest of the game through other subsystems.** Burned-down buildings go through the
  demolition utility (`DemolishRegion` with the "explode" flags). Lots take the "on fire" conditions 5 and 6. Flora
  goes to the flora simulator. The earthquake and the volcano change terrain through terrain brushes. Pipe bursts
  set the burst network flag through plumbing's own month step and demolish the pipe.

## 1. Identity

The layer is a city sub-object, not a saved component. The city creates it in `CreateCityComponents`
(`0x0045B01C`), keeps it at city `+0x1C4`, returns it from city slot 72 (`GetDisasterLayer`, `+0x120`), and calls
its `Init` after the civic building simulator's (foundations §3.7).

| Object | Manager type (`GetID`) | Class (CLSID) | Notes |
|---|---|---|---|
| disaster layer | — | `0x61F6ABF5` | IID `0x29B02A2C` (`cISC4DisasterLayer`, answered at `+8`); also answers `0x452294AA` at `+0xC` (its message target) |
| volcano manager | `0xC4C90997` | `0xA9B5AB9F` | instance class `0x29B5AB89` |
| fire manager | `0x621CDA33` | `0x69C1718E` | instance class `0x69C1718F`; the manager also answers `0x49B43F57` (`cISC4DisasterManagerFire`) |
| earthquake manager | `0x42963812` | `0x0A086BD3` | instance class `0xAA086D86` |
| pipe-burst manager | `0x4A1332E8` | `0x4A132FDF` | instance class `0x0A133012` |
| riot manager | `0xCA380B06` | `0x4A38206C` | instance class `0x4A38206D` |
| effect-disaster manager | `0x6A7B4397` | `0x6A7B3E9B` | instance class `0x6A7B3E9C` |
| toxic-spill manager | `0xEBFB0404` | `0xEBFB0402` | instance class `0xEBFB0403` |

The manager list is a static table of (type, class) pairs at `0x00A9E3F4`, in the order above *[compat]*. Each
manager is created through GZCOM `CreateInstance(class, 0x021E9CBF)` (`cISC4DisasterManager`). Each instance's
`GetID` returns **its manager's type**, not its class. For example, a fire instance answers `0x621CDA33`. The advisors
(§7.6) and MySims key on this value.

Random-number generators (foundations §6.3): earthquake `0x00B22AE8` (key `0x6C09C1AB`), pipe burst `0x00B22B60`
(`0xEC09C1D1`), riot `0x00B22B80` (`0x4C09C187`). The fire and the toxic spill use the fire instance's own
generator: a plain `cRZRandom` member of the fire instance, constructed with seed `0xFFFFFFFF`, so it is seeded from
the wall clock (fire instance constructor `0x0057AD90` → `0x00913A5C`; foundations §6.1). It is not reproducible in
the original either, so foundations §6.3 applies: a port may seed it from its host generator, provided it keeps
both `cRZRandom` paths (`lcg` for the integer draws, `mul` for the `double` draws) (§13).

## 2. The disaster layer

### 2.1 State

| Field | Meaning |
|---|---|
| managers | map from type to manager (each AddRef'd) |
| initialised | bool |
| disable counter `+0x38` | disasters are enabled while it is 0 (§2.3) |
| ongoing count `+0x40` | number of disasters that have begun and not ended |
| damage assessment | building count `+0x50`; total cost `+0x58` (int64); four (count, int64 cost) pairs by building kind at `+0x60/+0x68`, `+0x70/+0x78`, `+0x80/+0x88`, `+0x90/+0x98`; a vector of special-building records |
| dispatch timers | a fire map and a police map, keyed by **dispatch ID** (message data2, §2.5); an overall timer `+0x28` (cRZTimer, units 5 = seconds) |
| dispatch effectiveness | fire `+0xAC`, police `+0xB0` (float) |
| locations | vector of 20-byte entries {instance, x, z, zoom level, occupant} at `+0xC4` |
| next-location cursor `+0xC0` | index for `GoToNextDisaster` (§2.7) |

### 2.2 `Init` (`0x00578E70`) and `Shutdown` (`0x005780A0`)

`Init`, once:

1. For each (type, class) pair of §1 in table order, create the manager, ask it for its `GetID` and store it in
   the map.
2. Call each manager's `Init` (manager slot 3), in map order (ascending type).
3. Reset the disaster cost (§2.6).

`Shutdown` shuts every manager down (slot 4), releases them, clears the map, the locations (releasing their
occupants) and the timers, and unsubscribes from everything that §2.5 subscribed to.

`GetDisasterManager(type)` (slot 7) returns the map's manager for `type` **without AddRef**, or null *[compat]*.
Callers must not release it. HANDOFF run 37 was a crash caused by exactly that.

### 2.3 Enabling (`EnableDisasters` slot 5, `DisastersEnabled` slot 6)

`EnableDisasters(b)` decrements the disable counter when `b` is true and increments it when `b` is false. The
counter does not go below 0. Then it returns `DisastersEnabled()`, which is `counter == 0`. So calls nest: two
"disable" calls need two "enable" calls.

The layer itself never reads the counter, and no manager's entry point reads it. **None of the simulation-side
starters reads it either**: flammability, crime, aura (it calls the riot manager directly),
plumbing and the budget call the managers without asking. Disabling disasters therefore does not stop spontaneous
fires, arson, riots, pipe bursts or RiskyMoney earthquakes.

**Who uses the counter** (answers Q5). Nobody reads it. The only callers of `EnableDisasters` in the executable are
the player-drive input control (U-Drive-It): `EnterDrivingMode` (`0x00565160`) calls `EnableDisasters(false)`
(`0x00565189`) when it is given a vehicle, and `EndInput` (`0x005650E0`) calls `EnableDisasters(true)`
(`0x0056514E`) every time, both through the city's disaster layer (city slot 72) and both ignoring the result.
`DisastersEnabled` (slot 6) is called only from inside `EnableDisasters` itself (`0x005773E2`). Evidence: a scan of
every call through city slot 72 in the executable (119 sites) and of every use of the cached layer pointer
`0x00B43D3C` and of the layer pointers kept by the effect and toxic instances; no other site calls slot 5 or 6 of the
layer, the disaster menu (`cSC4View3DWin::InvokeDisaster` `0x007E9DB0`) does not ask, and no string or Lua name in
the executable mentions enabling disasters. The counter is cleared by `Init` (`0x00578E86`) and is not saved.

So the counter only records that the player is driving, and has no effect *[compat]*: disasters, the disaster menu
and the emergency pauses all go on during a mission. A replacement keeps the slots and the nesting (test 6) and
makes nothing depend on them. (If `EndInput` runs without a matching `EnterDrivingMode`, the counter stays at 0.)

### 2.4 Begin and end *[compat]*

**`DisasterHasBegun(instance)`** (slot 8, `0x00578FC0`):

1. `ongoing += 1`.
2. If it went from 0 to 1:
   1. `simulator.EmergencyPause()` (simulator slot 11).
   2. Start the damage assessment (§2.5).
   3. `city.DisableSave()` (slot 104).
3. `GetMostRecentDisasterLocation(instance, &x, &z)` (slot 12). x and z stay 0 if the instance has none.
4. Post `0xCA1B1E64` (DisasterPeriodStarted) with data1 = `ongoing`, data2 = x, data3 = z, data4 =
   `instance.GetID()`, void1 = the instance.

**`DisasterHasEnded()`** (slot 9, `0x00577E90`), only while initialised and `ongoing > 0`:

1. `ongoing −= 1`.
2. If it reached 0:
   1. `city.EnableSave()` (slot 103).
   2. `simulator.EmergencyResume()` (slot 14).
   3. End the damage assessment (§2.5).
   4. Post `0xAA1B2002` (DisasterPeriodEnded).

`IsADisasterOngoing()` (slot 10) is `ongoing ≠ 0`.

`DisasterHasEnded` takes no instance. A disaster that calls `DisasterHasBegun` twice must call `DisasterHasEnded`
twice. The fire calls each **once per fire episode** (§5.9), not once per burning building.

### 2.5 Damage assessment and dispatch timers

**Start** (`0x00578F10`):

1. Subscribe to `0x99EF1143` (occupant removed) and to the dispatch messages `0xE94671EC`, `0xE94671EB`,
   `0x894671F6` and `0xE94671ED` (dispatch.md).
2. Reset the cost (§2.6).
3. Clear both timer maps.
4. Reset and start the overall timer (seconds).

**Occupant removed** (`DoOnOccupantRemoval`, `0x00578BA0`), only while `city.GetEstablished()` (slot 22) is
true:

1. `cost := occupant property 0x8A1C3E72` (int64, the demolition cost). The value is 0 when the property is
   absent.
2. If the occupant is a building (`0x278128A0`):
   1. `buildingCount += 1`.
   2. Read the building-type flags from the BDS, then add 1 and `cost` to the kind pairs:
      - flag bit 1 → `+0x60/+0x68`;
      - bit 2 → `+0x70/+0x78`;
      - bit 4 → `+0x80/+0x88`;
      - bit `0x20` → `+0x90/+0x98`.
   3. If the flags have any bit of `0x1800400`, push a special-building record. The record is {name, cost}: the
      name is the BDS `GetBuildingName` (BDS slot 38) of the building's type (building occupant slot 12), and the
      cost is the int64 `cost` of step 1. The flags are the BDS building profile's (`GetBuildingProfile`, BDS slot
      39) flag word at profile `+0x1C`.
3. If the occupant is a network piece (`0xC772BF98`) and the property lookup failed, `cost := 10`. This is one
   int64 constant (`0x00B0A1C0`, read only at `0x00578D41`), the same for every network type; it is not a table.
4. `totalCost += cost`.

Every removed occupant counts, whatever removed it, while a disaster is ongoing. This includes the player's own
bulldozing during the pause.

**Dispatch messages** (`DoDispatchMessage`, `0x005784B0`). Message data1 is the dispatch type, and it selects a
map:

- types 1 and 5 → the fire map;
- types 0 and 6 → the police map;
- any other type → the message is ignored.

The key is message data2, the **dispatch ID** (a dispatch record of dispatch.md, not a station). Any of the four
messages with a mapped type creates the key's entry when it is absent: a new cRZTimer with units 5 (seconds), not
started. Then:

- `0xE94671EC` starts the timer if it is not running. A restarted timer keeps its accumulated time.
- `0xE94671EB`, `0x894671F6` and `0xE94671ED` stop it if it is running.

So a stop message for an unknown ID leaves a zero-time entry, which still counts in the sum below (it adds 0).

**End** (`EndDamageAssessment`, `0x00577CA0`):

1. Unsubscribe.
2. Run `EndDispatchTimers` (`0x00577AF0`):
   1. Stop the overall timer and every timer in both maps.
   2. Read the times with the timer's **float** `GetElapsedTime` overload (`0x00890377`, units 5): fractional
      seconds, not truncated.
   3. `fire := Σ(fire-map seconds) / (float(N_fire) × overall seconds)`, where `N_fire` is the fire protection
      simulator's (`0x00B43D78`) `GetTotalDispatchCount()` (slot 25, unsigned; the number of dispatch units over
      all stations, protection.md). If `N_fire = 0`, `fire := 0`.
   4. `police` is the same with the police-map seconds and the police simulator's (`0x00B43D98`) slot 26 count.
   5. Store them at `+0xAC` / `+0xB0`. Nothing is passed to the simulators.

   The divisor is **not** the number of timers. An empty map gives Σ = 0, hence 0. The overall time is never 0
   while a period ran, and a port may return 0 there *[choice]*.

The advisors read these at `0xAA1B2002` (advisors.md §7.6).

### 2.6 Cost (`GetDisasterCost` slot 18, `ResetDisasterCost`)

`GetDisasterCost()` (slot 18, `0x00578900`, pop 0) returns a **pointer to the layer's statistics record**, not
`totalCost` *[compat]*. The record starts at layer instance `+0x50`; the interface sits at `+0x08`, so the pointer is
the interface pointer + `0x48`. Its layout, in record offsets (instance offset in parentheses):

| Record | Instance | Contents |
|---|---|---|
| `+0x00` | `+0x50` | `uint32` building count |
| `+0x08` | `+0x58` | `int64` total cost |
| `+0x10` / `+0x18` | `+0x60` / `+0x68` | kind pair for flag bit 1: `uint32` count, `int64` cost |
| `+0x20` / `+0x28` | `+0x70` / `+0x78` | kind pair for bit 2 |
| `+0x30` / `+0x38` | `+0x80` / `+0x88` | kind pair for bit 4 |
| `+0x40` / `+0x48` | `+0x90` / `+0x98` | kind pair for bit `0x20` |
| `+0x50` | `+0xA0` | vector of special-building records {name, cost} |
| `+0x5C` | `+0xAC` | `float` fire effectiveness (§2.5) |
| `+0x60` | `+0xB0` | `float` police effectiveness (§2.5) |

The only reader is the safety advisor (`0x00416ED1`): it copies `+0x00` into the damaged-building count, `+0x08` into
the damage cost, and `+0x60` × 100 and `+0x5C` × 100 into the police and fire effectiveness percentages. A
replacement must return a pointer to a record with this layout and keep it valid for the layer's lifetime.

`ResetDisasterCost` zeroes the counts, the costs and the special-building records.

### 2.7 Locations (slots 12–17, `GoToNextDisaster` slot 11)

- **`RegisterDisasterLocation(instance, x, z, occupant, zoomLevel, focus)`** (slot 13, pop 24):
  1. Append the entry and AddRef the occupant.
  2. If `focus` is true and the "follow disasters" preference byte (`0x00B43CD8 + 0xF08`) is set, call
     `GoToNextDisaster`.
- **Occupant-based entries** *[compat]*. An entry is 20 bytes: {instance, x, z, zoom, occupant}. Its stored x and
  z change only when the entry is **resolved** (`ResolveDisasterLocation`, `0x00577540`):
  - With an occupant: if the occupant manager still holds it (occupant manager slot 27, `FindOccupant(occ, false,
    null, null)`), take the occupant's position (occupant slot 8) and convert it to a cell (city slot 95,
    `PositionToCell`). The result is **written back into the entry**. If the occupant is gone, nothing changes.
  - Without an occupant: call the instance's `GetLocation` (instance slot 6); if it returns true, store its x and
    z in the entry.

  Only two functions resolve: `GetMostRecentDisasterLocation` (slot 12, at `0x0057764E`) and
  `GetNextDisasterLocation` (`0x005777C3`, used by `GoToNextDisaster`). `GetDisasterLocationByIndex` (slot 17) does
  **not** resolve. So an entry registered with (0, 0) reads (0, 0) through slot 17 until something resolves it,
  and afterwards keeps the cell of its last resolution, not the occupant's current cell *[quirk]*. In practice an
  entry is resolved when:
  - `DisasterHasBegun` calls slot 12 for its instance (§2.4), which resolves that instance's newest entry;
  - `RegisterDisasterLocation` runs with `focus` and the preference set, since `GoToNextDisaster` resolves the new
    entry (and moves the cursor one step back);
  - the player uses the "next disaster" view command (`0x0992FD36`, `GoToNextDisaster`).

  The walking views (MySim, MyStreetSim) read slots 16 and 17 and therefore see these stale cells.
- **`UnregisterDisasterLocations(instance)`** (slot 14) removes all of the instance's entries.
  **`UnregisterDisasterLocationByOccupant(instance, occupant)`** (slot 15) removes the matching entries. Both
  release the occupants.
- **`GetMostRecentDisasterLocation(instance, &x, &z)`** (slot 12, pop 12) searches from the newest entry. It
  takes the newest entry overall when `instance` is null, else the newest entry of that instance. It returns false
  when there is none.
- **`GetDisasterLocationCount`** (slot 16) and **`GetDisasterLocationByIndex`** (slot 17, pop 16, `0x00577920`)
  give plain access to the stored values, unresolved. Slot 17 AddRefs the instance it returns; the caller releases
  it.
- **The cursor** (`+0xC0`). `RegisterDisasterLocation` sets it to the new entry's index (`count − 1`) on every
  registration (`0x005783D0`). `GetNextDisasterLocation` (`0x00577770`):
  1. If there are no entries, return false.
  2. If `cursor ≥ count`, set `cursor := count − 1`.
  3. Use the entry at the cursor (resolved as above).
  4. `cursor −= 1`; below 0 it wraps to `count − 1`.

  The first call after a registration therefore goes to the newest entry, and further calls walk backwards to
  older ones, cycling. The unregister calls do not touch the cursor; step 2 clamps it on use. The constructor's
  initial value only matters before the first registration, when there is no entry anyway.
- **`GoToNextDisaster`** (`0x00577810`) is view-side:
  1. Take the next location (`GetNextDisasterLocation`, above).
  2. Convert the cell to a position (city slot 97) and read the terrain height.
  3. Move the camera there.
  4. If the camera is zoomed out more than the entry's zoom level and the view is open, zoom in to that level.

  It has no simulation effect.

The "zoom level" is the tuning byte `0xCA821B8C` of each disaster (3 for fire, riot and pipe burst; §5.1).

## 3. The manager framework

### 3.1 The base manager (`cSC4DisasterManager`)

Every manager keeps a list of its instances, each AddRef'd.

- `AddInstanceToList(i)` (slot 8) and `RemoveInstanceFromList(i)` (slot 9). While the manager's `Shutdown` is
  walking the list, these calls go to temporary add and remove lists that are applied afterwards.
- `Shutdown` calls each instance's `Shutdown`, then releases it.
- **Instance `Shutdown`** (instance slot 3) is not uniform *[compat]*. Every instance below that removes itself does so
  through `RemoveInstanceFromList`:

  | Instance | Removes itself from the manager | `DisasterHasEnded` | Locations |
  |---|---|---|---|
  | fire (`0x0057A540`) | yes | no | unregistered; `KillAllFires` first posts `0x49E3DB1D` data2 0 for every fire and removes each fire occupant |
  | riot (`0x0057D420`) | yes | no | unregistered |
  | pipe burst (`0x0057C460`) | yes | no | kept (its natural end unregisters them before calling `Shutdown`, §8) |
  | toxic spill (`0x0057EB80`) | yes | **yes**, then unregisters | unregistered |
  | volcano (`0x0057F920`) | yes | **yes** if the disaster had begun and not yet ended (through `EndVolcanoLava`, §10) | unregistered with it |
  | earthquake (`0x00576300`) | no (state 4 removes it first, §7) | no | kept (state 4 unregisters them first) |

  The fire also unsubscribes, releases its filter and toast list and clears its fire IDs; the others unregister from
  the animation tick and release what they hold. None changes its phase. The effect instance was not checked.
- `GetActiveInstanceCount()` (slot 10) counts the instances whose `GetPhase()` is 1 (active).
- `GetInstances()` (slot 11) returns the list.

The compiled slot order is in foundations §17: slot 6 is `AttemptDisasterStart(int32 x, int32 z)`, slot 7 is the
argument-less `AttemptDisasterStart()`. What x and z mean differs per manager:

- cells for the riot, the pipe burst and the toxic spill;
- world units for the earthquake and the volcano (budget.md §4.3).

### 3.2 The instance interface (`cISC4DisasterInstance`)

| Slot | Method | Fire's answer |
|---|---|---|
| 3 | `Shutdown` | forwards to the instance's own `Shutdown` |
| 4 | `GetManager()` | the layer's manager for `0x621CDA33` |
| 5 | `GetID()` | the manager type (§1) |
| 6 | `GetLocation(int& x, int& z)` | **false** (the fire keeps its locations in the layer) |
| 7 | `GetPhase()` | 0 = idle, 1 = active, 3 = ended |

Fire and riot use phases 0, 1 and 3. **Phase 2 is used by the volcano**: its `GetPhase` (`0x0057F440`) maps state 0
→ 0, state 1 → 2, states 2 and 3 → 1, state 4 → 3, and states 5 to 8 → 2 (§10). A volcano with flowing lava
(state 5) is therefore not "active" for `GetActiveInstanceCount`. The earthquake and the pipe burst always answer
phase 1 (a shared constant function, `0x008CD450`); they leave their manager when they end.

## 4. Who starts disasters

| Source | Call | Spec |
|---|---|---|
| flammability simulator (spontaneous) | fire `IgniteCell(x, z, true, false, 0)` per ignition attempt, cause 5 | flammability.md |
| crime simulator (arson) | fire `IgniteOccupant`, `SetCause(2)` | crime.md |
| riot followers | fire `IgniteOccupant(target)`, `SetCause(2)` from the pedestrian's ignite state (`cSC4Pedestrian::DoIgniteTick`) | §6.6 |
| fire spreading | fire `IgniteCell(x, z, true, false, fireID)` | §5.10 |
| volcano lava | fire ignitions where the lava flows (`cSTEVolcanoLava`) | §10 |
| effect actions (meteors, UFOs and so on) | fire `IgniteCellRadius` with `SetCause(4)` (`0x005BCBF0`, `0x005BCC80`); toxic spill (`0x005BD3C0`) | §11 |
| player: torch tool | fire `IgniteCell` with `teleport` (`cSC4ViewInputControlTorch`), cause 1 or 3 | §5.4 |
| tutorial | fire `TriggerFire` | — |
| aura simulator | riot `AttemptDisasterStart(cellX, cellZ)` | aura.md |
| plumbing simulator (month) | pipe-burst `AttemptDisasterStart(x, z)` | plumbing.md |
| budget simulator (RiskyMoney) | earthquake `AttemptDisasterStart(x, z)` in world units | budget.md §4.3 |
| MySim `DoDisaster`, the player's car (`cSC4Vehicle`) | earthquake, volcano | mysims.md |
| disaster menu (`cSC4ViewInputControlEffectDisaster`) | the menu entry's manager `AttemptDisasterStart` (slot 6), or an effect disaster (§11) | §11 |
| cheat command `0x0BFB41BA` "ToxicSpill" | toxic spill at (x, z, amount) | §9 |

Cause codes (fire `SetCause`, read by the advisors): 1 and 3 player; 2 arson; 4 effects; 5 flammability.

## 5. Fire

The fire manager owns **one persistent fire instance**. All fires in the city are entries in it.

### 5.1 Tuning *[compat]*

The tuning is read once (`ReadTunableValues`, `0x00579C10`) from exemplar type `0x6534284A`, group `0x29B6C670`,
instance `0xC9B6B73D` ("Flame"). The "Default" column applies **only when the whole exemplar is missing**
*[compat]*. When the exemplar exists but lacks a property, nothing is defaulted: an array property stays empty, a
curve stays empty, and D6 and the zoom byte keep whatever the constructor left (the constructor does not set them;
treat them as 0). When the exemplar has an array property, the array takes the stored length.

| Property | Meaning | Default | Shipped |
|---|---|---|---|
| `0xA9B6ECD3` (D3) | stage durations (counter units, 5 values) | 10, 10, 10, 10, 10 | 30, 25, 20, 20, 25 |
| `0xA9B6ECD4` (D4) | spread divisor per stage (5 values) | 10 ×5 | 4, 3, 2, 2, 1 |
| `0xA9B6ECD5` (D5) | spread radius in metres per stage (5 values) | 0, 3, 5, 7, 9 | 32, 48, 48, 80, 80 |
| `0xA9B6ECD6` (D6) | wind factor | 0 | 1 |
| `0xA9B6ECD7` (D7) | top-flame count curve (bounding-box area → flames) | — | (0, 1) (10000, 5) |
| `0xA9B6ECD8` (D8) | side-flame count curve | — | (0, 0) (10000, 6) (50000, 8) |
| `0xA9B6ECD9` (D9) | **coverage curve**: average fire-protection value → resistance | — | (0, 0) (0.25, 0.1) (50, 0.5) (75, 1.5) (100, 4) (150, 8) (200, 10) |
| `0xA9B6ECDA` (DA) | **flammability → ignition percent** curve | — | (0, 0) (30, 5) (50, 10) (75, 20) (90, 30) (115, 40) (160, 50) (200, 50) |
| `0xCA821B8C` | zoom level for the disaster location | 4 | 3 (u8) |

The rest of this spec calls the properties by their last two digits, D3 to DA. D3–D5 are `Uint32` arrays; D6–DA
are `Float32`. *[compat]* The instance keeps the stage thresholds **cumulatively** (`D3cum`): 30, 55, 75, 95, 120. Curves are response curves (foundations §4), clamped at both ends.

**Array lengths.** The cumulation always reads D3 indices 0 to 4, and the stage lookup (`0x005796B0`) compares the
counter (unsigned) with `D3cum[0..4]` and returns 5 when none is larger. D4 is indexed by the stage `s` of §5.10
(0 to 5) and D5 by the fire's current stage (at most `maxStage − 1`). Entries beyond those indices are ignored.
Fewer entries than an index needs is undefined in the original (it reads past the array). A port must treat the
shipped five-entry arrays exactly; for shorter arrays it may clamp the index to the last entry *[choice]*. D4 needs
a sixth entry only when `s` reaches 5 (the counter is at least `D3cum[4]` but the fire did not burn down, which
needs the no-burn-down flag or `maxStage` above 5).

The update period is **1000 ms** of the instance's timer (cRZTimer units 4, milliseconds).

### 5.2 State of the fire instance

| Field | Meaning |
|---|---|
| fires | list of fire entries (each a fire occupant, §5.11) |
| fire IDs | set of fire-episode IDs; each ID has one location in the layer |
| toast list | occupants to burn down at once (§5.7) |
| phase | 0 idle, 1 active, 3 ended (§5.9) |
| timer, next update | units 4 (ms); `next` advances by 1000 |
| cause | the **first** `SetCause` value of the episode (§5.7); reset to 0 when an episode starts (§5.9) |
| ignition counts | buildings `+0xD0`, flora `+0xD4` since the last report |
| no-burn-down flag `+0x54` | copied from the tutorial each update (§5.8) |
| city size (cells) | cached at `Init` |

### 5.3 `Init` (`0x0057A310`)

1. Read the tuning (§5.1) and register with the fire manager (`AddInstanceToList`).
2. Point the fire occupants' static top-flame and side-flame curves at D7 and D8.
3. Cache the city cell counts and hold the demolition utility.
4. Restart the timer (units 4).
5. Build an occupant-type filter for {flora `0x74758926`, building `0x278128A0`, prop `0xA823821E`}.
6. Subscribe to `0x06CDB65B` (view level changed) and `0xA6B79602` (view update); both only refresh the flame
   visuals.
7. Run `Update` once.

The animation-tick registration happens later, when a fire appears (§5.9).

### 5.4 `Ignite(occupant, force, fireID, focus)` (slot 6, `0x0057A7D0`)

Returns true if the occupant burns afterwards.

1. If `force` is false, the simulator is paused (slot 15), and it is **not** in an emergency pause (slot 17):
   return false. A manual pause stops new fires; the disaster pause does not.
2. The occupant's type must be building, flora or prop; else return false.
3. Its flammability property `0x29244DB5` must be non-zero; else return false.
4. If `force` is false, compute the **coverage**:
   1. Take the average of the fire-protection coverage grid over the occupant's cell rectangle, clamped to the
      city (fire-protection simulator slot 5 grid, grid slot 8 `GetAverageValueInCellRect`, as a byte).
   2. Then `r := round(D9(coverage))`, rounding half to even (the 12582912 trick; foundations §18).
   3. If `r > 0`: return false.

   With the shipped curve, coverage ≤ 50 lets the fire start (D9(50) = 0.5 → 0) and coverage ≥ 51 blocks it
   (test 1).
5. If the occupant already burns (its property `0x69DBEBE1` is true): with `force`, add 1 to that fire's counter
   and return true; else return false.
6. If `fireID` is 0, draw one: `RandomUint32Uniform()` (the raw draw, §13). The draw happens only here, after all
   the refusals above. If the ID is new, register a location with the layer:
   `RegisterDisasterLocation(instance, 0, 0, occupant, zoomLevel, focus')`, where `focus' := focus and the fire-ID
   set was empty` (`0x0057A9BA`–`0x0057A9D6`; the set's size is at instance `+0xF4`, the zoom byte at `+0xC4`). Only
   the first ID of an episode can move the camera. Then add the ID to the set.
7. Create a fire occupant (class `0x69D25C5F`):
   1. Add it to the list and `Init` it.
   2. Set its target to the occupant (§5.11) and its fire ID to `fireID`.
   3. Insert it in the occupant manager.
8. With `force`: set the counter to `D3cum[0]` (30), or to half of it (15) when the target's maximum stage is
   below 2.
9. Add a transient aura effect at the occupant's position (aura.md §8.2).
10. Post `0x49E3DB1D` with data1 = the fire occupant and data2 = 1 (fire started).
11. Count the ignition: flora → `+0xD4`, building → `+0xD0` (props are not counted).
12. Run `UpdatePhase` (§5.9).

**The refusals, exactly.** `Ignite` returns false at these five exits and no others:

| Exit | Test | Detail |
|---|---|---|
| step 1 (`0x0057A80D`) | not `force`, not emergency-paused (simulator slot 17), paused (slot 15) | |
| step 2 (`0x0057A907`) | type not flora, prop or building | |
| step 3 | flammability `0x29244DB5` = 0 | read through the occupant's own property holder (occupant slot 6) with the property helper `0x005FD480`, as an unsigned integer (variant slot 25, converting); a missing property reads 0 |
| step 4 | not `force` and `round(D9(coverage)) > 0` | the rectangle is the occupant's `GetBoundingCityCells` (occupant slot 11) intersected with the city (`0x0048ED90`; no overlap gives an empty rectangle); the average is `GetAverageValueInCellRect(x, z, w, h)` (`0x0045DE50`: sum / (w × h), truncated by `_ftol`, as a byte); D9 is the curve at instance `+0xA4`; the rounding uses the 12582912 constant at `0x00A9E434` |
| step 5 | target's `0x69DBEBE1` true and not `force` | |

The "burning" property `0x69DBEBE1` is set only by the fire occupant's `SetTarget` (`0x004719DF`) and cleared only
by the fire occupant's `Shutdown` (`0x00470E5D`), which runs when `KillFire` removes the fire occupant (§5.10). A
target therefore reads "burning" for exactly as long as a fire occupant on it exists.

`IgniteOccupant(o)` (manager slot 4, `0x0057B930`) returns false for a null `o`. Otherwise it calls `Ignite(o,
false, 0, true)` and returns **true whatever `Ignite` returned** *[quirk]*.

### 5.5 `IgniteCell(x, z, useProbability, teleport, fireID)` (slot 7, `0x0057B170`)

1. `p := DA(flammability grid value at (x, z))`, kept as a `float` (`0x0057B1D8`). The grid is the adjusted
   flammability grid (flammability.md).
2. If `useProbability`: `r := RandomDoubleRangeUniform(0, 100)` (one draw, the `mul` path); return unless
   `r ≤ p`. Without `useProbability` nothing is drawn.
3. If `fireID` is 0, draw one now (`RandomUint32Uniform()`, raw). This happens once per call that passed step 2,
   **even when no occupant in the cell then ignites**. A call refused in step 2 draws no ID.
4. For each occupant in the cell that passes the type filter: `Ignite(occupant, teleport, fireID, not teleport)`.
   All of them share the ID of step 3.

The fire manager's `AttemptDisasterStart(x, z)` is `IgniteCell(x, z, false, false, 0)` and returns null.

### 5.6 `IgniteCellRadius(x, z, r0, r1)` (slot 10, `0x0057AF10`)

1. Draw one fire ID for the whole call (`RandomUint32Uniform()`, raw), first and always.
2. Visit every occupant in the clamped box of radius `r1` cells around (x, z) that passes the type filter. Let
   `d` be the 2D distance from its position to the cell centre, and `w` the cell width (city slot 91, 16 m).
   - `d < r0 × w`: add it to the toast list (it burns down at the next `Update`, if flammable; §5.7).
   - `r0 × w ≤ d < r1 × w`: `Ignite(occupant, false, id, false)`.
   - Otherwise: nothing.

### 5.7 Other entry points

- **`ExtinguishCell(x, z, amount)`** (slot 8, `0x0057B350`). It asks the occupant manager for the occupants on
  the single cell (x, z) (`GetOccupantsByStandardCityCells`, occupant manager slot 40) through an
  "active fire" filter (`IsOccupantIncluded` `0x0046FD70`): fire occupants (`0x69D25C5F`) with a non-null target.
  Fires already extinguishing are included. For each: if `counter ≤ amount`, set it to 0 and run `ExtinguishFire`
  (below); else `counter −= amount`. Fire crews and trucks call it (dispatch.md).
- **`QuickBurnCell(x, z)`** (slot 9) adds the cell's filtered occupants to the toast list.
- **The toast list** (`AddToToastList`, `0x0057AB20`). Each new entry is AddRef'd, with no duplicate check. If the
  list already holds `n ≥ 1` entries, `i := RandomUint32Uniform(n)` and the entry is inserted **before** element
  `i`; into an empty list it is simply added. So an entry never goes to the end of a non-empty list, and every
  insertion into a non-empty list costs one draw (§13).
- **`ProcessToastList`** handles at most **5** entries per `Update`, from the front. Each still-flammable occupant
  is burned down (`ToastOccupant`, §5.10), then released; the rest wait. It runs **only inside `Update`**, and
  `Update` runs only while the instance is on the animation tick (an episode is on, §5.9), plus once at `Init`.
  Entries queued while no fire burns (by `QuickBurnCell`, or by `IgniteCellRadius` when every occupant fell inside
  `r0`) therefore wait until the next fire episode *[compat]*.
- **`ExtinguishFire(fire)`**. If the fire has no extinguish time yet, set it to `elapsed ms +
  RandomUint32Uniform(4000)` (one draw; none when the time is already set). The fire then lasts until that time
  (§5.10 step 2) and dies with `KillFire`.
- **`Extinguish(o)`** (slot 11) and **`BurnDown(o)`** (slot 12) look up the fire whose target is `o` and run
  `ExtinguishFire` or `BurnDownFire` on it.
- **`GetFireCount()`** (slot 16) counts the list. **`GetFireCountInCellRect(rect)`** (slot 15, `0x00579A60`)
  counts every fire in the list, including those with an extinguish time, whose fire occupant's bounding cells
  (cISC4Occupant slot 11, `GetBoundingCityCells`) overlap `rect`, inclusive on both axes.
- **`SetCause(c)`** (slot 13, `0x00579230`) sets the cause **only when it is 0**, and returns the cause. The first
  cause of an episode wins; the episode start resets it to 0 (§5.9, `0x0057A193`). Because the reset happens inside
  the first `Ignite` of an episode, a `SetCause` made before that ignition is lost. **`GetCause()`** (slot 14).

### 5.8 `Update` (slot 4, `0x0057BF10`)

It runs on the animation tick `0x86AD10EE` while registered (§5.9). The instance also handles `0x66956813` (the
hour), but no hour agent is registered for it *[inferred]*. Each run:

1. `ProcessToastList` (§5.7).
2. `UpdatePhase` (§5.9).
3. If there are fires: read the timer in whole milliseconds (the integer `GetElapsedTime`, truncating). If
   `next` is 0, set `next := elapsed`. If `elapsed ≥ next`: run `UpdateAllFires`, then `next += 1000` (a
   constant, not tuning). At most one step runs per tick; a backlog (a stalled frame) is worked off one step per
   later tick *[compat]*.
4. If any ignitions were counted, post `0x49D15B73` with data1 = buildings and data2 = flora, then zero both.

**The fire list** is a `std::list` at instance `+0xE0`. `Ignite` appends the new fire at the end (`push_back`,
`0x007E98C0`).

**`UpdateAllFires`** (`0x0057BEA0`):

1. Read the tutorial's no-burn-down flag (tutorial system slot 8) into `+0x54`.
2. Walk the list from the front. For each node, remember the **next** node before working on this one, then:
   1. If the fire's target (`GetTarget`) is null, run `KillAllFires` and stop. The fires before it in the list
      have already been stepped in this pass.
   2. Otherwise run `UpdateFire` on it.
   3. Continue with the remembered next node.

Because the next node is taken before the step, a fire appended during the pass (by a spread's `IgniteCell`) is
stepped **in the same pass** when the spreading fire was not the last node at the start of its step, and is **not**
stepped when the spreading fire was the last node (its remembered next was the end) *[compat]*.

### 5.9 Phase (`UpdatePhase`, `0x0057A0D0`)

- **Fires exist and phase ≠ 1**:
  1. Reset the cause to 0 (§5.7).
  2. Only if the simulator exists, the rest:
     1. `layer.DisasterHasBegun(instance)`.
     2. Register with the animation tick manager (slot 5, priority `0xC0`).
     3. `next := timer elapsed` (whole milliseconds).
     4. Set the phase to 1: post `0xA9C7CAB6` (data1 = `0x621CDA33`, data2 = the phase).
     5. Set the severity to 1: post `0x49D15B72`.
- **No fires and phase = 1**:
  1. Set the severity to 0 (post `0x49D15B72`) and the phase to 3 (post `0xA9C7CAB6`).
  2. `layer.UnregisterDisasterLocations(instance)`.
  3. `layer.DisasterHasEnded()`.
  4. History `RecordHistoryData(0x49D15B74, 1)` (history slot 12; one fire episode).
  5. Unregister from the animation tick manager (slot 6).
  6. Clear the fire-ID set.

One episode lasts from the first fire to the moment the list is empty. The emergency pause lasts the same time.

### 5.10 One fire step (`UpdateFire`, `0x0057BB90`) *[compat]*

For one fire with target `t`:

1. If `t` is no longer in the occupant manager (occupant slot 32 `+0x80` with 1 returns false): `KillFire`, stop.
2. If the fire has an extinguish time: if the timer's elapsed milliseconds are **strictly greater** than that time
   (unsigned, `0x0057BBE0`), `KillFire`. Stop either way.
3. **Resistance**:
   1. `k := round(D9(average coverage over t's cell rectangle))`, as in §5.4 step 4, rounding half to even.
   2. If the fire is being hosed (`hosed > 0`, the number of hoses on it): `k += 4 × hosed`.
4. **Change**: `delta := +1` if `k ≤ 0`; else `delta := −RandomUint32Uniform(k + 1)` (the `lcg` path), a value
   in `[0, k]`.
5. Let `crossed20` be true when the counter was below 20 and `counter + delta ≥ 20`.
6. `counter += delta`. If the counter is now negative: set it to 0, run `ExtinguishFire`, stop.
7. **Stage**: `s := the first i in 0..4 with counter < D3cum[i]`, else 5. Compare it with the fire's current
   stage:
   1. If `s` differs and `s ≥ maxStage`: if the no-burn-down flag is clear, run `BurnDownFire` and stop.
      Otherwise skip to step 10 (the fire stays at its stage).
   2. If `s` differs and `s < maxStage`: set the stage to `s`. If `counter ≥ 20`, set `crossed20`.
8. **Lot condition**. If `t` is a building, or a prop with `crossed20`, and the lot manager gives `t` a lot:
   - set the lot's condition (lot slot 36) to **6** if `s ≥ maxStage − 1`;
   - else to **5** if `s ≥ maxStage − 2`.

   The comparisons are unsigned. With `maxStage = 2`, `maxStage − 2 = 0`, so the lot takes condition 5 at the
   first step and 6 at stage 1 *[quirk]*.
9. **Spread**:
   1. `q := 1.0f / float(D4[s])`, in `float` (D4 = 0 gives infinity, so the fire always spreads).
      `r := RandomDoubleRangeUniform(0, 1)` (the `mul` path). This draw is made only by a step that reaches this
      point: steps that stopped earlier, or that burned the target down in step 7, draw nothing here.
   2. If `r < q`, choose a cell (`ChooseSpreadCell`, `0x005792B0`):
      1. Take the fire's bounding box (occupant slot 10, min and max x and z in metres) and
         `rad := float(D5[current stage])`.
      2. `(wx, wz)` := the weather simulator's wind vector (slot 22), each × D6.
      3. Widen the box symmetrically: `minX −= wx + rad`, `maxX += wx + rad`, and the same for z with `wz`. A
         negative wind component therefore narrows the box.
      4. `x := RandomDoubleRangeGaussianFast(minX, maxX)`, then `z := RandomDoubleRangeGaussianFast(minZ, maxZ)`
         (three `mul` draws each).
      5. Convert (x, z) to a cell (city slot 95, `PositionToCell`).
   3. If `0 ≤ cx < cellsX` and `0 ≤ cz < cellsZ` (city slots 93 and 94, `+0x174` and `+0x178`, the cell counts):
      `IgniteCell(cx, cz, true, false, fire's ID)` (§5.5). The new fires join the same episode and location.
10. Done.

`maxStage` is the target's property `0x49BEDA31` (default **2**). Stages above `maxStage` are never shown.

**`KillFire(fire)`** (`0x0057B640`):

1. Post `0x49E3DB1D` with data2 = 0 (fire ended).
2. `removed := layer.UnregisterDisasterLocationByOccupant(instance, target)`. It returns true if it removed at least
   one (instance, occupant) entry (`0x00577DE0`).
3. **Only if `removed`**: look for another fire in the list with the same ID (fire occupant slot 13). If there is
   one, register its target: `RegisterDisasterLocation(instance, 0, 0, that target, zoomLevel, false)`. Otherwise
   erase the ID from the set (`0x0046FAB0`). When nothing was removed (the dying fire's target was not the ID's
   registered occupant), the ID and the locations stay as they are.
4. Remove the fire occupant from the occupant manager (`RemoveOccupant(fire's occupant, true, 0)`, so its
   `Shutdown` runs, §5.11) and from the list.

**`BurnDownFire(fire)`** runs `KillFire`, then `ToastOccupant(target)`.

**`ToastOccupant(o)`** (`0x005796E0`):

1. Play the visual effect "burndown_building", or "burndown_flora" for flora. If `o` still burns, extinguish it.
2. **Flora**: play "fire_flora_scorch", then hand `o` to the flora simulator (slot 23, `+0x5C`; flora.md).
3. **Building or prop**: if the prop property `0x2A89B677` is true (default true) and `o` has a lot:
   1. `demolition.DemolishRegion(1, the lot's region, 0, 0x300 | property 0x6A43150F, …)`. The flags are those
      of demolition.md §4.9: explode, except on water-compatible lots.
   2. If the occupant's `0xAA5C04C8` (radiation spew radius) is above 0: pollution slot 66 (`+0x108`) at its
      position with `0xAA5C04C5` and `0xAA5C04C8`. A burning nuclear plant spews radiation.

   Otherwise remove `o` from the occupant manager (slot 21).
4. `toasted += 1`.

### 5.11 The fire occupant (class `0x69D25C5F`)

A fire occupant is a visual occupant placed on its target. It holds:

- the counter;
- the stage and the maximum stage;
- the fire ID;
- the number of hoses (`SetHosed(+n)` / `(−n)`, clamped at 0);
- the extinguish time;
- the top- and side-flame counts.

`SetTarget(t)` does the following:

1. Read `maxStage` from `0x49BEDA31` (default 2).
2. Compute the flame counts from D7 and D8 at the target's bounding-box top and side areas.
3. Set `t`'s `0x69DBEBE1` ("burning") to true and call occupant slot 12 with 2 *[inferred: highlight state]*.
4. For a building, attach the automata controller "fire_occupant" (city slot 47, controller manager slot 10, the by-name `AddControllersFromOccupantGroup`; automata.md §8.2).
   That brings the fleeing people.

Removing the fire occupant clears `0x69DBEBE1`.

### 5.12 Timing in the harness

With no fire station (coverage 0, `k = 0`) and the default `maxStage` 2, the counter rises by 1 per second. The
stage reaches 1 at 30 and 2 at 55, so the building burns down at the **55th update** after an ordinary ignition.
The episode start sets `next` to the current time, so the first update runs at the first tick after the ignition
and update *n* runs about *n* − 1 seconds after it: the burn-down comes **≈ 54 real seconds** after the ignition. A
forced ignition starts at 30, so it burns down at the 25th update (≈ 24 s). The fire spreads each second with
probability 1/4 at stage 0 and 1/3 at stage 1.

## 6. Riot

### 6.1 Tuning

The riot reads exemplar `0x6534284A` / `0x29B6C670` / `0x4A38206D` ("Riot"). The manager reads the first two
properties once at its `Init` (`0x0057D280`) into two process-wide statics. Each instance reads the rest at its
`Init` (`0x0057E3D0`): it first sets the defaults, then lets each property that is present override its value.

| Property | Meaning | Default | Shipped |
|---|---|---|---|
| `0xCADB6B3E` | maximum riots at once (static `0x00B0A1E8`) | 3 | 3 |
| `0x2ADB6B6D` | maximum leaders, a process-wide static (`0x00B0A1EC`) | **5** | 10 |
| `0xEA400620` | number of stages | 6 | 6 |
| `0xEA400621` | seconds per stage | 30 | 30 |
| `0xEA400622` | stored at instance `+0x54` and **never read** | 3 | — |
| `0xEA400623` | leaders spawned on reaching stage *s* | 0, 1, 0, 1, 0, 0 | 0, 1, 0, 1, 0, 0 |
| `0xEA400624` | police curve: coverage → suppression | see below | (0, 0) (64, 0) (65, 1) (255, 5) |
| `0xCA821B8C` | zoom level (u8, instance `+0x74`) | none | 3 |

Defaults in detail:

- The two statics keep their initial values (3 and 5) when the exemplar or the property is missing.
- The police curve: when the exemplar exists but the curve has no points, it becomes (64, 0) (65, 1) (255, 3).
  When the exemplar is missing, the curve stays empty.
- The zoom byte is not initialised by the constructor and has no default. A port should use 0 when the property is
  missing *[choice]*.
- `0xEA400622` is unused: nothing in the riot code reads `+0x54`. A port need not keep it.

### 6.2 Start (`AttemptDisasterStart(x, z)`, `0x0057EA10`)

The aura simulator passes cell coordinates (aura.md). If `GetActiveInstanceCount()` (manager slot 10, the
instances in phase 1) is below the maximum riots (§6.1):

1. Create an instance (AddRef) and add it to the manager.
2. `Init` it (§6.3).
3. `AddRiotLeader(x, z, false)` (§6.4).
4. Release the creation reference. The manager's list keeps its own.

The result is **always null** *[compat]*, also when an instance was created (`0x0057EA10` returns 0 on every path).
Callers cannot tell a riot from a refusal.

Ended riots (phase 3) do not count in `GetActiveInstanceCount`, so they never block a new riot. An instance whose
first `AddRiotLeader` failed stays in phase 0 and does not count either.

### 6.3 Instance `Init`

1. Register with the animation tick manager.
2. Start a timer with units 5 (seconds) (`0x00890181`, then started at `0x008905C4`). `next` (`+0x48`) is not
   touched: the constructor set it to 0 (`0x0057DB29`).
3. For each leader already in the list, mark the network around the cell of its last network occupant
   (`0x0057E85C`). At a fresh start the list is empty, so this marks nothing; it matters only for an instance that
   already has leaders. The locking itself is `MarkRiotNetworkOccupants` (`0x0057D300`), which marks the network
   occupants within ±3 cells as riot-locked:
   each one's player removal counter is decremented (`UnsetRemovalFlags(2)` on `AsOccupant()`, network occupant slot
   48), so the player cannot bulldoze it until the riot manager shuts down (§6.7). This is not a network flag and
   does not touch traffic. A start within 3 cells of the west or north edge locks nothing *[quirk]*. Full rule:
   network-manager.md §10.

**Where locks are taken.** `MarkRiotNetworkOccupants(unused, x, z)` loops x outermost with unsigned bounds and calls
`LockNetworkOccupant` (`0x0057DA40`) on each tile occupant: a null occupant or one already in the riot's lock set is
skipped; otherwise it is AddRef'd, its removal flag 2 is unset (`AsOccupant`, network occupant slot 48, then
`UnsetRemovalFlags(2)`), and it joins the set. Only the riot calls it. It has exactly three callers:

1. `AddRiotLeader`, after the location registration (`0x0057DF18`, §6.4 step 5);
2. the update step, when a surviving leader stands on a new network occupant (`0x0057E382`, §6.5 step 6);
3. `Init`, for leaders already in the list (`0x0057E85C`, step 3 above).

### 6.4 `AddRiotLeader(x, z, spawned)`

1. Find the nearest road tile (`FindNearestRoadTile`, `0x0057D4C0`), which rewrites (x, z) to the cell it
   found. Fail if there is none. The search uses the city's `SpiralIterator` in its clockwise mode, exactly as
   plumbing.md §7.2 describes it: the centre (x, z) first, then ring 1, ring 2 and so on, each ring starting at
   (x + r, z − r + 1) and running +z first. The bounding box is the whole city,
   `[0, cellsX − 1] × [0, cellsZ − 1]`, so cells outside it are skipped. Rings 0 to 7 are searched (Chebyshev distance ≤ 7). At each cell, take the
   traffic network map's tile (city slot 40, map slot 14); the first cell whose tile occupant is of network type
   0, 3 or 6 (network occupant slot 22, road, street and avenue) wins.
2. Read the police coverage at the cell (police simulator slot 5 grid, `GetCellValue`). Require
   `trunc(curve(coverage)) ≤ 1` and `leaders < ` the maximum leaders (§6.1; 10 shipped). With the shipped curve
   that is coverage ≤ 112.
3. Create a pedestrian automaton of kind `0x4107` (city slot 44, pedestrian manager slot 15):
   1. Place it at the cell's corner (city slot 96, `CellCornerToPosition`) plus `(16 × u1, 16 × u2)` metres, on
      the terrain height. `u1` then `u2` are two `RandomDoubleUniform()` draws on the riot generator
      `0x00B22B80`, made only after the checks of steps 1 and 2 passed. The offset is therefore in [0, 16] m on
      each axis, not centred.
   2. Set it to riot-leader mode (automaton slot 51 with 8) and start it (slot 9 with 1).
4. `layer.RegisterDisasterLocation(instance, 0, 0, automaton, zoomLevel, not spawned)`.
5. Mark the network around the cell (§6.3) and run `UpdatePhase` (§6.7).

A new leader record starts with stage −1 and maximum stage −1 and a counter of 0. Its "last network occupant"
(record `+0x18`) starts as the tile occupant that `FindNearestRoadTile` returned (`0x0057DE67`–`0x0057DE78`), not
null, so the first step does not re-mark the start cell. Records are appended at the end of the leader list
(`push_back`, `0x0057D8D0`).

### 6.5 Update (once per real second)

`Update` (`0x0057DFE0`) runs on every animation tick but does its work only once per whole second: it reads the
timer with the integer `GetElapsedTime` (units 5, truncated seconds), and when `elapsed ≥ next` it sets `next :=
elapsed + 1` (no backlog is kept) and runs the step. `next` starts at 0 (constructor, `0x0057DB29`; `Init` does not
change it), so the first step runs at the first tick after `Init`, at elapsed 0. Step *n* therefore runs at about
*n* − 1 seconds.

In the step, the leader list is walked from the front; before each leader is processed the next node is remembered
(`0x0057E026`). A leader spawned in step 4 is appended at the end, so it is processed **in the same pass** when the
spawning leader was not the last node, and not when it was *[compat]*.

For each leader:

1. `counter += trunc(1.0f − curve(police coverage at its cell))` (truncation toward zero, `_ftol`). With a curve
   value below 2 this adds 1 or 0; only a curve of 2 or more makes it negative.
2. `stage := counter / secondsPerStage` (unsigned division), or −1 when the counter is negative or when the
   automaton is no longer in the occupant manager.
3. When the stage changes, update the riot effects ("Riot%d" automaton effects). The first step moves a new
   leader from −1 to stage 0.
4. When `maxStage < stage < numStages` (signed):
   1. `maxStage := stage`.
   2. If the riot's leader count is below the maximum leaders (checked once) and `0xEA400623[stage] > 0`, call
      `AddRiotLeader(cell, true)` that many times (each call checks the maximum again).
   3. Send the formatted riot message for the stage (format string at `0x00B22B9C`) to the automata controller
      manager (city slot 47, slot 5). Stage 0 sends one too, at the first step.
5. The leader **ends** when its stage is ≥ `numStages` or −1:
   1. Set the automaton back (slot 51 with 1) and detach its controller (controller manager slot 14).
   2. `layer.UnregisterDisasterLocationByOccupant` (layer slot 15).
   3. Release the automaton, erase the leader and release its last network occupant.
6. Otherwise:
   1. `cell := floor(position / 16)`, from the automaton's position (automaton slot 7), computed exactly (× 0.0625,
      then the 12582912 rounding with a −0.5 bias; constants `0x00A9E77C`, `0x00A84D2C`, `0x00A9E780`). Take the
      tile at that cell (traffic map slot 14, `SafeGetTileInfoAtCell`, §6.4) and its occupant (the tile's first
      field). If that occupant is **non-null** and differs from the leader's last network occupant, mark the
      network around the cell (`MarkRiotNetworkOccupants`, §6.3; `0x0057E382`) and remember it (AddRef the new,
      Release the old). **A walking leader keeps locking more road** *[compat]*. Only the cell under the leader at
      each step is sampled; cells it crossed between two steps are not.
   2. The leader follows the road under it (`DoRiotLeaderTick`: random turns on the traffic network).

Unopposed (coverage 0), a leader gains 1 per step. Its counter is *n* after step *n*:

- it reaches stage 1 at step 30 (≈ 29 s), where it spawns 1 leader;
- stage 3 at step 90 (≈ 89 s), where it spawns another;
- it ends at step 180 (≈ 179 s).

If the coverage under a leader rises to 255 (curve 5), it loses 4 per step and ends at the next step (stage −1).

### 6.6 Followers set fires

Pedestrians following a leader enter an "ignite" state with a target occupant. In that state the pedestrian
calls the fire manager's `IgniteOccupant(target)` and `SetCause(2)` (arson), then leaves the state
(`cSC4Pedestrian::DoIgniteTick`). Riots therefore start fire episodes of their own. Each one goes through §5,
including its own `DisasterHasBegun`.

### 6.7 Phase

- **0 → 1** when the first leader exists: `layer.DisasterHasBegun`, and history `0xAA8C87C7` + 1.
- **1 → 3** when no leaders remain (`UpdatePhase` `0x0057D390`): unregister the instance's locations, then
  `layer.DisasterHasEnded`. Nothing else happens.

**The instance does not leave its manager after phase 3** *[compat]*. It stays registered on the animation tick
and keeps running an empty `Update` every second. Nothing calls its `Shutdown` until the riot manager's own
`Shutdown`, when the city closes. Because the network locks are released only when the manager's list becomes
empty or at the manager's `Shutdown` (network-manager.md §10), **every piece a riot ever locked stays locked for
the rest of the session**. An instance whose first `AddRiotLeader` failed stays in phase 0, ticking, in the same
way.

## 7. Earthquake (short form)

*Short form*: the terrain deformation is described by its tuning and its order of operations, not by its
geometry. A port may draw a different fault line, provided it uses the same brush tuning.

- **Start** (`AttemptDisasterStart(x, z)`, slot 6, `0x00576D80`). Only when the manager's list is empty; otherwise
  return null (budget.md §4.3):
  1. Create the instance, add it to the manager and `Init(x, z)` it.
  2. Unless its state is already 2: clock := 0, state := 2, and start the deformation (`StartDeformation`,
     `0x00575C40`).
  3. Return the instance.

  Step 2 runs in the same call as `Init`, so **the rumble state 1 never gets a tick** under the manager. Nothing else
  calls `Init` or sets state 1 in this build (`Init` has one caller, `0x00576DCC`; the disaster menu also goes
  through slot 6, §11), so the rumble code is dead *[inferred: no other caller found]*. The intensity therefore keeps
  its constructor value **0.1**, every started quake uses brush strength `0x4A08C500(0.1)` = 0.19 and width
  `0x4A08C501(0.1)` = 0.55, and "QuakerShaker" never plays.
- **`Init`**:
  1. The first time, read the tuning (below).
  2. Clamp the epicentre to `[0, SizeX − 1] × [0, SizeZ − 1]` in world units (city slots 89 and 90).
  3. Compute the fault line (`ComputeFaultLine`, `0x00576640`; below).
  4. Register with the animation tick manager (`0xC0`) and play "QuakeStart".
  5. State 1 places automata repulsors along the line.
  6. `layer.RegisterDisasterLocation(instance, cellX, cellZ, null, 0, false)`.
  7. `layer.DisasterHasBegun(instance)`.
- **States** (each animation tick adds the tick's microseconds to a state clock, data2 of `0x86AD10EE`):
  - 1, rumble: the intensity grows by `µs × 1e-7`, so it reaches 1.0 after 10 s. "QuakerShaker" plays every
    `0x4A08C508` µs.
  - 2, deformation: see the next item.
  - 3, settle: lasts 3,000,000 µs (3 s), then goes to 4.
  - 4, end: stop accumulating terrain texture updates, unregister the locations, `layer.DisasterHasEnded`,
    remove the instance from the manager and shut it down.
- **Deformation** (state 2):
  1. Set the brush strength and width from the intensity: `0x4A08C500` and `0x4A08C501` at the epicentre.
  2. The brush travels the fault line at `0x4A08C507 / 1e6` metres per µs, in 8 m steps.
  3. Each step applies terrain brush `0x11` on one side and `0x12` on the other (terrain slot 57, `ApplyBrush`).
     The strength scales with `0x4A08C502` and the width with `0x4A08C503`, both by the distance travelled.
  4. Each tick then calls terrain slot 78 (`+0x138`) and redraws the touched rectangle of each side (terrain slot
     68, `+0x110`, once per side).
  5. It ends (state 3, terrain `EndLongTerrainOperation`) when the line is done, the strength drops to 0.001 or
     below, or the width falls under 0.002.

  The damage to buildings is the terrain's response to the brush *[inferred: the earthquake itself demolishes
  nothing]*.
- **Deformation in detail** (`StartDeformation` `0x00575C40`, `DoDeformation` `0x00575DE0`):
  1. At the start: `strength := 0x4A08C500(intensity)`, `width := 0x4A08C501(intensity)`; the per-step parameter
     step is `8 / (SizeX × 3)` (city slot 89); two "Quaker" effects start; terrain slot 87 begins the long terrain
     operation.
  2. Each tick: `distance += µs × speed`; the number of steps is `floor((distance − travelled) / 8)`.
  3. Each step takes the next point pair from the two polylines (`GetNextPointsOnFaultLine`), computes
     `str := 0x4A08C502(travelled) × strength` and `wid := 0x4A08C503(travelled) × width`, stops (state 3) when
     `str < 0.001` or `wid < 0.002` or the line is done, applies brush `0x11` to side A and `0x12` to side B (terrain
     slot 57), places a "Quaker" effect every `0x4A08C504` metres, and adds 8 to `travelled`.
- **Fault line** (`ComputeFaultLine`). All draws use the earthquake generator `0x00B22AE8`, each a
  `RandomDoubleRangeUniform`:
  1. Once per process, a base angle in `[0, π]` (kept in a static, `0x00B22ADC`).
  2. `angle := base + RandomDoubleRangeUniform(−slope, +slope)`, with `slope` = `0x4A08C506` in radians;
     `dir := (cos angle, sin angle)`.
  3. Segment length `L := SizeX_world / 3`, where `SizeX_world − 1` is the clamp bound of `Init`.
  4. Two polylines, A and B, both start at the epicentre. Each gets three more points: point *k* is the previous
     point plus `L` along `+dir` (A) or `−dir` (B), plus a perpendicular offset. For the first step one draw `d` in
     `[−curviness, +curviness]` (`0x4A08C505`) gives A the offset `+d` and B `−d`. For the second and third steps,
     A and B each draw their own offset (A first).

  That is 1 + 1 + 1 + 2 + 2 draws, the first only once per process. Each polyline has four points, so the line
  spans `3L` (the city's width) in each direction from the epicentre.
- **Tuning** (`0x6534284A` / `0x29B6C670` / instance **2**, "Earthquake"). It is read once per process, at the
  first `Init` (static flag `0x00B22AD8`), and only if the exemplar exists. The values live in process-wide
  statics whose initial values are the defaults: curves empty; spacing 40; curviness 30; slope 0; speed 25;
  shaker period 10000 µs; repulsor spacing 100. After the reads, still inside the "exemplar exists" branch, the
  slope is multiplied by π/180 (a `double` constant, stored back as `float`) and the speed by 1e-6 (to metres per
  µs), **whether or not the property was present**. A missing speed property therefore gives 25 m/s. If the whole
  exemplar is missing, no conversion runs and the speed stays 25 *metres per µs* *[quirk]*: the brush crosses the
  whole line in one tick. Shipped:
  - `0x4A08C500` = (0, 0.1) (1, 1);
  - `0x4A08C501` = (0, 0.5) (1, 1);
  - `0x4A08C502` = (0, 1) (200, 1) (800, 0.8) (1200, 0.3) (2048, 0);
  - `0x4A08C503` = (0, 1) (500, 1) (900, 0.82) (1800, 0.2) (3000, 0);
  - `0x4A08C504` effect spacing 35 m;
  - `0x4A08C505` curviness 300;
  - `0x4A08C506` maximum slope variation 4° (converted to radians);
  - `0x4A08C507` speed 400 m/s;
  - `0x4A08C508` 10000 µs;
  - `0x4A08C509` repulsor spacing 100 m.

## 8. Pipe burst (short form)

- **Start** (`AttemptDisasterStart(x, z)`, `0x0057D180`):
  1. Ask the occupant manager for a watered pipe in cell (x, z), through the manager's pipe filter (a cell query
     that returns the first match). Return null if there is none.
  2. Otherwise create an instance for that pipe (`CreateDisasterInstance`, `0x0057CF90`). The instance asks the
     occupant for `0x49C05B9F` (the pipe occupant).
  3. Return null if its `Init` fails.

  The argument-less overload starts from a random cell (a fresh `cRZRandom` seeded with −1) and spirals outward
  until it finds a watered pipe.
- **`Init(pipe)`**:
  1. Read the tuning the first time.
  2. Add the instance to the manager.
  3. Subscribe to `0xC6CDBF1A` (view level), `0x99EF1142` and `0x99EF1143`.
  4. Register with the animation tick manager.
  5. Start a leak at the pipe (below). Fail when the pipe already leaks.
  6. `layer.RegisterDisasterLocation(instance, 0, 0, pipe, zoomLevel 3, true)`.
  7. `layer.DisasterHasBegun(instance)`.
- **Leak**:
  A leak is a record of the instance (cell, position, clock, burst time, effect). **The pipe burst inserts no
  occupant of its own** into the occupant manager; the only occupant involved is the existing pipe. `StartLeakAtPipe`
  (`0x0057C7C0`) refuses a cell that already has a leak, else appends a record and creates its effect
  (`CreateLeakEffectForPipe`, `0x0057C5B0`):
  1. It shows "burst_pipe_aboveground" or "burst_pipe_underground" when the view level is 1 or 3; at any other
     level no effect is made. In a session without a view the instance takes level 1.
  2. Whether or not an effect was made: clock := 0, burst time := `RandomSint32RangeUniform(lo, hi)` µs on the
     pipe generator `0x00B22B60` (foundations §6.1: `lo + RandomUint32Uniform(hi − lo)`), with `lo`, `hi` the tuning
     in µs; shipped [30, 60) s.
  3. It posts `0xEA1F0DCD` (x, z) to tell plumbing the cell lost water (plumbing.md).

  A view-level change (`0xC6CDBF1A`) recreates the effect of **every** leak, so it also resets every clock, draws
  a new burst time and posts `0xEA1F0DCD` again.
- **Leak removal by occupant messages.** On `0x99EF1143` (occupant removed), and on `0x99EF1142` (occupant
  inserted) for an occupant without the preview property `0x029244DB`: if the occupant is a network piece
  (`0xC772BF98`) that is a pipe (`0x49C05B9F`), the leak at that pipe's cell, if any, stops its effect and is
  erased without bursting. Rebuilding or bulldozing a leaking pipe therefore ends that leak.
- **Tick**:
  1. First the wait entries (`TickLeaksInWait`, `0x0057CB30`): each ages by 1; at age 4 it starts leaks at its
     cells (each cell's pipe through the pipe filter) and is erased. Leaks therefore start on the fourth tick after
     the burst's tick, so a burst spreads along the pipe.
  2. Then each leak adds the tick's µs (unsigned). When a leak **with an effect** has `clock ≥ burst time`, run
     `BurstPipe(x, z)` (`0x0057CB90`), stop the effect, and erase the leak. `BurstPipe`:
     1. Find the cell's pipe (occupant manager slot 32 with the pipe filter). If there is none, do nothing at all.
     2. Play "pipe_explodes" at the pipe's position.
     3. For direction `i` = 0 to 3 with cell offsets (−1, 0), (0, −1), (+1, 0), (0, +1) (tables `0x00A9E700` and
        `0x00A9E6F0`): if the pipe connects in direction `i` (network occupant slot 47 with `(i, 4)`, 4 being the
        pipe network) and the neighbour cell is inside the city, collect it. If any were collected, append one
        wait entry (age 0) with them.
     4. If the demolition utility exists: `demolition.DemolishRegion(1, pipe, 0, 0x302, 0, 0, 0)`: cause 2,
        explode (demolition.md §4.9); then post `0xEA1F0DCD` (x, z).
     5. Dig a sinkhole (`0x0057C1A0`): terrain brush `0x14` (terrain slot 57, `ApplyBrush`) with a brush record
        whose start and end are both `(x × w, z × w)`, `w` being terrain slot 6 (the cell size), strength 1.0,
        width 1.0, `+0x10` = 0, the byte `+0x14` = 1, `+0x20` = 3 and `+0x34` = 100.0 (the meaning of the last
        four fields is not established; the earthquake's records use the same layout for start, end, strength
        and width).
  3. When no leaks and no waiting entries remain: `layer.UnregisterDisasterLocations`, `layer.DisasterHasEnded`,
     then `Shutdown` (§3.1).
- *[quirk]* A leak without an effect, because the view was at a level other than 1 or 3 when the leak started or
  the effect could not be created, never bursts until a view-level change recreates its effect *[the view-level
  message `0xC6CDBF1A` is the only path that recreates it]*. A session without a view takes level 1, so the effect
  is attempted; whether the effects manager creates it with no 3D view needs a runtime check (§17 Q2). If it does
  not, the instance never ends and the emergency pause never lifts.
- Tuning: instance **3**, "PipeBurst": `0x4A08C600` = 30, `0x4A08C601` = 60, `0xCA821B8C` = 3. Read once per
  process at the first `Init` (`0x0057C220`) into statics: the burst-time bounds in µs (`0x00B0A1D8`,
  `0x00B0A1DC`, each `ftol(seconds × 1e6)`) and the zoom byte (`0x00B22B59`). Defaults when the exemplar is
  missing: **5 s and 10 s** (5,000,000 and 10,000,000 µs) and zoom 0. When the exemplar exists but lacks a bound,
  the original reads an uninitialised `float` local for `0x4A08C600`, or reuses `0x4A08C600`'s value for
  `0x4A08C601` (both reads share one local); a port should keep the default for a missing bound *[choice]*.

## 9. Toxic spill (short form)

- The manager subscribes to the cheat command `0x0BFB41BA` "ToxicSpill" (x, z, amount).
- **`AttemptDisasterStart(x, z)`** spills **120** units at cell (x, z). There is one instance, created on first
  use.
- **Instance `Init`**:
  1. Start the timer (milliseconds *[inferred: the 1000 step below]*).
  2. Add the instance to the manager and register with the animation tick manager (priority `0xFF`).
  3. `layer.DisasterHasBegun(instance)`.
- **`SpillToxicWaste(x, z, amount)`**:
  - If a toxic occupant already sits on the cell, add `amount` to it.
  - Otherwise:
    1. Create a toxic occupant on the cell with `amount`.
    2. Insert it in the occupant manager.
    3. `layer.RegisterDisasterLocation(instance, x, z, occupant, 0, false)`.
- **Update**, every 1000 ms. For each toxic occupant:
  1. `amount −= 1` if nobody hoses it; else `amount −= 4 × hosed`, floored at 0.
  2. At 0: remove it from the occupant manager, release it, and `layer.UnregisterDisasterLocationByOccupant`.

  When none remain, the instance shuts down: it unregisters from the tick, unregisters its locations, calls
  `layer.DisasterHasEnded` and leaves the manager.
- An unattended spill of 120 lasts 120 s. With one hazmat crew it lasts 30 s.

## 10. Volcano (short form)

- **Start** (`AttemptDisasterStart(x, z)`, slot 6, `0x0057F600`). Only when the manager's list is empty:
  1. Create the instance and add it.
  2. Set the location (`0x0057F7A0`: store x and z, and register a location at their cell, `PositionToCell`,
     with no occupant and zoom 0).
  3. `Init`.
  4. Go to state 3.
  5. Return the instance.

  The argument-less start (`0x0057F5A0`) makes a clock-seeded generator and draws `x := RandomUint32Uniform(cellsX)`
  then `z := RandomUint32Uniform(cellsZ)` (city slots 93 and 94), and passes them to slot 6, which takes world
  units *[quirk]*: such a volcano always lands within the first `cellsX` × `cellsZ` metres, the north-west corner.
- **`Init`**:
  1. Clamp the location to the city.
  2. Subscribe to `0xCA493C1D`.
  3. Read the tuning.
  4. Register with the animation tick manager (`0xC0`).
  5. Set state 1.
  6. Register the location and call `layer.DisasterHasBegun`.
- **States** (`SetState`, `0x0057F820`; setting the current state again does nothing). All of 1 → 3 → 5 happen
  inside the start call, with no tick in between:
  - 1: set by `Init`.
  - 3: start the "volcano_control" effect once (`StartDeformation`, `0x0057F4F0`), created so that its completion
    posts `0xCA493C1D` with data1 = the instance. Then, if lava is enabled (`0x6A4BC8ED`), no lava flows, and no
    lava has run before: create the lava (`cSTEVolcanoLava`, rates 0) and go to state 5.
  - 5: if lava is enabled and none flows: clock := 0 and start the flow at the vent with radius `0x0A4BC752`
    (as `float`).
  - 7: the **end request**, sent by `0xCA493C1D` for this instance (the "volcano_control" effect finished). Mark
    "end requested". If lava is flowing, stay in the flow (state 5 again, which changes nothing); otherwise go to
    state 8.
  - 8: if lava is enabled, end the lava (`EndVolcanoLava`). Then, if no lava flows, remove the instance from the
    manager and shut it down (`0x0057F520`).
- **Tick** (`0x0057FB90`), only while lava is enabled and flowing: `clock += µs` (unsigned); `t := clock × 1e-6`
  seconds. When `t > T` (strictly): `EndVolcanoLava`, then state 8 if the end was requested, else state 3. Back in
  state 3 the effect already runs and the lava has run once, so the volcano waits there for the end request.
  Otherwise, in state 5, set the rates below.
- **`EndVolcanoLava`** (`0x0057F740`): shut the lava down and mark that lava has run; then, if the disaster had begun,
  `layer.DisasterHasEnded()` and `layer.UnregisterDisasterLocations(instance)`. **The emergency pause therefore ends
  when the lava stops (T seconds after the start), not when the instance goes away.**
- **Lava**:
  1. Each tick sets the production rate to `curve 0xEA4BC6E4(t / T) × 0xAA4BC72A / 100000` and the
     solidification rate to `curve 0x4A4BC714(t / T) / 100000`, with `T = 0x0A4BC742` = 50 s. *[quirk]* Both
     rates are 0 when the **production** curve has no points: the solidification guard tests the production
     curve's point count.
  2. After T, the lava ends and the state goes to 3, or to 8 if the instance was asked to end.
  3. The lava ignites fires where it flows (§4).

  The lava only flows when `0x6A4BC8ED` is true (shipped true).
- Tuning: instance **1**, "Volcano":
  - production (0, 0) (0.075, 0) (0.25, 0.25) (0.8, 0.25) (1, 0);
  - solidification (0, 0) (0.4, 0) (0.9, 0.01) (1, 0.15);
  - scale 6.

  It is read at **every** `Init` (`0x0057F9F0`) into process-wide statics. Defaults when the exemplar is missing:
  T = 20 s, scale 1, radius 100, curves empty, and lava **disabled**. When the exemplar exists, the lava flag is
  first reset to false and then taken from `0x6A4BC8ED` if present.

  The lava itself is a terrain class (INVENTORY rank 5).

## 11. Effect disasters and the disaster menu (short form)

The disaster menu's entries are exemplars of group `0x29B6C670`:

| Instance | Name |
|---|---|
| 1 | Volcano |
| 2 | Earthquake |
| 3 | PipeBurst |
| 4 | FlameItem |
| 5 | Meteors |
| 6 | Tornado |
| 7 | Lightning |
| `0x0A760F65` | RobotAttack |
| `0x0C059A77` | Truckasaurus |
| `0x4B9547F1` | UFOs |
| `0x4A38206D` | Riot |

Their properties:

- `0x4A08C700` is the visual effect's name.
- `0x4A08C701` is a manager type. Meteors, UFOs and Truckasaurus give `0x6A6D0EE7`, the tornado `0x02963821`,
  the robot `0xCA9C3DA7`.
- `0x4A08C702` is a bool.

The menu tool (`cSC4ViewInputControlEffectDisaster`):

1. When the entry's manager type is one of the seven layer managers, the tool calls that manager's
   `AttemptDisasterStart` (slot 6) on mouse-up.
2. Otherwise, the layer has no such manager. The tool uses the effect-disaster manager (`0x6A7B4397`):
   1. It places the visual effect.
   2. It hands it to `StartDisasterEffect(effect, position, …)`, which creates an effect instance.

The effect-disaster manager and its instances read no tuning.

**Effect instance**:

1. `Init` adds it to the manager, subscribes to `0x2A7B582D` (effect finished) and `0x2A7B582E` (effect moved),
   and calls `layer.DisasterHasBegun`.
2. `0x2A7B582E` for this instance updates its location.
3. `0x2A7B582D` for this instance shuts it down, which ends the disaster.

The damage comes from the effect's actions in the effects manager (INVENTORY rank 7): fires in a radius with cause
4, and toxic spills.

## 12. Messages

| ID | Sender | data |
|---|---|---|
| `0xCA1B1E64` DisasterPeriodStarted | layer, every `DisasterHasBegun` | data1 ongoing count, data2/data3 location, data4 instance ID, void1 instance |
| `0xAA1B2002` DisasterPeriodEnded | layer, when the count reaches 0 | — |
| `0x49E3DB1D` fire started / ended | fire | data1 fire occupant, data2 1 / 0 |
| `0x49D15B73` ignitions | fire `Update` | data1 buildings, data2 flora |
| `0x49D15B72` severity | fire | 1 when fires appear, 0 when they end |
| `0xA9C7CAB6` phase | fire | data1 `0x621CDA33`, data2 phase |
| `0xEA1F0DCD` pipe burst at a cell | pipe burst | x, z |
| history `0x49D15B74` + 1 | fire, per episode | — |
| history `0xAA8C87C7` + 1 | riot, per riot | — |

The layer subscribes during a disaster to `0x99EF1143`, `0xE94671EC`, `0xE94671EB`, `0x894671F6` and
`0xE94671ED` (§2.5).

## 13. Random numbers *[compat]*

- **Fire.** The fire instance's own `cRZRandom` (§1) draws, per call ("raw" is `RandomUint32Uniform()`, which
  advances `lcg`; the `double` draws advance `mul`):
  - `Ignite`: a raw fire ID, only when `fireID` is 0 and every refusal check passed (§5.4);
  - `IgniteCell`: with `useProbability`, one `RandomDoubleRangeUniform(0, 100)`; then, if it passed and `fireID`
    is 0, one raw fire ID (§5.5);
  - `IgniteCellRadius`: one raw fire ID, first and always (§5.6);
  - `UpdateFire`: one `RandomUint32Uniform(k + 1)` when `k > 0`; then, only if the step reaches the spread
    (§5.10 step 9), one `RandomDoubleRangeUniform(0, 1)`; and when it spreads, two
    `RandomDoubleRangeGaussianFast` (x then z, three draws each);
  - `ExtinguishFire`: one `RandomUint32Uniform(4000)` when the fire has no extinguish time yet;
  - `AddToToastList`: one `RandomUint32Uniform(n)` per insertion into a non-empty list of `n` entries (§5.7).

  The order follows the list order of the fires, and nested calls draw in call order (a spread's `IgniteCell` draws
  before the next fire's step).
- **Riot.** The riot's generator (`0x00B22B80`) draws two `RandomDoubleUniform()` per created leader (x first,
  then z), after the leader passed its checks (§6.4).
- **Pipe burst.** The pipe burst's generator draws the burst time per leak. Its argument-less start uses a
  throwaway generator.

## 14. Persistence

Nothing in this spec is saved. The riot instance class has `Write`, `Read` and `GetGZCLSID` slots (vtable
`0x00A9E800`). The layer never writes it, because the layer is not in the city's record *[inferred]*. A
replacement must not add a record.

## 15. Interface slot tables *[compat]*

`cISC4DisasterLayer` (vtable `0x00A9E390`), `cISC4DisasterManager` and `cISC4DisasterManagerFire` (`0x00A9E4DC`)
are in `docs/abi/interfaces.tsv`. The proposed rows for the fire instance's interfaces and the toxic manager are in
`build-runs/re-specs12-rows.tsv` of the re-specs12 lane. The `cISC4DisasterLayer` slot 18 row (`GetDisasterCost`,
`0x00578900`) must say it returns a pointer to the statistics record of §2.6; the replacement row and the fire
occupant rows below are in `build-runs/re-answers23-abi-rows.tsv` of the re-answers23 lane.

**Fire instance, internal interface (`0x00A9E5B0`)**, called by the fire manager:

| Slot | Method | Pop |
|---|---|---|
| 3 | `Init` | 0 |
| 4 | `Update` | 0 |
| 5 | `Shutdown` | 0 |
| 6 | `Ignite(occupant, bool force, uint32 fireID, bool focus)` | 16 |
| 7 | `IgniteCell(x, z, bool useProbability, bool teleport, uint32 fireID)` | 20 |
| 8 | `ExtinguishCell(x, z, amount)` | 12 |
| 9 | `QuickBurnCell(x, z)` | 8 |
| 10 | `IgniteCellRadius(x, z, r0, r1)` | 16 |
| 11 | `Extinguish(occupant)` | 4 |
| 12 | `BurnDown(occupant)` | 4 |
| 13 | `SetCause(c)` | 4 |
| 14 | `GetCause()` | 0 |
| 15 | `GetFireCountInCellRect(rect)` | 4 |
| 16 | `GetFireCount()` | 0 |

`cISC4DisasterInstance` of the fire (`0x00A9E590`) is given in §3.2. The fire manager's `IgniteOccupant` (4),
`IgniteCell` (5), `ExtinguishCell` (6), `QuickBurnCell` (7), `IgniteCellRadius` (8), `SetCause` (9), `GetCause`
(10) and `GetFireInstance` (11) forward to the slots above.

## 16. Quirks

1. A manual pause blocks non-forced ignitions. The emergency pause does not (§5.4).
2. Lot condition 5 applies from the first step when `maxStage = 2` (§5.10 step 8).
3. Disabling disasters (§2.3) does not stop spontaneous fires, arson, riots, pipe bursts or RiskyMoney earthquakes.
4. Every removed occupant counts in the damage cost while a disaster runs, including the player's bulldozing
   (§2.5).
5. A pipe leak without an effect never bursts (§8).
6. `IgniteOccupant` returns true even when the ignition was refused (§5.4).
7. The riot's `AttemptDisasterStart` always returns null (§6.2).
8. A riot instance never leaves its manager after it ends, so its network locks last until the city closes (§6.7).
9. A walking riot leader locks every road it reaches (§6.5).
10. Only the first `SetCause` of a fire episode counts (§5.7).
11. The fire tuning defaults apply only when the whole exemplar is missing (§5.1).
12. A manager-started earthquake never rumbles and always has intensity 0.1 (§7).
13. The volcano ends the disaster when its lava stops (§10). Its argument-less start uses cell counts as world
    units (§10).
14. Disaster locations read by index keep the cell of their last resolution, (0, 0) if never resolved; they do not
    follow a burning building's or a walking riot leader's occupant (§2.7).
15. `GetDisasterCost` returns a pointer to a statistics record, not a number (§2.6).
16. A fire lit during an `UpdateAllFires` pass is stepped in the same pass unless the spreading fire was the last in
    the list (§5.8); the same holds for riot leaders spawned during a riot step (§6.5).

## 17. Open questions

- **Q1.** *(resolved)* D3–DA are `0xA9B6ECD3`–`0xA9B6ECDA` (§5.1). data-formats.md does not list them yet.
- **Q2.** Does a pipe-burst instance end in a session with no 3D view (§8 quirk)? This needs a runtime run with
  water funding below 50 %.
- ~~**Q3.** Is phase 2 ever used by an instance (§3.2)?~~ **ANSWERED 2026-10-15 (re-answers14):** yes, by the
  volcano: states 1 and 5–8 report phase 2 (§3.2, §10).
- ~~**Q4.** What does network flag 2 (riot-blocked) change in traffic? That belongs to the network manager spec
  (INVENTORY rank 3).~~ **ANSWERED 2026-10-15 (re-specs13c):** nothing in traffic. The "2" is the removal
  privilege bit, not a network flag: the riot decrements the occupant's player removal counter (cISC4Occupant
  slot 13 with 2), so `CanRemove(2)` is false and the player's bulldozer refuses the piece; it is incremented back
  when the riot manager's list becomes empty or the riot manager shuts down. network-manager.md §10.
  **Corrected 2026-10-15 (re-answers14):** an ended riot stays in its manager's list (§6.7), so in practice the
  locks are released only at the riot manager's `Shutdown`, when the city closes.
- ~~**Q5.** Which callers read `DisastersEnabled` (slot 6) or call `EnableDisasters` (slot 5)? No simulation-side
  caller was found (§2.3).~~ **ANSWERED 2026-10-16 (re-answers19):** only the player-drive input control calls
  `EnableDisasters`: false on entering driving mode (`0x00565189`), true at `EndInput` (`0x0056514E`). Nothing
  outside `EnableDisasters` reads `DisastersEnabled`; the counter has no effect (§2.3).

The cores13 implementers' questions Q6–Q25 (QUESTIONS.md DS1–DS20) and DS21–DS24, all answered 2026-10-15
(re-answers14):

- ~~**Q6** (DS1). The fire generator.~~ A `cRZRandom` member of the fire instance, seeded from the clock (§1).
- ~~**Q7** (DS2). The draw methods.~~ §13 lists every draw with its method.
- ~~**Q8** (DS3). The fire ID in `IgniteCell`.~~ Drawn once per call that passes the probability, even when nothing
  ignites (§5.5).
- ~~**Q9** (DS4). Timer seconds.~~ The riot gate truncates (integer `GetElapsedTime`); the effectiveness uses
  fractional seconds (§2.5, §6.5).
- ~~**Q10** (DS5). The `GoToNextDisaster` cursor.~~ Set to the newest entry on every registration, then walks
  backwards (§2.7).
- ~~**Q11** (DS6). Effectiveness.~~ Divided by the protection simulator's dispatch-unit count, not the timer
  count; 0 when that count is 0 (§2.5).
- ~~**Q12** (DS7). The dispatch key.~~ data2, the dispatch ID; any of the four messages creates the entry (§2.5).
- ~~**Q13** (DS8). Instance `Shutdown`.~~ Per instance, §3.1.
- ~~**Q14** (DS9). D3–D5 lengths.~~ §5.1 "Array lengths".
- ~~**Q15** (DS10). Which fires count.~~ §5.7.
- ~~**Q16** (DS11). Riot defaults.~~ §6.1.
- ~~**Q17** (DS12). `0xEA400622`.~~ Unused (§6.1).
- ~~**Q18** (DS13). The spiral.~~ plumbing.md §7.2's clockwise spiral, rings 0–7 (§6.4).
- ~~**Q19** (DS14). The riot after phase 3.~~ It stays in the manager (§6.7).
- ~~**Q20** (DS15). Short-form defaults.~~ §7, §8, §10, §11.
- ~~**Q21** (DS16). The fault line.~~ §7.
- ~~**Q22** (DS17). Rumble to deformation.~~ Same call; the rumble never ticks (§7).
- ~~**Q23** (DS18). The sinkhole brush.~~ §8.
- ~~**Q24** (DS19). The pipe-burst occupant.~~ There is none; leaks are records (§8).
- ~~**Q25** (DS20). The volcano's states.~~ §10.
- ~~DS21. Toast processing.~~ Only inside `Update` (§5.7).
- ~~DS22. The special-building record.~~ {BDS name, cost} (§2.5).
- ~~DS23. The network default cost.~~ One constant, 10 (§2.5).
- ~~DS24. The riot's lock release order.~~ Ascending occupant address (a `std::set`); not observable, so any order
  conforms.

The glue13 lane's questions (QUESTIONS.md DS25–DS40; the lane's own numbers Q26–Q42 are used here), answered
2026-10-16 (re-answers23):

- ~~**Q37** (DS35). Do occupant locations follow the occupant?~~ **DIFFERS** from the earlier text "whenever it is
  read": entries are written only when resolved, which only slot 12 and `GoToNextDisaster` do; slot 17 reads the
  stored values (§2.7). The glue's observation (0, 0, or the cell at a focused registration) MATCHES.
- ~~**Q38** (DS36). When does the riot lock network occupants?~~ MATCHES §6.5 step 6, now with the exact rule and
  the three callers (§6.3). The core, which locks only at `AddRiotLeader`, **DIFFERS**.
- ~~**Q39** (DS37). The riot's second offset.~~ MATCHES §6.5: `next` starts at 0, the first step runs at elapsed 0,
  stage 1 at step 30 (29 s). The core's choice of a first step 1 s after `Init` **DIFFERS**.
- ~~**Q40** (DS38). Which `Ignite`s are refused?~~ Exactly the five exits of §5.4 ("The refusals, exactly"). The
  log's refused targets could not be attributed to one exit statically; the shadow should log the original-side
  inputs (pause and emergency pause, type, flammability, the clamped rectangle and its average coverage, and
  `0x69DBEBE1`) at each refusal. For spread targets the likeliest exit is step 5: a spread cell often lies on a
  target that already burns (any fire occupant on it, including one in the same pass). Partly open.
- ~~**Q41** (DS39). Slot 18.~~ The slot is right; the return type is a pointer to the statistics record (§2.6).
  **DIFFERS** from the earlier §2.6. The logged 332771428 is `0x13D5B064`, the layer identity `0x13D5B014` + `0x50`.
- ~~**Q42** (DS40). Is a fire lit during the pass stepped?~~ Sometimes: a `std::list` walked with the next node
  taken before each step (§5.8). The mirror's "never" **DIFFERS**. Also: a null target triggers `KillAllFires`
  only after the earlier fires were stepped; the extinguish-time test is strict (§5.10 step 2); `KillFire`
  re-registers or erases the ID only when its target's entry was removed (§5.10).

## 18. Conformance tests (must exist in `engine/conformance` if the layer is replaced)

1. **Coverage threshold.** D9 is shipped. Average coverage 50 → D9 = 0.5 → round half to even = 0 → `Ignite`
   succeeds. Coverage 51 → D9 = 0.5 + (1/25) × 1 = 0.54 → 1 → `Ignite` fails. Coverage 0 → 0 → succeeds.
2. **Stage lookup.** With D3 shipped, the cumulative thresholds are 30, 55, 75, 95, 120. Counter 0 → stage 0; 29
   → 0; 30 → 1; 54 → 1; 55 → 2; 119 → 4; 120 → 5; 500 → 5.
3. **Unattended burn-down.** Coverage 0, `maxStage` 2, ordinary ignition (counter 0), ignition at timer 10000 ms:
   - the first update runs at the first tick with timer ≥ 10000 ms; update *n* at the first tick with timer
     ≥ 10000 + 1000 (*n* − 1) ms;
   - after 29 updates, counter 29, stage 0;
   - after 30, stage 1;
   - after 55, stage 2 → `BurnDownFire` (the 55th update, at timer ≥ 64000 ms), one `0x49E3DB1D` with data2 0.

   A forced ignition (counter 30) burns down at the 25th update.
4. **Hosed fire.** Coverage 0, one hose: `k = 4`, so each update draws `RandomUint32Uniform(5)` ∈ {0..4} and
   subtracts it. From counter 3, a draw of 4 gives −1 → counter 0, `ExtinguishFire`. The fire dies after its
   random extinguish delay (< 4000 ms).
5. **Episode pause.** Light one fire in an idle city:
   1. Expect `EmergencyPause`, `DisableSave` and `0xCA1B1E64` with data1 1 and data4 `0x621CDA33`.
   2. Light a second fire while it burns: no second `DisasterHasBegun`.
   3. When both are gone, expect `EnableSave`, `EmergencyResume`, `0xAA1B2002` and history `0x49D15B74` + 1,
      each once.
6. **Nested enable counter.** `EnableDisasters(false)` twice, then `EnableDisasters(true)` once →
   `DisastersEnabled()` = false. Once more → true. A further `EnableDisasters(true)` keeps the counter at 0.
7. **Riot timing.** Coverage 0, one leader; step *n* runs at riot timer second *n* − 1 (the first at 0):
   - step 1: stage 0, one riot message for stage 0, no spawn;
   - step 30 (29 s): stage 1 → 1 spawned leader;
   - step 90 (89 s): stage 3 → 1 more;
   - step 180 (179 s): stage 6 → the first leader ends.

   If the coverage under the leader becomes 255 (curve 5), its next step adds −4 → stage −1 → it ends. When the
   last leader ends, the phase goes to 3 and `DisasterHasEnded` runs, but the instance stays in the manager and
   `AttemptDisasterStart` keeps returning null.
8. **Riot leader limit.** Police coverage 112 → curve 1 + 4 × 47/190 ≈ 1.989 → trunc 1 < 2 → the leader is created.
   Coverage 113 → ≈ 2.011 → refused.
9. **Toxic spill.** Amount 120, unattended:
   - after 119 updates, 1 left;
   - at 120, the occupant is removed and `DisasterHasEnded` is called.

   With one hose, 30 updates.
10. **Dispatch effectiveness.** Overall time 100 s, two fire-map timers of 30 s and 50 s, fire protection
    `GetTotalDispatchCount()` = 4 → (30 + 50) / (4 × 100) = 0.2, stored at `+0xAC`; nothing is passed to the
    simulator. With a dispatch count of 0 the result is 0. A 2.5 s timer counts 2.5, not 2.
11. **Location cursor.** Register A, B, C (in that order). `GoToNextDisaster` visits C, B, A, C, … Register D after
    the first call: the next call visits D.
12. **Toast insertion.** Toast list [P]: adding Q draws `RandomUint32Uniform(1)`, which is always
    0 and the list becomes [Q, P]. An empty list takes the first entry with no draw.
13. **Riot road search.** Centre (10, 10), the only road at (11, 10): found in ring 1 at its first cell (x + 1,
    z − 1 + 1). With roads at both (10, 11) and (11, 10), (11, 10) wins (ring 1 starts at (x + 1, z) and runs
    +z first).
14. **Volcano phases.** Start a volcano with lava enabled: right after the start call its state is 5 and
    `GetPhase` is 2 (not counted as active). After T seconds of ticks: `DisasterHasEnded` once, state 3, phase 1.
15. **Location resolution.** Ignite a building at cell (12, 7) with `focus` false while no other fire burns:
    `GetDisasterLocationByIndex` reads (0, 0) until `DisasterHasBegun` runs in the same `Ignite`; after it, slot 17
    reads (12, 7). Register a riot leader's automaton, move the automaton three cells: slot 17 still reads the old
    cell; `GetMostRecentDisasterLocation` returns the new cell and slot 17 then reads it too.
16. **Statistics pointer.** `GetDisasterCost()` = interface pointer + `0x48`. After one building with demolition
    cost 500 is removed during a disaster, `*(uint32*)p = 1` and `*(int64*)(p + 8) = 500`.
17. **Same-pass stepping.** Fires [A, B]; A's step spreads to C (list [A, B, C]): C is stepped in this pass, after B.
    Fires [A]; A's step spreads to C: C is not stepped until the next pass.
18. **KillFire ID handling.** Fires A and B share an ID, only A's target registered. Kill B: the unregister call
    removes nothing, so the ID and A's entry stay. Kill A instead: A's entry is removed and B's target is
    registered with `focus` false.
19. **Riot locks while walking.** A leader moves from road piece P to road piece Q between two steps: the step after
    the move locks every network occupant within ±3 cells of Q's cell. Moving back onto P later locks around P
    again (no new pieces if nothing changed).
