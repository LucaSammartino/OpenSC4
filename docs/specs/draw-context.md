# Spec: the 3D draw context (`SC4DrawContext`, `cSC4PackedRenderState`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [view3d.md](view3d.md) (§1–§2: why the 3D view draws through this object and not the framework's 2D
draw context; §3.1 `+0x11C`; §4.1 the draw pass that calls `SetDefaultRenderStateUnilaterally`),
[graphics-formats.md](graphics-formats.md) (§3.8: the S3D material record, its flags and enums, and the texture
filter overrides), [texture-bindings.md](texture-bindings.md) (§3: the material object), [camera.md](camera.md)
(§1.3–§1.4: the view transform and projection the context loads), [lighting.md](lighting.md) (§4.3:
`GetModelLight`), [render-properties.md](render-properties.md) (the boolean, integer and float properties read
here), [view-utilities.md](view-utilities.md) (§7.1: the mesh layout and primitive types).

## 0. Summary

Every occupant, prop and model of the city is drawn through one `SC4DrawContext`. It sits on the graphics driver
(the `cIGZGDriver` of the SDK, an OpenGL-like interface) and keeps a **cache of the driver's state** in one 32-bit
word, so that a state is only sent to the driver when it changes. On top of the cache it offers **packed render
states** (a value and a mask over that word), the conversion of an S3D **material** into such a state with its
texture stages, the model-view **transform** composed from the camera and the model, model **shading** from the
lighting manager, and the drawing of meshes and whole model instances, with an emulated second texture pass when
the second stage is off.

## 1. The object *[compat]*

0x200 bytes, created by the 3D renderer's constructor (view3d.md §3.1, `+0x11C`); constructor `0x007D5F10` clears
the scalar fields described below, gives the highlight-colour vector (`+0x54`) 11 white entries and the
texture-stage table (`+0xE0`) none. It does **not** clear the entire object (see §1.3).
The renderer's `Init` (`0x007C6410`, at `0x007C670C`–`0x007C67F0`) then fills the links and calls `Init`:

| Offset | Content | Source |
|---|---|---|
| `+0x08`, `+0x0C` | two ints | the renderer's `+0x68`, `+0x6C` |
| `+0x10`, `+0x14`, `+0x18` | the zoom as a float and as an int, the rotation | the view utilities |
| `+0x24` | the view utilities | the renderer's `+0x114` |
| `+0x28`, `+0x2C` | two values | the renderer's `+0x14`, `+0x18` |
| `+0x30` | the **graphics driver** | the global `0x00B4E1EC` |
| `+0x34` | the renderer | |
| `+0x38` | the **render properties** | the renderer's `+0x48` |
| `+0x3C` | the **camera** | `SetCamera` (§8) |
| `+0x40` | the camera control | the renderer's `+0x8C` |
| `+0x44` | the **lighting manager** | the renderer's `+0x118` |
| `+0x48`, `+0x4C` | 0 | |

Other state: initialised (`+0x00`); two driver bytes (`+0x78`, `+0x79`) and the driver's texture-stage count
(`+0x7C`); the **state word** (`+0x80`, §2); the default packed state (`+0x88`) and its combiner record (`+0xC0`);
the default shade (`+0x94`–`+0x9C`, RGB); the depth offset (`+0xBC`); the highlight packed state (`+0xCC`) and its
combiner record (`+0xD4`); the stage table (`+0xE0`, 0x68 bytes a stage) and its size (`+0xEC`); the model-view
valid flag (`+0x138`), the model transform (`+0x13C`), the composed transform (`+0x174`) and its 4 × 4 matrix
(`+0x1AC`); a stack of packed states (`+0x1EC`–`+0x1F0`); the shade-override flag (`+0x1F8`); the index base
(`+0x1FC`).

### 1.1 `Init` (`0x007D6320`)

Once: the driver fills a record (driver slot 64); its stage count goes to `+0x7C` and the stage table is sized to
max(count, 2) default stage records (`0x007D3C60`); two of its bytes go to `+0x78` and `+0x79`. Then:

- the 11 **highlight colours** (RGBA): 0 (1, 1, 1, 1), 1 (0, 1, 0, 1), 2 (1, 0, 0, 1), 3 (0.2, 0.2, 0.4, 0.6),
  4 (0.66, 0, 0, 0.7), 5 (0.66, 0, 0, 1), 6 (0.5, 0, 0, 1), 7 (0, 0.37, 0, 1), 8 (0.5, 0, 0, 1),
  9 (0, 0.7, 0.2, 0.3), 10 (0.7, 0, 0.2, 0.3);
- the default packed state `0x230A8587` with mask all ones, and the highlight state `0x030A85B5` with mask
  `0x1FFFFFFF` (§5.2);
- the two complete 12-byte combiner records in §1.2.

### 1.2 Combiner byte records (DC4)

Offsets below are byte offsets within each record. All twelve bytes are copied by `SetTexCombiner`
(`0x007D3085`–`0x007D30A1`), independently of the meanings assigned by a particular driver.

| Record / point in lifecycle | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 0A | 0B |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Owner default `+0xC0`, constructor | 01 | 00 | 03 | 01 | 01 | 00 | 03 | 01 | 00 | 00 | 00 | 00 |
| Owner highlight `+0xD4`, constructor | 01 | 00 | 03 | 01 | 01 | 00 | 03 | 01 | 00 | 00 | 00 | 00 |
| Owner default, after `Init` | 02 | 00 | 01 | 01 | 00 | 01 | 03 | 01 | 00 | 00 | 00 | 00 |
| Owner highlight, after `Init` | 02 | 02 | 00 | 01 | 01 | 02 | 00 | 01 | 00 | 00 | 00 | 00 |
| Stage default at stage `+0x18` | 01 | 00 | 03 | 01 | 01 | 00 | 03 | 01 | 00 | 00 | 01 | 00 |

Values are hexadecimal bytes. Evidence: constructor stores `0x007D6011`–`0x007D609E`, including **word**
zeros at `+0xCA` and `+0xDE`; `Init` changes only the bytes written at `0x007D64CB`–`0x007D6521`.
The stage constructor writes word zero at `0x007D3C84`, then overwrites that word with 1 at
`0x007D3D75`. These zeros are established stores, not inferred padding. The records occupy 12 bytes;
the next owner fields are the highlight packed state at `+0xCC` and the stage vector at `+0xE0`.

### 1.3 Native lifecycle and owner boundary (DC3)

`SC4DrawContext` is a directly allocated, non-virtual helper here. The constructor writes an initialized
**byte**, initially false, at offset zero (`0x007D5F1A`), not a vtable. The renderer constructs it at
`0x007C9F6B`. No native draw-context `Begin` or `End` body or signature is established by these call sites;
the request to implement those as owner methods is withdrawn.

In particular, `DrawPreDynamicView` gets each list node's callback pointer from node `+8`, loads **that
callback's** vtable, and calls its slot 3 with the draw context as the single argument
(`0x007C3E30`–`0x007C3E3E`). `DrawPostDynamicView` does the same using a different list
(`0x007C3E70`–`0x007C3E7E`). The context is the argument, not the receiver. The former inference in
view3d.md §3.3 must not be used as an owner API contract.

The shutdown helper `0x007D2720` is called from renderer shutdown (`0x007C713D`). If initialized, it
clears the initialized byte, then nulls only the render-properties pointer `+0x38`; otherwise it does
nothing. It performs no driver calls, reference releases, stage clearing or stack clearing. `Init`
tests that byte first; a repeated call skips all setup (`0x007D632A`–`0x007D6330`). After shutdown,
the renderer must repopulate the properties link before operations needing properties; there is no
automatic pointer recovery.

