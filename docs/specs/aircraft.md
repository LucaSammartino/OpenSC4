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
their handling from named Lua group records: 18 airplane fields and 13 helicopter fields. Two dispatch sources let
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
`LoadPlayerDriveParameters` refreshes maximum speed and player gains for U-Drive-It (§4.3.3).

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

`Init` (`0x005017C0`) loads both parameter sets, evaluates three synthetic flight calibrations (`0x004FF840`,
`0x004FFAE0`, `0x004FFD80`; §3.2.1), and prepares effects: the propeller (`fx_00_planepropstill` / `fx_00_planeprop`, or the fire-plane
versions `fx_00_fireplanepropstill` / `fx_00_fireplaneprop`), sky writing (`fx_00_skywriting`), the water drop
(`fx_00_waterdrop`), the indicator (`aircraftindicate`) and the bomb (`jetbomb`, U-Drive-It).

#### 3.2.1 Constructor and teardown additions (AF4) *[compat]*

The airplane constructor installs phase 0, drop-active false, countdown 0,
water ammunition 3, zero velocity (+0x234 triple), zero angular fields
(+0x240 triple), null effect pointers +0x208/+0x20C/+0x210/+0x214, and null
fire filter +0x344. Loaded-placement byte +0x348 and dispatch state +0x334
start at zero. Home +0x34C triple starts at zero. These are constructor
values, not fields restored by the airplane record.

The player gains +0x300 through +0x32C, in ascending four-byte order, start
with binary32 bits `3ECCCCCD,3E99999A,3E4CCCCD,3F800000,3F000000,3E4CCCCD,
3DCCCCCD,3E99999A,3DCCCCCD,3E4CCCCD,3E4CCCCD,3DCCCCCD`. The fire-search
anchor +0x338 triple starts at the file-backed sentinel (−1,−2,−3).
Evidence: `0x004FCFE0–0x004FD25C`.

The three helpers previously called “ray probes” are **not terrain-ray queries**.
Each starts from orientation `(0,sin(ElevatorMax),cos(ElevatorMax))`, with
velocity equal to that direction times TakeOffSpeed. Each permits up to 100
force/integration/attitude iterations with a synthetic 0.1-second step. Their
synthetic six-value control records are respectively `(1,0,0,1,1,0)`,
`(−1,0,0,owner[+0x31C],1,0)`, and
`(owner[+0x320],1,1,owner[+0x31C],1,0)`.
No city terrain is sampled in this initialization calculation. The first two
return an absolute velocity-component ratio, and the third an angular quantity;
the complete convergence/attitude arithmetic is included in the AF2 observation
boundary (§9). Evidence: `0x004FF840–0x005000A2`.

Airplane Shutdown releases/nulls the fire filter first. For each present effect
in order +0x20C, +0x208, +0x214, +0x210: stop through +0x10(1), clear its owner
field, Release through +0x08. It then tail-calls aircraft-base Shutdown and
returns that result. This cleanup occurs **before**, not after, the base call
(`0x004FD2C0–0x004FD37B`).

Worked case: create without reading a record and without visiting a goal;
expect phase 0, ammo 3, zero countdown and velocity. Supply four mock effects
and a false-returning base Shutdown: expect the listed stop/Release order,
all owner pointers cleared and false returned. No effect stop follows the
base call on the airplane path.

### 3.3 AI goals (`AI_PursueGoal`, `0x00504590`; automata.md §6.6)

The goal stack drives a set of behaviours, each a steering routine over the flight model:

- `AI_GotoDestination` (fly to a point: take off first when on the ground, turn to the vector, follow it, keep the
  approach altitude over the ground); `AI_Land` (pick a runway direction and an approach box, descend along the
  approach ray to the base altitude, touch down, `SetAirplaneState`, stop, taxi); `AI_TakeOff` (pick a runway
  direction, taxi to its start, turn, roll and climb); `AI_Taxi`, `AI_Stop`, `AI_TurnToVector`, `AI_FollowRay`,
  `AI_FollowVectorAndTurn`, `AI_ApproachAltitude` *[summary of each control law]*.
- **Runways.** `PickRunwayDirection` (`0x00502520`) selects from paired runway endpoints using the
  clearance-derived random weights in §3.3.2. It does not choose a minimum-clearance direction.
  `PickApproachBox` uses 500 as a minimum nominal leg length, not a maximum (§3.3.3).
- **Fire planes.** `FindFireNearLocation` uses the first returned burning occupant in a square search
  region, rather than computing the nearest one (§3.3.2).
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

#### 3.3.2 Runway clearance, selection and fire search (AF3) *[compat]*

