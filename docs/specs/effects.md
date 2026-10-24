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

### 3.1 Legacy placement decoding and component world positions (EF19) *[compat]*

The 56 file bytes in the placement column are **not a memory image** of the
56-byte transform. All offsets in the following table are relative to the
placement, not the containing component. Words use the resource stream's scalar
byte order (little endian in the shipped resource).

| File offset | Value | Transform memory destination |
|---|---|---|
| `00,04,08` | binary32 matrix row 0: `m00,m01,m02` | `+04,+08,+0C` |
| `0C,10,14` | row 1: `m10,m11,m12` | `+10,+14,+18` |
| `18,1C,20` | row 2: `m20,m21,m22` | `+1C,+20,+24` |
| `24,28,2C` | binary32 translation `tx,ty,tz` | `+28,+2C,+30` |
| `30` | binary32 uniform scale `s` | `+34` |
| `34` | unsigned 32-bit revision word | low byte replaces memory byte `+00`; upper 24 bits discarded |

There is no transpose, axis exchange, Euler/quaternion conversion or matrix
normalisation. Memory byte `+01` is a separate transform flag byte: loading ORs
in rotation bit 4 and translation bit 1 **even for an identity matrix and a zero
translation**. Scale bit 2 is cleared if the loaded scale compares equal to 1,
and set otherwise (including unordered/NaN). Other flag bits are retained.
The setters increment revision byte `+00` three times, but the final file word
overwrites those increments. Memory bytes `+02,+03` are not file fields.
Writing emits the nine matrix words, three translation words, scale word, then
the revision byte zero-extended to a 32-bit word. Thus arbitrary upper revision
bits are lost in native load/write even though a framing-only adapter may retain
the original 56 bytes. Float words are copied without a numeric conversion.
Evidence: matrix read `005DA650–005DA675`, placement destinations
`005DA975–005DA9C3`, translation `005DA9DF–005DAA24`, scale/revision
`005DAA26–005DAA6B`, writer `005DAA80–005DAACB`.

For component placement, write L for the local transform and P for the parent.
Rows are defined by this observable convention: world coordinate i is parent
translation i plus parent scale times row i dotted with **L's translation**.
L's own matrix and scale do not affect its world position. Both brush
(`00581560`) and game (`005BC3B0`) use these shortcuts, in this precedence:

1. If L's flag byte is zero, use P's translation directly.
2. Otherwise, if P's flag byte is zero, use L's translation directly.
3. Otherwise evaluate the arithmetic below, even when L's translation is zero.

The test is the entire flag byte, not a test of just one bit or an inspection
of the matrix. In particular, a legacy-loaded identity has a nonzero flag byte.
Direct paths preserve finite binary32 translation bits, including negative zero.
Brush stores x and z; game stores x, y and z before deriving its cell/map bounds.
Do not substitute a zero-offset shortcut for rule 3: arithmetic can change zero
signs and can propagate NaNs from a parent matrix.

For exact arithmetic let `R` mean rounding an individual x87 arithmetic operation
to the caller's precision (normally 24 significant bits, nearest/even; foundations
§18), with x87 exponent range. Let `S` mean a binary32 memory store/reload.
For each row i, form three products in the order z, x, y and define
`di = R(R(R(mi2 × L.tz) + R(mi0 × L.tx)) + R(mi1 × L.ty))`.
There is no fused multiply-add and the first sum is **z-product + x-product**,
not x-product + y-product. The following store boundaries also apply if the
caller has leaked 53-bit precision:

| Component | Scaled offsets | World translation stores |
|---|---|---|
| brush | `qx = S(R(P.s × d0))`, `qz = S(R(d2 × P.s))` | `x = S(R(P.tx + qx))`, `z = S(R(P.tz + qz))` |
| game | `qx = S(R(P.s × d0))`, `qy = S(R(d1 × P.s))`; first store `d2s = S(d2)`, then `qz = R(d2s × P.s)` kept in x87 | `x = S(R(P.tx + qx))`, `y = S(R(P.ty + qy))`, `z = S(R(P.tz + qz))` |

The brush does not compute y. The game's extra composed matrix/scale work does
not change these translations. Evidence: brush dot/order/scale stores
`005815B6–005815F9`, translation stores `005815FB–00581615`, shortcuts
`005815AD–005815B4`, `0058161E–00581631`; game dots and z spill
`005BC426–005BC46D`, scale/translation `005BC471–005BC4B9`, shortcuts
`005BC412–005BC420`, `005BC5FF–005BC67B`.

**EF19 conformance vectors (proposed, unexecuted).** Supply transform flags
explicitly; normal numeric vectors use the normal 24-bit precision.

| ID | Fixture | Required result |
|---|---|---|
| EF19-1 | File matrix words 1 through 9, translation (10,11,12), scale 2, revision `123456AB` hex; destination flags initially zero | matrix rows (1,2,3), (4,5,6), (7,8,9); translation (10,11,12); scale 2; flags 7; revision `AB`; writing ends with word `000000AB` |
| EF19-2 | Legacy numeric identity, zero translation, scale 1, revision 0 | flags 5, revision 0; this is not a flag-zero identity |
| EF19-3 | L translation (1,2,3), nonzero flags; P rows (1,2,3), (4,5,6), (7,8,9), scale 2, translation (10,20,30), nonzero flags | game world (38,84,130), cell (2,8); brush current (38,130) |
| EF19-4 | L.tx and L.tz are negative zero, P flag byte zero; repeat with L flag byte zero and P negative-zero translation | direct paths preserve negative-zero bits; the first test has precedence if both flags are zero |
| EF19-5 | Both flags nonzero; L translation (1,1,1); P row 0=(1,`−16777216`,`16777216`), scale 1, translation zero | x=0: first sum rounds `16777216+1` to `16777216`, then cancellation gives zero; x+y+z order would give 1 |
| EF19-6 | Both flags nonzero; L translation zero; P scale 1, P.tx=negative zero and row 0 all positive zero | x is positive zero under nearest rounding; a direct zero-offset shortcut would incorrectly retain negative zero |

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

### 6.2 File queue and bootstrap lifetime (EF18) *[compat]*

`EffectsParser` and `EffectsBootstrap` are export labels for functions in the
manager/parser arrangement, not two additional independently constructed COM
owners. The constructor labelled `EffectsParser::EffectsParser` (`0059EEC0`)
constructs `cSC4EffectsParser` (effects-parser §5). The receiver of
`EffectsParser::ParseQueuedFiles` (`0058E4E0`) and
`EffectsBootstrap::LoadAllEffects` (`005945B0`) is the effects manager.
The file queue is the separate `nSCRes::cFileParser` at manager `+C78`;
the effects command parser is at `+C7C`. Init obtains CLSID `5CF9459A`,
IID `1CF94583` for the former, initializes it, and obtains CLSID `A956AD14`,
IID `6956BABC` for the latter (`00594CB1–00594D20`). These boundaries are
supported synchronous operations, not an Unsupported feature or a worker job.

**Queue contract.** Installing a command parser through file-parser slot 5
retains it and initializes its command dictionary; replacing a different parser
retains the replacement before releasing the old one (`00680930`). Queue slot 6
appends a copied input path and its parent directory, retaining insertion order
and duplicates; flag bit 0 marks a required input, and flag bit 1 requests
a directory watcher (shared for inputs with the same parent directory). Appending
sets `filesModified` true (`00681DA0–00681F52`). The queue is an input-file list,
not a list consumed by a successful parse. `NumInputFiles` and `InputFilePath`
inspect this retained list (`006809C0`, `00680A30`). The modified flag is a byte,
explicitly settable, and queried by slot 14 (`00680900`, `00680920`).

Calling `ParseQueuedFiles` asks file-parser slot 8 to parse **all** registered
inputs synchronously, in list order; slot 8 delegates to slot 7 with start=0,
count=NumInputFiles (`00680990`). With a ready installed parser the file parser
clears `filesModified`, begins a parse session, parses each path with its saved
parent directory, and finishes the session. On an error it records a diagnostic
and still finishes the session; successfully published earlier commands are not
rolled back. The command grammar and owner writes remain effects-parser and §19
contracts. This is repeated parsing of the retained inputs, not creation of a new
effects manager. The ordinary all-success result is true, with diagnostic
`Successfully parsed %d lines`; file-open failures produce
`Couldn't open file '%s'` (`00680EA0`, `006812F0`, continuation `006813ED`).
File exceptions are caught per input and processing continues with the next
queued input. A required input's exception stores its diagnostic and makes the
batch fail. An optional input's error code 2 (cannot open) is suppressed; another
optional-input exception stores its diagnostic but still does not make the batch
fail. A later successful batch result can therefore overwrite that optional
diagnostic with the success line count. A failure already recorded for a required
input remains a failure through later inputs. Evidence: per-input catch
`006813A2–006813E9`, accumulation `006813ED–0068140A`.
The literal not-ready path (no parser, or its ready query is false) stores
`Not ready to parse` but returns true because its error flag remains clear
(`00681450–006814AF`): preserve this odd success result as a quirk.

The manager does not convert the result into a public boolean or throw a new
queue exception. False: retrieve slot 10's diagnostic, pass it to the UI service
slot 45 when that service exists, and send no completion message. True: pass
null to that UI slot (clear the error), then, if the message service exists, send
a standard message of type `C9F775BB` with dispatch argument 0, and release the
temporary message. There is no separate success latch: repeated successful calls
send repeated completion messages. The two services are independently optional.
Evidence: `0058E4E3–0058E526`, `0058E526–0058E56F`.

**Bootstrap selection and order.** The ordinary domain has initialized manager,
file parser, command parser and framework services. Read command-line option
`packedeffects`, lowercase its string, and choose packed loading by default.
Literal `off` disables it; literal `on` enables it; other strings leave the
default enabled. No whitespace trimming is added (`00594610–005946A6`).
Build `main.fx` beneath the directory returned by `00602250`, then begin one
collection (§18.6). Within that collection:

1. If packed loading is enabled and the resource service exists, request key
   `(type EA5118B0, group EA5118B1, instance 1)` as IID `CA51189B` and, when
   obtained, apply its collection reader (slot 5) to this collection. A failed
   resource request does not abort bootstrap.
2. In that same enabled/resource-service branch, if `main.fx` exists, attach the
   effects parser to this manager, install its adjusted parser interface in the
   file parser, append `main.fx` with flags 3 and call `ParseQueuedFiles`. If the
   file does not exist, call `DoRenderPropertiesChanged` instead. This file test
   and render-properties alternative are skipped when the resource service is
   absent (`00594711–005947D9`).
3. With a resource service, enumerate all keys of type `EA5118B0` in the service's
   enumeration order. Skip **every** key whose instance is 1, regardless of group;
   for each other key, request IID `CA51189B` and apply a successfully obtained
   reader to the collection. Release the previously held resource before each
   such request. This enumeration also happens in unpacked mode
   (`005947DE–00594888`).
4. In unpacked mode, attach/install the parser, append the data-directory path
   `Effects\\config.fx` with flags 3, then append the previously built `main.fx`
   path with flags 2. Parse the retained queue once, send progress message 2 with
   an empty string, then invoke effects-parser slot 2 to detach its manager and
   collection interfaces (effects-parser §11). The code does not pretest these
   two files and does not suppress this cleanup on an ordinary false parse
   result (`0059488A–005949B1`). The packed `main.fx` branch has no matching
   explicit detach here.
5. End the collection, release the resource enumeration and last held resource,
   and destroy temporary paths (`005949CD–00594A24`).

The bootstrap has no explicit aggregate success return, no rollback, and no
implicit effect activation. Its resource readers and file commands publish into
the same collection in the order above. Reload during ticking follows §6.1;
shutdown releases the command parser, shuts down the file parser and releases
it. No separate bootstrap object must be persisted.

**EF18 queue/bootstrap conformance vectors (proposed, unexecuted).** Host services
must record calls individually, not return a prebuilt bootstrap result.

| ID | Fixture | Required result |
|---|---|---|
| EF18-1 | Queue contains A, B, A; parse twice, all succeed; both reporting services present | inputs visited A,B,A twice; queue remains length 3; two UI clears and two `C9F775BB` sends |
| EF18-2 | File parser returns false with diagnostic D; UI absent, message service present | diagnostic queried, no completion message; earlier publications retained |
| EF18-3 | Default packed option, base resource succeeds, main.fx absent, extra keys have instances 1,2,3 | Begin; base reader; render-properties refresh; skip instance 1; readers 2 and 3 in service order; End; no file append |
| EF18-4 | Option `OFF`, resource service present, extra resource R; required config.fx fails, optional main.fx succeeds | Begin; no base request; R read; attach/install; append config flags 3 then main flags 2; parse both; error report; progress 2; detach; End; R's publications retained |
| EF18-5 | Packed enabled, resource service absent, main.fx exists | Begin then End; no file parse, no render-properties alternative, no enumeration |
| EF18-6 | No installed command parser | file parser stores `Not ready to parse` yet returns true; manager clears UI error and sends completion if those services exist |
| EF18-12 | Required config.fx succeeds, optional main.fx cannot be opened (error code 2) | optional exception suppressed; batch succeeds; success UI/message path; retained queue still has both paths |

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
  directly. Here identity means a zero transform flag byte; see §3.1 for the exact
  shortcut precedence, legacy decoding, product order and binary32 stores.
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
- **SetTransform** (`0x005BC3B0`): computes the world position by §3.1. `cell := (floor(x / 16), floor(z / 16))`. A change of
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

### 18.6 Collection publication and reload (EF18) *[compat]*

The collection interface in §18.1 is a publication service, not a mutex.
`BeginCollection`/`EndCollection` delimit a rebuild of bindings and notification
indices; there is no nesting counter or transaction rollback. The generic
collection's Begin (`0058BEF0`) clears the effect-ID binding vector and message
trigger vector and clears its camera-present byte. It **retains** the description
arrays and their name maps, including component descriptions covered by
effects-parser §11.1. `SetCameraParams` (`00588810`) copies the float curve and
four following scalar words and sets camera-present true; Begin clears only that
presence flag, not the stored curve/scalars. The manager's Begin (`0058F690`) first unsubscribes its
message target from every previous trigger's message ID when a message service
exists, clears its ID-pair and trigger lookup tables, then performs that generic
Begin. Existing runtime effects are not stopped by this operation.

**Visual-effect publication and copies.** `AddEffectDescription` (`0058D560`)
folds the supplied name to lowercase and selects a slot by the pair
(folded name, **exact description priority**), using `00587170`. A matching pair
is replaced at the same stable index; a different priority appends a description
and an additional name entry. It assigns a value copy of the description before
performing fixups. It does not retain a pointer to the caller's mutable record,
and does not reject unresolved references or validate a preexisting nonnegative
index against table bounds.

The stored trailing description string is folded to lowercase. For each copied
component whose signed description index is negative, fold its name, then resolve
by type: particles (0), model particles (16), decals (1), or that type's component
directory (3–15). A hit replaces the index; a miss leaves the negative value.
Meta type 2 instead gets index 0 without checking that its child name exists.
Other types retain their index. A component whose index was already nonnegative
is skipped entirely, including its name folding. This fixes names only at
publication time; later definition of a missing component is not a retroactive
fixup (`0058D6C2–0058D77B`).

For every copied parameter binding, fold its resource name. If binding flags bit
0 is set, look up that name in the collection's shake-description map; otherwise
if bit 2 or bit 3 is set, use the light-description map. On a hit replace the
binding's last word with that description index; a miss preserves it. Other flag
combinations preserve that word. Bit 0 takes precedence. These are literal map
choices (`0058D781–0058D805`), not a generic asset-validation pass.
`GetEffectDescription` (`0058B500`) folds its lookup name, returns false on a
miss without writing the destination, and on a hit returns true and assigns a
value copy if a destination was supplied. A null destination is an existence
query. This is separate from the manager's max-priority selection in §4.1.

**IDs and messages.** `AddEffectBinding(name,group,instance)` (`0058B580`)
appends a copied string and the two unsigned words; it does not resolve the name
or create a runtime effect. Parser callers fold their own names (§19.4).
`AddMessageTrigger` (`0058B8B0`, interface thunk `0058C0E0`) appends a copied
(message ID,string) record without deduplication. End (`005900A0`) retains the
current camera parameters for manager use, resolves each ID binding's name
against the effect-name map and inserts (group,instance) → description index into
the manager lookup. Missing names add no mapping. All matching name entries are
visited in native name-map traversal order; insertion is **first-wins** for an
already present ID pair (`0058F230`), including conflicts between successive
binding records. It does not substitute the max-priority rule from Create by
name for this operation. The runtime ID lookup therefore uses the binding chosen
at collection End, not a fresh name-priority query.

End also assigns message-ID → trigger-vector-index in vector order (later
duplicate IDs overwrite earlier indices), and subscribes the manager message
target once for each trigger record, including duplicates, when the message
service exists (`00590270–00590370`). It does not clear descriptions or activate
runtime effects. A subsequent Begin unsubscribes those records and clears these
derived tables. Thus reloading an empty collection retains descriptions but
removes its previous IDs/triggers and camera-presence state. Copy/publication
failure is not transactional: there is no owner undo of earlier copies or calls.

| ID | Fixture | Required result (proposed, unexecuted) |
|---|---|---|
| EF18-7 | Publish effect `Smoke`, priority 1; republish `SMOKE`, priority 1; publish `smoke`, priority 2 | first slot replaced, second priority has a second slot; caller records remain independent |
| EF18-8 | Copied components: unknown particle name/index −1; unknown meta/index −1; mixed-case particle name/index 7 | first name folded and index stays −1; meta name folded/index 0; last name and index untouched; publication does not reject |
| EF18-9 | Bind distinct single-description names A then B to the same ID pair, End | pair resolves to A; a missing-name binding creates no pair |
| EF18-10 | Two triggers with the same message ID; End then Begin with message service | End's lookup selects second trigger; two subscribe calls then two unsubscribe calls; trigger/binding lists cleared; descriptions retained |
| EF18-11 | Publish effect with one missing component; add that component afterwards; inspect stored effect | index remains unresolved until the effect is republished; no implicit validation/republication |

### 18.7 Native collection and queue observation (EF20) *[compat]*

**DIFFERS from treating the collection getters as enumeration or the resource
reader as a read-only iterator.** This section describes the Windows 1.1.641
observation boundary. Offsets below are native capture facts, not requirements
that a replacement use the same containers internally. A snapshot and a trace
of publication calls answer different questions: a snapshot describes retained
state; a trace also describes overwritten definitions and bootstrap choices.

#### 18.7.1 Getter call and ownership contract

Use the collection sub-object as the receiver: manager primary pointer plus
`94` hex, or resource primary pointer plus `4`. Windows calls are `__thiscall`:
ECX is that receiver, explicit arguments occupy successive four-byte stack
slots, and the callee removes them. All seven named getters below return a C++
**bool in AL**, not a pointer, index, HRESULT, or a defined whole-EAX value.
The upper 24 bits of EAX are unspecified. The two-argument getters pop 8 bytes;
the component getter pops 12. Slot numbering starts at QueryInterface = 0.

| Slot | Method/address | Explicit arguments in declaration order | Successful nonnull output |
|---|---|---|---|
| 6 | GetEffectDescription `0058B500` | nonnull NUL-terminated byte name, `cSC4EffectDescription*` destination or null | assigns a value copy of the selected record |
| 8 | GetParticleDescription `00588DD0` | name, `cSC4ParticlesDescription*` destination or null | assigns a value copy, including owned members |
| 10 | GetDynamicParticleDescription `00588E50` | name, `cSC4DynamicParticleDescription*` destination or null | assigns a value copy |
| 12 | GetDecalDescription `00588ED0` | name, `cSC4DecalDescription*` destination or null | assigns a value copy |
| 14 | GetShakeDescription `00588F50` | name, `cSC4ShakeDescription*` destination or null | assigns a value copy |
| 16 | GetLightDescription `00588FD0` | name, light-description destination or null | copies the vector of three-float values, vector of floats, and final word |
| 18 | GetComponentDescription `00589070` | name, 32-bit component type, component-description **pointer-to-pointer** or null | writes the stored object pointer and retains it through that object's slot 4 (`+10` hex) |

All fold a copy of the input name bytewise with the CRT lowercase operation
before searching the appropriate dictionary. A miss returns false without
writing the output. A hit with null output returns true without making a
description copy or acquiring a component reference. A value destination must
already be a valid constructed object of the right type; it is not an untyped
byte buffer or an out-pointer to native storage. Its allocations remain the
caller's responsibility. A component output instead owns one acquired reference;
release it with the component-description interface's slot 5 (`+14` hex), not
with a guessed COM slot 2. Component types 3 through 15 select separate buckets;
there is no bounds check for an invalid type. Null name is not an enumeration
request and is outside the getter's valid domain. See effects-parser §11.1 for
the separate null-name **registration** rule.

The visual getter uses the first matching dictionary entry, not the manager's
maximum-allowed-priority selection. Consequently one lookup per unique folded
name cannot recover the other priorities. Nor can named component lookup recover
anonymous descriptions. There are **no count, index-getter, or enumeration slots**
in this 24-slot interface; slot 23 is a deleting-destructor thunk, not a count.

Even a null-output getter overwrites the collection's scratch string at `+10C`.
It can allocate and is not strictly read-only or reentrant. An observation that
promises no native mutation must use the container framing below rather than
call these getters. Evidence: `00588DD4–00588E41`,
`00588E96–00588EC1`, `00588F1D–00588F43`, `00588FA0–00588FCA`,
`00589020–0058905F`, `00589078–005890F8`, `0058B546–0058B573`;
slot 23 `0058FE10` adjusts the receiver and enters destructor `00590080`.

#### 18.7.2 Retained description and name framing

Let **C** mean the collection sub-object, never the primary manager. All offsets
in this subsection are relative to C. Pointers and scalar words are 32 bits,
little endian. A vector is three words: begin, logical end, allocation end.
Only the half-open interval [begin, logical end) contains records. Equal begin
and end means empty, including the all-null representation; spare capacity is
not a record. Divide the byte difference by the element stride, checking exact
divisibility. Strings here are three-word byte sequences (begin, end, capacity),
with length end minus begin; exclude capacity, allocation addresses and the
terminating byte from the recorded value. These are not on-disk strings.

| C offset | Retained sequence | Element stride / interpretation |
|---|---|---|
| `+18` | visual descriptions | `38` hex, inline; §3 memory record |
| `+38` | particle descriptions | 4, each element is a pointer to its description object |
| `+58` | dynamic/model particle descriptions | 4, pointers |
| `+78` | decal descriptions | `64` hex, inline |
| `+98` | shake descriptions | `28` hex, inline |
| `+B8` | light descriptions | `1C` hex, inline; vector triples at record +0 and +C, final word +18 |
| `+C4` | component directories | `20` hex per directory, types 3 through 15 in ascending order |
| `+F0` | effect ID bindings | `14` hex: string at +0, group word +C, instance word +10 |
| `+FC` | message triggers | `10` hex: message ID word +0, string at +4 |

Each component directory has its name dictionary at +0 and a vector of
description pointers at +14. The normal constructed directory count is 13;
the actual vector end still defines the readable interval. Record the slot
index even for an anonymous component or a component with no incoming visual
reference. Pointer arrays are not arrays of inline objects. Neither a null
pointer nor a description's first word is a string or a description count.

Name dictionaries begin at C+4 (visual), +24 (particle), +44 (dynamic particle),
+64 (decal), +84 (shake), +A4 (light), and at each component-directory start.
A dictionary header occupies `14` hex bytes: its bucket vector is at header+4
and its element-count word at header+10. The bucket vector uses the same
three-word framing and has four-byte bucket-head pointers. Visit buckets in
ascending index, following each nonnull node's next pointer at node+0 until
null. A node holds the three-word name at +4 and the description-index word
at +10. Count nodes, not distinct names; visual priorities can share a name.
Preserve both the description vector and the sequence of (name,index) entries.
The header count is a consistency check, not permission to read that many
contiguous nodes. Detect cycles, repeated nodes, unreadable spans and out-of-range
indices and reject an incomplete capture rather than silently truncate it.
Evidence: lookup `0051C9F0–0051CA50`, publication `0058D560`, collection merge
`0058CA50`, callback traversal `00587230`, constructor `0058C580–0058C6DA`.

For a visual record, follow its component vector at +8 with stride `6C` hex,
its parameter-binding vector at +14 with stride `18` hex, and its string at
+2C. Each parameter binding is flags at +0, a three-word string at +4, a float
word at +10, and an index word at +14. Preserve the component's **memory**
transform as specified in §3.1; never feed it to the legacy file decoder.
Exclude padding and pointer/capacity words from value comparisons. Copy float
bits, including signed zero and NaNs, without conversion through decimal text.
For the other description families, record the typed value fields and their
owned sequences using their description contracts (§§8–9, §19 and the referenced
component specs); the four-byte vector element above is only the route to that
payload, not its serialized representation. Record family, stable vector index,
all names referring to that index, and payload separately. Counting/name
equivalence alone must be labelled as such; it is not payload equivalence.

Camera presence is a **byte at C+D0**. The camera value at C+D4 is a float-vector
triple followed by four scalar words at C+E0, +E4, +E8, +EC. Capture both the
presence byte and the stored value: Begin can clear presence while leaving old
values intact. End copies that value to manager primary +CB4 (C+C20), with its
four scalars at primary +CC0 through +CCC, independently of whether it is newly
present. Capture this retained manager copy separately if comparing End's
publication. Evidence: `00588819–00588849`, `0058BF25`, `005900AE–005900D7`.

The derived ID dictionary is at manager primary +D00 (C+C6C); the derived
message dictionary is at primary +D14 (C+C80). Both have the same header/bucket
framing. ID nodes are next pointer, group, instance, description index, each
one word; message nodes are next pointer, message ID, trigger-vector index.
Keep these derived tables separate from the ordered source bindings/triggers:
duplicates and unresolved bindings make their counts differ (§18.6).
Evidence: `0058F699–0058F719`, `005901E7–0059021D`, `0058F263–0058F2AA`.

#### 18.7.3 Resource reader is publication, not inspection

Resource IID **CA51189B** returns the resource's primary interface, whose vtable
is `00AA0AC0`. Slot 5 (`+14` hex, `005A3BC0`) has one explicit argument: a
nonnull destination effects-collection pointer. It is a **void** operation,
callee pop 4, passing resource+4 as the source collection to `0058D0E0`.
It does not take a stream, an IID, an output collection pointer, a Boolean
flag or an enumeration callback. Resource IID **C97CF5CE** instead returns
resource+4, with the ordinary collection vtable `00AA0A60`. Both successful
QI paths acquire a reference; balance that acquisition through the returned
interface. These are distinct from the resource's stream Read at primary slot 7
(`005A51E0`). Evidence: `005A3B40–005A3B85`, `005A3BC0–005A3BC7`,
constructor `005A4CC0` and the two vtable images.

The collection reader changes the destination and can change the source.
In particular, when the destination visual vector is empty, it swaps the
description dictionaries/vectors, component directories, binding vector and
trigger vector between the two collections. If source camera-present is true,
it copies the source camera value to the destination; this path does not set
the destination camera-present byte. For a nonempty destination, source byte
C+108 selects callback publication when nonzero, or the direct merge when zero.
That byte is initialized to zero. The reader itself does not supply the
manager's Begin/End bracket. Observe the game's bracket and its post-call state;
do not add a second bracket or invoke the reader to populate a capture sink.
Evidence: empty test and swaps `0058D0E0–0058D475`, camera copy
`0058D475–0058D4AE`, branch `0058D4B1–0058D4C9`.

#### 18.7.4 Read-only queue/bootstrap capture protocol

Capture on the owning thread at a quiescent boundary after the naturally
executed collection End/bootstrap/parse call has returned, before another owner
operation. Borrow already-held native identities for this interval; do not
retain description pointers for later dereference. Copy readable value spans
into capture-owned memory before returning. Do not call getters, Begin, End,
resource publication, ParseFiles, SetParser, AddInputFilePath, flag setters,
Start/Stop or reload to manufacture a snapshot. Do not turn a twice-read header
into a synchronization guarantee: if a concurrent mutation cannot be excluded,
refuse that sample. Preserve a validity result and exact compared scope.

