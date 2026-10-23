# Spec: animated sprite model instances (`cS3DModelInstanceAnimatedSprite`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable. The round-2/3 instruction source has SHA256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, matching the canonical Windows export
(image base `0x00400000`). This is a bounded contract publication; AS1/AS3 and the qualifications tracked in
the questions file remain open. Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [draw-context.md](draw-context.md) (every draw call, the packed state),
[view-utilities.md](view-utilities.md) (§7 picking, the metres per pixel), [camera.md](camera.md) (the camera's local
transform and view volume), [model-maker.md](model-maker.md) (which creates model instances),
[graphics-formats.md](graphics-formats.md).

## 0. Summary

An animated sprite is a model instance drawn as one camera-facing quad. Its picture comes from a sprite
animation: each zoom has orientation records and frames packed across texture pages (§4.1). A refreshing draw
selects an orientation record from the instance's +Z direction and context rotation, then packs the temporal
frame into the atlas. Quad geometry follows §5.1; cached geometry need not refresh with every frame or camera
change (§5.2).

## 1. The object *[compat]*

- Built by `0x007FDCB0` (the constructor entry; its body `0x007FDCBA`), from a pool, with the virtual base
  `cS3DModelInstanceBasic` (owner, custom data, bounds, transform; its slots are the `cS3DModelInstanceBasic::*`
  names). The `Pick` and `Draw` thunks at `+4` (`0x007FE230`, `0x007FEE90`) are what the scene calls.
- Its draw state (made by the constructor): colour and depth write, depth test less-or-equal, alpha test "greater"
  at 0.5, blending source alpha / one minus source alpha, one texture, mode 1.
- Destructor body `0x007FEEF0`: releases the per-zoom resources.

### 1.1 Defaults and lifetime

Offsets below are from the complete sprite object, not the adjusted shared-instance pointer.
Constructor entry `0x007FDCB0` establishes zero in EBX; body `0x007FDCBA–007FDFB1` uses it for defaults.

| State | Constructor value |
|---|---|
| owner/custom data (+0x10,+0x14); u16 reference count (+0x18) | null/0; **0**, not 1 |
| properties (+0x1C); flags/highlight (+0x20,+0x21) | 1; 0/0 |
| shade RGB (+0x24..+0x2C); fade alpha (+0x30) | (1,1,1); 1 |
| sequence controller (+0x34) | null |
| transform counter/flags (+0x38,+0x39) | 0/0 |
| rotation (+0x3C..+0x5C); position (+0x60..+0x68); scale (+0x6C) | identity; (0,0,0); 1 |
| local box min (+0x70..+0x78), max (+0x7C..+0x84) | (0,0,0), (1,1,1) |
| cached zoom/rotation (+0x88,+0x89) | 255/255 |
| last orientation-entry pointer (+0x98) | null |
| frame dirty/view dirty/cached transform counter (+0x9C..+0x9E) | 1/1/255 |
| frame (+0x9F); rate (+0xA0); clock (+0xA4) | 0; binary32 `0x3A83126F` (0.001); 0 |
| mode/direction/skip (+0xA8..+0xAA); frame count (+0xAC) | 3/+1/0; 0 |
| texture binding (+0xC4); five zoom holders (+0xC8..+0xD8) | null; allocated with null interface and entry pointers |
| frame metric record (+0x94) | allocated 20 bytes, all zero |
| draw vertices (+0xDC..+0x138) | zero position/UV, white `0xFFFFFFFF` colour |

The per-zoom scales and unused derived holder metrics are not initialized before loading; do not invent
safe loaded defaults for them. The constructor initially fills cached local corners (+0x13C..+0x168) from
global `0x00B4E65C`; no cached-corner validity is promised before UpdateView. Base defaults are verified at
`0x00801CF0–00801D67`, specifically `0x00801D2B–00801D5D`, and sprite overrides
at `0x007FDDA0–007FDF83`.

AddRef increments the **u16** count modulo 65536, returns its zero-extended value, and has no overflow check.
Release decrements only counts greater than one. Counts 0 and 1 both invoke deleting destruction and return 0
without first storing a decremented count (`0x00801900–00801932`). QueryInterface adds a reference on success;
unknown IID returns false without clearing the output. The sprite additionally accepts IID `0xA818EDD8`,
returning its sprite interface; IID 1 and `0x26A68449` return the shared instance interface
(`0x007FB130–007FB174`, `0x00801940–00801996`). No resource-table IID is accepted here.

Destruction releases each retained zoom interface, returns each holder to its pool, frees the frame-metric
record, releases the texture binding and runs base destruction. It does not delete the optional sequence
controller. Deleting destruction invokes the sprite pool free only if argument bit 0 is set and returns
the original object address (`0x007FEEF0–007FF003`).

Independent lifetime vector: count 65535 → AddRef returns/stores 0 → Release destroys and returns 0.
Count 2 → Release stores/returns 1 → Release destroys/returns 0.

## 2. Creation and loading *[compat]*

- `GetAnimatedModelFromResourceKey(key)` (`0x007FEDE0`) by the key's type: `0x29A5D1EC` →
  `GetMeshAnimFromResourceKey` (`0x007FED50`), which **builds one of these sprite instances** and has it load the
  key (slot 22, so type `0x29A5D1EC` is the sprite animation), returning it or null; `0x5AD0E817` →
  `GetSpriteAnimFromResourceKey` (`0x007FD7D0`), which fetches the resource through the resource manager. Other
  types give false. The two Mac-matched names look swapped against what the functions build *[inferred]*.
- Loading uses model-header IID `0x29A5D1EC` and per-zoom record IID `0x69B6F01C`. The latter is an
  **interface ID**, not a resource type; its serialized type is `0x09ADCD75` (model-instances §6).

### 2.1 Load transaction and ownership

`Load(key)` passes the supplied key unchanged to resource-manager slot 3, requested IID `0x29A5D1EC`,
output pointer and two zero options. Failed acquisition returns false. Success calls owner slot 21,
releases the acquired interface, and returns that call's boolean (`0x007FD0C0–007FD114`).

`LoadFromResource(null)` returns false unchanged. A non-null interface's slot 3 returns a borrowed header.
The owner copies u32 header +0x3C to frame count +0xAC **before any acquisition**. It builds a key with
type=header +0x0C, group=header +0x10 and, in order i=0..4, instance=header[0x14+4i]. Before each request
it copies float header[0x28+4i] to scale[0xB0+4i]. These are five array slots, with no zoom subtraction.
Each resource-manager slot-3 request uses that key, IID `0x69B6F01C`, output interface and two zero options.
First failure stops further requests.

For each success, the holder releases its previous interface, clears interface/data pointers, adds a reference
to the new interface, stores it and its slot-3 borrowed data pointer plus 0x20, and derives §4.1 metrics.
Load releases the temporary acquired reference. The model-header input itself is borrowed, without AddRef
or Release by LoadFromResource (`0x007FD120–007FD1F1`, `0x007FC320`).

Five successful zooms are insufficient: it acquires framework service `0x1AC0E11A`, IID `0xFAC0E219`, then
calls service slot 6 with a key copied from header +0,+4,+8 and output texture binding. Only a non-null
binding gives true. Before testing it, the function releases the old binding at +0xC4 and installs the result,
including null. If service is absent or fewer than five zooms succeeded, it leaves the old binding untouched.
The temporary service reference is released on exit (`0x007FD1F3–007FD2A5`).

