# Spec: particle effect components (`cSC4ParticlesEffect`, `cSCParticleSimulatorBase`, the particle description)

Status: **behaviour spec, partial**, clean-room input. This document describes *behaviour*, not code. It was written
by the analysis team from observing the original program and its data files. Implementers must work from this spec,
the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words and tags (*[compat]*, *[choice]*, *[inferred]*, *[quirk]*) as in [foundations.md](foundations.md).
Written 2026-10-21 by the re-particles4 lane. Evidence: the Ghidra export (instruction bytes, call and data
references) only; the shared REA session was unavailable (no app open), so no fresh pseudocode was used. Every
address is SimCity 4.exe GOG 1.1.641.

Scope: the **particle component** of a visual effect (component type 0 in [effects.md](effects.md) §3): which
class is built, the particle description record (on disk and in memory), the shared simulator base (start, stop,
particle list, particle pool, finish), the start of a particle component and the order of its per-frame update.

Out of scope here: the effects manager, the directory resource as a whole, the finish test and brush/game components
([effects.md](effects.md) §3–§9); the dynamic-particle family (`cSC4DynamicParticle*`, root version 4); decals; the
vertex-buffer drawing of particles (§10 gaps). The emitter self-stop is in effects.md §7.1 (with one correction, §8).

## 0. What this spec settles and what it leaves open

| Owner | Status here | Where |
|---|---|---|
| `cSCParticleSimulatorBase` (the eleven functions `0x005B9CC0`–`0x005B9FB7`, slots of §9.2) | **fully specified** | §4, §5 |
| `cSC4EffectsParticle` (constructor `0x005B9DF0`) | **fully specified** | §5 |
| `cSC4ParticlesDescription` (`0x005B1660`, `0x005B1CB0`) | **fully specified** | §2 |
| `cSC4ParticlesDescriptionBase` (constructor `0x005B0FE0`, copy `0x005B17F0`) | **specified** (layout, defaults, file order); meanings partly open (PT1) | §2, §3 |
| `cSC4ParticlesDescriptionBase::cTractorPoint`, `::cWiggle` | **partial** (element shape only) | §2 |
| `cSC4ParticlesEffect` | **partial**: specified identity, factory, `Start` (complete), `SetDescription` (the fields §5.1 and `Start` read), `InitEffectsParticle` (complete), `Stop`, update order, emitter dispatch, pool, slots; open: physics, forces, height-based emission, the three emitters' rate integration, rendering (§10) | §1, §5.1, §6, §7, §8, §9, §10 |

## 1. Identity *[compat]*

Three classes implement the particle component. All share the simulator base sub-object at `+4`.

| Class | Constructor | Main vtable | `+4` vtable | Factory |
|---|---|---|---|---|
| `cSC4ParticlesEffect` | `0x005B2B50` | `0x00AA0EA0` | `0x00AA0E58` | `0x00590BC0` |
| `cSC4GeometrySourcedParticlesEffect` | `0x005A8ED0` (calls the base constructor first) | `0x00AA0B88` | `0x00AA0B40` | `0x00590EA0` |
| `cSC4ModelBasedParticlesEffect` | `0x005AC510` (likewise) | `0x00AA0C58` | `0x00AA0C10` | `0x00590D40` |

The `+4` vtable names come from the ABI table (docs/abi/vtables.tsv); the main vtables are paired with them by the
constructors, which store both (`0x005B2B66`/`0x005B2B6C`, `0x005A8ED9`, `0x005AC51A`).

**Which class is built** (`CreateComponentFromDescription`, `0x00591748`–`0x005917A9`). For a component of type 0,
the description is the particle table entry at the component's description index. The manager builds **one particle
component per copy**, the copy count being the component record's `u16` at `+0x4E` (a count of 0 builds none). For
each copy:

1. description `+0x08` bit 0 clear → `cSC4ParticlesEffect`;
2. bit 0 set and description `+0x1D8` equal to 0.0 (an unordered compare counts as "not equal") →
   `cSC4ModelBasedParticlesEffect`;
3. bit 0 set otherwise → `cSC4GeometrySourcedParticlesEffect`.

## 2. The particle description *[compat: data format]*

`cSC4ParticlesDescription` is `cSC4ParticlesDescriptionBase` (`0x204` bytes, no vtable) followed by a `u32`
reference count at `+0x204` (constructor `0x005B1660` sets it to 0; the description is held by
`cRZAutoRefCount`). Its assignment (`0x005B1CB0`) is the base's assignment `0x005B17F0`, a member-by-member copy
that deep-copies the vectors and the string *[inferred from its callees]*; the reference count is not copied.

**The section** (inside `cSC4EffectsResource::Read`, `0x005A5225`–`0x005A52F4`, after the `u16` root version): a
`u16` section version, a `u32` count, then that many records. Section version **1** uses reader A
(`0x005B0B50`); any other version uses reader B (`0x005B1DC0`). Reader B differs from A in exactly two places, marked
**B** below.

Value encodings: `u8`, `u16`, `u32` little-endian; `f32` IEEE single; `vec2`/`vec3` 2 or 3 `f32`; `box` 6 `f32`
(min xyz, max xyz); `f32[]` a `u32` count then that many `f32`; `vec3[]` a `u32` count then that many `vec3`;
`u32[]` likewise; `string` a `u32` length then the characters (no terminator).