For the queue, read manager primary +C78 as a borrowed file-parser pointer and
+C7C as the command-parser identity. A null file parser means absent, not an
empty ready queue. With a file parser **F**, its installed command parser is at
F+10; input vector at F+14 has stride `2C` hex. Each input contains a parent
directory string object at +0, path string object at +14, and required byte at
+28. These string objects include a vptr: byte begin/end/capacity are at
string-object +4/+8/+C. Capture strings by length without calling their vtables.
The required byte retains original flag bit 0; watch-request bit 1 is not saved
in that byte. Capture ordered duplicates. Read filesModified at F+34 (one byte)
and diagnostic string object at F+20. Parsing leaves the input vector retained,
even on success. Evidence: `00680BE0`, `006809C0–006809D9`,
`00680A30–00680A47`, `00681E10–00681E65`, `00680900–00680923`.

For clients allowing pure accessor calls rather than a strictly no-write memory
snapshot, file-parser slot 12 returns the 32-bit input count in EAX (no args),
slot 13 takes a 32-bit zero-based index and returns a **borrowed const char***
in EAX (pop 4), slot 10 returns the borrowed diagnostic const char* (no args),
and slot 14 returns filesModified in AL (no args). Index must be below count;
slot 13 does not check it. These accessors do not dequeue, parse, clear errors or
clear filesModified. Copy borrowed characters within the same quiescent interval;
never free them. Slot 9 is the flag setter and is excluded from observation.

There is no independent EffectsBootstrap object or persistent success/packed-mode
latch to poll. For bootstrap **history**, observe the natural `005945B0` call and
record its input option/resulting packed choice, requested resource keys and
outcomes in order, collection bracket, file registrations (including original
flags), parse outcomes/diagnostics, and C9F775BB sends. A post-bootstrap snapshot
alone cannot recover failed resource attempts, overwritten descriptions, the
original watch-request bit per file, or how many successful parses occurred.
Mark this history unavailable if observation began late; do not infer it from an
empty queue or filesModified=false. This is a capture-format rule, not a new
native state field. Use §6.2 for the behavior of each observed operation.
An observer needing tick framing also records primary +1B8 (u32 time), +1BC
(float bits), +C84/+C8C (LOD/offset), and the new and active index vectors at
+BA0/+BAC (stride 4); no list-node traversal is involved in those vectors (§2).

#### 18.7.5 Conformance vectors (proposed, unexecuted)

These are spec vectors for an implementation/capture lane; no native game run
or engine test execution is claimed here.

| ID | Fixture/operation | Required observation |
|---|---|---|
| EF20-1 | Two visual records named `smoke`, priorities 1 and 3; query `SMOKE` with null output | AL true, scratch name changed, no copied record; enumerate two indices and both name nodes, regardless of unique-name count |
| EF20-2 | Missing particle name; valid destination prefilled with sentinel fields | AL false; destination unchanged; whole EAX must not be tested as a Boolean |
| EF20-3 | Existing type-6 component; lookup once with null output, once with object-pointer output | first acquires no reference; second writes stored pointer and retains once; caller balances through description slot 5 |
| EF20-4 | Type-6 vector has one named entry and two anonymous registrations; capacity exceeds logical size | snapshot has three indices and one name node; no capacity entries or generated anonymous names |
| EF20-5 | Two duplicate ID bindings, one unresolved name, and two triggers with one message ID; End | preserve all source records; derived ID first-wins and derived message points to last trigger; derived counts are not source counts |
| EF20-6 | Camera value published, End, then Begin | collection presence false, old collection value and manager End-copy still recorded separately; snapshot must not zero them |
| EF20-7 | Resource slot 5 naturally publishes into an empty visual collection | source/destination description storage exchanged; observer records post-state without invoking reader again; no Boolean return or implicit Begin/End inferred |
| EF20-8 | Queue two identical paths, required bytes 0 and 1; successful parse | two retained inputs in original order, same paths and parent directories, filesModified false; successful parse does not imply queue count zero |
| EF20-9 | Same final queue produced by different watch flags or repeated parses; observer attaches afterwards | snapshots can match; original per-input watch flags and parse count are unavailable, not reconstructed as false/one |
| EF20-10 | Empty vector with spare allocation, or malformed end/cycle while sampling | first records zero elements; malformed/unreadable sample is refused and never counted exact |

### 18.8 Natural-call observation and complete description payloads (EF21) *[compat]*

**DIFFERS from obtaining a resource by another service request, or assuming that
a city-load/month notification is a lock.** The observation contract below is
for Windows 1.1.641. It permits a default-off observer to record calls the game
already makes. It supplies no new native calls and no requirement to reproduce
allocator addresses. The attachment addresses are executable-version-specific
evidence, not additional interface slots. Installing interception is a harness
operation; the original has no observer-registration API for these events.

#### 18.8.1 Attachment, borrowed resource identity and event lifetime

Attach before the manager's natural Init/bootstrap if complete history is
required. At `005945B0`, ECX is the manager primary M; there are no explicit
arguments and no Boolean result. Its direct caller is Init at `00594D72`.
Record entry/normal return separately from parse success. Preserve the actual
option-service reply, option bytes and resulting packed choice (§6.2), rather
than treating the option's presence or this routine's EAX residue as success.
The resulting packed-choice byte is at bootstrap-frame ESP+12 at `00594723`
and `0059488A`; that frame offset is valid at those sites only. The original
does not store that choice in a persistent manager field.

Observe the **existing** resource-service slot-4 calls at `0059475A` and
`00594861`. At callee entry, ECX is the service, and the five stack arguments
after the return address are: pointer to a three-u32 key (type, group,
instance), IID `CA51189B`, pointer to the resource-primary output cell, zero,
zero. Copy the key before returning to its caller. Capture AL at return and,
only on success, the pointer in the original output cell. This is the live
resource primary R, not an effects collection and not a persist-base pointer.
The fixed request is `(EA5118B0,EA5118B1,1)`; the enumeration path supplies the
enumerated key and skips instance 1. Failed attempts remain events even though
they produce no publication. Evidence: argument construction
`0059472D–0059475F`, `00594822–00594866`.

The two natural publication dispatches are `00594768` and `0059486F`.
Resource primary slot 5 resolves to `005A3BC0`: ECX=R, one stack argument
destination C, void return, callee pop 4. Source C is R+4; destination is
M+94 in bootstrap. Copy the source payload **before** the operation, then
source and destination after it, because the empty-destination case swaps
storage (§18.7.3). A post-publication resource can consequently be empty even
when the requested directory was populated. The incoming resource reference
belongs to bootstrap. An observer neither acquires nor releases it and must
finish copying while that reference is live. The enumeration loop releases
the previous resource before requesting its successor (`0059483B–00594849`);
final cleanup releases the remaining resource (`005949E1–005949EB`). There
is no stable manager field containing this last resource. A late observer
reports the resource/history as unavailable; scanning manager memory or
requesting the same key again does not recover the original lifetime.

For a complete trace, distinguish the natural primary stream Read
(`005A51E0`, slot 7, stream pointer argument) from the void publication slot.
Record the stream identity and actual result, ordered primitive input replies
or an independently copied original input stream, and resulting collection.
Do not seek, read, close or retain that borrowed stream to fill gaps. A resource
cache hit may yield publication without a new Read; absence of Read is not an
empty input or successful read. The Read's root-version rejection (outside
3..4) is a failure, not a new empty directory (§3). Reference-count and stream
lifecycle events must be observed where naturally executed; final collection
contents cannot establish how often loading, QI, serialization or release ran.

At collection entry/publication calls copy the caller's name, description and
type/index inputs before the original can fold, replace or exchange them.
Capture returned values at their declared widths (§18.7.1), and retain both
input events and post-state. Begin/End, queue registration flags, command-parser
installation, file-parser parse AL/diagnostic, and message sends remain ordered
events (§6.2, §18.6 and §18.7.4). In particular `0058E4E0` is a **void manager
wrapper**: its call of file-parser slot 8 at `0058E4EB` supplies the parse AL;
the wrapper's return does not. The successful path sends `C9F775BB` at
`0058E566`. For the natural file-parser slot-6 registration, capture its path
and original u32 flags at entry, including watch bit 1; retained required-byte
state cannot recover that bit later. Reentrant events retain their nesting and
order. Preserve all registers, flags, calling convention, return channels and
exception paths; observer failure marks a capture invalid without skipping,
repeating or changing the original operation. No getter, QI, Begin/End, Parse,
resource request or message send is added merely to make observation possible.

#### 18.8.2 Quiescence, including reload

There is **no native city-load or month callback in these effects methods that
provides a collection/queue mutex or a published snapshot generation**. A
callback label alone is therefore insufficient evidence of synchronization.
On a proven single owner thread, a synchronous copy after the outermost
bootstrap has returned, or after the outermost effects tick has returned, is
between these methods' writes provided the observer makes no calls that pump
messages or reenter the owners. This is an explicit harness precondition, not
a claim that any plugin callback with the same calendar label has that order.
Validate the identity/thread and outermost-operation depth in the trace; if
another writer cannot be excluded, refuse the sample (§18.7.4). Never move a
borrowed pointer to a background worker for later traversal.

Reload is especially relevant. `DoEffectsTick` (`00593D00`) checks the file
parser's modified byte; when true it calls Begin (`00593D28`), parse
(`00593D2D`), End (`00593D36`), sets M+B8C=1, updates active effects, clears
M+B8C, then calls command-parser slot 2 (`00593D56`). A parser success message
can be dispatched **inside parse**, before End and active-effect repair. It is
not a commit barrier. End-return is sufficient for that collection's fixups,
but a combined collection/queue/live-manager snapshot must wait for the whole
outer reload/tick to return. M+B8C is an update-phase flag, not a lock covering
Begin/parse/End. Bootstrap similarly ends its collection at `005949D1`, then
releases enumeration/resource temporaries before returning. Init performs
further initialization after its bootstrap call. A load sample intended to
describe initialized manager state must follow outer Init return as well.
Do not dispatch another event while copying; if observation attaches mid-call,
history and the current generation are incomplete until a verified outer
boundary. Capture a fresh generation after every observed reload even when
names/counts happen to match. Evidence: instruction bytes
`00593D06–00593D59`, `0058E4EB–0058E576`, `005949CD–00594A24`.

#### 18.8.3 Payload value types and framing

All offsets/strides below are hexadecimal relative to the **description's own
start D**, not M or C. u8/u16/u32 mean exact-width unsigned values; s32 means
a signed word; f32 means the four stored binary32 bits. Compare float bits,
including signed zero and NaN payloads; no float-to-double canonicalization or
decimal round trip. V(T) is the three-pointer vector in §18.7.2 whose elements
are T. S is the three-pointer owned byte string there. Preserve lengths,
duplicates and order. A triple has three successive f32 values and stride C.
Do not compare allocator state, vptrs, reference counts or padding as payload.
Addresses may serve as trace-local identities with alias relationships, never
as expected replacement addresses. Anonymous and named descriptions have the
same payload layout; anonymity changes directory naming, not the object type.

**Particle description:** pointer-vector entries address D directly, with no
vptr/base adjustment. The allocation is `208` hex bytes, base value extent `204`;
D+204 is its intrusive reference count, excluded from payload. In particular
the misleading exported name of `005B1C80` is not a payload assignment: the
instructions decrement +204 and free on zero. Base assignment copies payload
but leaves this count alone (`005B1CB0–005B1CC2`).

| Particle coordinate | Complete retained value layout |
|---|---|
| 00,04,08 | three u32 flag words; retain full memory values, not stream masks |
| 0C,10,14 | f32 lifetime endpoints and scalar |
| 18 | u32 loop/count word |
| 1C | f32 |
| 20..84, every 4 bytes | f32 scalars, pairs and triples in native order; includes bounds and direction/acceleration data |
| 88 | V(f32), stride 4 |
| 94 | V(triple), stride C |
| A0,AC,B8,C4 | four independent V(f32), stride 4 |
| D0 | u32 scalar texture/model resource ID |
| D4,D5 | two u8 mode values; D6..D7 padding excluded |
| D8..104, every 4 bytes | f32 |
| 108 | V(wiggle), stride 1C; each element seven successive f32 at 00..18 |
| 114,118,11C,120,124,128 | f32 |
| 12C | V(f32), stride 4 |
| 138..15C, every 4 bytes | f32 |
| 160 | S, effectBase bytes |
| 16C,16E | two u16 values |
| 170..1A0, every 4 bytes | f32 |
| 1A4 | V(f32), stride 4 |
| 1B0 | f32 |
| 1B4 | u32 flags |
| 1B8 | V(tractor), stride 20; position triple +00, tangent triple +0C, f32 start/end +18/+1C |
| 1C4 | f32 reset speed |
| 1C8 | copied 32-bit word with no deterministic constructor default (§19.13); retain raw bits with provenance, not an invented zero |
| 1CC | V(timed effect), stride 10; S at element +00, f32 time +0C |
| 1D8,1DC | f32 |
| 1E0 | V(u32 model IDs), stride 4 |
| 1EC,1F0 | f32 |
| 1F4 | V(f32), stride 4 |
| 200 | f32 |

There are no unlisted payload bytes in the base extent. Constructor/reset and
semantic mutations are §19.13, §§19.16–26 and §§19.39–51. The table describes
memory, not legacy stream layout: old/new readers differ, notably retention
of flag words. Evidence: copy `005B17F0–005B1C77`; byte stores
`005B1946–005B1958`, halfword stores around `005B1B00–005B1B08`, nested-copy
calls `005B19FB`, `005B1BE2`, `005B1C0C`; 1C/20/10 strides in
`005B0450`, `00625270`, `005B1680`; readers `005B0B50`, `005B1DC0`.
An uninitialized/indeterminate input word is not suitable for an independently
predicted golden; report its validity separately instead of giving its raw
capture a deterministic conformance meaning.

| Family / extent | Complete payload fields; everything else is identity or stated padding |
|---|---|
| Dynamic/model particle, 40 | vptr/refcount at 00/04 excluded; u32 08; S 0C; six f32 at 18,1C,20,24,28,2C; u32 scalar model ID 30; V(u32) 34 |
| Decal, 64 | u32 flags 00 and ID 04; u8 08,09; padding 0A..0B excluded; f32 0C; V(f32) 10,1C,28; V(triple) 34; V(f32) 40; six f32 at 4C,50,54,58,5C,60 |
| Shake, 28 | f32 length/fade 00,04; V(f32) amplitude 08, frequency 14; f32 aspect 20; u32 table enum 24 |
| Light, 1C | V(triple) color 00; V(f32) strength 0C; f32 duration 18 |

Dynamic assignment is `00583750–005837B5`, stream scalar-width evidence
`005837C0–00583849`; decal assignment `005833F0–0058347A` explicitly copies
two bytes rather than the padding word; shake assignment
`00588080–005880BC`; light getter's typed copies `00589032–00589059`.
Their defaults and command semantics remain §19.13, §§19.27–28 and
§§19.31–38. Keep both dynamic scalar ID and its list even when one is dormant.
Light length's write to the **shake fade** is required (§19.32); a capture
must not attribute that write to an invented light-fade member.

#### 18.8.4 Component object payloads, including unnamed entries

The built-in type-3..8 descriptions have primary vptr at 00 and a refcount
sub-object at 04 (its vptr at 04, count at 08). Payload begins at 0C; neither
the primary vptr nor the secondary identity is payload. Directory index is
separate from object identity: record every vector slot, even when several
slots alias one object or there is no name-map entry.

| Type / native extent | Complete payload |
|---|---|
| 3 brush / 34 | u32 brush ID 0C; f32 rate 10, length 14; u8 apply/one-shot 18; padding 19..1B excluded; s32 level 1C; f32 strength endpoints 20,24, width endpoints 28,2C, target height 30. §8.1, §19.55 |
| 4 attractor / 1C | S at 0C; u8 group selector 18; padding 19..1B excluded. Preserve original string spelling. §19.55 |
| 5 scrubber / 50 | u32 flags 0C, word 10, integer radius 14, demolition flags 18; f32 limits 1C,20; u32 burn/toxic/extinguish 24,28,2C; u32 message and data 30,34; u32 shape kind 38; f32 shape values 3C,40,44,48 and duration 4C. §9.1, §19.55 |
| 6 sequence / 18 | V(sequence element) at 0C; stride 14, f32 duration endpoints +00,+04, S name +08. §19.14 and §19.29 |
| 7 sound / 1C | u32 word 0C, u32 sound ID 10; f32 interval 14 and length 18. §19.55 |
| 8 camera / 18 | u32 flags 0C; u8 zoom/rotation storage 10, u8 11; padding 12..13 excluded; f32 attach radius 14. Zoom and rotation options both write 10; do not split them into invented independent fields. §19.55 |
| 9,10,11,12,13,14,15 | Reserved directory slots, not seven additional built-in description classes. Native resource Read has sections/constructors for 3..8, then version-4 dynamic particles, then visual records; it has no 9..15 payload sections. Empty vectors and empty name maps are complete values for these buckets. |

The generic AddComponentDescription interface nevertheless accepts a caller's
description, asks its slot 0 for the type, and selects bucket (type−3) with
`20` hex-byte stride, **without a type-range check** (`0058AC1C–0058AC2C`).
Thus reserved does not mean forced empty: a foreign extension may publish an
object there. There is no universal native memory layout for that object's
private payload. A nonempty reserved bucket must be reported as an unsupported
foreign payload, with its slot/name/alias framing preserved, unless that
extension supplies its own contract. Do not read it as brush, guess a size,
invoke its factory/serializer, or count pointer equality as payload exactness.
This explicitly defines the closed built-in input domain; it is not a missing
native type-9..15 layout. The manager's generic factory dispatch likewise uses
the published description interface (`0059192A–00591964`).

Evidence for complete built-in extents/identities: constructors `005812F0`,
`00580F10`, `005BC9D0`, `005BDF10`, `005BE300`, `00581C10`; resource Read's
allocation/section sequence `005A56F2–005A5D35` and following dynamic/visual
sections; anonymous publication and ownership §§19.53–55. The table also
covers anonymous descriptions not reachable from any visual component.

#### 18.8.5 Coverage boundary and conformance vectors (proposed, unexecuted)

These contracts enable retained-payload and natural-history comparisons; they
do not turn a successful directory comparison into whole-owner credit.
For cSC4EffectsResource, compare stream inputs/results, reference/serialization
lifecycle and **pre/post** publication, including cache hits and failures.
For cSC4EffectsManager, additionally compare live/new/active effect state,
tick/lifecycle behaviour, publication fixups, derived ID/trigger lookups and
camera state using §§2,4–7,11,18.6–18.7. A parser/bootstrap trace and all retained
description families close the capture8 omissions, but manager lifecycle and
live effects still require their own measured probes. Explicit unavailable,
unsupported or refused samples carry no exactness credit. In particular no
snapshot-only scheme can retrospectively reconstruct a completed bootstrap.

| ID | Fixture / natural event | Required result |
|---|---|---|
| EF21-1 | Observer active before Init; packed resource success | one observed request key/IID/result and borrowed R; source before publication and source/destination after; no additional request, QI or publication |
| EF21-2 | Empty destination; source has two particle records and one anonymous brush | capture source payload before swap; post source may be empty; destination has all three payloads and anonymous slot without an invented name |
| EF21-3 | Enumeration releases R1, request R2 fails, later request R3 succeeds | ordered release/failure/success events; no traversal of R1 after release and no output dereference on failure; R3 publication distinct |
| EF21-4 | Observer attaches after bootstrap; retained manager directory readable | retained comparison possible in verified interval; resource input and bootstrap history unavailable, never reconstructed from manager counts |
| EF21-5 | Reload parse succeeds and synchronously sends C9F775BB before End | message event recorded; combined snapshot waits for outer reload/tick return; End/fixups, live repair and parser cleanup precede sample |
| EF21-6 | Month callback on another thread or reentered during Begin/parse | refuse collection/queue snapshot; same headers read twice do not change refusal |
| EF21-7 | Particle vector has one pointer; D+204 varies, two float members contain 80000000 and 7FC01234 | compare payload float bits exactly; refcount not a payload mismatch; all nested vectors recursively compared at their specified strides |
| EF21-8 | Dynamic description has scalar ID 7 and list [8,8]; one anonymous sequence contains wait then play MixedCase | preserve scalar and both IDs; two ordered sequence entries and exact original name bytes, no name-based deduplication |
| EF21-9 | Decal padding differs; shake table=1; light length command with fade=3 | padding ignored; shake enum compared as u32; light duration and mutated shake fade captured in their separate records |
| EF21-10 | Camera anonymous zoom 5 then rotation 2; no other options | flags 3, byte 10=2, byte 11=0; no independent stored zoom=4 |
| EF21-11 | Type-9..15 directories empty, then a foreign type-12 description is registered | empty case fully represented; later type-12 slot/name framing retained but private payload unsupported and whole-payload sample uncredited |
| EF21-12 | Same final directory after two natural parses with different watch flags | final snapshot may match; trace retains both parses and original flags; diagnostic/modified state compared separately |
| EF21-13 | Resource cache hit publishes without stream Read; another request reads unsupported root version | first records absence of a new Read, not empty input; second records failure and actual post-state, not synthetic successful publication |

## 19. Effects-script command contracts *[compat]*

This section concerns text-description commands, separate from the binary
directory reader and the runtime components in §§3–10. Evidence is static
Windows 1.1.641 export instructions and descriptor data, inspected 2026-10-23.
Conformance examples are proposed, unexecuted predictions. Sections 19.1–19.56
cover the registered command-owner family (EF15); foreign service and numeric
qualification boundaries remain explicit.

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
their x87/binary32 intermediate stores; §14 supplies EFP7's scale/probability
numeric qualification.
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
The `instance` execution contract is §19.4. Evidence:
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

### 19.4 Root identifiers and message-trigger records

The four resource-ID declarations below take **exactly two arguments** (three
token records including the command), not a minimum positional prefix. Missing
or surplus records, including switch records, throw `Wrong number of arguments`
before copying a name or changing a map. There are no default IDs.

| Root literal | Descriptor / handler | Parser map / result |
|---|---|---|
| `textureID` | `00A9F86C` / `0059F890` | texture names, `P+AC` |
| `modelID` | `00A9F87C` / `0059FE50` | model names, `P+B8` |
| `soundID` | `00A9F88C` / `0059FF40` | sound names, `P+C4` |
| `brushID` | `00A9F89C` / `005A0030` | brush names, `P+D0` |

Argument 1 is copied as the key, then each copied byte is lowercased by the
native CRT locale's tolower service (`009F0543` → `009F047B`). The input token
string is unchanged. In the ASCII domain, `Smoke` and `SMOKE` select the same
key. The four maps are separate: declaring a sound does not change a texture
of the same name. Map lookup/insertion takes place **before** converting
argument 2 through ParseUint; a missing key is inserted with unsigned value 0.
On successful conversion its returned 32-bit word overwrites that entry.
Duplicate declarations overwrite; there is no owner range check, resource
lookup, loader call or description construction. On a throwing conversion a
new zero entry remains, or an existing entry keeps its old value. These commands
do not create a component record or apply component options.

Evidence: arity/copy/fold/assignment in `0059F890–0059F975`,
`0059FE50–0059FF35`, `0059FF40–005A0025`, `005A0030–005A0115`;
zero insertion `0059F980–0059F9FD`, especially `0059F9CB` before insertion.
These hooks have no confirmed `c*Command` class name in the export. They are
identified by their positively registered descriptor/hook pairs, rather than
inventing class names from their command literals.

`effectID` (`00A9F8AC`, `0059A640`) requires **exactly three arguments**:
group ID, instance ID, effect name. It converts the first two with ParseUint
in that order, copies/lowercases the name by the same CRT service, then, if
`P+90` is nonnull, calls collection slot `+50(name,groupID,instanceID)`.
It does not resolve the name locally, change the current group-ID word, apply
component options, or check the collection's return. With a null collection,
conversion and folding still occur, but the collection call is omitted.
Bad arity throws `Wrong number of arguments` before conversions.
Evidence: `0059A640–0059A72A`.

`instance` (`00A9FA08`, `0059A730`) is the same registered descriptor at root
and within `effectGroup`. It requires **exactly two arguments**: instance ID,
effect name. It converts the ID, copies/lowercases the name, and, if `P+90`
is nonnull, calls `+50(name,P+49C,instanceID)`. It reads the current group word
at the call, without restoring or defaulting it. A null collection does not
skip conversion/folding. It has the same arity error and no local component
append. Collection duplicate policy and how IDs resolve at runtime belong to
that service. Evidence: `0059A730–0059A819`.

`messageTrigger` (`00A9F8E8`, `0059E820`) asks the positional reader for at
least two values. The first is converted with ParseUint; the second is copied
**without case folding**. It builds a temporary record of that unsigned word
and string, and, if `P+90` is nonnull, passes its address to collection slot
`+54`. It destroys the temporary after the call returns. Extra selected
positional values are ignored by this handler; a null collection still permits
the reader/conversion/string copy. No component options or local description
append occurs. Evidence: `0059E820–0059E8C3`; the owned call boundary is the
record `(messageID,name)`, not a guessed message-system registration algorithm.

### 19.5 Resource blocks and explicit resource import

`effectsResource` is `cEffectsResourceCommand`, descriptor `00A9F918`, enter
`00596810`, exit `00596970`. It requires exactly two arguments: resource
instance ID and resource name. Arity errors precede the check for an already
held resource at `P+94`; a nonnull resource then throws `Can't nest resource
blocks`. Nesting is tested by that pointer, not by generic block-stack depth.
The byte `P+A4` enables resource creation/saving. If it is zero, entry still
performs both checks and enters the generic block, but does not convert either
argument, create a resource, or change the collection.

When enabled, argument 1 is converted with ParseUint. The resource key is three
32-bit words `(EA5118B0,EA5118B1,instanceID)` hex. The live resource manager
at `00B43CA8` is asked through slot `+18` for that key, interface `CA51189B`,
output `P+94`, and final arguments 0,0. The owner ignores the call's status
and tests the returned pointer. A null result is not a local parse error:
entry continues with the old collection/key unchanged. A nonnull result gets
slot `+0C` with the raw name pointer from argument 2, with no local case
folding or numeric conversion; its result is ignored. The key is then copied
to `P+98..A0`; QI for `C97CF5CE` obtains the resource's collection, which
replaces retained `P+90` through the shared reference-assignment service.
Finally the temporary QI reference is released. All enabled and disabled normal
paths then enter the generic block. A manager pointer is required on enabled
entry; this owner supplies no null-manager guard. The shared QI/assignment
contracts govern reference operations, not a command-specific ownership transfer.
Evidence: `00596810–0059696D`.

On exit, if `P+A4` is zero or `P+94` null, the hook does nothing. Otherwise it
asks the same manager through `+7C(P+9C,&segment)` with initially null output.
A false AL throws `couldn't find segment for effect group 0x%08x` with the
stored group word; there is no save or collection restoration. On success it
calls manager `+30(&key,segment)`; false AL throws `couldn't save effects
resource key 0x%08x` with the instance word. Both errors leave the current
resource/collection held, and generic end dispatch does not pop a throwing hook.

After a successful save, entry's previous arbitrary `P+90` is **not** restored.
If `P+8C` is nonnull, QI for `C97CF5CE` on that object supplies the replacement
collection, assigned with shared reference management and its temporary QI
reference released. If `P+8C` is null, `P+90` is cleared before releasing its
old nonnull value. Next `P+94` is cleared before releasing the resource, then
the returned nonnull segment is released. The stored key remains unchanged.
This is save-and-restore-to-base behavior, not a general stack of collections.
Evidence: `00596970–00596AA5`.

`loadResource` is `cLoadResourceCommand`, descriptor `00A9F934`, handler
`00596AB0`. It asks for at least one initial positional value, converts only
the first with ParseUint, and forms the same fixed-type/group key. Extra
positionals are ignored. A null global resource manager makes it return after
conversion. With a manager, it calls `+0C(&key,CA51189B,&resource,0,0)` with a
null-initialized output. False AL throws `Effects resource 0x%08x not found`;
there is no import. With true AL it calls returned resource slot
`+14(P+90)` then releases that resource when nonnull. No guard protects the
import call against a true status with null output; valid successful fixtures
must supply a live resource. The current collection is passed even if null.
Import behavior and resource lookup are individual foreign services; there is
no local component append, P+90 replacement, or resource-block entry here.
Evidence: `00596AB0–00596B6A`.