`EvaluateClearance(P,D)` samples exactly twenty points, incrementally adding
`16 × D` to P before each query. Each updated coordinate is stored as binary32.
At sample i (1 through 20), call the terrain provider's region test +0x2C(X,Z),
then aircraft `GetAltitude`. If the region test succeeded, **replace** that
altitude by the auxiliary provider's +0x18(1,X,Z) result; do not take their
maximum here. The score is the largest positive `(height − original P.y)/(16×i)`,
initially zero. Equality, negative slopes and unordered comparisons do not replace
the current score. Thus this measures a rising slope, not absolute obstacle height
or aircraft clearance. Evidence: `0x004FD790–0x004FD899`.

`PickRunwayDirection` obtains a list of 12-byte position triples. Fewer than two
triples, or an odd count, calls owner +0x34(0) and returns without supplying a new
runway. Otherwise process pairs (A,B) in list order. Normalize B−A in X/Z with Y=0
for scoring; there is no zero-length guard. Evaluate from A in that direction,
and from B in its negation. For a true final flag use owner +0x290 as the slope
allowance; for false use +0x28C. Each weight is binary32
`max(EvaluateClearance − allowance, 0)`, widened to binary64 in the weight list.
These literal weights favor **positive excess slope**, a compatibility quirk.
Do not silently invert the score to obtain a more plausible clearance policy.
Evidence: `0x00502520–0x0050272C`.

Selection uses the shared random service at `0x00B21AF4`: sum positive weights;
if the sum is zero select a uniform index across all endpoints. Otherwise draw
uniformly over zero to the sum, scan in order, choose the first positive weight
strictly greater than the remaining draw, subtracting each preceding positive
weight. Exhaustion returns the last index. There is no deterministic first-wins
tie rule (`0x00913AD3–0x00913B73`). Pair the chosen index with its adjacent index
(even +1, odd −1); swap these indices when the final flag is false. Copy the
selected start to the supplied position output, normalize the full 3D difference
to the other endpoint into the direction output, and store that other endpoint
at owner +0x2F0/+0x2F4/+0x2F8 (`0x0050273D–0x00502800`).

Fire search (`0x00503B50`) uses an axis-aligned box X/Z = center ±100 and Y =
±largest finite binary32, not a radius-100 circle. Pass the owner's +0x344
filter and −1 to the fire service +0x90 query. On query success, count the
returned list; when nonempty, ask the **first** occupant's +0x20 operation for
the output position. Return true exactly for positive count. Release the query
list through service +0xC0 and release the retained service reference. Query
failure or an empty list returns false; it supplies no replacement target.
Evidence: `0x00503B9E–0x00503C96`.

Worked cases: a sample at i=2 with height 26 and P.y=10 contributes 0.5.
With every other contribution ≤0.25 the score is 0.5, even if GetAltitude
reported 100 but the accepted auxiliary provider reported 26. Two flat runway
directions with positive allowances both have weight zero and use uniform
selection. Weights 1 and 3 with a remaining draw exactly 1 select the second,
not the first. A fire at center+(99,0,99) is inside the query box despite its
horizontal distance exceeding 100; list ordering, not distance, chooses it.

#### 3.3.3 Approach-box distance and outputs (AF3) *[compat]*

For finite nondegenerate inputs, take the unit lateral vector `D × worldUp`
and choose its sign toward the city center; a zero center dot product chooses
the negated vector. Let M be the greater city X/Z dimension and let B be the
greater of the base-altitude providers (global +0x28 and terrain +0x58).
The nominal leg length L starts as `(B−runway.y)/owner[+0x290]`.
If L<500, replace it by 500. Otherwise cap L at M. This ordering can leave
L=500 on a city whose M is smaller than 500.

Store the supplied runway point at +0x2E0. From that point subtract L×D,
clip to world, measure the full 3D distance from the runway, and set Y to
runway.y plus that distance times +0x290; store at +0x2D4. Then add L times
the signed lateral vector, clip, add that full 3D leg distance to the running
distance and recompute Y by the same rule; store at +0x2C8. Then add L×D,
clip, accumulate the next full 3D leg distance and recompute Y; store at
+0x2BC. These are four position triples; the arithmetic does not create a
500-unit maximum box. Evidence: `0x005000B0–0x0050049A`, especially the
minimum/cap branches `0x0050023C–0x0050027B`.

Worked case: level runway, D=(1,0,0), +0x290=0.1, B−runway.y=20 and M=1024
give L=500. With clipping inactive the first leg ends 500 units behind the
runway and 50 units above it. Use the measured 3D legs, including their Y
differences, for subsequent heights; do not assume every accumulated leg is L.

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

#### 3.4.1a Non-finite comparisons (AF10) *[compat]*

The following describes the default **masked x87 exception** environment.
It does not prescribe NaN payload bits, exception flags or execution with
floating exceptions unmasked. There is no input sanitization. Comparisons
at `0x00503CEC–0x00503D51` distinguish unordered from less/greater:

