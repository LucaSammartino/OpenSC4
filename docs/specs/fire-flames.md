# Spec: fire flames (`cSC4Flame`, `GetPointsOnModel`, the fire occupant's flames)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [disasters.md](disasters.md) (§5.11 the fire occupant: its interface, the flame counts from the D7
and D8 curves, stages, targets; this spec only adds the flames), [effects.md](effects.md) (§4.1 creating a visual
effect by name), [model-instances.md](model-instances.md) (the meshes and transforms read here),
[geometry-utilities.md](geometry-utilities.md) (§2 `TwiceTriangleArea`), [foundations.md](foundations.md) (§6.1
the generators).

## 0. Summary

A burning occupant shows its fire as a set of **flames**, each a visual effect placed on the target. When the
target has a mesh, the flames are scattered over its triangles, weighted by area and set 1 m out along each face;
otherwise they are spread over its bounding box: some on top, the rest on the four sides. Each flame plays the
effect for the fire's stage, `fire_1`–`fire_5` on top or `wfire_1`–`wfire_5` on a wall.

## 1. The flame (`cSC4Flame`) *[compat]*

A flame holds: a "made" byte, a visible flag, the **wall** flag (`+0x02`), the **side** (`+0x04`, 0–3), the
position (x, y, z at `+0x08`), a transform and its effect (`+0x4C`).

- `Update(stage)` (`0x004710D0`): the current effect, if any, is stopped (effect slot 4) and released; the effects
  manager (global `0x00B43D1C`, slot 7, effects.md §4.1) makes the effect named `wfire_<stage + 1>` for a wall
  flame or `fire_<stage + 1>` otherwise (tables `0x00B07FA4` and `0x00B07F90`, stages 0–4); on success the transform
  is applied and, when the flame is visible, the effect starts (slot 3).
- `UpdateTransform()` (`0x00470CA0`): the effect's transform (slot 7) becomes a translation to the position and a
  rotation about the vertical axis by the side × 90, **passed to sine and cosine as radians** *[quirk]*: the four
  sides turn the flame by 0, 90, 180 and 270 radians (about 0°, 116.6°, 233.2° and 349.9°), not quarter turns.
- `Read(stream)` (`0x004700A0`): a 16-bit version (below 2 fails), the wall byte, the side, the position, then the
  transform (version 3 and later; earlier versions use the legacy transform reader); true when the stream reports no
  error.

## 2. Points on a model (`GetPointsOnModel(n, occupant, out)`, `0x00471CF0`) *[compat]*

1. The occupant's model instances (occupant slot 19); none is false.
2. For each instance that is a mesh container (model-instances.md §2), with its transform (instance slot 9):
   each mesh's indices are taken three at a time as triangles, `ceil(k ÷ 3)` triangles for the k indices of the
   mesh's first primitive *[inferred: the meshes are triangle lists]*. Each triangle (`0x00470EC0`) records its
   three vertices, the running total of `TwiceTriangleArea` over all triangles so far, its unit normal
   n = (v₁ − v₀) × (v₂ − v₀) ÷ \|…\|, a flag "n.z > 0", and a **side** code: 0 when \|n.x\| ≥ \|n.z\| and n.x ≥ n.z,
   1 when \|n.z\| > \|n.x\| and n.x ≥ n.z, 2 when \|n.x\| ≥ \|n.z\| and n.x < n.z, 3 otherwise (the faces +x, −z,
   −x, +z).
