# Spec: terrain overlays, decals and shadows (`cSTEOverlayManager`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [terrain-view.md](terrain-view.md) (the terrain view that owns the four managers, and its land and
water passes), [terrain.md](terrain.md) §15.5 (the view's `+0x78`–`+0x84` and the terrain's `+0xF8`),
[draw-context.md](draw-context.md) (the states, texture stages and texture transforms used here),
[texture-bindings.md](texture-bindings.md) (the binding factory), [render-properties.md](render-properties.md).

## 0. Summary

An overlay manager keeps textured **decals** and **shadows** that are drawn onto the terrain: each overlay is a
texture with a world-to-texture transform and a rectangle of cells it covers, and is drawn by redrawing those
terrain cells with the texture projected on them. Callers (tools, lots, buildings, effects) get a handle when
they add an overlay and use it to move, show, hide, recolour, fade or remove it. The terrain view owns four
managers: static and dynamic, each for land and for water.

## 1. The object *[compat]*

- 0xFC bytes, constructor `0x0073BC20`; vtables: `+0x00` `0x00AB39E8` (the interface, IID `0x895D6544`, 28 slots,
  §3), `+0x04` `0x00AB39D0` (the draw interface the draw registry calls: slot 3 `Draw`, `0x00739490`), `+0x08` a
  `cRZUnknown`, `+0x10` the message target. `QueryInterface` (`0x00735260`) answers `0x895D6544` with the object,
  anything else through the `cRZUnknown`.
- `Init(view, dynamic, water)` (`0x0073CB60`): keeps the view (`+0x8C`), the two flags (`+0x15` dynamic, `+0x16`
  water) and the initialised flag (`+0x14`); takes the texture binding factory (`+0x90`) and the terrain (`+0x88`);
  from the terrain's cell counts (`+0x5C`, `+0x60`) and cell width the cell scale (`+0x68` = 1 ÷ width) and the city
  size in metres (`+0x6C`–`+0x74`); the binding for key `0xAA592B77` *[inferred: the shadow texture of §4.3,
  `+0xD4`]*; the draw states (§4.1); for a static manager the per-cell index (`0x0073CB10`, §2.2); subscribes to
  `0xC9F775BB`; and reads `kShadowQuality` (integer 12, `+0x18`) and *[inferred]* `kShadowDebug` (integer 4,
  `+0x1C`).
- The terrain view creates four and initialises them as (static, land) `+0x78`, (static, water) `+0x7C`,
  (dynamic, land) `+0x80`, (dynamic, water) `+0x84`, registered at draw layers 2, 2, 5, 5 with priorities 0, 1100,
  1000, 1100 (terrain.md §15.5.4). The terrain keeps the first at its `+0xF8`.
- `0x0073AC90` (on shutdown) unsubscribes, releases the view, factory and terrain, empties both record tables and
  forgets the free lists.

### 1.1 Initialization and repeat calls (OV4, proven subset)

The constructor (`0x0073BC20`) sets initialized false, shadow quality 2, second-stage setting 0; parent, terrain,
factory and the three arenas' begin/end/capacity pointers are null. Each arena has free-head and free-tail −1.
It sets the shadow binding and index storage pointers null. It does **not** initialize the dynamic/water bytes,
cell dimensions, reciprocal scale, city rectangle or counters; those are established by Init. No public
getter/draw contract is supplied before Init.
`0x0073BC6A–0x0073BC79`, `0x0073BCF5–0x0073BD9B`.

Init (`0x0073CB60`) takes `(parent, dynamicByte, waterByte)`, returns no specified result and has no initialized
guard or rollback. It replaces the parent with AddRef-before-Release when different, stores both bytes and sets
initialized true **before** looking up the texture factory (framework class/interface `0x1AC0E11A/0xFAC0E219`).
It similarly retains the new factory and global terrain and releases replaced references. Identical pointers
avoid those reference operations. Terrain slots 7/8 provide signed cell counts; slot 6 returns the cell width.
Init stores width as binary32, reciprocal as binary32 of the x87 reciprocal, and city bounds `(0,0,nx,nz)`.
It resets counters and `+0x94`, releases any old `+0xD4` binding, then requests key `0xAA592B77` through
**factory slot 5** (byte offset `0x14`), into `+0xD4`. It initializes draw states, rebuilds the static index when
dynamic is zero, registers message `0xC9F775BB` when that global server exists, then reads quality/second-stage
settings. Repeated Init executes this sequence again and does not clear the overlay/handle/group arenas.
Missing factory/terrain/framework services do not produce a clean false result: required pointers are later
dereferenced without checks, after earlier mutations. Atomic rejection of repeated/failed Init is not native
behavior. Evidence: `0x0073CB60–0x0073CD3B` (including binding call `0x0073CCE7`).