| Input to the initial velocity limiter | Result before forces |
|---|---|
| Any NaN component (hence unordered length) | Tiny-speed and maximum-speed branches both skipped; original components retained |
| Finite length, NaN maximum | Maximum comparison unordered: velocity retained, except the earlier tiny-speed branch still applies |
| Infinite length, finite maximum | Scale by maximum / infinite length; infinite components become NaN, finite components become signed zero |
| Infinite length, +infinite maximum | Equality: retained |
| Infinite length, −infinite maximum | Scale factor is NaN, so components become NaN |
| Finite non-tiny length, +infinite maximum | Retained |
| Positive finite non-tiny length, −infinite maximum | Scale by −infinity: nonzero components become infinities; zero components become NaN |

The length used for the maximum comparison is stored as binary32 first.
The squared-length normalization gates require **ordered strictly positive**
squared length (`0x00503D17–0x00503D22`, `0x00503D71–0x00503D7C`). In the
force routine the tiny-speed fallback also requires ordered less-than, but
normalization itself divides unconditionally by the selected direction's
length (`0x004FE8D0–0x004FE92F`). Thus a NaN velocity does not select the
orientation fallback; a NaN or zero selected direction propagates through the
reciprocal. No later assertion of finite force or position is justified.

Animation converts milliseconds to binary32 seconds, stores, then applies
and stores the selected AI multiplier; the cap tests ordered `h>0.2`.
Positive infinity becomes 0.2, negative infinity remains negative infinity,
and NaN remains NaN. Zero times infinity gives NaN and also bypasses the cap.
Player ticking performs the conversion and cap without the AI multiplier.
The animation cap compares the stored product, whereas the player cap compares
the conversion still in x87 precision before using the stored local.
Evidence: `0x00504981–0x005049DA`, `0x00504B01–0x00504B26`.

Worked cases: velocity `(NaN,2,3)` remains `(NaN,2,3)` at the limiter; it
must not be normalized or replaced by orientation there. `(infinity,2,0)`
with maximum 150 becomes `(NaN,+0,+0)` before forces. Infinite positive
milliseconds and AI multiplier −1 produce negative infinity seconds; a
zero-millisecond frame and infinite AI multiplier produce NaN seconds.
Compare classification and unaffected finite component bits, not NaN payloads.

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

#### 4.2.6 Rotor sequencing, power and home completion (AF2/AF4) *[compat]*

The rotor state R is reached through owner +0x254; its parameter block P is
R+0, control block C through owner +0x258, and transform T through R+0x94.
These offsets identify native records, not public interface slots.
`NextFrame(R,h)` invokes engine-power, attitude/slide, horizontal motion,
then vertical motion, once each with the same seconds step. Only afterwards
does it copy T's internal X/Z (+0x18/+0x20) to R+0x18/+0x1C and, if changed,
to R+0x10/+0x14. Phase 1 then adds h to R+0x1C4. Phase 6 exits before these
four steps (`0x005398B1–0x00539988`).

For finite inputs, engine power (`0x005346D0–0x005348BC`) obeys these ordered
rules. Let E=R+0x148 and engine flag e=R+0x1CC. With e=0, set
E=clamp(E−100h,0,400). With e nonzero and E<400, set
E=clamp(E+100h,0,400); otherwise leave E unchanged. Thus flags other than 1
can spin the rotor but do not activate the powered-force branch below.

Set C+0x2C to `(C[+8]+100) × binary32 0.005`, without an initial clamp,
and C+0x28 to zero. Only when e=1 set the latter to
`C[+0x2C] × P.Power × clamp(E × binary32 0.0025,0,1)`.
Then form q from R+0xD4 (vertical velocity) twice and R+0xC8 (mass), in
the order `vy × k × vy × mass`:

| Condition | k (binary32 bits) | Final correction to C+0x28 |
|---|---|---|
| e=1 and vy≥0 | `3C9374BC` (0.018) | subtract q×4 |
| e=1 and vy<0 | same | add q |
| e≠1, R+0x10C<0 and R+0xCC>10 | `3CC0AD04` (0.02352) | add q×C[+0x2C]×4 |
| Other e≠1 | same | first clamp C+0x2C to [0,1], then add (1−C[+0x2C])×q |

Products and the indicated field stores use binary32 fields with x87
intermediates; 100,400,4 are exact, while 0.005 and 0.0025 are respectively
`3BA3D70A` and `3B23D70A`. This power stage does not itself move the object
or request AEMS sounds. Worked case: E=0, e=1, h=1, collective=0, vy=0,
P.Power=240 gives E=100, C+0x2C≈0.5 and C+0x28≈30. Repeat with e=2:
E still reaches 100, but C+0x28 stays zero for vy=0.

