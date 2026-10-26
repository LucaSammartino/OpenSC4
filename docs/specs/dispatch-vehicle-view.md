# Dispatch vehicle view remainder and the dispatch tool (behaviour spec)

Scope: the whole of `cSC4DispatchVehicleView` (43 functions, 16,029 bytes), its `TrackInfo` constructor (95 bytes)
and the dispatch tool `cSC4ViewInputControlDispatch` (16 functions, 3,367 bytes), as tools/coverage.py assigns them.
dispatch.md §4.1.3 already specifies the view's construction, QueryInterface prefix, tracking handles, Update gates,
fade and delayed removal, the dispatch number texture, the intermediate quads, the rectangle getter, the conflict and
relocation search and the final 2D submission. This spec adds everything else and closes the residual that §4.1.3
parked (DP10): the camera rotation of the quads, the kinds 3/4 images, the complete per-frame walk, the world-space
branch and the final 2D vertices. It also specifies the other tracking kinds (2 to 6), which dispatch does not use.

Related: dispatch.md (manager, pylons, slot table), city-situation.md and city-tools.md (the city-situations tool,
which uses the selectable kind 4), mysim-tools.md (`CalculateDragPylonOffset`), view-input-controls.md (the base
class), framework-misc2.md §3 (the standard message slot map).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile, and the x87 sequences in §4 and §5 were read
instruction by instruction in the disassembly. Constants were read from the PE. Written 2026-10-25
(re-dispatchview8).

## 1. Data model *[compat]*

### 1.1 View layout (`0xA0` bytes)

| Offset | Field |
|---|---|
| `+0x10` | initialized byte (Init sets 1, Shutdown 0) |
| `+0x14`, `+0x1C`, `+0x24` | packed render states built by Init (§5.6 uses the first two) |
| `+0x2C` … `+0x44` | seven icon resource handles (dispatch.md §4.1.3 lists the keys) |
| `+0x48`, `+0x4C` | the track multimap: header node, count |
| `+0x54`, `+0x58` | the hidden-kind set (`std::set<int>`): header node, count |
| `+0x60` … `+0x78` | per-kind limits, seven dwords, Init sets `0xFFFFFFFF` |
| `+0x7C` … `+0x94` | per-kind tracked counts, seven dwords |
| `+0x98` | the active indicator handle (0 = none) |
| `+0x9C` | the window-rectangle list: header of a `std::list` of 20-byte records (§3.10) |

### 1.2 Track entries

The multimap is keyed by the **low byte of the kind**, inserted with equal keys after existing equal keys
(`0x0046F050`, node `0x124` bytes: the 16-byte tree header, the key byte at `+0x10`, then the `0x10C`-byte TrackInfo
at `+0x14`). Every walk therefore visits entries by kind, then in tracking order. Lookups by handle are linear
(§3.2). TrackInfo, relative to the info:

| Offset | Field |
|---|---|
| `+0x00` | handle (dispatch.md §4.1.3) |
| `+0x04` | kind 0 … 6 |
| `+0x08` | occupant, one reference held |
| `+0x0C` | generated texture (number or image) |
| `+0x10` | display number |
| `+0x14` | image resource instance (kinds 3, 4) |
| `+0x18` | label, a `cRZString` (`0x14` bytes) |
| `+0x2C` | the icon resource handle, copied from the view member of its kind (§2) |
| `+0x30`, `+0x80` | the number/image quad and the icon quad: four `(x,y,z,u,v)` float32 records each |
| `+0xD0`, `+0xD4` | display width and height (float32) |
| `+0xD8` … `+0xE4` | last screen rectangle (signed ints) |
| `+0xE8` … `+0xF0` | anchor (float32 x, y, z) |
| `+0xF4` | frame index (dword; kind 4 reads its low byte) |
| `+0xF8` | fade (float32) |
| `+0xFC` | flags (table below) |
| `+0x100` | animated model instance, one reference held (kind 6) |
| `+0x104`, `+0x108` | flash accumulator and flash period, seconds (float32) |
| `+0x10C` | model processor (kind 6), owned |

`TrackInfo::TrackInfo` (`0x0046C5A0`) only builds the empty label, zeroes the anchor and the model pointer; every
other field is set by TrackOccupant (§2).