Failure rolls back **only this call's successful prefix**, in reverse order, releasing and nulling those
holders' interface/entry pointers. It returns false even after four successful zooms. Failed/unvisited slots
retain previous resources; overwritten old prefix resources are not recovered. Frame count, copied scales
and derived metrics are not rolled back. No frame/clock/dirty/last-entry reset or sequence-controller
construction occurs here. Repeated loads are not atomic replacements (`0x007FD2A5–007FD2E9`).

Independent vector: starting with A0..A4 and binding T, acquire B0,B1 then fail at i=2. Return false;
interfaces become [null,null,A2,A3,A4]; T remains; new frame count and scales 0..2 remain, scales 3..4 retain
old values. A fresh instance failing at i=0 remains empty and returns false.

## 3. The interface (vtable `0x00ABD220`) *[compat]*

| Slot | VA | Call | Behaviour |
|---|---|---|---|
| 1 | `0x007FC480` | `SetPosition(p)` | the translation; the transform changes |
| 2 | `0x007FE250` | `LookAt(target, up)` | §3.1 |
| 3 | `0x007FC4B0` | `SetOrientation(yaw, pitch, roll)` | §3.1 |
| 4–7 | `0x007FCD30`, `0x007FCC90`, `0x007FCE20`, `0x007FCDC0` | point and vector transforms | by the instance's transform and its inverse |
| 8 | `0x007FB180` | `SetView(zoom, rotation)` | two bytes; the frame is due again when they change |
| 9 | `0x007FCEA0` | `SetAnimation(index)` | picks a sequence of the animation table (its first frame and length) |
| 10 | `0x007FCF00` | `SetFrame(n)` | conditional remainder and byte narrowing; §4.3 |
| 11, 12 | `0x007FB1C0`, `0x007FB1D0` | `GetFrame`, `GetFrameCount` | |
| 13 | `0x007FCF60` | `AdvanceFrame()` | §4 |
| 14 | `0x007FB310` | `SetRate(r)` | |
| 15 | `0x007FCFB0` | `Tick(elapsed)` | §4 |
| 16 | `0x007FB330` | `SetPlayMode(m)` | §4 |
| 18 | `0x007FB340` | `SetFade(on, alpha)` | sentinel/NaN/disable rules in §4.4 |
| 19 | `0x007FB3B0` | `SetHighlight(index)` | flag 1 and the highlight index; 0 clears |
| 20 | `0x007FB410` | `SetSelfLit(on)` | flag 4 |
| 21 | `0x007FD120` | `LoadFromResource(resource)` | §2 |
| 22 | `0x007FD0C0` | `Load(key)` | the resource of type `0x29A5D1EC` for the key, then slot 21 |

Slot 0 (`0x007FAF40`) converts the sprite pointer to the shared instance interface using its virtual-base
offset; null stays null. It is not QueryInterface. Slot 17 returns 0.

### 3.1 Placement

The instance keeps a transform (rotation, translation, a change counter); `0x007FE0F0` and `0x007FE190` copy it
out and in. Its row-major matrix maps column vectors. The argument labels yaw/pitch/roll do not imply the
usual vehicle-heading convention: argument 0 rotates about **X**, argument 1 about **Y**, argument 2 about
**Z**, with right-handed signs. In real arithmetic define the rows:

| Rotation | Row 0 | Row 1 | Row 2 |
|---|---|---|---|
| A(a), argument 0 | (1,0,0) | (0,cos(a),−sin(a)) | (0,sin(a),cos(a)) |
| B(p), argument 1 | (cos(p),0,sin(p)) | (0,1,0) | (−sin(p),0,cos(p)) |
| D(r), argument 2 | (cos(r),−sin(r),0) | (sin(r),cos(r),0) | (0,0,1) |

`SetOrientation` compares each input's **bits** with binary32 −100 (`0xC2C80000`), forming mask bits 1,2,4.
Mask 0 is a complete no-op, including flags/counter, returning 0. Mask 7 overwrites the matrix with
`A(a) B(p) D(r)`, ignoring the old matrix. Thus a point experiences Z, then Y, then X rotations.
For masks 1–6, if transform flag 4 was set, first replace the matrix with identity, clear flag 4 and increment
the byte counter. Otherwise use the stored matrix as it stands. Then right-multiply by the present rotations
in argument order (A, then B, then D), skipping −100 arguments. Each applied rotation sets flag 4 and
increments the byte counter. Translation and scale are untouched; frame/view dirty bytes are untouched.
Every path returns 0. In particular −100 does **not** preserve a previous Euler component, and passing zero
counts as applying a rotation. Evidence: mask dispatch `0x007FC4B0–007FC4F2`; A `0x007FC4F9–007FC611`;
A/B `0x007FC649–007FC84D`; A/D `0x007FC885–007FC994`; B/D `0x007FC999–007FCAA7`;
D `0x007FCAAD–007FCB9E`; full mask `0x007FCBA3–007FCC67` (single-axis B/D share tails).

For full mask, put ca=cos(a), sa=sin(a), cp=cos(p), sp=sin(p), cr=cos(r), sr=sin(r). The rows are
`(cr*cp,−sr*cp,sp)`, `(sr*ca+(sp*sa)*cr,cr*ca−(sp*sa)*sr,−cp*sa)`,
`(sr*sa−(sp*ca)*cr,cr*sa+(sp*ca)*sr,cp*ca)`. Here ca,sa,cp are explicitly stored to binary32;
sp,cr,sr and the shared products sp*ca,sp*sa remain x87 values before their uses. Matrix results store as
binary32 (`0x007FCBA3–007FCC4D`). The partial-mask paths use their own staged products/stores and are not
last-bit equivalent to evaluating this full-mask expression with omitted angles set to zero. For example A
stores cos(a) to binary32, retains sin(a), stores the products for matrix rows 1/2 and later stores −sin(a)
to binary32 before the other column update (`0x007FC525–007FC60E`). Numeric execution inherits PC/RC;
these algebraic products are not permission to choose host trigonometry or a universal floating-point mode.

#### 3.1.1 Finite instruction schedule: notation and mask 1

This schedule specifies the normal, completing instruction path, with finite intermediates and no
unmasked floating exception. Incoming x87 precision control **PC** (24, 53 or 64 significant bits) and
rounding control **RC** are parameters, not inferred defaults. Write `P(x,y)`, `U(x,y)` and `V(x,y)` for
one x87 multiply, add and subtract respectively, rounded under the current PC/RC; preserve their written
nesting. `N(x)` changes the sign without arithmetic rounding. `B(x)` is an actual binary32 store under
RC followed by an exact reload when used later. Each named temporary stays an x87 transient unless
explicitly marked `B`. Matrix writes are B stores; integer copies of already stored words do not round.
These rules do not authorize fused operations, reassociation or replacing all intermediates with doubles.
An ordinary completing environment with unchanged PC/RC suffices for the examples; the local body does
not establish whole-call numeric-mode provenance.

For masks 1–6 the trigonometric primitives are **hardware x87 FCOS then FSIN**, each applied to an exact
load of the same binary32 angle. There are no foreign sine/cosine calls, double argument conversions or
foreign return formats in this body. Let `C,S` denote the actual hardware primitive transients, without
assuming that they equal correctly rounded host sin/cos or first rounding them to binary32. In each
partial stage cosine immediately becomes `c=B(C)`; sine initially remains `s=S`. Finite huge arguments
outside hardware range, nonfinite values, exception behavior and hardware approximation accuracy are
outside this normal-domain schedule; no CRT implementation is substituted. Evidence: `0x007FC525–007FC544`,
`0x007FC675–007FC694`, `0x007FC770–007FC789`, `0x007FC8B1–007FC8D0`,
`0x007FC9C5–007FC9DE`, `0x007FCAAD–007FCACD`.