The destructor-shaped helper `0x007D5BB0`, called by renderer destruction at `0x007CA063`, frees each
nonnull backing allocation in this order: `+0x1EC`, `+0x12C`, `+0xE0`, `+0x6C`, `+0x60`, `+0x54`.
It does not walk stage textures or release camera, lighting manager, driver or renderer references.
The renderer then frees the context allocation (`0x007CA069`). No draw-context reference-count API
is evidenced. The stage texture field is a borrowed pointer/value: `SetTexture` calls driver slot 58
when permitted, then overwrites the field without AddRef/Release (`0x007D4070`–`0x007D40AD`).

Constructor defaults already proved: stage-count fields `+0x7C` and `+0xEC` are 1 despite the empty
stage vector; state word and both packed records are zero; depth offset and index base are zero;
shade override and model-view-valid bytes are false; stack begin/end/capacity are null. Default shade
is **white**, not zero: the constructor copies three binary32 1.0 values from executable data
`0x00B0DC44`–`0x00B0DC4F` at `0x007D5FA0`–`0x007D5FBE`. Additional constructor/transform details
and the attributed method inventory appear below as they are resolved.

The two stored compact transforms (`+0x13C`, `+0x174`) start with counter/flags bytes zero, a
3 × 3 identity rotation, zero translation and binary32 scale 1 (`0x007D60DB`–`0x007D626B`).
Fog enabled `+0xA0` and lighting enabled `+0x90` start false. Fog colour cache `+0xA4`–`+0xB0`
and its two scalar caches `+0xB4`, `+0xB8` start at binary32 maximum finite `0x7F7FFFFF`
(`0x007D6000`, `0x007D62F0`–`0x007D630B`); these are cache sentinels, not the loaded driver fog state.
The constructor zeros the 0x38 bytes at `+0xF4` but makes no driver calls.

### 1.4 Stage storage defaults (DC5)

`Init` uses unsigned max(driver count, 2) records. The constructor `0x007D3C60` establishes the
following record; `0x007D3D41`–`0x007D3D72` explicitly overwrite the temporary matrix values with identity.

| Stage-relative offset | Width | Initial value |
|---|---|---|
| `+00` | 32 bits | texture value `0xFFFFFFFF` (not null) |
| `+04` | signed 32 bits | coordinate set 0 |
| `+08`, `+0C` | signed 32 bits each | wrap S, T: −1, −1 |
| `+10`, `+14` | signed 32 bits each | minification, magnification: −1, −1 |
| `+18`–`+23` | 12 bytes | stage combiner in §1.2 |
| `+24` | byte | transform-present false |
| `+25`–`+27` | 3 bytes | no stores established by this constructor; unspecified |
| `+28`–`+67` | 16 binary32 values | 4 × 4 identity matrix (diagonal 1, off-diagonal +0) |

Stage copying/assignment is a value copy, including the texture value; there is no texture reference
acquisition/release in those helpers or in reset. The constructor of the owner leaves its loaded
4 × 4 matrix at `+0x1AC` without initialization; `0x007D2730` merely copies those 64 bytes to an
output pointer and does not make them valid. Do not expose a fabricated identity before a transform load.

### 1.5 Attributed body inventory and foreign endpoints (DC3)

Addresses identify bodies, not a new virtual interface. Overloads and unnamed bodies are kept distinct.
The packed-state helper is `0x007D2B50`. The following owner bodies are identified by direct layout use
and/or receivers at callers; this is the audited native inventory, not a promise that all inlined
operations have out-of-line methods.

| Group | Owner bodies (VA suffixes below all have prefix `0x007D`) |
|---|---|
| Lifetime | `5F10` constructor; `6320` Init; `2720` shutdown helper; `5BB0` storage destructor |
| Transform | `2730` copy loaded 4 × 4; `2750` invalidate model-view; `2D20` plain view load; `3400` model transform; `3D80` camera; `4480` depth offset; `4B80` raw-matrix transform path (not a stack method) |
| Word state | `2760` colour mask; `27B0` depth test; `2800` depth mask; `2850` cull; `28A0` depth function; `28F0` blend factors; `2F50` alpha function/reference; `3E90` alpha-test enable; `3EF0` texture enables; `4010` blend enable |
| Stage state | `3030` combiner; `30B0` environment colour; `4070` texture; `40B0` environment mode; `4180` coordinates; `41E0` wrapping; `4280` filtering; `4340` matrix; `4420` clear matrix |
| Shade / environment | `2A20` transparency; `2A30` reset transparency; `30D0` lighting toggle; `3140` shade; `31B0` self-lit shade; `31E0` model shade; `3290` reset shade; `32D0` fog |
| State setup / replay | `4530` packed state; `47E0` emulated second pass; `4F80` write indexed highlight RGBA; `4FB0` highlight by index; `5200` default/top replay; `5230` unilateral reset; `5310` material; `5670` highlight material |
| Drawing | `2970` vertex buffer; `2980` index buffer; `2990`, `29C0`, `29F0` primitive overloads; `4A80` mesh; `5030` bounding box; `5710` model instance |

There is also a forwarding primitive body at `0x007FC2D0`: it replaces the receiver with context
`+0x30` and tail-calls driver slot 3. This does not introduce a lifecycle or stack operation.

Two export labels need owner correction: `0x007D30D0` and `0x007D32D0` are labelled
`cSC43DRender::TurnOnLighting` / `TurnOnFog`. They operate on the **draw context**. For example,
`0x007CB5DC` loads renderer `+0x11C` into the receiver before calling `0x007D30D0` at `0x007CB5E3`;
the body uses context driver `+0x30`, lighting manager `+0x44`, and default shade `+0x94`.
`0x007D4F80` is independently called eleven times by renderer `ReadProperties`, once per highlight
colour (`0x007C5FD8`–`0x007C6187`). It is not a pop operation. The range-copy helpers at `0x007D5170`,
`0x007D51A0`, `0x007D51D0`, and allocation helpers at `0x007D5AB0`, `0x007D5C20`, `0x007D5D60`,
`0x007D5EB0` are container internals, not additional rendering APIs.

Foreign calls below use the graphics-driver interface at context `+0x30`. The concrete DX7 table
`0x00AC4660` supplies these targets; another driver can supply different bodies at the same offsets.
These are not context methods, and the driver slots 3/4 are primitive draws, not Begin/End.

| Driver slot(s) | DX7 target(s) | Operation |
|---|---|---|
| 3, 4, 5 | `0x008857E0`, `0x00885A00`, `0x00881EC0` | DrawArrays, DrawElements, InterleavedArrays |
| 15, 16, 17 | `0x00882D10`, `0x00882CE0`, `0x00882D20` | colour mask, depth function, depth mask |
| 21, 22 | `0x00882F10`, `0x00882F60` | blend factors, alpha function |
| 27, 28, 29 | `0x008830A0`, `0x00884960`, `0x00884990` | environment colour, environment mode, texture parameter |
| 30, 31, 32, 33 | `0x00883180`, `0x00883100`, `0x00883240`, `0x00883310` | fog vector, fog integer, colour multiplier, alpha multiplier |
| 40, 41 | `0x00881EA0`, `0x00882AC0` | matrix mode, load matrix |
| 44, 45 | `0x00884EF0`, `0x00884FB0` | enable, disable |
| 51, 52, 53 | `0x00882020`, `0x00883550`, `0x00883600` | select stage, coordinates, stage matrix |
| 58, 62, 64 | `0x00884810`, `0x00884A70`, `0x00886E10` | texture binding, combiner, video-mode record |

Stack evidence is narrower than an API inventory: `SetDefaultRenderState` compares begin `+0x1EC`
with end `+0x1F0`, and passes either owner `+0x88` or the **eight bytes at end − 8** to packed replay
(`0x007D5200`–`0x007D5228`). It does not pop. Unilateral reset does not consult this stack. No push/pop
signature or mutation body has been attributed by this audit; adding generic Push/Pop methods remains
unsupported. The residual DC3 question records this precise limit.

