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

**The rebuild test (EF12, re-effects-contracts4).** There is no per-effect "already built" flag; the test is per
component description and compares the current level of detail `L` (`+0xC84`) with the previous one `L_prev`
(`+0xC88`; `DoZoomChanged` copies the old `L` there before computing the new one, `L = max(1, zoom − base + 1)`
with `base` at `+0xC8C`; all three start at −1 in the constructor). `UpdateVisualEffect(effect, newBuild)`
`0x005919D0` does two passes:

1. **Existing components** (records at effect `+0x114`, `0x44` bytes each): a record survives only if the reload
   flag `+0xB8C` is 0 and `L` is in its description's `[min, max)` (`+0x4C`, `+0x4D`); a survivor is refreshed
   (`0x0058DC50`) when `L != L_prev`. Every other record is hard-stopped (`Stop(1)`) and released; the list is
   compacted in order.
2. **Descriptions**, in order. A description with a valid type index (`+0x68 ≥ 0`) takes part — draws and adds its
   weight — only if **(in range at `L` and not in range at `L_prev`) or `newBuild` or the reload flag** is set
   (`0x00591C08`–`0x00591C32`). Otherwise it is skipped without touching the group state. A participating member
   creates a component only if it is in range at `L` (`0x00591C9B`).

So a rebuild with `newBuild = 0` and no reload draws for a select group only if one of its members **just entered
the range**, and the draw and running sum then cover only those entering members (members not entering are
skipped and do not count as "a different group"). A reload (`+0xB8C` = 1) destroys every component in pass 1 and
redraws every group. Consequences an implementer must keep:
- The new-build and reload draws include members that are **out of range at `L`**: their weight still counts, and if
  the draw lands on one, nothing in that group is created.
- The group comparison is against the last *participating* component's group, and a group-0 component resets it.
- The pick is "first running sum ≥ r" with an unsigned compare (`0x00591C84`); after a pick `r := 0x10000`, so if a
  group's weights sum past `0xFFFF` a second, later member is also created.

## 6. The tick (`DoAnimationTick` `0x0058E580`) *[compat]*

On `kSC4MessageAnimationTick` `0x86AD10EE` (data2 = elapsed microseconds `µs`):

1. **Dilate the time step.**
   - `dt' := trunc(r24(µs × dilation))`, where `dilation` is `cISC4Simulator::GetAnimationTimeDilation` (slot 32;
     simulator.md §7) and `µs` is taken as unsigned.
   - With no simulator, `dt' := µs`.
   - **Shipped dilation, measured from the bytes (`0x007026F0`, re-answers34):** the simulator keeps a five-float
     table at the pointer `+0x108`, filled by `ReadSimulatorTunableValues` `0x007025B0`:
     `[0] = 0.0, [1] = 1.0, [2] = 1.5, [3] = 2.0, [4] = 1.0` (the constants `0x702686`–`0x7026B1`). The getter
     consults, in order: `IsEmergencyPaused` (slot 17, `0x00703D70`) → `[4]`; the automata manager's
     `IsDrivingMode` (U-Drive-It active; slot 13 of the object cached at `0x00B43D44`, `0x00506940`) → `[4]`;
     `IsPaused` (slot 15, `0x00703D50`) → **`[0]` = 0.0**; otherwise by `GetSimSpeed` `0x008265C0`:
     speed 0 → `[1]` = 1.0, speed 1 → `[2]` = 1.5, speed 2 → `[3]` = 2.0.
     **Speed outside 0–2 (EF11, re-effects-contracts4):** `GetSimSpeed` is the raw `u32` at the simulator's
     `+0x3C` (`0x008265C0`), and the getter tests only `speed − 1 == 0` → `[2]` and `speed − 2 == 0` → `[3]`
     (`0x00702727`–`0x0070272B`); **every other value — 0, 3 and above, or anything negative — falls through to
     `[1]` = 1.0** (`0x0070272D`). No clamp, no arithmetic on the speed.
     **`IsHiddenPaused` (slot 16) is never consulted** — a hidden pause does not change the dilation (EF5).
   - *[quirk]* While paused the dilation is 0.0, so `dt'` is 0 and every component update runs with `dt = 0` —
     brushes make no progress, timers do not advance. (The earlier claim "1 while paused" is wrong; in a normal
     session the tick also does not arrive, because the animation-tick manager is paused, see §6.1.)
2. Add `dt'` to the total (`+0x1B8`) and store the total in seconds at `+0x1BC`.
3. For each effect on the active list, in list order, that **is started** (slot 5):
   1. If its flag bit 5 is set, give it the pick ray as parameter 9 (render).
   2. Call its update with `dt = r24(dt' × 1e-6)` seconds (the `double` constant `0x00A9F190`).
   3. The effect updates each of its components, in component order, **whose `IsActive` is true** (`0x005BEA10`).

A brush or game component therefore acts only inside this tick, and only while both it and its effect are active.

### 6.1 When the tick arrives at all *[compat]*

`kSC4MessageAnimationTick` is sent by `cSC4AnimationTickManager::DoAnimationTick` (`0x004487F0`) to each
tickable object. The manager keeps a pause counter (`+0x84`, `PauseAnimationSystem` `0x00447EE0`); while it is
non-zero the per-target sweep is skipped (`0x004488C0`–`0x004488D6`), so no ticks are delivered. Separately the
effects manager registers a callback with the tick manager in `Init` (`0x00594EEE`:
`EnableAutoStep(1, EffectsTickCallback)` on the `cISC4AnimationTickManager` at `+0xCD8`), and
`EffectsTickCallback` `0x0058D9A0` posts `0x697D01ED` (`DoEffectsTick`, §12) through the message server every
time the animation tick manager steps. So the effects manager's own tick runs **once per animation tick in any
session that ticks**, not only in the draw pass. The tick manager's own `DoAnimationTick` is driven from outside the
effects subsystem (the framework scheduler), which is why a paused game stops it and a ticking harness keeps it.

**Corrects (EF12, re-effects-contracts4): `DoEffectsTick` `0x00593D00` does not rebuild or activate anything on
an ordinary tick.** Its body, in order:

1. **Reload, only when pending.** If the object at `+0xC78` exists and its slot 14 (`+0x38`) returns true
   (`0x00593D12`), it locks (`+0x94` slot 3), runs `EffectsParser::ParseQueuedFiles` `0x0058E4E0`, unlocks
   (slot 4), sets the reload flag `+0xB8C` := 1, calls `UpdateActiveVisualEffects`, clears `+0xB8C`, and calls
   slot 2 of `+0xC7C`. Otherwise this step is skipped entirely.
2. **Snow line.** If `+0xB88` exists, its slot 22 (`+0x58`) fills a value stored at `+0xCB0` (`+0xCA8`, `+0xCAC`
   with it); on 25 December (local date, `Month == 12`, `DayOfMonth == 25`) with a terrain view it resolves
   `+0xDBC` once and calls `UpdateSnowline`.

New effects therefore are **not** activated by the tick: `ActivateNewVisualEffects` runs from
`CreateVisualEffect` (both overloads, `0x00593AB0` and `0x00593B68`) when the description was found, from the end of
`DeactivateUnusedVisualEffects` (`0x00592A01`, the draw-pass sweep) and from `UpdateActiveVisualEffects`.

**`UpdateActiveVisualEffects` `0x00593B90`** does nothing at all — not even activation — unless the byte
`+0x1AD` is set (`0x00593B93`). That byte is 0 from the constructor (`0x0059131B`) and `Init` (`0x00594A57`), set
to 1 only by `DoMessage` on `kSC4MessagePostAppServicesInitComplete` `0xAB96B05F` (`0x0059458A`, just before its
own `UpdateActiveVisualEffects`), and cleared by `Shutdown` (`0x00595828`). Once set it walks the active list,
calls `UpdateVisualEffect(effect, 0)` on each, then `ActivateNewVisualEffects`. Its callers are that message,
`DoZoomChanged`, the reload of step 1, and `DoRenderPropertiesChanged` (also with `+0xB8C` = 1, when the
cached pointer `+0xB90` changes, `0x00593E10+`).

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

**The finish test is the only place an end message is posted, and it is reachable only from the draw pass
(EF2, settled 2026-10-16, re-answers29).** The call chain has one route and no others:

- `0x005BFD70` (the finish test, name it `CheckFinished`) is called from exactly one place,
  `DeactivateUnusedVisualEffects` `0x005922A0`;
- `0x005922A0` is called from exactly one place, `Update(SC4DrawContext*)` `0x00593120`;
- `0x00593120` has **no direct callers**: it is dispatched as a vtable slot at `0x00A9F25C`, the draw-pass
  override next to the `cISC4EffectsManager` vtable `0x00A9F264`.

**Corrects / refines (re-specs21, [view3d.md](view3d.md) §8):** the chain above is right at function granularity;
the *call sites* are `0x00593985` (`DeactivateUnusedVisualEffects`), `0x0059395B` (`DrawScreenFlashes`) and
`0x0059397D` (`0x0058EC40`), all inside `Update`. `vftable_1` is installed at **`manager + 0x04`** (destructor
`0x0058FA60`: `mov [esi+4], 0xA9F250`), so `Update`'s `this` is `manager + 4` and it recovers the manager with
`lea ecx, [esi - 4]` (`0x00593958`, `0x0059397A`, `0x00593982`). **DIFFERS** on the return channel: `Update` ends
`mov al, 1` (`0x00593997`) then `ret 4` (`0x0059399D`), so it returns **`al` = true**, not `eax-or-void`. Its one
argument is an `SC4DrawContext*`. Whether the view's paint reaches it through `cSC43DRender`'s dynamic pass is
still *[inferred]* (view3d.md §8, open item V3D-1).