| Offset | File | Default (constructor) | Meaning (source of the name) |
|---|---|---|---|
| `+0x00` | `u32` | 0 | **flags A** (§3) |
| `+0x04` | **B only**: `u32 & 0xFF` | 0 | **flags C**; bit 0 `destroyBuildings` (collision) |
| `+0x08` | `u32 & 0x7FF` | 0 | **flags B** (§3) |
| `+0x0C` | `vec2` | 2.0, 2.0 | particle life range in seconds (`life a b` gives a−b, a+b); each particle's life is a uniform draw in it (§5.1) |
| `+0x14` | `f32` | 1.0 | the loop duration in seconds (§8.2; values ≤ 1e-6 act as 1e-6) |
| `+0x18` | `u32` | 0 | emit loop count (effects.md §7.1); 0 = endless |
| `+0x1C` | `f32` | 0 | `preroll` seconds; also caps the emitted time of one frame at `preroll − 0.01` when > 0 (§8.1) |
| `+0x20` | `vec2` | −1, −1 | `maintain … delay` range: the loop interval; a min ≥ 0 seeds the start countdown (§6 step 6) |
| `+0x28` | `vec2` | −1, −1 | the second interval range: seeds the start countdown when `+0x20` min < 0 and this min ≥ 0 (§6 step 6) |
| `+0x30` | `box` | min (0,1,0), max (0,1,0) | the direction box: each particle's initial velocity direction (§5.1) and the random-walk start vector (§6) |
| `+0x48` | `vec2` | 0, 0 | the initial speed range (§5.1) |
| `+0x50` | `box` | min (−1,−1,−1), max (1,1,1) | the emission volume (`source`); copied to the component's `+0x200..+0x214` by `SetDescription` (§6.1) |
| `+0x68`..`+0x78` | 5 × `f32` | 0 | variance amounts (§5.1): `+0x68` size, `+0x6C` the particle's `+0x2C` factor, `+0x70` rotation range, `+0x74` rotation `offset` (not read by §5.1), `+0x78` the particle's `+0x34` factor |
| `+0x7C` | `vec3` | three copies of the global at `0x00B22D54` (PT3) | colour; at particle creation each component is read as a variance amount for the particle's `+0x38..+0x40` factors (§5.1) |
| `+0x88` | `f32[]` | `[30.0]` | the emission rate curve (`rate`, `inject`, `maintain`) |
| `+0x94` | `vec3[]` | one entry (the vector at `0x00B0A5D4`) | the colour curve |
| `+0xA0`, `+0xAC`, `+0xB8` | `f32[]` each | `[1.0]` each | three further curves (size/alpha family, PT1) |
| `+0xC4` | `f32[]` | `[0.0]` | the rotation curve |
| `+0xD0` | `u32` | 0 | texture id (`texture`) |
| `+0xD4`, `+0xD5` | `u8`, `u8` | 0, 0 | `+0xD5` the alignment mode |
| `+0xD8`, `+0xDC` | `f32`, `f32` | 0, 0 | |
| `+0xE0` | `vec3` | the global at `0x00B22D54` | the constant force (gravity) |
| `+0xEC` | `f32` | 0 | the wind gain (`force -global_wind`, §7 step 3) |
| `+0xF0` | `f32` | 0 | radial speed (`force -bomb`; the vector that follows it is the centre `+0xF4`): added along the direction from the centre `+0xF4` to the new particle (§5.1) |
| `+0xF4` | `vec3` | the global at `0x00B22D54` | the radial centre (local space, §5.1) |
| `+0x100`, `+0x104` | `f32`, `f32` | 0, 0 | `+0x100` the drag coefficient (`force -drag`, §7.1); `+0x104` := `force -screw` value × 0.02 (`0x00AA02D8`; reader not traced) |
| `+0x108` | `cTractorPoint[]`: `u32` count, each two `vec3` (reader `0x005B0930`) | empty | tractor points |
| `+0x114` | `f32` | 0 | `wiggle` |
| `+0x118` | `vec2` | 1.0, 0 | |
| `+0x120` | `vec3` | 0, 1.0, 0 | |
| `+0x12C` | `f32[]` | empty | the warp alpha curve |
| `+0x138` | `f32` | 0 | collision `bounce` |
| `+0x13C`..`+0x148` | 4 × `f32` | 0 | `terrainRepel` (two numbers, `scout`, `vertical`) |
| `+0x14C` | `f32` | −1.0e9 | `killHeight` |
| `+0x150`, `+0x154` | `f32`, `f32` | 0, 1.0 | collision `death` / `deathByWater` |
| `+0x158` | `vec2` | −10000, 10000 | the height window (`aboveHeight`, `belowHeight`, `heightRange`) |
| `+0x160` | `string` | empty | the block's own name (written by the particle block header `0x0059D8F0`); with `collide` (flags A `0x100`) `SetDescription` looks up `<name>_terrain` and `<name>_water` (§6.1) |
| `+0x16C`, `+0x16E` | `u16`, `u16` | 0, 1 | |
| `+0x170` | `f32` | 1.0 | |
| `+0x174` | `vec2` | 5.0, 5.0 | random-walk `delay` range |
| `+0x17C` | `vec2` | 50.0, 50.0 | random-walk `strength` range |
| `+0x184`, `+0x188` | `f32`, `f32` | 0.1, 0.2 | random-walk parameters |
| `+0x18C` | `vec3` | the global at `0x00B22D54` | random-walk `preferDir`; differs from the global `0x00B22D70` by ≥ 1e-4 in any component → the preferred direction is on (§6.1) |
| `+0x198`, `+0x19C`, `+0x1A0` | 3 × `f32` | 0 | `windBank` and two alignment numbers |
| `+0x1A4` | `f32[]` | empty | attractor curve; empty or `+0x1B0` ≤ 0 disables the attractor (§7 step 2) |
| `+0x1B0` | `f32` | 1.0 | attractor range: the distance divisor of the attractor-point pull (§7.1); written by the value of `force -attractor`, `-motherDuck` or `-alphaAttractor` |
| `+0x1B4` | `u32` | 0 | |
| `+0x1B8` | `cWiggle[]`: `u32` count, each two `vec3` (reader `0x005B09D0`) | empty | wiggles |
| `+0x1C4` | `f32` | 0 | |
| `+0x1CC` | **B only**: a string list (reader `0x005B1D60`) | empty | particle age events: (name, age) pairs fired at birth or when a particle crosses the age (§8.3) |
| `+0x1D8` | `f32` | 0 | the source scale; picks the derived class (§1) |
| `+0x1DC` | `f32` | 0 | |
| `+0x1E0` | `u32[]` | empty | |
| `+0x1EC` | `f32` | 0 | |
| `+0x1F0` | `f32` | 4.0 | |
| `+0x1F4` | `f32[]` | `[400.0]` | |
| `+0x200` | `f32` | 4.0 | |

