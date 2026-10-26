# GZ 2D graphics: contexts, targets, graphics service and Windows canvas

Status: **behaviour spec with explicit gaps**, Windows SimCity 4 Deluxe 1.1.641.
This is clean-room input under [CONTRIBUTING.md](../../CONTRIBUTING.md).
*[compat]* denotes required observed behaviour; *[choice]* denotes an implementation
choice where the original has no safe contract. Addresses identify evidence, not
implementation dependencies. A symbol name shared with an unrelated class in the
export can be a folded function; it does not identify the owner of that slot.

## Outline

1. Scope, ownership and conventions
2. `cGZDrawContext`: interface and state
3. Context transforms, drawing and target changes
4. Render-to engines and dispatch
5. `cGZGraphicSystem`
6. `cGZAlphaBlt`
7. `cGZCanvasW32`
8. Conformance vectors
9. Evidence limits and open questions

## 1. Scope, ownership and conventions

The GZ layer supplies a widget-facing 2D drawing context, a target-specific engine,
a graphics/driver service, alpha-blit operations and the host canvas. The context
retains the drawing target and engine independently. The engine consumes absolute
canvas coordinates and owns target-specific drawing resources. Drawing is
synchronous at this boundary, except for explicitly queued host messages and
driver work completed by a flush.

This context is distinct from the game's 3D helper:
[draw-context.md](draw-context.md) specifies `SC4DrawContext` and packed 3D render
state; [view3d.md](view3d.md) explains the view/driver boundary. The font catalogue,
glyph cache, metrics and text drawing are in [drawing-fonts.md](drawing-fonts.md).
Do not substitute either context's slots or state layout for the other's.

The software span kernels, their colour types, source sampling, blending, spans
and destination records are normative in these related specs:

- [span-render-8.md](span-render-8.md), particularly section 10, specifies the
  buffer engine's lifetime and its caller relationship to the context.
- [span-render-1555.md](span-render-1555.md),
  [span-render-565.md](span-render-565.md),
  [span-render-4444.md](span-render-4444.md) and
  [span-render-8888.md](span-render-8888.md) specify the other destination kernels.

*[compat]* Rectangles used for intersection have exclusive right/bottom edges.
Two rectangles touching only on an edge have empty intersection. Empty
intersection is represented by `(0,0,0,0)`, rather than an inverted rectangle
(`0x0079DD60`). Coordinates supplied to `Draw` are interleaved 32-bit float
`(x,y)` pairs; rectangle primitives use two such pairs. Integer convenience
methods convert their arguments before drawing. Unless otherwise stated, slot
numbers below are zero-based and byte displacement is four times the slot.

The Windows object's field offsets are interoperability facts. An implementation
which does not expose the native object layout may represent the same logical
state differently. Do not treat uninitialized native storage as a specified
default.

## 2. `cGZDrawContext`: interface and state

### 2.1 Method inventory

The primary table is `0x00AC16C8` (48 entries). This is the widget-facing
interface, IID `0x2AE63219`, also returned for unknown IID `1` at the same pointer.
Successful `QueryInterface` adds one reference. Unsupported IIDs return false;
the method does **not** clear the caller's output (`0x0082BE90`).

| Slot(s) | Method(s), in slot order | Evidence |
|---|---|---|
| 0–2 | QueryInterface, AddRef, Release | `0x0082BE90`, `0x0082BE80`, `0x0082CF10` |
| 3–6 | PushTransform, ClipReset, Clip, ClipAbsolute | `0x0082CFE0`, `0x0082BEC0`, `0x0082C6D0`, `0x0082C190` |
| 7–9 | Offset, OffsetAbsolute, PopTransform | `0x0082BEE0`, `0x0082BF00`, `0x0082C780` |
| 10–11 | PushState, PopState | `0x0082CFF0`, `0x0082C7D0` |
| 12–15 | GetCanvasArea, GetClipRect, GetOrigin, GetTransformStackDepth | `0x0082BF20`–`0x0082C1F0` |
| 16–18 | IsTargetAttached, SetTarget, GetEngine | `0x0082C210`, `0x0082C830`, `0x0060FB80` |
| 19–21 | ResetToDefaultState, SetColor(index, colour), SetColor(colour) | `0x0082BF80`, `0x0082C220`, `0x0082BFC0` |
| 22–25 | SetAlphaBlend, SetBltModulate, SetAntialiasing, SetLineThickness | `0x0082C270`–`0x0082C2D0` |
| 26–27 | ClearGradient, SetGradientTransform | `0x0082C350`, `0x0082C360` |
| 28–31 | ClearBlt, SetBlt, ClearBltTiling, SetBltTiling | `0x0082C370`–`0x0082C480` |
| 32–35 | Clear, Point, Line, Fill | `0x0082BFE0`, `0x0082C020`, `0x0082C060`, `0x0082C0C0` |
| 36–37 | Ellipse and FillEllipse, folded to the same four-integer operation | `0x0082C100`; both submit kind 5, see §3.2 |
| 38–40 | Blt, BltTiled, Flush | `0x0082C530`, `0x0082C620`, `0x0082C520` |
| 41–43 | IsSuspended, Suspend, Resume | `0x0082BFD0`, `0x0082C4E0`, `0x0082C500` |
| 44–47 | Draw, DrawNoTransform, Lock, Unlock | `0x0082EA60`, `0x0082FC30`, `0x0082C6B0`, `0x0082C6C0` |

`GetEngine` exposes the engine pointer. The folded getter is a field read, not a
new owning reference. The caller must keep the context/target alive.
`IsTargetAttached` tests the target pointer, not initialization success of the
engine. `Release` destroys at the final reference; destruction releases both
engine and target and frees the context's temporary vectors (`0x0082CA40`).
**Destruction calls engine Shutdown**, just as target replacement does: with a
non-null engine it invokes slot 4, frees the six temporary vector allocations,
releases the engine through slot 2, then independently releases the target
through slot 2. A null engine does not suppress release of a retained target.
This is an explicit virtual call, not an assumed destructor side effect
(`0x0082CA43`–`0x0082CA52`, `0x0082CAC4`–`0x0082CADA`). Consequently a context's
final Release normally unlocks its buffer through Shutdown; the buffer engine's
own final-Release-without-Shutdown quirk in §4.2 still applies when that engine
is released directly. This answers GZG6 and differs from the proposed literal
reading that destruction only releases the two interfaces.

### 2.2 Logical state and defaults

| Field(s) | State | Construction/default contract |
|---|---|---|
| `+0x04`–`+0x10` | four caller colours | not initialized by the constructor examined |
| `+0x14`–`+0x20` | target-format conversions of those colours | not initialized by that constructor |
| `+0x24`, `+0x28` | retained target, retained engine | null |
| `+0x2C`, `+0x30` | float origin x/y | established as zero on attaching a non-null target |
| `+0x34`–`+0x40` | current integer clip | established from target on attachment |
| `+0x44`, `+0x45` | antialias and blit modulation bytes | zero |
| `+0x48` | alpha-blend mode (integer) | zero |
| `+0x4C`, `+0x50` | float line thickness, integer raster thickness | `1.0`, `1` |
| `+0x54`–`+0x60` | canvas area supplied by engine | established during attachment |
| `+0x64`–`+0x70` | source-supported destination bounds | established by SetBlt |
| `+0x74`, `+0x75` | active blit, tiling | cleared on non-null attachment |
| `+0x78` | nesting count of suspension | zero |
| `+0x7C` onward | transform stack, state stack, temporary geometry | empty vectors |

Evidence: constructor `0x0082C9A0`, attachment `0x0082C830`.
*[compat]* `ResetToDefaultState` is deliberately narrower than construction: it
sets alpha blend, modulation and antialiasing to zero, line thickness to `1.0`,
and clears blit, tiling and gradient. It does not reset the origin, current clip,
colour array, target, transform stack or state stack (`0x0082BF80`).

### 2.3 Attribute changes and engine notifications

`SetColor(colour)` changes index zero. The indexed operation stores the caller's
packed colour, obtains its target-native RGB representation through target slot
30 (`+0x78`), then notifies an existing engine with all four caller colours via
engine slot 7 `(first=0,count=4,colours)`. Alpha is preserved in the caller-colour
array. No bounds check of the colour index is present; valid callers use 0–3.
This operation needs an attached target even when no engine notification is
needed (`0x0082C220`).

Alpha mode, modulation and antialias setters suppress engine notifications when
the stored value already equals the supplied value. Modulation and antialias are
stored as bytes; alpha mode is stored as an integer. Their engine slots are 8,
9 and 10 respectively. These three setters tolerate an absent engine.

For finite ordinary inputs, line thickness is clamped at zero; the raster
thickness is the integer conversion of `(clamped thickness + 0.5)`, with a zero
result replaced by one. The original conversion is `_ftol2` under the caller's
floating-point environment. Stored thickness remains the float, not that integer.
Changed input triggers engine slot 11 if an engine exists. Negative inputs can
therefore trigger a notification even if clamping leaves the old value unchanged.
Instruction evidence: float loads/comparisons at `0x0082C2D1`–`0x0082C30B`,
addition and conversion `0x0082C30F`–`0x0082C31A`, minimum-one rule
`0x0082C31F`–`0x0082C326`. Non-finite values are outside the settled contract.

ClearGradient forwards `(null,0,0)` to engine slot 12; SetGradientTransform forwards
its arguments to the same slot. These calls require an engine. The buffer's
fixed-point gradient conversion is specified in §4.2; it must not be replaced
with the 3D packed-state gradient rules. Hardware-specific conversion remains GZG2.

## 3. Context transforms, drawing and target changes

### 3.1 Translation and clipping

`Offset` adds an integer pair to the float origin; `OffsetAbsolute` replaces the
origin. Neither alters already stored clip coordinates. `Clip(rect)` truncates
each float origin component using `_ftol2`, translates the supplied integer
rectangle by that pair, and intersects it with the current clip. `ClipAbsolute`
intersects directly without translation; it does not replace the clip.
The engine receives the resulting clip through slot 6 (`+0x18`).
Instruction evidence for origin conversion: `0x0082C6E4`–`0x0082C718`;
absolute intersection and notification: `0x0082C190`–`0x0082C1AA`.

When a blit is active and tiling is disabled, relative Clip and SetBlt also
restrict the engine's effective clip to the intersection with the blit's
supported bounds. The saved context clip remains the separately maintained
clip. `ClipAbsolute` directly notifies its intersection and has no corresponding
blit-bound adjustment. `ClipReset` copies the canvas area into the context clip
**without notifying the engine** (`0x0082BEC0`). A subsequent drawing operation
must not be assumed to repair that omission.

`PushTransform` saves exactly the two origin floats and four clip integers
(24 bytes). `PopTransform` restores that record and notifies the engine of its
restored clip. An empty pop has no effect. Stack depth is the number of saved
records. `PushState` is a separate 16-byte record of the mode bytes, alpha mode,
float line thickness and cached raster thickness; PopState restores antialias,
modulation, alpha and thickness through the setters. It does not restore colour,
gradient, blit, tiling, target or translation. An empty state pop has no effect
(`0x0082C780`, `0x0082C7D0`, `0x0082CFE0`, `0x0082CFF0`).

### 3.2 Geometry and dispatch

`Draw(kind,points,count)` adds the current origin to each float coordinate pair
in temporary storage, leaving the caller's points intact, then invokes
DrawNoTransform. DrawNoTransform does not add origin. Its kinds are:

| Kind | Geometry family |
|---|---|
| 0 | points |
| 1 | independent lines |
| 2 | independent cubic Bézier curves, four points per curve; trailing points ignored |
| 3 | rectangles, two points per rectangle |
| 4 | outline ellipses, two opposite bounding-box corners per ellipse |
| 5 | filled ellipses, two opposite bounding-box corners per ellipse |
| 6 | filled triangles, three vertices per triangle |
| 7 | filled quads, four vertices per quad |
| 8 | one filled polygon |

Unknown kinds do no drawing in the generic dispatcher (`0x0082FC30`). Filled
primitives first offer the target engine an opportunity to handle them. Engine
result zero means handled. Result 2 requests software rasterization. For filled
quads and polygons, result 1 requests tessellation; for an already triangular
kind-6 submission, result 1 produces no further drawing. Unrecognized result
values also produce no further drawing in these three filled dispatchers.
Instruction evidence for the triangle result switch is
`0x0082EA39`–`0x0082EA48`; quad and polygon dispatch are `0x0082FB40` and
`0x0082FB90`. The generic
rasterizer emits clipped spans to engine slot 22; span contents and blitter
selection are covered by the linked span specs. Direct filled-primitive
entry points are `0x0082EA20`, `0x0082FB40`, `0x0082FB90`.

Point adds `(0.5,0.5)` to its integer location and submits kind 0 with one point.
Line adds that same half-pixel shift to **both** integer endpoints and submits
kind 1 with two points. Fill submits the rectangle's integer edges converted
to floats, with no half-pixel shift, as kind 3/count 2. These three operations
use Draw and therefore receive the origin. Clear submits the **current clip**
as kind 3/count 2 through DrawNoTransform; Clear is a fill with current drawing
attributes, not an unconditional zeroing of all target bytes. Evidence:
`0x0082BFE0`–`0x0082C0F9`; point's integer loads, float addition and float stores
are verified at `0x0082C023`–`0x0082C046`.

#### 3.2.1 Ellipse convenience calls and route-specific geometry

*[compat, quirk]* Slots 36 and 37 both take **four signed 32-bit integers**
`(x0,y0,x1,y1)`, not a rectangle pointer and not centre/radius arguments. Both
convert those integers to four binary32 coordinates and call Draw with kind 5,
count 2. Thus the Ellipse-labelled convenience operation fills too; requesting
an outline requires generic kind 4. Both convenience calls receive the current
origin exactly once. The folded names do not justify making their behavior
different (`0x0082C100`–`0x0082C137`, including the 16-byte argument cleanup).