## 2. The state word (`+0x80`) *[compat]*

| Bits | State | Driver call when sent |
|---|---|---|
| 0 | colour write | `ColorMask` (slot 15) |
| 1 | depth write | `DepthMask` (slot 17) |
| 2 | depth test | `Enable` / `Disable` (slots 44 / 45) of capability 1 |
| 3 | alpha test | capability 0 |
| 4 | blending | capability 4 |
| 5 | texturing on stage 0 | capability 5, on stage 0 (`TexStage`, slot 51) |
| 6 | back-face culling | capability 3 |
| 7–9 | depth function | `DepthFunc` (slot 16) |
| 10–12 | alpha function | `AlphaFunc` (slot 22), with the reference below |
| 13–16, 17–20 | source and destination blend | `BlendFunc` (slot 21) |
| 21–24 | alpha reference, q = round(ref × 15), read back as q / 15 | |
| 25–27 | stage 0 texture-environment mode | `TexEnv` (slot 28) on stage 0 |
| 28 | texturing on stage 1 | capability 5 on stage 1 |
| 29–31 | stage 1 texture-environment mode | `TexEnv` on stage 1 |

The function and blend values use graphics-formats.md §3.8's enums (the compare enum, the blend enum). Mode 4 of
a texture-environment field is the **combiner** (§7.3); the other mode values are the driver's *[inferred]*.

## 3. State setters *[compat]*

Each setter compares with the word and calls the driver only on a change; a value of −1 (or −1.0 for the alpha
reference) means "keep".

| Setter | VA | Behaviour |
|---|---|---|
| `EnableColorMaskFlag(on)` | `0x007D2760` | bit 0 |
| `EnableDepthMaskFlag(on)` | `0x007D2800` | bit 1 |
| `EnableDepthTestFlag(on)` | `0x007D27B0` | bit 2 |
| `EnableAlphaTestFlag(on)` | `0x007D3E90` | bit 3; on is treated as off while `kForceNoAlphaTest` (boolean 8) is set |
| `EnableBlendStateFlag(on)` | `0x007D4010` | bit 4; on is off while `kForceNoAlphaBlend` (boolean 7) is set |
| `EnableCullFaceFlag(on)` | `0x007D2850` | bit 6 |
| `EnableTextureStateFlag(on, stage)` | `0x007D3EF0` | bits 5 and 28: on with stage 0 turns stage 0 on, with stage 1 both; off with stage 0 turns both off, with stage 1 only stage 1; stage 1 reaches the driver only with `kUseSecondStage` (boolean 11); on with a higher stage goes straight to the driver, off with one does nothing |
| `SetDepthFunc(f)` | `0x007D28A0` | bits 7–9 |
| `SetAlphaFunc(f, ref)` | `0x007D2F50` | bits 10–12 and 21–24; the driver gets the unrounded reference |
| `SetBlendFunc(src, dst)` | `0x007D28F0` | bits 13–20 |
| `SetModelViewTransformChanged()` | `0x007D2750` | the model-view becomes invalid (§8) |
| `SetTransparency(a)` / `ResetTransparency()` | `0x007D2A20` / `0x007D2A30` | `AlphaMultiplier` (slot 33) a / 1.0 |

## 4. Render-property overrides *[compat]*

Besides the two "force no" switches of §3: texture filters and wrap modes are overridden by
`kForcePointSampling`, `kForceBilinear` and `kForceTextureRepeat` as graphics-formats.md §3.8 states.
The boolean-11 gate is operation-specific; combiner, clear-transform, unilateral reset and stages above 1
in environment-mode setup are exceptions (§7.2–7.3). Model culling and bounding boxes follow two more booleans (§9.2).

## 5. Packed render states *[compat]*

### 5.1 `cSC4PackedRenderState::PackRenderState` (`0x007D2B50`)

A packed state is a value and a mask over the word of §2. The arguments, in order: colour write, depth write,
depth test, alpha test, blending, texture count (0: stage 0 off; 1: stage 0 on, stage 1 off; 2 or more: both on),
culling, depth function, source blend, destination blend, alpha function, alpha reference, stage 0 mode, stage 1
mode. The booleans and the count always set their bits; each of the last seven, when −1 (−1.0), clears its field
from the mask instead, so the state leaves it alone.

### 5.2 `SetRenderState(packed)` (`0x007D4530`)

The fields that differ from the word under the mask are written into the word and sent to the driver, group by
group in §2's order, with the force switches of §3 applied to alpha test and blending and stage 1 sent only with
`kUseSecondStage`. Whatever changed, stage 0 then gets wrap repeat / repeat and filters linear /
linear-mipmap-linear (§7), and both stages lose their texture transform.

The default state is colour and depth write, depth test with function 3 (less-or-equal), alpha function 1 (less)
at reference 8/15, blend 4 / 5 (source alpha, one minus source alpha) but blending, alpha test, texturing and
culling off, both modes 1. The highlight state is the same with depth write off, blending and stage-0 texturing on
and stage-1 texturing off; it leaves stage 1's mode alone.

- `SetDefaultRenderState()` (`0x007D5200`): the top of the stack of packed states, or the default when it is empty.
- `SetDefaultRenderStateUnilaterally()` (`0x007D5230`): for every stage, the texture transform and coordinate set
  are reset at the driver (`TexStageMatrix` slot 53 with none, `TexStageCoord` slot 52 with 0); every stage record
  becomes the default record; then the default state. view3d.md §4.1 calls it at the start of each draw.
- `SetRenderStateHighlight(colour)` (`0x007D4FB0`): the highlight state, then the highlight colour `colour` as the
  stage-0 environment colour (`TexEnv`, slot 27) and the highlight combiner (`SetCombiner`, slot 62) on stage 0.

## 6. Material render state (`SetRenderState(material, drawFlags)`, `0x007D5310`) *[compat]*

The material (texture-bindings.md §3) holds its flags (`+0x04`), the depth function (`+0x06`), the alpha function
(`+0x07`), the alpha threshold byte (`+0x08`), the blend pair (`+0x09`, `+0x0A`), the texture count T (`+0x0B`) and
two texture slots of 0x14 bytes from `+0x0C`. The draw flags come with the model instance: a byte of bits and a
float a (`+0x10`). With ref = threshold / 255:

1. Draw-flag bit 3 (**grey**): the driver colour multiplier := (a, a, a), and bit 1 is ignored.
2. Draw-flag bit 1 (**fade**): when a < 1, ref := ref × a and the alpha multiplier := a. Even at a = 1 a faded draw
   is blended with source alpha / one minus source alpha *[quirk]*.
3. The packed state is built (§5.1) from: colour write = flag `0x100`; depth test = flag `0x02`, but on with draw
   bit 5 and off with draw bit 4; depth write = flag `0x80`, but off with draw bit 4 or 5; alpha test = flag `0x01`;
   blending = flag `0x10`, or on when faded; culling = flag `0x08`; the depth function, or 4 (greater) with draw bit
   5; the blend pair, or 4 / 5 when faded; the alpha function; ref; stage modes from slot 0's and slot 1's mode byte
   (−1 for an absent slot). Then §5.2.