`UpdatePhysics` has a distinct home-completion path when owner byte +0x25C
is set. It resets controls, then checks the current goal. Goal 0 or 0x32
with distance to home <1 copies home exactly and requests phase 0. Otherwise
those goals blend position toward home using
`a=min(binary32(h × binary32 1/3),1)` and `(1−a)×position+a×home`.
For any other goal clear +0x25C. This path synchronizes internal transform
position from world position using binary32 `40555555`, and skips NextFrame
and collision detection for that visit. Evidence: `0x0053B4E0–0x0053B65F`.
Worked case: position (30,0,0), home zero, h=0.3 and goal 0x32 move to
approximately (27,0,0), with no phase-0 request yet. Starting at (0.5,0,0)
instead snaps to zero and requests phase 0, including its repeated-phase
side effects from §4.2.3.

#### 4.2.7 Shared destination claims before Shutdown (AF4) *[compat]*

Before base Shutdown the helicopter checks its destination (+0x4C triple)
against sentinel (−1,−2,−3). If any component is not within strict binary32
epsilon `38D1B717` of the sentinel, remove **all** matching triples from the
shared list at `0x00B2289C`, then write the sentinel to the owner. Matching
requires each absolute component difference to be ordered <epsilon; equality
at epsilon does not match. Removal unlinks and frees each matching node,
without stopping after the first. Then perform base Shutdown and the
§4.2.5 cleanup. This operation releases destination claims; it is not a
position-scale adjustment. Evidence: `0x00539664–0x005396D6`,
`0x005395F0–0x0053965C`.

Worked case: two shared entries exactly equal the helicopter destination,
and a third differs in X by 1. Shutdown removes the first two, leaves the
third, stores the sentinel, then calls base Shutdown. Repeated Shutdown
with the sentinel makes no claim-removal call.

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

#### 4.3.3 Airplane player refresh and state callbacks (AF5/AF6) *[compat]*

Airplane player refresh uses the same whole-record selection as helicopter
§4.3.1: direct provider +0x64, else script +0x0C fallback, else no changes.
The following record offsets map to airplane binary32 destinations. Multiply
speed by binary32 1000, all other rows by binary32 `3C23D70A` (0.01).
An ordered result equal to either signed zero preserves the current destination;
nonzero values and NaNs replace it. This is a refresh, not a fresh default load.
Evidence: `0x004FCCA0–0x004FCEEE`.

| Record offset | Airplane destination |
|---|---|
| +0x08 | +0x258 (maximum speed) |
| +0x60 / +0x78 | +0x300 / +0x304 |
| +0x64 / +0x68 / +0x6C | +0x308 / +0x30C / +0x310 |
| +0x7C / +0x80 / +0x84 | +0x314 / +0x318 / +0x31C |
| +0x88 / +0x90 / +0x70 / +0x8C | +0x320 / +0x324 / +0x328 / +0x32C |

Both owners call aircraft-base entry/exit first and return its result even
when they perform their own side effects. A non-null entered state with ID
0x10 triggers the player refresh. Airplane additionally sets that state's
+0x30/+0x34 limits to 0/1 and, when its phase is nonzero, requests AEMS start
group 8. Non-player entry instead zeroes owner +0x2A8/+0x2AC/+0x2FC/+0x2EC;
entry to 0x39 copies the current destination (+0x4C triple) to the fire-search
anchor (+0x338 triple). Every airplane entry clears its drop-active byte.
Player exit reloads AI airplane parameters and clears those four control
fields; every exit clears drop-active (`0x004FD640–0x004FD74C`).

Helicopter player entry refreshes parameters, invokes `0x00533FA0`, and sets
the entered state's limits to −100/+100. Player exit reloads AI parameters
and copies parameter +0x5C to runtime +0xDC and +0xC8. Exit from goal 0x35
instead requests dispatch state 2. Null exits do neither. Evidence:
`0x00535480–0x0053555D`. These callbacks describe scheduling of the owner
refresh; they do not imply that parameter loading alone changes the goal stack.

Worked case: start airplane maximum speed 150 and +0x300=2. Refresh with
record +0x08=0.2, +0x60=0 and all other fields zero. Maximum becomes
approximately 200; +0x300 stays 2. Refresh again with an absent provider:
both stay as they are. Exit player state: the AI loader restores its maximum
(150 for no record) and clears the four listed controls, independently of the
base callback's returned boolean.

### 4.4 Goal dispatch and owner callbacks (AF3/AF6) *[compat, bounded]*

Flight phase, goal ID and dispatch state are three separate values. Goal
stack operations below are primary owner +0xD0 PushState, +0xD4 PopState,
+0xE4 FindState and +0xEC current ID (automata.md §6.6). Unless stated
otherwise, pushed goals use priority 0, duration −1 and remaining flags/
payloads zero. The base stack controls the timing of entry/exit callbacks.