The names in the last column are the keywords of the effect-script compiler that write each field (the parser
writes the description at its block's `+0xF0`, `cParticleEffectCommand::EndBlock` `0x005A1A50`). A blank meaning
is a field the runtime reads but this lane did not trace (PT1). An implementation must still read and keep every
field, in this order, to stay aligned.

## 3. The flag words *[compat]*

**Flags A** (`+0x00`):

| Bit | Keyword / use |
|---|---|
| `0x1` | self-lit: the draw context's self-lit shade is set before drawing and reset after (§7 steps 7, 9) |
| `0x2` | `inject` / `rate … inject` |
| `0x4` | `maintain` |
| `0x8` | `emit … sustain`: allows a restart by `Start(1)` (§6) |
| `0x20` | half-length lead at creation (§5.1 step 8) |
| `0x10` | not set by the `force` parser (`0x0059A820`); not traced (PT1) |
| `0x4000000` | `force -alphaAttractor` (also sets flags B `0x10`); its reader is not traced (PT1) |
| `0x8000000` | `force -motherDuck`: the attractor-point pull of the stepped physics path (§7.1 step 2.3) |
| `0x10000000` | `force -tractor`: the tractor replaces the position step (§7.1 step 2.1) |
| `0x100`, `0x200`, `0x800` | `collide`; `sticky`; `killOutsideCity` |
| `0x400` | `terrainRepel` |
| `0x1000`, `0x2000` | source shape bits (`square`, `model` …); `0x2000` set → the city-side wind emission volume is recomputed each frame (`0x005B7030`) |
| `0x4000`, `0x8000`, `0x10000`, `0x20000`, `0x40000` | `pinToTerrain`, `pinToWater`, height window on, `seaOnly`, `lakeOnly` |
| `0x80000`, `0x100000`, `0x200000` | `emit … scale`, `areaScale`, `volumeScale` |
| `0x400000` | `scaleParticles` |
| `0x800000` | `randomWalk` (§6, §7 step 10) |
| `0x1000000` | random-walk `turn` |
| `0x2000000` | random-walk `preferSea` |
| `0x80000000` | an extra pre-pass `0x005B8410` each frame (§7 step 4) |

**Flags B** (`+0x08`, masked to 11 bits on load): bit `0x1` model/geometry source (§1); `0x10` `force -alphaAttractor`; `0x80` `windBank`; `0x100`
`wiggle`/`warp`; `0x200` `resetIncoming`.

## 4. The simulator base (`cSCParticleSimulatorBase`, at `+4` of the component) *[compat]*

State, offsets relative to the base sub-object:

| Offset | Field |
|---|---|
| `+0x05` | `started` (u8) |
| `+0x08` | reference count |
| `+0x0C` | `delayed_stop` (u8) |
| `+0x0D` | `release_when_done` (u8) |
| `+0x10` | the live-particle list: a circular doubly linked list with a heap sentinel; each node holds one particle pointer |
| `+0x14` | the live count |

**Start** (`0x005B9CF0`): succeeds, and returns true, only if (`started` is clear **or** a delayed stop is pending)
**and** `release_when_done` is clear. On success it clears `delayed_stop` and `release_when_done` and sets
`started`. Otherwise nothing changes and it returns false.

**Stop(delayed, release_when_done)** (`0x005B9E70`): does nothing unless `started`. Then `delayed_stop` :=
`delayed`.

- `delayed` = 0 (**immediate**): `started` := 0, `release_when_done` := 0; every particle in the list is handed to
  the class's `DestroyParticle` (slot 16 of the `+4` vtable) with the live count decremented for each; the list
  nodes are freed and the list is empty. Returns true.
- `delayed` ≠ 0: `release_when_done` := the second argument; if that is non-zero the component takes a reference on
  itself (slot 1). The particles keep running. Returns false.

**CheckForStop** (`0x005B9D40`), run at the end of every frame update (§7 step 10): only when the live count is 0:
if `delayed_stop` is set it calls `Stop(0, 0)` and returns true; else if `release_when_done` is set it calls
`Shutdown` (slot 5) then `Release` (slot 2) and returns true. Otherwise false.

**Release** (`0x005B9DB0`): above 1 it decrements and returns the new count; at 1 it calls `FinalRelease`
(slot 10) and returns 0 without storing the decrement. **FinalRelease** (`0x005B9DD0`): `Shutdown` (slot 5) then the
deleting destructor (slot 3 with 1).

**AddParticle(p)** (`0x005B9F80`): appends a node holding `p` at the tail and increments the live count.
**RemoveParticle(p)** (`0x005B9D80`): only clears the particle's first byte (the "alive" byte); the node stays until
the next update sweeps it (§7 step 5). **IsInDelayedStop** slot 9 is the shared function `0x005B9D30`.

## 5. Particle records and pools *[compat]*

Particles come from process-wide free lists, never from the general heap one by one; an empty list is refilled by
`0x005A61B0` (PT4). A pool is four words: record size, block
bytes, block chain head, free-list head; their file images are the initial values. The refill allocates block
bytes + 8 from the heap, links the block into the chain through its first word, and threads ⌊block bytes / record
size⌋ records starting at block `+4` into the free list (the last link is null). Effect particles (`0x00B0A67C`,
free head `0x00B0A688`): **68-byte** records, 8,704-byte blocks = 128 records per refill. Base particles
(`0x00B0A694`, free head `0x00B0A6A0`): **28-byte** records, 3,584-byte blocks = 128 per refill. Records go back to
the free head on `DestroyParticle` (`0x005B2240` and its base twin); blocks are never returned during play, only
by the exit-time functions that walk each chain (`0x00A7B930` and `0x00A7B980`, the same shape as `0x00A7B900`).

- Base particles (`cSCParticleSimulatorBase::CreateParticle` `0x005B9E20`): free list head `0x00B0A6A0` (pool
  object `0x00B0A694`). The particle's first 28 bytes are zeroed. *[quirk]* Three words at `+0x0C`, `+0x10`,
  `+0x14` are first set from the global at `0x00B22E20` and then overwritten by the zeroing: the net state is all
  28 bytes zero. `DestroyParticle` (`0x005B9D90`) pushes the record back on the list head (null is ignored).
- Effect particles (`cSC4ParticlesEffect::CreateParticle` `0x005B7C20`, slot 15 of the `+4` vtable): free list head
  `0x00B0A688`. The `cSC4EffectsParticle` constructor (`0x005B9DF0`) gives the same 28-zero-byte prefix (same quirk);
  six further `f32` fields are set from the global at `0x00B22D70`, then `InitEffectsParticle` (`0x005B7580`)
  fills the particle from the description (PT5). `DestroyParticle` (`0x005B2240`) returns it to `0x00B0A688`.

Byte 0 of a particle is the alive byte: an update sweep frees every particle whose byte 0 is 0.

### 5.1 Filling a new effect particle (`InitEffectsParticle` `0x005B7580`) *[compat]* (PT5)

Every random draw is `RandomDoubleRangeUniform(lo, hi)` = `lo + u × (hi − lo)` (`0x00913761`, arguments as
doubles) on the particle generator `cRZRandom` at `0x00B22DF8` (seed `0xEC09C261`, foundations.md, the
`cRZRandom` instance table); the result is stored as `f32`. Below, `D` is the description (`+0x24` of the
component), `P` the particle, and *draw(a, b)* one such draw. The draws happen in exactly this order:

1. `P+0x00` (alive) := 1; `P+0x08` (life) := draw(`D+0x0C`, `D+0x10`); `P+0x04` (age) := 0.
2. **Local position** `P+0x0C..+0x14`: draw x, then y, then z, each between the component's emission box min
   `+0x200/+0x204/+0x208` and max `+0x20C/+0x210/+0x214` (§6.1).
3. **Direction** `P+0x1C..+0x24`: draw x (`D+0x30`, `D+0x3C`), y (`D+0x34`, `D+0x40`), z (`D+0x38`, `D+0x44`).
4. **Speed** `s` := draw(`D+0x48`, `D+0x4C`) × the component's `+0x1CC` (set to 1.0 by `SetDescription`). If the
   direction's squared length (x² + y² + z², in that order) is > 0, the direction is scaled by `s / length`; a
   zero direction stays zero.
5. **Radial push** (only if `D+0xF0` > 0): `d` := local position − `D+0xF4`; `q` := dx² + dz² + dy² rounded to
   `f32`. If `q` ≥ 0 and |q| ≥ 1e-4 (`0x00AA0D00`): velocity += `d × (D+0xF0 / √q)`.
6. **To world**: position := `R·p × k + t`, velocity := `R·v + w`, where `R` is the 3×3 matrix at the component's
   `+0x90..+0xB0` (row-major: x' = `+0x90`·x + `+0x94`·y + `+0x98`·z, y' from `+0x9C..+0xA4`, z' from
   `+0xA8..+0xB0`), `k` the scale `+0xC0`, `t` the translation `+0xB4..+0xBC` and `w` the component velocity
   `+0xFC..+0x104` (zeroed by `SetDescription`). The position is rotated, then scaled, then translated. Each row
   is summed as (z term + y term) + x term on the x87 stack (`0x005B77DD`–`0x005B7920`), the stored results rounded
   to `f32`.
7. **Height-based placement**: if flags A has any of `0x7C000` (the global mask `0x00B22E18`, set by the static
   initialiser `0x00A73060`: `pinToTerrain`, `pinToWater`, height window, `seaOnly`, `lakeOnly`),
   `DoHeightBasedEmit` (`0x005B71E0`, given the particle) runs; if it returns false the fill stops here (no draw
   of step 9 onwards happens, so the generator advances by 8 draws instead of 14 or 15).
8. **Half-length lead** (flags A `0x20`): `f` := 1.0, or, when `D+0xDC` ≠ 0, `√(b² + a²) / D+0xDC` with
   `a` = v·(`+0x7C`,`+0x80`,`+0x84`) and `b` = v·(`+0x70`,`+0x74`,`+0x78`) (two axis vectors held by the
   component). Then for each component: position += ((v × 0.5) × f) × `c` × (1.0 / `s`), where `c` is the first
   element of the curve `D+0xAC`. *[quirk]* `s` is the step-4 speed, not the final velocity length; a zero speed
   divides by zero.
9. `P+0x18` := 0. Then five variance factors, each draw(1 − `V`, 1 + `V`) for the variance amount `V`, in this order:
   `P+0x28` (size) := draw(`D+0x68`) × component `+0x1E4` × component `+0x1C8`; `P+0x2C` := draw(`D+0x6C`);
   `P+0x34` := draw(`D+0x78`); `P+0x38` := draw(`D+0x7C`); `P+0x3C` := draw(`D+0x80`); `P+0x40` :=
   draw(`D+0x84`).
10. **Rotation** `P+0x30`: if `D+0x70` ≤ 0 it is 1.0 (no draw); else draw(−`D+0x70`, `D+0x70`).
11. Size adjustments: if flags B bit 0 is clear, `P+0x28` ×= the `f32` at `+0x04` of the object at the
    component's `+0x1E8` (the object whose `+0x08..+0x10` is the wind, §7 step 3); then if flags A `0x400000`
    (`scaleParticles`), `P+0x28` ×= the scale `+0xC0`.
12. If the alignment byte `D+0xD5` is 1: `P+0x30` += atan2(−`+0xA8`, `+0x90`) × 1/(2π) (`0x00B0A660` =
    0.15915494): the emitter's heading in turns.

Example: a description with life (3, 5), direction box (0,1,0)–(0,1,0), speed (2, 2), no radial push, an identity
transform at the origin and no flags gives a particle with age 0, life in [3, 5], velocity (0, 2, 0), size factor
draw(1 ± `D+0x68`) × `+0x1E4` × `+0x1C8`, and rotation 1.0 when `D+0x70` = 0.

## 6. Starting a particle component (`Start(mode)`, slot 0, `0x005B3000`) *[compat]*

1. **Restart filter.** The start proceeds only if `mode` is 0, or the description's emit loop count is 0, or flags A
   has `0x8` (`sustain`). So a finite, non-sustained emitter ignores `Start(1)` — the start that a zoom change
   gives a component re-created inside a running effect (effects.md §5.2). Then the base `Start` must succeed (§4).
2. Emission counters reset: the effect fields `+0x110` (accumulated time, a double), `+0x120`, `+0x128`, `+0x138`
   := 0.
3. The force accumulator `+0x13C..+0x144` := the description's constant force `+0xE0`.
4. **Random walk** (flags A `0x800000`). Clear: the walk vector `+0x150..+0x158` := 0. Set: each component drawn by
   `RandomRange` (`0x00913761`) between the box `+0x30..+0x44` (x from `+0x30`/`+0x3C`, y from `+0x34`/`+0x40`, z
   from `+0x38`/`+0x44`), then y := 0. Without `turn` (`0x1000000`): the walk delay `+0x148` := 0 and one
   random-walk step runs at once (`0x005B2730`). With `turn`: the vector is rescaled to a length drawn from the
   `strength` range `+0x17C`/`+0x180` (left unchanged if its length is 0), and the delay `+0x148` := a draw from the
   `delay` range `+0x174`/`+0x178`.
   The draws use the same generator as §5.1 (`0x00B22DF8`), in the order walk x, y, z, then (with `turn`) the
   strength, then the delay.
5. **`preferSea`** (`0x2000000`; without it go to step 6 and the preferred-direction state of §6.1 is kept as
   is). `+0x15C` := 1. The city (`0x00B43CEC`) converts the component position x `+0x194`, z `+0x19C` to a cell
   (`PositionToCell`, slot `0x17C`); its seaport developer (`GetSeaportDeveloper`, slot `0x130`) answers
   `GetCellSeaAccess(cellX, cellZ)` (slot `0x40`, port-developers.md). The first set bit of the answer, tested in
   the order `0x1`, `0x2`, `0x8`, `0x4`, picks the preferred direction `+0x160..+0x168`: `0x1` → (0, 0, −1),
   `0x2` → (−1, 0, 0), `0x8` → (0, 0, 1), `0x4` → (1, 0, 0); no bit → `+0x15C` := 0 and the direction is left
   unchanged. If `+0x15C` is then set, the direction is taken into the component's frame and divided by its
   scale: v' = `M`·v × (1.0 / `+0xF8`), with `M` the 3×3 at `+0xC8..+0xE8` stored by columns (x' = `+0xC8`·x +
   `+0xD4`·y + `+0xE0`·z, y' from `+0xCC/+0xD8/+0xE4`, z' from `+0xD0/+0xDC/+0xE8`).