4. When flag `0x20` is set, each of the T slots becomes a texture stage (§7): its texture (the slot's binding),
   coordinate set (slot `+0x0D`), wrap and filter bytes, its mode (mode 4 first loads the default combiner record),
   and a **texture transform** when slot `+0x0E` bit 0 is set: coordinate set 16 with s' = k (s − tx) and
   t' = k (ty − t), k = 1 / slot `+0x10` and (tx, ty) the composed transform's translation (§8); otherwise none.

`SetRenderStateHighlight(material, drawFlags)` (`0x007D5670`) uses the highlight state with the highlight colour
given by the draw flags' second byte, binds only slot 0's texture with its coordinate set, wrap and filters.

## 7. Texture stages *[compat]*

A stage record (0x68 bytes; defaults in §1.4) keeps the bound texture, coordinate set, wrapping, filtering,
combiner, transform flag and a 4 × 4 transform. Selection and suppression are **operation-specific** (§7.2).

| Call | VA | Behaviour |
|---|---|---|
| `SetTexture(texture, stage)` | `0x007D4070` | `SetTexture` (slot 58) |
| `SetTexCoord(set, stage)` | `0x007D4180` | `TexStageCoord` when the set changes |
| `SetTexWrapModes(s, t, stage)` | `0x007D41E0` | `TexParameter` (slot 29) 2 and 3; repeat when forced |
| `SetTexFiltering(mag, min, stage)` | `0x007D4280` | `TexParameter` 0 and 1, with the overrides of §4 |
| `SetTexTransform4(matrix, stage)` | `0x007D4340` | coordinate set 16, `TexStageMatrix`; stage 1's matrix is kept |
| `ClearTexTransform(stage)` | `0x007D4420` | when set: coordinate set 0 and no matrix |
| `SetTexEnvModeForStage(mode, stage)` | `0x007D40B0` | bits 25–27 or 29–31, `TexEnv` (slot 28) |
| `SetTexCombiner(record, stage)` | `0x007D3030` | `SetCombiner` (slot 62); the stage's mode bits become 4 |
| `SetTexColor(colour)` | `0x007D30B0` | the stage-0 environment colour |

### 7.1 The emulated second stage (`SetTexEnvMode`, `0x007D47E0`)

When `kUseSecondStage` is off and a material has two textures, `RenderModelInstance` draws each mesh a second time
after this call. If stage 1 is on in the word: stage 1's recorded texture, coordinate set, wrap modes, filters and
transform are put on stage 0; the depth function becomes 2 (equal) and alpha testing goes off; then by stage 1's
mode: 0, stage 0's mode 0 and blending off; 1, blend 0 / 2 (zero, source colour: a multiply) and blending on; 2,
stage 0's mode 0, blend 4 / 5 and blending on; any other mode keeps blending as it was. The Mac-matched name
`SetTexEnvMode` is kept although the function prepares this pass *[inferred]*.

### 7.2 Ordered stage/cache effects (DC5–DC7)

Let **allowed** mean stage = 0 or boolean 11 (`kUseSecondStage`) is nonzero. This gate applies to **all
nonzero** stages in binding, coordinates, wrapping, filtering and setting transforms, not just stage 1.
All indices here require allocated records; the owner does not check bounds. Lists below are in execution
order; an ordinary driver return advances execution without testing a success value.

| Operation | Selection / driver calls and local writes |
|---|---|
| Texture | If allowed, slot 58(texture, stage), **without** slot 51 and without equality test. Then store texture at stage `+00`, even when suppressed. |
| Coordinates | If allowed and requested set differs from cached `+04`, slot 51(stage), then 52(set). Always store requested set at `+04`, including suppressed stages. An equal set sends no selection. |
| Wrapping | If either requested S/T differs, and allowed: select once, parameter `(0,2,S)`, then `(0,3,T)` (forced repeat substitutes 3 for both). Then store **requested** S/T, even when suppressed. If both equal: no selection, calls or writes. |
| Filtering | If either requested mag/min differs, and allowed: select once, parameter `(0,0,mag)`, then `(0,1,min)`, with §4 overrides. Then store **requested** mag/min at `+14`/`+10`, even when suppressed. Equal pair does nothing. |
| Set transform | If allowed: invoke coordinate setter(16, stage), then select stage **again**, then slot 53(matrix,4,2,1). Always set flag `+24` true. Copy all 16 matrix words only for stage 1. Suppressed stages do **not** get coordinate set 16 through this path. No matrix equality test. |
| Clear transform | If flag false: nothing. Otherwise invoke coordinate setter(0,stage), then **unconditionally** select stage and call slot 53(null,3,2,1), then clear flag. Does not overwrite saved matrix. Thus suppressed stages still get select/clear, although their nested coordinate setter sends no calls. |
| Combiner | Unconditionally slot 62(record,stage), no selection/gate/equality test. Then set stage 0/1 word mode to 4, then copy all twelve bytes to stage `+18`. Above stage 1 there is no word-mode update. |
| Environment mode | Mode −1 returns immediately. Otherwise first write word 1 to stage `+22`. For stage 0/1 compare word mode; equal returns with only that word write. Changed stage 0/1 updates word **before** driver calls. Stage 1 suppresses select/mode if not allowed; stage 0 never suppresses. Above stage 1 always select and slot 28(0,0,mode), without a word-mode cache or boolean-11 gate. |

Primary instructions: texture `0x007D4091`–`0x007D40A8`; coordinates `0x007D41A1`–`0x007D41D3`;
wrap `0x007D4203`–`0x007D4273`; filter `0x007D42A3`–`0x007D4333`; transform
`0x007D4361`–`0x007D4411`; clear `0x007D4434`–`0x007D446B`; combiner
`0x007D3042`–`0x007D30A1`; mode `0x007D40B6`–`0x007D4175`.

Suppression updates the requested caches; turning boolean 11 on does **not** invalidate them. An equal
coordinate/wrap/filter request afterward can therefore remain silent. Changing only force-repeat or
force-filter properties also does not bypass requested-value equality. Graphics-formats §3.8's
"do nothing" wording applies only to suppressed **driver effects** for wrapping/filtering; local cache
writes still happen. It must not be read as discarding the request.

### 7.3 Unilateral setup and replay order (DC6–DC7)

Unilateral reset first iterates **every allocated record**, including stages above 1 and stages suppressed
by boolean 11: select i; slot 53(null,3,2,1); slot 52(0). This loop does not use the ordinary cached
setters or the boolean-11 gate. Only after all those calls, assign a freshly constructed default record
to every stage; then set the cached state word to bitwise complement of owner default packed value;
then replay the owner default (`0x007D5260`–`0x007D5302`). The default replay's ordinary tail sets
stage-0 wrap/filter values and clears transforms. It does not replay stack top, restore saved bindings,
or apply a combiner record. Thus stage 0 ends with wrap 3/3 and mag/min 1/7 in its requested cache;
other stages keep the constructor defaults, including coordinates 0 and texture `0xFFFFFFFF`.

Packed replay computes changed bits once from `(requested XOR cached) AND mask`. If nonzero, it first
stores `(cached AND NOT mask) OR requested` (requested is **not** masked here), selects stage 0,
then dispatches changed groups in order: colour mask, depth mask, depth-test enable, alpha-test enable,
blend enable, texture-0 enable, cull enable, depth function, alpha function/reference, blend factors,
stage-0 mode. If any stage-1 enable/mode bits changed and boolean 11 allows them: select stage 1,
texture-1 enable, stage-1 mode, select stage 0. Force-no-alpha-test/blend affect driver enable choices,
not the requested bits already stored by this replay. No driver result is checked.

Whether any packed bits changed or not, the tail is: wrap(3,3,stage0), filtering(1,7,stage0), clear
transform(stage1), clear transform(stage0), in that order (`0x007D47A6`–`0x007D47CD`). Cached wrap/filter
equality can suppress their calls. This tail is not a no-op merely because the packed word is unchanged.

The emulated pass first tests word bit 28; if clear it does nothing. Otherwise: bind stage-1 texture on
stage 0 and store it there; copy coordinates through the stage-0 cached setter; wrap S/T; filter mag/min;
set or clear the stage-0 transform from stage 1; change depth function to 2 if needed; disable alpha test
if needed; dispatch the mode-specific mode/blend effects in §7.1. These writes modify stage-0 caches
and word state, rather than saving/restoring them. Blend-enable calls still obey the force-no-blend
property. Model drawing performs the second mesh draw after this setup whenever boolean 11 is off and
material texture count is exactly 2 (`0x007D5A29`–`0x007D5A4D`); setup itself can be a no-op if bit 28 is off.

### 7.4 Invalid input, returned failures and retries (DC6)

Native setters generally return void: neither a checked error code nor transactional rollback is
established. Stage addressing is unchecked signed 32-bit `stage × 0x68` plus the stage-table pointer.
Invalid stage, null required pointer, invalid material texture count/binding or unmapped memory has
no safe-error contract here. Enum setters use the shifts/masks shown in §§2–5, not portable range
validation. Alpha values are not clamped before packing. A portable wrapper's validation errors are
policy outside this native contract.

For the driver calls in §§7.2–7.3 that **return normally**, even if a return register denotes failure
to that foreign implementation, the owner continues without testing it or attempting cleanup/retry.
The model-light query is a separate, explicitly checked foreign result (§8.1).
Texture/coordinate/wrap/filter caches advance after their calls; packed-state and alpha-function
caches advance before their calls; combiner mode/bytes advance after its call; transform-present is
set/cleared after its driver sequence. Therefore an equal retry of a cached coordinate or wrap/filter
pair is suppressed after a returned failure, whereas texture and combiner calls always run again.
There is no first-failure accumulator. An exception/nonreturning call has no recovery path in these
bodies; do not infer guaranteed later cleanup from it.

Normal model drawing cleans up after all meshes: restore default shade if override is pending, clear
the override flag, then unconditionally set alpha multiplier to binary32 1.0
(`0x007D5A5B`–`0x007D5A96`). That is end-of-loop work, not a cleanup handler around each foreign call.

## 8. Transforms and shading *[compat]*

- `SetCamera(camera)` (`0x007D3D80`): stores the camera, loads its projection matrix (`MatrixMode` 1, `LoadMatrix`
  of the transposed matrix, `MatrixMode` 0) and then the model-view as below.
- `SetModelTransform()` (`0x007D2D20`): when the model-view is not valid, the composed transform := the camera's
  view transform (camera.md §1.3), loaded as a column-major 4 × 4 matrix (rotation rows become columns, scaled by
  the transform's scale when it is scaled, translation in the last column) with `LoadMatrix` (slot 41); valid.
- `SetModelTransform(model)` (`0x007D3400`): records the model transform; the composed transform is the view
  transform (taken anew when the model-view is not valid) followed by the model's: rotation R = R_view R_model,
  translation t = t_view + s_view R_view t_model, scale s_view s_model, flags combined. It is loaded the same way
  and the model-view becomes **invalid**, so the next `SetModelTransform()` goes back to the plain view.
- `SetDepthOffset(n)` (`0x007D4480`): on a change, the camera's depth bias := `kDepthOffset` (integer property 16,
  2 by default) × n through `cS3DCamera::SetDepthBias` (`0x007FF0F0`, which also bumps the camera's projection
  counter), and the projection matrix is reloaded. The selection highlights use n = 4 (water-ripples.md §2.3).
- `SetShade(rgb)` (`0x007D3140`): the colour multiplier := rgb × the default shade; `SetSelfLitShade(rgb)`
  (`0x007D31B0`): rgb as is; `ResetShade()` (`0x007D3290`): the default shade, when an override is pending.
- `SetModelShade(instance, tint)` (`0x007D31E0`): the lighting manager's `GetModelLight(instance)` (slot 11,
  lighting.md §4.3), times the tint when given, as the colour multiplier.

