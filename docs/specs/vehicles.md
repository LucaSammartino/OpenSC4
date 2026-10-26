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
| `cSC4MassTransitTrain` | train | version 1 + complete Train record; no additional tail | automata.md §6.4.4 |
| `cSC4Tank` | vehicle | version 2 + complete Vehicle record + two signed byte model indices | §6; automata.md §6.4.3 |
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

- `Init` (`0x0055B9B0`) has the flag/default contract in §12.3 and calls the traffic automaton's `Init`; `DoStateTick` adds states 14 and 21
  (automata.md §6.2); `DoPathTick`, `DoPlummetTick`, `DoPullOverTick` are automata.md §6.5.
- **Collisions.** `HandleCollision` (`0x0055C8C0`) posts `0x49E64802`, picks a random direction, sets flag
  `0x10000000` and enters a 5000 ms state. `ResolveCollision` and `UpdateAfterDamage` (`0x0055F3F0`) report an
  accident to the traffic automata manager (`CreateAccident`), set flag `0x08000000`, raise an event (slot 102) and
  select a named injury/crash effect. This does **not** replace the vehicle's primary model; see §12.4.
- `UpdateMotionBlur` (`0x0055DD50`) loads the motion-blur model (`GetAnimatedModelFromResourceKey`) and fades it in
  using the opacity tween and four staggered samples (§12.4).
- **Player driving** (U-Drive-It, drive-tool.md): `TranslatePlayerDriveCommands` turns the commands into the
  steering and throttle tweens; `PlayerDriveUpdateSpeed` applies them; `PlayerDriveGetNetwork` finds the network the
  vehicle is on (network mask `0x40341004`); `DoPlayerDriveTick` (4.3 KB) moves it along the network, plays the AEMS
  engine and horn events and collides; `DoPlayerDriveOffRoadTick` moves it off the network;
  `ChooseNewPlayerDriveDestination` selects a neighboring network path (§12.16).

## 4. Trailers

A trailer vehicle drags its cars along the path of the head (`FindPointOnPathSegment`: the point on a path segment
at the car spacing, by segment intersection); its record lists the cars and the path cells (automata.md §6.4,
AM10) *[summary]*.

## 5. Trains

The exact TrainCar and Train record contracts, including load failure and reconstruction rules, are in §12.1.

- `Init` (`0x00557A20`) chooses a car family and target count (§12.3); animation ticks subsequently create
  missing cars (`cSC4TrainCar::Init`, flag `0x10000000`). `Shutdown` releases them.
- `StartTrip` (`0x005588D0`) checks for a rail accident before the inherited trip attempt; §12.11 gives
  the independent accident/message and trip results.
- `DoAnimationTick` (2.3 KB) moves the cars behind the engine; Train's
  `HandleCollision` requests a 20000 ms final state and perturbs car directions (§12.4).
- **Player driving**: the overspeed latch enables a curve test with a fixed random threshold; a separate
  excessive-duration test can also derail the train (§12.12).

## 6. The tank

`Init` (`0x0054C880`) loads the turret/barrel models and shot effect (`tankshot`, `fx_00_tankshot`);
initial barrel elevation is 0.2 radians. §12.13 gives aiming, power, cooldown and the projectile-effect
interface boundary. `DoPlayerDriveTick` plays the AEMS events. Its record is `u16 2`, complete Vehicle,
then two signed one-byte auxiliary model indices, first part 1 then part 2; it does not add two dwords.
[automata.md](automata.md) §6.4.3 gives the historical versions, saved-index meanings and read-result quirk.

## 7. Emergency vehicles *[compat]*

### 7.1 The base

The complete inherited record and Read/Write behavior, including the dispatch fields and the concrete
Ambulance wrapper, are in [automata.md](automata.md) §6.4.2. Ambulance has its own class ID and `u16 1`
wrapper before the complete Emergency record; it has no additional tail.

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

Its record is `u16 2`, complete Emergency record, then one crew `ref` requesting IID `0xA9B40F05`.
[automata.md](automata.md) §6.4.2 specifies exact version checks, inherited failure behavior, crew
reference assignment and worked cases; this is established from FireTruck's own Read/Write.

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

### 12.3 Construction and Init (VH4)

Construction is cumulative: Traffic → Vehicle → TrainCar → Train. Do not substitute saved-record staging
values for constructor initialization. Traffic begins with null leader, network and path references, empty
route and look-ahead deques, zero path index, unknown entry/exit bytes `0xFF`, zero remaining packed path
bytes, and transport value 7 at `+0x1E4`. It also sets the inherited flag byte at `+0x33` bit `0x10`.
Vehicle adds zeroed runtime counters/references, two unset vectors at `+0x264` and `+0x270`, drive-parameter
storage, four null blur-model slots and ring index zero. The unset vector is `(-1,-2,-3)`, not `(0,0,0)`.
Its sound/identity field at `+0x28` becomes `0xC7CC75F1`. A signed uniform integer draw between the configured
vehicle speed bounds (`0x00B21BCC`, `0x00B21BD0`) is converted to float and multiplied by the **f32 bits
`0x3991A2B5`**; the result supplies both default speed and an immediate speed-tween target.

TrainCar begins with its classification byte false and its rear-point vector equal to the global unset-point
value (`0x00B22964`); it makes a second signed integer speed draw using the train bounds
(`0x00B21D0C`, `0x00B21D10`) and the same conversion. Thus constructing a TrainCar consumes the inherited
vehicle draw as well as the train draw. Train adds target count/family zero, an empty owned-car vector,
three cached-cell coordinates `-1`, null independent network reference and zero blocked-time counter.
These runtime tuning globals are inputs; the executable's uninitialized data bytes are not tuning defaults.
Evidence: constructors `0x005544D0`, `0x005628F0`, `0x0055B150`, `0x0055B2F0`; numeric conversion and
unset-vector stores checked in instructions `0x0056295D–0x005629A3`, `0x005629F3–0x00562A4C`.

All four Init methods return success immediately when the occupant already reports initialized. For a fresh
object the owner additions are:

| Owner | Initialization contract |
|---|---|
| Traffic | Run Automaton Init. Only on success and when the loaded-occupant byte `+0x3C` is clear, refresh behaviour (slot 105) unless flag `0x4000` is set, then invoke slot 134 unless flag `0x2000` is set. Return the inherited result. |
| Vehicle | Set flag 8 to the runtime boolean at `0x00B0A0A8`; pick the lateral offset (slot 158, §12.7), then initialize Traffic. After that call, enable slot 176 with `(true,0)` if the city option queried through `0x00B43DDC` is true or the global override `0x00B2296C` is true. Return Traffic's result. |
| TrainCar | Initialize Vehicle; set flags `0x10000`, `0x4000`, `0x10000000` even if Vehicle returned false. Set classification byte `+0x2B8` to membership in either family `0x4217` or `0x4208`; return Vehicle's result. |
| Train | Initialize TrainCar and clear flag `0x10000000`. Select default speed with a floating range draw between the unsigned train speed bounds after conversion to m/ms; choose family/count below; zero `+0x2F4` and clear `+0x2F8`. Return TrainCar's result. |

The inherited Automaton Init (`0x005101D0`) asks slot 97 to construct/load model instances, initializes the
effect list only if that succeeded, and calls Occupant Init **even if model creation failed**. It returns the
model-creation result. There is no additional Traffic acquisition of city services, no local failure unwind,
and no transactional rollback of the Vehicle/TrainCar/Train additions. Shutdown is a separate operation.
In particular a failed fresh Init may leave an initialized occupant; a second Init can consequently return
success without retrying model creation. *[quirk]* Owner evidence: `0x0054D790`, `0x0055B9B0`,
`0x00556F80`, `0x00557A20`.

Train Init uses the first matching family in this ordered table; no match leaves the existing car-family value:

| Engine family | Car family |
|---|---|
| `0x4216` | `0x4203` |
| `0x4215` | `0x4201` |
| `0x4217` | `0x4208` |
| `0x422C` | `0x422D` |
| `0x422E` | `0x422F` |

For a nonzero car family, choose the target count as lower byte bound plus
`RandomUint32Uniform(upper−lower)`. Family `0x4203` uses the bounds at `0x00B21D06/07`; every other family
uses `0x00B21D08/09`. Pass the result to slot 195. This sets a target; it does not construct the cars in Init.
Preserve the random primitive's bound convention; do not add an extra `+1` to the argument.
Instruction evidence: `0x00557AD6–0x00557BC4`.

Conformance vectors (supply the tuning globals and random results explicitly):

| Input / operation | Required observation |
|---|---|
| Construct Vehicle; speed integer draw returns 36 | Default/target speed is f32 `36 × bits(0x3991A2B5)`; blur slots null, ring index 0; unset vectors `(-1,-2,-3)`. |
| Construct TrainCar | Observe two signed integer speed draws in order: vehicle bounds, then train bounds. |
| Fresh Traffic; model creation fails | Return false; still call Occupant Init; no behaviour refresh or slot 134. No rollback. |
| Loaded Traffic; model creation succeeds | Skip fresh-object behaviour and slot 134, preserving loaded route-related state. |
| Fresh TrainCar; Vehicle Init fails | Return false, but all three TrainCar flags and classification are still applied. |
| Train belongs to both `0x4216` and `0x4215`; count bounds 3 and 8, uniform result 2 | Family `0x4203`; uniform argument 5; target count 5; no car allocation by this Init. |
| Already-initialized Train | Success; no family checks, speed draws or target-count changes. |

### 12.4 Collision direction, damage effects and blur (VH6)

`HandleCollision(other)` (`0x0055C8C0`) returns success without action for a null other object. Otherwise it
posts `0x49E64802` with all six payload arguments zero, including when already in COLLIDE (20). An already
colliding object makes no new direction draw and does not restart the timeout. For a new collision create a
local random generator seeded `0xFFFFFFFF`, obtain a random unit direction R, and replace facing F by
`normalize(F + 0.5 R)`. The direction primitive rejects vectors outside the unit ball or with squared length
below `0.0001`, then normalizes; its three coordinates are uniform draws in `[-1,1]` before rejection
([geometry-utilities.md](geometry-utilities.md) §6.3). This is a three-dimensional perturbation, not a uniform
yaw. Set flag `0x10000000` and request final state 20 with 5000 ms timeout; return false if state creation
fails, retaining the preceding message/direction/flag changes. There is no direct model-transform replacement
in this method. Instruction constants and operation order: `0x0055C8C0–0x0055CA73`.

Train overrides this contract (`0x00558E00`). It requests final COLLIDE for **20000 ms** on the locomotive
unless already in state 20, then walks the owned cars in stored order using one local RNG seeded
`0xFFFFFFFF`. For each car, first copy the locomotive's measured speed `+0x94`, draw the same random unit
vector R, scale by `0.25/length(R)` **before** discarding its Y component, and normalize the car's facing plus
that horizontal perturbation. This does not normalize the projected XZ vector to length 0.25. The locomotive
facing is unchanged. If this car already has state 20, return **success immediately**, even if a previous
state request failed; later cars are untouched and the final locomotive speed reset is skipped. Otherwise
request the same final 20000 ms state on the car and remember any failure, continuing to later cars. After
the entire list, set locomotive speed to zero immediately and return whether all requests succeeded.
There is no target-null gate, vehicle collision-message post, or explicit `0x10000000` flag set in this override.
The early success and skipped reset are literal quirks. Numeric evidence:
`0x00558E18–0x00558E3F`, `0x00558E79–0x00558F4F`; duration bits `0x469C4000`, quarter-scale constant
`0x00A8821C=0.25`.

`ResolveCollision` (`0x0055F7B0`) takes a collision vector V, impulse I, PlayerDrive state S and drive parameters
P. First round `q = I/P[+0x48]` to f32. Let `m = sqrt(Vx²+Vy²+Vz²)`. If `m > 0.01` **or** `q > 2`, add
`(m−0.01) × 2000` to the state's unsigned damage count at `+0x60`, multiplying that increment by q when
`q >= 5`. Convert the accumulated value by truncation toward zero and store the low 32 bits. There is no
nonnegative clamp on this increment: a small vector with a large impulse can reduce damage. *[quirk]*
Next run UpdateAfterDamage. Finally set speed to `clamp(currentSpeed + dot(V,facing)×q, 0, currentSpeed)`;
this is an immediate tween change. Copy it to S's raw speed `+0x2C`, and zero S's acceleration accumulator
`+0x28` if the result is zero. Damage update precedes speed reduction. Numeric evidence:
`0x0055F7B0–0x0055F8F7`; `0x00B0A0B4=bits(0x3C23D70A)`, `0x00B0A0B8=2000`,
`0x00A9D170=5`.

`UpdateAfterDamage` (`0x0055F3F0`) does not itself calculate damage or reduce speed. Compare the unsigned
damage count D with the parameter health H at `P+0x5C`. Below `H/2`, do nothing. From `H/2` inclusive to H
exclusive, request the named effect **`udriveit_injured`**. At H or above, create a traffic-manager accident
with `(vehicle occupant,2,30000)`, store its result at `S+0x68`, set flag `0x08000000`, emit slot 102 event
`(11,1)`, and request **`udriveit_crash`**. These operations are not guarded against repeated calls at lethal
damage. If the effect request returns null, retain the old effect. Otherwise stop the old `S+0x74` effect with
argument 0, release it, retain the new effect, apply a transform at the vehicle position oriented from its
current facing with unit scale, and start it with argument 0. The selected object is an effect instance, not
a damaged mesh selected from a family or substituted for the primary vehicle model. This corrects §3's old
summary. Thresholds, accident arguments and event payload checked at `0x0055F413–0x0055F4DE`.

`UpdateMotionBlur` (`0x0055DD50`) maintains four model instances at `+0x2A4..+0x2B0`, indexed by byte
`+0x2B4`. If the current slot is null, request the animated model using the vehicle's resource key at
`+0x11C`; there is no separate hard-coded blur resource key. On success retain that model in the slot.
Place the current sample at the current position with orientation derived from facing; register a newly
created instance with the model system, and update an existing instance there. A failed model request does
not skip the alpha/ring updates. Let A be `min(GetCurrent(opacity tween at +0x1B0),1)`. In current-to-oldest
ring order, set alpha to A, then repeatedly subtract the f32 value `0.33333334` and clamp the *next* value
to at least zero. Advance the index modulo four. Thus the first sample's alpha has no lower clamp.
This method uses the opacity tween, not the measured-speed field `+0x94`. Evidence: resource-key address
`0x0055DD7C`, alpha loop and ring update `0x0055DE98–0x0055DF45`.

