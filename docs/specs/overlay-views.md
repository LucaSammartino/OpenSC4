# Spec: overlay views (`cSC4PathView`, `SC4MarkedCellView`, `cSTEVertBufExtn`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [render3d.md](render3d.md) (§11 view objects: these views are registered there and drawn in
their layer), [draw-context.md](draw-context.md), [render-properties.md](render-properties.md),
[terrain-view.md](terrain-view.md) (§4, which uses the vertex-buffer extension), [view-utilities.md](view-utilities.md)
(frustum tests).

## 0. Summary

Two small view objects draw marks over the city: the **path view** draws routes as ribbons with arrowheads, and
the **marked-cell view** shades a set of cells, following the terrain, the water or the lots on them. The
**vertex-buffer extension** is the terrain view's optional path for submitting terrain rows through a driver
vertex buffer.

## 1. The path view (`cSC4PathView`) *[compat]*

- **State:** a list of paths ordered by priority, a thickness t (`SetThickness`, `0x008B6DE0`), the current path
  texture (`SetPathTexture`, `0x007DCE20`, through the texture binding factory), and two packed render states made
  by `Init` (`0x007DCD90`): plain and textured (one texture), both colour write with no depth write and no depth
  test, culling, blended (source alpha, one minus source alpha) unless boolean 33 `kPreferOpaque` is on.
- `NewPath(colourBits, priority, flags)` (`0x007DDC50`): a new path, inserted before the first path whose
  binary32 priority compares greater than or equal to the new priority (equal priorities are newest first;
  unordered comparisons continue searching). Flag bit 0 draws an arrowhead at the start, bit 1
  at the end.
- `AddPathVertex(p)` (`0x007DDB30`): appends p to the newest path, unless it lies within √(10⁻⁵) m of the previous
  vertex in x and z.
- `ClearPaths` (`0x007DDBE0`) removes every path.
- `Update(context)` (`0x007DD9B0`, the view-object draw): with any path, depth offset 15, the identity model
  transform, then each path in order (`DrawPath`), then the depth offset is restored.

### 1.1 Drawing a path (`DrawPath`, `0x007DD410`)

A path of n ≥ 2 vertices becomes one triangle strip of 2n vertices (vertex format 10: position, colour, uv):

- **Offsets:** ends use ±t times the horizontal unit normal; joints use the squared-length miter
  rule with an unscaled outgoing-normal fallback (§1.2).
- **Texture coordinates:** first coordinate is the per-segment longitudinal projection scaled by
  r ÷ (2t); second coordinate is side 0/1 (§1.2).
- **Arrowheads** (`DrawArrowhead`, `0x007DD0D0`): with the start or end flag, a single triangle (format 10) is drawn
  at that end along the end segment, sized from t, and the ribbon's end point moves back to the arrow's base; no
  arrow is drawn when its available 3D segment is shorter than 2t. See §1.2 for exact geometry.
- A path with a texture draws with the textured state, that texture and repeat wrapping; the plain state is
  restored after.

### 1.2 Exact path record, geometry and boundaries (OVV2–OVV3)

`NewPath`'s first 32 bits are the **packed vertex colour**, copied unchanged to DrawPath's colour
field (`0x007DD427–0x007DD42C`); no separate opaque ID lookup, default white colour, or colour setter
is established by these bodies. Priority is a **float**, not a signed or unsigned integer
(`0x007DDC64–0x007DDC76`). Search continues only while oldPriority < newPriority; equal values insert
before the old one. The branch tests x87 C0 only: an unordered comparison (NaN) continues searching,
just like less-than (`0x007DDC6D–0x007DDC76`). NewPath selects this newly
inserted record for subsequent AddPathVertex/SetPathTexture, irrespective of its list position.

Relative to a record (excluding its two list links): +0 colour bits, +4 flags, +8 binary32 priority,
+0x0C..+0x33 owned point deque, +0x34 texture binding, +0x38 binary32 repeat factor. NewPath creates
an empty deque and a null binding. It copies the repeat slot from an **uninitialized stack slot**;
there is no native default repeat factor to promise. `SetPathTexture(key, repeat)` takes **two**
arguments, releases and nulls the selected record's previous binding, asks factory slot 5 for the
new one, then stores the second argument as repeat, whether or not loading succeeded. It does not
set a process/view-wide texture for future paths (`0x007DCE20–0x007DCE5F`). With no selected path,
this method has no sentinel guard; do not invent successful empty-list semantics.