6. **Loop-interval countdown** `+0x190`: if `D+0x20` ≥ 0 (compared as a double with 0.0), draw(`D+0x20`,
   `D+0x24`); else if `D+0x28` ≥ 0, draw(`D+0x28`, `D+0x2C`); else 0. (−1, the default, therefore means "no
   countdown".) This is the last random draw of `Start`.
7. `+0x188` := `mode`; `+0x18C` := 0; the pending-preroll byte `+0x118` := (`mode` ≠ 0). So the preroll of §7
   step 1 is added on the first frame only after `Start(1)` (a restart inside a running effect, effects.md §5.2),
   never after a first `Start(0)`. *[quirk, from `0x005B3403`–`0x005B3417`]*

Example (PT6): a description with loop count 1, interval (−1, −1), second range (2, 4), no random walk: `Start(0)`
resets the counters, sets the force to the constant force, clears the walk vector, makes one draw in [2, 4] for
`+0x190`, and leaves `+0x118` = 0.

### 6.1 What `SetDescription` (`0x005B7C90`) prepares for `Start` and §5.1 *[compat]*

- The emission box `+0x200..+0x214` := the description's `+0x50` box (min x, y, z then max x, y, z); the
  component velocity `+0xFC..+0x104` := 0; `+0x1CC`, `+0x1D0`, `+0x1E0`, `+0x1E4` := 1.0; `+0x194..+0x1B4` := 0.
- With flags A `0x1000`: unless `pinToTerrain`/`pinToWater` (`0x4000`/`0x8000`), the box y range is raised by the
  value of the object at `+0x1F0` (its slot `0x74`); then, without `0x2000`, the box's x range becomes 0..`+0x1F4`
  and its z range 0..`+0x1F8` (min x and min z := 0, max x := `+0x1F4`, max z := `+0x1F8`; a square source); with `0x2000`, the city-side volume
  `0x005B7030` fills it.
- **Preferred direction**: `+0x15C` := 1 when any component of `preferDir` `D+0x18C..+0x194` differs from the
  global `0x00B22D70` by at least 1e-4, and then `+0x160..+0x168` := `preferDir`; else `+0x15C` := 0.
- With `collide` (flags A `0x100`): the names `<D+0x160>_terrain` and `<D+0x160>_water` are looked up through the
  global `0x00B43D1C` (its slot `0x14`) and the results kept in the bytes `+0x218`, `+0x219`.

## 7. The frame update (main slot 9, `0x005B8540`; called with `dt` seconds and the draw context) *[compat]*

Nothing happens unless `started`. In order:

1. If the pending-preroll byte `+0x118` is set, `dt` += the description's `preroll` and the byte is cleared. The
   effect stores `dt` at `+0x108` and adds it to the accumulated time `+0x110` (double).
2. A value from the object at `+0x1F0` (its slot `0x74`) is stored at `+0x1FC`. If flags A `0x2000`, the
   city-side emission volume is recomputed. The attractor offset `+0x16C..+0x174` := 0 when the attractor curve is
   empty or its strength ≤ 0; else `UpdateAttractor` (`0x005B6C40`) computes it.
3. The force accumulator := constant force + walk vector + attractor offset + wind gain × the wind vector (three
   `f32` at `+0x08..+0x10` of the object at `+0x1E8`), component-wise in that order of addition.
4. If flags A bit 31 is set, the pre-pass `0x005B8410` runs.
5. **Sweep.** For every node in the live list: a particle whose alive byte is 0, or for which the per-particle
   update (main slot 20, `0x005B9BA0`, given `dt`) returns false, is unlinked, its node freed, the live count
   decremented and the particle handed to `DestroyParticle`.
6. **Emit**, unless the byte `+0x21A` is set: main slot 15 (for `cSC4ParticlesEffect` `0x005B96E0`, §8).
7. **Draw**: with no draw context or no live particles, main slot 19; otherwise slot 14 with the context, the
   self-lit shade if flags A `0x1`, the model transform and the render state `+0x1C`, then slot 17 (no model at
   `+0x1EC`) or slot 18 (a model), then the shade reset if self-lit.
8. `CheckForStop` (§4). If it returned false and flags A `0x800000` is set, one random-walk step (`0x005B2730`)
   with this frame's `dt`.

### 7.1 The per-particle update (main slot 20, `0x005B9BA0`; given the particle `P` and `dt`) *[compat]*

Particle fields used here: `P+0x00` alive byte, `P+0x04` age, `P+0x08` life (§5.1), `P+0x0C..+0x14` position,
`P+0x1C..+0x24` velocity (all `f32`). `F` is the component's force accumulator `+0x13C..+0x144` (§7 step 3), `D`
the description. The result is the alive byte after the update (false → swept, §7 step 5).

**Path choice.** If flags A has any bit of `0x18000D00` (the global mask `0x00B22E14`, set by the static
initialiser `0x00A73050`: `collide` `0x100`, `terrainRepel` `0x400`, `killOutsideCity` `0x800`, the attractor
point `0x8000000`, tractor `0x10000000`), the **stepped path** (`0x005B9790`) runs instead; otherwise the
**simple path**:

1. `age` += `dt` (stored `f32`). If not `age` < `life` (so also on equality), the alive byte := 0, result false.
2. `v` := `v_old` + `dt` × `F`, per component, stored.
3. Drag: `k` := 1.0 − `dt` × `D+0x100` (the **drag** coefficient); `v` := `v` × `k`, stored.
4. Position: `p` += (`v` + `v_old`) × (`dt` × 0.5) (trapezoid rule; the 0.5 is the double `0x00A92D28`; the
   sum `v + v_old` is rounded to `f32` before the multiply, `dt × 0.5` stays on the x87 stack). Result true.

**Stepped path** (`0x005B9790`):

1. `age` += `dt`; not `age` < `life` → alive := 0, result false (as above, before any stepping).
2. Loop with `r` := `dt` (the remaining time): the step `h` := 1/15 (the `f32` 0.06666667 at `0x00AA0CF4`) when
   1/15 < `r`, else `r`.
   1. Position: if flags A `0x10000000` (tractor) is clear, `p` += `h` × `v` (explicit Euler: the *old*
      velocity); otherwise `ApplyTractorForce` (`0x005B6600`, given `P` and `h`) moves the particle instead.
   2. Velocity: `v` += `h` × `F`.
   3. Attractor point (flags A `0x8000000`): `d` := `A` − `p` with `A` the component's attractor point
      `+0x178..+0x180` (written by `UpdateAttractor`, §7 step 2). If |`d`|² > 0: `L` := √|`d`|², `u` := `d` ×
      (1.0 / `L`), `t` := `L` / `D+0x1B0` capped at 1.0 (so `D+0x1B0` is the attractor's **range**), `s` := the
      attractor curve `D+0x1A4` sampled at `t` (below), and `v` += `u` × `s`. The pull is added once per step,
      not scaled by `h`.
   4. Drag: `k` := 1.0 − **`r`** × `D+0x100`; `v` := `v` × `k`. Quirk: the factor uses the remaining time `r`, not
      the step `h`, so a frame longer than 1/15 s damps several times with decreasing factors.
   5. Bounds: `q` := the position in the component's `+0xC8` frame (§6 step 5): `q.x` := (`p.z`·`+0xD0` +
      `p.y`·`+0xCC`) + `p.x`·`+0xC8`, `q.y` := (`p.x`·`+0xD4` + `p.z`·`+0xDC`) + `p.y`·`+0xD8`, `q.z` :=
      (`p.z`·`+0xE8` + `p.y`·`+0xE4`) + `p.x`·`+0xE0`; each sum is rounded to `f32`, multiplied by the scale
      `+0xF8` (rounded), then `+0xEC`, `+0xF0`, `+0xF4` are added. The particle is **inside** when `q.x` > 0,
      `q.z` > 0, `q.x` < `+0x1F4` and `q.z` < `+0x1F8` (the city's extent in that frame, `f32`; any NaN is
      outside).
      - Outside: with `killOutsideCity` (`0x800`) alive := 0 and the result is false at once.
      - Inside, with `collide` (`0x100`) or `terrainRepel` (`0x400`): `ApplyTerrainForces` (`0x005B8DC0`, given
        `P`, `q` and `h`); if it returns false, alive := 0, result false.
   6. `r` −= `h` (stored `f32`). If `r` > 0 (compared with the double 0.0 at `0x00A80990`), go to 2.
3. If flags C (`D+0x04`) bit 0 (`destroyBuildings`) is set, `TestBuildingCollision` (`0x005B8020`, given `P`)
   runs once. The result is the alive byte as it is now (the collision test may clear it).

**Curve sampling** (here and wherever a description `f32[]` is read at a fraction `t` in [0, 1]): with `n`
samples, `n` = 1 → sample 0. Otherwise `x` := `t` × (`n` − 1), `i` := ⌊`x`⌋; if `x` − `i` ≤ 0 the value is sample
`i`, else sample `i` + (sample `i`+1 − sample `i`) × (`x` − `i`). (The floor is done with the 12582912 rounding
trick at `0x005B995E`; for `x` ≥ 0 it equals the mathematical floor.)

Worked example (simple path): `dt` = 0.1, `v_old` = (0, 10, 0), `F` = (0, −9.8, 0), drag 0.5 → `v` after step 2
= (0, 9.02, 0); `k` = 0.95, `v` = (0, 8.569, 0); `p.y` += (8.569 + 10) × 0.05 = 0.92845.

## 8. Emission (correction to effects.md §7.1)

Proof from the vtable images (each slot 15 is at vtable `+0x3C`): the data word `0x00AA0EDC` (= `0x00AA0EA0` +
`0x3C`) and `0x00AA0C94` (= `0x00AA0C58` + `0x3C`) hold `0x005B96E0`; the word `0x00AA0BC4` (= `0x00AA0B88` +
`0x3C`) holds `0x005AA5D0`, and those are the only references to either function. So the three-way emitter
`0x005B96E0` is slot 15 of the main vtable of **both** `cSC4ParticlesEffect` (`0x00AA0EA0`, installed by the
base constructor `0x005B2B50`) and `cSC4ModelBasedParticlesEffect` (`0x00AA0C58`, inherited), and `0x005AA5D0`
is slot 15 of `0x00AA0B88`, the vtable `0x005A8ED0` installs: **`cSC4GeometrySourcedParticlesEffect`**. The `+4`
simulator vtable `0x00AA0E58` is not involved: its slot 15 is `CreateParticle` `0x005B7C20` (§9.2). The
self-stop protocol of effects.md §7.1 (budget exhausted → `Stop(0)`, delayed) is unchanged; `0x005AA5D0` also
self-stops, through main slot 1 with argument 0 at `0x005AA6D4` when its `+0x18` count is 0.

### 8.1 The emitter dispatch (`0x005B96E0`, main slot 15) *[compat]* (PT7)

1. If the byte `+0x10` is set, nothing happens.
2. `e` := this frame's `dt` (`+0x108`, §7 step 1). If the countdown `+0x190` > 0: `+0x190` −= `dt` (stored as
   `f32`); if it is still > 0, nothing is emitted this frame; otherwise `e` := −`+0x190` (the time past expiry).
