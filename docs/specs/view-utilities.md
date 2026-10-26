# Spec: view utilities (`SC4ViewUtilities`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [view3d.md](view3d.md) (§3.1–§3.2, §4.1: the 3D renderer that owns this object, hands it out
through `GetViewUtilities` and calls §5 from its draw pass), [camera.md](camera.md) (the camera control that feeds
§2: zoom, rotation, viewport, clip distances), [lighting.md](lighting.md) (`RotateVectorByViewRotation`,
`0x007F6CC0`, rotates vectors by the view rotation), [animation-coordinator.md](animation-coordinator.md) (uses the
visible area of §6), [terrain.md](terrain.md), [model-maker.md](model-maker.md).

## 0. Summary

The view utilities hold everything the renderer derives from the camera once per camera change: the zoom and
rotation, the camera's basis, the **view frustum** (corners, box, planes and a ground-plane form for quick tests),
the projections between world, view and viewport coordinates, and a second **draw frustum** that may be narrowed
to a dirty rectangle of the viewport. Everything that draws or picks in the 3D city asks them whether a box is
visible, where a point lands on screen, and which ground point lies under a pixel.

## 1. The object *[compat]*

- 0x470 bytes, created by the 3D renderer's constructor (`cSC43DRender`, `0x007C9B10`, kept at its `+0x114`, next to
  the draw context at `+0x11C`); the constructor `0x007F6DF0` clears every field (the corner and ground-point
  vectors to (0, 0, 0)) and sets `+0x3F4`, `+0x3F8` to 1 and `+0x410` to 0.3, three fields none of the functions
  here read. No vtable.
- The renderer (`+0x04`) supplies the camera through its slot 14 *[inferred: `cSC43DRender::GetCamera`]*; the city
  size in metres is at `+0x20` (x) and `+0x24` (z), which the renderer's `Init` (`0x007C6410`) sets to 4096 m each
  (`0x007C66D6`), the largest city, before the first camera update.

| Offset | Content |
|---|---|
| `+0x00` | valid: set at the end of a successful §2 update |
| `+0x08`, `+0x0C`, `+0x10` | the zoom as a float and as an integer, the rotation 0–3 |
| `+0x14`, `+0x18`, `+0x1C` | yaw, pitch, the rotation's part of the yaw (rotation × π/2) |
| `+0x28`, `+0x2C` (`+0x30`, `+0x34`) | viewport width and height in pixels (and as floats) |
| `+0x3C`, `+0x48` | camera position and look-at point |
| `+0x54`, `+0x60`, `+0x6C` | forward, right and up unit vectors |
| `+0x78` / `+0x84` | the view frustum's box (min / max); `+0x90` / `+0x9C` the box clipped to the city (§3.3) |
| `+0xA8`–`+0x104` | the view frustum's 8 corners (§3.1) |
| `+0x108`–`+0x11C` | the camera's view volume: left, right, bottom, top, near, far |
| `+0x120`–`+0x14C` | the ground points under the 4 viewport corners (§6, cached) |
| `+0x150`, `+0x154` | the camera control's metres per pixel and pixels per metre (camera.md §2.1) |
| `+0x158`, `+0x15C` | the camera control's distances to the near and far clip planes |
| `+0x160`–`+0x1BC` | the view frustum's 6 planes (§3.2) |
| `+0x208`–`+0x224` | the view frustum's two ground lines (§3.2) |
| `+0x228`–`+0x3D8` | the draw frustum (§5): narrowed flag, rectangle, boxes, corners, view box, planes, ground lines |
| `+0x414`–`+0x440` | the cached visible-area rectangles (§6); `+0x444` validates these, `+0x445` validates `+0x120`–`+0x14C` only |
| `+0x448`–`+0x454` | sin and cos of −yaw, sin and cos of pitch |
| `+0x458`–`+0x46C` | the viewport scales sx, sy, sz and their inverses |

## 2. Camera update (`UpdateCameraPosition(cameraControl)`, `0x007F83F0`) *[compat]*

**Lifecycle and reserved state (VU2/VU4):** constructor defaults at `+0x3DC`–`+0x410` are an opaque
reserved block: zero at `+0x3DC`–`+0x3F0` and `+0x3FC`–`+0x40C`, float32 one at `+0x3F4/+0x3F8`, and
float32 `0x3E99999A` at `+0x410`. The constructor `0x007F6DF0` writes these defaults; none of the
50 family entries in §12 consumes this block. No behavioral meaning beyond retained defaults is proved.
The object owns no renderer/camera reference: `+0x04` starts null and is a borrowed renderer pointer;
the surrounding renderer supplies it and the two city dimensions by field writes. There is no separate
SC4ViewUtilities Init, Shutdown, renderer setter, camera setter, AddRef/Release or deleting destructor
among these 50 entries. A portable input setter is an adapter to that surrounding field/provider wiring,
not a new attributed native method. The iterator deleting destructor in §12 is a different object.

Called when the camera changes. It clears both visible-area caches (§6) and the valid flag; without a renderer or
camera it stops there. Otherwise:

1. Camera position, look-at, and forward = look-at − position.
2. Zoom := the camera control's zoom (`+0x108`, an integer); rotation := its `+0x10C`; viewport size, clip
   distances (camera.md §2.6), metres per pixel and pixels per metre (`+0x134`, `+0x138`) and the camera's view
   volume.
3. Viewport scales: sx = width ÷ (right − left), sy = height ÷ (top − bottom), sz = 1 ÷ (far − near).
4. Angles: yaw = rotation × π/2 + **yawOffset[zoom]**, pitch = **pitch[zoom]**, with the tables below; the sin and
   cos of −yaw and of pitch are kept.
5. Basis: forward is normalised; right = normalise(forward × (0, 1, 0)); up = normalise(right × forward).
6. The view frustum (§3) is built, then the valid flag set.

| Zoom | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| yawOffset (`0x00ABCFC4`) | −π/8 | −π/8 | −π/8 | −π/8 | −π/8 |
| pitch (`0x00ABCFD8`) | 30° | 35° | 40° | 45° | 45° |

### 2.1 Angle helpers

- `GetCameraViewAnglesForRotation(rotation, zoom, &yaw, &pitch, &groundPitch)` (`0x007F7780`): yaw and pitch as in
  step 4, and groundPitch = atan(tan(pitch) ÷ cos(yawOffset[zoom])), the pitch seen along a city edge (32.0°,
  37.2°, 42.2°, 47.3°, 47.3° for zooms 0–4).
- `GetObjectYRotationAngle(rotation)` (`0x007F6CB0`): −rotation × π/2.
- `GetCityWidthFromOrthoView(width, zoom)` (`0x007F8300`): sin(a)×width + cos(a)×width, a = |yawOffset[zoom]|;
  evaluate the separate products before the sum (§2.3); the
  screen width of a square of that side (× 1.3066 at every zoom).
- `GetZoomAndRotation` (`0x007F66A0`) returns `+0x0C` and `+0x10`; `ViewportUnitsToWorldUnits(v)` (`0x007F6690`)
  returns v × `+0x150`.

### 2.2 Native numeric evaluation and invalid inputs (VU5)

The angle table values are **float32 constants**, not exact mathematical angles. All five yaw offsets
have bits `0xBEC90FDB`; the pitch bits for zoom 0–4 are `0x3F060A92`, `0x3F1C61AA`, `0x3F32B8C2`,
`0x3F490FDB`, `0x3F490FDB`. The quarter turn is `0x3FC90FDB` (`0x00ABCFEC`). These were read from
the shipped PE data at `0x00ABCFC4`–`0x00ABCFEC` and checked against the indexed loads at
`0x007F85BE`–`0x007F85ED`. Native yaw is signed integer rotation converted by x87, multiplied by the
quarter turn, then the offset added before a float32 store. The rotation-only angle is stored separately.
There is **no** zoom or rotation bounds check before the table reads. Out-of-range zoom is an out-of-table
memory read, potentially a fault or unrelated float data; returning false/None is a checked-adapter policy.
`UpdateCameraPosition` returns void and marks valid at the successful end even for numerically invalid
camera data; its only early exits are the missing renderer/camera paths (§2 entry).

The instructions use x87 registers with the caller's precision control and explicit float32 memory
stores, not fused multiply-add. Register width alone does not establish arithmetic precision (§2.3).
Native trig uses FSIN/FCOS, and angle helpers use
FPTAN/FPATAN (`0x007F85F0`–`0x007F863A`, `0x007F8370`–`0x007F8380`). No portable libm bit identity
is implied. sx and sy are computed with extent subtraction still on the x87 stack and each quotient stored
to float32. Their inverses divide one by those **stored** scales. sz is stored by a non-popping float32
store, and its inverse uses the still-retained register quotient (`0x007F8533`–`0x007F85A1`), rather
than reloading rounded sz or simply copying far−near. No method here changes the x87 control word:
rounding/exception mode is inherited from the caller. Numeric bit conformance must name that mode.

Forward normalization forms the squared length with x87 products/additions, FSQRT, and reciprocal,
then stores the three products to float32 (`0x007F8640`–`0x007F8677`); there is no zero-length guard.
Right and up similarly divide by their lengths; up is formed from the right cross product before that
right vector is normalized (`0x007F867A`–`0x007F8685`). Coincident eye/look-at and vertical forward
therefore propagate invalid values under masked exceptions. Zero viewport or view-volume extents
likewise take the ordinary division path: zero scales/infinite inverse scales or infinite scales/zero
inverses, and NaNs where both numerator and denominator vanish. No finite-value repair occurs.
The later length-to-extent steps used for corners do test squared length >0 before rescaling; this does
not turn the earlier unguarded basis normalization into a checked operation.

### 2.3 Native numeric profile and scalar worked cases (VU12)

The supported **ordinary game numeric profile** is x87 PC=00 (24-bit significand), RC=00
(nearest/even), with all six floating exceptions masked; its conventional control word is
**0x007F**. This is the profile established by application startup, not a control-word write in
any utilities entry. The startup wrapper selects mode 0 (`0x005F85E0`–`0x005F85E8`); the setter
changes only the precision bits (`0x008929E8`–`0x00892A05`). See foundations §18 for startup,
thread and plugin-induced precision changes. A utilities call inherits another profile if its
caller/thread has changed it. Static bytes alone cannot certify the control word of a particular
live renderer call, especially in a process with plugins.

For **native exact** observations, record the raw control word immediately before the original
entry on that calling thread, with the input bits and the float32/double/retained-register return
boundary. This is the runtime observation still required to attribute a live discrepancy to the
ordinary profile. Do not change the control word before measuring it. PC=53/64 or another RC
requires a separately labelled comparison using the same operation/store graph; a matching
geometric result under portable nearest/even float64/libm does not establish native numeric identity.

In §§3.5, 4 and 7.3 each arithmetic operator denotes **one** native add/subtract/multiply/divide
or square root, rounded to the selected PC/RC, with no reassociation or fusion. Under the ordinary
profile the significand rounding is like float32 for ordinary finite, normal-range values, but
register exponents retain x87 range. Exact float32 loads and signed-int32-to-register loads are
not pre-rounded to PC. A **store/reload** means an additional float32 range/rounding boundary;
a **store/retain** writes float32 but continues using the pre-store register value. Negation
changes the sign bit without an arithmetic rounding. Trig is the x87 hardware operation, not
a prescribed portable libm; its result is not automatically rounded to PC=24. Subsequent basic
arithmetic and explicit stores supply the rounding boundaries stated here. No CPU-independent
bit identity for transcendental instructions or NaN payloads is claimed.
If a trig-sensitive comparison still differs with matching CW and stores, the settling runtime
observation is the CPU family/model plus raw 80-bit trig results at the scalar helper's sine/cosine
boundaries (`0x007F8323`, `0x007F832B`) and the corresponding rounded products and final capture.
For camera update, also record the four stored float32 trig words at `+0x448`–`+0x454`.