### 8.1 Shade population, persistence and arithmetic (DC8)

`TurnOnLighting(on)` (`0x007D30D0`, corrected owner in §1.5) compares the requested byte with `+0x90`.
Equal means no calls or refresh. On a change to nonzero, a nonnull lighting manager receives slot 10
with a pointer to the **stored default shade** at `+0x94`. Its result is not checked. Then driver
slot 32 receives that stored RGB. A change to zero sends white RGB without overwriting the stored
default. The enabled byte changes after the driver call. This method does not change shade-override
`+0x1F8`. Consequently the stored default can remain the most recently obtained lighting colour even
while lighting is disabled (`0x007D30D8`–`0x007D3128`).

`SetModelShade` asks a nonnull lighting manager's slot 11 for model light into a temporary RGB.
It tests the returned **AL byte**; false, or a null manager, falls back to the stored default.
With a tint pointer, each component is multiplied by that tint and stored to binary32 separately
(`0x007D323B`–`0x007D3257`). It then calls driver slot 32 and sets shade-override true
(`0x007D326F`–`0x007D3275`). It never updates default shade or the lighting-enabled byte.
`SetShade` similarly sets override true after its driver call; `SetSelfLitShade` does so without
default-shade multiplication. `ResetShade` sends the stored default only when override is true,
and clears override afterward (`0x007D3293`–`0x007D32BD`). These are not success-tested mutations.

Material grey bit 3 sends `(a,a,a)` directly and clears the **local fade decision**, leaving both the
stored default and shade-override byte untouched (`0x007D5365`–`0x007D5379`). It neither sends alpha
multiplier 1 nor cancels an earlier multiplier. A previous mesh's fade therefore remains pending until
another multiplier call or normal model cleanup. Even a non-grey fade with a ≥ 1 makes no multiplier
call. Transparency setters themselves are uncached forwards (§3).

### 8.2 Numeric packing and texture matrix (DC8)

Alpha packing uses x87 multiply by binary32 15 (`0x00AB3CC8`), add binary32 12582912
(`0x00ABB4D4`, bits `0x4B400000`), **one binary32 store**, and extraction of the low four bits.
It is not an integer conversion or floor(x + 0.5). The multiply/add do not have an intervening binary32
store (`0x007D2CA2`–`0x007D2CC1`; `0x007D2FD7`–`0x007D2FF7`). With x87 round-to-nearest/even and
ordinary [0,1] references, this rounds 15r to nearest with **even half ties**, then retains its low nibble.
The ambient x87 control word is not changed by these bodies; other rounding modes must not be treated
as half-up. Binary32 input 0.5 gives q=8; binary32 0.7 (`0x3F333333`) gives q=10, since its product
with 15 is slightly below 10.5. There is no clamping step for out-of-domain values.

Direct `SetAlphaFunc` compares the incoming reference with the cached q reconstructed as binary32
q × binary32 `0x3D888889` (approximately 1/15). On dispatch, the driver receives the selected **original
reference**, while the owner stores its quantized nibble. Packed replay instead sends that reconstructed
binary32 reference (`0x007D46BE`–`0x007D46E1`). Thus equal packed q does not generally imply that repeated
direct calls with the same unquantized reference are suppressed. The −1 reference/function sentinels
retain the corresponding cached field. Material threshold conversion uses an x87 multiply by the
**binary64** constant at `0x00ABB4E0`, then stores binary32 before any fade multiplication
(`0x007D5348`–`0x007D5361`). Its binary64 bits are `0x3F70101010101010` (nearest binary64 1/255);
a decimal pseudocode literal is not evidence of a binary32 division.

For the material texture transform, the full 16-word buffer in column-major order is:

| Column | Row 0 | Row 1 | Row 2 | Row 3 |
|---|---|---|---|---|
| 0 | k | Z | Z | Z |
| 1 | Z | −k | Z | Z |
| 2 | Z | Z | Z | Z |
| 3 | −k·tx | k·ty | Z | Z |

Here k is x87 `1.0f / slot-scale`, tx/ty are stored composed translation, and Z is the loaded word
at `0x00B4E324`. That address is in the executable's PE zero-fill range (section RVA `0x00707000`,
raw size `0x1A000`, virtual size `0xB40D8`); its image-initial value is positive binary32 zero.
The exported direct references to it are loads, with no direct writer. This supplies the provenance
for the zero entries; neither z diagonal nor homogeneous diagonal is filled with 1.
The buffer goes to slot 53 with `(matrix,4,2,1)`, not a general model-view `LoadMatrix`.
Evidence: `0x007D55AA`–`0x007D5619`. k is stored to binary32 for element 0 while the x87 value remains
live for negation and translation multiplication, whose results are each separately stored to
binary32. Do not force an extra binary32 reciprocal rounding before those products. No zero-scale check exists.