3. If `preroll` `D+0x1C` > 0 and `e` > `preroll` − 0.01 (the double `0x00A92688`), `e` := `preroll` − 0.01.
4. The emitter is chosen by **flags A** (`D+0x00`): `0x4` (`maintain`) → `0x005B3CF0` (rate integral, no
   self-stop); else `0x2` (`inject`) → `0x005B8B50` (loop budget); else `0x005B87B0` (duration budget); each is
   given `e`.

### 8.2 The three emitters (given the emitted time `e`, §8.1) *[compat]*

State: the emission clock `+0x120` (double, in **loops**: one loop = `D+0x14` seconds), the completed-loop
counter `+0x128` (`u32`, read as unsigned), the clock rate `+0x130` (double) := 1.0 / `D+0x14` set by
`SetDescription` (`D+0x14` is used only when 1e-6 < `D+0x14`, else 1e-6 (`0x00A86510`); so `D+0x14` is the
**loop duration** in seconds, default 1.0), the fractional emission carry `+0x138` (`f32`). `Start` zeroes
`+0x120`, `+0x128`, `+0x138` (§6 step 2). The **density** `s0` := (`E+0x00` × `+0x1E0`) × `+0x1C4` (`f32`), where
`E` is the environment object at `+0x1E8` (set by main slot 13 `0x005B2220`; its `+0x04` is the size factor of
§5.1 step 11 and `+0x08..+0x10` the wind), `+0x1E0` is `SetParameter` (slot 5, `0x005B2490`) index 2 (1.0 after
`SetDescription`), and `+0x1C4` is the first value of `SetParameter` index `0x102`. `k` is the scale `+0xC0`.