**City width (`0x007F8300`):** read the float32 yaw offset for zoom, compare it with zero, and
negate only if it is negative. From this absolute angle a, form **sin(a)×width and cos(a)×width
separately**, then add the two products. Both products round at the selected precision. There
are **no internal float32 stores**, either of the trig values or the products; the sum returns
in the floating register (`0x007F8321`–`0x007F8337`). Replacing this with
width×(sin(a)+cos(a)) moves the two rounding boundaries and is observably different. Rounding
both trig results to float32 first is also not the native graph, even where it agrees by accident.
Width is a float32 argument, not an int32. This corrects the operation order implicit in §2.1's
mathematical formula and resolves the close-viewutil29 FSIN/FCOS/add/multiply probe discrepancy.

**Worked scalar cases (ordinary profile, stored-float32 observation):** a has bits 0x3EC90FDB
for zoom 1 or 2 after absolute value. Width=64 produces sine-product 0x41C3EF16,
cosine-product 0x426C835E and sum **0x42A73D74**; width=128 produces products 0x4243EF16,
0x42EC835E and sum **0x43273D74**. The add-then-multiply graph gives the reported one-ULP
higher words. These normal-range cases also permit software evaluation of the published rounding
graph; a live run must still log CW to certify its profile.

For `GetCameraViewAnglesForRotation`, the exact signed rotation enters the multiply by float32
quarter-turn, that product rounds, and only then is the float32 yaw offset added; the yaw output
is stored float32 (`0x007F7793`–`0x007F779B`). Rotation=3, zoom=3 gives product
**0x4096CBE4**, then yaw **0x408A3AE6**, rather than 0x408A3AE7. Pitch is the exact table word.
The derived angle evaluates hardware tan(pitch), hardware cos(offset), divides tan by cos, then
hardware atan of that quotient using 1 as the other atan operand; only its final output is stored
(`0x007F77AE`–`0x007F77C2`). Camera update likewise stores its yaw before reloading it for
sin/cos of its negation, stores pitch before its trig, and stores all four trig outputs float32
(`0x007F85D6`–`0x007F863A`). Do not feed an unrounded yaw expression to these trig calls.

## 3. The view frustum *[compat]*

### 3.1 Corners and box

With n = position + forward × near and f = position + forward × far, h = right × (right − left)/2 and
v = up × (top − bottom)/2, the corners are:

| Corner | 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| near (`+0xA8` + 12 i) | n + h + v | n + h − v | n − h − v | n − h + v |

and corners 4–7 are the same at f, for the orthographic camera (projection type 1) that the city view uses
*[inferred]*. For a perspective camera the original pushes the **previous** corners 4–7 away from the camera
position by far ÷ near, so they compound across updates *[quirk]*. Compatibility follows those previous
corners; correcting the perspective projection would require a separate adapter policy. The box (`+0x78`)
is the axis-aligned box of the 8 corners (`0x005D6900`).

**Off-centre volume (VU3):** centering is an assumption of the corner builder, not an enforced input
contract. It uses the two differences only and never adds (left+right)/2 or (bottom+top)/2 to the
forward axis (`0x007F83F0`, corner construction). The coordinate projections and point test still use the actual
left/right/bottom/top values. Thus an off-centre camera can disagree with corner-derived box/plane
tests. Do not silently recenter either path to make them agree. The narrowed draw branch likewise
uses rectangle-relative extents, except that an axis spanning the full viewport copies that axis's
original camera volume. No universal claim that every possible camera provider is centred follows.

### 3.2 Planes and ground lines

Six planes, each a normal and an offset so that a point p is inside when normal · p + offset > 0: right (`+0x160`,
normal = right vector, through corner 3), left (`+0x170`, −right, through corner 0), bottom (`+0x180`, up, through
corner 2), top (`+0x190`, −up, through corner 3), near (`+0x1A0`, forward, through corner 3), far (`+0x1B0`,
−forward, with the offset quirk in §3.5).

Because the right vector is horizontal, the left and right planes are vertical and are also kept as **ground
lines** in (x, z): line A from corner 3 along (corner 7 − corner 3) (`+0x208`), line B from corner 0 along
(corner 4 − corner 0) (`+0x218`).

### 3.3 City-clipped box (`GetFrustumAABBCityClipped(viewFrustum, &min, &max)`, `0x007F7F60`)

For the view frustum (true) or the draw frustum (false): the ground quadrilateral made of corners 0 and 3 (in x,
z) and the two points where the rays corner 0 → corner 4 and corner 3 → corner 7 meet the plane y = 0 is clipped
to the city square (0, 0)–(x size, z size) (`SC4GeometryUtilities::ClipPolygonToPolygon2D`, at most 10 points). The
result's x and z are the clipped polygon's extent (0 when nothing is left); min y is 0 and max y is corner 3's y.

### 3.4 Visibility tests

- `IsBoundingBoxInViewFrustum(min, max)` (`0x007F6770`): a box is in when, for each of four tests, its corner
  furthest towards the inside passes: line A with (x − ax) dz − (z − az) dx < 0, line B with the same expression
  > 0, then the top and bottom planes. **There is no near or far test** *[quirk]*: a box behind the camera or past
  the far plane passes when it lies between the four sides.
- `IsBoundingBoxInDrawFrustum(min, max)` (`0x007F6910`): the same with the draw frustum's lines and planes (§5).
- `IsPointInViewFrustum(p)` (`0x007F7E50`): p in view coordinates (§4) must satisfy left ≤ x ≤ right,
  bottom ≤ y ≤ top and near < −z < far.

### 3.5 Corner, plane and predicate evaluation graph (VU12)

The following fills in the numeric boundaries of §§2–5; arithmetic follows §2.3. All vectors
mentioned as owner fields are float32. Temporary extents are **not** computed by multiplying
an assumed unit basis by the requested length: each basis length is measured again.

**Basis normalization in camera update:** first store each look-at minus position component.
For forward, then right, then up, form squared length in **x²+y², then +z²** order, take its
square root, divide the float32 constant 1 by that root, retain the quotient, and multiply each
component by it, storing x,y,z float32. There is no intermediate squared-length, root or quotient
memory rounding. Right and up are formed by the sequential-store cross helper before right is
normalized, as §2.2 says. Each cross component evaluates its two products and subtraction, then
stores float32 before the next component's input loads (`0x005D6F18`–`0x005D6F47`). Basis
normalization evidence: `0x007F8640`–`0x007F8701`.

**Length-to-extent:** use copies of the stored basis, not values retained from its normalization.
Measure each requested vector independently in near, far, horizontal, vertical order:

| Builder | Squared-length addition order | Requested length | Rescaled-component boundaries |
|---|---|---|---|
| Camera update | x²+z², then +y² | near; far; (right−left)×0.5f; (top−bottom)×0.5f | near x/y retained, near z stored/reloaded; far/horizontal/vertical x/y/z stored/reloaded |
| Narrowed draw | z²+y², then +x² | draw near; draw far; (draw right−left)×0.5f; (draw top−bottom)×0.5f | every x/y/z stored/reloaded |

In each row, each squared component rounds separately and the additions follow the stated order. If
squared length is **ordered >0**, take its square root, divide the requested length by that root,
retain the factor and multiply the original components by it. There is no factor store. Otherwise
keep the original components unchanged, including for unordered squared length. For horizontal
and vertical lengths, subtract the extents, multiply by float32 0.5, and only then divide by the
root. Do not round the root or factor to float32 merely because the result vector is stored there.
Evidence: `0x007F8704`–`0x007F88A2`, `0x007FA529`–`0x007FA693`.

Call those four resulting vectors N,F,H,V. For camera corners, first add the relevant N or F
component to the position component, then apply the H sign, then the V sign in §3.1's corner
table; each operation is separate. The memory boundaries before the final corner stores are:

| Camera corners | x | y | z |
|---|---|---|---|
| near 0,1,2 | store/reload after H | store/reload after H | store/reload after position addition |
| near 3 | store/reload after position addition **and** after −H | store/reload after −H | none after position addition; N.z was already stored |
| orthographic far 4…7 | store/reload after H | store/reload after H | store/reload after position addition |

All final corner components are stored float32, z then x then y. Repeating a mathematically
equivalent common centre expression must preserve these differing boundaries. Evidence:
near `0x007F88A4`–`0x007F8A1C`, orthographic far `0x007F8A30`–`0x007F8BB0`.
The perspective branch retains far/near as its factor. For each old far corner, subtract position;
store/reload its z difference, multiply each difference by the factor, store/reload the x/y
products, then add position and store the result z,x,y (`0x007F8BB5`–`0x007F8D43`).

For a narrowed draw, midpoint coordinates are signed-int32 sums divided by 2 toward zero,
then stored float32 before unprojection. Each shortened-volume extent uses an exact signed-int32
difference multiplied by metres-per-pixel, then a float32 store. After unprojecting the midpoint
at depth zero, negate each N component and add it to that world point, storing the three results:
this is the draw base B, not an algebraically cancelled midpoint. Add N back to B to obtain the
near centre; for an orthographic far centre add F to B. The x centre is stored/reloaded before
corners 0 and 3 of each face, the y centre remains retained, and z is retained for the first
corner but stored/reloaded for the other three. At every corner, x/y are stored/reloaded after
the signed H addition, z stays retained after H, and all three final components are stored after
the signed V addition. The draw perspective branch follows the old-corner procedure above with
B in place of position. Evidence: `0x007FA4F8`–`0x007FA525`, `0x007FA6A1`–`0x007FAB5D`.

**Planes and lines:** normal components are copied or sign-negated from the stored basis, without
normalizing them again. Form each offset by summing the component products in the order below,
then negate and store float32; there is no intermediate sum/product store. Letter sequences mean
the order of the coordinate products, not a change of matrix orientation:

| Plane | View: point, product-add order | Narrowed draw: point, product-add order |
|---|---|---|
| right | view corner3; x,z,y | draw corner3; y,z,x |
| left | view corner0; y,x,z | draw corner0; x,z,y |
| bottom | view corner2; y,z,x | draw corner2; y,z,x |
| top | view corner3; z,x,y | draw corner3; z,x,y |
| near | view corner3; z,x,y | **view** corner3; y,z,x |
| far | view corner7; y,z,x, using **positive forward** | **view** corner7; y,z,x, using negative forward |

Thus the cached view far normal is negative forward but its offset is also the negative dot of
**positive** forward with corner7; it is not a geometrically consistent plane through corner7.
Narrowed draw near/far offsets use the view corners, not the new draw corners. Neither near nor
far plane is read by the box predicates. These quirks must not be repaired by rebuilding a
mathematically uniform six-plane set. The line origins copy their corner x/z bits; each direction
component is one far-minus-near subtraction followed by a float32 store. Evidence:
`0x007F8DA5`–`0x007F9060`, `0x007FABC4`–`0x007FAD9B`.

**Predicates:** point-in-frustum first stores/reloads all three position differences. It compares
the yaw-rotated x without storing it; stores/reloads the yaw-rotated z before the pitch stage;
compares the pitch-rotated y without storing it; then compares the negated pitch-rotated z without
storing it (`0x007F7E65`–`0x007F7F32`). Thus it is not numerically identical to projecting a
float32 vector first. Box predicates evaluate each line expression as two separately rounded
differences, two products, then subtraction, with **no computed float32 temporary stores**.
They select opposite support coordinates from the bottom normal's signs, evaluate the top plane
first and bottom second, each as **y×ny + z×nz, then +x×nx, then +d**, comparing the retained
result with zero. Their scratch stores copy coordinate/normal bits only. View and draw share
this graph with their distinct field bases (`0x007F6770`–`0x007F690C`,
`0x007F6910`–`0x007F6AAC`). Inclusion/equality and unordered branches remain §§3.4,7.2's rules.