Kinds 4 and 5 consume complete pairs of corners. Their centre is the midpoint
of the two corners; their radii are half the absolute coordinate differences,
so reversing either corner order leaves the same ellipse. They offer the
original kind to engine Draw first. Result 0 is handled; result 1 selects
tessellation, result 2 selects the generic raster route; any other result does
nothing (`0x0082E9D0`, `0x0082FAF0`). This route distinction is observable and
must be preserved instead of forcing one ellipse implementation on all engines.

For kind 5/result 1, let `r` be the larger radius. The polygon has `4*n`
vertices, where `n=1` for `r<=1`, otherwise `ceil(sqrt(r))`. Its angular step
uses the original constant approximately `6.28318`, divided by the vertex count;
the first angle is minus one step and successive angles decrease by one step.
Vertex coordinates are centre plus radius times cosine/sine, stored as
binary32. The angle accumulator and stored step are also binary32. The polygon
is submitted as kind 8 through DrawNoTransform. *[quirk]* With multiple corner
pairs, this tessellator repeatedly reads the **first** pair, rather than
advancing to subsequent pairs, and submits that same ellipse once per complete
pair. This is visible in the unchanged input pointer at `0x0082F470` and loop
back from `0x0082F5B6`; it is not a rule for the result-2 raster route.
Numeric evidence: `0x0082F473`–`0x0082F4F9`,
`0x0082F560`–`0x0082F5AC`.

For kind 5/result 2, normalize the corners to `(left,top,right,bottom)`.
The visited integer rows run from `ceil(top-0.5)` inclusive to
`ceil(bottom-0.5)` exclusive, intersected with the current clip's y range.
For a row `y`, use its pixel centre `y+0.5`. With centre `(cx,cy)`, radii
`rx,ry`, and `q=ry*ry-(y+0.5-cy)^2`, only `q>0` produces a span. The horizontal
half-width is `sqrt(q)*(rx/ry)`; span edges are
`ceil(cx-0.5-halfWidth)` and `ceil(cx-0.5+halfWidth)`, clipped to the context's
left/right, with the right edge exclusive. Empty spans and zero vertical radius
produce no output. This route advances to the next corner pair. Stored centres,
squared vertical radius and width ratio are binary32; the x87 square-root and
arithmetic between stores are not an invitation to round every intermediate to
binary32. Evidence: `0x0082E130`, with row conversion verified at
`0x0082E1E0`–`0x0082E215`; the called `0x009F0A20` is ceil, not a Lua routine.

#### 3.2.2 Curves, lines and filled-polygon handoff

Kind 2 is **not a connected polyline**: each complete group of four points is
one cubic curve with endpoint, two controls, endpoint. The result-1 route
evaluates that cubic at sixteen intervals (`t=1/16` through `1`), creates a
stroke quad for each adjacent pair, and submits kind 7 through DrawNoTransform.
It does not join the last endpoint of one curve to the first point of the next
(`0x0082EFE0`). The result-2 route uses fixed-point forward differences and the
same integer-line helper as kind 1 (`0x0082D5A0`, `0x0082E8A0`). Exact subdivision
is specified in §3.2.5; the remaining integer coverage observations are in §3.2.7.

Kind 1/result 2 consumes independent endpoint pairs, converting each component
to signed fixed point as `floor(component*256+0.5)` before integer-line
coverage. The product and addition use double constants, are passed to floor
as binary64, then converted to an integer; do not use truncation or nearest-even
rounding instead. This follows `0x0082E8A0`, with instruction evidence at
`0x0082E8D0`–`0x0082E942`; `0x009EFF60` dispatches to the CRT floor routine.
The line helper emits individual one-pixel, half-open spans (`0x0082D040`).
Its endpoint tests are directional; the inclusive loop limit alone does not
mean that both submitted endpoints are covered. An ordinary horizontal integer
Line from `(2,3)` to `(4,3)` covers x=2,3; reversing that call covers x=3,4.
An integer Line with identical endpoints produces no span. These examples use
an enclosing clip at least two pixels wide and high and the convenience call's
half-pixel shift. Instruction evidence: the fixed-point low bits are forced
odd at `0x0082D051`–`0x0082D068`; the endpoint diamond tests compare with 128
at `0x0082D18A`–`0x0082D27E`; rejected start/end columns are removed at
`0x0082D2C0`–`0x0082D2E5`. This corrects the previous inclusive-terminal
example. It does not establish identical caps for result-1 stroke quads.

Filled software triangles, quads and polygons use the same simple-polygon span
builder; triangles and quads pass each complete group separately, whereas kind
8 passes its whole point list. The resulting rows are clipped to
`clip.top <= y < clip.bottom`; x endpoints are read as **signed 16-bit** values
from the builder's packed record and intersected with the clip's half-open
horizontal interval. Only nonempty intervals reach engine slot 22
(`0x0082E430`, `0x0082E5C0`, `0x0082E740`). The builder's closure and coverage
are specified below; it does not implement a general polygon winding rule.

#### 3.2.3 Polygon closure, ordering and fixed-point coverage

For result 1, kind 8 is a **triangle fan**, with triangles
`(p0,p1,p2), (p0,p2,p3), …, (p0,p[n-2],p[n-1])`. The vertices are copied
unchanged, then submitted directly to engine Draw as kind 6/count `3*(n-2)`.
There is no ear selection, intersection detection, winding correction or second
software fallback based on the return from that final engine call. The last
triangle closes the boundary to p0 without requiring a repeated first vertex.
For a concave or self-intersecting input, the contract is this fan, even when
its triangles overlap or extend outside the intended outline. Counts below
three are unchecked allocation inputs to this route, not a supported empty
polygon convention (`0x0082F810`, final call `0x0082F943`–`0x0082F95A`).

For result 2, SetPolygonSimple emits nothing for fewer than three points.
Otherwise it rotates the input order to start at the **first** vertex having
the lowest y; equal minima do not replace that vertex. It implicitly connects
the last vertex to the first. One boundary follows increasing input indices,
the other decreasing indices, starting from that minimum. The forward boundary
supplies the left edge and the backward boundary the right edge. They are not
sorted or exchanged when reversed. Consequently `(0,0),(0,4),(4,4),(4,0)`
covers the 4×4 square, while reversing that order produces no nonempty spans.
Repeating the first point is unnecessary; equal-height/duplicate vertices are
skipped when seeking the next edge with a strictly higher ending row.
Evidence: `0x00932700` and `0x00932450`.

The coverage contract is two ordered chains, **one interval per visited row**.
It is not even-odd filling, nonzero-winding filling, or sorting all scanline
intersections. A chain advances through successive vertices only while their
ending rows equal the current row. The forward walk ends drawing when it runs
out of vertices. The backward walk ends drawing if a later ending row falls
below the current row. Thus concave outlines with multiple row intersections,
local downward turns and self intersections must retain this chain behavior;
they are not repaired into a general polygon. Only intervals whose ordered
left edge is strictly less than their right edge survive context clipping.

Pixel centres select rows. Each vertex has saved binary32 coordinates
`X=x-0.5`, `Y=y-0.5`, and ending row `ceil(y-0.5)`. The ceil argument is stored
as binary64 from the x87 subtraction **before** rounding through the saved
binary32 Y; these are distinct conversion paths. The first visited row is the
minimum vertex's ending row; an edge is active up to but excluding its ending
row. For an edge from saved coordinates `(Xa,Ya)` to `(Xb,Yb)`, let
`s=(Xb-Xa)/(Yb-Ya)`, stored as binary64. At an edge's first row r, its signed
16.16 accumulator is
`floor(((r-Ya)*s+Xa)*65536+0.5)+65535`, and its per-row increment is
`floor(s*65536+0.5)`. The edge x is the accumulator's arithmetic right shift
by 16. Both accumulators advance once per row. There is no fresh division
per row, so accumulated fixed-point error is observable. Intermediate x87
operations between stores retain their native precision; this is not a series
of binary32 arithmetic operations. The selected x is narrowed to 16 bits and
later sign-extended by the context before clipping. Numeric evidence:
`0x00932870`–`0x00932897`, `0x0093122D`–`0x0093128F`,
`0x00932549`–`0x0093256E`, `0x00932636`–`0x0093265A`.

#### 3.2.4 Outline ellipses through the cubic route

Kind 4/result 2 converts each corner pair into four independent cubics, then
submits them together through DrawNoTransform as kind 2, with sixteen points
per ellipse (`0x0082E9F4`–`0x0082EA07`, `0x0082CC60`). Result 1 also takes
this route when antialiasing is enabled; its non-AA outline-quad construction
remains a separate boundary. Unlike the filled-ellipse result-1 quirk, this
conversion advances to each subsequent corner pair. Trailing unmatched
corners are ignored. With centre `(cx,cy)`, radii `(rx,ry)`, and
`k=0.552284777164459228515625` (binary32 constant `0x00AC1788`), the cubic
control records, in order, are:

| Arc | Start; control 1; control 2; end |
|---|---|
| Right to bottom | `(cx+rx,cy); (cx+rx,cy+k*ry); (cx+k*rx,cy+ry); (cx,cy+ry)` |
| Bottom to left | `(cx,cy+ry); (cx-k*rx,cy+ry); (cx-rx,cy+k*ry); (cx-rx,cy)` |
| Left to top | `(cx-rx,cy); (cx-rx,cy-k*ry); (cx-k*rx,cy-ry); (cx,cy-ry)` |
| Top to right | `(cx,cy-ry); (cx+k*rx,cy-ry); (cx+rx,cy-k*ry); (cx+rx,cy)` |

The radius and `k*radius` temporaries are binary32; the centre arithmetic stays
in x87 until output stores. Each output coordinate is binary32. Shared endpoint
records are repeated, rather than using a connected-curve convention.
Instruction evidence: `0x0082CC90`–`0x0082CDB5`. This route changes geometry
kind and therefore offers kind 2 to the engine again; its resulting route must
not be inferred solely from the earlier kind-4 response.

#### 3.2.5 Cubic raster subdivision and overshoot

Kind 2/result 2 quantizes all eight components of each complete cubic to
signed 24.8 coordinates using `floor(value*256+0.5)`. For each coordinate
independently, denote those four integers by P0…P3 and define
`A=P3-3*P2+3*P1-P0`, `B=3*(P2-2*P1+P0)`, `C=3*(P1-P0)`.
The curvature bound is the maximum of `abs(2*B)` and `abs(2*(B+3*A))`
across x and y. N is the smallest power of two, starting at 1 and capped at
1024, for which `4*N*N` reaches that bound. This is adaptive subdivision,
unlike the sixteen-interval result-1 route (`0x0082D5A0`).

The polynomial `P(t)=P0+C*t+B*t*t+A*t*t*t` is sampled in signed 64-bit
fixed arithmetic. Write `H1=2^30/N`, `H2=2^30/N^2`, `H3=2^30/N^3`.
The initial position is `P0*2^30+2^29`; the first, second and third forward
differences are respectively `C*H1+B*H2+A*H3`, `2*B*H2+6*A*H3`,
and `6*A*H3`. All three scale factors are exact integers for the supported N.
Each new position adds the first difference, the first difference then adds
the second, and the second then adds the third. Arithmetic right shift by 30
selects the next 24.8 endpoint; adjacent endpoints feed the ordinary integer
line helper. Coefficient arithmetic before widening is 32-bit; forward
differences and their additions use 64-bit words. This does not use a floating
point Bézier evaluator after input quantization.

*[compat, quirk]* There are **N+1 segments**, not N. The samples run through
`t=(N+1)/N`, one step beyond the final control point. There is no final clamp
to P3. For the exactly straight cubic `(0.5,0.5),(1.5,0.5),(2.5,0.5),(3.5,0.5)`,
the bound is zero, N=1, and its two helper calls end at `(3.5,0.5)` and
`(6.5,0.5)`. This is a native overshoot, not a connected-polyline segment.
Evidence: subdivision `0x0082D7E4`–`0x0082D818`; scale/multiply/add widths
`0x0082D818`–`0x0082D9E9`; count increment, updates, arithmetic shifts and
calls `0x0082D9EC`–`0x0082DAB1`. The misleadingly named `0x009F0D10`
is the CRT 64-bit multiplication helper, confirmed by its instructions.

#### 3.2.6 Result-1 stroke corners and rounding (GZG7)

The sixteen-interval cubic route first forms, independently for x and y,
`a=(P3-P0)-3*(P2-P1)`, `b=3*(P0+P2)-6*P1`, and `c=3*(P1-P0)`.
Each coefficient is stored as binary32. It evaluates `((a*t+b)*t+c)*t+P0`
in that order, with x87 intermediates, at the exact binary64 multiples
`t=i*0.0625`, i=1…16. Each endpoint coordinate is stored as binary32 before
the stroke helper consumes it. Thus a literal Bernstein sum with rounding
after every multiplication is not the specified evaluation
(`0x0082F030`–`0x0082F146`).

For an interval A→B the helper appends eight **zero-initialized** floats before
testing length. It stores `dx=B.x-A.x` and `dy=B.y-A.y` as binary32. Corner
arithmetic below remains in x87 registers until each final binary32 store;
there is no integer coordinate rounding at this stage (`0x0082EE10`).

With the context AA byte `+0x44` nonzero, let `L=dx*dx+dy*dy`, evaluated without
intermediate binary32 stores. If `L <= binary32(1e-8)` (the exact threshold is
`9.99999993922529e-9`), the helper returns with all eight appended floats zero.
Otherwise let `s=(floatThickness*0.5)/sqrt(L)`, `u=dy*s`, `v=dx*s`.
The four corners in submission order are `(Ax+u,Ay-v)`, `(Ax-u,Ay+v)`,
`(Bx-u,By+v)`, `(Bx+u,By-v)`. Neither s nor u/v is rounded to binary32 before
the corner stores (`0x0082EE66`–`0x0082EEDC`). There are no joins or extra caps
between intervals. A zero-length interval submits an origin-zero quad, not a
quad located at A.