| Bit | Meaning | Set by | Read by |
|---|---|---|---|
| 0 | the occupant is a building (type `0x278128A0`) | TrackOccupant | Update (ring placement, 2D only) |
| 1 | the entry has a number or image quad | kinds 0–4 | BuildCallNumberDisplay, Update |
| 2 | selectable | kind 4 | §3.6, §3.7 |
| 3 | fades in and out | kinds 0, 1, 2, 4, 5, 6 | Update fade (dispatch.md §4.1.3) |
| 4 | pending removal | UntrackOccupant, Update | Update, §3.6, §3.9 |
| 5 | visible | every kind | Update, §3.6 |
| 6 | 3D model indicator | kind 6 | Update, BuildCallNumberDisplay (early exit) |
| 7 | locked (keeps its rectangle and anchor) | LockIndicator | Update |
| 8 | flashing | FlashIndicator | Update |
| 9 | progress tint | kind 6 | Update3DModelIndicator |
| 10 | alternate | UseAlternateModel | nothing in the view |
| 11 | top indicator of its occupant | SortIndicatorPriority | Update |

## 2. Tracking kinds *[compat]*

`TrackOccupant(occupant, kind, number, label)` (`0x0046F240`) after the limit test of dispatch.md §4.1.3 builds a
TrackInfo with fade 0, flash accumulator 0, flash period 1.0, frame 0, rectangle 0, no texture, model or processor,
then by kind (jump table `0x0046F71C`):

| Kind | Icon handle | Flags | Other fields | Users |
|---|---|---|---|---|
| 0, 1 | `+0x2C`, `+0x30` | `0x2A` | number := argument 3 | dispatch (fire, police) |
| 2 | `+0x44` | `0x2A` | number := argument 3 | other callers |
| 3 | `+0x34` | `0x22` | label given: copy it, instance 0; else instance := argument 3; fade 1.0 | image indicators |
| 4 | `+0x3C` | `0x2E` | as kind 3; frame 1 | the city-situations tool |
| 5 | `+0x40` | `0x28` | none | other callers |
| 6 | none (0) | `0x68` | the 3D model below | other callers |
| other | none | `0x28` | none | not used |

Kind 6: `GetAnimatedModelFromResourceKey((0x5AD0E817, 0xBADB57F1, 0x8BB90000), &m)`; on failure the entry keeps no
model. On success, with `X` = m's slot 0 (no arguments; the model-instance interface): `X` slot 4 receives the
occupant, slot 14 receives 1, m's slot 8 receives (zoom, rotation) from `GetZoomAndRotation`; m is held at `+0x100`;
a `0x40`-byte `cS3DModelProcessor` is allocated and constructed (null on allocation failure) into `+0x10C` and
`SetModel` is called on it with m's model base, **without a null test**; flags `|= 0x200`; `X` slot 15 reads a
material record whose first byte gets `|= 0x10`, written back by slot 16; `X` slot 8 fills a bounding box and
`+0xD4` := float32 |maxY − minY|; m's slot 14 receives float32 `0x3C449BA6` (0.012); the reference from the loader is
released.

Then, for every kind: occupant slot 7 is its type. Type `0x278128A0` (building) sets bit 0. Otherwise bit 0 is
cleared and, for types `0x496E5783` and `0xC7CC75F1` (vehicles), the occupant's automaton interface
(`0xA9B40F05`) gets slot 30 `(0x200000, true)` (the flag RemoveTrackInfo clears). The occupant is AddRef'd,
BuildCallNumberDisplay runs on the local info, the kind's count and the global handle advance (dispatch.md §4.1.3),
the info is copied into the map (`0x0046D420`, §3.12) and SortIndicatorPriority runs for the occupant. The local
copy's model reference and label are released; the copied processor pointer is now owned by the map entry.

## 3. Entry points and helpers *[compat]*

### 3.1 SetIndicatorMax(kind, max) (`0x0046B4E0`)

Stores `max` at `+0x60 + 4·kind`. No range check, no effect on entries already tracked. 0 means no limit for
TrackOccupant (dispatch.md §4.1.3) but **hides the kind** in Update (§5.2 step 3 compares `count < limit`).

### 3.2 FindTrackInfoByHandle(out, handle) (`0x0046BE40`)

Walks the map in order and writes the first node whose `+0x14` (the info handle) equals `handle`, else the header.

### 3.3 LockIndicator(handle, on) (`0x0046BEB0`), FlashIndicator(handle, on, period) (`0x0046BF00`)

Absent handles do nothing. Lock sets or clears bit 7. Flash on sets bit 8, period := `period`, and lowers the
accumulator to `period` when it is larger; flash off clears bit 8 only (the fade keeps its last flash value).

### 3.4 GetActiveOccupant() (`0x0046BF70`)

The occupant of the active handle's entry, borrowed; null when there is no such entry.