The constructor sets thickness to binary32 2, initialized=false, both packed state words/masks to
zero, and an empty circular path list with selected=sentinel (`0x007DDD50`). SetThickness simply
stores its float. AddPathVertex without a selected path is a no-op. Otherwise it rejects the new
point only when dx²+dz² < **binary64 1e−5**, using x87 arithmetic; y is ignored, equality appends,
and NaN does not trigger the reject (`0x007DDB73–0x007DDB99`, constant `0x00A95620`). There is no
finite/thickness validation. ClearPaths releases each texture and point deque and empties the list;
its body does **not reset selected**. Thus AddPathVertex/SetPathTexture after clearing a nonempty
list requires a new NewPath; otherwise selected is stale. Allocation failure is not translated to
a public false result. There is no path bounding-box cache, bounds getter, selection by ID, terrain
projection, or dirty/update invalidation in these bodies: Update rebuilds world-space geometry from
the list every draw. Empty list Update does nothing and succeeds; n<2 draws no primitives.

For ordinary finite, nonzero horizontal segments define N(A,B) = (dz,0,−dx)/sqrt(dx²+dz²).
An end emits P+tN followed by P−tN. At a joint let M=(N(previous,P)+N(P,next))/2 and q=M·M.
If q < **binary64 0.0001**, emit P±N(P,next), **without multiplying by t**. Otherwise emit
P±M×(t/q): this is a miter, not a normalized unit bisector. Equality takes the miter branch.
Evidence: `0x007DCE60`, `0x007DCFF8–0x007DD09F`, constant `0x00A84E60`. Zero horizontal
segments divide by zero; neither normal helper supplies a safe finite substitute.

DrawArrowhead receives mutable tip P and inward neighbor Q. Let L=|Q−P| in **3D**, and
a=min(binary32(L−t), binary32(binary64(4.5)×t)). If a<t, return false without drawing or moving P;
otherwise B=P+(Q−P)×a/L. Draw one triangle in order **P, B−2tN(P,Q), B+2tN(P,Q)**, format 10,
primitive 0, with the supplied packed colour at each vertex and zero texture coordinates for finite
inputs; then replace P with B. No separate arrow texture state is selected. This means an ordinary
positive-thickness arrow first becomes possible at L=2t. Evidence: `0x007DD10F–0x007DD160`
(4.5 is a double at `0x00ABB628`), `0x007DD188–0x007DD37B`.

Start arrow is drawn while processing the first segment, before its strip pair; the end arrow is
drawn after processing all points and before the final pair. Both precede the ribbon draw. With just
two points, the end arrow sees the already-shortened first point. Original L=4t therefore draws only
the start arrow, moves it 3t inward, and leaves the last t for the strip; both flags do not force two
triangles. Original L=6.5t draws the start at 4.5t and end at t, leaving a t-long strip.
Only the local endpoint copies move; the stored point deque is unchanged.

Format 10 has position, packed colour, then **(longitudinal coordinate, side coordinate)**, not the
reverse. Side is 0 for the plus-normal vertex and 1 for minus. Let k=repeat/(2t), s initially 0.
For each working segment A→B, D=(B−A)×k/|B−A|₃D, b=s−D·A. Both vertices X of the pair at A
receive longitudinal D·X+b; then s increases by |B−A|₃D×k. The final pair uses the last D,b
(computed before end-arrow shortening). Joint miters can therefore give the two sides different
longitudinal coordinates. This is not just a single accumulated value copied to both sides.
Evidence: `0x007DD410`, binary32 local stores and x87 operations through `0x007DD8B9`.

Independent geometric discriminators, with t=2 and horizontal points unless stated:

| Input | Native result |
|---|---|
| (0,0,0)→(10,0,0), no arrows | First strip pair (0,0,−2), (0,0,2); last (10,0,−2), (10,0,2). |
| Right-angle joint (0,0,0)→(10,0,0)→(10,0,10) | Joint pair (12,0,−2), (8,0,2), rather than unit-bisector offsets of length 2. |
| Exact reversal at (10,0,0), next=(0,0,0) | Fallback pair (10,0,1), (10,0,−1), even though t=2. |
| Start arrow on (0,0,0)→(8,0,0), both flags | Triangle (0,0,0),(6,0,4),(6,0,−4); end arrow suppressed. |
| First priority=1.0, then priority=1.0 | Second path draws first. |

Lifecycle residual: Init is idempotent and succeeds even without a render service, setting its flag;
Shutdown `0x0053C940` only clears that flag. Destruction clears paths and frees the list sentinel.
Complete shared COM/destructor and nonfinite bit-level behavior are not certified by this section.

## 2. The marked-cell view (`SC4MarkedCellView`) *[compat]*

