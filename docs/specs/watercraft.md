# Spec: watercraft (`cSC4Watercraft`): boats, ferries, tugs and U-Drive-It boats

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[summary]* is a correct outline whose details are not given.

Related specs: [automata.md](automata.md) §5.3 (the watercraft manager record), §6.4 (the boat record), §6.6 (the
state family 70–78, the crash cases and the idle and crash accumulators), §7.2 (the manager's `CreateAutomaton`
override with its spacing test), §11 (slot names); [drive-tool.md](drive-tool.md) (U-Drive-It);
[audio.md](audio.md) (AEMS events); [aircraft.md](aircraft.md) (the same drive-parameter scheme).

## 0. Summary

A watercraft is an automaton that moves on water cells. An AI boat wanders: it keeps a short list of water cells
ahead and steers along it. A ferry shuttles between two stations along a stored route and docks at each one, with a
berth and a waiting spot per station. A boat with a neighbour route sails a route to the edge of the map. Tugs in
U-Drive-It can throw a rope and tow another boat. Collisions with other boats, the shore and buildings make
player-driven boats sink or run aground and make AI boats fade out. Handling comes from a 40-value drive-parameter
list in the boat's automata group.

## 1. Class and fields *[compat]*

| Class | Class ID | Record |
|---|---|---|
| `cSC4Watercraft` | `0x096E75F5` (`GetGZCLSID` `0x0056DC60`) | `u16` 2 + the automaton base (automata.md §6.4; Write `0x00566EE0`, Read `0x00566F00`) |

The boat's own state is runtime only (not saved). The names below come from the debug text (§9) where it has them.

| Offset | Meaning |
|---|---|
| `+0x1E4`, `+0x218` | ferry station A and B (a station record: the station lot, its occupant, the berth position at `+0x10` and the berth direction at `+0x1C` of the record) |
| `+0x215`, `+0x249` | the boat has a dock request at station A, at station B |
| `+0x24C` | the station of the current crossing, with its compressed route (`+0x250`) and start position (`+0x25C`) |
| `+0x274` | the boat holds a berth at the current station |
| `+0x278`, `+0x27C`, `+0x27D` | route cells already used, route started, ferry mode |
| `+0x280`, `+0x284` | the boat it tows; the tug towing it |
| `+0x288` (D), `+0x289` (Q) | docking, queued |
| `+0x28C` | the ferry route reader |
| `+0x2A4`–`+0x2AC` | the **travel points**: a vector of 16-byte entries (`s32` cell x, `s32` cell z, `u8` tried-direction mask, `u32` direction 0–7) |
| `+0x2B0` | docking in progress |
| `+0x2B2` | sunk by a collision |
| `+0x2B4`, `+0x2B8` | lifetime limit (0 = none) and lifetime elapsed, ms |
| `+0x2BC`, `+0x2C0` | idle and crash accumulators (automata.md §6.6) |
| `+0x2CB`, `+0x2CC` | being towed; count the idle budget in every tick (set outside this class *[inferred]*) |
| `+0x2D0`, `+0x2D4` | time without moving; time going backwards, ms |
| `+0x2D8` | tow armed |
| `+0x2DC` (Speed), `+0x2E0` (Turn) | the current speed and turn |
| `+0x2E8`, `+0x2F4` | previous and current position (Pos) |
| `+0x300`, `+0x30C` | previous and current heading (Ornt), unit vectors |
| `+0x318`, `+0x31C`, `+0x328`, `+0x340` | the neighbour route: its network occupant, route bytes, reader and the "has a neighbour route" flag |
| `+0x344`–`+0x3E0` | the 40 drive parameters (§2) |
| `+0xDAC` | the U-Drive-It gear: 0 neutral, 1 reverse, 2 forward |
| `+0xDB0`–`+0xDC4` | six effects (§3) |

## 2. Drive parameters (`LoadDriveParameters`, `0x00567310`) *[compat]*

The list comes from the boat's automata group (slot 67, list at group `+0x60`) or, when the group has none, from its
parent chain (`+0x0C`, as for aircraft). Entry *k* is stored at `+0x344 + 4k`. A given value of 0 takes the
default. Message `0x26D31EC1` (§7) reloads the list.

| # | Default | Use | # | Default | Use |
|---|---|---|---|---|---|
| 0 | 0.1 | U-Drive-It acceleration factor | 20 | 0.2618 | turn rate (into the drive state, §6) |
| 1 | 0.001 | held-input steering gain per ms (§6.2.2) | 21 | 8000 | crash duration, ms (automata.md §6.6) |
| 2 | 1.0 | steering return rate per second (§6.2.2) | 22 | 0.7854 | |
| 3 | 0.05 | coasting drag | 23 | 0.95 | |
| 4 | 0.2 | | 24 | 2.1817 | |
| 5 | 5.0 | counter-steer boost | 25 | 1.0472 (60°) | run-aground angle (automata.md §6.6) |
| 6 | −0.1 | lowest speed (reverse) | 26 | 10 | |
| 7 | 0.3 | highest speed | 27 | 0.5 | player boat-to-boat crash threshold |
| 8 | −entry 9 | lowest steering | 28 | 0.25 | player shore crash threshold |
| 9 | 2.0 | highest steering | 29 | 0.75 | speed kept after a bump |
| 10 | 0.001 | per-ms scale of the acceleration | 30 | −2000 | |
| 11 | 50 | | 31 | 3000 | |
| 12 | 5.0 | | 32 | 150 | U-Drive-It reverse threshold |
| 13 | 0.001 | | 33 | 0 | U-Drive-It forward threshold |
| 14 | 0.001 | | 34 | 2.0 | |
| 15 | 0.1 | AI boat-to-boat crash threshold | 35 | 100 | longest tick, ms |
| 16 | 0.5 | AI shore crash threshold | 36 | 300 | dock request distance |
| 17 | entry 18 / entry 7 | | 37 | 90 | final approach distance |
| 18 | 0.3142 | | 38 | 1.0 | docking turn rate, degrees per ms |
| 19 | entry 20 / entry 9 | | 39 | 1.0 | docking move rate, units per ms |