### 19.6 Identifier and resource conformance vectors (unexecuted)

Reader behavior is effects-parser §9. Unless a row says otherwise, foreign
services return normally with live outputs. All constants in keys/IDs are hex.
These expectations are independent static predictions, not recorded game runs.

| ID | Fixture | Required result |
|---|---|---|
| EC7 | `textureID Smoke 7`; ParseUint=7; texture map empty | key `smoke` inserted as 0 before conversion, then set to 7; original `Smoke` token retained; no loader/collection call |
| EC8 | `modelID smoke bad`; empty model map; ParseUint throws | model key `smoke` remains 0; texture map unchanged; no successful assignment |
| EC9 | same failure with model `smoke` initially 9 | old 9 retained; no duplicate error |
| EC10 | `soundID Bell 8`, then `soundID BELL 10`; returned uints 8,10 | one sound entry `bell` ending at 10; brush/texture/model maps untouched |
| EC11 | any of four ID declarations with one or three arguments | arity exception, no folding, insertion or conversion |
| EC12 | `effectID G I Smoke`; uint replies FEDCBA98,89ABCDEF; collection C | conversions G then I, fold `smoke`, call `C+50(smoke,FEDCBA98,89ABCDEF)`; no P+49C write |
| EC13 | `instance I Smoke`; group=12345678, null P+90 | convert I and fold name; no collection call; group unchanged |
| EC14 | `effectGroup 9` entry, returning end, then `instance 7 Smoke` | group remains 9 after the block end; instance uses 9, not a restored outer value |
| EC15 | `messageTrigger M MiXeD ignored`; uint reply 42; collection C | call `C+54` once with record `(42,MiXeD)`; no conversion/read of `ignored`; temporary destroyed after call |
| EC16 | disabled `effectsResource bad name`, P+94 null | no uint conversion or resource call; generic block entered; exit hook does nothing |
| EC17 | `effectsResource` with bad arity and P+94 nonnull | arity error wins over nesting error; neither creation nor block entry |
| EC18 | enabled `effectsResource 7 Name`, null P+94; creation returns null | create request key `(EA5118B0,EA5118B1,7)`, IID CA51189B, 0,0; old key/P+90 unchanged; block entered |
| EC19 | same creation returns R, QI gives C; base P+8C=B; successful exit returns segment S, base QI gives K | R name setter precedes key store and assignment P+90=C; block entered. Exit obtains S using EA5118B1, saves key with S, assigns K, clears/releases R, releases S; stored key retained |
| EC20 | active enabled resource exit; segment request AL=0 | segment error; no save, restoration or resource release; generic block remains on stack |
| EC21 | segment request succeeds; save AL=0 | save error; no restoration/resource/segment normal cleanup; active resource/collection and generic block retained |
| EC22 | `loadResource 7 extra`; global manager null | reader/conversion of 7 occurs; extra ignored; no resource call |
| EC23 | same import with manager: load AL=0 | not-found exception; no import or local collection replacement |
| EC24 | same import with manager: load AL=1 and resource R | R+14 receives current P+90, then R released; no component-option lookup or block entry |

### 19.7 Camera parameters and deferred priority declarations

Root `camera` is `cCameraParamsCommand`, descriptor `00A9F8F8`, handler
`0059A2A0`. It selects **at least one** initial positional value, converts
every selected positional value with ParseFloat in order, and stores each
binary32 into an ordered float list. With exactly one value it appends four
more: each is the binary32 result of the preceding stored value multiplied by
binary32 0.5. Thus one value 160 produces `[160,80,40,20,10]`; two or more
values are used unchanged in length and order. It does not enforce five inputs,
sort them, or reject negative distances. Numeric domains here require finite
conversions and nonoverflowing arithmetic.

It then processes the following switches in fixed key order. Each selected
value is an individual ParseFloat followed by a binary32 store. Surplus values
in a switch's list are ignored. Defaults are local to each invocation; a
previous camera command does not supply absent values.

| Key / minimum values | Result | Default |
|---|---|---|
| `parallax` / 2 | first and second parallax endpoints | 100, 101 |
| `size` / 1 | camera size | 4 |
| `sideSwipe` / 1 | side-swipe value | 7 |

The temporary camera record has a copied float list at offsets `+00..08`,
the **first** parallax endpoint at `+0C`, and a span at `+10`. To form the
span, subtract the first stored endpoint from the second using x87, convert
through `_ftol2` (`009EEF04`), inspect the signed low 32-bit result, replace
nonpositive results with 0, then convert that signed integer to binary32.
For finite differences whose truncation fits signed 32 bits this is
`float(max(truncateTowardZero(second−first),0))`, including the final binary32
store; it is not the original fractional difference. Size and side-swipe
occupy `+14,+18`. With nonnull `P+90`, collection slot `+4C` receives the
record address; its status is ignored. The command then destroys both the
temporary camera record and its source float list. A null collection still
performs all reader/conversion/list work, omitting only the final collection
call. It neither creates a component record nor applies component options.
An exception before that call causes no collection update; token marking from
earlier switches remains. There is no command-local catch or rollback.

Evidence: positionals and binary32 stores `0059A2A0–0059A40F`, four half-value
appends `0059A416–0059A4E6` (constant `00A84D2C=3F000000`), switch order and
stores `0059A4EC–0059A5A5`, span conversion/record call
`0059A5A8–0059A63D`. `_ftol2` uses the x87 value and conversion correction
(`009EEF04–009EEF78`); its misleading pseudocode name is not evidence for a
Lua call. Values beyond the finite signed-32 span domain are not qualified here.

Root `setPriority` selects descriptor `00A9F908`, handler `005A24C0`
(export label `cEffectRemapCommand::Parse`). It selects at least two initial
positional values: a copied effect name, a ParseInt priority, then an optional
copied third string. The third string defaults empty; additional positional
values are ignored. Both strings preserve case. There is no numeric range
check, directory-name validation, component append, or immediate manager call.

It offers the record `(name, signedPriority, optionalString, byte=0)` to the
parser's ordered unique-name container at `P+4B0`. The retained record's layout
is name `+00..08`, priority `+0C`, optional string `+10..18`, byte `+1C`.
Insertion copies the record. Uniqueness compares only the first string's exact
bytes and length, case-sensitively; priority and optional string are not part
of the key. An existing equal name is retained **without replacement**; the
owner ignores the insert-result boolean. `Smoke` and `smoke` are separate
keys, whereas a second `Smoke` with a different priority is ignored after its
reader/conversion/copy work. Empty optional text does not set the final byte;
even a supplied third string leaves it zero. These are pending parser records;
§19.9 defines their consumption at effect-block entry.
Temporary strings are destroyed after the insertion returns. A throwing
conversion precedes insertion, so it cannot insert a zero-priority entry as
the resource-ID declarations do.

Evidence: record construction and optional string `005A24C0–005A25B1`,
container selection/insertion/cleanup `005A25B5–005A2619`, unique-key search
and rejected-duplicate return `005A2620–005A26BD`, name-only comparison
`004B5CA0–004B5D01`. All these are descriptor/behavior associations, without
promoting a medium-confidence symbol label into a confirmed original class name.

### 19.8 Camera and priority conformance vectors (unexecuted)

| ID | Input / independent primitive replies | Required result |
|---|---|---|
| EC25 | `camera 160`; ParseFloat=160; collection C | C+4C receives list `[160,80,40,20,10]`, first parallax 100, span 1, size 4, sideSwipe 7; no distance sorting |
| EC26 | `camera 160 90 30`; corresponding exact float replies | retained list `[160,90,30]`; no half-value expansion |
| EC27 | `camera 160 -sideSwipe 2 -size 3 -parallax 10 13.75`; exact replies | switches/conversions in parallax, size, sideSwipe order; record first=10, span=3, size=3, sideSwipe=2; span fractional part discarded |
| EC28 | `camera 160 -parallax 10 9.5` | record first=10, span=0; endpoints not swapped or rejected |
| EC29 | `camera 160 -parallax 10`; otherwise valid | reader marks parallax, then minimum-arity exception; no collection call; prior positional float conversion already happened |
| EC30 | camera as EC25 with null P+90 | complete local conversions/expansion/record cleanup; no collection call |
| EC31 | `setPriority Smoke -4`; ParseInt=−4; empty pending container | inserted `(Smoke,−4,empty,0)`; no manager/collection call or case fold |
| EC32 | repeat exact name `Smoke` with priority 9 and optional `Other` | original EC31 record retained; conversion and optional-string copy still occur, then duplicate insertion returns false |
| EC33 | after EC31, `setPriority smoke 9 Other surplus` | second entry `(smoke,9,Other,0)`; `surplus` ignored; case-sensitive distinct key |
| EC34 | `setPriority New bad`; ParseInt throws | no new pending entry; no default-zero insertion or manager call |

### 19.9 Root visual-effect block

Root `effect` selects `cGroupEffectCommand`, descriptor `00A9F944`, enter
`005A0D40`, exit `0059EB50`. This is the definition of a visual effect, distinct
from nested `visualEffect` references. Its live name is the parser string at
`P+434`; its description is at `P+444`. A nonempty live name throws
`Can't nest effects! (Perhaps you meant visualEffect?)` **before** selecting
arguments. The guard is the name's length, not generic block depth. With an
empty name, the positional reader requires at least one initial value and
copies only its first selected string into that name, preserving case. Further
positional names are ignored. It does not validate the name against the
directory. A quoted empty-name token, if supplied by the tokenizer, leaves this
guard empty; the owner supplies no additional nonempty-name validation.

Entry then replaces the description with a fresh default description: flags=0,
priority=1, no component records, no parameter bindings, empty trailing string,
and first message word (`description+20`) zero. The constructor does **not**
initialize message words `+24,+28`; assignment copies their indeterminate
temporary contents. They are not specified as zero or as the old description's
words. With a supplied `startMessage`, the selected words below replace this
indeterminate/default state individually. This is the text-constructor behavior,
not the binary-reader version defaults in §3.

The following keys are processed in order, after assigning the name and resetting
the description. Presence switches request a minimum of zero values; they OR
the indicated bit into the **description's** 32-bit flags. They do not set
component flags. Their surplus ordinary values have no additional effect.

| Key | Minimum values / interpretation | Effect |
|---|---|---|
| `viewRelative` | 0 | OR 001 hex |
| `noAutoStop` | 0 | OR 002 hex |
| `rigid` | 0 | OR 008 hex |
| `ignoreOrientation` | 0 | OR 040 hex |
| `hardStop` | 0 | OR 004 hex |
| `noPropagate` | 0 | OR 010 hex |
| `applyCursor` | 0 | OR 020 hex |
| `noLODStop` | 0 | OR 080 hex |
| `startMessage` | at least 1; first up to three selected values, ParseUint in order | store words at description `+20,+24,+28`; ignore fourth and later values |
| `manualRestart` | 0 | OR 100 hex |

Each successful message conversion is stored before the next; a throwing second
conversion retains the first word and all previous flag/name/reset writes.
Missing `startMessage` leaves the constructor state above. The option does not
set a separate presence flag or enforce an exact three-word count.

After those options, entry searches the pending `setPriority` container (§19.7)
for the current name using exact-case string comparison. If it finds a record,
it copies its signed priority verbatim into description `+04`, without a [1,5]
range check. A nonempty optional string replaces the live effect name,
preserving case. The record's byte at `+1C` is then set to 1. The record is not
erased, and the renamed name is not looked up again. This path does **not**
look up, mark, convert or validate an explicit `-priority` option.

With no matching pending record, `priority` is looked up with minimum one value.
When absent, default 1 remains. When present, only its first selected value is
converted with ParseInt. Results below 1 or above 5 throw
`argument out of range: expecting [1..5], got %d` before assigning priority or
entering the generic block. Earlier name/reset/options survive. Thus an inline
`-priority 0` fails, while a previously retained `setPriority Name 0` overrides
with 0 successfully at this owner boundary. Because this command is a root
descriptor, the generic root-fallback path itself does not post-reject an
unmarked inline priority on the override path (§19.1).

On either normal priority path, entry zeros words `P+4A0,P+4A4` and byte
`P+4AC`, then enters the generic block. It does not zero `P+4A8` here. These
scratch addresses are interface facts for the remaining select/sequence
contracts, not yet a complete definition of those algorithms. A null `P+90`
does not bypass name, option, pending-record or generic-block processing.

On normal exit, nonnull `P+90` receives collection slot
`+14(currentName,&description)`. Its result is ignored; then the live name is
cleared to empty. A null collection omits publication but still clears the name.
Exit does not clear/reset the description or pending-priority record, restore a
previous name, or parse more options. If publication throws, the clear is not
reached and the generic block is not popped. The next normal entry performs its
own description reset. Collection copy/validation/replacement remains the
individual foreign call's contract.

Evidence: enter nesting/name/default assignment `005A0D40–005A0E1E`,
default constructor `0058A930–0058A979` and assignment `0058B4A0–0058B4FD`,
ordered flag/message writes `005A0E23–005A0F87`, pending-record lookup and
rename/mark `005A0F87–005A1003` (finder `005A12E0` uses comparator
`004B5CA0`), inline validation and generic-block entry `005A1005–005A10A6`,
exit publication/clear `0059EB50–0059EBDA`. Nested dictionary execution is
specified separately in §§19.52–19.56.

### 19.10 Effect-block conformance vectors (unexecuted)

| ID | Fixture | Required result |
|---|---|---|
| EC35 | nonempty live name `Outer`; attempted `effect` without arguments | nesting exception precedes positional minimum check; name/description unchanged |
| EC36 | empty live name; `effect Name ignored`; no options/override; old description populated | live name `Name`, reset flags=0/priority=1/empty component and binding lists/empty trailing string/message0=0; message1,2 not constrained; scratch 4A0/4A4/4AC zero, generic block entered |
| EC37 | `effect Name -manualRestart -applyCursor -viewRelative -startMessage 10 20 30 40`; uint replies 10,20,30 | name assigned and description reset, options processed in table order; flags=121 hex, message words 10/20/30; 40 not converted |
| EC38 | `effect Name -rigid -startMessage 10 bad`; first uint=10, second throws | live name/reset/rigid bit and first message word retained; no manualRestart/priority lookup, scratch final reset or generic entry |
| EC39 | no pending override; `effect Name -priority 0`; ParseInt=0 | range exception; name retained, default priority 1; no generic block entry |
| EC40 | pending `(Name,0,Renamed,0)`; `effect Name -priority bad` | priority=0, live name `Renamed`, pending used byte=1; inline priority remains unmarked and unconverted; no second lookup for Renamed; block entered |
| EC41 | returning exit with collection C and current name `Renamed` | C+14 receives `Renamed` and current description; name then empty, description/pending record retained. Null C instead clears name without publication |
| EC42 | C+14 throws on exit | live name and description retained; generic block not popped; no implicit second exit |

### 19.11 `testEffect`: preview creation and ordered instance setup

The `cTestEffectCommand` descriptor `00A9F8D8` calls `0059D1D0`.
This root command creates a preview instance; it does not add a component to
the current effect description. It requires a nonnull preview manager at
`P+8C`. There is no current-effect-name or collection-presence guard.
The positional finder requests at least one value and its AL is ignored.
The first selected record supplies the effect name, preserving case. Further
selected records beyond the second are ignored by this owner.

The second selected record, if any, supplies the source position. If its
foreign CountArguments reply is exactly 3, ParseVector3 supplies `(x,y,z)`.
Every other count takes the ParseVector2 path: its `(x,z)` reply is sent to
terrain service `00B43CF4`, slot `+34(x,z)`, and source position becomes
`(x, binary32(height + 4.0), z)`. The addition uses x87 with a binary32 4.0
operand and one final binary32 store; do not narrow the returned height before
adding. Absent a second record, the position is `(512,100,512)` and no terrain
call occurs. CountArguments/ParseVector2 validation remains the reader's
contract; this owner does not add an exactly-two requirement to that branch.

Next the following options are read, in this order. Each lookup requests at
least one value, uses only the first selected record, and applies no owner
range check. Conversion failures propagate before manager lookup/creation.

| Option | Conversion | Absent value | Purpose |
|---|---|---|---|
| `sourceScale` | ParseFloat, binary32 store | 1 | uniform scale of source transform |
| `scale` | ParseFloat, binary32 store | 1 | uniform scale of instance transform |
| `trans` | ParseVector3 | `(0,0,0)` | translation of instance transform |

After these conversions, manager slot `+14(name)` tests existence. False throws
`No such effect`; no create call, preview-list append, speed/target/hard lookup
or description mutation occurs. True calls manager slot `+1C(name,&instance)`
with an initially null output pointer. A false create result returns normally,
releasing a nonnull returned pointer if supplied, and omits all remaining
setup. A true reply is required to provide a usable nonnull instance for the
remaining normal domain; this owner has no null-output recovery branch.

On successful creation, the instance is appended to the retained preview list
at `P+4BC` **before** any instance setter. The list takes a reference. This is
followed by instance slot `+20(&sourceTransform)`, then slot
`+1C(&instanceTransform)`. Each transform has identity 3-by-3 basis, the
translation and scalar above, and flags byte 1 for scalar exactly equal to
1, otherwise 3. The comparison is x87 equality: unordered also takes flags 3.
The source transform's change byte is 2. The same temporary is reused for the
instance transform: after the first setter, a nonzero flags byte causes basis
reset and one change-byte increment; replacing translation/scale increments
it twice more, modulo 256. Thus with ordinary setters that do not modify their
input, the second transform has change byte 5. This byte is not a floating
value. The transform layout is change byte `+00`, flags byte `+01`, basis
`+04..+24`, translation `+28..+30`, scalar `+34`; padding bytes are not set by
this command. The second setter is not omitted for identity values.

Only after both setters return are these further options looked up:

| Order | Option and minimum values | Required instance/list action |
|---:|---|---|
| 1 | `speed`, 1 | ParseFloat to binary32; slot `+30(0,&value,1)` |
| 2 | `target`, 1 | ParseVector3; slot `+2C(5,&vector,1)` |
| 3 | `hard`, 0 | append integer 1 if present, otherwise 0, to list `P+4C8` |

Missing speed or target omits its setter; there is no synthesized speed zero
or target vector. The hard list gets one entry even when `hard` is absent,
but only after all preceding setup returns. There is no start/restart call
in this command. Normal completion releases the local instance reference;
the preview list retains its own. A later conversion or foreign-call exception
does not undo an earlier preview append, transform setter or option mark, and
may leave preview and hard-list lengths unequal. Do not infer exceptional
reference cleanup from the normal release tail. No rollback is installed here.

Evidence: selection/defaults `0059D1DE–0059D289`; vector/terrain choice and
binary32 store `0059D292–0059D31E`; preliminary options
`0059D322–0059D3D4`; existence/create gates `0059D3D8–0059D442`;
retained-list append `0059D448–0059D453`, helper `0059D7E0`;
transform stores/comparisons/setters `0059D458–0059D5BF`;
late options and hard append `0059D5C2–0059D68E`, integer-list helper
`0059D6C0`; normal release `0059D693–0059D6AD`. Default translation reads
zero-fill word `00B22CC0` in the PE virtual data tail; it is not file data
following the section's raw bytes. Terrain offset 4.0 is `00A8E17C`.

### 19.12 Preview command conformance vectors (unexecuted)

The manager and instance replies are independent foreign inputs; these cases
do not call a renderer or claim an in-game verdict. Setter callbacks below
return without modifying their input transform.

| ID | Fixture | Required result |
|---|---|---|
| EC43 | `testEffect Name`; exists=true, create=true with I | retain I in preview list; source setter gets identity basis, `(512,100,512)`, scale 1, flags 1/change 2; instance setter gets identity basis, `(0,0,0)`, scale 1, flags 1/change 5; no speed/target setter; append hard=0; release local I |
| EC44 | `testEffect Name 10,20,30 ignored`; second record CountArguments=3 and vector reply `(10,20,30)` | source translation exactly that vector; no terrain call; extra selected record ignored |
| EC45 | second record CountArguments=2, ParseVector2=`(10,30)`, terrain height=7 | terrain slot +34 called with `(10,30)`; source translation `(10,11,30)`; one binary32 store after height+4 |
| EC46 | `testEffect Name -sourceScale 2 -scale 3 -trans 4,5,6 -speed 7 -target 8,9,10 -hard`; normal replies | source scalar 2, flags 3; instance scalar 3, translation `(4,5,6)`, flags 3; setters then speed slot +30 `(0,7,1)`, target slot +2C `(5,(8,9,10),1)`; hard=1 |
| EC47 | invalid trans conversion and exists would be false | conversion exception precedes existence call; sourceScale/scale/trans marks survive; no preview/hard append |
| EC48 | valid preliminary options; exists=false | `No such effect`; no create or late option lookup |
| EC49 | exists=true, create=false with nonnull I; speed token invalid | no append or setter or speed conversion; normal local Release I |
| EC50 | create=true with I; source/instance setters return; speed conversion throws | I already retained in preview list and both transforms sent; no target/hard lookup, hard list unchanged; exception propagated without rollback |

### 19.13 Component definition blocks: names, defaults and inheritance

These are six separate root owners, with entry/exit hooks established by the
descriptor slots, rather than the sometimes reversed exported Parse/EndBlock
names. They build named definitions in the collection at `P+90`; they do not
append references to the current `effect` description. They have no owner
guard against an already nonempty definition name and do not save an outer
definition for restoration. Dispatch and generic block-stack rules remain
effects-parser §12. Child algorithms are specified in §§19.17–19.51.

| Literal / owner | Descriptor | Entry / exit | Live name / description | Collection inherit / publish slots |
|---|---|---|---|---|
| `particles` / `cParticleEffectCommand` | `00A9F960` | `005A1A50 / 0059D9B0` | `P+E0 / P+F0` | `+20 / +1C` |
| `dynamicParticle` / `cDynamicParticleEffectCommand` | `00A9F97C` | `0059DB00 / 0059DCF0` | `P+2FC / P+30C` | `+28 / +24` |
| `decal` / `cDecalEffectCommand` | `00A9F998` | `0059DD80 / 0059DF50` | `P+350 / P+360` | `+30 / +2C` |
| `shake` / `cShakeEffectCommand` | `00A9F9B4` | `0059DFE0 / 0059E260` | `P+3C8 / P+3D8` | `+38 / +34` |
| `light` / `cLightEffectCommand` | `00A9F9D0` | `0059E2F0 / 0059E4D0` | `P+404 / P+414` | `+40 / +3C` |
| `sequence` / `cSequenceEffectCommand` | `00A9F9EC` | `0059E600 / 0059E780` | `P+480 / pointer at P+490` | no inheritance / `+44` |

For the **first four blocks**, the entire token-container count must exceed
one, otherwise entry throws `No name specified!` before changing the name or
description. Raw record 1 is the name, including when it begins with a dash;
the positional/switch finders are not called. Name bytes are copied with case
preserved. Entry replaces the description with the defaults below, then checks
the optional inheritance clause against **raw** records: if record 2 exists,
its complete string must equal `:`, or `Expecting ':'` is thrown. Record 2
alone is accepted as a trailing colon. If record 3 exists and the collection
is nonnull, its string is sent to the inherit slot with the live description
as output. A false reply throws `Unknown particle definition: %s` for
particles, dynamicParticle **and decal**; shake uses
`Unknown shake definition: %s`. The output is not restored on a false reply.
With a null collection the inherit call and unknown-name error are omitted.
Records after record 3 are not read by these owners. No inline options are
recognized here; a dash in record 2 is a bad colon, not an option.

For **light**, entry instead calls FindNonSwitchIndices with minimum one,
ignores AL, and copies selected name 0 preserving case. It resets its
description, then applies the same colon rule to selected record 1, if any.
Selected record 2, if any, supplies the inherited name to collection slot +40.
Failure throws `Unknown light definition: %s`. Further selected records are
ignored. These selected records need not be adjacent raw records. No switch
lookups are performed by light entry.

On successful inheritance, the collection's output replaces/changes the reset
description; defaults are not overlaid again. With no inheritance or with a
null collection, reset values remain. Only after this work returns does entry
invoke the generic block-entry service. Thus name assignment and reset survive
a bad colon or failed inheritance, but no generic block is entered on those
exception paths. The foreign collection performs the named lookup/copy;
returning an already parsed child-command result is not an owner primitive.

The **particles** entry additionally sets `P+494` to the four-byte word
immediately before the end pointer of the description list at `P+2A8` (zero
when empty), and `P+498` to binary32 1,
after successful inheritance and before block entry. These scratch writes are
omitted on a preceding failure. Other definition entries do not do them.

The following tables specify the reset state in native field coordinates
relative to each description, denoted D. They are logical values: lists are
copied/cleared, not shallow-copied allocator pointers. Padding and allocation
capacities are not conformance values. Field coordinates avoid inventing
semantic names for fields whose child command is still being researched.
All decimal scalar values below are binary32 unless explicitly called an
integer, byte or word.

**Dynamic particle reset:** integer `D+08=0`, empty string at `D+0C`,
`D+18=1`, words `D+1C,+20,+24,+28,+2C,+30=0`, empty word list at `D+34`.
The existing description's reference-count identity is retained; assignment
does not replace it with the temporary's identity. Constructor `005836A0`
and assignment `00583750–005837AC` establish these values.

**Decal reset:** integer words `D+00,+04=0`, byte `D+08=0`, byte `D+09=1`,
`D+0C=5`; float lists `D+10=[0]`, `D+1C=[1]`, `D+28=[1]`,
`D+40=[1]`; vector list `D+34=[(1,1,1)]`; words `D+4C,+50,+54=0`,
`D+58=1`, `D+5C,+60=0`. Bytes `+0A,+0B` are not assigned.
Evidence: `00582FE0–00583123`, assignment `005833F0–00583471`, vector
constant `00B0A214`; the list-construction helper `00581F20` uses signed
integer-to-float conversion, so integer initializer 1 produces float 1.

**Shake reset:** `D+00=5`, `D+04=1`, float list `D+08=[1]`, float list
`D+14=[60]`, `D+20=1`, integer `D+24=0`. The lists are first cleared;
these are single-element defaults, not appended to a previous definition.
Evidence: `0059E09F–0059E17F` (constants `40A00000`, `3F800000`,
`42700000`).

**Light reset:** empty vector list `D+00`, empty float list `D+0C`,
`D+18=2`. Unlike shake, these lists are populated with fallback values at
**exit**, not entry (`0059E392–0059E3CD`).

**Particles reset:** base constructor `005B0FE0–005B1527`, assignment
`005B17F0–005B1C77`, wrapper `005B1CB0`. The scalar state is:

| Field coordinates within D | Reset value |
|---|---|
| `+0C,+10` | 2 |
| `+14,+34,+40,+118,+124,+154,+170,+1B0` | 1 |
| `+20,+24,+28,+2C` | -1 |
| `+50,+54,+58` | -1 |
| `+5C,+60,+64` | 1 |
| `+14C` | bit pattern `CE6E6B28` |
| `+158,+15C` | -10000, +10000 |
| word `+16C`, word `+16E` | integer 0, integer 1 |
| `+174,+178`; `+17C,+180` | 5; 50 |
| `+184,+188` | `3DCCCCCD`, `3E4CCCCD` (binary32 0.1, 0.2) |
| `+1F0,+200` | 4 |
| bytes `+D4,+D5` | 0 |

Other **assigned scalar** words are zero: `+00,+04,+08,+18,+1C,+30,+38,
+3C,+44,+48,+4C,+68,+6C,+70,+74,+78,+7C,+80,+84,+D0,+D8,+DC,
+E0,+E4,+E8,+EC,+F0,+F4,+F8,+FC,+100,+104,+114,+11C,+120,+128,
+138,+13C,+140,+144,+148,+150,+18C,+190,+194,+198,+19C,+1A0,+1B4,
+1C4,+1D8,+1DC,+1EC`. In particular the zero vector defaults loaded from
`00B22D54` are in the PE virtual zero-fill region. The string at `D+160`
becomes empty. The description lists are:

| List coordinate | Reset contents |
|---|---|
| `+88` | `[30]` |
| `+94` | `[(1,1,1)]`, constant `00B0A5D4` |
| `+A0,+AC,+B8` | each `[1]` |
| `+C4` | `[0]` |
| `+108,+12C,+1A4,+1B8,+1CC,+1E0` | each empty |
| `+1F4` | `[400]` |