- **State:** the marked cells (x, z), a colour for all of them or one per cell, the cell size (w, h) and its
  inverse (`SetCellSize`, `0x007DC1D0`), and the switches `SetShouldIgnoreDepthBuffer` (`0x007DC1A0`) and
  `SetShouldUseWaterAltitude` (`0x007DC1B0`). `SetCellColor(r, g, b, a)` (`0x007DCB30`) packs the colour into bytes.
- `Init` (`0x007DC200`): the packed state, colour write without depth write, no texture, culling, blending source
  alpha / one minus source alpha, the depth test less-or-equal unless the depth buffer is ignored; and an occupant
  filter for lots.
- `Update(context)` (`0x007DC9F0`): with any cell, depth offset 4 and the identity model transform; the cells are
  drawn in their colours (`DrawCells`); then, with the outline colour set and `kPreferOpaque` off, they are drawn
  again with the depth test "greater" and additive blending (one, one) in that colour, so the parts hidden behind
  buildings show through brighter; the depth offset is restored.

### 2.1 Drawing cells (`DrawCells`, `0x007DC3A0`)

Cells outside the terrain are skipped. Each cell becomes a quad of two triangles in vertex format 1 (position,
colour), batched up to 1,024 vertices per indexed draw.

- **16 m cells** (w = h = 16): the corners are the terrain vertices of the cell, at the terrain altitude (or the
  water altitude where higher, with the water switch); the diagonal follows the terrain's split of that cell. A cell
  is skipped unless its first or third corner is in the view frustum. With lots considered, a cell under a lot that
  the filter accepts takes the lot's altitude at each corner, 15.5 m higher for lots of certain types (flags
  `0x304`), and is skipped for a lot with flag `0x100000` that does not show marks.
- **Other sizes:** clamp grid corners before scaling; sample lower terrain vertices and apply water-max;
  use a fixed diagonal (§2.2).

### 2.2 Exact marked-cell colour, corner and ordering corrections (OVV4–OVV5)

The preceding nearest-vertex/clamping description is approximate; this section controls. Constructor
defaults (`0x007DCC10`) are: ignoreDepth=true, useWater=false, useLots=false, size=(16,16), inverse
=(1/16,1/16), common packed colour **0x80C000C0** (RGBA 192,0,192,128), empty cells and colour vector,
outlineEnabled=false, zero packed state and null lot filter. The outline colour itself is not
initialized; it is unused until SetOutlineColor. Both colour setters take a pointer/reference to
**four binary32 floats in r,g,b,a order**, not four stack scalar arguments.

Conversion is the same for common and outline colours. For each channel c, add binary32 49152 in
x87, store the sum as binary32, reinterpret its bits u, subtract 0x47400000 modulo 2³², then form
the byte `(lowByte(u) − secondByte(u)) modulo 256`. Pack B in bits 0–7, G in 8–15, R in 16–23,
A in 24–31 (`0x007DCB37–0x007DCBCF`, `0x007DC2F0`; constant `0x00ABB5B4`). There is **no clamp**
or multiply-by-255 instruction. Under round-to-nearest-even, c=0,.5,1 gives 0,128,255; c=2 gives
254 and c=−.5 gives 129. Halfway c=1/512 gives 0 and c=3/512 gives 2. Nonfinite values have no
validation/reject branch; resulting bits depend on native floating-point state and payload.

SetCellColor stores the common colour and clears the per-cell colour vector's logical size, retaining
storage. Update uses common colour exactly when that vector is empty; otherwise it indexes one
packed colour per cell without a length guard. SetOutlineColor stores its packed value and sets
outlineEnabled=true. There is no proved independent disable method in this inventory; constructor
starts disabled and kPreferOpaque suppresses the second pass. No default outline colour is promised.
`0x007DCCF0` copies a vector of packed u32 colours; `0x007DCD30` appends a pair of 32-bit cell
coordinates; ClearCells `0x007DCD00` empties cells only, retaining allocation and colours. These
mutations do not deduplicate or sort. `0x007DC1C0` stores useLots. SetCellSize stores the two float
sizes and 1/size without rejecting zero, negative, infinity or NaN (`0x007DC1D0`).

Corner order is **(x,z), (x,z+1), (x+1,z+1), (x+1,z)**. The two index sequences (relative to
the first vertex of a quad) are:

| Terrain split / size | Six u16 indices |
|---|---|
| 16m, terrain split false | 0,1,2, 0,2,3 |
| 16m, terrain split true | 0,1,3, 3,1,2 |
| Other sizes | 0,1,2, 0,2,3 |

