# Spec: aircraft (`cSC4Aircraft`, `cSC4Airplane`, `cSC4Helicopter`) and the air dispatch sources

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[summary]* is a correct outline whose details are not given.

Related specs: [automata.md](automata.md) §5.2 (the aircraft manager), §6.4 (the records), §6.6 (the aircraft
state families and the two `AI_PursueGoal` dispatchers), §11 (slot names); [dispatch.md](dispatch.md) §4.3
(dispatch states 3, 4, 5, 6, 0 set by aircraft); [protection.md](protection.md) §8 (dispatch sources);
[drive-tool.md](drive-tool.md) (U-Drive-It); [audio.md](audio.md) (AEMS events); [vehicles.md](vehicles.md).

## 0. Summary

Aircraft are automata that fly: a base class with the altitude and world-clip helpers, the **airplane** (a fixed
wing flight model with runways: taxi, take-off, cruise, approach and landing; fire planes drop water) and the
**helicopter** (a rotor model with hover, climb over buildings, a spotlight and weapons in U-Drive-It). Both take
their handling from an 18-value **drive-parameter list** of their automata exemplar. Two dispatch sources let
airports and police stations with heliports send fire planes and police helicopters.

## 1. Classes *[compat]*

| Class | Class ID | Notes |
|---|---|---|
| `cSC4Aircraft` | `0xC96E75DC` | base (automata.md §6.4) |
| `cSC4Airplane` | `0xCBEB4F62` | §3; record with the home position |
| `cSC4Helicopter` | `0x8BA98EDA` | §4; record with the home position |
| `cSC4FirePlaneDispatchSource`, `cSC4PoliceHelicopterDispatchSource` | — | §5 |

## 2. The aircraft base

- `GetAltitude(x, z)` (`0x004FB0B0`): the height of the ground or water under the point, raised by the buildings and
  props there *[summary]*.
- `ClipToWorld(p)` (`0x004FB180`): keeps a point inside the city, with a margin of −1000 (so slightly outside the
  map is allowed) *[summary of the bounds]*.
- `DoPlayerDriveTick`: the automaton's, then the aircraft's own controls (§3.4, §4.4).

## 3. The airplane *[compat]*

### 3.1 Drive parameters (`LoadDriveParameters`, `0x004FC910`)

The source is the Lua `automata_group` definition's **`airplane` table**, imported by
`ImportAutomataGroups` (`0x00521C70`) and `ImportAirplaneParams` (`0x0051E1E0`). There is
**no numeric exemplar-property ID for this table**. The automaton's resolved group supplies
the record; if absent, use the first ancestor with a record. Inheritance selects a whole
record, not individual fields. `helicopter` is the alternative table for rotorcraft (§4.1),
and `playerdrive` is a separate record (§3.5). The group import checks `airplane` before
`helicopter` before `WatercraftDriveParams`; these alternatives share one record slot.

Each named field imports as a binary32 value; a missing/nil field becomes zero
(`ReadTableField`, `0x005FB535–0x005FB539`). Thus an empty table creates a present,
all-zero record and stops inheritance. This is a fixed named record, not a variable-length
list with an exposed count. A raw buffer shorter than 18 floats is not a supported input
to the flight owner; the original loader has no length check or zero-padding operation.

A value equal to either signed zero selects the default. No finite/range validation is
performed: nonzero negative values, infinities and NaNs survive loading. For entries 2
and 3 the zero test is **after multiplication by binary32 1000**; their fallbacks, 25
and 150, are unscaled (`0x004FC987–0x004FC9D6`). Ancestor lookup follows parents until
a record or null; it has no cycle detection (`0x004FC8E0`). A recordless cycle therefore
does not terminate. The loader does not establish an acyclic-data guarantee.

| # | Default | Derived default when 0 | # | Default | Derived default when 0 |
|---|---|---|---|---|---|
| 0 | 100 | | 9 | | entry 8 / 12 |
| 1 | 1 | | 10 | 10000 | |
| 2 | 25 | | 11 | | entry 10 |
| 3 | 150 | | 12 | 0.4 | |
| 4 | 0.2618 (15°) | | 13 | | entry 12 × 0.1 |
| 5 | | entry 4 × 0.25 | 14 | 0.5236 (30°) | |
| 6 | 0.7854 (45°) | | 15 | 0.08 | |
| 7 | | entry 6 × 0.5 | 16 | 0.68 | |
| 8 | 0.5236 (30°) | | 17 | 0 | |

With no list at all the defaults above are used, with 5 = 0.06545, 7 = 0.3927, 9 = 0.04363, 11 = 10000, 13 = 0.04.
`LoadPlayerDriveParameters` loads a second set for U-Drive-It *[summary]*.

The preceding decimal angle defaults are rounded descriptions, **not exact replacement
constants**. Use the binary32 encodings below. Direct no-record defaults are stored as
constants; zero-field defaults for 5/7/9/13 are calculated from the selected maxima.
In particular 9 multiplies by binary32 `0x3DAAAAAB` (approximately 1/12), rather than
dividing by exact 12. Stores round to binary32; the original arithmetic uses x87
intermediates (`0x004FCA96–0x004FCB60`, `0x004FCBE1–0x004FCC85`).

| Entry | Lua field / meaning | Exact no-record default (binary32 bits) |
|---|---|---|
| 0 | `Mass` | `42C80000` (100) |
| 1 | `WingArea` | `3F800000` (1) |
| 2 | `TakeOffSpeed`, scaled ×1000 | `41C80000` (25) |
| 3 | `MaxSpeed`, scaled ×1000 | `43160000` (150) |
| 4 | `ElevatorMax`, radians | `3E860A92` |
| 5 | `ElevatorChangeRate` | `3D860A92`; zero field: entry 4 ×0.25 |
| 6 | `AileronMax`, radians | `3F490FDB` |
| 7 | `AileronChangeRate` | `3EC90FDB`; zero field: entry 6 ×0.5 |
| 8 | `RudderMax`, radians | `3F060A92` |
| 9 | `RudderChangeRate` | `3D32B8C3`; zero field: entry 8 ×`3DAAAAAB` |
| 10 | `ThrottleMax`, force scale | `461C4000` (10000) |
| 11 | `ThrottleChangeRate` | `461C4000`; zero field: entry 10 |
| 12 | `FlapsMax` | `3ECCCCCD` (approximately 0.4) |
| 13 | `FlapsChangeRate` | `3D23D70A`; zero field: entry 12 ×binary32 0.1 |
| 14 | `StallAngle`, radians | `3F060A92` |
| 15 | `LiftFactorLevelFlight` | `3DA3D70A` |
| 16 | `LiftFactorMaxClimb` | `3F2E147B` |
| 17 | `LiftFactorFullStall` | `00000000` |