Do not treat this as a byte-zeroed object: `D+1C8` is copied from an
uninitialized temporary word, and padding at `+D6,+D7` is not assigned.
The wrapper constructor sets its temporary `+204` to zero, but the assignment
wrapper copies only the base; the live description's `D+204` therefore
**retains its previous value** at entry. An implementation must not assert a
deterministic zero for either `+1C8` or `+204` based on this reset. This
distinguishes logical base assignment from constructing an entirely new live
particles object.

### 19.14 Definition publication and sequence ownership

On normal exit, particles, dynamicParticle, decal and shake call the publish
slot in §19.13 with `(currentName,&liveDescription)` when the collection is
nonnull. They ignore its result, then clear their live name to empty. A null
collection omits publication but still clears the name. The description stays
in place; no default reset, inheritance, parent restoration or option parsing
occurs at exit. If publication throws, the name clear is not reached. Exit
does not itself guarantee a nonempty live name. Evidence: `0059D9B0–0059DA34`,
`0059DCF0–0059DD74`, `0059DF50–0059DFD4`, `0059E260–0059E2E4`.

**Light exit** first fills an empty float list at `D+0C` with `[1]`, then
fills an empty vector list at `D+00` with `[(0,0,1)]`. Existing nonempty
lists are left intact. These fallbacks occur even with a null collection.
It then publishes/clears the name as above. Thus a publication exception leaves
the fallback list mutations and name in place. Evidence:
`0059E4D9–0059E590`, constant `00B0A518`, name clear `0059E590–0059E5EF`.

**Sequence entry** uses FindNonSwitchIndices minimum one, ignores AL, and
copies selected name 0, preserving case; all other positional selections are
ignored. It does not interpret `:` or perform inheritance. It allocates a
new 28-byte sequence description with empty element list (`+0C`) and flags
word `+18=0` (`005BDF10–005BDF42`). It installs the new pointer at `P+490`,
taking a reference through new object's slot +10 before the store, then
releasing the old nonnull pointer through slot +14. Equal-pointer assignment
omits those reference operations. It then processes presence-only switches
`loop`, `noOverlap`, `hardStart` in that order, minimum zero values, ORing
flags 1,2,4 respectively into the installed description. There are no owner
range checks, positional upper bound, or automatic sequence elements. Finally
it enters the generic block. Name assignment precedes allocation; description
replacement precedes option lookups. Allocation failure is not a proven
recoverable command error: it installs null, and present flags dereference that
pointer. Do not add an inferred parse-error message for allocation failure.
Evidence: `0059E60B–0059E6DF`, flags `0059E6DF–0059E748`, block entry
`0059E748–0059E76F`.

**Sequence exit** publishes `(currentName,currentDescriptionPointer)` to
collection slot +44 if nonnull, ignoring its result. It clears the name, then
clears `P+490` before invoking the old nonnull description's slot +14. With no
collection it still clears/releases. Publication failure retains name and
pointer. Unlike the other definition exits, a normal sequence exit does not
retain the live description pointer (`0059E780–0059E812`). Collection-owned
retention is the independent collection primitive's contract.

### 19.15 Definition-block conformance vectors (unexecuted)

| ID | Fixture | Required result |
|---|---|---|
| EC51 | each of particles/dynamicParticle/decal/shake with command token only | `No name specified!`; old name/description unchanged; no generic block entry |
| EC52 | `decal Name :`; old lists populated | live name `Name`; exact decal defaults of §19.13; no inherit call; block entered |
| EC53 | `shake Name wrong Parent` | live name assigned, shake defaults installed including `[1]` and `[60]`; `Expecting ':'`; no inherit/block entry |
| EC54 | `dynamicParticle Name : Parent ignored`; collection inherit +28=false after writing a field | defaults then +28 called with Parent/output description; `Unknown particle definition: Parent`; foreign output mutation/name retained; no block entry |
| EC55 | `decal Name : Missing`; null collection | defaults retained; no unknown-name error; block entered |
| EC56 | `particles -Name`; two raw records, existing D+204=123 | name `-Name` accepted as raw string; defaults installed with D+204 still 123; D+1C8 unconstrained; P+494=0 from empty default +1B8 list, P+498=1; block entered |
| EC57 | particles inheritance succeeds; list at P+2A8 is nonempty and word at end-4 has binary32 7 | P+494 copies that word bit-for-bit; P+498=1; inherited data retained; block entered |
| EC58 | light selections `[Name,:,Parent,ignored]`, successful +40 output has empty float/vector lists | name assigned/reset then inheritance; lists remain empty until exit; no inline option lookup |
| EC59 | light exit with null collection and empty lists | D+0C becomes `[1]`, D+00 becomes `[(0,0,1)]`; name cleared; scalar D+18 retained |
| EC60 | light exit with nonempty float/vector lists; publish +3C throws | lists unchanged; live name/description retained; generic block not popped |
| EC61 | each of particles/dynamicParticle/decal/shake normal exit, collection returns false | correct publish slot receives live name/description; false ignored; name cleared and description retained |
| EC62 | `sequence Name : Parent -loop -noOverlap -hardStart`; allocation succeeds, old pointer O | no inheritance call; retain new empty description N, store N, release O; flags=7 in lookup order; positional colon/Parent ignored; generic block entered |
| EC63 | sequence normal exit, null collection, installed N | no publication; name cleared, P+490 set null, then N+14 called |
| EC64 | sequence +44 publication throws | name and installed pointer retained; no explicit exit release/clear |

### 19.16 Particle `effectBase` string binding

Particles' nested registrar associates literal `effectBase` with descriptor
`00A9FA28`, direct handler `0059D8F0`, owner `cParticleEffectBaseCommand`
(`00595C2D–00595C40`, literal pointer `00B0A378` to `00A9F500`).
The handler first requires the live particles name at `P+E0` to be nonempty;
otherwise it throws `Not in particles block` before positional selection.
It does not check the generic block stack or the current `effect` name.

For a nonempty particles name it requests at least one nonswitch value,
ignores the returned AL, and copies selected value 0 into the particles
description's string at `P+250` (`D+160` in §19.13), preserving case.
Additional selected values are ignored. There is no owner option lookup,
collection lookup, existence check, component creation or change to the live
particles name. Empty string is a valid primitive value when the reader
supplies it. A returning assignment replaces the previous binding; a later
generic unhandled-switch error does not roll back that change. Missing-value
failure follows the shared reader's exception contract.

Evidence: name guard `0059D8F8–0059D94A`, positional selection
`0059D94F–0059D95B`, selected string assignment `0059D960–0059D994`.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC65 | empty particles name; `effectBase` without a value | `Not in particles block` precedes minimum-value error; binding unchanged |
| EC66 | particles name `P`; `effectBase MixedCase ignored`; old binding `Old` | binding becomes `MixedCase`; second selection ignored; no foreign existence/collection call |
| EC67 | particles name `P`; selected empty string | binding becomes empty; live particles name remains `P` |
| EC68 | particles name `P`; `effectBase New -unknown`; nested post-check enabled | binding becomes `New`, then generic unhandled-switch error; binding survives |

### 19.17 Particle child dictionary and dispatch identity

`cParticleEffectCommand::RegisterCommands` (`005958F0`) installs the following
**27 entries in this order** in the particles block's own dictionary. Each
entry constructs a separate four-byte descriptor object; aliases share a
descriptor address, not an allocation. All have the ordinary command wrapper
`007E3BE0` at descriptor slot +04, no block-entry/exit override, and the direct
handler at slot +0C below. They are not additional root commands. Allocation,
ownership, name matching and generic post-command switch checking follow
effects-parser §12; these rows do not imply that an owner consumes every switch.

Names in the last column are the verified exported class names with the common
`(anonymous_namespace)::` prefix omitted. `unnamed` means that this executable's
export does not establish a class name; a name must not be invented from a
literal. An inventory row establishes dispatch identity, not execution closure.
Only handlers with published sections have execution contracts here.

| Order | Literal | Descriptor | Direct handler | Owner / contract |
|---:|---|---|---|---|
| 1 | `zoom` | `00A9FB78` | `005970E0` | unnamed; §19.18 |
| 2 | `color` | `00A9FB68` | `0059BDC0` | `cParticleColorCommand`; §19.25 |
| 3 | `alpha` | `00A9FB58` | `0059C060` | unnamed; §19.25 |
| 4 | `size` | `00A9FB48` | `005998E0` | unnamed; §19.25 |
| 5 | `aspect` | `00A9FB38` | `00599A10` | unnamed; §19.25 |
| 6 | `rotate` | `00A9FB28` | `00599B60` | `cParticleRotateCommand`; §19.25 |
| 7 | `source` | `00A9FB18` | `00597140` | `cParticleSourceCommand`; §19.45 |
| 8 | `emit` | `00A9FB08` | `0059C280` | `cParticleEmitCommand`; §19.47 |
| 9 | `force` | `00A9FAF8` | `0059A820` | `cParticleForceCommand`; §§19.49–19.50 |
| 10 | `warp` | `00A9FAE8` | `0059B680` | `cParticleWarpCommand`; §19.43 |
| 11 | `randomWalk` | `00A9FAD8` | `005979B0` | `cParticleRandomWalkCommand`; §19.23 |
| 12 | `stretch` | `00A9FAC8` | `00597C60` | unnamed; §19.18 |
| 13 | `life` | `00A9FAB8` | `00597CD0` | `cParticleLifeCommand`; §19.18 |
| 14 | `rate` | `00A9FAA8` | `0059CA70` | `cParticleRateCommand`; §19.19 |
| 15 | `inject` | `00A9FA98` | `0059CBB0` | `cParticleInjectCommand`; §19.19 |
| 16 | `maintain` | `00A9FA88` | `0059CC20` | `cParticleMaintainCommand`; §19.19 |
| 17 | `texture` | `00A9FA78` | `005A10B0` | `cParticleTextureCommand`; §19.39 |
| 18 | `model` | `00A9FA68` | `005A1340` | `cParticleModelCommand`; §19.40 |
| 19 | `align` | `00A9FA58` | `00597FB0` | `cParticleAlignmentCommand`; §19.20 |
| 20 | `collide` | `00A9FA48` | `00598120` | `cParticleCollisionCommand`; §19.22 |
| 21 | `terrainRepel` | `00A9FA38` | `00598320` | `cParticleTerrainRepelCommand`; §19.22 |
| 22 | `effectBase` | `00A9FA28` | `0059D8F0` | `cParticleEffectBaseCommand`; §19.16 |
| 23 | `timedEffect` | `00A9FA18` | `005A16B0` | unnamed; §19.41 |
| 24 | `color255` | `00A9FB68` | `0059BDC0` | `cParticleColorCommand`; §19.25 |
| 25 | `colour` | `00A9FB68` | `0059BDC0` | `cParticleColorCommand`; §19.25 |
| 26 | `colour255` | `00A9FB68` | `0059BDC0` | `cParticleColorCommand`; §19.25 |
| 27 | `alpha255` | `00A9FB58` | `0059C060` | unnamed; §19.25 |

There are 23 distinct descriptor addresses. In particular, the two final
registrations call constructors `00595D20` and `00595D30`, which only install
`00A9FB68` and `00A9FB58` respectively. They do not install a per-instance
normalization flag. Any alias-sensitive interpretation belongs to the handler's
inspection of its input, not to extra descriptor state.

Evidence: allocation/registration instructions `005958F0–00595D19`; literal
pointer cells `00B0A314–00B0A37C`; direct-handler words at the listed descriptors
+0C; constructor stores `00595D22`, `00595D32`. Wrapper behavior is specified
in effects-parser §12. The remaining child algorithms and other dictionaries
are specified in §§19.18–19.56.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC69 | construct a particles command with successful allocations | 27 registrations in table order, 23 distinct descriptor identities, one four-byte allocation per entry |
| EC70 | compare `color`, `color255`, `colour`, `colour255` registrations | four different descriptor objects with identical descriptor address and handler; no alias-specific object field |
| EC71 | compare `alpha` and `alpha255` registrations | different descriptor objects, both `00A9FB58` and handler `0059C060`; neither is a block command |

### 19.18 Particle `zoom`, `stretch` and `life`

Coordinates in the particle child contracts use the same `P` as §19.13 and
`D=P+F0`. Scalar stores described as floats are binary32. Numeric conversion,
reader selection and exception transport remain services as in §19.1 and
effects-parser §§9,12. A command's completed writes and switch marks survive
a subsequent conversion, reader or generic post-check error. No transaction
or automatic rollback is installed by these owners.

`zoom` (`005970E0`, descriptor `00A9FB78`) requires at least two **raw**
records including the command record. Otherwise it throws `Not enough
arguments`. It calls ParseUint on raw record 1 and discards the result.
There is no description write, live-name guard, positional finder, switch
lookup or zoom-range validation in this handler. Later raw records are ignored
by the owner. Thus this command can reject a malformed number while making
no description change even on success. A dash-prefixed raw record is still
the operand sent to conversion; generic switch handling is separate.

`stretch` (`00597C60`, descriptor `00A9FAC8`) has the same raw-count rule
and error, also without a live-name guard or switch lookup. It converts raw
record 1 with ParseFloat and stores the result at `D+DC` (`P+1CC`). Extra
records are ignored by the owner. It supplies no default when the operand is
missing, performs no clamp, and does not change a flag word.

`cParticleLifeCommand` (`00597CD0`) instead selects at least one nonswitch
value, ignoring the finder's return value. Let `m` be ParseFloat of selected
value 0 **rounded to binary32 before either arithmetic operation**. Let `r`
be ParseFloat of selected value 1 if supplied, otherwise zero. The second
conversion result is retained in x87 precision without a binary32 store
before use. Store `m-r` at `D+0C` and `m+r` at `D+10`, each rounded to
binary32 independently. Values after the second are ignored. No positivity,
ordering, finite-value or nonnegative-radius test is added by the owner.
Negative r therefore reverses the ordinary lower/upper ordering.

After both bound stores, look up `preroll` with minimum one. If present,
convert its first value with ParseFloat and store binary32 at `D+1C`.
If absent, copy the stored binary32 `m` to `D+1C`, even when a previous
life command set a different preroll. No other fields or flags change.
There is no particles-name guard here. A second positional conversion failure
precedes both bound writes; a preroll failure follows both and leaves the old
preroll in place.

Evidence: raw-count/conversion/no-write zoom `005970E0–00597138`; stretch
`00597C60–00597CC2`, store `00597CB6`; life selection and first rounding
`00597CD7–00597D01`, second operand `00597D05–00597D33`, arithmetic/store
order `00597D37–00597D60`, preroll `00597D62–00597DA0`. All arithmetic and
store widths were checked against instruction bytes.

### 19.19 Particle rate list, injection, maintenance and rate options

These three owners write the float list at `D+88` (`P+178`). They do not
require the live particles name to be nonempty and do not look up a named
collection or create a runtime particle system. Omitted options retain
previous description values, except for the explicit defaults below. All
bit operations preserve unrelated bits; none of these owners clears a mode
bit left by an earlier command.

| Owner | Initial selection and result | Further processing |
|---|---|---|
| `cParticleRateCommand`, `0059CA70` | First look up `inject` with minimum one. If present, resize the list to length one, ParseFloat only its first argument and replace element 0; OR `D+00` with 2. Positional values are not selected on this path. If absent, select at least one nonswitch value; on a nonzero **full EAX** finder result, clear the old list and append ParseFloat of every selected value, rounding each to binary32. A stipulated returning zero EAX preserves the old list. | Apply the common rate options below on either path. Normal minimum-value failure is the reader's exception, not a synthesized empty rate curve. |
| `cParticleInjectCommand`, `0059CBB0` | Select at least one nonswitch value; ignore finder return. Resize list to length one, convert first selected value with ParseFloat, replace element 0, then OR `D+00` with 2. Further selected values are ignored. | Apply the common rate options below. It does not look up `inject`. |
| `cParticleMaintainCommand`, `0059CC20` | Select at least one nonswitch value; ignore finder return. Resize list to length one, convert first selected value with ParseFloat, replace element 0, then OR `D+00` with 4. Further selected values are ignored. | Look up **only** `delay` with minimum one; if present, assign the endpoint pair below to `D+20,+24`. It does not run the common rate options. |

Resizing to one retains the previous element 0 when one exists and initializes
a newly grown element to zero; it truncates extra elements **before** conversion.
Consequently a conversion failure can leave the truncated old first value or
a new zero. The mode-bit OR occurs only after successful conversion/store.
The ordinary rate-list path clears the old list before its first conversion:
a failing conversion leaves the successfully appended prefix. The helper
misnamed `normalized` in the export (`00464180`) erases a vector range; this
is a list clear, not numeric normalization.

The owned **endpoint-pair helper** `005965B0` converts selected value 0 with
ParseFloat and rounds it to binary32. If there is a second selected value,
convert that with ParseFloat and return the two endpoints, each binary32;
otherwise return two copies of the first. Extra values are ignored. It does
not interpret a midpoint/radius, reorder, clamp or test the endpoints. A
failure of the second conversion precedes either destination-field write.

The common rate-option handler `00597DB0` queries these options **in table
order**, independently of their textual order. Each row completes before the
next is queried. All finder comparisons use AL except the positional rate
path's full-EAX comparison noted above.

| Order | Option / minimum | Result when present; result when absent |
|---:|---|---|
| 1 | `loop` / 1 | ParseFloat first value to `D+14`; if at least two values, ParseUint second to integer `D+18`; otherwise retain old `D+18`. Absent retains both. |
| 2 | `single` / 0 | First value, if any, ParseFloat to `D+14`; with zero values store binary32 bits `3A83126F` (0.001 rounded). Then set integer `D+18=1`. Absent retains both. It overrides completed `loop` writes. |
| 3 | `sustain` / 0 | OR `D+00` with `00000008`; absent retains flags. |
| 4 | `scale` / 0 | OR `D+00` with `00080000`; absent retains flags. |
| 5 | `areaScale` / 0 | OR `D+00` with `00100000`; absent retains flags. |
| 6 | `volumeScale` / 0 | OR `D+00` with `00200000`; absent retains flags. These three scale flags can coexist. |
| 7 | `delay` / 1 | Endpoint pair to `D+20,+24`; absent retains both. |
| 8 | `trigger` / 1 | Endpoint pair to `D+28,+2C`; if found, omit `retrigger` lookup entirely. |
| 9 | `retrigger` / 1 | Queried only if trigger absent. Endpoint pair to the same `D+28,+2C`; if both absent retain both. |

Zero-minimum flag options do not parse any supplied values. `loop` ignores
values after its second; `single` ignores values after its first. No option
sets an implicit count unless specified above. The two alias options trigger
and retrigger are not both consumed when both appear. Their leftover-switch
consequence follows the generic nested post-check policy, after completed
description changes. A later failure retains the base rate/injection change
and every earlier option's changes. For example, failed loop count conversion
retains the new loop duration but old count and skips single and all later
options.

Evidence: rate injection and full-EAX branch `0059CA77–0059CB1B`, append
loop `0059CB20–0059CB91`, common call `0059CB94–0059CB96`; inject list,
flag and helper call `0059CBB7–0059CC12`; maintain `0059CC2A–0059CCBA`;
resize service `00464130`, erase helper `00464180–004641A3`; endpoint helper
`005965B0–00596622`; common loop/single `00597DB9–00597E75`, flag ORs
`00597E7F–00597EFE`, endpoint destinations and alias priority
`00597F08–00597F95`. These instructions establish float stores versus
integer count/flag stores; the pseudocode alone is not the numeric evidence.

### 19.20 Particle `align`

`cParticleAlignmentCommand` (`00597FB0`, descriptor `00A9FA58`) first
requires the live particles name at `P+E0` to be nonempty, otherwise throws
`Not in particles block`. It then requires at least two raw records, otherwise
throws `Not enough arguments`. The guard is the name test, not a generic
stack-type test. It calls the foreign ParseEnum on **raw record 1**, table
`00B0A450`, final argument 0, with pairs `camera=0`, `ground=1`, `dirX=2`,
`dirY=3`, `dirZ=4` and a null terminator. It stores the low byte of the
return value at `D+D5` (`P+1C5`). Enum text conversion/failure remains a
primitive service; this owner supplies no extra fallback branch.

Next look up `damp` with minimum one: if present, convert its first value
with ParseFloat and store binary32 at `D+198` (`P+288`). If absent retain
the previous value. Then look up `bank` with minimum two. If bank is found,
do not query `windBank`. Otherwise look up `windBank` with minimum two;
when it is found, OR `D+08` with `80` **before converting either value**.
On either successful lookup, ParseFloat the first value to `D+19C`
(`P+28C`), then the second to `D+1A0` (`P+290`), each binary32. If neither
lookup succeeds, retain both fields and flags. There is no angle conversion,
normalization, clamp, or clearing of the wind-bank bit on a later bank command.
Extra raw/option values are ignored by the owner.

An enum conversion failure leaves all these fields unchanged. A damp failure
retains the new alignment byte. A windBank first-value failure still retains
its newly set flag; a second-value failure additionally retains the new first
bank field. When bank and windBank coexist, windBank remains unqueried, which
can cause the later generic post-check failure.

Evidence: name guard `00597FB8–00598009`; raw count, enum call and byte store
`0059800E–00598072`; enum pairs `00B0A450–00B0A47C`; damp lookup/store
`00598060–00598095`; bank/windBank precedence and flag
`0059809E–005980D0`; separate binary32 stores `005980EB–0059810D`.

### 19.21 Particle scalar/rate/alignment conformance vectors (unexecuted)

These are static predictions with independently stipulated reader/conversion
replies, not executed native or shadow results. Unless specified otherwise,
the live name is `P`, all unrelated description state is sentinel-filled,
conversions return the displayed exact values, and absent options return false.
Generic reader and post-check behavior is as in effects-parser §§9,12.

| ID | Fixture | Required result |
|---|---|---|
| EC72 | direct zoom owner with only command record | `Not enough arguments`; no conversion or description write |
| EC73 | empty live name; raw `zoom 9 ignored` | ParseUint on raw 1 only, result discarded; description unchanged; no name error or owner option lookup |
| EC74 | empty live name; raw `stretch -2 ignored`; ParseFloat=-2 | `D+DC=-2`; no selection or option lookup and no flag change; raw dash operand still sent to conversion |
| EC75 | `life 5 2 ignored`, preroll absent | bounds `D+0C=3,+10=7`, `D+1C=5`; third selected value unconverted |
| EC76 | `life 5 -2 -preroll 9` with reader selecting the two displayed values | bounds 7,3 without reorder; preroll 9 |
| EC77 | life selected m=5, r=2; preroll lookup returns value whose conversion throws; old preroll=99 | new bounds 3,7 survive; preroll stays 99 |
| EC78 | life first ParseFloat returns extended `1+2^-24`, second returns zero | first rounds to binary32 1 (nearest-even); both bounds and absent preroll become 1 |
| EC79 | old rate list `[9,8,7]`; `rate 2 bad 4`; second conversion throws | old list cleared; resulting prefix `[2]`; no common option lookup |
| EC80 | old rate list `[9,8,7]`; `rate 100 -inject 6 7`; inject finder selects 6,7 | list `[6]`, flag bit 2 set; 7 unconverted and positional 100 unselected; common options still queried |
| EC81 | old list `[9,8]`, flag word 0; direct `inject bad`; conversion throws | list truncated to `[9]`, no bit 2 write and no common options |
| EC82 | empty list, flags=2; `maintain 8 99 -delay 7 3` | list `[8]`, flags=6, delay endpoints 7,3 without reorder; 99 ignored; only delay queried after base selection |
| EC83 | direct inject with value 4, options `loop`=(8,6), `single`=zero values, sustain/scale/areaScale/volumeScale present | list `[4]`; duration bits `3A83126F`, integer count 1; flags OR mask `0038000A`; table-order lookups regardless of text order |
| EC84 | rate with loop=(8,bad), single present; old loop count=7; uint conversion throws | new base rate list and duration 8 survive; count remains 7; single not queried |
| EC85 | inject 4, old count=7; loop=(8) only | duration 8, count remains 7; injection does not imply single count |
| EC86 | inject 4; delay=(6); trigger=(9,2); retrigger also present; nested post-check enabled | delay 6,6; trigger endpoints 9,2; retrigger unqueried; generic leftover-switch error occurs after writes |
| EC87 | empty live name, raw `align` without operand | `Not in particles block` precedes raw-count error; no enum call |
| EC88 | name P; raw `align dirZ ignored`; ParseEnum returns 4; damp/bank/windBank absent | byte `D+D5=4`; all three float fields and wind flag retained |
| EC89 | name P; align returns 1; damp=2; bank=(3,4), windBank present | byte 1, damp 2, bank fields 3,4; windBank unqueried; no newly set wind flag; later post-check can fail |
| EC90 | name P; bank absent; windBank selected values 3,bad; second conversion throws | wind flag OR 80 and first bank field 3 survive; second bank field retained |

### 19.22 Particle collision and terrain repulsion

`cParticleCollisionCommand` (`00598120`) requires the live particles name
at `P+E0` to be nonempty; an empty name throws `Not in particles block`
before changing the description or querying options. There is no positional
count or selection requirement. With a nonempty name, OR `D+00` with `100`
before the first option lookup. Then process these options in table order.
All flag ORs preserve existing bits, and absent options retain their fields
except for the bounce-zero default.

| Order | Option / minimum | Behavior |
|---:|---|---|
| 1 | `bounce` / 1 | Present: ParseFloat first value to binary32 `D+138` (`P+228`), without range validation. Absent: replace the existing value with bits `3E99999A` (binary32 0.3) **only if it compares equal to zero**; this includes negative zero but excludes NaN. Retain any other value. |
| 2 | `sticky` / 0 | If present OR `D+00` with `200`. |
| 3 | `killOutsideCity` / 0 | If present OR `D+00` with `800`. |
| 4 | `effect` / **0** | If present, ParseFloat first selected value to `D+150` (`P+240`), with no range check. This is a scalar write, not an effect-name lookup. |
| 5 | `death` / 1 | If present, ranged-float conversion with bounds 0,1, then binary32 store to the **same** `D+150`, replacing a completed effect write. |
| 6 | `deathByWater` / 1 | If present, ranged-float conversion with bounds 0,1, then binary32 store to `D+154` (`P+244`). |
| 7 | `destroyBuildings` / 0 | If present OR `D+04` with 1. |

Extra supplied option values are not converted. There is no runtime collision,
terrain or building-service call here. The base collide bit survives a later
reader/conversion failure. A failed death conversion preserves an earlier
effect write. Omission does not clear sticky, kill or destroy flags.

The owned **ranged-float helper** `00597BF0` calls ParseFloat, then compares
the returned x87 value against the binary32 bounds **before rounding the result
to a destination float**. An ordered value below the lower bound or above the
upper bound throws
`argument out of range: expecting [%g..%g], got %g`, formatting the lower,
upper and returned value as doubles. Bounds are inclusive. The instruction
tests also accept an **unordered (NaN)** return; do not replace them with a
predicate that rejects all nonfinite values. Infinities outside finite bounds
are rejected. Success returns the unrounded x87 result to the caller's store.
The normal service domain assumes exception transport on error, not a boolean
or clamped numeric return.

The zero minimum for `effect` is intentional: after a successful lookup the
owner reads the first scratch index without checking the selected count. No
default value or owner missing-argument error exists for a zero-value successful
lookup. A usable first selected record is required for the normal conversion
contract; empty selections lie outside that domain and must not be modeled as
effect zero. This also applies to `preferDir` below.

`cParticleTerrainRepelCommand` (`00598320`) has **no** live-name guard.
It first ORs `D+00` with `400`, then selects at least two nonswitch values,
ignoring the finder's return value. ParseFloat first value to `D+13C`
(`P+22C`), then second to `D+140` (`P+230`), both binary32. Ignore later
selected values. It then independently looks up `scout`, `vertical`,
`killHeight`, each with minimum one, in that order. Convert the first value
of each present option with ParseFloat to binary32 `D+144` (`P+234`),
`D+148` (`P+238`), `D+14C` (`P+23C`) respectively. Absent options retain
the old values. No positivity, ordering, range or finite-value check is added.
The repel bit survives even the minimum-two reader exception; the first
positional store survives failure converting the second. No rollback occurs.

