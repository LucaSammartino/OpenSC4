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
  these sources. To avoid the pauses, a test city needs fire
  coverage above 50 everywhere flammable, police coverage, and water funding of at least 50 %.
- **Disasters run on the wall clock, not on game days** *[compat]*. The fire advances once per **1000 ms of real
  time** through its own timer and the animation tick. The riot advances once per real second. The earthquake,
  pipe burst, volcano and toxic spill use the animation tick's real microseconds. A turbo clock does not speed
  them up. While the simulator is in the emergency pause, game days do not pass at all. A long fire therefore
  costs real seconds, not game days. In the harness, an unattended fire with no fire station burns down its
  building after about 55 real seconds (§5.12, test 3).
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
generator (§13).

## 2. The disaster layer

### 2.1 State

| Field | Meaning |
|---|---|
| managers | map from type to manager (each AddRef'd) |
| initialised | bool |
| disable counter `+0x38` | disasters are enabled while it is 0 (§2.3) |
| ongoing count `+0x40` | number of disasters that have begun and not ended |
| damage assessment | building count `+0x50`; total cost `+0x58` (int64); four (count, int64 cost) pairs by building kind at `+0x60/+0x68`, `+0x70/+0x78`, `+0x80/+0x88`, `+0x90/+0x98`; a vector of special-building records |
| dispatch timers | a fire map and a police map, keyed by dispatch source; an overall timer `+0x28` (cRZTimer, units 5 = seconds) |
| dispatch effectiveness | fire `+0xAC`, police `+0xB0` (float) |
| locations | vector of 20-byte entries {instance, x, z, zoom level, occupant} at `+0xC4` |
| next-location cursor | index for `GoToNextDisaster` |

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
fires, arson, riots, pipe bursts or RiskyMoney earthquakes *[inferred: no read of slot 6 on those paths]*. Who
reads the counter is open (§17 Q5; presumably the disaster menu and the game-options switch).

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
   3. If the flags have any bit of `0x1800400`, push a special-building record.
3. If the occupant is a network piece (`0xC772BF98`) without the property, use a default cost per network type
   from the table at `0x00B0A1C0`.
4. `totalCost += cost`.

Every removed occupant counts, whatever removed it, while a disaster is ongoing. This includes the player's own
bulldozing during the pause.

**Dispatch messages** (`DoDispatchMessage`, `0x005784B0`). The dispatch type selects a map:

- types 1 and 5 → the fire map;
- types 0 and 6 → the police map.

The key is the dispatch source. `0xE94671EC` starts the source's timer. `0x894671F6`, `0xE94671EB` and
`0xE94671ED` stop it.

**End** (`EndDamageAssessment`, `0x00577CA0`):

1. Unsubscribe.
2. Run `EndDispatchTimers` (`0x00577AF0`): for each map, `effectiveness := Σ(timer seconds) / (number of timers ×
   overall elapsed seconds)`. The fire value is passed to the fire protection simulator (`0x00B43D78`) slot 25. The
   police value goes to the police simulator (`0x00B43D98`) slot 26. Both are kept at `+0xAC` / `+0xB0`.

The advisors read these at `0xAA1B2002` (advisors.md §7.6).

### 2.6 Cost (`GetDisasterCost` slot 18, `ResetDisasterCost`)

`GetDisasterCost()` returns `totalCost`. `ResetDisasterCost` zeroes the counts, the costs and the special-building
records.

### 2.7 Locations (slots 12–17, `GoToNextDisaster` slot 11)

- **`RegisterDisasterLocation(instance, x, z, occupant, zoomLevel, focus)`** (slot 13, pop 24):
  1. Append the entry and AddRef the occupant.
  2. If `focus` is true and the "follow disasters" preference byte (`0x00B43CD8 + 0xF08`) is set, call
     `GoToNextDisaster`.
- **Occupant-based entries**: an entry with an occupant takes its x and z from the occupant whenever it is read
  (`ResolveDisasterLocation`). This happens only while the occupant is still in the occupant manager. Otherwise
  the stored x and z stay.
- **`UnregisterDisasterLocations(instance)`** (slot 14) removes all of the instance's entries.
  **`UnregisterDisasterLocationByOccupant(instance, occupant)`** (slot 15) removes the matching entries. Both
  release the occupants.
- **`GetMostRecentDisasterLocation(instance, &x, &z)`** (slot 12, pop 12) searches from the newest entry. It
  takes the newest entry overall when `instance` is null, else the newest entry of that instance. It returns false
  when there is none.
- **`GetDisasterLocationCount`** (slot 16) and **`GetDisasterLocationByIndex`** (slot 17, pop 16) give plain
  access.
- **`GoToNextDisaster`** (`0x00577810`) is view-side:
  1. Advance the cursor (cycling).
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

Phases 0, 1 and 3 are the only values seen *[inferred: 2 unused]*.

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
instance `0xC9B6B73D` ("Flame"). Each property has a default that applies when it is missing.

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

The update period is **1000 ms** of the instance's timer (cRZTimer units 4, milliseconds).

### 5.2 State of the fire instance

| Field | Meaning |
|---|---|
| fires | list of fire entries (each a fire occupant, §5.11) |
| fire IDs | set of fire-episode IDs; each ID has one location in the layer |
| toast list | occupants to burn down at once (§5.7) |
| phase | 0 idle, 1 active, 3 ended (§5.9) |
| timer, next update | units 4 (ms); `next` advances by 1000 |
| cause | last `SetCause` value |
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
6. If `fireID` is 0, draw one (§13). If the ID is new, register a location with the layer:
   `RegisterDisasterLocation(instance, 0, 0, occupant, zoomLevel, focus)`. Then add the ID to the set.
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

`IgniteOccupant(o)` (manager slot 4, `0x0057B930`) returns false for a null `o`. Otherwise it calls `Ignite(o,
false, 0, true)` and returns **true whatever `Ignite` returned** *[quirk]*.

### 5.5 `IgniteCell(x, z, useProbability, teleport, fireID)` (slot 7, `0x0057B170`)

1. `p := DA(flammability grid value at (x, z))`. The grid is the adjusted flammability grid (flammability.md).
2. If `useProbability` and `p < uniform[0, 100)`: return. One draw is made per call (§13).
3. For each occupant in the cell that passes the type filter: `Ignite(occupant, teleport, fireID, not teleport)`.

The fire manager's `AttemptDisasterStart(x, z)` is `IgniteCell(x, z, false, false, 0)` and returns null.

### 5.6 `IgniteCellRadius(x, z, r0, r1)` (slot 10, `0x0057AF10`)

1. Draw one fire ID for the whole call.
2. Visit every occupant in the clamped box of radius `r1` cells around (x, z) that passes the type filter. Let
   `d` be the 2D distance from its position to the cell centre, and `w` the cell width (city slot 91, 16 m).
   - `d < r0 × w`: add it to the toast list (it burns down at once, if flammable).
   - `r0 × w ≤ d < r1 × w`: `Ignite(occupant, false, id, false)`.
   - Otherwise: nothing.

### 5.7 Other entry points

- **`ExtinguishCell(x, z, amount)`** (slot 8). For every active fire in the cell: if `counter ≤ amount`, set it
  to 0 and run `ExtinguishFire` (below); else `counter −= amount`. Fire crews and trucks call it (dispatch.md).
- **`QuickBurnCell(x, z)`** (slot 9) adds the cell's filtered occupants to the toast list.
- **`ProcessToastList`** handles at most **5** entries per `Update`. Each still-flammable occupant is burned down
  (`ToastOccupant`, §5.10), and the rest wait.
- **`ExtinguishFire(fire)`**. If the fire has no extinguish time yet, set it to `elapsed + Random(4000)` ms. The
  fire then lasts until that time (§5.8 step 2) and dies with `KillFire`.
- **`Extinguish(o)`** (slot 11) and **`BurnDown(o)`** (slot 12) look up the fire whose target is `o` and run
  `ExtinguishFire` or `BurnDownFire` on it.
- **`GetFireCount()`** (slot 16) and **`GetFireCountInCellRect(rect)`** (slot 15) count the list.
- **`SetCause(c)`** (slot 13) and **`GetCause()`** (slot 14).

### 5.8 `Update` (slot 4, `0x0057BF10`)

It runs on the animation tick `0x86AD10EE` while registered (§5.9). The instance also handles `0x66956813` (the
hour), but no hour agent is registered for it *[inferred]*. Each run:

1. `ProcessToastList` (§5.7).
2. `UpdatePhase` (§5.9).
3. If there are fires and `timer ≥ next`: run `UpdateAllFires`, then `next += 1000`. At most one step runs
   per tick; a backlog (a stalled frame) is worked off one step per later tick *[compat]*.
4. If any ignitions were counted, post `0x49D15B73` with data1 = buildings and data2 = flora, then zero both.

**`UpdateAllFires`**:

1. Read the tutorial's no-burn-down flag (tutorial system slot 8) into `+0x54`.
2. If any fire has a null target, kill all fires (`KillAllFires`) and stop.
3. Otherwise run `UpdateFire` on each.

### 5.9 Phase (`UpdatePhase`, `0x0057A0D0`)

- **Fires exist and phase ≠ 1** (and the simulator exists):
  1. `layer.DisasterHasBegun(instance)`.
  2. Register with the animation tick manager (slot 5, priority `0xC0`).
  3. `next := timer elapsed`.
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
2. If the fire has an extinguish time: if that time has passed, `KillFire`. Stop either way.
3. **Resistance**:
   1. `k := round(D9(average coverage over t's cell rectangle))`, as in §5.4 step 4, rounding half to even.
   2. If the fire is being hosed (`hosed > 0`, the number of hoses on it): `k += 4 × hosed`.
4. **Change**: `delta := +1` if `k ≤ 0`; else `delta := −RandomUint32Uniform(k + 1)`, a value in `[0, k]`.
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
   1. If `uniform[0, 1) < 1 / D4[s]`, choose a cell (`ChooseSpreadCell`): a Gaussian point around the fire's
      bounding box, widened by `D5[s] + wind`. `wind` is the weather simulator's wind (slot 22) × D6.
   2. If the cell is inside the city: `IgniteCell(x, z, true, false, fire's ID)` (§5.5). The new fires join the
      same episode and location.
10. Done.

`maxStage` is the target's property `0x49BEDA31` (default **2**). Stages above `maxStage` are never shown.

**`KillFire(fire)`** (`0x0057B640`):

1. Post `0x49E3DB1D` with data2 = 0 (fire ended).
2. `layer.UnregisterDisasterLocationByOccupant(instance, target)`.
3. If another fire has the same ID, register that fire's target as the ID's location. Otherwise erase the ID.
4. Remove the fire occupant from the occupant manager and the list.

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
4. For a building, attach the automata controller "fire_occupant" (city slot 47, controller manager slot 9).
   That brings the fleeing people.

Removing the fire occupant clears `0x69DBEBE1`.

### 5.12 Timing in the harness

With no fire station (coverage 0, `k = 0`) and the default `maxStage` 2, the counter rises by 1 per second. The
stage reaches 1 at 30 and 2 at 55, so the building burns down **55 updates** (≈ 55 real seconds) after an
ordinary ignition. A forced ignition starts at 30, so it takes 25 seconds. The fire spreads each second with
probability 1/4 at stage 0 and 1/3 at stage 1.

## 6. Riot

### 6.1 Tuning

The riot reads exemplar `0x6534284A` / `0x29B6C670` / `0x4A38206D` ("Riot"). The defaults apply when a property
is missing.

| Property | Meaning | Default | Shipped |
|---|---|---|---|
| `0xCADB6B3E` | maximum riots at once | — | 3 |
| `0x2ADB6B6D` | maximum leaders per riot | — | 10 |
| `0xEA400620` | number of stages | 6 | 6 |
| `0xEA400621` | seconds per stage | 30 | 30 |
| `0xEA400622` | (absent) | 3 | — |
| `0xEA400623` | leaders spawned on reaching stage *s* | — | 0, 1, 0, 1, 0, 0 |
| `0xEA400624` | police curve: coverage → suppression | — | (0, 0) (64, 0) (65, 1) (255, 5) |
| `0xCA821B8C` | zoom level | — | 3 |

### 6.2 Start (`AttemptDisasterStart(x, z)`, `0x0057EA10`)

The aura simulator passes cell coordinates (aura.md). If `GetActiveInstanceCount() < 3`:

1. Create an instance and add it to the manager.
2. `Init` it (§6.3).
3. `AddRiotLeader(x, z, false)` (§6.4).

The new instance is returned. Otherwise the result is null.

### 6.3 Instance `Init`

1. Register with the animation tick manager.
2. Start a timer with units 5 (seconds).
3. Mark the network occupants within ±3 cells of the start as riot-blocked (network occupant slot 48, flag 2).

### 6.4 `AddRiotLeader(x, z, spawned)`

1. Find the nearest road tile: a spiral of radius up to 7 over the traffic network for network types 0, 3 and 6
   (road, street, avenue *[inferred]*). Fail if there is none.
2. Read the police coverage at the cell (police simulator slot 5 grid, `GetCellValue`). Require
   `trunc(curve(coverage)) < 2` and `leaders < 10`. With the shipped curve that is coverage ≤ 112.
3. Create a pedestrian automaton of kind `0x4107` (city slot 44, pedestrian manager slot 15):
   1. Place it at the cell's position plus a random offset of up to 16 m, on the terrain height.
   2. Set it to riot-leader mode (automaton slot 51 with 8) and start it (slot 9 with 1).
4. `layer.RegisterDisasterLocation(instance, 0, 0, automaton, zoomLevel, not spawned)`.
5. Mark the network around the cell (§6.3) and run `UpdatePhase` (§6.7).

### 6.5 Update (once per real second)

For each leader:

1. `counter += trunc(1 − curve(police coverage at its cell))`.
2. `stage := counter / 30` (integer division), or −1 when the counter is negative.
3. When the stage changes, update the riot effects ("Riot%d" automaton effects).
4. When the stage reaches a new maximum below 6:
   1. Spawn `0xEA400623[stage]` leaders at its cell (`AddRiotLeader(…, true)`).
   2. Send the formatted riot message to the automata controller manager (city slot 47, slot 5).
5. The leader **ends** when its stage is ≥ 6 or −1, or when its automaton is gone:
   1. Set the automaton back (slot 51 with 1) and detach its controller (slot 14).
   2. `layer.UnregisterDisasterLocationByOccupant`.
   3. Erase the leader.
6. Otherwise it follows the road under it (`DoRiotLeaderTick`: random turns on the traffic network).

Unopposed (coverage 0), a leader gains 1 per second. It reaches stage 1 at 30 s, where it spawns 1 leader, and
stage 3 at 90 s, where it spawns another. It ends at 180 s. With coverage 255 (curve 5) it loses 4 per second and
ends at once (stage −1 after the first second).

### 6.6 Followers set fires

Pedestrians following a leader enter an "ignite" state with a target occupant. In that state the pedestrian
calls the fire manager's `IgniteOccupant(target)` and `SetCause(2)` (arson), then leaves the state
(`cSC4Pedestrian::DoIgniteTick`). Riots therefore start fire episodes of their own. Each one goes through §5,
including its own `DisasterHasBegun`.

### 6.7 Phase

- **0 → 1** when the first leader exists: `layer.DisasterHasBegun`, and history `0xAA8C87C7` + 1.
- **1 → 3** when no leaders remain: unregister the instance's locations, then `layer.DisasterHasEnded`.

## 7. Earthquake (short form)

*Short form*: the terrain deformation is described by its tuning and its order of operations, not by its
geometry. A port may draw a different fault line, provided it uses the same brush tuning.

- **Start** (`AttemptDisasterStart(x, z)`, slot 6). Only when no earthquake instance exists; otherwise return
  null (budget.md §4.3):
  1. Create the instance, add it to the manager and `Init(x, z)` it.
  2. Set its state to 2, which starts the deformation.
- **`Init`**:
  1. The first time, read the tuning (below).
  2. Clamp the epicentre to `[0, SizeX − 1] × [0, SizeZ − 1]` in world units.
  3. Compute the fault line.
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
  4. Each tick redraws the touched cells (terrain slot 69).
  5. It ends (state 3, terrain `EndLongTerrainOperation`) when the line is done, the strength drops to 0.001 or
     below, or the width falls under 0.002.

  The damage to buildings is the terrain's response to the brush *[inferred: the earthquake itself demolishes
  nothing]*.
