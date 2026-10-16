# Spec: the effects manager and the effects the simulation depends on (`cSC4EffectsManager`)

Status: **behaviour spec, short form for rendering**, clean-room input. This document describes *behaviour*, not
code. It was written by the analysis team from observing the original program and its data files. Implementers must
work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words and tags (*[compat]*, *[choice]*, *[inferred]*, *[quirk]*) as in [foundations.md](foundations.md).
Written 2026-10-15 by the re-specs16 lane (INVENTORY rank 7).

Scope: the city's **effects manager** and its **visual effects**, as far as the simulation and the disasters depend
on them:

- how an effect is created, started and stopped;
- how its components are chosen (detail priority, level of detail, random "select" groups);
- how it is ticked;
- how it finishes and posts its end message;
- the two component kinds that act on the game: the **brush effect**, which deforms terrain, and the **game effect**,
  which burns, demolishes, spills, extinguishes, pauses and posts messages;
- the shipped records that the volcano and the effect disasters use. This answers terrain.md TR2.

**Out of scope (the boundary).** Drawing: particles, decals, models, flashes, tints, camera shakes, the snowline and
the effect maps. Sound playback. The automata attractors, which are only named here (the automata layer, INVENTORY
rank 2). Effects reach the screen through the 3D view's draw pass (`Update(SC4DrawContext)` `0x00593120`). This spec
covers the draw pass in one respect only: it is where effects **finish** (§7). That is a simulation-visible fact.

Related specs:

- [disasters.md](disasters.md): the volcano §10, the effect disasters and the menu §11, the fire manager slots §15,
  the layer §2.4.
- [terrain.md](terrain.md): `ApplyBrush` and `tBrushParameters` §9.2, the component kinds §9.4, who applies brushes
  §9.5, the long terrain operation.
- [demolition.md](demolition.md): `DemolishRegion` and the flag bits §4.9.
- [network-tools.md](network-tools.md): §13.2, the prop visual effect.
- [network-manager.md](network-manager.md): §8.8, potholes.
- [pollution.md](pollution.md): §4.10, the pollution visual effects.
- [residential.md](residential.md): the buzzards.
- [graphics-formats.md](graphics-formats.md): the automata `fx_` effects.
- [simulator.md](simulator.md): §7, the animation time dilation.
- [foundations.md](foundations.md): the message objects §5, the random generators §6, x87 precision §18.
- [mysims.md](mysims.md): the tornado location message.

## 0. Summary for the harness and for planning

- The manager belongs to the city (city slot 85). It is **not** a saved component and writes no record (§13).
- Effect descriptions come from one resource, the **effect directory** (type `0xEA5118B0`, group `0xEA5118B1`,
  instance 1 in `SimCity_1.dat`). It holds 1154 named visual effects (§3).
- **Creating** an effect by name builds its components immediately, including the random choices of §5.3.
  **Starting** it starts the components. The **animation tick** advances started effects with a dilated time step
  that keeps running while the game is paused (§6). Brush and game components act **inside the animation tick**.
- An effect **finishes** only in the 3D view's draw pass (§7). That is also the only place where its **end message**
  is posted.
  - The volcano ends on the end message of `volcano_control` (§10.1).
  - An effect disaster ends on `kSC4MessageEffectDone` (§10.2).
  - *[inferred]* In a session that never draws the city, neither ends, and the emergency pause they hold never
    lifts (§15, quirk 1).
- **TR2 answer**: `volcano_control` picks one of nine "volcano creator" effects. All nine use VolcanoBrush
  `0xCA313035` at brush level 1, strength 1.0 and a fixed radius scale of 0.5–1.3 (by creator). The rate is 30–130
  applications per effect-second, for 2–4 effect-seconds (§8.4).

## 1. Identity

| Item | Value |
|---|---|
| Class | `cSC4EffectsManager`, CLSID `0x49822F75` |
| Interface | `cISC4EffectsManager`, IID `0xC97CF5CD`, vtable `0x00A9F264` (docs/abi, slots 0–10) |
| Owner | created in `cSC4City::CreateCityComponents` (`0x0045AA10`), held at city `+0x1FC`, returned by city slot 85, cached in the global `0x00B43D1C` |
| `Init` | runs in `cSC4City::Init` after the surface water (around `0x0045BF15`) |
| `Shutdown` | in the city's shutdown (`0x0045B1D0`): manager slot 4, then `Release` |
| Visual effect | vtable `0x00AA1170` (19 slots, §11.1); a second interface at `+4`, vtable `0x00AA1148`, lets an effect act as a component of another effect (§11.2) |

## 2. The manager's state that the simulation can observe

| Field | Meaning |
|---|---|
| `+0xB90` max priority | the highest description priority allowed (§4.1). It is 5 at construction, then set from the render properties (the effects detail option) by `DoRenderPropertiesChanged` (`0x00593E10`) |
| `+0xC84` current level of detail | `max(zoom − offset + 1, 1)`, where `offset` (`+0xC8C`) comes from the render properties. Zoom 1–5 gives 1–5 when the offset is 0 |
| `+0x1C0` select generator | a `cRZRandomSuper` seeded with the constant **`0x1105`** at construction (`0x005912E9`–`0x0059133F`). It is never reseeded and is used only by the select groups (§5.3) |
| `+0x1B8`, `+0x1BC` | total dilated microseconds, and the same in seconds (§6) |
| active list (`+0xBAC`..`+0xBB0`), new list (`+0xBA0`..`+0xBA4`) | indices of live effects |

## 3. The effect directory resource *[compat: data format]*

`cSC4EffectsResource::Read` (`0x005A51E0`) reads, in this order:

1. A `u16` root version (3 or 4).
2. Particle descriptions, decal descriptions (100-byte records) and two further render sections.
3. **Brush descriptions**: a `u32` count, then the records (§8.1). In the shipped file: 25 records at offset
   `0x98CA6`.
4. **Attractor descriptions**: a `u32` count, then each record is a `u16` version, a string and a `u8`. In the shipped
   file: 64 records.
5. **Game ("scrubber") descriptions**: a `u32` count, then the records (§9.1). In the shipped file: 57 records.
6. Sequence, sound and camera descriptions. With root version 4, dynamic particle descriptions.
7. **Visual effect descriptions**: a `u16` section version, a `u32` count, then the records below. Shipped: version
   2, 1154 records.
8. The **name map**. Each entry is a `u32` length, the name characters and a `u32` description index. The map ends
   with the name "end" (or a length of `0xFFFFFFFF`). Names are looked up case-insensitively (§4.1).

Strings are a `u32` length followed by the characters, with no terminator.

**Corrects (re-integrate1): the two record tables below give the *in-memory* sizes, not the on-disk ones.**
Measured against the shipped resource — type `0xEA5118B0`, group `0xEA5118B1`, instance 1, **1,094,484 bytes**,
root version **4**, brush section at `0x98CA2` (count `0x19` = 25; the records themselves begin at `0x98CA6`),
visual-effect section at **`0x9C274`** (section version 2, **1154** descriptions, **3373** components in total),
name map at `0x103065` with **1148** names. On disk:

| Record | On disk |
|---|---|
| visual-effect description | `u32 flags & 0x1FF`, `u32 priority`, `u32` component count, then that many **variable-length** component records, `u32` binding count, then that many 24-byte bindings (`u32 & 0xF`, string, `f32`, `u32`), then a string, then (section version > 1) 3 × `u32` |
| component record | **variable**: string (`u32 len` + chars), `u8` type, `u32 flags & 3`, 56-byte transform, `u8`,`u8` LOD, `u16`,`u16` copies/spacing, 4 × `f32`, `u16`,`u16` select group/weight, `u32` index |

So a parser must read the component records as a variable-length sequence inside the effect record; there is no
fixed `0x6C`-byte stride on disk (readers `0x005ABF50` effect, `0x005AB690` component, `0x005ABEC0` binding).

**Visual effect description** (`0x38` bytes in memory, reader `0x005ABF50`):

| Field | File | Meaning |
|---|---|---|
| `+0x00` flags | `u32`, masked to 9 bits | effect flags (§11.1, slot 17) |
| `+0x04` priority | `u32` | 0 is never chosen; shipped values are 1 (882 effects), 3 (271) and 5 (1) |
| `+0x08` components | `u32` count, then the component records | |
| `+0x14` parameter bindings | `u32` count, then `{u32 & 0xF, string, f32, u32}` | render |
| `+0x2C` | string | |
| `+0x20`..`+0x28` | three `u32`, only when the section version is above 1 | the last two are uninitialised (`0xCCCCCCCC`) in the shipped file |

**Component record** (`0x6C` bytes in memory, reader `0x005AB690`):

| Field | File | Meaning |
|---|---|---|
| `+0x00` name | string | the child effect's name for a meta component; the asset name for particles and decals; empty otherwise |
| `+0x0C` type | `u8` | 0 particles, 1 decal, 2 meta (a nested visual effect, found by name), 3 brush, 4 attractor, 5 game, 6 sequence, 7 sound, 8 camera, … up to 15; `0x10` model |
| `+0x10` flags | `u32 & 3` | bit 0: the component is **ignored by the finish test** (§7); bit 1: render |
| `+0x14` placement | 56 bytes (`S3DTransformLegacyLoad`) | local transform |
| `+0x4C`, `+0x4D` | `u8`, `u8` | level-of-detail range `[min, max)` |
| `+0x4E`, `+0x50` | `u16`, `u16` | particle copies and spacing (render) |
| `+0x54`..`+0x60` | 4 × `f32` | render |
| `+0x64` select group | `u16` | 0: always; `g > 0`: one member of group `g` is chosen (§5.3) |
| `+0x66` select weight | `u16` | |
| `+0x68` description index | `u32` | an index into the section of the component's type (brush, game, …) |