| Airplane goal/condition | Ordered owner actions |
|---|---|
| 0, phase>1 | Push 0x32 LAND; return |
| 0, phase=1 | Run Stop controller; do not emit a dispatch state here |
| 0x32 LAND | If dispatch state is neither 4 nor 0, request 6; run Land; on success request 0 then Pop |
| 0x33 destination | Clip destination; Pop on sentinel destination or successful GotoDestination |
| 0x35 LOITER | Request dispatch 5, then run §3.3.1 countdown/drop logic |
| 0x39 FIRE, fire found and ammo>0 | Request 3; Push 0x35 then 0x33 |
| 0x39 FIRE, no fire and ammo>0 | Pop current goal; request 3; Push 0x32, then 0x35, then 0x33 |
| 0x39 FIRE, ammo≤0, whether or not a fire was found | Request 4; Push 0x32 |
| Unrecognized goal | Pop, then execute goal-0 handling in the same visit |

The FIRE query uses the saved search anchor +0x338 and destination output
+0x4C (§3.3.2). It does not request a drop immediately on finding a fire;
the later LOITER visit activates the drop. Evidence:
`0x00504590–0x00504977`. Push order matters: the last pushed goal is active.

| Helicopter goal/condition | Ordered owner actions |
|---|---|
| 0x32 LAND, rotor phase=0 | Request dispatch 0, Pop, then perform goal-0 handling |
| 0x32 LAND, rotor phase≠0 | Request 6, then perform goal-0 handling |
| 0, phase=0 | Reset controls, disable spotlight, return |
| 0, phase≠0 | Invoke engine-enable helper; disable spotlight; steer home; set +0x25C when full 3D home distance <24 |
| 0x33 destination | Request 3; enable engine; disable spotlight; move toward X/Z destination while outside the completion gate; Pop when inside; building-lift control runs in either case |
| 0x34 CHASE, missing target or target current goal≠0x10 | Pop; return, without inventing another dispatch state |
| 0x34 valid target | Engine/spotlight on, then §4.2.2 chase rules |
| 0x35 LOITER | Request 5; engine/spotlight on; aim at ground height at destination; if 0x36 exists in stack, RespondToCrimeScene; stop horizontal motion; building-lift control |
| Unrecognized goal | Pop, then execute LAND/goal-0 handling in this visit |

On home steering, horizontal distance ≤8 selects the horizontal-stop helper;
otherwise MoveXZ receives the displacement and scalar 10. Horizontal distance
≤48 selects hover demand `home.y + horizontalDistance − 2`; greater distance
selects building lift. The <24 completion test uses **3D** distance, not the
horizontal distance. Evidence: `0x0053C441–0x0053C5AA`; the remaining claim/
crime/rescue branches at `0x0053BE73–0x0053C43C` need the observation in §9.

Helicopter `SetDispatchState(s)` is a no-op for repeated state. On a change,
if goal 0x36 exists and city service +0xBC exists, s=5 requests the owner's
`riot_police_repulsor` object through +0x14(name,0), then passes the result
to that service's +0x14. Every other s passes owner +0x14() to service +0x38.
Only after this service work does it store s at +0x274 and notify owner
+0x198(7,s). It does not call the emergency-vehicle base setter. Missing goal
or service skips only the repulsor action; store/notification still occur.
Evidence: `0x0053A360–0x0053A40A`. Entry/exit callbacks are in §4.3.3.

Worked cases: FIRE with an empty query and ammo 1 produces Pop, dispatch 3,
Push LAND, Push LOITER, Push destination; with ammo 0 it instead requests 4
and pushes LAND without the Pop/LOITER/destination sequence. LAND already
at dispatch 4 does not request 6; successful landing requests 0 before Pop.
A helicopter changing 3→5 while a 0x36 state exists makes the repulsor
request before notifying (7,5); repeating 5 makes neither request. A
helicopter already at phase 0 on LAND requests dispatch 0 before Pop.

### 4.5 Native observation boundary (AF11) *[compat, bounded]*

The class ID `0x8BA98EDA` identifies a helicopter class; it is not evidence
of an interface exposing rotor getters. The verified aircraft QI operation
is primary slot 0: Windows `__thiscall`, arguments `(u32 iid, void** out)`,
boolean result, callee pops 8 bytes. IID `0xC96E75DC` returns the aircraft
primary pointer and calls its slot 1 AddRef; every other IID delegates to
automaton QI. Balance each successful QI reference by slot 2 Release.
Evidence: `0x004FA7A0–0x004FA7C3`. Automaton `0xA9B40F05` and serializable
`0xE4FDA3D4` are the inherited contracts, not a thirteen-field rotor API.