### 8.3 Compact-transform branches, grouping and spills (DC8)

The second byte of each compact transform is tested as a **whole byte**, not as independent presence
bits for rotation, translation and scale (`0x007D3530`–`0x007D3543`). Model flags zero skip composition
entirely. Nonzero model flags with zero view flags copy the complete model payload/flags into the
composed record. When both are nonzero, the body composes the stored rotations/translations/scales,
ORs the two flag bytes and increments the composed counter byte (`0x007D376F`–`0x007D3797`). The
matrix-load path multiplies the rotation entries by scale only when composed flags have bit `0x02`
(`0x007D3826`–`0x007D38EB`); otherwise it copies stored rotation entries. Translation is always copied
and the last matrix row is explicitly `(0,0,0,1)` (`0x007D3969`–`0x007D3996`). Model-view-valid is cleared
after `LoadMatrix` (`0x007D39A3`–`0x007D39A9`).

The input compact record has counter at byte 0, whole flags at byte 1, rotation floats at `+4..+0x24`
(three rows in increasing address order), translation at `+0x28..+0x30`, scale at `+0x34`. The owner
first refreshes its composed record from the camera if invalid, incrementing its own counter, not
copying the camera counter (`0x007D3406`–`0x007D349A`). It then **always** increments the model-record
counter `+0x13C` and copies the input's thirteen payload floats and flags, even for zero model flags
(`0x007D34A0`–`0x007D352A`). It does not import the input counter or padding bytes.

| Whole model flags | Whole view/composed flags | Composition before matrix expansion |
|---|---|---|
| Zero | Any | Leave composed payload, flags and counter as they stand after any camera refresh. |
| Nonzero | Zero | Increment composed counter modulo 256; copy all thirteen model floats and whole flags without arithmetic. |
| Nonzero | Nonzero | Compose all rotation, translation and scale components, even when a component's conventional flag bit is absent; store flag OR and increment composed counter modulo 256. |

Thus absent component flags do not substitute identity components. In particular a stored view scale
participates in translation composition even if neither flag byte has bit `0x02`. Only the later
matrix expansion uses that bit. The following gives the arithmetic of the both-nonzero branch.
Use v[i,j], t[i], s for the pre-composition view rotation, translation and scale; m[i,j], u[i], z for
the input model values. Every load reads a binary32 value. Each multiplication and indicated addition
is a separate x87 operation under the ambient precision and rounding control; keep intermediates in
x87 registers except at the explicitly stated binary32 stores Q. No fused multiply-add is used.

For each rotation entry let Pk=v[i,k]×m[k,j]. The exact three-product addition grouping and final
binary32 store are:

| Composed rotation entry | Grouping | Binary32 store address |
|---|---|---|
| (0,0) | Q((P0+P2)+P1) | `0x007D3731` |
| (0,1) | Q((P2+P0)+P1) | `0x007D3741` |
| (0,2) | Q((P1+P0)+P2) | `0x007D3727`, then bit-copy at `0x007D3737` |
| (1,0) | Q((P0+P2)+P1) | `0x007D374F` |
| (1,1) | Q((P1+P0)+P2) | `0x007D3755` |
| (1,2) | Q((P1+P2)+P0) | `0x007D36C2`, then bit-copy at `0x007D3747` |
| (2,0) | Q((P2+P1)+P0) | `0x007D3763` |
| (2,1) | Q((P0+P2)+P1) | `0x007D3769` |
| (2,2) | Q((P2+P0)+P1) | `0x007D3661`, then bit-copy at `0x007D375B` |

There is no binary32 product or partial-sum spill within a rotation dot product. The three early
temporary stores in the table are the completed entries, not intermediate dot-product terms.
For translation let Di=((v[i,1]×u[1])+(v[i,2]×u[2]))+(v[i,0]×u[0]). Its asymmetrical spills matter:

| Entry | Exact store sequence | Evidence |
|---|---|---|
| t'[0] | Q(Q(D0×s)+t[0]) | Product spill `0x007D35BA`, reload/add `0x007D35CE`–`0x007D35D2`, final store `0x007D35F8` |
| t'[1] | Q(Q(D1×s)+t[1]) | Product spill `0x007D35C2`, reload/add `0x007D35D8`–`0x007D35DC`, final store `0x007D35FE` |
| t'[2] | Q((Q(D2)×s)+t[2]) | Dot-product spill `0x007D35B0`, reload/multiply `0x007D35C6`–`0x007D35CA`, final store `0x007D35EA`, bit-copy `0x007D35F2` |
| s' | Q(s×z) | `0x007D3775`–`0x007D377E` |

Translation completes before rotation stores; every rotation expression uses the old view rotation.
The composed flag OR/counter increment follows scale storage (`0x007D3784`–`0x007D3797`). For a normal
distinct input record these formulas describe the entire branch; passing overlapping owner storage
is not established by the callers and is not an additional supported aliasing contract.

Expansion writes column-major matrix float [4j+i] from rotation (i,j), multiplied by the **already
stored** composed scale with a separate Q for each entry when flag `0x02` is set; without that bit,
rotation floats are bit-copied. The scale-path stores are `0x007D3853`, `3871`, `3883`, `3899`, `38A8`,
`38B7`, `38CD`, `38DC`, `38EB` (all prefixed `0x007D`). Matrix floats 3,7,11 are explicit positive
zero; floats 12,13,14 bit-copy composed translation and float 15 is binary32 1.0. No flag suppresses
those translation copies. Call driver slot 41 once with the completed owner matrix; no MatrixMode
call occurs. Ignore its return, then clear valid (`0x007D39A3`–`0x007D39A9`). A normally returning
failure retains all preceding model/composed/matrix mutations and still clears valid. An equal retry
refreshes from the camera, increments/copies/composes again and calls the driver again; no equality
cache suppresses this operation. There is no status result, rollback or guaranteed clear after a
non-returning call. The algebraic summary in §8 is therefore not a substitute for these store rules.

### 8.4 Raw-matrix load (`0x007D4B80`, DC3/DC8)

This is an owner operation with one pointer argument to sixteen binary32 values (callee removes four
argument bytes at `0x007D4F71`). The receiver is the draw context; it is not a stack operation.
The four direct callers pass their context in ECX and a matrix pointer on the stack: border-mask draw
at `0x005DCE50`, underground-connection draws at `0x006572B0`/`0x006573B2`, and underground-occupant
draw at `0x0065B5DA`. These establish a raw-matrix overload; no Begin/End or Push/Pop API follows.

If model-view-valid `+0x138` is zero, fetch the camera view transform, increment the composed counter
byte `+0x174` modulo 256, and copy its thirteen payload floats and whole flag byte into `+0x178` through
`+0x1A8` and `+0x175` (`0x007D4B8A`–`0x007D4C1F`). Otherwise use the stored composed payload.
Unlike the compact overload's matrix expansion, this path **does not inspect flags or multiply by
the compact scale**. It neither records a model compact transform nor composes back into its payload.

For an unambiguous arithmetic description, number the input floats M[0] through M[15] in increasing
address order. Let v[i,j] be the stored rotation at owner `+0x178 + 4(3i+j)`, and t[i] the stored
translation at `+0x19C + 4i`, for i,j in 0..2. Products and each indicated addition are separate x87
operations under the ambient control word; intermediate expressions stay in x87 registers unless a
binary32 store is stated. There is no fused multiply-add. Let Q denote a binary32 store under the
ambient rounding mode. For i in 0..2 and j in 0..3 define p0=v[i,0]×M[j], p1=v[i,1]×M[4+j],
p2=v[i,2]×M[8+j], and p3=t[i]×M[12+j]. The temporary row values are:

| Temporary | Exact grouping before its single Q store |
|---|---|
| B[i,0] | Q(((p1+p3)+p2)+p0) |
| B[i,j], j=1,2,3 | Q(((p2+p1)+p0)+p3) |
| B[3,0] | Q(((M[4]+M[0])+M[8])×(+0.0)+M[12]) |
| B[3,j], j=1,2,3 | Q(((M[j]+M[8+j])+M[4+j])×(+0.0)+M[12+j]) |

The zero is executable binary32 data `0x00A81054` (bits `0x00000000`), explicitly multiplied at
`0x007D4C80`, `0x007D4CBE`, `0x007D4CD0`, `0x007D4CE6`. Do not replace these expressions by a direct
copy for nonfinite inputs. The four-term row grouping is established at `0x007D4CF3`–`0x007D4F29`.
Every B entry is stored to binary32 before the transpose; no partial sum or product within those
four-term expressions has a binary32 spill. For example B[2,2]/B[2,3] spill at `0x007D4D58`/`0x007D4D7E`,
B[1,2]/B[1,3] at `0x007D4DE7`/`0x007D4E0D`, and B[0,0..3] at
`0x007D4E32`/`0x007D4E5E`/`0x007D4E88`/`0x007D4EEE`; other live results spill in the final drain.

Write owner matrix float [4j+i] := B[i,j], then call driver slot 41 once with owner `+0x1AC`
(`0x007D4F30`–`0x007D4F5B`). No MatrixMode call occurs. The completed matrix is already stored when the
driver runs. Ignore its return value, then clear model-view-valid (`0x007D4F62`); no status is returned,
rollback performed or cleanup call issued. A normal-returning driver failure follows this same mutation
order. An equal retry still performs the load, and the cleared byte causes a camera refresh. If a call
does not return normally, there is no evidenced guaranteed post-call clear.

## 9. Drawing *[compat]*

### 9.1 Primitives and meshes

| Call | VA | Driver |
|---|---|---|
| `SetVertexBuffer(format, -, data)` | `0x007D2970` | `InterleavedArrays(format, 0, data)` (slot 5) |
| `SetIndexBuffer(-, base)` | `0x007D2980` | the index base := base |
| `DrawPrims(type, format, n, data)` | `0x007D2990` | the vertex array, then `DrawArrays(type, 0, n)` (slot 3) |
| `DrawPrims(…)` | `0x007FC2D0` | `DrawArrays` with the same arguments |
| `DrawPrimsIndexed(type, first, n)` | `0x007D29C0` | `DrawElements(type, n, 3, base + 2 first)` (slot 4; 3 = 16-bit indices) |
| `DrawPrimsIndexed(type, n, indices)` | `0x007D29F0` | `DrawElements(type, n, 3, indices)` |

`RenderMesh(mesh)` (`0x007D4A80`): a mesh with vertices (view-utilities.md §7.1's layout) sets its vertex array,
then draws each group (type, first, count): without indices `DrawArrays(type, first, count)`, with them the index
base := the index data and `DrawElements(type, count, 3, base + 2 first)`.

### 9.2 `RenderModelInstance(instance, model, drawFlags, highlight)` (`0x007D5710`)

1. Not highlighted, with `kShowBoundingBoxes` (boolean 26): the instance's box (instance slot 8) is drawn in red
   (1, 0, 0, 1) and, when its owner (occupant `0x26BB767E`) is an effects collection, the collection's box (slot
   10) in blue at alpha 0.5; nothing else is drawn. Highlighted with draw bit 3 set: nothing is drawn.
2. With boolean 30 (`kNoPartialBackingStoreCopies`) set: the instance's largest box extent × the view utilities'
   pixels per metre must reach `kRenderModelThreshold` (float 9), else nothing is drawn *[quirk]*. The shipped
   `Graphics Rules.sgr` sets the threshold from the City Detail option (12, 6, 2 for low, medium, high) and turns
   the boolean on only for some older Radeon cards (its comment: a "special low-impact scrolling mode"), so only
   those cards cull small models while drawing.
3. `SetModelTransform(instance's transform)`; the shade: `SetModelShade(instance, drawFlags + 4)` normally, or the
   draw flags' colour (`+0x04`) as is when highlighted or with draw bit 2 (then restored at the end).
4. For each of the model's meshes: its material state (§6, or the highlight form), `RenderMesh`, and the emulated
   second pass (§7.1) when `kUseSecondStage` is off and the material has 2 textures.
5. The alpha multiplier goes back to 1.

### 9.3 `DrawBoundingBox(box, colour)` (`0x007D5030`)

A shared mesh builder makes the box as six lit quads in the colour (`AddBoxMesh`, geometry-utilities.md §6.2); it
is drawn with the packed state `0x020A8197` / mask `0x1E1FE3FF` (writes, depth test less-or-equal, blending 4 / 5)
when the box is not empty.

## 10. Users

The 3D renderer's draw pass (view3d.md §4) sets the camera and the default state; every model and occupant `Draw`
method draws through `DrawPrims` and `RenderModelInstance` (58 call sites of `DrawPrims` alone, view3d.md §1); the
terrain view, the ripples and the selection (water-ripples.md) use the state setters and `SetDepthOffset`.

## 11. Quirks

1. A faded draw is blended even at full alpha (§6).
2. Small-model culling hangs on a boolean named for backing-store copies (§9.2).
3. Stage 1 state is recorded but not sent without `kUseSecondStage`, and is then replayed on stage 0 (§7.1).
4. The draw flags' bit 5 forces the depth function "greater" with the test on and no depth write: models drawn so
   appear only where something is in front of them *[inferred: the x-ray outline]* (§6).

## 12. Open questions

See [questions/draw-context.md](questions/draw-context.md): DC1–DC2 remain; DC3 retains stack-mutation
attribution limits. DC4–DC8 are answered
by §§1.2, 1.4 and 7.2–7.4. No runtime conformance result is implied by the source-derived vectors below.

## 13. Conformance tests

1. **Cache:** `EnableDepthTestFlag(true)` twice sends one `Enable(1)`.
2. **Force switch:** with `kForceNoAlphaTest`, `EnableAlphaTestFlag(true)` leaves alpha testing off.
3. **Alpha reference:** `SetAlphaFunc(1, 0.5)` stores q = 8 and a later state replay sends 8/15.
4. **Packed mask:** a packed state with depth function −1 leaves bits 7–9 unchanged.
5. **Default:** `SetDefaultRenderState` with an empty stack applies `0x230A8587`.
6. **Composition:** view rotation identity, translation (1, 2, 3), scale 2; model translation (1, 0, 0): the
   composed translation is (3, 2, 3) and the scale 2.
7. **Depth offset:** with `kDepthOffset` 2, `SetDepthOffset(4)` sets the camera's depth bias to 8.
8. **Second pass:** stage 1 mode 1 gives blend (0, 2), depth function equal, alpha test off.
9. **Culling:** with boolean 30 set, a model whose largest extent × pixels per metre is below the threshold is
   skipped.

### 13.1 Independent contract vectors for DC3–DC8

These are expected traces/state from native instructions, not measurements of a replacement. Select(i)
means driver slot 51; coordinate(n) means slot 52; matrix arguments are in slot-53 argument order.