The overlay-record constructor (`0x00735F40`) sets flags 1 (shown), draw-state byte 0 and collection stamp 0,
first matrix identity, alpha 1, colour from the three binary32 defaults at `0x00B0D4E4/E8/EC` (all 1 in the
shipped image), and binding/source pointers null. The cell rectangle and first free-list word are not initialized
there; allocation sets the live word −1 and placement establishes bounds before observation. The second matrix
is filled from scratch/default global `0x00B4B3C8`; no usable second-transform value is promised until a shadow
add fills it. Likewise the handle allocator copies a complete owner-constructed placement/aspect/target record;
there is no independently initialized public default handle. Evidence: `0x00735F40–0x007360F9`, with alpha's
literal 1 established at `0x00735FF2`, colour loads `0x007360BE–0x007360DC`, and allocator §2.3.

## 2. Records and handles *[compat]*

### 2.1 Two tables

- **Overlays** (`+0x98`, 0xB4 bytes each, with a free list `+0xA4`, `+0xA8`): a free-list link (−1 while in use),
  flags (`+0x04`: bit 0 shown, bit 1 repeat the texture, bit 3 shadow, bit 4 decal, bit 5 a texture matrix is set,
  bit 6 self-lit, bit 7 drawn in snapshots), the draw state index (`+0x08`, §4.1), the covered cell rectangle
  (`+0x0C`, x0, z0, x1, z1), a 4 × 4 world-to-texture matrix (`+0x1C`), the alpha (`+0x9C`), the colour (`+0xA0`, RGB),
  the texture binding (`+0xAC`) or a texture source object (`+0xB0`).
- **Handles** (`+0xAC` table, 0x30 bytes each): what callers hold. A handle with the high bit set names a handle
  record: its placement (nine floats from `+0x04`) and the overlay it drives (`+0x2C`). An overlay index whose high
  bit is set names a **group** (`+0xC0`, 0x24 bytes: eight overlay indices), and every call on a group is applied to
  its eight parts where the method supports group recursion. Handle −1 is "none"; scalar getters return their
  sentinels and placement getters leave their output untouched (§2.3).

Counters: the decals and shadows drawn in the last frame (`+0xD8`, `+0xDC`).

### 2.2 The cell index (static managers)

A static manager keeps, per terrain cell, the overlays that cover it (`0x00738230` adds an overlay's rectangle,
`0x007382E0` removes it), so that a frame only visits the overlays of visible cells (§4). Any change to an overlay
marks it changed (`0x00737D40`); a round decal (slot 7) also redisplays the terrain under each part's rectangle
(terrain slot 68).

Native bounds are half-open for cell iteration and redisplay: positive area requires x1>x0 and z1>z0.
Intersection helper `0x0048ED90` uses signed comparisons; a strictly separated rectangle becomes all zeros,
whereas touching edges retain a zero-width/height intersection at that edge. Thus clipping `(−3,2,−1,4)`
against `(0,0,8,8)` gives `(0,0,0,0)`, while `(−3,2,0,4)` gives `(0,2,0,4)`. Bounds are not swapped to
normalize inverted input. `MarkChanged` invokes terrain slot 68 `(0,waterByte,&rect,0)` only for positive area
(`0x00737D40–0x00737DA3`).