With AA zero, the helper instead uses half the **integer raster thickness**
at `+0x50`. This is an axis-dependent construction, including half-pixel caps:

| Test | Signed half-thickness h; cap k | Corners in order |
|---|---|---|
| `abs(dy) <= abs(dx)` (ties included) | h and k=0.5 are positive if dx>0, negative otherwise | `(Ax-k,Ay-h)`, `(Ax-k,Ay+h)`, `(Bx+k,By+h)`, `(Bx+k,By-h)` |
| `abs(dy) > abs(dx)` | h and k=0.5 are positive if dy>0, negative otherwise | `(Ax+h,Ay-k)`, `(Ax-h,Ay-k)`, `(Bx-h,By+k)`, `(Bx+h,By+k)` |

The zero-length non-AA interval follows the first row with negative h/k and
can therefore make a nonzero rectangle. This differs from the AA length gate
(`0x0082EEE6`–`0x0082EFCC`). Kind 2 emits all accumulated corners as one kind-7
DrawNoTransform call; a single cubic supplies 64 points, including zero quads.

Worked cases: for A=(1,2), B=(2,2), thickness 2, the AA corners are
`(1,1),(1,3),(2,3),(2,1)`; the non-AA corners are
`(0.5,1),(0.5,3),(2.5,3),(2.5,1)`. A constant cubic with all controls (3,4)
and AA enabled submits 64 copies of (0,0). With AA disabled and raster thickness
2, each of its sixteen quads is `(3.5,5),(3.5,3),(2.5,3),(2.5,5)`.
These cases settle the previously assumed perpendicular/degenerate behavior;
they do not require a device rasterizer to inspect the submitted corners.

#### 3.2.7 Non-AA outline ellipses and line phase (GZG1)

Kind 4/result 1 with AA disabled uses `0x0082DB00`, rather than the cubic
outline route. For each complete corner pair it stores centre and absolute
half-extents as binary32. For maximum radius r, take n=1 when r<=1, otherwise
`ceil(sqrt(r))`; let `m=(n+1)>>1`, `N=8*m`. The step is binary32 of the
binary64 constant **6.28318** divided by N. Start a binary32 angle accumulator
at zero and sample N+1 times, adding the stored step after each sample.
The input pointer advances to each new pair. Line-thickness state is not read.

There are an inner and an outer point at each sample. With cosine/sine of the
stored angle, the inner point is centre plus radius times cosine/sine. The
outer point adds an offset initially (1,0). Its arithmetic order is
`(offset+centre)+(radius*trig)`, not a rounded inner point plus offset. The
x87 trig results are stored as binary64; each point component is stored as
binary32 (`0x0082DB80`–`0x0082DD67`). Each sample after the first emits a quad
in the order previous outer, previous inner, current inner, current outer.
After samples m,3m,5m,7m, rotate the offset `(ox,oy)` to `(-oy,ox)`, recompute
the two points at that same angle, and emit another quad connecting the old
pair to the recomputed pair. The last sample is not forced equal to the first.
All quads are submitted together as kind 7 through DrawNoTransform.

Worked case: corners (-1,-1),(1,1) give n=m=1, N=8: eight ordinary quads plus
four offset-transition quads, **48 points** in one submission. The first inner
point is (1,0), first outer (2,0), regardless of line thickness. Two such pairs
give 96 points; an odd trailing point is ignored. This is a fixed one-unit
offset outline, not a centred stroke of the requested thickness.

The integer-line helper first forces all four signed 24.8 coordinates' low
bits to one. It chooses y as the major axis only when `abs(dy)>abs(dx)`;
equal diagonals use x. For ordinary clips wider and taller than one pixel it
orders the walk in increasing major-coordinate order and retains directional
endpoint acceptance (§3.2.2). Its error update advances the minor coordinate
only on a **strictly negative** error; zero stays on the current minor row.
Clipping advances this same error by the skipped major steps before emitting;
it does not recompute a slope from new integer endpoints
(`0x0082D051`–`0x0082D174`, `0x0082D280`–`0x0082D332`,
`0x0082D526`–`0x0082D560`). Arithmetic here is 32-bit integer arithmetic,
including products; it is not floating-point Bresenham rounding.

Worked cases, integer convenience Line, clip (0,0,10,10): (2,3)→(6,5)
emits pixels (2,3),(3,4),(4,4),(5,5). With clip (3,0,10,10) it emits the
last three of these; replacing the input start with (3,3) is not equivalent.
The descending case (2,3)→(6,1) emits (2,3),(3,3),(4,2),(5,2).
The positive diagonal (2,2)→(5,5) emits (2,2),(3,3),(4,4).

**GZG1 observation boundary.** Exact fractional endpoint diamond-boundary
ties and one-pixel-wide/high clips are not established as a complete coverage
rule here. The helper uses ordered clip limits themselves to infer the minor
step direction; equality of these limits takes a different branch. To settle
these cases, capture engine slot 22's ordered packed spans for both directions
of lines (0.5,0.5)→(4.5,2.5) and (0.5,2.5)→(4.5,0.5). For each line also
perturb each of its four endpoint components separately by ±1/256 (eight
variants). Compare full clip
(0,0,8,8) with clips (2,0,3,8), (0,1,8,2), (2,1,3,2), and (2,1,4,3).
Record the transformed float bits and the 24.8 arguments at `0x0082D040`,
not just final pixels, to distinguish conversion from coverage. For ellipse
transcendental last-bit agreement, record the complete kind-7 float array
for the worked radius-1 case and radii (4,2), with the native x87 control word.
These observations are required before claiming the remaining cases exact;
the question is narrowed to this explicit observation set.

### 3.3 Blit binding and suspension

SetBlt supplies a source plus placement and x/y scaling to engine slot 13. The
placement is translated by the current float origin. The engine returns success
and the bounds supported by that source; success becomes the active-blit byte.
Without tiling, the engine clip is restricted to those bounds. With tiling, the
ordinary context clip is used. ClearBlt sends a null source with zero placement
and unit scales, clears the active byte and restores the engine's context clip.
SetBltTiling sets the tiling byte and calls engine slot 15 with the supplied
tiling parameter. ClearBltTiling clears the byte and calls slot 14; if a blit is
still active its bounded effective clip is re-established. Set/ClearBlt do not
save a restorable source in PushState (`0x0082C370`–`0x0082C4DD`). Detailed
source/engine sampling details are further constrained in §4; the convenience
arguments themselves are specified below.

`Blt(source,sourceRect,destinationRect)` takes two pointers to four signed
32-bit edges. Both rectangles must have strictly positive width and height;
otherwise it makes no state or drawing calls. For valid rectangles it binds the
source at `(destination.left-source.left,destination.top-source.top)` with scales
`destination.width/source.width` and `destination.height/source.height`, clears
tiling, fills the destination rectangle, then clears the source. The placement
is deliberately **not** `destination.left-source.left*scaleX`. Widths,
heights and placement differences are integer subtractions before float
conversion; scale divisions occur in x87 and each argument is stored as
binary32. Placement and Fill subsequently receive the context origin by their
ordinary operations (`0x0082C530`–`0x0082C612`).

`BltTiled(source,tileRect,destinationRect,offsetX,offsetY)` applies the same
positive-rectangle test. It binds at
`(destination.left+offsetX,destination.top+offsetY)`, using unit scales, sets
tiling from tileRect, fills destinationRect, clears tiling, then clears the
source (`0x0082C620`). Offsets are signed integer pixel displacements, not
fractions or repeat counts. Neither convenience operation saves/restores an
earlier source or tiling state; both leave source and tiling disabled. A failed
SetBlt does not prevent their later Fill or cleanup calls.

Suspend increments the nesting count on **every** call, then invokes engine
Flush (slot 16) and Suspend (slot 17) on every call. Resume does nothing at zero;
otherwise it decrements the count and invokes engine Resume (slot 18) only when
the new count reaches zero. IsSuspended tests count greater than zero.
Evidence: `0x0082C4E3`–`0x0082C4F8`, `0x0082C500`–`0x0082C512`.
Flush, Lock and Unlock are engine forwards, to slots 16, 19 and 20 respectively.
Suspension is not a blanket guard on every context drawing entry point.

### 3.4 Attachment, reference ownership and failures

*[compat]* Setting the **same target pointer** is a no-op. Changing it shuts down
and releases the old engine before replacing the retained target (new target
AddRef before old target Release). Setting null detaches and does not clear all
the old drawing coordinates. A non-null attachment establishes origin zero and
clip from the target, asks the draw-engine service (class `0xEB2EA3CB`, IID
`0x4B2EA3B3`) for an engine, and initializes that engine for the target.

The engine then receives clip, all four colours, alpha mode, modulation,
antialias and thickness. Source and gradient are cleared. Engine GetCanvasArea
fills the saved canvas area. Active blit, tiling and suspension count are cleared.
The stacks are not emptied. This means a later PopTransform can restore a clip
saved against a different target (`0x0082C830`–`0x0082C99A`).

The native attachment routine assumes the service and engine exist and continues
to use the engine without a recoverable failure result. IsTargetAttached only
reports a retained target. *[choice]* A standalone core may report failure from
its own fallible attachment API and leave a usable detached context; that is not
an observed guarantee of the GZ method. Before attachment, calls such as
SetColor, clipping, gradient and drawing may dereference missing interfaces.
Do not interpret these as successful no-ops. The export also labels an unrelated
routine at `0x009C40F4` SetTarget; the contract above is for primary slot 17 at
`0x0082C830`.

## 4. Render-to engines and dispatch

### 4.1 Common method inventory

Buffer engine table `0x00AC17D0` and SimGL table `0x00ADB500` have the following
common first 23 slots. These engine slots are **not** the context's slots.

| Slot | Contract | Buffer entry | SimGL entry |
|---|---|---|---|
| 0 | QueryInterface | `0x0082FD60` | `0x0082FD60` |
| 1 | AddRef | `0x005F47F0` | `0x005F47F0` |
| 2 | Release | `0x008306C0` | `0x00A64110` |
| 3 | Init(target) | `0x00830710` | `0x00A643C0` |
| 4 | Shutdown | `0x0082FF20` | `0x00A64130` |
| 5 | GetCanvasArea(out rectangle) | `0x00A64430` | `0x00A64430` |
| 6 | SetClipRect | `0x0082FD90` | `0x00A64440` |
| 7 | SetColor(first,count,array) | `0x0082FF70` | `0x00A64140` |
| 8 | SetAlphaBlend | `0x0082FEA0` | `0x00A64250` |
| 9 | SetBltModulate | `0x0082FEB0` | `0x00A64270` |
| 10 | SetAntialiasing | `0x0082FEC0` | `0x00A64280` |
| 11 | SetLineThickness | `0x00648F10` | `0x00648F10` |
| 12 | SetGradientTransform | `0x0082FDB0` | `0x00A641B0` |
| 13 | SetBlt(source,placement,scales,out bounds) | `0x0082FFD0` | `0x00A64490` |
| 14 | ClearBltTiling | `0x0082FE60` | `0x00A64170` |
| 15 | SetBltTiling | `0x0082FE70` | `0x00A64180` |
| 16 | Flush | `0x00407990` | `0x00407990` |
| 17 | Suspend | `0x008302A0` | `0x00992150` |
| 18 | Resume | `0x008302C0` | `0x009921D0` |
| 19 | Lock / expose target record | `0x0082FED0` | `0x0062C980` |
| 20 | Unlock | `0x00407990` | `0x00407990` |
| 21 | Draw(kind,points,count), result code | `0x00831000` | `0x009922D0` |
| 22 | DrawPreclippedSpanList | `0x008302E0` | `0x006F7DC0` |

The shared QueryInterface supports IID **`0x2AE78A07`** and unknown IID **1**,
returns the same engine pointer with an added reference, and leaves the output
unchanged on unsupported IID (`0x0082FD60`–`0x0082FD80`). Hardware extension
entries and the observation boundary are in §4.4. The hardware owner is
a common base; SimGL inherits its initialization, state and reference handling,
with its own device hooks. These owners are not message-server subscribers at
the drawing boundary; lifecycle is driven through the slots above.

### 4.2 Buffer engine

Construction (`0x00830690`) clears source, gradient flag, destination-lock flag
and span-renderer pointer. Init first shuts down the previous binding; queries
target IID `0x20732180` (`cIGZBuffer`); locks it with `0x8040`; selects a
destination-format span renderer; then caches pixel pointer, stride, size,
colour type and pixel width. Query failure returns false. Lock failure releases
and clears the queried buffer and returns false.

| Destination colour type | Span renderer |
|---|---|
| 2 | 8-bit |
| 4, 6 | 1555 |
| 5 | 565 |
| 7 | 4444 |
| 8, 9 | 8888 |
| other, including 1 and 3 | rejected |

The selection, cached record and shutdown details are specified in the linked
span specs (particularly 565 and 8). *[compat, quirk]* Rejection **after a
successful lock** releases the buffer without a matching unlock. Allocation
failure for the four-byte span-renderer object stores a null pointer but Init
still returns success. Later span drawing assumes that pointer is non-null.
Normal explicit Shutdown destroys the span renderer, unlocks with `0x8040` if
marked locked, then releases the buffer. Final Release is not a substitute for
that explicit Shutdown; it can release without unlocking. These are failure
observations, not safe behavior required of a standalone allocator.

Buffer Lock copies six words to the caller: pixels, byte stride, width, height,
colour type, bytes per pixel, and returns true (`0x0082FED0`). This does not
acquire another destination lock. Engine alpha mode is reduced to `mode > 0`;
modulation and antialiasing retain their supplied byte values. Tiling clear
disables a flag; tiling set enables it and copies a four-word record.