Matrix rows occupy offsets `(3C,40,44)`, `(48,4C,50)`, `(54,58,5C)` from this method's incoming this pointer.
For a stage use snapshots `u=(u0,u1,u2)` and `v=(v0,v1,v2)` of its two affected **columns**, after any
identity reset and after preceding stages. They are old binary32 values, not newly assigned cells.
Mask 1 uses columns 1 and 2. The following numbered arithmetic groups execute in order; within each
group the entries execute left to right. All operations not wrapped in `B` remain transient:

| Group | Mask-1 A stage, `0x007FC525–007FC60E` |
|---|---|
| 1 | `t0=P(v0,s)`; `t1=B(P(v1,s))`; `t2=B(P(v2,s))` |
| 2 | `q0=P(u0,c)`; `q1=B(P(u1,c))`; `q2=B(P(u2,c))` |
| 3 | `r0=U(t0,q0)`; `r1=U(q1,t1)`; `r2=B(U(q2,t2))` |
| 4 | Copy r2 to m21, then store B(r0) to m01, then B(r1) to m11 |
| 5 | `p0=B(P(v0,c))`; `p1=B(P(v1,c))`; `p2=B(P(v2,c))`; `n=B(N(s))` |
| 6 | `h0=P(u0,n)`; `h1=B(P(u1,n))`; `h2=B(P(u2,n))` |
| 7 | `w0=U(h0,p0)`; `w1=U(h1,p1)`; `w2=B(U(h2,p2))` |
| 8 | Store B(w0) to m02, B(w1) to m12, then copy w2 to m22 |

Column 0 is untouched by the A stage. Original u0,v0,v1 stay on the x87 stack across the column-1
writes; u1,u2,v2 were snapshotted as raw words before those writes. Thus the column-2 update uses the
**old** column 1 even though the owner already contains the new column 1. The negative sine is newly
stored to binary32; it is not the unrounded negative of the sine used in group 1. Mask 1 first performs
the flag-4-dependent identity reset described above (`0x007FC4F9–007FC522`), then this A stage, then the
common end: OR transform flag 4, increment the byte counter once and return 0 (`0x007FCC50–007FCC69`).
With initial counter 255, final counter is 0 without reset or 1 with reset. All unrelated flag bits,
translation, scale and both frame/view dirty bytes are preserved. No foreign callback occurs on this path.

#### 3.1.2 Remaining partial stages and finite control flow

**A followed by B or D.** The A stages in masks 3 and 5 have the same eight arithmetic groups and B
boundaries as mask 1. In groups 1,2,5,6 respectively, the row-1/2 products are written
`P(s,v1),P(s,v2)`; `P(c,u1),P(c,u2)`; `P(c,v1),P(c,v2)`; `P(n,u1),P(n,u2)`.
These reverse the multiplicand order of the mask-1 table, with no change of finite product, operation
sequence or rounding boundary. Group 4 is unchanged. In group 8 mask 3 copies w2 to m22 **before** storing
B(w0),B(w1); mask 5 stores B(w0),B(w1) and then copies w2. Neither uses the newly written column 1 as
column-2 input. Evidence: mask-3 A `0x007FC675–007FC762`; mask-5 A `0x007FC8B1–007FC994`,
whose final stores share `0x007FCA95–007FCA9F`.

**B stage.** Snapshot columns 0 and 2 into u,v. Column 1 is untouched. Compute `c=B(C), s=S` by
FCOS/FSIN of argument 1, then execute these groups. Sine is **never** stored alone in the B stage.

| Group | B arithmetic, shared by masks 2/3; separately emitted with the same boundaries in mask 6 |
|---|---|
| 1 | `t0=P(v0,s)`; `t1=B(P(s,v1))`; `t2=B(P(s,v2))` |
| 2 | `q0=P(u0,c)`; `q1=B(P(c,u1))`; `q2=B(P(c,u2))` |
| 3 | `r0=V(q0,t0)`; `r1=V(q1,t1)`; `r2=B(V(q2,t2))` |
| 4 | Masks 2/3: store B(r0) to m00, B(r1) to m10, then copy r2 to m20; mask 6: copy r2 first, then those two stores |
| 5 | `p0=B(P(v0,c))`; `p1=B(P(c,v1))`; `p2=B(P(c,v2))` |
| 6 | `h0=B(P(u0,s))`; `h1=B(P(s,u1))`; `h2=P(s,u2)` |
| 7 | `w0=U(h0,p0)`; `w1=U(h1,p1)`; `w2=B(U(h2,p2))` |
| 8 | Store B(w0) to m02, B(w1) to m12, then copy w2 to m22 |

The row-0 products for the first updated column remain transient; for the second updated column it is
the **row-2 sine product** h2 that stays transient. Do not apply the A-stage spill pattern to B.
u0,u1,v0,v1 remain on the x87 stack across group 4; u2,v2 are saved raw words. Sources therefore survive
in-place column-0 replacement. The groups are proved at `0x007FC770–007FC84A` (masks 2/3) and
`0x007FC9C5–007FCA9F` (mask 6).

**D stage.** Snapshot columns 0 and 1 into u,v; column 2 is untouched. Compute `c=B(C), s=S` from
argument 2. Use the A-stage eight arithmetic groups with the mask-3/5 multiplicand ordering above,
including the later `n=B(N(s))`. Group 4 instead stores B(r0) to m00, B(r1) to m10, then copies r2 to m20.
Group 8 stores B(w0) to m01, B(w1) to m11, then copies w2 to m21. The surviving old sources are
u0,v0,v1 as x87 values and u1,u2,v2 as raw words. This one D body serves masks 4/5/6,
`0x007FCAAD–007FCB9B`. In particular a B/D call sees the **stored** B results, not retained B-stage products.

The complete finite dispatch is below. Reset R means write identity in this order:
`m10=+0, m20=+0, m00=1, m01=+0, m21=+0, m11=1, m02=+0, m12=+0, m22=1`, then clear only
transform flag 4, then increment counter +0x38 modulo 256. Every completed partial rotation ORs flag 4
and increments the counter once, in that order. At the final common end these two values are read,
then the new flag byte and counter byte are written (`0x007FCC50–007FCC5D`). A composite's first-stage
flag/counter writes occur **before** the next angle load. No extra increment is added after its final rotation.

| Mask (present arguments) | Dispatch entry | Initial flag 4 clear | Initial flag 4 set | Counter from 255: clear / set |
|---|---|---|---|---|
| 0 (none) | direct exit `007FCC60` | no writes | no writes | 255 / 255 |
| 1 (a) | `007FC4F9` | A(old) | R; A(identity) | 0 / 1 |
| 2 (p) | `007FC616` | B(old) | R; B(identity) | 0 / 1 |
| 3 (a,p) | `007FC649` | A(old); B(stored A) | R; A(identity); B(stored A) | 1 / 2 |
| 4 (r) | `007FC852` | D(old) | R; D(identity) | 0 / 1 |
| 5 (a,r) | `007FC885` | A(old); D(stored A) | R; A(identity); D(stored A) | 1 / 2 |
| 6 (p,r) | `007FC999` | B(old); D(stored B) | R; B(identity); D(stored B) | 1 / 2 |
| 7 (a,p,r) | `007FCBA3` | full replacement | full replacement, no R | 0 / 0 |