- **Tuning** (`0x6534284A` / `0x29B6C670` / instance **2**, "Earthquake"):
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
  1. Each leak gets a burst time, uniform in `[0x4A08C600, 0x4A08C601)` seconds = [30, 60) s, kept in µs.
  2. It shows "burst_pipe_aboveground" or "burst_pipe_underground" when the view level is 1 or 3.
  3. It posts `0xEA1F0DCD` (x, z) to tell plumbing the cell lost water (plumbing.md).
- **Tick**:
  1. Each leak adds the tick's µs. When a leak with an effect reaches its burst time, `BurstPipe`:
     1. Play "pipe_explodes".
     2. Queue the neighbouring connected pipe cells (inside the city) as a wait entry.
     3. `demolition.DemolishRegion(1, pipe, 0, 0x302, …)`: cause 2, explode (demolition.md §4.9).
     4. Post `0xEA1F0DCD`.
     5. Dig a sinkhole: terrain brush `0x14` at the cell's position.
  2. Each wait entry ages by one tick and starts leaks at its cells after 4 ticks, so a burst spreads along the
     pipe.
  3. When no leaks and no waiting entries remain: `layer.UnregisterDisasterLocations`, `layer.DisasterHasEnded`,
     then shut down.
- *[quirk]* A leak without an effect, because the view was at another level when the leak started, never bursts
  until a view-level change recreates its effect. In a harness with no 3D view, a pipe-burst instance therefore
  never ends and the emergency pause never lifts *[inferred: the view-level message `0xC6CDBF1A` is the only
  path that recreates the effect]*. This needs a runtime check (§17 Q2).