### 3.5 UseAlternateModel(kind byte) (`0x005658D0`)

`find` on the multimap key (the first entry of that kind); sets bit 10 and returns true, or returns false. No view
function reads bit 10.

### 3.6 IsSelectableIndicator(info) (`0x0046C1F0`)

True when bits 2 and 5 are set, bit 4 is clear, fade ≥ float32 `0x3F666666` (0.9; an unordered fade fails) and the
kind is not hidden.

### 3.7 FindIndicatorHandle(x, y) (`0x0046C4B0`)

Walks every entry and keeps the handle of the **last** one that passes §3.6's test and whose rectangle contains the
point, edges included. 0 when none. The rectangle is the one stored by the last Update.

### 3.8 SetActiveIndicator(handle, frame) (`0x0046D310`)

Nothing when `handle` is already active. Otherwise the old active entry (if found) gets frame 1 and its display is
rebuilt, and the active handle becomes 0; then a non-zero `handle` whose entry exists and is selectable becomes
active with frame := `frame` and its display is rebuilt. An unselectable handle leaves none active.

### 3.9 ShowAllIndicators(kind, show) (`0x0046D3A0`)

`show` false inserts `kind` into the hidden set, `show` true erases it (`0x0046D170`); then
SortIndicatorPriority(null).

### 3.10 SortIndicatorPriority(occupant) (`0x0046C600`)

- Null: collect the distinct non-null occupants of all entries in a temporary set (pointer order) and call itself
  for each.
- An occupant: over its entries that are not pending removal and whose kind is not hidden, take the highest kind
  (starting from 0). Then every entry of that occupant gets bit 11 when its kind equals that value, else loses it.
  Pending and hidden entries are updated too; with no eligible entry, entries of kind 0 get bit 11.

Update draws only bit-11 entries, so one occupant shows only its highest-kind indicators.

### 3.11 Window rectangles (`0x0046D1D0`, `sEnumChildWindows` `0x0046C3C0`)

When the main window (global `0x00B43DC8`) exists: clear the list at `+0x9C`, append the main window's area
(window slot 50) with category `0xFF`, then enumerate its children with window slot 32
`(0x22BA0121, sEnumChildWindows, view)`. The cdecl callback `(parent, child id, child, view)` returns true always;
it appends the child's area with category `0xFF` when the child is non-null and visible (slot 71), is narrower **or**
shorter than the parent, its id is not `0xAA32BCE6`, `0xEA8CAD14` or `0x4C30E4FA`, and its slot 113
`(0xE6E998FE, &out)` returns false (an object returned there is released).

### 3.12 Lifetime and containers

- Destructor body `0x0046D730`: restores the three vtables, frees the window list, clears and frees the hidden set
  and the map headers, destroys the `cRZUnknown` base; the deleting destructor `0x0046F220` frees with
  `GlobalHeapFree` when flag bit 0 is set. Thunks: `0x0046D7E0` and `0x0046D800` are QueryInterface adjustors
  (ECX −4, −12), `0x0046D7F0` the deleting destructor's (ECX −4).
- `0x0046D420` copies a TrackInfo: plain fields, the label (string copy), both quads, the model with an AddRef
  through its virtual base, the processor pointer as is.
- `0x0046B770`, `0x0046B7B0`, `0x0046B7F0` (subtree erase), `0x0046BD80` (node insert), `0x0046C020` (range erase),
  `0x0046C1A0` (set destructor), `0x0046C2A0` (equal range), `0x0046C300` (insert unique), `0x0046D170` (erase by
  key, returns the count) are the STLport tree bodies of the int sets; `0x0046F050` the multimap's insert.

## 4. BuildCallNumberDisplay remainder (`0x0046C8B0`) *[compat]*

### 4.1 Kinds 3 and 4: images (`0x0046CB52`–`0x0046CCDA`)

After the shared texture release (dispatch.md §4.1.3), kinds 3 and 4 always take this branch (before bit 1):

1. Image source. A non-zero instance at `+0x14`: global resource manager `0x00B43CA8` slot 3 receives
   `(key (0x856DDBAC, 0x46A006B0, instance), 0x06F1568E, &res, 0, 0)`; false returns at once; the image is res's
   slot 3. No instance: GZCOM slot 3 creates `(0xA2FFB5D3, 0x02FFBB60, &factory)`; null returns at once; factory
   slot 4 receives `(label, &image, 8)`; the factory is released; a null image returns at once.