Evidence: collision guard/initial flag `00598120–0059818D`; bounce stores
and x87 zero comparison `005981A2–005981E4`; flag/option sequence
`005981EE–00598303`; effect minimum zero `00598234`, death/water bound
arguments `0059828F–005982DC`; ranged helper comparisons/status-bit tests
`00597BFD–00597C14`, error formatting `00597C16–00597C48`; zero data
`00A81054`; terrain initial flag and separate operand stores
`00598326–00598373`, options/stores `00598379–00598418`.

### 19.23 Particle random walk

`cParticleRandomWalkCommand` (`005979B0`) ORs `D+00` with `00800000`
before its first lookup. It has no particles-name guard, positional selection
or positional count requirement. It changes a description only; it does not
sample random numbers or perform a simulation step. Options are queried in
the order below. Absent options retain all their associated fields; no existing
flag is cleared.

| Order | Option / minimum | Result when present |
|---:|---|---|
| 1 | `delay` / 1 | Midpoint/radius result at `D+174,+178` (`P+264,+268`). |
| 2 | `strength` / 1 | Midpoint/radius result at `D+17C,+180` (`P+26C,+270`). |
| 3 | `turn` / 1 | Ranged-float first value with bounds 0,1 to binary32 `D+184` (`P+274`); second value if supplied, separately ranged 0,1 to `D+188` (`P+278`), otherwise store zero at `D+188`. Values are endpoints, not midpoint/radius, and are not reordered. |
| 4 | `wait` / 0 | OR `D+00` with `01000000`. |
| 5 | `preferSea` / 0 | OR `D+00` with `02000000`. |
| 6 | `preferDir` / **0** | ParseVector3 of the first selected value, then copy its three binary32 words in component order to `D+18C,+190,+194` (`P+27C,+280,+284`). No normalization, length test or extra preference flag. A zero-value successful selection has the unchecked-first-index limitation in §19.22. |

For delay and strength, convert the first value with ParseFloat and round it
to binary32 midpoint `m`. Convert the second value with ParseFloat to x87
radius `r` if supplied, otherwise use zero. Store `m-r`, then `r+m`, each
rounded to binary32. There is no bound, positivity or order test; a negative
radius reverses the bounds. Values after the second are ignored. This is
**different from** the endpoint-pair helper used by rate delay (§19.19).
A second-value conversion failure precedes both destination writes for
delay/strength, while a second-value turn failure follows the first turn
write. Ranged-float acceptance, including unordered returns, is §19.22.
All earlier writes and the initial walk flag survive a later failure.

Evidence: initial flag `005979BD–005979D5`; delay first rounding and x87
arithmetic `005979F7–00597A44`; strength `00597A74–00597AC1`; turn
bounded calls/separate stores/default zero `00597AC7–00597B3E`; flag
lookups/ORs `00597B44–00597B80`; preferDir minimum zero, vector call and
ordered word stores `00597B8A–00597BD5`. Literal cells point to `turn`
at `00A96DCC`, `wait` at `00A9F474` and `preferSea` at `00AA0000`.

### 19.24 Particle collision/random-walk conformance vectors (unexecuted)

Fixtures use the independent primitive replies of §19.21. Ranged helper
fixtures stipulate the conversion return itself; they do not assert that a
particular text spelling is accepted for NaN or extended-precision values.

| ID | Fixture | Required result |
|---|---|---|
| EC91 | empty particles name; collide with malformed bounce | name error before any flag, option lookup or conversion |
| EC92 | live name P; collide with no options; bounce is negative zero | OR flags with 100; bounce becomes bits `3E99999A`; no positional selection |
| EC93 | collide bounce absent; old bounce is NaN | bounce remains NaN; no 0.3 replacement |
| EC94 | collide: effect converts to 9, death converts to 0.25, deathByWater converts to 1; all flag options present | `D+150=0.25`, `D+154=1`; flags OR `B00` at `D+00`, OR 1 at `D+04`; scalar effect lookup minimum is zero |
| EC95 | collide effect=9; death conversion returns 1.25 | range error; new effect value 9 and collide bit survive; no water/destroy lookup |
| EC96 | ranged-float helper, bounds 0,1; stipulated NaN result | returns unordered value without range error; caller may store NaN |
| EC97 | ranged-float helper, bounds 0,1; return `1+2^-30` in x87 precision | range error even though binary32 rounding would give 1; no destination store |
| EC98 | empty live name; terrainRepel lacks its second nonswitch value | repel bit 400 already set when reader throws minimum-two error; no name error |
| EC99 | terrainRepel selected 2,bad; second conversion throws | repel bit and `D+13C=2` survive; `D+140` and optional fields retained |
| EC100 | terrainRepel selected -2,4,99; scout=5, vertical=6, killHeight=7 | first two scalar fields -2,4; extra value ignored; optional fields 5,6,7; no ordering/clamp or runtime service |
| EC101 | empty live name; randomWalk with no options | only OR `D+00` with `00800000`; all scalar/vector state retained; no RNG call |
| EC102 | randomWalk delay=(5,2); strength=(5,-2); turn=(0.75) | delay 3,7; strength 7,3; turn fields 0.75,0; no reorder |
| EC103 | randomWalk turn=(0.25,2); second ranged conversion fails | first turn field 0.25 and walk flag survive; second turn field retained; wait/preferSea/preferDir unqueried |
| EC104 | randomWalk wait/preferSea present; preferDir successful lookup supplies vector `(0,3,4)` | flags OR `03800000`; vector stored unchanged `(0,3,4)`; no vector normalization; lookup minimum zero |

### 19.25 Particle appearance lists and variation

These five child handlers select at least one nonswitch value, ignoring the
finder return, **before** clearing their corresponding list. They have no
particles-name guard. Convert and append every selected value in selection
order, using the conversions below. Then query `vary` with minimum one;
if present, use only its first selected value. If absent, retain the existing
variation field. No mode flags, runtime resources or collection lookups change.
Allocator capacity is not conformance state; successful ordinary list services
are assumed. Minimum-value failure precedes list clear; a later conversion
failure leaves the successfully appended prefix and prevents option processing.

| Literal / handler | List in D (in P) | Element conversion before append | First `vary` operand and destination |
|---|---|---|---|
| `color`, `colour`, `color255`, `colour255` / `cParticleColorCommand`, `0059BDC0` | vector list `D+94` (`P+184`) | foreign ParseColor, append the three returned binary32 words | owned ranged-vector conversion (below), bounds 0,1 on each component, then store three words at `D+7C,+80,+84` (`P+16C,+170,+174`) |
| `alpha`, `alpha255` / unnamed, `0059C060` | float list `D+A0` (`P+190`) | ParseFloat, round to binary32 | scalar bounded conversion below, binary32 at `D+78` (`P+168`) |
| `size` / unnamed, `005998E0` | float list `D+AC` (`P+19C`) | ParseFloat **times binary32 50**, then round once to binary32; no intermediate binary32 store of the conversion result | scalar bounded conversion below, binary32 at `D+68` (`P+158`); vary is **not** multiplied by 50 |
| `aspect` / unnamed, `00599A10` | float list `D+B8` (`P+1A8`) | ParseFloat, round to binary32 | scalar bounded conversion below, binary32 at `D+6C` (`P+15C`) |
| `rotate` / `cParticleRotateCommand`, `00599B60` | float list `D+C4` (`P+1B4`) | ParseFloat, round to binary32, **no angle factor** in this handler | scalar bounded conversion below, binary32 at `D+70` (`P+160`) |

List operands themselves have no owner-level clamp, positivity or unit-range
test. ParseColor is a foreign primitive, not the owned ranged-vector helper;
do not substitute vary's validation for its input grammar. Negative size and
aspect values and alpha values above one can be stored if conversion returns
them. `rotate` additionally queries `offset` with minimum one **after** vary:
if present ParseFloat its first value to binary32 `D+74` (`P+164`), with
no angle factor or range check; if absent retain the previous offset. It ignores
further offset values. A vary failure prevents the offset lookup.

Scalar vary is an inlined ParseFloat followed by the same inclusive 0,1 x87
comparison policy as §19.22: ordered out-of-range values throw the scalar
range error **before rounding**, unordered NaN returns are accepted, and no
destination write occurs on error. Extra selected vary values are ignored.
The check is on the variation operand, never on the completed list.

The owned **ranged-vector helper** `0059BF10` calls ParseVector3 into a
temporary three-component binary32 value, then checks each component in x,y,z
order against inclusive binary32 bounds, lower then upper. Ordered violations
throw `argument out of range: expecting components [%g..%g], got [%g %g %g]`,
formatting lower, upper, x,y,z as doubles. The instruction tests accept
unordered NaN components, as for the scalar helper. Only if every component
passes are the three output words returned for assignment: an invalid y does
not partially replace the description's x variation. The input vector is
already binary32 before comparison, unlike the scalar vary's unrounded x87
return. This helper is an owned decision, while ParseVector3 is foreign.

For color and alpha, **after** the entire list and successful/absent vary,
inspect the final three bytes of the **raw command record 0**. If and only if
they equal `255`, scale the list in place by binary32 constant bits
`3B808081` (approximately 1/255). Each already-stored component/element is
multiplied in x87 precision and rounded back to binary32, visiting list order
and x,y,z order within each color. This is multiplication by that constant,
not division by an exact integer. Do not clamp the result, and **do not scale
the variation fields**. Non-255 names leave the list unchanged. The descriptor
has no normalization flag (§19.17); the raw command spelling controls this
decision. Registered aliases all have at least three bytes; a direct-handler
fixture with a shorter command name reaches a foreign substring range failure
after the earlier list/vary changes, rather than a special owner syntax error.

Failure order is observable: bad second list conversion leaves an unscaled
one-element prefix even for a 255 alias; bad vary leaves the complete unscaled
list. Existing variation survives a missing vary or failed validation. Repeated
successful commands replace the list, so a later 255 command scales its new
list once, not the old normalized list again. No renderer call occurs here.

Evidence: color selection/clear/ParseColor append `0059BDCB–0059BE44`,
vary/three-word stores `0059BE46–0059BE98`, suffix and scale dispatch
`0059BE9E–0059BEF5`; vector helper comparisons/error `0059BF27–0059BFDB`,
color component multiplications/stores `0059C014–0059C039`; alpha selection,
append and vary `0059C06B–0059C161`, suffix/range path `0059C167–0059C23E`,
scale loop `0059C256–0059C271`; size multiplication/store
`0059993C–0059994F`, constant `00A84D24=42480000`; aspect append
`00599A6C–00599AA3`; rotate append `00599BBC–00599BF3`, vary and raw offset
`00599C0D–00599C8E`. Inlined scalar checks are
`0059999E–005999BB`, `00599AEC–00599B09`, `00599C3C–00599C59`,
`0059C13C–0059C161`; scalar error tails `005999CB`, `00599B19`,
`00599CA1`, `0059C1F7`. Bounds 0,1 are `00A81054`, `00A81228`;
error-format doubles `00A80990`, `00A80AB0`; normalization constant
`00A90BBC=3B808081` and immediate `0059BEF0`.

### 19.26 Particle appearance conformance vectors (unexecuted)

Conversion replies are independent inputs as in §19.21; binary32 operations
assume the ordinary nearest-even mode. These cases do not assert the grammar
of the foreign color/vector/float readers beyond their returning values.

| ID | Fixture | Required result |
|---|---|---|
| EC105 | empty live name; size selection returns 2,-1 | list `D+AC=[100,-50]`; old vary retained; no name check or flags |
| EC106 | size first conversion returns x87 value `1+2^-24` | multiply before rounding; stored first element bits `42480001`; rounding the conversion first would incorrectly produce 50 |
| EC107 | old aspect list `[9]`; selected 2,bad; second conversion throws | new list `[2]`; old aspect vary retained; no vary lookup |
| EC108 | rotate selected 0.25,2; vary=0.5; offset=-3 | list `[0.25,2]` with no angle conversion, vary 0.5, offset -3 |
| EC109 | rotate values valid; vary returns 2; offset present | range error; list survives, old vary/offset retained; offset unqueried |
| EC110 | aspect vary returns NaN; base list `[2]` | NaN stored in aspect vary; no range error |
| EC111 | alpha255 selected 255,128,-255; vary=0.5 | list `[1, bits 3F008081, -1]`; vary 0.5 unchanged by normalization |
| EC112 | alpha selected 255,128 | list `[255,128]`, no normalization; same descriptor as alpha255 |
| EC113 | alpha255 selected 255,128; vary returns 2 | scalar range error before suffix scaling; list stays `[255,128]`; old vary retained |
| EC114 | alpha255 selected 255,bad; second conversion throws | prefix `[255]` survives unscaled; no vary or suffix work |
| EC115 | color255 ParseColor returns `(255,128,0)`; vary ParseVector3 returns `(1,0.5,0)` | color list `[(1,bits 3F008081,0)]`; variation exactly `(1,0.5,0)` without scaling |
| EC116 | colour ParseColor returns `(255,128,0)` | stored vector stays `(255,128,0)`; colour255 with identical replies scales it as EC115 |
| EC117 | color255 list parse succeeds `(255,0,0)`; vary returns `(0.5,2,0)` | vector range error, complete color list remains unscaled; all three old variation fields retained |
| EC118 | color vary returns `(NaN,0.5,1)` | all components accepted and assigned, including NaN; no component normalization |
| EC119 | alpha255 selected 256 with absent vary | result bits `3F808081`, above one; no owner clamp |

### 19.27 Dynamic-particle child dictionary and contracts

`cDynamicParticleEffectCommand::RegisterCommands` (`00595D40–00595DEC`)
registers exactly four nested entries in the order below. Each uses a fresh
four-byte ordinary command descriptor with wrapper `007E3BE0` at +04 and
the listed direct handler at +0C. Registration/dispatch rules remain
effects-parser §12. The dynamic-particle definition's live name is `P+2FC`,
and its description is `Y=P+30C` (§19.13). All **four** child handlers require
that name to be nonempty, throwing `Not in dynamic particle block` before
selection or description writes. They test this name rather than the generic
stack type; the particles name is irrelevant. No option omitted below is
consumed, and generic leftover-switch errors can follow completed writes.

| Order | Literal / pointer cell | Descriptor | Direct handler / verified class |
|---:|---|---|---|
| 1 | `effectBase` / `00B0A388` | `00A9FBB8` | `0059DA40`, unnamed |
| 2 | `model` / `00B0A384` | `00A9FBA8` | `005A1C50`, `cDynamicParticleModelCommand` |
| 3 | `mass` / `00B0A38C` | `00A9FB98` | `00598430`, unnamed |
| 4 | `friction` / `00B0A390` | `00A9FB88` | `005984D0`, `cDynamicParticleFrictionCommand` |

`effectBase` selects at least one nonswitch value, ignores the finder return,
and copies the first selected string preserving case to `Y+0C` (`P+318`).
Further selected values are ignored. There is no default empty assignment on
a missing operand, collection lookup or existence test; the reader supplies
the minimum-value exception. Empty string is assignable if the reader supplies
it. No switches are queried, and the live definition name is unchanged.

`mass` selects at least one nonswitch value, ignores the finder return, and
stores ParseFloat of the first at binary32 `Y+18` (`P+324`). Further values
are ignored. No switch lookup, clamp, inverse-mass calculation, positivity
or finite-value test occurs. A conversion failure leaves the old mass.

`friction` selects at least two nonswitch values, ignores the finder return,
and stores ParseFloat first to binary32 `Y+20` (`P+32C`), then second to
binary32 `Y+24` (`P+330`). Extra selected values are ignored. It then looks
up `angular` with minimum one: if present convert its first value with
ParseFloat to binary32 `Y+28` (`P+334`); if absent retain the existing angular
value. These are direct scalar assignments, without range checks or angle
factors. A failing second conversion retains the completed first store;
a failing angular lookup/conversion retains both positional stores.

`model` selects nonswitch values with minimum **zero** and tests the **full
signed EAX** return. If EAX is zero or negative, return with the description
unchanged. A positive return builds a temporary list of unsigned 32-bit IDs
from **all** selected values, preserving order and duplicates. For each name,
copy the selected string, lowercase each signed input byte through the same
CRT locale service `009F0543` as modelID (§19.4), and look up that key in the
model-name map at `P+B8`. In ASCII, `House` and `HOUSE` select the same key;
the input token string is not modified. A missing key throws
`No such model: '%s'` using the original selected string. It does not insert
a zero map entry, call ParseUint, or load a resource. A present key contributes
its mapped word, including zero, to the temporary list.

Only after every selected name resolves is the description changed. With
exactly one resulting ID, assign it to scalar `Y+30` (`P+33C`), then clear
the word list at `Y+34` (`P+340`). With any other count, replace that word
list with the temporary list **and retain the old scalar ID**. In the normal
positive-return/no-selected-record fixture this replaces the list with empty;
it does not reset the scalar to zero. No option lookups or flags occur.
Unknown-name failure after an earlier successful name leaves both scalar
and description list unchanged because only the temporary prefix was built.
This is different from the incremental appearance and rate-list replacements.
Foreign allocation/assignment exceptions have no added rollback guarantee.

Evidence: dictionary allocations, vptr stores and registration calls
`00595D40–00595DEC`; descriptor +0C words `00A9FBC4`, `00A9FBB4`,
`00A9FBA4`, `00A9FB94`; base-name guard/selection/assignment
`0059DA48–0059DAE4`; mass guard/conversion/store `00598438–005984B6`;
friction guard `005984D8–0059852A`, two scalar stores and angular
`00598533–005985A8`; model guard and signed-return comparison
`005A1C58–005A1CC4`, name copy/lowercase/map-find `005A1D19–005A1D6B`,
temporary append `005A1D6D–005A1DA7`, original-name error
`005A1DE0–005A1E12`, scalar-versus-list publication `005A1E1B–005A1E5B`.
Lowercase wrapper `009F055D` calls `___tolower_mt` (`009F047B`).

### 19.28 Dynamic-particle child conformance vectors (unexecuted)

Normal fixtures have a nonempty dynamic name, unrelated state sentinels,
returning conversion/allocation/assignment services and no unlisted options.
Cases stipulating EAX test the owner branch independently of the reader.

| ID | Fixture | Required result |
|---|---|---|
| EC120 | construct dynamicParticle descriptor, allocations succeed | four four-byte child descriptors registered in §19.27 order with exact literal/handler pairs |
| EC121 | empty dynamic name; particles name nonempty; call each of effectBase/model/mass/friction with missing operands | each throws dynamic-block error before selecting or changing any state |
| EC122 | effectBase selects `MiXeD`, extra | `Y+0C=MiXeD`; extra ignored; no name/collection validation or switch lookup |
| EC123 | mass selects -2,99 | `Y+18=-2`; no inverse, clamp or second conversion |
| EC124 | friction selects 2,bad; second conversion throws | `Y+20=2`; old `Y+24,+28` retained; angular unqueried |
| EC125 | friction selects 2,3,99; angular selects -4 | fields `Y+20,+24,+28=2,3,-4`; 99 ignored, no angular unit conversion |
| EC126 | model finder returns EAX=0 or `FFFFFFFF`, regardless of stipulated scratch indices | no description write or model lookup |
| EC127 | model positive finder, selected `HOUSE`; map house=7; old scalar=9, list `[1,2]` | scalar becomes 7, list cleared; input spelling unchanged; no resource load |
| EC128 | model positive finder, selected `A`, `B`, `a`; map a=7,b=0; old scalar=9 | list becomes `[7,0,7]`, old scalar remains 9; order/duplicates/zero preserved |
| EC129 | model positive finder, selected A,Missing; map a=7 only; old scalar=9,list `[1,2]` | `No such model: 'Missing'`; scalar/list unchanged; no map insertion for missing |
| EC130 | independently stipulated positive full EAX with empty selected list | description list becomes empty; scalar retained; no model lookup |

### 19.29 Sequence child dictionary, `wait` and `play` *[compat]*

The sequence registrar `00596190` installs precisely these two four-byte
ordinary command descriptors, in order. Their class names are unresolved;
the literals alone do not establish C++ class identity. Both descriptors use
the ordinary nested wrapper `007E3BE0` at +04 (effects-parser §12).

| Order | Literal / pointer cell | Descriptor | Direct handler (+0C) |
|---:|---|---|---|
| 1 | `wait` / `00B0A400` | `00A9FCC8` | `005A2400` |
| 2 | `play` / `00B0A3FC` | `00A9FCB8` | `005A20C0` |

Each successful command appends one element to the installed sequence
description's element list at +0C, preserving earlier elements and command
order. An element has two binary32 duration fields at +00,+04 and an owned
effect-name string at +08 (20-byte native element stride). `wait` selects at
least one nonswitch value and makes an element with an empty name; `play`
selects at least two and copies the first selected string as the name,
preserving case and without looking it up. The first duration is ParseFloat
of the first value for `wait`, or the second for `play`. If another selected
value exists, its ParseFloat supplies the second duration; otherwise the
already-rounded first duration is copied bit for bit. Additional selected
values are ignored. These are two endpoints as entered: neither sorting nor
midpoint/radius conversion occurs. Negative, reversed and nonfinite values
have no owner validation. Reader conversion remains a foreign contract.

Both ignore the positional finder's return, consume no switches, and append
only after all required conversions succeed. A missing minimum or conversion
exception leaves the existing element list unchanged. A later generic
leftover-switch exception can follow a successful append. Neither command
checks a live block name, stack type or whether `P+490` is nonnull: successful
operand processing requires the installed description pointer established by
sequence entry (§19.14). A null pointer is not a recoverable `Not in sequence
block` parse error. No effect is created or played by either command.

Evidence: registration `00596190–005961F0`, descriptor words
`00A9FCC8/00A9FCB8`; `wait` conversion/store/copy and append
`005A2441–005A2496`; `play` string assignment `005A2100–005A2123`,
duration stores/copy and append `005A2139–005A218E`; append helper
`005A21C0–005A2212`. Instruction bytes confirm binary32 stores, copying
the rounded first duration on omission, and absence of numeric predicates.

### 19.30 Sequence child conformance vectors (unexecuted)

Static predictions with stipulated reader results, not native/game goldens.
Each successful fixture starts with an installed sequence description S and
an existing element E. Unless specified, conversions succeed.

| ID | Input / fixture | Required result |
|---|---|---|
| EC131 | registrar on empty dictionary | exactly wait/A9FCC8 then play/A9FCB8, both ordinary wrapper, no other entries |
| EC132 | wait selects 3 | S elements `[E,(3,3,empty)]`; flags and name unchanged |
| EC133 | wait selects 4,-2,99 | append `(4,-2,empty)`; 99 ignored; no sort |
| EC134 | play selects `MiXeD`,2 | append `(2,2,"MiXeD")`; no name lookup or effect creation |
| EC135 | play selects Missing,-3,7,99 | append `(-3,7,"Missing")`; fourth value ignored; no missing-effect error |
| EC136 | wait or play; first duration converts, second throws | existing `[E]` unchanged; no partial appended element |
| EC137 | wait first duration returns negative zero, second absent | both stored fields bits `80000000` |
| EC138 | play name assigned; minimum satisfied; first duration returns NaN, second returns infinity | append name and returned binary32 nonfinite values; no owner range exception |
| EC139 | live name empty but nonnull S; wait selects 1; unknown switch remains | append `(1,1,empty)` before generic leftover-switch error |
| EC140 | sequence description pointer null, valid wait values | no owner name/null validation error; append requires nonnull pointer, outside recoverable command domain |

### 19.31 Shake and light child dictionaries *[compat]*

Shake registrar `00595FB0–00596084` and light registrar `00596090–0059618B`
install the following entries in the listed order. Every entry allocates a
fresh four-byte ordinary command, wrapper `007E3BE0` at +04 and the direct
handler at +0C. Repeated descriptors are aliases, not shared object instances.
Only the one child class explicitly named below is resolved by the export;
the other child class names remain unknown, without inference from literals.

| Block | Literal / pointer cell | Descriptor | Direct handler / verified child class |
|---|---|---|---|
| shake | `length` / `00B0A3C8` | `00A9FC78` | `005986A0`, `cShakeLengthCommand` |
| shake | `amplitude` / `00B0A3CC` | `00A9FC68` | `0059A060`, unnamed |
| shake | `frequency` / `00B0A3D0` | `00A9FC58` | `0059A120`, unnamed |
| shake | `shakeAspect` / `00B0A3D4` | `00A9FC48` | `00598760`, unnamed |
| shake | `table` / `00B0A3D8` | `00A9FC38` | `005987D0`, unnamed |
| light | `strength` / `00B0A3E4` | `00A9FCA8` | `0059A1E0`, unnamed |
| light | `length` / `00B0A3E0` | `00A9FC98` | `00598840`, unnamed |
| light | `color` / `00B0A3E8` | `00A9FC88` | `0059D060`, unnamed |
| light | `color255` / `00B0A3F0` | `00A9FC88` | `0059D060`, unnamed |
| light | `colour` / `00B0A3EC` | `00A9FC88` | `0059D060`, unnamed |
| light | `colour255` / `00B0A3F4` | `00A9FC88` | `0059D060`, unnamed |

### 19.32 Shake scalar commands and the light length side effect *[compat]*

These handlers have **no** live-name/block-type guard, including light length.
Coordinates below are parser-relative P, with shake description `P+3D8`
and light description `P+414` (§19.13). They do not create runtime objects.

**Shake `length`** requires at least two raw records (including the command
record), throwing `Not enough arguments` before writes otherwise. It does
not use nonswitch selection: raw record 1 goes to ParseFloat and is stored
as binary32 shake length `P+3D8`. Next it queries `fade` (minimum one
argument); if present, ParseFloat of its first selected argument is stored
at shake fade `P+3DC`. If absent, the previous fade is retained until the
following clamp. After a successful/absent fade, if stored fade is **ordered
greater than** stored shake length, fade is replaced by length's exact
binary32 bits. Equality and unordered comparisons leave fade unchanged.
Thus negative values are allowed, and this clamp does not impose a zero
lower bound. Extra raw/selected values are ignored. Bad length conversion
leaves both fields unchanged; bad fade conversion or missing fade argument
leaves the new length and old fade, without the final clamp.

**Shake `shakeAspect`** has the same raw count minimum/error, converts raw
record 1 with ParseFloat and stores binary32 `P+3F8`. There is no range,
finite-value or sign check and no switch query. Extra records are ignored.
**Shake `table`** also has that raw minimum/error and passes raw record 1
to foreign ParseEnum with table `00B0A4F0` and third argument zero. The
table is exactly `random→0`, `sineY→1`, then a null terminator. Its returned
32-bit value is stored at `P+3FC` without additional validation. Matching,
numeric fallback and error text belong to ParseEnum, not this owner. Either
handler leaves its old destination if its conversion throws. Unlike
nonswitch selection, these raw-record accesses do not consume other switches.

**Light `length`** selects at least one nonswitch operand, ignores the finder
return, and stores its first ParseFloat as binary32 **light** length `P+42C`.
Further selected operands are ignored. It then queries `fade` with minimum
one, but stores a present first ParseFloat at **shake fade `P+3DC`**, not a
light field. It performs exactly the ordered fade-versus-**shake length
`P+3D8`** clamp described above, even when fade is absent. This cross-description
write is required behavior: light duration does not bound the fade.
Bad fade lookup/conversion leaves the new light length and old shake fade
without clamping. The live names of both definitions are irrelevant.

No scalar handler supplies extra defaults beyond retained fields and the
clamp just stated. Unqueried switches reach the generic post-check after
completed writes. Evidence, verified against instruction bytes: shake length
`005986C1–00598751`, binary32 stores `005986F5/00598729`, ordered clamp
`00598732–0059874B` (x87 status mask `41`); aspect `00598760–005987C2`,
store `005987B6`; table `005987D0–00598839`; light length
`0059884E–005988C9`, light store `0059886E`, **shake** store `005988A1`,
comparison/copy `005988AA–005988C3`.

### 19.33 Shake/light curves and light color aliases *[compat]*

These four handlers select at least one nonswitch operand, ignore the finder
return, and replace a list after successful minimum selection. None checks
the live name or generic block type. Selection failure leaves the old list;
then clearing precedes the first conversion, and values append one at a time.
A later conversion exception therefore leaves only the converted prefix.
Order and duplicates are preserved. No range, finite-value or sign check,
interpolation, resource call or switch query occurs.