3. No triangle at all is false. Else `out` gets n entries from a new generator seeded from the clock (seed −1,
   foundations §6.1), so **flame placement is not deterministic** *[compat-limit]*. For each entry:
   - r := `RandomDoubleRangeUniform(0, total)`; the triangle is the first whose running total is ≥ r;
   - u := `RandomDoubleUniform()`, v := `RandomDoubleRangeUniform(0, 1 − u)`, w := 1 − u − v; the point is
     u v₀ + v v₁ + w v₂ (this is not uniform over the triangle: it gathers points towards v₀ *[quirk]*);
   - the point moves 1 m along the unit normal, then through the instance's transform, s (M p) + t;
   - the entry is (point, the triangle's flag, its side). True.

### 2.1 Physical index and selection boundaries (FF3)

The first primitive supplies its **count**, but its first-index offset is **ignored**. The index
pointer starts at the index-buffer data plus two bytes, and each iteration reads the preceding,
current and following u16, then advances six bytes (`0x00471E00–0x00471E06`,
`0x00471E33–0x00471E5A`, `0x00471E80–0x00471EA2`, `0x00471FC7–0x00471FD7`). Thus the triangles
use physical indices 0,1,2; 3,4,5; etc., even when the first primitive requests a different offset.
For nonzero k the loop count is `(k−1)/3 + 1`; the final partial triple reads beyond the primitive's
count. Index values multiply the vertex stride and add the vertex data pointer without bounds
checking. Missing buffers, short physical storage or out-of-range vertex indices are unsafe native
reads, not established false/error returns. For k=4 with six readable indices, two triples are read;
changing only the primitive first-index field does not change those triples.

No model instances and no collected triangle return false (§2). Otherwise a local **cRZRandom** is
constructed with seed −1 (`0x004720B3–0x004720BC`), and requested count zero can return true without
placement draws. The area draw promotes the stored float total to double for RandomDoubleRangeUniform.
Selection compares float cumulative totals against its still-register result. It selects the first
**greater/equal or unordered** comparison: a NaN comparison also stops the search. If all comparisons
are ordered-less, it computes a triangle pointer **one past the vector**, then continues sampling;
there is no native fallback triangle or false result (`0x004720F0–0x00472142`).

The normal helper stores the cross-product components as floats, sums their squares and takes x87
FSQRT, divides 1 by the result without a zero-length check, then multiplies and stores each component
as float (`0x00470F86–0x00470FD9`). Zero-area geometry is not rejected: under ordinary masked floating
exceptions its normal components become NaNs. No nonfinite-coordinate validation is added here.
The u draw is stored as float (`0x00472142–0x00472147`); `1−u` is stored as float and also passed as
double for the v range (`0x0047214B–0x00472170`). Thus replacing the complete sampler with arithmetic
rounded to float after every operation is not proved equivalent. Exact area accumulation and all
placement stores are specified in §2.2; these explicit stores do not imply a float operation after
every arithmetic instruction.

### 2.2 Native area accumulation (FF3)

Here **F** means a store to binary32 followed by reload; arithmetic without F remains in the x87
register format under the active control word. **Each arithmetic operation still rounds its
significand according to that control word.** Main-thread startup sets 24-bit precision:
`0045236C` calls `005F85E0`, which supplies mode 0 to the setter at `008929E0–00892A09`.
The setter clears both PC bits for mode 0; mode 1 selects 53-bit and mode 2 selects 64-bit.
Foundations §18 describes thread/caller exceptions. These flame helpers neither change nor restore
the control word. Normal main-thread evaluation therefore rounds each arithmetic instruction to
24 significant bits, retaining the wider x87 exponent until an explicit float store. It is not
unconditionally double or 80-bit-significand arithmetic. Examples assume nearest/even with masked
exceptions; any different precision assumption is stated explicitly.

TwiceTriangleArea (`0x005D75D0–0x005D7653`) uses a different subtraction origin and different spill
pattern from the normal calculation. Let a = v₀ − v₁, b = v₂ − v₁. All three a differences and b.z
remain in registers; b.x and b.y are stored as F (`0x005D75DB–0x005D7606`). It computes:

| Quantity | Expression and store boundary |
|---|---|
| c.x | F(a.y × b.z − a.z × F(b.y)) |
| c.y | F(a.z × F(b.x) − a.x × b.z) |
| c.z | a.x × F(b.y) − a.y × F(b.x), retained in a register |
| Returned twice-area | sqrt((c.z × c.z + c.y × c.y) + c.x × c.x), retained in a register |

Cross-component stores are `0x005D7612` and `0x005D7621`; c.z is not spilled before squaring.
The ordered square/add/square-root sequence is `0x005D7636–0x005D764E`. There is no division by two.
The caller adds the previous **stored float** cumulative total to this register result, then stores
the new cumulative total as F (`0x00470F08–0x00470F17`). The mesh loop copies that stored word to
the next triangle's accumulator (`0x00471FC4–0x00471FCB`). It does not sum all triangle areas in
double and round only once; nor does it first round the returned twice-area to float.

Independent area vector: start with vertices (0,0,0), (4096,0,0), (0,4096,0), then append two
triangles with vertices (0,0,0), (1,0,0), (0,1,0). Their twice-areas are 2²⁴, 1, 1. Stored cumulative
totals are **16777216, 16777216, 16777216**, whereas rounding only the final sum gives 16777218.
Thus the two appended nondegenerate triangles can have no positive-width selection interval.

### 2.3 Barycentric, normal and placement stores (FF3)

F and unmarked register arithmetic have the meanings in §2.2. These are store boundaries, not a
promise that the active x87 precision/rounding control equals a portable double implementation.
The three local vertices are v₀/v₁/v₂ in physical index order; n is the stored float unit normal.

Normal construction (`00470F1D–00470FD5`) forms a = v₂ − v₀ and b = v₁ − v₀: a.x is retained
in a register, a.y/a.z and every b component are F. It computes b × a using the retained a.x and
stored other differences, then stores **all three** cross components as F. The reciprocal length
is 1/sqrt((n.x² + n.y²) + n.z²), with no intermediate float store; each component is multiplied by
that reciprocal and stored as F. This is separate from the twice-area helper's spill pattern.

For barycentrics (`00472142–00472245`): u = F(first draw). Let k be the register subtraction
1 − u; its F copy is retained locally, while a double copy of **k**, without reloading F(k),
supplies the upper bound of the second draw. Let v be that second draw retained in a register;
w = F(k) − v also remains in a register. Define the products by the following table:

| Products | Store boundary |
|---|---|
| u × v₀.x | Register |
| u × v₀.y, u × v₀.z | F |
| v × v₁.x, v × v₁.y | Register |
| v × v₁.z | F |
| w × v₂.x, w × v₂.y | F |
| w × v₂.z | Register |

The weighted local coordinates are:

| Coordinate | Ordered addition/store boundary |
|---|---|
| X | F(F(u×v₀.x + v×v₁.x) + F(w×v₂.x)) |
| Y | F((F(u×v₀.y) + v×v₁.y) + F(w×v₂.y)) |
| Z | w×v₂.z + F(F(u×v₀.z) + F(v×v₁.z)); a float output copy is written, but the register Z survives |

The output position is repeatedly overwritten on the way to its final value. The normal-offset
coordinates (`00472248–00472279`) are Lx = F(n.x + X), Ly = n.y + Y, Lz = Z + n.z.
Float copies of Ly/Lz are written to the output, but their **register values before float-store conversion** feed the
matrix. Lx feeds it through its float local copy. For each matrix row j, the evaluation order is
(Lz × M[j,2] + Ly × M[j,1]) + Lx × M[j,0] (`0047227F–004722D1`). Rows 0/1 are F before
scaling; row 2 is retained, even though a float copy is written. Translation, matrix and scale are
the instance's 56-byte transform fields; here the matrix starts at +4, translation
at +0x28 and scale at +0x34 of the transform pointer.

After multiplication by scale, rows 0/1 are again F before adding translation; row 2 stays in its
register across its output float copy. Finally each scaled coordinate plus its translation is
stored as F (`004722D4–0047230B`). In particular the z path is not equivalent to rounding every
intermediate position or every matrix/scale output to float before continuing. Wall/side bytes are
copied from the triangle before placement; they are not recomputed after the instance transform.

Independent cancellation vector (**53-bit precision**, prescribed draw outputs, nearest/even): v₀=(0,0,1), v₁=(1,0,1),
v₂=(0,1,67108864), u=0.25, v=0.25, hence w=0.5. The stored normal is (0,−1,2⁻²⁶). Use identity
matrix, scale 1 and translation (0,0,−33554432). The weighted Z is 33554432.5, retained across
its output float copy; final position is **(0.25,−0.5,0.5)**. Reloading the float z after each
position/matrix/scale write instead gives z=0. This is an arithmetic conformance vector, not a
claim that a clock-seeded run reproduces those exact draws. At the normal main-thread **24-bit**
precision the arithmetic itself rounds away the half-unit, and this vector's native z is **0**.

Independent exponent-range vector (**24-bit precision**, nearest/even, masked exceptions):
v₀=(0,0,2¹²⁷), v₁=(1,0,2¹²⁷), v₂=(0,1,2¹²⁷), with u=v=0.25, w=0.5, normal (0,0,1).
Use matrix diag(1,1,4), scale 0.25 and zero translation. Weighted z and normal-offset z are 2¹²⁷;
the third matrix row yields **2¹²⁹** in the x87 register. Its float output copy overflows to
positive infinity, but the register survives and scaling returns it to 2¹²⁷. Final position is
**(0.0625,0.125,2¹²⁷)**, all finite. Reloading that intermediate float copy instead leaves z at
positive infinity. Thus even 24-bit significand rounding does not license substituting binary32
exponent-range arithmetic for every register operation.

## 3. Making the flames (`CreateFlames`, `0x00472380`) *[compat]*

With T and S the top-flame and side-flame counts (disasters.md §5.11) and the 3D view's current view level
(global `0x00B43DC4`, slot 22):

- **On the model:** `GetPointsOnModel(T + S, target, points)`. When true, one flame per point: wall := the point's
  flag, position and side from the point; the fire remembers that its flames are on the model (`+0x64`).
- **On the box** (otherwise): the target's bounding box (min, max; occupant slot 10). Flames are made while the
  count is **≤ T + S**, so there are **T + S + 1** *[quirk]*; each starts at the box's centre in x and z, and:
  - a flora target (type `0x74758926`): y = min.y;
  - else the first T flames are top flames: y = max(max.y − 1, min.y), and when T > 1, x and z are drawn with
    `RandomDoubleRangeGaussian` over the box (foundations §6.1);
  - the others are wall flames on a random side `RandomUint32Uniform(4)`: side 0 x = max.x, side 1 z = min.z,
    side 2 x = min.x, side 3 z = max.z, the free horizontal coordinate uniform over the box, y uniform over
    [min.y, max.y].

  These draws use the fire code's own generator (`0x00B21648`).
- Every flame: position and transform set; when the view level is not 1, a visible flame is hidden (effect slot 4).

## 4. Stage, start and view updates *[compat]*

- `SetStage(s)` (`0x004728B0`): when s differs, it is stored; with no flames yet, `CreateFlames` runs; then every
  flame's `Update(s)`.
- `Init` (`0x00471170`): an already initialized occupant returns true immediately. Otherwise it calls the
  base occupant `Init`, joins occupant group `0x5005`, sets each existing flame's made byte to one and calls
  its `Update(stage)`. It returns true regardless of the base return and does **not** call `CreateFlames`
  (`0x00471173–0x004711BC`). This is the call-order rule where disasters §5.11 says that Init builds flames.
- `ViewUpdate` (`0x00472790`, on the view messages of disasters.md §5.3 step 6): only for flames on the model, and only
  when `GetPointsOnModel` gives as many points as there are flames: every flame is **placed afresh** at a new random
  point (its wall flag, side and position), and hidden when the view level is not 1. So model flames jump to new
  places on every view update *[quirk]*.

### 4.1 Construction, visibility and replacement boundaries

The native flame constructor is `0x00470BC0–0x00470C9F` (224 body bytes), despite its vector-resize
label in the export. Fresh flame allocations call it at `0x00472420` and `0x00472527`. Its initial
made/visible/wall bytes are **0/1/0**, side is zero, effect is null, and the embedded transform is
identity. The position comes from the shared word at `0x00B21640`; its numeric value is not established
here. Box top/flora flames inherit the initial wall=false and side=0 unless explicitly overwritten.

The fire constructor (`0x004711C0–0x0047122A`) leaves the flame vector empty, on-model false and stage
**−1** (complete-object `+0x44`). Init receives an interface pointer four bytes into that object;
its `+0x40` is the same stage. `SetStage` compares before doing anything: an equal stage, including
−1 on a fresh object, creates nothing and updates nothing. A changed stage is stored before
create-if-empty and updates; there is no failure propagation (`0x004728B0–0x004728F7`). Update directly
indexes the two effect-name tables with the stage (`0x004710FD–0x00471110`); there is no clamp or
defined fallback for stages outside 0–4.

Visibility assignment first compares the requested byte; an equal byte makes no effect call. A changed
byte is stored even when there is no effect; with an effect,
show starts it with argument 1 and hide stops it with argument 1 (`0x00470270–0x00470290`). Update
instead stops an old effect with argument 0, clears/releases it, then attempts replacement. Failure
does not restore the old effect (`0x004710D3–0x00471160`). During Shutdown, a made flame's effect is
queried for activity (slot 5) and stopped with argument 0 only if active before release. An unmade
flame's effect is released without that stop (`0x00470DE0–0x00470E28`). This establishes shutdown
cleanup; it is not a claim that every destruction entry has been reconciled.

`UpdateTransform` with no effect does nothing. Its temporary transform starts with header bytes 2
and 5. Rotation uses x87 FSIN/FCOS with intermediate float stores (`0x00470CD1–0x00470D1A`), rather
than an established portable f64 library-trigonometry rule. Matrix signs, counter writes and the
remaining placement arithmetic are specified only when independently proved below.

### 4.2 Rotation bytes and placement call order (FF4)

UpdateTransform constructs a **temporary** 56-byte transform. Its change-counter byte (+0) is 2 and
kind byte (+1) is 5 (matrix plus translation); the remaining two header bytes are not written. The
scale is float 1, translation copies the flame's position, and effect slot 7 receives the temporary.
It does not update the flame's embedded transform or increment its counter. With no effect it
returns before constructing that temporary (`0x00470CA2–0x00470CAA`, `0x00470CB8`,
`0x00470CD7–0x00470CF9`, `0x00470D9E–0x00470DAC`). This does not prescribe downstream effect-counter
behavior outside that call.

The side is an unsigned 32-bit word: native conversion loads it signed, adds exactly 2^32 if its
sign bit is set, then multiplies by float **90.0**, retaining the angle in x87
(`0x00470CB0–0x00470CC8`; constants `0x00A80AA8`, `0x00A882A8`). No degrees-to-radians conversion
occurs. With c=cos(angle), s=sin(angle), the matrix's **rows**, at temporary +4,+0x10,+0x1C, are:

| Row | Real-valued entries |
|---|---|
| 0 | c, 0, s |
| 1 | 0, 1, 0 |
| 2 | −s, 0, c |

The actual instructions compute two additional angles equal to double zero (`0x00A80990`). Their
cosines and sines are each stored to float before combination. The side-angle cosine is stored to
float before the matrix products, while its sine stays in x87 until the individual matrix stores
(`0x00470CFA–0x00470D1A`, `0x00470D24–0x00470DA6`). Each final entry is stored to float; there is no
f64-library-trig guarantee. The table gives signs/geometry, not permission to erase native signed
zeros: side zero stores −0 at matrix +8 and +0x18, and +0 at +0x1C under ordinary nearest rounding.
For side 1, the first basis vector rotates toward negative z: approximately (−0.448074,0,−0.893997).

Box placement draws side first, **free horizontal coordinate second, height third**; each coordinate
draw is stored to float before assignment. Sides 0/2 draw z, sides 1/3 draw x
(`0x00472612–0x004726D0`). Top flames with T>1 draw Gaussian x then z
(`0x004725CD–0x00472609`). Flora inherits constructor wall=false and side=0; top flames explicitly
write wall=false and retain side=0; wall flames write wall=true and their selected side. Hiding clears
the visible byte even without an effect (§4.1); view level 1 does not automatically re-show a flame
previously hidden by these placement paths.

Call order is also observable with existing effects: model construction writes wall/position, calls
UpdateTransform, then writes side and calls again (`0x00472433–0x0047245A`). ViewUpdate writes wall/side,
calls UpdateTransform using the old position, then writes the new position and calls again
(`0x00472836–0x0047285C`). Box wall creation calls once after side assignment with the old position,
then again after position assignment (`0x004726D4–0x004726F3`); fresh flames have null effects so those
calls make no effect-slot calls. None of these direct position/side writes mutates the embedded
transform or its counter. Test the two successive slot-7 calls with a live effect during ViewUpdate,
and separately test that a fresh flame's embedded identity transform stays unchanged.

### 4.3 Persistence, destruction and inventory boundary (FF2)

The fire writer (`0x00470130–0x0047020A`) writes u16 version **3**, then calls the base occupant writer.
On base success it writes the three words at adjusted receiver +0x38,+0x3C,+0x40, the serializable
field at +0x4C, and the flame vector count. Each flame is written inline: u16 version **3**, wall via
stream slot 5, side via slot 9, position via the vector writer, embedded transform via the transform
writer. There is no separate standalone flame-write body in this path. Made, visible and effect
pointer are omitted. A per-flame stream error returns false immediately; the final result is the
inverse stream-error flag, including the base-failure branch. There is no rollback.

The transform writer (`0x005DA8C0–0x005DA92B`) writes its kind byte, then matrix if bit 2 is set,
translation if bit 0 is set, scale if bit 1 is set, in that order. It omits the change counter and
padding bytes. A freshly constructed flame's kind-zero embedded identity therefore writes no matrix,
translation or scale, even though its separate position is serialized.

Fire Read (`0x00471A70–0x00471B8A`) accepts **only version 3**; unlike flame Read it rejects higher
versions too. On base success it reads those three words, releases/clears the old serializable field,
reads its replacement with IID `0x26BB767E` and argument 0, then reads the flame count. It constructs
and reads fresh 0x50-byte flames and appends each successful one without first clearing existing
flames. On a flame read failure it returns false with earlier additions retained; the failed fresh
allocation is not reclaimed on that branch. Made/visible/effect state therefore starts from the
flame constructor and is not restored by this wire record. A failed allocation reaches a null flame
read; no clean allocation-failure result is established. Outside explicit version/flame failure,
the result is the inverse final stream-error flag, even if the base reader returned false.

Shutdown first checks initialized state; if false it returns true **without** cleaning flames
(`0x00470DC3–0x00470DCA`). Its initialized path performs §4.1's stop/release rules and frees each
flame allocation while erasing pointers. Direct destruction is different: `0x00471290–0x004712DC`
restores vtables, frees the flame-vector **storage**, Releases the nonnull target at complete-object
+0x54, then destroys the base occupant. It neither calls Shutdown nor walks/reclaims flame pointees
or their effects. Deleting wrapper `0x00471600–0x0047161D` runs that body and frees the complete object
iff flags bit 0 is set. The caller must not assume direct destruction reproduces Shutdown's cleanup.

The following is the measured owner-name inventory, with two independently identified owner bodies
added. Lengths are function body bytes, not address-span estimates; shared inherited/thunk bodies
are not silently included.

| Owner | Address / body bytes | Body |
|---|---|---|
| Fire occupant | `0046FDC0` / 6 | Class ID |
| Fire occupant | `0046FE00` / 36 | Extinguish |
| Fire occupant | `0046FE30` / 39 | QueryInterface |
| Flame | `004700A0` / 135 | Read |
| Fire occupant | `00470130` / 219 | Write, including inline flame writer |
| Fire occupant | `00470210` / 19; `00470230` / 19 | Position and bounds forwarding |
| Fire occupant | `00470250` / 80 | Visibility |
| Flame | `00470BC0` / 224 | Constructor, mislabeled vector resize |
| Flame | `00470CA0` / 275 | UpdateTransform |
| Fire occupant | `00470DC0` / 242 | Shutdown |
| Flame | `004710D0` / 145 | Update |
| Fire occupant | `00471170` / 77 | Init |
| Fire occupant | `004711C0` / 107 | Constructor |
| Fire occupant | `00471290` / 77 | Destructor body, unnamed |
| Fire occupant | `00471600` / 30 | Deleting wrapper |
| Fire occupant | `00471900` / 367 | SetTarget |
| Fire occupant | `00471A70` / 283 | Read |
| Fire occupant | `00472380` / 1013 | CreateFlames |
| Fire occupant | `00472790` / 275 | ViewUpdate |
| Fire occupant | `004728B0` / 72 | SetStage |

The 19 named bodies total **3439 bytes**; with the constructor and destructor identified above this
is **21 / 3740**, not the brief's 16 / 3513. Triangle construction (`0x00470EC0`, 418) and
GetPointsOnModel (`0x00471CF0`, 1662) are separate non-owner helpers. The legacy owner/count scope
is reconciled below. This table is not a claim that every inherited occupant method belongs to
this small owner surface.

### 4.4 Legacy subsystem scope versus semantic owners (FF2)

The archived attribution in main `build-runs/coverage-owners-base.tsv` (also independently present
in `coverage-owners-occprops.tsv`) identifies the **16/3513** selection exactly. It is three
cSC4Flame bodies plus thirteen bodies attributed by reachability to **fn:GetPointsOnModel**;
it is not the inventory of cSC4Flame plus cSC4FireOccupant.

| Archived attribution owner | Addresses / decimal body bytes |
|---|---|
| cSC4Flame | `004700A0`/135, `00470CA0`/275, `004710D0`/145 |
| fn:GetPointsOnModel | `0046FE80`/59, `0046FEC0`/249 |
| fn:GetPointsOnModel | `004702B0`/114, `00470330`/119, `004703B0`/61, `004703F0`/54 |
| fn:GetPointsOnModel | `00470430`/678, `004706E0`/233, `004707D0`/220, `004708B0`/667 |
| fn:GetPointsOnModel | `00470EC0`/418, `00471070`/45, `004710A0`/41 |
| Total | **3/555 + 13/2958 = 16/3513** |

The actual GetPointsOnModel body `00471CF0`/1662 is **not** in those sixteen; neither is the
mislabeled flame constructor `00470BC0`/224. This demonstrates why a reachability-owner subtotal
cannot stand in for a complete class/API contract. Current attribution assigns the actual sampler
to cSC4FireOccupant by free-neighbor rules and counts 35/6518 for that owner, plus 3/555 for
cSC4Flame; these are again different scopes. No coverage-map or name-map change is proposed here.

Seven additional native fire-occupant adjustor bodies can be classified without expanding that
legacy subtotal:

| Body / bytes | Receiver adjustment and target |
|---|---|
| `00471230`/8 | −4, QueryInterface `0046FE30` |
| `00471240`/8 | −8, QueryInterface `0046FE30` |
| `00471250`/8 | −12, QueryInterface `0046FE30` |
| `00471260`/8 | −12, deleting destructor `00471600` |
| `00471270`/8 | −32, QueryInterface `0046FE30` |
| `00471280`/8 | −4, Write `00470130` |
| `00471B90`/8 | −4, Read `00471A70` |

Each is a tail jump after the stated adjustment, with no independent lifecycle work. They do not
turn direct destruction into Shutdown. Their target contracts are in §4.3. They are included in
the current 35-body fire-occupant attribution above, not added a second time to that number.

FF2 is parked after the final round for the exact remaining **complete operand/byte contracts of
the twelve non-triangle reachability helpers** above, and complete inherited-occupant shared-slot
membership beyond the seven proved wrappers.
Triangle construction, sampler arithmetic, the 21 lifecycle/visual bodies in §4.3 and the exact
legacy membership/count are preserved. Numerical reconciliation alone does not close the complete
owner-contract request or authorize blanket completion credit.

## 5. Users

The fire disaster (disasters.md §5) creates fire occupants; their stage changes and the view messages drive this
spec. `GetPointsOnModel` has no other caller.

## 6. Quirks

1. The flame's yaw uses degrees as radians (§1).
2. Box fires get one flame more than the counts say (§3).
3. Model flames are placed by a clock-seeded generator, biased towards each triangle's first vertex, and re-placed
   on every view update (§2, §4).

## 7. Open questions

See [questions/fire-flames.md](questions/fire-flames.md): FF1 asks whether the "n.z > 0" wall flag is meant for
model space with z up or is an original slip; FF2 retains only the exact parked helper/shared-slot
contracts of §4.4. FF3's numeric/index boundaries are answered in §§2.1–2.3.

## 8. Conformance tests

1. **Effect name:** a wall flame at stage 2 plays `wfire_3`; a top flame at stage 0 plays `fire_1`.
2. **Box count:** T = 1, S = 6 on a target without a mesh gives 8 flames: 1 top, 7 walls.
3. **Top height:** a box from y = 0 to y = 20 puts top flames at y = 19.
4. **Side code:** a face with normal (0, 0, −1) has side 1; (0.6, 0, 0.8) has side 3.
5. **Yaw:** side 1 rotates by 90 radians, cos = −0.448.