**Worked extent/corner case:** a stored vector (3,4,0), requested length 10, has squared length
25, root 5, factor 2 and stored result (6,8,0) in either builder. For position (0,0,0), stored
forward=(0,0,−1), right=(1,0,0), up=(0,1,0), volume left/right=−2/2, bottom/top=−1/1,
near/far=2/4, the orthographic near corners are (2,1,−2), (2,−1,−2), (−2,−1,−2),
(−2,1,−2); far corners have z=−4. All values are exact under the ordinary profile. The cached
view far plane is (0,0,1,−4) by the native quirk, rather than the geometric (0,0,1,4).
The independent draw-state fixture in §5.1 exercises the predicates and corner selector without
depending on a camera provider.

## 4. Coordinates *[compat]*

**View coordinates** (`ProjectPointToViewCoordinates`, `0x007F77D0`): d = p − camera position;
x₁ = d.x cos(−yaw) − d.z sin(−yaw), z₁ = d.x sin(−yaw) + d.z cos(−yaw); then
x = x₁, y = d.y cos(pitch) − z₁ sin(pitch), z = d.y sin(pitch) + z₁ cos(pitch). Depth is −z.
`ProjectPointToFixedViewCoordinates` (`0x007F6710`) applies the same rotation without the translation.

**Viewport coordinates** (`ProjectPointFromViewCoordinatesToViewportCoordinates`, `0x007F66C0`):
vx = (x − left) sx, vy = (top − y) sy, vz = (z + near) sz, so vz is 0 at the near plane and −1 at the far plane.

| Function | VA | Effect |
|---|---|---|
| `ProjectPointToViewportCoordinates` | `0x007F7850` | world to view to viewport |
| `ProjectPointToViewportCoordinates2D` | `0x007F78E0` | the same, leaving z in view coordinates |
| `ProjectPointToViewportZ` | `0x007F78A0` | only vz |
| `UnProjectPointFromViewCoordinatesToWorldCoordinates` | `0x007F7920` | the inverse rotation, then + camera position |
| `UnProjectPointFromViewportCoordinatesToWorldCoordinates` | `0x007F79B0` | viewport to view (with the inverse scales) to world |
| `ProjectPointFromViewportCoordinatesToFixedViewCoordinates` | `0x007F8390` | viewport to world, then the fixed-view rotation |

**Numeric evaluation and stores (VU12):** the equations above are ordered operations under §2.3,
with the following additional boundaries. Write cy/sy for stored cos/sin of −yaw, cp/sp for
stored cos/sin of pitch, and distinguish sy from the viewport scale of the same conventional name.

| Path | Evaluation order and memory boundaries |
|---|---|
| Fixed-view rotation, `0x007F6710` | x₁=(x×cy)−(z×sy), store x; z₁=(z×cy)+(x×sy), store/retain z₁; y=(y×cp)−(z₁×sp), store y; z=(z₁×cp)+(original y×sp), store z. Original x/z and original y remain available in registers; do not read overwritten components. |
| World-to-view, `0x007F77D0` | Subtract position in x,y,z order. x/y differences store/reload; z difference store/retain. Then the fixed-view graph, using that retained z difference; z₁ is again store/retain. |
| View-to-viewport, `0x007F66C0` | For x, subtract left, store/retain, multiply sx, store final x. For y, form top−y, store/retain, multiply viewport sy, store final y. For z, form near+z, store/retain, multiply sz, store final z. |
| World-to-viewport, `0x007F7850`; 2D, `0x007F78E0` | Call world-to-view, so its three final stores are reloaded. Apply the viewport graph in x,y,z order; the 2D entry stops after y and leaves the stored view z. |
| Scalar viewport Z, `0x007F78A0` | Form (z−position.z)×cy and (x−position.x)×sy, add them, multiply cp; form (y−position.y)×sp and add; add near, multiply sz. No internal float32 store; return the register result. This is not the stored-vector projection graph. |
| View-to-world, `0x007F7920` | Form y′=(y×cp)−(z×(−sp)), store y′; z′=(y×(−sp))+(z×cp), store/retain z′. Form x′=(x×cy)−(z′×(−sy)), store x′; z″=(x×(−sy))+(z′×cy), store z″. Then reload x′,y′,z″ and add the corresponding position component, storing x,y,z. |
| Viewport-to-world, `0x007F79B0` | In z,y,x order, multiply by the corresponding inverse scale and store/retain the product; subtract near for z, form top−product for y, add left for x, storing each final component. Then call view-to-world. |
| Viewport-to-fixed, `0x007F8390` | The preceding viewport-to-world graph including its stores, then fixed-view rotation on the resulting world vector. |

Evidence: the complete small bodies `0x007F66C4`–`0x007F676A`,
`0x007F77D4`–`0x007F79F8`, `0x007F8394`–`0x007F83DE`. None has an intermediate double
memory store. A store/retain is observable through aliasing or overflow/underflow, but is **not**
permission to round the register operand again before its next operation. Point-in-frustum uses
the different stores described in §3.5.

**Worked projection case:** position=(10,20,30), cy=cp=1, sy=sp=0, volume left=−2, top=4,
near=2 and scales sx=2, viewport sy=3, sz=0.25. World point (11,22,26) projects to view
(1,2,−4), then viewport **(6,6,−0.5)**; the 2D entry instead leaves z=−4, and scalar
viewport-Z returns −0.5. Inverse scales (0.5,1/3,4) return the viewport triple to world
(11,22,26) under the ordinary profile (the stored float32 inverse 1/3 rounds the product 6×1/3
to 2). This case checks graph wiring without relying on trig accuracy.

`ProjectBoundingBoxToViewportRectangle(min, max, &topLeft, &bottomRight)` (`0x007F7A40`) projects four chosen box
corners (2D projection) and rounds: left and top to nearest of (v − 0.5), right and bottom to nearest of v. The
corners, as (x, y, z) picks of min / max:

| Rotation | left | top | right | bottom |
|---|---|---|---|---|
| 0 | (min, max, max) | (min, max, min) | (max, max, min) | (max, min, max) |
| 1 | (min, max, min) | (max, max, min) | (max, max, max) | (min, min, max) |
| 2 | (max, max, min) | (max, max, max) | (min, max, max) | (min, min, min) |
| 3 | (max, max, max) | (min, max, max) | (min, max, min) | (max, min, min) |

A rotation outside 0–3 leaves the outputs unwritten.

### 4.1 Rays and the ground

- `ViewportPositionToRay(x, y, &origin, &direction)` (`0x007F9080`): the camera's eye ray through pixel (x, y),
  direction normalised; false without a renderer, a camera or a ray.
- `GetRayIntersectionWithGroundPlane(origin, direction, &out)` (`0x007F6AB0`): out = origin + t direction with
  t = −origin.y ÷ direction.y, unguarded.
- `GetViewportPointIntersectionWithGroundPlane(x, y, &out)` (`0x007F9160`): the pixel's ray met with y = 0; it
  always returns true and does not check the ray's result *[quirk]*.
- `RayIntersectsGroundPlane(origin, direction, &out)` (`0x007F6B50`): the geometry utilities' ray–plane test with the
  plane (0, 1, 0, 0).
- `GetViewportRectGroundCorners` (`0x007F9EA0`): for a pixel rectangle (top-left, bottom-right), the ground (x, z) of
  its four corners in the order top-left, top-right, bottom-left, bottom-right.

**Failure writes (VU7):** `ViewportPositionToRay` returns false without writing either caller output when
the renderer, camera or camera eye-ray operation is absent/fails (`0x007F9080`–`0x007F90E3`,
`0x007F9150`). Its zero-initialized private eye-ray temporaries are not copied on failure. The ground wrapper
instead allocates six **uninitialized** float slots, passes them as the two outputs, ignores the boolean, and
immediately divides their y components (`0x007F9160`–`0x007F9183`). Its failure result is always true but its
coordinate is indeterminate stack-dependent data, not specified zeroes or NaNs. A portable implementation
may expose a checked adapter; zero-initializing that adapter does not reproduce a native failure coordinate.
On success the wrapper evaluates t = −(origin.y / direction.y), then each output component as
direction.component × t + origin.component; it does not force output.y to literal zero.

### 4.2 City helpers

- `GetCityCornerPosition(corner, &out)` (`0x007F6BA0`): out = (x, 0, z) by corner and rotation, with W = `+0x20`,
  D = `+0x24`:

| Corner | rotation 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| 0 | (0, D) | (0, 0) | (W, 0) | (W, D) |
| 1 | (0, 0) | (W, 0) | (W, D) | (0, D) |
| 2 | (W, 0) | **(W, W)** | (0, D) | (0, 0) |
| 3 | (W, D) | (0, D) | (0, 0) | (W, 0) |

  Corner 2 at rotation 1 uses the x size for z *[quirk]*; cities are square, so it does not show. Other corner or
  rotation values leave x and z unwritten.
- `GetVectorForEdge(edge)` (`0x007F6D10`), edge mod 4: 0 (0, 0, −1), 1 (1, 0, 0), 2 (0, 0, 1), 3 (−1, 0, 0).
- `GetVisibleEdges(&a, &b)` (`0x007F7750`): a = (rotation + 1) mod 4, b = (rotation + 2) mod 4.
- `ClipWorldPositionToCityPosition(p)` (`0x007F7710`, with `ClipPositionToCity2D` `0x007F6620`): x below 0 becomes 0
  and x ≥ W becomes W − 1; z likewise with D; y is kept.
- `RoundDownToNearestBlockBoundary(v, block)` (`0x007F6D70`) and `RoundUpToNearestBlockBoundary` (`0x007F6DB0`):
  v is first **truncated** to an integer, then rounded down (up) to a multiple of block. So RoundUp(0.5, 64) = 0
  and RoundDown(−0.5, 64) = 0 *[quirk]*.

### 4.3 Discrete vector rotation (VU11)

`RotateVectorByViewRotation(&v, rotation)` (`0x007F6CC0`, a free cdecl helper; export name supplied by
lighting) belongs to this utility family, not to the yaw/pitch projection. It changes v in place using
the **signed C remainder** r = rotation % 4:

| r | resulting (x, y, z) |
|---|---|
| 0 | unchanged |
| 1 | (−old.z, old.y, old.x) |
| 2 | (−old.x, old.y, −old.z) |
| 3 | (old.z, old.y, −old.x) |
| −1, −2, −3 | unchanged |

There is no angle, normalization, translation, validity check or pointer check. The y word is never written.
Rotation −4 is unchanged, −3 is unchanged, and 5 acts as 1. The signed remainder adjustment is at
`0x007F6CC4`–`0x007F6CCF`; negations use x87 sign change and store float32, while the unnegated swapped
component is copied as a raw word (`0x007F6CD9`–`0x007F6D0A`). Finite inputs, infinities and signed zero
follow those operations; a signaling NaN is subject to the active x87 exception mode. No null-vector fallback
exists. `GetVectorForEdge` uses the same signed remainder: negative nonmultiples of four set y to +0 but
leave x/z untouched (`0x007F6D18`–`0x007F6D2D`), rather than selecting the Euclidean-modulo edge.

### 4.4 Integer operations (VU8)

Pixel bounds use the **float-bit bias conversion**, not FISTP or a saturating float-to-int operation:
add float32 constant 12582912 (`0x4B400000`) to the projected coordinate (after subtracting 0.5 for
left/top), store once to float32, interpret its word as an integer, and subtract `0x4B400000` modulo
2³² (`0x007F7B84`–`0x007F7BE3`; the other rotation arms repeat this). With round-to-nearest/even,
this produces nearest/even while the biased result lies in [2²³,2²⁴). For example left input 2.0
uses 1.5 and yields 2; left input 3.0 uses 2.5 and also yields 2. Right inputs 2.5/3.5 yield 2/4.
Outside that range, including infinity and NaNs, the result follows the bit operation, not int32 range
validation. An unmasked x87 exception can interrupt a conversion; no local handler is installed.

