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
| `+0x2CB`, `+0x2CC` | being towed; visible-area spawn idle-budget mode (§5.1.2) |
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
| 2 | 1.0 | steering return rate per second (§6.2.2) | 22 | 0.7854 | sinking angle endpoint (§5.7.2) |
| 3 | 0.05 | coasting drag | 23 | 0.95 | shore-slide speed retention (§5.7.2) |
| 4 | 0.2 | movement distance scale (§5.1.1) | 24 | 2.1817 | crash attitude parameter; not the normal bow cap (§5.7) |
| 5 | 5.0 | counter-steer boost | 25 | 1.0472 (60°) | run-aground angle (automata.md §6.6) |
| 6 | −0.1 | lowest speed (reverse) | 26 | 10 | strict shore-slide terrain-height tolerance (§5.1.3) |
| 7 | 0.3 | highest speed | 27 | 0.5 | player boat-to-boat crash threshold |
| 8 | −entry 9 | lowest steering | 28 | 0.25 | player shore crash threshold |
| 9 | 2.0 | highest steering | 29 | 0.75 | speed kept after a bump |
| 10 | 0.001 | per-ms scale of the acceleration | 30 | −2000 | lower command-curve domain/output endpoint (§6.2.1) |
| 11 | 50 | forward turning-radius scale (§5.1.1) | 31 | 3000 | upper command-curve domain/output endpoint (§6.2.1) |
| 12 | 5.0 | reverse turning-radius scale (§5.1.1) | 32 | 150 | U-Drive-It reverse threshold |
| 13 | 0.001 | AI speed slew per ms (§5.1.3) | 33 | 0 | U-Drive-It forward threshold |
| 14 | 0.001 | AI turn slew per ms (§5.1.3) | 34 | 2.0 | loaded/reloaded, no owner-side read in the export |
| 15 | 0.1 | AI boat-to-boat crash threshold | 35 | 100 | longest tick, ms |
| 16 | 0.5 | AI shore crash threshold | 36 | 300 | dock request distance |
| 17 | entry 18 / entry 7 | speed-to-bow-lift gain | 37 | 90 | final approach distance |
| 18 | 0.3142 | bow-angle cap, gain numerator and twice the visual height lift | 38 | 1.0 | docking turn rate, degrees per ms |
| 19 | entry 20 / entry 9 | banking gain | 39 | 1.0 | docking move rate, units per ms |

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
contracts are expanded below; the deleting entry must not be mistaken for the complete destruction body.

### 3.1 Construction, destruction and forwarding contracts (WC3, partial)

Construction (`0x0056D9A0`) takes already allocated object storage and returns that
same pointer. It first constructs the automaton base with four zero arguments,
then installs the watercraft interface tables. It initializes station A/B/current
references and their route-storage pointers to null, both tow references to null,
the neighbour occupant and route storage to null, previous/current positions and
headings to the shared zero float, and all six effect references to null. The
per-boat random source is constructed with seed argument `0xFFFFFFFF`; this is
not evidence of a particular generator sequence. It sets inherited byte +0x33
bit `0x10` and inherited type ID +0x28 to `0x897F7EC2`.

Travel-point storage initially contains **eleven** zeroed 16-byte entries, with
begin at the allocation, end and capacity-end at allocation+176. This is an
initial populated vector, not eleven slots of empty capacity; later Init/picker
operations can clear or replace it. Allocation requests 176 bytes. Null allocation
is not converted to a false constructor result: after skipping the first entry's
stores, later entries would access addresses derived from null. Do not describe
this as successful empty-vector construction on allocation failure. Evidence:
`0x0056DB16–0x0056DB71`. Other runtime timers/flags receive their initialization
in Init (§3); the constructor does not establish every Init postcondition.

Complete destruction (`0x0056DD20`) releases each nonnull effect in reverse
offset order (+0xDC4 down to +0xDB0), frees neighbour route storage, releases the
neighbour occupant, frees travel-point storage, releases tug then towed boat,
frees the current route storage and releases the current station, then does the
same for station B and station A, and finally destroys the automaton base
(`0x00513E60`). Null references/storage are skipped. This body does **not** call
watercraft Shutdown, stop effects or return held berths; those are Shutdown's
separate lifecycle duties. Storage deallocation occurs only in the deleting
destructor when its flags' low bit is set. Evidence: `0x0056DD00–0x0056DE3D`.

The following are forwarding/interface methods, not additional simulation steps.
Their boolean results are in the low byte; other register bits are not boolean
data. Each adjustor preserves the target's arguments, result and failure behavior.

| Entry | Inputs / result | Contract |
|---|---|---|
| `0x00566F30` QueryInterface | interface ID, output pointer; boolean | For `0x096E75F5`, write the complete object pointer, AddRef through owner slot 1, return true. All other IDs delegate unchanged to automaton QueryInterface (`0x00505D80`), including its output/failure rules. The output pointer must be valid. |
| `0x0056DC60` GetGZCLSID | no arguments; u32 | Return `0x096E75F5`. |
| `0x00566EE0` Write | output record; base result | Write u16 version 2, then call base Write and preserve its result. Failure of the version write is not checked before base serialization. No watercraft runtime fields follow the base payload. |
| `0x00566F00` Read | input record; boolean | Read u16 version and accept exactly 2, then return base Read's result. A mismatching version returns false without calling base Read (§3). |
| `0x0056DC70`, `0x0056DC80`, `0x0056DC90`, `0x0056DCD0`, `0x0056DCE0` | ID, output; boolean | QueryInterface from interface pointers +4, +8, +12, +32, +56 respectively; subtract the corresponding offset before forwarding. |
| `0x0056DCA0` | deletion flags; object result | Subtract 12 then call the deleting destructor. |
| `0x0056DCB0`, `0x0056DCC0` | record; target result | Subtract 4 then forward to Write or Read respectively. |
| `0x0056DCF0`, `0x0056F7B0` | target arguments/result | Subtract 4 then forward to Shutdown or Init respectively (§3). |
| `0x00566E00` sign helper | float value; floating result | Return -1 for a negative finite value, +1 for either signed zero or a positive value. This is not a signum returning zero at zero. |
| `0x0056BD50` travel-point erase | vector receiver, pointer to an existing entry; entry pointer | Remove that 16-byte entry, shift later entries left, shorten end by 16, preserve capacity, return the original entry pointer (now the next entry, or end). No bounds validation or allocation-failure path. |

Evidence for forwarding and Write result preservation: `0x00566EE0–0x00566F53`,
`0x0056DC70–0x0056DCF7`; erase `0x0056BD50–0x0056BD7E`. These contracts expand
the inventory but do not close WC3's remaining motion helpers or inherited-step
contracts.

### 3.2 Geometry helper contracts (WC3, partial)

The following are whole-boat receiver contracts. They do not transfer ownership
of the receiver or supplied output buffers. Output storage and the inherited
services must be valid; none reports a recoverable failure for a null output
pointer, malformed bounds or a failed inherited getter.

| Entry | Inputs/result | Behavior and inherited boundary |
|---|---|---|
| `0x00567A50` | writable three-float output; void | Sample primary slot 17 GetRadius once; write `P-radius*H` using runtime P at +0x2F4 and H at +0x30C. Include all three components; do not flatten or normalize H. |
| `0x00567AB0` | writable three-float output; void | Same sampling/fields, write `P+radius*H`. Neither endpoint helper changes boat fields, transform or effects. |
| `0x00567B10` | no arguments; floating result | Obtain two vec3 bounds from the +4 occupant interface slot 10; return the largest `max-min` side, including y. No absolute value, radius division, margin or model lookup. It ignores the getter's return value. |
| `0x005681B0` | no arguments; boolean | Obtain the cell rectangle from the +4 occupant slot 11. Test city slot 99 on `(xmin,zmin),(xmax,zmin),(xmin,zmax),(xmax,zmax)` in that order. Return true at first outside corner, false if all pass. Do not push a state here: callers own the 72 push. This corner order differs from the inanimate detector. |

Endpoint x/y products are stored as floats before adding/subtracting P; the z
product remains in floating evaluation until the final output store. Bounds
x/y differences pass through float temporaries while the z difference remains
in floating evaluation for the final maximum comparison. Ordinary finite
conformance uses tolerance 1e-5; no stable repair of NaNs is specified.
Evidence: `0x00567A50–0x00567B80`, `0x005681B0–0x00568225`.

The manager collision search and shared footprint builder signatures/results
and failure paths are in §5.7.3. These contracts expand the owner inventory;
they do not certify the untraced entries in §10.1.

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
  `FindState`) returns false; the manager requests removal (§5.1.3).
9. With state 16, 74, 75, 77 or 78 in the stack, or while being towed, or with `+0x2CC`, skip to 13.
10. **Lifetime.** With a limit that the elapsed time has passed, the manager is asked (manager slot 41) whether the
    boat may stay; without a manager, on a no, or with state 76 in the stack: `PushState(73)`; on a yes the limit is
    cleared and flag `0x10000` cleared.
