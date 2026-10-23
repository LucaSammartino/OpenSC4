# Spec: road vehicles, trains and tanks (the concrete traffic automata)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[summary]* is a correct outline whose details are not given.

Related specs: [automata.md](automata.md) (the automaton base, records §6.1–§6.4, the state types and their
per-tick behaviour §6.5, the slot names §11 — this spec uses those slot numbers), [dispatch.md](dispatch.md) §4.3
(the emergency dispatch states), [drive-tool.md](drive-tool.md) (U-Drive-It),
[traffic-simulator.md](traffic-simulator.md), [disasters.md](disasters.md) (fires, riots, toxic spills),
[audio.md](audio.md) (AEMS events).

## 0. Summary

The automata that travel on networks share `cSC4TrafficAutomaton` (routes, leaders, collisions) and, for road and
rail vehicles, `cSC4Vehicle` (paths, damage, motion blur, player driving). Subclasses add trailers (articulated
vehicles), trains (a locomotive with cars), the tank (a drivable reward vehicle that shoots), the emergency
vehicles (dispatch states and re-pathing; the fire truck with its fire crew and the patrol car with riot police),
the bus (station stops) and the getaway van. Their records are automata.md §6.4; their state ticks are automata.md
§6.5. This spec gives the rest.

## 1. Classes *[compat]*

| Class | Base | GZCOM / record | Notes |
|---|---|---|---|
| `cSC4TrafficAutomaton` | `cSC4Automaton` | record automata.md §6.3 | §2 |
| `cSC4Vehicle` | traffic automaton | `0x896E714A` | §3 |
| `cSC4TrailerVehicle` | vehicle | `0x4A5B69ED` | §4 |
| `cSC4TrainCar` | vehicle | version 2 + Vehicle record | §5, §12.1 |
| `cSC4Train` | train car | version 2 + TrainCar record, car refs (`0x4A416254`), independent network ref (`0xA821EF94`) | §5, §12.1 |
| `cSC4Tank` | vehicle | vehicle record + two dwords | §6 |
| `cSC4EmergencyVehicle` | vehicle | emergency record | §7 |
| `cSC4FireTruck`, `cSC4PatrolCar` | emergency vehicle | + a ref (`0xA9B40F05`) | §7.2, §7.3 |
| `cSC4RiotPolice` | pedestrian | pedestrian record + a ref | §7.3 |
| `cSC4Bus`, `cSC4GetawayVan` | vehicle | vehicle record (+ a dword for the van) | §8 |

Each class has the usual `GetGZCLSID`, `Read`/`Write` (automata.md §6.4) and adjustor thunks for its `Init` and
`Shutdown` at `+4`.

## 2. The traffic automaton *[compat]*

- **Animation tick** (`DoAnimationTick`, `0x0054E050`): a pending speed tween advances (`+0x1AC` toward `+0x1A8`,
  time scaled by a constant; when done the speed `+0x1A0` := the target `+0x1A4`); then the tween and effect
  updates (slots 83, 103); when the automaton moves (slot 86) and its state does not handle the tick (slot 109):
  without a finished route (slot 87), a path move (slot 129) when it has a network tile, a tile position and a
  route, else a straight-line move (slot 130); at the end of a route that is not held (slots 85, 165) it removes
  itself (slot 13); then slot 105 and slot 102 with (0, 0).
- **Routes.** `BeginRoute` (`0x00554AB0`) clears the look-ahead deque, pops the first route entry and finds the
  network occupant at the first cell (traffic network) that carries the route's direction, places the automaton on
  it (slot 132) and, when the route has one cell, finishes it (flag 4). The look-ahead deque is refilled to the
  count slot 175 asks (`0x00554870`), each entry through slot 167/168; an entry that cannot be made stops the fill.
  `SetRouteInfo` copies a route deque and returns to the default speed (slot 160).
- **Outside connections.** `FindBestNeighborConnection` (`0x00554EA0`) chooses, among the region's neighbour
  connections of the network types 3, 0x0C, 8, 2 and 0x0A, the best one for the trip *[summary]*.
- **Leaders.** `FindLeader` (`0x00551C20`): with fewer than two route entries, the nearest automaton ahead in the
  current tile, or else in the next tile in the direction of travel; otherwise the first tile of the route that
  has one. `FindLeaderInTile` keeps the automata of the tile that are not removed (slot 85 of the other), that the
  automaton may follow (slot 73) and that drive straight (flag `0x2000`) or travel the same lane bits, and returns
  the nearest one (3D distance, the vertical part ignored).
