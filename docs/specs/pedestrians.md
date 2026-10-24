# Spec: pedestrians (`cSC4Pedestrian`, `cSC4PedestrianOneShot`), fire crews and the traffic effect manager

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[summary]* is a correct outline whose details are not given.

Related specs: [automata.md](automata.md) §5.1 (the pedestrian manager), §6.4 (the pedestrian record), §6.5 (the
per-state behaviour table: `BEE_LINE`, `CROWD`, `QUEUE`, `STRIKE`, `RIOT_LEADER`, `RIOT_FOLLOWER`,
`IGNITE_OCCUPANT`), §11 (slot names); [vehicles.md](vehicles.md) §2 (the traffic automaton) and §7.2 (the fire
truck, which creates fire crews); [disasters.md](disasters.md) §5.11 (the fire occupant) and §6.5–§6.6 (riots);
[automata-support.md](automata-support.md) (the automata tuning values `T+offset` used below);
[traffic-simulator.md](traffic-simulator.md) (accidents).

## 0. Summary

Pedestrians are traffic automata that walk. Their per-state behaviour is in automata.md §6.5; this spec adds what
the pedestrian class itself does around it: its speed, its flags per state, its collision push, its crowd
reaction, the one-shot variant that plays an animation a few times, and the fire crew, a pedestrian that a fire
truck puts on the scene to hose a fire. It also covers the small manager that attaches the accident controller to
road occupants.

## 1. Classes *[compat]*

| Class | Class ID | Record |
|---|---|---|
| `cSC4Pedestrian` | `0x896E75AF` (`0x00546030`) | automata.md §6.4 (`u16` 5, Write `0x00544650`, Read `0x005446C0`) |
| `cSC4PedestrianOneShot` | `0xAA09DC28` (`0x005468C0`) | the pedestrian's; its own `Init` and animation tick are in §3 |
| `cSC4SimFireCrew` | `0x4A222E4D` (`0x0054A430`) | §4.5 |
| `cSC4TrafficEffectManager` | — | none (§5) |

## 2. The pedestrian *[compat]*

### 2.1 Construction and Init

The constructor (`0x00545F20`) builds a traffic automaton with occupant type `0x496E5783`, clears the pending
push (§2.4) and its collision timer, and sets the **default speed**: a uniform random integer between `T+0x18` and
`T+0x1C` (tuning `0x4998C905`, shipped 15 and 20) × 1/3600, which also becomes the speed tween's target.

`Init` (`0x005444F0`) initializes a per-object random parameter before the traffic automaton's `Init`.
The initialization guard is the initialized query on the interface at `this+4` (slot 5,
`0x005444F9`), rather than a separate pedestrian flag. When that query is false, automaton slot 158
(`+0x278`) performs the following operation:

- Take the two binary32 bounds `T+0x120` and `T+0x124` (property `0x29DD40C1`; shipped −0.5, −0.5).
- Promote both bounds to double arguments and make one `RandomDoubleRangeUniform` call on the automata
  RNG (`0x00B21D78`). Store the result rounded to binary32 at object `+0x200`.

The concrete pedestrian's table `0x00A9A718`, one-shot table `0x00A9AB50`, and fire-crew table
`0x00A9B488` all inherit this operation at `0x00544620`. It is **not** the constructor's integer
default-speed draw or the one-shot play-count draw. The field's semantic name is not established;
its storage and random-value contract do not require guessing one. Equal bounds still consume the
range draw; with shipped bounds the stored value is exactly −0.5. An already initialized object
skips this operation. If the subsequent base Init fails, there is no rollback of this stored value;
a retry performs it again if the initialized query is still false.

Evidence: table entries `0x00A9A990`, `0x00A9ADC8`, `0x00A9B700`; floating-point loads, double
arguments and binary32 store `0x00544620`–`0x00544643`; range draw `0x00913A46`–`0x00913A55`.