Conformance vectors:

| Input / operation | Required observation |
|---|---|
| Null collision target | Success; no message, RNG or state change. |
| Nonnull target; already in state 20 | One all-zero `0x49E64802` post; no direction draw or new timeout. |
| F=(1,0,0), supplied unit R=(0,1,0) | New facing approximately `(0.89442719,0.44721360,0)`; flag set before final 5000 ms state request. |
| Train with two noncolliding cars; each supplied R=(0,1,0) | Each car keeps its facing after normalization and receives measured-speed copy plus final 20000 ms state; locomotive facing unchanged and speed reset to zero. |
| Train's first car already COLLIDE; second car untouched initially | First car still receives speed copy and a direction draw, then immediate success; no second-car work and no locomotive speed reset. |
| Damage D=49,50,99,100 with H=100 | Respectively no effect, injured, injured, crash; only the last creates accident `(2,30000)` and event `(11,1)`. |
| D=100,H=100; effect request fails | Accident/flag/event still occur; prior effect retained. |
| D=100,H=100; call UpdateAfterDamage twice | Two accident requests and two `(11,1)` events; no once-only latch. |
| Current speed 0.02, V=(-0.01,0,0), F=(1,0,0), q=1 | Speed becomes 0.01; raw state speed matches. |
| Blur ring index 2; opacity 1; all slots present | Slots 2,1,0,3 receive approximately `1,2/3,1/3,0`; next index 3. |
| Blur opacity -0.25; current model request fails | Existing current slot absent; next three slots receive 0; ring still advances. |

### 12.5 Neighbour ranking and look-ahead contracts (VH2)

The neighbour selector (`0x00554EA0`) ranks successful pathfinder results by **route-entry count**, not
Euclidean distance or a random draw. Only status 1 qualifies; an equal-sized route does not replace the
earlier winner. Clamp each candidate destination cell to the city rectangle before pathfinding. The selector
requires the city, neighbour-connection service and pathfinder service; a missing prerequisite returns null.
Ranking evidence: `0x00555114–0x0055523B`. Collect connections in network-type order **3, 12, 8, 2, 10**,
preserving each collection's order. Convert the automaton's X/Z position to the start cell through the city's
coordinate conversion. Both start and end rectangles are single cells. The path request uses travel mask 2
(also starting), traffic density 0, flag 1 and destination filter 3. Associate its output with a temporary route
deque; after each attempt reset the finder and clear that deque. A successful zero-entry route is eligible
and beats every positive entry count. The winning connection is retained during selection, but the returned
object is a **network occupant at its clamped end cell**, obtained through the cell-lookup service at
`0x00B43D54` (slot 14), not the neighbour-connection object itself. No winning connection, missing lookup
service, or missing cell lookup leaves the output null. This lookup does not cause another ranking/pathfinding pass. The selector neither installs
the winning route on the automaton nor calls SlowDown. Request evidence: `0x00554F00–0x00554F59`,
`0x0055501F–0x005550ED`, `0x0055514C–0x00555287`; pathfinder slot meanings: pathfinder.md §7.

Look-ahead fill (`0x00554870`) seeds an empty deque with the current retained network reference and packed
path word, then fills to **slot 175's count plus one**, including the seed. It stops at a terminal node. A
failed expansion marks the preceding node terminal and returns false. The base slot 175 (`0x00504FE0`)
returns the unsigned tuning byte at `0x00B21D28`; it is not a fixed literal count. Flag `0x40` selects the
explicit-route expansion (slot 168, `0x005527E0`); when clear, use slot 167 (`0x00550F40`). Slot 159 is
`SlowDown` (`0x0054DA00`), not the neighbour ranking operation.

**Node and path-word contract.** A look-ahead node comprises a retained network reference, a packed `u32`
path word, and a terminal bool. The word's low byte is exit direction; byte 1 is entry direction; bits 24–27
are the path's lane/type selector. The expansion routines preserve that selector, set the new entry to
`(old exit + 2) mod 4`, and pack a successful exit as
`selector × 0x01000000 + entry × 0x100 + exit`; byte 2 and bits 28–31 are zero in this newly packed word.
They accept a previous node and an output node, and return a bool. A false result may already have changed
the output reference or consumed explicit route data: these are not transactional operations.
Evidence: `0x00550F59–0x00550F79`, `0x0055107E–0x00551090`, `0x005527F7–0x00552824`,
`0x0055293D–0x00552952`, `0x005529FA–0x00552A07`.

**Ordinary expansion (slot 167).** The traffic-network service's next-tile query receives the previous
reference and path word, mode **0**, and selector. No result, or a result with no network occupant, fails.
The returned occupant becomes the output reference even if no exit subsequently succeeds. Obtain its path
information, and ask for an ordered four-direction list using its directional traffic data. This is an
interface boundary with the traffic-network and traffic-simulator services: it uses their returned order and
eligible-direction count, not Euclidean distance, the neighbour selector's shortest-route comparison, or an
owner-side random permutation. The query at `0x0054E6F0` supplies the automaton's travel type, the simulator's
travel-type scaling value, and the returned tile to obtain a 4 × 4 directional table; automaton slot 170 may
adjust that table for attraction before traffic-network slot 6 supplies the ordered row for the new entry.
The underlying service's ranking/draw policy is outside these four owner slots.

In that order, an exit qualifies when the network occupant accepts `(selector, new entry, candidate exit)`
through network-occupant slot 53 (`+0xD4`). An automaton whose slot 141 returns true additionally excludes the
previous path word's **entry byte** from this list. Traffic's base slot 141 returns false, so it does not make
that exclusion. On an accepted exit, set the packed word and succeed; the ordinary expansion does not itself
set the terminal bool on this path (the fill routine initializes the output bool to false).

A rejected exit has a special fallback when its list index is **strictly less** than the eligible-direction
count and fallback is allowed. Request a generated path from the occupant's path-information service (slot 9,
`+0x24`) with `(selector, new entry, 0xFF, tuning-data at 0x00B21D78, output word, 0)`, mark the output
terminal and return true. The owner does not test the generator's return value. Fallback is forbidden if the
automaton's byte `+0x1E8` is nonzero, travel type is 7 or 8, automata flag `0x200000` is set, or its current
network occupant exists and reports network flag 2. Otherwise fallback is allowed. Exhausting all four exits
returns false. Evidence: `0x00550F7C–0x005510CA`, `0x0054E6F0–0x0054E77F`,
`0x0054EA30–0x0054EA77`; the base slot-141 target `0x00648EF0` returns false.

**Explicit-route expansion (slot 168).** Empty route data fails. Otherwise take the first route triple's
unsigned X and Z bytes, obtain the network list at that cell, and **consume that triple before checking for
a usable connection**. The third byte is not used by this routine. If another triple remains, derive the
desired exit by comparing it with the consumed cell: negative Z delta → 1, positive Z delta → 3, otherwise
negative X delta → 0, otherwise → 2. Thus a duplicate cell selects 2, and Z takes precedence for a diagonal
pair. Inspect the cell's network occupants in service-list order and use the first non-null one accepting
`(selector, new entry, desired exit)`. Retain it, pack the word, clear terminal and succeed. A missing list or
no accepting occupant fails without restoring the consumed triple. Evidence: `0x00552824–0x0055295F`.

Consuming the last triple instead marks the output terminal and asks the next-tile service using the previous
reference/path, mode **1**, and selector. No next tile fails. Otherwise retain its occupant and try four exits
cyclically, starting with `(new entry + 2) mod 4`; use the first that accepts the same selector and new entry.
No accepted exit fails. Success packs the word and leaves terminal true. This branch does not use ordinary
expansion's generated-path fallback, nor slot 141's exclusion. Evidence: `0x00552962–0x00552A15`.

**Fill result and failure.** A terminal tail prevents further expansion; already having the target node
count succeeds without expanding. A missing tail reference
also prevents expansion. Expansion failure appends no output node and marks the existing tail terminal;
successful expansion appends its retained result. Request count zero still requires the seed (target size one).
Evidence: `0x00554894–0x00554938`, `0x00554978–0x005549C8`, `0x00554A6D–0x00554AA8`.

**SlowDown (slot 159).** Request a speed tween to zero. With positive `GetSpeedModifier()` (`slot 125`), its
duration is binary32 `(1 / modifier) × 1.5`, in that operation order; with a nonpositive modifier it is
binary32 `0.1` (`0x3DCCCCCD`). This is unrelated to ranking. Instruction evidence:
`0x0054DA04–0x0054DA42`; literals `0x00A81228 = 1`, `0x00A880D4 = 1.5`.

Conformance vectors (use controlled service results for the foreign network/pathfinder interfaces):

| Setup | Required outcome |
|---|---|
| Connections A/B/C in collection order give `(status,count)` = `(1,9)`, `(1,4)`, `(1,4)` | B wins; C does not replace it |
| A gives `(2,1)`, B gives `(1,8)`, C gives `(1,0)` | C wins; status 2 is ineligible |
| City 8 × 6; candidate end cell `(9,−1)` | Finder receives end rectangle `(7,0,7,0)` |
| Winning connection A at clamped cell; network-cell lookup returns occupant B | Return retained B; do not run another path request |
| A wins but its network-cell lookup is missing | Return null, not the neighbour connection A |
| Old word `0xA3EE0102`; accepted exit 3 | Selector 3, entry 0; new word `0x03000003` |
| Ordinary service order `[3,2,0,1]`; only exits 2 and 0 accept | Choose 2; no geometric re-ranking |
| Ordinary rejection at index 0, eligible count 1, fallback allowed | Generate terminal path immediately, even if later exits might accept |
| Same rejection with eligible count 0, or fallback forbidden | Do not generate; continue trying exits |
| Explicit cells `(4,5)` then `(3,4)` | Consumes `(4,5)` and seeks exit 1 (Z comparison wins) |
| Explicit duplicate cells `(4,5)` then `(4,5)` | Seeks exit 2 |
| Explicit first cell has no usable network | False; first triple remains consumed |
| Last explicit cell; new entry 1 | Next-tile mode 1; exit tests are 3, 0, 1, 2; terminal true |
| Fill target size 3; only seed exists and expansion fails | False; size remains 1; seed becomes terminal |
| Slot 175 returns 0 and deque starts empty | Seed only; true without expansion |
| Speed modifiers 2, 0, −1 | SlowDown durations 0.75, 0.1, 0.1 respectively; all target speed 0 |

### 12.6 Inherited state and dispatch contracts (VH9)

These are native inherited contracts for traffic, vehicle, TrainCar and Train. A controller is a retained
interface on the state, not an instruction to invoke a foreign vehicle owner's tick.

**Countdown and controller precedence.** The base state dispatcher first subtracts the tick's millisecond
delta from a strictly positive state timeout. Only a result **strictly below zero** pops the state: reaching
exactly zero does not pop it and subsequent ticks no longer decrement that zero. After a pop, the newly
exposed state receives the full tick delta; no residual-time calculation occurs. If the stack became empty,
the dispatcher returns true. Otherwise its controller, if present, gets `(automaton, state type, full dt)`
before the built-in handler. A true controller result bypasses that handler. A false controller result runs
the built-in handler. Evidence: `0x00508AA0–0x00508C11`.

**PLOP_JUMP.** With removal flag 1 set, return false without moving. Otherwise X/Z remain unchanged and the
candidate Y is `old Y + dt_ms × vertical velocity`. When candidate Y is strictly below the saved floor, clamp Y
to the floor and leave the velocity unchanged. When it is at or above the floor apply
`velocity := velocity + gravity × dt_ms`; gravity is binary32 `0xB8690455` (about −0.000055555563).
MoveTo receives the resulting position and a null orientation, and the handler returns true. Landing does
**not** pop the state or reset velocity. Evidence: `0x005055F0–0x0050568F`; the ordered operations and floor
comparison are at `0x00505630–0x00505676`.

**ATTACH_ANIM.** AttachAnim takes the model resource input, animation selector, return/handled bool and state
timeout. It first pushes state 19 with the supplied timeout and no controller, then stores the bool and
selector and acquires an added model instance. A successful model acquisition retains that model, selects the
animation (model slot 16) and sets its animation mode/value through model slot 18 to `(1,0)`; return true.
A failed push returns false with no added model. Failed model creation returns false **with the pushed state
still present**. Evidence: `0x005102A0–0x00510358`.

