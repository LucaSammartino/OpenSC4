# Spec: model instances (`cS3DModelInstanceBasic`, `cS3DModelInstance`, `cS3DModelProcessor`, sprite animations)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [graphics-formats.md](graphics-formats.md) (§3 the S3D model, §3.9 the **animated instance's
playback**, §3.12 building a frame, which this spec does not restate), [draw-context.md](draw-context.md)
(`RenderModelInstance`), [view-utilities.md](view-utilities.md) (§7 picking and the triangle walk),
[lighting.md](lighting.md) (§4.4 `GetVertexColors`), [animated-sprite.md](animated-sprite.md) (the sprite
instance, which reads the resources of §6), [lot-models.md](lot-models.md) and
[geometry-utilities.md](geometry-utilities.md).

## 0. Summary

Everything the scene draws is a **model instance**. `cS3DModelInstanceBasic` is the common part: a reference count,
an owner, custom data, properties, draw attributes and the interface the scene calls (bounds, transform, pick,
draw, sort value); the lot models, the animated sprites and the model instances below build on it.
`cS3DModelInstance` is a static model: it holds the meshes and materials of one S3D frame and draws them through
the draw context. `cS3DModelInstanceAnimated` plays the frames of an S3D animation (graphics-formats §3.9).
`cS3DModelProcessor` takes over a static instance to light its vertices itself, to give it vertex normals and to sort
its meshes. The sprite-animation resources (`cS3DResSpriteAnim*`) hold the per-zoom sheets that sprite instances
draw.

## 1. The basic instance (`cS3DModelInstanceBasic`) *[compat]*

A virtual base (IID `0x26A68449`; constructor `0x00801CF0`) of every model instance. The instance interface, as the
base implements it (vtable `0x00ABD468`; derived classes replace the slots marked "derived"):

| Slot | Call | Base behaviour (VA) |
|---|---|---|
| 0 | `QueryInterface(iid)` | IIDs 1 and `0x26A68449` give the basic part, with a reference (`0x00801940`) |
| 1, 2 | `AddRef`, `Release` | a 16-bit count; the last release runs the most-derived deleting destructor (`0x00801900`, `0x00801910`) |
| 3, 4 | `GetOwner`, `SetOwner(o)` | a counted reference (`0x00801850`, `0x00801860`) |
| 5, 6 | `GetCustomData`, `SetCustomData(d)` | one 32-bit value (`0x00801890`, `0x008018A0`) |
| 7 | `GetBounds(box)` | calls slot 8 of the most-derived object (`0x008018B0`) |
| 8 | `GetWorldBounds(box)` | derived |
| 9 | `GetTransform(t)` | the identity (`cS3DTransform::MakeIdentity`, `0x00801DC0`) |
| 10 | `SetTransform(t)` | ignored |
| 11 | `Pick(context)` | false |
| 12 | `Draw(context, …)` | nothing (`0x008018D0`) |
| 13, 14 | `GetProperties`, `SetProperties(p)` | one 32-bit value (`0x008018E0`, `0x008018F0`) |
| 15, 16 | `GetDrawAttributes(a)`, `SetDrawAttributes(a)` | a 20-byte record copied out and in (`0x00801E10`, `0x00801E50`) |
| 17 | `GetSortValue(v)` | derived |
| 18 | `GetResModel()` | null |