The buffer gradient setter takes a nullable transform pointer plus two
binary32 anchor coordinates. A non-null transform contains three successive
four-float rows: constants, x coefficients, y coefficients. For each of the four
channels it scales all three coefficients by **16,711,680** (`255*65536`), stores
each product as binary32, then converts to a signed integer with the current
x87 integer-rounding mode (nearest-even in the default environment). Its cached
x/y coefficients are those converted integers. Its cached constant is the
truncation toward zero of the converted constant plus `0x8000`, less the sum of
the converted x coefficient times anchorX and converted y coefficient times
anchorY. The latter products and sum remain x87 intermediates until the final
truncation; do not round those products to binary32. The constant's `+0x8000`
is integer arithmetic before conversion back to x87. Null only disables the
gradient flag, leaving cached coefficients intact. This is buffer-engine
conversion, not a universal hardware-device colour rule
(`0x0082FDD0`–`0x0082FE45`, `_ftol2` at `0x0082FE34`).

When antialiasing is zero, Draw returns **2**, irrespective of primitive kind.
With antialiasing enabled, kinds 6 and 7 handle complete groups of three and four
vertices respectively; incomplete trailing vertices are ignored. Kind 8 handles
one polygon. These handled cases return **0**. All other kinds return **1**.
The antialiased path writes pixels directly through the buffer and bypasses the
span renderer (`0x00831000`, `0x008308D0`); see span-render-8 section 10.5 for its
relationship to normal span drawing. This document does not assert exact edge
coverage for the antialiased path (GZG2).

### 4.3 Hardware base and SimGL

Hardware Init queries the target as `cIGZBuffer`, caches its width/height, and
calls the hardware initialization hook (slot 24 / `+0x60`). Failed query or failed
hook releases and clears the target and returns false. Success marks render
state dirty and returns true (`0x00A643C0`). Colour changes copy the caller's
packed words. Alpha mode records both `mode > 0` and `mode > 1`, then marks state
dirty. Modulation stores a byte and marks state dirty; antialias stores its byte.
Tiling maintains an enable byte plus four words (`0x00A64140`–`0x00A64280`).

The common hardware Draw handles kinds **3** and **6** through rectangle and
triangle submission. Before either it applies pending state through a device
hook and updates a dirty coordinate transform. Other kinds return **1**
(`0x00A666C0`), requesting tessellation where the generic dispatcher supports it.
The rectangle and triangle helpers split geometry against
clip and source/tiling bounds; their exact subdivision, gradient rounding and
device vertex values remain GZG2. No software span blend formula is implied for
device rendering.

SimGL construction (`0x009920E0`) builds that hardware base, clears three device
resource pointers, retains the draw-engine service when available, initializes
its half-pixel offset to `0.5`, and sets the observed control bytes `+0xD5=0`,
`+0xD6=1`, `+0xD7=0`. Its factory probes target IID `0xE3094DC8`; unsupported
targets return false (`0x00993270`). Factory construction is `0x00991E50`.
The drawing context's engine-selection service, rather than callers testing the
framebuffer depth, chooses this implementation.

SimGL Draw has a specialized route only for kind 3, with `+0xD8` enabled and a
non-null source. In that route it treats the vertex count as a count of point
pairs; no complete rectangle returns zero. Entry tests and pair count are
verified in bytes at `0x009922DC`–`0x00992317`. Other inputs reach the later
fallback path, which calls `0x00A666C0` with the original kind, points and count
and returns its result (`0x00992B96`–`0x00992BAC`). The specialized path's normal
completion returns 0; a separate exit returns 3 (`0x00992B7D`–`0x00992B93`).
These are engine result codes, not Boolean success values. Specialized texture
subdivision remains subject to the §4.4 observation boundary. Draw and
HWSetImage export pseudocode failed; these findings use paged instructions.

### 4.4 Hardware extension, image failure and observation boundary (GZG2)

The verified SimGL extension prefix at table `0x00ADB500` is below. Names here
describe behavior rather than recovered C++ declarations. Entries after slot
33 are not assigned to this interface merely because adjacent words look like
function pointers.

| Slot / byte offset | Entry | Established role / argument boundary |
|---|---|---|
| 23 / `+0x5C` | `0x009932E0` | deleting destructor, low-bit deletion flag |
| 24 / `+0x60` | `0x00992BD0` | hardware initialization hook, low-byte success |
| 25 / `+0x64` | `0x00992CA0` | hardware shutdown hook |
| 26 / `+0x68` | `0x00992D40` | apply pending device state |
| 27 / `+0x6C` | `0x00992F10` | rectangle pointer; driver viewport receives left, targetHeight-bottom, width, height |
| 28 / `+0x70` | `0x00992F50` | projection pointer forwarded to the driver after matrix-mode 1 |
| 29 / `+0x74` | `0x00992F80` | source-buffer pointer; low-byte image-binding success |
| 30 / `+0x78` | `0x00993340` | image/device-state operation; payload meaning requires the capture below |
| 31 / `+0x7C` | `0x009930B0` | image/render-state binding; complete argument interpretation requires capture |
| 32 / `+0x80` | `0x00993170` | vertex-array pointer and count, 24-byte records; driver primitive selector 0 |
| 33 / `+0x84` | `0x009931F0` | vertex-array pointer and count, 24-byte records; driver primitive selector 6 |

Base construction (`0x00A64840`) clears words +4, +8 and +0x24, dirty bytes
+0x3D/+0x3E, bytes +0x50/+0x84, and words +0x88 through +0xA8. Do not infer
defaults for all other fields from these stores; the context supplies its
drawing defaults through the ordinary setters. Base Shutdown (`0x00A64130`)
tail-dispatches the virtual hardware shutdown hook at +0x64. It does not itself
run the buffer engine's unlock sequence.

HWSetImage first releases/clears its old cache pointer at +0xBC. A null source
then returns false immediately: this early exit does **not** clear +0xC0.
For a non-null source it obtains the source's associated cache through buffer
slot 37 (+0x94), retaining a non-null result. If absent it allocates a 0x68-byte
cache, temporarily retains it, and calls buffer slot 38 (+0x98) to attach it;
successful attachment retains it in +0xBC, and the temporary reference is
released. It then clears/releases the old +0xC0 and queries the cache for IID
`0xEB4D101F` into +0xC0. Success returns true. An absent cache or failed query
releases/clears both +0xBC and +0xC0 and returns false
(`0x00992F80`–`0x009930A3`). No old cache is restored on failure.

Worked ownership case: bind a source whose cache exists but rejects
`0xEB4D101F`. The previous cache reference is released first; the new cache
gets a reference and loses it on failure; the old texture interface is released;
both stored pointers finish null and AL=0. A subsequent null-source call also
returns AL=0. A null-source call made directly after a successful binding clears
the cache reference but leaves the old texture reference until later cleanup.

**GZG2 observation boundary.** Dispatch, table entries and the failure sequence
above do not establish exact hardware vertices or buffer AA pixel coverage.
The following native captures are the remaining acceptance evidence; each must
include return codes and ordered calls, not just screenshots:

* Capture hooks 27–33 and their driver calls for one rectangle and one triangle
  crossing clip (1,1,5,5), source 2×3 with six different texels, placement
  (0.25,-0.5), scales (1,1), (2,0.5), and (1.000001,1). Repeat with tiling off/on
  and a gradient whose channels independently vary by 1/255 per pixel. Record
  source bounds returned by slot 13 and every vertex's raw float/color/UV bits.
  This resolves source-bound rounding, texture splits and gradient conversion
  without substituting buffer-engine rules for device rules. Include slot-13
  QI failure, image-hook failure, and allocation failure with output bounds
  prefilled by four different sentinels.
* Capture SimGL kind 3 with +0xD8=0/1 and source absent/present, then kind 6.
  Record which route reaches the base, the condition reaching result 3, texture
  cache/lock calls and cleanup. Capture slot 30/31 argument words at their call
  sites, and the concrete object's interface extent, to finish the payload and
  extension inventory rather than guessing signatures from folded stubs.
* For buffer AA (`0x008308D0`), record every output byte in a 4×4 target initially
  black for a white triangle (0,0),(3,0),(0,3), then the same triangle shifted by
  (0.5,0.5), both windings and clips (0,0,4,4)/(1,1,3,3). Repeat as two triangles
  sharing an edge, as kind 7 and kind 8, in formats 2,4,5,6,7,8,9, alpha off/on
  and alpha 128. Record row padding too. This settles edge inclusion, overlap,
  quantized coverage, format branches and whether untouched bytes stay intact.

These are explicit unverified observations, not native-exact rendering claims.

## 5. `cGZGraphicSystem`

### 5.1 Identities, inventories and state boundary

Construction is `0x0098FDC0`, service class `0xC416025C`, priority 1,500,000.
QueryInterface (`0x0098DEF0`) exposes IID `0x0073283C` at object `+0x1C`,
IID `0xC47B747C` at `+0x20`, IID `0x03FA40BF` at `+0x24`, and IID `0xE98B2F57`
at `+0x28`; other requests delegate to the system-service base. Each successful
query adds a reference. **Offsets inside a method invoked through an adjusted
interface are relative to that interface pointer**, not automatically to the
complete service. This matters especially for folded getters and the driver
interface's state fields.

Driver/control interface table `0x00ADAFF0`:

| Slot(s) | Operations in order |
|---|---|
| 0–2 | QI, AddRef, Release thunks |
| 3–6 | GetGDriver; driver-count getter; GetGDriverByIndex; GetGDriverByID |
| 7–9 | SetGDriver; SetDefaultDriverClassID; SetDriverSettingsIniFilePath |
| 10–11 | IsScreenSettingAvailable; DowngradeToAvailableScreenSetting |
| 12–16 | SetUIRenderingMode; RenderBufferToScreen; DisableScreenBuffer; GetScreenPixelFormat; LazyFlush |
| 17–20 | no-op one-word setter `0x00648F10`; SetForceNoTranslucency; IsTranslucencyEnabled; stored binary32 getter `0x007624E0` (§5.4) |
| 21–23 | EnableCursorEmulation; EnablePageFlipping; IsPageFlippingEnabled |

Entries for slots 5–16 are respectively `0x0098F0A0`, `0x0098FCB0`,
`0x0098E070`, `0x00990010`, `0x0098AD70`, `0x0098E1F0`, `0x0098E290`,
`0x0098EA30`, `0x0098F110`, `0x0098ACE0`, `0x0098ACF0`, `0x0098AD10`.
Slots 18–19 and 21–23 are `0x0098E020`, `0x0098AD20`, `0x0098E040`,
`0x0098AD30`, `0x00741740`.

The graphics interface table `0x00ADB050` extends through slot 68 before the next
service table at `0x00ADB164`. The following inventory
preserves unnamed operations without assigning speculative public names:

| Slots | Entry facts / identified contract |
|---|---|
| 0–2 | QI/AddRef/Release thunks |
| 3–4 | buffer factory helpers `0x0098E190`, `0x0098E140` |
| 5–9 | `0x007D6680`, `0x00989EB0`, `0x0098E980`, `0x0098E9A0`, `0x0098ED70`; slots 7–9 expose screen-buffer-related interfaces/format |
| 10 | pure virtual in base |
| 11–15 | `0x0098E1D0`, `0x00989EA0`, `0x00648EF0`, `0x007624D0`, `0x0098E120` |
| 16–20 | `0x0098AC70`, `0x0098E5C0`, pure virtual, `0x0098DFA0`, `0x0098DFB0` |
| 21–25 | pure virtual, `0x0098ACC0`, `0x0098ACD0`, pure virtual, `0x00407990` |
| 26–31 | `0x0098E850`, `0x0098E870`, `0x0098E880`, `0x0098E910`, `0x0098DFC0`, `0x0098DFE0` |
| 32–36 | `0x0098AD60`, pure virtual, `0x0098E9E0`, `0x0098E9B0`, `0x0098EA00` |
| 37–42 | `0x0098A040`, `0x00989EC0`, `0x0098A3A0`, `0x0098A1C0`, `0x0098A6D0`, `0x0098A5B0` |
| 43–49 | `0x0098AA60`, `0x0098A870`, `0x0098AB70` twice, `0x0098ABA0`, `0x0098AC30`, `0x0098ACA0` |
| 50–57 | `0x0098EF50`, `0x0098E840` twice, `0x00989E30`, `0x00989E50`, `0x00989E60`, `0x00989E70`, `0x00989E80` |
| 58–60 | `0x004E13E0` three times |
| 61–65 | `0x00648F10` four times, `0x00407990` |
| 66–68 | pure virtual in base |

The buffer/drawing/IO helper signatures and semantics not identified above are
open in GZG3; this inventory alone is not an implementation contract for those
operations. The concrete W32 subclass overrides base pure slots; it must not be
implemented by calling a base pure stub. Service table `0x00ADB17C` has Init at
slot 8 (`0x00990880`) and Shutdown at slot 9 (`0x0098FCE0`), with ordinary
system-service identity, running state and lifecycle operations around them.

Complete-object construction starts uninitialized, with no framework, driver
collection entries, canvas, screen buffer or engine cache. The constructor sets
`+0x34=1`, bytes `+0x40=0`, `+0x41=1`, `+0x42=0`, the no-translucency flag
`+0xEC=0`, and bytes `+0xFC=1`, `+0xFD=1`; their meanings must be interpreted
through the appropriate adjusted interface, not guessed from these raw defaults.

### 5.2 Startup, shutdown and message dispatch

Init is idempotent once initialized. With no framework and no framework available
globally, it returns false. It loads/configures drivers and screen resources,
creates the SimGL buffer cache, registers itself with the framework, and
subscribes to message `0x8ACAA95F` through the message-server-2 service. It also
gets canvas service class `0xA3460DEB`, IID `0x66B217A1`; a missing canvas makes
initialization unsuccessful. Exact driver preference/INI resolution and rollback
after each intermediate failure remain GZG3 (`0x00990880`, LoadAllDrivers
`0x009904D0`, CallbackDriverLoad `0x00990210`).