On tick, removal flag 1 returns false before accessing the model. With a model, obtain its model-properties
record (model's property-interface slot 15, `+0x3C`). If its final float is below 1, set it to
`min(1, old value + dt_ms / (T × 0.5))` through model slot 18 with first argument 1, where T is runtime tuning
float `0x00B21C84`. The additions/division use the instruction order shown, with a binary32 store before the
clamp. Independently, selectors **2 and 3** suppress animation-completion popping. For every other selector,
a present model whose current frame (slot 11) is at least frame count (slot 12) minus one causes PopState and
returns true. Otherwise return the state's saved bool, including when the model is null. Do not pop a null
model automatically. Evidence: `0x00508F80–0x00509067`, especially `0x00508FD5–0x0050901C` and
`0x0050901F–0x00509065`.

| Conformance setup | Required outcome |
|---|---|
| Timeout 20 ms; tick 20 ms, then tick 1 ms | Timeout becomes zero and stays zero; neither tick pops on timeout |
| Timeout 20 ms; tick 21 ms, previous state exists | Pop once; exposed controller/handler gets 21 ms |
| Controller returns true for PLOP_JUMP | No built-in hop movement |
| Hop floor 10, Y 10.1, velocity −0.01, dt 20 ms | Y becomes 10; velocity remains −0.01; no pop |
| Hop floor 10, Y 10, velocity 0, dt 20 ms | Y stays 10; gravity changes velocity to approximately −0.0011111113 |
| Attached selector 2 or 3, last frame reached | No completion pop; return saved bool |
| Attached selector 1, count 5, current frame 4 | Pop and return true |
| Attached null model, saved bool false/true | Return false/true respectively; state remains |
| Attached property value 0.8, T 1000 ms, dt 200 ms | Set `(1,1)` after clamping |

**RANDOM_WALK (11).** State-local data are desired direction `+0x28` (three f32s), turn timer in
seconds `+0x34`, signed walk/rest timer in seconds `+0x38`, and rest-animation-issued byte `+0x3C`.
Use the state's behaviour record `+0x24`, or the default record `0x00B09988` when null. Its offsets
`+0x50/+0x54` are turn-interval bounds, `+0x58` random turn half-range in revolutions,
`+0x5C` minimum turn in revolutions, `+0x68/+0x6C` walking-duration bounds,
`+0x70/+0x74` rest-duration bounds and `+0x78` deceleration duration. All draws here use the global
generator `0x00B21D78`, uniform **double** range arguments converted from the f32 bounds.
The default values at those offsets are respectively `(10,15), 0.1, 0.2, (0,0), (5,10), 0.5`.

A negative walk/rest timer denotes rest. Add `dt_ms × f32(0.001)` and store f32. On reaching zero
or above, draw a new walking duration, apply that behaviour (slot 84), clear the animation byte and
play the state's animation selector `+0x1C`; return true without movement this tick. While still
negative, measured speed below f32 `0.01` issues the empty-name animation once and sets the byte.
A nonnegative timer denotes walking. Walking-duration lower bound within f32 `0.0001` of f32
`0.00001` disables the walking countdown. Otherwise subtract elapsed seconds; strictly negative
draws a rest duration, stores its negation, and requests speed zero over the deceleration duration;
return true without movement. Exact zero remains walking for this tick.

During walking subtract elapsed seconds from the turn timer. While it is strictly negative, add a
new turn-interval draw and draw r from `[-half_range − 0.00001, half_range + 0.00001]` (the epsilon
in these range arguments is **double**). Let a be f32 `atan2(desired.z, desired.x) + r × f32(2π)`;
add `minimum_turn × f32(2π)` when r is nonnegative, subtract it otherwise. Store desired direction
`(cos(a),0,sin(a))` as f32. Repeat for a still-negative turn timer. There is no iteration cap or
repair for nonpositive configured intervals. Evidence: `0x0050CF90–0x0050D1C8`.

Let D be that direction plus the automaton's auxiliary vector `+0xA4` when flag `0x4000` is clear;
with that flag set, use the desired direction alone. Steering direction is
`normalize(facing + D × (dt_ms × f32(0.002)))`, using the actual 3D components. There is no
zero-length normalization guard. Multiply each normalized component by current speed tween,
store f32, then multiply by slot 125's speed factor and dt_ms to obtain displacement. Add to the
old position. Without flag `0x20000`, terrain service slot 44 adjusts the old/candidate segment
using mask `0xC49`. With `0x20000` but without `0x40000`, replace candidate Y with terrain height
at candidate X/Z. With both flags, leave Y alone. MoveTo receives candidate and steering direction.
Refresh behaviour (slot 105) only when `0x4000` is clear. Return true; this handler has no removal
flag early return. Arithmetic uses x87 intermediates and the f32 stores in the stated order;
normalization reciprocal itself is stored f32. Evidence: `0x0050D1C8–0x0050D3BA`.

**Controller attraction (state 18, MAGNET).** This is the remaining controller-position handler
sometimes called ZOT in the question; there is no distinct ZOT case in the native state dispatcher.
The actual dispatch is type 18 to slot 117 (`0x00508E10`). Removal flag 1 returns false. Without a
controller, refresh behaviour and return true. Otherwise ask controller slot 11 for its position,
subtract current position and take the 3D length. Length **greater than** f32 `0.0001` produces a
normalized direction multiplied by the current speed tween, **without dt_ms or slot 125**; add
this displacement to current position. Unless flag `0x40000` is set, replace Y with terrain height
at candidate X/Z. MoveTo receives candidate and null orientation. Refresh behaviour even when the
distance was too small to move; return true. It does not stop at, clamp to, or pop on reaching the
controller. Evidence: dispatcher `0x00508AA0`, numeric body `0x00508E60–0x00508F74`.

**Base dispatch slots.** Traffic and its Vehicle/TrainCar/Train descendants inherit these literal
contracts; emergency dispatch storage/state transitions in §7 are overrides, not base behaviour.
Slot 23 returns a **u16 zero** call number; slot 24 discards the supplied call number. Slot 25
discards the supplied dispatch state; slot 26 always returns **3**. Slots 27/28 get/set the u32
dispatch-info ID at `+0x88` without notifications, validation or state changes. The shared bodies'
exported names can describe unrelated classes: contracts follow the table targets, not those names.
Evidence: traffic primary table `0x00A9C068`, words `0x00A9C0C4–0x00A9C0D8`, bodies
`0x0098CAF0`, `0x00648F10`, `0x004FAD00`, `0x007C9FC0`, `0x004FAD10`.

| Additional conformance setup | Required outcome |
|---|---|
| Rest timer −0.1 s, dt 1000 ms | One walking-duration draw; behaviour and animation restart; no displacement |
| Walking timer 0.1 s, lower duration bound 1, dt 100 ms | Zero does not enter rest; movement still runs |
| Same, dt 101 ms; rest draw 7 | Timer −7; speed target zero using configured deceleration; no turn draw/movement |
| Duration lower bound zero, timer zero | Countdown disabled by epsilon comparison; walking persists |
| Turn timer zero, dt zero | No turn draws |
| MAGNET controller at (10,0,0), position zero, speed tween 2, flag 0x40000 | New position (2,0,0) for both dt 1 and dt 1000; null orientation |
| MAGNET controller coincident, controller tick returned false | No MoveTo; behaviour refresh; true result |
| Base owner: set call number 7, dispatch state 9, info ID 42 | Getters return 0, 3, 42 respectively; no dispatch event |

### 12.7 Init lateral offset and initial path (VH10)

**Slot 158** (`0x0055B690`) takes no explicit arguments and changes only the lateral-offset float
at `+0x200`: draw `RandomDoubleRangeUniform` from the two binary32 tuning inputs at
`0x00B21CF0` and `0x00B21CF4`, promoted to double, then narrow the returned value once to
binary32. It neither reads an exemplar nor fills drive-parameter storage. It runs after setting flag 8
and before Traffic Init, even if that Init subsequently fails. **Slot 134** is `0x00550B60`, a
position-to-network-path initializer with no explicit arguments. It is distinct from `BeginRoute`
(`0x00554AB0`, which takes a bool and can consume a route front). Traffic Init discards slot 134's
result and returns the inherited Init result. Consequently lack of an initial path is not an Init
failure. The existing fresh/loaded, flag `0x4000`, and flag `0x2000` conditions in §12.3 still apply.

The initial-path contract is:

- If all three position components are within a **strict** binary32 `0.0001` of `(-1,-2,-3)`,
  return false before querying network geometry. The epsilon is bits `0x38D1B717`.
- Obtain the city network service and the owner's traffic manager (slot 174). Convert current X/Z
  to cell coordinates through the city service and ask for that cell's network occupant (service
  slot 11). A missing occupant or missing path-info interface (occupant slot 49) returns false.
  This routine does not check the returned traffic manager for null before using it.
- Ask the traffic manager for the owner's transport/path type (manager slot 68). Initialize the
  desired exit byte to `0xFF`. The owner's attraction-vector getter (slot 122) can supply a heading;
  if it succeeds, normalize that vector in place and ask owner slot 171 for an exit on this occupant,
  with the final bool true. Otherwise do that only when flag `0x4000` is clear, a behavior template
  exists, and the stored attraction vector has length strictly greater than `0.00001`. The successful
  getter branch has no extra zero-length guard before normalization. *[quirk]*
- Ask path-info slot 5 for a path with that transport type, entry `0xFF`, selected exit, the current
  position, an output path-selection byte, and final bool false. No route deque entry is read or
  consumed. A null path returns false; any preceding attraction-vector changes remain.
- For a path, call placement slot 132 with `(occupant, selection byte, path, true)`. Copy the current
  position and ask path-info slot 19 to project that copy onto the path and return its segment index.
  Save the index at `+0x1FC`, replacing negative results by zero. The projected copy is the final
  MoveTo position; it is not simply the first point of the selected path.
- With at least two path points, use point 1 minus point 0 for the facing direction; if its length is
  strictly below `0.00001`, try last point minus first. If that is also strictly below the threshold,
  use existing facing. Normalize the selected vector and narrow its components to binary32.
  With fewer than two points, copy existing facing without this normalization. MoveTo receives
  the projected position and that facing; return true. There is no rollback or final MoveTo-result
  check. Geometry selection/projection are network services, not a callback to a game vehicle owner.

Instruction evidence: slot-158 double arguments and single float store `0x0055B690–0x0055B6BA`;
Vehicle ordering `0x0055B9C3–0x0055B9E0`; Traffic result handling `0x0054D7A3–0x0054D7EF`;
slot-134 sentinel and service calls `0x00550B60–0x00550C6A`, heading selection
`0x00550C70–0x00550D58`, path selection and placement `0x00550D5E–0x00550DA1`, projection/index
`0x00550DA1–0x00550DD8`, facing fallbacks and MoveTo `0x00550DD8–0x00550F30`.
The constructor installs primary table `0x00A9C068` (`0x005544FB`); its slot 134 word at
`0x00A9C280` is `0x00550B60`. This also resolves the misleading drive-parameter name in §12.3.

| Conformance setup | Required observation |
|---|---|
| Slot 158 bounds 0.25 and 0.75; range primitive returns 0.5 | One double-range draw with those bounds; float `+0x200 = 0.5`; drive-parameter storage untouched |
| Fresh Vehicle, inherited model Init fails | Offset draw still precedes inherited Init; no slot 134; return false |
| Successful fresh Traffic Init; initial position exactly `(-1,-2,-3)` | Slot 134 false with no geometry queries; Traffic Init true |
| Successful fresh Traffic Init; network occupant or its path-info absent | Slot 134 false; Traffic Init true; no placement or MoveTo |
| Empty route; valid network path selected at current position | Empty route is permitted and stays empty; placement/projection proceed |
| Selected path `[ (0,0,0), (3,0,4) ]`; projection returns `(1.5,0,2)`, index −1 | Save index 0; MoveTo projected position with facing `(0.6,0,0.8)` (binary32 components) |
| Selected path has one point; existing facing `(0,0,2)` | MoveTo uses `(0,0,2)` unchanged |
| Path selection returns null after attraction normalization | False; no placement/MoveTo; normalized attraction remains; no route-front pop |

### 12.8 Train references are runtime car references (VH11)

There is no train-specific second identity-to-runtime binding phase after §12.1's Read: the shared
stream supplies the actual shared serializable object, which is immediately queried for car IID
`0x4A416254`; the resulting retained car interface is the list entry used by runtime operations.
A portable implementation must resolve a reference to that same object instance through its shared
object registry, rather than construct another TrainCar from an ID. If the reference reader yields
null, cannot resolve an object, or cannot obtain that IID, no entry is appended; Train ignores the
helper's failure return and continues to its tail, retaining the nested Vehicle Read result.
No missing-entry placeholder or later train retry is created. If two references resolve to the same
car, both are appended in order, aliasing one runtime object, with the reference ownership obtained
for each occurrence; there is no duplicate-ID rejection or deduplication. Existing list entries are
also retained. A valid live TrainCar supplies its serializable interface as part of the same COM
object; a separate optional serializable identity is not native car state. The generic writer queries
that interface (`0xE4FDA3D4`) and sends it to the shared stream: a null object or failed query can
produce a null reference, not a train-level early validation error. This does not make a valid
TrainCar's serializable interface optional. Preserve stream errors separately from Train's own
return value. *[quirk]* Train's permissive loading and duplicate aliases must not be replaced by
atomic validation or by copying a car per list entry.

Evidence: reference helper interface acquisition `0x005D9739–0x005D9773`; shared-object Read and car
QueryInterface `0x005D96D6–0x005D970E`; Train null test/append without a duplicate search
`0x0055AB50–0x0055ABB0`; TrainCar's car-interface query `0x00557040`; generic writer's serializable
query/null forwarding `0x005D99BE–0x005D99EB`. The decompiler removes reachable Train append blocks;
the append contract here is verified against instructions, not that pseudocode omission.

| Conformance setup | Required observation |
|---|---|
| Registry maps references A and B to distinct live cars | Read appends those same runtime objects in A/B order; no post-load owner binding callback |
| References `[A, missing, A]`, successful nested Vehicle Read | Append two entries aliasing A; skip missing; consume all three; Train result true |
| Reference resolves to an object without the car IID | Skip it; no guessed car allocation; continue Train tail |
| Existing `[X]`, load `[A,A]` | Result `[X,A,A]`; both A entries share one runtime state |
| Valid live TrainCar written twice | Query its serializable interface for each list occurrence; use shared-stream identity semantics, not two cloned runtime records |

### 12.9 Route geometry and serialization boundary (VH7)

This subsection specifies the route/path distinction, owner geometry and saved-field aliases.
The final load-to-live binding and opaque-byte meanings are narrowed to the explicit runtime
observations below; no successful post-load reconstruction is claimed without those observations.

**Route replacement and failure.** `SetRouteInfo` (`0x005559D0`) rejects an empty input deque before
replacing the existing route. With a nonempty input it copies the route first. If immediate BeginRoute
was requested and fails, return false with the copied route and BeginRoute's preceding changes retained;
do not restore the old route. Only successful setup (or setup without immediate BeginRoute) requests
return to default speed. BeginRoute's optional bool selects a preparation-only success: after clearing
look-ahead, preparing the route and clearing flag `0x2000`, true returns without selecting a first path.
The ordinary path takes the first cell and consumes that route step before looking for an acceptable
network occupant; a missing occupant/path returns false after that consumption. Evidence:
`0x005559D0`, `0x00554AC8–0x00554B15`, `0x00554B43–0x00554B68`, `0x00554BAA–0x00554D13`.

**Distance advancement.** Slot 133 (`0x005504E0`) takes a signed f32 distance, start point, output point,
optional index pointer, optional path-point collection and optional remainder pointer. Omitted collection
and index use the owner's current path `+0x1F8` and segment index `+0x1FC`. An optional remainder is
zeroed before checking for a missing collection; a missing collection returns false before copying the
start point. Otherwise copy start to output. A negative index resets to zero. An index already at or
beyond the last segment returns false and supplies the whole input distance as remainder. Points are
three f32s, not route-cell triples. Each next point is passed through owner slot 157 before calculating
its 3D distance from the running output point. For a nondegenerate segment, normalize its delta. A step
less than or equal to that segment's length, including a **negative** step, moves output by the normalized
delta times the step and returns true. Negative distance does not walk previous segments. Exact equality
does not advance the segment index. A larger step advances the index and consumes complete intermediate
segments. Crossing the final endpoint returns false, supplies the distance beyond it as remainder, and
extrapolates the output along the final segment rather than clamping it to the endpoint. Evidence:
`0x005504E0–0x00550586`, `0x0055058C–0x005506DF`, `0x005506E2–0x0055075A`.