- Tuning: instance **3**, "PipeBurst": `0x4A08C600` = 30, `0x4A08C601` = 60, `0xCA821B8C` = 3.

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

- **Start** (`AttemptDisasterStart(x, z)`). Only when no volcano exists:
  1. Create the instance and add it.
  2. Set the location and `Init`.
  3. Go to state 3.
- **`Init`**:
  1. Clamp the location to the city.
  2. Subscribe to `0xCA493C1D`.
  3. Read the tuning.
  4. Register with the animation tick manager (`0xC0`).
  5. Set state 1.
  6. Register the location and call `layer.DisasterHasBegun`.
- **States**:
  - 3: start the terrain deformation (manager-side) and prepare the lava.
  - 5: start the lava flow at the vent (`cSTEVolcanoLava`), radius `0x0A4BC752` = 20.
  - 7: ending.
  - 8: end the lava and the deformation.
- **Lava**:
  1. Each tick sets the production rate to `curve 0xEA4BC6E4(t / T) × 0xAA4BC72A / 100000` and the
     solidification rate to `curve 0x4A4BC714(t / T) / 100000`, with `T = 0x0A4BC742` = 50 s.
  2. After T, the lava ends and the state goes to 3, or to 8 if the instance was asked to end.
  3. The lava ignites fires where it flows (§4).

  The lava only flows when `0x6A4BC8ED` is true (shipped true).