2. When the texture service `0x00B43CA4` exists, `SetupTexture(image, service, &info.texture, 1, 0, 0, 0)`.
3. W := image slot 9, H := image slot 10; the image is released, then res when non-null.
4. Kind 4: display size 35 × 35 (`0x420C0000`); side := `GetNearestPowerOfTwo(W)`; t := trunc(W · 0.25);
   c := the frame's low byte; left := trunc(t·c), right := trunc(t·c + t) (x87, `_ftol2`); number-quad depth 0.3,
   icon depth 0.
5. Kind 3: display size 32 × 32; side 64; left 0, right W; icon depth 0.3 (`0x3E99999A`), number-quad depth 0.
6. Both continue with the quad construction of dispatch.md §4.1.3, the number quad using measured height H,
   measured width `right − left`, left u = left·r, right u = right·r (r = 1/side stored float32), top v = r·0.0 and
   bottom v = H·r. Kind 4 thus shows frame c of a strip of four frames, t pixels wide.

The early returns of step 1 leave the old display size and quads in place, with no texture.

### 4.2 Camera rotation (`0x0046CF41`–`0x0046D0DB`)

After the quads are built, both are turned about the vertical axis to face the camera. With the view utility U
(`0x00B43DD8`), a := FPATAN of `(−U.+0x54, −U.+0x5C)` (that is atan2(−U.+0x54, −U.+0x5C)); c := float32 cos a;
s := float32 sin a. The coefficients pass through multiplications by the float32 constant 0.0 at `0x00A81054`;
for finite values they reduce to the plain rotation below, with c and s rounded to float32 before use.

For each of the four vertices, the icon vertex first, then (only when bit 1 is set) the number vertex, in vertex
order, in place, from the original (x, y, z):

- x' = float32( (z·s + x·c) + 0·y )
- y' = float32( (z·(±0) + x·(±0)) + y )
- z' = float32( (z·c − x·s) + 0·y )

Products and sums are x87 values, one float32 store per component; u and v are not touched. The `3×3` local
filled from `0x00B21600` at `0x0046CF15` is never read.

## 5. Update: the per-frame walk (`0x0046D990`) *[compat]*

### 5.1 Frame setup

After the timer sample and the context gate (dispatch.md §4.1.3), Update allocates a per-frame map
occupant → byte count and an obstacle list, then applies the `0x00B43DC4` slot 22 gate (on failure both are freed
and AL 1 returns; the timer stays stopped). It samples the simulator (`0x00B43D5C`, when present) slot 17 into a byte
S (0 without a simulator), zeroes seven per-frame per-kind counters, rebuilds the window rectangles (§3.11) and
copies every record **after the first** (the main window) into the obstacle list. The walk then visits the map in
order; each step saves the successor before processing the entry.

### 5.2 Per entry, in order

1. Fade and delayed removal (dispatch.md §4.1.3). A completed removal ends the entry.
2. Flash, when bit 8 is set: accumulator := float32(accumulator + elapsed·0.001) (elapsed = the timer's float32
   milliseconds); when the stored accumulator is **greater** than the period it becomes 0; then fade :=
   float32 cos(accumulator / period · float32 π/2) (`0x00A881B8`). This overrides the step 1 fade.
3. Visibility: bit 5 and bit 11 are required; the kind must not be hidden; kind 4 is skipped while S is non-zero;
   the frame counter of the kind must be below its limit (unsigned, `+0x60`), and is then incremented.
4. When bit 4 is clear, the occupant's automaton interface is queried; when its slot 31 `(1)` is true the entry
   becomes pending removal (bit 4). The query is released.
5. Bit 6: Update3DModelIndicator(context, info, elapsed) (§5.7) and the entry ends.
6. Position: occupant slot 8 into p. With the terrain `0x00B43CF4`: when its slot 11 `(p.x, p.z)` is true, h := slot
   13 `(p.x, p.z)`, else h := slot 29. p.y := h unless p.y > h. The transform T is the identity rotation with
   translation p (flags bytes 1, 1).
7. Screen rectangle (float32 left, top, right, bottom):
   - bit 7: the stored integer rectangle, converted;
   - bit 0 (building): the occupant's per-frame count n (1 for the first indicator of that occupant in this frame,
     then 2, 3 …); p is projected to the viewport in place (`ProjectPointToViewportCoordinates2D`); with
     θ = (72·n − 90)° (an int, times the float32 π/180 at `0x00B07F7C`), left := float32(cos θ·50 + p.x − 21), top :=
     float32(sin θ·50 + p.y − 21), right := float32(left + 42), bottom := float32(top + 42): a ring of five
     positions around the building, starting at −18°;
   - otherwise the icon quad's vertices 0 and 2 plus the translation (the identity rotation's zero products only
     add ±0), each component float32, are projected: vertex 0 gives (left, top), vertex 2 (right, bottom).