**Vehicle front anchor.** The vehicle placement override (`0x0055D5A0`) retains the new network/path
interfaces and resets the segment index. Its ordinary placement starts from the first two transformed
path points; the first is the stored front anchor `+0x264`. Facing is the normalized difference of those
points. Centre position is `front anchor − facing × radius × 0.5`, with radius at `+0x7C`. The
preparation-only placement instead clears flag 4 without replacing this geometry. During path reattachment
(`0x0055D210`), positive saved path distance `+0x260` is limited by the current available gap before
advancing the front anchor. Facing comes from the selected segment, or the final segment after path
exhaustion. Segment length at most f32 `0.00001` reuses saved facing `+0x270`. Otherwise normalize it;
when saved facing differs from the unset vector `(-1,-2,-3)` by at least f32 `0.0001` in any component,
replace it by `(new + new + saved) × f32(1/3)`. **Do not renormalize that average.** Centre is again the
front anchor minus half-radius times this resulting facing; store that facing and clear flag `0x20`.
Evidence: `0x0055D645–0x0055D82B`, `0x0055D210–0x0055D55A`; the average's stores and centre subtraction
were checked in instructions at `0x0055D435–0x0055D54E`.

| Geometry conformance setup | Required observation |
|---|---|
| Existing route A; SetRouteInfo receives empty B | False; A remains |
| Path points (0,0,0), (10,0,0); index 0; start zero; step −2 | True; output (−2,0,0); index 0; remainder 0 |
| Same path; step 10, then a separate call starting at endpoint with index 0 and step 1 | First returns true at endpoint with index 0; second exhausts path, returns false at (11,0,0), remainder 1 |
| Vehicle front anchor (10,0,0), facing (1,0,0), radius 4 | Centre (8,0,0), not (6,0,0) |

#### Lane matching and live path representation

The route-cell deque and the selected network path are different representations.
A route cell occupies three bytes: unsigned cell X, unsigned cell Z, and a byte
preserved by serialization. BeginRoute and the route-following helper use the first
two bytes to find the cell and derive the next exit; they do not use the third byte
as a lane mask or a path-point coordinate. A decreasing Z selects exit 1, increasing
Z exit 3; at equal Z, decreasing X selects exit 0 and nondecreasing X exit 2.
This includes equal consecutive cells, which select 2 rather than rejecting the
route. The final cell has no following-cell exit and uses the network's available
path selection. Evidence: `0x00554B43–0x00554BA6`, `0x00552824–0x00552844`,
`0x00552925–0x00552A15`. This establishes the third byte's treatment by these
consumers, not a meaning for that byte in the pathfinder that produced it.

The **live path key** at `+0x1F4` is a separate u32. Its low byte is the exit,
the next byte the entry, and the low nibble of its high byte is the network/path
type passed to the network service. Do not reinterpret the route's three bytes as
this key. The live network occupant is `+0x1EC`, its retained path interface
`+0x1F0`, the selected point collection `+0x1F8`, and the current segment index
`+0x1FC`. The lateral offset is f32 `+0x200`. The point collection contains vec3
f32 points; it is borrowed from the path service, rather than serialized as route
cells. Evidence: key getter `0x00504F70`, offset getter `0x00504F80`,
`0x0054E951–0x0054E960`, `0x00550768–0x005509C7` and
`0x00550F40–0x005510CF` (the latter packs exit, entry and type).

For FindLeaderInTile, let K be the candidate's live path key, Q the requested
key, and M1/M2 the two caller-supplied masks. The path test accepts
`(K & M1) == (Q & M1)` **or** `(K & M2) == (Q & M2)`. It does not require both
equalities and does not test nonzero intersection. A candidate with flag `0x2000`
bypasses this path test, after the earlier candidate eligibility tests. An all-zero
mask therefore makes its equality true, including when the keys have no common
bits. Default masks used by FindLeader are `0x0FFF00FF` and `0x0FFFFF00`.
Within the eligible candidates, planar X/Z distance is used; an equal-distance
later candidate replaces the earlier one. Evidence: `0x0054FCD9–0x0054FD1D`,
`0x0054FD3B–0x0054FD87`, caller `0x00551C20`.

#### Reattachment, placement and load boundary

Slot 131 takes a network occupant, a u32 path key and a bool allowing fallback
path selection. It requires that occupant's path interface, an available selected
path and at least two points. With fallback disabled it requests that exact path
with secondary argument zero; with fallback enabled it first requests the same
exact path and, only when absent, asks the network service for another path using
the key's type and the owner's current point. Failure returns false without
calling slot 132 or registering the selected path. Success passes occupant, key,
points and the bool to slot 132 and registers the selected path with the traffic
service, clearing the returned registration's first dword. Evidence:
`0x0054E920–0x0054EA22`; vehicle's additional front-anchor handling is above.
Null occupant is not checked by this routine before its first interface call;
callers must not treat it as a supported false-return input.

Traffic slot 132 retains the new occupant and obtains its path interface. It
uses the supplied points, or requests points for the key when omitted. Without
an interface or points it clears the selected point collection, releases the
retained path interface and resets the segment index. On a usable selection it
retains the interface, stores the key and point collection and resets the index.
Ordinary placement transforms the first two points with slot 157, obtains their
height from the occupant when the path does not supply its own height, and places
the owner at the first point facing the normalized difference. Preparation-only
placement instead clears flag 4. Network changes also perform the traffic
service's old-occupant removal/new-occupant insertion notifications; they are not
an implicit rollback boundary. Evidence: `0x00550760–0x00550B54`.

**Read does not reconstruct path points.** The traffic version-2 reader restores
references/scalars and a deque of three-byte entries. It never calls slot 131,
slot 132, the point-advancement helper or a network path selector. A successful
Read is consequently not evidence that a usable point collection now exists.
Explicit reattachment must satisfy the slot-131 protocol above before consumers
may use that collection. The animation dispatcher chooses its straight-line
handler when any of the live occupant, path interface or point collection is
missing, or flag `0x2000` is set. Evidence: `0x00554D20–0x00554E9C`,
`0x0054E0F9–0x0054E139`.

**Serialization uses the interface receiver, not the primary object receiver.**
Let `P` denote the primary traffic-automaton address and `S` its serialization
interface. `QueryInterface(0xE4FDA3D4)` returns `S=P+8`. The traffic constructor
installs the serialization vtable at `P+8`; that table's Write/Read entries are
the bodies `0x00551DE0/0x00554D20` themselves. Thus those bodies receive `S`,
without an additional subtraction. The property-interface forwarding thunk
`0x005552E0` subtracts four from its receiver at `P+12`, arriving at **the same
`S`**, not at `P`. Vehicle::Read forwarding its receiver unchanged is therefore
compatible with this inherited serialization contract. This is an established
interface-layout relationship, not a proposed fix to record bytes. Evidence:
QI `0x00505DCC–0x00505DEC`, Traffic QI delegation `0x0054D816–0x0054D81A`,
constructor `0x005544FB–0x0055450F`, vtable `0x00A9BFC4` slots 3/4,
thunk `0x005552E0–0x005552E7`, Vehicle forwarding `0x0055BD20–0x0055BD28`.

The literal version-2 stream order in automata.md §6.3 consequently has these
aliases. All offsets in the **live** column below are from `P`; the **record**
column is from `S`. Adding eight to a record-relative displacement translates
an address; it does not add eight bytes to a stream field or change its width.

| Field following the base record | Record access from `S` | Live field from `P` / meaning |
|---|---|---|
| Automaton reference, IID `0xA9B40F05` | `+0xA8` | `+0xB0`, retained leader reference used by UpdateCollision |
| First u32 | `+0x1DC` | `+0x1E4`, transport selector; constructor default 7 |
| u8 | `+0x1E0` | `+0x1E8`, saved traffic option byte; constructor zero; its operational meaning remains unestablished |
| Network reference, IID `0xA821EF94` | `+0x1E4` | `+0x1EC`, retained network occupant |
| Next u32 | `+0x1EC` | `+0x1F4`, packed live path key described above |
| Last scalar u32 | `+0x1F4` | `+0x1FC`, current segment index, preserved as a 32-bit word |
| Count and three-byte route entries | deque rooted at `+0x224` | deque rooted at `+0x22C`, route cells described above |

Constructor accesses independently confirm the live occupant/interface/points/index
at `+0x1EC/+0x1F0/+0x1F8/+0x1FC`, route deque at `+0x22C`, option byte zero,
and selector 7 (`0x00554524–0x00554542`, `0x00554571–0x005545B2`). Leader use
is at `0x0054E1D0–0x0054E263`. Record accesses are
`0x00554D9A–0x00554E11` and `0x00551E17–0x00551E64`.

The stream contains neither the retained path interface at live `+0x1F0` nor
the selected point-collection pointer at `+0x1F8`. Their absence from Read's
writes is not an offset defect. On a newly constructed owner they remain null
until another operation binds them. Conversely, Read is not a promise to clear
pre-existing geometry when applied to an already live owner: it restores the
listed fields without selecting/replacing that collection. The base reader
marks loaded state at record `+0x34`, hence live `+0x3C`, even before its version
check (`0x0050DFDB–0x0050DFF3`). Traffic Init skips its fresh-owner initialization
work when that loaded byte is set (`0x0054D7AC–0x0054D7EB`). In particular one
must not assume that calling Init repeats slot 134 to repair a loaded path.
The manager's object-list Read resolves automaton references and inserts them
into the list; the inspected loop makes no path-binding call
(`0x0050FF40–0x0050FF8F`). The animation dispatcher's missing-geometry branch
is a straight-line tick, not automatic reattachment (`0x0054E109–0x0054E139`).

**Remaining runtime observations (VH7 narrowed, not a full load closure).**

- **VH7-R1 — first post-load binding.** Load a saved, moving ordinary vehicle
  with a nonnull network reference, nonzero key and segment index, and at least
  two route cells. Capture its primary address `P` and serial receiver `S` at
  Traffic Read entry; require `S=P+8`. From Read return through city/occupant
  initialization and the first animation tick, watch writes to `P+0x1F0` and
  `P+0x1F8`. Record the **first** writer/call stack and first nonnull binding
  if different, its occupant/key/point
  arguments, the path-service result/count, any slot-131/132 calls, the segment
  index before/after them, and whether path registration occurs. This settles
  the actual later binding call and whether the saved index is preserved,
  reset or explicitly restored. If no binding occurs, record which handler
  runs with those fields null; do not report successful reconstruction.
- **VH7-R2 — saved option byte.** For the same `P`, trace reads/writes of
  `P+0x1E8` outside serialization during route start, following and route end.
  Record the first setter/consumer and its input condition or property. This
  establishes an operational name for that byte; the stream/layout alone
  establishes its address, preservation and zero constructor default.
- **VH7-R3 — route annotation producer.** At the route supplied to
  `SetRouteInfo`, record each cell's three bytes, then trace the producer's
  write of byte 2 (the third byte) before that copy. Capture the writing
  instruction/call stack and the source value across a straight route, turn,
  network transition and transfer. Follow the source's setter/enum to establish
  its meaning. Merely correlating its value with a direction or mode does not
  establish that meaning. The inspected vehicle/traffic consumers continue to
  preserve and ignore it as specified above.

**Worked load case.** For a newly constructed owner, read leader L, selector 7,
option byte `0x5A`, network N, key `0x01000203`, segment-index word 1, and route
cells `(4,5,0xA7),(5,5,0x3C)`. The required post-Read state is leader at live
`+0xB0`, N at `+0x1EC`, that key at `+0x1F4`, index 1 at `+0x1FC`, and those
two exact triples in live deque `+0x22C`; `+0x1F0/+0x1F8` remain null. For
example with `P=0x10000000`, `S=0x10000008`, the network destination
`S+0x1E4` and `P+0x1EC` are both `0x100001EC`, and deque
`S+0x224=P+0x22C=0x1000022C`. Read issues no point query. Write must reproduce
the option/annotation bytes without interpreting them. An implementation
without the binding observation must keep this case explicitly unbound
(Unsupported for a request requiring complete route geometry), rather than
fabricating points from the triples or resetting the saved index. Separately,
an explicit successful slot-131 call must follow its protocol above; that is
a placement test, not evidence for which post-load caller invokes it.

#### Available distance and route end

The traffic and vehicle slot-151 gap calculations each start with two limits of
16 and return the lesser. The path-end limit changes only when a path exists
and flag `0x10` is set. Traffic uses 3D distance from centre to the last point,
minus its radius; a result strictly below 1 becomes zero. Vehicle instead uses
3D distance from its front anchor to the last point, with no radius subtraction;
again, strictly below 1 becomes zero. Exactly 1 survives in either case.
Evidence: `0x00551300–0x005513DD`, `0x0055D9D0–0x0055DA7D`.

The leader limit changes only for an existing leader when slot 150 allows it.
Traffic uses distance from its centre to the leader's slot-66 point, minus its
own radius and 3. Vehicle forms the leader's rear point as leader centre minus
leader direction times leader radius. Let R be front-anchor-to-rear distance
and C front-anchor-to-leader-centre distance. If C is strictly less than R, the
leader limit is C minus 3; otherwise it is R minus 1. Subtract half the follower's
radius after that choice. Equality therefore selects R minus 1. There is no
final zero clamp: the result can be negative. Evidence:
`0x005513DD–0x0055147F`, `0x0055DA7D–0x0055DBD2`; the vehicle comparison,
subtractions and half-radius multiplication are verified at
`0x0055DB82–0x0055DBAE`.

The traffic path tick requests f32 distance from speed times speed modifier
times tick delta, then caps it by this available distance. Only a positive
result calls the advancement helper; its false return sets route-finished flag
4 and sends event 4 with zero data. Nonpositive motion sets flag `0x20`; a gap
strictly below minus the radius removes a nonpersistent owner (flag `0x40`
exempts it). Once route, look-ahead and movement are exhausted, the default
route-end path requests the finished-route action. At animation dispatch, a
finished route removes the owner unless slot 85 or slot 165 holds it; held
owners survive. Evidence: `0x0054FDF6–0x0054FE62`,
`0x0054FEB4–0x0054FF0D`, `0x0055004C–0x0055008B`,
`0x0054E141–0x0054E174`.

#### Vehicle speed factors