For exactly binary32 16×16, reject a cell unless its coordinates compare unsigned <W,<H. Fetch all
four terrain heights; with water enabled, replace each by its water height only if higher. **Then**
frustum-test corners 0 and 2; if neither passes, skip before looking for a lot. With useLots, city
lookup slot 32 uses the cell and the retained filter; QueryInterface ID is 0xA821EF94. The filter
allocation is 16 bytes and its construction sets mask **0x1F4F** (`0x007DC200`). An accepted lot with
flags 0x100000 and show-marks false suppresses the cell; otherwise **replace**, rather than max,
all y values with lot altitude at that corner, plus 15.5 if flags &0x304. Water is not reapplied to
lot heights and frustum is not retested after lot replacement. Temporary lot and occupant interfaces
are released even on the hidden-lot skip (`0x007DC54C–0x007DC6C7`).

For **other sizes**, independently add the corner's 0/1 grid offsets to the signed cell coordinates,
clamp those **grid coordinates** to [0,W],[0,H], and only then multiply by w,h. The world positions
are not clamped to 16W,16H and the whole cell is not rejected for an outside lower corner. Thus
large outside cells can produce collapsed quads. Select altitude indices from worldX/16 and worldZ/16
using the lower-vertex conversion: subtract .5, add binary32 12582912, store binary32, subtract
0x4B400000 from its bits, then increment only if the unrounded coordinate equals that candidate+1.
For ordinary nonnegative coordinates this is **floor**, not nearest and not half-up: 0.5→0, 0.75→0,
1→1, 1.5→1. Water-max applies here too, before the same corner-0/2 frustum test. Lots are never
queried in this branch (`0x007DC7C2–0x007DC8E7`; constants `0x00ABB5AC/0x00ABB5B0`).

There is no independent safety check for the computed terrain index with arbitrary cell sizes;
out-of-range terrain access is not a defined successful native boundary. Zero size may collapse all
corners; negative/nonfinite sizes are accepted by the setter but do not imply a safe draw. Empty
cells cause Update to return success without state changes. All cells are visited in insertion order,
four format-1 vertices and six u16 indices each, flushing after 1024 accepted vertices. No persistent
geometry/grid cache exists. Update applies identity model, offset 4, first pass, optional additive
outline pass with depth greater, then restores only the old depth offset (`0x007DC9F0`).

Independent discriminators: w=h=8, cell(1,1) has world corners (8,8),(8,16),(16,16),(16,8), and
altitude indices (0,0),(0,1),(1,1),(1,0). W=H=4,w=h=8,cell(7,7) collapses all corners to
(32,32), rather than being skipped. For a 16m cell with terrain y=2, water y=10 and lot y=3,
the frustum sees y=10 but accepted unraised lot geometry draws at y=3. Setting RGBA floats
(1,.5,0,1) gives packed **0xFFFF8000** and clears per-cell colours.

## 3. The terrain vertex-buffer extension (`cSTEVertBufExtn`) *[compat]*

A module-level helper of the terrain view (terrain-view.md §4):