With no list at all the same defaults are used, with 8 = −2.0, 17 = 1.0472 and 19 = 0.1309. Uses not given are
not traced *[summary]*.

## 3. Init and Shutdown *[compat]*

`Init` (`0x0056DE40`), once: the automaton's `Init`, the drive parameters, and six effects from the city's effect
manager, in this order: `SMLmushroomcloud`, `boat_grounding`, `boat_splash`, `tugropethrow`, `tugropeconnection`,
`waterski` (`+0xDB0`–`+0xDC4`). It sets flags `0x8000`, `0x2000`, `0x20000` and 4, registers the boat for
message `0xAC400A31` (§7), clears the tow, the stations, the travel points and the timers, copies the position and the
orientation into the current and previous position and heading (normalising the headings), and enables the active
sound (slot 77, `EnableActiveSound(true)`).

**Initialization and record failure boundaries (WC3, partial).** Init first tests the inherited initialized
predicate on the +4 interface. When already initialized it returns true without redoing watercraft setup.
Otherwise it calls base Init but does **not** inspect the result before loading tuning or acquiring effects
(`0x0056DE49–0x0056DE64`). Each effect creation uses an output reference; any previous reference is cleared
and released first. The factory's return status is not inspected and there is no rollback or false return
for an absent effect. The normal endpoint returns true (`0x0056E330`). This is not a promise that missing city
or effect-manager services are safe: the city and effect manager are dereferenced without a null guard in
this path. Those services are required. Null effects are tolerated by the ordinary command/effect guards;
individual crash paths still require separate auditing under WC5.

The four flag writes are in the order above (`0x0056E02D–0x0056E05F`). Resetting the timers **includes** the
lifetime limit +0x2B4, lifetime elapsed +0x2B8, still time +0x2D0, backwards time +0x2D4, crash time +0x2C0,
and idle time +0x2BC (`0x0056E141–0x0056E171`, `0x0056E2A0`). It also zeros speed, turn, +0x2E4 and +0x2C4.
Both stored headings are divided by their Euclidean lengths; zero-length headings have no finite-vector
fallback in this initialization.

Read consumes a u16 version and accepts **only 2**. Other versions return false without invoking base Read.
For version 2, return exactly the base Read result. There is no separate check of the version-read status
before the comparison, and no boat-local serialized payload after the base record (`0x00566F0A–0x00566F2F`).
Write sends u16 2 then delegates to base Write; it does not short-circuit on the version-write status.

`Shutdown` (`0x0056D0E0`), once: a towed boat is sent to `SLOW_REMOVE` (73) and released; the tug is released; a
dock request at A or B gives back its waiting spot and berth (`cSC4WatercraftManager::DoneDocking`); the message
registration is removed; the stations, the route occupant and the six effects are released (each effect is stopped
first); then the automaton's `Shutdown`.

The inherited Shutdown call occurs even if the initialized predicate was false: only watercraft cleanup is
conditional (`0x0056D0E0`). The +4 interface's Init and Shutdown entry points subtract 4 from their object
pointer and use these same owner methods (`0x0056F7B0`, `0x0056DCF0`). The deleting destructor at
`0x0056DD00` first runs destruction at `0x0056DD20`, conditionally frees object storage when bit 0 of its
argument is set, and returns the original object pointer. Detailed destruction and allocation-failure
contracts remain WC3; the deleting entry must not be mistaken for the complete destruction body.

## 4. States *[compat]*

`DoStateTick` (`0x00573340`) and the enter and exit hooks extend automata.md §6.6:

| State | Enter (`0x0056E340`) | Tick | Exit (`0x00570140`) |
|---|---|---|---|
| 7 `IDLE` | | the idle budget; above 60,000 ms `PushState(73)` (slot 52) and the same tick also runs the crash tick of 70 *[quirk]* | |
| 16 `PLAYER_DRIVE` | active sound off; in the drive state: `+0x30` := 0, `+0x234` := entry 20, `+0x34` := the group's AEMS record value at `+8` (1.0 without one), `+0x144` := 15, the shot count `+0x178` := 0, `+0x1AC` := 2000.0; end the tow and the docking | | active sound on; end the tow |
| 70, 71 crash | flags `0x10000000` and `0x8000000`; `NotifyAutomatonEvent(0xB, 0)` (slot 102); crash time 0; end the tow and the docking | crash time += dt; `DoCrashSequenceTick` (automata.md §6.6) | |
| 72 `IMMEDIATE_REMOVE`, 73 `SLOW_REMOVE` | flags `0x10000000`, `0x8000000`; `Remove(true)` / `Remove(false)`; end the tow and the docking | the same remove every tick | |
| 74 `FERRY_DOCK` | flag `0x10000000`; D := 1; docking in progress; end the tow | `DoDockingTick` (§5.3); when it returns false, `PopState` | D := 0; switch the active station (§5.4) |
| 75 `FERRY_QUEUE` | flag `0x10000000`; Q := 1; speed and turn 0; end the tow | `Ferry_DockRequests` (§5.2) | flag `0x10000000` off; Q := 0 |
| 76 `MARINA_PARKED` | speed and turn 0; end the tow and the docking | | |
| 77 `BEING_TOWED` | being towed := 1; flag `0x10000000`; end the docking | `DoFollowingTick` (§6.3) | flag `0x10000000` off; the tug's towed-boat reference and the boat's tug are released |
| 78 `UNDOCKING` | | `DoUndockingTick` (§5.5); when it returns false, `PopState` | |