The dispatch table is at `0x007FCC6C`, selected by mask minus one (`0x007FC4E8–007FC4F2`). Reset sites
are `0x007FC4FE–007FC522`, `0x007FC61F–007FC644` with tail `0x007FC76A–007FC76D`,
`0x007FC64E–007FC672`, `0x007FC85B–007FC880` with tail `0x007FCAA7–007FCAAA`,
`0x007FC88A–007FC8AE`, and `0x007FC99E–007FC9C2`. Composite first-stage ends are
`0x007FC765–007FC76D` and `0x007FCAA2–007FCAAA`.

Masks 1–7 finally preserve every flag except setting transform flag 4; sprite flags at +0x20 are a
different byte. Position (+0x60..68), scale (+0x6C), frame dirty (+0x9C), view dirty (+0x9D) and cached
counter (+0x9E) are **untouched**, including on reset. Mask 0, with all three argument words exactly
`C2C80000`, preserves the entire instance bit for bit: no identity reset, trig, matrix, flag or counter write.
All eight masks return integer 0. No status is inferred from the number of rotations.

Argument bits for dispatch are sampled at entry, but the angles are loaded at their respective stage
starts, after reset and any preceding matrix writes. The function does not promise an entry-time angle
snapshot if the input vector aliases the matrix. The snapshots above are of each stage's **matrix**
sources; analytic vectors below use disjoint input storage.

#### 3.1.3 Full-mask comparison schedule

Mask 7 loads argument 1, copies argument 2's raw word, then loads argument 0, all before matrix writes.
FCOS(a), FSIN(a), FCOS(p), FSIN(p), FCOS(r), FSIN(r) execute in that order. The first three results are
stored as `ca=B(Ca), sa=B(Sa), cp=B(Cp)`; `sp=Sp, cr=Cr, sr=Sr` stay transient. No foreign parameter
conversion or return boundary occurs (`0x007FCBA3–007FCBD1`). First compute `e=P(sp,ca)`, then
`f=P(sp,sa)`, both transient. The following rows execute top to bottom; operations within a row execute
left to right, and each final result is a binary32 matrix store:

| Destination | Ordered calculation before its B store |
|---|---|
| m00 | `P(cr,cp)` |
| m01 | `N(P(sr,cp))` |
| m02 | `sp` (the store does not replace the transient in e/f) |
| m10 | `j=P(f,cr)`; `k=P(sr,ca)`; `U(j,k)` |
| m11 | `j=P(cr,ca)`; `k=P(f,sr)`; `V(j,k)` |
| m12 | `N(P(cp,sa))` |
| m20 | `j=P(sr,sa)`; `k=P(e,cr)`; `V(j,k)` |
| m21 | `j=P(e,sr)`; `k=P(cr,sa)`; `U(j,k)` |
| m22 | `P(cp,ca)` |

Evidence: `0x007FCBD3–007FCC4D`. There are no B stores of e,f,j,k. This path never reads the old matrix
and never clears flag 4. It uses the common end exactly once. Full replacement is not a shortcut for
any partial-mask schedule, even when the omitted rotations are geometrically identity.

`LookAt(target,up)` constructs back=normalize(position−target), right=normalize(up×back), trueUp=back×right;
the matrix columns are right,trueUp,back. Thus local **−Z** points toward the first argument; it is a target
point, not a direction or source. It preserves position/scale, ORs flag 4 and increments the byte counter by
**2**; return 2 (`0x007FE250–007FE38D`). Differences and normalization intermediates have binary32 stores;
normalization uses reciprocal square root length via x87 FSQRT and division (`0x007FE257–007FE2AA`,
`0x007FE2AE–007FE310`). There is no zero-length/parallel-vector fallback. No portable NaN payload is promised.
Frame selection reads the matrix's **+Z column** (the back vector after LookAt), not local −Z.

**R2-R1, proposed UNEXECUTED noncommuting vector.** Full arguments (π/2,π/2,0), encoded as
`(0x3FC90FDB,0x3FC90FDB,0)`, have ideal rows `(0,0,1),(1,0,0),(0,1,0)`.
Reversing A/B gives rows `(0,1,0),(0,0,−1),(−1,0,0)` and is detectably different. Full arguments reset any
previous matrix and increment the counter once. Ideal values are geometric expectations with component
tolerance 0.000002 for these supplied angles under ordinary finite x87 precision/rounding, not exact-bit results.

**R2-R2, proposed UNEXECUTED noncommuting vector.** Full arguments (π/6,π/3,π/4), binary32
`(0x3F060A92,0x3F860A92,0x3F490FDB)`, have ideal rows
`(√2/4,−√2/4,√3/2)`, `(3√6/8,√6/8,−1/4)`, `(−√2/8,5√2/8,√3/4)`.
These follow by multiplying the three matrices above using sin(π/6)=1/2, cos(π/3)=1/2,
cos(π/6)=sin(π/3)=√3/2, sin(π/4)=cos(π/4)=√2/2. Independent constants are
√2≈1.4142135623730950488, √3≈1.7320508075688772935, √6≈2.4494897427831780982.
Use the same geometric tolerance as R1; a host trig calculation is not the oracle.

**R2-R3, proposed UNEXECUTED sentinel mixture.** After R1 with flag 4 set and counter c, call
`(−100,0,π/2)`. Mask 6 resets identity and composes B(0)D(π/2); ideal rows
`(0,−1,0),(1,0,0),(0,0,1)`, counter c+3 modulo 256. It does not retain R1's X rotation.
Then `(−100,−100,−100)` preserves all nine matrix words, flags and counter exactly. From position
(10,20,30), `LookAt((10,20,29),(0,1,0))` instead yields identity and counter +2; the target is one unit
along local −Z. All of these tests are proposed, with no native or implementation execution.

`SetPosition` copies three float bit patterns, ORs transform flag 1, increments the u8 change counter even
for equal input, and returns 1 (`0x007FC480–007FC4A7`). `SetTransform` increments the destination owner's
counter modulo 256, copies thirteen dwords (rotation, translation, scale) and the input flags byte, and
does not copy the source counter or change frame/view dirty bytes. `GetTransform` increments the **output
record's** counter, copies those thirteen dwords and flags, and leaves the owner's counter unchanged.
Neither copy method defines a status return (`0x007FE0F0–007FE22A`). With owner counter 255, importing
any transform produces counter 0, independently of its source counter.

`LookAt(target, up)` begins with normalized (position−target), so local +Z points **away from** the target.
It obtains local +X by normalizing up × +Z and +Y by +Z × +X. Position and scale stay unchanged; it ORs flag 4,
adds 2 to the counter modulo 256 and returns 2. It does not check zero distance/parallel up before division.
Evidence: `0x007FE250–007FE38D`. Example: position (0,0,0), target (0,0,−1), up (0,1,0) gives identity.
The noncommuting geometric order and bitwise −100 omission/reset rule are proved above, including the
finite partial-mask store schedules in §§3.1.1–3.1.3. AS6 remains partial for whole-call PC/RC provenance,
native numeric qualification and the AS1 controller boundary; those gaps do not reopen the geometric rule.

### 3.2 Shared instance surface and overrides

The shared instance vtable at `0x00ABD1D0` has these 19 slots, separate from the sprite's 23-slot interface:

| Slots | Entry addresses | Contract/body |
|---|---|---|
| 0,1,2 | `0x007FDFD0`, `0x007FDFE0`, `0x007FDFF0` | QueryInterface/AddRef/Release via `0x007FB130`, `0x007FB110`, `0x007FB120` |
| 3,4,5,6 | `0x007FE000`, `0x007FE010`, `0x007FE020`, `0x007FE030` | Basic owner/custom-data get/set |
| 7,8 | `0x007FE090`, `0x007FE0E0` | local box; world box via `0x007FD680` |
| 9,10 | `0x007FE0F0`, `0x007FE190` | transform copies (§3.1) |
| 11,12 | `0x007FE230`, `0x007FEE90` | Pick/Draw thunks to `0x007FD710`, `0x007FEA90` |
| 13,14,15,16 | `0x007FE040`, `0x007FE050`, `0x007FE060`, `0x007FE070` | Basic properties/draw-attributes get/set |
| 17,18 | `0x007FE240`, `0x007FE080` | sort value via `0x007FD640`; null resource model |

The shared-interface thunks subtract the displacement at [this−4]; they do **not** unconditionally subtract
4. Basic forwarding additionally subtracts 0x138; the deleting-destructor thunk `0x007FDFC0` instead
subtracts 8. Several exported functions combine an adjustment entry and an internal body as disjoint ranges.
Do not count each internal jump label as a separate native function (`0x007FE090`, `0x007FE0F0`, `0x007FE190`).

`GetBounds(out)` copies local min/max without loading, drawing, recomputation or transformation, and defines
no status. `GetWorldBounds(out)` adds position independently to each local min/max float; it does not apply
rotation or scale or mutate the box. Before any draw the box therefore remains constructor [0,1]^3 shifted
by position (`0x007FE0A0–007FE0DB`, `0x007FD680–007FD701`). Position (10,20,30) gives world min (10,20,30),
max (11,21,31), even after importing a different rotation/scale without UpdateView.

`GetSortValue(out)` writes 0 if binding or metric-record pointer is null; otherwise it writes the dword at
binding.table[metrics.page], with no page bounds check or reference acquisition. It is a texture-table value,
not a depth-distance formula, and defines no status (`0x007FD640–007FD670`). `GetResModel()` returns null via
the shared zero body `0x004E13E0`. These getters do not refresh cached frame selection.

Additional owner helpers beyond the two interfaces: NextFrame `0x007FB1E0`, SetZoomResource `0x007FC320`,
GetTextureResourceData `0x007FD2F0`, metric decoder `0x007FD3D0`, selection/cache wrapper `0x007FE390`,
UpdateView `0x007FE400`, constructor entry/body `0x007FDCB0`/`0x007FDCBA`, destructor body/deleting entry
`0x007FEEF0`/`0x007FEFE0`. GetFrameCount uses shared body `0x007FB1D0` reading +0xAC despite its unrelated
export alias. This inventory identifies the verified surfaces; reconciliation to the brief's exact
60-function/10482-byte owner accounting remains AS3, rather than a complete-owner claim.

### 3.3 Bounded round-2 evidence inventory

The following sizes are export function extents, not implementation credit or a claim that every byte is
algorithmic. They distinguish newly traced owner bodies from adjustment thunks. The five bodies total
4,849 export bytes; the two thunks total 16. These are overlapping evidence for the existing owner, not an
additional owner or a reconciliation of the requested 60/10,482 inventory.

| Entry | Export bytes | Evidence category |
|---|---:|---|
| SetOrientation `0x007FC4B0` | 1978 | owner body, algebra/masks traced; partial-path last-bit qualification remains |
| LookAt `0x007FE250` | 318 | owner body, target/basis/counter traced |
| UpdateView `0x007FE400` | 1669 | owner body, finite geometry/box traced |
| Draw `0x007FEA90` | 701 | owner body, cache and outgoing calls traced |
| Pick `0x007FD710` | 183 | owner body, cached triangle calls traced; helper semantics remain view-utilities §7 |
| Pick thunk `0x007FE230` | 8 | pointer adjustment only |
| Draw thunk `0x007FEE90` | 8 | pointer adjustment only |

GetViewVolume `0x007FF340` (127 bytes) is camera support for the fallback equation, not sprite-owner bytes.
SetModelTransform `0x007D2D20` is a foreign draw-context call; its entry/camera-copy path and published
contract were consulted, not its full 553-byte algorithm. Frame/record decoding retained from round 1 is
separate evidence (§4.1); neither it nor unknown owner inventory is obtained by subtracting these sizes
from a total. AS2 usage/resource-name attribution remains excluded.

## 4. Animation and frames *[compat]*

- The clock (`Tick`, slot 15): a rate whose bits are `0xC2C80000` (−100) calls slot 14 with −200.
  The native sprite's `SetRate` simply stores that float and returns 2: **there is no resource-rate lookup**.
  A rate of 0 does nothing. Otherwise position := position +
  elapsed × rate × direction. When |position| reaches 1: below 2, one frame is advanced (slot 13) and the direction
  subtracted; at 2 or more, frames are advanced until |position| < 1 when `+0xAA` is set *[inferred: frame
  skipping]*, else one frame is advanced and the position set to exactly 1 *[quirk]*. Subtraction uses the
  direction **after** advancing/reversal. Return 2 reports taking an advance path, including an unchanged
  frame; §4.2 gives the non-progress and return boundaries.
- Advancing (`AdvanceFrame`, slot 13): with an animation table, the table's sequence steps (`0x007FAE90`); without
  one (`0x007FB1E0`), with more than one frame, the next frame; at the end, by the play mode (`+0xA8`): 3
  loops to frame 0, 2 reverses (ping-pong, direction −1), anything else stops (direction 0, position 0). Each change
  marks the frame dirty (`+0x9C`).

### 4.2 Exact stepping and returns

`SetRate` preserves the supplied 32 bits, with return 2 (`0x007FB310–007FB321`). `SetPlayMode` stores only
the low byte, without touching direction, clock or the separate sequence controller; its return register is
not a defined status (`0x007FB330–007FB33C`). Frame and owner direction are unsigned and signed bytes
respectively. `AdvanceFrame` always returns 2, including no change and stopping (`0x007FCF60–007FCFA6`).
Without a sequence controller, counts at most one leave everything unchanged. For larger counts:

| Boundary/state | Mode 3 | Mode 2 | Any other mode |
|---|---|---|---|
| Forward at last frame | frame 0 | previous frame, direction −1 | retain frame, direction 0, clock 0 |
| Reverse at frame 0 | last frame | frame 1, direction +1 | retain frame, direction 0, clock 0 |
| Direction 0 | retain frame | retain frame | retain frame |

All rows for counts greater than one mark the frame dirty, including a stopped direction. Interior steps move
one frame according to the sign of direction. Evidence: `0x007FB1E0–007FB306`.

`Tick` evaluates elapsed × stored rate × signed direction + stored clock with x87 arithmetic, storing the
clock as binary32. The first threshold tests use the still-held x87 result, rather than reloading that store.
After every advance it reads the **new** signed direction and subtracts it from the current stored clock.
It returns 0 for zero rate or magnitude below 1; it returns 2 after an advance, even if no frame changed.
For magnitude at least 2, a zero skip byte advances once and overwrites the clock with +1. A nonzero skip byte
repeats advance then subtraction until magnitude is below 1. There is no progress/iteration guard: direction 0
with a large clock can loop forever, as can a negative rate that drives the clock opposite the subtraction.
Single-frame playback is not specially guarded by Tick; it terminates only when those subtractions converge.
Evidence: `0x007FCFB3–007FD0B7`; threshold data `0x00A81228=1`, `0x00A8825C=2`.