`BuildCellIndex` establishes shift 0, grid nx by nz, nx*nz null list heads and a zero 16-bit collection stamp
(`0x0073CB10–0x0073CB57`). For shift k in the indexing routines, covered coarse bounds are arithmetic shifts
`[x0>>k,z0>>k,(x1+(2^k−1))>>k,(z1+(2^k−1))>>k]`. Addition traverses X outermost, Z innermost and **prepends**
the overlay ID to each list, without duplicate suppression (`0x00738230–0x007382D8`). Removal traverses Z
outermost, X innermost and unlinks every matching ID while preserving others' order (`0x007382E0–0x0073838B`).
Collection appends on first encounter, using overlay `+0x0A` versus the manager's incremented 16-bit stamp;
it does not sort the selected IDs (`0x00738480–0x007384B7`, `0x0073854E–0x00738580`). Exact visible-row endpoint
inputs and stamp-wrap effects remain under OV8. A sorted/deduplicated host selection is not this owner algorithm.

### 2.3 Allocation, reuse, getters and invalid records (OV4/OV5)

All three arenas allocate ascending zero-based indices when no freed entry exists. Removal appends to a
free **queue**, and allocation takes its head: reuse is **FIFO**, independently for overlays, handles and groups.
The first word is −1 while live; a free entry's word links to the next (the tail word is set to 0 and is not
followed when head equals tail). No generation is encoded in an ID. A stale handle may therefore operate on a
new occupant of the same slot. There is no portable result for an out-of-range ID or null required pointer;
native paths dereference the computed address, rather than rejecting the operation atomically.
Evidence: handle allocate/free `0x00739EC0–0x00739FB1` / `0x00736210–0x0073624D`, group allocate
`0x00739FC0–0x0073A06D`, overlay allocate `0x0073AEC0–0x0073AF7B`, removal `0x007388C0–0x00738A08`.

Ordinary source/binding decals allocate an overlay **and** an indirect placement handle, even if the clipped
rectangle is empty. Round decals allocate eight overlays, a group and one indirect handle. The city-wide
rectangle add returns a **direct overlay ID**, without a placement handle: not every add allocates one.
Slots 12–14 ignore nonnegative direct IDs. For indirect removal, the handle is freed first; group removal
recurses over its eight indices in array order and then frees the group. Direct overlay removal on a static
manager removes its index entries and marks the old rectangle before checking whether its free-list word
already says free. Payload is left in freed records (including binding/source references); removal itself
does not release them. Reuse/arena destruction, rather than removal, disposes retained payload references.
There is no general live-record validation on setters/getters. Evidence: `0x0073B127–0x0073B141`,
`0x0073C069–0x0073C082`, `0x0073B2CF–0x0073B38E`, `0x0073B9A7–0x0073BA04`, `0x007388F2–0x00738A08`.

Scalar getters accept direct overlay IDs or resolve an indirect handle. If that handle resolves to a group,
they read **part 0** only, never a reduction/average across parts. A group ID by itself is not a public handle.

| Slot | Actual value | −1 behavior |
|---|---|---|
| 22 (`0x00736580`) | shown: `(flags & 1)`, boolean | false |
| 23 (`0x007365E0`) | binary32 alpha at `+0x9C`, returned on x87 stack | +0 |
| 24 (`0x00736640`) | three colour words from `+0xA0` | writes three zero words |
| 25 (`0x007366C0`) | zero-extended **byte** at `+0x08` | 0 |
| 26 (`0x00736720`) | self-lighting: `(flags >> 6) & 1`, boolean | false |

Slot 24 has output first and handle second (callee pops 8); it returns the output address in EAX. No null
output check exists, even for handle −1. Slot 14's no-handle/direct-ID path, in contrast, does not touch output.
These identities follow the field loads, not the earlier descriptive function name `IsShown` for slot 26.

Slots 16/18/19 write shown/colour/draw-state and mark a nonempty static rectangle even when supplied values
are identical; dynamic writes omit redisplay. Slot 19 writes only the low byte. Slot 13 always re-places;
static re-placement with the same clipped rectangle still marks it once, and a changed rectangle removes old
index entries, marks old, writes new bounds, inserts new entries, then marks new. Slot 12's tolerance is in §3.2.
Evidence: `0x00737EB8–0x00737EED`, `0x00738078–0x007380B0`, `0x00738128–0x0073814D`,
`0x007393DF–0x0073943E`. These are owner operations, not caller-supplied computed rectangles or redraw results.