`GetSpeedModifier` starts with the inherited modifier rounded to f32: tuning
`0x00B09964`, except flag `0x80` converts a strictly positive tuning to 1 and
all other ordinary finite tunings to 0. Without a live network occupant it
returns this value. With one, define E as: occupant supports type 12, or it
supports type 2 and the network manager's elevated-position test succeeds and
world Y exceeds terrain height by **strictly more than 5**. These tests are
literal; do not apply the elevation test to every network type.

Unless flag `0x1000` is set or state equals 16, multiply by the network-type
factor table (`0x00B21CA8`): entry 2 when E, otherwise the first supported type
in ascending order 0 through 12. No supported type leaves the value unchanged.
Each such multiplication stores f32. E or state 16 then bypasses turn and slope
factors, even when flag `0x1000` prevented the network factor. That flag alone
does not bypass turn or slope factors.

For remaining owners, equal entry/exit bytes multiply by turn tuning
`0x00B21C88` and f32 `0.6`, in that order. Otherwise entry equal to
`(exit + 1) mod 4` has no turn factor. Any other entry applies the turn tuning
only when a point collection with **more than three** points exists. The turn
result is stored as f32. Flag `0x40` bypasses the slope factor. Otherwise let
Yd be direction Y, A the slope-angle tuning `0x00B21CF8`, and T the slope tuning
`0x00B21CFC`. Compute asin(Yd), multiply by T, divide by A, and clamp the result
to [0,T]. For Yd positive return modifier times (1 minus that value); for Yd
negative return modifier times (1 plus that value); for zero return modifier.
The division is not protected against zero A. **Quirk:** asin uses signed Yd,
not its absolute value, so ordinary positive A/T produce a zero clamped value
on a downhill direction, rather than a downhill speed bonus. Evidence:
`0x00505AB0–0x00505AE5`, `0x0055DF50–0x0055E1E5`; numeric instruction checks
at `0x0055DFF8–0x0055E009`, `0x0055E027–0x0055E0EC` and
`0x0055E0FD–0x0055E1C9`. Runtime tuning values are inputs, not values read from
the PE's zero-initialized globals.

| Additional geometry conformance setup | Required observation |
|---|---|
| Route cells (4,7,b), (4,7,c), with differing preserved third bytes | Derived next exit 2 for both b/c choices |
| K=1, Q=3, M1=3, M2=3; eligible candidate without flag `0x2000` | Reject despite shared bit 1 |
| K=1, Q=3, M1=3, M2=1; otherwise eligible | Accept through the second equality |
| Arbitrary distinct K/Q and M2=0; otherwise eligible | Accept through zero-mask equality |
| Valid interface but selected path has one point | Slot 131 returns false; no slot-132 placement or registration |
| Successful traffic Read on an owner without a point collection | No network geometry query or newly constructed point collection during Read |
| Traffic centre zero, last point (5,0,0), radius 4; flag `0x10`, no leader | Gap 1; moving last point to (4.5,0,0) makes gap 0 |
| Vehicle front anchor zero, last point (1,0,0); flag `0x10`, no leader | Gap 1, independently of vehicle radius |
| Vehicle anchor zero, follower radius 4, leader centre (10,0,0), direction (1,0,0), radius 2; slot 150 true | Leader limit 5 |
| Same, leader radius zero | C=R=10; leader limit 7, not 5 |
| Same, leader centre (1,0,0), radius zero | Leader limit −2; do not clamp to zero |
| No endpoint or leader limit active | Gap 16 |
| Non-E network, flag `0x1000`, state not 16, equal entry/exit, inherited modifier 1, turn tuning 0.5, level direction | Modifier f32 0.3 (the network factor alone was skipped) |
| E true, no exemptions, inherited modifier 1, table entry 2 = 0.8, equal entry/exit, nonzero slope | Modifier f32 0.8; no turn or slope factor |
| State 16, inherited modifier 1, arbitrary supported network and slope | Modifier 1; no network, turn or slope factor |
| No E/state/turn exemptions, network factor 1, no turn factor, A=1, T=0.5, Yd=−0.5 | Modifier 1; signed downhill value clamps to zero |

### 12.10 Player-drive commands and signed speed (VH1)

These rules refine §3. Distances are metres and tick input `dt` is milliseconds.
`f32(x)` denotes a stored IEEE binary32 value; constants below are binary32 unless
explicitly called double. Do not replace milliseconds by seconds throughout the
algorithm: command accumulation uses `ds = f32(dt × f32(0.001))`, whereas speed
integration uses `dt` directly. The acceleration accumulator and the speed target
are separate persistent values. The current speed is the inherited speed tween's
current value, not necessarily either of them.

The drive state is obtained through slot 56; drive parameters through slot 67,
first its direct parameter reference (`+0x64`), otherwise its service (`+0x0C`).
Missing parameters make command translation return false without forwarding to
Automaton translation. The parameter fields used by this owner are:

| Parameter offset | Meaning |
|---|---|
| `+0x08`, `+0x0C` | Forward upper bound and signed reverse lower bound |
| `+0x10`, `+0x14` | Continuing-throttle and braking/change-direction rates |
| `+0x18`, `+0x1C` | Off-network speed multiplier and coasting deceleration |
| `+0x20` | Steering limit with flag `0x2000` |
| `+0x24`, `+0x28` | Steering rate at zero/full measured speed with that flag |
| `+0x2C`, `+0x30` | Steering rate without that flag, and return-to-centre rate |
| `+0x4C`, `+0x5C` | Train overspeed duration scale, and tick suspension threshold |

#### Vehicle command translation

Forward/back and positive/negative steering commands have elapsed-time values
at drive-state `+0x7C/+0x80` and `+0x84/+0x88`. A value **at least zero** is
active. In particular a newly pressed command with value zero already affects
translation. Use the continuing-throttle rate iff current speed is positive and
forward is active, or current speed is negative and back is active; otherwise
use the change-direction rate. Forward adds `ds × rate` to the acceleration
accumulator (`+0x28`); otherwise back subtracts it. With neither pedal active,
reset the accumulator to zero. There is no accumulator clamp. Forward wins simultaneous pedals, but
rate selection still considers both commands and the sign of current speed.

With flag `0x2000`, steering rate is the linear interpolation from parameter
`+0x24` to `+0x28`, at `clamp(measuredSpeed / forwardBound, 0, 1)`; measured speed
is the automaton's `+0x94`, **not** drive-state `+0x94` (a command timer).
Without the flag, use parameter `+0x2C`. The receiver is verified at
`0x0055C707–0x0055C70D`.
Negative steering is processed first: active subtracts `ds × rate`; absent
returns a negative steering value toward zero by `ds × returnRate`, stopping
at zero. Positive steering is then processed: active adds `ds × rate`; absent
returns a positive value toward zero by the same return rate, stopping at zero.
This ordering matters when a command and automatic centring both affect one
tick. Finally clamp steering (`+0x5C`) to the signed parameter limit with flag
`0x2000`, or to ±`bits(0x3FC90FDB)` (π/2) without it. Forward to the inherited
Automaton translator **after** these owner changes. Evidence:
`0x0055C600–0x0055C8A3`; clamp instructions `0x0055C82E–0x0055C89C`.

#### Vehicle speed update

The helper takes drive state, parameters, permission to begin reversing, an
on-network boolean, and `dt`. Let `old` be state speed `+0x2C`. Start with the
parameter speed bounds; off-network multiply both by parameter `+0x18` (the
forward bound is stored binary32). Integrate `old + dt × accumulator` and clamp
to the forward upper bound. The lower bound is the signed reverse bound only
when `(old < 0 and reverseEnabled)` or `(old == 0 and permissionToBeginReverse)`;
otherwise it is zero. `reverseEnabled` is state byte `+0x228`. Thus a positive
speed cannot cross straight into reverse in one integration.

If **both** pedal timers equal exactly −1, apply coasting deceleration
`dt × f32(0.001) × parameter(+0x1C)` toward zero, stopping at zero. Other negative
timer values are inactive commands but do not satisfy this coasting test. Save
this result, rounded to binary32, at state `+0x2C` **before** grade adjustment.

For grade adjustment let `y` be the automaton direction's vertical component,
`h = abs(y)` and `s` the saved speed. Only `h > f32(0.2)` enters this adjustment.
When `(s < 0) == (y < 0)`, reduce `s` by the following factors:

| Location | Grade factor |
|---|---|
| On network | `(4 − h) × 0.25` |
| Off network, `h <= f32(0.8)` | `(2 − h) × 0.5` |
| Off network, `h > f32(0.8)` | Zero |

Then, for every grade-adjusted case, add
`h × y × f32(−0.0003) × dt`. For family `0x4202`, suppress a result that reverses
the saved speed's sign (positive-to-negative or negative-to-positive). A disabled
reverse byte also suppresses any final negative value. Suppression means zero.
The helper's return/tween value can therefore differ from state `+0x2C`.
When the result differs from the speed tween's existing target, retarget it
from its current value and zero both tween time fields; an equal target leaves
the tween untouched. Evidence: `0x0055EA20–0x0055ED00`, specifically pre-grade
store `0x0055EB69` and grade/tween instructions `0x0055EB6C–0x0055ECF8`.

Conformance vectors (all omitted commands inactive, grade zero, reverse enabled;
parameters and random/service inputs must be supplied by the test):

| Setup / operation | Required observation |
|---|---|
| `dt=1000`, accumulator 0, positive current speed, forward timer 0, rates `1/1024` and `1/512` | Accumulator becomes `1/1024`; zero elapsed time is active. |
| Same, negative current speed, both pedals active | Forward wins, but the continuing rate still applies because back is active. |
| No pedals active, previous accumulator nonzero | Accumulator becomes zero. |
| Steering `−1/8`, `ds=1`, rate `1/4`, return rate `1/8`, negative command active only | Negative step gives `−3/8`; positive-side centring does nothing. |
| Steering `−1/8`, same rates, positive command active only | Negative-side centring reaches zero first; result `1/4`, before final clamp. |
| Positive old speed, large negative acceleration, permission to reverse true | Clamp to zero; next update starting at zero may use the reverse bound. |
| Old speed −1, accumulator 0, reverse disabled, grade zero | Final speed zero even if permission to begin reverse is true. |
| Raw speed 1, `y=0.5`, on network, `dt=0` | Save raw speed 1; return/tween target `0.875`. |
| Raw speed 1, `y=0.5`, off network, `dt=0` | Save 1; return `0.75`. |
| Raw speed 1, `y=0.9`, off network, `dt=0` | Save 1; return zero. |
| Raw speed positive, `y=−0.5`, `dt=0` | No multiplicative reduction; sign comparisons differ. |
| Result equals existing tween target | Preserve both tween time fields; do not restart convergence. |

### 12.11 Train composition, following and trip start (VH5)

The family/count draws and cumulative constructor draws are already specified
in §12.3. Car length is each created occupant's slot 17, not a train-wide tuning
constant. The head passes its colour byte to creation; resource/model selection
within the requested family belongs to the vehicle manager's creation contract
([automata-managers.md](automata-managers.md) §5), not an additional train random
draw. Retained car identities and teardown are §12.8.

#### Target count and incremental creation

Setting the target count (slot 195, `0x005577F0`) keeps only its low byte. If it
is less than the current count, remove cars **from the tail**, calling each
non-null car's Remove(false) and Release before shrinking the list. Store the
new target even when no removal is needed. Increasing the target does not
create anything immediately. Animation first runs Vehicle animation, refreshes
an invalid cached tail cell/network/path from the last car (head if empty), and
attempts at most **one** missing car per tick. Cached-cell invalidity tests x or
z less than zero. Following is skipped in state 20, but this creation attempt
precedes that test. Evidence: `0x00558D20–0x00558DB3`, `0x00557BF0–0x00557CED`.

For one-car creation (slot 199, `0x0055A8B0`), previous means the last car or the
head if empty. With previous length `L`, require
`distance(previousRear, cachedSpawnPoint) − L × 0.5 >= 2 × L`.
Equality permits creation. The family must be nonzero and the head's manager
(slot 174) available. Normally use the selected car family. For freight family
`0x4203`, the car that will reach the target count uses family `0x421A` instead.
Manager slot 15 receives family, cached point, the head's `+0x84` value, a zero
direction vector, zero, head colour byte, zero. A null result or failure to query
`0x4A416254` returns false without appending. A successful queried reference is
released locally after the list has retained its own reference.

Before appending, clear the new car's flag 2, set the previous car/head as its
leader (slot 49, SetLeader), MoveTo the cached point with the previous direction, and place it
through slot 132 with cached network/path, null point collection and true.
If the head is in state 16, push state 16 on the car with timeout −1 and other
arguments zero. No fill-to-target loop is permitted. Evidence:
`0x0055A8B0–0x0055AAC7`; spacing comparison `0x0055A935–0x0055A940`.

#### Following geometry

Slot 198 (`0x0055A050`) places cars in list order, with **two anchor searches per
car**. Front/rear anchors are `+0x264/+0x2BC`; saved direction is `+0x270`.
The initial reference point is the head's rear. The front spacing is
`f32(previousLength × 0.5 + f32(carLength × 0.5))`; the rear spacing is car length.
Search the head's current path from its last segment backward, then the saved
network/path deque from newest to oldest, obtaining each earlier point collection
from network slot 49 and its path-interface slot 3. Continue the current segment
cursor across searches, rather than restarting it for each anchor. A successful
candidate becomes the next search's reference point. The current segment's
forward orientation remains earlier endpoint → later endpoint.

For a segment `A→B`, reference `C` and spacing `r`, find intersections of
`A+t(B−A)` with the sphere of centre C and radius r, keeping **0 <= t <= 1**.
Both endpoints are included. Reject a segment when all three component
differences have magnitude less than `f32(0.0001)`. The quadratic uses stored
binary32 segment differences and coefficients `a = dot(B−A,B−A)` and
`b = 2 dot(B−A,A−C)`, discriminant `b² − 4a(dot(A−C,A−C)−r²)`.
Negative discriminant yields no root. Roots are stored binary32:
`tPlus=(sqrt(discriminant)−b)/(2a)`, `tMinus=(−b−sqrt(discriminant))/(2a)`.
A tangent is still reported as two roots when both parameters are in range.
The primary output is the plus root if valid, otherwise the minus root. With
two roots, try the minus root **first**. An acceptable point P requires
`length(C−P) > f32(0.00001)` and
`dot(normalize(B−A), normalize(C−P)) >= f32(0.1)`.
The same dot threshold is used on the front and rear legs; do not use nearest
distance alone to choose the intersection. Evidence for arithmetic, closed
endpoints, root ordering and tests: `0x0055BB10–0x0055BCE5`,
`0x0055CFD0–0x0055D201`; spacing/search `0x0055A221–0x0055A484`.