`cS3DTransform::MakeIdentity` (only when the transform's "dirty" byte is set) writes the identity rotation, a zero
translation and scale 1, clears the byte and bumps the change counter (the transform layout is view-utilities §7).

### 1.1 Construction, records and lifetime (MI3, MI10)

The basic constructor starts with **zero** native references, null owner, custom data 0 and properties 2
(`0x00801D2B`–`0x00801D35`). A successful query adds a reference; an unsupported query returns false without
writing the output (`0x00801940`). The 16-bit reference count increments modulo 65536 and returns the
zero-extended result. Release decrements only counts greater than 1; **both 0 and 1 destroy the most-derived
allocation and return 0** (`0x00801900`, `0x00801910`). Thus incrementing 65535 returns 0, and the following
Release destroys the allocation; there is no native overflow protection or hidden surviving ownership.
A portable handle may protect memory safety, but must not describe surviving handles after that destruction
as native behaviour. A newly returned counted instance normally has 1, distinct from the constructor's 0.

Owner getters return a borrowed pointer. Setting the same owner causes no reference calls. Basic owner
replacement releases the old owner **before** adding a reference to the new one, then stores the pointer
(`0x00801860`); destruction releases the remaining owner once (`0x00801D93`). Static owner replacement uses
the opposite order: retain new, store new, release old (`0x00801B50`). These orders matter to callbacks.

The 20-byte draw record is copied field-for-field, including its otherwise uninterpreted bytes. Its relative
layout and constructor defaults are:

| Record bytes | Default in both basic and static instances | Contract |
|---|---|---|
| `+0` | 0 | Flag byte; static SetModel ORs `0x0E` into this byte only |
| `+1` | 0 | Separate byte, preserved by the copy |
| `+2..3` | Unwritten by these constructors | Preserved 16-bit field; no deterministic native default |
| `+4`, `+8`, `+12` | binary32 1, 1, 1 | Three copied words, initialized from `0x00B0E35C..64` |
| `+16` | binary32 1 | Copied word |

Evidence: basic construction `0x00801D3C`–`0x00801D5D`, static construction
`0x00802AA0`–`0x00802ACA`, copies `0x00801E10`/`0x00801E50` and `0x00801F00`/`0x00801F40`;
the executable data at `0x00B0E35C..64` contains three `0x3F800000` words. A portable API may choose initial
values for the two unwritten bytes, explicitly as a choice; it must preserve them after a supplied record.

### 1.2 Transform counter (MI3)

The change counter is an **unsigned byte**, incremented modulo 256. MakeIdentity increments the destination
counter only when its byte `+1` is nonzero; otherwise it changes nothing, even if the other fields are not an
identity (`0x00801DC4`–`0x00801DFC`). Static GetTransform increments the caller's destination counter, copies
the 13 binary32 words at `+4..+0x34`, then copies byte `+1`; it does not return the instance's counter
(`0x00801B80`). SetTransform increments the **instance's existing** counter, copies the same words and byte
`+1`, and ignores the supplied counter (`0x00801BE0`). Neither operation copies bytes `+2..3`.

## 2. Static model instances (`cS3DModelInstance`) *[compat]*

An object of `0xA4` bytes from a pool (`operator new` `0x00801B00`, released by `0x008020F0`; constructor
`0x00802950`, destructor `0x00802620`, deleting destructor `0x00802B00`). It is also a **mesh container**
(IID `0x26A68448`): the meshes (12-byte records: vertex block, index block, primitive block, each counted) and the
materials (52-byte records, graphics-formats §3.8) of one model frame.

Static construction starts with empty mesh/material arrays, null owner and model, custom data 0, properties 2,
an identity transform with counter 0 and byte `+1` zero, and stored bounds min=(1,1,1), max=(0,0,0)
(`0x00802950`–`0x00802AFE`). The bounds-refresh flags start set. Properties store only the low 16 bits
(`0x008017A0`), unlike the basic class's full 32 bits. Static instances have a **32-bit**, initially zero,
reference count shared by their instance and container interfaces: increment modulo 2³², Release destroys
at 0 or 1 (`0x004F0240`, `0x008016B0`, static vtable `0x00ABD554`). The static factory explicitly adds the
first reference after construction (`0x00804104`–`0x00804108`); the basic 16-bit rule must not be applied to it.

Mesh container slots (vtable `0x00ABD554`; the bare container of `0x00ABD4E0` has the same slots 3–7):

| Slot | Call | VA |
|---|---|---|
| 0 | `QueryInterface`: `0x26A68449` gives the basic part; 1 and `0x26A68448` the container | `0x008017B0` |
| 3 | `GetMeshCount()` | `0x00801AD0` |
| 4, 5 | `GetMesh(i)` | `0x00801AF0` |
| 6, 7 | `GetMaterial(i)` | `0x00801EC0` |
| 8 | deleting destructor | `0x00802B00` |

Instance slots (vtable `0x00ABD508`, thunks `0x008026A0`–`0x008027C0` to the bodies):

| Slot | Call | Behaviour (body) |
|---|---|---|
| 3, 4 | owner | a counted reference (`0x00801B40`, `0x00801B50`) |
| 5, 6 | custom data | (`0x00801700`, `0x00801710`) |
| 7 | `GetBounds(box)` | the model's box, refreshed from the model (its slot 7) when marked stale (`0x00801FC0`) |
| 8 | `GetWorldBounds(box)` | the same box, then transformed by the instance's transform (`0x00802010`) |
| 9, 10 | `GetTransform`, `SetTransform` | the 56-byte transform; setting it bumps the change counter (`0x00801B80`, `0x00801BE0`) |
| 11 | `Pick(context)` | view-utilities §7 over the meshes (`0x00801720`) |
| 12 | `Draw(context, …)` | `RenderModelInstance(context, instance, transform, draw attributes, …)` (`0x00801750`; draw-context.md) |
| 13, 14 | properties | a 16-bit value here (`0x00801790`, `0x008017A0`) |
| 15, 16 | draw attributes | (`0x00801F00`, `0x00801F40`) |
| 17 | `GetSortValue(v)` | the first material's first texture ID, else 0 (`0x00801F80`) |
| 18 | `GetResModel()` | the model (`0x00614A20`) |

`SetModel(model)` (`0x00801C40`) takes a counted reference to the model and inspects its first material
(graphics-formats §3.8). When its bytes `+9` and `+10` are 8 and 1 *[inferred: the blend pair]*, the instance's
properties get bit 2 set and bit 1 cleared, its draw attributes bits 1–3 set, and the material loses flag `0x80`
(depth write) and its byte `+0x18`; a material with flag `0x10` (blending) only changes the properties that way.
`0x00801CD0` refreshes the instance's box from the model.

## 3. Animated model instances (`cS3DModelInstanceAnimated`) *[compat]*

The playback (constructor `0x007FD870`, `SetFrame`, `SetSpeed`, `Advance`, `Step`, `SetAnimMode`) is
graphics-formats §3.9. The other animation slots:

- `GetFrame()` (slot 11, `0x007FAF90`) and `GetFrameCount()` (slot 12, `0x007FC150`): from the sub-range table at
  `+0xA4` when set, else the current frame and the header's frame count F.
- `SetAnimation(index)` (slot 9, `0x007FDA90`): with a sub-range table only, selects range `index` (its first frame
  and length) and builds its frame; without one it does nothing. graphics-formats §3.9 notes that the table is
  never set.
- Building frame f (`0x007FC2E0`): the model's frame f (model slot 8, `GetModelFrame`) into the instance's meshes;
  a frame equal to the current one is not rebuilt.

## 4. The model processor (`cS3DModelProcessor`) *[compat]*

A helper that relights a static model instance on the CPU. Its state: the instance, its mesh container, the target
vertex format (standard 10: position, colour, uv; graphics-formats §3.5), the original vertex blocks, the vertex
normals per mesh, and the mesh order. Constructor `0x00803060`; destructor `0x008030A0`.

- `SetModel(instance)` (`0x008038D0`):
  1. A previous instance first gets its original vertex blocks back; then the new instance is taken.
  2. An animated instance (IID `0xA818EDD8`) with more than one frame is refused: false, nothing kept.
  3. Its mesh container (IID `0x26A68448`) must exist, be nonempty and have every vertex block in standard
     format 2 (position, uv). Otherwise both instance and container references are cleared and the result is false.
  4. Each mesh's vertex block is replaced by a new one in format 10 with the same vertices: position and uv copied,
     colour white (`0xFFFFFFFF`), the same bounds. The original blocks are kept for step 1. True.
- `CreateVertexNormals()` (`0x00803370`): for each mesh, each triangle (the triangle walk, view-utilities §7.1)
  adds its normal (`0x0080B490`, Newell's formula, whose length is twice the triangle's area) to its three vertices
  and its length to their weights; each vertex normal is then the sum ÷ the weight sum, when the weight is above 0.
  The result is the area-weighted mean of the unit face normals, **shorter than 1** where faces meet at an angle
  *[quirk]*.
- `LightModel(lighting, flip)` (`0x00803E10`): makes the normals when there are none; sets flag 8 on material 0
  *[inferred: use vertex colours]*; takes the instance's rotation (slot 9), negated when `flip` is set; then for
  each mesh whose original block was format 2, the lighting manager's `GetVertexColors` (lighting.md §4.4) writes
  the vertex colours from the normals and that rotation.
- `SortMeshes(direction)` (`0x00802DB0`): rebuilds the order from container indices 0 upwards on every call.
  It inserts each index before the first previous score for which the new score is **not strictly less**.
  Scores are the dot product of mesh-box centre with direction, largest first for finite unequal values.
  Equal scores (including signed zeros) reverse container order. An unordered comparison also inserts before
  that entry; this is not a stable sort (`0x00802E82`–`0x00802EFC`).
- `ReleaseModel()` (`0x00802B30`): clears/releases the instance, releases the container, leaves the container
  pointer stored, and marks that reference already released. It does not restore vertices or clear the saved
  original blocks. It requires a nonnull container and is not safe to repeat as a native operation.

### 4.1 Ownership, returns and restoration (MI6, MI8)

Construction has null instance/container, target vertex format 10, empty saved-block/normal/order arrays, and
the already-released flag false (`0x00803060`). A successful SetModel owns separate counted references to the
instance and its queried mesh container, even when they are parts of the same allocation. It also owns one
reference per saved original vertex block; each container mesh owns its replacement block. The target format
does not relax the eligibility check for original format 2. Null input, absent container, zero meshes and any
non-format-2 block all return false with neither instance nor container retained
(`0x00803AFC`–`0x00803B0D`, `0x00803DD6`–`0x00803E03`). The multi-frame animation case is an earlier refusal.
Only completion of a nonempty conversion returns true (`0x00803DCA`).

On SetModel, the previous container's vertex slots are restored before its saved originals and normals are
cleared and its container reference is dropped. Rebinding the same instance still performs that restoration
and conversion. Destruction also restores saved originals: **first** release/clear the instance, **then** restore
each original vertex slot, **then** release/clear the container unless ReleaseModel already released it,
and finally discard the processor arrays (`0x008030A4`–`0x0080312F`). Reference replacement retains the restored
block before releasing the replaced block. Thus ordinary destruction restores vertices, unlike ReleaseModel
alone. After ReleaseModel, a later destruction/rebind can still use that borrowed container pointer; an external
owner must keep it valid. Repeated ReleaseModel, ReleaseModel with no container, and use of a dangling released
container have no defined safe native result. An idempotent portable clear is a separate API choice.
The shipped direct caller is network `FlipAndLight`, which calls ReleaseModel after LightModel
(`0x00616CA6`–`0x00616CAD`); this supports its transfer-of-ownership use, not an idempotency guarantee.

Static SetModel's material changes affect the **instance material copy**, not the source resource material.
The original frame builder copies source material fields into the destination material array
(`0x00806E41`–`0x00806E59`, copy helper `0x00802240`), while retaining shared texture bindings. SetModel changes
the instance's first material bytes at `+4` and `+0x18` (`0x00801C92`, `0x00801C9B`). Sharing texture bindings
does not make the material flags shared. A second instance built from the resource therefore starts with the
resource's original flags. The model pointer itself is counted, with retain-new/store/release-old order and a
same-pointer shortcut, but the material inspection still runs after that shortcut (`0x00801C40`).

Sort's centre computation uses each original container mesh's current vertex-block bounds. It resets the
output list, not the container mesh array (`0x00802DF8`, `0x00802E22`). The three min+max sums and dot product
use x87 operations; the score is stored as binary32 before comparison (`0x00802E28`–`0x00802E79`). The insertion
rule above applies literally to NaNs as well as finite scores; it does not establish a total order for NaNs.

## 5. Bounding boxes and small types *[compat]*

`cS3DVertexArray::UpdateBounds` (`0x00801040`) is geometry-utilities §6.1. `cS3DBoundingBox::Center` (`0x0046FFC0`)
is (min + max) ÷ 2 per axis.

## 6. Sprite-animation resources (`cS3DResSpriteAnimFactory`) *[compat]*

The factory (destructor `0x00808B70`) creates two resource types (`0x00808BD0`) and reads them by type
(`SerialRead`, `0x00808B20`):

| Type | Object | Interface | Reader |
|---|---|---|---|
| `0x29A5D1EC` | sprite animation (`0x5C` bytes) | `0x29A5D1EC` (`0x008085C0`) | `ReadAnim` (`0x00808660`) |
| `0x09ADCD75` | sprite-animation table (`0x20` bytes) | `0x69B6F01C` (`cS3DResSpriteAnimTable::QueryInterface`, `0x00808630`) | `ReadTable` (`0x00808920`) |

CreateInstance allocates an unread object and queries the requested interface; successful query supplies the
returned counted reference. Unsupported type or unsupported requested interface returns false and null;
the latter destroys the new allocation (`0x00808BD0`). SerialRead dispatches on the existing resource's type
and returns the reader's Boolean; it does not itself release or null the resource on failure (`0x00808B20`).
Neither constructor invents unread animation headers. The table constructor sets its block pointer null
(`0x00808BA0`); its destructor frees that block once, then destroys the persistence base (`0x00808AD0`). The
animation destructor destroys the persistence base and allocation, with no sheet references to release
(`0x00808A00`). Both own-IID queries retain the returned interface and otherwise delegate to the persistence
base (`0x008085C0`, `0x00808630`); the table's **resource type is not its own IID**.

### 6.1 Sprite animation (`ReadAnim`)

All fields are little-endian `u32`:

1. A type tag, which must be `0x29A5D1EC`.
2. Five header values *[inferred: not interpreted here]*.
3. Five per-zoom table keys k₀–k₄ (instances of `0x09ADCD75` tables). k₀ and k₁ are read and replaced by 0. A key
   below the resource's own instance I counts as relative: k := k + I (so 0 means I).
4. One more value.

Then each zoom z gets a source zoom and a scale s_z, with the zoom sizes P = (8, 16, 32, 73, 146) (`0x00ABD668`):

- **Downwards:** zoom 4 is its own source (s₄ = 1). For z = 3, 2, 1, 0: a key equal to the current source's
  shares it, s_z = P_z ÷ P_source; a different key becomes the source, s_z = 1.
- **Upwards, only when k₄ = I:** start with no source. For z = 0 … 4, a key other than I becomes the source,
  leaving its downward scale unchanged; a key equal to I takes the last source's key and scale P_z ÷ P_source
  **only if a nonzero source has already been found**. Leading I keys retain their downward key and scale.
  The source is not inherited from the downward pass (`0x0080884E`–`0x008088D9`). All-I keys therefore retain
  the downward scales (8/146, 16/146, 32/146, 73/146, 1); they do not get reset to five unit scales (MI7).

The key comparison is unsigned and relative-key addition wraps at 32 bits (`0x00808731`–`0x0080873B`). Each
scale ratio is evaluated by x87 division then stored as binary32. A zero key cannot supply a downward source;
if the downward pass finishes without a nonzero source the reader returns false (`0x00808839`). In particular,
I=0 forces keys 0 and 1 to zero and fails at that gate, even if higher keys were nonzero. This is separate from
opaque MI1 fields, which this reader still only copies.

The sprite instance scales a frame by s_z when it differs from 1 (animated-sprite.md §4.1). Zooms 0 and 1 always end
up with the instance's own key or a shared one, but the sprite draws nothing there.

### 6.2 Sprite-animation table (`ReadTable`)

A type tag `0x09ADCD75`, then the rest of the resource as one block: valid when its size is 32 + 8 n, where n is
the 32-bit value at block offset `0x1C`. So a table is a 32-byte header holding the entry count n at `+0x1C`,
followed by n entries of 8 bytes (animated-sprite.md AS1 asks what the header and entries hold).

For well-formed tables, total stream size is 36+8n including the tag. Native validation subtracts 4 from the
stream size and compares the remaining byte count to **wrapping u32** (32+8n), without first checking a
32-byte minimum (`0x00808957`–`0x00808988`). Checked arithmetic and rejection of short headers or wrapped
counts in a portable reader are memory-safety choices, not evidence that the native parser rejected them.

## 7. Users

Every scene object (lots, buildings, props, networks, automata) holds model instances; the occupant and automata
code creates them through the model maker (model-maker.md). Automata use the processor (`SetModel` when a vehicle's
model changes, `LightModel` when it turns), and so do the dispatch view's 3D indicator, the tank and the
model-based particle effects (`SortMeshes`, `LightModel`); the effects model processor makes normals.

## 8. Quirks

1. A basic instance reports the identity transform and ignores a new one (§1).
2. Processed models get vertex normals shorter than 1 at creases (§4).
3. Only format-2 models are relit; others are left as they are (§4).
4. A sprite animation never has its own sheet at zooms 0 and 1 (§6.1).

## 9. Open questions

See [questions/model-instances.md](questions/model-instances.md) MI1–MI2: what the five header values and the last
value of a sprite animation hold; what `flip` in `LightModel` is used for.

## 10. Conformance tests

1. **Basic transform:** a lot model's `GetTransform` gives the identity even after `SetTransform`.
2. **Sort value:** a static instance whose first material's first texture is `0x12345678` sorts by `0x12345678`.
3. **Processor refusal:** `SetModel` with a two-frame animated instance is false.
4. **Normals:** two triangles of equal area at right angles sharing a vertex give it a normal of length
   1/√2 ≈ 0.707.
5. **Mesh order:** boxes centred at z = 1, 3, 2 with direction (0, 0, 1) sort as 3, 2, 1.
6. **Shared sheet:** keys k₂ = k₃ = k₄ = K ≠ I: zoom 3 uses K with scale 73 ÷ 146 = 0.5, zoom 2 with 32 ÷ 146.

The following independent vectors discriminate the MI3/MI10 rules; they are expected behaviours, not results
from a reimplementation:

7. **Defaults:** construct a basic instance: owner=null, custom=0, properties=2, count=0. A static instance has
   properties=2 too. Setting properties to `0x12345678` reads back that value on basic and `0x5678` on static.
8. **Draw bytes:** set a 20-byte record with byte 0=`0xA1`, byte 1=`0xB2`, bytes 2..3=`0xC3,0xD4` and distinct
   final words. Get returns all 20 bytes unchanged. A blend-pair SetModel changes byte 0 to `0xAF`, preserving
   byte 1 and bytes 2..19 except independently specified material changes.
9. **Counter source and wrap:** with instance counter 255 and source counter 17, SetTransform leaves the
   instance counter 0; a second set leaves 1. GetTransform into a destination whose counter is 40 yields 41,
   irrespective of the instance counter. MakeIdentity with counter 255 and byte `+1` nonzero yields counter 0;
   repeating it with byte `+1` zero makes no further changes.
10. **Count width:** on a basic allocation at count 65535, AddRef returns 0 and Release destroys once, returns 0.
    On a static allocation at count 65535, AddRef returns 65536 and Release returns 65535 without destruction.
    Release on a newly constructed zero-count basic or static allocation also destroys it. No access after
    destruction is part of this vector.
11. **Owner events:** replacing A by B gives `Release(A), AddRef(B)` on basic and `AddRef(B), Release(A)` on
    static; setting B again gives no calls. Clear then destroy produces only one Release(B).
12. **All-own sheets:** I=1000 and all five input keys 0 produce five keys 1000 with scales
    (8/146,16/146,32/146,0.5,1), rounded on binary32 storage. With inputs (arbitrary,arbitrary,2000,2000,0),
    output keys are (1000,1000,2000,2000,2000) and scales (8/16,1,32/73,1,146/73): the first two retain their
    downward results, zoom 2 keeps its downward scale when it becomes the upward source. I=0 fails.
13. **Sort restart and ties:** container centres A=(0,0,0), B=(1,0,0), C=(2,0,0). Direction +x gives C,B,A;
    direction +y next gives C,B,A again, since equal scores reverse original container order. Starting with
    direction −x instead gives A,B,C, but +y still gives C,B,A. Scores (2,NaN,1) in container order give C,B,A;
    scores (+0,−0,+0) do too. These distinguish restart, ties and unordered insertion separately.
14. **Processor return/lifetime:** a two-frame animated instance, a non-format-2 container, an empty container,
    a missing container, and null input each give false and no retained instance/container. A nonempty
    format-2 container gives true. Hold an external container reference: after successful conversion,
    ReleaseModel alone leaves format-10 vertices; subsequent destruction restores the exact original block
    identities without a second container Release. Destruction without ReleaseModel restores them too.
15. **Material isolation:** build two instances from a material with blend bytes (8,1), flag `0x80` and nonzero
    byte `+0x18`. SetModel on the first clears its flag `0x80` and byte `+0x18`, and ORs draw byte 0 with `0x0E`;
    source and second instance retain their flag and byte. Texture binding identities may remain shared.
