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

### 12.9 Route geometry findings (VH7, partial)

This subsection settles individual geometry boundaries. VH7 remains open for the complete route/record
reconstruction and placement contract; these findings alone do not close the traffic or vehicle owner.

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

**Unresolved record-layout distinction (VH7 remains open).** The literal version-2
stream order in automata.md §6.3 has these instruction-verified native accesses:

| Field following the base record | Read destination / Write source |
|---|---|
| Automaton reference, IID `0xA9B40F05` | `+0xA8` |
| First u32 | `+0x1DC` |
| u8 | `+0x1E0` |
| Network reference, IID `0xA821EF94` | `+0x1E4` |
| Next u32 | `+0x1EC` |
| Last scalar u32 | `+0x1F4` |
| Count and three-byte route entries | deque rooted at `+0x224` |

These are offsets from the receiver of `Read`/`Write`, not confirmed semantic
aliases of the live fields listed above. In particular, naming the serialized
network reference as live `+0x1EC` or calling the following u32 the live path key
would contradict these accesses. Read's known serialization thunk subtracts four
from its interface receiver; Vehicle::Read forwards its receiver unchanged.
The Read body makes no further receiver adjustment. The working PE independently
contains the displacements `0x1E0` at `0x00554DA0` and `0x1E4` at
`0x00554DB4`/`0x00551E3A`. Evidence: `0x005552E0–0x005552E7`,
`0x0055BD20–0x0055BD28`, `0x00554D9A–0x00554E11`,
`0x00551E17–0x00551E64`. The remaining question is the semantic receiver/layout
relationship and the later load-to-live binding, not whether Read itself selects
points. Until established, implementations must not infer an eight-byte adjustment
or silently rename the stream fields. These observations do not yet establish a
native save/load defect.

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
is drive-state `+0x94`. Without the flag, use parameter `+0x2C`.
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