**Emitting one particle** (all three): `P` := `CreateParticle` (`+4` slot 15). A null `P` is skipped. If `P`'s
alive byte is 0 → `DestroyParticle` (`+4` slot 16). Otherwise (`0x005B8B50`, `0x005B87B0` only) the per-particle
update (main slot 20, §7.1) runs on `P` with an age offset `a` (pre-ageing: the particle is simulated forward by
`a` seconds at birth); false → `DestroyParticle`; true → if flags A bit 31 is set, the birth events of §8.3 fire,
then `AddParticle` (`+4` slot 11). `0x005B3CF0` adds without the update and without the bit-31 events.

**`maintain` (`0x005B3CF0`).** `t` := `+0x120` + `e` × `+0x130` (double, stored). Only when `t` > `+0x128`
(strict; `+0x128` stays 0 here, so every frame with a positive clock): `n` := ⌈`rate[0]`⌉ (the first sample of the
rate curve `D+0x88`, rounded up by the 12582912 trick at `0x005B3D33`) minus the live count (§4); that many
particles are emitted. No density, no scale, no loop counting, no self-stop. *Quirk:* the difference is
unsigned; a live count above `n` (unreachable with a fixed description) would wrap.

**`inject` (`0x005B8B50`, bursts).** `t` := `+0x120` + `e` × `+0x130` (double, stored). If not `t` > `c` (`c` =
`+0x128`) nothing happens. `n0` := `rate[0]`; if `n0` > 20.0 (`0x00AA0F1C`) and flags B bit 0 is clear, `n0` :=
((`E+0x00` × `+0x1E0`) × `+0x1C4`) × `n0`; `n` := ⌈`n0`⌉. Then loop:
1. `a` := (`t` − `c`) × `D+0x14` (`f32`: seconds since the burst was due). If `a` < `D+0x10` (the life range's
   upper end) and `n` ≠ 0, `n` particles are emitted, each pre-aged by the same `a`.