Shutdown is idempotent and returns true. When initialized it clears that state,
releases the retained canvas, unsubscribes `0x8ACAA95F`, removes its canvas
message handler for `0x1A10630A`, shuts down/releases the buffer cache, frees all
drivers and removes its framework registrations (`0x0098FCE0`). Objects retained
by callers still require normal reference management; Shutdown is not deletion.

Message-server-2 target table `0x00ADAFA0` has QI/AddRef/Release at slots 0–2 and
DoMessage at slot 3 (`0x0098ED10`). Message `0x8ACAA95F` obtains class
`0xEB2EA3CB`, IID `0x4B2EA3B3`, and invokes its slot 9 (`+0x24`) when present.
Other message types are ignored; the handler returns true in either case.
The callback at `0x0098EC80` interprets canvas-event IID `0xDA1062FA`: event
subtypes 0, 1 and 7 invoke the registered graphics listener's slots `+0x108`,
`+0x10C` and `+0x110`. Unsupported subtypes cause no such call. Obtained message
interfaces are released. Exact Windows procedure translation is GZG4.
OnTick (`0x0098DF90`) delegates to a graphics-service helper; no independent
simulation timing contract is established by that method.

### 5.3 Driver selection and screen contracts

SetGDriver with the current pointer does nothing. Changing it releases the old
driver, clears the related driver flags, retains the new driver and requests
mode enumeration/setup when its enumerated mode count is zero. It also notifies
the screen-related object when one is available (`0x0098E070`). Driver getters
must not be presumed to transfer ownership; callers needing ownership query or
retain the returned interface.

GetGDriverByIndex uses an **unsigned** zero-based index in the driver
collection's ordered traversal; it returns null at or beyond the end, including
an empty collection. GetGDriverByID performs an exact driver-ID lookup and
returns null when absent. Both return the stored pointer without AddRef; neither
creates a driver or substitutes the active/default driver. This settles the
getter ownership/failure portion of GZG3 (`0x0098F0A0`, `0x0098FCB0`).

Screen setting is `(width,height,bits)`. Availability requires an enumerated
mode matching those three values and the service's current mode discriminator.
*[quirk]* A nonzero driver ID is looked up, but the enumerated mode calls still
use the active driver in the routine inspected (`0x0098E1F0`). A missing selected
driver returns false; exact handling of an inactive/missing active driver is
not promised safe.

Downgrade tries a reduced candidate **before** checking availability. Depth above
16 becomes 16 first. Otherwise it chooses the next size from this descending
pixel-area ladder, using strict greater-than comparisons against each size's
area: 2048×1536, 1600×1200, 1280×1024, 1024×768, 800×600, 640×480,
320×240. A candidate is the largest ladder size with area strictly less than
the input's area; if none exists it returns false. It tests availability and
continues reducing until a match or exhaustion. It mutates the caller's setting
on unsuccessful attempts too (`0x0098E290`). The area is a native integer
product; overflow cases are outside the settled contract.

SetForceNoTranslucency stores the supplied byte and the accompanying word.
IsTranslucencyEnabled returns true exactly when that byte is zero; it does not
interrogate device capability (`0x0098E020`–`0x0098E036`,
`0x0098AD20`–`0x0098AD2D`). LazyFlush sets a pending byte rather than performing
an immediate flush. DisableScreenBuffer disables its flag. Exact presentation
semantics, copy regions, page flipping and hardware-versus-software fallback in
RenderBufferToScreen remain GZG3. SetUIRenderingMode configures driver render
state for 2D and a projection cached on its first setup; exact matrix, state
ordering and viewport-change implications are also GZG3. Do not reuse the 3D
helper's default-state method as a substitute.

### 5.4 Driver preferences, UI state and observation boundary (GZG3)

Driver/control slot 17 consumes one four-byte stack argument and returns without
reading it or changing the object (`0x00648F10`–`0x00648F12`). No Boolean result
is established by that stub. Slot 20 loads binary32 at **interface-relative
+0xF0** into x87 ST0 and returns (`0x007624E0`–`0x007624E6`). The driver interface
is complete object +0x20, so that is complete object +0x110, not +0xF0. Its public
semantic name is not recovered; it must not be implemented as a constant or a
query of camera magnification merely because the folded entry has that label.

LoadAllDrivers (`0x009904D0`) uses the supplied settings path only if nonempty
and the file exists; otherwise it constructs the application-directory path
`GZGraphic2.ini`. If section `Graphics Drivers` exists, enumerate its entries
through the driver-load callback. An existing empty section therefore does not
trigger the missing-section defaults. If the section is absent, try the built-in
entries in order: `SimGLRef.dll, C4554841` (OpenGL), `SimGLSoft.dll, 7ACA35C6`
(Software), `SimGLDx7.dll, BADB6906`. A nonzero requested default ID already
present in the collection bypasses the current-driver INI preference. Otherwise
start with `0xBADB6906`, read `Current Graphics Driver` / `Driver` as hexadecimal
with `%X` when present, and look that ID up. If missing, try `0x7ACA35C6`; if
that too is missing, fail. This path does not select the first enumerated driver.
The load callback's parsing/plugin failure behavior remains in the capture set.

Worked preference case: with both OpenGL and Software loaded, a valid explicit
default `0xC4554841` wins over an INI Driver value `7ACA35C6`. With an invalid
explicit default, that INI selects Software. With no current-driver entry and
no DirectX driver, Software is the fallback; OpenGL alone is not that fallback.

SetUIRenderingMode is a void operation. It obtains the active driver through
control slot 3 and tests driver byte-offset +0x144; a false result skips the
state/projection operations. There is no null-driver guard before that test.
For a true result the ordered driver calls are below. Arguments are in logical
call order, not push order; offsets refer to the **driver** table:

| Ordered calls | Arguments |
|---|---|
| +0xCC, +0xB4, +0xCC, +0xB0 | 1; 5; 0; 5 |
| +0xD4, +0xD0, +0xB4, +0xB0 | (0,3,2,1); 0; 2; 1 |
| +0xB4 five times | 1; 0; 4; 3; 6 |
| +0x84, +0x80, +0x44, +0x3C | float 1; three floats (1,1,1); 0; 1 |
| +0x88, +0x10C, +0x12C, +0x130 | (1,1); 0; no arguments; output rectangle pointer |
| +0xA0, +0xA4, +0xA0, +0xA8 | 1; projection pointer; 0; no arguments |

The +0x130 output is (l,t,r,b). At the first qualifying call, the static
16-float projection is initialized, in memory order, to
`(2/(r-l),0,0,0; 0,-2/(b-t),0,0; 0,0,0,0; -1,1,0.5,1)`.
The divisions use signed integer extents loaded into x87 and final binary32
stores. The static guard skips recomputation on subsequent calls, even if the
viewport changes; the translations do not compensate for nonzero l/t.
The matrix and call ordering are instruction-verified at
`0x0098EA30`–`0x0098EC73`, rather than inferred from the decompiler's bad stack
locals. Worked case: first viewport (10,20,650,500) gives scales binary32(0.003125) and
binary32(-1/240), translation (-1,1,0.5,1). Changing to (0,0,800,600) leaves
those cached scales in the next call.

**GZG3 observation boundary.** The base graphics table in §5.1 is an entry
inventory; it does not establish names/types or failure ownership for every IO
slot. Remaining evidence is narrowed to the following captures on the concrete
W32 object (base pure-virtual entries are not test targets):

* Trace graphics slots 3–68 at real call sites during buffer creation, file
  load/save and screen resize. For unresolved entries record argument locations,
  callee stack cleanup, AL/EAX/ST0 return channel, IID, adjusted this, and every
  output store. For IO slots 37–49 use valid indexed/24-bit/32-bit BMPs, a missing
  path, a read-only save path, and truncation at header/palette/first/last row;
  prefill outputs and count QI/AddRef/Release, open/close, lock/unlock. This
  distinguishes allocated, borrowed and unchanged-on-failure outputs. Observe
  real callers for folded entries rather than assigning names from other owners.
* Record driver discovery and screen allocation at Init with the explicit
  default and INI cases above; fail each driver factory, canvas acquisition,
  buffer factory/lock, cache factory and framework registration in turn. Record
  exact ordered cleanup, initialized byte, retained references and a second Init
  followed by Shutdown. Record requested/actual size, format, flags and stride
  for screen buffers with requested depth 0/16/32 and active software/hardware
  drivers. This resolves partial-init rollback and format defaults.
* Trace RenderBufferToScreen (`0x0098F110`) with an 8×6 source carrying distinct
  corner pixels, a smaller 5×4 canvas and dirty rectangle (-1,1,7,5). Repeat
  screen buffer enabled/disabled, page flipping enabled/disabled, pending
  LazyFlush clear/set, and each lock/copy/present failure. Record every source
  and destination copy rectangle, present/flip call and pending-flag mutation.
  This resolves copy-versus-present and failure rules without assuming that a
  successful final screenshot proves the intermediate copy extent.

These captures are still required for full graphics-service compatibility;
the settled preference and UI-state cases do not credit the unobserved IO or
presentation operations.

## 6. `cGZAlphaBlt`

### 6.1 Identity, defaults and complete operation inventory

This is a system-service wrapper around the standalone alpha/blit operations,
not a per-target drawing context. Construction allocates 32 bytes, initializes
service class `0x44136FDC` with priority `0xFFF0BDC0`, and installs its interface
at complete object `+0x1C` (`0x009895A0`). QI for IID `0xE3260321` returns that
interface with an added reference; other requests delegate to the service base
(`0x00985D00`). No independent source, destination, clipping or blend state is
stored by these forwarding methods. There is no owner-specific message dispatch
in the examined interface.

Alpha interface table `0x00ADA6F0`:

| Slot(s) | Operation(s) in order |
|---|---|
| 0–2 | QI, AddRef, Release thunks |
| 3–7 | GenericBufferClip; InitAlphaSystem; GetAlphaLevelCount; AlphaSurfaceFromFile; MaskSurfaceFromFile |
| 8–9 | AlphaBlt; AlphaBltNoClip |
| 10–11 | AlphaBltConstAlpha; AlphaBltConstAlphaNoClip |
| 12–13 | AlphaBltConstSource; AlphaBltConstSourceNoClip |
| 14–15 | ShadeBltConstAlpha; ShadeBltConstAlphaNoClip |
| 16–17 | ShadowBlitConstAlpha; ShadowBlitConstAlphaNoClip |
| 18–19 | BrightBlitConstAlpha; BrightBlitConstAlphaNoClip |
| 20–21 | ShadeBlt; ShadeBltNoClip |
| 22–23 | ShadeBltStretch; ShadeBltStretchNoClip |
| 24–25 | ShadeBltWithAdditionalAlpha; ShadeBltWithAdditionalAlphaNoClip |
| 26–27 | ShadeBltStretchWithAdditionalAlpha; ShadeBltStretchWithAdditionalAlphaNoClip |
| 28–29 | AlphaBltSelfAlpha; AlphaBltSelfAlphaNoClip |
| 30–31 | AlphaBltHalfAlphaVariableSource1ConstSource2; corresponding NoClip |

Wrapper entry ranges: `0x00988BE0`–`0x00988EA0` and
`0x009895F0`–`0x00989800`. Slot 14 starts at `0x00989680`; a decompiler label
at `0x0098968A` is inside that wrapper, not its callable entry.

*[compat]* GetAlphaLevelCount returns **32** (`0x00988C00`). InitAlphaSystem
passes its arguments to the shared initialization helper. Every drawing method
forwards to its corresponding standalone operation without applying a context
origin, selecting a new target engine, or saving/restoring context attributes.
The clipped and NoClip variants are distinct entry points; a caller choosing
NoClip must already satisfy that operation's bounds requirements. ConstAlpha,
ConstSource, additional-alpha and self-alpha inputs must not be collapsed into
one interpretation merely because their names resemble span-renderer modes.

### 6.2 Surface IO and failure boundary

AlphaSurfaceFromFile and MaskSurfaceFromFile are legacy file loaders returning
a buffer pointer, with null on file-open failure. They create class
`0xC470D325` as IID `0x20732180`. The loaders expect a legacy indexed BMP-like
layout: seek to byte 14, read `0x428` bytes of DIB header plus palette, obtain
width/height there, and read padded rows. They construct colour type 2 output,
reverse bottom-up source rows when copying into the locked output, and unlock
after a successful destination lock (`0x009888E0`, `0x00988A80`). They do not use
the image decoder selected by the draw context or any 3D texture loader.

These routines are not evidence for acceptance of arbitrary BMP variants,
negative-height images or truncated files. Section 6.3 specifies their raw
index handling and unchecked-input behavior from the instructions, whose
argument widths and cleanup differ from the misleading exported local types.
The 32-level count is a service fact, not proof of a particular divisor or
endpoint rounding rule for every alpha drawing method. The clipping and loader
details settled below do not close the remaining standalone pixel-kernel
portion of GZG5.

### 6.2.1 Ordinary alpha kernel and remaining observations (GZG5)

AlphaBltNoClip (`0x00986A50`) takes source buffer, destination buffer, alpha-byte
buffer, source rectangle and destination point. It locks source, destination,
then alpha with **0x8040**. Failure of the first returns its false AL without
an unlock; failure of the second unlocks source; failure of the third unlocks
destination then source. Normal cleanup unlocks alpha, destination, source and
returns AL=1. This is a low-byte result, not a defined 32-bit Boolean. No
rectangle validation is added by this entry. The clipped wrapper passes local
rectangle copies after the §6.3 clip operation.