| Block / literal | Handler | List coordinate | Conversion and units |
|---|---|---|---|
| shake amplitude | `0059A060` | `P+3E0` (shake +08) | each ParseFloat rounded to binary32, no scaling |
| shake frequency | `0059A120` | `P+3EC` (shake +14) | each ParseFloat rounded to binary32, no frequency/unit conversion |
| light strength | `0059A1E0` | `P+420` (light +0C) | each ParseFloat rounded to binary32, no scaling |
| light color aliases | `0059D060` | `P+414` (light +00) | each ParseColor result copied as three binary32 words |

Light color's four aliases are differentiated only by the final three bytes
of raw command record 0. After **all** selected colors append, a `255` suffix
scales every stored component with binary32 constant bits `3B808081`, using
the same x87 multiplication and binary32 rounding helper `0059BFF0` as
§19.25. There is no clamp and no variation option. A non-255 alias stores
ParseColor's values unchanged. Failed color conversion leaves an **unscaled**
prefix even for a 255 alias. A direct fixture whose command name has fewer
than three bytes reaches the foreign substring range failure after replacing
the list. Generic leftover-switch errors follow completed list replacement
and suffix normalization. Missing `vary` is not defaulted or consumed;
unlike particle color, this handler has no variation fields to update.
Light exit may later supply fallback values only if its lists are empty
(§19.14); these handlers do not add fallbacks themselves.

Evidence: amplitude `0059A06F–0059A10D`, frequency
`0059A12F–0059A1CD`, strength `0059A1EF–0059A28D`; their clear and
binary32-store instructions are `0059A091/0059A0C1`,
`0059A151/0059A181`, `0059A211/0059A241`. Color
selection/clear/append `0059D073–0059D115`, suffix and multiplication
dispatch `0059D117–0059D1BD`; scale helper per §19.25. Descriptor and
literal pointer cells in §19.31 were read directly as PE words.

### 19.34 Shake/light child conformance vectors (unexecuted)

Static predictions with stipulated successful reader replies except the
failure explicitly named. No native/game oracle or execution is claimed.

| ID | Fixture | Required result |
|---|---|---|
| EC141 | empty dictionaries; register shake then light separately | shake exactly five entries and light exactly six, in §19.31 order; fresh alias objects, common color descriptor |
| EC142 | shake length raw 3; old fade 4; no fade option | length=3, fade=3 |
| EC143 | shake length raw -2; fade returns -1 | length=-2, fade=-2, no lower bound |
| EC144 | shake length raw 9; fade conversion throws; old fade 4 | length=9 survives; fade=4; no clamp |
| EC145 | shake length raw 3; fade returns NaN | length=3, fade NaN, no ordered clamp |
| EC146 | shake length raw NaN; fade returns 4 | length NaN, fade=4 |
| EC147 | shake length, shakeAspect or table with command record alone | `Not enough arguments` before conversion/write |
| EC148 | shakeAspect raw record 1 returns -5, extra records and switches | P+3F8=-5; extra operands ignored; switches not queried |
| EC149 | table ParseEnum returns 1 from sineY, then stipulated conversion failure on next command | first sets P+3FC=1, next retains 1; table random=0,sineY=1 and third argument zero |
| EC150 | light length selects 100; shake length=5,fade=1; fade returns 9 | light length=100; shake length=5,fade=5 |
| EC151 | light length selects 2; shake length=5,fade=9; no fade switch | light length=2; shake fade=5 |
| EC152 | light length selects 2; shake fade=1; fade conversion throws | light length=2 survives; shake fade=1 retained |
| EC153 | amplitude old list [1]; selects 2,-3,2 | replacement [2,-3,2], no normalization or name guard |
| EC154 | frequency old list [60]; selects 30,bad; second conversion throws | replacement prefix [30] survives |
| EC155 | strength minimum selection throws; old list [7] | old [7] unchanged; no clear or conversion |
| EC156 | strength selects -1,NaN | replacement [-1,NaN], no owner validation |
| EC157 | color255 ParseColor returns (255,128,-255) | list [(1,bits 3F008081,-1)], no clamp |
| EC158 | colour with same ParseColor reply as EC157 | list [(255,128,-255)], no scaling |
| EC159 | colour255 selects two colors, second ParseColor throws | first appended color survives unscaled |
| EC160 | color255 returns (255,0,0); unknown vary switch remains | normalized [(1,0,0)] before generic leftover-switch error; no variation lookup |

### 19.35 Decal child dictionary *[compat]*

Decal registrar `00595DF0–00595FAC` registers exactly eleven entries,
seven unique descriptors, in the order below. Each entry allocates a fresh
four-byte ordinary command with wrapper `007E3BE0` at +04. The listed
handler occupies +0C; only the three explicitly named child classes are
resolved by the export. No block-name/type guard occurs in any of these
seven handlers. Description D is `P+360`; defaults remain §19.13, not
per-command defaults.

| Order | Literal / pointer cell | Descriptor | Direct handler / verified class |
|---:|---|---|---|
| 1 | color / `00B0A398` | `00A9FC28` | `0059CCD0`, unnamed |
| 2 | color255 / `00B0A3A0` | `00A9FC28` | `0059CCD0`, unnamed |
| 3 | colour / `00B0A39C` | `00A9FC28` | `0059CCD0`, unnamed |
| 4 | colour255 / `00B0A3A4` | `00A9FC28` | `0059CCD0`, unnamed |
| 5 | alpha / `00B0A3A8` | `00A9FC18` | `0059CE40`, unnamed |
| 6 | alpha255 / `00B0A3AC` | `00A9FC18` | `0059CE40`, unnamed |
| 7 | size / `00B0A3B0` | `00A9FC08` | `00599CE0`, cDecalSizeCommand |
| 8 | aspect / `00B0A3B8` | `00A9FBF8` | `00599E50`, unnamed |
| 9 | rotate / `00B0A3B4` | `00A9FBE8` | `00599F10`, unnamed |
| 10 | life / `00B0A3BC` | `00A9FBD8` | `005985C0`, cDecalLifeCommand |
| 11 | texture / `00B0A3C0` | `00A9FBC8` | `005A1E80`, cDecalTextureCommand |

### 19.36 Decal curves and variation *[compat]*

All five appearance handlers select at least one nonswitch operand, ignoring
the finder's return. After successful selection they clear their destination
list and append each converted value in selection order. Failed minimum
selection preserves the old list; failed conversion leaves the new converted
prefix. None sorts, clamps or rejects nonfinite list values. Extra selected
values all contribute to the list, rather than being ignored.

| Command | List in D / P | Conversion | Option after completing list |
|---|---|---|---|
| color aliases | `+34 / P+394` | ParseColor, three binary32 words copied | no variation lookup |
| alpha aliases | `+28 / P+388` | ParseFloat, binary32 | vary → `D+4C / P+3AC` |
| size | `+1C / P+37C` | ParseFloat, binary32; **no ×50 scale** | vary → `D+50 / P+3B0`, then cityScale |
| aspect | `+40 / P+3A0` | ParseFloat, binary32 | none |
| rotate | `+10 / P+370` | ParseFloat, binary32; no unit conversion | vary → `D+54 / P+3B4`; **no offset lookup** |

Where present above, `vary` requests at least one argument, parses only the
first with ParseFloat, then checks the unrounded x87 return against inclusive
0 and 1. Ordered values outside that interval throw
`argument out of range: expecting [%g..%g], got %g`; unordered NaN passes.
Successful values round to binary32 at the destination. Missing vary retains
the previous scalar; a conversion/range exception also retains it, but the
new complete base list remains. For size only, presence-only `cityScale`
(minimum zero) is queried **after** successful/absent vary and ORs flag
`10` hex into `D+00`. It never clears that flag when absent. Vary failure
prevents the cityScale lookup and flag update.

After color/alpha lists and, for alpha, successful/absent vary, inspect the
final three bytes of raw command record 0. Exactly `255` causes in-place
scaling by binary32 constant `3B808081`, multiplying each already-stored
component/value in x87 precision and rounding it back to binary32. Color
uses helper `0059BFF0` (§19.25); alpha has its own equivalent scalar loop.
No result clamp occurs. Variation is **not** scaled. A conversion or alpha
variation error prevents scaling, leaving the new list/prefix unscaled.
A direct name shorter than three bytes reaches foreign substring range
failure after preceding list/vary mutations. Other aliases preserve reader
values. These handlers query only the switches explicitly stated; generic
leftover-switch failures can follow their completed writes/scaling.

Evidence, numeric points verified against instruction bytes: color
`0059CCD0–0059CE3E`, scale call `0059CE27–0059CE2D`; alpha selection
`0059CE4B–0059CE53`, float store `0059CEA1`, vary checks/store
`0059CF17–0059CF41`, suffix scalar multiply/store
`0059D036–0059D040`; size list store `00599D3C–00599D41`, vary
`00599DB7–00599DD9`, cityScale `00599DDF–00599DF8`; aspect
`00599E5F–00599EFD`, float store `00599EB1`; rotate
`00599F23–0059A018`, list store `00599F71`, vary checks/store
`00599FE7–0059A009`. Bounds and normalization constant are the same
PE values cited in §19.25. All native list clears precede conversion.

### 19.37 Decal `life` and `texture` *[compat]*

**Life** selects zero or more nonswitch operands. If any exist, it converts
only the first with ParseFloat and stores binary32 duration `D+0C`
(`P+36C`), with no range/finite/sign test. Otherwise duration is retained.
It then queries presence-only options (minimum zero) in the fixed order
below, independent of textual ordering. Missing options retain existing
state. Further selected values are ignored. A failed duration conversion
prevents all option work.

| Option, lookup order | Update |
|---|---|
| static | OR `40` hex into word flags `D+00`; set mode **byte** `D+09=1` |
| loop | set mode byte `D+09=1` |
| single | set mode byte `D+09=2` |
| sustain | set mode byte `D+09=3` |

Thus sustain wins if several modes are present, but static's flag remains
set. No duration default, automatic flag clearing or mutual-exclusion error
is supplied by the command (`005985CE–0059868E`, binary32 duration
store `005985FE`, byte stores `00598627/47/67/87`).

**Texture** processes the following switches **before** nonswitch selection
or any texture-name lookup. Missing switches retain the corresponding state.
Each parameterized switch uses only its first selected argument, leaving
extra selected values unused by this owner.

| Lookup order / switch | Minimum | Conversion and update |
|---|---:|---|
| draw | 1 | ParseEnum with table `00B0A4C0`, third argument zero; store only returned low byte at `D+08` |
| light | 0 | OR flags `D+00` with 2 |
| water | 0 | OR flags with 4 |
| ring | 0 | OR flags with `20` hex |
| repeat | 1 | OR flags with 8 **before** ParseFloat; store binary32 `D+58` (`P+3B8`) |
| offset | 1 | ParseVector2; copy its two returned words to `D+5C,+60` (`P+3BC,+3C0`) after successful parse |

Draw table entries are `decal→0`, `additive→1`, `modulate→2`,
`decalInvertDepth→3`, `decalNoOverlap→4`, followed by null. Matching,
numeric fallback and syntax errors are foreign ParseEnum behavior. There
is no owner enum-range check beyond the byte truncation, nor range or
finite-value validation of repeat/offset. Repeat lookup failure occurs
before its OR; repeat conversion failure preserves the OR and old scalar.
Offset failure preserves both old offset components and earlier changes.

After these options, call nonswitch selection with minimum **one if the
current texture ID `D+04` is zero, otherwise zero**. Inspect the finder's
full signed EAX; if nonpositive, leave the ID unchanged and perform no name
lookup. If positive, use only the first selected name: copy it, fold its
bytes to lowercase with the same foreign transform used in §19.28, then
find the copy in the parser texture map at `P+AC`. A missing entry throws
`No such texture: '<original spelling>'`, preserving the old ID and all
earlier option writes. A found entry's full 32-bit ID replaces `D+04`,
including zero. Extra names are ignored and their existence is not tested.
The original input name is not rewritten, no missing-key insertion occurs,
and no resource load or renderer call occurs. A zero ID remains the signal
for a required name on a later invocation. Generic unconsumed switches are
checked after this handler, so that error need not roll back any writes.

Evidence: enum byte store `005A1EC8`; flags `005A1EEA/1F09/1F28/1F48`,
repeat binary32 store `005A1F65`, offset copies `005A1FA2–005A1FB0`;
minimum and signed finder test `005A1FB6–005A1FD7`; name transform and
map find `005A2030–005A205A`; ID assignment `005A2068–005A206A`;
missing-name exception `005A208B–005A20BD`. Table words `00B0A4C0–00B0A4EC`.

### 19.38 Decal child conformance vectors (unexecuted)

Static predictions with stipulated foreign reader replies; no native/game
golden execution. Coordinates D refer to the decal description.

| ID | Fixture | Required result |
|---|---|---|
| EC161 | registrar on empty dictionary | exactly eleven entries in §19.35 order; seven unique descriptors, each alias a fresh ordinary command |
| EC162 | size selects 2,-3 | D+1C list [2,-3], no ×50; previous vary and cityScale flag retained when options absent |
| EC163 | size selects 2; vary returns 2; cityScale present; old flags 0 | range exception with list [2] retained; old vary retained, cityScale not queried, flags still 0 |
| EC164 | size selects 2; vary returns NaN; cityScale present | list [2], NaN vary, flags OR 10 hex |
| EC165 | aspect selects 2,3; vary remains an unqueried switch | list [2,3] before generic leftover-switch error; no variation update |
| EC166 | rotate selects 90,-90; vary returns 0.25; offset remains | list [90,-90], vary 0.25, no unit conversion/offset lookup; generic leftover-switch error follows |
| EC167 | color255 returns (255,128,-255) | D+34=[(1,bits 3F008081,-1)] |
| EC168 | colour returns (255,128,-255); vary remains | D+34=[(255,128,-255)]; no scaling/variation lookup; later leftover-switch error |
| EC169 | alpha255 selects 255,128; vary returns 0.5 | D+28=[1,bits 3F008081], D+4C=0.5 unchanged by normalization |
| EC170 | alpha255 selects 255,128; vary returns 2 | range exception, list remains [255,128] unscaled, old vary retained |
| EC171 | alpha255 selects 255,bad; second conversion throws | prefix [255] survives unscaled; no vary/suffix work |
| EC172 | any decal curve, minimum selection throws | old destination list retained |
| EC173 | empty live decal name; life no selected operands, all four switches present in arbitrary textual order; old duration 7,flags=0 | duration=7, flags=40 hex, mode byte=3; no name guard |
| EC174 | life selects -2,99, no switches | duration=-2, second value ignored; flags/mode retained |
| EC175 | life first conversion throws with static present | all description fields retained; static unqueried |
| EC176 | texture draw ParseEnum returns 258; light,water,ring present; no name required, finder EAX=0 | draw byte=2; flags OR 26 hex; old ID retained |
| EC177 | texture repeat selected argument conversion throws; old flags=0, repeat=1 | flags=8, repeat=1; no offset/name lookup |
| EC178 | texture offset ParseVector2 throws after light succeeds | flags OR 2 survives; both old offset components retained; no nonswitch selection |
| EC179 | texture ID=0; options succeed; minimum-one finder throws | preceding option changes survive, ID stays 0; no name lookup |
| EC180 | texture ID=7; finder EAX=0 or FFFFFFFF regardless of scratch indices | minimum-zero selection; ID retained, no map lookup |
| EC181 | texture positive finder selects `TeX`,Missing; map tex=0 | ID becomes 0; original spelling unchanged; Missing ignored; next invocation requires name |
| EC182 | texture light succeeds; positive finder selects `MiSs`; no map key | `No such texture: 'MiSs'`; light flag remains set, old ID retained, no missing-key insertion |
| EC183 | texture repeat returns -3, offset returns (2,-4); old flags=1,ID=7; no name | flags=9, repeat=-3, offset=(2,-4), ID=7, no owner numeric validation |

### 19.39 Particle draw options and `texture` *[compat]*

Particle coordinates D are relative to `P+F0`. Owned helper
`ParseDrawOptions` (`005A1220`) is called by particle texture and model
with D. It queries options in this fixed order; absence retains fields.

| Option | Minimum | Update after successful conversion/presence |
|---|---:|---|
| draw | 1 | ParseEnum with table `00B0A408`, third argument zero; store low byte at `D+D4` |
| light | 0 | OR word `D+00` with 1 |
| noCull | 0 | OR word `D+00` with `10` hex |
| sortOffset | 1 | ParseFloat first argument, round/store binary32 at `D+D8` |

Draw table is `decal→0`, `decalInvertDepth→1`, `decalIgnoreDepth→2`,
`depthDecal→3`, `depthDecalMasked→4`, `additive→5`,
`additiveIgnoreDepth→6`, `modulate→7`, then null. It differs from decal's
table (§19.37). Enum matching/fallback/errors belong to foreign ParseEnum.
There is no owner enum range check before byte truncation and no numeric
validation of sortOffset. Parameterized switches use only the first selected
argument. Failure leaves prior option writes; draw conversion failure precedes
all later options, sortOffset failure retains its old scalar after any earlier
flag updates. Evidence: enum store `005A1262`, flag ORs
`005A1281/005A129D`, binary32 store `005A12CF`; table words
`00B0A408–00B0A44C`.

`cParticleTextureCommand` (`005A10B0`) has **no live particle-name guard**.
It first executes that complete draw-options helper, then presence-only
`vflip` and `hflip` (minimum zero), ORing respectively `40000000` and
`20000000` hex into `D+00`. It then selects nonswitch values with minimum
one if current scalar texture/model ID `D+D0` (`P+1C0`) is zero, otherwise
zero. As with decal texture, a nonpositive **full signed EAX** from selection
causes no name lookup or ID change. A positive return uses only the first
selected string, copies it and folds that copy to lowercase, then searches
texture map `P+AC`. A missing name throws `No such texture: '<original
spelling>'` without changing the ID or inserting a key. A match copies its
full 32-bit ID to `D+D0`, including zero. Additional selected names are
ignored, so missing later names do not throw.

The old ID survives name-selection or lookup failure, but all preceding
option changes remain. No model list is cleared here, no mode byte is reset,
and no runtime texture is loaded. Missing options retain earlier values.
Only options above are queried; generic leftover-switch checks follow the
completed handler. Evidence: draw helper call `005A10C7`, flips
`005A10E6/005A1104`, ID-dependent minimum and signed finder branch
`005A110A–005A112B`, map lookup/assignment `005A1193–005A11BD`,
missing-name exception `005A11DE–005A1210`. Folding uses foreign
`009F0543`, as in the other resource-name handlers.

### 19.40 Particle `model` options and binding *[compat]*

`cParticleModelCommand` (`005A1340`) requires the particle live name at
`P+E0` to be nonempty, throwing `Not in particles block` before any writes
or option selection otherwise. It does not use a generic stack-type test.
It then ORs model flags `D+08` (`P+F8`) with 1 and writes mode **byte**
`D+D4=3` (`P+1C4`), before running the draw-options helper (§19.39).
A present draw option can therefore overwrite this byte; absent draw leaves
3. Draw-helper failure retains the initial flag/mode changes and any
completed helper updates. There is no automatic texture-ID/list reset at
this stage.

The following option lookups follow the entire helper, in table order.
All flags are OR operations, with no clearing on omission. Parameterized
options use only the first selected argument; binary32 speeds have no
range, sign, finite-value or unit conversion check.

| Option / lookup order | Minimum | Update |
|---|---:|---|
| fakePerspective | 0 | model flags `D+08` OR 2 |
| moveEntireSlave | 0 | model flags OR 8 |
| slaveApplyAlpha, then applyAlpha **only if slaveApplyAlpha absent** | 0 each | if either queried switch is present, model flags OR `10` hex |
| modelSpeed | 1 | ParseFloat to binary32 `D+1DC` (`P+2CC`), **then** model flags OR `40` hex |
| modelSpeedStatic | 1 | ParseFloat to the same `D+1DC`, without clearing or setting flag `40` |
| sustain | 0 | model flags OR `20` hex |
| applyLighting | 0 | model flags OR 4 |
| noCullFaces | 0 | model flags OR `400` hex |

The alias lookup short-circuits: with slaveApplyAlpha present, applyAlpha is
not queried/consumed and can cause a later generic leftover-switch error.
When both speed options are present, modelSpeedStatic's value wins but
modelSpeed's flag remains set. A failing modelSpeed conversion leaves its
old speed and does not set flag `40`; a failing modelSpeedStatic conversion
retains the successful earlier speed/flag. All preceding state changes survive.

Next select nonswitch values with minimum **zero**, even if the scalar model
ID is zero. If the full signed EAX is nonpositive, leave scalar ID and model
list unchanged. Otherwise resolve **all** selected names into a temporary
ID list, in order: lowercase a copy, find in model map `P+B8`, and append
the mapped full 32-bit ID. Missing name throws `No such model: '<original
spelling>'`. No nonexistent key is inserted. That failure leaves both live
ID destinations unchanged, while earlier flag/mode/speed changes survive.
Order, duplicates and zero IDs are accepted. Input spelling is not rewritten.

On complete resolution, exactly one temporary ID replaces scalar `D+D0`
and clears the live model list `D+1E0` (`P+2D0`). For any other count,
replace that live list and retain the old scalar ID; this includes the empty
list in a direct fixture with independently stipulated positive finder EAX.
These are the same scalar/list publication rules as dynamic-particle model
(§19.28), but with this handler's distinct guard/options and coordinates.
The ID scalar is shared with particle texture; neither successful resolution
nor omitted names loads a model or creates a runtime effect. Generic
leftover-switch checking occurs after these writes.

Evidence, numeric operations checked against instruction bytes: guard
`005A1361–005A13A1`, initial flags/mode/helper `005A13A1–005A13C3`,
short-circuit alpha options `005A1412–005A1436`; speed stores and flag
`005A1467–005A147E`, `005A14AE–005A14B3`; remaining flags
`005A14D4/005A14F3/005A1512`; minimum-zero and signed selection
`005A151C–005A152D`; name folding/find/temporary append
`005A15B0–005A15EA`; scalar/list publication `005A1623–005A164A`,
`005A1688–005A168D`; missing-name error `005A1651–005A1683`.

### 19.41 Particle `timedEffect` insertion *[compat]*

Handler `005A16B0` has no live-name or generic-stack guard. Before any
selection, it ORs `80000000` hex into particle flags `D+00` (`P+F0`).
It selects at least two nonswitch operands, ignoring the finder's return.
The first string is copied as the effect name without case folding or
lookup; ParseFloat of the second rounds to binary32 time. Extra selected
values are ignored, and no switches are queried. No duration sign, range
or finite-value validation, time-unit conversion or effect creation occurs.

On successful parsing, insert one `(name,time)` record in the timed-effect
list at `D+1CC` (`P+2BC`), native stride 16 with owned string at +00 and
binary32 time at +0C. Scan from the existing beginning and insert immediately
before the **first existing time ordered greater than the new rounded time**;
if none, append. Equal times are passed, preserving insertion order among
ties. Unordered comparisons are also passed: a new NaN reaches the end,
while an existing NaN does not itself stop a finite insertion. This is a
linear insertion rule, not a promise that lists containing NaNs are globally
sorted. All existing records and names are retained, with no deduplication.

Failed minimum selection or time conversion leaves the list unchanged but
retains the initial flag OR. On success, a later generic leftover-switch
error leaves the inserted record. Defaults are neither substituted for
missing operands nor written on failure. Allocation failures are not assigned
an invented parse-error contract.

Evidence: initial flag `005A16B9`, selection `005A16C7–005A16CF`,
string copy `005A1702–005A171A`, binary32 time store `005A1735`,
ordered x87 predicate `005A1750–005A175C` (status mask `41`),
insert/append calls `005A1767–005A1780`; element insertion helpers
`005A17A0` and `005A1920`. No class name is established by this handler's
literal or descriptor; its dispatch identity is §19.17.

### 19.42 Particle resource/timed child conformance vectors (unexecuted)

Static predictions with stipulated foreign reader replies and resource maps.
Model fixtures have a nonempty particle live name unless specified.

| ID | Fixture | Required result |
|---|---|---|
| EC184 | draw helper with all options; ParseEnum returns 261; sortOffset returns -2; old flags=0 | D+D4 byte=5, flags=11 hex, D+D8=-2; table decal=0 through modulate=7 as §19.39 |
| EC185 | draw helper light/noCull succeed then sortOffset conversion throws | flags OR 11 hex survives; old sortOffset retained |
| EC186 | texture empty live name, vflip+hflip; ID=7, finder EAX=0 | flags OR 60000000 hex; no name error/lookup; old ID retained |
| EC187 | texture ID=0; draw/light succeeds, minimum-one selection throws | earlier draw/light changes survive; ID remains 0 |
| EC188 | texture positive finder selects `TeX`,Missing; map tex=9 | scalar D+D0=9, later name ignored; model list/mode retained apart from explicit draw changes |
| EC189 | texture hflip succeeds; map lacks selected `MiSs` | original-spelling missing-texture error, old ID retained, hflip survives |
| EC190 | model empty particle live name; all switches present | `Not in particles block` before flag/mode/helper/selection writes |
| EC191 | model no operands/options; old flags=0,scalar=7,list=[1,2], mode=0 | model flags=1,mode=3; minimum-zero selection; scalar/list retained |
| EC192 | model draw ParseEnum returns 7; no names | mode byte=7 overrides initial 3; model flag 1 retained |
| EC193 | model slaveApplyAlpha and applyAlpha present; no names | model flags OR 11 hex; applyAlpha unqueried; generic leftover-switch error after handler |
| EC194 | model modelSpeed=2,modelSpeedStatic=-3 | speed=-3; model flags OR 41 hex; no sign check or static flag clearing |
| EC195 | model modelSpeed conversion throws, old flags=0,speed=7 | initial model flags=1 and mode=3 survive; speed=7, flag 40 hex unset |
| EC196 | modelSpeed=2 succeeds then modelSpeedStatic throws | speed=2 and model flags OR 41 hex survive; no later sustain/name work |
| EC197 | model fakePerspective,moveEntireSlave,sustain,applyLighting,noCullFaces; no names, old model flags=0 | flags=42F hex including initial 1; no name minimum or resource lookup |
| EC198 | model selected `A`, map a=9; old scalar=7,list=[1,2] | scalar=9,list cleared |
| EC199 | model selected A,B,a, map a=9,b=0; old scalar=7 | list=[9,0,9], scalar remains 7 |
| EC200 | model selected A,Missing, map a=9 only; fakePerspective succeeds | original-spelling model error; scalar/list unchanged, model flags OR 3 survives |
| EC201 | model independently stipulated positive finder EAX, empty selected list | model list cleared by empty replacement; old scalar retained; no map lookup |
| EC202 | timedEffect minimum-two selection throws; old flags=0,list L | flags=80000000 hex, list L unchanged |
| EC203 | timedEffect selects `MiXeD`,bad; time conversion throws | flag OR retained, list unchanged, no name lookup |
| EC204 | timed list [(A,1),(B,3),(C,3)]; timedEffect selects `MiXeD`,3,99 | [(A,1),(B,3),(C,3),(MiXeD,3)]; extra value ignored, case preserved |
| EC205 | same initial list as EC204; insert X at -2 | [(X,-2),(A,1),(B,3),(C,3)] with no sign check |
| EC206 | timed list [(A,1),(N,NaN),(B,3)]; insert X at 2 | [(A,1),(N,NaN),(X,2),(B,3)]; unordered existing N is passed |
| EC207 | timed list [(A,1),(B,3)]; insert N at NaN | append (N,NaN); no range exception |
| EC208 | empty particle name; timedEffect valid X,2 with unknown switch left | flag OR and (X,2) append precede generic leftover-switch error; no name guard |

### 19.43 Particle `warp` *[compat]*

`cParticleWarpCommand` (`0059B680`) queries only switches, in the order
below, with no live particle-name or block-type guard. Nonswitch operands
are not selected. Missing switches retain their corresponding fields/lists;
this command does not reset the whole warp description. Coordinates D are
particle-relative `P+F0`. Successful changes survive a later option error
or generic leftover-switch error.

**screw** requests minimum one argument. Its first ParseFloat is multiplied
in x87 precision by the **binary64** constant 0.02 at `00AA02D8`, then
rounded once to binary32 `D+104` (`P+1F4`). There is no owner numeric
validation. Extra selected arguments are ignored. Conversion failure leaves
the old scalar.