Block helpers call `_ftol2` (`0x009EEF04`, signed 64-bit truncation) and consume only its low 32-bit
word as signed n. Finite values outside int32 but within int64 therefore wrap before block rounding.
Subsequent operations are signed IDIV (quotient truncates toward zero) and 32-bit wrapping arithmetic:
down first subtracts block from n when n≤0 and the signed division remainder is nonzero; up first
adds block when n≥0 and that remainder is nonzero. Both then divide the adjusted n by block and
multiply the quotient by block, returning the signed product through x87 FILD
(`0x007F6D75`–`0x007F6DA0`, `0x007F6DB5`–`0x007F6DE0`). They do not reject block≤0 or overflow.
Block zero raises integer divide error; INT_MIN divided by −1 can also raise it. Negative blocks
follow these same steps, e.g. Down(−65,−64)=0 and Up(65,−64)=0. Up(2147483647,64) wraps to
−2147483584. With invalid conversion masked, nonfinite/out-of-int64 input makes the runtime's
signed64 conversion return integer indefinite (high word `0x80000000`, low word zero); these helpers
therefore consume n=0. With invalid conversion unmasked, execution can trap instead
(`0x009EEF13`, `0x009EEF63`–`0x009EEF78`). There is no helper-supplied failure result.

The draw rectangle centre uses a **32-bit wrapping sum** of the two coordinates and signed division
by two, truncating toward zero; subtractions from that centre also wrap before conversion to float.
It neither widens to int64 nor clamps/reorders the rectangle. The native operation is visible in the
rectangle-centre sequence of `0x007FA31D` onward. For left=INT_MAX, right=INT_MAX, centre.x is −1;
for left=−4, right=1 it is −1. Inverted/empty rectangles take the regular narrowed-frustum path.

## 5. The draw frustum (`BuildDrawFrustumDataFromViewportRect(topLeft, bottomRight)`, `0x007F9F50`) *[compat]*

Drawing may be limited to a dirty rectangle of the viewport (pixels); the renderer's draw pass calls this with its
rectangles (view3d.md §4.1). The call clears both visible-area caches and stores the rectangle (`+0x22C`). For the
whole viewport the draw frustum is a copy of the view frustum (flag `+0x228` := 0), and the four viewport ground
points (`+0x120`) are computed if not cached. Otherwise (flag := 1), with c the rectangle's centre (integer halves):

1. The draw view box: left and right := (rect left − c.x) × factor and (rect right − c.x) × factor, top and bottom
   := (c.y − rect top) × factor and (c.y − rect bottom) × factor, factor = `+0x150`; when the rectangle spans the
   full width (or height), that pair is the camera's own. Near and far are the camera's.
2. The ground points (`+0x2E4`) of the rectangle's corners (§4.1).
3. Corners as in §3.1 around the line through c (c unprojected at viewport depth 0, moved back by near along
   forward), then the box, the six planes and the two ground lines as in §3.2, and the city-clipped box (§3.3).
   The perspective case has the same defect as §3.1.

`IsBoundingBoxInDrawFrustum` then tests against this frustum.

### 5.1 Cached draw-state layout and isolated fixtures (VU18)

Offsets below are relative to the **0x470-byte utilities owner**, not to a separately allocated
frustum. All geometry scalars are float32; vectors are packed x,y,z triples. A box is min followed
by max, each a triple. A plane is nx,ny,nz,d. These are the fields actually copied or built by §5:

| Owner offset | Bytes | Meaning | Whole-viewport source |
|---|---:|---|---|
| `+0x228` | 1 | narrowed flag | byte zero |
| `+0x229`–`+0x22B` | 3 | alignment padding; no §5 geometry field | not copied |
| `+0x22C/+0x230/+0x234/+0x238` | 16 | signed int32 left, top, right, bottom viewport rectangle | supplied arguments |
| `+0x23C/+0x248` | 24 | frustum AABB min/max | `+0x78/+0x84` |
| `+0x254/+0x260` | 24 | city-clipped AABB min/max | `+0x90/+0x9C` |
| `+0x26C + 12i`, i=0…7 | 96 | corners in §3.1 order | `+0xA8 + 12i` |
| `+0x2CC/+0x2D0/+0x2D4/+0x2D8/+0x2DC/+0x2E0` | 24 | volume left, right, bottom, top, near, far | `+0x108` through `+0x11C` |
| `+0x2E4 + 12i`, i=0…3 | 48 | ground points TL, TR, BL, BR | `+0x120 + 12i` (freshly cached) |
| `+0x314 + 16i`, i=0…5 | 96 | right, left, bottom, top, near, far planes | `+0x160 + 16i` |
| `+0x374 + 12i`, i=0…5 | 72 | ground-reduced plane triples (nx,nz,d) | `+0x1C0 + 12i` |
| `+0x3BC/+0x3C0/+0x3C4/+0x3C8` | 16 | line A: origin x,z; direction x,z | `+0x208/+0x20C/+0x210/+0x214` |
| `+0x3CC/+0x3D0/+0x3D4/+0x3D8` | 16 | line B: origin x,z; direction x,z | `+0x218/+0x21C/+0x220/+0x224` |

There are no other holes between `+0x22C` and the end at `+0x3DC` (exclusive). In particular,
`+0x374`–`+0x3BB` is **not padding**: the narrowed builder writes the first two reduced-plane
triples from its right/left planes and zeros the other four; the whole-viewport branch copies all
six from the view state. The source view reduced-plane array was omitted from §1's summary table.
The first box, second box, corners and volume have displacement `+0x1C4` from their view counterparts;
planes, reduced planes and lines instead have displacement `+0x1B4`. A universal displacement is wrong.
Evidence: whole-copy stores `0x007F9FCA`–`0x007FA1A5`, array copies
`0x007FA217`–`0x007FA2AE`, line stores `0x007FA2B0`–`0x007FA30B`.

The draw box predicate consumes lines A/B and bottom/top planes at `+0x334/+0x344`, without
consulting the narrowed flag or either AABB (`0x007F6913`–`0x007F6A9E`). The false branch of
`GetFrustumAABBCityClipped` selects corner base `+0x26C` directly (`0x007F7F89`–`0x007F7F97`);
it does not use the cached city-clipped box as input.

**Worked fixture:** in a private owner set line A to (10,0,0,1), line B to (0,0,0,1), bottom plane
to (0,1,0,0), and top plane to (0,−1,0,10). The box min=(2,2,2), max=(3,3,3) passes the draw
predicate. The box min=(10,2,2), max=(11,3,3) fails on line A; changing `+0x228` alone cannot
change either result. For the city-clipped-box call, use city W=D=20 and draw corners
0=(2,4,3), 3=(8,4,3), 4=(2,−4,11), 7=(8,−4,11). The false selector returns
min=(2,0,3), max=(8,4,7), regardless of the view corners and cached draw-box canaries. Other
corners are unconsumed by that call. These read-only fixtures need no renderer/camera provider.

**Failure state:** no valid-flag check precedes this operation. The whole-viewport branch copies the
cached view state and runs its ground wrappers even with an absent renderer/camera; failed wrappers
have the indeterminate coordinates described in §4.1. The narrowed branch directly dereferences the
borrowed renderer to ask for its camera (`0x007FA31D`–`0x007FA322`), so a null renderer can fault.
If that renderer returns a null camera, it exits after the initial rectangle stores/cache invalidations,
before writing the narrowed flag, draw geometry or draw ground corners (`0x007FA325`–`0x007FA32B`).
Those earlier draw fields remain stale; no zero-frustum or false-return contract exists. Likewise a
failed §2 camera update clears validity/caches but retains previously computed geometry.

## 6. Visible area (`GetGroundPlaneVisibleAreaRectangle(&innerMin, &innerMax, &outerMin, &outerMax)`, `0x007F9B40`)

Computed once per camera update or draw rectangle (cache flag `+0x444`) from the ground points P₀–P₃ under the
viewport corners (top-left, top-right, bottom-left, bottom-right):

- **Inner** (`+0x414`): the x–z box of the midpoint of the top edge, (P₀ + P₁)/2, and of the bottom edge, (P₂ + P₃)/2.
- **Outer** (`+0x42C`): the x–z box of P₀–P₃ (`0x007F6550` takes a minimum of four, `0x007F65B0` a maximum), with
  the minimum rounded down and the maximum up to multiples of **64 m**, after truncation as in §4.2.

Every y is 0. Any output pointer may be null.

**Cache source and ordering (VU9):** `+0x444` validates the four visible-area vectors at `+0x414`–`+0x440`.
On a miss §6 casts four fresh full-viewport rays in TL, TR, BL, BR order, even when all four output pointers
are null (`0x007F9B46`–`0x007F9BC7`). It neither reads nor sets `+0x445` and does not consume the draw-corner
array. `+0x445` exclusively validates the full-viewport ground-corner array at `+0x120`–`+0x14C` for the
**whole-viewport branch** of §5. Both flags are cleared at the entry to every camera update and every draw
rectangle build (`0x007F83FE`, `0x007F9F64`–`0x007F9F6A`). Consequently the whole-viewport branch always
recasts its four rays before setting `+0x445` (`0x007FA1AB`–`0x007FA210`), and copies that array into the
draw ground corners; a narrowed rectangle casts its own four rays and leaves `+0x445` clear. Calling §6 after
either branch casts another four full-viewport rays once, then reuses only its own rectangle cache. A second
§6 call, without an intervening invalidation, makes no ray calls. Thus the two caches agree in steady camera
state but are independent observations of a mutable camera provider.

## 7. Picking *[compat]*

A pick record holds the ray origin (`+0x00`), its direction (`+0x0C`) and the nearest hit distance so far (`+0x28`).

That nearest field is Euclidean distance from origin to the intersection point, **not** the signed triangle
ray parameter. Both TrianglePick and MeshPick call LengthOfLine after intersection (`0x007F6B23`,
`0x007F9993`). Only a strict ordered decrease updates it: ties and NaNs do not improve it. An accepted hit
behind the origin still has positive distance, and a nonunit direction does not rescale that distance.
See model-instances §2.1 and its conformance vector 17 (MI5).

- `TrianglePick(pick, a, b, c)` (`0x007F6AF0`): when the ray hits the triangle (`SC4GeometryUtilities::
  RayIntersectsTriangle`) closer than the nearest distance, that distance is updated and the result is true.
- `MeshPick(pick, mesh, transform)` (`0x007F91C0`): for a mesh with vertices, each of its primitive groups is walked
  triangle by triangle (§7.1); each triangle's vertices are transformed (below) and tested as above. **The first
  triangle that is closer than the nearest distance ends the mesh** with true *[quirk]*: a nearer triangle later in
  the mesh is not found.
- `ModelPick(pick, instance, model)` (`0x007F9A40`): for each of the model's meshes (slot 3 the count, slot 5 a mesh),
  the transform starts as the identity of kind 0 and the instance fills it (slot 9); true when any mesh hit. The
  model instances' pick methods call it (for example through `0x00801720`).

Transforms: a kind byte (`+0x01`), a 3 × 3 matrix M (rows at `+0x04`, `+0x10`, `+0x1C`), a translation t (`+0x28`)
and a uniform scale s (`+0x34`). Kind 0: none; 1: v + t; 2: s v; 3: s v + t; 4: M v; 5: M v + t; any other: s (M v) + t.

### 7.1 Triangle walk (`0x0080AE60`–`0x0080B3B0`)