### 2.2 The animation tick (`0x00544580`)

It reads flag `0x20` before the tick, runs the traffic automaton's tick (vehicles.md §2), and when the automaton
moves (slot 86) compares the flag before and after: newly set → slot 176; set before → slot 19 (`PlayAnim`) with
0 (the walk animation again *[inferred]*). It returns the traffic tick's result.

### 2.3 State flags (`HandleEnterState`, `0x00546A10`)

When the traffic automaton's enter hook does not handle the state:

| Entered state | Flags (on / off) | Then |
|---|---|---|
| 1 `DISAPPEAR` | `0x2000`, `0x4000`, `0x20000` on | |
| 2 `BEE_LINE`, 3 `CROWD` | `0x2000` on, `0x4000` off | |
| 4 `QUEUE` | 2, `0x2000` on, `0x4000` off | |
| 6 `STRIKE` | 2 on, `0x2000` off | re-path (slot 134) |
| 7 `IDLE` | none | |
| 8 `RIOT_LEADER` | the riot animation `0x3AB60B61` (`0x004FDF90`); 2, `0x2000`, `0x10000`, `0x1000000`, `0x2000000`, `0x10000000` on, `0x4000` off | the first leader step (`DoRiotLeaderTick` with "start") |
| 9 `RIOT_FOLLOWER` | the riot animation; 2, `0x2000` on, `0x4000` off | |
| any other | `0x4000` off, `0x2000` off | re-path (slot 134) |
| none (empty stack) | `0x2000` off, `0x4000` off | |

`DoStateTick` (`0x00546BC0`) does nothing with an empty state stack; otherwise the traffic automaton's state tick
first, and when that does not handle it the pedestrian ticks of automata.md §6.5 by state: 2 `DoBeeLineTick`,
3 `DoCrowdTick`, 4 `DoQueueTick`, 6 `DoStrikeTick`, 8 `DoRiotLeaderTick`, 9 `DoRiotFollowerTick`,
12 `DoIgniteTick`.

### 2.4 Walking with collisions

`MoveForwardWithCollision(dir, dt)` (`0x00545D60`): the step is the speed modifier (slot 125) × the current speed
(slot 34) × dt along the normalised direction. Unless flag `0x20000` is set, the new position is put on the ground
by the traffic network map (city slot 40, slot 44, flags `0xC49`). When the move collides (below) and a push is
pending and non-zero, the push is added, the position put on the ground again and tested again; a move that still
collides is not made, otherwise the pedestrian moves there (slot 16). The pending push is then cleared. The result
is "moved".

`TestCollisionExpensive` (`0x00545840`) runs at most once per 500 ms of accumulated time (between tests it reports
no collision). It asks the occupant manager for the automata (occupant type `0x496E5783`) in the box from
(x − 1, y, z − 1) to (x + 1, y + 2, z + 1) around the new position. Each other automaton nearer than 1.0 gets a
pending push away from this one (the vector from this position to it), or a random push with x and z uniform in
[−0.5, 0.5] and y 0 when it is nearer than 0.01. Any such hit is a collision.

`DoBeeLineTick` (`0x00546150`) walks along the stored bee-line heading when it is not zero, then the behaviour
refresh (slot 105); it always returns 1.

### 2.5 Crowd reactions (`HandleCrowdReactionEvent`, `0x005448D0`)

Ignored (false) while a flag (slot 31) is set. Otherwise the reaction kind (`+0x254`) becomes 1 when the event's
mode is non-zero and 2 when it is 0, the speed tweens back to the default speed (slot 36), and the pedestrian turns
toward the event (slot 16) *[summary of the turn]*; the result is true. `DoCrowdTick` plays the reactions
(automata.md §6.5).

### 2.6 The other ticks *[summary]*

- `DoQueueTick` (`0x00545690`): with a controller, the controller's queue step (controller slot 28, automata.md
  §3.3) and a straight-line move; with flag `0x20` it faces the queue head and plays the animation `idle`
  (slot 21); then slot 105. Without a controller it pops the state.
