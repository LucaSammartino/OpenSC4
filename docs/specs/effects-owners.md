# Effects owners: model-based particles, dynamic particles, camera effect, effect tool (behaviour spec)

Scope: the bodies that effects.md, particles.md and effects-parser.md leave unspecified for these owners, as
tools/coverage.py assigns them: `cSC4ModelBasedParticlesEffect`, `cSC4DynamicParticleEffect`,
`cSCParticleSimulatorBase` (its misattributed leftovers), `cSC4CameraDescription` (which also holds most of the
camera effect's methods), `cSC4CameraEffect`, `cSC4CameraParams`, `cSC4EffectsModelProcessor`,
`cSC4EffectsResourceFactory`, `cSC4DynamicParticleDescription` and the effect placement tool
`cSC4ViewInputControlEffect`. It also records that `cSC4DecalDescription`, `cLightEffectCommand`,
`cSC4EffectDescription::cDescriptionRec`, the decal, shake and group parser commands and the three parser helpers
`ParseRateCurveCommands`, `ParseRangedVector3` and `ParseDrawOptions` are already specified (§9).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the renderer, the fake perspective, the
deformer and the camera helpers also in the disassembly. Constants were read in the PE. Written 2026-10-25
(re-effects8).

Notation. "Sample curve c at f" is the inline keyframe lookup these bodies share: with n keys, n = 1 → key 0;
otherwise x := f·(n − 1), i := floor(x) (computed with the 1.5·2^23 rounding trick and a correction when
float(i + 1) = x), r := x − i; r > 0 → key i + r·(key i+1 − key i), else key i. There is no clamp: f > 1 reads past
the last key *[quirk]*. Vector curves lerp per component.

## 1. Model-based particles (`cSC4ModelBasedParticlesEffect`) *[compat]*

On top of the ordinary particle component (particles.md §1–§13; effects.md §7). Fields used here: `+0x14` the particle
list, `+0x24` the particle description D, `+0x108` animation rate, `+0x110` deform time (double), `+0x188`/`+0x18C`
the two "camera collision" parameters, `+0x1A0`–`+0x1B4` a bounding box (min, max), `+0x1D0` alpha factor, `+0x1E8`
the camera tuning record C, `+0x21C`–`+0x224` model references (`+0x220` the model, `+0x224` the animated model),
`+0x228` the effects model processor (§6), `+0x268` a description reference, `+0x26C` the deform/light byte
(effects.md EF7), `+0x274` box scale, `+0x278` the chained system.

### 1.1 Small members

- `0x005AC110(d)`: the reference at `+0x40` := d (new one retained through `0x005AF940`, old one released through
  `0x005B1C80`), nothing when equal.
- `Shutdown` (`0x005AC140`): when initialized, clear and release `+0x21C`, then `+0x220` (through its virtual base),
  then the ordinary Shutdown (`0x005B2440`).
- `Stop(mode)` (`0x005AC190`): when a chained system (`+0x278`) exists and `+0x18` is 0, its slot 1 `Stop(mode)`; then
  the ordinary Stop (`0x005B21E0`).
- Destructor body `0x005AC5B0`: the chained system's slot 8; release `+0x268`; destroy the processor `+0x228`; release
  `+0x224` (virtual base) and `+0x220`; then the ordinary particle destructor (`0x005A62B0`). The deleting destructor
  `0x005AC590` frees on flag bit 0; `0x005AC580` is its thunk for the `+4` view.

### 1.2 `ApplyFakePerspective(bias, camera, P, &pos, &scale)` (`0x005AD890`, pop 20)

With the camera's eye E (`+0x3C`–`+0x44`), forward F (`+0x54`–`+0x5C`), right R (`+0x60`–`+0x68`) and up U
(`+0x6C`–`+0x74`):

1. depth := F·pos + bias (x87); scale := float32(C.`+0x30` / depth · scale).
2. u := (pos − E)·R and v := (pos − E)·U (each sum stored float32).
3. t := (scale − C.`+0x20`) / C.`+0x24`. When t > 0: t := min(t, 1); pos += ((scale − 1)·t·u)·R, then
   pos += ((float32(scale) − 1)·t·v)·U (each component stored float32): particles larger than the threshold spread away
   from the view centre.
4. When depth ≥ |scale| (or unordered): return true. Otherwise the particle is closer than its own size: with
   w := F·P.velocity (`P+0x1C`–`+0x24`), when depth > 0, w < 0, |u| < C.`+0x28` and |v| < C.`+0x28` it bounces off the
   camera: P.velocity −= 2w·F, then += C.`+0x2C`·u·R and += C.`+0x2C`·v·U; the effects manager (`0x00B43D1C`) slot 7
   looks up "camera_collision"; when found, that effect gets slot 8 (a translation-only transform at pos) and slot 3
   (`+0x188`), and is released; return true. In every other close case the particle's attached effect (`P+0x44`),
   when its slot 5 is true, gets slot 4 (`+0x18C`) and is released and cleared; return **false** (the particle is not
   drawn).

### 1.3 `RenderActiveParticles(context)` (`0x005ADD50`, pop 4)

1. The box := min (1, 1, 1), max (0, 0, 0) (empty). The material record starts from the globals `0x00B0A5AC`–`0x00B0A5B4`
   with alpha 1.0 and flags 0; with a model, its slot 15 fills it; D's flag byte bit 0 adds material flag 4.
2. bias := (E.y / F.y − F·E) + C.`+0x30` (float32; the E.y / F.y term is literal).
3. For every particle P of the list:
   - pos := P's position (`+0x0C`–`+0x14`). When D's spin (`+0x104`) is non-zero: θ := (pos.y − O.y)·spin with O the
     emitter origin (`+0xB4`–`+0xBC`); with d := (pos.x − O.x, pos.z − O.z): pos.x += d.x·cos θ − d.z·sin θ,
     pos.z += d.z·cos θ + d.x·sin θ (the rotated offset is **added to the position**, not to the origin *[quirk]*).
   - f := P.age (`+4`) / P.life (`+8`), float32. size := sample D's size curve (`+0xAC`) at f × P.`+0x28` × 0.02.
   - D flag bit 1: `ApplyFakePerspective(bias, camera, P, &pos, &size)`; false → next particle.
   - The shared builder `0x005ACA30(P, f, size, &pos, &T)` fills the particle transform T (not owned here).
   - Colour := sample D's colour curve (`+0x94`, 12-byte keys) at f (one key → that key), times P.`+0x38`–`+0x40`;
     alpha := sample D's alpha curve (`+0xA0`) at f × `+0x1D0` × P.`+0x34`.
   - With a model (`+0x220`): its slot 10 gets T; material flag 2 is cleared when alpha = 1 and the alpha curve has at
     most one key, set otherwise. With an animated model (`+0x224`) of more than one frame (slot 12): the frame clock
     P.`+0x48` advances by (rate `+0x108` × D.`+0x1DC` when that is positive) × |P.velocity| (D flag bit 6), or by
     rate / (frames per second, the word `+2` of the model's slot 17 record) × 1000; D flag bit 5 holds the last frame
     once the clock reaches the frame count N (slot 10 `SetFrame(N − 1)`), otherwise the clock wraps by N and the frame
     is floor(clock). Without animation, when `+0x26C` is set: D flag bit 2 lights the processor
     (`LightModel(0x00B43DDC, 0)`) and D flag bit 8 deforms it (§6, with float32 `+0x110`). Then the material gets the
     colour and alpha (slot 16) and the model draws (slot 12 `(context, 0)`).
   - The particle's attached effect (`P+0x44`), when its slot 5 is true: D flag bit 4 → slot 12 `(3, &alpha, 1)`; D
     flag bit 3 → slot 7 with a translation-only transform from T, else slot 8 with T.
4. `context.ResetShade()`. When the box was extended (min.x ≤ max.x), it grows by r := size key 0 × `+0x274` × 0.02
   on every side.

## 2. Dynamic particles (`cSC4DynamicParticleEffect`) *[compat]*

The shared `cSC4DynamicParticleSystem` (particles.md §16) is the global `0x00B22C30` with a user count at `+0x30`.

- `InitDynamicParticleSystem` (`0x00585110`): creates it on first use (`0x60` bytes, AddRef; an allocation failure
  dereferences null); count += 1; on the first user it is registered with the renderer (`0x00B43DD0` slot 32
  `(system, 3, 1000)`) and its `+0x0C` target with the animation tick manager (slot 5, tick `0xC0`). Returns it.
- `ShutdownDynamicParticleSystem` (`0x00583860`): count −= 1; at 0: tick unregistration (slot 6), renderer slot 33,
  Release, global := null.
- `SetVoidParams(id, data)` (`0x00584170`): 7 → own slot 12 `(data[0])`; `0x10` → particle `+0x14` := data[0];
  `0x11` → `LinkParticles(system, data[0], particle +0x14)`; `0x12` → `AttachSlaveEffect(data[0], data[1])`; true for
  these, false otherwise.
- `SetDescription(d)` (`0x005843B0`): retain d (cRZRefCount AddRef, slot 2) and release the old one (slot 1). When d's
  model id (`+0x30`) is neither 0 nor −1, or its id list (`+0x34`–`+0x38`) is not empty, own slot 11 gets a uniformly
  random list element (`cRZRandom::RandomUint32Uniform`) or else the model id. `+0x9D` := whether the effects manager
  (slot 5) knows the effect named "<d's name>_collide".
- `Stop(mode)` (`0x005854B0`): when started (`+0x9C`): clear it; the slave effect `+0x98` gets slot 4 `(mode)` and is
  released and cleared; the particle `+0x90` is deleted from the system and cleared;
  `ShutdownDynamicParticleSystem`.

## 3. Camera effect and camera description *[compat]*

### 3.1 The description (`cSC4CameraDescription`, vtables `0x00A9EE0C`/`0x00A9EDF8`)

Constructor `0x00581C10`: reference base at `+4`, flags `+0x0C` := 0, zoom byte `+0x10` and rotation byte `+0x11` := 0,
radius `+0x14` := 0. Write (`0x00581C50`, stream slots): version 0 (slot 7), flags (slot 9), the two bytes (slot 5),
radius (slot 12). Read (`0x00581EB0`): the same order, flags masked to 4 bits. The deleting destructor `0x00581960`
(thunk `0x00581950`, ECX −4) frees on flag bit 0.

### 3.2 The camera effect (vtables `0x00A9EE48`/`0x00A9EE38`/`0x00A9EE24`)

Factory `0x00581D60` (`0x24` bytes) → constructor `0x00581CA0(description)`: description `+0x10` (retained, its slot 4),
position `+0x14`–`+0x1C` from the zero vector `0x00B22C00`, bytes started `+0x20`, updating `+0x21`, pending `+0x22`,
subscribed `+0x23` := 0. Deleting destructor `0x00581D10` (thunk `0x00581990`, ECX −8) releases the description
(slot 5). Component type (`0x005818D0`): 8. QueryInterface on the message view (`0x005818E0`): IIDs 1 and `0x452294AA`
answer that view (null when it is the object start's `+4` image of a null object) and AddRef through the primary.

| Member | Behaviour |
|---|---|
| `Start` (`0x00581D80`, owner `cSC4CameraEffect`) | when not started: with the camera control (renderer slot 16): flag bit 0, the setting byte (`0x00B43CC4` → `+0x0C` → `+0x4BC`) set and the current zoom (`+0x108`) below zoom byte − 1 → `SetZoom(zoom byte)`; flag bit 1 under the same setting → `SetRotation(rotation byte)`; flag bit 2 → `SetViewTarget(&position)` now (updating set around the call). Then, unless flag bit 3 is set or the radius is positive: started := 1, pending := subscribed := 0. Otherwise subscribe the message view to `0xA6B79611`, subscribed := 1, started := 1, pending := flag bit 3 |
| `Stop` (`0x005819A0`) | when started: clear it; when subscribed, unsubscribe and clear |
| `IsStarted` (`0x00581940`) | the started byte |
| `SetTransform(local, world)` (`0x005819E0`) | position := world's translation; when local's rotation flag is set: world rotation flag clear → local's translation, else world translation + world scale · (world rotation · local translation) |
| `SetParameter(14, data)` (`0x00581B20`) | only id 14: when the radius is positive, project (data[0..2] − position) onto the plane of normal data[3..5]; when that distance is below the radius, pending := 1; AL true for 14, false otherwise |
| `Update(dt)` (`0x00581BE0`) | when pending: updating := 1, camera control `SetViewTarget(&position)`, updating := 0 |
| `DoMessage` (`0x00581910`) | `0xA6B79611`: when not updating, pending := 0, AL true; other types AL false |

### 3.3 `cSC4CameraParams` (`0x00587F30`)

Constructor of the collection's camera parameters (effects.md §18.6): an empty float vector, then the five keys
2500, 1250, 625, 312, 156 appended; scalars `+0x0C`, `+0x10` := 0, `+0x14` := 4, `+0x18` := 7.

## 4. The effect placement tool (`cSC4ViewInputControlEffect`) *[compat]*

The tool effects.md describes as "the menu tool" (`SetEffectName` `0x004BB590`). Fields: capture byte `+0x28`,
pressing `+0x29`, deferred `+0x2A`, pick point `+0x2C`–`+0x34`, a transform `+0x38` (identity rotation, translation
`+0x60`–`+0x68`, its two flag bytes), the effects manager `+0x70`, the effect name `+0x74` (string), group id `+0x88`,
`+0x8C` has-effects, `+0x8D`/`+0x8E` "button trigger"/"trigger" effects exist, `+0x8F` pick flag, and five effect
slots: `+0x90` (kind 5), `+0x94` (kind 7, terrain-controlled), `+0x98` (kind 4), `+0x9C` (kind 1), `+0xA0` (kind 2).

| Member | Behaviour |
|---|---|
| constructor `0x004BAF60` | zero vector for the pick and transform, identity rotation, everything else null/0 |
| `Init` (`0x004BABE0`) | once: base Init; retain the effects manager (`0x00B43D1C`) into `+0x70`; pressing := 0 |
| `Shutdown` (`0x004BAC30`) | release a held capture; release the manager; each effect `+0x90`, `+0x9C`, `+0xA0`, `+0x98`, `+0x94` is stopped (slot 4 `(0)`), cleared and released; base Shutdown |
| destructors `0x004BB0B0`, `0x004BB140` | Shutdown when initialized; release the five effects, the name, the manager; base destructor; free on flag bit 0 |
| `SetEffectGroupID(g)` (`0x004BB160`) | `+0x88` := g; `+0x8D`/`+0x8E` := whether the manager (slot 5) knows "<name>_button_trigger" / "<name>_trigger"; each effect slot is released and re-made by the manager's slot 6 `(g, kind, &slot)` for kinds 5, 4, 7, 1, 2; `+0x8C` := any trigger name or any made effect; the kind-7 effect, when made, gets slot 3 `(0)` |
| `PlayTriggerEffect` (`0x004BB360`) | without a name: when g ≠ 0, manager slot 6 `(g, 3, &e)`; with a name: manager slot 7 makes "<name>_trigger"; e gets the transform (slot 8) and Start (slot 3 `(0)`) and is released |
| `OnMouseDownL(x, z)` (`0x004BB410`) | without the city view false; the city view's slot 36 picks the point (with `+0x8F`); on a hit: translation := pick, transform flags updated; "<name>_trigger" known → `PlayTriggerEffect`; pressing := 1; kind 1 gets the transform and Start; kind 2 stops; kind 4 gets the transform, stops when running, starts; deferred := (kind 7 exists and is not running); when not deferred, kinds 7 and 5 get parameter 11 `(0)` and parameter 10 `(pick)`; capture := 1, `SetCapture`, cursor (base `+0x10`). True |
| `OnMouseMove(x, z)` (`0x004BAD30`) | picks the point; no hit → kind 2 stops. Hit → translation := pick; while captured and not deferred, kinds 7 and 5 get parameter 10 `(pick)`; kind 1 gets the transform; kind 2 starts when not pressing and not running, then gets the transform. True |
| `OnMouseUpL(x, z, flags)` (`0x004BAE20`) | not captured → false. pressing := 0; kind 1 stops; kind 2 gets the transform and starts; kinds 7 and 5 get parameter 11 `(0)`; with kind 7: deferred → own slot 30 (`SetTerrainControlledEffectState`, vtable `0x00A902E0`) `(true)`, else flags bit 1 → slot 30 `(false)`; release the capture, capture := 0. True |
| `OnMouseExit` (`0x004BAF00`) | kind 2 stops; base OnMouseExit |
| `SetTerrainControlledEffectState(on)` (`0x004BAF20`) | on → kind 7 gets the transform and starts; off → kind 7 stops (no null test) |

"Stops" is slot 4 `(0)`, "starts" slot 3 `(0)`, "is running" slot 5, "transform" slot 8 with `+0x38`, "parameter p
(v)" slot 11 or 12 `(p, &v, 1)` (`+0x2C` for 10 and `+0x30` for 11).

## 5. Dynamic particle description *[compat]*

The deleting destructor `0x00583700` and body `0x00583720` free the id list (`+0x34`) and the name (`+0x0C`), then the
reference base (`0x0090D964`). Constructor and assignment are effects.md (`005836A0`, `00583750`).

## 6. `cSC4EffectsModelProcessor::DeformModel(t)` (`0x005AC200`) *[compat]*

On the effects model processor (a `cS3DModelProcessor` with its particle description at `+0x40`, here M):

1. No vertex normals yet (`+0x24` = `+0x28`) → `CreateVertexNormals`.
2. s := t·M.`+0x114`; du := s·M.`+0x118`, dv := s·M.`+0x11C`; their wrapped offsets du − 8·floor(du/8) and
   dv − 8·floor(dv/8).
3. For every source mesh of the standard vertex format 2 (five floats per vertex), with the destination buffer from
   the model's slot 4 (six words per vertex): copy the position; when M's deformation list (`+0x108`) is not empty,
   `0x005AC070(t, list, &position)` moves it; u, v := the wrapped offsets + the source uv. When M's shading curve
   (`+0x12C`) is not empty: a := sample it at ((N·L + 1)·0.5) with N the vertex normal and L := M.`+0x120`–`+0x128`;
   the colour's alpha byte := k − (k >> 8) with k = round(a·256) (via the 49152.0 trick), so 1.0 gives 255.

## 7. `cSC4EffectsResourceFactory` *[compat]*

- `CreateInstance(type, iid, &out)` (`0x005A3C30`): only type `0xEA5118B0`: GZCOM creates `0x2A5118AD` with IID
  `0xCA51189B` and its QueryInterface answers `(iid, out)`; other types and failures → `*out := null`, false.
- `SerialRead`/`SerialWrite` (`0x005A3EC0`/`0x005A3F40`): only when the resource's type (slot 5) is `0xEA5118B0`: the
  stream is queried for `0x051B11BD` / `0xC51B11CB` and the owner's slot 7 / slot 6 reads or writes through it.
- Destructor `0x005A3C00`: the base `cGZPersistResourceFactory` destructor, free on flag bit 0.

## 8. `cSCParticleSimulatorBase` leftovers *[compat]*

The owner also holds web-network bodies under a folded name (web-network.md, web-network-http-*.md) and a model-maker
destructor (model-maker.md). The rest: the deleting destructors `0x0048DAC0` (body `0x0048CDE0`) and `0x005B9F50`
(restores the base vtable, clears the list at `+0x10`); and Release `0x00725130` of an unrelated folded owner: count
`+8` above 1 → decrement; otherwise restore the vtable, release `+0x3044`, free every block of the vector
`+0x301C`–`+0x3020` and the vector (`0x00724BF0`), free the object, return 0.

## 9. Owners already specified elsewhere

`cSC4DecalDescription` (effects.md, `00582FE0`, `005833F0`), `cLightEffectCommand` (effects.md, `0059E4D0`),
`cSC4EffectDescription::cDescriptionRec` (effects.md and effects-parser.md, `005997B0`), `ParseRateCurveCommands`
(effects.md, `00597DB0`), `ParseRangedVector3` (effects.md, `0059BF10`), `ParseDrawOptions` (effects.md, `005A1220`;
its enum table `0x00B0A408`: decal 0, decalInvertDepth 1, decalIgnoreDepth 2, depthDecal 3, depthDecalMasked 4,
additive 5, additiveIgnoreDepth 6, modulate 7). The anonymous-namespace parser commands `cDecalEffectCommand`
(exit `0059DF50`), `cShakeEffectCommand` (exit `0059E260`) and `cGroupEffectCommand` (the `effect` block exit
`0059EB50`) are the block exits of effects.md §19 and effects-parser.md §12.

## 10. Quirks

1. Curve samples are not clamped: an age beyond the life reads past the last key.
2. The spin adds the rotated offset to the particle position (§1.3).
3. A particle closer to the camera than its own size and not bouncing is skipped and loses its attached effect
   (§1.2).
4. `SetTerrainControlledEffectState(false)` without a kind-7 effect dereferences null.

## 11. Conformance tests

1. Curve [0, 10] at f 0.25 → 2.5; at f 1.0 → 10; [5] at any f → 5.
2. Fake perspective with t = 0.5, scale 3, u = 2: pos moves by 2·0.5·2 = 2 along R.
3. Camera effect with radius 10: an id-14 point at distance 9.9 from the position in the plane sets pending; the next
   Update calls `SetViewTarget` once.
4. Effect tool: a press with no kind-7 effect sends parameters 11 and 10 to kind 5 only; release clears pressing.
5. DeformModel alpha: a = 1 → 255, a = 0 → 0, a = 0.5 → 128.