11. State 76 in the stack, or top state 7: skip to 13.
12. The elapsed time grows. If the boat cannot move (§5.6): on land, `PushState(72)`. Otherwise, with the frame time
    clamped to 0–entry 35: `Engine_AIBoatMotion`, `Movement_PositionNew`, `Engine_DetectCollision`,
    `Ferry_DockRequests` and `Ferry_ClearDockRequests`.
13. The effect and model updates (slots 104 and 107).

### 5.1.1 Position integration and publication (WC4)

`Movement_PositionNew` (`0x0056C430`) receives movement time from the caller;
it does **not** clamp it internally. Before integrating, clamp turn to
`[p[8],p[9]]` and speed to `[p[6],p[7]]`. Capture the current transform and
runtime position. Exact zero turn selects straight motion; nonzero turn with
nonzero speed selects curved motion. Zero speed does not turn in place: it
only invokes inherited FindState(16), discarding the result. This call is not
a state pop. Afterwards copy old runtime position to previous position and
store the integrated runtime position, even in the zero-speed case.

For straight motion (`0x00568230`) let heading be `H`, position `P`, and
`d=p[4]*v*dt`. The displacement uses the **horizontal** heading:

`N=(H_x,0,H_z)/sqrt(H_x²+H_z²)`, `P'=P+d*N`.

Write `P'` to the output position and transform translation; preserve heading.
The reciprocal length, normalized x and y, each displacement component and
each final position component pass through float stores. Thus an implementer
must not replace this with an unnormalized H or let heading y change height.
There is no zero-horizontal-heading recovery: a zero denominator propagates
invalid floating values rather than reporting false. Negative dt is used as
given. Evidence: `0x00568230–0x005682E8`.

For curved motion (`0x00569940`), define the radius using the clamped values:

| Speed | Radius R |
|---|---|
| `v>0` | `p[11] * ((v/p[7]+0.5) + (1-abs(turn)/p[9]))` |
| `v<=0` | `p[12] * ((abs(v)/abs(p[6])+0.5) + (1-abs(turn)/abs(p[8])))` |

The owner calls this helper only with nonzero speed and turn. Let
`U=(0,1,0)`. For positive turn set `S=normalize(U cross H)`; otherwise
`S=normalize(H cross U)`. Set the turn center to
`C=(P_x+R*S_x, water_level, P_z+R*S_z)`, sampling terrain slot 29 for its y.
Let `V=normalize(P-C)` using **all three components**, and
`theta=(p[4]*v*dt)/R` for positive turn, negated otherwise. Distance is stored
as float before division by the float radius. Define the yaw rotation

`Q_theta(x,y,z)=(cos(theta)*x+sin(theta)*z, y,
                 -sin(theta)*x+cos(theta)*z)`.

The new runtime position is `C+R*normalize(Q_theta(V))`. The owner saves old
heading into previous heading (+0x300) and stores
`normalize(Q_theta(H))` as current heading. Turn magnitude affects the radius;
it is not an additional multiplier on theta. No radius/normalization failure
is converted into a boolean result. For non-level input the radial normalization
includes the difference from water level; do not substitute a flat xz circle.
At dt=0 such an input can still move to radius R about C. Evidence:
`0x00569940–0x00569E61`, radius `0x00569A33–0x00569ACC`,
center/angle `0x00569AD0–0x00569BC0`.

The curved helper writes transform translation as `(P'-P)+P` componentwise,
with intermediate float rounding, rather than simply copying `P'`. It also
samples terrain slot 29 a second time when `abs(P'_z-C_z)>0.00001`, discarding
that sample. It does not return a failure flag.

After position/heading integration, call the visual attitude helper
`0x00569E70` when speed or turn is nonzero. With zero speed that helper itself
does nothing. With nonzero speed it applies banking helper `0x005682F0`, then
bow attitude, and adds `p[18]/2` to **transform** y. Runtime position y is not
changed by this lift. The banking angle is
`clamp((v/p[7])*p[19]*turn, -p[20], p[20])`, only when `v>0`;
reverse speed skips banking. The bow angle is `clamp(p[17]*v,0,p[18])`.
Entry 18 is both the default gain numerator and this helper's direct cap.
These scalar rules are verified at `0x0056830A–0x00568361`,
`0x00569EEE`, `0x00569F6F–0x00569FA3`, `0x0056A1B7–0x0056A1C3`;
the complete finite-vector attitude composition is below. This corrects the earlier
identification of entry 24 as the normal bow cap/lift.

Use the transform's 3-by-3 layout with rows at offsets (+4,+8,+0xC),
(+0x10,+0x14,+0x18), (+0x1C,+0x20,+0x24); its columns are right, up,
forward. Define `Rot(a,b)` for a unit axis a and angle b by

`Rot(a,b)V=cos(b)*V+(1-cos(b))*dot(a,V)*a-sin(b)*(a cross V)`.

The minus sign specifies this transform convention. Equivalently, with
`(x,y,z)=a*sin(b/2)` and `w=cos(b/2)`, the rotation rows are

| Row | x component | y component | z component |
|---|---|---|---|
| 0 | `1-2*y*y-2*z*z` | `2*x*y+2*w*z` | `2*x*z-2*w*y` |
| 1 | `2*x*y-2*w*z` | `1-2*x*x-2*z*z` | `2*y*z+2*w*x` |
| 2 | `2*x*z+2*w*y` | `2*y*z-2*w*x` | `1-2*x*x-2*y*y` |

At positive speed, the banking helper **replaces** the transform rotation
with `Rot(normalize(H),bank_angle)`; it does not multiply that matrix into
the old yaw. At negative speed it leaves the incoming rotation alone.
The bow helper saves `B=normalize(scale*banked_up_column)`, uses
`A=normalize(H cross (0,1,0))`, and forms
`F=normalize(Rot(A,-bow_angle)*H)`. It normalizes F a second time, then
sets `R=normalize(B cross F)` and `U=F cross R`. The final rotation columns
are `(R,U,F)`. Preserve scale and x,z translation; lift transform y by p[18]/2.
Heading H in runtime fields is not replaced by F. Thus normal visual pitch
does not feed back into the next runtime heading integration. Banking about
H supplies an up reference; yaw is reconstructed from H and the pitched F.

All matrix entries, vectors at normalization boundaries and final components
are floats; the implementation uses half-angle sin/cos and doubled quaternion
products rather than a general matrix library's arbitrary Euler order. For
conformance allow the section 11 tolerance, not bit equality to a reassociated
Rodrigues expression. Each helper marks rotation changed (flag 4, incrementing
the transform revision byte); the bow helper also marks translation changed
(flag 1, another increment). At speed zero neither helper changes the transform.
There are no finite fallbacks for zero H, vertical H, zero scale or parallel
B/F: reciprocal-length division can produce invalid values. Evidence:
`0x005682F0–0x005684BA`, `0x00569E70–0x00569FA3`,
`0x0056A0FD–0x0056A2FD`.

Finally, publish the transform through slot 75, copy runtime position and heading
to inherited +0x40/+0x58, then call slot 100 `(true,true)`, slot 104, slot 107,
in order. Publication still occurs at zero speed. These calls' results are
ignored; Movement_PositionNew has no success/failure result
(`0x0056C761–0x0056C84A`). AI speed/turn selection and inherited animation
still require WC4; the integration equations do not close the whole tick.

### 5.1.2 Visible-area idle mode (WC1)

`Init` clears byte +0x2CC. The external writer is the watercraft manager's
`PopulateVisibleArea(float dt)` (`0x005723D0`), after successful boat creation
and placement: it writes lifetime limit 45000 ms, elapsed lifetime zero and
the spawn mode byte to +0x2CC (`0x00572718–0x0057272C`). Failed creation does
not perform these writes. This is a spawn attribute, not a stillness detector.

The mode begins false for each population attempt. When manager slot 13 is
true, an available RNG first admits the attempt only for a uniform double draw
`r<=0.05000000074505806`. A second available draw selects mode true exactly
when `r>0.5`; equality keeps false. A missing RNG leaves false and does not
reject the attempt. When slot 13 is false, an available RNG admits only
`r<=0.10000000149011612`, with mode false and no mode-selection draw. These
decisions precede the edge-cell loop; the mode applies to all successful
boats from that attempt. True chooses creation group `0x4417`, false chooses
`0x4418`; both pass creation flags `0x10002` (`0x00572462–0x005724CC`,
`0x0057262E–0x00572683`). No additional byte-sized +0x2CC writer is present
in the exported instruction inventory.

While true, animation adds the **second** frame time to +0x2BC even while
moving; after the inherited step it bypasses ordinary lifetime accounting
and autonomous movement (§5.1 step 9). The idle threshold is tested whether
or not the byte is set: accumulated idle strictly above 60000 ms pushes 73;
exactly 60000 does not. The early speed-modifier return precedes accumulation.
The ordinary stillness timeout remains independent and can return false first.
Clearing the mode itself does not clear an existing idle accumulator
(`0x00572D14–0x00572D58`, `0x005730A6`).

### 5.1.3 Animation caller and remaining tuning uses (WC2)