- Tuning: instance **1**, "Volcano":
  - production (0, 0) (0.075, 0) (0.25, 0.25) (0.8, 0.25) (1, 0);
  - solidification (0, 0) (0.4, 0) (0.9, 0.01) (1, 0.15);
  - scale 6.

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

- **Fire.** The fire instance's generator draws, per call:
  - a new fire ID in `Ignite`, when none is given;
  - one value in `IgniteCellRadius`;
  - one `uniform[0, 100)` per `IgniteCell` with `useProbability`;
  - per `UpdateFire`, one `RandomUint32Uniform(k + 1)` when `k > 0` and one `uniform[0, 1)` for the spread (always
    drawn), plus two Gaussian draws when it spreads;
  - one `Random(4000)` per `ExtinguishFire`.

  The order follows the list order of the fires.
- **Riot.** The riot's generator draws the leader offset (two values) per leader.
- **Pipe burst.** The pipe burst's generator draws the burst time per leak. Its argument-less start uses a
  throwaway generator.

## 14. Persistence

Nothing in this spec is saved. The riot instance class has `Write`, `Read` and `GetGZCLSID` slots (vtable
`0x00A9E800`). The layer never writes it, because the layer is not in the city's record *[inferred]*. A
replacement must not add a record.

## 15. Interface slot tables *[compat]*