2. `c` += 1 (stored). If the loop count `D+0x18` ≠ 0 and `c` ≥ it: main slot 1 `Stop(0)` (the delayed stop of
   effects.md §7.1) and return.
3. If `D+0x28` ≥ 0: `t` −= RandomRange(`D+0x28`, `D+0x2C`) × `+0x130` (the pause between bursts, in seconds,
   converted to loops; stored). If now `t` < `c`: the countdown `+0x190` := (`c` − `t`) × `D+0x14` (`f32`),
   `+0x120` := `c`, return (§8.1 then waits out the pause).
4. Repeat from 1 while `c` < `t` (double).

**Default (`0x005B87B0`, a rate over a duration).** `s` := `s0`, times `k` with `emit … scale` (`0x80000`),
else `k`² with `areaScale` (`0x100000`), else `k`³ with `volumeScale` (`0x200000`) (first bit that is set wins;
products `k·k·s0`, `(k·k)·k·s0`). A flag `w` := false. Loop, with `r` := `e`:
1. Step `h` := 1/15 if 1/15 < `r`, else `r` (as §7.1). `c1` := `+0x128` + 1; `t` := `+0x120` + `h` × `+0x130`
   (double, stored).
2. **Loop end** when `t` > `c1` (strict): `+0x128` := `c1`. If `D+0x18` ≠ 0 and `c1` = `D+0x18`: without `sustain`
   (`0x8`) → main slot 1 `Stop(0)`, return; with `sustain` → `+0x128` := `D+0x18` − 1 and `+0x120` := `D+0x18` −
   1e-6 (the clock is pinned at the end of the last loop, so the last rate value repeats for ever). Otherwise, if
   `D+0x28` ≥ 0: `w` := true, `h` −= (`t` − `c1`) × `D+0x14` (the step is cut at the loop end) and `+0x120` :=
   `c1`. With `D+0x28` < 0 the clock simply runs on into the next loop.