8. Bounds: w := float32(right − left), h := float32(bottom − top); B := the first window record inset by 32 on each
   side, centre as dispatch.md §4.1.3. Without bit 7 the rectangle is clamped, keeping its size: left < B.left →
   left := B.left, right := left + w, out := true; else right > B.right → right := B.right, left := right − w,
   out := true; then top < B.top → top := B.top and the entry goes 2D at once; else bottom > B.bottom → top :=
   B.bottom − h and 2D at once; else out → 2D. Locked entries skip the clamp.
9. Path: building entries, kind 4 and entries with bit 1 always go 2D; any other entry that was not moved by step 8
   is drawn in the world (§5.4). Since kinds 0–4 have bit 1, the world path serves kind 5 only.

### 5.3 2D path

The 42 × 42 candidate at (left, top), the relocation search and the obstacle append are dispatch.md §4.1.3. Then a
non-building entry's p is projected to the viewport in place (a building's already is). The rectangle and anchor are
stored (dispatch.md §4.1.3; a locked entry re-stores its old anchor). With (ax, ay) the anchor and (px, py) the
projected point: dx := px − ax, dy := py − ay, k := 1/sqrt(dx² + dy²) (x87), θ := float32 atan2(−float32(dx·k),
dy·k). The icon therefore turns so that its +y side points at the occupant; a zero distance gives a NaN angle.

The final quads (`0x0046EA4A`–`0x0046EDF1`), four `(x,y,z,u,v)` records each:

| Quad | Template vertices before the transform |
|---|---|
| icon | `(−32,−32,0,0,0)`, `(−32,32,0,0,1)`, `(32,32,0,1,1)`, `(32,−32,0,1,0)` |
| number | `(−hw,−hh,0,u0,v0)`, `(−hw,hh,0,u1,v1)`, `(hw,hh,0,u2,v2)`, `(hw,−hh,0,u3,v3)`, hw = display width·0.5, hh = display height·0.5, the uv of the world number quad's four vertices |

The tilt angle φ is the double constant 0.0 at `0x00A80990`, so the 2×2 matrix is the rotation by θ with
c := float32 cos θ and s := float32 sin θ: icon x := float32(float32((y·(−s) + z·0) + x·c) + ax),
y := float32(float32((z·(−0) + y·c) + x·s) + float32(ay)); z stays 0. The number quad is not rotated: x :=
float32(x + ax), y := float32(y + ay). The submission is dispatch.md §4.1.3's.

### 5.4 World path (`0x0046E845`–`0x0046E9DA`)

On the same context: ClearTexTransform(0); SetModelTransform(&T) (`0x007D3400`); SetTransparency(float32(fade·0.5));
SetRenderState(view `+0x1C`); SetTexture(icon, 0); DrawPrims(6, 2, 4, icon quad); SetTransparency(fade);
SetRenderState(view `+0x14`); SetTexture(icon, 0); DrawPrims the icon quad again; ResetTransparency;
SetModelTransform() (no argument). The number draws coded after each icon draw are unreachable (bit 1 is clear on
this path). The rectangle := the truncated (`_ftol2`) float rectangle of step 7, the anchor := p (world
coordinates). Nothing is added to the obstacle list. The icon quad is the camera-facing quad of §4.2 in world units.

### 5.5 End of the walk

The timer restarts (dispatch.md §4.1.3), the obstacle list and the per-frame map are freed, AL 1.

### 5.6 Render states

The world path's two passes use the states Init packs at `+0x1C` (half fade) and `+0x14` (full fade); the 2D
submission uses `+0x14` (dispatch.md §4.1.3).

### 5.7 Update3DModelIndicator(context, info, elapsed) (`0x0046B8F0`)

Nothing without an occupant or a model. X := the model's slot 0.

1. Progress q := 0. When the occupant's automaton interface exists, with r := automaton slot 56 and t := slot 67 (both
   non-null): when automaton slot 31 `(0x08000000)` is true, q := 1; else, walking t's parent chain (`0x0046B4A0`:
   the first object with a non-null `+0x64`, through `+0x0C`), when that object u exists and float32 `u.+0x5C ≠ 0`,
   q := (unsigned dword `r.+0x60`) / `u.+0x5C` (x87).