A mesh refers to a **separate vertex-buffer record** (position in the first 12 bytes of each vertex;
16-bit stride at buffer `+0x04`, data pointer at buffer `+0x08`, count at buffer `+0x06`),
optional 16-bit indices, and groups of 12 bytes (type, first, count). Without indices the group's vertices start at
`first`; with them its indices do. Triangle i of a group of count n, by type (`0x0080AE80` the count,
`0x0080AEF0` the indices):

| Type | Triangles | Triangle i |
|---|---|---|
| 0 | n / 3 | (3i, 3i + 1, 3i + 2) |
| 1 | n − 2 | (i, i + 1, i + 2) |
| 2 | n − 2 | (0, i + 1, i + 2) |
| 3 | n | (i, i, i) |
| 4 | n / 2 | (2i, 2i + 1, 2i) |
| 5 | n − 1 | (i, i + 1, i) |
| 6 | n / 2 | (2i, 2i + 1, 2i + 2) for even i, (2i, 2i + 2, 2i + 3) for odd i |
| 7 | n − 2 | (j, j + 1, j + 2) with j = i + (i mod 2) |

Other types have no triangles. Types 3–5 give degenerate triangles, which the ray test misses. Types 6 and 7 are
quads and quad strips (graphics-formats.md §3.7); their second triangles do not follow the quads, but no shipped
model uses either type *[quirk]*. The index reader
(`0x0080B090`) knows eight index formats (signed and unsigned 8 and 16 bits, two 32-bit codes, float and double,
floored), but the walk only uses unsigned 16 bits.

The native count helper uses signed subtraction without saturation for n−1/n−2 (`0x0080AEA8`, `0x0080AEB8`);
the iterator's signed comparison makes nonpositive counts stop immediately (`0x0080B3B6`). Invalid vertex
or index references are unchecked memory accesses, not native skip rules (model-instances §9.1, MI9).

#### 7.1.1 Private mesh resource graph (VU17)

The mesh argument is a **12-byte, three-pointer record**, without a vtable. All pointers here are
32-bit process addresses. They must point to separately accessible records/arrays for the entire
synchronous call; MeshPick and iterator select-group neither acquire references nor destroy them.

| Record | Offset/type | Consumed meaning |
|---|---|---|
| Mesh (0x0C bytes) | `+0x00` pointer V | vertex-buffer record; null makes MeshPick return false |
| Mesh | `+0x04` pointer I | optional index descriptor; null means nonindexed |
| Mesh | `+0x08` pointer G | group descriptor; required once V has a nonzero vertex count |
| Vertex buffer V (native allocation 0x30 bytes) | `+0x00` uint32 | vertex-format word, copied to iterator `+0x0C` |
| V | `+0x04` uint16 | byte stride, zero-extended to iterator `+0x10` |
| V | `+0x06` uint16 | vertex count; MeshPick's early nonzero check |
| V | `+0x08` pointer | first vertex byte, copied to iterator `+0x14` |
| V | `+0x0C`–`+0x2F` | unconsumed by these two calls; no bounds/refcount initialization needed for this fixture |
| Index descriptor I | `+0x00` pointer | first uint16 index; **another indirection**, not inline indices |
| Group descriptor G (0x10 bytes) | `+0x00` uint16, `+0x02` two bytes | ownership word and padding, unconsumed here |
| G | `+0x04` pointer | first 12-byte group record |
| G | `+0x08` pointer | one-past-last group record |
| G | `+0x0C` pointer | allocation end; unconsumed here |
| Group (0x0C bytes) | `+0x00/+0x04/+0x08` int32 | primitive type, first element, element count |

MeshPick derives the group count as signed byte difference (G.end−G.begin) divided by 12,
truncated toward zero; it does not read a scalar count field. Use well-formed begin/end pointers
and nonnegative counts in fixtures. Select-group indexes G.begin directly, without checking G.end.
It copies type/count and V's format/stride/data, sets iterator index-format to **3 (uint16)**,
computes the triangle count, and resets current triangle `+0x28` to zero. With I present it sets
the iterator index pointer to I.data + 2×first and leaves the vertex base unchanged. Without I it
sets the index pointer to null and advances the vertex base by stride×first. The vertex-format
word does not change the 12-byte position copy: the direct copy helper always copies x,y,z bits.
Only I's first pointer is read: a private four-byte descriptor is sufficient; no index count,
format, vtable, capacity or other tail field is required by this observer contract. This is a
consumed-record layout, not a promise about a resource owner's larger allocation/lifetime ABI.

Evidence: V dereference/count `0x007F91CE`–`0x007F91E0`; G count
`0x007F9218`–`0x007F9238`; complete select-group `0x0080B320`–`0x0080B3A5`;
12-byte mesh-array stride `0x00806AF4`–`0x00806AF8`; V allocation/initialization
`0x00801214`–`0x00801226`, `0x006156B7`–`0x00615721`; G allocation/initialization
`0x0080124D`–`0x00801264`. Do not pass these borrowed fixture records to native resource
destructors, builders, mutation or ownership routines.

**Worked fixture:** V.format=1, V.stride=12, V.count=4, V.data points to
v0=(99,99,99), v1=(0,0,0), v2=(2,0,0), v3=(0,2,0). G.begin points to one group
(type=0, first=1, count=3); G.end=G.begin+12 and G.allocationEnd=G.end. With mesh.I=null,
select-group produces vertex base V.data+12, index pointer null, triangle count 1 and current 0;
next-triangle returns v1,v2,v3. With a separate uint16 array (99,1,2,3), I.data pointing to it,
and mesh.I=&I, the same group produces vertex base V.data, index pointer I.data+2 and the
same triangle. For either variant, kind-0 transform, pick O=(0.5,0.5,2), D=(0,0,−1) and
nearest=100, MeshPick returns true and stores nearest=2. Repeating with nearest=2 returns
false and preserves it. Setting V.count=0 returns false even if G is null; count>0 requires
the full graph. ModelPick can reuse this graph behind its documented model slot5 provider.

### 7.2 Geometry predicates, ordering and unsafe resources (VU6/VU10)

`RayIntersectsTriangle` (`0x005D6FF0`) is two-sided Möller–Trumbore with **absolute determinant
tolerance 0.01f**, bits `0x3C23D70A` from `0x00A823A4`. It rejects |det| below that value, including
degenerate triangles; equality is accepted (`0x005D707F`–`0x005D709E`). Barycentrics include all
three edges: u≥0, u≤1, v≥0, u+v≤1. There is **no t sign test**: t=0 and intersections behind the
origin both succeed (`0x005D70ED`–`0x005D71D7`). The optional output is the float32 intersection
point O+tD, not t. On geometric rejection it is unwritten. The status-word branches also allow unordered
determinant/barycentric values through: a NaN can yield true with NaN intersection coordinates
(`0x005D708B`, `0x005D70F9`, `0x005D710E`, `0x005D7179`, `0x005D718E`).
`TrianglePick` computes the Euclidean length from origin to that returned, rounded point using
`LengthOfLine`, compares it strictly with the saved nearest distance, and updates only if smaller
(`0x007F6B1D`–`0x007F6B35`). Equal or unordered distance leaves nearest untouched. Thus scaling
the direction changes parameter t but not the intended metric, and a behind-origin hit can win.

`RayIntersectsPlane` (`0x005D7360`) forms a=n·O+d and b=n·D. For |b|≥0.01f it writes O−(a/b)D
and returns true, without requiring forward t. For |b|<0.01f, |a|<0.01f is considered coplanar:
return true and copy O. Otherwise return false and **write (0,0,0)** to a nonnull output
(`0x005D7397`–`0x005D7416`). A null output is allowed. Unordered |b| takes the division branch;
with a small finite |b|, unordered |a| takes the false/zero-output branch. The unguarded §4.1 arithmetic helper is distinct.

The city clipper's subject order is corner 3, corner 0, ground hit from corner 0, ground hit from
corner 3. The city clip polygon is (0,0),(0,D),(W,D),(W,0); edges are visited **last→first** then
0→1, 1→2, 2→3, giving z≥0, x≥0, z≤D, x≤W in that order (`0x007F7F60`, `0x005D79D0`).
For edge A→B, inside means (P.x−A.x)(B.z−A.z)−(P.z−A.z)(B.x−A.x)≥0, with equality included.
This requires the clip polygon's stated winding. At each edge, walk subject last→first then each
successive vertex: inside→inside emits current, outside→inside emits intersection then current,
inside→outside emits intersection, outside→outside emits nothing. Output order and duplicate
boundary points are retained; there is no deduplication or degeneracy cleanup. Empty output remains
empty on later edges. `ClipPolygonToPolygon2D` uses a 40-point scratch polygon and truncates each
edge's result to caller capacity; the view caller capacity is 10. It does not sort extrema before clipping.
NaN half-space expressions classify outside (`0x005D7854`–`0x005D7869`). Intersections call
`RaysIntersect2D`, which rejects a determinant magnitude below the same 0.01f threshold and leaves
its output unwritten (`0x005D7455`–`0x005D7473`); the clipper ignores that return. Near-parallel
crossings can consequently emit stale/indeterminate intersection data. No deterministic cleaned
polygon is promised for such a failure. The view wrapper ignores plane-intersection booleans too;
its plane miss writes zero by the rule above. If clipping leaves no points, output x/z extrema are
zero; min.y=0 and max.y remains corner3.y, rather than an all-zero 3D box.

Triangle counts and indices are signed 32-bit calculations. Counts n−1/n−2 are **not saturated**;
the iterator uses signed current≥triangleCount to stop, so ordinary undersized counts produce no
triangles. Index/first/stride address arithmetic wraps at 32 bits. There is no range validation against
vertex/index counts and no stride≥12 check: each referenced position copies 12 bytes unconditionally
(`0x0080B020`–`0x0080B082`). A stride below 12 overlaps records; invalid references can fault or read
other accessible memory. These are unsafe-input effects, not skipped-triangle guarantees. A missing
vertex-buffer pointer or zero vertex count makes MeshPick return false before iterating
(`0x007F91CE`–`0x007F91E0`). The mesh pointer itself and its group structures are dereferenced without
null guards. ModelPick does not skip a null mesh returned by model slot5: it calls the instance's transform
slot9 then MeshPick, so malformed resources can fault. It visits all meshes even after a hit; each mesh
stops on its first strict improvement, preserving model order (`0x007F9A70`–`0x007F9B1B`).

The matrix wording in §7 is literal **row dot products**: output x uses +04/+08/+0C, y uses
+10/+14/+18, z uses +1C/+20/+24 (`0x007F943C`–`0x007F949A`). Kind5 adds translation to the row results.
The default kind applies uniform scale after each row dot and
then translation. Its row sums evaluate z×row[2], then add y×row[1], then x×row[0]. In the first
vertex's default arm, x/y row sums stay in registers until multiplying scale and storing float32, then
translation is added; the z row sum is first stored float32, reloaded for scale, and translation is
added before its final float32 store (`0x007F977B`–`0x007F9820`). This store asymmetry matters to
bit-level reproductions. Do not transpose M or rotate its translation. A kind byte above 5 always takes the
default, including 255; there is no malformed-kind rejection.

### 7.3 Complete three-vertex transform stores and pick distance (VU12)

For each vertex, the matrix row dot evaluates **z×row[2], then +y×row[1], then +x×row[0]**,
with each multiply and add rounded independently under §2.3. There is no store inside the row
reduction. Vertices are processed first, second, third; each vertex's x/y/z rows are evaluated
in that order. The table lists the additional float32 **store/reload** boundaries before the
final output stores; an absent boundary means the value remains in a register. It applies to
**all three vertices**, including the previously unspecified second and third default arms.