### 3.2 Init and effects

`Init` (`0x005017C0`) loads both parameter sets, builds the three ray probes (`0x004FF840`, `0x004FFAE0`,
`0x004FFD80`), and the effects: the propeller (`fx_00_planepropstill` / `fx_00_planeprop`, or the fire-plane
versions `fx_00_fireplanepropstill` / `fx_00_fireplaneprop`), sky writing (`fx_00_skywriting`), the water drop
(`fx_00_waterdrop`), the indicator (`aircraftindicate`) and the bomb (`jetbomb`, U-Drive-It).

### 3.3 AI goals (`AI_PursueGoal`, `0x00504590`; automata.md §6.6)

The goal stack drives a set of behaviours, each a steering routine over the flight model:

- `AI_GotoDestination` (fly to a point: take off first when on the ground, turn to the vector, follow it, keep the
  approach altitude over the ground); `AI_Land` (pick a runway direction and an approach box, descend along the
  approach ray to the base altitude, touch down, `SetAirplaneState`, stop, taxi); `AI_TakeOff` (pick a runway
  direction, taxi to its start, turn, roll and climb); `AI_Taxi`, `AI_Stop`, `AI_TurnToVector`, `AI_FollowRay`,
  `AI_FollowVectorAndTurn`, `AI_ApproachAltitude` *[summary of each control law]*.