- **Collision** (`UpdateCollision`, `0x0054E1A0`): with flag 8 the leader is refreshed when it is gone or flag
  `0x10` is set. With flag `0x800`, or flag `0x200000` while the occupant flag 4 is clear, the automaton is not
  blocked (flag `0x10` cleared) and returns to its default speed when it has no leader. In behaviour `0x0D` nothing
  is done. Otherwise the traffic network is asked whether the way ahead is taken (traffic network slot 33 with the
  tile and the collision position); taken: flag `0x10` on, and an automaton with flag `0x20` that met result 1 enters
  state `0x0D` (wait) for a tuning time; then it slows behind the obstacle (slot 159). Not taken: flag `0x10` off
  and the default speed (slot 160) — unless it was blocked and still too close (`0x00A9BCE8`).
- `HandleExitState` (`0x005535D0`) stops the traffic effects (`cSC4TrafficEffectManager::StopEffect`) and deletes
  the dynamic particle system of the state; `HandleEnterState` starts them *[summary]*.

## 3. The vehicle *[compat]*

Plummet entry constants, splash duration and the loaded PullOver quirk are specified in §12.2;
that clarification resolves the earlier conflicting descent descriptions in automata.md §6.5.

- `Init` (`0x0055B9B0`) sets its flags and calls the traffic automaton's `Init`; `DoStateTick` adds states 14 and 21
  (automata.md §6.2); `DoPathTick`, `DoPlummetTick`, `DoPullOverTick` are automata.md §6.5.
- **Collisions.** `HandleCollision` (`0x0055C8C0`) posts `0x49E64802`, picks a random direction, sets flag
  `0x10000000` and enters a 5000 ms state. `ResolveCollision` and `UpdateAfterDamage` (`0x0055F3F0`) report an
  accident to the traffic automata manager (`CreateAccident`), set flag `0x08000000`, raise an event (slot 102) and
  replace the model by its damaged form *[summary]*.
- `UpdateMotionBlur` (`0x0055DD50`) loads the motion-blur model (`GetAnimatedModelFromResourceKey`) and fades it in
  with the speed *[summary]*.
- **Player driving** (U-Drive-It, drive-tool.md): `TranslatePlayerDriveCommands` turns the commands into the
  steering and throttle tweens; `PlayerDriveUpdateSpeed` applies them; `PlayerDriveGetNetwork` finds the network the
  vehicle is on (network mask `0x40341004`); `DoPlayerDriveTick` (4.3 KB) moves it along the network, plays the AEMS
  engine and horn events and collides; `DoPlayerDriveOffRoadTick` moves it off the network;
  `ChooseNewPlayerDriveDestination` picks a target about four cells away *[summary]*.

## 4. Trailers

A trailer vehicle drags its cars along the path of the head (`FindPointOnPathSegment`: the point on a path segment
at the car spacing, by segment intersection); its record lists the cars and the path cells (automata.md §6.4,
AM10) *[summary]*.

## 5. Trains

The exact TrainCar and Train record contracts, including load failure and reconstruction rules, are in §12.1.

- `Init` (`0x00557A20`) makes the cars (`cSC4TrainCar::Init`, flag `0x10000000`), with random lengths and
  colours *[summary]*; `Shutdown` releases them.
- `StartTrip` (`0x005588D0`) builds the trip and posts `0x0C05E4CD`; a train that cannot start reports an accident
  *[summary]*.
- `DoAnimationTick` (2.3 KB) moves the cars behind the engine and plays random AEMS sounds; `HandleCollision`
  enters a 5000 ms state with a random direction.
- **Player driving**: `DoPlayerDriveTick` and `PlayerDriveCheckForDerail`: going too fast through a curve (a random
  draw against the speed) **derails** the train (flag `0x08000000`, an event, the derail sound) *[summary]*.

## 6. The tank