The candidate output is persistent scratch during one following call, initially
the runtime unset scalar (`0x00B22964`) in every component. A sphere miss leaves
it unchanged; orientation rejection may leave the primary root in it. On a
failed front search the owner still copies this scratch into the front anchor.
On a failed rear search it uses `front − carLength × oldCarDirection` for the
rear anchor. The subsequent reference is the search scratch, including on that
rear fallback; do not silently substitute the fallback rear. Once history has
been exhausted, later legs do not start a fresh traversal. For the first exhausted
leg the search stops when the reference differs from the earliest endpoint by
at least `f32(0.0001)` in any component, or its distance exceeds the spacing;
otherwise traversal merely becomes inactive. There is no successful-placement
boolean returned by this void owner helper. These are compatibility quirks for
incomplete geometry, not a guarantee of valid coordinates.

After both anchors, place the car on the selected network/path/point collection
through slot 132 with true. Let D=front−rear. If its length is below
`f32(0.00001)`, use saved direction unless it is componentwise within
`f32(0.0001)` of the unset vector or also shorter than that length threshold;
otherwise use the head direction. Centre is `front − D × 0.5` **before**
normalizing D; MoveTo with normalized D and save that direction. Set each car's
leader through slot 49 to the preceding car, or the head for the first car,
then use this car as the preceding object for the next list entry. This rebuilds
the leader chain; it does not copy speed. In a complete consist, when the last car leaves a
different non-null previous network, query that old network's cell and notify
the newly selected path interface's slot 39 with `(head, cellX, cellZ)` before
placing the car. Evidence: `0x0055A53E–0x0055A894`.

TrainCar slot 132 first performs Vehicle placement. For the classification byte
set by §12.3 and a resulting non-null network, query IID `0x4A10AFC2` (the
[underground network interface](network-occupants.md) §3). On success, its slot 99
determines opacity: result 2 requests `(target 1, duration f32(0.1))` through
automaton slot 33; any other result requests `(0, f32(0.6))`. Release the query
reference. Unclassified cars, no network or failed QI add no opacity request.
This override changes visibility, not the chosen path or car length.
Instruction evidence: `0x00557320–0x005573BC`.

Leader-chain vector: after creation or a following pass, a head H with cars
A, B, C has leaders `A→H`, `B→A`, `C→B`, regardless of their current speeds.
Evidence: creation `0x0055AA26–0x0055AA2D`, following
`0x0055A83F–0x0055A861`; inherited slot name [automata.md](automata.md) §11.

#### StartTrip and the independent rail-accident check

Train StartTrip has three inputs: occupant, an inherited trip argument, and a
forward-start boolean. The second argument is forwarded unchanged. Obtain the
occupant's two cell coordinates through its slot 59, then call traffic simulator
slot 17, CheckRailAccident(cellX,cellZ). If true, call the head manager's
CreateAccident with the head's occupant interface, kind 2 and duration 20000 ms.
When the message service exists, also post standard message `0x0C05E4CD`, setting
its first cell coordinate through message slot 11 and second through slot 16;
send through service slot 4 with zero and release the message. These effects
occur **before** the inherited Traffic StartTrip, even if it subsequently
returns false. A false inherited result returns false without train geometry.
A true result with no point collection or with flag `0x2000` returns true without
train geometry. The check itself belongs to
[traffic-simulator.md](traffic-simulator.md); this owner performs no extra random
draw for it. Evidence: `0x005588D0–0x005589CA`; service binding/target verified at
`0x00AB33D0` slot 17 → `0x00711590`.

For an ordinary successful start, false forward-start selects the last path
point as front and path index `count−2`. True forward-start with positive path
distance `+0x260` first reduces it to `min(distance, slot151())`, then calls
slot 133 from the first point to derive the front and path index. With
nonpositive distance it uses the path interface's slot 19 point query instead.
Derive the rear using slot 182 with front, rear output, head length and the saved
path-history deque. For a rear with no NaN component, D=front−rear. If any rear
component is NaN, use the path's last point for front and
`D=normalize(last−first) × headLength`; the native normalization has no zero-length
guard in this fallback. Infinity alone is not the NaN test.
Centre is front−D/2. MoveTo with normalized D only when `length(D)>f32(0.00001)`;
otherwise pass a null direction. Save D after that optional normalization and
clear flag `0x20`. Return the inherited result. There is no rollback of the
earlier accident/message. A missing required simulator or failed message
allocation is not a graceful false-return path in the original. Evidence:
`0x005589D0–0x00558D1B`.

#### Animation failure and random accounting

Flag `0x20` clear resets blocked seconds to zero. While set, only a speed-modifier
slot-125 result **greater than** `f32(0.01)` increments that timer by
`dt × f32(0.001)`. At timer >=10 seconds, draw a full unsigned 32-bit value and
Remove(false) iff it is **less than `0x7FFFFFFF`**. A rejected draw leaves the
timer high, permitting another draw next eligible tick. These instructions are
`0x00557CF3–0x00557D55`. This is removal, not a sound draw. The inspected Train
animation and player-drive routines do not contain the previously claimed
random horn/sound selection; their additional random draws are the overspeed
range, derail test and collision perturbations (§12.4). Sound service behaviour
is [audio.md](audio.md); do not invent a per-car sound draw here.

| Train conformance setup | Required observation |
|---|---|
| Three retained cars, set target `0x102` | Target 2; remove/release old tail once. |
| Target 5, count 0, enough space for many cars | One creation attempt in one animation tick. |
| Previous length 10, spawn separation 25 | Creation allowed; separation just below 25 rejects. |
| Family `0x4203`, target 3, count 2 | Request `0x421A`, same head colour; no train model-selection draw. |
| Manager returns object whose TrainCar QI fails | Return false; list unchanged. |
| Straight path increasing x, head rear x=20, head/car lengths 10 and 6 | First car front x=12, rear x=6, centre x=9, direction `(1,0,0)`. |
| Segment x=0..10, reference x=5, radius 2 | Two roots x=7 and 3; choose x=3 by forward-orientation test. |
| Segment x=0..10, reference x=12, radius 2 | Accept endpoint x=10. |
| Identical segment endpoints | Reject before quadratic division. |
| Classified TrainCar, underground query returns category 2 / other | Opacity request `(1,0.1)` / `(0,0.6)`, after Vehicle placement. |
| Accident predicate true, inherited StartTrip false | Accident `(head,2,20000)` and cell message precede false return. |
| Predicate false, inherited StartTrip false | False, no accident/message and no train geometry. |
| Blocked timer 10, speed modifier >0.01, draw `0x7FFFFFFE` / `0x7FFFFFFF` | Remove / retain; equality does not remove. |
| Blocked flag clear or speed modifier exactly f32(0.01) | Clear timer / retain timer without increment or draw. |

### 12.12 Train player-drive speed and derail (VH1)

Train uses its own translator (`0x005570C0`), calling Automaton translation first.
Forward active adds `ds × parameter(+0x10)` to the acceleration accumulator;
otherwise back active subtracts `ds × parameter(+0x14)`; neither active resets
it to zero. It does not select Vehicle's rate by the sign of current speed.
The player-drive tick (`0x005591A0`) first calls the inherited Automaton tick,
preserving its return. With drive parameters available, a drive-state damage
counter (`+0x60`, unsigned) greater than parameter `+0x5C` suspends the remaining
train update and returns true. Otherwise integrate the state target speed as
`clamp(oldTarget + dt × accumulator, 0, forwardBound)`.

Let M be the forward bound and T the overspeed timer (`+0x2F4`, milliseconds).
Current tween speed >=M **or** an existing overspeed latch (`+0x2F8`) enters
overspeed processing. A prior positive T together with a **strictly positive**
back timer cancels overspeed: clear T/latch and set the target to
`clamp(M + dt × accumulator, 0, M)`. A freshly pressed back timer of zero does
not cancel it. Otherwise add dt to T, set the latch, draw a double uniformly
between −a and +a, and store target
`f32(M − double(0.1) × M + draw)`, where
`a = f32(f32(T / parameter(+0x4C)) × cachedAmplitude)`.
The cached amplitude is initialized **once for the process** to
`f32(double(0.1) × M)` using the first train to reach this initialization;
later trains do not recompute it from their own M. Below M with no latch,
clear T/latch. Report intensity to AEMS slot 9 as T divided by
`f32(parameter(+0x4C) × 0.5)`, or zero if that denominator is nonpositive.
Evidence: `0x0055923A–0x0055940E`, cache `0x00B22958/5C`.

Only when T is exactly zero does this owner immediately retarget the speed tween
(duration zero). Let C be current tween speed and Q the newly stored state target:

| Comparison | Immediate tween target |
|---|---|
| C<Q | `max(0, min(Q, f32(C + dt × parameter(+0x10))))` |
| C=Q | C |
| C>Q; N=`f32(C − dt × parameter(+0x14))` and N>=Q | Q |
| C>Q; N<Q and N>0 | Zero |
| C>Q; N<Q and N<=0 | N |

The last two rows are literal compatibility behaviour, not an ordinary braking
clamp. During positive T, the jittered state target does not itself retarget
the tween here. Evidence: `0x00559410–0x00559466`, `0x005595B2–0x00559628`.

Derail checking (`0x00557680`) requires the overspeed latch and at least one car.
Compare the head direction with the fifth car if there are at least five cars,
otherwise with the last car. Form their direction dot product by adding z and
y products first, then x. Only dot <`f32(0.05)` consumes a
RandomDoubleUniform draw, and only a draw strictly less than the **double**
`0.10000000149011612` triggers the curve derail. This is a fixed threshold,
not a speed-scaled probability. A trigger calls collision slot 65 with self and
a zero direction, sets flag `0x08000000`, and emits slot 102 `(11,true)`.

Independently, nonzero T greater than parameter `+0x4C` reports the runtime
value at `0x00B22960` through AEMS slot 9 and repeats the same collision/flag/event
sequence. Equality does not trigger this duration branch. Both branches can
fire during one check; it is not an else-if or a single combined random draw.
The check does not clear the overspeed latch. Evidence:
`0x00557680–0x005577E0`, threshold constant `0x00A9C7E8` read as **64 bits**.

Train movement follows the front/rear and saved-path contracts in §12.11.
For nonnegative speed and elapsed time, the head divides this tick into
`N=clamp(ceil(currentTweenSpeed×dt),1,6)` movement steps, rather than Vehicle's
time-based count below. Each step's distance is
`f32((dt/N)×f32(currentTweenSpeed))`. The count uses the x87 product before a
binary32 speed store; the distance uses the subsequent binary32 speed sample.
The ceil conversion is the rounding operation described in §12.15; the final
clamp compares unsigned integers. Evidence: `0x0055971A–0x005597D6`.

Each head step advances the front anchor through slot 133. If advancement
fails, next-player-path slot 197 must succeed; otherwise call slot 13 with
false and return true immediately. A successful step gets the rear anchor
through slot 182, using length and the saved-path deque. Form D=front−rear.
Only when its length is below `f32(0.00001)` substitute saved direction
`+0x270`; if that is componentwise within `f32(0.0001)` of the unset vector,
or its length is also below the length threshold, return true immediately.
There is no head-direction fallback here. Centre is `front−D×0.5`, computed
**before** normalizing D. MoveTo with that centre and normalized D, save D,
clear flag `0x20`, and copy the rear anchor and D into drive-state
`+0x44/+0x50/+0x38`. This head rule differs from the car-following fallback
in §12.11. Evidence: `0x005597F3–0x00559A29`.

Then query collision slot 190 with an initially unset direction and fraction
1. A hit stops the remaining steps; call Vehicle ResolveCollision only if
the parameter service was obtained. Otherwise continue all N steps. After
motion/collision, update the dashboard transform and position/heading/velocity
outputs, then check derail. Finally set the automaton's measured speed
`+0x94` to actual 3D centre displacement divided by dt, and return the
inherited Automaton tick result. Early exits above skip these final updates.
Evidence: `0x00559A2C–0x00559C25`. The dashboard is an interface output;
it is not the source of the train's integrated target speed.

The next-player-path helper (`0x00559C30`) obtains the adjoining network using
the opposite exit `(exit+2) mod 4` and current path-type nibble. Missing
adjoining network returns false. A connected path can be used directly when
its first point lies at most **2 metres** from the current path's last point.
Otherwise request candidate paths with the direction-selection outputs of
slot 171, current last point and distance limit `f32(0.5)`; choose the first
candidate if any, or use the path service's fallback selection with wildcard
`0xFF`. A null selected point collection returns false. Success calls trip
slot 131 with the new network, selected path key and true, notifies
network-service slot 38 of the adjoining
network's cell, and also notifies the further network's cell if that lookup
succeeds. This path switch does not itself move the centre or add another
collision check. It assumes a usable current point collection; do not turn
that precondition into a promise of safe empty-path reconstruction.
Evidence: `0x00559C3E–0x00559E6E`, endpoint comparison
`0x00559CD6–0x00559D10` (constant `0x00A8825C`, binary32 2).

| Train player-drive conformance setup | Required observation |
|---|---|
| T>0, latch set, back timer 0 / positive | Continue overspeed / cancel it. |
| First cache initialization with M=1, later train M=2 | Both use cached f32(0.1) amplitude; the second baseline is still its own `2−double(0.1)×2`. |
| T=0, C=1, Q=0.75, `dt × brakingRate=0.125` | Immediate tween target 0.75. |
| T=0, C=1, Q=0.75, `dt × brakingRate=0.5` | Immediate tween target zero, not 0.5. |
| Latch clear, any directions | No curve draw. |
| Latch set, no cars | No curve draw. |
| Six cars, fifth direction opposed to head, sixth aligned | Compare fifth car; a curve draw is eligible. |
| Dot exactly f32(0.05) | No curve draw. |
| Eligible dot, draw exactly double threshold / just below it | No curve derail / derail. |
| Curve triggers and T exceeds duration limit | Observe two collision/flag/event sequences. |
| T equals duration limit and curve fails | No duration derail. |
| Speed 0, dt=33 | One zero-distance movement step. |
| Speed 0.125, dt=16 | Two steps, distance 1 each. |
| Speed 0.125, dt=64 | Six steps; distance f32((64/6)×0.125) each. |
| Front (8,0,0), rear (4,0,0) | Centre (6,0,0), direction (1,0,0); halve the unnormalized span. |
| Coincident anchors, saved direction unset | Return true before MoveTo, dashboard and derail check. |
| Advance fails and next-player-path fails | Slot 13(false), return true; no remaining steps or final dashboard/derail update. |
| Collision on step 2 of 6 | No steps 3–6; resolve once if parameters exist, then final dashboard/derail work. |
| Connected next path endpoint separation 2 / greater than 2 | Direct connected-path acceptance / candidate-or-fallback selection. |