The manager's shared animation pass (`0x0050A7B0`) calls automaton slot 72
with the original two float frame times when the +4 occupant interface slot 5
permits ticking. A false result immediately calls manager slot 8
`RemoveAutomaton(boat,false)` (`0x0050A883–0x0050A8AD`). The manager obtains
the next list link before invoking the boat, so removal does not invalidate its
iteration. False is a removal request, not an implicit PushState(73), PopState,
retry, or an ignored return. Watercraft's manager table supplies slot 8 at
`0x0056F800` (table `0x00A9DF38`). This dispatch is also used by the
watercraft-specific manager tick through its inherited animation pass.

Entries 13 and 14 independently slew current speed and turn toward their
AI-selected targets. For finite nonnegative dt and nonnegative rates, define
`approach(x,target,s)=target` when `abs(target-x)<=s`, otherwise
`x+sign(target-x)*s`. Speed becomes
`clamp(approach(v,target_v,dt*p[13]),p[6],p[7])`; turn then becomes
`clamp(approach(turn,target_turn,dt*p[14]),p[8],p[9])`. Equality snaps to the
target. The products are evaluated separately, not a shared acceleration
or a multiplication by p[10]. The helper receives one float dt, returns no
boolean, and does not clamp dt internally. Callers supply its movement-time
limit where documented. Negative time/rates do not get repaired: the ordered
comparison and signed increment can move away from the target. Target
selection itself remains WC4. Evidence: `0x005721C4–0x0057230A`.

Entry 26 is a **world-height** tolerance, not a speed or angle. In the early
shore-slide part of helper `0x0056AF90`, form proposed horizontal position
`(P_x,P_z)+dt*p[4]*v*(H_x,H_z)` with its supplied dt. Only when city slot 98
accepts that point, map it through city slot 95 to a cell and sample terrain
slot 18 there. The slide's contact path requires
`abs(sampled_height-P_y)<p[26]`; equality fails and clears the contact byte
+0x2CA through the existing crash fallback. The gate precedes the contact
transform/publication. This identifies the parameter's use without claiming
the still-unresolved contact basis. Evidence: `0x0056B23E–0x0056B2F7`,
fallback `0x0056B585`.

Entries 22–24 are the crash angle/retention parameters in §5.7.2; 24 has no
normal bow-lift role. Entries 30–31 supply the signed endpoints in both command
curves (§6.2.1). Entry 34 is stored at +0x3CC by tuning load/default setup;
there is no read of that watercraft field in the exported owner/helper
instructions. Retain it for tuning fidelity; no observed movement, collision,
timer or effect behavior is to be invented for it. This is an export-level
negative finding, not a guessed semantic label.

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
itself change the current-crossing record or its held-berth byte +0x274. See §5.4.1 for reservation ownership.

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

#### 5.4.1 Route encodings and reservation ownership (WC7)

The wandering direction numbering is distinct from the compressed reader's four cardinal codes:

| Wandering code | Cell delta (x,z) | Horizontal heading |
|---|---|---|
| 0 | (-1,-1) | diagonal -x/-z |
| 1 | (0,-1) | -z |
| 2 | (1,-1) | diagonal +x/-z |
| 3 | (1,0) | +x |
| 4 | (1,1) | diagonal +x/+z |
| 5 | (0,1) | +z |
| 6 | (-1,1) | diagonal -x/+z |
| 7 | (-1,0) | -x |

Cell deltas are from `0x00567000–0x0056705D`, table `0x00567060`. World positions
come from city slot 97's cell-to-world conversion; route x/z are cell indices, not raw
world coordinates. For the heading-to-direction helper, normalize the stored current
heading (+0x30C). With x positive, use acos(x) in degrees; with x negative use acos(-x).
In each half-plane, angle <22.5 gives the x cardinal, [22.5,67.5) gives the diagonal
selected by z's sign, and >=67.5 gives the z cardinal. Zero z selects the x cardinal;
zero x selects code 5 for positive z and code 1 otherwise. Normalization includes y,
so a tilted heading must not be treated as a normalized horizontal vector. Zero length
has no repaired direction. Evidence: `0x00567FD0–0x005681AD`, constants 22.5/67.5 at
`0x00A9DA88`/`0x00A9DA84`.

**Compressed route consumed by this owner.** SetData references a byte vector and resets
the reader; the owner copies route bytes before binding it, so the reader is not an owner
of a second byte buffer. An empty vector is exhausted immediately. A nonempty stream begins
with three bytes: low six bits of a descriptor, unsigned initial x, unsigned initial z.
GetNext returns the current coordinate, then advances its internal coordinate for the
following call. Coordinates are stored with 16-bit wrapping arithmetic and are consumed
by the boat as signed 16-bit cell indices. At the end of a complete stream, the current
endpoint is returned once with outgoing direction 0xFF; only the next call returns false.
False does not supply a new coordinate. Evidence: `0x006D64B0–0x006D6507`,
`0x006D6510–0x006D6567`, `0x006D66C0–0x006D66E8`.

Each following descriptor's low six bits are metadata. With bit 6 set it is followed
by an unsigned count and ceil(count/4) packed direction bytes, least-significant two-bit
direction first. Codes 0/1/2/3 step (-1,0)/(0,-1)/(1,0)/(0,1). A zero count terminates
at the current endpoint, returning it successfully once. With bit 6 clear the descriptor
is followed by unsigned absolute x and z bytes; those replace the reader's next coordinate,
with bit 7 retained as metadata. A run descriptor clears that metadata bit. The watercraft
uses the emitted coordinates, not these metadata/direction fields, when appending travel
points (tried mask zero, direction -1). There is no malformed-input false result distinct
from exhaustion: the reader checks cursor equality with end only at descriptor boundaries.
Truncated headers, absolute records or packed runs can read beyond the supplied buffer;
they are not validated input. Do not interpret a false GetNext as a safe truncated-data
diagnostic. Evidence: `0x006D656A–0x006D65D6`, `0x006D6681–0x006D66E8`,
`0x00570902–0x005709C4`.

On the first ferry-route fill after initialization, discard the recorded used-cell count
of reader results (initially 2), then append further successful results and increment that
count. Exhaustion with no buffered travel points makes picking return false; exhaustion
with buffered points retains them and returns true after the fill attempt. There is no
automatic removal or station switch just from reader exhaustion. The neighbor reader also
returns false when empty; successful points are appended without the ferry used-cell count.
The route picker tries to fill to ten points. Its signed remainder/backtracking rules are
in §5.2. Evidence: `0x00570890–0x005709C4`, `0x00570A9D–0x00570ADC`.

SetNbrRouteData retains a different nonnull occupant before releasing the old one; identical
pointers do not change reference counts. A null occupant is allowed. It sets neighbor-route
mode, copies the bytes, resets the neighbor reader, then consumes reader results while their
3D distance from the occupant is <=3 times this boat's radius. The test point uses the city's
world x/z for that cell and terrain slot 29's water height. The first result strictly beyond
that distance is consumed too; subsequent filling starts with the next result. With no occupant
it does no initial skipping. No unconsumed-point rewind occurs. Evidence: `0x0056F510–0x0056F667`.

**Berth bookkeeping is two distinct records.** The manager's reservation belongs to the
station lot passed to RequestBerth: the destination selected by OtherStationInformation.
RequestBerth returns false for an absent station entry or occupied berth; otherwise it stores
a retained boat in that station's berth slot and returns true. It does not change the boat's
current-crossing station or held byte. Queue acquisition similarly uses the destination's
queue slot. Thus successful destination acquisition during a crossing does not relabel the
departure record. Docking reads the destination's stored berth position/direction; request
and departure-clear distances use the station lot's position instead. Evidence:
`0x0056CEC0–0x0056CF47`, `0x0056CF50–0x0056CFD7`, `0x005704D0–0x00570611`.

The owner sets held-berth +0x274 to true in InitializeCurrentRoute, unconditionally, even
without a manager acquisition call. SetStationInformation(start=true) does this for station A
and sets docking-in-progress +0x2B0. The manager's CreateFerry separately records the new boat
in the selected starting station's berth slot after setting station information; it is not
an owner-side RequestBerth call (`0x00570F93–0x00571037`). At docking exit,
SwitchActiveStation chooses the other station and reinitializes the route, making the reached
station current and setting held-berth again. Missing either station removes the boat.