`cISC4DisasterLayer` (vtable `0x00A9E390`), `cISC4DisasterManager` and `cISC4DisasterManagerFire` (`0x00A9E4DC`)
are in `docs/abi/interfaces.tsv`. The proposed rows for the fire instance's interfaces and the toxic manager are in
`build-runs/re-specs12-rows.tsv` of the re-specs12 lane.

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

## 17. Open questions

- **Q1.** *(resolved)* D3–DA are `0xA9B6ECD3`–`0xA9B6ECDA` (§5.1). data-formats.md does not list them yet.
- **Q2.** Does a pipe-burst instance end in a session with no 3D view (§8 quirk)? This needs a runtime run with
  water funding below 50 %.
- **Q3.** Is phase 2 ever used by an instance (§3.2)?
- **Q4.** What does network flag 2 (riot-blocked) change in traffic? That belongs to the network manager spec
  (INVENTORY rank 3).
- **Q5.** Which callers read `DisastersEnabled` (slot 6) or call `EnableDisasters` (slot 5)? No simulation-side
  caller was found (§2.3).

## 18. Conformance tests (must exist in `engine/conformance` if the layer is replaced)

1. **Coverage threshold.** D9 is shipped. Average coverage 50 → D9 = 0.5 → round half to even = 0 → `Ignite`
   succeeds. Coverage 51 → D9 = 0.5 + (1/25) × 1 = 0.54 → 1 → `Ignite` fails. Coverage 0 → 0 → succeeds.