The native parameter storage can be identified **inside the lifetime of an
initialized rotor**, using R and P from §4.2.6. The P-relative binary32 field
offsets, in the thirteen-field order of §4.1, are
`5C,60,64,68,6C,70,0C,24,14,1C,20,74,78` (hexadecimal).
This is data layout, not a callable signature or an independent ownership
reference. Do not retain P across Shutdown; InitInstance owns it (§4.2.5).
The named Lua values are input records **before** conversion/zero preservation;
capturing only these converted fields cannot recover which ancestor/table
or zero-valued source produced them. The home triple is owner +0x1F0/+0x1F4/
+0x1F8 and phase is R+0x04. Neither is a published getter in this evidence.

A complete goal observation must include stack order, each state's type and
all type-specific fields from automata.md §6.2, not just the current ID.
Controller references, timeout, final flag, animation/SFX and saved flags
must retain their stream meaning. In-memory +0x24 behavior-template pointers
are not persisted; do not compare native pointer addresses with replacement
identities. Current-ID +0xEC and FindState +0xE4 calls used by AI are not
complete-stack enumeration APIs.

The aircraft manager is city slot 45, IID `0x098F9BF4`, CLSID `0xC98F9D4A`.
Its slot 15 (+0x3C) creation contract is the seven-argument `__thiscall`
contract in automata.md §7.2: group ID, position reference, flags, model-key
reference, callback pointer, byte priority, optional orientation pointer;
returns an automaton pointer in EAX, callee pops 28 bytes. The returned pointer
has **no caller reference**. AddRef before retaining it. Passing a zero model
instance invokes random exemplar choice; it is not a deterministic fixture.

There is no established complete public live-helicopter enumeration in this
answer. Manager Write's airport-assignment map (automata.md §5.2) cannot stand
in for its live population. The documented group-tracking operations at manager
+0x7C/+0x80/+0x84 do not by themselves prove that every live group is tracked,
or establish an all-groups iterator. Do not guess their container ABI.
AF11 is narrowed to the lifetime/enumeration/fixture observation in §9.6.
No safe deterministic group/exemplar **values** or complete getter signatures
are claimed here.

Worked case: a city has one airport-assigned helicopter and a second live
helicopter made by another group/source. An archive scan returning only the
first is incomplete even if its four record versions match. A QI-acquired
aircraft pointer has one observer-owned reference; reading its parameter block
does not confer a second reference on that block. A fresh COM object's preset
is a constructor observation and cannot supply the city's expected home/goals.

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

AF2–AF6, AF10 and AF11 have static answers or explicit residual native
observations in §9; their entries are in
[questions/answered/aircraft.md](questions/answered/aircraft.md). AF10's
comparison contract is specified in §3.4.1a. Archiving the other entries means
the question has been narrowed, **not** that the complete owner or native
shadow is verified. Summary paragraphs and pending observations confer no
full-flight, full-AI or full-shadow conformance credit.

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
28. Apply the §3.4.1a non-finite limiter/time cases separately from the full
    flight tick. Compare finite components and NaN/infinity classification;
    do not require a particular NaN payload or assert a finite final position.
29. Clearance: provide twenty samples, original Y=10, only sample 2 at height
    26, the rest ≤10. Expect score 0.5. Make sample 2 accepted with auxiliary
    height 26 and GetAltitude 100: still 0.5. Reject it: score 90/32=2.8125.
30. Selection: weights [1,3], draw exactly 1 choose index 1. All-zero weights
    use the uniform integer draw. An odd endpoint count invokes owner +0x34(0)
    without supplying new outputs. Test both values of the final direction flag.
31. Use §4.4's FIRE no-fire/ammo and LAND dispatch cases, recording ordered
    Push/Pop/dispatch actions, not just the final stack top.
32. Use §4.2.6's engine-flag 1/2 cases and home-completion snap/blend cases.
    Check that home completion skips the four-step rotor frame that visit.
33. Use §3.2.1's false-base airplane teardown and §4.2.7's duplicate-claim
    helicopter teardown cases. Their base-call positions differ.
34. Use §4.3.3's zero/absent airplane player-record refresh. Zero preserves a
    gain until the AI reload callback; it is not a fresh player default.
35. Use §4.5's two-helicopter enumeration counterexample. Until §9.6 is
    observed, report the archive scan's actual subset, not a complete population.

## 9. Residual native observations (AF2–AF6, AF11)

These are the exact remaining evidence requests, rather than permission to
fill gaps with conventional flight equations or guessed interfaces. No city
runtime trace was taken in this round. The static pass establishes the rules
above; it does not establish the remaining cross-service contracts below.
An RE-side trace may record native state and service traffic; implementers
receive only the resulting prose, numeric fixtures and input manifest.