ClearDockRequests requires a current station, held byte, and distance strictly greater than
p[36]. Once outside that radius it clears +0x2B0 before checking for a manager. With a manager,
it calls DoneDocking for the **current** station and this boat, clears held-berth, and clears
flag 0x10000000. Without a manager it leaves held-berth and that flag unchanged. Equality
does nothing. EndDocking has the same manager-dependent held-byte and flag clearing,
without the distance test; it always clears +0x2B0 once its station/held gate passes.
Its DoneDocking call likewise names the current station. These are not
implicit releases of every berth the manager might hold for that boat. Evidence:
`0x0056D4D0–0x0056D525`, `0x0056D530–0x0056D5EC`, `0x0056F3F0–0x0056F47C`.

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
   (`0x00B22A7C` clear). The other boat is the first in the manager's list passing the maximum-extent distance gate below whose
   footprint overlaps (`0x005690D0`). A crash happens when |speed − other speed| / entry 7 is above entry 27 (this
   boat player-driven) or entry 15. A player crash plays AEMS event 7. The boat with the lower or equal size class
   (`0x00566D60`) sinks: a player-driven loser gets `NotifyAutomatonEvent(0xB, 0)` and `PushState(71)` for entry 21
   ms (and "sunk by a collision"); an AI loser fades out (`PushState(73)`), unless the winner is the player's boat,
   in which case it sinks. Below the threshold the boats bump (`0x0056A310`): the other boat takes a new speed when
   it can move and this boat keeps entry 29 of its speed (the transfer is the caller's speed, not a mass-weighted exchange).
3. **The shore and buildings** (`Crash_InanimateCollisionDetection`, `0x005687C0`): a hit faster than entry 28
   (player) or entry 16 (AI), as |speed| / entry 7, plays AEMS event 7 for the player and runs `Crash_DetermineCase`
   (automata.md §6.6); slower hits bump.

#### 5.7.0 Boat selection, size and response (WC5)

Use the **top** state equal to 16 to identify player drive in this detector; merely
finding 16 lower in the stack is insufficient. The caller's player state or the
global AI-collision enable admits the manager scan. It does not filter candidate
boats by their player state. Null candidates and the caller itself are skipped;
scan order is the manager list's order, not closest distance.

For boat `i`, let `L_i` be the largest of the three side lengths of the bounds
returned by its +4 interface slot 10 (`0x00567B10`). The broad-phase gate is
`length(P_A-P_B) < max(L_A,L_B)`, with all three coordinates participating.
It uses neither a sum of radii nor the sum of the two lengths. A distance equal
to the maximum fails. The first admitted boat whose oriented footprints overlap
is returned; no boat yields null (`0x005690D0`, gate `0x00569195–0x005691CC`).

The footprint overlap helper (`0x00506450`) returns false if either footprint
is invalid. For finite valid rectangles with centers `C_A,C_B`, unit axes
`e_i,f_i` and half extents `h_i,k_i`, it accepts exactly when all four separating
axis inequalities hold, for `n` in `{e_A,f_A,e_B,f_B}`:

`abs(dot(C_B-C_A,n)) <= h_A*abs(dot(e_A,n)) + k_A*abs(dot(f_A,n))
                       + h_B*abs(dot(e_B,n)) + k_B*abs(dot(f_B,n))`.

Edge and corner contact therefore overlap. This is a two-dimensional footprint
test, after the three-dimensional distance gate. The original tests axes in
A-first-axis, A-second-axis, B-first-axis, B-second-axis order and rejects on the
first positive separation (`0x005064E6–0x005065F9`). Construction is specified
in §5.7.3; it uses model bounds rather than the broad-phase occupant bounds.

Size class is 3 when the group has tag `0x4414`, otherwise 2 when it has tag
`0x4413`, otherwise 1; tag `0x4414` wins if both occur. This classification
requires a group and does not provide a null-group fallback (`0x00566D60–0x00566D9D`).
For an admitted overlap, the speed threshold is **strictly**
`abs(v_A-v_B)/p_A[7] > (p_A[27] if A is player-driven, else p_A[15])`.
It compares scalar speeds, even when headings oppose. Equality bumps. Above it:

| Player involvement | Size relation | Result, in order |
|---|---|---|
| Neither boat | A smaller | A pushes 73; B unchanged. |
| Neither boat | A larger | B pushes 73; A unchanged. |
| Neither boat | Equal | A pushes 73, then B pushes 73. |
| At least one player | A smaller | A sinks; B unchanged. |
| At least one player | A larger | B sinks; A unchanged. |
| At least one player | Equal | A sinks, then B sinks. |

Here "sinks" is an explicit `(0xB,0)` notification followed by PushState(71)
with **that victim's** `p[21]` duration and the timed-state flag set. A player
victim additionally gets +0x2B2 set to 1 before that notification; an AI victim
does not. In particular, when a player wins, the AI loser really enters 71,
but does not acquire the collision-mushroom byte. Existing bytes are not cleared
on AI victims. State entry independently sends its own `(0xB,0)`, so the explicit
notification and the entry notification must not be collapsed into one event.
If A is player-driven, AEMS event 7 receives slot 6 then slot 5 before victim
transitions; this detector does not issue that sound for a player B while A is AI.
Evidence: `0x0056C176–0x0056C363` (size comparisons and player branches).

Below/equal threshold, roll A back (§5.7.2), sample A's current speed `v`, and
ask whether B can move. If yes, set B's speed to `v`; otherwise leave B's speed
unchanged (including any state push from its movement gate). Then set A's speed
to `p_A[29]*v`. There is no direction projection, exchange, RNG or overlap-based
impulse (`0x0056C36A–0x0056C396`). Continue to shore detection using this reduced
speed. A shore bump only rolls position back; it does not apply this attenuation.

The detector returns true for outside-city removal, a boat crash, or a shore
crash, and false for no hit or only bumps. A boat crash returns before shore
detection. Outside-city removal pushes 72 before all boat/shore work.

#### 5.7.1 Crash dispatch and effect prerequisites (WC5, partial)

Crash entry sends event (0xB,0), independently of any duration in the state record.
The owner first calls base HandleEnterState and returns its result; the watercraft
entry effects still run even if that result is false (`0x0056E340–0x0056E54D`).

DoStateTick adds dt to crash elapsed before calling DoCrashSequenceTick. Inside that
call, first test outside-city footprint; outside pushes 72 and skips crash animation.
Otherwise, for either top state 70 or 71, add dt **again** to crash elapsed, stop wakes,
and retarget the speed tween to zero if necessary (sample start, target/elapsed/duration
zero). Thus an ordinary in-city crash state tick advances the accumulator by twice dt.
Calling DoCrashSequenceTick directly advances it only once. The idle-timeout fallthrough
can call it after pushing 73, in which case the second addition does not occur because
the top state is neither 70 nor 71. Evidence: `0x005733A2–0x005733BD`,
`0x00570270–0x0057036D`, `0x0057040D–0x00570451`.

State 71 places/starts the mushroom effect at the complete boat transform when +0x2B2
is set, then places/starts splash if present, then calls sinking motion helper 0x0056A440.
State 70 places/starts grounding, then calls heel motion helper 0x0056AF90 with dt.
Stopped effects receive transform then start(false); already running effects receive
transform only. Splash is optional. Grounding is required in state 70; mushroom is
required when its +0x2B2 gate is set: the null tests in those paths lead to a refresh
that dereferences the missing effect, rather than safely skipping it. Missing effects
do not turn this into a false-return recovery path. Evidence: `0x0057036D–0x005704BD`.
The remaining rotation-basis and shore-contact contracts still require WC5;
this dispatch contract alone does not specify those motion helpers.

#### 5.7.2 Crash angle ramps and bump rollback (WC5, partial)

Let `t` be crash elapsed at +0x2C0 **after** the dispatch additions, `T=p[21]`,
`a` the old angle at +0x2C4, `A=p[22]` and `B=p[25]`. These are float fields;
the original uses x87 intermediates and rounds when storing them. Neither helper
clamps elapsed or completes a last frame at its endpoint.

| Helper | Angle update | Boundary result |
|---|---|---|
| Sinking, `0x0056A440` | Store `a' = ((t/T)*A - a) + a`. | If `a' >= A`, return false without publishing a new transform, heading, effects or model. The angle store still occurs. Otherwise publish the crash transform and return true. |
| Grounding/heel, `0x0056AF90` | First reject `t >= T`. While the sliding/contact byte +0x2CA remains set and `t < T/2`, store `a' = a + ((t/T)*B-a)`; otherwise store `a' = a + (2.5*B*(t/T)-a)`. The difference inside parentheses is rounded to float before addition. | If `a' >= B`, return false without the final heel transform. The earlier contact/slide phase can already have changed speed/position and published its transform. Otherwise perform the heel rotation and return true. |

This is a ramp in **total crash elapsed**, not integration of frame time into angle.
In ordinary finite arithmetic the old angle cancels, but retaining the subtraction
and addition matters for rounding. The fast heel branch can finish at `t=0.4T`;
the contact branch switches to that branch at exactly `T/2`. Losing the contact byte
earlier can therefore cause immediate completion in the same call. No random number
draw occurs in either helper. Evidence: `0x0056A511–0x0056A562`,
`0x0056B58B–0x0056B61E`, constants `0x00A949FC=-0.5`,
`0x00A95530=2.5`; contact clearing at `0x0056B585`.

Sinking begins its vertical displacement only for `a' > c*A`, where
`c=float32(0.6)=0.6000000238418579`. Let `W` be terrain slot 29's level and
`H` terrain slot 13's height at the boat's current x,z. Define
`D=min((W-H)*0.5,0)` and `u=(a'-c*A)/(c*A)`. The published transform's y is
`W-u*D`, whereas the runtime position field +0x2F8 is written as `W-D*(t/T)`.
These are distinct writes, not an instruction to synchronize the two y values.
The old runtime position is copied to +0x2E8 first; x,z stay unchanged. At exactly
`a'=c*A` neither translation nor the old-position copy happens. Both samples of
`W` are separate terrain calls; the equations assume the level is unchanged
between them. Evidence: `0x0056ADE2–0x0056AF23`. This formula preserves the
original sign and minimum even for terrain heights above the level; it must not
be silently changed into a positive-depth clamp.

After a successful sinking transform, current heading is copied into inherited
heading +0x58, then slots 100 `(true,true)`, 104 and 107 are called, in that order
(`0x0056AF23–0x0056AF6F`). The axes and complete transform composition remain
unresolved; the scalar ramps above do not close that gap.

The bump rollback helper `0x0056A310` restores runtime position from previous
position (+0x2E8), preserves the current transform's orientation, replaces its
translation with that restored position, publishes it through slot 75, copies the
position to inherited +0x40, then calls slots 100 `(true,true)`, 104 and 107.
It does not itself change speed, turn or heading; collision callers apply their
speed response separately. Thus bump rollback is not a reversal of the whole
movement state. Evidence: `0x0056A310–0x0056A434`.

#### 5.7.3 Model-to-footprint construction (WC3/WC5)

The shared builder `0x0050A310` takes a destination footprint, an automaton,
three float offset components by value, and a float extent multiplier; it
returns the destination address. The watercraft manager search `0x005690D0`
takes `(boat,offset_x,offset_y,offset_z,multiplier)` on a **manager** receiver,
not a boat receiver. It constructs the caller footprint once with those
arguments and each candidate with `(0,0,0,1)`. Normal collision detection also
uses `(0,0,0,1)` for the caller (`0x0056C0FC–0x0056C11D`).

Use automaton slot 82 to obtain the model and the model's geometry interface
slot 7 for local minimum/maximum bounds. Let `dx=max_x-min_x`,
`dz=max_z-min_z`. Query the automaton for interface `0x4A416254`: on success
choose margins (2,1), otherwise (0.5,0.5), to subtract from (dx,dz). Release any returned
interface even if the query result is false. There is no clamp of negative
dimensions. The query changes the margins; it does not supply the geometry.
The geometry interface's slot 9 supplies the model transform, initialized to
identity before the call. The builder ignores both geometry call results.

For translation T and rotation values at transform offsets +4 and +0xC,
the 2D rectangle is:

- Center `C=(T_x+offset_x,T_z+offset_z)`. The y offset and the bounds' local
  center are ignored; offsets are added in world coordinates without rotation.
- Half extents `h=(dx-margin_x)*0.5*multiplier`,
  `k=(dz-margin_z)*0.5*multiplier` (dx,dz above are before margin subtraction).
  This multiplier does not scale the center or alter the broad-phase gate.
- First axis `e=(R_at_+4,-R_at_+0xC)/length(R_at_+4,-R_at_+0xC)`;
  the perpendicular axis is implicit in the overlap test. Model scale is not
  multiplied into the extents. No zero-projection fallback is supplied.

Differences, adjusted dimensions, centers, half extents, reciprocal axis length
and final axis components pass through float storage. In particular dz*0.5 is
stored before multiplying by the supplied multiplier. Evidence:
`0x0050A37A–0x0050A3F6`, `0x0050A46F–0x0050A505`; constants at
`0x00A8825C`=2, `0x00A81228`=1, `0x00A84D2C`=0.5.

Failure is **not** a safe empty-footprint result: the builder clears six floats
at destination +4 through +0x18, but does not clear its validity byte or axis.
Only the successful geometry path writes validity=true and the axis. A null
model or absent geometry interface returns the destination with those fields
retained. The manager's scratch footprint is not initialized to a known valid
byte before this call. Consequently missing-model overlap has no stable
true/false contract to reproduce; do not invent a guaranteed noncollision.
Ordinary collision conformance uses a valid model and nonzero projected axis.
Evidence: `0x0050A31E–0x0050A374`, `0x0050A4E0–0x0050A51E`,
`0x005690D0–0x00569109`.

#### 5.7.4 Inanimate corner detection (WC3/WC5)

`Crash_InanimateCollisionDetection()` returns a boolean and, on a hit, writes
two crash-direction bytes +0x2C8/+0x2C9. Obtain the **occupant cell rectangle**
through the +4 interface slot 11, not the oriented model footprint. For
`(xmin,zmin,xmax,zmax)`, examine corners in this order:
`(xmin,zmin)`, `(xmin,zmax)`, `(xmax,zmin)`, `(xmax,zmax)`.
First scan all four for an occupant-grid slot 6 hit, passing third argument 0;
stop at the first hit. Only with no occupant hit and docking byte +0x2B0 clear,
scan water-navigability slot 8 with arguments `(x,z,0,0)` and select its first
false result. Thus a later occupant corner outranks an earlier unnavigable
corner. Docking bypasses the navigability scan, not the occupant scan. No
selected corner returns false and leaves the two direction bytes unchanged.
No null-service recovery or city-boundary test occurs inside this helper;
Engine_DetectCollision tests outside-city first (§5.7.0).

Classify inherited heading `(H_x,H_z)` from +0x58/+0x60, independently of speed:

| Index | Dominant component | x sign | z sign |
|---|---|---|---|
| 0 | `abs(x)>abs(z)` | x>0 | z<0 |
| 1 | same | x>0 | z>=0 |
| 2 | same | x<=0 | z<0 |
| 3 | same | x<=0 | z>=0 |
| 4 | `abs(x)<=abs(z)` | x>0 | z<0 |
| 5 | same | x>0 | z>=0 |
| 6 | same | x<=0 | z<0 |
| 7 | same | x<=0 | z>=0 |

The selected corner and index write the following `(byte_2C8,byte_2C9)`;
every cell returns true, including `(0,0)`. Equal magnitudes take the z-dominant
half; zero x is nonpositive and zero z is nonnegative. A zero heading therefore
uses column 7, without normalizing or raising an error.

| Corner | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| 0 | 1,0 | 1,0 | 0,1 | 0,1 | 0,0 | 1,1 | 0,0 | 1,1 |
| 1 | 1,1 | 1,1 | 0,0 | 0,0 | 1,0 | 0,1 | 1,0 | 0,1 |
| 2 | 0,0 | 0,0 | 1,1 | 1,1 | 0,1 | 1,0 | 0,1 | 1,0 |
| 3 | 0,1 | 0,1 | 1,0 | 1,0 | 1,1 | 0,0 | 1,1 | 0,0 |

Evidence: scan/order `0x005687D7–0x00568890`; classification
`0x00568897–0x00568912` and equivalent blocks for the other corners;
jump tables `0x00568B20–0x00568B9F`; result stores
`0x00568919`, `0x005689B1`, `0x00568A49`, `0x00568AE1`, `0x00568AF7`.
Several result tails are exported as separate functions: reading only the
attributed helper's disassembly omits those writes.

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
Evidence: `0x0056E97D–0x0056EBAE`.

#### 6.2.3 Command side effects and completion (WC6)

The owner processes speed, fire, skier, steering, tow, final steering clamp, heading/rudder,
speed tween, then the shared automaton translator, in that order. Time here is the supplied
command time, without the movement clamp. There is no second acceleration pass using leftover
braking time: the single speed update in §6.2.1 can cross zero. The shared translator's true
result is the successful endpoint; the missing-group/AEMS failures in §6.2.1 are the only
ordinary false returns. This owner does not choose an engine sound from the magnitude of
throttle: the group AEMS lookup is a prerequisite and the shared translator handles the
command event described below. Evidence: `0x0056E5E3–0x0056E601`, `0x0056EFBF–0x0056EFDB`.

**Fire.** A nonnegative fire channel (+0x98) invokes the model's slot-5 hook. Only an armed
byte +0x1FF requests the named model action `whalefire` (slot 12, two zero options).
On successful action start, reset the cooldown accumulator +0x1E0, clear the armed byte,
and increment the shot count +0x178. Failure or an unarmed byte leaves these three fields
unchanged. The owner does not compute a projectile world position or place a separate
fire effect: placement belongs to the named model action. A negative channel skips this
entire fire step. Evidence: `0x0056E8A0–0x0056E959`.

**Skier placement.** Both the start and refresh use the boat's complete current transform
(GetTransform, slot 12), supplied directly to `waterski` slot 8. There is no owner-side
distance-behind offset. Starting uses effect slot 3 with argument false; stopping uses
slot 4 with true. A missing effect skips the skier processing. Group membership, rather
than exact group ID equality, gates this feature. The effect asset supplies any visual
offset. Evidence: `0x0056E959–0x0056EBAE`, notably `0x0056EA6B–0x0056EA87`.

**Tow button.** Exact group ID 0x4410, exactly zero speed, and byte +0xED nonzero gate
arming or ending a tow. With no towed boat, set the armed byte even if `tugropethrow`
is absent. When present and stopped, give it the current complete boat transform and
start it with false. With a towed boat, end the tow instead. Independently of that button
gate, a running rope-throw effect gets the current boat transform every command call.
Neither this button byte nor the armed byte is cleared by the command handler.
Evidence: `0x0056EE30–0x0056EEC2`.

**Rendered steering and heading.** Clamp the final steering to p[8]–p[9], storing it in
both drive-state +0x5C and boat +0x2E0. Rudder fields are +0x238 = -0.125 times that
steering, +0x23C = 0, +0x240 = f32(2π). For normalized heading h, let a = acos(h.z).
The heading field +0x244 is a when h.x >= 0, otherwise f32(2π)-a; it is measured in
radians and is not reduced again at the 2π endpoint. This preserves the two successive
subtractions used by the negative-x case. Evidence: `0x0056EEC2–0x0056EF87`.

**Speed tween.** If the stored target differs from abs(v), sample the existing tween
into its start value, set its target to abs(v), and set both elapsed and duration to zero.
Equal targets leave the tween untouched; there is no tolerance test. Evidence:
`0x0056EF8D–0x0056EFBF`. Thus the 2000 ms duration set on entering player drive is not
preserved when this target changes.

**Shared command bookkeeping.** The final call consumes the modified drive-state record
and dt. For each of its 13 channels i, first set edge byte +0xE4+i iff the float difference
`previous-current` is strictly between 0 and 250 (current at +0x7C+4i, previous at
+0xB0+4i). Then copy current into previous. A channel is in its blocked/cooldown condition
when its signed integer duration (+0x128+4i) is -1 or exceeds its signed integer limit
(+0x15C+4i). In that condition, an unset armed byte (+0x1F8+i) accumulates dt into
its timer (+0x1C4+4i) while that timer is below its float interval (+0x190+4i);
otherwise it arms and resets the timer to zero. This is a pre-increment comparison,
so crossing the interval arms on the next call. An already armed byte stays armed.
Outside that condition clear the armed byte; if current is not the exact -1 sentinel,
dispatch AEMS event 0x14 through slots 6 then 5. This call returns true. Evidence:
`0x00505730–0x00505778`, `0x00509070–0x0050912A` (integer duration comparison verified
at `0x0050909C`, `0x005090B2`).

**Drive-step ordering.** Before the owner drive tick, base drive returns false if the
removal predicate (slot 31, true) holds; otherwise it advances every nonnegative channel
by dt, saturating at FLT_MAX, invokes virtual command translation, and updates the two
terrain cell coordinates at drive-state +0x26C/+0x270 from the boat's x/z position.
The owner ignores that base return. It then ends any tow whose normalized direction
from this boat to the towed boat has positive dot product with this heading, performs
neutral drag, checks movement permission, and only then clamps dt for movement and
collision. Its return is always true, including when movement is denied. The drag uses
the original dt, even above p[35]; channels must both equal -1 for drag to apply.
Evidence: `0x00508C50–0x00508CF1`, `0x0056D310–0x0056D4C7`.

### 6.3 Towing

A tug's armed tow is used by message `0xAC400A31` (§7). A towed boat (state 77, `DoFollowingTick`, `0x005694B0`)
returns false without a tug. With a tug it first sets live position height to terrain slot
29's water level, and otherwise returns true, including when no translation occurs.

Define a boat's back/front points as stored current position (+0x2F4) minus/plus its own
radius times stored current heading (+0x30C). Let A be the tug's back point with y replaced
by water height, C this boat's front point, V=A-C, L=length(V), and g=L-(this boat's radius+10).
The desired heading is the horizontal direction from this boat's **live position** to the
tug's stored back point. Normalize it and the live heading, then compute their acos angle
in degrees (no dot clamp); NaN angle is replaced by zero. Only when (angle>=1 or L>=1) and
g>0 does a motion update occur. Translate the live position by `(V/L)*g` in that single call;
there is no dt factor and no p[39] speed cap. Then turn the live heading about Y by the signed
minimum of angle and dt*p[38] degrees, using the cross-product sign and rotation equations
in §5.3.1 (zero cross product chooses positive). Publish MoveTo and shift current snapshots
to previous before recording the new live position/heading. A slack rope causes no turning
or snapshot publication, even when heading is wrong. dt is not clamped; it affects turning
only. Evidence: `0x00567A50–0x00567B05`, `0x005694B0–0x005698DF`.