For the established ordinary domain, source depth is 16, destination type 4/6
or 5, alpha bytes are **levels 0…31**, and 1555 source/destination words have
their high bit clear. Alpha is read at the source rectangle's corresponding
pixel, with its own byte stride. The initialized lookup table implements for
each five-bit channel s,d and level a:
`min(31, floor((32*d+a*(s-d))/32))`.
The table is built with integer additions and unsigned shifts by five, not
float interpolation or division by 31 (`0x00985ED0`–`0x00985F21`); the 1555
kernel's shifts/masks and lookup addresses are verified at
`0x009854F0`–`0x0098555C`. Type 4/6 channels occupy bits 10–14,5–9,0–4.
Type 5 uses bits 11–15,**6–10**,0–4; its low green bit (bit 5) is absent from
the lookup and reconstructed output. It does not interpolate six green bits.
If source colour-keying is enabled, matching source words leave destination
unchanged while the source/destination/alpha positions still advance.
A source depth other than 16, or a destination type outside 4/5/6, reaches
normal cleanup without pixel writes and still returns AL=1 when all locks
succeeded. Success is therefore not proof that a format was rendered; there
is no fallback to an 8888 span-renderer formula in this operation.

Worked cases after InitAlphaSystem: source 0x7FFF, destination 0, destination
type 4, alpha 31 gives **0x7BDE**, not 0x7FFF; alpha 0 leaves zero. In type 5,
source 0xFFFF, destination 0 and alpha 31 gives **0xF79E**. With a lock failure
in the alpha buffer, the trace is source Lock, destination Lock, alpha Lock
failure, destination Unlock, source Unlock; there are no pixel writes.

**GZG5 observation boundary.** The ordinary table does not establish every
shade, shadow, bright, stretch, self-alpha or half-source operation. To finish
those contracts, capture the standalone callees of interface slots 10–31
in §6.1, separately for clipped and NoClip entries. Use 2×2/3×2 sources with
distinct pixels, padded strides and destination sentinels; destination types
2,4,5,6,7,8,9; alpha/constant/additional-alpha values 0,1,15,16,30,31,32,255;
source words with high alpha bits both clear and set. Record actual parameter
widths before conversion, all input/output bytes, AL/EAX/ST0, lock flags/order
and unlocks after failure of each acquisition. For stretch use source 3×2 to
destination 5×3 with a left/top clip and repeat without it; for half-source use
different constants for source 2 so averaging order cannot hide in equal inputs.
Include keyed pixels and alias source/destination in a separate run. These
observations resolve format acceptance, rounding, sampling phase, additional
alpha composition, return channels and cleanup independently per operation.
NoClip comparisons use already valid bounds, not accidental out-of-bounds reads.

Legacy-loader widths, quantization and ownership remain specified in §6.3.2.
For short reads there is deliberately no invented zero-fill guarantee: capture
stream read results and destination bytes for truncation after header, within
palette and midway through a row, with a known initial buffer pattern. Count
the references and invalid-free attempt described there under open/lock failure.
Allocator-dependent unread bytes are observations of that run, not stable
format semantics. Thus GZG5 is narrowed to these pixel/failure observations;
it is not a claim that all methods share the ordinary kernel.

### 6.3 Standalone clipping and legacy-loader details

`GenericBufferClip(destination,sourceRect,destinationRect,optionalClip)` mutates
both rectangle records in place. It clips destinationRect to destination
right/bottom obtained from buffer slot 12, but to **zero** at left/top, rather
than to the returned area's left/top. Each removed destination edge moves the
corresponding source edge by exactly the same signed integer displacement.
It then applies optionalClip's right, left, bottom, top restrictions, with the
same paired source adjustment. It does not clip against a source buffer or
validate that sourceRect lies within one. Success requires strict positive
destination width and height. Only the low return byte is a boolean; the upper
24 bits are not a result contract (`0x00930BE0`–`0x00930CE7`).

*[compat, quirk]* Failed clipping does **not** restore either input rectangle.
When clipping against the buffer makes an axis empty, it can return immediately,
before clipping later edges or applying optionalClip. The four optionalClip
edges, when reached, are all applied before the final emptiness test. Consumers
must not expect canonical zero rectangles after failure. The helper performs
neither buffer locking nor reference acquisition.

The ordinary clipped AlphaBlt wrapper takes alpha/source/destination inputs,
a source rectangle, a **destination point**, and optional destination clip.
It copies the source rectangle, builds a destination rectangle at that point
with the source rectangle's original width/height, clips the local copies,
and calls AlphaBltNoClip only if clipping succeeds. The caller's rectangle and
point are unchanged, including on failure (`0x00988F60`). This must not be
confused with GenericBufferClip's directly mutable arguments or the stretched
variant's two-rectangle mapping.

#### 6.3.1 Stretch clipping and source mutation

`GenericBufferClipStretch(destinationBuffer,sourceRect,destinationRect,optionalClip)`
takes mutable signed-integer source and destination rectangles. It records the
original destination left/top and the two ratios
`sx=source.width/destination.width`, `sy=source.height/destination.height`,
each stored as binary32 after signed integer extent loads and x87 division. It clips
destination edges in the same order as ordinary clipping: buffer right, left
at zero, buffer bottom, top at zero, then optional right, left, bottom, top.
The buffer dimensions come from slots 9 and 10; it does not use an area origin.
Source bounds are not inspected. Unlike ordinary clipping, no source edge is
changed during that destination intersection (`0x00930CF0`–`0x00930E64`).

Failure from an empty buffer intersection can stop before later destination
edges are changed. Failure after optional clipping leaves all four optional
edge changes in place. In either case sourceRect is unchanged. If destination
needs no edge change, positive width and height suffice for success: sourceRect
is left completely unchanged too. *[quirk]* When an edge did change, the helper
returns false, without changing sourceRect, if **either** `sx==0.5` and the
remaining destination width is 1, **or** `sy==0.5` and the remaining height is 1.
This special rejection does not occur on the unchanged-destination path
(`0x00930E41`–`0x00930EA1`).

For successful changed destinations, source left/top become their old values
plus `trunc((newDestinationEdge-oldDestinationEdge)*ratio)` on each changed
left/top edge. Source right is then `trunc(newSourceLeft+newDestinationWidth*sx)`;
bottom is `trunc(newSourceTop+newDestinationHeight*sy)`. The latter truncations
include the source origin in the expression, rather than truncating an extent
and adding an origin afterward. Products/additions stay in x87 until the
truncating conversion; the stored ratios are binary32. If the resulting source
edges on an axis are equal, a zero left/top makes right/bottom increase by one;
any nonzero left/top makes left/top decrease by one instead. This is an
equality repair, not validation or clipping against a source surface.
The helper returns a boolean only in the low byte, locks neither buffer and
acquires no references. It has no initial guard against zero destination
extent before ratio division. Numeric/conversion evidence:
`0x00930D14`–`0x00930D69`, `0x00930EA1`–`0x00930F59`.

#### 6.3.2 Indexed-file conversion and ownership

`AlphaSurfaceFromFile(filename,levels)` takes a filename pointer and an
**unsigned 32-bit** level count. MaskSurfaceFromFile takes the filename pointer
alone. Both obtain a new type-2, 8-bit buffer, with dimensions read as 32-bit
words at DIB offsets 4 and 8. Both use a source stride of width rounded up to a
multiple of four and request buffer initialization with width, height, type 2,
pixel depth 8. Both read precisely after the fixed `14+0x428`-byte prefix;
the BMP file's pixel-data offset and palette entries do not select or convert
the pixel values. Successful output is bottom-up source data copied into
top-down destination rows, **width bytes per row**, using the destination's
actual stride (`0x00988952`–`0x00988996`, `0x00988A20`–`0x00988A5A`).

For AlphaSurfaceFromFile, each source byte (including padded row bytes) is
replaced by unsigned integer `byte / (256 / levels)`, with **two separate
integer divisions**. This is not multiplication by levels followed by a shift.
For levels 32, source bytes 0–7 become 0, 8–15 become 1, and 248–255 become 31.
MaskSurfaceFromFile preserves the index bytes verbatim: it does not turn a
nonzero index into 1, invert it, or apply palette brightness. The consumer
determines how a mask byte is used. Numeric evidence:
`0x009889A8`–`0x009889D5` uses 32-bit unsigned division for the level count and
the quantization; the mask copy is `0x00988B80`–`0x00988B96`.

The loaders do not validate the file signature, dimensions, header fields,
buffer creation/initialization result, allocation result, seeks or read counts.
Short reads therefore leave unread allocation bytes in use; no deterministic
zero-fill, null return or valid partial image is promised. With a nonempty
source allocation, levels 0 divides by zero in `256/levels`, and levels above
256 produce a zero divisor for the per-byte division. Negative/overflowing
dimensions are not a supported alternate orientation. These are unchecked
native inputs, not additional image formats.

*[compat, quirk]* Creation precedes opening the file. Open failure returns null
without releasing the created interface. The temporary `0x428`-byte header
allocation is not freed on the examined return paths. On successful lock,
the loaders unlock with `0x8040`, free the source pixel allocation, close the
file, and return the created interface without releasing it: the returned
reference belongs to the caller. Lock failure skips unlock and does not
release the created interface; worse, cleanup adds one source stride to the
last-row pointer, attempting to free the **one-past-end** source address for
positive height, before closing the file and returning the buffer. That invalid
free can prevent normal return; it is not a reliable null-on-lock-failure API.
No safe allocator is required to reproduce a corrupt free. Evidence:
`0x009889E1`–`0x009889F3`, `0x00988A5D`–`0x00988A7D`,
`0x00988B41`–`0x00988B56`, `0x00988BBD`–`0x00988BDD`.

## 7. `cGZCanvasW32`

### 7.1 Identity and method inventory

The canvas is the Windows host/event adapter. Primary IID `0x66B217A1` returns
the complete pointer, IID `0xE46397DB` returns `+4`, IID `0x3AC7A210` returns
`+8`; remaining requests delegate to its embedded system-service base.
Success adds a reference (`0x0098B7A0`). Its service subobject is at `+0x14`,
class `0xA3460DEB`, priority `0x1312CF`. The primary table `0x00ADAD88` ends
after slot 30; the following bytes are a string, not additional methods.

| Slot(s) | Contract / entry facts |
|---|---|
| 0–2 | QI, AddRef, Release (`0x0098B7A0`, `0x0098B820`, `0x0098B830`) |
| 3 | store canvas type and request initial `(0,0,width,height)` bounds, `0x0098BCC0` |
| 4 | realize once from a four-word bounds record; §7.4, `0x0098DD70` |
| 5 | byte-valued realized-state getter, `0x00680920` |
| 6 | Shutdown, `0x0098DDB0` |
| 7–8 | suspend/release window; resume/recreate window, `0x0098DC40`, `0x0098DCB0` |
| 9–10 | set four bounds; copy saved bounds, `0x0098BD80`, `0x0098CAB0` |
| 11–12 | show and focus; show, `0x0098BA30`, `0x0098BA50` |
| 13–14 | set title; forward to slot 15, `0x0098B9B0`, `0x0098CAE0` |
| 15 | request repaint event 4 unless paint is suppressed, `0x0098C690` |
| 16–17 | signed client mouse x/y getters, `0x008265C0`, `0x0068D1B0` |
| 18 | filter/repost queued keyboard input; returns discarded-message count, `0x0098CDB0` |
| 19 | post a packaged host message, `0x0098BA60` |
| 20 | address of stored HWND word, not HWND value, `0x00578900` |
| 21–22 | shut down and delete canvas; center window in work area, `0x0098B850`, `0x0098B8E0` |
| 23–24 | width, height getters `0x0098CC30`, `0x0098CC40` |
| 25–28 | folded entries `0x0068D1C0`, `0x0081F910`, `0x006AAEB0`, `0x0040D390` (GZG4) |
| 29–30 | input-channel enabled query; nested enable/disable, `0x0098BAC0`, `0x0098BDC0` |

The embedded service table `0x00ADAD08` has Init at slot 8 (`0x0098BAF0`)
and Shutdown at slot 9 (adjusting thunk `0x0098DEE0`). The ordinary service and
primary canvas pointers are not interchangeable. Other subinterfaces supply
message/listener plumbing; their exact thunk mapping is GZG4.

### 7.2 Defaults and lifecycle

Construction (`0x0098C7B0`) starts with no host handle, no active initialization,
empty stored title, zero saved bounds, wheel-message ID `0xFFFFFFFF`, canvas type
`0x1B`, and two zero input-disable counters. It constructs reusable canvas,
mouse and key message objects and acquires message-server services.
This does not itself create a usable drawing target.

Service Init obtains the user's locale ANSI code page; zero falls back to
`0x10`. It acquires the graphics service and message-server-2, initializes from
the graphics service's screen setting through the primary canvas interface,
and emits canvas event IID `0xDA1062FA`, subtype **0**, on successful initialization.
Missing graphics/message-server-2 or failed canvas initialization returns false
(`0x0098BAF0`). The code-page field is relative to the service pointer in this
routine; do not combine its exported field offsets with complete-object offsets.

Windows realization (`0x0098D9C0`) coordinates the graphics driver window/mode,
and registers `MSWHEEL_ROLLMSG`. Registration failure stores `0xFFFFFFFF`.
Window realization and release emit canvas-event subtypes **8** and **9**
under the conditions in §7.4. These native host resources
are *[compat-limit]*: other operating systems may use their own window system
while preserving the canvas/listener contract.

Shutdown acts only when initialized, emits subtype **1**, invokes the canvas's
window cleanup, releases retained services and reusable messages, restores both
disabled input channels by balancing their counters, and clears initialization.
It returns true, including when already shut down (`0x0098DDB0`).

### 7.3 Bounds, title, messages and input

Set bounds saves `(left,top,right,bottom)` and, when a host handle exists, calls
the host positioning operation with width `right-left`, height `bottom-top`
and flags `0x14`. Get bounds copies the saved values rather than querying the
OS. Width/height getters return those differences; verified integer loads and
subtraction at `0x0098CC30`–`0x0098CC46`.

Set title frees the old saved title. A nonempty input is copied into a new
zero-terminated buffer and applied to the host window if present. An empty
input clears stored title but does **not** call the host title setter, so a
previously displayed title may remain (`0x0098B9B0`). Show-and-focus invokes
ShowWindow with command 1 then SetFocus; the other show method invokes only
ShowWindow with that same command (`0x0098BA30`, `0x0098BA50`).