`Init` (`0x0054C880`) loads the turret model and the shot effect (`tankshot`, `fx_00_tankshot`), speed 0.2;
`HandleEnterState` sets its driving parameters (0.4, 2π, 30, 100, effect `0xAC0ECE77`, 2000);
`TranslatePlayerDriveCommands` (1.9 KB) aims the turret and fires: a shot is an effect at the aimed point that
damages what it hits *[summary]*; `DoPlayerDriveTick` plays the AEMS events. Its record adds two dwords.

## 7. Emergency vehicles *[compat]*

### 7.1 The base

- **Dispatch states** are dispatch.md §4.3. `SetDispatchState` (`0x0052FF60`) also: state 1 sends the AEMS event
  `0x6A451908` (siren off) and the **path-blocked message**; state 3 enters state 100, sets flags `0x80`, `0x10000`,
  `0x800` and `0x1000`, turns the siren on (slot 194(1)) and starts the siren sound (AEMS event `0xEA451903` with
  `0x29DE9021` for automata of group `0x4228`, `0x09DC5728` for group `0x4202`); state 6 clears those flags but
  `0x10000` and turns the siren off; states 0, 2, 4 and 5 send `0x6A451908`.
- **Path-blocked message** `0xCA8DCDB4` (posted): data1 = slot 192 (the call number *[inferred]*), data2 = the
  re-path attempts, IGZUnknown = the vehicle's occupant.
- `HandleDispatched(target, home)` (`0x005329B0`): stores the target, clears flag `0x400` (and its speed modifier),
  keeps the home station, and when the route's last cell has no road extends the route to a nearby road
  (`AppendRouteToNearbyRoad`, the nearest road cell by squared distance).
- `DoEnRouteTick` (`0x0052FD50`): moves (path or straight line); at the end of the route, arrives (slot 79) or is
  removed; while it holds a valid network occupant it keeps its leader fresh; otherwise it is removed.
- `DoRepathingTick` (`0x0052FE50`): a new path to the target (vehicle manager slot 17, mode 3); success → state 3
  and back to the previous state, attempts := 0; failure → attempts + 1, and while it is at most the tuning byte
  `0x00B21CB9`: the path-blocked message and a wait state 7 for a tuning time; past that it is removed.
- `GetDebugExtraText` appends one of " [at station]", " [route blocked]", " [done]", " [en route]",
  " [no resources]", " [handling event]", " [returning home]" and " repath attempts: %d".

### 7.2 The fire truck

- `GetFireCount` (`0x00533410`): the number of fires in the city (the fire disaster `0x621CDA33`, interface
  `0x49B43F57`) and the number of burning occupants in a box around the truck. `GetToxicSpillCount`: the same for
  toxic spills (`0x4C1E8345`).
- `DoHandlingEventTick` (`0x005335F0`): in state 2 without a crew and with fires or spills nearby → state 5; with
  none nearby, state 5 and not in behaviour `0x10` → state 2; in state 2 with no fire and no spill left at all → go
  home (slot 29).
- The **fire crew**: `CreateFireCrew` makes a `cSC4SimFireCrew` (pedestrian, with a competence) and sets it up;
  `ReleaseFireCrew`; `SetDispatchState` creates or releases it with the state; U-Drive-It commands deploy and call
  it back (`HandlePlayerDriveCommand`, `TranslatePlayerDriveCommands`); `GetStreamTargetPoint` gives the water
  target.

### 7.3 The patrol car and riot police

- The patrol car's `SetDispatchState` creates or releases **riot police** (a search radius of 20, a 2000 ms
  delay); its arrest tick is automata.md §6.5.
- `cSC4RiotPolice` (a pedestrian): `Init` chooses a random model (`0x29A5D1EC`/`0x49A593E7`) and the
  `riot_police_repulsor` effect and announces itself (`0xA9E3B6E2`); `SetupRiotPolice` (1.5 KB) places the squad
  around the patrol car and the rioters (`0xAA0F3F75`) *[summary]*; `DoAnimationTick` reports (`0x8A0F3F7B`) and
  swaps models at random.

## 8. Bus and getaway van

- The bus's `DoStopAtStationTick` (`0x00524180`) stops at a station, picks the pedestrians to board or leave at
  random from the occupants of group `0x496E5783` nearby, then continues (the base stop tick).
- The getaway van's `PostInit` (`0x005339B0`) sets its speed (5) and a random model *[summary]*.

## 9. Quirks

1. A traffic automaton that is blocked but no longer too close keeps flag `0x10` only until the next collision test.
2. The emergency base sends the siren-off event for states 0, 2, 4 and 5 even when the siren was never on.