| ID | Input / prior state | Expected observations |
|---|---|---|
| DC-V01 | Construct then Init once | Owner combiners exactly §1.2; all stage records exactly §1.4 except unspecified padding; default shade white; no fabricated loaded model-view matrix. |
| DC-V02 | Fresh stage 0, coordinate(0), then coordinate(2) twice | First call no driver effect; second Select(0), coordinate(2); third silent. Texture(T,0) twice instead sends slot 58 twice, no Select. |
| DC-V03 | Boolean 11 off, allocated stage 2; wrap(1,2,2), then turn boolean 11 on and repeat | First call stores 1/2 with no driver calls; second is silent due to requested cache equality. Change T to 3: Select(2), parameters (0,2,1), (0,3,3). |
| DC-V04 | Allowed stage 0, coordinate cache 0; set transform M twice | First Select(0), coordinate(16), Select(0), matrix(M,4,2,1). Second Select(0), matrix(M,4,2,1). Matrix cache stays identity because only stage 1 copies M. |
| DC-V05 | Boolean 11 off, stage 1 coordinate 7, transform flag false; set M then clear | Set: flag true and all 16 saved words=M, coordinate remains 7, no driver calls. Clear: coordinate becomes 0, Select(1), matrix(null,3,2,1), flag false; saved M persists. |
| DC-V06 | Three allocated stages, boolean 11 off; unilateral reset | For i=0,1,2 first emit Select(i), matrix(null,3,2,1), coordinate(0), with no gate. Only after the whole loop reset records and force default word replay. Stage-0 requested wrap/filter ends 3/3, mag/min 1/7; stages 1/2 remain −1 pairs. Stack top is ignored. |
| DC-V07 | Cached word already equals packed request; stage-1 transform flag true | No changed-word dispatch, but packed tail still executes and clears stage 1 before stage 0; stage-1 Select/clear occurs even with boolean 11 off. |
| DC-V08 | Coordinate changed 0→3; foreign coordinate call returns a failure-valued register normally | Owner stores 3. Identical retry emits no select/coordinate calls. A repeated texture bind still dispatches. No safe-error result or rollback is provided by owner. |
| DC-V09 | Default shade (0.5,0.25,1), manager slot 11 returns false, tint (0.5,2,0.25) | Multiplier (0.25,0.5,0.25); stored default unchanged; override true. Reset sends (0.5,0.25,1) and clears override. |
| DC-V10 | Fade mesh a=0.25 followed by grey mesh a=0.75 in one model draw | Fade sends alpha multiplier 0.25. Grey sends RGB (0.75,0.75,0.75), makes no alpha call, so 0.25 persists. Normal model cleanup restores pending shade first, then alpha 1. |
| DC-V11 | Pack alpha r=0.7f, nearest/even control mode | q=10, not 11. Replay sends binary32 10 × 0x3D888889. Direct SetAlphaFunc sends original 0x3F333333, storing q=10. |
| DC-V12 | Texture scale 2, composed tx=3, ty=−4 | Full columns: (0.5,0,0,0), (0,−0.5,0,0), (0,0,0,0), (−1.5,−2,0,0), subject to image-initial Z=0. No z/homogeneous identity entries. |
| DC-V13 | SetDefaultRenderState with one 8-byte record at stack end−8 | Replays that record without decrementing end. Empty begin=end replays owner +0x88. No push/pop is implied. |
| DC-V14 | Initialized context; shutdown twice | First clears initialized then properties pointer, no foreign calls. Second does nothing. Stage texture values, stack pointers and camera pointer remain untouched. |

### 13.2 Compact/raw transform vectors (DC3/DC8)

These are source-derived expectations, not game or replacement measurements. Finite numeric examples
use masked x87 exceptions, 64-bit significand precision and nearest/even rounding, including Q stores.
They deliberately include non-orthonormal payloads to expose operation order; no normalization occurs
in these bodies. Unless stated otherwise, valid starts true (so no camera fetch), composed counter=40,
model counter=7, unspecified translation=0 and scales=1. The input counter never becomes an owner counter.
All counter arithmetic below is modulo 256. Driver failures below mean a failure-valued return that
returns normally, not an exception. Hex words describe binary32 bits; NaN payload selection is not asserted.

| ID | Input / prior state | Expected observations |
|---|---|---|
| DC-V15 | Compact model flags=0 with dirty model rotation/translation/scale; view flags=2, rotation I, scale=7, translation (10,20,30) | Model record copies all input payload/flags and counter becomes 8. Composed payload/flags/counter stay unchanged at 40. Loaded matrix is 7I with translation (10,20,30); slot 41 once, then valid=false. Dirty model components do not compose. |
| DC-V16 | Compact view flags=0 with dirty view payload; model flags=0x80, rotation 3I, translation (4,5,6), scale=5, input counter=200 | Copy model payload/flags to composed; composed counter=41, model counter=8. Loaded rotation 3I (bit 2 absent), translation (4,5,6); composed scale remains 5. No arithmetic uses the dirty view payload. |
| DC-V17 | Compact view flags=0x80, rotation 2I, scale=5, translation (1,2,3); model flags=1, rotation 3I, scale=7, translation (4,5,6) | Compose rotation 6I, translation (41,52,63), scale=35, flags=0x81, counter=41; model counter=8. Loaded rotation is 6I because bit 2 is absent, despite scale participating in composition. No absent component becomes identity. |
| DC-V18 | Compact flags both 1; all view rotation entries=1; model rows (A,A,A), (−A,−A,−A), (1,1,1), A=2^80 | Composed rotation rows are (0,0,1), (0,1,0), (0,0,0); zeros have bits 00000000, ones 3F800000. Replacing every dot product by one fixed addition order changes these results. Final matrix transposes these rows as §8.3 specifies. |
| DC-V19 | Compact flags both 1; each view row=(1,1,0), view scale=3, translation (−3,−3,−3); model translation (1,2^−24,0) | Each unspilled Di=1+2^−24. Translation x/y=2^−22 (34800000), z=0 (00000000). x/y spill the scaled product to 3+2^−22 before adding −3; z spills Di to 1 before scaling. A common spill policy for all axes fails. |
| DC-V20 | Compact flags both 2; view row 0=(1,1,0); model column 0=(1,2^−24,0); view scale=1, model scale=3 | Composed rotation (0,0) first stores 1 (3F800000). Matrix float 0 then stores 3 (40400000). Combining rotation dot product and scale before that store would give 40400001. |
| DC-V21 | Compact flags both 2; both rotations I; view scale=1+2^−23, model scale=1−2^−24 | The scale product is 1+2^−24−2^−47, which stores as 1 (3F800000) at +0x1A8 before expansion. Loaded rotation is I. |
| DC-V22 | Raw M=I; composed rotation I, translation (10,20,30), scale=7, flags=2 | Loaded rotation I, translation (10,20,30); composed payload/flags/counter and model record/counter remain unchanged. Slot 41 once, no mode call, then valid=false. In contrast DC-V15's compact path loads 7I. |
| DC-V23 | Raw view row 0=(0,1,1), t[0]=1; M column 0=(0,A,1,−A), A=2^80; other M entries zero except M[15]=1 | B[0,0], hence matrix float 0, is 1 (3F800000): ((A−A)+1)+0. Accumulating p0,p1,p2,p3 instead yields 0. This independently fixes raw column-0 grouping. |
| DC-V24 | Raw M[0]=positive infinity, M[4]=M[8]=0, M[12]=9 | Matrix float 3 (B[3,0]) is NaN from infinity×0, rather than a copy of 9. The explicit zero product cannot be eliminated for arbitrary nonfinite input. |
| DC-V25 | Compact view/model flags both 1; valid=true, counters 40/7; first slot-41 call returns failure, retry same model; camera supplies a nonzero-flag view on refresh | At first driver entry model counter=8, composed counter=41, completed matrix present and valid still true; after return valid=false. Retry fetches camera, increments composed to 42 without copying camera counter, then composes to 43; model counter=9. Two loads, no rollback or silent equal retry. |
| DC-V26 | Raw valid=true, counters 40/7; first slot-41 call returns failure, retry same M | First entry retains composed counter=40/model counter=7; matrix is complete and valid still true. Return clears valid. Retry refreshes view and increments composed to 41; model counter stays 7. Two loads, no mode calls or failure cleanup. |