| Transform kind | x path | y path | z path |
|---|---|---|---|
| 0 | bit copy | bit copy | bit copy |
| 1 | add tx, final store | add ty, final store | add tz, final store |
| 2 | multiply s, final store | multiply s, final store | multiply s, final store |
| 3 | multiply s, store/reload, add tx | multiply s, store/reload, add ty | multiply s, retain product, add tz |
| 4 | row dot, final store | row dot, final store | row dot, final store |
| 5 | row dot, retain sum, add tx | row dot, store/reload, add ty | row dot, store/reload, add tz |
| any other byte | row dot, multiply s, store/reload, add tx | row dot, multiply s, store/reload, add ty | row dot, store/reload, multiply s, retain product, add tz |

Every arithmetic output is finally stored float32 before triangle intersection. Kind0 copies
bits without executing floating arithmetic. For arithmetic arms the final component store order
is z,x,y (temporary z stores followed by an integer bit copy do not introduce another rounding).
In particular, the default arm does **not** store all row sums before scale, or all scaled
products before translation. Second-vertex evidence is `0x007F9824`–`0x007F98C2`, third-vertex
evidence `0x007F98C6`–`0x007F995A`; first vertex `0x007F977B`–`0x007F9820`.
Other kind arms: `0x007F92D0`–`0x007F9776`.

**Worked order case:** put (16777216,1,−16777216) in any of the three vertex positions and
use each matrix row (1,1,1), scale=2, translation=(3,4,5), kind=6. In the ordinary profile the
row reduction is (−16777216+1)+16777216 = **1**; the transformed point is **(5,6,7)**.
An x,y,z left-to-right reduction gives zero and therefore (3,4,5). Moving this vertex among
the three slots must preserve (5,6,7); the stored/reloaded z row also equals 1 in this exact
case. This is a transform-unit vector; the three identical-row matrix collapses a triangle
and is not itself a successful ray-picking fixture. Use §7.1.1 for a complete nondegenerate pick.

For pick distance, `LengthOfLine` subtracts intersection minus origin in x,y,z order without
storing the differences; forms **z²+y², then +x²**; takes the square root and returns that
register value, with no internal float32 store (`0x005D7668`–`0x005D768E`). TrianglePick and
MeshPick compare this retained length with the saved float32 nearest first; only a strict
ordered improvement stores the length to nearest (`0x007F6B23`–`0x007F6B35`,
`0x007F9993`–`0x007F9A00`). Do not round a distance capture to float32 **before** doing the
native comparison when modelling another precision profile.

The arithmetic ground helper similarly retains its factor: divide origin.y by direction.y,
negate that quotient, then for x,y,z multiply by that direction component, add the origin
component, and store the final component float32. There is no quotient/product memory store
(`0x007F6AB8`–`0x007F6ADD`). This describes the unguarded helper, not the tolerance-based
external ray/plane or ray/triangle predicates in §7.2. Their geometric accept/reject rules,
rounded intersection outputs and unchecked-resource limits remain in that section; this
answer does not claim a universal bit model for external camera/model providers or hardware trig.

## 8. Users

The 3D renderer updates the object on camera changes; the terrain, model, overlay and automata views test their
boxes against the frustums; the tools pick through §4.1 and §7; the animation coordinator reads §6.

## 9. Quirks

1. The box tests skip the near and far planes (§3.4).
2. `GetViewportPointIntersectionWithGroundPlane` ignores a failed ray (§4.1).
3. `GetCityCornerPosition` corner 2 at rotation 1 uses the x size for z (§4.2).
4. The block rounding truncates first (§4.2, §6).
5. A mesh pick stops at the first closer triangle (§7).
6. Primitive types 6 and 7 (quads and quad strips) index unusually (§7.1).
7. A perspective camera's far corners are scaled from their previous values (§3.1).

## 10. Open questions

VU1–VU11 are archived in [answered questions](questions/answered/view-utilities.md). Reserved fields
have only the observable defaults stated in §2; their original source-level names are unknown. Native
indeterminate memory/floating exception paths are not replaced here by invented checked behavior.

## 11. Conformance tests

1. **Angles:** zoom 4, rotation 1 gives yaw = π/2 − π/8 = 67.5° and pitch 45°; groundPitch at zoom 0 is 32.0°.
2. **Ground ray:** origin (0, 100, 0), direction (0, −1, 1)/√2 meets the ground at (0, 0, 100).
3. **Box test:** a box behind the camera but between the four side planes is reported visible.
4. **Edges:** rotation 3 gives visible edges 0 and 1; edge 1 is (1, 0, 0).
5. **Corners:** in a 1024 m city, corner 0 at rotation 0 is (0, 0, 1024).
6. **Rounding:** RoundDown(−65, 64) = −128, RoundUp(65, 64) = 128, RoundUp(0.5, 64) = 0.
7. **Clip:** (−3, 7, 2000) in a 1024 m city becomes (0, 7, 1023).
8. **Pick:** a strip of 4 vertices (type 1) yields triangles (0, 1, 2) and (1, 2, 3).
9. **Viewport depth:** a point at depth near has vz 0; at depth far, −1.
10. **Signed rotation:** v=(2,7,11), rotations 1/2/3 give (−11,7,2)/(−2,7,−11)/(11,7,−2);
    rotation −1 leaves (2,7,11). For edge −1 and output sentinel (9,8,7), the result is (9,0,7).
11. **Failed eye ray:** with a missing renderer and sentinel origin/direction, direct ray creation returns
    false and preserves both vectors. The ground wrapper returns true; assert no deterministic coordinate.
12. **Independent caches:** with a counting eye-ray provider, full draw, §6, §6 produces ray-call totals
    4, 8, 8; narrowed draw, §6, §6 likewise produces 4, 8, 8 from a fresh state. Return a different ground
    mapping for the second set of four calls to distinguish fresh §6 rays from reuse of draw ground corners.
13. **Behind and non-unit direction:** O=(0.25,0.25,2), D=(0,0,2), triangle
    A=(0,0,0), B=(1,0,0), C=(0,1,0) succeeds with t=−1 and hit=(0.25,0.25,0).
    TrianglePick with nearest=3 updates to 2, not −1 or 1. Reversing B/C still hits. A second identical
    pick with nearest=2 is false. On-plane O=(0.25,0.25,0) succeeds at distance zero.
14. **Determinant threshold:** the same triangle with D=(0,0,1/128) misses, preserving the intersection
    sentinel, despite meeting its interior as a line. D=(0,0,1/64) succeeds. Test both windings.
15. **Plane tolerance and writes:** for y=0, O=(3,1,4), D=(1,1/128,0) returns false and writes zero;
    with O=(3,1/256,4) the same D returns true and copies O as coplanar. D=(0,1,0), O=(3,2,4)
    returns true with (3,0,4), despite negative t. Test optional null output separately.
16. **Rectangular city:** W=80,D=48, corner2/rotation1 gives (80,0,80). An invalid corner with
    sentinel (9,8,7) gives (9,0,7). This distinguishes the shipped W-for-z quirk and partial writes.
17. **Off-centre volume:** compare left/right=(−2,2) and (1,5) with all other camera state equal.
    Their generated frustum corners match; projection x of any point differs by −3×sx. Point-frustum
    tests use the changed bounds. No common-centre correction is allowed.
18. **Conversion ties/overflow:** use the pixel-bound ties in §4.4 and centre(INT_MAX,INT_MAX)=−1;
    the integer stage of Up with n=2147483647, block=64 gives −2147483584. This is not a native
    float32 entry input: converting 2147483647 to float32 supplies 2147483648, for which native Up
    gives −2147483648 (§13.2). Down(−65,−64)=0. Fault tests for block zero belong in
    an isolated native harness, not an ordinary in-process portable geometry test.
19. **Matrix orientation:** rows (1,2,3),(4,5,6),(7,8,9), vertex (1,2,4), kind4 produce (17,38,59).
    Kind5 with t=(10,20,30) gives (27,58,89); kind255, s=2 gives (44,96,148).
20. **Ordering:** two overlapping triangles at distances 8 then 2, initial nearest=10, in one mesh
    leave nearest=8. Swap their primitive order and nearest=2. Put them in separate meshes and both
    are visited, yielding 2 in either mesh order. Undersized strip n=1 has raw count −1 and emits none.
21. **Ordered clipping:** clip the already-contained polygon [(1,1),(1,3),(3,3),(3,1)] to
    [(0,0),(0,4),(4,4),(4,0)]; it retains that vertex order. Replace its first point by (0,0) to test
    inclusive boundaries without deduplication. Reverse only the clip winding to distinguish the
    retained half-planes. A wholly disjoint polygon yields zero output vertices; the view wrapper's
    max.y still comes from corner3.
22. **Min/max call frame:** four float32 slots (1,−2,3,−4), in that argument order, give −4/+3
    for min/max. Caller removes 16 argument bytes; no receiver is needed. Store ST(0) to float32
    before comparing the words `0xC0800000`/`0x40400000`. Four quiet NaNs give
    `0x7F7FFFFF`/`0xFF7FFFFF`. With (+0,−0,+0,−0), both give positive zero; with
    (−0,+0,−0,+0), both give negative zero. Equal values do not replace the first selected zero.
23. **Scalar entry width:** RoundUp(float32 word `0x4F000000`,64), captured to float64, is
    exactly −2147483648; RoundUp(float32 word `0x4EFFFFFF`,64) is exactly 2147483520.
    These are distinct from the internal INT_MAX vector in test 18. RoundDown(−65,64)=−128
    with either float32 or float64 capture. For ViewportUnitsToWorldUnits, local metresPerPixel=2
    and float32 input 3 give 6; thiscall removes four argument bytes and leaves the object unchanged.
24. **Receiver-independent callee cleanup:** angle arguments (rotation=1,zoom=4,three distinct
    float32 outputs) produce test 1 and remove 20 argument bytes, with no receiver. The tolerant
    ground-plane predicate on origin (3,2,4), direction (0,1,0), local output returns AL=1,
    output (3,0,4), and removes 12 argument bytes. Do not infer a meaningful upper EAX word.
25. **Isolation and binding:** snapshot all 0x470 owner bytes around each R-class call in §13.1,
    use disjoint local outputs, and require byte equality of the owner. For visible-area queries on
    a local clone with invalid caches, require the cache changes and ray-call counts from test 12;
    the live owner must remain unchanged. Exercise iterator-next on a local 0x2C record and require
    only its current index to advance as specified in §7.1. Reject an unmatched image before any
    native call; `VA 0x007F6550` binds to module base plus `0x003F6550`, not that absolute VA
    in a differently based process. A native harness should also check stack balance and nonvolatile
    registers for each convention; geometry equality alone does not verify a call declaration.

## 12. Complete family checklist and ownership accounting (VU4)

This is an **address/body-byte checklist**, not a claim of core implementation or verification. The
2026-10-22 export and attribution inputs give **48 entries / 19574 bytes** to SC4ViewUtilities, including
the free rotation helper through the free-neighbour rule. Its two separately mapped min/max helpers
add **2 / 174**, making the requested family **50 / 19748**. The earlier **49 / 19670** family total
omitted the 78-byte rotation entry; no byte denominator or maps are changed by this spec. Sizes below
are export `body_bytes`, including switch-table ranges where present (not textual instruction lengths).
Functions owned by cSC43DRender/cSC4View3DWin are not imported into this total. Names in italics below
are descriptive roles for unnamed bodies, not proposed public interfaces. Vectors are float32 triples,
P2 are pairs; mutable arguments are outputs/in-place; required pointers are not null-checked unless stated.