2. Bit 9: with q' = 1 − q, red := float32(0.921875·q + 0.12890625·q'), green := float32(float32(0.0546875·q) +
   float32(0.89453125·q')), blue := float32(float32(0.05859375·q) + float32(0.09765625·q')); X slot 15 reads the
   material record, its fields `+4`, `+8`, `+0xC` get red, green, blue and `+0x10` gets 1.0, X slot 16 writes it
   back. Green at q = 0, red at q = 1.
3. The processor, when present, gets `LightModel(0x00B43DDC, 0)`.
4. Occupant slot 8 into p; the model's slot 8 gets (zoom, rotation); p.y := float32((p.y + 10) + display
   height·0.5).
5. The model's transform (virtual-base slot 9) is read into M, its translation set to p, its flag bytes updated
   (`+1 |= 5`, `+0 += 3`), and M is turned by a := float32 7.0 (the never-written `0x00B07F6C`) · float32 π/180 about
   the vertical axis, per row i: `M[i][0] := M[i][0]·c − M[i][2]·s`, `M[i][2] := M[i][0]·s + M[i][2]·c` (c :=
   float32 cos a, s := sin a in x87; rows 1 and 2 round each product to float32 first, row 0 keeps them in x87;
   row 2's `M[2][2]` sums an x87 product and a float32 one). The model's slot 10 writes M back. The spin is 7° per
   Update call, independent of the frame time.
6. Model slot 18 `(fade < 1.0, fade)`, slot 15 `(elapsed)`, slot 20 `(1)`; X slot 12 `(context, 0)` draws. The
   automaton reference is released.

## 6. The dispatch tool (`cSC4ViewInputControlDispatch`) *[compat]*

### 6.1 Layout and lifetime

Over the `cSC4ViewInputControl` base (city-tools-input.md §2: id `+0x0C`, cursor id `+0x10`, the city view `+0x1C`):
message target `+0x28`; tool type `+0x2C`; "can drop here" byte `+0x30`;
drag offsets `+0x34`, `+0x38` (ints); double-click timer `+0x40` (a `cRZTimer`); default cursor `+0x58`; hovered
pylon `+0x5C` (held); cursor text `+0x60` (`cRZString`); cursor-text enabled byte `+0x74`; pylon filter `+0x78`
(held); saved pylon position `+0x7C`…`+0x84`; saved dispatch state `+0x88`. The constructor `0x004BA7D0` takes the
type and zeroes the rest (the position from the zero vector `0x00B21800`); the destructor `0x004BA880` destroys the
string and the timer, then the base, and frees on flag bit 0. `0x004BA870` is the message target's QueryInterface
adjustor (ECX −0x28, to `0x004B9DE0`).

Every dispatch-manager call goes either through the city (`0x00B43CEC` slot 39) or the global `0x00B43D04`; both are
the same manager (dispatch.md §8 names the slots).

### 6.2 Init (`0x004BAAB0`) and Shutdown (`0x004B9EF0`)

Init, once: id `0x68E1900A`; cursor id `0xC2E4353D` for types 0 and 6, `0x02E43537` for 1 and 5, 1 for 2–4 (`+0x58`
keeps the same value); `BeginDispatchTool(type)`; timer units 4; base Init; subscribe `+0x28` to
`0x09C8B8EA`, `0xE94671EB`, `0xE94671EC`, `0xE94671ED`, `0x894671F6`; `+0x74` := 1 and the cursor text (§6.3); the
filter: a `0x0C`-byte `cSC4OccupantFilter` given the tool's vtables `0x00A9023C`/`0x00A90224` and AddRef'd (an
allocation failure calls AddRef through null). AL 1.

Shutdown, when initialized: the city view's slot 41 `(0x49D02C4D)` removes the cursor text; unsubscribe the five
messages; release the hovered pylon (no UI state change); when a pylon is active, restore its position (pylon
occupant slot 9 with `+0x7C`) and dispatch state (`+0x88`) and `SetActivePylon(null)`; `EndDispatchTool(type)`;
release the filter; base Shutdown. AL 1.

### 6.3 Cursor text (`0x004BA8B0`)

The cursor text := the localized string `(0x6A231EAA, 0x4A54ED13)` formatted with
`GetAvailablePylonCount(type)`; when the city view exists its slot 40 receives `(0x49D02C4D, 0, 0, &text, 0)`.

### 6.4 DoMessage (`0x004BA940`, on the `+0x28` target)

- `0x09C8B8EA` (input control change: void1 the old control, void2 the new one): old = this tool and new ≠ it →
  unless the new one's id (slot 5) is `0x48E2A1F8` (right-drag scrolling, view-input-controls.md §2), deactivate: remove the cursor text, `+0x74` := 0, the hovered pylon
  gets UI state 1 and is released, an active pylon gets its position and dispatch state restored and
  `SetActivePylon(null)`. New = this tool and old ≠ it → `+0x74` := 1 and the cursor text.
- `0xE94671EB`…`0xE94671ED`, `0x894671F6`: the cursor text when `+0x74` is set.
- `0x99EF1143` (not subscribed): when void1 is the hovered pylon's occupant, release it (no null test).
- Others: AL 0; the handled ones AL 1.

### 6.5 Picking and hover

`PickPylonOccupant(x, z, &out)` (`0x004B9FF0`): renderer `0x00B43DD0` slot 65 `(x, z, filter, &hit)`; on a hit, the
hit's slot 3 object queried for the pylon IID `0xC9B6D7D6` gives `out` (acquired); the hit is released. The filter's
predicate (`0x004B9E40`, vtable slot 3) accepts an occupant of type `0xA9B6D7DE` whose pylon has a non-zero UI state,
is not being dragged (slot 16 false), and has no owner (slot 20) or an owner without interface `0x0A1DA42F`.

`0x004B9D90` hovers a pylon: when it changes, the old one gets UI state 1 and is released, the new one is held and
gets UI state 3.

### 6.6 OnMouseMove(x, z, flags) (`0x004BA320`)

Without a city view nothing happens. With an active pylon (dragging): cursor `0x13B138D0`; while the timer runs
and less than 500 units passed, return at once (no move); when it runs and 500 passed, reset it; the point is
`(x + dragX, z + dragZ)`. Without one: a picked pylon with UI state ≠ 0 is hovered with cursor `0x13B138D0`; any
other result un-hovers and sets the default cursor. Then the city view's slot 36 `(x, z, &pt, 0)`; on success
pt.y := the terrain altitude (city slot 84, terrain slot 13) as float32, and "can drop" := the manager exists, flags
bit 1 is clear and `CanDispatch(type, pt.x, pt.z)`; an active pylon is moved to pt and gets UI state 3 when it can
drop, UI state 1 otherwise. `+0x30` := "can drop". AL 1.

### 6.7 OnMouseDownL(x, z, flags) (`0x004BA510`)

Every exit calls `PlayResultSound(0)` (slot 28, which does nothing). Without a city view: AL 0. A second click
within 500 units of the first (the timer runs): reset the timer; p := the active pylon; drop it (§6.8); when
`GetAutomatonFromPylon(p)` is non-null the city's slot 43 object gets slot 34 `(automaton)` (follow it) and message
`0x49F10111` is posted with p; AL 1
(p is used after the drop released the manager's reference; the dispatch record keeps it alive). Otherwise the timer
restarts and:

1. A picked pylon other than the active one: without flags bit 0, drop the active one, `SetActivePylon(pylon)`, save
   its position and dispatch state, compute the drag offset (`CalculateDragPylonOffset`: the projected pylon minus
   the mouse, truncated), the city's slot 43 object gets slot 34 `(0)` and message `0xC9F1011A` is posted with the
   pylon; with flags bit 0, drop the active one and, when the pylon has an automaton, follow it and post
   `0x49F10111`. Either way the click is handled (AL 1). The same pylon as the active one is not handled.
2. Otherwise, with the active pylon the point is offset by the drag offset; the city view's slot 36 picks pt
   (failure: AL 0) with its altitude. An active pylon is moved to pt and dropped (AL 0). Without one: when
   `CanDispatch(type, pt.x, pt.z)` and `0x004BA0D0(type, &pt, 1)` (type ≠ 8, the manager exists and
   `Dispatch(type, pt.x, pt.z)` is non-zero) the click is handled (AL 1).

### 6.8 DropActivePylon (`0x004BA1B0`)

p := the active pylon; none → AL 0. Its position P; when its source station exists, the station's bounding cells
give corners A = (minX, minZ) and B = (maxX + 1, maxZ + 1) through city slot 96. When `A.x ≤ P.x ≤ B.x` and
`A.z ≤ P.z ≤ B.z` (edges included): `RevokeDispatchByPylon(p)`, `SetActivePylon(null)`, AL 0. Otherwise:
`CanDispatch(type, P.x, P.z)` false or type 8 → `SetActivePylon(null)`, AL 0; else `Dispatch(type, P.x, P.z)`, then
`SetActivePylon(null)`, AL = (ID ≠ 0). The path after the first test reads the manager global without a null test.

### 6.9 Small members

`OnKeyDown(key, flags)` (`0x004B9D60`): Tab (9) is consumed; other keys are consumed unless the base
`IsKeyToBePassed` says otherwise. `0x004BA120` posts a standard message: type, IGZUnknown (slot 33), data 1–4,
through message server `0x00B43CCC` slot 4 `(message, 0)`, then releases it.

## 7. Quirks (reproduced)

1. Kind 6's processor gets `SetModel` even when its allocation failed (null `this`).
2. A limit of 0 lets TrackOccupant accept the kind but hides every entry of it in Update.
3. The flash accumulator is reset only when it is strictly greater than the period.
4. The 2D icon angle is NaN when the anchor equals the projected point.
5. The 3D-model indicator spins 7° per Update call, so its speed follows the frame rate.
6. The tool's filter AddRef is called through null when its 12-byte allocation fails.
7. DoMessage's `0x99EF1143` branch dereferences the hovered pylon without a null test (the message is not
   subscribed).
8. A double click follows the automaton of a pylon whose active reference was just released.

## 8. Conformance tests

1. Kinds: TrackOccupant with kinds 0–6 gives flags `0x2A, 0x2A, 0x2A, 0x22, 0x2E, 0x28, 0x68` (plus bit 0 for a
   building occupant), frame 1 only for kind 4, fade 1.0 only for kind 3.
2. Kind 4 strip: an image 200 × 50 with frame 2 gives side 256, t 50, u from 100/256 to 150/256, v to 50/256, size
   35 × 35; frame 3 gives u to 200/256.
3. Rotation: U.+0x54 = 0, U.+0x5C = −1 gives a = 0: the quads are unchanged; U.+0x54 = −1, U.+0x5C = 0 gives
   a = π/2: (x, y, z) → (z, y, −x) up to float32 rounding of cos π/2.
4. Ring: three building indicators of one occupant at projected (100, 100) get rectangles starting at θ −18°, 54°,
   126°: the first at left = float32(cos(−18°)·50 + 79).
5. Clamp: with B = (32, 32, 768, 568) a rectangle (10, 40, 52, 82) becomes (32, 40, 74, 82) and goes 2D; a rectangle
   inside B of kind 5 on a non-building takes the world path and does not reach the obstacle list.
6. Priority: an occupant with kinds 1 and 3 tracked draws only kind 3; hiding kind 3 (ShowAllIndicators(3, false))
   makes kind 1 the top one.
7. Selection: FindIndicatorHandle returns the last of two overlapping selectable entries in map order; an entry with
   fade 0.89 is never returned.
8. Drop: a pylon dropped inside its station's cells revokes and returns false; outside, with CanDispatch true and
   Dispatch returning 7, returns true; type 8 returns false without Dispatch.

## 9. Function inventory

`cSC4DispatchVehicleView`: `0x0046B4E0`, `0x0046B500`, `0x0046B550`, `0x0046B770`, `0x0046B7B0`, `0x0046B7F0`,
`0x0046B830`, `0x0046B8F0`, `0x0046BD80`, `0x0046BE40`, `0x0046BEB0`, `0x0046BF00`, `0x0046BF70`, `0x0046BFA0`,
`0x0046C020`, `0x0046C1A0`, `0x0046C1F0`, `0x0046C2A0`, `0x0046C300`, `0x0046C3C0`, `0x0046C4B0`, `0x0046C600`,
`0x0046C8B0`, `0x0046D0F0`, `0x0046D170`, `0x0046D1D0`, `0x0046D2B0`, `0x0046D310`, `0x0046D3A0`, `0x0046D420`,
`0x0046D650`, `0x0046D730`, `0x0046D7E0`, `0x0046D7F0`, `0x0046D800`, `0x0046D810`, `0x0046D990`, `0x0046EFB0`,
`0x0046F050`, `0x0046F120`, `0x0046F220`, `0x0046F240`, `0x005658D0`. `cSC4DispatchVehicleView::TrackInfo`:
`0x0046C5A0`. `cSC4ViewInputControlDispatch`: `0x004B9D60`, `0x004B9D90`, `0x004B9E40`, `0x004B9EF0`, `0x004B9FF0`,
`0x004BA0D0`, `0x004BA120`, `0x004BA1B0`, `0x004BA320`, `0x004BA510`, `0x004BA7D0`, `0x004BA870`, `0x004BA880`,
`0x004BA8B0`, `0x004BA940`, `0x004BAAB0`.