Packaged message posting accepts a non-null package only if its type getter
returns `0x8006` and its element count is greater than two. It posts the first
three supplied words as Windows message ID, wParam and lParam to the saved
handle and returns whether PostMessage succeeds. Unsupported package shape
returns false (`0x0098BA60`). It does not synchronously invoke the window
procedure.

Channel query accepts selectors **1** and **2**, returning whether the respective
disable count is zero. Other selectors return false. Disable with a bitmask
increments each selected channel's count. Enable decrements a positive count
but never underflows it. Channel 2 additionally notifies message-server-2 on
transitions into and out of its disabled state; nested calls do not repeat that
notification. A combined mask affects both channels independently
(`0x0098BAC0`, `0x0098BDC0`).

DoMessage (`0x0098BD00`) returns true for all input messages. For type
`0xFA0C6390`, it queries IID `0x1A0C615F`; subtype **13** updates two stored
coordinate words from that message. They are signed **client x and y in pixels**,
not desktop coordinates, deltas or dimensions. Other types, failed QI and other
subtypes do not update those words. The acquired interface is released. The
handler receives the message-target subobject at complete-object `+0x10`:
the stores at handler-relative `+0x2C/+0x30` therefore update the same complete
`+0x3C/+0x40` words read by primary slots 16–17. The eight-byte position getter
and both stores are verified at `0x0098BD43`–`0x0098BD5C`; the getters use integer
loads at `0x008265C0` and `0x0068D1B0`.

### 7.4 Native realization, interfaces and event translation

**Window lifetime.** The canvas does not independently create or destroy an
HWND. It asks the active graphics driver to change mode through driver slot 66
(`+0x108`), passing the canvas window-procedure bridge. The first native message
records the HWND if none was saved, then installs resource icon 1000 as the class
icon. Realization requires the framework interface `0x23EA70A1` and graphics
driver-control interface `0xC47B747C`. With creation requested and no saved HWND,
it searches the driver's mode records in enumeration order for the first matching
client width, height, current mode discriminator and low-six-bit screen format.
It requests that mode, then accepts driver slot 50's status only when **zero**.
Success centers the window if the graphics service reports a windowed mode,
registers `MSWHEEL_ROLLMSG` (zero becomes sentinel `0xFFFFFFFF`), and sends canvas
event 8. A missing interface, no matching mode, missing driver or nonzero status
returns false; it does not emit event 8 (`0x0098D9C0`–`0x0098DB9E`, matching
tests `0x0098DA90`–`0x0098DADA`, status test `0x0098DB12`–`0x0098DB30`).

With creation disabled **or an HWND already saved**, the same helper instead
requests driver mode `-1` with arguments `(null,1,1)` when that interface and
driver exist. It clears the saved HWND, frees and clears its auxiliary allocation,
emits event 9, and returns true. This cleanup still occurs when driver QI fails;
the helper never calls DestroyWindow directly (`0x0098DBA1`–`0x0098DC3D`). Thus
calling this helper with creation enabled while an HWND exists releases it; it
is not an idempotent ensure-window operation.

Primary slot 4 is idempotent when the realized byte is already nonzero. Otherwise
it saves the supplied four-word bounds, clears the auxiliary allocation pointer,
invokes realization, and **sets the realized byte even on failure**, returning
the helper's result (`0x0098DD70`). Slot 7, when realized, sets its suspended byte,
shows the window, emits event 6, releases the native window as above, then clears
the realized byte; it always returns true. Slot 8, when unrealized, obtains the
graphics bounds, saves them, invokes realization and sets the realized byte. On
failure it calls Shutdown, including that method's normal service cleanup; on
success it clears suspension, emits event 7, then shows/focuses the window. When
already realized it returns true without these actions (`0x0098DC40`,
`0x0098DCB0`). Native failure does not promise rollback to the preceding HWND.

Centering uses SPI_GETWORKAREA and the outer GetWindowRect size, places that
rectangle centrally using signed integer division, and passes current visibility
as MoveWindow's repaint argument. Failed work-area/window queries leave it alone;
fullscreen mode skips centering (`0x0098B8E0`). Primary slot 21 first calls canvas
Shutdown if realized, then invokes its deleting destructor with flag 1 and
returns integer zero (`0x0098B850`). This deletes the canvas irrespective of
outstanding reference counts: it is not an ordinary Release or an application
quit request. The indirect call uses complete-object `+0x18`'s slot 3, whose
thunk subtracts 24 and reaches the deleting destructor; flag bit 0 controls
GlobalHeapFree (`0x0098B85F`–`0x0098B867`, `0x0098CC20`–`0x0098CC27`,
`0x0098CD90`–`0x0098CDAD`). A close message instead emits event 10; it does not
directly delete the canvas.

**Adjusted interfaces and listeners.** QI for `0xE46397DB` returns complete `+4`.
Its slots 0–2 subtract 4 before invoking primary QI/AddRef/Release; slot 3 returns
the primary canvas pointer without AddRef; slot 4 returns the **HWND value**;
slot 5 returns a zero 16-bit value; slot 6 takes `(listener,enable)` and manages
native-message listeners. QI for `0x3AC7A210` returns `+8`; its slots 0–2 subtract
8 and slot 3 returns the stored message-server pointer, borrowed. The `+0xC`
subinterface's slot 3 is the folded true-returning one-argument handler; the
`+0x10` subinterface's slot 3 is DoMessage. Their QI/ref thunks subtract 12/16;
the service and service's embedded unknown thunks subtract 20/24. These are
adjustments to one lifetime, not independent reference counts (tables
`0x00ADAD3C`–`0x00ADAD88`; thunks `0x0098CB00`–`0x0098CC20`). The `+0xC/+0x10`
tables do not imply extra QI identities beyond §7.1's explicit cases.

Listener enable rejects null or an already-stored pointer; a new pointer is
inserted at the front and retained once overall. Disable removes/releases an existing pointer
and returns true; null or missing pointers return false. Native messages visit
listeners in reverse registration order before built-in handling. Each receives
`(HWND,message,wParam,lParam,out handled)` with handled initially false. The first
listener setting it true ends processing and supplies the window-procedure
return value (`0x0098C6F0`, `0x0098D28C`–`0x0098D2F0`). Listener mutation during
dispatch is not guarded by a snapshot.

**Windows messages.** Except where noted, the following handled messages return
zero. The registered legacy wheel message is first rewritten to `WM_MOUSEWHEEL`
with `wParam` shifted left 16 bits, before listeners run. Unlisted messages,
including middle-button, double-click, WM_SYSCHAR, WM_SYSDEADCHAR and WM_QUIT,
reach DefWindowProc; this window procedure does not implement a thread-level
WM_QUIT pump (`0x0098D230`–`0x0098D71A`).

| Native message | Observable translation |
|---|---|
| WM_DESTROY (2) | Return zero; no explicit PostQuitMessage. |
| WM_MOVE (3) | Get client rectangle in desktop coordinates, offset its origin to the two **unsigned** lParam words, save bounds and notify the screen buffer's bounds setter. |
| WM_SIZE (5) | Keep mapped client origin, replace width/height with unsigned lParam words, save and notify buffer bounds; no separate resize canvas event here. |
| WM_ACTIVATE (6) | Emit canvas subtype 2 when LOWORD(wParam) is nonzero, otherwise subtype 3. Minimized high word does not select the subtype. |
| WM_PAINT (15) | For saved HWND only, BeginPaint, repaint helper, EndPaint. If paint suppression is clear, emit canvas subtype 4; otherwise fill the client area with brush handle 16 when HDC is non-null. |
| WM_CLOSE (16) | Emit canvas subtype 10. |
| WM_SETCURSOR (32) | Return 1 without default processing. |
| WM_INPUTLANGCHANGE (81) | Refresh locale ANSI code page from LOWORD(lParam); zero becomes `0x10`; return DefWindowProc's result. |
| WM_KEYDOWN / WM_SYSKEYDOWN (256/260) | Unless key input disabled or virtual key `0xE5`, emit key subtype 5. |
| WM_KEYUP / WM_SYSKEYUP (257/261) | Same gate, key subtype 6. |
| WM_CHAR (258) | Unless key input disabled, emit text subtype 4 using the encoding path below. |
| WM_MENUCHAR (288) | Call DefWindowProc; preserve a nonzero result, otherwise return `0x10000`. |
| WM_MOUSEMOVE (512) | Save signed lParam x/y; unless mouse input disabled, emit mouse subtype 13 with clamped client position. |
| WM_LBUTTONDOWN / WM_RBUTTONDOWN (513/516) | Save signed x/y; unless disabled, capture the HWND if needed and emit mouse subtype 7/8. |
| WM_LBUTTONUP / WM_RBUTTONUP (514/517) | Save signed x/y; unless disabled, release capture if this HWND owns it and neither left nor right button remains down, then emit subtype 10/11. |
| WM_MOUSEWHEEL (522) | Unless disabled, convert signed lParam desktop x/y through ScreenToClient; emit mouse subtype 14 with clamped position and wheel amount. Does not directly update saved x/y. |
| WM_EXITSIZEMOVE (562) | Save mapped client desktop bounds, notify buffer bounds, then emit canvas subtype 5. |

Mouse positions in emitted messages clamp each axis to `[0,width-1]` and
`[0,height-1]`, with the lower clamp performed first. A zero-sized axis can
therefore yield -1. Raw signed positions are stored before mouse-disable checks;
message feedback through DoMessage can subsequently replace them with the
clamped subtype-13 position. Mouse flags map native MK_SHIFT/MK_CONTROL to
bits 1/2, current Alt key state to bit 4, and native left/right/middle button
state to bits `0x10/0x20/0x40`. Keyboard flags sample all six states with
GetKeyState and additionally preserve lParam bit `0x40000000`. Key messages
carry the virtual-key word and LOWORD(lParam) repeat count. The signed loads,
clamps and message arguments are visible at `0x0098BE50`–`0x0098BEE8`,
`0x0098B6E0`–`0x0098B79F` and `0x0098C400`–`0x0098C478`.

Wheel amount is signed `trunc(delta*100/120)`. **Only a positive result above
999 is replaced by 100**; negative extremes do not receive the corresponding
clamp. A successful SPI_GETWHEELSCROLLLINES query multiplies by
`min(unsigned lines,10)`; query failure leaves that amount unscaled
(`0x0098BF20`–`0x0098C07F`). This is a scaled wheel amount, not raw wheel ticks.

WM_CHAR compares the locale code page with the current framework code page.
When equal, it emits the sign-extended low input byte directly, with repeat
count and sampled flags. When unequal, it appends the byte to a pending string;
on the first byte it determines the encoding's additional-byte count and adds
one. Once that many bytes exist, it converts the whole string to framework
encoding, emits one subtype-4 message per converted **signed byte**, and clears
both strings and the pending count. It does not emit Unicode code points or a
single multi-byte text message (`0x0098D080`–`0x0098D226`; byte sign extension
at `0x0098D0D8`).

**Queue filtering is not a normal dispatch pump.** Slot 18 removes messages with
PeekMessage's inclusive filter `0x100..0x108`, all HWNDs, and PM_REMOVE. It saves
at most 1024, removes character/system-character/dead-character messages, and
works backwards over key releases: matching earlier presses for the same key
and intervening matching-key records are discarded, stopping at another key
release. Surviving records are reposted in their original order. The return is
the number explicitly marked discarded; PostMessage failures are not included.
The routine also contains mouse coalescing tests, but those are unreachable
under this keyboard-only PeekMessage filter. A further PeekMessage occurs after
the 1024th saved message, **before** the count check: a 1025th available message
is consumed without being saved, reposted or counted. Verify this boundary with
a recording host, not a live input queue (`0x0098CDB7`–`0x0098CDED`,
`0x0098CE40`–`0x0098CE49`, complete routine `0x0098CDB0`–`0x0098D077`).

## 8. Conformance vectors

These are declarative vectors for future core tests; no engine files are added
by this RE lane. A recording target/engine/service can check call ordering and
reference events without rendering a game frame. Expected results below are
specified behaviour, **not measured game-run results**. The linked span specs
supply pixel-result vectors for their kernels. Set up an attached context unless
a vector explicitly says otherwise.