Independent examples, with no sequence controller: four frames, frame 3, direction +1, mode 2, clock 0,
rate 1, elapsed 1.25 yields frame 2, direction −1, clock **2.25**, return 2. Four frames, frame 0,
direction −1, mode 3, clock 0, rate 1, elapsed 1.25 yields frame 3, clock −0.25, return 2.
One frame, direction +1, skip enabled, rate 1, elapsed 2.5 keeps frame 0 and leaves clock 0.5, return 2.

### 4.3 Sequence-controller boundary contracts

The optional owner pointer `+0x34` is a mutable controller, not a decoded sprite frame record. Its fields are:
u16 selected sequence at +0, u16 sequence count at +2, u16 selected length at +4, pointer to u16 starts at +8,
u16 absolute current frame at +12, mode byte at +14 and signed direction byte at +15. Starts include a terminal
entry, since selecting i calculates the low 16 bits of starts[i+1] − starts[i].

`SetAnimation(i)` returns 0 if absent or signed i is at least the count. It does not reject negative i before
indexing: such calls are outside a safe decoded-record contract. An accepted call changes selection/length
but **does not reset** controller current frame, direction or clock. It copies the low byte of current frame
to the owner and dirties only if the full u16 differs from the old owner byte; return 2 (`0x007FCEA0`).

`SetFrame(n)` without a controller stores low byte n (no modulo), dirtying if the full argument differs from
the current byte. With a controller it uses signed remainder by selected length **only when n > length**;
equality is not wrapped. It adds that value to starts[selected] with u16 wrapping, stores controller current,
then compares/copies to the owner byte; return 2 (`0x007FCF00–007FCF53`). A zero length and positive n faults
at division; negative n is not normalized. Example: starts [10,13], selected 0, length 3: n=3 gives absolute
frame 13, n=4 gives 11, n=−1 gives 9.

Controller advance uses its own mode/direction, independently of the owner's. Length at most one returns true
unchanged; otherwise it steps inside [start,start+length), loops/reverses/stops using the same boundary modes
as the table above. Stopping only clears controller direction, not owner clock/direction. `AdvanceFrame`
copies its u16 current frame to the owner as above. Evidence: `0x007FAE90–007FAF34`, `0x007FCF60`.

### 4.4 Fade, highlight and self-light contracts

These setters preserve unrelated bits of the flags byte and do not dirty the frame cache.

`SetFade(false, alpha)` always clears bit 2 and sets stored alpha to 1. With on=true: exactly −100 leaves
both fields untouched; exactly −200 clears bit 2 and restores alpha 1; otherwise alpha at most the binary32
constant `0x3F7D70A4` (approximately 0.99) enables bit 2 and stores the supplied bits. Larger alpha disables
and restores 1. There is **no lower bound**, and the native unordered comparison also accepts NaN. Return is
always 2 (`0x007FB340–007FB3A8`; constants `0x00A98F64`, `0x00ABD0DC`, `0x00ABD0E0`). Thus alpha 1 disables,
alpha −1 enables, and disabling with alpha −100 still disables. This supersedes a guessed [0,1] interval.

`SetHighlight(index)` compares the full integer with the old unsigned index byte and compares (index!=0)
with bit 1; if both agree it returns 0. Otherwise it sets/clears bit 1 from the full integer's nonzeroness,
stores its low byte and returns 2 (`0x007FB3B0–007FB401`). Example: 256 enables highlight but stores index 0;
repeating 256 still returns 2. `SetSelfLit(on)` compares the input byte against bit 4 normalized to 0/1,
returns 0 when equal, otherwise toggles bit 4 according to nonzeroness and returns 2 (`0x007FB410–007FB44B`).

### 4.1 Frame lookup (`GetTextureResourceData` `0x007FD2F0`, `GetTextureData` `0x007FD3D0`)

The borrowed record has u32 divisors at +0,+4,+8; floats at +0x0C,+0x10,+0x14,+0x18; eight-byte entries
start at +0x20. Let K be binary32 `0x40C90FDB` (2π). Holder +8,+0x10,+0x18 store K divided by the three
unsigned divisors, rounded to binary32. Holder +0x0C,+0x14 copy record +0x0C,+0x14. It derives
P=trunc((record[+0x10]−record[+0x0C])/holder[+0x10]+binary32(0.1))+1 and
R=trunc((record[+0x18]−record[+0x14])/holder[+0x18]+binary32(0.1))+1.
Holder +0x20=P, +0x24=R, +0x28=low32(P×R), +0x1C=divisor0. There is no rate/frame-count read here
(`0x007FC36D–007FC438`). This describes consumer layout, not a file reader.

Selection takes float angles a,b, an unused third argument, zoom, rotation and output pointer. Define B(x):
store binary32(x+12582912), reinterpret bits as u32, add `0xB4C00000` modulo 2^32. This is a bit conversion,
**not** truncation or ties-away rounding; in its intended range and nearest-even mode it rounds ties to even.
π=`0x40490FDB`, reciprocal2π=`0x3E22F983`, quarter-turn=`0x3FC90FDB`.