- `DoStrikeTick` (`0x00544950`): with a controller target, re-paths toward a box around it that shrinks by one cell
  on each side; once inside, or when no path is found, it returns to its source occupant (slot 29).
- `DoRiotLeaderTick` (`0x00546220`) and `DoRiotFollowerTick` (`0x00546530`): disasters.md §6.5 and automata.md
  §6.5; both walk with `MoveForwardWithCollision` and random turns, the follower toward its leader (IID
  `0xA9B40F05`) past 10 units.
- `DoIgniteTick` (`0x00545280`): the controller of the newest state 12 in the stack gives the target; the fire
  disaster (`0x621CDA33`, interface `0x49B43F57`) ignites it with cause 2 (arson); then `SetState(1)`
  (disasters.md §6.6).

### 2.7 Base code reached through the pedestrian

The automaton and traffic-automaton functions that the pedestrian's tables reach first (the small getters, the
state-record writer `0x0050EF50` of automata.md §6.2, the traffic `SetDestination` `0x0054E5B0`, the path tick
`0x0054FDC0` of slot 129, and the traffic slots 131–134 at `0x0054E920`, `0x00550760`, `0x005504E0`,
`0x00550B60` with their helpers `0x00550F40`, `0x005510D0`, `0x00551300`) are base-class code: their behaviour is
vehicles.md §2 and automata.md §6.5 *[summary]*.

## 3. The one-shot pedestrian (`cSC4PedestrianOneShot::Init`, `0x00544DA0`) *[compat]*

### 3.1 Initialization and count selection

The constructor (`0x00546860`) uses pedestrian construction and initializes the **remaining play count**,
an unsigned byte at `+0x270`, to 5 (`0x005468B1`). Init uses the initialized query and slot-158 operation
of §2.1, then calls the traffic automaton's Init directly. It returns that call's Boolean result.
Failure leaves the existing count and model speeds unchanged by the one-shot-specific setup; the
earlier slot-158 operation, if performed, is retained.

Every **successful** Init, including a call on an already initialized object, sets all non-null model
instances' animation speed to binary32 `0x3C23D70A` (approximately 0.01 frames/ms, or 10 frames/s), through
instance slot 14, `SetSpeed`. Null entries are skipped. This does not select an animation, reset its
frame/accumulator, or change its playback mode. The model playback contract is
[model-instances.md](model-instances.md) §3 and [graphics-formats.md](graphics-formats.md) §3.9.

After setting the speeds, select a new count, replacing any remaining count:

| Exemplar input | Bounds used |
|---|---|
| No exemplar, no property `0xEA958845`, or fewer than two array elements | The two tuning bytes `T+0x13C`, `T+0x13D` (property `0x6A09F630`; shipped 3, 10) |
| Property `0xEA958845` has at least two `u8` elements | Element 0 is the lower bound, element 1 the upper; ignore later elements |

For ordered bounds `lo ≤ hi`, the count uses the inclusive bounded-integer RNG contract: draw with
span `hi − lo + 1` and add `lo`. This is one bounded draw even when both bounds are equal, even when
there are no models, and on every successful reinitialization. The slot-158 draw precedes base Init;
this separate integer draw follows successful base Init and all model-speed writes.

There is no validation or swapping of reversed bounds. More precisely, let `N` be `hi − lo + 1`
reduced modulo 2³², and let `R` be the next unsigned 32-bit automata random word. The selected count is
`(lo + floor(R × N / 2³²)) modulo 256`, using a full 64-bit unsigned product. In particular, adjacent
reversed bounds give `N = 0`, still consume a random word, and store `lo`. Both zero and 255 are
representable counts; zero does not request removal during Init.