**wiggle** requests minimum four arguments. It appends one 28-byte record
to list `D+108` (`P+1F8`), leaving existing records in order. Let the
record's seven binary32 fields be `w0…w6` at +00,+04,…,+18:

- First ParseFloat rounds to `w2`; second ParseFloat rounds to `w0`.
  Fields `w1,w3` are zero.
- Third ParseFloat rounds to an amplitude A. Fourth argument passes the
  owned ranged-float helper (§19.23), inclusive 0…1 with unordered NaN
  accepted, and returns a rounded binary32 phase T.
- Form angle `T × binary32(40C90FDB)` in x87 precision, without an
  intermediate binary32 angle store. Then `w4` is binary32 of x87
  `cos(angle) × A`, `w5=+0`, and `w6` is binary32 of the negated x87
  `sin(angle) × A`. The native trigonometric operations are x87 FCOS/FSIN.

Append occurs only after every conversion and phase check succeeds.
Extra arguments beyond four are ignored. No frequency/amplitude sign or
finite-value check occurs. A failed phase check leaves the old list, not a
partial record. The range helper's error is the scalar range error in §19.23.

**wiggleDir** requests minimum two arguments and appends a separate record
to the same list after successful processing. First ParseFloat rounds to
`w0`; ParseVector3 of the second copies its three returned words to
`w4,w5,w6`, with no normalization. If a third selected argument exists,
its ParseVector3 supplies `w1,w2,w3`; otherwise those three fields are
zero. Arguments beyond the third are ignored. Failure leaves this record
unappended but preserves an earlier wiggle append from the same command.
Neither wiggle variant sets a model flag just by appending.

**wiggleVerts** requests minimum zero and, when present, ORs `100` hex
into model flags `D+08` (`P+F8`). No other work or flag clearing occurs.

**uv** requests minimum **one**, but after parsing/storing its first
ParseFloat at binary32 `D+114` (`P+204`) it unconditionally accesses
the **second selected index** and passes that record to ParseVector2. A
valid two-argument fixture copies the returned words to `D+118,+11C`
(`P+208,+20C`), then ORs `100` hex into model flags. There is no second
argument count check or owner validation of its numbers. A one-value
invocation passes the declared minimum yet is outside the safely bounded
index-access domain; do not invent a minimum-two parse error or default
vector. Vector failure retains the new scalar and both old vector fields,
without this option's flag OR. Extra selected arguments are ignored.

**alpha** requests minimum two arguments. ParseVector3 of the first gives
V=(x,y,z). In x87 precision compute `r=1/sqrt((x×x+y×y)+z×z)` using the
binary32 inputs and binary32 constant 1, with no intermediate binary32 sum
or reciprocal store. Store each `r×component` as binary32 direction
`D+120,+124,+128` (`P+210,+214,+218`); the native store order is z,x,y.
There is no zero-length or finite-value guard, so zero/nonfinite inputs are
not turned into an owner validation exception. Next clear the alpha curve
`D+12C` (`P+21C`) and process **all remaining selected arguments** as
ParseFloat values. Each unrounded return is checked against inclusive 0…1
before binary32 append. Ordered out-of-range throws the same scalar-range
error as §19.36; unordered NaN passes. List order/duplicates are retained.
After the whole curve succeeds, OR model flags with `100` hex.

Alpha conversion failure before the vector result leaves direction/list
unchanged. Failure in a later curve value leaves the new normalized
direction and converted curve prefix; this alpha option's final flag OR
has not occurred, although wiggleVerts or uv may already have set it.
No runtime warp or renderer call is performed here.

Evidence, instruction bytes checked: screw binary64 multiply/store
`0059B6B6–0059B6C4`; wiggle scalar stores `0059B706/0059B731/0059B74C`,
phase/trigonometry/append `0059B769–0059B7A2`; wiggleDir vector copies
and optional third/default `0059B7FD–0059B880`; wiggleVerts
`0059B885–0059B89E`; uv minimum-one and unconditional second-index access
`0059B8A8–0059B8F4`, field stores/flag `0059B8D7–0059B918`;
alpha normalization `0059B952–0059B9A8`, clear `0059B9B7`, range/append
`0059B9EC–0059BA4F`, final flag `0059BA6D–0059BA79`.
Record constructor `0059BAD0` zeros +04…+18; each variant assigns +00
before append. Append/copy helpers `0059BB10/0059BB60` preserve all seven
words and use native stride `1C` hex. Phase constant `00A9F384=40C90FDB`;
normalization constant `00B0A508=3F800000`.

### 19.44 Particle warp conformance vectors (unexecuted)

Static predictions with stipulated reader replies; no native/game execution.
All coordinates D are particle-relative. Valid fixtures have sufficient
selected indices for the accesses specified above.

| ID | Fixture | Required result |
|---|---|---|
| EC209 | empty live particle name; screw first ParseFloat=50 | D+104=1 using binary64 0.02 multiply; no name guard |
| EC210 | screw conversion throws, old D+104=7 | old 7 retained; no later options queried |
| EC211 | wiggle returns first=2, second=3, amplitude=4,phase=0; old list [E] | append seven fields (3,0,2,0,4,0,negative zero); E retained |
| EC212 | wiggle phase returns ordered 2; old list [E] | ranged-float exception; list [E] retained, no partial append |
| EC213 | wiggleDir returns 3 and vector (1,2,3), no third argument | append (3,0,0,0,1,2,3); no vector normalization |
| EC214 | wiggleDir returns 3, vector (1,2,3), third vector (4,5,6), extra fourth value | append (3,4,5,6,1,2,3); fourth ignored |
| EC215 | wiggle succeeds; wiggleDir third-vector parse throws | first append retained; no wiggleDir record appended |
| EC216 | wiggleVerts only, old model flags=2 | flags=102 hex, other state retained |
| EC217 | uv first scalar=2, second ParseVector2=(3,-4); old flags=0 | D+114=2,D+118=3,D+11C=-4; flags=100 hex |
| EC218 | uv scalar=2 succeeds, second-vector parse throws; old flags=0,vector=(7,8) | scalar=2 survives; vector=(7,8), flags=0 |
| EC219 | uv supplies exactly one selected value | declared minimum-one succeeds; no minimum-two error/default vector; second-index access outside safely bounded fixture domain |
| EC220 | alpha vector (0,0,2), remaining values 0,0.5,1; old flags=0 | direction (0,0,1), replacement curve [0,0.5,1], flags=100 hex |
| EC221 | alpha vector (0,0,2), remaining values 0.5,2; old flags=0 | range exception after direction=(0,0,1) and curve prefix [0.5]; flags=0 |
| EC222 | wiggleVerts succeeds; alpha vector (0,0,2), curve value 2 | range exception; flags already 100 hex, direction updated, old curve cleared |
| EC223 | alpha vector (0,0,2), curve returns NaN | direction (0,0,1), curve [NaN], flags OR 100 hex; no owner range exception |

### 19.45 Particle `source` *[compat]*

`cParticleSourceCommand::Parse` (`00597140`) chooses a source shape, writes
the two source-bound vectors, then applies the independent options below.
It does not require positional values or a nonempty particle name. Here P
is the parser context, D=P+F0 is the live particle description. Source bounds
are D+50 (lower) and D+5C (upper); flags A are D+00, flags B D+08.
Every invocation starts with local bounds (-1,0,-1), (1,0,1), irrespective
of previous bounds. Omitted secondary options preserve their old fields.

The following shape lookups short-circuit in table order. Only the first
present shape is consumed. `min` is the finder's minimum argument count;
additional selected arguments beyond those specified are ignored.

| Shape / min | Bounds and additional changes |
|---|---|
| `point` / 0 | Both bounds (0,0,0); if any selected value, ParseVector3 first and use it for both bounds. |
| `square` / 1 | ParseFloat first as r; lower (-r,0,-r), upper (r,0,r). If a second value exists, ParseVector3 it as center and add it to each bound. |
| `quad` / 1 | ParseVector2 first as (a,b); lower (-a,0,-b), upper (a,0,b); optional second value is a Vector3 center added to both. |
| `cube` / 1 | ParseFloat first as r; lower (-r,-r,-r), upper (r,r,r); optional second value is a Vector3 center added to both. |
| `box` / 1 | ParseVector3 first as (a,b,c); lower (-a,-b,-c), upper (a,b,c); optional second value is a Vector3 center added to both. |
| `dice` / 1 | ParseFloat first directly to D+1D8, then set both bounds to the shared zero vector. No center is read. |
| `model` / 0 | OR flags A with 40 hex; retain the local default bounds. No value is converted. |
| `modelBase` / 0 | OR flags A with 80 hex; retain the local default bounds. No value is converted. |
| `city` / 0 | OR flags A with 1000 hex; if values exist, the endpoint-pair helper (§19.19) supplies lower.y and upper.y. X/Z retain the local defaults. |
| `cityWindySide` / 0 | OR flags A first with 1000 hex, then with 2000 hex; optional endpoint pair changes Y as for city. |
| none | Use the local default bounds. |

There is no positivity, ordering, finiteness or overlap test. Negative extents
can produce reversed bounds. Scalar radii are stored as binary32 before
center addition; vector components and each resulting sum are binary32.
Negation uses the parsed value, with binary32 stores before center addition.
Neither dimensions nor centers are multiplied by 50. Shape flags accumulate;
choosing point after model does not clear the model bit. A failed shape
conversion leaves old destination bounds intact; a city endpoint failure can
leave its already-set mode bits. Dice writes D+1D8 before bound publication.

After shape completion, all six bound components are published before the
first secondary lookup. These lookups are independent and occur in this
order, regardless of text order. Masks below are hexadecimal.

| Option / min | Result when present |
|---|---|
| `scaleParticles` / 0 | OR flags A with 400000. |
| `pinToTerrain` / 0 | OR flags A with 4000. |
| `pinToWater` / 0 | OR flags A with 8000. |
| `terrainOnly` / 0 | OR flags A with 10000; set lower height D+158 to zero. |
| `waterOnly` / 0 | OR flags A with 10000; set upper height D+15C to zero. |
| `seaOnly` / 0 | Same upper-height write as waterOnly; then OR flags A with 20000. |
| `lakeOnly` / 0 | Same upper-height write as waterOnly; then OR flags A with 40000. |
| `belowHeight` / 1 | OR flags A with 10000, then ParseFloat first to upper height D+15C. |
| `aboveHeight` / 1 | OR flags A with 10000, then ParseFloat first to lower height D+158. |
| `heightRange` / 2 | OR flags A with 10000; ParseFloat first to lower height, then second to upper height. |
| `killOutsideCity` / 0 | OR flags A with 800. |
| `resetIncoming` / 0 | OR flags B with 200. |

Zero-minimum flags ignore values. Height options ignore values beyond the
listed count and do not reorder or validate endpoints. HeightRange overrides
earlier completed height writes. A second heightRange conversion failure
keeps its new lower endpoint and old upper endpoint. Every later failure
retains bounds, flags, consumed switches and earlier completed writes. Shape
alternatives left unconsumed remain subject to the nested post-check (§19.1).
No renderer, terrain or city service is called by this handler.

Evidence: shape defaults/point `00597159–0059721D`; square/quad
`00597222–005973E1`; cube/box `005973E6–005975B7`; dice and model
`005975BC–0059764E`; bound stores `00597652–0059768C`; secondary options
`00597692–005978AE`; modelBase/city/cityWindySide `005978B9–005979A6`.
Instruction bytes distinguish 32-bit stores and mode ORs. Shared zero at
`00B22CC0` is in the PE virtual zero-fill region.

### 19.46 Particle source conformance vectors (unexecuted)

Each fixture supplies successful reader/conversion services except where
explicitly failing. These are static predictions, not native observations.

| ID | Fixture | Required result |
|---|---|---|
| EC224 | source without options; old bounds arbitrary | bounds (-1,0,-1), (1,0,1); unrelated state retained |
| EC225 | point (2,3,7) | both bounds (2,3,7) |
| EC226 | square 2 with center (10,20,30) | lower (8,20,28), upper (12,20,32) |
| EC227 | quad (2,5), center (10,20,30) | lower (8,20,25), upper (12,20,35) |
| EC228 | cube -2 | lower (2,2,2), upper (-2,-2,-2); no rejection |
| EC229 | box (2,3,5), center (10,20,30) | lower (8,17,25), upper (12,23,35) |
| EC230 | point present together with malformed square | only point lookup is consumed; square not converted; later post-check may reject it after point bounds written |
| EC231 | dice 4 | D+1D8=4, both bounds zero |
| EC232 | cityWindySide with endpoints 8,-3 | flags A OR 3000 hex; bounds (-1,8,-1), (1,-3,1) |
| EC233 | existing model flag; source point without value | zero bounds; model flag retained |
| EC234 | terrainOnly, waterOnly, seaOnly, lakeOnly, belowHeight 4, aboveHeight 3, heightRange 9,2 | height endpoints 9,2; flags A OR 70000 hex; every listed option consumed |
| EC235 | heightRange first conversion 9, second throws; killOutsideCity present | new lower height 9, old upper; flags A OR 10000 hex; new default source bounds retained; killOutsideCity unqueried |
| EC236 | scaleParticles, pinToTerrain, pinToWater, killOutsideCity, resetIncoming | flags A OR 40C800 hex; flags B OR 200 hex; default bounds |

### 19.47 Particle `emit`: rate and launch parameters *[compat]*

`cParticleEmitCommand::Parse` (`0059C280`) operates on the same D=P+F0
as the particle children above. It needs no positional value and supplies no
implicit injection or rate when their switches are absent. It first queries
`inject` (minimum one). When present it resizes D+88's rate list to one,
converts only the first selected value to binary32 element 0, then ORs flags
A with 2. Otherwise it queries `rate` (minimum one); when present it clears
the rate list and appends every selected ParseFloat result as binary32.
The resize/clear failure prefixes are exactly those in §19.19. Inject wins
over rate; the unqueried rate switch remains unconsumed.

It next applies the nine common rate-option lookups from §19.19, in the
same order and with the same destinations and semantics: loop, single,
sustain, scale, areaScale, volumeScale, delay, trigger, and retrigger only
when trigger is absent. Here those operations are part of this owner's
contract even though its instructions contain their own copy. No maintain
lookup occurs. After them it processes the following independent switches
in table order, even if their textual order is different. Omitted switches
preserve the corresponding fields. All finder branches test AL.

| Option / minimum | Selected values and result |
|---|---|
| `velocity` / 1 | First is ParseVector3 v; optional second is ParseFloat speed m (default 1); optional third is ParseFloat variation t (default 0). Set direction bounds D+30 and D+3C to normalized(v) minus/plus (t,t,t); speed endpoints D+48,+4C become (1−t)m and (1+t)m. |
| `speed` / 1 | ParseFloat first as m, optional second as r (default 0), both rounded to binary32. Replace only speed endpoints with m−r, m+r. |
| `dir` / 1 | ParseVector3 first as v; optional second is split into argument records. One subargument: ParseFloat it and use that variation on all axes. Three: ParseFloat subarguments in **Z, Y, X conversion order**, use the resulting X/Y/Z variation vector r. Any other subargument count throws `Expecting float or vector3 for second -dir argument`. Without a second value use r=(0,0,0). Normalize v only if its squared length is ordered greater than zero; set direction bounds to v−r, v+r. Speed endpoints are untouched. |
| `vel` / 1 | ParseVector3 first as v. If its squared length is ordered greater than zero, set m to its length and normalize v; otherwise m=0 and retain v unchanged. Replace both speed endpoints with m and both direction bounds with v. |
| `base` / 0 | OR flags A with 20 hex. Ignore selected values. |

Extra selected values beyond these counts are ignored, except rate's list.
Each row completes before the next; speed overrides velocity's speed only,
dir overrides its direction only, and vel overrides both. There is no owner
test for negative speed, variation, reversed endpoints, or finite values.
Velocity has **no zero-length guard**: zero v reaches division by zero and
the multiply, so ordinary masked x87 execution yields NaN direction bounds.
Dir and vel do guard zero/unordered squared lengths; they retain a zero
vector, rather than inventing an axis. No renderer or runtime emitter is
created. Parsed switches and all completed earlier rows survive exceptions.
Velocity/dir do not publish their bounds until their conversions succeed;
in particular, a dir shape error leaves earlier velocity/speed writes intact
and skips vel/base. The generic post-check can still reject unhandled input.

Numeric qualification (R means binary32 rounding; arithmetic otherwise uses
the x87 intermediate precision): velocity forms q=(x²+z²)+y², computes
f=1/sqrt(q), retains x*f in extended precision and rounds y*f,z*f before
adding/subtracting t. Its parsed m is rounded; its third ParseFloat t is
used without an intermediate binary32 store. All final bounds/endpoints
are binary32. Dir forms q=(z²+y²)+x², tests q directly, uses f=1/sqrt(q)
if ordered positive, retains x*f extended and rounds y*f,z*f before final
bound arithmetic. Its variation components are binary32. Vel forms the
same q, stores R(q) **before** testing the unrounded q; on positive q its
length is sqrt(R(q)), its reciprocal is computed from that length, all
three normalized components are rounded, and length is rounded when
stored to both speed endpoints. Thus these three spellings are not
interchangeable floating-point algorithms. Speed's two sums use rounded
m and r, with binary32 destinations. No ×50 unit scaling occurs.

Evidence: injection/rate `0059C28B–0059C39E`; common rate options
`0059C3A0–0059C575`; velocity conversions and normalization/stores
`0059C57B–0059C6D6`; speed `0059C6DC–0059C75E`; dir split, reverse
conversion order and guarded normalization `0059C764–0059C93C`;
vel q store/test and reload `0059C97A–0059C9BE`, final stores/base
`0059C9C0–0059CA35`; dir error `0059CA46–0059CA62`. Constant
`00B0A508` is binary32 1. Instruction bytes establish the precision
boundaries; the pseudocode's integer casts are not numeric conversions.

### 19.48 Particle emit conformance vectors (unexecuted)

Successful primitive replies are supplied independently. No native execution
or game/shadow oracle has been performed for these predictions.

| ID | Fixture | Required result |
|---|---|---|
| EC237 | emit with no switches and no values | description unchanged; no implicit rate, speed or direction |
| EC238 | inject 3 and rate 4,5 both present | rate list [3], flags A OR 2; rate not queried/consumed; post-check may fail after mutation |
| EC239 | rate first ParseFloat succeeds as 7, second throws | list [7]; loop and later switches not queried |
| EC240 | velocity (0,2,0), speed 8, variation 0.25 | direction lower (-0.25,0.75,-0.25), upper (0.25,1.25,0.25); speed endpoints 6,10 |
| EC241 | velocity (0,0,0), no optional values | no owner exception; masked x87 yields NaN direction endpoints; speed endpoints 1,1 |
| EC242 | velocity (0,1,0),4,0.5 then speed 10,2 | direction lower (-0.5,0.5,-0.5), upper (0.5,1.5,0.5); speed endpoints 8,12 |
| EC243 | dir (0,0,0) with scalar variation 2 | direction (-2,-2,-2), (2,2,2); existing speed retained |
| EC244 | dir (1,0,0) with variation (2,3,4) | conversions for variation occur 4,3,2; bounds (-1,-3,-4), (3,3,4) |
| EC245 | velocity completed, then dir second value splits into two records | dir throws specified shape error; velocity results retained; vel/base unqueried |
| EC246 | vel (3,4,0) after speed and dir | both speeds 5; both directions (R(0.6),R(0.8),0); prior speed/direction overridden |
| EC247 | vel zero vector, then base | both speeds zero, both directions zero; flags A OR 20 hex |
| EC248 | speed -2,-4 | endpoints 2,-6 without sorting or owner rejection |
| EC249 | loop 2,5 and single without value | duration bits 3A83126F, loop count 1, as common options; existing rate retained |

### 19.49 Particle `force`: acceleration and attraction *[compat]*

`cParticleForceCommand::Parse` (`0059A820`) requires no positional values.
All offsets in this and the next section are from **parser context P**,
including description fields and the parser's cumulative tractor state.
Flags A are P+F0 and B P+F8. Absent options retain state. Lookups and
completed mutations occur in the order below, independent of text order.
All switch tests use AL. Extra values beyond the specified conversions
are ignored except where a list explicitly consumes every remaining value.
No option tests sign, finiteness or range; there is no owner rollback.

| Order | Option / minimum | Result when present |
|---:|---|---|
| 1 | `reset` / 0 | Zero acceleration vector P+1D0,+1D4,+1D8 and clear tractor list P+2A8. This does not reset other force fields, mode bits, attraction/explosion lists, cumulative tractor time or multiplier. |
| 2 | `gravity` / 1 | ParseFloat first as g; acceleration.y becomes old y−g, rounded to binary32. X/Z retained. |
| 3 | `wind` / 1 | ParseVector3 first, normalize without a zero-length guard; optional second ParseFloat scales it. Add resulting vector to existing acceleration. No ×50 scaling. |
| 4 | `global_wind` / 1 | ParseFloat first directly to binary32 P+1DC. |
| 5 | `screw` / 1 | ParseFloat first, multiply by **binary64** 0.02, store binary32 P+1F4 (the same field used by warp screw). |
| 6 | `bomb` / 1 | ParseFloat first to binary32 P+1E0. If a second value exists, ParseVector3 it to P+1E4,+1E8,+1EC. Otherwise retain the old vector. |
| 7 | `drag` / 1 | ParseFloat first to binary32 P+1F0. |
| 8 | attraction alternatives | First present of motherDuck / 1, attractor / 1, alphaAttractor / 1, automata / 2; details below. Later alternatives are not queried or consumed. |
| 9 | tractor alternatives | tractorRel / 2, else tractor / 2; §19.50. |
| 10 | `tractorResetSpeed` / 1 | ParseFloat first to binary32 P+2B4. |
| 11 | `tractorTimeScale` / 1 | Rescale tractor state and lifetime as §19.50. |
| 12 | `terrainRepel` / 2 | Sequential ParseFloat stores to P+22C and P+230; then OR flags A with 400 hex. Optional third/fourth/fifth conversions to P+234,+238,+23C. Additional values ignored. |
| 13 | `explosion` / 2 | ParseFloat first to binary32 P+2DC, clear float list P+2E4, then append ParseFloat of **all** remaining selected values as binary32. |
| 14 | `explosionFront` / 2 | ParseFloat first to binary32 P+2F0, then second to P+2E0. |

MotherDuck, attractor and alphaAttractor share one scalar/list operation:
ParseFloat first to binary32 P+2A0, clear float list P+294, then append
each remaining selected ParseFloat result as binary32. A sole scalar
therefore leaves an empty list. On successful completion motherDuck ORs
flags A with 08000000 hex; attractor adds no bit; alphaAttractor ORs
flags A with 04000000 hex and flags B with 10 hex. These bits are never
cleared by switching spellings. None of these three resets integer P+2A4.

Automata is consumed even when it performs no write. It acts only when
the attraction list is empty **or** integer P+2A4 is nonzero. If that
condition holds, set P+2A0=1, ParseUint first to P+2A4, clear the list,
ParseFloat second and append its binary32 value as the sole element.
It ignores further values and adds no flags. If the list is nonempty
and P+2A4=0, neither value is converted and every old field is retained.

Wind numeric rule: from binary32 vector (x,y,z), form
q=(x²+y²)+z² in x87 precision; f=1/sqrt(q). Round normalized X and Y
to binary32. A binary32 copy of normalized Z is also made, but without
the optional amplitude the unrounded Z product is used in the final add.
With amplitude, use its unrounded ParseFloat reply, multiply the rounded
normalized components, round scaled X/Y, and retain scaled Z extended
until its final acceleration add. Each acceleration destination is binary32.
Zero wind reaches division by zero and multiplication; it is not rejected
or silently replaced by zero. Gravity's reply is likewise not rounded
before subtraction. Other direct scalar stores are binary32.

Failure prefixes are observable. A bomb vector conversion failure retains
the new bomb scalar. Attraction scalar failure leaves the old list; a
later element failure leaves the new scalar and successfully appended
prefix, without the branch's final mode bits. Automata's ParseUint failure
retains P+2A0=1 and the old list; its second conversion failure retains
the new ID and empty list. TerrainRepel adds its bit after the second
store, before optional stores. Explosion first failure preserves its list;
a later failure leaves the new scalar and appended prefix. ExplosionFront
second failure retains its new first field. Later options are unqueried
after any exception. The handler calls no wind, terrain, automata or
explosion runtime service; it only records description data.

Evidence: reset/gravity `0059A82D–0059A8BE`; wind precision and addition
`0059A8C4–0059A9A5`; direct scalars/bomb `0059A9AB–0059AAD0`;
motherDuck/attractor/alphaAttractor `0059AAD9–0059AD6F`; automata
guard and writes `0059AD74–0059AE0E`; terrainRepel `0059B153–0059B23E`;
explosion `0059B247–0059B3EC` (tractor vector branch occupies an
out-of-line part of this address range); explosionFront `0059B3F2–0059B43D`.
Binary64 0.02 is at `00AA02D8`; instruction bytes verify the operand width.

### 19.50 Force tractor points, cumulative time and scaling *[compat]*

TractorRel wins over tractor; if present, tractor is not queried. Each
accepts at least two selected values. A new 32-byte tractor record has
position Vector3 at +00, tangent Vector3 at +0C, start time +18 and end
time +1C. Construction initializes all eight binary32 words to zero.
The list at P+2A8 appends one complete record; it is not sorted or
searched for duplicate points. Existing records are retained.

ParseVector3 first as new position p. CountArguments on the second
selected value determines its interpretation: exactly one subargument
uses ParseFloat, any other count uses ParseVector3 (which can itself
reject malformed input). Read the current binary32 multiplier s=P+498
before those conversions. With a scalar, form magnitude R(value/s),
and a tangent direction d=p−previous record's position, or p−source
bounds center if the list is empty. Round each difference to binary32.
If q=(d.y²+d.z²)+d.x² is ordered positive, set tangent to
R(d * (magnitude/sqrt(q))) componentwise; otherwise retain d unchanged.
There is no zero-multiplier guard. With a vector v, use tangent
R(v*(1/s)) componentwise, without normalizing it.

Only **after** tangent calculation, tractorRel with a nonempty list
adds the previous record's position to p, rounding each component.
Thus scalar tangent calculation uses the supplied position before the
relative addition. With an empty list both spellings use the source
center and the supplied position is absolute. Source center is half the
sum of bounds componentwise; X/Y sums remain extended before halving,
whereas the Z sum is rounded before halving. The resulting center is
binary32 (helper `0046FFC0`, half constant binary32 0.5).

The new start time is the current binary32 P+494. If a third selected
value exists, ParseFloat it as duration, compute duration*s + old P+494
with no store between multiply and add, then store the same rounded
result to P+494 and the record's end time. Without a third value, the
record's end remains **zero** and P+494 is unchanged. Extra values are
ignored. Append the record, then OR flags A with 10000000 hex. There
is no check that end exceeds start. A duration conversion failure appends
nothing; a later append failure can leave the newly advanced P+494
without a new record or newly set mode bit. Earlier force mutations remain.

TractorTimeScale converts its first value to binary32 t, replaces P+498
with t, then multiplies, in order, cumulative time P+494 and lifetime
endpoints P+FC,+100 by t, rounding each store. For every tractor record
in list order, multiply tangent X/Y/Z by 1/t (the reciprocal remains
extended), then multiply its start/end times by t, with binary32 stores.
Positions are unchanged. This is a multiplication of the **current**
state, not a ratio against the old multiplier, so repeated invocations
compound the scaling although P+498 holds only the latest t. The same
command's newly appended tractor is included. No zero or negative t
validation occurs. ResetSpeed is not rescaled. Reset does not reset
P+494 or P+498, even though it clears the tractor list.

Evidence: record constructor `0059B450–0059B48F`; shape selection,
scalar division and direction differences `0059AE13–0059AF59`;
guarded normalization `0059AF5B–0059AF88`; relative position addition
`0059AF8E–0059AFC0`; time/append/flag `0059AFC4–0059B017`;
vector tangent branch `0059B2F9–0059B332`; ResetSpeed/TimeScale
`0059B021–0059B151`; append helper `0059B4E0–0059B522` copies
one 32-byte record or grows the list; source center `0046FFC0–0046FFF9`.
These are owned data operations, not calls into a path-following service.