For each capture record executable/version, city/resource manifest, object
class and source group, initial goal history, original milliseconds and the
converted seconds, selected parameter records, and the x87 control word.
Capture inputs before each named stage and outputs immediately after it;
an end-of-frame position alone cannot distinguish the stages. A finite set
of captures settles those cases, not every continuous control input.

### 9.1 AF2: attitude, motion and environment corrections

The unresolved fixed-wing boundary is lift-frame construction and angular
update in `CalculateForces` (`0x004FE850`) / attitude helper `0x004FE050`,
followed by environment corrections in `UpdatePhysics` (`0x00503CB0`) and
`DetectCollisions` (`0x00501C30`). Observe old orientation and complete basis,
roll/pitch/yaw/control fields, force vector, limited velocity, candidate
velocity/position, each terrain/runway/collision query and response, corrected
motion, and each phase request. Use level, banked and pitched flight; zero
speed; phase 0/1 ground roll; airborne phases; and contact just below, at and
above each returned surface. Include both obstacle-hit and no-hit results
with the same incoming motion. This distinguishes a lift-axis rotation from
a coefficient change and distinguishes projection from a positional clamp.

For the rotor, capture R, C and the ten transforms before/after the four
§4.2.6 stages, especially `YawPitchRollSlideReal` (`0x00538860`),
`MoveXZReal` (`0x00539000`) and `MoveYReal` (`0x005348C0`), then collision
correction (`0x0053AC50`). Required inputs: one control at a time at zero,
each sign, each limit and just outside; engine off/spinning/full; phase
0/1/5/6; climb/descent; terrain, water and building contact independently.
Record the actual transforms and height-service results: their in-memory
basis convention cannot be recovered from the four final owner fields alone.
The ±890/250 overview remains a summary, not a complete angular law.

Worked discrimination: give identical level-flight velocity and controls to
two runs, changing only bank sign. Compare force immediately after
CalculateForces, before collision/ground projection. A later ground-aligned
velocity is insufficient evidence for either lift direction. For rotor
motion use h=0 and h=0.1 with the same controls; distinguish immediate control
clamping/phase transitions from integrated displacement.

### 9.2 AF3: continuous steering and remaining goal branches

Runway scoring, approach placement, search geometry, LOITER, chase gates,
reset/hover/building lift, and the §4.4 goal cases have static rules.
Still required: control outputs and completion predicates of airplane
ApproachAltitude (`0x004FD8A0`), Stop (`0x004FDCF0`), Turn (`0x005004A0`),
FollowRay (`0x00500680`), Taxi (`0x00500980`), FollowVectorAndTurn
(`0x00500D50`), TakeOff (`0x00502850`), GotoDestination (`0x00502BC0`) and
Land (`0x00503030`); helicopter MoveXZ (`0x005368B0`), and goal 0x36/0x37/
0x38 claim/crime/rescue branches (`0x0053BE73–0x0053C43C`).

Observe the complete goal stack before/after, requested target/ray, signed
heading/cross-track/height errors, speed, control demands and helper return
values. Sweep both signs and equality at each reached branch threshold;
record branches actually reached, not a guessed degree/radian threshold.
For helicopter claims supply empty, matching and duplicate-claim lists and
record city/crime/rescue provider success and failure, selected target and
claim insertion/removal before Push/Pop. For airplane landing/takeoff include
empty/odd runway lists, zero-length pairs, both runway flags, and each
approach leg transition. Do not turn §3.3.2's owner-removal request into an
invented successful runway selection.

Worked discrimination: hold a taxi target fixed and use equal positive and
negative heading errors at equal speed, then repeat at zero speed. Capture
the actual rudder/throttle commands and completion boolean separately.
A destination reached after several frames cannot identify that control law.

### 9.3 AF4: lifecycle attachments and phase-trigger causes

Constructor defaults, calibration identity, phase-setter sound actions,
allocation ownership and both teardown orders are specified above. Remaining
evidence is the helicopter's other constructor fields; full effect attachment
creation; all Init/PhysicsInit substep order beyond §4.1.1; and collision/
physics causes of phase changes. The Init pseudocode exports fail for both
owners; this answer does not infer their missing instructions from the
successful Read/Shutdown paths.

Capture constructor → Read (separate loaded/unloaded runs) → Init entry →
base Init → model/socket and named-effect creation → InitInstance/parameter
overlays → PhysicsInit → Init return. Record every allocation, retained
reference, effect name/socket, model key and return boolean. Repeat with
base Init false, each effect creation returning null, and a repeated Init.
Observe ordinary ground departure, airborne arrival and collision, logging
phase **requests** and ordered AEMS calls even when the requested phase equals
the current phase. This separates request causes from setter side effects.