"End the tow" (`EndTowing`, `0x00567E80`): the towed boat pops its state and both references are released.
"End the docking" (`EndDocking`, `0x0056D4D0`): with a current station and a held berth, clear docking in progress,
give the berth back (`DoneDocking`), clear the held flag and flag `0x10000000`.

## 5. AI movement *[compat]*

### 5.1 The animation tick (`DoAnimationTick`, `0x00572CF0`)

The tick gets two frame times in ms; the first drives movement and timers, the second the idle budget and the
sounds *[inferred]*.

1. A speed modifier (slot 125) below 0.00001 ends the tick with true.
2. With `+0x2CC`, the idle budget grows by the second time; above 60,000 ms, `PushState(73)`.
3. A non-zero speed updates the active sound (slot 78).
4. The tow rope (`tugropeconnection`) follows the boat while it tows and stops when it does not.
5. The automaton's step (`0x00508740`); when it fails, skip to 13.
6. A negative speed adds to the time going backwards, otherwise that time is 0.
7. **Wakes.** The boat counts as moving when its speed is not 0, or it moved at least 0.2 × the city's cell size
   (city slot 91, computed once) since the last tick, or its heading changed by at least 0.01. Moving restarts the
   wake effects (`0x0056F1E0`: effect slot 44 for `SMLWake`, `SMLWash`, `MEDWake`, `MEDWash`, `LargeWake`,
   `LargeWash`) and clears the still time; otherwise the still time grows and above 250 ms the wakes are stopped
   (`0x0056EFE0`, effect slot 45 for the same names) *[inferred: which slot starts and which stops]*.
8. A boat still for more than 5000 ms whose state stack holds none of the states 7, 16, 75, 76, 77, 78 (slot 57
   `FindState`) returns false; what the caller does with that is not traced.
9. With state 16, 74, 75, 77 or 78 in the stack, or while being towed, or with `+0x2CC`, skip to 13.
10. **Lifetime.** With a limit that the elapsed time has passed, the manager is asked (manager slot 41) whether the
    boat may stay; without a manager, on a no, or with state 76 in the stack: `PushState(73)`; on a yes the limit is
    cleared and flag `0x10000` cleared.
11. State 76 in the stack, or top state 7: skip to 13.
12. The elapsed time grows. If the boat cannot move (§5.6): on land, `PushState(72)`. Otherwise, with the frame time
    clamped to 0–entry 35: `Engine_AIBoatMotion`, `Movement_PositionNew`, `Engine_DetectCollision`,
    `Ferry_DockRequests` and `Ferry_ClearDockRequests`.
13. The effect and model updates (slots 104 and 107).

### 5.2 Picking cells (`Engine_AIPickLocation`, `0x00570620`)

The travel points are topped up to 10 entries, starting from the last point, or from the boat's cell and the
direction of its heading when the list is empty. A start outside the city ends the pick.

- **Neighbour route** (`+0x340`): the next cell of that route is appended; when the route is used up,
  `PushState(73)`.
- **Ferry** (`+0x27D`): on the first pick the route reader skips the cells already used (`+0x278`, at least 2 at
  the start of a crossing); each pick appends the next route cell and counts it.
- **Wandering.** The boat's radius in cells is 3 in group `0x4414`, 2 in group `0x4413`, else 1. From the last
  point it tries the eight directions in the order straight on, +1, −1, +2, −2, +3, −3, +4 (`Assist_PickNewDir`,
  `0x00566F60`, using signed remainder by 8 as detailed below), never the direction straight back and never one already tried from that point.
  The neighbour cell in that direction (`0x00567000`) must be navigable for that radius (surface water slot 8,
  `IsNavigable(x, z, r, 0)`, terrain.md §10.4) and the boat must not overlap another boat there (§5.7); the first
  that passes is appended. On the first eligible candidate failure in a search round, the boat backs up: with 6 or more points it drops 5, with 4–5 it
  drops 3, else it clears the list; the direction is marked tried in the new last point and the search goes on.
  At most 15 rounds are made.

**Direction and backtracking details (WC7, partial).** Cell increments are
`0=(-1,-1), 1=(0,-1), 2=(1,-1), 3=(1,0), 4=(1,1), 5=(0,1), 6=(-1,1), 7=(-1,0)`
in (cell x, cell z). The step helper leaves both coordinates unchanged for an out-of-range direction
(`0x00567000–0x0056705D`, jump table `0x00567060`). Direction choice uses signed 32-bit remainder
with truncation toward zero, **not** a nonnegative mathematical modulo. For example, heading direction 2
produces candidate values `2,3,1,4,0,5,-1,6`; the last is skipped as the reverse direction. The -1 candidate
does not step to a different cell. Its low-byte tried-mask bit is zero (shift count 31), so it cannot be
recorded as a normal direction bit. Do not silently wrap it to 7. Evidence: `0x00566FBD–0x00566FCC`.

The first failure of an eligible navigability/overlap test in each outer search round does the backtrack
described above immediately; it does not wait for all eight candidates. After that one backtrack, later
failures in the same round advance to the next candidate. At a nonempty survivor list, the byte tried mask
is ORed with `low8(1 << (survivor.direction & 31))`: the direction comes from the **new last point**,
not the rejected direction or the popped point. Evidence: `0x005709C4–0x00570A19`,
`0x00570A19–0x00570A9D`. The remaining compressed-reader and heading-quantization contract is still WC7.

