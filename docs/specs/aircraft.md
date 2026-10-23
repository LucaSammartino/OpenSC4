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
forces. For finite speed `s = length(v)`:

| Condition, tested in this order | Velocity supplied to the force model |
|---|---|
| `s < epsilon` and `s > 0` | `v / s`, hence speed **1**, not zero |
| `s < epsilon` and `s = 0` | unchanged zero vector |
| Otherwise, `s > MaxSpeed` and `s > 0` | `v × (MaxSpeed / s)` |
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
| `alpha < sigma` | `C0 + ((Cs − C0) / sigma) × alpha` |
| `alpha >= sigma` | `Cs + ((alpha − sigma) × (C90 − Cs)) / (A − sigma)` |

There is no denominator guard for a loaded zero or out-of-range stall angle in
this calculation (`0x004FF1EE–0x004FF243`). Define the pressure-like scale,
lift magnitude and drag normalization in this operation order:

- `Q = ((((FlapsMax × f) + 1) × s) × s) × AirResistanceFactor × 0.5`;
- `L = C × Q`;
- `K = ThrottleMax / ((((MaxSpeed × MaxSpeed) × AirResistanceFactor) × 0.5) × k)`.

Here `k` has binary32 bits `0x3DCCCCCD` (approximately 0.1). `K` is stored as
binary32 before being used by the drag calculation (`0x004FE85A–0x004FE8A0`);
`Q` is formed at `0x004FF1CB–0x004FF1EE`. Flaps scale both lift and aerodynamic
drag through `Q`; they do not add a separate constant force.

Let `Fy_lift` be the vertical component after applying magnitude `L` to the
attitude-derived lift direction. The extra rolling resistance is zero except in
phases **0/1** when `Fy_lift < m × g`. In that case its magnitude is

`R = (1 − Fy_lift / (m × g)) × m × s × k`.

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

### 4.2 Flight and control overview

- **Parameters** (`LoadDriveParameters` `0x005340A0`): the 13-field record and
  exact conversion constants are specified in §4.1.
- `PhysicsInit` and `InitInstance` set up the rotor model; `NextFrame` (`0x005397C0`) computes the engine power,
  then yaw, pitch, roll and slide (`YawPitchRollSlideReal`, with limits ±890 and 250), the
  horizontal move (`MoveXZReal`) and the vertical move (`MoveYReal`, kept above `GetAltitude`) *[summary]*.
- **AI controls**: `AI_ControlReset`, `AI_ControlHover` (bounds −150 and 100 *[inferred: vertical rates]*),
  `AI_ControlMoveXZ` (bands ±10 and ±20 *[inferred]*), `AI_ControlLiftOverBuildings` (climbs when the ground ahead
  is higher), and `AI_PursueGoal` (automata.md §6.6, including the chase of case `0x34`).
- **Spotlight** (`UpdateSpotlight`, 2 KB): the light cone follows the ground below or the target *[summary]*.
- **States** (`SetHelicopterState`): each starts or stops its AEMS rotor sounds.
- **U-Drive-It**: `OnPlayerTakeOver` loads the player parameters; `TranslatePlayerDriveCommands` (2.4 KB) flies it
  and fires `helicoptermissle` or, for the UFO, `ufolaser`.

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
12. Scalar pressure and ground resistance: `s=10`, resistance factor 1,
    `FlapsMax=0.4`, command `f=0` gives `Q=50`; `f=1` gives approximately 70.
    With mass 100 and vertical lift zero, phases 0/1 give `R≈100`; phase 2
    gives zero. Vertical lift equal to weight gives zero rolling resistance;
    half weight gives `R≈50` in phase 1. Tolerance `0.0001`.
13. Thrust gate: with `ThrottleMax=10000`, `d=(0,0,1)`, `t=−0.5`, compare
    `v=(0,0,10)`, zero velocity, and `v=(0,0,−10)`. Phase 1 produces thrust
    magnitudes `−5000`, `−5000`, `0` respectively; phase 2 produces `−5000`
    in all three cases. Test the thrust component separately from total force.
