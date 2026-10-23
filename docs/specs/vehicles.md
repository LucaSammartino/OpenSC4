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
  `ChooseNewPlayerDriveDestination` picks a target about four cells away *[summary]*.

## 4. Trailers

A trailer vehicle drags its cars along the path of the head (`FindPointOnPathSegment`: the point on a path segment
at the car spacing, by segment intersection); its record lists the cars and the path cells (automata.md §6.4,
AM10) *[summary]*.

## 5. Trains

The exact TrainCar and Train record contracts, including load failure and reconstruction rules, are in §12.1.

- `Init` (`0x00557A20`) chooses a car family and target count (§12.3); animation ticks subsequently create
  missing cars (`cSC4TrainCar::Init`, flag `0x10000000`). `Shutdown` releases them.
- `StartTrip` (`0x005588D0`) builds the trip and posts `0x0C05E4CD`; a train that cannot start reports an accident
  *[summary]*.
- `DoAnimationTick` (2.3 KB) moves the cars behind the engine and plays random AEMS sounds; Train's
  `HandleCollision` requests a 20000 ms final state and perturbs car directions (§12.4).
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
| Vehicle | Set flag 8 to the runtime boolean at `0x00B0A0A8`; load drive parameters (slot 158), then initialize Traffic. After that call, enable slot 176 with `(true,0)` if the city option queried through `0x00B43DDC` is true or the global override `0x00B2296C` is true. Return Traffic's result. |
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

### 12.5 Neighbour ranking and look-ahead evidence (VH2, partial)

The neighbour selector (`0x00554EA0`) ranks successful pathfinder results by **route-entry count**, not
Euclidean distance or a random draw. Only status 1 qualifies; an equal-sized route does not replace the
earlier winner. Clamp each candidate destination cell to the city rectangle before pathfinding. The selector
requires the city, neighbour-connection service and pathfinder service; a missing prerequisite returns null.
The complete path-request setup and helper contracts remain VH2's open portion; this paragraph must not be
read as closing the owner implementation. Ranking evidence: `0x00555114–0x0055523B`.

Look-ahead fill (`0x00554870`) seeds an empty deque with the current retained network reference and packed
path word, then fills to **slot 175's count plus one**, including the seed. It stops at a terminal node. A
failed expansion marks the preceding node terminal and returns false. The base slot 175 (`0x00504FE0`)
returns the unsigned tuning byte at `0x00B21D28`; it is not a fixed literal count. Flag `0x40` selects the
explicit-route expansion (slot 168, `0x005527E0`); when clear, use slot 167 (`0x00550F40`). Slot 159 is
`SlowDown` (`0x0054DA00`), not the neighbour ranking operation.