### 12.13 Tank player drive, firing and interface boundary (VH1)

The record wrapper and two saved model-index **bytes** are specified in
[automata.md](automata.md) §6.4.3 (AT31); the primary-view aiming fields below are separate from the
serialization-view fields at similar numeric offsets.

Tank Init calls inherited Vehicle Init before its owner model setup. Initial
turret yaw is zero, barrel elevation `f32(0.2)`, shot power 65 and owner activity
field `+0x340` zero. Yaw/elevation/power fields are `+0x34C/+0x350/+0x354`;
these are not speed defaults. The owner loads the named `tankshot` /
`fx_00_tankshot` effect and turret/barrel model handles. Entering player drive
configures gauges `[0,f32(0.4),elevation]`, `[0,2π,2π−yaw]` and `[30,100,power]`;
sets channel-7 action limit 12, action count zero, key `0xAC0ECE77`, and repeat
delay 2000 ms. Evidence: `0x0054C880`, `0x0054A780`.

The translator sets owner activity byte `+0x340` true. When slot 18 is true,
only Vehicle translation follows. Otherwise the owner performs the following
aiming, firing, adjustment and gauge operations before Vehicle translation.

Yaw is first brought into the **inclusive** interval `[0,P]`, where
`P=bits(0x40C90FDB)` (binary32 2π), by repeated subtraction/addition of P.
An existing yaw equal to P remains P. Aim command timer `state+0x9C >=0`
requests unprojection of integer screen coordinates `+0x214/+0x218` through
the view interface, slot 36, with final argument zero. Failure leaves yaw
unchanged. Let U be the normalized horizontal vector from body position to
the returned world point and V the normalized horizontal body direction.
Compute `A=f32(acos(V·U))`; if `U.z×V.x−V.z×U.x >0`, replace A by
`f32(P−A)`. The dot-product helper is acos: the export's TransformDirection
name is misleading (`0x009F0090`, acos instructions `0x009F00C8–0x009F0118`).
These normalizations have no zero-length guard or dot clamp in this owner.

Let `d=A−yaw` and `step=clamp(d,−r×dt,r×dt)`, with
`r=bits(0x3B09421E)` radians/ms. If `abs(d)>bits(0x40490FDB)` (binary32 π),
subtract step from yaw; otherwise add it. Store the result binary32. There is
no second wrap in this translation, and the step magnitude is bounded using
`abs(d)`, not the smaller wrapped angular distance. Evidence:
`0x0054B074–0x0054B33D`.

The power-cycle command (`+0xA0`) acts when its timer is **at least 250 ms**:
reset that timer to zero, add 5 to power, wrap a result **greater than 100**
to 30, and clamp to `[30,100]`. This occurs before firing. After firing, apply
the adjustment pairs below in table order. Each acts only if its current
timer is nonnegative and `current−history >0`. Add/subtract that difference
times the binary32 rate, then clamp/store the owner's value before processing
the next pair. Thus simultaneous adjustments can differ from a single net sum.

| Current/history timer | Adjustment | Bounds |
|---|---|---|
| `+0x90/+0xC4`, then `+0x94/+0xC8` | Elevation + then −, rate `bits(0x3983126F)` radians/ms | `[0,f32(0.4)]` |
| `+0xA8/+0xDC`, then `+0xAC/+0xE0` | Power + then −, rate `f32(0.02)` per ms | `[30,100]` |

The shot therefore uses the power before these last two adjustments; its
barrel transform is the model's current transform. Gauge values become
`[elevation,P−yaw,power]` at state `+0x250/+0x25C/+0x268`, and owner
`+0x344/+0x348` retain binary32 screen coordinates. Two process-wide sound
timers, one each for yaw/elevation, accumulate dt while **at most 50 ms**;
only a later call seeing a timer **greater than 50** resets it and evaluates
the current call's absolute angle change. Yaw change >`f32(0.0017453292)`
starts AEMS event 12, otherwise stops it; elevation change >`f32(0.004)`
starts event 15, otherwise stops it. This is not accumulated angular change
over the timer interval. Evidence: `0x0054B4AB–0x0054B7AB`.

Tank's tick calls Vehicle's tick first and normally preserves its return.
With a drive state, both pedal timers exactly −1, and either no current
network or reverse-enable byte true, it can pivot in place. Missing
parameter service/AEMS in that eligible branch returns false. A steering
timer different from −1 selects the pivot: negative-steering timer wins;
otherwise use the positive-steering timer with opposite sign. Let θ be the
selected timer times parameter `+0x58` (negated for positive steering), and
reset that selected timer to zero. Start AEMS event 19. For body direction D,
normalize `D×(D.z,0,−D.x)` and form a quaternion with that axis,
`sin(−θ/2)` and `cos(−θ/2)`; apply its row-vector rotation to D and store
the resulting direction. For a horizontal D this is
`(D.x cosθ+D.z sinθ,0,D.z cosθ−D.x sinθ)`.
There is no stationary-speed test: pedal inactivity is the criterion.
Rebuild front/rear anchors as `position±0.5×length×D`, copy D into both
drive-state direction vectors, MoveTo at the unchanged position and refresh
slot 100 with `(0,true)`. No collision check is added by this pivot branch.
With active pedals or no steering timer, stop event 19. A missing drive
state, or the eligible pedal condition on a network with reversing disabled,
returns the inherited result without the pivot. Evidence:
`0x0054AAE0–0x0054AC86`, quaternion/anchors `0x0054AC86–0x0054AFF7`.

Firing requires fire timer `+0x98 >= 0`, a non-null barrel model (`+0x360`),
and channel-7 ready byte (`state+0x1FF`) true. On firing, zero channel elapsed
`+0x1E0`, clear ready, and increment action count `+0x178`. Obtain the muzzle
origin from the barrel transform translation through model-reg-effects. The
projectile effect receives the following interface calls:

| Effect operation | Payload |
|---|---|
| Slot 11 | Attribute 15, `currentSpeed × bodyDirection`, true |
| Slot 12 | Attribute 0, pointer to **stored f32(power × f32(0.01))**, true |
| Slot 3 | Start with zero |

The missing 0.01 scale in some pseudocode is not an alternate convention: the
instructions explicitly multiply and store binary32. CurrentSpeed is the
Vehicle speed tween's current value; direction is the body direction, not a
screen-space target or the turret direction. Model-reg-effects supplies the
barrel/muzzle transform separately. The tank owner neither picks a hit occupant
nor calls a damage method in this firing routine. Pass the effect attributes
and transform to the [effects interface](effects.md); effect asset behaviour
and the damage consumer are separate from these owner calls. Do not implement
an invented direct damage amount from unscaled power. Evidence:
`0x0054B33D–0x0054B4A8`, including the power scale/store and effect virtual calls.

Vehicle translation then reaches Automaton translation (`0x00509070`), which
updates command history and channel readiness. For channel 7, while count is
below its limit (or limit is −1), a false ready byte first tests elapsed against
delay: below delay adds dt; at or above delay sets ready and resets elapsed to
zero. It does not set ready on the same tick that addition first reaches the
delay. The tank consumes the previous ready value before this inherited
update. At count >=12, inherited translation keeps ready false. Thus continuous
fire cannot bypass the action limit or the readiness phase by using a large dt.
The effect's own lifetime is not this 2000 ms cooldown.

Tank animation (`0x0054B7C0`) applies turret/barrel orientation to model instances,
updates model-reg-effects, and uses the owner drawing helpers for those parts.
Renderer allocation, model transforms, lighting and releases remain interface
calls; the automaton's principal model and the projectile effect are separate
resources. Shutdown (`0x0054A850`) owns cleanup of the part drawings and their
references in addition to inherited Vehicle shutdown. IO is the inherited Vehicle
record followed by the two owner dwords in [automata.md](automata.md) §6.4;
do not serialize a renderer pointer or active projectile to replace those fields.

| Tank conformance setup | Required observation |
|---|---|
| Fresh successful Init | Yaw 0, elevation f32(0.2), power 65. |
| Fire timer zero, barrel present, ready true, power 65 | Fire once; attribute 0 is f32(65 × f32(0.01)); count increments, elapsed resets, ready clears. |
| Same, ready false or barrel absent | No projectile Start and no owner count increment. |
| Current speed 0.125, body direction `(1,0,0)` | Effect attribute 15 vector `(0.125,0,0)`, independent of turret yaw. |
| After a shot, elapsed 0, dt=2000 | Inherited phase increases elapsed to 2000 but does not make ready true yet. |
| Next held-fire translation with elapsed 2000, ready false | No shot in owner phase; inherited phase makes ready true and resets elapsed. |
| Action count already 12 | Inherited phase cannot re-arm even after 2000 ms. |
| Power 100, cycle timer 249 / 250 | Remain 100 / become 30 and reset timer. |
| Power 65, fire ready, power-increase difference 100 ms | Shot attribute uses 65×0.01; final gauge power is f32(65+100×f32(0.02)). |
| Elevation f32(0.4), both elevation differences 100 ms | Increase clamps first; final elevation f32(f32(0.4)−100×f32(0.00025)). |
| Adjustment current timer −1, or current equals history | No adjustment for that command. |
| Yaw exactly P, aim inactive | Preserve P; yaw gauge is zero. |
| Sound timer 50, dt=1 | Accumulate to 51; no sound decision yet. |
| Sound timer 51, yaw/elevation change exactly its threshold | Reset timer; stop the corresponding event. |
| Pivot eligible, horizontal D=(1,0,0), negative-steering timer t | Direction (cosθ,0,−sinθ), θ=t×parameter(+0x58); position unchanged; selected timer becomes zero. |
| Both pedals inactive, current network present, reversing disabled | No pivot and no additional MoveTo. |

### 12.14 Concrete-owner method and interface inventory

Primary slots below are zero-based, four bytes per slot. These are owner
contracts, not proposed C ABI additions. State numbers, dispatch/message entry
and inherited tick ownership remain §12.6 and [automata.md](automata.md) §6.5/§11.

| Owner / primary table | Owner methods and primary slots |
|---|---|
| Vehicle, `0x00A9D488` | QI 0=`0x0055B660`; Init 3=`0x0055B9B0`; Shutdown 4=`0x005602F0`; drive command intake 60=`0x0055B6C0`; collision 65=`0x0055C8C0`; animation 72=`0x0055BA20`; debug 93=`0x00562AC0`; state 109=`0x0055CA90`; entry 110=`0x0055B6D0`; player tick 115=`0x005603F0`; commands 121=`0x0055C600`; speed modifier 125=`0x0055DF50`; path tick 129=`0x0055CAF0`; trip 131=`0x0055D210`; placement 132=`0x0055D5A0`; gap 151=`0x0055D9D0`; lateral offset 158=`0x0055B690`; manager 174=`0x0055BF60`. |
| TrainCar, `0x00A9C938` | QI 0=`0x00557040`; Init 3=`0x00556F80`; attachment update 50=`0x005573F0`; collision 65=`0x00557080`; entry 110=`0x00557410`; player tick 115 is a false-return stub; collide tick 120=`0x005572D0`; path tick 129 is inherited no-op; placement 132=`0x00557320`; placement accessor 135=`0x00557070`; lateral offset 158 returns zero; no ordinary leader-speed update in slot 162; visibility helper 176=`0x00555A50`. |
| Train, `0x00A9CD68` | QI 0=`0x00557220`; Init 3=`0x00557A20`; Shutdown 4=`0x00559E70`; collision 65=`0x00558E00`; animation 72=`0x00557BF0`; following eligibility 73=`0x00558DC0`; entry/exit 110/111=`0x00557530/0x00557620`; player tick 115=`0x005591A0`; commands 121=`0x005570C0`; speed modifier 125=`0x00557830`; path tick 129=`0x00558510`; trip 131=`0x005588D0`; placement 132=`0x005574F0`; initial path 134=`0x0055AD00`; route hold 165=`0x00558FA0`; route helper 172=`0x00557170`; look-ahead count 175=`0x0053CC20`; truncate 193=`0x00558D20`; target 195=`0x005577F0`; next player path 197=`0x00559C30`; follow cars 198=`0x0055A050`; create car 199=`0x0055A8B0`. |
| Tank, `0x00A9B9B8` | Inherits Vehicle QI; Init 3=`0x0054C880`; Shutdown 4=`0x0054A850`; animation 72=`0x0054B7C0`; debug 93=`0x0054CAA0`; entry 110=`0x0054A780`; player tick 115=`0x0054AAE0`; commands 121=`0x0054B030`; helpers 126–128=`0x0054A740/0x0054A750/0x0054A770`; part-drawing helpers 192–197=`0x0054CF60/0x0054A950/0x0054C2E0/0x0054CBA0/0x0054A9C0/0x0054C5B0`. |

TrainCar QI recognizes `0x4A416254`; Train recognizes that IID and `0x8A41625A`.
Both return the same primary interface with AddRef and forward other queries
to Vehicle. Vehicle forwards unsupported queries to Traffic. Tank uses that
same chain rather than introducing an owner QI override. Refcounting and
occupant/model interfaces are inherited, including secondary-interface pointer
adjustments. Constructors/defaults and failure non-rollback for Vehicle,
TrainCar and Train are §12.3. Their records/ownership are §12.1/§12.8, with
Vehicle Read/Write at `0x0055BCF0/0x0055F640`, Train at
`0x0055AAD0/0x00557930`, and Tank at `0x0054A8E0/0x0054AA30`.
Tank class ID is `0xEBE0DCAD` (`0x0054CE30`). None of these four introduces an
owner DoMessage body: message dispatch remains the inherited occupant/automaton
contract. Notifications such as StartTrip's cell message and collision's
`0x49E64802` are outputs, not new inbound message handlers. Drawing updates use
inherited model/effect slots plus the concrete additions above; IO never requires
running a game's owner method through a foreign manager.

The remaining Vehicle displacement/reattachment instruction audit is tracked
in VH1. Tank's owner boundary is fully stated in §12.13; projectile-asset
simulation is delegated to the effects interface rather than invented here.

### 12.15 Vehicle player-motion scheduling (VH1)