`Engine_AIBoatMotion` (`0x00571880`, 2.8 KB) picks cells when needed, steers the heading toward the next point and
sets the speed; `Movement_PositionNew` (`0x0056C430`, with curved-motion helper `0x00569940`,
straight-motion helper `0x00568230` and height/tilt helper `0x00569E70`) moves the boat by speed × time,
turns it, keeps it on the water surface and tilts the model
*[summary]*.

### 5.3 Ferries: requests and docking

`Ferry_OtherStationInformation` (`0x0056F480`) returns station B when the current crossing starts at station A
(positions equal within 0.0001 on each axis), else station A: the station the boat is heading for.

`Ferry_DockRequests` (`0x005704D0`): within entry 36 (300) of the station's position and with a manager:

- queued (top state 75): when the berth is free, take it (manager `0x0056CEC0`), `PopState`, `PushState(74)`;
- otherwise: take the berth and `PushState(74)`; or take the waiting spot (`0x0056CF50`) and `PushState(75)`; or,
  with both taken, `PushState(73)`.

The manager keeps, per station lot, one berth and one waiting spot, each holding at most one boat.
`Ferry_ClearDockRequests` (`0x0056D530`): a boat holding a berth that is more than entry 36 away from its current
station gives the berth back (`DoneDocking`), clears the held flag and flag `0x10000000`.

`DoDockingTick` (`0x00572770`): on land → `PushState(72)`. The heading is flattened (y = 0, renormalised). Farther
than entry 37 (90) from the berth, the boat sails on with the AI motion and the frame time clamped to entry 35.
Nearer, it turns toward the berth direction by at most dt × entry 38 degrees (sign from the cross product) and
moves toward the berth position by at most dt × entry 39. **Docked** when the angle is below 1° and the distance
below 1: the tick returns false and the state pops.

#### 5.3.1 Final approach arithmetic and completion boundary (WC4/WC7, partial)

Request distance uses the destination station lot's reported world position, **not** its berth point; the
comparison is strict `< p[36]`. An occupied berth while already queued returns without popping state 75,
requesting another waiting spot, or removing the boat (`0x005704D0`). Acquiring a destination berth does not
itself change the current-crossing record or its held-berth byte +0x274. The initial station-request flags
and route-switch ownership remain WC7.

For docking let P be the live position, B the destination berth position, H the live heading, and D the
destination berth direction. Initial range `d=length(P-B)` uses all three coordinates. OnLand pushes 72
but **does not return**: docking continues and its boolean follows the normal motion/completion rules.
If H.y != 0, first set H.y=0 and normalize H. For d > p[37] use the AI motion, position, collision order
with `clamp(dt,0,p[35])` and return true. Equality uses final approach.

In final approach normalize H and D using their full Euclidean lengths. Let
`theta=acos(dot(H,D))*57.29578` in degrees; there is no dot-product clamp before acos. If the f32
representation of theta is NaN, substitute zero. Before moving, if theta < 1 and d < 1, return false.
Equality at either threshold still produces a motion tick. Otherwise set
`a=min(theta,dt*p[38])` for theta >= 0 (zero otherwise),
`m=min(d,dt*p[39])` for d >= 0 (zero otherwise). Neither dt nor the caps are clamped to nonnegative
values in final approach; a negative dt can give negative steps.

For d != 0, `P := P + (B-P)*(m/d)`; d=0 skips translation. Set
`sign=+1` if `D.x*H.z-D.z*H.x >= 0`, else -1, and `phi=sign*a*0.017453292`.
Rotate about Y: `H.x := cos(phi)*oldH.x + sin(phi)*oldH.z`,
`H.z := -sin(phi)*oldH.x + cos(phi)*oldH.z`; H.y is unchanged. Publish P/H through MoveTo,
copy stored current position/heading to their previous fields, then copy the live values to current.
Return **true** after this update; there is no post-motion completion test. A boat reaching the berth
in this call completes on the next docking call. Evidence: `0x005729CF–0x00572A87`,
`0x00572A8A–0x00572B76`, `0x00572C62–0x00572CED`. The acos helper is the x87 wrapper at
`0x009F0090`, with acos arithmetic at `0x009F00C8–0x009F00D6`.

For exactly antiparallel finite horizontal unit headings, theta is 180 and the zero cross product selects
the positive rotation. A zero-length heading/direction has unguarded normalization and can propagate NaN
components; theta's NaN-to-zero substitution is not a repair of the stored vector. Implementations must
not invent a deterministic fallback direction and call it original behaviour.

### 5.4 Stations and routes

`Ferry_SetStationInformation` (`0x0056F670`) stores both stations; when asked to start, it begins the crossing from
station A and sets docking in progress. `Ferry_InitializeCurrentRoute` (`0x0056F3F0`) makes the given station the
current one, holds its berth, sets the used-cell count to 2 when it was 0, clears "route started", loads its
compressed route and clears the travel points. `Ferry_SwitchActiveStation` (`0x0056F700`), on leaving state 74:
with requests at both stations, the crossing restarts from the station just reached; otherwise `Remove(true)`.

`SetNbrRouteData` (`0x0056F510`) gives the boat a neighbour route: its network occupant, its route bytes and the
flag; the reader then skips cells while they are within 3 × the boat's radius of the occupant's position.

### 5.5 Undocking (`DoUndockingTick`, `0x0056BD80`)