Types 3–15 index the description tables that the resource reader filled in section order. So a type 3 component
with index `i` is brush record `i`, and a type 5 component with index `i` is game record `i` (`0x0059192A`–
`0x0059196F`).

## 4. Creating an effect

### 4.1 By name: slot 7 `CreateVisualEffect(const char* name, cISC4VisualEffect** out)` (`0x005939B0`) *[compat]*

1. A null `name` or a null `out` returns false.
2. The name is lowercased. If no description has this name, return false and leave `*out` alone.
3. **Choose the description** (`FindEffectDesc` `0x0058F5A0`):
   1. Among the descriptions with this name, take the one with the highest priority `p` such that
      `0 < p ≤ maxPriority` (`+0xB90`).
   2. If several have that highest priority, the first one met in the name map wins.
   3. If none qualifies, the index is −1, and the call **still succeeds**.
4. Make a new effect object (`NewVisualEffect` `0x00590A60`) holding the name and the index, and put it on the new
   list.
5. AddRef it into `*out`.
6. **Activate** the new effects (§5.1).
7. Return true.

An effect with index −1 has no components. It starts and stops without doing anything. The draw pass counts it as
done without stopping it (§7 rule 1), so it **never posts its end message** *[quirk]*.

### 4.2 By ID pair: slot 6 `CreateVisualEffect(uint32, uint32, cISC4VisualEffect**)` (`0x00593AE0`)

The same as §4.1, except that the description is found by an ID pair (`HasEffectDesc` `0x0058EF90`). Pollution
uses it (pollution.md §4.10). **Slot 5** `HasVisualEffect` (`0x005916C0`) answers whether a name exists.

### 4.3 What a caller does next

The usual sequence:

1. `SetEndMessage` (slot 10), when the caller wants to know when the effect is done.
2. `SetTransform` (slot 8), with the world position and the translation's y set to the terrain altitude.
3. `Start(0)` (slot 3).
4. Then either:
   - keep the reference and later `Stop(1)` and `Release`; or
   - `Release` at once, and the manager recycles the effect when it finishes (§7).

## 5. Components

### 5.1 Activation (`ActivateNewVisualEffects` `0x005921B0`)

Unless the manager's sweep flag `+0x1B4` is set, the manager repeats the following up to 10 times while the new
list is not empty. The draw pass sets the flag while it sweeps the active list (`0x005922C5`, `0x0058DCFC`) and
clears it at the end of `Update` (`0x00593990`).

1. Move the new effects to the active list.
2. Build each one's components (`UpdateVisualEffect(effect, 1)` `0x005919D0`).

A meta component creates its child effect through the same path. The child lands on the new list and is built in
the next round, so nesting is limited to 10 levels per activation.

### 5.2 Which components exist (`UpdateVisualEffect` `0x005919D0`) *[compat]*

A component exists only while the current level of detail `L` (§2) is in its range `[min, max)`. On a zoom change
(`DoZoomChanged` `0x00593BE0`, then `UpdateActiveVisualEffects` `0x00593B90`):

- a component that leaves the range is hard-stopped (its `Stop(1)`) and released;
- a component that enters the range is created. If the effect is started and not stopping, the new component is
  started with `Start(1)`; a newly activated effect's components use `Start(0)`.

**Shipped data**: every component of every shipped effect has the range `[1, 6)`. Since `L` is 1–5, components never
appear or disappear with the zoom. The damage done by brush and game components therefore does **not** depend on
the zoom or on the detail options, apart from the priority filter of §4.1.

**Corrects (re-integrate1): the `[1, 6)` claim above is false as written.** Measured over all **3373** shipped
components, only **2533** have `[1, 6)`. The other **840** do not:

| Range | Count | Range | Count | Range | Count |
|---|---|---|---|---|---|
| `(5, 6)` | 182 | `(1, 3)` | 30 | `(3, 5)` | 8 |
| `(4, 5)` | 156 | `(3, 6)` | 27 | `(1, 5)` | 5 |
| `(3, 4)` | 141 | `(1, 4)` | 23 | | |
| `(2, 3)` | 106 | `(2, 6)` | 10 | | |
| `(1, 2)` | 101 | | | | |
| `(4, 6)` | 98 | | | | |

So components **do** appear and disappear with the zoom. The narrower claim worth keeping — and the one the
implementer's test asserts — is that **every component that reaches a brush or game record, directly or through
meta, is `[1, 6)`**; that still has to be checked per effect, but it is the property the simulation depends on.
Quirk 8 is amended accordingly.

Creating a component of a table type calls that description's factory:

| Type | Factory |
|---|---|
| brush | `0x00581740` |
| game | the `cSC4ScrubberDescription` factory |

The factory may draw random numbers (§8.2).

### 5.3 Select groups *[compat]*

Components of the same select group `g > 0` are alternatives. While building, the manager walks the component list
in order:

1. At the **first** component of a group (a group number different from the previous component's), it draws
   `r := RandomUint32Uniform(0x10000)` from the select generator (`+0x1C0`, §2) and resets a running sum to 0.
2. Each member of the group adds its weight to the running sum. The **first member whose running sum is ≥ r** is
   created.
3. After that, `r` is set to `0x10000`, so the remaining members are skipped.

With weights summing to 65535 and `r` in [0, 65535], exactly one member is created. The draw happens only when a
group's component is being created (it enters the level-of-detail range, or the effect is new), so with the shipped
ranges it happens **once per effect**, when the effect is created (`0x00591C38`–`0x00591C92`).

## 6. The tick (`DoAnimationTick` `0x0058E580`) *[compat]*

On `kSC4MessageAnimationTick` `0x86AD10EE` (data2 = elapsed microseconds `µs`):

1. **Dilate the time step.**
   - `dt' := trunc(r24(µs × dilation))`, where `dilation` is `cISC4Simulator::GetAnimationTimeDilation` (slot 32;
     simulator.md §7) and `µs` is taken as unsigned.
   - With no simulator, `dt' := µs`.
   - Shipped dilation: 1 while paused or emergency-paused, 1.5 at speed 1, 3 at speed 2, and 1 otherwise.
   - *[quirk]* The tick does not stop while the game is paused, so effects keep running.
2. Add `dt'` to the total (`+0x1B8`) and store the total in seconds at `+0x1BC`.
3. For each effect on the active list, in list order, that **is started** (slot 5):
   1. If its flag bit 5 is set, give it the pick ray as parameter 9 (render).
   2. Call its update with `dt = r24(dt' × 1e-6)` seconds (the `double` constant `0x00A9F190`).
   3. The effect updates each of its components, in component order, **whose `IsActive` is true** (`0x005BEA10`).

A brush or game component therefore acts only inside this tick, and only while both it and its effect are active.

## 7. Finishing and the end message *[compat]*

The draw pass (`Update(SC4DrawContext)` `0x00593120` → `DeactivateUnusedVisualEffects` `0x005922A0`) tests each
active effect in list order (`0x005BFD70`):

1. An effect that is **not started**, or whose index is **−1**, counts as done at once. It is not stopped and posts
   nothing.
2. Otherwise the effect is **not done** when any of these holds:
   - flag bit 1 ("no auto stop") is set;
   - a component whose flag bit 0 is **clear** still reports `IsActive`, tested in component order;
   - flag bit 7 is set and the effect is not soft-stopping.
3. Otherwise the effect **finishes**:
   1. `Stop(1)`.
   2. If an end message type was set (slot 10; type ≠ 0), **post** (queued, through the message server `0x00B43CCC`)
      a standard message with that type and data1.
   3. It counts as done.

A done effect whose only remaining reference is the manager's (reference count 1) leaves the active list and is
recycled: its name, IDs and index are reset. A done effect that someone still holds stays on the list. Because it is
no longer started, rule 1 applies to it from then on, so the end message is posted **once**.

No other path posts the end message (`0x005922A0` is called only from `0x00593985`, inside the draw pass).

Consequences:

- **Brush** components (flag 0) keep an effect alive until their stroke ends (§8.3). **Game** components (all
  shipped ones carry flag 1) never do. **Meta** components keep the parent alive while the child effect is started.
- An effect's lifetime also depends on its particles and decals (flag 0), which are rendering objects outside this
  spec. The exact end time of a shipped effect is therefore an open question (EF1).

**Corrects (re-integrate1): the flags are now measured, and the "ignore" bit is not where one might expect.**
Over all **3373** shipped component records (`desc+0x10 & 3`: bit 0 = ignored by the finish test, bit 1 = render):

| Kind | | `flags=0` | `flags=1` | `flags=2` | Counts in the finish test? |
|---|---|---|---|---|---|
| 0 | particles | 1821 | 0 | 100 | **yes** — all 1921 |
| 1 | decal | 406 | 0 | 0 | **yes** |
| 2 | meta (nested effect) | 591 | 0 | 0 | **yes** |
| 3 | brush | 25 | 0 | 0 | **yes** |
| 4 | attractor | 0 | 64 | 0 | no |
| 5 | game | 0 | 56 | 0 | no |
| 6 | sequence | 10 | 0 | 0 | **yes** |
| 7 | sound | 4 | 299 | 0 | 4 count, 299 no (98.7% ignored) |
| 8 | camera | 0 | 41 | 0 | no |
| 16 | model | 3 | 0 | 0 | **yes** |

So:

- **Particles and decals are *not* ignored.** They carry `flags = 0` and are exactly the components that hold an
  effect. The second bullet above is correct and is now *measured fact*, not a hedge.
- **The ignore bit is set uniformly on game, attractor and camera components, and on 98.7% of sound.** "Game
  components never keep an effect alive" is confirmed by measurement (56 of 56).
- Bit 1 (render) is used on exactly the 100 particle components of `robot_attack_control`'s family.
- The description field is `desc+0x10 & 1`; `0x00591730` copies it to **runtime `comp+0x40` bit 2 (value 4)**,
  which is what `0x005BFD70` tests. Table types 3–15 additionally set runtime bit 1 (value 2). An implementer
  must test the **runtime** bit, not the description bit.

**Where the two passes of the effects report disagree** (marked, not silently resolved): the first pass reasoned
that particle components *must* carry the ignore bit, because `cSCParticleSimulatorBase::started` is written
only by `Start` `0x005B9CF0` and `Stop` `0x005B9E70` and so particles can never self-terminate — which would pin
every effect forever. The second pass measured the shipped directory and found `flags = 0` on all 1921 particle
components. **The measurement stands**, and the tension is now the open question: how does a `cSC4ParticlesEffect`
component ever report inactive? Two facts narrow it — its component `Update` (slot 4, `0x00648F10`) is an empty
stub while slot 9 `0x005B8540` is the draw-time update, so particles advance and `CheckForStop` `0x005B9D40` is
evaluated **only in the draw pass**; and `cSC4DynamicParticleEffect::ApplyEffect` `0x00584890` *does* self-stop,
so the pattern exists and simply is absent here. See §16 EF1.

## 8. The brush effect *[compat]*

### 8.1 Description (`cSC4BrushDescription`, `0x34` bytes)

The text form (`cSC4EffectsParser` `0x005A2CF0`) and the binary record (read `0x00581380`, write `0x00581400`) carry
the same fields.

| Field | Binary | Text option | Meaning |
|---|---|---|---|
| version | `u16` | — | 0 |
| `+0x0C` brush ID | `u32` | `-name n` (the terrain brush's name, mapped to its ID; an unknown name is "No such brush") | a terrain brush, terrain.md §9.1 |
| `+0x10` rate | `f32` | `-rate f`, or `-apply f` | applications per effect-second, or the count of a one-shot application |
| `+0x14` length | `f32` | `-length f` | stroke duration in effect-seconds; 0 means no limit |
| `+0x18` one-shot | `u8` | set by `-apply` | |
| `+0x1C` level | `u32` | `-zoom n` stores `n − 1` | `tBrushParameters.level` |
| `+0x20`, `+0x24` | 2 × `f32` | `-strength a [b]` | strength range |
| `+0x28`, `+0x2C` | 2 × `f32` | `-width a [b]` | radius-scale range |
| `+0x30` | `f32` | `-level f` | target height |

Each record is 39 bytes.

### 8.2 Creation (`0x00581490`)

On creation from its description, the brush component:

1. Draws `strength := RandomDoubleRangeUniform(s0, s1)`.
2. Then draws `radius := RandomDoubleRangeUniform(w0, w1)`. Both draws use the static `cRZRandom` at `0x00B22BE8`
   (foundations §6.3 lists it as "brush effects"). Each is one `RandomDoubleUniform` on the generator's
   multiplicative state, stored as `float`.
3. Takes the city terrain (`0x00B43CF4`).

Every shipped record has `s0 = s1` and `w0 = w1`. The values are therefore fixed (`a + u·0 = a`), but the **two draws
still advance the generator**.

### 8.3 Runtime (vtable `0x00A9EDD0`, `0x38` bytes)

State: `active`, `longOp`, accumulator `acc`, elapsed `t`, previous position `prev`, current position `cur`.

- **Start(mode)** (`0x005812C0`): `prev := cur`; `active := 1`; `longOp := 0`; `acc := 0`; `t := 0`.
- **SetTransform(local, parent)** (`0x00581560`): `cur := (x, z)` of the parent translation plus the parent's
  rotation and scale applied to the local offset. With an identity local or parent, the other's translation is used
  directly.
- **IsActive** returns `active`.
- **Update(dt)** (`0x005817F0`):
  - **One-shot** (`+0x18` set): `active := 0`, then `Apply(trunc(rate))`. Nothing else: no long operation, no
    accumulator.
  - **Stroke**:
    1. The first time, start a long terrain operation (terrain slot 87 `BeginLongTerrainOperation`) and set `longOp`.
    2. `acc := r24(r24(dt × rate) + acc)`.
    3. `t := r24(dt + t)`.
    4. `n := floor(acc)`. The original uses the "add 1.5·2²³" trick with an equality fix-up, which gives exactly
       `floor` for `0 ≤ acc < 2²²`.
    5. `Apply(n)`.
    6. `acc := r24(acc − n)`.
    7. If `length > 0` and `t > length`: `active := 0`.
- **Stop(mode)** (`0x00581530`): if `longOp`, end the long terrain operation (terrain slot 88); `active := 0`.
- **Apply(n)** (`0x00581640`):
  1. If either `prev` or `cur` is outside the city (city slot 98 `LocationIsInBounds`), do nothing. *[quirk]* `prev`
     is **not** updated in that case.
  2. Otherwise build `tBrushParameters` (terrain.md §9.2): level = description `+0x1C`, strength = the drawn
     strength, radius = the drawn radius, `data4 = 3`, target height = description `+0x30`.
  3. For `i = 0 … n−1`: `from := prev`, `to := cur`, `redisplay := (i = n − 1)`, then terrain slot 57
     `ApplyBrush(brushID, &params)`.
  4. Then `prev := cur`. This happens even when `n = 0`.

### 8.4 Shipped brush records (`SimCity_1.dat` effect directory; answers TR2)

| # | Brush | Rate | Length (s) | One-shot | Level | Strength | Radius scale | Used by |
|---|---|---|---|---|---|---|---|---|
| 0 | CraterBrush `0x8A33E17A` | 1 | 0 | yes | 2 | 4 | 0.65 | `mushroomcloud` |
| 1 | CraterBrush | 2 | 0.5 | no | 3 | 3 | 0.4 | `powerplant_explosion` |
| 2 | VolcanoBrush `0xCA313035` | 30 | 3.0 | no | 1 | 1 | 0.5 | `volcanocreator1a` |
| 3 | VolcanoBrush | 40 | 4.0 | no | 1 | 1 | 0.5 | `volcanocreator1b` |
| 4 | VolcanoBrush | 45 | 3.5 | no | 1 | 1 | 0.6 | `volcanocreator2a` |
| 5 | VolcanoBrush | 50 | 3.5 | no | 1 | 1 | 0.6 | `volcanocreator2b` |
| 6 | VolcanoBrush | 55 | 3.0 | no | 1 | 1 | 0.8 | `volcanocreator3a` |
| 7 | VolcanoBrush | 65 | 3.0 | no | 1 | 1 | 0.8 | `volcanocreator3b` |
| 8 | VolcanoBrush | 65 | 3.0 | no | 1 | 1 | 1.0 | `volcanocreator4a` |
| 9 | VolcanoBrush | 80 | 3.5 | no | 1 | 1 | 1.0 | `volcanocreator4b` |
| 10 | VolcanoBrush | 130 | 2.0 | no | 1 | 1 | 1.3 | `volcanocreator5` |
| 11–15 | CraterBrush | 2 | 0 | yes | 3 | 4, 6, 3, 2, 5 | 0.5, 0.7, 0.8, 0.6, 0.9 | `meteorcrater_a`–`_e` |
| 16–20 | CraterBrush | 2 | 0 | yes | 3 | 4, 6, 3, 2, 5 | 0.5, 0.8, 1.0, 0.6, 0.7 | `meteorcraterwater_a`–`_e` |
| 21 | CraterBrush | 2 | 0 | yes | 3 | 4 | 0.5 | `lasercraterwater_a` |
| 22 | ErosionBrush `0x6A4BAAFA` | 1 | 0 | yes | 2 | 1 | 1 | `erodefx_terrain` |
| 23 | SmoothingBrush `0xEA313BB9` | 1 | 0 | yes | 2 | 1 | 1 | `erodefx_terrain` |
| 24 | ErosionBrush | 1 | 0 | yes | 4 | 0.3 | 0.25 | `stream_cut_fx_terrain` |

All target heights are 100, all ranges are degenerate (the strength and radius columns give the single value), and
every component range is `[1, 6)`.

The brush exemplars (type `0x6534284A`, group `0x6A01FC2A`) give each component's radius and strength per level
(terrain.md §9.1):

| Brush | Component | Kind | Radius per level | Strength per level | Neutral |
|---|---|---|---|---|---|
| VolcanoBrush | 0 | 0 | 800, **500**, 300, 150, 100 | 0.0024 | 0 |
| VolcanoBrush | 1 | 0 | 300, **200**, 100, 75, 50 | 0.002 | 255 |
| CraterBrush | 0 | 0 (oriented) | 800, 500, 300, **250**, 150 | 0.1, 0.075, 0.04, **0.025**, 0.01 | 128 |

The level the effects use is in bold. With neutral 0, component 0 of VolcanoBrush only **raises** (by `b × 0.0024` per
application). With neutral 255, component 1 only **lowers** (by `(255 − b) × 0.002`), which digs the crater. The
bitmaps are terrain data (terrain.md §9.3).

## 9. The game effect *[compat]*

### 9.1 Description (`cSC4ScrubberDescription`, `0x50` bytes, constructor `0x005BC9D0`)

The text keywords `scrubberEffect`, `gameEffect`, `demolishEffect` and `mapEffect` all use this parser (`0x005A30B0`).

| Field | Text option | Meaning |
|---|---|---|
| `+0x0C` flags | `noNetworks` 1, `noFlora` 2, `dezone` 4, `single` 8, `-pauseSim [d]` 0x10, `-pauseSimHidden` 0x20, `-pauseClock` 0x40 | |
| `+0x14` demolish radius | `-demolish r` | cells, an integer |
| `+0x18` demolish flags | default 1; `explode` gives `0x1300 \| 2`; `createRubble` `0x1300`; `createBurntRubble` adds `0x300`; `demolishEffectID n` sets the low byte | demolition.md §4.9 |
| `+0x1C`, `+0x20` | `-minDemolishSize`, `-maxDemolishSize` | metres; 0 = no limit |
| `+0x24` burn radius | `-burn r` | |
| `+0x28` toxic amount | `-toxic n` | |
| `+0x2C` extinguish amount | `-extinguishFire u` | |
| `+0x30`, `+0x34` | `-message type [data]` | |
| `+0x38`..`+0x48` | `-blob k a b [c]` / `-rect …` | the effect-map shape (render; k = 1–8). Defaults: `+0x3C` = 16.0 |
| `+0x4C` duration | the `d` of `-pauseSim` | effect-seconds; 0 = none |

**Binary record**:

- version 1 (write `0x005BCA60`, read `0x005BD260`);
- `u16` version;
- `u32` flags (masked to 7 bits);
- `u32` `+0x10`, `+0x14`, `+0x18`;
- `f32` `+0x1C`, `+0x20`;
- `u32` `+0x24`, `+0x30`, `+0x34`, `+0x38`;
- `f32` `+0x3C`;
- 8 bytes `+0x40`;
- `f32` `+0x48`, `+0x4C`;
- then, only when the version is above 0: `u32` `+0x28` and `+0x2C`.

That is 70 bytes at version 1.

### 9.2 Runtime (vtable `0x00AA0FF8`, constructor `0x005BC310`)

State:

- `pending` (initially 1);
- `started`;
- `timer`;
- `count`;
- the cell (initially −1, −1);
- `data` (`+0x58`, initially the description's message data).

Slots:

- **SetParameter(13, p, …)** (slot 6, `0x005BC2B0`): `data := *(uint32*)p`. Other indices are ignored.
- **SetTransform** (`0x005BC3B0`): computes the world position. `cell := (floor(x / 16), floor(z / 16))`. A change of
  cell sets `pending := 1` (and recomputes the map rectangle).
- **Start** (`0x005BD070`). Only when not started and the simulator exists:
  1. `started := 1`; `count := 0`.
  2. Flag `0x10` → simulator slot 9 `Pause`; `0x20` → slot 10 `HiddenPause`; `0x40` → the 24-hour clock
     (`0x00B43DBC`) slot 26 with 1.
  3. `timer := 0`.
- **IsActive** returns `started`.
- **Update(dt)** (`0x005BD4D0`):
  - **single** (flag 8): `Apply`, then `started := 0`. *[quirk]* The pauses are not undone. No shipped record
    combines `single` with a pause.
  - otherwise:
    1. `timer += dt` (24-bit rounding).
    2. If `duration > 0` and `timer > duration`: `Stop(0)` and return.
    3. Else, if `pending`: `Apply`, `count += 1`, `pending := 0`.
- **Stop** (`0x005BD340`). Only when started:
  1. `started := 0`.
  2. Undo the pauses in the order resume (slot 12), hidden resume (slot 13), clock slot 26 with 0.
  3. Remove the effect map if one was applied.

A moving game effect (one attached to a walking robot, say) therefore applies once on its first update and again on
each update after it enters a new cell.

### 9.3 `Apply` (`0x005BD3C0`): the order of actions

At the effect's current cell `(cx, cz)`:

1. **Burn** (`+0x24 > 0`) (`0x005BCBF0`):
   1. Get the fire manager: city slot 72 → layer slot 7 `GetDisasterManager(0x621CDA33)` → QI `0x49B43F57`.
   2. Call `IgniteCellRadius(cx, cz, 0, burn)` (fire manager slot 8; disasters.md §5.6).
   3. Then call `SetCause(4)` (slot 9).
2. **Toxic** (`+0x28 > 0`): layer slot 7 `GetDisasterManager(0xEBFB0404)`, then its slot 6
   `AttemptDisasterStart(cx, cz)`.
   - *[quirk]* The amount is ignored: the spill is always 120 (disasters.md §9).
   - *[quirk]* There is no null check on the manager.
3. **Demolish** (`+0x14 > 0`) (`0x005BD0F0`):
   1. The cell rectangle `[cx − r, cx + r] × [cz − r, cz + r]` is clipped to the city (city slots 93 and 94).
   2. It is passed as a full `SC4CellRegion` to demolition slot 6:
      `DemolishRegion(true, region, privilege 0, flags = +0x18, dezone = flag 4, filter, null, …)`.
   3. The **filter** (`0x005BCE80`) skips:
      - signposts (`0x4B44FBE2`) and text decals (`0x2B3B7D86`);
      - with `noNetworks`, network occupants (`0xA821EF94`);
      - with `noFlora`, flora (`0x74758926`);
      - when `max > 0`, occupants whose largest bounding-box extent is ≥ max;
      - when `min > 0`, occupants whose smallest extent is ≤ min.
4. **Extinguish** (`+0x2C > 0`) (`0x005BCC80`): fire manager slot 6 `ExtinguishCell(cx, cz, amount)`.
5. **Message** (`+0x30 ≠ 0`): **post** (queued) a standard message with type `+0x30`, data1 `data`, data2 `cx`,
   data3 `cz`.
6. **Map** (`+0x38 ≠ 0`): if `count > 0`, remove the previous map; then apply the map at the cell (render).

### 9.4 Shipped game records (57; those with a simulation action)

| # | Used by | Action |
|---|---|---|
| 0 | `debriscorefx_terrain` | burn 1 |
| 1 | `mushroomcloud` | single; demolish r = 10, flags 1 |
| 2 | `mushroomcloud` | burn 15 |
| 3, 4 | `mushroomcloud`, `powerplant_explosion` | hidden pause, 5 s |
| 5, 6 | `powerplant_explosion` | demolish 2; burn 6 |
| 7, 8 | `electrical_fire_starter_terrain`, `lava_fallout_burn_terrain` | burn 1 |
| 9–12 | `volcanocreator1a`, `1b`, `2a`, `2b` | demolish 5, flags 2; burn 10 |
| 13–15 | `volcanocreator3a`, `3b`, `4a` | demolish 8, flags 2; burn 12 |
| 16 | `volcanocreator4b` | demolish 10, flags 2; burn 12 |
| 17 | `volcanocreator5` | demolish 10, flags 2; burn 14 |
| 18, 20, 21 | `robot_walkfx_slave`, `shooting_platesfx_slave`, `robot_firefx_slave` | noNetworks; demolish 1, flags `0x1301` |
| 22 | `robot_landfx_slave` | noNetworks; demolish 1, flags `0x1301`; burn 2 |
| 19, 23, 36 | `robot_walkfx_slave`, `meteor_stuff_slave`, `tsaurus_walkfx_slave` | message `0x2A7B582E` (§10.2) |
| 24, 25 | `meteor_stuff_terrain`, `meteor_stuff_water` | demolish 6, flags 2; burn 10 |
| 26 | `tornadofx_slave` | noNetworks; demolish 4, flags `0x1302`, max size 48 m; message `0x0A36F1E9` (kSC4MessageTornadoLocation, mysims.md) |
| 27 | `earthquakefire_terrain` | noNetworks, noFlora; demolish 2, flags `0x1302` |
| 28 | `quaker` | noNetworks, noFlora; demolish 1, flags `0x1302` |
| 29 | `lightningstrike_trigger` | burn 2 |
| 30 | `sparkles_on_demolish` | hidden pause, 5 s |
| 31 | `obliterate_finale` | hidden pause, 15 s |
| 32 | `lasercrater_a` | noNetworks, noFlora; demolish 1, flags `0x1301` |
| 33 | `biglasercrater_a` | demolish 2 |
| 34 | `mouthfirefx_slave` | noNetworks; demolish 1, flags `0x1301`; burn 2 |
| 35 | `tsaurus_walkfx_slave` | noNetworks; demolish 1, flags `0x1301` |
| 40, 45 | `rocket_explodefx_terrain`, `molotov_flames` | burn 2, burn 1 |
| 41, 42 | `rocket_explodefx2_terrain`, `rocket_explodefx3_terrain` | burn 1 |
| 44 | (no effect uses it) | demolish 1 |
| 46 | `smlmushroomcloud` | hidden pause, 5 s |
| 47 | `tankfire_terrain` | noNetworks; demolish 1, flags `0x1302`; burn 2 |
| 48–50 | `water1drop_slave`, `water1drop_terrain`, `waterdrop` | extinguish 10000 |
| 51, 52 | `helifire_terrain`, `jetfire_terrain` | demolish 2, flags `0x1302`; burn 2 |
| 53 | `ufolasercrater_a` | noNetworks; demolish 1, flags `0x1302`; burn 2 |
| 54 | `whalefire_terrain` | noNetworks; demolish 5, flags `0x1302`; burn 2 |
| 55 | `p_train_toxic_slave` | toxic 20 (spills 120, §9.3) |
| 56 | `collisions_active` | extinguish 4 |
| 37–39, 43 | `runwaysmall`/`medium`/`large`, `whalefx_slave` | effect maps only (render) |

Flags 1 and 2 are the cause byte of demolition.md §4.9. `0x1301` and `0x1302` add the explode bits `0x300` with
`0x1000`.

## 10. The effects the disasters use

### 10.1 The volcano (disasters.md §10)

1. **Starting the effect.** In state 3, the volcano instance's `StartDeformation` (`0x0057F4F0`) runs once (flag
   `+0x41`):
   1. It creates `"volcano_control"` (the string at `0x00A9E9E0`) by name through `0x0057FCE0`.
   2. It sets the end message to `0xCA493C1D` with data1 = the instance.
   3. It sets the transform to the vent: identity rotation, scale 1, translation `(x, GetAltitude(x, z), z)`.
   4. It starts the effect with `Start(0)` and **keeps** the reference.
2. **What the effect does.**
   - **`volcano_control`** (index 367, priority 1) has two kinds of component:
     - one select group of nine meta components, `volcanocreator1a`, `1b`, `2a`, `2b`, `3a`, `3b`, `4a`, `4b` and
       `5`, with weights 7281 × 8 and 7287;
     - the attractor `volcano_disaster` (attractor 18, flag 1).
   - **Each creator** (priority 1) has:
     - one VolcanoBrush stroke (brush records 2–10, §8.4);
     - one game record (records 9–17, §9.4): demolish 5–10 cells, burn 10–14, both applied **once**, at the vent
       cell, on the first tick;
     - four sounds, smoke and fallout particles, and lava and crater decals.

   **Measured component tree** (re-integrate1, from the shipped directory):

   | Effect | Component | Kind | `flags` | Ignored? |
   |---|---|---|---|---|
   | `volcano_control` (367) | `volcanocreator1a`…`4b` (×8) | 2 meta | **0** | no |
   | | `volcanocreator5` | 2 meta | **0** | no |
   | | attractor idx 18 | 4 | **1** | **yes** |
   | `volcanocreator1a` (358), 11 components | brush idx 2 | 3 | 0 | counts |
   | | 4 × sound | 7 | 1 | ignored |
   | | particles `volcano_smoke_small` (423), `volcano_fallout_up` (428) | 0 | **0** | **count** |
   | | decals `glowing_lava_small` (128), `glowing_lava_small_water` (129), `craterdecal_small` (134) | 1 | **0** | **count** |
   | | game idx 9 | 5 | 1 | ignored |
   | `volcanocreator5` (366), 13 components | brush idx 10 | 3 | 0 | counts |
   | | 4 × sound | 7 | 1 | ignored |
   | | 4 particles (`volcano_smoke_large`, `volcano_fallout`, `volcano_fallout_up`, `volcano_fallout_wide`) | 0 | **0** | **count** |
   | | 3 decals (`glowing_lava_large`, `glowing_lava_large_water`, `craterdecal_large`) | 1 | **0** | **count** |
   | | game idx 17 | 5 | 1 | ignored |

   Exactly one creator is built. Its meta component's `IsActive` is **the child effect's `started` byte** (the
   `+0x08`-thunk at `0x005C04F0`), not the child's components' `IsActive`, so `volcano_control` cannot finish
   before the child has been finished by a previous sweep — it waits for a **full extra sweep**.
3. **Ending.** The volcano's `DoMessage` (`0x0057F6C0`) maps `0xCA493C1D` whose data1 is this instance to
   `SetState(7)`:
   - state 7 sets the end-requested flag (`+0x3E`);
   - if the lava is enabled and running, it returns to state 5 and the lava finishes first;
   - otherwise it goes to state 8.

   So the volcano's end is triggered by the effect **finishing** (§7). The deformation stroke is at least `length`
   effect-seconds (2–4 s × dilation), and the particles and decals may hold the effect longer (EF1).

   **Corrects (re-integrate1): the finish condition is now measured.** `volcano_control` finishes at the first
   draw-pass sweep (`0x005922A0`) after (i) the chosen `volcanocreator*` child has itself been finished on a
   previous sweep, and (ii) at that sweep that child's **brush** (`t > length`, 2.0–4.0 dilated effect-seconds),
   its **2–4 particle components** and its **3 decal components** are all inactive. The attractor and the game
   record never hold it (both `flags = 1`). Brush and decals have known end times; **particles are the sole
   unknown** (§16 EF1). A static bound: the end is ≥ the child's brush length plus one extra draw-pass sweep,
   and ≤ that plus any decal lifetime.
4. **Shutdown.** The volcano's `Shutdown` (`0x0057F920`) stops the effect with `Stop(1)` and releases it.
5. **A random start.** The volcano manager's argument-less `AttemptDisasterStart` (`0x0057F5A0`; Ghidra names it as
   a function of the effect-disaster manager) does the following:
   1. It makes a throwaway `cRZRandom` seeded with −1 (the wall clock).
   2. It draws `x := RandomSint32RangeUniform(0, cellsX)` (city slot 93), then `z` the same way with `cellsZ` (slot
      94).
   3. It calls its own `AttemptDisasterStart(x, z)`.

### 10.2 The effect disasters (disasters.md §11)

**The menu tool** (`cSC4ViewInputControlEffect`, `SetEffectName` `0x004BB590`) builds its effects from the exemplar's
name `N` (`0x4A08C700`):

| Effect | Tool field |
|---|---|
| `N_trigger`, `N_button_trigger` | |
| `N_button` | |
| `N_single` | `+0x98` |
| `N_control` | `+0x94` |
| `N_active` | `+0x9C` |
| `N_inactive` | |

**Starting the disaster.** When the exemplar's manager type is not one of the layer's managers, the effect-disaster
tool's `SetTerrainControlledEffectState(true)` (`0x005803D0`) does the following:

1. It places `N_control`.
2. It QIs the effect-disaster manager for `0xCA7B3F3C`.
3. It calls `StartDisasterEffect(N_control, position, type)` (slot 3, `0x00577360`, pop 12). This:
   1. creates the instance (`0x005772B0`, which AddRefs the effect);
   2. calls the instance's `Init` (`0x00577010`);
   3. hands it the position (instance slot 7).

When the type **is** a layer manager (volcano `0xC4C90997`, earthquake `0x42963812`, pipe burst `0x4A1332E8`), the
tool only places `N_control` and never starts it. Mouse-up calls that manager's `AttemptDisasterStart(x, z)`
instead.

**The instance's `Init`** (disasters.md §11):

1. `SetEndMessage(0x2A7B582D kSC4MessageEffectDone, instance)`.
2. Effect slot 13 `SetParameter(13, &instance, 1)`. It reaches every component's slot 6, so each game component's
   `data` becomes the instance.
3. `Start(0)`.
4. The manager's `AddInstance`.
5. Subscribe to `0x2A7B582D` and `0x2A7B582E kSC4MessageEffectLocation`.
6. `DisasterHasBegun`.

The game records with message `0x2A7B582E` (19, 23, 36) then report the cell (data2, data3) with data1 = the
instance, which updates the instance's location (`0x00576E70`).

**Corrects (re-integrate1): "data1 = the instance" holds only for slaves reached as *meta* children.** There is
no single spawn API: each particle class builds a child effect name by concatenating its own base name with a
fixed suffix (`FUN_0041E270` `0x0041E270`) and calls `CreateVisualEffect` (slot 7). The suffixes and their
callers are `_slave` (`cSC4ModelBasedParticlesEffect::CreateParticle` `0x005AEF20`, per particle, and
`cSC4DynamicParticleEffect::Start` `0x00585190`, once), `_death` (`DestroyParticle` `0x005AF390`), `_collide`
(`cSC4DynamicParticleEffect::ApplyEffect` `0x00584890`), and `_terrain` / `_water` (`FUN_005B8290`, on impact).
Parameter 13 reaches **game components only** (`cSC4Scrubber` slot 6 `0x005BC2B0` accepts index 13 and copies
`*value` into `this+0x58`; particle `SetVoidParams` `0x005B2200` accepts only index 7; decals only index 6;
meta recurses via `0x005C0540`). Slaves spawned through the **particle** path get the particle's own `+0x26C`
value (flag `+0x26B`), **not** the disaster instance. Only a slave reachable as a **meta component** of a
control effect the instance started can post `data1 = the instance`. Whether the location update matches on
something other than `data1 == instance` is open (§16 EF6).

**The shipped slave family** (measured: 1148 names in the name map):

| Suffix | Count | Role | Examples (index) |
|---|---|---|---|
| `_slave` | **143** | per-particle / per-emitter companion | `robot_walkfx_slave`(368), `meteor_stuff_slave`(384), `tornadofx_slave`(399), `tsaurus_walkfx_slave`(455), `default_dynparticle_slave`(**1116**), `car_slave`(586), `helifire_slave`(993), `debriscorefx_slave`(197), `blimpycreatorfx_slave`(528), `fireworks_slave`(464) |
| `_terrain` | **33** | impact on terrain | `meteor_stuff_terrain`(390), `earthquakefire_terrain`(402), `tankfire_terrain`(952), `rocket_explodefx_terrain`(590), `erodefx_terrain`(1076), `whalefire_terrain`(1034) |
| `_water` | **20** | impact on water | `meteor_stuff_water`(396), `lava_meets_water`(353), `heliwake_water`(668), `whalefire_water`(1035) |
| `_death` | **21** | on particle death | `robot_walkfx_death`(380), `robot_landfx_death`(381), `tsaurus_walkfx_death`(461), `bolt*fx_death`(409–415), `skydiverdeath_death`(1043) |
| `_collide` | **3** | on collision | `p_train_collide`(1117), `p_train_toxic_collide`(1120), `p_vehicle_collide`(1114) |
| `_control` | **6** | disaster control | `volcano_control`(367), `robot_attack_control`(382), `meteorstrike_control`(383), `tornado_control`(400), `ufo_control`(453), `tsaurus_attack_control`(463) |

`default_dynparticle_slave` being a real shipped effect **confirms** the literal lookup string in
`cSC4DynamicParticleEffect::AttachSlaveEffect` `0x005840E0`. Lightning has no `_control`, as the table above
says. So `meteor_stuff_terrain` / `_water` / `_slave` share the base `meteor_stuff`, and `robot_walkfx_slave`
has base `robot_walkfx` — the §10.2 guess below is now proven and generalised to five suffixes.

Three named effects, measured:

- **`meteor_stuff_terrain` (390)** — 8 components: particles `meteorhit` (450, `flags=0`), sound (144, ignored),
  game idx 24 (`flags=1`, ignored), and 5 meta `meteorcrater_a`…`_e` in select group 1 weight 13107.
- **`meteor_stuff_water` (396)** — decal `meteor_hit_water`, particles `meteor_splash_water`, game **idx 25**,
  5 meta `meteorcraterwater_a`…`_e`.
- **`meteor_stuff_slave` (384)** — 2 particles (`meteortrailburn`, `meteortrailsmoke`), 2 decals
  (`meteor_shadow_terrain`, `meteor_shadow_water`), sound 143, game idx 23, camera 4.

The game components in all three are `flags = 1` → ignored, so they never hold those effects alive; the meta
craters do.

**The shipped control effects** (all priority 1, all components `[1, 6)`):

| Menu entry | `N_control` | Components | Damage (through particle-spawned "slave" effects *[inferred from the names; the particle descriptions are render data and were not traced]*) |
|---|---|---|---|
| Meteors | `meteorstrike_control` (383) | particles `meteorcore` | `meteor_stuff_terrain` / `_water`: game 24 / 25 and one of five crater brushes (select group, weights 13107 each; records 11–15 / 16–20, one-shot, 2 applications); `meteor_stuff_slave`: location |
| Tornado | `tornado_control` (400) | particles `tornado_core` | `tornadofx_slave`: game 26 (demolish 4, tornado location), attractor `tornado_disaster` |
| RobotAttack | `robot_attack_control` (382) | particles `giant_robot_landing`, `giant_robot_walk`, `giant_robot_fire` | `robot_walkfx_slave` (18, 19), `robot_firefx_slave` (21), `robot_landfx_slave` (22), `shooting_platesfx_slave` (20) |
| Truckasaurus | `tsaurus_attack_control` (463) | a sound, meta `tsaurusphase_in`, particles `tsaurus_phase_in`, `tsaurus_walk` × 7, `tsaurus_dies`, attractor `tank_repulser` | `tsaurus_walkfx_slave` (35, 36), `mouthfirefx_slave` (34) |
| UFOs | `ufo_control` (453) | meta `ufo` (particles `ufo_b` × 3, `ufocreator`, `ufo_group_appear`) | `ufolasercrater_a` (53), `lasercrater_a` (32), `biglasercrater_a` (33), `lasercraterwater_a` (brush 21) |
| Lightning | (no `_control`, no manager type) | | `lightningstrike_trigger`: one of seven bolts (weights 9362 × 6, 9363), game 29 (burn 2), attractor `tornado_disaster` *[inferred: played as the tool's trigger effect, with no disaster instance]* |

### 10.3 Other callers (cited, not repeated)

| Caller | Effect | Where |
|---|---|---|
| Prop visual effects | started and stopped through the prop occupant (effect slots 3 and 4 with 0) | network-tools.md §13.2 |
| Potholes | `"pothole"` / `"pothole_rail"` | network-manager.md §8.8 |
| Buzzards | `"buzzards"`, created by name and released at once | residential.md, residential-education-health.md |
| Pollution | the ID-pair overload (slot 6) | pollution.md §4.10 |
| Automata | `fx_` names | graphics-formats.md |
| Obliterate-city tool | `"sparkles_on_demolish"` (`0x0049D710`) and `"obliterate_finale"` (`0x0049CDF6`); their game records hidden-pause the simulator for 5 s and 15 s | |

The remaining names in §9.4 (`mushroomcloud`, `powerplant_explosion`, `collisions_active`, `p_train_toxic_slave`,
the rocket, tank, helicopter, jet and whale effects) are not named in the program. They are referenced from data:
exemplars, automata scripts or other effects.

## 11. Interface slot tables *[compat]*

### 11.1 `cISC4VisualEffect` (vtable `0x00AA1170`)

| Slot | Offset | Address | Method | Pop | Notes |
|---|---|---|---|---|---|
| 0 | `+0x00` | `0x005BE7F0` | QueryInterface | 8 | |
| 1 | `+0x04` | `0x004BDF00` | AddRef | 0 | |
| 2 | `+0x08` | `0x005BE840` | Release | 0 | |
| 3 | `+0x0C` | `0x005BE920` | `Start(mode)` | 4 | below |
| 4 | `+0x10` | `0x005BEBA0` | `Stop(mode)` | 4 | below |
| 5 | `+0x14` | `0x007D6680` | `IsStarted` | 0 | AL |
| 6 | `+0x18` | `0x00644FC0` | `IsStopping` (soft stop in progress) | 0 | AL |
| 7 | `+0x1C` | `0x005BE860` | set the secondary transform (`+0x6C`) | 4 | |
| 8 | `+0x20` | `0x005BE880` | `SetTransform(const cS3DTransform&)` | 4 | |
| 9 | `+0x24` | `0x005BFAC0` | attach to a model or occupant | 4 | render |
| 10 | `+0x28` | `0x006CE090` | `SetEndMessage(uint32 type, uint32 data1)` | 8 | |
| 11 | `+0x2C` | `0x005BEA50` | `SetParameter3(index, float*, count)`: `count × 3` floats, then slot 12 | 12 | |
| 12 | `+0x30` | `0x005C05E0` | `SetParameter(index ≤ 19, float*, count)`: stored and forwarded to the components | 12 | |
| 13 | `+0x34` | `0x005BFC20` | forward a parameter to the components only (their slot 6), unless flag bit 4 | 12 | |
| 14 | `+0x38` | `0x005BEA60` | `GetTransform(cS3DTransform&)` | 4 | |
| 15 | `+0x3C` | `0x005BEAC0` | get the secondary transform | 4 | |
| 16 | `+0x40` | `0x005C0740` | add a component | 8 | |
| 17 | `+0x44` | `0x005BFC90` | `TestFlag(n)`, n ≤ 8 | 4 | AL |
| 18 | `+0x48` | `0x005C05C0` | scalar deleting destructor | 4 | |

**`Start(mode)`**:

1. Start each component with its slot 0 and the same mode.
2. If `mode ≠ 2` and the index ≥ 0, register the effect with the manager (`0x005908A0`).
3. If a start message is set (`+0x120`), post it: type `+0x120`, data1 `+0x124`, data2 `+0x128`, the IGZUnknown =
   the effect.
4. `started := 1`; `stopping := 0`.

**`Stop(mode)`**:

1. Mode 0 on an effect with flag bit 2 ("hard stop") becomes mode 1.
2. Stop each component with its slot 1 and that mode.
3. If `mode ≠ 2`, unregister the effect (`0x0058E340`).
4. If `mode ≠ 0`: `started := 0`, `stopping := 0`. Mode 0 is a soft stop: `stopping := 1`.

The known flag bits:

| Bit | Value | Meaning |
|---|---|---|
| 1 | 2 | no auto stop |
| 2 | 4 | hard stop |
| 4 | `0x10` | slot 13 does not forward |
| 5 | `0x20` | takes the pick ray |
| 7 | `0x80` | finishes only after a soft stop |

### 11.2 The effect component interface

Three implementations:

| Implementation | Vtable |
|---|---|
| the visual effect itself, at `+4` | `0x00AA1148` |
| the brush effect | `0x00A9EDD0` |
| the game effect | `0x00AA0FF8` |

| Slot | Method | Pop | Visual effect (`+4`) | Brush | Game |
|---|---|---|---|---|---|
| 0 | `Start(mode)` | 4 | → slot 3 | `0x005812C0` | `0x005BD070` |
| 1 | `Stop(mode)` | 4 | → slot 4 | `0x00581530` | `0x005BD340` |
| 2 | `IsActive` | 0 | → `IsStarted` | `0x007D6680` | `0x00644FC0` |
| 3 | `SetTransform(local, parent)` | 8 | `0x005BEC20` | `0x00581560` | `0x005BC3B0` |
| 4 | `Update(float dt seconds)` | 4 | `0x005BEA10` | `0x005817F0` | `0x005BD4D0` |
| 5 | `SetParameter(index, float*, count)` | 12 | → slot 12 | `0x005BE2E0` (no-op) | `0x005BE2E0` (no-op) |
| 6 | `SetParameter`, the second channel | 12 | → slot 13 | `0x005BE2E0` | `0x005BC2B0` (index 13 only) |
| 7 | AddRef | 0 | | | |
| 8 | Release | 0 | | | |

### 11.3 `cISC4DisasterManagerEffect` (IID `0xCA7B3F3C`, vtable `0x00A9E190` in the effect-disaster manager)

| Slot | Method | Pop |
|---|---|---|
| 0–2 | QueryInterface, AddRef, Release (thunks, `this − 0x24`) | |
| 3 | `StartDisasterEffect(cISC4VisualEffect*, const cS3DVector3* position, uint32 type)` → instance | 12 |
| 4 | `AttractDisasterEffect(…)` | 4 |
| 5 | `CancelAttract()` | 0 |

The proposed `docs/abi` rows are in `build-runs/re-specs16-rows.tsv` of this lane.

## 12. Messages

| ID | Direction | Data |
|---|---|---|
| `0x86AD10EE` kSC4MessageAnimationTick | in | data2 µs; drives §6 |
| `0x697D01ED` | in | `DoEffectsTick` (`0x00593D00`): parses queued effect files, snowline (render) |
| `0x99EF1142` / `0x99EF1143` | in | occupant inserted / removed: the effect maps of static buildings (render) |
| `0xA6B79602` kSC4MessageViewZRChange3, `0xA6B79620` kSC4MessagePostViewInit | in | zoom / view change → §5.2 |
| `0xAB96B05F` kSC4MessagePostAppServicesInitComplete | in | `UpdateActiveVisualEffects` |
| `0xC9F775BB` | in | render properties changed → §2 |
| `0x4921FF8D`, `0x4921FF8E` | in | cheats |
| an effect's start message | out, queued | type and data set on the effect (slot 3) |
| an effect's end message | out, queued, from the draw pass only | type, data1 (slot 10) |
| game-record messages | out, queued, from the tick | type `+0x30`, data1 `data`, data2 `cx`, data3 `cz` |
| `0xCA493C1D` | volcano_control → volcano | data1 = the volcano instance |
| `0x2A7B582D` kSC4MessageEffectDone | effect → effect-disaster instance | data1 = the instance |
| `0x2A7B582E` kSC4MessageEffectLocation | game record → instance | data1 = the instance, data2 / data3 = cell |
| `0x0A36F1E9` kSC4MessageTornadoLocation | game record 26 → MySims | data2 / data3 = cell |

Order within one tick: the effects in active-list order, the components in component order, and in each game
component the actions in the order of §9.3. All posts are queued, so their receivers run after the tick.

## 13. Persistence

Nothing. The manager is not in the city's record, effects are not saved, and the generators of §14 are not saved.
After a load, a volcano or effect disaster that was running is not running (disasters.md §14 already says the layer
saves nothing).

## 14. Random numbers *[compat]*

| Draw | Generator | When |
|---|---|---|
| select group `RandomUint32Uniform(0x10000)` | the manager's `cRZRandomSuper` at `+0x1C0`, seed `0x1105` | once per select group per component build (§5.3) |
| brush strength, then radius | static `cRZRandom` `0x00B22BE8` (`RandomDoubleRangeUniform`, two per brush component) | when a brush component is created (§8.2) |
| volcano random start | a throwaway `cRZRandom(−1)` | §10.1 |

- Particles, decals and dynamic particles use their own static generators (foundations §6.3). These are render only.
- **Creating** an effect makes these draws, whether or not it is ever started. The menu tool creates `N_control`
  when the tool is selected, so selecting the volcano tool makes one select draw (and two brush draws for the chosen
  creator) before any volcano exists.
- The select generator's seed is a constant. A fresh city's first volcano therefore picks a predictable creator if
  no other select group has drawn since the manager was built (§17 test 3).

## 15. Quirks

1. **Effects finish only when the city is drawn** (§7). The volcano's state 7 and the effect disasters' end both
   depend on it. A harness that ticks the simulation but never runs the 3D view's draw pass keeps the
   disaster, and therefore the emergency pause, forever. **This is now proven, not inferred** (re-integrate1;
   EF2 can be closed): `0x005BFD70` has exactly one reference in the whole decomp, in
   `cSC4EffectsManager::DeactivateUnusedVisualEffects` `0x005922A0` (`cSC4EffectsManager.c:1987`); that in turn
   is referenced only from `cSC4EffectsManager::Update(SC4DrawContext*)` `0x00593120` (`:2993`), which has no
   direct call sites and lives in a vtable slot at `0x00A9F25C`. The end-message post exists in exactly one
   place, `0x005BFD70`. So a session that never runs the draw pass never runs the finish test, never finishes an
   effect and never posts an end message. One residual *[inferred]*: that `0x00593120` is reached only from the
   3D view's draw pass — a caller-side fact outside the effects manager — but it does not change the conclusion,
   because the finish test is unreachable by any other route regardless. Test 10 asserts this.
2. An effect whose name exists but whose priorities are all above the detail setting is created with index −1. It
   does nothing and never ends (§4.1).
3. Effects keep running while the game is paused, with dilation 1 (§6).
4. The brush effect does not advance `prev` when either end is outside the city (§8.3).
5. A `single` game record with a pause flag would leave the simulator paused (§9.2). No shipped record does this.
6. The toxic amount of a game record is ignored (§9.3).
7. Creating an effect consumes random draws even when it is never started (§14).
8. Damage could depend on the zoom through the level-of-detail ranges (§5.2). **Amended (re-integrate1):** 840
   of 3373 shipped components are *not* `[1, 6)`, so components do come and go with the zoom. What the shipped
   data still supports is the narrower claim that everything reaching a brush or game record is `[1, 6)` — that
   is the invariant worth asserting in tests.
9. **Particle components are advanced only in the draw pass** (§7, re-integrate1). `cSC4ParticlesEffect`'s
   component vtable is `0x00AA0B88`; slot 4 `+0x10` is `0x00648F10`, an **empty stub**, while slot 9 `+0x24` is
   `0x005B8540` `Update(float, SC4DrawContext*)`, the draw-time update. So particles never advance, never die
   and never clear `started` outside the draw pass — which is why quirk 1 bites twice over. Note also that
   `FUN_005BEA10` is the **`+4`-subobject** component driver (`this` = `object+4`), so its `+0x110`/`+0x114`
   reads are the effect's `+0x114`/`+0x118` component array — there are not two arrays.

### 15.1 MATCHES / DIFFERS / PARTLY (re-integrate1)

- **MATCHES** — the finish test's rules 1–3 of §7 (`0x005BFD70` returns 1 = done: `started == 0` or `index < 0`
  at once with no message; `flags & 2` means never auto-stop; any non-ignored component still `IsActive` blocks
  the finish; `flags & 0x80` without a soft stop blocks it; otherwise `Stop(1)` then a **queued** end message
  through `0x00B43CCC`); §7's "Game components never do [keep an effect alive]" (measured 56 of 56);
  §10.1's `volcano_control` composition (nine meta creators in one select group plus one attractor); §10.2's
  guess at the slave naming rule, now proven and generalised to five suffixes; §3's `flags & 3` meaning.
- **DIFFERS** — three, each corrected in place above: (1) **§3** — the record tables give *in-memory* sizes
  (`0x38`, `0x6C`); on disk the component record is **variable-length** and nested inside the effect record;
  (2) **§5.2** — "every component has the range `[1, 6)`" is false for **840 of 3373**; (3) **§10.2** — "data1 =
  the instance" holds only for slaves reached as meta children, not for particle-spawned ones (§16 EF6).
- **PARTLY** — §7's "An effect's lifetime also depends on its particles and decals (flag 0)" is now *measured
  fact* (both carry `flags = 0` and both count), but the mechanism is asymmetric: decals self-stop, particles
  have no self-stop path at all (§16 EF1). And the earlier reasoning that particles *must* be ignored is
  superseded by the measurement — both readings are recorded in §7 rather than one being dropped.

## 16. Open questions

- **EF1.** The exact finish time of `volcano_control` and of the disaster control effects. It depends on the
  particle and decal components' `IsActive`, which are render objects. A runtime trace of `0x005BFD70` on a
  volcano would settle it.

  **Narrowed (re-integrate1).** Brush and decal end times are known; decals self-stop when their fade product
  passes 1.0 and their mode byte `+0x15 == 2` (`Update` `0x00582920`). **Particles are the sole unknown**, and
  the question is now sharp: *do `cSC4ParticlesEffect` components ever report inactive, and by what path?* The
  bytes show no self-stop — `cSCParticleSimulatorBase::started` (`+0x05`) is written only by `Start` `0x005B9CF0`
  and `Stop` `0x005B9E70`, and `CheckForStop` `0x005B9D40` fires only when a stop was already requested
  (`delayedStop`) and the live-particle count `+0x14` hits 0 — yet the shipped data gives all 1921 particle
  components `flags = 0`, so the finish test waits on them. **Probe:** one runtime trace of `0x005BFD70` on a
  live `volcanocreator*`, logging per component `comp+0x40` and each `IsActive()` return. That also gives the
  real wall-clock end of `volcano_control`. Static alternative: disassemble the emission path around
  `cSC4ParticlesEffect::CreateParticle` `0x005B7C20` and look for a terminal `Stop(0)` when the emitter is
  exhausted. Static bound on the end time: ≥ the child's brush length (2–4 dilated effect-seconds) plus one
  draw-pass sweep.
- ~~**EF2.** Confirm at runtime that a volcano never reaches state 7 when the city view does not draw (quirk 1).~~
  **CLOSED (re-integrate1), statically.** `0x005BFD70` is reachable only from `DeactivateUnusedVisualEffects`
  `0x005922A0`, which is reachable only from the draw-pass override `0x00593120` (vtable `0x00A9F25C`); the end
  message is posted in exactly one place. Quirk 1 is proven. Consequence: the volcano instance stuck in state
  5/7 and the effect-disaster emergency pause **never lift** in a session that only ticks.
- ~~**EF3.** The path by which particle systems spawn the "slave" effects, and whether the parameter 13
  forwarded by the effect-disaster instance reaches them.~~ **ANSWERED (re-integrate1)** — §10.2 has the suffix
  table, the callers and the measured census (143 / 33 / 20 / 21 / 3 / 6). Parameter 13 reaches **game
  components only**; particle `SetVoidParams` `0x005B2200` accepts only index 7 and decals only index 6. The
  shipped inventory is enumerated. **One sub-question is now open in its own right — EF6 below.**
- **EF4.** The meaning of visual effect flag bits 0, 3, 6 and 8. An effect created while the draw pass sweeps
  (`+0x1B4`, §5.1) stays on the new list until the next activation. When that next activation happens in a session
  that only ticks is untested.
- **EF5.** Whether a hidden pause counts as "paused" for the time dilation, which would make the 5 s and 15 s pauses
  of §9.4 real-time seconds.
- **EF6.** What `data1` is on a `kSC4MessageEffectLocation` (`0x2A7B582E`) posted from a game record inside a
  **particle-spawned** slave (§10.2). Statically it is the game component's `data` field (`+0x58`), which for
  such a slave is the particle's `+0x26C` value and **not** the disaster instance — so **§10.2's earlier
  "data1 = the instance" is wrong for that case**. Either the location update matches on something other than
  `data1 == instance`, or those slaves are reached as meta children in some effects. **Probe:** break on
  `0x005BD3C0` when `+0x30 == 0x2A7B582E` and compare `data1` with the instance pointer; if they match, there
  is a parameter-13 path not yet found, or `cSC4DisasterInstanceEffect::DoMessage` `0x00576E70` matches on
  something else.
- **EF7.** The exact semantics of `cSC4ModelBasedParticlesEffect +0x26B / +0x26C` and `+0x26F / +0x270` (§10.2).
  Ghidra splits them into five byte fields, and there is no `SetVoidParams` on that class to populate them from a
  parent's parameter 13. **Probe:** the class layout from `0x005AEC90` in full, or a runtime watch.
- **EF8.** Who posts and who consumes `kSC4MessageTornadoDead` `0x0A36F1E8` and
  `kSC4MessageMissionEffectStarted` `0x5F900DCC` — both are in the message name table `0x00B08360`–`0x00B086F8`
  and neither is traced. **Probe:** xref on each constant.
- **EF9.** Where the message names for `0x86AD10EE` and `0x697D01ED` live — they are **not** in the `0x00B08360`
  table (that table holds 115 `{u32 id, const char* name}` pairs). **Probe:** scan for another name table.
- **EF10.** Whether "every component that reaches a brush or game record is `[1, 6)`" holds per effect (§5.2).
  **Probe:** the assertion already made by `engine/formats/tests/effects_directory.rs`.

## 17. Conformance tests (must exist in `engine/conformance` if the manager or the brush effect is replaced)

1. **Brush stroke count.** Brush record 2 (rate 30, length 3.0, stroke), ticks of 33,333 µs, dilation 1:
   - `dt' = 33333`, `dt = r24(0.033333) = 0.03333300`;
   - the per-tick `n` is 0, 1, 1, 1, …;
   - the stroke ends after **91 ticks** with **90 applications** (`t = 3.0333`, the leftover `acc = 0.99909`).
2. **Dilation.** The same at dilation 1.5:
   - `dt' = trunc(49999.5) = 49999`;
   - `n` = 1, 1, 2, 1, 2, …;
   - 61 ticks, **91 applications**.

   At 100,000 µs and dilation 1, each tick applies exactly 3, for 31 ticks (93 applications).
3. **Select draw.** A fresh manager (seed `0x1105`, foundations §6.1 `cRZRandomSuper`):
   - the first tempered word is `0xAAE64AC3`, so `r = 43750`;
   - the running sums 7281, 14562, … first reach ≥ 43750 at the 7th member;
   - the first `volcano_control` built by that manager therefore uses **`volcanocreator4a`** (brush record 8:
     rate 65, length 3.0, radius scale 1.0).

   The next five draws give `r` = 24188, 47502, 45072, 37594 and 53135, which pick members 4, 7, 7, 6 and 8 (1-based).
4. **One-shot crater.** `meteorcrater_b` (brush record 12): its first update calls `ApplyBrush(0x8A33E17A)` **twice**
   from `prev` to `cur`:
   - level 3, strength 6, radius 0.7, `data4 = 3`, target 100;
   - redisplay off then on;
   - then the brush is inactive, with no long terrain operation.
5. **Out of bounds.** A stroke whose `prev` is outside the city applies nothing, and keeps the old `prev` until both
   ends are inside.
6. **Game record order.** Game record 17 at cell (40, 40) in a 64 × 64 city, in this order:
   1. `IgniteCellRadius(40, 40, 0, 14)`;
   2. `SetCause(4)`;
   3. `DemolishRegion` over cells [30, 50] × [30, 50] with flags 2 and dezone false;
   4. no message.

   At cell (2, 60), the rectangle is clipped to [0, 12] × [50, 63].
7. **Move to a new cell.** A non-single game record applies on its first update. Moved within the same cell it does
   nothing. Moved to the next cell it applies again (`count = 2`).
8. **Pause record.** Game record 3:
   1. `Start` → `HiddenPause` once.
   2. Updates with `dt = 1.0`: `timer` = 1, …, 5 → nothing; at `timer = 6 > 5` → `Stop(0)` → `HiddenResume` once.
   3. A second `Stop` does nothing.
9. **Priority filter.** Two descriptions named `x`, with priorities 1 and 3. With max priority 2, `x` resolves to the
   priority-1 one. With max priority 0, `CreateVisualEffect("x")` returns true with index −1, and the effect never
   posts its end message.
10. **No end without the draw pass.** Start `volcano_control` with an end message, then tick past its brush length
    without calling the draw pass. No end message may be posted.
11. **The finish rule** (§7, re-integrate1). Effect E finishes at the first draw-pass sweep at which `E.started`
    is true and `E.index ≥ 0`, `E.flags & 2` is clear, every runtime component `c` with `(c.flags & 4) == 0` has
    `c.IsActive()` false, and `E.flags & 0x80` is clear or `E.stopping` is true. At that moment `Stop(1)` is
    called and, if `endMessageType != 0`, one **queued** `cRZMessage2Standard` is posted with
    `type = endMessageType`, `data1 = endMessageData1`. `E.started` then becomes 0, so it never posts twice. A
    done effect with refcount 1 is recycled (`name`/IDs reset, `index = −1`); one still referenced stays on the
    active list.
12. **The description bit is not the tested bit** (§7, re-integrate1). A component with `desc+0x10 & 1` set is
    built with runtime `comp+0x40` bit 2 (value 4) set and is skipped by the finish test; one with it clear is
    built with bit 2 clear and blocks the finish while it is active. Table types 3–15 additionally get runtime
    bit 1 (value 2).
13. **Shipped flag census** (§7, re-integrate1). Parsing the shipped effect directory gives `flags = 0` on all
    1821 + 100 particle, 406 decal, 591 meta, 25 brush, 10 sequence and 3 model components, and `flags = 1` on
    all 64 attractor, 56 game, 41 camera and 299 sound components (4 sound components have `flags = 0`).
14. **LOD ranges are not uniform** (§5.2, re-integrate1). The shipped directory has 3373 components of which
    2533 are `[1, 6)` and 840 are not. Every component that reaches a brush or game record, directly or through
    meta, must still be `[1, 6)`.
15. **`volcano_control` end condition** (§10.1, re-integrate1). The effect finishes only after the chosen
    `volcanocreator*` child has finished on a previous sweep **and** that child's brush, particle and decal
    components are all inactive on this one. The attractor and the game record never hold it.