| ID | Setup and action | Expected result |
|---|---|---|
| GZG-V01 | Context QI for unknown IID with output sentinel | false; sentinel unchanged |
| GZG-V02 | Intersect `(0,0,10,10)` with `(10,2,20,8)` | `(0,0,0,0)`, empty |
| GZG-V03 | Attach target with area `(0,0,100,80)` | origin zero; clip copied; engine initialized/notified; no active source/tiling/suspension |
| GZG-V04 | Reattach identical target pointer | no shutdown, AddRef, Release or Init |
| GZG-V05 | PushTransform; Offset `(5,7)`; Clip; PopTransform twice | first pop restores saved origin/clip and notifies once; second pop has no effect |
| GZG-V06 | Clip `(2,3,9,10)` at origin `(5,7)`, initial clip `(0,0,20,20)` | context clip `(7,10,14,17)` |
| GZG-V07 | Same input via ClipAbsolute | clip `(2,3,9,10)` |
| GZG-V08 | ClipReset after smaller clip | context clip becomes canvas area; no engine SetClip notification |
| GZG-V09 | PushState; change colour, origin, alpha, AA, thickness; PopState | alpha/AA/thickness restored; changed colour and origin retained |
| GZG-V10 | ResetToDefaultState after setting origin and colour | origin and colour retained; alpha/modulation/AA zero; thickness one; blit/tiling/gradient cleared |
| GZG-V11 | SetAA(1) twice with recording engine | one slot-10 notification |
| GZG-V12 | SetLineThickness(0), then SetLineThickness(1.6), default FP environment | float/raster pairs `(0,1)`, `(1.6,2)` |
| GZG-V13 | Point(2,3), origin `(10,20)` | DrawNoTransform receives kind 0/count 1, `(12.5,23.5)` |
| GZG-V14 | Fill `(2,3,6,8)`, origin `(10,20)` | kind 3/count 2, `(12,23),(16,28)` |
| GZG-V15 | Clear at nonzero origin with clip `(1,2,7,9)` | kind 3 uses `(1,2),(7,9)`, without origin translation |
| GZG-V16 | Suspend twice, Resume three times | two Flush/Suspend pairs; one engine Resume on second Resume; count ends zero |
| GZG-V17 | SetBlt success with bounded source, enable tiling, clear tiling | engine clip restricted, then ordinary clip, then restricted again |
| GZG-V18 | Buffer Init target fails buffer QI | false, no destination lock |
| GZG-V19 | Buffer Init target QI succeeds, Lock fails | false; queried buffer released and cleared |
| GZG-V20 | Buffer Init with type 5; explicit Shutdown | 565 selected; destination Lock/Unlock both use `0x8040`; retained target released |
| GZG-V21 | Successful lock, unsupported type 3 | false; queried buffer released; native trace contains no unlock |
| GZG-V22 | Span-renderer allocation fails after supported-format lock | native Init returns true, null renderer; do not execute a native draw with this fixture |
| GZG-V23 | Buffer Draw kind 6/count 7, AA zero | result 2, no AA polygon calls |
| GZG-V24 | Same kind/count with AA enabled | two 3-vertex AA polygons, trailing vertex ignored, result 0 |
| GZG-V25 | Buffer Draw kind 3, AA enabled | result 1; direct handling declined |
| GZG-V26 | Hardware Init QI succeeds but hardware hook fails | false; target released and cleared |
| GZG-V27 | Hardware alpha mode 0,1,2 | state predicates `(false,false)`, `(true,false)`, `(true,true)`; dirty each time |
| GZG-V28 | Graphics receives unrelated message then `0x8ACAA95F` | both return true; only latter calls draw-engine service slot 9 if available |
| GZG-V29 | SetForceNoTranslucency(1,word), query; then set byte 0 | false then true, independent of driver capabilities |
| GZG-V30 | Downgrade `(1024,768,32)` with matching 16-bit active-driver mode | success and `(1024,768,16)` |
| GZG-V31 | Downgrade `(1024,768,16)` with 800×600 available | success and `(800,600,16)`; original mode is not tested first |
| GZG-V32 | Downgrade `(320,240,16)` | false; no smaller ladder entry |
| GZG-V33 | Alpha service GetAlphaLevelCount | 32 |
| GZG-V34 | Alpha service drawing method through recording delegate | matching standalone operation, unchanged arguments, no context-state calls |
| GZG-V35 | Alpha/mask surface filename cannot be opened | null result |
| GZG-V36 | Canvas bounds `(10,20,110,70)` | saved bounds identical; width 100, height 50 |
| GZG-V37 | Nonempty title then empty title, live HWND | only nonempty title sent to OS; saved title cleared afterward |
| GZG-V38 | Disable channel 2 twice, enable three times | counts 1,2,1,0,0; transition notifications only at first disable and second enable |
| GZG-V39 | Query canvas input selector 3 | false, not the conjunction of selectors 1 and 2 |
| GZG-V40 | Packaged message wrong type or count 2 | false, no PostMessage |
| GZG-V41 | Packaged type `0x8006`, count 3; host PostMessage succeeds | first three words posted unchanged; true |
| GZG-V42 | Unrecognized canvas DoMessage | true; no stored-coordinate mutation |
| GZG-V43 | Engine returns 1 for kind 6, then for kind 7 | triangle has no generic drawing; quad takes tessellation route |
| GZG-V44 | Engine returns 2 for kinds 6/7/8 | matching generic span rasterizer invoked |
| GZG-V45 | Final context Release with retained engine and target; recording references | engine Shutdown, temporary-vector frees, engine Release, target Release, in that order; no second target shutdown |
| GZG-V46 | Final context Release with null engine and retained target | target Release still occurs |
| GZG-V47 | Invoke context slots 36 and 37 with `(1,2,5,8)` and origin `(10,20)` | each submits kind 5/count 2, float points `(11,22),(15,28)`; both have identical behavior |
| GZG-V48 | Kind 2/count 7 with engine result 1, recording subsequent draws | one complete cubic, sixteen stroke quads submitted as kind 7; final three input points ignored |
| GZG-V49 | Kind 5/result 2, bounds `(0,0),(4,4)`, enclosing clip | spans `[1,3)` at rows 0 and 3; `[0,4)` at rows 1 and 2; twelve pixels total |
| GZG-V50 | Same raster ellipse with corners reversed | identical spans to V49 |
| GZG-V51 | Kind 5/result 1, two disjoint corner pairs with first bounds `(0,0),(2,2)` | two identical four-vertex polygon submissions based on the first pair; second ellipse bounds are not used |
| GZG-V52 | Blt sourceRect `(2,3,6,5)`, destinationRect `(10,20,18,26)`, zero origin | SetBlt placement `(8,17)`, scales `(2,3)`; ClearBltTiling; Fill destinationRect; ClearBlt |
| GZG-V53 | BltTiled tileRect `(2,3,6,5)`, destinationRect `(10,20,18,26)`, offsets `(-2,4)` | placement `(8,24)`, scales `(1,1)`; set supplied tile record; fill destination; clear tiling then source |
| GZG-V54 | Either convenience blit with zero-width source or destination | no bind, fill or cleanup calls; prior state untouched |
| GZG-V55 | Buffer gradient all-zero transform with arbitrary finite anchors | x/y coefficients zero, constants `0x8000`, gradient enabled; subsequent null setter disables without clearing cache |
| GZG-V56 | GenericBufferClip, 10×10 destination, source `(0,0,8,8)`, destination `(-2,-1,6,7)`, extra clip `(1,2,5,6)` | true low byte; source `(3,3,7,7)`, destination `(1,2,5,6)` |
| GZG-V57 | GenericBufferClip, 10×10 destination, source `(0,0,4,4)`, destination `(12,3,16,7)` | false low byte; source right becomes -2, destination right becomes 10; no rollback or canonical empty rectangle |
| GZG-V58 | Valid indexed alpha input bytes `0,7,8,247,248,255`, levels 32 | output `0,0,1,30,31,31`; mask loader instead preserves all six values |
| GZG-V59 | Valid indexed alpha input byte 255, levels 3 | output 3, since inner divisor is 85; no clamp to levels-1 |
| GZG-V60 | Legacy loader lock fails, positive dimensions; recording allocator that rejects invalid frees without dereferencing | no Unlock or buffer Release; attempted pixel free uses one-past-end address; no reliable normal-return assertion |
| GZG-V61 | Graphics driver index equals count; missing driver ID | null in both cases; no AddRef, active-driver fallback or driver creation |
| GZG-V62 | Canvas primary slot 15; paint suppression clear, then set | canvas subtype 4 only on first call; no PeekMessage, TranslateMessage or DispatchMessage |
| GZG-V63 | Canvas slot 4 with unrealized canvas; recording driver has no matching mode | false, no event 8; realized-state getter nevertheless true |
| GZG-V64 | Canvas native helper creation requested while HWND exists; driver-control QI fails | HWND cleared, auxiliary allocation freed/cleared, event 9, true; no DestroyWindow call |
| GZG-V65 | Enable native listeners A, B, A; A declines, B handles message with result 123 | enables return true, true, false; one retained reference per listener; visit B first and stop; return 123, skip A and built-in/default handling |
| GZG-V66 | Disable nonexistent listener, then remove A, then remove A again | false, true, false; one Release only |
| GZG-V67 | Mouse enabled; canvas 100×50; WM_MOUSEMOVE at signed client `(-3,60)` | immediate saved position `(-3,60)`; subtype-13 message carries `(0,49)`; feeding it through DoMessage makes getters return `(0,49)` |
| GZG-V68 | Mouse disabled; same move | saved raw position still changes; no mouse message |
| GZG-V69 | Mouse move on zero-width, zero-height canvas | emitted clamped coordinates `(-1,-1)` |
| GZG-V70 | Key enabled; WM_SYSKEYDOWN, virtual key `0x41`, repeat 3, lParam bit 30 set, Shift held | key subtype 5, key `0x41`, repeat 3; flags include 1 and `0x40000000`; no menu/default route |
| GZG-V71 | Key enabled; WM_KEYDOWN with virtual key `0xE5` | zero return, no key message |
| GZG-V72 | Wheel delta +120, scroll-lines query returns 3; then delta +1200, query returns 3 | subtype 14 amounts 300 and 300: +1200 becomes 1000, is replaced by 100, then multiplied by 3 |
| GZG-V73 | Wheel delta -1200, scroll-lines query returns 3 | subtype 14 amount -3000; positive-only reset does not apply |
| GZG-V74 | Equal locale/framework encodings; WM_CHAR byte `0xE9`, repeat 2 | one key subtype-4 message with signed byte -23 and repeat 2 |
| GZG-V75 | Slot-18 host queue contains exactly 1025 unrelated-key-down records in filter range, no releases | saves/reposts first 1024 in order, consumes last without reposting, discarded count zero |
| GZG-V76 | WM_DESTROY, WM_CLOSE, WM_QUIT, with recording native default handler | destroy returns zero; close emits canvas event 10; quit reaches default handler; none directly calls PostQuitMessage |
| GZG-V77 | QI Windows subinterface `0xE46397DB`; primary slot 20 and subinterface slot 4 | primary returns address of HWND storage; subinterface returns stored HWND value; getter calls do not AddRef |
| GZG-V78 | Recording lifetime fixture invokes primary slot 21 with realized byte nonzero and outstanding references | Shutdown first, deleting destructor with flag 1 next, integer zero; no reference-count test or framework quit request; do not dereference the native canvas afterward |

### 8.1 Additional native geometry and stretch-clip vectors

These are declarative expectations from native instructions, not game-run
measurements. All geometry vectors use a recording engine and enclosing clip
unless another clip is specified.

| ID | Input / observation boundary | Expected behavior |
|---|---|---|
| GZG-V79 | Integer convenience Line `(2,3)` to `(4,3)`, result 2, clip `(0,0,10,10)` | two one-pixel spans, at `(2,3)` and `(3,3)`; no `(4,3)` |
| GZG-V80 | Reverse V79, then identical endpoints `(2,3)` to `(2,3)` | reverse covers `(3,3),(4,3)` in increasing x order; identical endpoints emit nothing |
| GZG-V81 | Kind 8/result 1, five vertices p0…p4 | one direct engine kind-6 submission with nine points: p0,p1,p2,p0,p2,p3,p0,p3,p4; final response does not trigger another fallback |
| GZG-V82 | Kind 8/result 2, vertices `(0,0),(0,4),(4,4),(4,0)` | span `[0,4)` on rows 0,1,2,3 |
| GZG-V83 | Reverse the vertex order of V82 | no nonempty spans; boundaries are not exchanged to fix winding |
| GZG-V84 | Append `(0,0)` to V82; separately submit only two points with result 2 | repeated first vertex preserves V82; two-point polygon emits no spans |
| GZG-V85 | Kind 4/result 2, two different ellipse corner pairs; engine handles following kind 2 | one kind-2 submission/count 32: four cubics for each ellipse, with repeated endpoints; second pair is used |
| GZG-V86 | Kind 2/result 2, straight cubic `(0.5,0.5),(1.5,0.5),(2.5,0.5),(3.5,0.5)` | N=1; line helper receives endpoints at x=0.5,3.5,6.5, y=0.5 throughout; overshoot is retained |
| GZG-V87 | Stretch clip, buffer 10×10, source `(2,4,6,8)`, destination `(-2,-2,6,6)`, no extra clip | true low byte; destination `(0,0,6,6)`, source `(3,5,6,8)` |
| GZG-V88 | Stretch clip, buffer 10×10, source `(0,0,2,2)`, destination `(9,0,13,4)` | false low byte; destination `(9,0,10,4)`, source unchanged because sx=0.5 and clipped width=1 |
| GZG-V89 | Stretch clip, source `(0,0,1,1)`, destination `(0,0,2,2)`, extra clip `(0,0,1,2)` | false low byte and source unchanged; same call without extra clip succeeds with both rectangles unchanged |
| GZG-V90 | Stretch clip, source `(5,7,6,8)`, destination `(9,9,13,13)`, buffer 10×10 | true; destination `(9,9,10,10)`; extent truncation collapses source axes, then equality repair gives source `(4,6,5,7)` |

## 9. Evidence limits and open questions

The export was queried before reading matching decompilation. Native table
entries were checked against the executable's read-only table bytes where the
export did not expose an owner-named table. Numeric rules explicitly described
above use instruction evidence where decompiler types/calls are misleading;
in particular several `_ftol2` conversions are incorrectly labelled as Lua calls
in the pseudocode. No REA session was used. The 2026-10-26 additions use the
export, paged instruction bytes and read-only executable table/constant bytes;
failed SimGL pseudocode is not treated as a successful decompilation.

This spec settles the interface-level context, engine selection/initialization
boundary, buffer dispatch and host/service rules listed above. It does **not**
claim all generic geometry coverage, SimGL vertex emission, all graphics
service IO helpers or standalone alpha pixel kernels. The Windows canvas event
translation is settled in §7.4, including the misleading former pump/listener labels.
GZG7 is settled by §3.2.6. GZG1–GZG3 and GZG5 have explicit remaining native
observation sets in §§3.2.7, 4.4, 5.4 and 6.2.1 respectively. Their disposition
is recorded in [the answered archive](questions/answered/gz-graphics.md), not
as unbounded open implementation questions. **No listed runtime capture was
performed in this round.** These are narrowed evidence requirements, not
permission to choose arbitrary behavior or credit the unobserved operations as
native-exact. An engine lane can implement the settled contracts and linked
software kernels while keeping those observations outside its compatibility claim.