3. **Rate**: the rate curve `D+0x88` sampled (§7.1 sampling) at the loop phase `+0x120` − `+0x128` (in [0, 1)).
   `acc` := `rate` × `h` × `s` + `+0x138`; `m` := ⌊`acc`⌋; `+0x138` := `acc` − `m`. So `rate` is particles per
   second.
4. If `m` > 0: `m` particles are emitted with age offsets `a` = `r`, `r` − `h`/`m`, `r` − 2`h`/`m`, … (`r` is the
   remaining frame time at the start of this step, not `h`; `h`/`m` is computed once as `f32`).
5. `r` −= `h`. If `w`: `r` −= RandomRange(`D+0x28`, `D+0x2C`) (the pause between loops); if `r` < 0 the
   countdown `+0x190` := −`r` and return.
6. Repeat from 1 while `r` > 0.

Worked example (default emitter): `D+0x14` = 2 s, rate curve `[10]`, density 1, no scale bits, one 0.1 s frame
from a fresh `Start`: one step `h` = 0.1, `t` = 0.05 (no loop end), `acc` = 10 × 0.1 = 1.0 → `m` = 1, one particle
pre-aged by 0.1 s, carry 0.

### 8.3 Particle age events (`D+0x1CC`, flags A bit 31) *[compat]* (PT2)

`D+0x1CC` (reader B only) is a list of 16-byte elements: a name and, at element `+0x0C`, an age `T` (`f32`).
- **At birth** (`0x005B84C0`, from the inject and default emitters when flags A bit 31 is set, §8.2): if the
  particle is alive, the elements are visited in order while their `T` is exactly 0.0; each fires (below). The
  first element with `T` ≠ 0 ends the visit, so zero-age events must come first in the list.
- **Each frame** (the pre-pass `0x005B8410`, §7 step 4, before the sweep): for every live particle that is alive
  and every element: the element fires when `P.age` − `T` ≤ 0 and `P.age` − `T` + `dt` > 0 (this frame's `dt`,
  `+0x108`), i.e. when the particle's age is about to cross `T` in this frame's update.
- **Firing** (`0x005B8290`, given the element's name and the particle position `P+0x0C`): the name is combined with
  the description's own name `D+0x160`, an effect is looked up by it and, when found, started at the particle's
  position with an identity orientation. *[inferred from damaged pseudocode; the composition of the name is not
  byte-verified]*

## 9. Slot tables *[compat]*

### 9.1 Main vtable `0x00AA0EA0` (`cSC4ParticlesEffect`); slots 0–8 are the component interface of effects.md §11.2

| Slot | Function | Role |
|---|---|---|
| 0 | `0x005B3000` | `Start(mode)` §6 |
| 1 | `0x005B21E0` | `Stop(mode)`: stores the mode at `+0x18C`, calls base `Stop(mode == 0, 0)` (effects.md §7.1) |
| 2 | `0x005A8FA0` | `IsActive`: the base `started` byte |
| 3 | `0x005B3430` | `SetTransform` |
| 4 | `0x00648F10` | `Update(dt)`: a shared 3-byte no-op; the work is slot 9 |
| 5, 6 | `0x005B2490`, `0x005B2200` | `SetParameter` channels |
| 7, 8 | `0x005A8F80`, `0x005A8F90` | AddRef, Release (forward to `+4`) |
| 9 | `0x005B8540` | frame update §7 |
| 10 | `0x005B7C90` | `SetDescription` |
| 11–14 | `0x005B3820`, `0x005B3870`, `0x005B2220`, `0x005B3B90` | not traced (14 is the pre-draw step) |
| 15 | `0x005B96E0` | emit (§8) |
| 16 | `0x005B7170` | not traced |
| 17 | `0x005B3DA0` | draw particles through a vertex buffer |
| 18 | `0x005B4B90` | draw particles as models |
| 19 | `0x005B6390` | the no-draw step |
| 20 | `0x005B9BA0` | per-particle update; false = the particle dies |

### 9.2 `+4` vtable `0x00AA0E58` (the simulator interface)

Slots 0–3 QueryInterface `0x0048C940`, AddRef `0x005A8FB0`, Release `0x005A8FC0`, deleting destructor `0x005B2FD0`;
4 `Init` `0x005B2390`; 5 `Shutdown` `0x005B2440`; 6 `Start` `0x005B9CF0`; 7 `IsStarted` `0x005A8FE0`; 8 `Stop`
`0x005B9E70`; 9 `IsInDelayedStop` `0x005B9D30`; 10 `FinalRelease` `0x005B9DD0`; 11 `AddParticle` `0x005B9F80`;
12 `RemoveParticle` `0x005B9D80`; 13, 14 shared one-line getters (`0x0040D3A0`, `0x004777E0`); 15 `CreateParticle`
`0x005B7C20`; 16 `DestroyParticle` `0x005B2240`. The geometry-sourced class differs in 3, 4, 5 and 16
(`0x005A8FD0`, `0x005AC030`, `0x005A6310`, `0x005A6200`).

## 10. Gaps (not specified here)

The helpers the physics of §7.1 calls (terrain forces `0x005B8DC0`, building collision `0x005B8020`, tractor
`0x005B6600`), height-based emission `0x005B71E0`, `UpdateAttractor` `0x005B6C40`, the parts of `SetDescription`
`0x005B7C90` not listed in §6.1 and §8.2, and drawing (`0x005B3DA0`, `0x005B4B90`). These are visual only:
no simulation state depends on them except through the finish test, which depends only on §4 and effects.md §7.1.

## 11. Quirks

1. `Stop(0)` of the component is the *delayed* stop and `Stop(1)` the immediate one (effects.md §7.1).
2. The three `0x00B22E20` stores in particle creation are dead (§5).
3. `RemoveParticle` does not unlink; the node is swept on the next update.
4. A finite emitter cannot be restarted by `Start(1)` (§6 step 1).

## 12. Open questions

See [questions/particles.md](questions/particles.md): PT1, PT2 (narrowed), PT3 open; PT4 (§5), PT5, PT6, PT7 answered 2026-10-21 (§5.1, §6, §8.1).