While the boat can move and its position is navigable with radius 2 (surface water slot 7, terrain.md §10.2; with a
neighbour route, `+0x340`, the lot passed is the one at the route occupant's position), it moves away from the
route occupant's position, horizontally, toward the point 2 × radius from it, by at most the frame time × entry 39
per tick (at least 1 unit to move at all). On land → `PushState(72)`. The tick returns false (the state pops) once
the boat is 2 × radius away.

The undocking boolean is false when movement is impossible, navigability fails, or the route occupant is
missing. It is also false when the **pre-motion** 3D distance to the occupant's water-level position is
at least twice this boat's radius, or when the horizontal distance to the outward target is below 1.
After a qualifying motion attempt it returns true, even when that attempt reaches its target
(`0x0056BF10–0x0056BF71`, `0x0056C073–0x0056C089`). Pushing 72 for OnLand does not itself choose
the boolean. The outward direction is `(P.x-O.x,0,P.z-O.z)` divided by its horizontal length;
there is no coincident-position guard. The target is O plus twice this boat's radius along that direction.
For a finite nonzero target displacement, movement length is `min(horizontalDistance,dt*p[39])`,
without the general movement dt cap; after translation reset height to terrain slot 29's water level.

### 5.6 Can it move?

`Engine_MovementPossible` (`0x005698E0`): not when flag 1 is set; not when a corner of its cell rectangle
(occupant slot 11) is outside the city (`0x005681B0`, city slot 99; then also `PushState(72)`); not in states 70 and
71. `OnLand` (`0x00567DF0`): the boat's cell is inside the city and the terrain height at the boat (terrain slot 18)
is above the level of terrain slot 29 *[inferred: the sea level]*.

### 5.7 Collisions (`Engine_DetectCollision`, `0x0056C090`)

1. A corner of the cell rectangle outside the city → `PushState(72)`.
2. **Other boats** are tested when this boat is player-driven, or when neither is and AI collisions are enabled
   (`0x00B22A7C` clear). The other boat is the first in the manager's list within the two boats' combined size whose
   footprint overlaps (`0x005690D0`). A crash happens when |speed − other speed| / entry 7 is above entry 27 (this
   boat player-driven) or entry 15. A player crash plays AEMS event 7. The boat with the lower or equal size class
   (`0x00566D60`) sinks: a player-driven loser gets `NotifyAutomatonEvent(0xB, 0)` and `PushState(71)` for entry 21
   ms (and "sunk by a collision"); an AI loser fades out (`PushState(73)`), unless the winner is the player's boat,
   in which case it sinks. Below the threshold the boats bump (`0x0056A310`): the other boat takes a new speed when
   it can move and this boat keeps entry 29 of its speed.
3. **The shore and buildings** (`Crash_InanimateCollisionDetection`, `0x005687C0`): a hit faster than entry 28
   (player) or entry 16 (AI), as |speed| / entry 7, plays AEMS event 7 for the player and runs `Crash_DetermineCase`
   (automata.md §6.6); slower hits bump.

## 6. U-Drive-It *[compat]*

### 6.1 The drive tick (`DoPlayerDriveTick`, `0x0056D310`)

The automaton's drive tick; a tug whose towed boat is ahead of it (positive dot product with the heading) ends the
tow; with no throttle and no reverse (both −1 in the drive state) the speed decays toward 0 by
dt × 0.001 × entry 3 (and stops at 0); then, when the boat can move, `Movement_PositionNew` with the frame time
clamped to 0–entry 35, and `Engine_DetectCollision`.

### 6.2 Commands (`TranslatePlayerDriveCommands`, `0x0056E5C0`)

The group must have an AEMS sound set (group `+0x64`, or its parent's); otherwise nothing happens.

- **Gear.** Throttle at least entry 33 → forward with the throttle; else reverse input at least entry 32 → reverse
  with −the input; else neutral.
- **Speed.** The speed moves toward the gear's target (clamped to entries 6–7) by dt × entry 10 per tick, or by
  dt × entry 10 × entry 0 in reverse and when braking; a change of direction first brakes through 0 *[summary of
  the cases]*.
- **Steering.** Left and right inputs change the steering at rates from entries 1, 2 and 5 (entry 5 when steering
  against the current turn), scaled by dt × entry 10 × entry 0, clamped to entries 8–9 *[summary]*; the turn
  (`+0x2E0`) is the steering, and the rudder angle shown is −0.125 × the steering.
- **Fire.** With the fire button, the effect `whalefire` is played; when it plays, the drive state's shot count
  grows and the button is cleared.
- **Water skier** (group `0x4409`): a toggle starts the `waterski` effect behind the boat while it moves forward, or
  stops it; it also stops when the boat stops.
- **Tow rope** (group `0x4410`, a tug): with the boat stopped and the tow button, a tug that tows nothing arms the
  tow (`+0x2D8`) and plays `tugropethrow` at the boat; one that tows ends the tow.
- The speed tween is reset when |speed| changes; then the automaton's own command translation.

#### 6.2.1 Command values and nonlinear speed response (WC8 correction)

Let `p[k]` denote drive entry k, `dt` the command time in milliseconds, `v` the stored speed,
and `clamp(x,a,b)=min(max(x,a),b)`. This section supersedes interpretations of the summary above
as direct analog-speed clamping or braking in neutral. All inputs in these equations are finite;
invalid tuning (zero denominators, reversed bounds) has no specified recovery.

Commands arrive in the shared drive-state record. Forward and reverse channels are floats at
`+0x7C` and `+0x80`; `-1` is the inactive sentinel used by the drive tick. The watercraft translator
selects forward when forward >= p[33], otherwise reverse when reverse >= p[32], otherwise neutral.
Thus forward wins when both qualify. It writes gear 2/1/0 and signed command `q` at `+0x28`:
forward, minus reverse, or zero respectively. The thresholds are inclusive. This happens only after
the automata group and its AEMS record (including parent fallback) have been found; failure to find
either returns false before changing commands. Evidence: `0x0056E5E3–0x0056E679`.

The target-speed helper at `0x00567080` is **nonlinear**. Define f32 constants
`a=0.910000026226`, `b=1.57079637051`, `n=f32(-a*0.5)`,
`P=f32(tan(a*b))`, `N=f32(atan(n*b))`. First let `x=clamp(q,p[30],p[31])`, then

| Stage | Positive input | Zero or negative input |
|---|---|---|
| Normalized command `u` | `x*a/p[31]` | `x*n/p[30]` |
| Curve `w` | gear 2 uses `tan(u*b)` for either sign of u | other gears use `atan(u*b)` for either sign of u |
| Speed before clamping | `w*p[7]/P` when w > 0 | `w*p[6]/N` when w <= 0 |

The target `t` is that speed clamped to `[p[6],p[7]]`. The curve choice uses **gear**, while the
scaling choice uses the sign of its result. These are x87 transcendental operations with f32 stored
constants, not a linear percentage mapping. The caller clamps the helper result to the same speed
bounds again. Evidence: `0x0056708A–0x005671C8`, particularly `0x00567152–0x00567188`.

For q != 0 set `A=f32(dt*p[10])`, `B=f32(A*p[0])`. The speed response is:

| Condition on speed before this command | New speed and command-channel side effect |
|---|---|
| v != 0 and sign(t) differs from sign(v) | Move v toward t by at most B, snapping to t if closer. For old v > 0 write reverse channel := p[32]; for old v < 0 write forward := p[33]. |
| Otherwise sign(q)=sign(v) and abs(t) <= abs(v) (zero has positive sign) | Keep v. Rewrite forward channel with the inverse-speed curve if v > 0, otherwise rewrite reverse channel. |
| All other q != 0 cases | Move v toward t by at most A; use B instead when q < 0 and sign(q) differs from sign(v). |
| q = 0 | Keep v; no neutral braking occurs in this method. |

For the last movement row, the limited increment is `sign(q)*step`; the snap comparison uses
`abs(t-v)`. Opposite-gear response is a single approach to t: it can **cross zero in one call**.
There is no stop-at-zero rule or separate allocation of leftover time. Evidence:
`0x0056E694–0x0056E8A0`, especially `0x0056E759–0x0056E7AD`.
Coasting belongs to the subsequent drive tick (§6.1) and requires both channels to equal -1.

The inverse curve (`0x005671D0`) uses the same a and n, but independently cached constants
`P'=f32(tan(a*b))`, `N'=f32(tan(n*b))`. For `s=clamp(v,p[6],p[7])`, let
`z=s*P'/p[7]` for s > 0, else `s*N'/p[6]`, and `h=atan(z)*0.63661975`.
Return `clamp(h*p[31]/a,p[30],p[31])` for h > 0, else
`clamp(h*p[30]/n,p[30],p[31])`. Its signed result is written directly into the selected
channel, including the reverse-channel case; no extra absolute value is applied.

#### 6.2.2 Steering channels and skier toggle (WC8 correction)

The steering inputs at drive-state `+0x88` (L) and `+0x84` (R) are elapsed-input floats,
not booleans. Negative is inactive. Let T be drive-state `+0x5C`, and
`r=f32(f32(dt*0.001)*p[2])`. The translator applies these two ordered responses:

1. If L >= 0 and T <= 0, replace L by `-T/p[1]-dt` only when T < `-p[1]*L`,
   then set T := `-p[1]*L`. Otherwise, if T < 0, increase T toward zero by r.
   When R > 0 in this return branch, use `max(r,r*p[5])` and clear R to zero.
2. Using the resulting T and channels: if R >= 0 and T >= 0, replace R by
   `T/p[1]+dt` only when `p[1]*R < T`, then set T := `p[1]*R`.
   Otherwise, if T > 0, decrease T toward zero by r; when L > 0 in this return branch,
   use `max(r,r*p[5])` and clear L to zero.

Return-to-zero steps stop at zero. Finally clamp T to `[p[8],p[9]]`, copy it to the boat's
turn, set drive-state `+0x23C=0`, `+0x240=6.2831855`, and `+0x238=-0.125*T`.
Simultaneous channels do **not** select a separate neutral-steering case: the ordering above
decides the result. The timer-rebasing terms have different signs and must be preserved.
Evidence: `0x0056EBAE–0x0056ED5A`, `0x0056EEC2–0x0056EF1F`.

For group membership 0x4409 with a skier effect, a nonzero toggle byte at drive-state +0xEB
stops an already running effect, or starts a stopped one only when v > 0. After this toggle
step a running effect is stopped whenever v <= 0; a running effect with v > 0 has its
transform refreshed. This owner does **not** clear the toggle byte when stopped. A toggle
supplied while stopped cannot start the effect in that call; if still supplied on a later
forward-moving call, it can start it. No additional remembered arm bit exists in this method.
Evidence: `0x0056E97D–0x0056EBAE`. Full effect placement remains part of WC6.

### 6.3 Towing

A tug's armed tow is used by message `0xAC400A31` (§7). A towed boat (state 77, `DoFollowingTick`, `0x005694B0`)
keeps on the water surface and stays radius + 10 behind its tug: it turns toward the tug by at most dt × entry 38
degrees and moves toward the point behind it when farther; it does nothing without a tug.

## 7. Messages (`DoMessage`, `0x00569230`) *[compat]*

- `0x26D31EC1`: reload the drive parameters.
- `0xAC400A31` while the tow is armed: disarm it; take the world point of the message's payload; when the pick
  is valid and a manager exists, the nearest boat at that point (`0x00568C10`), not this one, in group `0x4417`, and
  not in state 70, 71, 72 or 73, becomes the towed boat: the tug is recorded in it, a boat that was docking, queued
  or parked (74–76) pops that state, it pushes state 77, and this boat's own tug reference is released
  *[inferred: the payload is the rope's landing point]*.
- Anything else goes to the automaton's handler.

## 8. Quirks

1. The 60 s idle timeout pushes `SLOW_REMOVE` and, in the same tick, also runs the crash accumulator and the crash
   sequence tick (the switch falls through from case 7 into case 70; bytes `0x0057339C`–`0x005733B5`).
2. The idle budget is counted twice per tick for a boat with `+0x2CC` that is idle: once in the animation tick and
   once in the state tick *[inferred: both run in the same frame]*.

## 9. Debug text (`GetDebugExtraText`, `0x00570AE0`)

`Tow: %d, (FT,MT): (%d,%d), (Q,D): (%d,%d), Turn: %3.2f, Speed: %3.2f, Cell(%3d,%3d), Pos(%3.2f,%3.2f,%3.2f),
Ornt<%3.2f,%3.2f,%3.2f>`: whether it tows, two counts of the manager (`+0xA9C`, `+0xAC4`), queued, docking, turn,
speed, cell, current position and current heading.

## 10. Open questions

See [questions/watercraft.md](questions/watercraft.md) for the remaining owner-closure questions.
WC8's proposed command choices are corrected in §6.2.1–§6.2.2.

### 10.1 Owner-closure evidence status (2026-10-23)

WC3 is not closed by counting names. Re-running only the attribution phase of `tools/coverage.py` with the
current main-checkout export, names, ABI tables and coverage metadata yields **75 functions / 38,073 bytes**
for this owner; the question's earlier **79 / 38,556** is not the current attribution result. Neither count
means every item has a complete behaviour contract. The current inventory below records every attributed
entry once, grouped for audit; numeric addresses here are interface/evidence facts, not source listings.
Names in parentheses describe helper roles, not new public ABI names. Sections labelled partial and all
untraced helper bodies still prevent a whole-owner closure claim.

| Area | Attributed entry addresses | Contract status |
|---|---|---|
| Message-target refcount adjustors | 004FAF30, 004FAF40 | subtract 0x38 from this, delegate AddRef/Release to inherited implementation |
| Base animation step attributed by reachability | 00508740 | inherited detail still WC4; do not count as a watercraft-specific public method |
| Size/sign and small route/math helpers | 00566D60, 00566E00, 00567000, 00567A50, 00567AB0, 00567B10, 00567ED0, 00567FD0, 005682F0, 0056BD50 | partial/untraced; WC3–WC5/WC7 |
| Record, query, direction choice, tuning | 00566EE0, 00566F00, 00566F30, 00566F60, 00567310 | §§1–3, §5.2; full signatures/failures still WC3 |
| Command curves | 00567080, 005671D0 | §6.2.1 |
| Land, tow, city and movement gates | 00567DF0, 00567E80, 005681B0, 005698E0 | §§5.6, 6.3; reference/failure audit still WC3 |
| Collision and bump | 00568570, 005687C0, 005690D0, 0056C090, 0056A310 | summary §5.7; exact overlap/response still WC5 |
| Position and motion helpers | 00568230, 00569940, 00569E70, 0056C430 | summary §5.2; full equations still WC4 |
| Crash motion | 0056A440, 0056AF90, 00570270 | full ramps/axes/RNG still WC5 |
| Message and movement ticks | 00569230, 005694B0, 0056BD80, 0056D310 | §§5.5, 6.1, 6.3, 7; partial WC4/WC7 |
| Teardown/docking helpers | 0056D0E0, 0056D4D0, 0056D530 | §§3–5; full failure audit WC3 |
| Construction/class/destruction | 0056D9A0, 0056DC60, 0056DD00, 0056DD20 | §§1, 3; construction/destruction body still WC3 |
| Small interface adjustors | 0056DC70, 0056DC80, 0056DC90, 0056DCA0, 0056DCB0, 0056DCC0, 0056DCD0, 0056DCE0, 0056DCF0, 0056F7B0 | Init/Shutdown adjustors in §3; other targets still WC3 |
| Initialization/state/command hooks | 0056DE40, 0056E340, 0056E5C0, 00570140 | §§3, 4, 6.2; inherited contract still WC3/WC6 |
| Wake stop/start | 0056EFE0, 0056F1E0 | summary §5.1; WC4 |
| Ferry and neighbour-route setup | 0056F3F0, 0056F480, 0056F510, 0056F670, 0056F700, 005704D0 | §§5.3–5.4; reader/errors/ownership still WC7 |
| AI selection, motion, docking | 00570620, 00571880, 00572770 | §§5.2–5.3; final approach expanded, remaining WC4/WC7 |
| Debug/animation/state dispatch | 00570AE0, 00572CF0, 00573340 | §§4, 5.1, 9; inherited animation and caller still WC2/WC4 |

## 11. Conformance tests

1. A boat group without drive parameters gets entry 21 = 8000 and entry 36 = 300; one that gives entry 9 = 3 and
   entry 8 = 0 gets entry 8 = −3.
2. A ferry 250 units from its station with the berth free pushes state 74; with the berth taken and the waiting spot
   free, state 75; with both taken, state 73.
3. A docking boat 0.5 units from the berth, turned 0.5° from the berth direction, finishes docking (the state pops).
4. With entry 7 = 0.3 and entry 27 = 0.5, a player boat meeting an AI boat at a relative speed of 0.12
   (0.4 of entry 7) bumps; at 0.18 (0.6 of entry 7) a crash happens.
5. The wandering direction helper from direction 2 produces 2, 3, 1, 4, 0, 5, -1, 6; the picker
   skips reverse direction 6. A -1 cell step leaves the cell unchanged (§5.2).

### 11.1 Command conformance vectors (WC8)

These are spec-side vectors for the engine lane to encode in its runner; no engine files were
read or changed to produce them. Use default tuning unless overridden. All command tests have
a valid group and AEMS record. Compare stored f32 values with tolerance 1e-6 except exact
flags, channels assigned from parameters, and unchanged values.

| ID | Inputs | Expected observable result |
|---|---|---|
| WC8-01 | F=-1, R=-1, v=0.1, dt=100 | Gear 0, q=0, v remains 0.1 in the translator. A separate coasting drive tick reduces v to 0.095. |
| WC8-02 | F=3000, R=150, v=0, dt=100 | Forward wins: gear 2, q=3000, target 0.3, v=0.1 (A=0.1). |
| WC8-03 | F=-1, R=2000, v=0.005, dt=100 | Gear 1, q=-2000, target -0.1, B=0.01, v=-0.005; reverse channel rewritten to 150. Speed crosses zero. |
| WC8-04 | F=3000, R=-1, v=-0.005, dt=100 | Gear 2, q=3000, target 0.3, v=0.005; forward channel rewritten to 0. |
| WC8-05 | F=3000, R=-1, v=0.3, dt=100 | Keep v=0.3; replace F with the inverse-speed curve (approximately 3000), rather than decelerating. |
| WC8-06 | L=100, R=-1, T=0, dt=10 | Negative response yields T=-0.1. |
| WC8-07 | L=-1, R=100, T=0, dt=10 | Positive response yields T=0.1. |
| WC8-08 | L=100, R=100, T=0, dt=10 | First response T=-0.1; second response returns toward zero by 0.05 and clears L: T=-0.05. This is not neutral. |
| WC8-09 | L=-1, R=-1, T=-0.2, dt=10 | Return step r=0.01: T=-0.19. |
| WC8-10 | L=0, R=-1, T=-0.2, dt=10 | Rebase L to 190, then T=-0.19. With mirrored R=0, L=-1, T=0.2, R becomes 210 and T=0.21. |
| WC8-11 | Group 0x4409, stopped skier, toggle=1, v=0 | Effect stays stopped; owner does not clear toggle. Next call with toggle=1 and v>0 can start it. |
| WC8-12 | Group 0x4409, running skier, toggle=0, v=0 | Stop effect with argument true; no toggle-byte write. |
| WC8-13 | Group missing, arbitrary channels and v | Return false without selecting a gear or changing speed/steering. Same for missing AEMS after parent fallback. |

### 11.2 Initialization and docking boundary vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC3-I01 | Already initialized; nonzero existing timers/effects | Init returns true without watercraft reset or effect acquisition. |
| WC3-I02 | First Init, valid city/effect manager; effect factory reports failure with null output | Continue subsequent acquisitions and setup, set flags 0x8000/0x2000/0x20000/4, return true; no rollback. |
| WC3-I03 | First Init with lifetime limit 1000, lifetime elapsed 500 | Both become zero, as do idle/crash/still/backwards accumulators. |
| WC3-R01 | Version 1 or 3, otherwise valid stream | Read returns false and does not call base Read. Version 2 calls base Read and propagates its success or failure. |
| WC7-D01 | Final approach, aligned heading, distance exactly 1, dt=1 and default p[39] | Translate to berth and return true. Next call at distance 0 returns false. |
| WC7-D02 | Final approach, heading difference exactly 1 degree, distance 0, dt=1 | Rotate to alignment and return true; next call returns false. |
| WC7-D03 | Final approach, distance 0.5, angle 0.5 degree | Return false before MoveTo or previous/current snapshot updates. |
| WC7-D04 | OnLand true, final approach, distance 2, aligned, dt=1 | Push 72, continue translation by 1, return true. OnLand alone does not force false. |
| WC7-D05 | Dock range exactly p[37], aligned heading, dt=200, distance=90, defaults | Final approach uses unclamped dt and reaches berth, returning true. It does not use the 100-ms general motion cap. |
| WC7-D06 | Queued, destination within request radius, berth occupied | Retain state 75, no replacement queue request or remove state. |
| WC7-D07 | Destination station position exactly p[36] away | No berth or waiting-spot request; comparison is strict. |
| WC7-U01 | Undocking: movement impossible, or unnavigable point, or null route occupant | Return false (the state dispatcher can pop 78). |
| WC7-U02 | Undocking: distance already exactly twice own radius | Return false without outward translation. |
| WC7-U03 | Undocking: target displacement 0.5 with remaining outward radius gap | Return false; target displacement must be at least 1 to attempt motion. |
| WC7-R01 | Direction helper base 0, alternatives 0 through 7 | Values 0,1,-1,2,-2,3,-3,4; negative results are not wrapped into 0–7. |
| WC7-R02 | Backtrack from six travel points, survivor direction 3 and tried mask 0x01 | Drop five points; survivor mask becomes 0x09, regardless of rejected candidate direction. |