Worked discrimination: fail only the spotlight's second effect allocation.
Record whether Init succeeds, whether the first effect remains owned and
whether Shutdown stops/releases it. The §4.3.2 both-present update gate alone
cannot settle partial-attachment cleanup or Init's result.

### 9.4 AF5: control translation, cadence and spotlight geometry

Parameter refresh/scheduling and weapon success accounting are specified in
§§4.3–4.3.3. Remaining boundaries are airplane translator `0x00501080`,
helicopter translator `0x005399D0`, base command readiness, named-effect
construction/projectile initialization and spotlight geometry after its
activation gate (`0x00537C46–0x0053832B`).

Capture each raw command code and analog magnitude, original/converted time,
old/new controls, readiness byte, repeat timer, use count, active parameter
record, named-effect request and success. Use idle, single axis ±1, reversal,
simultaneous axes, hold/release/repress, zero time and a long capped frame.
For weapons record the attached resource/model, position/orientation,
velocity, target, ownership handoff and start/stop call sequence; a successful
string lookup does not prove projectile setup. No fixed cooldown follows from
the owner success counter alone.

For spotlight capture both complete output transforms and terrain-query
arguments/results, with target absent/stationary/moving, player vs AI,
environment 1 vs another value, sloped terrain and changing helicopter yaw/
height. Include an inactive pair: geometry still runs (§4.3.2). Capture prior
aim/target fields as well as current inputs to reveal smoothing/history.

Worked discrimination: hold weapon command across frames at times 0, 1, 2
and around each observed readiness transition; record failures followed by
success without changing input. Separately move a spotlight target laterally
between two equal-time frames. Compare the requested transform before terrain
projection and the final terrain-facing transform; their difference identifies
projection rather than target smoothing.

### 9.5 AF6: message envelope and external failure callbacks

The exact owner dispatch requests, repeated-state gate, repulsor order and
entry/exit parameter callbacks are in §§4.4/4.3.3. A complete aircraft-owned
message-ID/subscription/payload table is not established here. Do not identify
owner notification arguments `(7,s)` with a particular framework message ID
or invent an aircraft DoMessage override from this call alone.

Observe the receiver invoked by owner +0x198(7,s), including full envelope,
sender/target references, delivery timing and nested callbacks, for 3→5,
5→6, 6→0, repeated 5 and missing listener. Also observe runway failure,
home-occupant removal, fire query failure, chase target removal and a lost
crime/rescue claim during pursuit. Capture goal/dispatch transitions on both
sides of the external callback and reference releases. These observations
settle whether an external failure removes, retries, lands or merely returns;
the normal-success dispatch table cannot answer that by itself.

Worked discrimination: on helicopter 3→5 with 0x36 in the stack, log the
repulsor request, stored dispatch value seen inside the notification receiver,
and any nested goal changes. Repeating 5 must produce no owner notification;
any observed message then belongs to another source and needs its own cause.

### 9.6 AF11: complete population and independent fixture history

Required native observation: an RE-side lifetime log beginning before city
aircraft initialization, recording **every** successful manager registration
and removal, class identity and stable observer ID. At a paused frame boundary,
reconcile that live set against manager count and the archive's emitted set;
list explicitly which helicopters Write omits and why. City load, ordinary
spawning, dispatch and player takeover must all be included. A late-start log
or a union of selected tracked groups cannot establish completeness.

For one successfully initialized city helicopter, record its actual script
group definition, parent chain, named `helicopter` and `playerdrive` inputs,
explicit exemplar/model key and creation flags/priority. Repeat creation using
that **recorded** manifest and the §4.5 signature in the same city/resource
state; obtain identical constructor/Init inputs before accepting it as a
deterministic scenario. No concrete group ID, exemplar or scenario is supplied
by this round. Record manager registration, ownership, successful QI refs,
normal Remove/Shutdown, manager removal and final Release. Until that trace
establishes cleanup order, a private uninitialized COM object remains only a
metadata fixture and must not be treated as a spawned helicopter.

Expected home must come from the independently recorded initial placement or
the **input** serialized home triple, plus the Read/Init ordering. Expected
goals must come from the input serialized stack and ordered external commands,
controller actions, elapsed times, service responses and random draws through
each transition. Capture those inputs when they occur; reading the resulting
native home or stack and copying it into the expected owner is circular.
Capture inherited goal records using the documented serialized formats,
including controller identities and version-dependent fields (§4.5), rather
than assuming a fixed-size raw object copy.

Worked discrimination: a post-Read helicopter with saved home (10,20,30) and
an already-active goal must be reconstructed from that input record and its
subsequent commands. A fresh owner at zero/home phase 0 seeded with the
native output stack does not test Read, goal progression or independent
reconstruction, even if every final field agrees.