2. **Stage lookup.** With D3 shipped, the cumulative thresholds are 30, 55, 75, 95, 120. Counter 0 → stage 0; 29
   → 0; 30 → 1; 54 → 1; 55 → 2; 119 → 4; 120 → 5; 500 → 5.
3. **Unattended burn-down.** Coverage 0, `maxStage` 2, ordinary ignition (counter 0):
   - after 29 updates, counter 29, stage 0;
   - after 30, stage 1;
   - after 55, stage 2 → `BurnDownFire` (the 55th update), one `0x49E3DB1D` with data2 0.

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
7. **Riot timing.** Coverage 0, one leader:
   - stage 1 at 30 s → 1 spawned leader;
   - stage 3 at 90 s → 1 more;
   - the first leader ends at 180 s.

   Coverage 255: the counter is −4 after 1 s → stage −1 → the leader ends.
8. **Riot leader limit.** Police coverage 112 → curve 1 + 4 × 47/190 ≈ 1.989 → trunc 1 < 2 → the leader is created.
   Coverage 113 → ≈ 2.011 → refused.
9. **Toxic spill.** Amount 120, unattended:
   - after 119 updates, 1 left;
   - at 120, the occupant is removed and `DisasterHasEnded` is called.

   With one hose, 30 updates.
10. **Dispatch effectiveness.** Overall time 100 s, two fire timers of 30 s and 50 s → (30 + 50) / (2 × 100) =
    0.4, passed to fire protection slot 25.