- **Runways.** `PickRunwayDirection` (`0x00502520`) takes the airport's runways from the aircraft manager
  (`GetRunwayList`) and picks the direction with the best clearance (`EvaluateClearance`: the altitude along the
  approach against the airplane's height); `PickApproachBox` places the approach start up to 500 units out, clipped
  to the world.
- **Fire planes.** `FindFireNearLocation` finds the nearest burning occupant near the target; the plane flies over
  it and drops water *[summary]*.
- Dispatch states: dispatch.md §4.3 (3, 4, 5, 6, 0).

#### 3.3.1 Airplane LOITER countdown and ammunition (AF3) *[compat]*

Airplane LOITER (53) requests dispatch state 5. On the first visit with the
water-drop flag clear it sets that flag, installs a **2.0-second remaining
countdown**, and subtracts one from the ammunition count. This subtraction has
no positive-ammunition gate: entering LOITER with zero ammunition produces −1.
While the flag is already set, visiting the goal does not reset the countdown
or subtract another unit. The goal clears the flag and pops when the countdown
is **less than or equal to zero**, including exactly zero. Thus neither a strict
`>2000 ms` elapsed test nor an uncapped wall-clock timer describes this goal.

The animation tick first subtracts its physics step from a positive countdown,
before pursuing the goal. That step is in seconds, uses the AI time multiplier,
and has the 0.2-second upper cap of §3.4.1. It is subtracted only once per tick;
a countdown that crosses below zero is not clamped to zero. The first LOITER
visit installs 2.0 **after** that tick's countdown update. Floating rounding can
therefore move expiry between neighboring ticks even when their nominal total
is two seconds. Compare the actual remaining value with zero.

Construction gives ammunition **3**, a clear drop flag and countdown **0**.
`Read` does not read or reset those three fields: a newly constructed object
keeps those defaults, while reading into an existing object leaves its current
values alone. A restored LOITER goal on a fresh object consequently starts a
new 2.0-second drop and changes ammunition from 3 to 2; saved drop progress and
ammunition are not reconstructed from the goal record. A change into flight
phase 1 refills ammunition to 3; asking for phase 1 again does not refill (§3.5).
The positive-ammunition checks in PUT_OUT_FIRE select future goals; they do not
protect LOITER's own decrement.

Evidence: countdown installation/decrement and inclusive expiry
`0x0050472C–0x00504776`; animation time conversion/cap and countdown update
`0x00504981–0x005049F7`, goal call `0x00504A6A–0x00504A71`; constructor
`0x004FD059–0x004FD065`; Read `0x004FD3C0–0x004FD416`.

### 3.4 Flight model *[summary]*

`UpdatePhysics` (`0x00503CB0`) integrates `CalculateForces` (thrust, lift, drag and the control surfaces, 3 KB),
keeps the plane above the ground (`GetAltitude`) and on the runways when rolling, detects collisions
(`DetectCollisions`: buildings and other automata in its path; a crash plays its AEMS event), clips to the world and
changes the airplane state (`SetAirplaneState`, which starts and stops the engine sounds). `DoAnimationTick` runs
the goals and the physics; in U-Drive-It, `TranslatePlayerDriveCommands` turns the player's commands
into control inputs and drops `jetbomb` bombs.

#### 3.4.1 Physics time and translational integration *[compat]*

This subsection specifies the numerical **candidate motion**, before terrain, runway,
collision and attitude corrections. It does not supply those corrections or close AF2.
`UpdatePhysics` takes a binary32 time step in **seconds**. The animation caller converts
the incoming milliseconds by binary32 `0x3A83126F` (approximately 0.001), applies
the selected AI time multiplier for non-player aircraft, then caps the result at
binary32 `0x3E4CCCCD` (approximately 0.2 seconds). Player-drive ticking converts and
caps the same way, without that AI multiplier. These callers have no lower clamp or
subdivision into several shorter steps. The original millisecond argument still goes
to the animation/control callbacks; the converted step goes to AI pursuit and physics
(`0x00504981–0x005049DA`, `0x00504A6A–0x00504A7E`,
`0x00504B01–0x00504B42`). The animation pseudocode's apparent reuse of the millisecond
argument for physics is incorrect: the instructions reload the converted local.

At the start of the physics step, limit the **whole 3D velocity**, before calculating
forces. Let `s0 = length(v)` be the x87 square-root result and `s` its saved binary32
value. For finite inputs:

| Condition, tested in this order | Velocity supplied to the force model |
|---|---|
| `s0 < epsilon` and `s0 > 0` | `v / s0`, hence speed **1**, not zero |
| `s0 < epsilon` and `s0 = 0` | unchanged zero vector |
| Otherwise, `s > MaxSpeed` and `s0 > 0` | `v × (MaxSpeed / s0)` |
| Otherwise | unchanged |

Here `epsilon` has binary32 bits `0x38D1B717` (approximately 0.0001).
The tiny-speed branch takes priority over `MaxSpeed`, even when that maximum is
less than 1. The maximum-speed comparison is strict: equality leaves velocity
unchanged (`0x00503CBB–0x00503D9E`). There is no second speed limit immediately after
acceleration, so the candidate new velocity can exceed the maximum within this step.
The table covers finite inputs; it does not establish a sanitization policy for NaNs.

Let `v0` be that limited velocity, `F` the calculated force, `m` the loaded mass,
`p0` the old position and `h` the seconds step. The candidate motion is:

- `a = F / m`;
- `v1 = v0 + a × h`;
- `p1 = p0 + (v0 + v1) × (h × 0.5)`.

The displacement therefore uses the mean of old and new velocity, rather than only
`v1`. The force is evaluated once for this candidate. Mass is used as loaded: this
step does not replace a negative or otherwise invalid mass with a safe default.
After the environment-dependent corrections, position and velocity are committed,
collision detection runs, position is clipped to the world, and the attitude update
runs, in that order (`0x00503D9E–0x00503E80`, `0x0050431D–0x00504400`). For ground
phases 0/1, the subsequent velocity is projected onto the updated orientation:
`v := orient × dot(v, orient)` (`0x00504418–0x00504447`). These later operations can
change the candidate values; a conformance fixture must expose the candidate stage
separately if it tests only the integration above.

#### 3.4.2 Force magnitudes *[compat, partial AF2]*

The verified scalar part of `CalculateForces` (`0x004FE850`) is given here so that
an implementation need not invent a conventional aerodynamic formula. The complete
attitude-to-lift-direction construction and angular/control update remain open under
AF2; this subsection alone is not a complete force-owner replacement.

Inputs are velocity `v`, the supplied orientation `d`, normalized throttle `t` and
flaps command `f`, plus the loaded parameters in §3.1. The force routine itself does
not clamp either command to a control range. Let `s = length(v)` and let `u` be the
normalized velocity, using `d` instead when `s < epsilon`. Only this direction
substitution uses the fallback: speed and dynamic pressure still use the actual `v`.
The substituted direction is normalized too; there is no additional zero-vector
fallback (`0x004FE8A2–0x004FE92F`).

The angle used for the coefficient curve is **not** the angle between the two full
3D vectors. With `u_y` and `d_y` their vertical components, it is

`alpha = clamp(acos(clamp(sqrt(1 − d_y²) × u_y + sqrt(1 − u_y²) × d_y, −1, 1)), 0, A)`.

`A` has binary32 bits `0x3FC90FDB` (approximately π/2). The square-root operands
are not individually clamped. The inner result is stored as binary32 before the
inverse cosine; the angle result is bounded afterwards
(`0x004FF119–0x004FF1B4`). The export labels the called helper at `0x009F0090`
`TransformDirection`, but its math body is inverse cosine: it evaluates
`atan2(sqrt((1+x)×(1−x)), x)` (`0x009F00C8–0x009F00D6`). Do not infer a vector
transform or substitute an ordinary angle-of-attack calculation from that name.
In particular, for horizontal `d` and horizontal velocity, this expression gives
`alpha = A`, regardless of their horizontal headings.

Write `sigma = StallAngle`, and `C0`, `Cs`, `C90` for
`LiftFactorLevelFlight`, `LiftFactorMaxClimb`, `LiftFactorFullStall`.
The lift coefficient is piecewise linear:

| Angle | Coefficient `C` |
|---|---|
| `alpha <= sigma` | `C0 + ((Cs − C0) / sigma) × alpha` |
| `alpha > sigma` | `Cs + ((alpha − sigma) × (C90 − Cs)) / (A − sigma)` |

There is no denominator guard for a loaded zero or out-of-range stall angle in
this calculation (`0x004FF1EE–0x004FF243`). Define the pressure-like scale,
lift magnitude and drag normalization in this operation order:

- `Q = ((((FlapsMax × f) + 1) × s) × s) × AirResistanceFactor × 0.5`;
- `L = C × Q`;
- `K = ThrottleMax / ((((MaxSpeed × MaxSpeed) × AirResistanceFactor) × 0.5) × k)`.

Here `k` has binary32 bits `0x3DCCCCCD` (approximately 0.1). `L` is stored as
binary32 before scaling the lift direction (`0x004FF28A–0x004FF2AE`). `K` is stored as
binary32 before being used by the drag calculation (`0x004FE85A–0x004FE8A0`);
`Q` is formed at `0x004FF1CB–0x004FF1EE`. Flaps scale both lift and aerodynamic
drag through `Q`; they do not add a separate constant force.

Let `Fy_lift` be the binary32 vertical component after applying magnitude `L` to the
attitude-derived lift direction. Let `W` be the magnitude of the stored binary32
gravity force, `binary32(m × g)`. The extra rolling resistance is zero except in
phases **0/1** when `Fy_lift < W`. In that case its magnitude is

`R = (1 − (1 / W) × Fy_lift) × m × s × k`.

Gravity uses binary32 `g = 0x411CCCCD` (approximately 9.8), with downward force
`−m × g`. The vertical lift component, not the total magnitude `L`, reduces rolling
resistance. Equality with weight produces no extra resistance
(`0x004FF1B4–0x004FF1C7`, `0x004FF328–0x004FF367`).

The drag magnitude opposing `u` is
`D = R + (alpha × alpha + k) × Q × K`. Thrust magnitude is `T = t × ThrottleMax`,
along the supplied `d`. Only in phases 0/1, if **both** `t < 0` and `dot(v,d) < 0`,
replace `T` with zero. Thus reverse throttle brakes forward motion, but cannot
continue accelerating an already backward-moving plane on the ground. Zero dot
product does not suppress it; airborne phases do not apply this gate
(`0x004FF2B2–0x004FF2F6`). The returned force is the sum of lift, thrust,
gravity and `−D × u` (`0x004FF317–0x004FF3E2`).

These equations describe operation ordering, not a requirement to round every
arithmetic operation to binary32. The original uses x87 intermediate arithmetic
and binary32 stores. Compare floating conformance results with an explicit tolerance;
do not replace the cited encoded constants with the rounded decimal text.

### 3.5 Flight-phase sound actions (AF4) *[compat]*

The airplane's flight phase is independent of its goal stack. A request for the
current phase is a complete no-op, including ammunition and sounds. For a
different phase, the following table gives the ordered actions; sound requests
are conditional on the aircraft's player-drive flag. Here **start** is an AEMS
events-service request at slot `+0x14`, and **end** one at `+0x18`; the numbers
are event groups, not effect resource IDs.

| Requested phase | Non-sound action | Ordered sound requests when player-driven |
|---|---|---|
| 0 | store phase 0 | end 9, end 8, end 10, start 10 |
| 1 | refill ammunition to 3; store phase 1 | from old phase 0: start 9; from old phase 3: end 13, start 13; from any other old phase: none |
| any other value, including 3 | store requested phase | none in this setter |

In particular, merely setting phase 3 does not start group 13 here. This table
defines the setter's actions; collision and flight-physics triggers for other
audio requests remain part of AF2/AF4. Evidence: `0x004FCEF8–0x004FCFA8`.

## 4. The helicopter *[compat]*

### 4.1 Drive-parameter record (AF1)

The `automata_group` **`helicopter` table** has **13** named binary32 fields,
not twelve or eighteen. Record selection and missing-field import follow §3.1.
`LoadDriveParameters` (`0x005340A0`) overlays the existing rotor preset: a zero
field keeps the previous destination value, and absence of any ancestor record
leaves the preset unchanged. This differs from the airplane's fixed-default loader.

| Entry | Lua field | Conversion before zero fallback | Destination preset field |
|---|---|---|---|
| 0 | `Mass` | none | mass |
| 1 | `Power` | none | engine power |
| 2 | `SpeedMultiplier` | none | horizontal speed multiplier |
| 3 | `MaxSpeed` | ×1000, then ×binary32 `40555555` | speed limit |
| 4 | `DragFactor` | none | translational drag |
| 5 | `RotationalDragFactor` | none | rotational drag |
| 6 | `BankMax` | ×binary32 `440F3D4D` | bank maximum |
| 7 | `BankChangeRate` | same | bank change rate |
| 8 | `PitchMax` | same | pitch maximum |
| 9 | `PitchChangeRate` | same | pitch change rate |
| 10 | `YawRateMax` | same | maximum yaw rate |
| 11 | `BankMultiplier` | none | visible bank multiplier |
| 12 | `PitchMultiplier` | none | visible pitch multiplier |

`440F3D4D` is approximately 572.957824707 (tenths of degrees per radian),
not rounded 572.958. The speed multiplier constant is approximately 3.33333325386.
Their order and float-width operands are established at `0x0053412A–0x0053414F`
and `0x0053418A–0x0053422B`. These fields have no non-finite/range rejection.
Named import is `0x0051E4A0`; it is not a numeric exemplar property.

#### 4.1.1 Initial rotor preset and reconstruction (AF9)

The thirteen fields have concrete initial values. They are installed when
`InitInstance(0)` first creates the runtime rotor model, before either parameter
overlay. They are not additional saved fields. The normal `Init` always selects
preset **0** (`0x0053A8DD–0x0053A8F9`); the second static preset is not selected by
this initialization path. The initial values below are already in internal units:
do not apply the Lua conversion factors to them.

| Field | Initial value | Exact binary32 bits |
|---|---:|---|
| Mass | 1.5 | `3FC00000` |
| Power | 240 | `43700000` |
| SpeedMultiplier | 0.699999988079071 | `3F333333` |
| MaxSpeed | 227 | `43630000` |
| DragFactor | 4 | `40800000` |
| RotationalDragFactor | 1 | `3F800000` |
| BankMax | 300 | `43960000` |
| BankChangeRate | 400 | `43C80000` |
| PitchMax | 300 | `43960000` |
| PitchChangeRate | 400 | `43C80000` |
| YawRateMax | 400 | `43C80000` |
| BankMultiplier | 1 | `3F800000` |
| PitchMultiplier | 1 | `3F800000` |

Evidence: the preset-copy instructions `0x005345A0–0x00534660`, selection/copy at
`0x00538607–0x00538620`, and the file-backed binary32 words in preset 0 at
`0x00B09B70` (stride `0x7C`). The angular fields come from offsets `0x0C`,
`0x24`, `0x14`, `0x1C`, `0x20`; mass through pitch multiplier come from offsets
`0x5C–0x78`. This is a bit copy, not a recomputation from degrees or radians.

Initialization then applies **player parameters first**, followed by the
`helicopter` AI overlay (`0x00538622–0x0053863D`). Therefore the copied preset
and the final active parameters need not be identical. In particular, if a player
parameter record exists and its converted speed is zero, `OnPlayerTakeOver`
uses the preset's **yaw-rate maximum (400)** as its speed fallback, not its
MaxSpeed (227), then the AI overlay runs. With no player record and no AI record,
MaxSpeed stays 227; with an all-zero player record and no nonzero AI MaxSpeed,
it becomes 400 (`0x005342A6–0x005342CF`). Preserve this quirk.

`Read` restores the nested aircraft record and the home vector and marks placement
as loaded; it does **not** construct a rotor model or restore a flight phase
(`0x00534400–0x00534456`). Subsequent `Init` uses that home placement, performs
the allocation/preset/overlay sequence above if no runtime model exists, and calls
`PhysicsInit`. `PhysicsInit` explicitly selects phase **0** with engine disabled
(`0x005361D5–0x005361EE`). Thus a freshly loaded helicopter starts its physics in
phase 0, irrespective of the goal stack preserved by the base record. The goal
stack and flight phase are separate. `InitInstance` is a no-op when the runtime
model already exists (`0x005385C4–0x005385CE`); calling it again does not recopy
the preset or reapply either overlay. `PhysicsInit` still resets the phase when
it is called on that existing model.

### 4.2 Flight and control overview

- **Parameters** (`LoadDriveParameters` `0x005340A0`): the 13-field record and
  exact conversion constants are specified in §4.1.
- `PhysicsInit` and `InitInstance` set up the rotor model; `NextFrame` (`0x005397C0`) computes the engine power,
  then yaw, pitch, roll and slide (`YawPitchRollSlideReal`, with limits ±890 and 250), the
  horizontal move (`MoveXZReal`) and the vertical move (`MoveYReal`, kept above `GetAltitude`) *[summary]*.
- **AI controls**: `AI_ControlReset`, `AI_ControlHover` (§4.2.1),
  `AI_ControlMoveXZ` (bands ±10 and ±20 *[inferred]*), `AI_ControlLiftOverBuildings` (climbs when the ground ahead
  is higher), and `AI_PursueGoal` (automata.md §6.6, including the chase of case `0x34`).
- **Spotlight** (`UpdateSpotlight`, 2 KB): the light cone follows the ground below or the target *[summary]*.
- **States** (`SetHelicopterState`): each starts or stops its AEMS rotor sounds.
- **U-Drive-It**: `OnPlayerTakeOver` loads the player parameters; `TranslatePlayerDriveCommands` (2.4 KB) flies it
  and attempts the named weapon effects (§4.3).

#### 4.2.1 Reset and hover controls (AF3) *[compat]*

These are updates of the rotor **control record**, not complete motion ticks.
Let its four binary32 controls be `b` (lateral, record +0), `p` (longitudinal,
+4), `c` (collective, +8), and `r` (yaw, +12). The record belongs to the
helicopter at +0x258; the runtime rotor belongs to it at +0x254. Offsets here
identify observed fields, not additional serialized data. For finite inputs
and a nonnegative step `h` in **seconds**, let `limit(x,L,U)` keep x within the
inclusive bounds L and U. The constants below are binary32 operands; the
original performs the arithmetic with x87 intermediates and stores binary32
results. These rules do not assert non-finite equivalence for the wider rotor
model (AF2).

**Reset** approaches `b`, `p` and `r` toward zero and `c` toward −100, each by
at most `100*h` per call. More explicitly, each new control is its old value
plus the target-minus-old difference limited to `[−100*h,100*h]`. It then
updates two runtime angular demands using the **new** controls:

| Runtime output | Value |
|---|---|
| bank demand (+0x104) | `limit((b * BankChangeRate) * 0.025, −BankMax, BankMax)` |
| pitch demand (+0x10C) | `limit(((g * PitchChangeRate) * (−p)) * 0.025, −PitchMax, PitchMax)` |

`g` is the runtime pitch factor (+0x1C8), supplied by the rotor simulation;
reset neither replaces it with 1 nor recomputes it. `0.025` has bits
`3CCCCCCD`. The longitudinal sign is deliberate: reset stores `p` back in the
control record but uses **−p** for the pitch demand. The yaw/collective changes
do not make reset a call to the motion integrator. Evidence: control updates
`0x00534F79–0x005350B3`, demands `0x005350B6–0x0053516B`.

**Hover** changes only `c`. Let `vy` be rotor vertical velocity (+0xD4), `y`
the aircraft's current world Y, and `H` the requested **absolute world height**.
First form `u = vy * 0.3`, then a predicted height `y + u*0.5`. The desired
collective increment is `((H − (y + u*0.5))*0.1 − u)*0.5`. The native
expression adds the old collective and subtracts it again before limiting this
increment; retain that operation order when reproducing rounding. Limit the
increment to `[−100*h,100*h]`, add it to the old collective, and finally limit
the result to **[−150,100]**. These last two constants bound a control value,
not a world altitude or a vertical velocity. There is no direct assignment of
position or vertical velocity. The binary32 operands 0.3, 0.1 and 0.5 have bits
`3E99999A`, `3DCCCCCD` and `3F000000`. Evidence:
`0x005353C9–0x0053547D`, including both final bound stores.

The animation caller converts milliseconds to seconds with binary32
`3A83126F`, then applies the AI multiplier where eligible and caps the result
at binary32 `3E4CCCCD` (0.2). This step is passed to both `AI_PursueGoal` and
`UpdatePhysics` (`0x0053C647–0x0053C69E`). Thus the `h` supplied to reset,
hover and chase is already in seconds. In this caller the AI multiplier is
suppressed when the helicopter is player-driven, or when its chase state
references a target containing PLAYER_DRIVE (`0x0053C613–0x0053C645`).

#### 4.2.2 Chase distance gates (AF3) *[compat]*

For CHASE (52), a null target or a target whose **top** state is not
PLAYER_DRIVE (16) makes the helicopter pop the chase goal and return before
steering. A valid target supplies position `T`, orientation `D` and speed `s`.
The lead position is `L = T + D*(s*1000*h)`, with the multiplication by 1000
literal. The caller has already converted milliseconds to seconds (§4.2.1):
do not multiply an unconverted millisecond delta by 1000 again. The speed's
unit convention and conversion are preserved rather than replaced by a
conventional `s*h` lead. Evidence: `0x0053BBD4–0x0053BC90`.

Let `v=(L.x−x,0,L.z−z)` and `d=length(v)`. The horizontal direction is formed
by dividing v by d before the distance branches; there is no zero-distance
guard on this normalization. The helicopter requests engine startup, enables
the spotlight and asks its aim helper to track L before the following gates.

| Condition, evaluated with the pre-teleport d | Behavior |
|---|---|
| d > 192 and current opacity exactly 0 | place X/Z at `L − normalize(v)*128`, preserve the old world Y, synchronize the rotor's scaled position, request opacity 1 |
| d > 192 and current opacity exactly 1 | request opacity 0; no teleport on this visit |
| d > 192 and opacity strictly between 0 and 1 | no teleport or new opacity request |
| d > 32 | call horizontal movement with v, speed argument 2 and h, then building-clearance lift |
| d ≤ 32 | call the same horizontal movement, then hover at `L.y+20` |

The opacity operations precede the last two rows. Teleportation does not
recompute d or v for the subsequent controls on that visit. At exactly 192
there is no fade/teleport operation; at exactly 32 the hover branch is used.
There is no positive-opacity teleport. The opacity request duration is the
shared runtime fade-duration value; no new literal duration is assigned here.
Evidence: `0x0053BC94–0x0053BE5A`; binary32 distances 192, 128, 32 at
`0x00B09FDC`, `0x00B09FD8`, `0x00B09FD4`; height offset 20 at `0x00A98F94`.
The rotor position synchronization multiplies each world component by binary32
`40555555` (approximately 3.33333325386). Spotlight aim smoothing and the
movement/building-lift laws remain AF5/AF3.

#### 4.2.3 Helicopter phase requests (AF4) *[compat]*

The helicopter's phase setter does **not** have the airplane's same-phase
early return. Every invocation stores the requested runtime phase and applies
the actions below. The engine flag is independent of the phase number for
requests other than 0, 1 and 4. Sound requests use the §3.5 AEMS operations and
are conditional on the player-drive flag.

| Requested phase | Engine and motion effects | Ordered sound requests |
|---|---|---|
| 0 | engine off; clear landing latch; reduce each nonzero finite linear/angular velocity vector to zero | end 9, end 8, end 10, start 10 |
| 1 | engine on | none |
| 4 | engine on | start 9 |
| any other value, including 3 | preserve engine flag and velocities | none |

Afterward, on **every** call, notify the base engine hook of whether the engine
flag is nonzero. When the engine is off, send operation `+0x1C(1)` to the
embedded object at helicopter +0x240 and `+0x18(0)` to the one at +0x22C;
when on, send `+0x18(0)` to +0x240 and `+0x1C(1)` to +0x22C. These paired
rotor model/animation notifications repeat even for an unchanged phase. Their
concrete attachment construction remains in AF4; they are not extra AEMS sound
group requests. Phase 0's velocity operation divides zero by the vector's
length and multiplies each component only when squared length is positive;
it does not replace that operation with unconditional zero stores.

Evidence: request dispatch `0x00533DC0–0x00533E27`, phase-0 velocity operations
`0x00533E31–0x00533EE7`, sound actions `0x00533EF3–0x00533F1C`, unconditional
phase write and notifications `0x00533F20–0x00533F96`. This corrects any summary
that applies the airplane's repeated-phase no-op rule to the helicopter.

#### 4.2.4 Building-clearance height demand (AF3) *[compat]*

`AI_ControlLiftOverBuildings` (`0x00537130`) requests an absolute hover height;
it does not move the helicopter directly. For finite inputs, take two samples:
the current world position `P`, then `P + 0.3*V`, where `V` is the runtime
rotor's linear velocity at +0xD0/+0xD4/+0xD8. The multiplier is binary32
`3E99999A`; do not multiply it by the supplied frame step. Each predicted
coordinate is stored as binary32 before querying its height. The original
obtains the auxiliary height service once before taking either sample.

At each sample, obtain the aircraft-base `GetAltitude` height. If the terrain
provider accepts that X/Z point, also obtain the auxiliary height at that point
(the service returned by global `0x00B43D1C` operation +0x24, queried by
operation +0x18 with arguments **1, X, Z**). The sample height is the greater
of the two; outside the terrain provider's accepted region it is just
`GetAltitude`. This specifies the owner-side combination without assigning a
new physical meaning to the auxiliary service's height.

Let `B` be the greater of both sample heights and the home Y coordinate
(helicopter +0x1F4). Start with requested height `H = B + 20`. If current
world Y is greater than H, subtract `min(Y-H,50)` from H. Otherwise leave H
unchanged. Then call the §4.2.1 hover controller with this H and the original
frame step. In particular, descending from above the clearance plane can
request a height **below B+20**; it does not clamp the final demand back to
that plane. No terrain ray sweep or additional intermediate sample is made.
Evidence: sampling/maximums `0x0053714A–0x00537277`, home-height floor and
20-unit offset `0x00537279–0x00537294`, descending adjustment and original-step
forwarding `0x00537298–0x005372E9`. Constants 20 and 50 are binary32
`41A00000` and `42480000`.

#### 4.2.5 Runtime allocation and Shutdown cleanup (AF4, partial) *[compat]*

On first `InitInstance`, the rotor runtime owns a 0x1D0-byte state, a separate
0x7C-byte parameter block and ten separately allocated 0x94-byte transform
objects. The parameter block receives the §4.1.1 preset and overlays before
the transforms are allocated. Transform allocation order, expressed as offsets
of the owning pointer within the runtime state, is +0x94, +0x90, +0xA8,
+0xAC, +0xB0, +0x98, +0x9C, +0xB4, +0xA0, +0xA4. The state word at +0x08
is cleared before these allocations; +0x1BC is cleared at their completion.
An existing runtime skips this entire operation. Evidence:
`0x005385C4–0x0053885D`.

This initialization has **no recoverable allocation-failure result or rollback**.
Failed allocations are represented as null, but the state, parameter block
and several transforms are subsequently dereferenced without a null guard.
Do not infer a successful inert helicopter or a clean false return from these
allocation branches. This is an original-program failure-path quirk, not a
requirement to emulate a host access violation.

After the aircraft-base Shutdown call, helicopter cleanup proceeds even when
that call returned false. In order, each present external effect at helicopter
+0x200, +0x204, +0x224 and +0x228 receives stop operation +0x10(1), then its
owning field is cleared and the retained object released through +0x08.
The embedded objects at +0x22C and +0x240 each receive +0x1C(1) then +0x04(),
in that order. Runtime cleanup frees the ten transforms in the allocation
order above, then the parameter block, then the runtime state, clearing the
runtime pointer. The separate control buffer at helicopter +0x258 is also
freed and cleared when present, independently of runtime presence. The return
value remains the aircraft-base Shutdown result. Evidence:
`0x005396D4–0x005397B7`, `0x00533C40–0x00533D44`.
This settles runtime ownership and the post-base cleanup sequence; attachment
construction, the pre-base Shutdown scale handling, and the full Init/physics
phase-trigger contracts remain AF4.

### 4.3 Named player weapon requests (AF5, partial) *[compat]*

Both aircraft command translators first run the base command translator. They
then examine command slot 7 (the binary32 value at player record +0x98). For
a finite value below zero no weapon request is made. A nonnegative value,
including zero, calls operation +0x14 on the aircraft's embedded effect object
at +0x18C. Only when the player record's readiness byte at +0x1FF is nonzero
does the owner then attempt a named effect, through operation +0x30 with the
name and two zero arguments. The airplane attempts `jetbomb`. The helicopter
attempts **`helicoptermissle` first and `ufolaser` only if that attempt fails**.
The latter is a failure fallback, not an aircraft-type/UFO branch. Preserve
the original `missle` spelling.

On named-effect success, the owner clears the player record's repeat timer at
+0x1E0 and readiness byte at +0x1FF, and increments its slot-7 use count at
+0x178 by one (32-bit wrapping). If all attempts fail, those three fields are
unchanged by this owner block. It does not decrement the airplane's water-drop
ammunition, install a fixed weapon cooldown, or construct a projectile itself
here. Cadence comes from base command readiness and effect processing; effect
attachment, projectile setup and the rest of the player-control contract are
still AF5. Evidence: airplane `0x0050114C–0x005011ED`, helicopter
`0x00539A61–0x00539B50`.

#### 4.3.1 Helicopter takeover parameter refresh (AF5) *[compat]*

`OnPlayerTakeOver` (`0x00534270`) uses the current player parameter record,
preferring the direct record at provider +0x64. Only if it is absent does it
ask the provider's script object (+0x0C) for its record. Missing provider,
missing script or failed fallback leaves all five destinations unchanged.
It does not merge a present direct record field by field with the script.

For a present record, the speed cap in the supplied rotor parameter block
(+0x68) receives record +0x08 multiplied first by binary32 1000 and then by
binary32 `40555555` (approximately 3.333333254). If that converted value
compares equal to zero, use the supplied block's +0x20 yaw-rate maximum.
The four other destinations are helicopter-owned input gains:

| Player record field | Helicopter destination | Use |
|---|---|---|
| +0x60 | +0x260 | pitch-input change rate |
| +0x64 | +0x264 | yaw-input change rate |
| +0x68 | +0x268 | collective-input change rate |
| +0x74 | +0x26C | reversal multiplier for pitch/yaw input |

Each gain replaces its current value only when the source is nonzero. Both
signs of zero preserve the previous value; negatives and NaNs are copied,
without validation. The same zero comparison on converted speed falls back
only for zero, not NaN. Each result is stored as binary32. Repeating takeover
therefore preserves earlier nonzero gains when a later record supplies zeros;
this helper is a refresh, not a reset to the initial preset. It makes no
phase, engine, weapon-use or sound change. The initial allocation calls this
refresh before the AI overlay (§4.1.1). This specifies the helper itself;
release scheduling and the full command translator remain AF5.
Evidence: record selection `0x00534273–0x005342A0`, speed conversion/fallback
`0x005342A6–0x005342CF`, gain refresh `0x005342D2–0x0053435D`.

#### 4.3.2 Spotlight pair activation (AF5, partial) *[compat]*

`UpdateSpotlight` does nothing unless **both** effect pointers at helicopter
+0x200 and +0x204 exist. With both present it queries PLAYER_DRIVE (state
0x10), then requests that the pair be active exactly when the environment
service at `0x00B43DC4` operation +0x58 returns **1**, the helicopter's
spotlight-enabled byte (+0x220) is nonzero, and the runtime flight phase is
nonzero. Other environment return values do not enable the pair. This gate
does not require a chase target or a PLAYER_DRIVE state.

For each effect, in +0x200 then +0x204 order, query active status through
+0x14(). When activation is requested, start with +0x0C(0) only if inactive;
when deactivation is requested, stop with +0x10(1) only if active. This avoids
restarting an already active effect. A failed activation gate does **not**
return from the update: the subsequent transform/aim work still runs for the
present pair. Geometry and terrain projection are the remaining AF5 spotlight
contract; this gate alone is not a complete spotlight update.
Evidence: `0x00537B8A–0x00537C46`.

## 5. The air dispatch sources *[compat]*

`cSC4FirePlaneDispatchSource` and `cSC4PoliceHelicopterDispatchSource` track their stations like the ground sources
(protection.md §8): when a building occupant (`0x278128A0`) with a non-zero unit count (property `0x0C0AFDD7` for
fire planes) is inserted, it is added to the source's list, its units are numbered after the other stations' in a
bit vector, and its available-units property (`0xCC0AFDDB`) is set to all units (`2^n − 1`); removal reverses it
*[summary of the removal]*.

The available-units property is a **32-bit** mask. Station insertion accepts a positive
signed 32-bit unit count; it does not clamp that count to 32. Its mask is
`(1u32 << (count modulo 32)) − 1`, with subtraction wrapping in 32 bits
(`0x006ACBEC–0x006ACBF5` count gate; `0x006ACC39–0x006ACC53` shift, decrement and property write).
Consequently counts 31, 32, 33, 63 and 64 produce `0x7FFFFFFF`, **0**, 1,
`0x7FFFFFFF` and **0**, respectively. In particular the previous `2^n − 1`
description is valid only for counts 1–31, not 32. Counts with the sign bit set are
rejected by this insertion gate. The separate station-number allocation uses the full
accepted count; a small wrapped availability mask does not shrink station membership.

## 6. Quirks

1. `ClipToWorld` allows positions 1000 units outside the city.

## 7. Open questions

See [questions/aircraft.md](questions/aircraft.md) (AF2–AF6). AF1, AF7 and AF8
are resolved in §§3.1, 4.1 and 5; their entries are archived in
[questions/answered/aircraft.md](questions/answered/aircraft.md). The flight-model,
control, lifecycle, weapon and message summaries do **not** close those owner contracts.

## 8. Conformance tests

1. An airplane exemplar without drive parameters gets 100, 1, 25, 150 and the angle defaults of §3.1; one that
   gives entry 2 as 3 gets 3000.
2. An entry 5 of 0 with entry 4 = 0.4 gives 0.1.
3. A fire-plane station with a unit count of 3 sets its available-units property to 7.
4. Import `airplane = { ElevatorMax = 0.4 }`: its other named fields import as zero;
   entry 5 becomes binary32 0.1 and the record prevents ancestor-field inheritance.
   With no `airplane` table, an ancestor record is used; with no record anywhere,
   entry 9 has bits `3D32B8C3` and entry 4 has bits `3E860A92`.
5. Load entry 2 = −3: takeoff speed is −3000. A NaN `ElevatorMax` remains NaN,
   including its derived zero `ElevatorChangeRate`; infinity `MaxSpeed` remains
   infinity. This tests the absence of loader sanitization, not later flight behavior.
6. Fire-plane station count/mask pairs: 0/no insertion, 1/1, 31/`7FFFFFFF`,
   32/0, 33/1, 63/`7FFFFFFF`, 64/0, `80000000`/no insertion. No AF8 error or
   saturation is substituted for the original wrapping mask.
7. Helicopter record with all fields zero preserves every preset field. Giving
   only `BankMax = 1` changes that maximum to binary32 `440F3D4D` while preserving
   `PitchMax`; an ancestor with `PitchMax = 2` is not merged into the present record.
8. Isolate the pre-force velocity limiter (§3.4.1). With `MaxSpeed = 150`,
   `(0,0,0)` stays zero, `(0.00001,0,0)` becomes approximately `(1,0,0)`,
   `(90,120,0)` stays unchanged, and `(120,160,0)` becomes approximately
   `(90,120,0)`. With `MaxSpeed = 0.5`, the tiny nonzero vector still becomes
   speed 1: it does not also pass through the maximum-speed branch.
9. Isolate candidate integration from surface and attitude corrections. For
   `p0=(1,2,3)`, `v0=(10,0,0)`, `m=100`, `F=(200,0,0)`, `h=0.1`, expect
   `v1≈(10.2,0,0)` and `p1≈(2.01,2,3)`, with component tolerance `0.00001`.
   At `v0=(150,0,0)` and the same force/step, candidate `v1.x≈150.2`:
   there is no post-acceleration speed cap at this stage.
10. Player-drive time conversion: input 100 ms gives approximately 0.1 s;
    1000 ms gives the encoded 0.2-s cap and only one physics step. Input 0 ms
    gives 0 s. A negative finite input is not raised to zero by this caller.
11. Isolate the scalar lift curve (§3.4.2), with `sigma=A/3`, `C0=0.08`,
    `Cs=0.68`, `C90=0`. Angles `0`, `sigma/2`, `sigma`, `(sigma+A)/2`, `A`
    yield coefficients approximately `0.08`, `0.38`, `0.68`, `0.34`, `0`.
    Tolerance `0.000001`. Horizontal velocity and horizontal orientation
    produce angle `A` through the specified inverse cosine, not angle zero.
    For the separate boundary case `sigma=A`, `alpha=A`, the coefficient is
    approximately `Cs`: equality uses the lower segment, avoiding the upper
    segment's zero denominator.
12. Scalar pressure and ground resistance: `s=10`, resistance factor 1,
    `FlapsMax=0.4`, command `f=0` gives `Q=50`; `f=1` gives approximately 70.
    With mass 100 and vertical lift zero, phases 0/1 give `R≈100`; phase 2
    gives zero. Vertical lift equal to weight gives zero rolling resistance;
    half weight gives `R≈50` in phase 1. Tolerance `0.0001`.
13. Thrust gate: with `ThrottleMax=10000`, `d=(0,0,1)`, `t=−0.5`, compare
    `v=(0,0,10)`, zero velocity, and `v=(0,0,−10)`. Phase 1 produces thrust
    magnitudes `−5000`, `−5000`, `0` respectively; phase 2 produces `−5000`
    in all three cases. Test the thrust component separately from total force.
14. Fresh helicopter, no player or AI parameter record: after Init, all thirteen
    parameter words equal §4.1.1 and phase is 0 with engine disabled. Compare the
    words exactly, including `SpeedMultiplier=3F333333`; do not derive them from
    rounded decimal angles or speeds.
15. Fresh helicopter with a present all-zero player record and absent AI record:
    MaxSpeed is 400 (`43C80000`), while the other twelve rotor fields keep their
    preset values. A subsequent all-zero AI overlay still preserves 400. With AI
    `MaxSpeed=3`, expect approximately 10000 internal units instead (the §4.1
    conversion overrides the player result).
16. Round-trip a helicopter whose runtime phase is 4 and whose base goal stack
    contains a nondefault goal. Read alone creates no runtime rotor state. Init
    reconstructs the normal preset/overlays and starts phase 0 with engine off;
    the base goal remains available independently. On an existing runtime model,
    change Mass to 7, then call InitInstance again: Mass remains 7. A direct
    PhysicsInit resets phase to 0 but does not recopy Mass from the preset.
17. Isolate helicopter reset with controls `(b,p,c,r)=(30,−30,0,−20)`,
    `h=0.1`, `g=1` and the normal preset. Expect controls approximately
    `(20,−20,−10,−10)`, bank demand +200 and pitch demand +200. Tolerance
    0.0001. Repeat at `h=0`: controls stay fixed but angular demands are still
    recomputed. A step of 1 reaches `(0,0,−100,0)` without overshooting.
18. Isolate hover with `y=100`, `vy=10`, old collective 0, target height 100,
    `h=0.1`: expect collective approximately −1.575 (tolerance 0.00001).
    With zero velocity, target 1000 and the same old collective/step, expect
    +10 due to the increment limit. Starting at collective 99 instead gives
    100 after the final bound. A very low target, old collective −149 and
    `h=0.1` gives −150. Position and velocity are unchanged by this helper.
19. Chase lead: target `(10,20,30)`, orientation `(1,0,0)`, speed 0.02 and
    already converted step 0.1 seconds gives lead approximately `(12,20,30)`.
    Use tolerance 0.00001. Do not feed 100 milliseconds directly to the helper.
    With target stationary, helicopter `(0,40,0)` and target `(200,10,0)`,
    opacity 0: teleport to approximately `(72,40,0)`, synchronize the scaled
    rotor position, request opacity 1, then use the pre-teleport movement branch
    (`d=200>32`). At opacity 1 request opacity 0 without moving instantly;
    at opacity 0.5 make neither opacity request nor teleport. Distances exactly
    192 and 32 exercise respectively the no-fade and hover branches.
20. Airplane LOITER with a clear drop flag and ammunition 0 installs countdown
    2.0, sets the flag and changes ammunition to −1. Another goal visit without
    a countdown advance makes no further decrement. With the flag set, direct
    countdown values `0.0001`, `0`, `−0.0001` respectively keep, pop and pop the
    goal; popping clears the flag. An animation tick starting with countdown
    0.125 and a 0.125-second physics step reaches zero and expires that visit.
    Read on a fresh airplane does not restore the saved drop progress or ammo:
    its first restored LOITER visit starts 2.0 with ammunition 2 (§3.3.1).
21. Airplane phase 0→1 refills ammo to 3 and, if player-driven, requests start
    group 9. Set ammo to 1 and request 1 again: ammo stays 1, no sound request.
    Phase 3→1 refills and requests end 13, start 13. Phase 1→0 requests end 9,
    end 8, end 10, start 10. Clear the player flag: phase/refill changes still
    occur, with no sound requests.
22. Helicopter phase 4→4, player flag set: engine remains enabled and start 9
    and the base/model notifications repeat. Phase 0→0 repeats end 9, end 8,
    end 10, start 10 and the engine-off notifications. Phase 3 preserves an
    already enabled engine; requesting it from an engine-off object keeps the
    engine off. Neither case adds a sound request.
23. Ready helicopter weapon command 0: successful `helicoptermissle` makes no
    `ufolaser` attempt. Failed missile followed by successful laser counts as
    one use and clears readiness/repeat timer. Both failing preserves those
    fields. Not-ready command 0 still makes the preliminary +0x14 call but no
    named attempt. Command −1 makes neither call. Successful airplane `jetbomb`
    changes the player use count, not its water-drop ammunition.
24. Building clearance, finite values: P=(10,100,20), runtime V=(10,0,20)
    queries current X/Z=(10,20) then predicted X/Z approximately (13,26),
    irrespective of the hover frame step. Give current/future GetAltitude
    heights 30/40, auxiliary heights 35/60, both points accepted, and home
    Y=50. Then B=60 and preliminary H=80; with current Y=100 the descending
    adjustment requests H=60. With the same service heights and Y=130 it
    requests 30 (the gap is exactly 50); Y=180 still requests 30. With Y=70
    it requests 80. Compare requested heights exactly for these inputs.
    Reject both points at the terrain provider: no auxiliary height queries,
    B=50 and the preliminary H=70. Feed the resulting demand to the hover
    controller rather than changing position directly.
25. First runtime allocation makes one state, one parameter block and ten
    transforms in §4.2.5 order. Repeating InitInstance makes no allocations
    and preserves the current parameters. Shutdown with all four effects
    present stops/releases them in offset order, cleans both embedded objects,
    frees all runtime allocations and the control buffer, and clears both
    runtime/control pointers. Force the aircraft-base Shutdown result false:
    these cleanup actions still occur and the helicopter returns false.
    Missing external effects are skipped; no runtime but a present control
    buffer still frees that buffer. Do not treat allocation failure as a
    successful initialization case.
26. Takeover with a present direct record whose speed and gains are all zero,
    supplied yaw-rate maximum 400 and existing gains (10,20,30,4): speed
    becomes 400 and gains remain (10,20,30,4), even if the fallback script
    would supply nonzero fields. Refresh with record gains (5,0,−2,3): expect
    (5,20,−2,3). Refresh with signed-zero gains preserves these values;
    missing provider preserves speed as well. A nonzero source speed 0.03
    gives approximately 100 (tolerance 0.00002). No phase or weapon-use change.
27. Spotlight with either pointer absent: no status/start/stop or geometry
    calls. With both present, environment result 1, enable byte 1 and phase 4,
    first inactive/second active: start only the first, argument 0. Repeat:
    neither restarts. Change the environment result to 2: stop both, argument
    1. Repeat while inactive: no stop requests. Independently clear the enable
    byte or set phase 0: same inactive demand. For every both-present case,
    including those demanding inactive effects, continue to geometry work.