| VA | Bytes | Entry / logical arguments and result | Contract / ownership |
|---|---:|---|---|
| `005D6900` | 668 | *point-array AABB*(points,count,&min,&max) → void | Initialize both from first point even if count≤0; scan subsequent points in order with strict per-component comparisons. NaNs never replace an existing component; a first NaN stays. Used for 8 frustum corners (§3.1); reach-owned helper. |
| `005D6F10` | 59 | *cross*(a,b,&out) → void | a×b; writes x,y,z in order without copying all inputs first, so output aliasing an input affects subsequent components. Reach-owned basis helper (§2). |
| `007F6550` | 87 | MinOfFourFloats(a,b,c,d) → x87 scalar | Start FLT_MAX, replace only on strict smaller comparisons in argument order; NaNs ignored, all-NaN returns FLT_MAX. Separate fn owner (§6). |
| `007F65B0` | 87 | MaxOfFourFloats(a,b,c,d) → x87 scalar | Start −FLT_MAX, replace only on strict greater comparisons; NaNs ignored, all-NaN returns −FLT_MAX. Separate fn owner (§6). |
| `007F6620` | 104 | ClipPositionToCity2D(&point2D) → void | Clamp adjacent x/z floats using dimensions (§4.2); NaNs stay untouched. |
| `007F6690` | 13 | ViewportUnitsToWorldUnits(v) → x87 scalar | v×metresPerPixel (§2.1). |
| `007F66A0` | 21 | GetZoomAndRotation(&zoom,&rotation) → void | Copy cached integer fields (§1). |
| `007F66C0` | 67 | ProjectPointFromViewCoordinatesToViewportCoordinates(&p) → void | §4. |
| `007F6710` | 96 | ProjectPointToFixedViewCoordinates(&p) → void | §4. |
| `007F6770` | 413 | IsBoundingBoxInViewFrustum(min,max) → bool | §3.4. |
| `007F6910` | 413 | IsBoundingBoxInDrawFrustum(min,max) → bool | §3.4. |
| `007F6AB0` | 49 | GetRayIntersectionWithGroundPlane(origin,direction,&out) → void | §4.1. |
| `007F6AF0` | 88 | TrianglePick(&pick,a,b,c) → bool | §7.2, Euclidean metric. |
| `007F6B50` | 66 | RayIntersectsGroundPlane(origin,direction,&out) → bool | §7.2 tolerant plane dependency. |
| `007F6BA0` | 181 | GetCityCornerPosition(corner,&out) → void | §4.2; always writes y=0, even invalid inputs. |
| `007F6CB0` | 13 | GetObjectYRotationAngle(rotation) → x87 scalar | §2.1. |
| `007F6CC0` | 78 | RotateVectorByViewRotation(&v,rotation) → void | §4.3; free-neighbour attribution. |
| `007F6D10` | 78 | GetVectorForEdge(&out,edge) → void | §§4.2–4.3; signed remainder/partial writes. |
| `007F6D70` | 49 | RoundDownToNearestBlockBoundary(v,block) → x87 scalar | §4.4. |
| `007F6DB0` | 49 | RoundUpToNearestBlockBoundary(v,block) → x87 scalar | §4.4. |
| `007F6DF0` | 2326 | SC4ViewUtilities(storage) | Constructor/defaults, §§1–2. |
| `007F7710` | 51 | ClipWorldPositionToCityPosition(&p) → void | §4.2, leaves y. |
| `007F7750` | 44 | GetVisibleEdges(&a,&b) → void | §4.2; additions wrap, signed remainders. |
| `007F7780` | 71 | GetCameraViewAnglesForRotation(rotation,zoom,&yaw,&pitch,&groundPitch) → void | §§2.1–2.2. |
| `007F77D0` | 126 | ProjectPointToViewCoordinates(&p) → void | §4. |
| `007F7850` | 75 | ProjectPointToViewportCoordinates(&p) → void | §4. |
| `007F78A0` | 64 | ProjectPointToViewportZ(p) → x87 scalar | §4. |
| `007F78E0` | 54 | ProjectPointToViewportCoordinates2D(&p) → void | §4. |
| `007F7920` | 133 | UnProjectPointFromViewCoordinatesToWorldCoordinates(&p) → void | §4. |
| `007F79B0` | 77 | UnProjectPointFromViewportCoordinatesToWorldCoordinates(&p) → void | §4. |
| `007F7A40` | 1018 | ProjectBoundingBoxToViewportRectangle(min,max,&TL,&BR) → void | §§4,4.4. |
| `007F7E50` | 270 | IsPointInViewFrustum(p) → bool | §3.4. |
| `007F7F60` | 527 | GetFrustumAABBCityClipped(useView,&min,&max) → void | §§3.3,7.2. |
| `007F8300` | 56 | GetCityWidthFromOrthoView(width,zoom) → x87 scalar | §§2.1–2.3. |
| `007F8390` | 83 | ProjectPointFromViewportCoordinatesToFixedViewCoordinates(&p) → void | §4. |
| `007F83F0` | 3204 | UpdateCameraPosition(cameraControl) → void | §2; no valid-input bool result. |
| `007F9080` | 216 | ViewportPositionToRay(x,y,&origin,&direction) → bool | §4.1. |
| `007F9160` | 88 | GetViewportPointIntersectionWithGroundPlane(x,y,&out) → bool | §4.1; unchecked temporary failure. |
| `007F91C0` | 2133 | MeshPick(&pick,mesh,transform) → bool | §§7,7.2. |
| `007F9A40` | 246 | ModelPick(&pick,instance,model) → bool | §§7,7.2. |
| `007F9B40` | 852 | GetGroundPlaneVisibleAreaRectangle(&innerMin,&innerMax,&outerMin,&outerMax) → void | §6; optional outputs. |
| `007F9EA0` | 169 | GetViewportRectGroundCorners(&TL,&TR,&BL,&BR,rectTL,rectBR) → void | §4.1; four P2 ground outputs. |
| `007F9F50` | 3888 | BuildDrawFrustumDataFromViewportRect(TL,BR) → void | §5. |
| `0080AE80` | 67 | *primitive triangle count*(type,n) → int32 | §7.1 count table, signed truncating division; no saturation (§7.2); reach. |
| `0080AEF0` | 235 | *primitive vertex indices*(type,i,&triple) → void | §7.1 indices, no strip winding flip; unknown type leaves outputs; neighbour. |
| `0080B000` | 31 | *iterator deleting destructor*(this,flags) → this | Set vtable to `0x00ABD92C`; GlobalHeapFree(this) iff flags bit0; no buffer frees; neighbour. |
| `0080B020` | 99 | *iterator vertex copy*(this,indices,&triangle) → void | Copy 12 bytes from data+index×stride for each vertex, no validation (§7.2); reach. |
| `0080B090` | 623 | *iterator index decode*(this,indices,&decoded) → void | Formats 0/1 signed/unsigned8, 2/3 signed/unsigned16, 4/5 raw32, 6 float32, 7 float64 narrowed to float32; numeric formats floor via bias plus exact-integer correction; unknown format leaves output. §7.1; neighbour. |
| `0080B320` | 137 | *iterator select group*(this,mesh,groupIndex) → void | Set primitive/count, vertex format/stride/data; fixed index format3; use first×stride vertex-base offset without indices, first×2 index-base offset with indices; set signed triangle count and current=0 (§7.2); neighbour. |
| `0080B3B0` | 106 | *iterator next*(this,&triangle) → bool | Signed current≥count returns false, preserving triangle; otherwise primitive triple, optional index decode, vertex copy, increment current, true; reach. |

The 50-body checklist does not make every nearby function a member. In particular `0x007F7A00`
copies cached view min/max to two required vector outputs (57 bytes); `0x007F8290` copies x/z of
draw ground corners to four required P2 outputs (87 bytes); `0x007F8340` is a cached-angle getter
(69 bytes, same angle rule using cached zoom/rotation). They operate on this object's layout but are
**outside the measured 50-entry attribution set**; retain these native getter contracts without adding
their bytes to that set. `0x007F8170` is a transform/AABB helper (273 bytes) and `0x007F82F0` computes
atan2(−first,−second) (15 bytes); neither is a missing camera setter. No shared-map edits are proposed.
The constructor at `0x0080AE60` writes only the iterator vtable (no other defaults) and is shared by
attribution, not a 51st family-owned body. Geometry triangle/plane/clipping dependencies in §7.2 likewise
retain their geometry owner rather than being double-counted here.

## 13. Callable native contract and shadow isolation (VU16)

This section describes the 32-bit Windows machine interface, not public OpenSC4 C ABI exports or
recovered C++ source declarations. All 50 entries in §12 are covered below. A row's convention,
ordered argument types and result together are its callable declaration. Every explicit argument
occupies **one four-byte stack slot**, including float32 and the boolean selector. Argument one is
at entry stack pointer +4, argument two at +8, and so on; arguments are placed right to left.
Pointers and integers are 32 bits. There are no by-value vectors, doubles, hidden structure-return
pointers, variadic entries, or register float arguments in this family.

**C** means cdecl: no receiver, caller removes the listed argument bytes. **T** means thiscall:
receiver in ECX, not in the argument list or byte count, and callee removes the listed bytes.
**S** means a receiver-independent callee-pop entry, callable as stdcall with the listed arguments:
incoming ECX is unused. This is an operational declaration, not a claim about whether an original
C++ declaration was static. In all cases EAX, ECX, EDX and condition flags are volatile; preserve
EBX, ESI, EDI, EBP and the stack balance. T with zero argument bytes still receives ECX.

Type vocabulary: **i** signed int32, **u** uint32, **f** float32, **V** pointer to three float32s,
**P** pointer to two float32s, **Q** pointer to two int32 pixel coordinates, **I** pointer to one
int32, **F** pointer to one float32, **J** pointer to three 32-bit indices, **Tri** pointer to three
consecutive V-sized records (36 bytes), **Pick** pointer to the §7 pick record (at least 0x2C bytes),
**X** pointer to the §7 transform (0x38 bytes). **Mesh**, **Instance**, **Model**, **Control** are
pointers to the native resources/providers described in §§2 and 7. Types denote memory layout;
read-only inputs and writable outputs retain the roles in §12. A Q is not a P.

Result **B** means boolean in AL (zero or one); upper EAX bytes are not a result contract.
**R** means one x87 ST(0) result, with capture rules in §13.2. **A** means a pointer in EAX,
**i** an int32 in EAX, and **void** no result to inspect. Do not treat leftover EAX or ST(0) from a
void entry as a return value. Required pointers remain required even for functions that may leave
their outputs unwritten on some inputs.

### 13.1 Complete call and storage table

Storage class **L** uses local mutable arguments and has no receiver/provider dependency; **R**
reads the owner but does not change it; **P** invokes borrowed providers; **W** writes the owner
and may invoke providers; **I** requires a private iterator. These classes constrain shadow calls,
not the game's normal calls. Arguments below use the names/order in §12.