Paper reuse vector: allocate indices 0,1,2; remove 1 then 0; the next allocations reuse 1 then 0, then append 3
if no other frees occur. For a group with differing part colours/alpha, getters observe part 0 only.

## 3. The interface (`0x00AB39E8`) *[compat]*

| Slot | VA | Call | Behaviour |
|---|---|---|---|
| 3 | `0x0073C0B0` | `AddDecal(source, positionXZ, width, turns)` | a decal textured from a texture source object (its size from the object's slot 3) |
| 4 | `0x0073BF10` | `AddDecal(binding, positionXZ, width, turns)` | the same from a texture binding; −1 without one |
| 5 | `0x00736150` | `AddDecal(key, positionXZ, width, turns)` | the factory's binding for the key (factory slot 6), then slot 4 |
| 6 | `0x007361B0` | `AddDecal(key, positionXZ, width, turns)` | the same with factory slot 5 |
| 7 | `0x0073B170` | `AddRoundDecal(key, positionXZ, width, turns)` | a **group** of eight parts covering a disc (§3.1) |
| 8 | `0x0073C9A0` | `AddCityDecal(key)` | one overlay over the whole city (`0x0073B8A0`, flags `0x12`) *[inferred]* |
| 9 | `0x0073B3B0` | `AddShadow(binding, …)` | a shadow overlay |
| 10 | `0x00735480` | `AddShadow(…)` | −1 when `kShadowQuality` is 0; else the factory's shadow binding (factory slot 11) and slot 9 |
| 11 | `0x0073C0D0` | `AddModelShadow(…)` *[inferred]* | a shadow built from a model's footprint |
| 12 | `0x0073AD80` | `SetPosition(handle, xz)` | for a handle record: when the two floats change, the overlay is placed again |
| 13 | `0x0073ADE0` | `SetPlacement(handle, nine floats)` | the whole placement, then placed again |
| 14 | `0x00736490` | `GetPlacement(handle, out)` | the nine floats |
| 15 | `0x007388C0` | `Remove(handle)` | a group's parts are removed, then the record is freed |
| 16 | `0x00737E50` | `SetShown(handle, on)` | flag bit 0 |
| 17 | `0x00737F00` | `SetAlpha(handle, a)` | `+0x9C`; a static manager only redraws when a reaches 0 or 1 or moves by more than the step `0x00AB39A0` |
| 18 | `0x00738010` | `SetColor(handle, rgb)` | `+0xA0` |
| 19 | `0x007380C0` | `SetDrawState(handle, index)` | `+0x08` |
| 20 | `0x007364F0` | `EnableSelfLighting(handle, on)` (descriptive name) | when the low bit of `on` is set, OR flag bit 6; false does not clear it; groups recurse over all eight parts |
| 21 | `0x00738160` | `SetTextureScale(handle, size, offset)` | the matrix becomes a scale of 1 ÷ size with the second axis flipped and an offset of 1 + offset ÷ size; flag bit 5 |
| 22–26 | `0x00736580`–`0x00736720` | getters | shown; alpha; colour (output first); draw-state byte; self-lit (§2.3) |
| 27 | `0x007352A0` | `GetStatsString(out)` | "%d decals, %d shadows" from the counters |

Slots 0–2 are `QueryInterface`, `AddRef`, `Release`. The ordinary decal add signatures in the table above use
`positionXZ, width, angleInTurns`, rather than an input nine-float record. Slots 5/6 forward all three. The exact
stored placement and owner computations are in §3.2. `ComputeCellRect` (`0x007354E0`) is a different helper;
its margin must not be imported into ordinary `PlaceOverlay` coverage.

### 3.1 Round decals (`0x007394B0`)

A disc of radius r is split into eight rectangles of cells that follow its outline (half-chords of the circle at
the cells' rows); each becomes one part of the group, flagged as a part (bit 4), so that the decal touches only the
cells inside the disc *[inferred: the coverage circles shown when placing a building]*.

### 3.2 Placement record and owner calculations (OV1/OV5)

The public record is nine consecutive **binary32** values, copied bit-for-bit by slots 13/14. It is not a
three-dimensional centre and three-dimensional extents. Offsets below are relative to the public record;
the handle stores it at `+0x04` and also stores an owner-derived texture aspect at `+0x28`.

| Offset | Name | Consumed by `PlaceOverlay` |
|---|---|---|
| `00`, `04` | cx, cz | world X/Z centre; the only values changed by slot 12 |
| `08` | w | full world width |
| `0C` | turns | rotation; multiplied by binary32 `0x40C90FDB` (6.283185482025146484375), then native x87 sine/cosine |
| `10` | stretch | multiplies the retained texture aspect, making depth/width |
| `14`, `18` | repeatU, repeatV | multipliers of the two texture coordinates, independent of footprint size |
| `1C` | offset | added to **both** texture coordinates |
| `20` | retained | copied and returned, but not read by this placement routine |

Ordinary binding/source adds construct `[cx,cz,w,turns,1,1,1,0,0]`. The binding form obtains texture dimensions
from factory slot 8 and retains binary32 `height/width` separately; the source form obtains dimensions from
source slot 3. This retained aspect is not overwritten by slot 13. The round add constructs the same defaults
and likewise retains the binding aspect. Evidence: `0x0073AFB2–0x0073B073`, `0x0073BF33–0x0073BFC1`,
`0x0073B1AE–0x0073B237`, copies `0x007364AE–0x007364E9`, `0x0073AE05–0x0073AE37`.

Let F mean a binary32 store/reload, E native x87 arithmetic under the caller's control word (this routine does
not change it), a = F(stretch × retainedAspect), hx = F(w × 0.5), hz = F((a × w) × 0.5). The multiplication
forming a is stored before use. In the zero-angle path the world-to-texture matrix is stored as four consecutive
rows, used with a **row vector** `[x,y,z,1]`:

| Row | Four entries |
|---|---|
| 0 | A, 0, 0, 0 |
| 1 | 0, 0, 0, 0 |
| 2 | 0, D, 1, 0 |
| 3 | Tu, Tv, 0, 1 |

The finite, nonzero-size evaluation is A = F(repeatU × E(1/w)), R = E(E(1/a) × E(1/w)),
D = F(repeatV × F(R)), Tu = F(F(repeatU × F((hx−cx) × E(1/w))) + offset),
Tv = F(F(repeatV × F((hz−cz) × R)) + offset). The first reciprocal is kept in x87 precision;
R is also stored to binary32 before the multiplication yielding D. This gives the familiar real-number
coordinates `u=repeatU*(0.5+(x−cx)/w)+offset`, `v=repeatV*(0.5+(z−cz)/(a*w))+offset`, but that algebraic
simplification is not the rounding contract. Zero products in the matrix are actually floating multiplications
in several positions; the displayed zeros describe ordinary finite inputs, not NaN propagation or signed-zero
canonicalization. Evidence: `0x0073A1C1–0x0073A34A`, matrix stores `0x0073AB99–0x0073ABF8`.

For nonzero turns the owner rotates world X/Z about `(cx,cz)` before the above transform. With
c = F(cos(E(turns × binary32(2π)))) and s = F(sin(E(turns × binary32(2π)))), its real-number equations are
`x′=c*(x−cx)−s*(z−cz)+cx`, `z′=s*(x−cx)+c*(z−cz)+cz`. The matrix also transforms its third output coordinate;
it is not a two-dimensional texture-only rotation. Both signed zeros take the no-rotation branch; unordered
angle comparison takes the rotation branch. Full rotated coefficient rounding remains part of OV1 until
the ordered intermediates are specified. Evidence: `0x0073A34E–0x0073A63A`.

Repeat flag bit 1 is set unless **both bit patterns** repeatU and repeatV are `0x3F800000`.
The unrotated footprint uses hx/hz, regardless of repeat factors or offset. For a rotated placement with
both repeat factors exactly one, replace hx by F((abs(c) + a*abs(s)) × hx) and hz by
F((abs(c) + F(1/a)*abs(s)) × hz), with sums/products kept in x87 precision until those stores.
For any other repeat factors this footprint expansion is skipped. Four cell bounds are then stored to binary32:
`[(cx−hx)*cellScale,(cz−hz)*cellScale,(cx+hx)*cellScale,(cz+hz)*cellScale]`.
The owner converts minima to floor and maxima to ceil for normal finite city coordinates under round-to-nearest,
then intersects with the city's cell rectangle. No 0.07 margin is applied here. Exact conversion and cell-index
ordering are tracked under OV8. Evidence: `0x0073A5F5–0x0073A675`, `0x0073ABFB–0x0073AC6F`,
conversion helper `0x00735380–0x00735475`.

Slot 12 ignores -1 and nonnegative direct overlay IDs. It repositions only when either absolute coordinate
difference is **at least** binary32 `0x38D1B717` (0.00009999999747378752), or is unordered. Otherwise neither
coordinate is written. Slot 13 copies all nine values and re-places even for identical values. Slot 14 ignores
-1/direct IDs and leaves output untouched; for an indirect ID it copies all nine words. There are no null-pointer,
range, free-record or generation checks on these paths; callers must supply live records and valid pointers.
Evidence: `0x007352D0–0x0073531A`, `0x0073AD80–0x0073AE52`, `0x00736490–0x007364EE`.

#### 3.2.1 Ordered rotated coefficients

The following completes the coefficient order for finite nonzero size/aspect; it does not substitute a generic
matrix multiplication (which can round differently). All operations between explicit F stores use E, with
left-to-right sums as parenthesized. Start with rw=E(1/w), R=E(E(1/a)*rw), q=F(0*rw),
du=F((hx−cx)*rw), dv=F((hz−cz)*R), rv=F(R). The unrotated coefficients before rotation are
A=F(repeatU*rw), B=F(repeatU*q), C=F(repeatU*q), P=F(repeatV*F(0*R)),
Q=F(repeatV*E(0*R)), S=F(repeatV*rv). Keep T=E(F(repeatU*du)+offset) in x87 precision;
V=F(F(repeatV*dv)+offset) is stored. Unrotated rows are `[A,P,0,0]`, `[B,Q,0,0]`, `[C,S,1,0]`,
`[F(T),V,0,1]` (the middle rows' last two zeros are literal integer zero stores).

When turns is ordered equal to zero, use those rows directly. Otherwise take c/s as in §3.2, n=F(−sin(angle)),
DX=F(cx*(1−c)+cz*s), DZ=E(cz*(1−c)−cx*s), b0=F(B*0), q0=F(Q*0), t0=F(T*0),
v0=F(V*0), c0=F(c*0), n0=E(n*0). The double literal 1 is used in `1−c`. Then store:

| Row | Four binary32 entries, each rounded at its enclosing F |
|---|---|
| 0 | F(((A*c+C*s)+t0)+b0), F(((P*c+S*s)+v0)+q0), F(c0+s), F(s*0+c0) |
| 1 | F(((A+C)*0+t0)+B), F(((P+S)*0+v0)+Q), +0, +0 |
| 2 | F(((C*c+A*n)+t0)+b0), F(((S*c+P*n)+v0)+q0), F(c+n0), F(c0+n0) |
| 3 | F(((C*DZ+A*DX)+b0)+T), F(((S*DZ+P*DX)+q0)+V), F(DX*0+DZ), F((DZ*0+DX*0)+1) |

The two translated texture entries are stored before the other rotated coefficients; DX is rounded but DZ
remains in x87 precision until those uses. Source: `0x0073A1E2–0x0073A34A`, rotation and stores
`0x0073A364–0x0073A5F1`. No additional angle reduction, trigonometric snapping, FMA, or matrix normalization
is performed by this owner. The trigonometric operation itself follows native x87 behavior and caller FP state;
exceptional-angle/size outcomes are not replaced with a fabricated finite rectangle.

### 3.3 Alpha acceptance (OV6)

Slot 17's argument and stored alpha are binary32; the threshold at `0x00AB39A0` is **binary64**
`0x3FA999999999999A` (the double representation of 0.05). A static manager first rejects an ordered equal
old/new value. Otherwise it accepts new alpha exactly 0 or 1, or an ordered absolute x87 subtraction result
**strictly greater** than that double threshold. A rejected sub-step change does not update stored alpha;
the next call compares against the last accepted value. An accepted write stores the supplied bits and calls
`MarkChanged`. There is no clamp: negative values, values above one and infinities can be accepted. New NaN is
rejected in the static path (including when old is NaN); with old NaN only an endpoint new value is accepted.
A dynamic manager stores every input, including equal values and NaN, without calling `MarkChanged`.
Groups recurse through all eight parts in order; -1 is ignored. Evidence: `0x00737F00–0x00738000`, in particular
the double comparison at `0x00737FCB` and rejection at `0x00737FD6`.

First exact paper vectors (not an executed native/game verdict):

| Input/state | Required result |
|---|---|
| Placement `[32,48,16,0,1,1,1,0,99]`, retained aspect 2, cellScale 1/16, city containing it | a=2, hx=8, hz=16; matrix rows `[1/16,0,0,0]`, `[0,0,0,0]`, `[0,1/32,1,0]`, `[-1.5,-1,0,1]`; cell rectangle `(1,2,3,4)`; repeat clear; retained 99 has no placement effect |
| Same with repeatU=2 and offset=1/4 | A=1/8, Tu=−2.75, Tv=−0.75; footprint unchanged; repeat set |
| Slot 13 receives the same nine words again | copy and placement still occur |
| Slot 12: old `(0,0)`, new `(2^−14,0)` | no write, because 2^−14 is below the coordinate threshold |
| Static alpha old 0; write binary32 `0x3D4CCCCC` | rejected, stored 0 |
| Static alpha old 0; write binary32 `0x3D4CCCCD` | accepted: 0.05000000074505806 is greater than double 0.05 |
| Static alpha old 0; write 0.03125 then 0.0625 | first rejected; second accepted relative to stored 0 |
| Static alpha old 1; write 1 | no write/redraw |
| Dynamic alpha old 1; write a quiet NaN | stores that NaN, no redraw call |

### 3.4 Slot 21: exact replacement matrix (OV7, proven subset)

For a single overlay, slot 21 replaces all sixteen matrix entries; it does not compose a scale with placement.
Let r = E(1/size), t = F(E(r × offset) + 1). The consecutive binary32 rows are:

| Row | Four entries |
|---|---|
| 0 | F(r), +0, +0, +0 |
| 1 | +0, F(−r), +0, +0 |
| 2 | F(r), +0, 1, +0 |
| 3 | +0, t, +0, 1 |

Thus the first coordinate is `(x+z)/size` and the second is `−y/size+1+offset/size` in real-number terms,
not a simple world X/Z rectangle mapping. The reciprocal remains in x87 precision until each store; t uses
that reciprocal, not a rounded reciprocal. The owner ORs flag bit 5 and marks the rectangle on static managers,
even for equal input. There is no size-zero guard or clamp: with masked FP exceptions, size +0 gives +infinity,
−infinity, +infinity at indices 0,5,8, and a NaN at index 13 when offset is zero. Negative zero reverses the
infinities. Unmasked FP exceptions follow caller FP state. The method ignores −1, resolves an indirect handle,
but **does not recurse over groups**; a handle resolving to a group is an unsupported unsafe native call.
Evidence: `0x00738160–0x00738220`, especially stores `0x007381D7–0x00738207`.

Paper vector: size=16, offset=8 gives rows `[1/16,0,0,0]`, `[0,−1/16,0,0]`, `[1/16,0,1,0]`,
`[0,1.5,0,1]`; input `[16,8,16,1]` produces `[2,1,16,1]`.

## 4. Drawing (`Draw`, `0x00738A50`) *[compat]*

A dynamic manager is not drawn under `kRenderSnapshot` (boolean 28). Otherwise:

1. The model-view is reset to the view and inverted, so that a camera-space position (texture coordinate set 16)
   can be taken back to the world; each overlay's texture transform is that inverse followed by its own matrix.
2. The overlays: for a static manager with its cell index, those that cover visible cells; otherwise every overlay
   in use.
3. With `kRenderDecals` (boolean 10): the decals (§4.2). With `kRenderShadows` (boolean 9): the shadows, the rough
   form (`DrawShadowsRough`, `0x00737870`) when `kShadowQuality` < 3 or `kUseSecondStage` is off, else the full form
   (`DrawShadows`, `0x00736BF0`).
4. The texture transform is cleared and the depth offset restored.

### 4.1 Draw states (`InitDrawStates`, `0x0073BA10`)

Five packed states, all with colour write, depth test, blending and one texture, and stage-0 mode 1 except state 4
(which keeps the mode):

| Index | Depth write | Depth function | Blend | Alpha test |
|---|---|---|---|---|
| 0 | no | less-or-equal | source alpha / one minus source alpha | no |
| 1 | no | less-or-equal | source alpha / one (additive) | no |
| 2 | no | less-or-equal | zero / source colour (multiply) | no |
| 3 | no | greater | source alpha / one minus source alpha | no |
| 4 | yes | less | source alpha / one minus source alpha | greater than 0.5 |

### 4.2 Decals (`DrawDecals`, `0x00736790`)

Depth offset 2. Each overlay in use whose flags have bits 0 and 4 (and bit 7 under `kRenderSnapshot`): the alpha as
the transparency, the colour as the shade (self-lit with bit 6) with vertex colours off for a self-lit decal, its
draw state, its texture (filters linear / linear-mipmap-linear), wrap clamp / clamp, or repeat / repeat with bit 1,
or repeat / clamp with bit 5, the texture transform, then its cell rectangle is drawn (`DrawRect`, `0x00735720`)
from the terrain view's land or water arrays (`0x007541C0`–`0x00755700`, by the water flag, the dynamic flag and
the vertex-buffer extension). Transparency, shade and vertex colours are restored.

### 4.3 Shadows (`DrawShadows`, `0x00736BF0`; `DrawShadowsRough`, `0x00737870`)

Depth offset 3, draw state 4, the shadow colour and strength from the 3D view manager (`0x00B43DD0` slot 9, then
slot 14) as the environment colour, alpha testing at "greater or equal" scaled by the strength when the strength is
above a minimum, a texture combiner; the full form adds a second stage with the shadow texture (`+0xD4`, clamped,
point-sampled) and a second texture transform, and on land reads a terrain height there (terrain slot 15)
*[inferred]*. Each shadow's cells are then drawn as for decals.

## 5. Messages

`0xC9F775BB` is subscribed by `Init` and unsubscribed at shutdown *[inferred: a terrain or zoom change that makes the
overlays be placed again]*.

## 6. Users

The terrain view creates and draws the managers; the tools, lots, buildings and effects add and update decals and
shadows through the terrain view's `GetOverlayManager(index)` (slot 18) or the terrain's `+0xF8`.

## 7. Quirks

1. A dynamic manager's overlays vanish from snapshots (§4).
2. Alpha changes smaller than the step are not redrawn in a static manager (§3, slot 17).
3. Every call on a group repeats on its eight parts, so a round decal costs eight overlays (§3.1).

## 8. Open questions

See [questions/overlay-manager.md](questions/overlay-manager.md) OV1–OV3: the placement record's nine floats and
the add calls' last arguments; slot 20 and the add forms of slots 8, 9 and 11; what message `0xC9F775BB` is.

## 9. Conformance tests

1. **Handles:** a call with handle −1 does nothing.
2. **Groups:** `SetShown(group, false)` hides all eight parts.
3. **Shadow quality:** with `kShadowQuality` 0, slot 10 returns −1.
4. **Texture scale:** `SetTextureScale(h, 16, 0)` sets flag bit 5 and a matrix scale of 1/16.
5. **Snapshot:** a dynamic manager draws nothing while `kRenderSnapshot` is on.
6. **Stats:** after drawing 3 decals and 2 shadows, slot 27 gives "3 decals, 2 shadows".

### PR review: slot 20 self-lighting

The body at `0x007364F0` ignores handle -1, resolves indirect handles as in section2.1, and applies itself recursively to all eight indices of a group. For a resolved overlay it sets flag bit6 only when the low bit of the second argument is1. It does not clear the bit for0, and makes no redraw call. This is a two-argument member call, callee pops8; no return value is specified. Evidence: branches at `0x007364FA`, `0x00736515`, recursion at `0x00736548`, bit test at `0x00736558` and flag update at `0x0073656F`.

Conformance: starting with flags0, calls with on=0,1,0 yield flags0,0x40,0x40. A group applies that same operation to its eight indices; -1 leaves every record unchanged.
