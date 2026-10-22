# Spec: camera and projection (`cSC4CameraControl`, `cS3DCamera`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [INVENTORY-M6.md](INVENTORY-M6.md) (§3.3, the camera, zoom and rotation overview; §9 rank 3 proposes
this spec), [graphics-formats.md](graphics-formats.md) (§4.2, the model keys per zoom and rotation),
[zones.md](zones.md) (§5.2, a ViewZRChange3 subscriber), [foundations.md](foundations.md) (§5.1, messages; §18, the
x87 precision of the float maths), [save-records.md](save-records.md) (§3.2, the camera control's class record).

## 0. Summary

The city view looks at the ground with an **orthographic** camera. `cSC4CameraControl` (class `0xC9C628EC`) owns the
view state: a view target on the ground, a zoom (0–4, with a fractional magnification above 4), a rotation (0–3)
and the city size; from them it sets the projection box and the eye of a `cS3DCamera` (class `0xE9C6262A`), keeps
the target inside scroll bounds, converts between world points and a 2D "camera plane", and announces every change
with view messages. `cS3DCamera` is a general camera: viewport, subview, an orthographic or perspective projection
matrix with a sub-pixel offset, a look-at view transform, and `Project` / `UnProject` / `GetEyeRay`.

World coordinates: x and z on the ground (metres, a cell is 16 m), y up. Screen coordinates: pixels, y down.

## 1. `cS3DCamera` *[compat]*

### 1.1 State (0x1A8 bytes; vtable `0x00ABD2C4`)

| Offset | Field | Default |
|---|---|---|
| `+0x04` | reference count | 0 |
| `+0x08`, `+0x0C` | viewport width, height (float pixels) | 100, 100 |
| `+0x10` | depth size | 65536 |
| `+0x14` | subview in use (bool) | false |
| `+0x18`, `+0x1C`, `+0x20`, `+0x24` | subview x, y, width, height | 0, 0, 100, 100 |
| `+0x28` | the **local transform** (camera to world): a change counter (byte), flags `+0x29` (1 translated, 2 scaled, 4 rotated), a 3×3 rotation `+0x2C`…`+0x4C` whose columns are the camera's right, up and back axes, the position `+0x50`…`+0x58`, a uniform scale `+0x5C` | identity |
| `+0x60` | the **view transform** (world to camera), same layout, rebuilt from the local one when its counter differs (§1.3) | identity |
| `+0x9C`…`+0xD8` | the projection matrix (4×4, row-major, column vectors: `p' = M p`) | |
| `+0x11C`…`+0x158` | its inverse | |
| `+0x160` / `+0x170` | the last look-at target and up vector | both (0, 0, 0) |
| `+0x17C` | u32 projection change counter (the matrix is rebuilt when it differs from u32 `+0x15C`) | 1; cached counter 0 |
| `+0x180` | projection type: 2 perspective, anything else orthographic (`SetOrtho` stores 1) | 0 |
| `+0x184`, `+0x188` | near, far | 1, 2 |
| `+0x18C`, `+0x190`, `+0x198`, `+0x194` | left, right, bottom, top | −1, +1, −1, +1 |
| `+0x19C`, `+0x1A0` | pixel offset x, y | 0, 0 |
| `+0x1A4` | depth bias, set by `SetDepthBias` (§1.2) | 0 |

The constructor's box has side length **2**, not 1. The remembered up is zero, not Y-up:
`0x007FFCB4–0x007FFCF3` copies the zero word at `0x00B4E660` into both remembered vectors;
`0x007FFD10–0x007FFD4D` stores the box, near/far and counters. The local/view transforms start as
identity with byte counters zero. These are constructor state facts, independent of later look-at calls.

### 1.2 Methods

| Method | Address | Behaviour |
|---|---|---|
| `GetGZCLSID()` | `0x007FF010` | `0xE9C6262A` |
| `QueryInterface(iid, out)` (slot 0) | `0x007FF040` | IIDs 1 and `0xE9C6262A` |
| `Release` (slot 2) | `0x007FF020` | count ≤ 1: slot 3 with 1 and return 0; else decrement |
| deleting destructor (slot 3) | `0x007FF130` | frees to the global heap on flag bit 0 |
| `SetDepthBias(b)` | `0x007FF0F0` | `+0x1A4 := b` and the projection change counter `+0x17C` is bumped; the 3D draw context's `SetDepthOffset(n)` passes `kDepthOffset` × n (draw-context.md §8) |
| constructor / factory | `0x007FF930` / `0x00800320` | the defaults of §1.1 (0x1A8 bytes from the global heap) |
| `SetViewport(w, h)` | `0x007FF150` | viewport := (w, h); subview := (0, 0, w, h), not in use; projection counter += 2 |
| `SetSubview(x, y, w, h)` | `0x007FF080` | subview := (x, y, w, h), in use; projection counter += 1 |
| reset subview (descriptive) | `0x007FF0B0` | subview := (0, 0, viewportWidth, viewportHeight), not in use; projection counter += 1; export alias `cSC43DRender::UnsetClipRect` |
| `SetDepthSize(d)` | `0x007FF070` | `+0x10 := d`; projection counter += 1 |
| `SetOrtho(l, r, b, t, n, f)` | `0x007FF2E0` | the box; type := 1; projection counter += 1 |
| `SetPixelOffset(dx, dy)` | `0x007FF2C0` | `+0x19C`, `+0x1A0`; projection counter += 1 |
| `GetViewVolume(&l, &r, &t, &b, &n, &f)` | `0x007FF340` | the box shifted by the pixel offset: `l + δx`, `r + δx`, `t − δy`, `b − δy`, n, f, with `δx = (r − l) / viewportWidth × dx` and `δy = (t − b) / viewportHeight × dy` |
| `SetPosition(p)` | `0x007FF250` | local position := p, flag 1, counter += 1 |
| `GetPosition(&p)` | `0x007FF230` | the local position |
| `SetLookAt(target, up)` | `0x007FFD60` | §1.3; flag 4, counter += 2; remembers target and up |
| `GetLookAt(&p)` | `0x007FF280` | the remembered target |
| `ResetXforms()` | `0x007FF190` | each transform that has flags becomes the identity (flags 0, counter + 1) |
| `LocalXform()` / `ViewXform()` | `0x007FF0E0` / `0x007FFED0` | the transforms; `ViewXform` rebuilds the view transform first if needed |
| `UpdateViewXform()` | `0x007FF3D0` | §1.3 |
| `ProjectionMatrix()` / `_UpdateProjection()` | `0x007FFEF0` / `0x007FF580` | §1.4 |
| `GetProjectionType()` / `GetViewState()` | `0x007FF110` / `0x007FF3C0` | `+0x180` / the local counter |
| projection counter getter (descriptive) | `0x007FF120` | u32 `+0x17C`; export alias `cSC4City::GetFloraSimulator` |
| `Project(world, &screen)` | `0x007FFF10` | §1.5 |
| `UnProject(screen, &world)` | `0x00800120` | §1.5 |
| `GetEyeRay(screenXY, &origin, &direction)` | `0x00800340` | §1.6 |

### 1.3 Look-at and the view transform

`SetLookAt(target, up)`: `back := normalize(position − target)`, `right := normalize(up × back)`,
`trueUp := back × right`; the local rotation's columns are (right, trueUp, back). So the camera looks along
`−back`, right-handed.

There is no zero-length or parallel-vector fallback. Each normalization computes a square root and its
reciprocal, then multiplies the components (`0x007FFD85–0x007FFDAF`, `0x007FFDE5–0x007FFE09`). With the
normal masked x87 exceptions, a coincident position/target or a parallel up/back produces NaNs in the
rotation; the method still remembers the arguments and performs its counter/flag updates. NaN payloads
and signs are not a portable contract.

The view transform is the inverse of the local one: rotation transposed (when the local one is rotated), translation
`−Rᵀ · position`, and, when the local one is scaled, scale `1 / s` with the translation multiplied by it. A world
point `p` in camera space is `v = (R_view · p) × s_view + t_view`.

### 1.4 The projection matrix

With `sx = 1 / (r − l)`, `sy = 1 / (t − b)`, `sz = 1 / (f − n)`, `px = 1 / subviewWidth`,
`py = 1 / subviewHeight`, the offsets `ox = −2 · px · dx` and `oy = −2 · py · dy` (the pixel offset as clip-space
units), and the bias `β = −(+0x1A4 / depthSize)`; if the subview is in use, first `sx := sx · (px · viewportWidth)`,
`ox := (px · viewportWidth − 1) − 2 · px · subviewX + ox`, and the same for y.

- **Orthographic** (type ≠ 2): `m00 = 2 sx`, `m03 = ox − (l + r) sx`; `m11 = 2 sy`, `m13 = oy − (b + t) sy`;
  `m22 = −2 sz`, `m23 = β − (n + f) sz`; `m33 = 1`; the rest 0. This is `glOrtho` plus the pixel offset.
- **Perspective** (type 2): `m00 = 2n sx`, `m02 = (l + r) sx − ox`; `m11 = 2n sy`, `m12 = (b + t) sy − oy`;
  `m22 = −(n + f) sz − β`, `m23 = −2n f sz`; `m32 = −1`, `m33 = 0` (`glFrustum`).

The inverse is stored alongside (diagonal reciprocals and the matching offsets).

Projection invalidation is a wrapping **u32** counter, separate from the wrapping **byte** transform
counters. SetOrtho writes all six arguments verbatim, sets type 1 and increments once even when the box
is unchanged (`0x007FF2E0–0x007FF334`); it performs no range/degeneracy checks. SetViewport increments
twice (viewport and reset-subview); the other projection setters increment once. ProjectionMatrix compares
the cached and current u32 counters and calls the builder only when unequal (`0x007FFEF0–0x007FFF03`).
The type-2 builder branch proves perspective **state evaluation**; it does not prove a public perspective
box setter, its signature or its invalidation count. No such setter has been identified in the native owner
inventory (§7). An implementation-only `set_frustum` is an extension until CA3's remaining entry-point
question is resolved; mirroring SetOrtho is not a measured native contract.

### 1.5 `Project` and `UnProject`

`Project(p)`: `v` as in §1.3, `c = M · (v, 1)`; if `|c.w| < 0.0001`, false. Otherwise `ndc = c / c.w` and
`screen.x = (ndc.x + 1) · subviewWidth / 2 + subviewX`, `screen.y = (1 − ndc.y) · subviewHeight / 2 + subviewY`,
`screen.z = ndc.z`; true.

`UnProject(s)`: `ndc.x = 2 (s.x − subviewX) / subviewWidth − 1`, `ndc.y = 2 (subviewY + subviewHeight − s.y) /
subviewHeight − 1`, `ndc.z = s.z`; `c = M⁻¹ · (ndc, 1)`; false when `|c.w| < 0.0001`; else `v = c / c.w`, then the
inverse view transform: subtract the view translation, divide by the view scale, multiply by the transposed view
rotation.

### 1.6 `GetEyeRay(s)`

- Orthographic: `d := target − position` (the look-at target); `q := UnProject(s.x, s.y, 0)`; if `|d|² > 0`, `d` is
  scaled to the length `far`; `origin := q − d`, `direction := d`. The ray runs from one far-distance behind the
  picked point through it.
- Perspective: from the whole viewport (not the subview), `(nx, ny) = (2 s.x / w − 1, 2 (h − s.y) / h − 1)`;
  `origin :=` the camera position; `direction := −R · (nx, ny, −near)` with the local rotation `R`.

## 2. `cSC4CameraControl` *[compat]*

### 2.1 State (0x160 bytes; vtable `0x00ABAD18`)

| Offset | Field | Default |
|---|---|---|
| `+0x04` | initialised | false |
| `+0x05` | loaded from a save | false |
| `+0x08` | reference count | 0 |
| `+0x0C` | the attached `cS3DCamera` (counted) | null |
| `+0x10`…`+0x18` | the camera-plane origin (rotation corner + eye offset) | (0, 0, 0) |
| `+0x1C`…`+0x24` | the rotation corner (§2.3) | (0, 0, 0) |
| `+0x38`…`+0x40` | the eye position | (0, 0, 0) |
| `+0x44`…`+0x4C` | the **view target** | (0, 0, 0) |
| `+0x50`…`+0x58` | the view-target velocity | 0 |
| `+0x5C`…`+0x7C` | view direction; up axis at `+0x68`; right axis at `+0x74` | zero; (0, 1, 0); zero |
| `+0xB0`, `+0xB4` | the position tweak (pixels, §2.3) | 0, 0 |
| `+0xB8`…`+0xC8` | scroll step per zoom: 100, 50, 25, 12.5, 6.25 (m) | |
| `+0xF0` | custom magnification | 1 |
| `+0xF4`, `+0xF8`, `+0xFC` | minimum zoom 0, maximum zoom 4, magnification limit 5 | |
| `+0x100`, `+0x104` | previous zoom and rotation | 0, 0 |
| `+0x108`, `+0x10C` | **zoom** (0–4) and **rotation** (0–3) | 0, 0 |
| `+0x110`, `+0x114` | new zoom and rotation (as sent in the messages) | 0, 0 |
| `+0x118` | yaw base | float bits `0xBEC90FDB`, approximately −π/8 |
| `+0x11C` | the current pitch (π/6 at construction) | |
| `+0x120` | the eye's horizontal distance, 8192 m | |
| `+0x124`, `+0x128` | near and far distances | 0, 0 |
| `+0x12C`, `+0x130` | viewport size (int pixels) | 800, 600 |
| `+0x134`, `+0x138` | metres per pixel and pixels per metre | 1, 1 |
| `+0x13C`…`+0x144` | the eye offset from the target | (0, 0, 0) |
| `+0x148` | scroll bounds in use | true |
| `+0x14C`…`+0x154` | the scroll-bounds polygon (a vector of 2D points) | empty |
| `+0x158`, `+0x15C` | city width and depth (m) | 4096, 4096 |

Pre-init values are literal stores, not a valid computed camera basis: constructor
`0x007CC9A6–0x007CCA68` zeros the vectors except `+0x6C = 1`, and
`0x007CCAF8–0x007CCB65` zeros clip distances and eye offset. In particular, there are **not two identity
camera-plane axes**. The clip getter simply copies the two stored zero values (`0x007CB9F0–0x007CBA08`).

### 2.2 Zoom and rotation → projection (`UpdateCameraZoomAndRotationParams`, `0x007CBE40`)

Tables (by zoom 0–4): **cell width in pixels** 8, 16, 32, 73, 146 (`0x00ABACE0`); **pitch** 30°, 35°, 40°, 45°, 45°
(`0x00ABACCC`).

1. `pixels := cellPixels[zoom] × magnification`; `cellSpan := 16 · (sin 22.5° + cos 22.5°) ≈ 20.90501`
   (`SC4ViewUtilities::GetCityWidthFromOrthoView(16, zoom)`, angle table `0x00ABCFC4`, −22.5° at every zoom):
   metres per pixel `+0x134 := cellSpan / pixels`, pixels per metre `+0x138 :=` its reciprocal. At magnification 1:
   2.613126, 1.306563, 0.653281, 0.286370, 0.143185 m per pixel.
2. `yaw := (rotation + 1) · float(π/2) + yawBase`; `pitch := pitch[zoom]` (stored at `+0x11C`).
   The constructor yaw base is approximately −π/8, so default yaws are 3π/8, 7π/8, 11π/8, 15π/8.
   The order and single-precision operands are explicit at `0x007CBE98–0x007CBEB0`; the multiplier at
   `0x00ABAD04` is `0x3FC90FDB`. The former −π/4 formula was incorrect.
3. `d := 8192 / cos(pitch)`; with `W, H :=` the city size, each at least 4096: near `:= (d − √(W² + H²) / cos(pitch))
   × 0.95`, far `:= d × 1.5`.
4. `SetOrtho(−a, a, −c, c, near, far)` with `a = viewportWidth × metresPerPixel × 0.5` and
   `c = viewportHeight × metresPerPixel × 0.5`.
5. Eye offset `:= (cos(yaw) · 8192, tan(pitch) · 8192, sin(yaw) · 8192)` (the y term is `sin(pitch) · d`).

### 2.3 Eye placement and pixel snapping (`UpdateCameraPositionParams`, `0x007CC290`)

1. **Rotation corner** (`0x007CBA10`): for rotation 0, 1, 2, 3 the city corner (0, 0, 0), (W, 0, 0), (W, 0, H),
   (0, 0, H). An index outside 0…3 writes only the output y=0, leaving its x/z words unchanged
   (`0x007CBA14–0x007CBA22`, `0x007CBA70`); there is no zero-corner fallback.
   The camera-plane origin is that corner plus the eye offset `o`.
2. `k := 1 − ((target − corner) · o) / |o|²` (0 when `|o|² = 0`); **eye := target + k · o**. Every eye position lies on
   the plane through `corner + o` perpendicular to `o`, so the depth range is the same everywhere.
3. `camera.SetPosition(eye)`, `camera.SetLookAt(target, (0, 1, 0))`, pixel offset (0, 0).
4. **Snapping**: project the rotation corner, round its screen x and y to the nearest integer (the 1.5·2²³ addition),
   and set the pixel offset to `(x − round(x) + tweakX, −(y − round(y)) + tweakY)`. The city is therefore drawn on
   the same pixel grid at every scroll position *[compat]*.
5. Copy the view direction and camera-plane axes from the view transform, build the camera plane's equation through
   the eye, and keep the target's camera-plane point.

### 2.4 Camera-plane conversions

- `ConvertWorldPointToCameraPlanePoint(p)` (`0x007CBCE0`): `(p − planeOrigin)` dotted with the view transform's first
  and second rows, times the view scale: a 2D point in metres.
- `ConvertCameraPlanePointToWorldPoint(q)` (`0x007CBD70`): the inverse, a point on the plane through the
  plane origin.
- `IntersectWithGroundPlane(direction, origin)` (`0x007CBA90`): false when `|direction.y| < 0.0001`; else
  `t := −origin.y / direction.y`; false when `t < 0`; else the point `origin + t · direction` (on y = 0).

### 2.5 Scroll bounds

`UpdateScrollBounds` (`0x007CCC60`, only when bounds are in use): the four city corners (`SC4ViewUtilities::
GetCityCornerPosition`, corners 1, 0, 2, 3) in camera-plane coordinates `A, B, C, D`, and
`lift := (o.y × 0.48) / viewRow1.y − metresPerPixel × viewportHeight × 0.5`. The polygon is six points:
`(A.x, A.y + lift)`, `(B.x − 500, B.y + lift)`, `(B.x − 500, B.y)`, `(D.x, D.y)`, `(C.x + 500, C.y)`,
`(C.x + 500, C.y + lift)`: the city's outline widened by 500 m left and right and raised by `lift`
*[inferred: the screen-space meaning of `lift`]*.

`ClampScrollToBounds(from, to, &out)` (`0x007CC650`), for finite points and a valid bounds vector:

1. Bounds off or an empty vector copies `to`. Otherwise retain the original arguments, set working start S
   to `from` and endpoint T to `to`, and clear two direction-history flags. Visit edges in vector order,
   starting with (last point, point 0). There is **no signed-area winding calculation**.
2. For previous vertex A and current vertex B, let v=A−B and length L=√(v.x²+v.y²).
   Skip L less than the **double** constant 1e−8 (`0x00AB1F88`); this tests length, not length squared
   (`0x007CC719–0x007CC744`). For other edges use n=(v.y/L, −v.x/L) and signed distance
   d(P)=n·P−n·A. Positive is the accepted side. This expects counterclockwise vertices in Cartesian x/y;
   reversing the winding reverses the accepted half-planes.
3. Consider an edge only when d(T)≤0 **and d(S)>d(T)**. Its fraction is d(S)/(d(S)−d(T)). Choose the strictly
   smallest fraction below the initial best 1 (`0x007CC775–0x007CC7B6`). Equal fractions preserve the first
   edge. A boundary endpoint reached at fraction 1 is not selected. Outside starts are not projected onto
   an edge: negative fractions may be selected; equal-distance/tangential moves select nothing.
4. No selected edge returns T. Otherwise, if the stored float fraction is at least float(0.001), subtract
   float(0.0001); below that threshold use **zero**, not a negative inset. These constants are bits
   `0x3A83126F` and `0x38D1B717` (`0x007CC7E3–0x007CC806`). The inset is a fraction, not a world distance.
   Form remaining vector R=(T−S)·(1−adjustedFraction), with component float stores, then Q=T−R.
5. Classify the dot product of the **original** (to−from) with edge direction (B−A): negative versus
   nonnegative (zero is nonnegative). Remember each category. If both have now occurred, return Q
   immediately (`0x007CC872–0x007CC8A9`, `0x007CC971`). This is an extra stop rule, not a winding test.
6. Otherwise normalize (B−A), project R onto it, set S=Q and T=Q+projectedR, and repeat. At most N such
   projections are allowed for N vertices. Exhausting N returns the **original from**, even if an intermediate
   point was closer (`0x007CC8AF–0x007CC95C`).

A zero-length outside move has d(S)=d(T) on every edge and returns the outside point unchanged. An outside
move further outward is limited to its tangential component at the original outside line; it is not brought
inside. These are native arithmetic/control-flow consequences, not claims from a runtime capture.

`ClampTargetToScrollBounds` (`0x007CD000`) converts the **rotation corner** and target to the camera plane
and clamps the segment **corner → target**, not target → itself (`0x007CD00C–0x007CD068`). If either output
component differs from the target-plane point by at least float(0.0001), convert the output to a temporary
world point and intersect the view direction with the ground plane. Success stores the hit and clears
velocity; failure keeps target **and velocity**. Either outcome still updates position and sends
CameraUpdated, ViewTChange2, ViewTChange3 when a camera is attached (`0x007CD0CD–0x007CD100`). Below the
component threshold it performs no update/message work.

### 2.6 Commands

| Method | Address | Behaviour |
|---|---|---|
| `Init(target)` | `0x007CDB20` | once: if loaded from a save, the new zoom/rotation := the loaded ones, else `target :=` the argument; `AttachToCamera`, §2.2, §2.3, scroll bounds, PostViewInit; initialised := true; true |
| `AttachToCamera(cam)` | `0x007CD200` | a different camera: keep it (counted), `SetViewport(800×600 or the size set)`; if initialised: §2.2, §2.3, bounds, PostViewInit |
| `DetachFromCamera()` | `0x007CB970` | release the camera |
| `SetZoom(z)` (float) | `0x007CDC50` | if `4 < z < 5`: magnification `:= 2^(z − 4)` (CRT `pow`) and `z := 4`; else magnification 1. Refused (false) for `z` outside 0…4 and when the integer zoom and the magnification are unchanged. Else `SetCustomMagnification`, previous := current, zoom := `trunc(z)`, ViewZRChange1, §2.2, §2.3, bounds, CameraUpdated, ViewZRChange2, ViewZRChange3, `ClampTargetToScrollBounds`; true |
| `SetZoomAndRotation(z, r)` (ints) | `0x007CDE60` | validates zoom/rotation, then calls SetCustomMagnification(1) **before** testing whether the integer pair is unchanged; equal pair then returns false; changed pair uses the zoom/rotation update sequence |
| `SetRotation(r)` | `0x007CD540` | invalid r or unchanged rotation returns true at once; changed rotation preserves zoom **and custom magnification**, updates pair fields, then uses the update/message sequence; false if it changed state with no attached camera |
| `SetCustomMagnification(m)` | `0x007CD6E0` | when `m` differs and the **current** magnification is in (0, 1000): store it, §2.2, §2.3, bounds, PostViewRefresh *[quirk: the range test is on the old value]* |
| `SetViewTarget(p)` | `0x007CD810` | every component finite and within ±100000: target := p, velocity 0; §2.3, CameraUpdated, ViewTChange2, ViewTChange3; true |
| `SetViewTargetVelocity(v)` | `0x007CBC60` | stores `v` when every component is a number (no NaN) |
| `ScrollByViewportAngle(angle, amount)` | `0x007CD330` | camera-plane step `(cos(angle), −sin(angle)) × amount × step[zoom]`, clamped (§2.5), converted back **directly into target storage**, then terrain ray cast along the view direction (terrain view slot 3), else the ground plane. A hit replaces target; if both fail the plane point remains. With a camera: §2.3 and CameraUpdated/ViewTChange2/ViewTChange3, true, even after failed intersections. Velocity is not cleared. Requires a camera for the conversions; its late false return is not a safe null-camera guard |
| `SetCameraPositionTweakAdd(x, y)` | `0x007CDA20` | replaces the two stored pixel offsets, not accumulation; on a change with camera: §2.2, §2.3; with view utilities also update its camera position and build bounds **only if empty**; no view messages |
| `SetCitySize(w, h)` | `0x007CDAA0` | `+0x158`, `+0x15C`; when initialised the bounds polygon is emptied and rebuilt after §2.2, §2.3 |
| `SetViewportSize(w, h)` / `GetViewportSize` | `0x007CB9B0` / `0x007CB990` | setter stores signed ints and, if attached, passes their float conversions to camera.SetViewport; no control projection-box/position/bounds recomputation or messages (`0x007CB9B0–0x007CB9E2`) |
| `GetVirtualZoom()` | `0x007CBC40` | zoom, plus 1 when the magnification exceeds 1.5 |
| `GetCustomMagnification()` / `GetDistancesToClipPlanes` | `0x007624E0` / `0x007CB9F0` | `+0xF0` / near and far |

Command ordering matters: integer SetZoomAndRotation with an unchanged pair can still reset magnification,
recompute and send PostViewRefresh before returning false (`0x007CDF12–0x007CDF2C`). A changed rotation-only
call never calls the magnification setter (`0x007CD55B–0x007CD5AE`). SetZoom(NaN) returns false before any
setter or state write: unordered x87 comparison sets the tested less-than bit (`0x007CDCB2–0x007CDCC1`).
SetCustomMagnification, however, checks the **old** value, so old=1 permits storing NaN; subsequent calls
are then blocked by the old-value range test (`0x007CD6E0–0x007CD72C`).

Init is idempotent even without a camera: it sets initialised true after its optional camera work; subsequent
calls return true without reattachment (`0x007CDB23–0x007CDB28`, `0x007CDC3B–0x007CDC4D`). DetachFromCamera
releases and nulls only the camera, retaining initialised, loaded and all numeric state (`0x007CB970–0x007CB987`).
The observed city-view Shutdown calls Detach then Release (`0x007F15B8–0x007F15D2`); the control destructor
frees the bounds vector/object and does **not** release the camera (`0x007CCC20–0x007CCC4F`). There is no
Shutdown slot in its seven-slot native vtable (§7), nor an identified separate control Shutdown body.
A pure-core operation that clears initialised is adapter policy, not this native teardown contract.
AttachToCamera acquires the new camera before releasing the old one, sets its viewport and, when initialised,
performs the updates/init message. A distinct null input reaches SetViewport with null this
(`0x007CD211–0x007CD243`); use Detach for native detachment.

Malformed saved indices are not repaired. Read preserves raw u32 words; the zoom update indexes the cell
and pitch arrays directly (`0x007CBE47–0x007CBE54`, `0x007CBEB4`), and scroll directly indexes its step array.
An invalid rotation leaves corner x/z unchanged (§2.3). There is no native bounded-table/zero-corner fallback.
The result of an out-of-array native read is not a deterministic valid-state contract. Containment in a safe
implementation must be labelled a safety policy. Read also accepts NaN target components without calling
SetViewTarget's validation; loaded Init bypasses its supplied target.

### 2.7 Messages (through the message server 2)

`SendPostViewChangeMessage(kind)` (`0x007CC0B0`) and the inline senders use: CameraUpdated `0x4A9F2FFE` (data1 =
the camera control); ViewZRChange1–3 `0xA6B79600`–`0xA6B79602` and ViewTChange1–3 `0xA6B79610`–`0xA6B79612`, PostViewInit
`0xA6B79620` and PostViewRefresh `0xA6B79621`, each with data1 = the previous (zoom, rotation) pair `+0x100` and
data2 = the new pair `+0x110`. A zoom or rotation change sends ViewZRChange1 (`0x007CC000`) before the update, then
CameraUpdated, ViewZRChange2 and ViewZRChange3 after it; a target move sends CameraUpdated, ViewTChange2 and
ViewTChange3.

Despite the word “Post” in the descriptive helper names, every send here calls message-server-2 **slot 3,
offset +0x0C**, synchronously. The native slot at `0x0092EA65` snapshots recipients and invokes their slot-3
callbacks before returning (`0x0092EAFB–0x0092EB10`); these are not queued posts. Camera call sites include
`0x007CC087`, `0x007CC13E`, `0x007CC1AA`, `0x007CC1CA` and `0x007CDC27`.
Payload pairs are **pointers to mutable fields**, not copied old/new values. The sender reuses one message
object, changes its type/data between synchronous sends, and releases it afterward. A receiver observes
the state at its callback; a deferred adapter snapshot is not native timing or pointer identity.
Before ViewZRChange1 the current and previous/new pair fields have already changed, while the main
projection/position recomputation follows that callback. Optional magnification refresh work happens earlier.

### 2.8 Save record (class `0xC9C628EC`; `Write` slot 3 `0x007CBB10`, `Read` slot 4 `0x007CBBA0`)

u16 version **1**; the view target (three floats); then six u32: previous zoom, previous rotation, zoom, rotation,
new zoom, new rotation. `Read` is **not transactional**. It sets loaded true before reading even the version
(`0x007CBBAC`); a version other than 1 returns false with loaded still true. For version 1 it passes the actual
target and six field addresses to the stream in order, without intermediate error checks
(`0x007CBBC7–0x007CBC25`), then returns the inverse of the stream error flag. Earlier writes survive a later
failure. A failed primitive read's effect on its own destination belongs to the stream implementation;
there is no camera rollback. Successful version-1 reads retain raw indices and float bits without validation,
and neither recompute the camera nor access the city. `Write` emits the same 38-byte layout and returns the
inverse stream error flag.

## 3. Users

- `cSC4View3DWin` (the city view) and its input controls drive the camera control (zoom commands, scrolling, the
  initial target); its record holds a reference to the camera control (save-records.md §3.2).
- Every ViewZRChange / ViewTChange subscriber: the model maker, the terrain view, the lighting manager, the zone
  manager (zones.md §5.2), effects (effects.md) and others.
- `cS3DCamera` is also used by the 3D renderer, the region view and thumbnail rendering (its constructor has five
  callers).

## 4. Quirks

1. Pixel snapping of the rotation corner (§2.3).
2. `SetCustomMagnification` tests the old magnification, not the new one (§2.6).
3. Zooms between 4 and 5 are magnifications of zoom 4; `GetVirtualZoom` reports 5 only above 1.5× (§2.6).
4. The city size used for the clip planes is at least 4096 m in each direction (§2.2).
5. A clamped move that runs out of steps keeps the start point (§2.5).

## 5. Open questions

- **CA1:** (§2.5) the geometric meaning of the 0.48 factor and of `lift` in the scroll polygon.
- **CA2** (answered 2026-10-20 by re-drawcontext): `SetDepthBias` (§1.2), from the draw context's depth offset.

## 6. Conformance tests

1. **Tables:** metres per pixel at magnification 1: 2.613126, 1.306563, 0.653281, 0.286370, 0.143185 for zooms 0–4;
   pitch 30°, 35°, 40°, 45°, 45°; yaw for rotations 0–3: 3π/8, 7π/8, 11π/8, 15π/8.
2. **Ortho matrix:** `SetOrtho(−400, 400, −300, 300, 10, 20)` with an 800×600 viewport and no offset gives
   `m00 = 1/400`, `m11 = 1/300`, `m22 = −0.2`, `m23 = −3`, `m03 = m13 = 0`; a pixel offset (1, 0) adds `−2/800`
   to `m03`.
3. **Project / UnProject:** with an identity view, the box above and viewport 800×600: the point (0, 0, −15)
   projects to (400, 300, 0); (400, 0, −10) to (800, 300, −1); `UnProject` of each gives the point back.
4. **Look-at:** position (0, 0, 10), target 0, up (0, 1, 0): right (1, 0, 0), up (0, 1, 0), back (0, 0, 1).
5. **Eye placement:** zoom 0, rotation 0, city 4096×4096, target (2048, 0, 2048): the eye offset is
   (cos(3π/8)·8192, tan 30°·8192, sin(3π/8)·8192) and the eye lies on the plane through corner + offset.
6. **SetZoom:** `SetZoom(4.5)` gives zoom 4 and magnification √2 ≈ 1.41421 (`GetVirtualZoom` 4); `SetZoom(4.75)`
   magnification ≈ 1.68179 (`GetVirtualZoom` 5); `SetZoom(5)` and `SetZoom(−1)` are refused; repeating a zoom is
   refused.
7. **Messages:** a rotation change sends ViewZRChange1, CameraUpdated, ViewZRChange2, ViewZRChange3 in that order
   with data2 pointing at (zoom, new rotation).
8. **Record:** version 1, three floats, six u32; version 2 is refused.
9. **Ground intersection:** origin (0, 100, 0), direction (1, −1, 0) → (100, 0, 0); direction (1, 0, 0) → false;
   direction (0, 1, 0) → false (behind).
10. **Native constructor discriminator:** before look-at, remembered target and up are both zero, the box is
    (−1, +1, −1, +1, 1, 2), projection type 0 and projection/cache counters 1/0. The first matrix request gives
    orthographic diagonal (1, 1, −2, 1) and depth translation −3, then cache counter 1. A centred side-1 box fails.
11. **Degenerate look-at:** identity camera at zero, target zero, up Y produces NaN rotation components;
    position (0, 0, 1), target zero, up Z produces finite back Z and NaN right/trueUp. Neither call substitutes
    identity or an alternate up; local counter advances by 2.
12. **Failed record mutation:** begin with loaded false and target (1, 2, 3). Reading version 2 returns false
    but loaded becomes true. With a version-1 mock stream that writes target (7, 8, 9) and previous zoom 17,
    then flags an error and leaves all remaining destinations untouched, Read returns false but preserves
    those writes. All six integer reads are attempted. No atomic commit is permitted.
13. **Lifecycle and malformed data:** pre-init near/far are 0/0, direction/right zero and up Y. Init with no
    camera returns true and sets initialised. A second Init with a camera does not attach it. After normal
    attachment/Init, Detach preserves initialised, target and zoom; another Init is still a no-op. Read a
    complete version-1 record with zoom 17, rotation 9 and NaN target x: Read succeeds if the stream has no
    error and retains those raw fields. Do not dereference the invalid zoom in a fidelity test. Separately,
    GetRotationCorner(9) into (123, 456, 789) produces (123, 0, 789).
14. **Magnification versus command result:** attached camera at zoom 4/rotation 0/magnification 2:
    SetZoomAndRotation(4,0) resets magnification to 1, emits PostViewRefresh, then returns false without
    ZR-change messages. From the same initial state, SetRotation(1) keeps magnification 2 and emits the
    normal ZR sequence. SetRotation(−1) and SetRotation(4) return true without changing anything.
15. **Input and viewport edges:** SetZoom(NaN) preserves all state and sends nothing. From old magnification
    1, SetCustomMagnification(NaN) stores NaN; a following SetCustomMagnification(1) cannot repair it.
    Two tweak calls (2,3) then (4,5) leave (4,5). With a nonempty polygon that second tweak does not rebuild
    bounds. Resizing an attached, computed camera from 800×600 to 1024×768 changes camera viewport/subview
    and adds 2 to its projection counter; the control's existing ortho box, eye, bounds and messages remain
    unchanged until another recomputation trigger.
16. **Synchronous observations:** a ViewZRChange1 receiver runs inside the command and sees the new current
    integer pair and old projection (absent earlier magnification work); later receivers see updated camera
    data. Pair data pointers refer to the control's fields, and the sender reuses its message object for
    CameraUpdated/change2/change3. A recorder of copied events delivered after return cannot satisfy this.

### 6.1 Independent scroll discriminators (CA6)

These are analytic consequences of §2.5's native branches, **not captured game outputs** and not derived
from core tests. Unless specified otherwise use P=[(0,0),(10,0),(10,10),(0,10)], bounds enabled, finite float
inputs. Approximate coordinates allow 0.0001 per component for the shown arithmetic stores; exact equality
cases remain exact. No terrain is needed for the pure clamp cases.

| From → to / setup | Required result or observation | Discriminates |
|---|---|---|
| (12,5) → (12,5) | (12,5), unchanged outside | no outside-start projection/fallback |
| (12,5) → (13,6) | (12,6), still outside | negative fraction becomes zero; tangent slide stays on outside line |
| (12,5) → (11,6) | (11,6), still outside | an inward move with d(S)≤d(T) is accepted |
| (5,5) → (10,5) | exactly (10,5) | fraction 1 is not selected or inset |
| (5,5) → (15,5) | approximately (9.999,5) | fractional inset gives 0.001 world units |
| (5,5) → (25,5) | approximately (9.998,5) | doubled move doubles inset distance |
| P narrowed to x=0…1; (0,5) → (2000,5) | (0,5) | fraction 0.0005 is replaced by zero |
| Same narrow P; (0,5) → (1000,5) | approximately (0.9000,5) | fraction stored as float(0.001) takes subtract branch |
| Reverse P; (5,5) → (6,5) | (5,5), versus (6,5) for P | fixed directed half-planes, no automatic winding repair |
| (5,5) → (15,15) | approximately (9.999,9.999); right edge selected before top at the tied 0.5, then opposite direction categories stop at Q | first-edge tie and history stop; final symmetric coordinates alone do not identify first edge |
| Synthetic two-point vector [(0,0),(0,h)], (−1,0) → (1,0), h bits 0x322BCC77 | (1,0); h≈9.99999994e−9, both edges skipped | compares length with double 1e−8 |
| Same with h bits 0x322BCC78 | approximately (−0.0002,0); h≈1.00000008e−8, edge active | rejects a length-squared or float-threshold substitution |

For the ground-failure paths, use attached-camera/terrain stubs with defined conversion results. Have the
camera-plane inverse return (7,8,9), direction (1,0,0), and terrain return false without writing the hit.
ScrollByViewportAngle leaves target (7,8,9), preserves velocity, returns true and synchronously sends
CameraUpdated/ViewTChange2/ViewTChange3. In ClampTargetToScrollBounds, force a clamp displacement above
the component threshold with the same failed ground direction: the **old** target and velocity survive,
but position update and those three messages still occur. A displacement below threshold sends nothing.
An exhaustion fixture must force N projections without an earlier no-edge/history stop; its expected result
is the original from. No general “inside after clamp” property is valid for these routines.