- `GetVertexBufferExtension(driver)` (`0x0075CD40`): only with boolean 25 `kUseVertexBuffers`. The driver's
  interface `0x09CD86F9` makes a vertex buffer of format 11; when the made format is 11 and it holds at least two
  terrain rows (W + 1 vertices each, W the terrain's cell width), the index list is sized for as many rows as fit
  (at most the terrain's height) × W × 6, and the extension is used; otherwise it is released.
- `DrawPrimsIndexedUsingVertBuf(row, column, n)` (`0x0075CB50`): when the next two rows of n + 1 vertices do not
  fit, the buffer is flushed and locked again (`CreateBuffer` `0x0075C9F0`); the two rows' terrain vertices (32 bytes
  each) are copied in (`0x0075CA30`) and the n cells' six indices each are appended, rebased to the copied rows.
- `FlushBuffer` (`0x0075CAD0`): unlocks the buffer and draws its indices as triangles (the count rounded down to a
  multiple of 3). `SetIndexBuffer` (`0x0075CAB0`) sets the terrain's index source; `ReleaseVertexBufferExtension`
  (`0x0075CA90`) releases the buffer.

### 3.1 Exact batching and failure contract (OVV7)

This is a process-wide helper, not one independent buffer per view. Let W and H be terrain cell dimensions,
C the driver's vertex capacity, S = W + 1, V the used vertex count, and I the used index count.
`GetVertexBufferExtension` accepts C ≥ 2S and pre-sizes its **16-bit** index vector to
`min(floor(C/S), H) × W × 6`: there is **no subtraction of one vertex row**. This is storage sizing,
not a promise that that many independent cell rows fit. Format 11 is requested and checked. Acquisition
does not lock. Rejected format/capacity releases the queried interface; query failure returns null.
Evidence: `0x0075CD7F–0x0075CE53`, especially unsigned division at `0x0075CE12`.

`CreateBuffer` calls interface slot 6 with `(handle, C)` and keeps its returned pointer. There is no
separate lock-mode argument at this call site. Success sets V = I = 0 and returns true; null returns
false without resetting those counts (`0x0075C9F0–0x0075CA25`). The terrain callers explicitly call this
before submission (e.g. `0x007521CB`, `0x0075B822`); submission is not a lazy initial lock.

The submission arguments are `(row, firstColumn, endColumn)` with an **exclusive** end; n is their
difference. If V + 2(n+1) > C (equality fits), flush, then call slot 6 again. If the resulting/current
pointer is null, return without copying or drawing. Otherwise copy n+1 vertices from row and row+1,
each 32 bytes, into two consecutive compact rows. There is no row cache or overlap reuse. n = 0 still
copies two vertices and appends no indices. Negative ranges and requests larger than C are not
validated; there is no repeated capacity test after relocking (`0x0075CB78–0x0075CC0F`).

`SetIndexBuffer(pointer, byteFlag)` borrows the source; it does not AddRef or copy it. Source starts at
`pointer + 12 × firstColumn` bytes, plus `12 × W × row` when byteFlag is nonzero. Each of the 6n source
entries is an unsigned 16-bit value q **relative to a pair of full terrain rows**, not an absolute
terrain vertex number. With A and B the compact starts of the copied first and second rows, append
q + A − firstColumn when q ≤ W, otherwise q + B − S − firstColumn, storing the result modulo 65536.
No check tests whether q actually refers to one of the copied columns; hidden/degenerate indices
outside that range are neither repaired nor culled. Index-vector shortage grows it to I+6n+16 entries;
it does **not** flush (`0x0075CC14–0x0075CCD1`, word loads/stores `0x0075CCAA/0x0075CCC1`).

Flush with a null lock pointer does nothing. Otherwise call slot 8 `(handle)` to unlock, clear the
pointer, and, if signed I > 0, call slot 10 `(handle, 0, 3 × trunc(I/3), indices)`. Ignore both driver
return values, then set V = I = 0. Flush never relocks. Release calls slot 2 and clears only the
interface pointer: it does not unlock, flush pending indices, destroy the index vector, or clear the
stored lock pointer. Thus release while locked/pending is not an implicit successful submission;
subsequent helper calls require a valid reacquisition/caller sequence. No allocation-failure recovery
is present in the vector-growth path. Evidence: `0x0075CAD0–0x0075CB40`, `0x0075CA90`.

Independent discriminators derived from these instructions (no portable implementation consulted):

| Input/sequence | Native consequence |
|---|---|
| W=4, H=8, C=10 | Accept; initial index-vector length 48, not 24. No lock yet. |
| Lock; row=2, columns [1,3), source q=1,2,6,2,7,6 | Copies six vertices; compact indices 0,1,3,1,4,3. Row selection changes only source address when byteFlag is set. |
| Same range, source q=0 | Rebased first index is 65535; no validity repair. |
| V=4, C=10, n=2 | Equality fits: no flush. V becomes 10. |
| Empty range [2,2), lock valid | V increases by 2, I unchanged; later flush unlocks even if I=0. |
| Flush; small submission without CreateBuffer | Null lock pointer: no new copy/draw. |
| Grow index vector with room for vertices | Grow storage; no driver unlock/draw. |

### 3.2 Method checklist and proof limits (OVV6)

The export currently names **49 non-template entries**, not the question's unverified 54. The full
named checklist follows; presence in this checklist does not certify the entire body. Three additional
unnamed marked-cell wrappers are also identified. Library list/deque helpers are not counted as owners.

| Owner | Named methods and addresses | Proved scope / residual |
|---|---|---|
| cSC4PathView (15) | ctor 0x007DDD50; Init 0x007DCD90; Shutdown 0x0053C940; SetThickness 0x008B6DE0; SetPathTexture 0x007DCE20; NewPath 0x007DDC50; AddPathVertex 0x007DDB30; ClearPaths 0x007DDBE0; Update 0x007DD9B0; GetEndOffsets 0x007DCE60; GetJointOffsets 0x007DCF20; DrawArrowhead 0x007DD0D0; DrawPath 0x007DD410; destructor thunk 0x007DDDB0; deleting destructor 0x007DDDC0 | §1.2 inputs/finite geometry; full shared-reference lifecycle residual |
| cSC4PathView::PathInfo (1) | destructor-labelled helper 0x008EBCD0 | Called on point-deque subobject; no separate full-record constructor/destructor ABI inferred from label |
| SC4MarkedCellView (14) | AddRegionCells 0x004B9250; SetShouldIgnoreDepthBuffer 0x007DC1A0; SetShouldUseWaterAltitude 0x007DC1B0; SetCellSize 0x007DC1D0; Init 0x007DC200; Shutdown 0x007DC2C0; SetOutlineColor 0x007DC2F0; DrawCells 0x007DC3A0; Update 0x007DC9F0; SetCellColor 0x007DCB30; ctor 0x007DCC10; destructor thunk 0x007DCC80; deleting destructor 0x007DCC90; ClearCells 0x007DCD00 | §2.2; complete region allocation/failure and independent outline-disable residual |
| cSTEVertBufExtn (7) | CreateBuffer 0x0075C9F0; CopyTerrainRow 0x0075CA30; ReleaseVertexBufferExtension 0x0075CA90; SetIndexBuffer 0x0075CAB0; FlushBuffer 0x0075CAD0; DrawPrimsIndexedUsingVertBuf 0x0075CB50; GetVertexBufferExtension 0x0075CD40 | §3.1 covers all seven mapped static helper bodies; no invented COM ctor |
| SC4PathInfoView (5) | Init 0x00542EE0; ctor 0x00542F50; destructor thunk 0x00542F90; deleting destructor 0x00542FA0; Update 0x00542FC0 | network-overlay-views §§1.1–1.2; setter/static-init/shared-reference residuals |
| SC4TrainJunctionView (7) | Init 0x005633C0; Shutdown 0x00563730; GetArrowBitmapQuad 0x005637E0; ctor 0x005649D0; GetPathBitmap 0x00564A60; DrawTrainJunction 0x00564BE0; Update 0x005657A0 | network-overlay-views §2.1; marker rotation, cell setters/destructors residual |

Additional unnamed wrappers: marked-cell use-lots byte store 0x007DC1C0; packed-colour vector
assignment 0x007DCCF0; append cell(x,z) 0x007DCD30. AddRegionCells scans signed inclusive x outer,
z inner and appends set mask cells (`0x004B9250–0x004B92DC`); the full growth/failure path remains
unaudited. Marked-cell depth switch is a byte store and does not repack initialized state. Init
sets its flag and returns true even without the render service; Shutdown releases/nulls its filter
and clears the flag. Its deleting destructor releases the filter, both vectors and base/free;
it does not draw/flush (`0x007DCC90`).

Primary view-interface tables: **0x00ABB648** (path), **0x00ABB614** (marked), **0x00A99F48**
(path-info), **0x00A9D974** (train). All contain exactly five entries: QueryInterface 0x005BCB40,
AddRef 0x005BE3E0, Release 0x005BCB30, owner Update, Pick 0x00735290. The first three adjust this
by +4 and forward to shared unknown/refcount methods; Pick returns false without a hit. No bounds
or transform getter/setter appears in these interfaces. Shared zero-release destructor dispatch is
not proved merely by these thunks; §3.4 supplies the separately checked dispatch chain. OVV6 remains
open: the historical 49 named +3 identified unnamed entries do not establish the claimed 54-entry
inventory, and remaining setters/public-interface completeness are residual. No whole-owner/runtime/
replacement/default claim follows. All vectors in this spec are instruction-derived expectations,
not game runs.

### 3.3 Publication qualification by owner (2026-10-23) *[compat]*

This ledger qualifies the native contracts added in §§1.2, 2.2, 3.1–3.2 and
network-overlay-views §§1.1–1.2, 2.1. **Supported** means the stated finite-input rule has primary
instruction evidence; **partial** means only the listed portion of a method is established;
**unknown** means no public contract for that operation is certified. These are method qualifications,
not owner-coverage counts. Decompiler labels alone do not establish argument types or ownership.
All proposed vectors remain **UNEXECUTED**.

| Owner | Supported method contracts | Partial / unknown public surface |
|---|---|---|
| cSC4PathView | Constructor defaults; Init/Shutdown flag behavior; float SetThickness; selected-record SetPathTexture; packed-colour/float-priority NewPath; horizontal duplicate rejection; finite endpoint/miter/3D-arrow/UV DrawPath and ordered Update; ClearPaths resource order. §§1.2, 3.4. | Partial: allocation-failure recovery and exact nonfinite/x87 exceptional outcomes. Unknown: completeness of any additional public setters/getters. Shared QI/AddRef/Release and deletion are supported only by the explicit tables in §3.4. |
| PathInfo | ClearPaths releases each binding before invoking 0x008EBCD0 on the deque subobject, then frees the node (0x007DDBF0–0x007DDC09). | Partial: 0x008EBCD0 destroys deque storage; its name does not prove a standalone record-destructor calling convention. Unknown: independent public construction/refcount/setters; none inferred. |
| SC4MarkedCellView | Constructor; boolean byte setters; float size/reciprocal setter; common/outline float-reference colour conversion; packed-colour vector assignment; append/ClearCells; Init/Shutdown; finite grid/terrain/water/lot ordering and Update passes; §3.4 deletion. | Partial: AddRegionCells inclusive scan/append, not complete growth/failure behavior. Unknown: independent outline-disable API or proof of its absence (OVV4). Arbitrary unsafe terrain indices and short colour arrays are not successful-input guarantees. |
| cSTEVertBufExtn | All seven named static helper bodies, within §3.1's caller sequence: borrowed index input, two compact copied rows, u16 rebasing, capacity equality, lock failure, flush/release distinction. | Partial: allocation failure/invalid ranges and caller use after Release. Unknown: any additional public helper surface. This process-wide helper has no established owner COM vtable or constructor. |
| SC4PathInfoView | Constructor/Init; Update colour bytes/pulse **step**, format/state, strip cap versus full-vector arrows/bars, heights/cross/traversal/failure boundaries; §3.4 shared lifetime and deletion. | Partial: scratch alpha when fewer than three strip vertices are written. Unknown: pulse **initial value**, public filter/depth setters (NOV2). No per-instance pulse reset inferred. |
| SC4TrainJunctionView | Constructor sentinels; Init packed state/direction table/resource calls; Shutdown; finite horizontal-chord bitmap classification, quad/UV, draw order/transparency/frame stacking/origin/model selection; §3.4 deletion dispatch. | Partial: resource-provider output initialization on failure, marker rotation-helper handedness and remaining model state. Unknown: actual cell setters, current/target identities (NOV3), additional public interface completeness. Destruction is not an implicit Shutdown. |

Audit anchors for the R1 additions (these are evidence ranges, not an assembly transcript):

| Contract family | Primary checks and qualification |
|---|---|
| Path record and state | 0x007DDC64–76 float compare/C0 branch; 0x007DCE20–5F selected binding/repeat replacement; 0x007DDD50 constructor; 0x007DDB30 horizontal rejection. Equal-newest-first supersedes the former §7 example. |
| Ribbon, arrows, UV | 0x007DCE60 horizontal offsets; 0x007DCFF8–0x007DD09F squared miter/threshold; 0x007DD10F–188 3D length, float stores and double 4.5; DrawPath 0x007DD410 (1194 bytes), especially 0x007DD5E0–0x007DD732 projections and 0x007DD78D–0x007DD8B9 final pair/state/submission. Arithmetic descriptions state finite geometry, not a universal bit-exact floating-point emulator. |
| Marked-cell conversion, grid, passes | 0x007DCB37–CF and 0x007DC2F0 float loads, bias/store and byte subtraction; DrawCells 0x007DC3A0, 0x007DC500–593 terrain/water/frustum before lot, 0x007DC5C3–6C7 lot flags/replacement/releases, 0x007DC7D0–8E7 grid clamp/index conversion; Update 0x007DC9F0. Constructor suppression does not answer OVV4. |
| Row batching and failure | 0x0075C9F0, 0x0075CA30, 0x0075CA90, 0x0075CAB0, 0x0075CAD0; submission 0x0075CB50 (429 bytes), 0x0075CCAA–C1 word load/store and wrap; acquisition 0x0075CD40–0x0075CE53 division and capacity. No driver return is invented for the void submission/flush helpers. |
| Diagnostic path view | Update 0x00542FC0 (2914 bytes, two exported ranges): pulse before manager lookup, 0x00543110–BC byte colours/25 cap, 0x00543231–0x005435FF full-vector 3D arrows, 0x00543600–0x00543A63 bar precedence/cross; Init 0x00542EE0 and constructor 0x00542F50. Static initialization remains unknown. |
| Train view | Init 0x005633C0 (866 bytes); quad 0x005637E0–0x00563968; classifier 0x00564A60–0x00564BDF; DrawTrainJunction 0x00564BE0 (1194 bytes); Update 0x005657A0. Byte/word arguments, float/immediate transparency and .01 increment are checked independently of decompiler types. No proof of marker handedness or current/target names follows. |

### 3.4 Shared view reference lifetime and owner cleanup (OVV6, bounded) *[compat]*

The following applies to the **four view objects**, not PathInfo or the static vertex-buffer helper.
Let P be the primary view pointer and U=P+4 its unknown/refcount subobject. Constructors call
0x0090DA1E on U, setting its 32-bit count at P+8 to zero. The primary first three slots add 4
to `this`; the secondary first three slots subtract 4 then enter those primary thunks, so the
shared bodies receive U in both cases (0x005BCB40/50, 0x005BE3E0/0x005BE420,
0x005BCB30/60). Names attached to folded thunks do not change their instruction-level meaning.

- **QueryInterface** (0x009457C6–E0): 32-bit numeric ID and output-pointer arguments, callee removes
  8 bytes, boolean result in AL. Only ID **1** succeeds: write U to the output, call U's AddRef slot,
  return true. Other IDs return false without writing the output or acquiring a reference. This is
  not proof of a separate discoverable view-interface ID; a null output on success is not guarded.
- **AddRef** (0x00968B60–66): increment the 32-bit count and return the new count in EAX. No atomic
  operation or overflow check is present; arithmetic wraps modulo 2³².
- **Release** (0x0090D997–AD): when the unsigned count is greater than 1, store count−1 and return
  it in EAX. At count **0 or 1**, call secondary slot 3 with a single 32-bit deleting-flags value 1,
  then return EAX=0. This path does **not** first store zero to the count. Thus zero-count Release
  reaches deletion too; it is not a decrement-to-0xFFFFFFFF rule.

Constructor-installed tables and slot-3 cleanup targets were checked as original data and instructions:

| Owner | Secondary table at U | Slot 3: adjust U−4, then deleting body | Cleanup before optional object free |
|---|---|---|---|
| Path view | 0x00ABB630 | 0x007DDDB0 → 0x007DDDC0 | ClearPaths, free circular-list sentinel, base cleanup. |
| Marked-cell view | 0x00ABB5FC | 0x007DCC80 → 0x007DCC90 | Release nonnull lot filter through slot 2, free colour and cell storage, base cleanup. |
| Path-info view | 0x00A99F30 | 0x00542F90 → 0x00542FA0 | Base cleanup only. No pulse reset, draw or resource acquisition. |
| Train view | 0x00A9D95C | 0x00564A20 → 0x00564A30 | Nullable model-reference helpers for +0x48 then +0x44, base cleanup. No call to Shutdown and no release of the six bitmap bindings in this deleting body. |

Each deleting body tests bit 0 of its 32-bit flags and calls GlobalHeapFree (0x005E5620) only when
set, returning the original P and removing 4 argument bytes. Shared base cleanup 0x0046B4C0 adjusts
to U and enters 0x0090D990, which restores the base table; it does not itself release resources.
Train's model helper 0x004201B0 null-checks the stored pointer and dispatches the adjusted interface's
slot 2. No validity guarantee for arbitrary corrupt model pointers is implied. Train Shutdown's
six-binding release order remains a separate contract in network-overlay-views §2.1.

This proves shared QI/AddRef/Release and the connected deletion paths individually. It does not
prove all external callers balance references or invoke train Shutdown before deletion. The two
newly traced unnamed train cleanup entries are evidence of dispatch, not a reconciliation of the
historical 54-entry claim; the source membership of that claim is still missing. Public setters,
filter/depth switches and independent outline-disable completeness remain separate open questions.

## 4. Users

Tools and views create these view objects and register them with the renderer (render3d.md §11): routes and
dispatch paths use the path view, zoning, lot and query previews use the marked-cell view; the terrain view owns the
vertex-buffer extension.

## 5. Quirks

1. A path vertex closer than about 3 mm to the previous one is dropped (§1).
2. Marked cells under certain lots float 15.5 m above the lot (§2.1).
3. Hidden parts of marked cells are drawn a second time, additively (§2).

## 6. Open questions

See [questions/overlay-views.md](questions/overlay-views.md): OVV1 retains the lot-flag identities
(`0x304`, `0x100000`). The repeat factor is supplied to SetPathTexture (§1.2); its unwritten
NewPath value is not a default. OVV4 retains the independent outline-disable API/absence question;
OVV6 retains inventory and lifecycle/setter gaps (§3.2). OVV2, OVV3, OVV5 and OVV7 have been
answered in the archive; their instruction-derived contracts do not certify complete owners.

## 7. Conformance tests

1. **Order:** paths with priorities 2, 1, 2 created in that order draw as 1, 2 (second), 2 (first).
2. **Strip:** a straight path of 3 vertices draws one strip of 6 vertices, 2t wide.
3. **Duplicate:** a vertex 1 mm from the previous one is not added.
4. **Cells:** 100 cells of 16 m draw in one batch; 300 cells draw in two (1,200 vertices).
5. **Vertex buffers:** with `kUseVertexBuffers` off the extension is never made.