Vehicle requires a drive state and drive parameters; absent state, absent
parameter container, or failure of both direct/service parameter retrieval
returns false. Before inherited Automaton ticking, an unsigned drive-state
damage counter greater than parameter `+0x5C` returns true without movement.
After the inherited tick, a false return stops this owner update. Speed
updating precedes the movement steps; neither collision nor substepping causes
another command translation or acceleration integration during the same tick.
Evidence: `0x00560424–0x005604F3`, `0x005605A7–0x005605C7`.

For ordinary positive elapsed times, Vehicle's step count is
`N=ceil(dt×bits(0x3CF83E10))`, approximately ceil(dt/33). There is **no**
Train-style six-step clamp. For both owners the instruction-level ceil
conversion starts with the x87 product x. Store `x+0.5+12582912` as binary32,
interpret its bits as a u32, and subtract `0x4B400000` modulo 2³² to obtain B.
If x equals the signed-integer value of B−1, use B−1 instead of B. This
preserves exact-integer boundaries of the binary32 rounding trick. Vehicle
does not clamp a zero count to one: the quotient is still evaluated before
the loop, so zero dt is not a guaranteed safe physics input.

Sample the speed tween once as binary32. Store substep elapsed time
`f32(dt/N)` separately; movement distance is
`f32((dt/N)×sampledSpeed)`, retaining the x87 quotient until the multiply
before that final store. Do not integrate speed once per substep, normalize
dt to seconds, use the measured-speed field as the target, or reuse Train's
distance-dependent step count. Evidence: `0x005605C9–0x00560645`; constants
`0x00A9D350/0x00A9D0B8` are binary32 `0.030303031206130981/12582912`.

In the off-network movement branch, slot 187 receives the substep elapsed
time, drive state, distance, parameters and network result. Each such movement
call is followed by collision slot 190 with unset direction and initial
fraction 1. A hit resolves collision and terminates the loop; an ordinary
miss proceeds to the next step. Dashboard work follows the loop. Section 12.16
defines slot 187 and the on-network movement/reattachment branch. Evidence:
`0x00561158–0x00561204`.

| Vehicle scheduling setup | Required observation |
|---|---|
| dt=32, speed 0.125 | One step, distance 4. |
| dt=33, speed 0.125 | Two steps, elapsed 16.5 and distance 2.0625 each: 33 times the stored multiplier is slightly greater than 1. |
| dt=34, speed 0.125 | Two steps, elapsed 17 and distance 2.125 each. |
| dt=224, speed 0.125 | Seven steps, distance 4 each; no six-step clamp. |
| Missing state or parameters | False before owner movement. |
| Damage counter equals threshold / exceeds threshold | Continue / true before inherited tick and movement. |
| Off-network collision at step 1 of 2 | One movement, one collision query, one resolution; skip step 2. |

### 12.16 Vehicle player-drive geometry (VH1)

Together with §§12.10, 12.12, 12.13 and 12.15, this closes the owner-side VH1
physics contract. The following network/path queries and collision probes are
foreign geometry inputs; the vehicle owns the anchors, facing, flags and drive
state changes described here. Tank projectile simulation remains behind the
effects interface specified in §12.13; firing does not directly damage occupants.

#### Coordinates and branch selection

Use the existing centre `C` (`+0x40`), facing `D` (`+0x58`), radius `r`
(`+0x7C`), front anchor `H` (`+0x264`) and saved path facing `F` (`+0x270`).
Drive-state `+0x44` is the **rear anchor** `B`, not the centre; `+0x50` is
its motion direction `V`, and `+0x38` is the direction baseline copied by
on-network motion. Steering angle is drive-state `+0x5C`. A vector is unset
when all three component differences from `(-1,-2,-3)` are strictly below
binary32 `0.0001`; equality at the tolerance makes it set. Normalize a vector
only when its 3D length exceeds binary32 `0.00001`.

After inherited ticking, query `PlayerDriveGetNetwork` at `H`, with no output
position, obtaining an occupant and a surface boolean. When state `+0x8C`
equals −1, the off-network boolean is whether the occupant is absent;
otherwise it is the inverse of the surface boolean. Changes to that boolean
notify owner slot 102 with `(10, boolean)` and store state byte `+0x228`.
Use the on-network branch only when this boolean is false **and** the returned
occupant is nonnull. The decision is made once before speed integration and
substeps. The other branch sets flag `0x2000`; if its returned occupant differs
from the retained one, it calls placement slot 132 with that occupant, key zero,
no points and preparation-only true. The on-network branch clears `0x2000`.
Evidence: `0x00560500–0x00560574`, `0x00560647–0x00560655`,
`0x00560998–0x005609A6`, `0x00561065–0x0056108C`.

The yaw rotation used below is, for finite ordinary inputs,
`R=(cos(a)×D.x + sin(a)×D.z, D.y, cos(a)×D.z − sin(a)×D.x)`.
This is steering of the proposed motion, not a direct replacement of the
vehicle's facing. The routines use x87 sine/cosine and binary32 matrix/vector
temporaries; do not assume every arithmetic operation rounds to binary32.
In particular the network branch retains cosine in the x87 calculation while
storing sine, whereas the off-road branch stores cosine before constructing
its matrix. Evidence: `0x005606A5–0x0056080B`, `0x0055ED32–0x0055EE38`.

#### On-network displacement and reattachment

Before the loop, if flag `0x2000` was set or `H` is unset, obtain the returned
occupant's path interface and ask owner slot 171 for direction-based path
filters, using `R` and true. Request candidate keys at `H` with path type 1,
entry filter `0xFF`, that exit filter and distance 16. In candidate order,
request each key's points with secondary argument zero and project `H` onto
them through the path interface. Accept the first nonnull collection whose
projected height differs from the original `H.y` by **strictly less than 4**;
restore `H` after rejecting a projected candidate. If this finds none, repeat
with both filters `0xFF` and distance 16. Pass occupant, selected key, selected
points (possibly null) and true to slot 132. If points were selected, store
the projection's segment index. This search has no nearest-candidate sort or
owner-side random draw. Evidence: `0x00560812–0x00560998`, candidate helper
`0x0055BF70–0x0055C022`.

For each of the `N` steps in §12.15:

1. Call signed advancement slot 133 (§12.9) with the fixed substep distance,
   start `H`, a separate output front point, the live segment index and point
   collection, and remainder at `+0x260`. A negative step retains that helper's
   backwards extrapolation along the current segment. If it returns false,
   call `ChooseNewPlayerDriveDestination` with the current direction scratch
   vector (initially `D`, subsequently the chosen facing below). Failure
   disables the owner through slot 13 with false and returns true immediately.
   Success continues this substep using the advancement output; it does not
   repeat advancement against the new path in this substep.
2. Form the difference of the current indexed pair of **stored path points**.
   With length above the threshold, normalize it; if `F` is set, replace each
   component by `(new + new + F) × f32(1/3)`. The sums and resulting components
   have binary32 stores, and the result is **not renormalized**. With a
   degenerate segment use `F` unchanged when set, otherwise `(0,0,1)`.
3. Copy the output front to the previous-position fields `+0x4C..+0x54`.
   Apply centre `outputFront − chosenFacing × r × 0.5` and that facing through
   slot 16. Save the facing in `F`, copy the output to `H`, and clear flag
   `0x20`. Store rear anchor `B=outputFront − chosenFacing × r`; copy the
   chosen facing to both drive-state `+0x50` and `+0x38`.
4. Query collision slot 190 with the usual unset direction and fraction 1.
   A hit resolves collision and ends substepping; a miss continues. Geometry
   has already been written when this query occurs.

The half-radius products used for the centre and full-radius products used for
the rear are distinct binary32 temporaries. A smoothed facing shorter than one
therefore also shortens the front-to-centre/rear separation. Evidence:
`0x005609BA–0x00560A04`, `0x00560A06–0x00560BD5`,
`0x00560BD5–0x00560D9B`.

On path exhaustion, `ChooseNewPlayerDriveDestination` requests the neighbor
in direction `(lowByte(pathKey)+2) mod 4`, with additional arguments zero and
true. No neighbor returns false. At a neighbor, slot 171 supplies filters using
the supplied direction and true. Ask for type-1 candidates at `H` with those
filters and distance 4; try the first key, then fall back to a query retaining
the first filter and using `0xFF` for the second. Try only the first key from
each returned list. A usable collection calls slot 132 with neighbor, key,
points and true and returns true; otherwise false. There is no direct random
draw here. Evidence: `0x0055F938–0x0055FA55`.

After the network loop, steering outside the parameter `+0x34` bounds requests
another path on the current occupant: the negative test also requires live
points, while the positive test does not. Slot 171 supplies direction filters;
the path request uses type 1, `H`, and false as its final argument. For a new,
nonnull collection different from the current collection, project a copy of
`H`; only an absolute height difference below 4 permits slot 132 with true
and replacement of the segment index. Copy that projected point to the
previous-position fields. Do not apply another movement step here.

There is then an additional side-path request only with positive slot-38
speed, live points, and drive-state byte `+0xE6` or `+0xE7`. Construct the
horizontal side vector `(-D.z,0,D.x)`, negating it when `+0xE6` is set (that
byte wins if both are set), normalize it and scale its length to 5. Add it
to **facing `D`**, not to centre `C`, to form the direction argument. Ask the
path interface at byte slot `+0x9C` using current key, `C`, that argument,
binary32 `0.8`, and output point/key/index. On success project the returned
key's collection; an absolute projected-height difference from `C.y` below
4 permits slot 132 with the tick's network occupant, returned key, null
points and true, followed by the returned segment index. These post-loop
queries can run after a collision terminated the loop. Evidence:
`0x00560DA0–0x00561060`.

#### Off-network anchors, steering and obstacle response

Slot 187 (`0x0055ED10`) receives elapsed substep time, drive state, signed
distance `d`, parameters and the tick's network result. Its geometry uses
`d`, not another elapsed-time multiplier. At `d=0`, return true without
initializing anchors, querying heights or applying a transform. The caller's
separate slot-190 query still follows it.

For nonzero distance, compute `R` from the current facing and steering. If `H`
is unset, initialize `H=C+D×r×0.5`. Compute proposed front `H+R×d` and proposed
rear `B+V×d`. Normalize their difference to get `U`, or use current `D` when
its length is at most the threshold. Advance the **old** rear by `U×d`, then
call `PlayerDriveGetNetwork` with that rear as both input and output, allowing
it to replace the height. Form prospective front `B+U×r`. Probe the segment
from old `H` to that front through slot 189, with outputs for the horizontal
hit point and horizontal normal. This probe precedes the caller's separate
slot-190 collision phase. Evidence: `0x0055ED10–0x0055F079`.

On a probe miss, set `H=B+U×r`, first overwrite `H.y` with `B.y`, then resolve
`H`'s height with another in-place network query. Recompute `U` from `H−B`
with the same normalization/fallback rule, and reset `B=H−U×r`. Update each
rear-direction component by `V += (U−V)×parameter(+0x38)`; there are binary32
stores before adding the scaled differences. Do not clamp that coefficient,
multiply it by `dt`, normalize the blended direction, or replace baseline
`+0x38`. Evidence: `0x0055F229–0x0055F380`.

On a probe hit, take `H.x/H.z` from its hit point, set `H.y=B.y`, and resolve
`H`'s height in place. Normalize the horizontal `(U.x,U.z)` to `W`. For the
supplied horizontal normal `n`, form
`J = W − (n × f32(1.1)) × dot(W,n)` and store its components as binary32;
let `q` be the binary32 stored length of `J`. No normalization of `n` or
zero-horizontal-length guard is inserted by this owner. At `q >= f32(0.9)`,
replace `U` by `(J.x/q,0,J.z/q)` and set new speed to the binary32 value of
current speed tween × q. At smaller q, new speed is zero and `U` retains
its pre-hit value. Clear acceleration accumulator `+0x28`; if the speed
tween target differs from new speed, capture its current value as start,
set its target, and zero its elapsed/duration words. Store new state speed
`+0x2C` and `V=U`, then reset `B=H−U×r`. Evidence:
`0x0055F083–0x0055F224`; constants at `0x00A8FAF4/0x00A88218` are binary32
`1.100000023841858/0.8999999761581421`.

In both outcomes apply centre `H−(H−B)×0.5` and facing `U` through slot 16,
then return true. This midpoint computation follows the rear/height changes;
it is not simply the old centre plus rotated direction times distance. This
helper does not replace the saved path-facing vector `F`. Evidence:
`0x0055F383–0x0055F3EF`.

#### Worked geometry cases

The cases below supply flat, unchanged heights and no collision unless a hit
is specified. All vectors are `(x,y,z)`; foreign queries return exactly the
paths or probe results stated. Speed/substep selection is independently
covered by §12.15.

| Setup | Required result |
|---|---|
| Network points `(0,0,0),(10,0,0)`, index 0, `H=(2,0,0)`, unset `F`, radius 4, distance 3 | `H=(5,0,0)`, facing `(1,0,0)`, centre `(3,0,0)`, rear `(1,0,0)`; both state directions become `(1,0,0)`; one collision query after placement. |
| Same, but saved `F=(0,0,1)` | Facing `(2×f32(1/3),0,f32(1/3))`; centre `H−facing×2`, rear `H−facing×4`. Facing length remains below one. |
| Same unsmoothed path and start, distance −3 | Front `(−1,0,0)`, centre `(−3,0,0)`, rear `(−5,0,0)`; facing stays `(1,0,0)`, index 0. |
| Reattachment candidate projection is 4 above `H.y`; next candidate is 3 above | Reject the first, restore the input front, accept the second and its index. |
| Off-road `C=(0,0,0)`, `D=(1,0,0)`, unset `H`, rear `(−2,0,0)`, rear direction `(0.5,0,0)`, radius 4, steering 0, distance 1, blend parameter 0.25 | Initialize front `(2,0,0)`; finish front `(3,0,0)`, rear `(−1,0,0)`, centre `(1,0,0)`, facing `(1,0,0)`, rear direction `(0.625,0,0)`. Baseline direction is unchanged. |
| Same off-road geometry, probe hit at horizontal `(2.5,0)`, normal `(1,0)` | Horizontal response length is about 0.1, below 0.9: speed and accumulator become zero, facing stays `(1,0,0)`; front `(2.5,0,0)`, rear `(−1.5,0,0)`, centre `(0.5,0,0)`. |
| Off-road distance zero with unset front | Helper returns true and leaves the front unset; no slot-189/height/transform call inside the helper. The tick still performs its slot-190 query. |