## 10. Open questions

See [questions/vehicles.md](questions/vehicles.md). Record and loaded-state clarifications are in §12.

## 11. Conformance tests

1. An emergency vehicle that fails to re-path more than the tuning byte `0x00B21CB9` times is removed, after that
   many path-blocked messages.
2. `SetDispatchState(3)` on an emergency vehicle of group `0x4202` (not `0x4228`) starts the siren sound
   `0x09DC5728`.
3. A fire truck in state 5 with no fire and no spill nearby moves to state 2; with none in the city it then goes
   home.
4. `FindLeaderInTile` returns the nearest eligible automaton of the tile, ignoring height.

## 12. Vehicle owner clarifications *[compat]*

### 12.1 Train and TrainCar records (VH3)

Train **inherits TrainCar**, which inherits Vehicle. The two names in §1 must not be interpreted as two
independent direct subclasses of Vehicle. TrainCar uses the same small version wrapper as Bus; the folded
Windows function names happen to say Bus. TrainCar's serializable vtable at `0x00A9C894` selects Write
`0x00524160`, Read `0x00557010`, and class ID getter `0x0055B240`. Its queried car interface is
`0x4A416254` (`QueryInterface`, `0x00557040`).

The wire sequence is as follows. Integers use the existing stream's little-endian scalar encoding; `ref`
means the shared GZ-serializable reference protocol, not a pointer-sized integer or an inline class record.

| Owner | Ordered wire fields |
|---|---|
| TrainCar | `u16 2`; complete Vehicle record from automata.md §6.4. No TrainCar-specific trailing fields. |
| Train | `u16 2`; complete TrainCar record (therefore another `u16 2`, followed by Vehicle); `u32` car count; that many car `ref`s; one raw 32-bit field from owner `+0x2C0`; `vec3` from `+0x2D4`; network-occupant `ref`; one raw 32-bit field from `+0x2E4`. |

The final four entries are independent of the nested traffic record's network reference. In particular,
do not collapse the train's trailing network reference into the inherited reference. The car list is in
stored order; the reference reader requests IID `0x4A416254` for each car and IID `0xA821EF94` for the
train's trailing network occupant. The two raw 32-bit fields must round-trip bit-for-bit; their scalar
encoding does not authorize converting them to a different native type.

Each wrapper accepts **exactly** version 2, not every version at most 2. Train first consumes and checks its
own two bytes; a mismatch returns false without entering TrainCar. It then consumes and checks the
TrainCar version; a mismatch returns false without entering Vehicle. Vehicle failure returns false and
prevents the train tail from being read. No rewind or transactional rollback is provided. TrainCar
returns the result of Vehicle Read/Write. Train Write emits both version words before calling Vehicle
Write, and emits its tail only if that succeeds.

On a successful nested Vehicle Read, Train reads every declared car reference. A null reference is
**skipped**, rather than occupying an empty element or failing the whole Train Read. Each non-null car is
appended to the owned list; this routine does not clear an existing list first. After those reads it
reads the raw field and vector, releases any previous trailing network occupant, reads its replacement,
and reads the last raw field. Its result remains the nested Vehicle result: there is no additional
explicit failure test for a null car or network reference. Reference/stream framework exceptions remain
framework failures. There is no extra car placement, model generation or path-point reconstruction in
this wrapper; inherited Vehicle/traffic Read behavior still applies. *[quirk]* Loading into an already
populated object therefore must not be specified as atomic replacement of the car list.

Evidence: Train Write `0x00557930`; Train Read version checks `0x0055AAE5–0x0055AB32`, car-reference and
append instructions `0x0055AB50–0x0055ABB0`, trailing fields `0x0055ABB2–0x0055AC0A`; TrainCar wrapper
version instructions `0x0052416A–0x00524174`, `0x00557021–0x00557033`.

Conformance vectors:

| Input / operation | Required observation |
|---|---|
| Read TrainCar with leading `u16 1` | False after consuming two bytes; Vehicle reader not entered. |
| Read Train with leading versions `2, 3` | False after consuming four bytes; Vehicle reader not entered. |
| Write empty Train with successful Vehicle writer | Prefix `u16 2, u16 2`, nested Vehicle record, `u32 0`, then both raw fields, vector and trailing network reference in the table's order. |
| Successful Train Read declares three car refs `[A, null, B]` | All three references consumed; owned list gains `[A, B]` in that order; result is true. |
| Same load with existing owned list `[X]` | Resulting list `[X, A, B]`; no implicit release of X by this reader. |
| Nested Vehicle Read fails | False; no train car-count or tail read. |
| Successful load has a null trailing network ref | Old trailing network reference released; replacement null; no extra Train failure solely for null. |

### 12.2 Plummet entry, splash and loaded pull-over (VH8)

The entry rule for state 14 is the base automaton's `HandleEnterState`, reached through the vehicle and
traffic overrides. It sets plummet vertical speed (`state+0x28`) to binary32 **`0xBB88888A`**, approximately
**−0.00416666735**, the orientation oscillation coefficient (`+0x2C`) to **0.5**, the saved orientation-Y
centre (`+0x30`) to the current orientation Y, and the initial wobble countdown (`+0x34`) to a uniform
double draw in **[1.0, 2.5)** multiplied by double **1000.0**, then stored as binary32 milliseconds.
Splash countdown (`+0x38`) is zero and the unsaved splash effect (`+0x3C`) is null. These are entry values,
not reconstructed values to apply after reading a saved plummet state.

The separate **−0.00083333347** value is the later downward acceleration coefficient used by
`DoPlummetTick`; it is not the entry vertical speed. The third value, binary32 **`0x3991A2B5`**
(approximately **+0.000277777814**), replaces vertical speed when the falling vehicle reaches water above
the ground. It is the splash-phase upward speed. Preserve these three distinct quantities rather than
choosing one of the previously conflicting descriptions. Evidence: entry stores
`0x00511517–0x00511539`; acceleration initialization in `0x0055E1F0`; splash-speed store in that same
tick, before effect creation.

On that water transition the effect named `car_splash` is placed at the contact point and started. There
is **one uniform double draw in [0,1)** from the shared automata random generator, narrowed to binary32
seconds, supplied to the automaton's fade-to-zero call. The same narrowed seconds value multiplied by
binary32 **1000.0** becomes the state's splash countdown in milliseconds. There is no added minimum or
separate random-duration draw. The effect itself is not serialized. Ground contact instead clears the
countdown and removes the occupant. The existing state tick's strict expiration boundary remains in
force; this clarification does not change it. Evidence: RNG call and narrowing `0x0055E4E0–0x0055E4EA`,
fade `0x0055E4EE–0x0055E4F9`, milliseconds conversion `0x0055E4FF–0x0055E50A`.

PullOver's unsaved direction vectors are **not rebuilt by Read** when its saved elapsed counter is
nonzero. The initialization branch of `DoPullOverTick` tests exact equality of `state+0x28` to zero;
only that branch captures and normalizes the current orientation and constructs the perpendicular
pull-over direction. A loaded positive elapsed counter bypasses that branch and uses the state
object's existing vectors. Do not reset elapsed or synthesize fresh vectors from the occupant's
orientation. A portable loader must retain the state factory's initialization for the unsaved vectors;
if a caller supplies an existing state, those vectors remain as supplied. This is literal behavior,
even when it produces a stationary or otherwise odd loaded pull-over. *[quirk]* Evidence: the exact-zero
branch and vector initialization in `0x0055E550`; the saved-state layout is automata.md §6.5.

Conformance vectors:

| Input / operation | Required observation |
|---|---|
| Enter plummet with orientation Y = 0.25; range RNG returns 1.75 | Speed bits `0xBB88888A`, oscillation 0.5, centre 0.25, wobble 1750 ms, splash 0, null effect. |
| Load a plummet record with custom five saved floats | Preserve the five floats; do not rerun the preceding entry draw. |
| Water contact; uniform RNG returns exactly 0.25 | One RNG call; fade-to-zero duration 0.25 s, splash countdown 250 ms, vertical speed bits `0x3991A2B5`. |
| Ground contact | Removal; no random splash duration and no `car_splash` effect. |
| PullOver elapsed 0, nonzero orientation | First tick captures/normalizes orientation and constructs its perpendicular. |
| PullOver elapsed 250, existing unsaved vectors U and V | First tick bypasses initialization and uses U and V unchanged by load, even if occupant orientation differs. |