- p = unsigned remainder B((((a+π)−holder.base)×unsigned(P))×reciprocal2π) by P.
- b'=b−signed(rotation)×quarter-turn; if ordered b'<0 add K once. Set
  y=B((unsigned(divisor0)×reciprocal2π)×b'); if y>=divisor0 take unsigned remainder by divisor0.
- Entry address = entryBase + 8×(p×R+y×(P×R)), with native 32-bit integer products/sum.

Temporal frame does **not** select this entry: the decoder below packs it across pages. Draw supplies
a=asin(back.y), b=atan2(back.x,back.z), where back is the owner's +Z matrix column,
rotation=zero-extended low byte of context view rotation on the Draw path (`0x007FEB0D–007FEB62`).
The selector itself accepts a signed integer rotation argument. The conditional single addition of K is to the
camera-relative **yaw** b', not to a negative pitch a; the pitch-like input instead uses a+π−holder.base.
Active x87 precision/rounding affects transients;
do not round every operation. An absent holder pointer writes null; an allocated unloaded holder is not
guarded against zero divisors. No zoom range check exists (`0x007FD2F0–007FD3C7`).
Draw passes the owner's frame byte directly to the metric decoder; it does not add a sequence start a second
time. A sequence's absolute first/current frame has already been copied to that byte by §4.3.

**R2-D1, proposed UNEXECUTED rounding boundary vector.** With nearest-even RC and finite small inputs,
the conversion B alone gives B(1/2)=0, B(3/2)=2, B(−1/2)=0, B(−3/2)=`0xFFFFFFFE`.
With P=8, the latter takes unsigned remainder 6. These exact rational inputs isolate the stored-addition/bit
conversion; they are not a claim that an approximate angle multiplication lands exactly on a half tie.
Changing RC changes this expectation. Direction bins must follow the stated conversion and native unsigned
remainders, rather than a portable ties-away rounding helper.

| Entry byte | Meaning as consumed by the owner |
|---|---|
| 0 | unsigned initial texture page |
| 1 | exponent k; side S=32-bit (1 << (k&31)) |
| 2,3 | unsigned initial pixel x,y |
| 4,5 | signed width,height; absolute values W,H; negative flips that UV axis |
| 6,7 | signed pixel offsets ox,oy |

For nonzero temporal frame n, delta=low32(W×n). If unsigned(delta+x)>unsigned(S−W), subtract
trunc((S−x)/W)×W from delta, set x=0, add H to y (if y>S−H set y=0 and increment page), and set
rowSpan=trunc(S/W)×W. While unsigned(delta)>unsigned(S−W), subtract rowSpan, add H to y and
roll/increment page as above. Finally add delta to x. The branch skipping the first crossing retains an
output-pointer integer in the rowSpan register; for valid packing its following loop cannot run.
Do not turn malformed records into a regular atlas algorithm (`0x007FD470–007FD512`).

UV edges use binary32 reciprocal 1/S, without half-texel bias. Vertices 0,1 share u0 and 2,3 share u1;
vertices 0,3 share v0 and 1,2 share v1. Positive dimensions give u0=x/S,u1=(x+W)/S and v0=y/S,v1=(y+H)/S;
negative dimensions swap the corresponding pair. Output is u32 page and float W,H,ox,oy. When ordered
abs(scale−1) >= binary32 `0x38D1B717` (approximately 0.0001), all four metrics are scaled; UVs/page are
unchanged (`0x007FD3D0–007FD63E`).

Independent vector: entry [2,6,48,32,16,−16,−3,4], frame 5 gives side 64, page 3, x=0,y=0 and
UVs [(0,0.25),(0,0),(0.25,0),(0.25,0.25)]. Scale 2 gives [page 3,32,32,−6,8].

## 5. Drawing (`Draw`, `0x007FEA90`; `UpdateView`, `0x007FE400`) *[compat]*

### 5.1 Quad construction and local box

This section describes finite inputs with nonzero denominators and valid loaded records. It does not promise
safe behavior for unloaded records, null cameras, zero viewport width, or malformed camera transforms.
`UpdateView` has no status return and no validation/failure branch for those inputs.

Let the decoded metric record be `[page,W,H,ox,oy]` (§4.1). Let `C` be the camera's local transform, and
`r=(C[+4],C[+0x10],C[+0x1C])`, `b=(C[+0x0C],C[+0x18],C[+0x24])`, `uy=C[+0x14]`.
These are the right column, back column and up-column Y component of its row-major matrix. Camera
translation and camera scale are not read here. `g` is custom magnification, stored to binary32 on return
from the camera controller (`0x007FE407–007FE418`).

If view utilities exist, `m` is their binary32 metres-per-pixel field +0x150. Otherwise get the camera's
view-volume left and right endpoints and compute `m=(right-left)/unsigned(context.width)`, storing the result
to binary32. The width conversion loads signed i32 and adds exactly 2^32 for negative values
(`0x007FE41C–007FE470`). The endpoints include the camera's view shift: with raw horizontal endpoints L,R,
camera viewport width V and horizontal shift D, they are `L+(R-L)/V*D`, `R+(R-L)/V*D`, each stored as
binary32 by GetViewVolume (`0x007FF340–007FF388`). Use the returned endpoints; do not cancel the shift before
their stores.

In real arithmetic the construction is:

- `d=H-oy`, `t=(m/uy)*g`, horizontal step `h=(r*m)*g`.
- For d>=0, `a=(t*d)/b.y` and displacement `B=b*a`; for d<0, B=(0,0,0).
- Upper left v0 = `(B.x-h.x*ox, B.y+t*oy-h.y*ox, B.z-h.z*ox)`.
- Upper right v3 = `(B.x+h.x*(W-ox), B.y+t*oy+h.y*(W-ox), B.z+h.z*(W-ox))`.
- Lower left v1 copies v0.x,z; its Y is `B.y-t*d-h.y*ox`.
- Lower right v2 copies v3.x,z; its Y is `B.y-t*d+h.y*(W-ox)`.

The corners at complete-object +0x13C,+0x148,+0x154,+0x160 are v0,v1,v2,v3 respectively.
Their order is upper-left, lower-left, lower-right, upper-right; the triangle fan and Pick use that order.
The vertical edges change world Y only. The construction does **not** use the camera-up column as a
vertical quad span. Owner rotation affects frame selection; owner scale is not applied to these corners.
Decoded resource scale has already affected W,H,ox,oy. Owner position is added later (§5.2).
Evidence: displacement `0x007FE47B–007FE507`, horizontal step `0x007FE50B–007FE564`, upper corners
`0x007FE641–007FE6C1`, `0x007FE7D8–007FE812`, lower corners `0x007FE92B–007FE987`.

Numeric qualification: these equations specify dependency/sign/order, not a portable last-bit oracle.
The routine inherits x87 precision/rounding control. In particular d and t are binary32 stores
(`0x007FE49D`, `0x007FE4AC`); horizontal X stores `r.x*m` before multiplying g and storing again;
the Y/Z products `r.component*m` stay in x87 before multiplication by g and their binary32 stores
(`0x007FE51E–007FE564`). B.y and B.z are stored as binary32 (`0x007FE4ED`, `0x007FE4FF`), while B.x remains
in the x87 stack. Products h.y*ox, h.z*ox, and each h.component*(W-ox) store to binary32 before their sums;
h.x*ox remains in the stack (`0x007FE652–007FE693`). Upper-left x,z have binary32 temporary stores as well
as corner stores (`0x007FE699`, `0x007FE6AD`). The lower baseline `B.y-t*d` stores to binary32 before the
two lower Y sums (`0x007FE943–007FE987`). Do not replace this with blanket binary32 or binary64 arithmetic.

The box is **reset** every UpdateView: min=(1,1,1), max=(0,0,0), followed by inclusion of the origin;
the inverted-X test therefore yields min=max=(0,0,0). Include v0 and then v3 componentwise. Next take
the resulting maximum M and include the single point `(-M.x,M.y,-M.z)`. No lower corner is included.
Thus the exact finite rule is the box of `{origin,v0,v3,(-M.x,M.y,-M.z)}`, with M taken before the last
inclusion. It may shrink on the next UpdateView; it is not a cumulative four-corner world box.
This finite rule describes numeric extrema; signed-zero payloads are not specified here.
Evidence: reset/origin `0x007FE51E–007FE641`; upper-corner inclusion `0x007FE6C7–007FE92B`;
reflected-maximum inclusion `0x007FE98D–007FEA82`. UpdateView itself does not clear either dirty byte.

**R2-Q1, proposed UNEXECUTED independent literal vector.** Use raw camera matrix rows
`(1/2,-1/2,1/2)`, `(0,1/2,1)`, `(-1/2,-1/2,1/2)`, m=2, g=1/2,
metrics `[page=3,W=6,H=10,ox=2,oy=3]`. This deliberately supplied, nonsingular raw matrix is not normalized
by UpdateView; it tests a rotated camera without irrational constants. All intermediate arithmetic here is
exact in binary32: d=7, t=2, h=(1/2,0,-1/2), a=14, B=(7,14,7).

| Corner | Local cached position | With owner position (10,20,30) |
|---|---|---|
| v0 | (6,20,8) | (16,40,38) |
| v1 | (6,0,8) | (16,20,38) |
| v2 | (9,0,5) | (19,20,35) |
| v3 | (9,20,5) | (19,40,35) |

Box progression: origin only [0,0]^3; after v0, min=(0,0,0), max=(6,20,8); after v3,
max=(9,20,8); final min=(-9,0,-8), max=(9,20,8). Importing owner matrix
rows `(0,0,1),(1,0,0),(0,1,0)` and owner scale 7 does not change these cached positions when the same
metrics are selected. It can change which orientation record is selected before this calculation.

**R2-Q2, proposed UNEXECUTED shrink/extremum vector.** Refresh Q1 with W=H=2, ox=oy=1/2,
same camera/m/g: d=3/2, t=2, a=3, B=(3/2,3,3/2). Corners v0,v1,v2,v3 are
`(5/4,4,7/4)`, `(5/4,0,7/4)`, `(9/4,0,3/4)`, `(9/4,4,3/4)`.
After origin/v0/v3, max=(9/4,4,7/4); final box is min=(-9/4,0,-7/4), max=(9/4,4,7/4),
independently of the previous larger box. The Y maximum is exactly shared by the upper corners; the reflected
point keeps that maximum and supplies both negative horizontal extrema.

### 5.2 Refresh, translation and draw state

Draw returns 1 immediately for a nonzero **boolean highlight-pass argument** or signed context zoom<2.
The stored highlight bit does not cause this early return (`0x007FEA90–007FEAAD`). There is no upper zoom
range check: a context zoom outside the valid holder range must not be described as a safe successful draw.
The normal loaded Draw domain is context zoom 2–4. After the signed early test, lookup uses the cached
**low byte**: e.g. full context zoom 258 selects holder 2, but compares unequal to cached byte 2 on every draw
and therefore refreshes every time. Full zoom 5 indexes beyond the five-holder array without a guard;
it is not a defined empty draw. The same full-integer comparison/low-byte storage distinction applies to
context rotation (`0x007FEABD–007FEB44`).

If view-dirty is already set it remains set. Otherwise compare full context zoom/rotation integers to the
zero-extended cached bytes, setting view-dirty when either differs. If view-dirty or frame-dirty is set,
copy the low bytes of context zoom/rotation, refresh orientation/metrics/UVs (§4.1), call UpdateView **only if
view-dirty**, then clear frame-dirty (`0x007FEAB3–007FEB80`). A changed record or frame alone need not rebuild
geometry. A changed camera matrix, custom magnification or mpp alone does not participate in invalidation.

If view-dirty is set or the cached transform counter differs from the owner's byte counter, rewrite draw
vertex positions as cached local corners plus owner position, componentwise with binary32 stores. Copy the
owner counter to the cache and clear view-dirty (`0x007FEB81–007FEC62`). Equal counters (including wrap after
256 changes) can leave old translated positions. Rotation/scale are not applied to the cached quad here.

Then call, in order (`0x007FEC69–007FED3B`):

1. Context `SetModelTransform()` with **no model argument** (draw-context §8: camera view transform).
2. `SetModelShade(shared instance, owner RGB)` normally, or `SetSelfLitShade(owner RGB)` when flag 4 is set.
3. `SetRenderState(owner packed state)`.
4. If flag 2 is set: `SetTransparency(alpha)` then `SetAlphaFunc(4, binary32(alpha*double(0.5)))`.
5. `SetTexture(binding.table[metrics.page],0)`; no page/binding validity check here.
6. `DrawPrims(2,10,4,owner draw-vertex block)`, i.e. four interleaved position/colour/UV vertices, stride 24.
7. `ResetShade()`, then `ResetTransparency()`; return 1.

The routine does not branch on any of those calls' returns. There is no forced restoration of packed render
state, texture, alpha function, or a previous model transform, and no unwind/restore promise if a callee faults.
The two reset calls use the context's reset semantics, not a saved copy of arbitrary incoming state.

**R2-Q3, proposed UNEXECUTED cache vector.** After Q1 is drawn with equal cached context zoom/rotation,
frame-dirty=view-dirty=0 and equal transform counters, change only m from 2 to 4 and change the camera matrix.
The next Draw retains the Q1 metric selection and all four translated positions. Its context draw calls still
run. With the original camera restored, m=4, and view-dirty explicitly set, the next Draw refreshes: d=7,t=4,
h=(1,0,-1), B=(14,28,14), corners `(12,40,16),(12,0,16),(18,0,10),(18,40,10)`;
box min=(-18,0,-16), max=(18,40,16). Frame-dirty and view-dirty end zero and the counter cache is copied.
If only frame-dirty is set, metrics/UVs refresh but UpdateView and translation are skipped when counters match.

**R2-Q4, proposed UNEXECUTED fallback vector.** Remove view utilities; give the camera raw horizontal
volume L=−10,R=30, viewport width V=80, shift D=4 and context width 20. GetViewVolume returns left=−8,
right=32, so m=2 exactly. With Q1's remaining inputs the complete Q1 intermediate/corner/box results follow.
The zero-width and zero-denominator cases have no safe-result contract.

**R2-Q5, proposed UNEXECUTED box-omission vector.** Use raw nonsingular camera rows `(1,0,0)`,
`(1,1,1)`, `(0,0,1)`, m=g=1, W=4,H=2,ox=3,oy=1. Then d=t=a=1,
h=(1,1,0), B=(0,1,1), and v0,v1,v2,v3 are `(-3,-1,1),(-3,-3,1),(1,1,1),(1,3,1)`.
After origin/v0/v3 the extrema are min=(-3,-1,0), max=(1,3,1); the last candidate (-1,3,-1) gives
final min=(-3,-1,-1), max=(1,3,1). The exact X minimum stays −3; it is not symmetrized to −1.
The lower corner Y=−3 is outside this box. This deliberately raw camera fixture tests the literal box
algorithm without assuming an orthonormal camera or silently widening it to all four corners.

## 6. Picking (`Pick`, `0x007FD710`)

Pick copies the four **translated draw vertices**, not the local corners, and tests triangles (0,1,2), then
(2,3,0), using `SC4ViewUtilities::TrianglePick` (view-utilities §7). It returns true on the first hit, else the
second helper's boolean. It never refreshes frame, quad or translation; calling Pick after a setter but before
a refreshing Draw therefore tests stale geometry. It does not apply the model transform, inspect highlight,
or reject zooms itself (`0x007FD710–007FD7C6`).

## 7. Users

The model maker creates sprite instances for occupants whose models are sprite animations; the scene draws and
picks them through the two thunks; animation ticks drive the clock.

## 8. Quirks

1. Sprites vanish at the two farthest zooms (§5).
2. Fade sets the alpha-test threshold to alpha times 0.5, with function code 4 (§5.2).
3. The resource-key functions' names do not match what they build (§2).
4. Without frame skipping a long tick advances one frame and leaves the clock at 1, so the next tick advances
   again (§4).

## 9. Open questions

See [questions/animated-sprite.md](questions/animated-sprite.md). AS1 controller provenance and AS2 shipped
usage remain open; AS3 inventory accounting and remaining AS4–AS9 qualifications are tracked there.
The record consumer and IID are specified in §§2/4.1; `0x69B6F01C` is an IID, not the resource key type.

## 10. Conformance tests

1. **Zoom:** at zoom 1 the instance draws nothing; at zoom 2 it draws one quad.
2. **Direction:** use §4.1's two angle inputs and native conversion, with conditional half-tie vector R2-D1.
3. **Loop:** play mode 3 goes from the last frame to frame 0; mode 2 steps back to the previous frame.
4. **Clock:** rate 10, direction 1, elapsed 0.25: with `+0xAA` set, two frames and 0.5 kept; without it, one
   frame and the position 1.
5. **Geometry/cache:** proposed UNEXECUTED R2-Q1–Q5 specify raw inputs, intermediate values, corners, bounds
   and dirty transitions; R2-R1–R3 specify noncommuting orientation and sentinel behavior. No native,
   game or implementation execution is claimed for these vectors.