### 19.51 Particle force conformance vectors (unexecuted)

Primitive replies and successful list allocation are stipulated unless a
failure is explicit. These are static predictions without a native oracle.

| ID | Fixture | Required result |
|---|---|---|
| EC250 | force without switches | no state change |
| EC251 | reset; old acceleration (3,4,5), one tractor, attraction list [8], time 9, multiplier 2 | acceleration zero, tractor list empty; attraction list, time, multiplier and flags retained |
| EC252 | acceleration (1,2,3); gravity 4, wind (0,3,0),6 | acceleration (1,4,3) |
| EC253 | reset then wind zero | reset completed; masked x87 produces NaN acceleration; no owner zero-vector exception |
| EC254 | screw 50; global_wind -2; drag -3 | P+1F4=1, P+1DC=-2, P+1F0=-3 |
| EC255 | bomb scalar 7, vector conversion throws; drag present | P+1E0=7, old bomb vector; drag unqueried |
| EC256 | motherDuck scalar 2 and list 3,4; attractor also present | P+2A0=2, list [3,4], flags A OR 08000000 hex; attractor unqueried |
| EC257 | alphaAttractor scalar 5 only | P+2A0=5, empty list; flags A OR 04000000 hex, B OR 10 hex |
| EC258 | motherDuck list conversion throws after one success | new scalar and one-element prefix; no new motherDuck bit; tractor unqueried |
| EC259 | automata malformed values, existing nonempty attraction list and ID=0 | consumed switch; no conversions or writes |
| EC260 | automata ID reply 23, scalar reply 6; list empty | P+2A0=1, P+2A4=23, list [6]; no mode-bit change |
| EC261 | automata ID reply 23 then scalar throws; old ID nonzero | P+2A0=1, ID=23, empty list |
| EC262 | terrainRepel values 2,3,4,5,6 | stores at P+22C,+230,+234,+238,+23C respectively; flags A OR 400 hex |
| EC263 | terrainRepel first 2, second throws | P+22C=2, other repel fields retained; no new 400 hex bit |
| EC264 | explosion first 7, remaining 2,3; explosionFront 4,5 | P+2DC=7, list [2,3], P+2F0=4, P+2E0=5 |
| EC265 | empty tractors, source center zero, s=2, time=3; tractor (0,4,0),6,5 | append position (0,4,0), tangent (0,3,0), times 3,13; cumulative 13; flags A OR 10000000 hex |
| EC266 | previous tractor position (10,0,0); s=1; tractorRel (2,0,0),4 | tangent (-4,0,0) computed before position addition; appended position (12,0,0); end=0 without duration |
| EC267 | s=2, time=7; tractor (1,2,3), vector (4,6,8), no duration | tangent (2,3,4), times 7,0; cumulative stays 7 |
| EC268 | tractorRel and tractor both present | only tractorRel consumed; one append before possible unhandled-switch post-check failure |
| EC269 | tractorTimeScale 2; time=3, lifetime 4,6, tractor tangent (2,4,6), times 1,3 | multiplier=2, time=6, lifetime 8,12, tangent (1,2,3), times 2,6; positions/resetSpeed retained |
| EC270 | repeat EC269 scaling with another tractorTimeScale 2 | multiplier still 2; time=12, lifetime 16,24, tangent (0.5,1,1.5), times 4,12 |
| EC271 | time=7; successful tractor conversions except duration throws | no append, old time=7, no new tractor flag; prior force changes survive |

### 19.52 Visual-effect and effect-group nested dictionaries *[compat]*

The root effectGroup owner (§19.2) registers exactly one child: `instance`,
descriptor `00A9FA08`, direct handler `0059A730`, already specified in
§19.4. Registration `005967D0–00596808` allocates four bytes and installs
that descriptor, or passes null to the generic registrar on allocation
failure. It does not embed a second implementation of instance.

EF16 clarification: the descriptor's +0C word is `0059A730` in both
registrations (root and effectGroup). `00596980` is an instruction inside
the effectsResource exit hook `00596970`, not an instance entry point.
Thus §19.4 and effects-parser §6.1 apply unchanged; EC275's equal-handler
assertion is literal equality. Evidence: descriptor bytes at
`00A9FA08–00A9FA17`, nested registration `005967E2`, root inventory §6.1.

The root effect owner (§19.9) registers the following **19** children in
this order (`cGroupEffectCommand::RegisterCommands`, `00596B70`). The
first 17 each allocate a separate four-byte object. All 17 descriptors
share wrapper `007E3BE0` at +04, no-op `00407990` at +08, and the direct
handler at +0C below. Their +00 destruction entry is `007E3DB0`.
Class names below are only names identified in the export; unnamed
handlers remain unnamed. Execution contracts follow in §§19.53–19.56.

| Order | Literal | Descriptor | Direct handler | Identified owner / execution contract |
|---:|---|---|---|---|
| 1 | particleEffect | 00A9FEAC | 005A0120 | cGroupParticlesCommand; §19.53 |
| 2 | dynamicParticleEffect | 00A9FE9C | 005A0350 | cGroupDynamicParticleCommand; §19.53 |
| 3 | decalEffect | 00A9FE8C | 005A0480 | cGroupDecalCommand; §19.53 |
| 4 | shakeEffect | 00A9FE7C | 005A05F0 | cGroupShakeCommand; §19.54 |
| 5 | flashEffect | 00A9FE6C | 005A08E0 | cGroupFlashCommand; §19.54 |
| 6 | tintEffect | 00A9FE5C | 005A0A90 | cGroupTintCommand; §19.54 |
| 7 | soundEffect | 00A9FE4C | 005A2A50 | cGroupSoundCommand; §19.55 |
| 8 | chainEffect | 00A9FE3C | 0059D830 | unnamed; §19.54 |
| 9 | brushEffect | 00A9FE2C | 005A2CF0 | cGroupBrushCommand; §19.55 |
| 10 | scrubberEffect | 00A9FE1C | 005A30B0 | cGroupScrubberCommand; §19.55 |
| 11 | demolishEffect | 00A9FE1C | 005A30B0 | cGroupScrubberCommand; §19.55 |
| 12 | gameEffect | 00A9FE1C | 005A30B0 | cGroupScrubberCommand; §19.55 |
| 13 | mapEffect | 00A9FE1C | 005A30B0 | cGroupScrubberCommand; §19.55 |
| 14 | automataEffect | 00A9FE0C | 005A3710 | cGroupAttractorCommand; §19.55 |
| 15 | sequenceEffect | 00A9FDFC | 005A3890 | cGroupSequenceCommand; effects-parser §6.2, §§9–12 |
| 16 | cameraEffect | 00A9FDEC | 005A3930 | cGroupCameraCommand; §19.55 |
| 17 | visualEffect | 00A9FDDC | 005A0BD0 | cGroupGroupCommand; §19.53 |
| 18 | select | 00A9FDC0 | enter 00596E70, exit 00596FE0 | unnamed block; §19.56 |
| 19 | particleSequence | 00A9FED4 | enter 00596F50, exit 00596F40 | unnamed block; §19.56 |

There are 16 distinct descriptor addresses. ScrubberEffect, demolishEffect,
gameEffect and mapEffect have separate allocated objects but identical
descriptors, with no alias-specific object word. Select and particleSequence
allocate 24 bytes each and use the generic block initialization contract.
Select calls constructor `00595850` then installs `00A9FDC0`;
particleSequence's constructor `00596F20` calls `00677290` then installs
`00A9FED4`. These constructor symbols have folded or unrelated export
names; they do not prove a new source class name. Both block descriptors
have +00=`007E3C40`, +04=`007E3B10`, +08=`00407990`, +0C=`00676B30`,
+10=`006773D0`, and enter/exit hooks +14/+18 as in the table.

Every allocation failure still calls the generic registrar with null for
that child; its validation/exception behavior is effects-parser §12.
Registration retains/initializes each successful descriptor before
proceeding to the next child. No collection lookup or effect creation
occurs merely from registering this dictionary.

Evidence: complete ordered registrations `00596B70–00596E6C`, literal
pointer cells `00B0A2C4–00B0A30C`; descriptor words at the addresses
above, especially distinct `00A9FE8C` for decalEffect and `00A9FDEC`
for cameraEffect; block construction `00596E05–00596E66`,
`00596F20–00596F31`; effectGroup child `005967D0–00596808` and
literal pointer `00B0A2A4`. A scanning helper that pairs a descriptor
with a later registration can misassign these folded/common rows; the
table is checked against the actual registration instructions.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC272 | successful effect child registration | 19 children in table order, 16 distinct descriptor addresses; 17 four-byte allocations followed by two 24-byte allocations |
| EC273 | compare scrubberEffect/demolishEffect/gameEffect/mapEffect | four allocated descriptor objects with the same 00A9FE1C descriptor and 005A30B0 direct handler; no per-object alias flag |
| EC274 | compare particleEffect, decalEffect and cameraEffect | descriptors FEAC, FE8C, FDEC respectively (suffixes of table addresses); no off-by-one association |
| EC275 | register effectGroup children successfully | one four-byte instance descriptor, same 00A9FA08 and handler as the root instance registration |
| EC276 | register select and particleSequence | distinct block descriptors, generic wrappers shared; enter/exit hooks differ as table; registration does not itself execute enter |

### 19.53 Nested named component references *[compat]*

All nested direct handlers in §§19.53–19.55 first require a nonempty current
effect name (`P+434`, the string belonging to the effect under construction).
Otherwise they throw `Not in effects block`, before positional or option
selection. This is a name-state test, not a test of generic block-stack depth.
The generic dispatcher may reach them through outer dictionaries (§19.1).
Positional selection excludes switches according to effects-parser §9.
Unless explicitly stated, extra positional arguments are ignored by the owner;
unhandled switches are still subject to the generic post-handler check.

These four commands require at least one positional name and use its original
bytes without local case folding. With a nonnull collection at `P+90`, validate
through the table's lookup slot, passing `(name,0)`; a false return throws the
listed error before record creation. With a null collection, validation is
skipped and a local record is still appended. The descriptions themselves are
not fetched/copied here.

| Literal / handler | Collection slot | Failure text | Record type at D+0C |
|---|---|---|---:|
| particleEffect / 005A0120 | +20 | Unknown particle definition: %s | 0 |
| dynamicParticleEffect / 005A0350 | none, even with a collection | none | 16 |
| decalEffect / 005A0480 | +30 | Unknown decal definition: %s | 1 |
| visualEffect / 005A0BD0 | +18 | Unknown visual effect: %s | 2 |

Each constructs a fresh 108-byte description record, copies the name, sets
the type and copies low 16 bits of `P+4A0` to select-group word `D+64`, then
applies all common options (effects-parser §§3,13,14) before appending one copy
to the local component list `P+44C/450/454`. Only particleEffect and
dynamicParticleEffect set/clear bit 2 of flags `D+10` from byte `P+4AC`;
decalEffect and visualEffect retain their fresh flags. There is no renderer
creation or collection insertion. Normal completion destroys the temporary
name after copying it into the list. Failure in validation/options prevents
append; earlier token marks and parser changes survive, as in §19.1.

Fresh-record defaults, verified at `005997B0–005998B7`: empty name, type 0,
flags/count/transform marks 0, identity 3×3 rotation, translation (0,0,0),
scale 1, emitScale (1,1), sizeScale (1,1), LOD bytes (0,0), select group
and weight 0, shells words `D+4E=1`, `D+50=16`, anonymous index `D+68=FFFFFFFF`.
Common options supply default LOD (1,6) even when no option occurs.

ParticleEffect additionally processes `shells` **after** common options,
minimum one value. The first ParseUint supplies low 16 bits to `D+4E`.
If a second value exists, let x be its raw ParseFloat return. Compute
0.5+x, then add 12582912, each at active x87 precision/rounding, then
store binary32 t. Interpret t's raw bits as a 32-bit word and subtract
`4B400000` modulo 2^32 to obtain j. Compare the original x with the
signed integer j−1 loaded through the x87 integer-load operation. Ordered
equality selects j−1; otherwise select j. Store its low 16 bits at `D+50`.
This is the literal biased-rounding algorithm, not a saturating uint cast.
No range check or conversion of a third value occurs. The default second
word remains 16 when only one value is supplied.

Evidence: handlers `005A0120–005A0345`, `005A0350–005A0478`,
`005A0480–005A05E9`, `005A0BD0–005A0D39`; particle flag/options
`005A0210–005A0241`, shells conversion and binary32/integer comparison
`005A0274–005A02E1`. The x87 comparison retains x across the temporary store.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC277 | each of the four names, live effect, collection accepts, no options, select ID 65538, particleSequence byte 1 | append one record with table type, name unchanged, group 2, LOD (1,6); flags 2 only for particle/dynamic |
| EC278 | dynamicParticleEffect with absent definition in a nonnull collection | no validation lookup; append type 16 |
| EC279 | particle/decal/visual collection rejects name | table error; no common options or append |
| EC280 | no current effect; any nested command | context error precedes even positional selection |
| EC281 | particle shells returns uint 65538 and exact float 3; nearest-even PC53 | D+4E=2, D+50=3; raw t=4B400004, j=4, equality with j−1 selects 3 |
| EC282 | particle shells only first value | D+50 remains 16; no second ParseFloat |

### 19.54 Nested light/shake references and chain name *[compat]*

ShakeEffect, flashEffect and tintEffect each require one positional name.
With a collection they validate `(name,0)` through +38 (shake) or +40
(flash/tint); rejection throws `unknown shake definition: %s` or
`unknown light definition: %s`. Null collection skips validation.
Each appends a separate 24-byte entry to `P+458/45C/460`, **not** a common
108-byte component record: initial flags 0, empty string at +4, +10 word 0,
+14 word FFFFFFFF. It then sets flags and copies the name without folding.
Thus subsequent option failure leaves the already appended entry.

| Command | Flags ORed into the appended entry | Additional lookups in order |
|---|---:|---|
| shakeEffect | 1 | noEpicentre (minimum 0), else noEpicenter (minimum 0); if neither present, OR 2 |
| flashEffect | 4 | epicentre (minimum 0), else epicenter (minimum 0); if present, OR 2 and set +10 binary32 to first returned float, or 1000 (`447A0000`) when its index list is empty |
| tintEffect | 8 | none |

The second spelling is not queried when the first is present. These commands
do not apply common transform/probability options, copy the current select
group, or use particleSequence. Each call creates another entry; the three
commands do not merge records by name. Shake with no option has flags 3;
flash with no option has flags 4 and +10 remains zero; tint has flags 8.

ChainEffect (`0059D830`) requires one positional value, copies it verbatim
into the parser's chain-name string at `P+470`, replacing its prior contents,
and stops. It neither validates through the collection nor creates a component
or an auxiliary entry. Common options are not applied.

Evidence: `005A05F0–005A075F`, `005A08E0–005A0A88`,
`005A0A90–005A0BC8`, append helper `005A0760`, 24-byte entry copy
`00588D90`, and chain `0059D830–0059D8E5`. Flash's binary32 store/default
are `005A0A61` and `005A0A72`; shake's short-circuit lookups are
`005A071B–005A0753`.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC283 | shake A, flash B, tint C; no options, live collection accepts | three entries, flags 3,4,8; original names; no 108-byte records |
| EC284 | shake with both noEpicentre and noEpicenter | flags 1; second spelling unqueried/unmarked by this handler |
| EC285 | flash -epicenter with no values | flags 6, +10 bits 447A0000 |
| EC286 | flash epicentre present but conversion throws | appended named entry with flags 6 survives, +10 remains 0 |
| EC287 | chainEffect MixedCase then chainEffect Other | chain string Other; zero collection calls and appends |

### 19.55 Anonymous nested components *[compat]*

These commands first apply the same current-effect guard as §19.53. They
ask the positional reader for minimum zero values. SoundEffect uniquely
rejects any returned positional value with `sound description unimplemented`;
the other anonymous handlers ignore selected positional values. They allocate
and retain a fresh typed description, process the options below in the listed
order, call the shared anonymous insertion helper, and release their local
reference on normal return. They do not inherit a named definition. Allocation
failure is not translated into a recoverable parse status; no success path
with a missing typed object is promised.

`AddAnonDescription` (`005A2960–005A2A42`) does nothing when collection
`P+90` is null, including skipping common options and local append. With a
collection, it creates the fresh §19.53 record with an empty name, calls
collection +44 `(0,typedDescription)` and saves the returned index at `D+68`,
queries the typed description's +00 type into `D+0C`, copies low 16 bits of
current select ID to `D+64`, sets/clears flags bit 1 according to the helper's
boolean argument, applies common options, then appends one local record.
It does not examine the collection insertion result for failure. Collection
insertion precedes options: an option exception can leave an anonymous
collection entry with no local record. Other flags retain fresh defaults;
particleSequence has no special effect here. Collection ownership and type
identity are effects-parser §11.1; this helper does not delegate option
processing or record construction to that service.

**SoundEffect** (`005A2A50`): fresh payload has +0C=0, sound ID +10=0,
location update interval +14=0.5, length +18=0 (floats stored binary32).
Require `name` (minimum 1), copy and lowercase its first value using the same
CRT service as §19.4, and resolve the sound map `P+C4` exactly; no suffix
fallback. Missing option throws `Need at least -name for anonymous soundEffect`;
missing map entry throws `Unknown sound %s`. Set sound ID from the map.
Then `locationUpdateRate` (1): ParseFloat x; if ordered x>0, store 1/x at
+14 using the active x87 arithmetic and binary32 store, otherwise store +0.
Then `length` (1) stores its ParseFloat at +18. Helper boolean is true
exactly when the **stored length** compares ordered equal to zero (either
sign); a NaN is false. It is not the location rate and is not always true.
Evidence: constructor `005BE300–005BE336`, reciprocal `005A2C02–005A2C2B`,
length/store/comparison `005A2C57–005A2C7A`.

**BrushEffect** (`005A2CF0`): fresh fields are brush ID +0C=7, rate +10=1,
length +14=0, apply byte +18=0, zoom word +1C=FFFFFFFF, strength pair
+20/+24=(1,1), width pair +28/+2C=(1,1), level +30=100. Require `name`
(1), lowercase it, resolve brush map `P+D0` first by that key, then by
that key with `brush` appended. If neither exists throw `No such brush: '%s'`;
if name is absent throw `need at least -name for an anonymous brush effect`.
Set +0C from the map. Options in order: `rate` (1) stores binary32 +10;
`apply` (1) overwrites the same rate and sets apply byte 1; `length` (1)
stores +14; `zoom` (1) ParseInt must be 1..5 inclusive and stores value−1
at +1C; `strength` (1), then `width` (1) use ParseFloatRange
(`005965B0–00596622`): convert the first token and store binary32 before
attempting a second conversion. With at least two indices, convert the second
and return the two binary32 endpoints in their input order; with one index,
repeat the first stored binary32 value. Ignore later indices; do not sort;
`level` (1) stores +30. Helper boolean is false. Float stores do not clamp.
Evidence: `005812F0–00581345`, `005A2CF0–005A30A4`.

**ScrubberEffect and all three aliases** (`005A30B0`): no alias changes
defaults or semantics. Fresh +0C flags and +10/+14/+1C/+20/+24/+28/+2C/
+30/+34/+38 words are zero; +18=1, +3C float=16, +40/+44 copy the
same shared word `00B22E48`, +48/+4C=0. The shared word is a constructor
input, not inferred zero from an executable-file image. Process this table
in order; absent options retain the corresponding fresh values.

| Option (minimum values) | Typed payload mutation |
|---|---|
| demolish (1), burn (1), toxic (1) | ParseFloat then CRT _ftol2, low 32-bit result at +14,+24,+28 respectively; these are integer conversions, not float stores |
| extinguishFire (1) | ParseUint at +2C |
| pauseSim (0), pauseSimHidden (0), pauseClock (0) | OR +0C flags 10,20,40 hex respectively; if an optional first value exists, store its ParseFloat binary32 at +4C (later option wins) |
| message (1) | first ParseUint to +30; optional second ParseUint to +34 |
| blob (3) | ParseInt first in 1..8 to +38; float second to +3C, third to +40 **and** +44, optional fourth to +48 |
| rect (4) | ParseInt first in 1..8 to +38; float second to +3C, third to +40, fourth to +44, optional fifth to +48; overrides blob fields when both present |
| noNetworks (0), noFlora (0), dezone (0), single (0) | OR +0C with 1,2,4,8 respectively |
| explode (0), createRubble (0), createBurntRubble (0) | initialize upper portion to 0 and low portion to 1; explode sets upper 1300 hex and low 2; createRubble sets upper 1300 hex; createBurntRubble ORs upper with 0300 hex |
| demolishEffectID (0) | when present convert first selected value with ParseUint and mask FF, replacing low portion; combine upper and low portions by OR at +18 |
| minDemolishSize (1), maxDemolishSize (1) | float stores +1C,+20 |

At least one of demolish/burn/toxic/extinguishFire/pauseSim/pauseSimHidden/
pauseClock/message/blob/rect must be present; otherwise throw
`Need some options for anonymous scrubber effect` **before** querying
noNetworks and subsequent options. Modifiers such as single or minDemolishSize
alone do not satisfy this. The positional minimum on demolishEffectID really
is zero although the present branch dereferences the first returned index;
empty results have no defined successful conversion, not a default ID.
Helper boolean is true. Integer range failures use the §19.1 range error,
before subsequent conversions and helper insertion. `_ftol2` is a separate
CRT numeric service (call `009EEF04`), supplied independently in owner tests;
in its finite signed-integer domain it truncates toward zero. No owner
clamp validates the other fields. Evidence: constructor `005BC9D0–005BCA3F`;
handler `005A30B0–005A3707`, especially conversion calls/stores
`005A316F–005A31F4` and admission check before `005A34EA`.

**AutomataEffect** (`005A3710`, cGroupAttractorCommand): fresh payload
has empty string +0C and group byte +18=0. Query `name` (1) first; if present
copy its first token's original bytes. Otherwise query `group` (1) and copy
that token and set group byte 1; absence of both throws
`Need at least -name or -group for anonymous attractor effect`. When both
occur, group is unqueried and can remain an unknown switch for the generic
check. No map lookup or case folding. Helper boolean true. Evidence:
`00580F10–00580F5C`, `005A3710–005A3889`.

**CameraEffect** (`005A3930`): fresh flags +0C=0, bytes +10/+11=0,
attach radius float +14=0. Options: `slave` (0) OR flags 8; `target` (0)
OR flags 4; `attachRadius`
(1) float store +14; `zoom` (1) ParseInt 1..5, store value−1 in byte +10
and OR flags 1; `rotation` (1) ParseInt 0..3, store value in **the same
byte +10**, then OR flags 2. Byte +11 remains zero: do not repair the
overwriting store. Range checks precede each store/flag. Helper boolean true.
Evidence: `00581C10–00581C42`, `005A3930–005A3B2A`; overlapping byte
stores `005A3A71/005A3AB2`.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC288 | sound named positional argument, live effect | sound description unimplemented; no allocation/insertion |
| EC289 | sound map beep=9; name BEEP, rate 4, length −0, exact arithmetic | typed ID 9, interval bits 3E800000, length 80000000; record flags bit 1 set |
| EC290 | brush map only oakbrush=12; name OAK, rate 2, apply 3, zoom 5 | ID 12, rate 3, apply 1, zoom word 4; record flags bit 1 clear |
| EC291 | scrubber aliases, only -single | all throw admission error before single is queried; no insertion |
| EC292 | scrubber -demolish 2.75; CRT returns 2, explode, ID 511 | integer +14=2, +18=000013FF; helper boolean true |
| EC293 | scrubber pauseSim=2 and pauseClock=3; blob (2,4,5,6), rect (3,7,8,9,10) | flags 50 hex; +4C=3; shape fields +38=3,+3C=7,+40=8,+44=9,+48=10 |
| EC294 | automata name MiXeD and group Other | payload string MiXeD, group byte 0; group unqueried |
| EC295 | camera target, zoom 5, rotation 2 | flags 7, byte +10=2, +11=0; no separate stored zoom 4 |
| EC296 | anonymous helper, null collection, valid typed payload | no type query, common options or local append; local reference released normally |
| EC297 | live collection accepts anonymous payload; common LOD option throws in select | collection insertion retained; no local 108-byte append or normal local-release continuation |

### 19.56 Select and particleSequence block state *[compat]*

Select entry (`00596E70`) requires the current effect name, then increments
32-bit counter `P+4A4` modulo 2^32, copies that new word to active select ID
`P+4A0`, and saves the **current component count** at `P+4A8`. It then
calls its inherited +0C block-parser hook; no positional arity or local
option processing is imposed before that call. Entry mutations survive a
throw from that hook. Generic block push occurs only after entry returns.

Select exit (`00596FE0`) first clears active select ID to zero. Consider all
108-byte component records from saved index `P+4A8` to current end. Let Z
be the number with zero 16-bit weight at +66 and A the **count** of those
with positive weight. A is not the sum of weights. If Z>0 compute unsigned
32-bit q=(65536−A)/Z with integer truncation, assign low 16 bits of q to
every formerly zero weight, and add q modulo 2^32 to A for each such record.
Existing positive weights are untouched in this pass. Finally, if the entire
component vector is nonempty, add low 16 bits of (65535−A) to its **last
record's** weight, modulo 65536. This final adjustment occurs even if the
saved range is empty, and even if Z=0. There is no probability clamp,
normalization by sum, rejection of empty select, or remainder redistribution
to any earlier record. Auxiliary shake/light entries do not participate.

Nested select blocks have no owner nesting guard or saved stack of IDs/start
indices: inner entry overwrites `P+4A0/4A8`; inner exit clears ID to zero,
and outer exit uses the overwritten start. Do not restore outer select state.
Counter wrap to zero likewise has literal consequences for prob/LOD options.
Evidence: entry writes `00596EC2–00596EF7`; exit count/unsigned division
`00597033–00597060`, replacement `00597075–00597088`, final whole-list
tail adjustment `005970B4–005970CD`.

ParticleSequence entry (`00596F50`) requires the current effect name, sets
byte `P+4AC=1`, then calls the inherited +0C hook. Exit `00596F40` simply
sets that byte to zero. No nesting guard, prior-value restoration, select-ID
change or automatic list insertion occurs. Thus inner exit also disables
an outer particleSequence. Its effect on newly appended particle/dynamic
records is §19.53; it does not retroactively rewrite older records.

These hooks register no private child dictionary: they use the generic
lookup/fallback and push/pop timing in effects-parser §12. Their constructor
identities remain unnamed (§19.52); do not invent a source class name from
the literal or folded export name.

| ID | Fixture | Required result (unexecuted) |
|---|---|---|
| EC298 | enter select with counter FFFFFFFF and component count 3 | counter and active ID 0; saved start 3; prob subsequently fails as outside select |
| EC299 | select range weights [0,0] | q=32768, A after filling=65536; final [32768,32767] |
| EC300 | select range weights [100,200] | Z=0,A=2; final [100,197], not normalized by the sum |
| EC301 | select range [0] | q=65536 stores 0; A=65536; final tail weight 65535 |
| EC302 | empty select after an older component of weight 9 | old last weight becomes 8; empty whole vector stays empty |
| EC303 | outer select then inner select; inner closes then outer closes | inner ID/start overwrite outer; both exits clear ID; both use inner saved start, each makes its own tail adjustment |
| EC304 | nested particleSequence, inner exits, then particleEffect | particleSequence byte 0; new particle record flags bit 2 clear |
| EC305 | camera slave and target, no numeric options | flags 12, bytes +10/+11 zero; helper boolean true |

EF15 closure inventory: root owners/descriptors map to §§19.2–19.16 and
effects-parser §6.1; particle child descriptors to §§19.17–19.28,
19.39–19.51; sequence to §§19.29–19.30; shake/light to §§19.31–19.34;
decal to §§19.35–19.38; visual-effect/effectGroup nested descriptors to
§§19.52–19.56. The descriptor tables in those sections define the inventory,
including unnamed registered owners, aliases, constructors and block hooks;
only positively identified export class names are assigned. This closes the
registered `(anonymous_namespace)::c*Command` execution publication requested
by EF15. Generic reader/registrar/error transport, numeric conversions,
collection, renderer, audio and resource primitives remain the individually
specified service boundaries, not an unimplemented command-family catch-all.