So a session that never draws the city never runs the finish test, never finishes an effect and never posts an
end message. **Quirk 1 of §15 is proven, not inferred.** Consequence for the disasters: a volcano instance that
never sees a draw-pass sweep stays in state 5 or 7 and its effect-disaster emergency pause never lifts. A
replacement that must work without a 3D view has to run the same sweep from somewhere else, or synthesise the
end message.

**Which components decide the finish (EF1, 2026-10-16).** The test walks the effect's **runtime** component
records (0x44 bytes each, built by `UpdateVisualEffect` `0x005919D0` and `CreateComponentFromDescription`
`0x00591730`) and asks each one's `IsActive` (component vtable slot 2). The bit that says "ignore me in the
finish test" is:

> bit 0 of the **description** component's flags word (`desc + 0x10`), copied by `0x00591730` into **bit 2
> (value 4) of the runtime record's flags at `+0x40`**. `0x005BFD70` tests the runtime bit.

Table component types 3–15 additionally set runtime bit 1 (value 2). An implementer must test runtime
`flags & 4`, not the description bit directly.

`IsActive` per component kind:

| Kind | `IsActive` returns | Becomes false when |
|---|---|---|
| brush (`0x00A9EDD0`) | the byte at `+0x24` | a one-shot on its first update (`0x005817F0`); a stroke when `t > length`; `Stop` `0x00581530` (also ends the long terrain operation) |
| game (`0x00AA0FF8`) | the byte at `+0x25` | `Stop` `0x005BD340`; internally when `timer > duration`; a `single` record after its one `Apply` |
| particles | the `started` byte at `+0x05` | **soft** `Stop(0)`: delayed — the particles play out and `CheckForStop` `0x005B9D40` clears `started` when the live count `+0x14` reaches 0. **hard** `Stop(1)`/`Stop(2)`: immediately, particles destroyed. The emitter also **self-stops** when its emit budget is exhausted (§7.1) |
| decal | the byte at `+0x08` | `Stop` `0x00581FF0`, **or self-stop** when its fade timer passes 1.0 and its mode byte `+0x15 == 2` (`Update` `0x00582920`) |
| meta (nested effect) | the **child effect's `started`** byte at `+0x24` | when the draw pass finishes the child |

**Corrects (re-specs20, audio.md §9.2):** this table has no **sound** row. A sound component's `IsActive` is
the started byte at `+0x1C`, returned by the shared `0x00580B50`; it becomes false only when `Stop`
(`0x005BE4A0`) is called, which is either the effect's `Stop` or the component's own `Update` self-stop at
`elapsed > desc+0x18` when that length is non-zero. So **a sound component with `length == 0` that is not
ignored pins its effect forever**. Add the row. **PARTLY** — the five rows present are correct.

**Component type numbers and which of them count (from the shipped effect directory, 2026-10-16).** The
description's `component_type` `u8` selects a description table, and `description_index` indexes it. Bit 0 of the
component's `flags` word is the ignore bit (§7 above). Across all 1154 shipped effect descriptions:

| Type | Description table | Records | Named | Ignore bit set |
|---|---|---|---|---|
| 0 | particles (1192) | 1921 | yes | **never** — always counts |
| 1 | decals (310) | 406 | yes | **never** — always counts |
| 2 | meta: a nested effect by name | 591 | yes | **never** — always counts |
| 3 | brushes (25) | 25 | no | **never** — always counts |
| 4 | attractors (64) | 64 | no | **always** |
| 5 | scrubbers / game records (57) | 56 | no | **always** |
| 6 | sequences (15) | 10 | yes | **never** — always counts |
| 7 | sounds (305) | 303 | no | 299 of 303 (four exceptions) |
| 8 | cameras (48) | 41 | no | **always** |
| 16 | (unnamed; seen as `p_vehicle`, `p_train`, `p_train_toxic`) | 3 | yes | never |

So the rule the shipped data follows is simple: **brushes, particles, decals, meta and sequences decide when an
effect finishes; attractors, game records, cameras and almost all sounds never do.** The earlier reading that
"all shipped game components carry flag 1" is confirmed, but the guess that the particle components must be
ignored is **wrong** — see below.

**Particles do not need to be ignored, because they are finite.** Each particle description carries `life`, an
`emit_loop_count` and an `emit_loop_interval`; every shipped volcano particle has **`emit_loop_count = 1`**, so
the emitter runs one loop and the particles die on their `life` timer. The `started` byte is only cleared by
`Start` and `Stop`, but a finite emitter still reaches "no particles left" and its `CheckForStop` path.

**Consequences for `volcano_control` (EF1, now fully bounded from the data).** Each of the nine
`volcanocreator*` children holds, and finishes when the **last** of these is inactive:

| Component | Count? | Lifetime in the shipped data |
|---|---|---|
| one brush stroke (table index 2–10) | yes | `t > length`, 2–4 effect-seconds |
| four sounds | no (ignore bit) | — |
| `volcano_smoke_small/medium/large` particles | **yes** | `life` 3–9 s, `emit_loop_count` 1 |
| `volcano_fallout`, `volcano_fallout_up`, `volcano_fallout_wide` particles | **yes** | `life` 10–20 s, `emit_loop_count` 1 |
| `glowing_lava_*` and `glowing_lava_*_water` decals | **yes** | `life` **45 s** |
| `craterdecal_small/medium/large` | **yes** | `life` **50 s** |
| one game record (table index 9–17) | no (ignore bit) | — |

The binding constraint is therefore the **crater decal's 50 s**, not the brush stroke. The parent
`volcano_control` has nine meta components (all counting) plus one attractor (ignored), and a meta reports its
child's `started` byte — so the parent finishes one **full extra draw-pass sweep** after the child. In short:

> `volcano_control` finishes at the first draw-pass sweep at least **50 effect-seconds** after the chosen
> `volcanocreator*` child started (the crater decal's `life`), plus one further sweep for the meta parent.
> The brush stroke (2–4 s) and the particles (≤ 20 s) all end well before that.

The units are the effect-seconds of §6 (the tick's `dt`), so they are dilated like everything else.

**The rule an implementer needs.** Effect `E` finishes at the first draw-pass sweep (`0x005922A0`) at which all
of these hold:

1. `E.started` and `E.index ≥ 0` (otherwise it is "done" with no message);
2. `E.flags & 2` (bit 1, no auto-stop) is clear;
3. for every runtime component `c` with `(c.flags & 4) == 0`, `c.IsActive()` is false;
4. `E.flags & 0x80` (bit 7) is clear, **or** `E.stopping` is set.

At that moment the manager calls `E.Stop(1)`, then — when `E.endMessageType` is not 0 — posts a **queued**
standard message with `type = endMessageType`, `data1 = endMessageData1`. `E.started` then becomes 0, so the
message cannot be posted twice.

Object offsets used above (from the destructor `0x005C0550`, Start `0x005BE920`, Stop `0x005BEBA0`, TestFlag
`0x005BFC90` and the builder `0x005919D0`): `+0x08` reference count, `+0x20` description index (−1 = none),
`+0x24` `started`, `+0x25` `stopping`, `+0x28` flags, `+0x2C` / `+0x30` end message type and data1,
`+0x114` / `+0x118` the runtime component array begin and end (stride 0x44), `+0x148` the "built" gate that
makes `Start` and `Stop` no-ops when it is 0.

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
components. **The measurement stands.** The tension is resolved by §7.1: the particle emitter *does* self-stop,
from inside the draw-time emit path. See §16 EF1.

### 7.1 The particle emitter's self-stop (EF1, re-answers34) *[compat]*

The bytes alone settle EF1 without a runtime trace. Three facts, all from instruction bytes:

1. **The delayed-stop protocol.** `cSCParticleSimulatorBase::Stop` `0x005B9E70` takes two mode bytes. With
   `started` set: `+0x0C` := mode1, and if mode1 == 0 the stop is **immediate** (`started` := 0, `+0x0D` := 0,
   every particle destroyed, the list cleared); if mode1 != 0 the stop is **delayed** (only `+0x0C` is set,
   particles keep running). `cSC4ParticlesEffect::Stop` `0x005B21E0` stores its mode at `+0x18C` and calls the
   base with `(mode == 0, 0)` — so **the component's `Stop(0)` is the delayed one and `Stop(1)` destroys at
   once** (the reverse of the naming one might expect). `CheckForStop` `0x005B9D40` (called at the end of
   every draw-time update `0x005B8540+0x23E`) returns true only when the live count `+0x14` is 0 *and* a
   delayed stop was requested (`+0x0C` != 0); it then calls base `Stop(0,0)` — the immediate form — which
   clears `started`, so `IsActive` goes false.
2. **The emitter self-stops through the same `Stop(0)`.** The emitter paths are reached from the draw-time
   update (`0x005B8540` slot 9 → slot 15 of the component's main vtable). Slot 15 (vtable `+0x3C`) differs by class
   (vtable words `0x00AA0EDC`, `0x00AA0C94`, `0x00AA0BC4`; corrected 2026-10-21 by re-particles4, particles.md §8):
   - `cSC4ParticlesEffect` (main vtable `0x00AA0EA0`) and `cSC4ModelBasedParticlesEffect` (`0x00AA0C58`,
     inherited) → `0x005B96E0`: after the loop-interval countdown `+0x190` (particles.md §8.1), the
     description's flags A (`+0x00`) pick one of three emitters — `0x04` (`maintain`) → `0x005B3CF0` (a plain
     rate-integral emitter with **no** self-stop), otherwise `0x02` (`inject`) → `0x005B8B50` (loop-budget
     emitter: `+0x128` counts loops, compared with the description's `emit_loop_count` at `+0x18`; when the
     budget is exhausted it calls `Stop(0)` at `0x005B8D67`), else `0x005B87B0` (the rate emitter: when
     its loop counter `+0x128` reaches `emit_loop_count` it calls `Stop(0)` at `0x005B8B29`, unless flag A
     `0x08` holds it at the last loop; there is no separate seconds budget, §7.1.1).
   - `cSC4GeometrySourcedParticlesEffect` (main vtable `0x00AA0B88`, installed by `0x005A8ED0`) → `0x005AA5D0`:
     when its emit state says the last loop is done (its `+0x18` count is 0) it calls `Stop(0)` (main slot 1,
     argument 0) on itself at `0x005AA6D4`/`0x005AA6EB`.
   The `+4` simulator vtable `0x00AA0E58` plays no part (its slot 15 is `CreateParticle` `0x005B7C20`).
3. **The budget fields.** `emit_loop_count` is the particle description's `+0x18` (`u32`); the loop-interval
   countdown is the effect's `+0x190`, seeded in `Start` `0x005B3000` from the description's `+0x20..+0x2C`
   range. So every shipped volcano particle (all `emit_loop_count = 1`, `life` 3–20 s) emits one loop and
   then delays its stop; the last particle death clears `started`; the finish test then passes.

#### 7.1.1 The emitter contract (EF13, re-effects-contracts4) *[compat]*

Offsets are on the particle effect (main object); `D` is its description (`+0x24`). Fields: phase `P` (`+0x120`,
`double`, in loops), loop counter `n` (`+0x128`, `u32`), rate `ρ` (`+0x130`, `double`, loops per second),
fractional-particle carry `c` (`+0x138`, `float`), countdown `w` (`+0x190`, `float`, seconds), step `dt`
(`+0x108`, `float`). Description fields: flags A `D+0x00`, period `D+0x14` (seconds per loop), max burst age
`D+0x10`, loop count `D+0x18` (0 = unlimited), pre-roll `D+0x1C`, start-delay range `D+0x20..+0x24`, interval
range `D+0x28..+0x2C`, count curve `D+0x88..+0x8C` (a `float` vector).

- **`SetDescription`** `0x005B7C90`: `ρ := 1.0 / D+0x14`, with `D+0x14` replaced by `1e-6` unless it is greater than
  `1e-6` (`0x005B7CF3`–`0x005B7D44`); clears the "emission suppressed" byte `+0x21A`.
- **`Start(arg)`** `0x005B3000`: if `arg != 0`, `D+0x18 > 0` and flag A bit 3 (`0x08`) is clear, it returns
  without starting (a finite emitter is never restarted by the zoom path's `Start(1)`). Otherwise, after the base
  start: `P := 0`, `n := 0`, `c := 0`, total age `+0x110 := 0`; then `w :=` uniform `double` in `[D+0x20, D+0x24]`
  if `D+0x20 ≥ 0`, else uniform in `[D+0x28, D+0x2C]` if `D+0x28 ≥ 0`, else `0` — both draws from the global
  `cRZRandom` at `0x00B22DF8` (`RandomDoubleRangeUniform` `0x00913761`, `0x005B3399`–`0x005B33FD`), stored as
  `float`. Last: `+0x188 := arg`, `+0x18C := 0`, pre-roll pending `+0x118 := (arg != 0)`.
- **Draw-time update** `0x005B8540` (only while started): `t := dt`; if pre-roll is pending, `t += D+0x1C` once
  and the flag clears; `dt := t` (stored), total age `+= t`. Then **every live particle is aged first**: each
  particle whose alive byte is set is updated by main slot 20 (`+0x50`) with `dt`; a dead one or one whose update
  returns false is unlinked, the live count decremented and the particle destroyed (`0x005B86A5`–`0x005B86F8`).
  **Then**, unless `+0x21A` is set, the emitter dispatch (main slot 15) runs (`0x005B870A`); new particles are
  never aged by the same step's slot-20 pass (they get their own initial age, below). `CheckForStop` comes last.
- **Dispatch** `0x005B96E0`: nothing if a delayed stop was requested (`+0x10`, the simulator base's `+0x0C`;
  `0x005B96E1`) — **(d): no emission after `Stop(0)`**, the existing particles just run out. If `w > 0`:
  `w −= dt`; if still `> 0` return; else the emit time is `e := −w` (the overshoot only). If `w ≤ 0` on entry,
  `e := dt` (and `w` stays where it is; only the emitters reseed it). If `D+0x1C > 0` and `e > D+0x1C − 0.01`
  (`double`), `e := D+0x1C − 0.01`. Then by flags A: `0x04` → `0x005B3CF0`, else `0x02` → loop-budget, else the
  rate emitter.

**Loop-budget emitter** (`inject`, `0x005B8B50`), one **burst per loop**:
1. `P := P + e·ρ` (in `float`, stored `double`). If `P ≤ n` return.
2. Burst size `N`: `x := curve[0]`; if `x > 20` and `D+0x08` bit 0 is clear, `x := x · m` where `m` is the product
   of the `float`s at `*(+0x1E8)`, `+0x1E0`, `+0x1C4` (detail/density scales); `N := ceil(x)` (the
   `1.5·2^23` rounding trick).
3. While `P > n`: the burst's age is `a := (P − n) · D+0x14` seconds; if `a < D+0x10` and `N ≠ 0`, create `N`
   particles, each initialised by main slot 20 with age `a` (a particle failing its init is destroyed). Then
   `n := n + 1`; **if `D+0x18 ≠ 0` and `n ≥ D+0x18`, `Stop(0)` (delayed, `0x005B8D67`) and return.** If
   `D+0x28 ≥ 0`: `P −= RandomRange(D+0x28, D+0x2C) · ρ`; if now `P < n`, `w := (n − P)·D+0x14`, `P := n`, return.
So (a): with `D+0x20 < 0` and `D+0x28 < 0` the first burst comes on the first update after `Start` (age = that
step); otherwise after the seeded `w`. (b): one burst of `N` per loop, not a rate.

**Rate emitter** (neither `0x02` nor `0x04`, `0x005B87B0`; the "duration-budget" emitter of item 2 above), sub-steps
of at most `1/15` s (`0x00AA0CF4`) until the remaining `e` is used up:
1. `s := min(e, 1/15)`; `P += s·ρ`. If `P > n + 1`: `n := n + 1`; if `D+0x18 = 0` or `n ≠ D+0x18`: when
   `D+0x28 ≥ 0`, cut `s` back to the loop boundary (`s −= (P − n)·D+0x14`), `P := n`, and mark a gap. If
   `n = D+0x18`: **flag A bit 3 clear → `Stop(0)` (delayed, `0x005B8B29`) and return; set → hold**
   (`n := D+0x18 − 1`, `P := D+0x18 − 1e-6`, emission continues at the curve's end).
2. Rate `r`: one curve entry → `curve[0]`; else linear interpolation at position `(len − 1)·(P − n)` (no clamp; edges below).
   `x := r·s·m' + c` where `m'` is the same three-scale product, further multiplied by the scale `+0xC0` once,
   squared or cubed for flags A bits 19 / 20 / 21 (the lowest set bit wins; none set → no `+0xC0` factor; details below); `N := floor(x)`, `c := x − N`.
3. Create `N` particles with initial ages `e, e − s/N, e − 2s/N, …` (oldest first).
4. `e −= s`; if a gap was marked, `e −= RandomRange(D+0x28, D+0x2C)`; if that is negative, `w := −e` and return.
So (c): the budget is **loops of `D+0x14` seconds, counted in `+0x128` against `D+0x18`** — the same counter and
field as the loop-budget emitter; there is no separate seconds budget. The count curve gives particles per second.

**Plain emitter** (`maintain`, flag A `0x04`, `0x005B3CF0`; EF14 (a), re-effects14-4): it keeps the live count at
a fixed target. It does not sub-step, never advances `n`, never wraps `P`, never reads or writes `w` or `c`,
never calls `Stop`, and never ages new particles.
1. `P := P + e·ρ` (`float` `e` times `double` `ρ` plus `P`, in x87 extended precision, stored `double`;
   `0x005B3CF0`–`0x005B3D08`). If `P ≤ n` (unsigned `n`, which stays at the 0 that `Start` wrote) return. So
   nothing happens until the first dispatch with `e > 0`. After that, every dispatch tops up, because `P` only grows.
2. Target `T := ceil(curve[0])` (the same `+0.5`, `1.5·2^23` rounding trick as the loop-budget burst, with the
   "equals `k−1`" correction, `0x005B3D30`–`0x005B3D65`). **No detail/density scale `m`, no `+0xC0` scale.**
3. `d := T − live`, where live is the simulator's live count (main `+0x18`). If `d = 0` return. Otherwise run `d`
   iterations, counted as an unsigned 32-bit number. Each one calls `CreateParticle` (simulator slot 15 `0x005B7C20`). That
   function gives the particle age 0 and a fresh life (below). The particle goes to `AddParticle`
   (simulator slot 11) if its alive byte is set, which `InitEffectsParticle` always does; otherwise it goes to
   `DestroyParticle` (slot 16). Main slot 20 is **not** called, and flag A bit 31's birth hook (`0x005B84C0`,
   used by the other two emitters) is not run either. New particles therefore start at age 0 at the emitter
   origin, and the next step's aging pass is their first update.
4. Unguarded case: if `live > T`, or if `T` is negative, `d` wraps around and the loop runs about `2^32` times. The
   loop does not stop when a creation fails. This cannot happen with shipped data unless a description with a smaller
   `curve[0]` replaces the running one. A conforming implementation creates nothing when `T ≤ live` `[choice]`.
Example: `curve[0] = 2.3`, `D+0x14 = 1`, with no start delay. On the first update with `dt = 0.05`, `P = 0.05 > 0`
and `T = 3`, so 3 particles are created. Two of them die in a later aging pass (live 1), and the next dispatch
creates 2 more.

**Emitter details** (EF14 (b)–(e), re-effects14-4; from instruction bytes):
- **Rate scale `+0xC0`** (`0x005B87BD`–`0x005B8828`): `m' := ((*(+0x1E8)) · (+0x1E0)) · (+0x1C4)`, rounded to `float`.
  The bits of flags A are tested in the order 19, 20, 21, and **the first set bit wins**: bit 19 → `m' := m' · k`, bit 20 →
  `m' := (k·k) · m'`, bit 21 → `m' := (k·k·k) · m'` (with `k = +0xC0`, each stored as `float`). If none is set, `k`
  is not applied. Only the rate emitter uses `k`. `SetDescription` resets `+0x1E0` (and `+0x1CC`, `+0x1D0`,
  `+0x1E4`) to 1.0.
- **Period floor** (`0x005B7CF3`–`0x005B7D44`): the `1e-6` is a local `float`. The code compares `1e-6 < D+0x14`
  and, on false or NaN, divides by the local instead. **`D+0x14` itself is never written.** Only `ρ` sees the
  floor. The burst age `(P − n)·D+0x14`, the gap cut-back `(P − n)·D+0x14` and `w := (n − P)·D+0x14` all use the
  raw field, so with `D+0x14 = 0` every burst age is 0.
- **Particle life and main slot 20** (`UpdateParticlePhysics` `0x005B9BA0`). `InitEffectsParticle` (`0x005B7580`,
  called from `CreateParticle`) sets alive := 1, age (`+0x04`) := 0, and life (`+0x08`) := a uniform `double` in
  `[D+0x0C, D+0x10]` from the global `cRZRandom`, stored as `float`. This is the first random draw for each particle. The life is therefore drawn per
  particle, not passed in by the caller. Slot 20 `(particle, t)` does `age := age + t` (`float`, stored first). If `age < life` is
  false (`age ≥ life`), it clears the alive byte and **returns false**. Otherwise it integrates the motion over `t`
  and returns true. The loop-budget and rate emitters "initialise with age `a`" by calling slot 20 with `t = a`
  on a fresh particle, so **init fails exactly when `a ≥ life`**. The aging pass calls it with `dt`, so a particle
  dies on the first step where its accumulated age reaches `life`. Exception: when flags A `& 0x18000D00` (the
  global mask at `0x00B22E14`, set at startup) is nonzero, slot 20 goes to `UpdateParticlePhysicsExtra`
  `0x005B9790`. That function makes the same age test first, then sub-steps the motion in pieces of at most 1/15 s, and can also return false
  through `ApplyTerrainForces` `0x005B8DC0`. That happens below the water level `+0x1FC` with kill chance `D+0x154`,
  or under the terrain with kill chance `D+0x150` (1.0 means always; otherwise the particle dies when a uniform draw ≤ the
  chance). Both cases clear the alive byte.
- **Curve lookup** (rate emitter, `0x005B891B`–`0x005B89D3`). With `L := len − 1`: if `L = 0`, `r := curve[0]`.
  Otherwise `x := (P − n)·L` (extended precision), `i := floor(x)` (the rounding trick with the "equals `k+1`"
  correction), and `f := x − i`. If `f > 0`, `r := curve[i] + f·(curve[i+1] − curve[i])`; otherwise
  (`f ≤ 0`, including NaN) `r := curve[i]`. **There is no clamp.** For `P − n` in `[0, 1]`, the only values
  the counters normally produce, this is exact (at `P − n = 1`, `f = 0` and `r = curve[L]`). `P − n > 1` is reachable
  in the original when one sub-step adds more than one loop (`ρ > 15`, period below 1/15 s, with no gap). The
  original then reads past the end of the vector, which is undefined. A conforming implementation clamps
  `P − n` to `[0, 1]` `[choice]`. **An empty curve is not guarded either.** The plain and loop-budget emitters read
  `curve[0]` unconditionally, and the rate emitter takes `L` as the unsigned `0xFFFFFFFF`. Treating an empty curve as
  0 is `[choice]`.

So `volcano_control`'s finish time stands as measured in §7: the **crater decal's 50 effect-seconds** binds,
the particles end on their own loop budget (well before 50 s), the brush stroke ends at 2–4 s, and the meta
parent needs one further draw-pass sweep. The wall-clock end depends only on the dilation of §6.

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

   So the volcano's end is triggered by the effect **finishing** (§7). The deformation stroke is only 2–4
   effect-seconds, but the **decals hold the effect much longer**: the lava decals live 45 s and the crater
   decal **50 s**, and those (with the 3–20 s particles) are what the finish test waits for. The parent then
   needs one further draw-pass sweep for its meta child. So `volcano_control` ends about **50 effect-seconds**
   after its creator starts (§7, EF1).
   So the volcano's end is triggered by the effect **finishing** (§7). The deformation stroke is at least `length`
   effect-seconds (2–4 s × dilation), and the particles and decals may hold the effect longer (EF1).

   **Corrects (re-integrate1): the finish condition is now measured.** `volcano_control` finishes at the first
   draw-pass sweep (`0x005922A0`) after (i) the chosen `volcanocreator*` child has itself been finished on a
   previous sweep, and (ii) at that sweep that child's **brush** (`t > length`, 2.0–4.0 dilated effect-seconds),
   its **2–4 particle components** and its **3 decal components** are all inactive. The attractor and the game
   record never hold it (both `flags = 1`). **Completed (re-answers34, EF1, §7.1):** the particles do not hold
   it long — every emitter self-stops through `Stop(0)` when its loop/duration budget is exhausted, and
   `CheckForStop` then clears `started` when the last particle dies. The binding component is the **crater
   decal's 50 s**, so `volcano_control` ends ~50 effect-seconds after its creator starts, plus one sweep for
   the meta parent.
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
instance, which updates the instance's location (`0x00576E70`). **This is only established for components the
instance's parameter 13 reached** — the control effect and its meta descendants. The damage in the table below
runs through *particle-spawned* slaves, which get a different parameter 13 value (§16 EF3), so their `data1` is
open.

**Corrects (re-integrate1): "data1 = the instance" holds only for slaves reached as *meta* children.** There is
no single spawn API: each particle class builds a child effect name by concatenating its own base name with a
fixed suffix (`FUN_0041E270` `0x0041E270`) and calls `CreateVisualEffect` (slot 7). The suffixes and their
callers are `_slave` (`cSC4ModelBasedParticlesEffect::CreateParticle` `0x005AEF20`, per particle, and
`cSC4DynamicParticleEffect::Start` `0x00585190`, once), `_death` (`DestroyParticle` `0x005AF390`), `_collide`
(`cSC4DynamicParticleEffect::ApplyEffect` `0x00584890`), and `_terrain` / `_water` (`FUN_005B8290`, on impact).
Parameter 13 reaches **game components only** (`cSC4Scrubber` slot 6 `0x005BC2B0` accepts index 13 and copies
`*value` into `this+0x58`; decals only index 6; meta recurses via `0x005C0540`). Slaves spawned through the
**particle** path get the particle's own `+0x26C` value (flag `+0x26B`), **not** the disaster instance. Only a
slave reachable as a **meta component** of a control effect the instance started can post `data1 = the
instance`. **Settled (re-answers34, EF6):** the instance's handler `0x00576E70` matches strictly on
`data1 == instance`, so those particle-spawned slaves' location updates are dropped, not re-matched.
**Amended (re-answers34, EF7):** "particle `SetVoidParams` accepts only index 7" is the *shared*
`cSC4ParticlesEffect` handler `0x005B2200`; `cSC4ModelBasedParticlesEffect` overrides it with `0x005AC040`,
which also accepts index 13 (into its own `+0x26F`/`+0x270` channel). Whether the location update matches on
something other than `data1 == instance` is settled: it does not.

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

**The shipped control effects** (all priority 1; the components that reach a brush or game record are `[1, 6)`,
EF10):

| Menu entry | `N_control` | Components | Damage (through particle-spawned "slave" effects *[inferred from the names; the particle descriptions are render data and were not traced]*) |
|---|---|---|---|
| Meteors | `meteorstrike_control` (383) | particles `meteorcore` | `meteor_stuff_terrain` / `_water`: game 24 / 25 and one of five crater brushes (select group, weights 13107 each; records 11–15 / 16–20, one-shot, 2 applications); `meteor_stuff_slave`: game 23 posts the location message, but its `data1` is not the instance (EF6) so the instance does **not** track it |
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

**Corrects (re-specs20, audio.md §9.1):** this list is incomplete. There are **five** implementations of
this interface. The **sound component** (vtable `0x00AA1118`) is a fourth with the identical nine-slot shape
— its slots 5 and 6 share the no-op `0x005BE2E0` with brush and game — and `cSC4CameraEffect` is a fifth
with a *different* layout (description at `+0x10`, started byte at `+0x20` rather than the sound component's
`+0x0C` / `+0x1C`). Full sound-component table in [audio.md](audio.md) §9.1. **PARTLY** — the three rows
below are correct.

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
| `0x2A7B582E` kSC4MessageEffectLocation | game record → instance | data1 = the game component's `data` field (see the note), data2 / data3 = cell |
| `0x0A36F1E9` kSC4MessageTornadoLocation | game record 26 → MySims | data2 / data3 = cell |
| `0x0A36F1E8` kSC4MessageTornadoDead | — | **dead ID** (re-answers34, EF8): the name-table entry at `0x00B08430` is its only reference; nothing posts it and nothing consumes it |

**Note on `0x2A7B582E`'s data1 (EF3, re-answers29; settled re-answers34, EF6).** The message's `data1` is the
game component's `data` field at `+0x58`, which is filled from parameter 13 when one arrives and otherwise from
the description. The effect-disaster instance forwards parameter 13 = itself to the control effect and every
meta descendant, but a slave spawned by a *particle* gets the particle's own `+0x26C` instead (§16 EF3). The
instance's handler `0x00576E70` requires `data1 == this − 0xC` (the instance pointer) for **both** message
types before it acts (`0x00576E93`–`0x00576EBE`), so `data1 = the instance` holds for game records inside the
control effect and its meta children, and a particle-spawned slave is simply ignored — its location updates
are dropped. §10.2's damage table is unaffected (the damages do not depend on the location messages).

Order within one tick: the effects in active-list order, the components in component order, and in each game
component the actions in the order of §9.3. All posts are queued, so their receivers run after the tick.

## 13. Persistence

Nothing. The manager is not in the city's record, effects are not saved, and the generators of §14 are not saved.
After a load, a volcano or effect disaster that was running is not running (disasters.md §14 already says the layer
saves nothing).

## 14. Random numbers *[compat]*

| Draw | Generator | When |
|---|---|---|
| select group `RandomUint32Uniform(0x10000)` | the manager's `cRZRandomSuper` at `+0x1C0`, seed `0x1105` | once per participating select group per component build: new build, reload, or a member entering the level-of-detail range (§5.3 rebuild test, EF12) |
| particle emitter start countdown | global `cRZRandom` `0x00B22DF8` (`RandomDoubleRangeUniform`), at most one per `Start` that starts | §7.1.1 (EF13); render only |
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
   depend on it. A harness that ticks the simulation but never runs the 3D view's draw pass keeps the disaster,
   and therefore the emergency pause, forever. **Proven 2026-10-16 (re-answers29, EF2):** the finish test is
   unreachable by any other route, so this needs no runtime check. A replacement that must work without a 3D
   view has to run the sweep elsewhere or synthesise the end message.
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
3. The game does not stop the *animation-tick manager* when the user pauses (there is no `PauseAnimationSystem`
   call wired to the pause), so `kSC4MessageAnimationTick` keeps arriving while paused — but the dilation is
   **0.0** while paused (§6), so `dt'` is 0 and every component update is a no-op: brushes make no progress and
   timers do not advance. The earlier reading "dilation 1 while paused" was wrong (re-answers34); the effect of
   a pause is the same as before (nothing advances) but the mechanism is the zero dilation, not a missing tick.
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
  **Amended (re-answers34): the asymmetry is gone — particles self-stop too, through the emit budget
  (§7.1), so the "no self-stop path at all" half of this entry is obsolete.**

## 16. Open questions

- ~~**EF1.** The exact finish time of `volcano_control` and of the disaster control effects. It depends on the
  particle and decal components' `IsActive`, which are render objects.~~ **Bounded 2026-10-16 (re-answers29),
  closed 2026-10-20 (re-answers34) — see the ANSWERED entry above.** The bound stands: particles are not
  ignored (`flags = 0`), every shipped volcano particle has `emit_loop_count = 1` and a `life` of 3–20 s, the
  decals live 45 s (lava) and 50 s (crater), and the end is **~50 effect-seconds plus one draw-pass sweep**.
  What re-answers34 added is the mechanism (§7.1): the emitter self-stops through `Stop(0)` when its loop or
  duration budget is exhausted, and `CheckForStop` converts that to an immediate stop when the last particle
  dies — so a runtime trace of `0x005BFD70` is no longer needed.
- ~~**EF2.** Confirm at runtime that a volcano never reaches state 7 when the city view does not draw (quirk 1).~~
  **ANSWERED 2026-10-16 (re-answers29): settled statically; no runtime run needed.** The finish test has exactly
  one caller (`0x005922A0`), which has exactly one caller (`0x00593120`), which has none — it is a draw-pass
  vtable slot at `0x00A9F25C`. So no draw pass means no finish, no end message, and the volcano never reaches
  state 7. Quirk 1 is **proven**. §7.
- ~~**EF3.** The path by which particle systems spawn the "slave" effects (`meteor_stuff_terrain`,
  `robot_walkfx_slave`, …), and whether the parameter 13 forwarded by the effect-disaster instance reaches them.
  The effect-location messages depend on it. The particle descriptions were not parsed.~~ **ANSWERED
  2026-10-16 (re-answers29) for the path; the `data1` question DIFFERS and stays open.**
  - **Spawning.** Each particle class builds a child effect *name* by concatenating the particle description's
    base name with a fixed suffix (the `cRZString` `+` at `0x0041E270`) and calls the manager's
    `CreateVisualEffect` (slot 7). The suffixes, all literal strings in the executable:

    | Suffix | Built in | When |
    |---|---|---|
    | `_slave` | `cSC4ModelBasedParticlesEffect::CreateParticle` `0x005AEF20` | every time a particle is created |
    | `_slave` | `cSC4ModelBasedParticlesEffect::SetDescription` `0x005AEC90` | once, via `HasVisualEffect` (slot 5) |
    | `_death` | `…::SetDescription` `0x005AEC90` / `DestroyParticle` `0x005AF390` | on particle death |
    | `_slave` | `cSC4DynamicParticleEffect::Start` `0x00585190` | once at start |
    | `_collide` | `cSC4DynamicParticleEffect::ApplyEffect` `0x00584890` | on collision with speed > 0.01 |
    | `_terrain` | `cSC4ParticlesEffect::ApplyTerrainForces` `0x005B8DC0` | a particle drops under the terrain and `+0x218` is set |
    | `_water` | same | under the water line and `+0x219` is set |
    | `default_dynparticle_slave` (literal, no base) | `cSC4DynamicParticleEffect::AttachSlaveEffect` `0x005840E0` | when the description has no base name |

    The base name is `cSC4ParticlesDescription + 0x160`, or `cSC4DynamicParticleDescription + 0x0C`. So
    `meteor_stuff_terrain` / `_water` / `_slave` share the base `meteor_stuff`, and `robot_walkfx_slave` has the
    base `robot_walkfx`. Slaves are **independent top-level effects**, not meta children.
  - **Parameter 13 (`0xD`) is an opaque pointer channel**, not a float: `ForwardParameter` `0x005BFC20` sends it
    to every component's slot 6 `SetVoidParams`, and only the **game** component accepts index 13 (copying
    `*value` into its `data` field at `+0x58`, which becomes the message's `data1`). Brush and decal ignore it;
    dynamic particles take only 7, `0x10`, `0x11` and `0x12`; a **meta** component forwards it recursively into
    the child's components. *(Amended re-answers34, EF7: the shared particle handler `0x005B2200` takes only 7,
    but `cSC4ModelBasedParticlesEffect`'s override `0x005AC040` also takes 13 — into its own `+0x26F`/`+0x270`
    channel, not into any game-like `data` field.)*
  - **The effect-disaster instance's value is the instance pointer.** `cSC4DisasterInstanceEffect::Init`
    `0x00577010` does, in order: `SetEndMessage(0x2A7B582D, this)`, `ForwardParameter(13, &this, 1)`,
    `Start(0)`, then subscribes to `0x2A7B582D` and `0x2A7B582E`. So parameter 13 **does** reach every game
    component of the control effect and of every meta descendant — but **not** particle or decal components, and
    **not** a slave spawned by a particle.
  - **Particle-spawned slaves get a different value.** `0x005AEF20` forwards `ForwardParameter(13, &this+0x26C, 1)`
    (gated by `this+0x26B`), `0x005AF390` the same with `+0x26C`, `0x005AF060` with `+0x270` gated by `+0x26F`.
    That is the **particle's own field**, not the disaster instance. `cSC4DynamicParticleEffect`'s slave paths
    never forward parameter 13 at all.
  - **Messages** (the executable's own name table, pairs around `0x00B08360`–`0x00B086F8`):

    | ID | Name | Carries |
    |---|---|---|
    | `0x2A7B582D` | `kSC4MessageEffectDone` | `data1` = the effect-disaster instance (the end message, so posted only from `0x005BFD70`) |
    | `0x2A7B582E` | `kSC4MessageEffectLocation` | `data1` = a game component's `data`; `data2` / `data3` = the cell. Posted from `cSC4Scrubber::Apply` `0x005BD3C0` |
    | `0x0A36F1E9` | `kSC4MessageTornadoLocation` | game record 26 in `tornadofx_slave` |
    | `0x0A36F1E8` | `kSC4MessageTornadoDead` | **new**; not previously listed |

  **DIFFERS from §10.2**, which assumed the location messages carry `data1 = the instance`. Statically `data1` is
  the game component's `data` field, which for a particle-spawned slave is the particle's `+0x26C` value. Either
  the location handler matches on something other than `data1 == instance`, or those slaves are reached as meta
  children in some effects. **Settled by EF6 below (re-answers34): the handler matches strictly on
  `data1 == instance`, so the slaves are simply not tracked.** Also settled there: the semantics of `+0x26B` /
  `+0x26C` / `+0x26F` / `+0x270` on `cSC4ModelBasedParticlesEffect` (EF7), and the full shipped list of `_*`
  effects was enumerated by re-integrate1 (§10.2).
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
- ~~**EF1.** The exact finish time of `volcano_control` and of the disaster control effects.~~
  **ANSWERED 2026-10-20 (re-answers34): MATCHES — statically, no runtime trace needed.** The particle emitter
  *does* self-stop; the mechanism is §7.1. In short:
  - `cSCParticleSimulatorBase::Stop` `0x005B9E70` takes two bytes: mode1 == 0 is an **immediate** stop
    (`started` := 0, all particles destroyed), mode1 != 0 is **delayed** (`+0x0C` set, particles play out).
    `cSC4ParticlesEffect::Stop` `0x005B21E0` maps its mode to `(mode == 0, 0)`, so the component's `Stop(0)`
    is the delayed one.
  - `CheckForStop` `0x005B9D40`, called at the end of every draw-time update, converts a delayed stop into an
    immediate one when the live count `+0x14` reaches 0 — that is the moment `IsActive` goes false.
  - The emitter self-stops through `Stop(0)` when its budget is exhausted: `cSC4ParticlesEffect` at
    `0x00AA6D4`/`0x00AA6EB`; `cSC4ModelBasedParticlesEffect` via its `emit_loop_count` budget at `0x005B8D67`
    or its duration budget at `0x005B8B29` (three emitter paths picked by the particle description's flag bits
    2 and 1, §7.1).
  - So every shipped volcano particle component ends on its own (`emit_loop_count = 1`), and the finish time of
    `volcano_control` is the **crater decal's 50 effect-seconds** plus one draw-pass sweep for the meta parent —
    the earlier bound is now exact. §7.1 has the mechanism; the wall-clock second needs only the dilation of §6.

  (Supersedes the 2026-10-16 bound and the "no self-stop path" reading recorded in §7; the runtime trace probe
  is no longer needed.)
- ~~**EF4.** The meaning of visual effect flag bits 0, 3, 6 and 8, and when an effect created during a draw-pass
  sweep is activated in a session that only ticks.~~ **ANSWERED 2026-10-20 (re-answers34): MATCHES (bits) /
  settled (activation).**
  - **Bits.** The 9-bit description flags are stored verbatim into the effect's `+0x28` (reader `0x005ABF50`
    masks `& 0x1FF`; `TestFlag` slot 17 `0x005BFC90` reads them back). A complete byte-level scan of the
    effects module (`0x0058D000`–`0x005C5000`) finds tests of exactly five bits on `+0x28`: bit 1 (finish test
    `0x005BFD89`), bit 2 (hard stop, `0x005BEBB6`), bit 4 (slot 13 does not forward, `0x005BFC26`/`0x005C05E6`),
    bit 5 (pick ray, `0x0058E6A7`), bit 7 (finish test `0x005BFDD2`). **Bits 0, 3, 6 and 8 have no consumer
    anywhere in the module**: no `TEST AL,1`-style test of `+0x28` for those bits exists. They are stored and
    observable through `TestFlag`, and their only plausible use is external/scripting. A replacement may accept
    them and ignore them.
  - **Activation.** `ActivateNewVisualEffects` `0x005921B0` is called from four places: both `CreateVisualEffect`
    overloads (`0x00593AB0`, `0x00593B68`), the end of `DeactivateUnusedVisualEffects` (`0x00592A01`, the draw
    pass), and `UpdateActiveVisualEffects` `0x00593B90`. The last one runs on `kSC4MessageDoEffectsTick`
    `0x697D01ED` (from `DoMessage` `0x00594476`) and on zoom/render-property changes. `0x697D01ED` is posted by
    `EffectsTickCallback` `0x0058D9A0`, which the manager registered with the animation-tick manager in `Init`
    (`0x00594EEE`: `EnableAutoStep(1, 0x0058D9A0)`). So **in a session that only ticks, an effect created during
    a draw-pass sweep is activated on the next animation tick** (when the sweep flag `+0x1B4` is clear), not only
    on the next draw pass. §6.1.
- ~~**EF5.** Whether a hidden pause counts as "paused" for the time dilation.~~
  **ANSWERED 2026-10-20 (re-answers34): DIFFERS from the earlier dilation reading.** `GetAnimationTimeDilation`
  `0x007026F0` consults, in order: `IsEmergencyPaused` (slot 17) → the paused value; the object at global
  `0x00B43D44` (a city manager whose slot 13 is `cSC4AutomataManager::IsDrivingMode` `0x00506940` — true while
  the U-Drive-It view `0x2B709676` is up) → the paused value; `IsPaused` (slot 15) → **0.0**; otherwise the
  speed value. `IsHiddenPaused` (slot 16) is **never called**. So:
  - a hidden pause does **not** change the dilation — the 5 s and 15 s hidden pauses of §9.4 run in
    **effect-seconds dilated at the current speed**, not real-time seconds;
  - the shipped values are 0.0 (paused), 1.0 (emergency pause, U-Drive-It driving, speed 0), 1.5 (speed 1),
    2.0 (speed 2) — the earlier "1 while paused, 3 at speed 2" was wrong (§6, quirk 3 amended).
  Note the harness consequence: a hidden pause does not slow or freeze effects, so those 5 s/15 s pauses elapse
  faster than wall-clock when the sim runs at speed 1 or 2.
- ~~**EF6.** What `data1` is on a `kSC4MessageEffectLocation` (`0x2A7B582E`) posted from a game record inside a
  **particle-spawned** slave.~~ **ANSWERED 2026-10-20 (re-answers34): DIFFERS — the handler matches strictly on
  `data1 == instance`, so particle-spawned slaves never update the instance's location.** The instance's
  message handler (registered as the `cIGZMessageTarget2` sub-object slot at `0x00A9E254`, function
  `0x00576E70`) does:
  - type `0x2A7B582D` (`EffectDone`): reads the message's data1 (`[vt+0x24]` slot 9) and requires it to equal
    `this − 0xC` — the instance pointer (`LEA ECX,[EDI−0xC]; CMP EAX,ECX` at `0x00576EB9`–`0x00576EBE`).
  - type `0x2A7B582E` (`EffectLocation`, the `SUB EAX,0x2A7B582D; DEC EAX` path at `0x00576E86`): **the same
    equality check first** (`0x00576E93`–`0x00576E95`); only when `data1 == instance` does it read data2/data3
    (slots `+0x38`/`+0x4C`) into the instance's `+0x4`/`+0x8` and call the location setter at `[this−4] vt+0xC`.
  So `cSC4Scrubber::Apply` `0x005BD3C0` (which posts `data1 = data (+0x58)`) only drives the instance when its
  `data` is literally the instance — established only through parameter 13. A particle-spawned slave whose
  `data` is the particle's `+0x26C` is ignored. §10.2's damage table stays correct (the damages do not depend
  on the location messages); only the *tracking* of those slaves by the instance does not happen. The part of
  §10.2's table that says "`meteor_stuff_slave`: location" holds only when that slave is reachable as a meta
  child (it is not, per EF3) — so in practice the instance does not track tornado/meteor/robot walk slaves.
- ~~**EF7.** The exact semantics of `cSC4ModelBasedParticlesEffect +0x26B / +0x26C` and `+0x26F / +0x270`.~~
  **ANSWERED 2026-10-20 (re-answers34): MATCHES the five-byte split, with one correction — there *is* a
  `SetVoidParams`.** From the bytes (`0x005AC510` ctor, `0x005AEC90` `SetDescription`, `0x005AC040`):
  - **`+0x26B`** (`u8`): the *forward-parameter-13 gate*. Read twice, both before forwarding parameter 13 to a
    freshly spawned slave effect: in `CreateParticle` `0x005AEF20` at `0x005AF02A` and in
    `DestroyParticle` `0x005AF390` at `0x005AF8B7` — when it is non-zero the child gets
    `ForwardParameter(13, &this+0x26C, 1)`. **Nothing in the effects module ever writes it** (a full ModRM/SIB
    scan for writes to disp32 `0x26B` finds none; the ctor at `0x005AC510` initialises `+0x26C..+0x270` but not
    `+0x26B`), so its value is whatever the pooled allocation held — effectively uninitialised *[quirk]*.
    A replacement should treat it as an explicit, default-off "forward the parameter to slaves" switch.
  - **`+0x26C`** (`u8`): a render latch, set by `SetDescription` (`0x005AEDEB`) to the return of
    `cS3DModelProcessor::SetModel` — non-zero when the description's model was set. Read only in
    `RenderActiveParticles` `0x005AE4A1`, where it gates `LightModel` and `DeformModel` for the effect.
  - **`+0x26D`** (`u8`): "a `<base>_slave` effect exists" — `SetDescription` formats `%s_slave`
    (`0x00AA0CD0`) and stores the `HasVisualEffect` result (`0x00AEE79`).
  - **`+0x26E`** (`u8`): "a `<base>_death` effect exists" — `%s_death` (`0x00AA0CC4`), `0x00AEEF7`.
  - **`+0x26F`** (`u8`) / **`+0x270`** (`u32`): the *parameter-13 channel for the chain hand-off*.
    `SetVoidParams` **does exist on this class** — `0x005AC040` (slot 6 of the component vtable `0x00AA0C58`):
    index 13 sets `+0x26F = 1` and `+0x270 = *value`; any other index falls through to the shared particles
    handler `0x005B2200` (index 7 only). `HandOffParticleToChainSystem` `0x005AF060` reads `+0x26F` at
    `0x005AF27C` and forwards `13, &this+0x270` to the new slave; `DestroyParticle` reads `+0x26B` and forwards
    `13, &this+0x26C`.
  So the two channels are: `+0x26C` holds the value forwarded on the *per-particle* paths, `+0x270` the value
  forwarded on the *chain hand-off* path, and `+0x26B`/`+0x26F` are their respective enable bytes. §10.2's
  "particle `SetVoidParams` accepts only index 7" needs the amendment: that is the *shared* `cSC4ParticlesEffect`
  handler `0x005B2200`; the ModelBased override `0x005AC040` also accepts index 13.
- ~~**EF8.** Who posts and who consumes `kSC4MessageTornadoDead` `0x0A36F1E8` and
  `kSC4MessageMissionEffectStarted` `0x5F900DCC`.~~ **ANSWERED 2026-10-20 (re-answers34).**
  - `kSC4MessageTornadoDead` `0x0A36F1E8`: the constant occurs **once** in the whole image — its name-table
    entry at `0x00B08430`. Nothing posts it and nothing dispatches on it. It is a dead ID in this build (its
    live sibling `kSC4MessageTornadoLocation` `0x0A36F1E9` is posted by the MySims handlers `0x0042E5DF` /
    `0x0042EE8C` and consumed by `cSC4MySimAgentSimulator::DoMessage` `0x0042EFDC`). A replacement need not
    implement it; keeping it out of the name table would change nothing observable.
  - `kSC4MessageMissionEffectStarted` `0xCC0D905F` (the ID as stored in the table; the question's `0x5F900DCC`
    is a transposed reading): **nobody posts it** either — the only references are
    `cSC4CitySituationManager::Init` `0x0052DD20` (subscribe, message-server slot 6 `+0x14`),
    `DoMessage` `0x0052EFEC` → handler at `0x0052F096`, and `Shutdown` `0x00527C58` (unsubscribe, slot 6
    `+0x18`). The handler checks `[this+0x9F0]`, calls its slot 15 `+0x3C` and, when that returns true, stores
    the message's sender (`[vt+0x80]`) into the `cRZAutoRefCount<cIGZWin>` at `+0xA98` (`0x0052F0B9`–
    `0x0052F0C0`) — i.e. it binds the sender window for a city situation, but in this build nothing ever sends
    the message. Both are latent hooks, not live traffic.
- ~~**EF9.** Where the message names for `0x86AD10EE` and `0x697D01ED` live.~~
  **ANSWERED 2026-10-20 (re-answers34): DIFFERS from the "115 pairs" reading.** There is only **one** message
  name table and it is larger than thought: it spans `0x00B07FD8`–`0x00B08FE0` (**513** `{u32 id, const char*}`
  pairs, every one resolving to a printable name), not the 115 pairs at `0x00B08360` (that address is simply a
  position inside the same table — `kSC4MessageMouseScrollBegin` sits there). `kSC4MessageAnimationTick`
  `0x86AD10EE` is entry `0x00B08250` (name string `kSC4MessageAnimationTick` at `0x00A8C06C`; its sibling
  `0x86AD10EF` "kSC4MessageAnimationTickPriority" is `0x00B08258`). `kSC4MessageDoEffectsTick`-style name for
  `0x697D01ED` **does not exist**: the ID appears in exactly four places (`0x0058D9CA` the posting callback,
  `0x0059446C`/`0x00594DA5` the dispatch and subscribe in the effects manager) and no name-table entry and no
  name string anywhere in the image. So `0x697D01ED` is an unnamed internal message. §12 amended.
- ~~**EF10.** Whether "every component that reaches a brush or game record is `[1, 6)`" holds per effect (§5.2).~~
  **ANSWERED 2026-10-20 (re-answers34): MATCHES — measured on the shipped directory.** The assertion in
  `engine/formats/tests/effects_directory.rs` (`effects_directory_sweep`) was run against the installed game
  (`FC4_GAME_DIR` = the GOG install; `SimCity_1.dat`, 1,094,484 bytes, root version 4). Output:
  `effects_directory: 72 effects reach a brush or game record, all through [1, 6) components; 887 other
  components have another range`. So the narrower claim holds for all **72** effects that reach a brush or game
  record, and the remaining **887** non-`[1, 6)` components are all outside those reach sets (render-only).
  §5.2 and quirk 8 stand as written; test 14 has its measured counterpart.

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
16. **The particle emitter self-stops** (§7.1, re-answers34). A `cSC4ParticlesEffect` or
    `cSC4ModelBasedParticlesEffect` component started with `Start(0)` reaches inactive on its own: its emitter
    calls `Stop(0)` on itself when the loop or duration budget is exhausted (`0x00AA6D4`, `0x005B8D67`,
    `0x005B8B29`), the delayed stop (`+0x0C`) is converted by `CheckForStop` `0x005B9D40` to an immediate stop
    once the live count `+0x14` is 0, and only then does `IsActive` return false. A hard `Stop(1)` destroys the
    particles at once. No other path clears `started`.
17. **The dilation table** (§6, re-answers34). With the shipped simulator: paused → 0.0; emergency-paused or
    U-Drive-It driving → 1.0; speed 0 → 1.0; speed 1 → 1.5; speed 2 → 2.0. A hidden pause changes nothing.
    (`dt' = trunc(µs × dilation)`; at dilation 0 the component updates still run, with `dt = 0`.)

## 18. The manager's interface sub-objects *[compat]*

**Answer to glue22 question 9** (`enable = effects` faults `ACCESS_VIOLATION` writing `0x4D` at a wild `eip`
about 100 ms after `Init` returns, with `eax = esi = ebp = 0`, `edi = 0x0.....801E`, and no method of the
replacement entered). Verdict: **DIFFERS** from the replacement's current shape. The original
`cSC4EffectsManager` is **not** one interface; it is six C++ sub-objects with their own vptrs, and the game
holds two of them by *offset*, not by the primary pointer. The change to make is in §18.4.

### 18.1 The sub-object map

The constructor/destructor of `cSC4EffectsManager` (`0x0058FA60`) stores every vptr, which fixes the layout
exactly:

```c
this->field0_0x0    = vftable;             // 0x00A9F264
this->field1_0x4    = vftable_1;           // 0x00A9F250
this->field2_0x8    = &PTR_FUN_00a9f240;   // 0x00A9F240
this->field3_0xc    = &PTR_FUN_00a9f210;   // 0x00A9F210
this->field4_0x10   = vftable_4;           // 0x00A9F1F8
this->field133_0x94 = (int)vftable_5;      // 0x00A9F198
```

The Mac build agrees independently: `cSC4EffectsManager` has exactly five non-virtual thunks, and the
`__ZThnN_` adjustment in each mangled name is the sub-object offset (`__ZThn4_`, `__ZThn8_`, `__ZThn12_`,
`__ZThn16_`, `__ZThn148_` — i.e. +4, +8, +12, +16, +148 = `0x4`, `0x8`, `0xC`, `0x10`, `0x94`).

| Offset | Windows vtable | Slots | Methods on that sub-object (Mac thunks) | Interface |
|---|---|---|---|---|
| **`+0x00`** | `0x00A9F264` | 12 | the §11 `cISC4EffectsManager` slot table | `cISC4EffectsManager`, IID `0xC97CF5CD` |
| **`+0x04`** | `0x00A9F250` | 5 | QI, AddRef, Release, `Update(SC4DrawContext*)`, `Pick(cS3DVector3 const&, cS3DVector3 const&, SC4DrawContext*, float*)` | the draw-pass / pick interface |
| **`+0x08`** | `0x00A9F240` | 4 | QI, AddRef, Release, **`DoMessage(cIGZMessage2*)`** | **`cIGZMessageTarget2`, IID `0x452294AA`** |
| **`+0x0C`** | `0x00A9F210` | 12 | QI, AddRef, Release, D0, D1, … | unnamed base *[inferred]* |
| **`+0x10`** | `0x00A9F1F8` | 6 | QI, AddRef, Release, D0, D1, … | unnamed base *[inferred]* |
| **`+0x94`** | `0x00A9F198` | 24 | QI, AddRef, Release, **`BeginCollection`**, **`EndCollection`** | the effects-collection interface, IID `0xC97CF5CE` |

`DoMessage` is `0x00594120`, which is slot 3 of the `+0x08` vtable `0x00A9F240` — that is how the `+0x08`
identity is pinned. The draw-pass `Update(SC4DrawContext*)` (`0x00593120`, §7) is slot 3 of `0x00A9F250`, so
the draw-pass sub-object is **`+0x04`** and the `+4` experiment had the right offset.

### 18.2 What `QueryInterface` hands out

`cSC4EffectsManager::QueryInterface` is `0x0058D9E0`. It returns a **sub-object pointer**, never the primary,
for two IIDs:

| `riid` | returned | sub-object |
|---|---|---|
| `0xC97CF5CD` | `this` | `+0x00` |
| `1` (`GZIID_cIGZUnknown`) | `this` | `+0x00` |
| **`0x452294AA`** | **`this + 8`** | **`+0x08`** (`cIGZMessageTarget2`) |
| **`0xC97CF5CE`** | **`this + 0x94`** | **`+0x94`** (collection) |
| `0x0A5EAA1A` | forwarded to the object at `this + 0xD98` | a nested object, not a base |

Any other IID returns 0 with the out pointer left alone. Note the same IID/CLSID confusion that broke the
automata classes (automata §15): the two "extra" IIDs here are **interface** IIDs, and the primary pointer is
only correct for `1` and `0xC97CF5CD`.

### 18.3 Why the replacement faults the way it does

The message service (`0x00B43CCC`) takes a `cIGZMessageTarget2*` and later dispatches to it. The original
hands out `this + 8`. A replacement that is only as big as one interface has **nothing at `+0x08`**, so a
target pointer formed the way the original forms it points just past the object; fetching `*(obj + 8)` as a
vptr yields adjacent heap, the call jumps to whatever that contains (the observed `eip = 0x0.....8023`,
"no module"), and the garbage instruction faults on a write to `0x4D`. That also explains why **no method of
the replacement is entered after `Init`** and why `AddNotification` still returns normally: the subscription
succeeds and only the later dispatch goes wrong. The `+4` draw-pass sub-object does not help because the
dispatch is on `+0x08`, not `+0x04`, and `Update` is never reached before the fault
*[inferred — the exact call site of the dispatch was not traced]*.

The stack frame `SimCity 4.exe+0x5BF36` in the crash report is the return address of `call [edx+0x0C]` at
`0x45BF33` in `FUN_0045bd20` (the city's Init loop), i.e. the `Init` of the component at `cSC4City + 0x12C`
(`CreateObject(0x098F964D, 0xC6BB8334, …)`, `decomp/cSC4City.c:1999`), which runs **after** the effects
manager's own `Init` (`0x45BF12`, next instruction `0x45BF15`). So the fault is during a later component's
`Init`, not inside `cFC4EffectsManager::Init` — consistent with a dispatch that arrives after our `Init`
returned.

### 18.4 The change to make

Do **not** ship a single-interface object. Either

1. **Lay out all six sub-objects at exactly `+0x00`, `+0x04`, `+0x08`, `+0x0C`, `+0x10`, `+0x94`** and put the
   matching vtable at each, with `QueryInterface` returning `this`, `this+8` and `this+0x94` for
   `0xC97CF5CD`/`1`, `0x452294AA` and `0xC97CF5CE` respectively (§18.2); or
2. keep the object small but make every `QueryInterface` answer return a sub-object that **actually exists**
   at the offset the caller will use, and never return `this + N` for an `N` past the end of the object.

At minimum the object must be large enough to hold `+0x94`, since that is a real sub-object the original
returns from `QueryInterface`, and it must expose `cIGZMessageTarget2` at `+0x08` with `DoMessage` at that
sub-object's slot 3. Return the **sub-object** pointer for those IIDs, not the primary — the same rule as
automata §15.

State the original keeps at `+0x1B4`, `+0x1B8`, `+0x1BC`, `+0x1C0`, `+0xB90`, `+0xBAC`, `+0xC84`, `+0xC8C`,
`+0xD64` and `+0xD98` (§2 and §18.2) is inside the primary object and must exist at those offsets too if the
replacement is to survive the game's own direct field access.

### 18.5 Still open

- The interface identities of the `+0x0C` and `+0x10` bases (12 and 6 slots, both with `D0`/`D1`) are
  *[inferred]* to be internal bases: neither is returned by `QueryInterface` (`0x0058D9E0` handles only
  `0xC97CF5CD`, `1`, `0x452294AA`, `0xC97CF5CE` and `0x0A5EAA1A`).
- The exact dispatch site that reaches `+0x08` before the first draw needs a runtime probe: log every call
  into the replacement after `Init` together with `this` and `this+8`, or break on `cIGZMessageTarget2`'s
  `DoMessage` in the original and record the caller.

## 19. Effects-script command contracts *[compat]*

This section concerns text-description commands, separate from the binary
directory reader and the runtime components in §§3–10. Evidence is static
Windows 1.1.641 export instructions and descriptor data, inspected 2026-10-23.
Conformance examples are proposed, unexecuted predictions. An implementation
must not treat this partial publication as whole-family closure of EF15.

### 19.1 Shared input and service contracts

The root registration inventory and descriptor/hook identities are in
[effects-parser.md §6.1](effects-parser.md#61-root-dictionary-selected-by-the-owner).
A handler receives a vector of 20-byte token records, including command token 0,
and the effects parser context P. Numeric/vector conversions are individual
foreign services; renderer, audio and resource calls are likewise separate
services. Their successful replies must be supplied individually in owner tests.
They must not be replaced by a foreign service returning a completed description.

The switch and positional reader's concrete behavior, mutation and minimum-arity
rules are in [effects-parser.md §9](effects-parser.md#9-switch-and-positional-reader-contract-compat).
In particular, handled `-word` tokens become `+word`, the first duplicate wins
each lookup, ordinary records are retained, and unknown unhandled switches can
be rejected after a handler has already changed parser state.

Error exits are C++ exceptions, with no command-local rollback; see
[§10](effects-parser.md#10-error-transport-and-owner-failure-prefix-compat).
The collection's named/anonymous identity, replacement and retained-reference
contract is in [§11.1](effects-parser.md#111-component-directory-service-used-by-commands).
The parser's positively identified final-release/destructor chain is in
[§11.2](effects-parser.md#112-native-parser-final-release).
The generic registrar, duplicate policy, descriptor ownership and block dispatch
are in [§12](effects-parser.md#12-generic-registration-and-dispatch-dependency-compat).
These dependencies and effects-parser.md §13 answer EFP1–EFP6 at their
command-owner boundaries. Section 13 specifies all five rotation keys and
their x87/binary32 intermediate stores; broader numeric qualification is EFP7.
Descriptor registration alone does not
prove a command's execution algorithm.

### 19.2 Two root command owners

`cRenderPropCommand` is the descriptor `00A9F85C`, root literal `renderprop`,
handler `00596630`. It requires exactly three records: the command, property
name and value; too few or too many constructs a parse error before renderer
lookup. There are no defaults. It searches the renderer-properties service at
`P+A8` in bool, integer, then float order (lookup slots `+30,+34,+38`), accepting
the first nonnegative index. It converts only that selected value with the
matching ParseBool, ParseInt or ParseFloat primitive and calls the corresponding
setter (`+24,+28,+2C`). A name present in several property categories therefore
uses the bool category first. If all three lookups return negative indices it
throws `Unknown render property`. This changes renderer property state, not an
effect-description record; renderer name matching and setter validation remain
renderer-service contracts. A live renderer-properties pointer is required.
Evidence: `00596630–0059673D`.

`cEffectGroupIDCommand` is block descriptor `00A9F8BC`, root literal
`effectGroup`, enter hook `00596750`. It requires at least one argument after the
command; absence throws `No ID specified!`. It converts token 1 through
ParseUint and stores the full returned word in `P+49C` before entering the
generic block. It does not enforce an upper argument count, create an effect
description, or install a default ID. Its own nested registrar `005967D0`
registers only `instance` with descriptor `00A9FA08`; generic dispatch can still
fall back to outer-block and root dictionaries (§19.1). The registered exit hook
is `00648F10`, a return-only hook.
No restoration of the prior group-ID word is performed by the enter hook.
The `instance` execution contract is still open under EF15. Evidence:
`00596750–005967F8` and root registration §6.1.

### 19.3 Command and dependency conformance vectors

Keep these as independent expectations when adding engine vectors; none has
been executed against the original parser in this lane. No engine files are
part of this publication.

| ID | Input / independent primitive replies | Required observable result |
|---|---|---|
| EC1 | `renderprop property value`; bool lookup returns 7, int/float would also recognize it; ParseBool returns true | bool lookup then bool conversion then bool setter `(7,true)` only; no int/float lookup or conversion; no description mutation |
| EC2 | same three records; bool returns -1, int returns 2; ParseInt returns -9 | bool lookup then int lookup, integer conversion and integer setter `(2,-9)` only |
| EC3 | `renderprop property` | arity exception before renderer lookup; no setter |
| EC4 | three records; all three renderer category lookups return -1 | bool, int, float lookups in that order, then unknown-property exception; no value conversion or setter |
| EC5 | `effectGroup id surplus`; ParseUint(token 1) returns `FEDCBA98` hex | P+49C becomes `FEDCBA98`, then block entry; surplus not rejected by this owner |
| EC6 | `effectGroup` only; P+49C initially 9 | `No ID specified!` exception, no uint conversion or block entry, P+49C stays 9 |

The dependency vectors SW1–SW8, ER1–ER3, DI1–DI5 and LT1–LT3 in
effects-parser.md §§9–11 cover the first-duplicate rule, retained records,
failure prefixes, case-folded per-type names, anonymous indices, pointer
replacement and final lifetime. They supplement the existing runtime vectors
in §17 rather than claiming command coverage from runtime tests.