Evidence: guard/base result `0x00544DA5`–`0x00544DCB`; model-speed writes `0x00544DD1`–`0x00544DFE`;
property and length check `0x00544E00`–`0x00544E55`; byte bounds, 32-bit span, bounded draw and byte
store `0x00544E59`–`0x00544E70`. The bounded RNG uses the high half of an unsigned product
(`0x00913A06`–`0x00913A11`).

### 3.2 The play-count consumer: the one-shot animation tick

The primary table installs `0x00544E80` in automaton **slot 72** (`+0x120`, entry `0x00A9AC70`).
This is the one-shot's own `DoAnimationTick`, not the pedestrian animation tick of §2.2 or its
animation-step slot 103. It consumes the count **directly** from `+0x270`; calling a count getter,
an animation-end callback, or a pedestrian state handler is not required to consume it.

The tick accepts the ordinary two inputs (microseconds, milliseconds). It ignores the first and
uses the second, binary32 milliseconds, for the inherited opacity/fade update (automaton slot 83,
`0x005091C0`) and for `Advance(ms)` (instance slot 15) on each non-null model in list order. The fade
update happens first. Each model's Advance result is ignored.

Only the **first non-null** model determines whether a play is charged. Immediately after advancing
that model, compare its current frame (instance slot 11) with its frame count (slot 12) minus one.
The comparison is signed 32-bit `frame ≥ frameCount − 1`; it is a sampled final-frame test, not an
edge test or a check of Advance's return value. A later model cannot charge a play, even when the
first model is below its final frame. Empty/all-null lists never charge a play.

| First non-null model after Advance | Count update and tick result |
|---|---|
| Frame below `frameCount − 1` | Count unchanged; return true |
| At/past final frame, count greater than zero | Decrease the byte count by exactly one; return true if it remains nonzero |
| At/past final frame, count is now zero (including zero before the test) | Keep zero, call automaton slot 13 `Remove(false)`, and return false |
| No non-null model | Count unchanged; return true |

The removal request uses the ordinary automaton removal operation (`0x00509130`), not immediate
destruction or a new one-shot state. Remaining non-null models are still advanced in order after
that request; none of them causes another decrement or removal request in that tick. The tick
does not itself restart an animation, select a mode, or reset a frame. Thus the usual looping
model provides subsequent plays, but a held final frame charges again on each tick, including a
zero-millisecond tick. A large delta can charge at most one play per call. A zero count stays
alive until a qualifying final-frame observation; it is not an infinite-play sentinel.

Unlike the ordinary pedestrian/traffic tick, this body has no simulator-pause query, stationary
flag gate, walking/state tick, animation-end callback, or animation-clock increment. These are
properties of a direct call; whether a paused scheduler delivers a call is outside this body.

Evidence: fade input `0x00544E86`–`0x00544E95`; model Advance and first-model selection
`0x00544E9B`–`0x00544EC5`; frame/count comparison and saturating decrement
`0x00544EC7`–`0x00544EF6`; `Remove(false)`, continued iteration and Boolean return
`0x00544EF8`–`0x00544F20`. The instructions establish both the argument positions and the return
value; the exported pseudocode misidentifies a count-field index and leaves the result undefined.

### 3.3 Count accessors

The same primary table has two additional slots: 177 (`+0x2C4`, `0x005468D0`) reads the unsigned
byte count, and 178 (`+0x2C8`, `0x005468E0`) replaces it with its byte argument. Neither advances or
restarts playback, changes speed, or requests removal. Their table entries are `0x00A9AE14` and
`0x00A9AE18`. The tick in §3.2 reads/writes the field itself rather than dispatching these accessors.

## 4. The fire crew (`cSC4SimFireCrew`) *[compat]*

### 4.1 Creation and model

The fire truck creates the crew and gives it a **competence** (`SetCompetence`, `0x00546CB0`; vehicles.md §7.2).
`Init` (`0x00549890`), once:

1. When the truck's dispatch state (truck slot 26) is 5 (on the scene), the **competence level** is the first i
   in 0, 1, 2 with `T+0x108`[i] ≤ competence (tuning `0xA9D6CC18`, shipped 1.0, 0.6, 0.0); without a match it is 3
   *[quirk: only 0–2 index the tables]*. Level 2 sends crowd message `0xA9E3B6E2` with (−1, `0x40`). The crew gets
   two models (type `0x5AD0E817`, group `0xBADB57F1`): instance `0x21010000`, `0x21010000`, `0x21030000` and instance
   `0x21020000`, `0x21020000`, `0x21040000` for levels 0, 1, 2.
2. Otherwise one model at random of `0x21050000`, `0x21060000`, `0x21070000` (the walking crew).
3. Flags `0x2000` and `0x4000` on, the pedestrian's `Init`, the hose registration points (`InitRegTranslationArray`,
   graphics-formats.md "Consumer 2"), and a registration for message `0x99EF1143`.

### 4.2 Setup on the scene (`SetupFireCrew`, `0x00547EF0`) *[summary]*

With the truck on the scene, the crew is placed beside the truck: along the truck's direction from truck slot 143
(its heading when that is zero) when the truck is not player-driven, or on a random side (a fixed turn left or
right of the heading) when it is; at the ground height, with its start and target positions recorded. It then sends
message `0xAA0F3F75` to the truck with the level's value (`0x29DE8FD6`, `0xC9DE8EE4`, `0xC9DE8EE4` for levels 0, 1,
2). Off the scene it is placed from the truck's transform. Both end with a fade-in: the opacity is set to 0 and
tweened to 1.0 over `T+0xBC` (slot 33).

### 4.3 The animation tick (`0x00549A50`)