This is an endpoint-gap constraint using the **follower's** radius+10, not a target distance
between boat centers or a movement rate. Coincident/zero direction vectors have unguarded
normalization; no fallback heading is specified. Antiparallel finite headings use a positive
rotation. Exiting 77 clears flag 0x10000000 and releases both tow references, but does **not**
clear being-towed +0x2CB (`0x005701AB–0x0057021B`).

## 7. Messages (`DoMessage`, `0x00569230`) *[compat]*

- `0x26D31EC1`: reload the drive parameters.
- `0xAC400A31` while the tow is armed: disarm it; take the world point of the message's payload; when the pick
  is valid and a manager exists, the first matching boat at that point (`0x00568C10`), not this one, in group `0x4417`, and
  not in state 70, 71, 72 or 73, becomes the towed boat: the tug is recorded in it, a boat that was docking, queued
  or parked (74–76) pops that state, it pushes state 77, and this boat's own tug reference is released
  *[inferred: the payload is the rope's landing point]*.
- Anything else goes to the automaton's handler.

The tow-point selector is **not a nearest-distance search**. Iterate manager-list order,
skip null entries and the supplied exclusion pointer, and return the first boat whose 3D
distance from the pick is strictly below twice **that candidate's** radius. Distance equality
does not qualify. It returns a borrowed pointer or null. Group/state/self filters are applied
by DoMessage after this selection, not used to seek another candidate. Thus a first candidate
failing those filters prevents a later eligible candidate from being selected. Equal distances
have no special tie handling: list order wins. Evidence: `0x00568C10–0x00568CD0`,
constant 2 at `0x00B0A158`, caller `0x00569230–0x005694A5`.

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
| Collision and bump | 00568570, 005687C0, 005690D0, 0056C090, 0056A310 | §§5.7.0–5.7.2 give boat selection/response, SAT and rollback; model-to-footprint and full shore path still WC5 |
| Position and motion helpers | 00568230, 00569940, 00569E70, 0056C430 | §5.1.1 gives straight/curved equations and publication; full visual attitude still WC4 |
| Crash motion | 0056A440, 0056AF90, 00570270 | §§5.7.1–5.7.2 give dispatch, ramps, height writes and endpoints; rotation/contact still WC5 |
| Message and movement ticks | 00569230, 005694B0, 0056BD80, 0056D310 | §§5.5, 6.1, 6.3, 7; partial WC4/WC7 |
| Teardown/docking helpers | 0056D0E0, 0056D4D0, 0056D530 | §§3–5; full failure audit WC3 |
| Construction/class/destruction | 0056D9A0, 0056DC60, 0056DD00, 0056DD20 | §§1, 3.1; construction/destruction and allocation failure described |
| Small interface adjustors | 0056DC70, 0056DC80, 0056DC90, 0056DCA0, 0056DCB0, 0056DCC0, 0056DCD0, 0056DCE0, 0056DCF0, 0056F7B0 | §3.1 gives every offset/target, argument/result forwarding |
| Initialization/state/command hooks | 0056DE40, 0056E340, 0056E5C0, 00570140 | §§3, 4, 6.2; inherited contract still WC3/WC6 |
| Wake stop/start | 0056EFE0, 0056F1E0 | summary §5.1; WC4 |
| Ferry and neighbour-route setup | 0056F3F0, 0056F480, 0056F510, 0056F670, 0056F700, 005704D0 | §§5.3–5.4; WC7 reader, errors and ownership answered |
| AI selection, motion, docking | 00570620, 00571880, 00572770 | §§5.2–5.3; final approach expanded, remaining WC4/WC7 |
| Debug/animation/state dispatch | 00570AE0, 00572CF0, 00573340 | §§4, 5.1, 9; inherited animation and caller still WC2/WC4 |

### 10.2 Final owner-closure boundary for this lane

This document supplies 116 individually named spec-side vectors. It does **not** certify
complete owner closure: WC3–WC5 remain open; WC1–WC2 and WC6–WC8 are answered. In particular,
the equations in §5.1.1 are authored behavior requirements, not permission to
substitute an opaque provider result for missing owner computation.

| Open item | Evidence delivered | Remaining work before claiming closure |
|---|---|---|
| WC1 (answered) | §5.1.2 identifies the manager spawn writer, mode/RNG choice and activation semantics | None for this question. |
| WC2 (answered) | §§2, 5.1.1–5.1.3 give parameter uses, unused entry 34 and the animation false-return removal request | None for this question; full AI target selection and crash contact basis remain separately in WC4/WC5. |
| WC3 | 75-entry attribution inventory, Init/record boundaries, construction/destruction and ten adjustors | Complete signatures/results/failure contracts for every remaining helper; reconcile the historical 79-entry brief and inherited-call coverage. |
| WC4 | Commands, routes/following/final docking, straight/curved integration, visual basis and target slew | Full AI target selection, inherited animation/player-drive behavior, complete wake/effect/model behavior and far-approach contracts. |
| WC5 | Dispatch/events, boat selection/SAT/response, rollback, scalar crash ramps/heights, model footprint and inanimate corner tables | Sink/heel axes/contact/transform composition. |

These gaps are not answered by an inventory count or by the existing summary
sections. An engine lane can implement the newly specified arithmetic, but cannot
mark every owner-closure row closed using this document yet.

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

### 11.3 Command completion and drive ordering vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC6-01 | Nonnegative fire channel, armed=1, model action start succeeds, shots=3 | Request whalefire; shots=4, cooldown=0, armed=0. |
| WC6-02 | Same, action start fails | Shots, cooldown and armed remain unchanged. Negative fire channel makes no request. |
| WC6-03 | Group member 0x4409, skier toggled on while v>0, boat transform M | Effect gets M, then start(false); a running skier gets M again in the refresh step; no owner-side translation offset. |
| WC6-04 | Exact group 0x4410, v=0, tow button nonzero, no tow and no rope effect | Armed byte becomes 1; no failure return solely from missing effect. |
| WC6-05 | T=3, default p[8]=-2/p[9]=2, h=(1,0,0) | Final T=2 in both records; rudder=-0.25, rudder base=0, period=f32(2π), heading approximately π/2. |
| WC6-06 | Same heading with h=(-1,0,0) | Heading approximately 3π/2. For h=(0,0,1), heading=0. |
| WC6-07 | Speed tween target=0.1, v=-0.2, current sample=0.15, elapsed=5, duration=2000 | Start=0.15, target=0.2, elapsed=duration=0. Equal abs(v) and target would preserve all four fields. |
| WC6-08 | Shared channel previous=10/current=0; duration=2/limit=1; armed=0; timer=95/interval=100; dt=10 | Edge=1, previous=0, timer=105, armed stays 0. Next same call arms and resets timer to 0. Difference 0 or 250 gives edge=0. |
| WC6-09 | Shared channel duration=limit=2, current=0, armed=1 | Clear armed and issue event 0x14 via slots 6 then 5. Current=-1 suppresses event. Duration=-1 uses cooldown condition instead. |
| WC6-10 | Drive tick dt=200, both throttle/reverse=-1, v=0.1, default drag, movement allowed | Drag yields 0.09 using 200 ms; movement receives 100 ms. Owner returns true even when base drive returned false or movement is subsequently denied. |

### 11.4 Route, reservation and towing vectors

Byte sequences below are hexadecimal. Reader tests compare emitted cell coordinates;
they do not pass malformed buffers to a reader lacking bounds checks.

| ID | Inputs | Expected observable result |
|---|---|---|
| WC7-R03 | Compressed bytes `01 0A 14` | First GetNext true at (10,20), outgoing FF; next false. Empty bytes give false immediately. |
| WC7-R04 | Bytes `01 0A 14 41 04 E4` | Emit (10,20),(9,20),(9,19),(10,19),(10,20), then false; packed directions 0,1,2,3. |
| WC7-R05 | Bytes `01 0A 14 41 00` | Emit endpoint (10,20) once, then false; zero run count terminates without packed bytes. |
| WC7-R06 | Bytes `01 0A 14 82 1E 28` | Emit (10,20), then absolute endpoint (30,40), then false. Bit 7 does not alter the cell coordinates. |
| WC7-R07 | Neighbor setup, first emitted cell exactly 3 radii from occupant, second beyond | Consume both cells; next reader result is the third cell. A null occupant consumes neither. |
| WC7-R08 | Ferry reader exhausted, travel buffer empty / nonempty | Picker returns false / true respectively; no station switch just from exhaustion. |
| WC7-B01 | Current crossing A, requesting berth approaching B, held=0 | Manager reserves B for this boat; crossing remains A and held remains 0 until route initialization. |
| WC7-B02 | InitializeCurrentRoute with held=0, used count=0, no acquisition call | Held=1, used count=2, started=0; reader reset and travel points cleared. Nonzero used count is preserved. |
| WC7-B03 | Current A, held=1, distance exactly p[36] | ClearDockRequests changes nothing. Beyond radius with manager, release A, clear held, docking-in-progress and 0x10000000. |
| WC7-B04 | Same beyond radius, no manager | Docking-in-progress clears; held and 0x10000000 remain. EndDocking has this same result without a distance gate. |
| WC7-T01 | Selector list: candidate A distance=15/radius=10, candidate B distance=1/radius=10 | Select A despite B being nearer. At A distance=20 it fails the strict radius test and B wins. |
| WC7-T02 | First geometric candidate fails group/state filter; next candidate passes | DoMessage does not tow the later candidate; no repeated picker search. |
| WC7-T03 | Follower radius=5, L=25, aligned finite headings, dt=0 | g=10, translate exactly 10 along endpoint-gap direction; no turn. Tug radius only affects its back point. |
| WC7-T04 | Same with L=15, heading wrong by 90 degrees, dt=1000 | No translation or turn, because g=0; height is still set to water level and tick returns true. |
| WC7-T05 | No tug | Following returns false, without setting height. Exiting state 77 releases references but preserves being-towed byte. |

### 11.5 Crash dispatch boundary vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC5-C01 | Enter state 70 or 71, state duration=8000 | Watercraft entry notification is (0xB,0); elapsed becomes zero. Base entry result is returned without suppressing owner effects. |
| WC5-C02 | In-city state 71, crash elapsed=100, state tick dt=10 | Elapsed presented to sinking helper is 120; direct crash-sequence call with those initial values presents 110. |
| WC5-C03 | Outside city, state 70, elapsed=100, state tick dt=10 | Dispatcher advances to 110; crash sequence pushes 72 without another increment or heel animation. |
| WC5-C04 | State 71, +0x2B2=0, splash absent | No mushroom/splash call; sinking helper still runs. Do not generalize this tolerance to missing grounding in state 70. |

### 11.6 Crash scalar and rollback vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC5-S01 | Sink helper directly: T=8000, A=1, t=2000, old angle=0 | Store angle 0.25 and continue; no vertical translation yet. |
| WC5-S02 | Sink helper directly: T=8000, A=1, t=8000 | Store angle 1, return false without final transform/model publication. |
| WC5-S03 | Sink helper: A=1, angle just above c, t/T=0.75, W=10, H=14 | D=-2; runtime y=11.5, transform y=10+2u. Separate terrain level samples and distinct y writes must be represented. |
| WC5-S04 | Sink helper: W=10, H=6, angle above c*A | D=0; both y writes equal 10, regardless of positive depth below level. |
| WC5-S05 | Heel: T=8000, B=1, t=3200, +0x2CA clear | Fast ramp reaches 1 and returns false; no final heel transform. |
| WC5-S06 | Heel: T=8000, B=1, t=3200, contact byte remains set after slide phase | Slow ramp reaches 0.4 and continues. At t=4000 the fast ramp applies and completes instead. |
| WC5-S07 | Heel: t=T | Return false before any contact phase or angle store. |
| WC5-S08 | Bump rollback: previous position=(1,2,3), current=(4,5,6), speed=0.2, turn=0.7 | Runtime/inherited/transform translations become (1,2,3); orientation, speed and turn unchanged by helper; publication then slots 100,104,107. |

### 11.7 Boat collision selection and response vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC5-B01 | L_A=10, L_B=6, center distance=10, overlapping test rectangles | Candidate rejected before footprint test. At distance just below 10, footprint test is reached. |
| WC5-B02 | Two valid axis-aligned rectangles, centers (0,0) and (2,0), both half extents (1,1) | Overlap true, including edge contact. Moving second center past 2 separates them. Invalidating either rectangle makes overlap false. |
| WC5-B03 | Group has both 0x4413 and 0x4414 | Class 3. Only 0x4413 gives 2; neither gives 1. |
| WC5-B04 | Player A and AI B, same class, speed ratio above p_A[27] | AEMS 7 slots 6,5; A sets collision byte, notifies and enters 71; then B notifies/enters 71 without setting its collision byte. Each enter also notifies. Both use their own durations. |
| WC5-B05 | Player A larger than AI B, ratio above threshold, B collision byte initially 0 | Only B sinks; B collision byte stays 0. Return true and skip shore detection. |
| WC5-B06 | AI A smaller than player B, AI collisions enabled, ratio above p_A[15] | Only A sinks; no AEMS event 7 from this detector; A collision byte is not set. |
| WC5-B07 | AI A and AI B same class, ratio above p_A[15] | Push 73 on A then B, with no explicit crash notifications; return true. |
| WC5-B08 | Exact threshold equality, v_A=0.2, p_A[29]=0.75, B movement gate true, no shore hit | Roll back A; B speed=0.2, A speed=0.15; return false. No heading-based sign adjustment. |
| WC5-B09 | Same bump, B movement gate false | B speed unchanged; A still attenuates to 0.15. |
| WC5-B10 | A and B both have v=0.2 and opposite headings | Relative-speed ratio is zero, hence bump for nonnegative threshold; do not use relative velocity magnitude. |

### 11.8 Lifetime and forwarding vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC3-L01 | Successful constructor with valid base/storage services | Returned pointer is receiver; travel vector size=11, capacity=11, all entry bytes zero; six effects and tow references null; type ID=0x897F7EC2. |
| WC3-L02 | QueryInterface for owner ID through +32 adjustor, valid output | Output is complete owner pointer, not the +32 pointer; one AddRef; true. |
| WC3-L03 | Write: version writer reports failure, base writer reports success | Version 2 write is attempted; base writer is still called; returned result is base result. |
| WC3-L04 | Complete destruction, all references nonnull | Effect releases descend from +0xDC4 to +0xDB0, then route/tow/station cleanup in §3.1 order, then base destruction; no owner Shutdown/berth-return calls. |
| WC3-L05 | Three travel points [A,B,C], erase B | Vector [A,C], size reduced by 1, capacity unchanged; returned pointer addresses C. Erasing C next returns new end. |
| WC3-L06 | Sign helper inputs -0.0, +0.0, -2, +2 | Results +1,+1,-1,+1. |

### 11.9 Position integration vectors

Use float tolerance 1e-5 for positions/rotations and 1e-6 for scalar parameters;
publication order and unchanged fields are exact.

| ID | Inputs | Expected observable result |
|---|---|---|
| WC4-M01 | Straight: P=(10,2,20), H=(3,4,0), p[4]=0.2, v=0.1, dt=100 | Runtime P'=(12,2,20); heading unchanged. Visual helper subsequently adds p[18]/2 to transform y only. |
| WC4-M02 | Same with dt=-100 | Runtime P'=(8,2,20); integration has no internal nonnegative-time clamp. |
| WC4-M03 | Curved: P=(0,0,0), H=(0,0,1), water=0, defaults, v=p[7]=0.3, turn=p[9]=2, dt=100 | R=75, theta=0.08, P'=(75*(1-cos(0.08)),0,75*sin(0.08)); heading=(sin(0.08),0,cos(0.08)). |
| WC4-M04 | Same speed, turn=1 | R=100, theta=0.06; do not multiply theta again by turn. |
| WC4-M05 | Reverse: v=p[6]=-0.1, turn=p[8]=-2, defaults, dt=100, level input | R=7.5, theta=+0.26666667, center on the opposite side from positive turn; no banking; bow angle=0, visual height still raised by p[18]/2. |
| WC4-M06 | v=0, turn=1, previous/current positions differ initially | No position/heading integration; previous position becomes current; FindState(16) called, no pop. Visual helper no-op; transform and inherited fields still published, then slots 100,104,107. |
| WC4-M07 | v=0.1, p[17]=1, p[18]=2, p[24]=4, turn=0 | Bow angle=0.1, transform y increased by 1; runtime y unchanged by attitude helper; p[24] has no effect here. |
| WC4-M08 | Curved dt=0, R>0, finite H, P_y different from water level | Position is C+R*normalize(P-C), which can differ from P; no special zero-dt early return. |

### 11.10 Spawn idle mode and collision construction vectors

Use float tolerance 1e-5 for finite footprint values. Service order, selected
corner, booleans and direction bytes are exact. RNG vectors assume all other
population gates pass; mode draws are doubles.

| ID | Inputs | Expected observable result |
|---|---|---|
| WC1-I01 | Manager slot 13 true, available RNG draws 0.05 then 0.500001, successful creation | Group 0x4417; limit=45000, elapsed=0, +0x2CC=1, even if the boat moves. |
| WC1-I02 | Same, second draw exactly 0.5 | Group 0x4418 and +0x2CC=0. First draw above 0.05000000074505806 instead rejects before the mode draw. |
| WC1-I03 | Slot 13 false with draw 0.1, or either slot-13 result with missing RNG | Group 0x4418 and mode false; missing RNG makes no draws. Failed creation never writes boat timers/mode. |
| WC1-I04 | Speed modifier allows tick, mode true, idle=59990, second time=10 | Idle becomes 60000, no idle-triggered push; a following positive increment pushes 73. Mode bypasses ordinary autonomous motion/lifetime accounting after the inherited step. |
| WC1-I05 | Mode false, idle=60001; separately speed modifier below 1e-5 | First case still tests idle and pushes 73; second case returns true before any idle addition or threshold test. |
| WC5-F01 | Model bounds min=(-4,-1,-8), max=(6,3,12), translation=(100,5,200), identity rotation, offset=(3,99,-4), multiplier=2, query false | Center=(103,196), half extents=(9.5,19.5), axis=(1,0), valid=true. Local bounds center and y offset have no effect. |
| WC5-F02 | Same, query true with returned interface | Half extents=(8,19); interface released. Returned interface with false query is still released but uses false-query margins. |
| WC5-F03 | Same bounds, R_at_+4=0, R_at_+0xC=2, multiplier=1 | Axis=(0,-1); dimensions unchanged by model scale. Zero projected axis has no finite fallback. |
| WC5-F04 | Destination has validity=true and axis=(0,1), model absent | Six floats +4..+0x18 cleared; validity and axis retained; destination address returned. No guaranteed noncollision for uninitialized scratch storage. |
| WC5-F05 | Two candidate boats qualify, second is closer | First list entry wins. Candidate footprints use zero offset and multiplier 1 regardless of caller's offset/multiplier. |
| WC5-I01 | Rectangle=(2,3,4,5), occupant hit only at corner 2, unnavigable corner 0, H=(2,0,1) | Query occupant corners (2,3),(2,5),(4,3); no navigability query; corner 2/column 1 writes (0,0), returns true. |
| WC5-I02 | No occupant hits, docking false, first unnavigable corner 1, H=(1,0,1) | All occupant queries precede navigability; equality selects column 5; writes (0,1), returns true. |
| WC5-I03 | No occupant hits, docking true, all water cells unnavigable, preexisting direction=(1,0) | No navigability calls, returns false and retains (1,0). An occupant hit would still count while docking. |
| WC5-I04 | Occupant hit at corner 0, H=(0,0,0) | Column 7, writes (1,1), returns true. No heading normalization and no speed test. |
| WC5-I05 | For each corner 0–3 and each finite representative heading for columns 0–7 | Verify all 32 cells of §5.7.4's direction table; return true even for (0,0). |

### 11.11 Caller, parameter and visual-basis vectors

Float tolerance 1e-5 applies to finite visual vectors; call order, retained
runtime heading and removal arguments are exact.

| ID | Inputs | Expected observable result |
|---|---|---|
| WC2-P01 | Shared manager pass permits occupant tick; boat returns false from stillness timeout | Call slot 72 with both original times, then manager slot 8 with (boat,false); no caller-inserted fade state. Next list link was obtained before the boat call. |
| WC2-P02 | AI dt=10, p[13]=0.001, p[14]=0.002, current v=0.1/turn=0, targets v=0.3/turn=-1 | After slew v=0.11, turn=-0.02 before final bounds clamps. Target within or exactly at the step is assigned directly. |
| WC2-P03 | Shore-slide candidate inside city, p[26]=10, sampled terrain height minus runtime y is 10; separately 9.99 | Equality fails contact and clears +0x2CA; 9.99 passes the height gate. The latter still needs the contact transform specified by WC5. |
| WC2-P04 | Change only p[34], reload tuning, ordinary finite owner inputs | New value stored at +0x3CC; no observed change to owner movement/collision/timer/effect calculations. |
| WC4-V01 | H=(0,0,1), positive v, turn=0, bow angle b | F=(0,sin(b),cos(b)), R=(1,0,0), U=(0,cos(b),-sin(b)); runtime H stays (0,0,1). Transform lift=p[18]/2. |
| WC4-V02 | H=(0,0,1), positive v, banking angle a, bow angle zero | Banking up reference=(sin(a),cos(a),0); final R=(cos(a),-sin(a),0), U=(sin(a),cos(a),0), F=(0,0,1). |
| WC4-V03 | Negative v, identity incoming rotation, H=(0,0,1), p[17]>0 and p[18]>0 | Bank leaves incoming rotation, bow angle zero, final basis identity; visual lift still applies. Speed zero instead leaves transform unchanged. |
| WC4-V04 | Nonzero speed, vertical H=(0,1,0) | Bow axis H cross world-up is zero; no finite fallback. Do not substitute a default axis or report a boolean failure. |

### 11.12 Geometry helper contract vectors

| ID | Inputs | Expected observable result |
|---|---|---|
| WC3-G01 | Runtime P=(10,20,30), H=(2,3,4), GetRadius=5 | Rear output=(0,5,10); front output=(20,35,50). Each call samples radius once; neither normalizes H or changes boat fields. |
| WC3-G02 | Occupant bounds min=(1,2,3), max=(4,12,8) | Extent helper returns 10, including vertical size; it does not use horizontal radius or model-footprint margins. |
| WC3-G03 | Rectangle=(2,3,4,5); only (2,5) outside | City queries (2,3),(4,3),(2,5), then true; no fourth query and no state push. All corners inside gives false. |