| VA | Convention / argument bytes | Ordered stack argument types | Result | Storage |
|---|---|---|---|---|
| `005D6900` | C / 16 | V points, i count, V min, V max | void | L |
| `005D6F10` | C / 12 | V a, V b, V out | void | L |
| `007F6550` | C / 16 | f a, f b, f c, f d | R | L |
| `007F65B0` | C / 16 | f a, f b, f c, f d | R | L |
| `007F6620` | T / 4 | P point | void | R |
| `007F6690` | T / 4 | f v | R | R |
| `007F66A0` | T / 8 | I zoom, I rotation | void | R |
| `007F66C0` | T / 4 | V p | void | R |
| `007F6710` | T / 4 | V p | void | R |
| `007F6770` | T / 8 | V min, V max | B | R |
| `007F6910` | T / 8 | V min, V max | B | R |
| `007F6AB0` | C / 12 | V origin, V direction, V out | void | L |
| `007F6AF0` | C / 16 | Pick pick, V a, V b, V c | B | L |
| `007F6B50` | S / 12 | V origin, V direction, V out | B | L |
| `007F6BA0` | T / 8 | i corner, V out | void | R |
| `007F6CB0` | C / 4 | i rotation | R | L |
| `007F6CC0` | C / 8 | V v, i rotation | void | L |
| `007F6D10` | C / 8 | V out, i edge | void | L |
| `007F6D70` | C / 8 | f v, i block | R | L |
| `007F6DB0` | C / 8 | f v, i block | R | L |
| `007F6DF0` | T / 0 | none (ECX is owner storage) | A | W |
| `007F7710` | T / 4 | V p | void | R |
| `007F7750` | T / 8 | I a, I b | void | R |
| `007F7780` | S / 20 | i rotation, i zoom, F yaw, F pitch, F groundPitch | void | L |
| `007F77D0` | T / 4 | V p | void | R |
| `007F7850` | T / 4 | V p | void | R |
| `007F78A0` | T / 4 | V p | R | R |
| `007F78E0` | T / 4 | V p | void | R |
| `007F7920` | T / 4 | V p | void | R |
| `007F79B0` | T / 4 | V p | void | R |
| `007F7A40` | T / 16 | V min, V max, Q TL, Q BR | void | R |
| `007F7E50` | T / 4 | V p | B | R |
| `007F7F60` | T / 12 | u useView (low byte), V min, V max | void | R |
| `007F8300` | C / 8 | f width, i zoom | R | L |
| `007F8390` | T / 4 | V p | void | R |
| `007F83F0` | T / 4 | Control cameraControl | void | W |
| `007F9080` | T / 16 | i x, i y, V origin, V direction | B | P |
| `007F9160` | T / 12 | i x, i y, V out | B | P |
| `007F91C0` | C / 12 | Pick pick, Mesh mesh, X transform | B | L |
| `007F9A40` | C / 12 | Pick pick, Instance instance, Model model | B | P |
| `007F9B40` | T / 16 | V innerMin, V innerMax, V outerMin, V outerMax | void | W |
| `007F9EA0` | T / 24 | P TL, P TR, P BL, P BR, Q rectTL, Q rectBR | void | P |
| `007F9F50` | T / 8 | Q TL, Q BR | void | W |
| `0080AE80` | C / 8 | i type, i n | i | L |
| `0080AEF0` | C / 12 | i type, i triangleIndex, J triple | void | L |
| `0080B000` | T / 4 | u flags (bit 0) | A | I |
| `0080B020` | T / 8 | J indices, Tri triangle | void | I |
| `0080B090` | T / 8 | J indices, J decoded | void | I |
| `0080B320` | T / 8 | Mesh mesh, i groupIndex | void | I |
| `0080B3B0` | T / 4 | Tri triangle | B | I |

Evidence is the entry argument loads, receiver uses, exit cleanup and tail transfers in the
2026-10-22 instruction export, not its often-unknown prototype column. In particular min loads
four 32-bit floats at `0x007F6556/6D/80/93` and ends in caller-pop return at `0x007F65A6`;
max has the corresponding loads at `0x007F65B6/CD/E0/F3`, return `0x007F6606`.
The two S entries overwrite/use ECX as a temporary rather than reading a receiver and remove
12 bytes at `0x007F6B8F` and 20 at `0x007F77C4`. The angle outputs are 32-bit stores at
`0x007F779B`, `0x007F77AE`, `0x007F77C2`. The composite projections return with four-byte
cleanup at `0x007F7898` and `0x007F7913`. Other composites tail-call T targets:
`0x007F79F8` to `0x007F7920`, and `0x007F83DE` to `0x007F6710`; a tail transfer does
not imply caller cleanup. Ground-corner output pointers precede the two Q inputs
(`0x007F9EA6`–`0x007F9EED`); cleanup is 24 bytes at `0x007F9F46`.
The constructor returns its storage in EAX (`0x007F6DF0`–`0x007F7705`); the iterator
deleting destructor returns its receiver there (`0x0080B019`–`0x0080B01C`).

### 13.2 Scalar returns and the block input width (VU15/VU16)

An x87 return is **not an SSE float32 register or an eight-byte stack return**. All eight scalar
entries leave one value in ST(0), whose register format is extended precision; they do not perform
a final float32/float64 return store. A native bridge must consume that one value exactly once.
For a float32 comparison, capture it with a float32 store; for a float64 comparison, capture with
a float64 store and state that choice. Either capture width is callable on x86; the callee does
not distinguish a C compiler's float-return declaration from a double-return declaration at this
boundary. Thus a float32-return cdecl adapter with four float32 parameters is sufficient for min/max;
a double-parameter declaration is wrong. A float64-return adapter still has **four float32 inputs**.
For min/max, the finite selected value was already float32, so widening it to float64 is exact.
No return-width choice resolves inherited x87 control-word or intermediate-rounding questions in VU12.

The native visible-area caller stores min/max returns to float32 (`0x007F9C32/3A`,
`0x007F9C5B/60`, `0x007F9C6E/76`, `0x007F9C95`). Object-angle and viewport-depth consumers
also store float32 (`0x0048FA90`, `0x007C2FAC`). Block-rounding consumers do so at
`0x007C428D/9C/AB/C0`. These establish an explicit float32 observation boundary for those
uses. Some other consumers continue x87 arithmetic without an immediate store, for example
ViewportUnitsToWorldUnits at `0x005F0FA8` and city width at `0x007CBE69`–`0x007CBE75`;
inserting a float32 capture there is an additional rounding boundary. A shadow must name whether
it compares a stored float32 observation or a wider captured result.

Both block-rounding inputs are **float32 v followed by int32 block**, not doubles or caller-retained
x87 inputs: `0x007F6D70` and `0x007F6DB0` load the four-byte argument at entry stack +4;
the block is at +8. Their internal truncated low word, wrapped arithmetic and signed division
remain as §4.4 describes; the final signed word is loaded exactly into ST(0) at `0x007F6D9C`
and `0x007F6DDC`. Consequently test 18's INT_MAX case is an **integer-stage** vector.
The representable float32 entry value 2147483648 truncates to a signed64 value whose low word
is INT_MIN, and Up with block 64 returns −2147483648. The largest float32 below that input,
2147483520, is already a multiple of 64 and returns itself. A float64 capture preserves every
possible final int32 exactly; a float32 capture can round a final integer. This resolves VU15
without asserting a source-language floating return type.

### 13.3 Live reads, isolated writes and borrowed providers

For an **L** entry, use locally owned mutable vectors/records and immutable input arrays; no live
view owner is needed. MeshPick reads the mesh, its vertex/index/group buffers and the transform,
but writes the supplied pick's nearest distance, not the mesh or a view owner. It may borrow stable,
validated live mesh buffers synchronously, or use a complete local mesh fixture. Bounds, indices,
stride and group count must describe accessible memory; the native unchecked reads in §7.2 still
apply. TrianglePick similarly requires a private pick even when vertices come from live storage.
Scalar helpers read image constants, and faulting block divisors remain unsuitable for a normal
same-process observer. Output/input aliasing is permitted only where the behavior is deliberately
being tested (notably the cross helper's sequential stores); otherwise use disjoint buffers.

For an **R** entry, a stable live 0x470-byte owner is sufficient on the thread that owns its view
state, with all writable arguments in local storage. These calls do not invoke the renderer,
camera, model or instance. They include city clipping, cached getters, projection/unprojection,
frustum predicates and the city-clipped box calculation. None initializes lazy owner caches.
The observer must obtain the object through the renderer's actual view-utilities pointer (§1),
not reinterpret the renderer or the camera as that object. Do not race camera updates or destroy
the borrowed object/buffers while a call is in progress. Copying the owner is also sufficient for
these calls; their field reads do not require the borrowed renderer pointer to be dereferenced.

**P** bodies do not store owner fields, but they are not unconditional read-only calls on the whole
live object graph. ViewportPositionToRay asks renderer slot 14 for a camera (`0x007F9092`), then
calls the native camera eye-ray entry (`0x007F90DD`). The viewport ground wrapper and four-corner
wrapper inherit this dependency. ModelPick asks model slots 3/5 and instance slot 9 for count,
mesh and transform (`0x007F9A56`, the following mesh loop and transform fill in §7). Calling a
getter-shaped virtual method is not evidence that an arbitrary provider has no side effects.
For a guaranteed isolated comparison use private provider objects/backing resources satisfying
those native layouts and slots, or separately audit the particular live provider's writes and
thread/lifetime rules. A byte copy of the view owner alone retains its live renderer pointer and
therefore does not isolate those calls. The concrete camera call also requires the native camera
layout; a fake vtable alone cannot replace that direct call. Missing/failing providers retain the
partial-output and indeterminate wrapper behaviors in §§4.1 and 5; a shadow must not compare
indeterminate stack coordinates as fixed reference values.

**W** requires writable private 0x470-byte owner storage. The constructor has no provider calls;
its result is a local initialized owner, not a request to initialize a live renderer's owner twice.
UpdateCameraPosition overwrites camera/frustum state and invalidates caches. BuildDrawFrustum
overwrites the draw rectangle/geometry and cache flags. GetGroundPlaneVisibleAreaRectangle is
also a writer even though it is named a getter: it lazily fills `+0x414`–`+0x440` and the ground
cache/validity state in §6 (`0x007F9B46`, `0x007F9C3A/60/76/8A`). A warm cache can avoid
some writes/provider calls but is not a general live-read contract. Use private owner storage and
the P-class provider discipline for these three operations. Preserve initial cached state when
testing failed updates; reconstructing a zero owner changes the stale-state behavior.

**I** uses a separate 0x2C-byte native iterator record: vtable +0; primitive +4; group count +8;
vertex format/stride/data +0x0C/+0x10/+0x14; index format/data +0x18/+0x1C; triangle count
+0x20; reserved +0x24; current triangle +0x28. Select-group overwrites its fields and next advances
current; vertex-copy and index-decode read its fields and write supplied outputs. Initialize every
field needed for a direct call, rather than relying on the constructor (which only sets the vtable,
§12). Deleting-destructor with flags bit0 clear only restores the vtable and returns the receiver;
bit0 set calls GlobalHeapFree (`0x0080B000`–`0x0080B01C`). Use bit0 clear for stack/local records.
Bit0 set requires a disposable allocation from that native heap, never live iterator storage or a
foreign allocator. There is no buffer free or ownership transfer for the referenced vertex/index
arrays. None of these safety rules confers new numerical equivalence or whole-owner shadow credit.

### 13.4 Supported executable binding

The examined binding is GOG Deluxe 1.1.641, PE32/i386 (machine `0x014C`, optional-header magic
`0x010B`), preferred image base `0x00400000`, SizeOfImage `0x007C8000`, timestamp `0x4C12BFB7`.
The examined on-disk `bin/SimCity4.exe` SHA-256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
These are executable identity facts, not an instruction to read a file from a particular install
path at run time. A differently patched executable is not covered merely because its version
string is the same.

All VAs in this spec are preferred-image VAs: subtract `0x00400000` to obtain an RVA and add
the executable module's load base to locate the entry in process. Examples: min RVA `0x003F6550`,
max `0x003F65B0`, rotation `0x003F6CC0`, iterator-next `0x0040B3B0`. These private entries are
not exported symbols or COM slots. Bind to the executable module, not the observer DLL, and
validate the image identity and executable section bounds before calling. The measured image is
supported at its preferred base; executing a rebased/modified copy has not been established here.
Fail closed for such an image unless its relocation correctness and operand targets are independently
verified. The RVA formula is address arithmetic, not proof that a copied code buffer is callable:
these bodies use absolute image constants, jump tables, globals and direct calls. Do not detach or
copy them into an observer's memory. No native signature, live-write allowance or compatibility
with a different game build should be inferred from an attribution name alone.