1. The pedestrian's tick.
2. When the crew is not marked for removal and the truck is player-driven (state 16 in its stack) or on the scene:
   - the target is the truck's stream target point (`cSC4FireTruck::GetStreamTargetPoint`) or, without one, the
     nearest burning occupant within `T+0x114` (tuning `0xC9EE640C`, shipped 128; `FindTargetsInRadius`,
     `0x00549000`, which also takes the fire occupant `0x69D25C60` as the crew's fire);
   - **no target**: the hose effects stop; once, after at least one fire was handled, the crew sends crowd message
     `0xA9E3B6E2` with (1, `0x50`); then it goes back to the truck (truck slot 199) or, without a truck, is
     removed;
   - **a target**: the crew turns toward it, creates the two hose effects from the effect manager when it has none,
     and aims them at the target; at level 2 the aim wanders by a uniform ±75 on x and z, smoothed by a factor of
     0.002 × the frame time; the first effect restarts when it stops; the second starts once the hose has run for
     0.8 s (time += frame time × 0.001).
3. Otherwise it sends message `0x8A0F3F7B` to the truck with the level's value and stops the hose effects.
4. When the truck's dispatch state is 1 and the crew's first model has reached its last frame, that model is
   replaced by a random walking model (step 4.1.2).

### 4.4 Hosing and messages

The crew counts on its fire: assigning a fire adds the crew's strength, 2 (1 at level 2), to the fire occupant's
hosed count (fire slot 12 `AddHosed`, disasters.md §5.11) and to the target occupant (slot 5 of interface
`0xABF98CB3` *[inferred: the same count on the burning building]*); releasing the fire takes it away. The fire is
released when the crew shuts down (`0x00547D90`, which also sends crowd message `0xA9E3B6E2`, stops and releases
both hose effects and drops the message registration) and on message `0x99EF1143` naming the fire or the target
(`0x00548DC0`), which also stops both hose effects (with the arguments 1 and 0). `SendFireCrewMessage`
(`0x00546D20`) posts a `cRZMessage2Standard` of the given type with the given data to the message server.

### 4.5 Record (Write `0x00547BD0`, Read `0x00547C80`)

`u16` 2 (Read requires 2), the pedestrian record, `ref` the truck (IID `0xA9B40F05`), `f32` competence, `u8`
level, `s32`, `bool`, `vec3`, `f32`, `vec3` *[as read: the hose and position state]*.

## 5. The traffic effect manager (`cSC4TrafficEffectManager`) *[compat]*

It keeps a list of effects on road occupants.

- `CreateAccident(occupant, …, duration)` (`0x00554040`): appends an entry with the duration, the centre cell of
  the occupant's cell rectangle and the occupant, starts it (`0x0054EC60`), attaches the automata controller
  **`traffic_accident_2`** to the occupant (city slot 47, controller manager slot 10; the result is kept as
  "attached") and keeps a reference to the occupant.
- `StopEffect(entry)` (`0x00552B80`): when the entry is in the list, an attached entry removes the occupant's
  controllers (controller manager slot 14, `0x0054ECC0`), the entry is unlinked and its occupant released.

The traffic automaton's exit hook stops its effects (vehicles.md §2).

## 6. Quirks

1. A competence below every threshold (possible only when a tuning list does not end with 0) gives level 3, which
   reads past the three-entry model tables.
2. `TestCollisionExpensive` reports no collision between its 500 ms tests, so most steps never collide.

## 7. Open questions

See [questions/pedestrians.md](questions/pedestrians.md) (PN2–PN3). PN1 is settled by §2.1 and §3;
the initialization hook and play-count consumer have no remaining behavior question.

## 8. Conformance tests

1. With tuning `0x4998C905` = (15, 20), a new pedestrian's default speed is one of 15/3600 … 20/3600.
2. A fire crew of competence 0.7 with the shipped thresholds (1.0, 0.6, 0.0) has level 1 and models `0x21010000` and
   `0x21020000`; one of competence 0.5 has level 2, models `0x21030000` and `0x21040000`, and hoses with strength 1.
3. Entering state 4 sets flags 2 and `0x2000` and clears `0x4000`; entering state 7 changes no flag.
4. Two pedestrians 0.005 apart: the first one's test gives the second a push with |x|, |z| ≤ 0.5 and y = 0.

### PR review conformance: initial random parameter

For concrete `cSC4Pedestrian` with equal tuning bounds -0.5 (the shipped values of property `0x29DD40C1`), first initialization performs one double-range draw and stores f32 -0.5 at+0x200 before the base Init. An already initialized object skips this draw. For unequal bounds, use the existing RNG contract; do not substitute the default-speed integer draw from section2.1. Overrides of slot158 retain their own behavior. The identity is byte-proved by the constructor table store `0x00545F4A`, table entry+0x278, and body `0x00544620`-`0x0054464A`.

The primary tables of `cSC4PedestrianOneShot` (`0x00A9AB50`, first entry `0x00544780`) and `cSC4SimFireCrew` (`0x00A9B488`, first entry `0x00547920`) also map slot158 to `0x00544620`, so the same initial-parameter operation applies to these two inherited calls.

### PN1 conformance: initialization and count consumption

The initialization fixtures control the initialized query and base-Init result. RNG words below are
inputs to the bounded count draw, not to the earlier double-range draw. Observe only the draws
owned by this class; base Init may have its own effects. Tuning bounds for the double draw are
−0.5, −0.5 unless specified otherwise.

| ID | Fixture/action | Required observation |
|---|---|---|
| PN1-I1 | Newly constructed one-shot, before Init | Count 5; no one-shot model-speed write yet |
| PN1-I2 | Not initialized, successful Init, models `[null, A, B]`, shipped count bounds, count RNG word `0x00000000` | Slot 158 makes one range draw and stores binary32 −0.5 before base Init; after success SetSpeed bits `0x3C23D70A` on A then B; one bounded draw with span 8 stores count 3; return true |
| PN1-I3 | As I2, count RNG word `0xFFFFFFFF` | Count 10; both model frames, accumulators and modes retain their pre-Init values |
| PN1-I4 | Already initialized, remaining count 1, successful Init, exemplar byte array `[2, 4, 99]`, count RNG word `0x80000000` | No slot-158 draw; both model speeds set again; span 3 selects count 3, replacing 1; ignore element 2 |
| PN1-I5 | Not initialized, count 5, model speed 0.25, base Init returns false | Slot-158 parameter still becomes −0.5; no one-shot speed write or count draw, count remains 5 and speed 0.25; return false. Retry with initialized query still false makes another slot-158 draw |
| PN1-I6 | Successful Init with no exemplar, missing count property, or count property with 0/1 elements | In each case use both tuning bounds, 3 and 10; a lone property byte does not override either bound |
| PN1-I7 | Successful Init, empty model list, exemplar `[0, 0]` | One bounded draw with span 1 stores zero; no removal request; return true |
| PN1-I8 | Successful Init, exemplar `[255, 255]` | One bounded draw with span 1 stores 255 without signed-byte interpretation |
| PN1-I9 | Successful Init, exemplar `[10, 9]`, any random word | Span 0 still consumes one word and stores 10 |
| PN1-I10 | Successful Init, exemplar `[10, 3]`, random word 2 | Span `0xFFFFFFFA`, product high half 1, stored count 11; do not reorder or clamp the bounds |

For the tick fixtures, A/B are live animated-model interfaces with controlled frame/count results
**after** Advance. Record opacity-update, Advance, frame/count-query and removal calls. Unless
specified, the first tick input is 123456 microseconds and the second is binary32 25 milliseconds.

| ID | Fixture/action | Required observation |
|---|---|---|
| PN1-T1 | Models `[A]`, A frame 2/count 4, remaining count 3 | Fade update with 25, then Advance A with 25; frame/count queried afterward; count 3, no removal, return true; microseconds never passed to Advance |
| PN1-T2 | Models `[null, A, B]`, A frame 3/count 4, B also final, remaining count 3 | Advance A, charge once to 2, Advance B; only A supplies the final-frame test; return true |
| PN1-T3 | Models `[A, B]`, A frame 2/count 4, B frame 3/count 4, remaining count 3 | Advance both; no count decrement or removal; return true |
| PN1-T4 | Models `[null, A, B]`, A final, remaining count 1 | Fade update, Advance A, Remove(false), Advance B in that order; count 0, exactly one removal request, return false |
| PN1-T5 | Count 0, A frame 2/count 4; next tick A frame 3/count 4 | First call returns true with no removal; second calls Remove(false), returns false and keeps count 0, never wrapping to 255 |
| PN1-T6 | Empty list or `[null, null]`, remaining count 0 | Fade update only; count 0, no model queries or removal, return true |
| PN1-T7 | A frame 3/count 4, accumulator 0, remaining count 3; three direct zero-ms ticks whose Advances leave that frame unchanged | Counts 2, 1, 0; results true, true, false; only third call requests removal, although every Advance reports no step |
| PN1-T8 | A frame 0/count 1, remaining count 2; two ticks | Counts 1 then 0; second tick requests removal and returns false even if the single-frame model cannot advance |
| PN1-T9 | A reports frame 0/count 0 (interface fixture), remaining count 2 | Signed threshold is −1, so count becomes 1 and result is true; no extra zero-frame guard |
| PN1-T10 | Large second delta, model reports final, remaining count 255 | Count 254, one decrement regardless of elapsed time or any loop count reported by the model |
| PN1-T11 | Direct tick while simulator is paused, or pedestrian flag `0x20` is set without `0x2000`/2 | Same fade, Advance and count behavior as T1–T4 for the supplied model result; this body queries neither pause nor those flags |
| PN1-T12 | Set count through slot 178 to 0 while A is before its final frame; read through slot 177; tick | Getter reports 0; setter causes no playback/removal call; tick behaves as first call of T5 |
