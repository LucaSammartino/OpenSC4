# The network layout view and its custom view (behaviour spec)

Scope: `cSC4NetworkLayoutView` (the overlay that paints every transport network and some zones as coloured triangles
over the terrain) and `cSC4NetworkLayoutCustomView` (the GZCOM custom view that installs it, together with a path
view of the ferry routes), with its class-id function `GZCOM_cSC4NetworkLayoutCustomViewCLSID`. The path view itself
is overlay-views.md §1; the network types and edge words are network-rules.md §1.2–§1.3; the custom-view hook of the
data views is map-view.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly; the decompile was only a guide. The tables and
constants were read in the PE. Written 2026-10-25 (re-layoutview8).

---

## 1. Identity *[compat]*

**Custom view**: class id `0x4B8EA424` (`0x0060A7C0` returns it; registered with factory `0x0060AAA0`,
app-directors.md), interface `0x8A175D6B`. `0x14` bytes: vtable `0x00AA75DC` (0 QI `0x0060A7D0`, 1 AddRef, 2 Release,
3 `InitCustomView` `0x0060C0A0`, 4 `ShutdownCustomView` `0x0060A8B0`), a reference-count sub-object at `+0x04`
(vtable `0x00AA75C8`, slot 0 the deleting thunk `0x0060A810` = `this − 4`, then `0x0060AA50`), `+0x0C` the layout
view and `+0x10` the path view (both held, null at creation). QI answers IID 1 and `0x8A175D6B` (AddRef, true); any
other IID returns false without writing. The destructor (`0x0060AA50`) releases `+0x10`, then `+0x0C`, destroys the
reference-count base and frees when flag bit 0 is set. A data view creates it when its exemplar names this class
(map-view.md: creation, then slot 3 with no arguments).

**Layout view**: `0x5C` bytes, constructor `0x0060AC10`. Vtable `0x00AA760C` (slot 3 `Update` `0x0060A920`, the draw
callback); a `cRZUnknown` sub-object at `+0x04` (vtable `0x00AA75F4`, its deleting thunk `0x0060AC70` = `this − 4`,
then `0x0060AC80`).

| Offset | Meaning | Constructor |
|---|---|---|
| `+0x0C`, `+0x10`, `+0x14` | the vertex vector (begin, end, capacity), 16-byte vertices `{x, y, z, colour}` | null |
| `+0x18` | byte: initialised | 0 |
| `+0x1C` | packed render state (8 bytes) | 0 |
| `+0x24…+0x58` | the palette: 14 packed colours | copied from `0x00AA7574` |

The destructor (`0x0060AC80`) frees the vertex buffer, destroys the `cRZUnknown` base and frees when flag bit 0 is set.

### 1.1 The palette

Index i is at `+0x24 + 4i`. Entries 0–8 colour the network **codes** 1–9 (§3.1); 9–13 colour zones (§3.3).

| i | Used for | Default |
|---|---|---|
| 0 | code 1, Street | `0x40C8C8C8` |
| 1 | code 2, Rail | `0xFF232323` |
| 2, 3, 4 | codes 3, 4, 5: OneWayRoad, Road, Avenue | `0xFF969696` |
| 5, 6 | codes 6, 7: Monorail, LightRail | `0xFF780094` |
| 7, 8 | codes 8, 9: GroundHighway, Highway | `0xFFAF1E1E` |
| 9 | airport zone | `0xFF30305F` |
| 10 | seaport zone | `0xFF30305F` |
| 11 | plopped park | `0xFF507D50` |
| 12 | plopped rail building | `0xFF30305F` |
| 13 | every other zone | `0x1AFFFFC8` |

A colour of 0 means "do not draw".

---

## 2. Layout view Init (`0x0060AAF0`) and Update (`0x0060A920`)

**Init**: once (`+0x18`). When the renderer `0x00B43DD0` exists: its slot 10 (result ignored) and
`PackRenderState(+0x1C; 1,1,1,0, !kPreferOpaque, 0, 1, 3, 4,5, −1, −1.0, 1, −1)` (draw-context.md §5.1;
`kPreferOpaque` is render-property boolean 33). Then the palette exemplar: the resource manager `0x00B43CA8` slot 3
`GetResource(key, 0xA52160F5, &r, 0, 0)` with key `{type 0x6534284A, group 0x690F693F, instance 0x0B9EC13D}`. On
success: `r` slot 7, its slot 9 `GetProperty(0xAB9EC143)`; when the property exists, its value (slot 5): data (slot 103)
and count (slot 4); the first `min(count, 14)` words replace the palette from index 0. Initialised := 1 (on every path),
release `r`, return true.

**Update(dc)**: save the depth offset (`dc+0xBC`); `SetDepthOffset(4)`; reset the model transform and set an identity
one; `ResetTransparency`; `SetRenderState(+0x1C)`; draw the vertex vector in batches of at most 384 vertices:
`SetVertexBuffer(1, n, p)`, `DrawPrims(0, 0, n)` (draw-context.md: format 1, primitive type 0, the triangle list);
restore the depth offset; return true.

---

## 3. RebuildMap (`0x0060ACD0`) *[compat]*

Builds the vertex vector from the city. It reads neither `+0x18` nor the renderer.

### 3.1 The side grid

1. The list node allocator: the framework service `(0x3AE4BEA3, 0x3AE4BEAB)`, cached in `0x00B207E4` (as
   city-tools-input.md §8.13). A list L; an OR network filter F (`cSC4NetworkORFilter(0x1F4F)`, `0x10` bytes, held).
2. L := every network occupant of the city: occupant manager slot 41
   `GetOccupantsByStandardCityCells(&L, null, null, 0xC772BF98, −1)` (occupant-manager.md §8.5).
3. `(W, H) :=` the city's cell counts (city slots 93, 94). G := a zeroed 1 MB grid of 256 × 256 cells, cell `(x, z)` at
   index `x·256 + z`, each cell four 32-bit **side codes** (side i as in the edge words: 0 −x, 1 −z, 2 +x, 3 +z).
4. For each occupant O of L, in list order: `N := O.QueryInterface(0xA821EF94)` (the network occupant); Release O.
   Skip O when `N.HasAnyNetworkFlag(0x1F4F)` (slot 9) is false. Otherwise:
   1. `(x, z) := N.GetOccupiedCell()` (slot 59); `t := N.GetNetworkFlag()` (slot 12) `& 0x1F4F`.
   2. When `t = 9` (Road and Street): if both `N.GetEdgeMaskForType(0)` and `GetEdgeMaskForType(3)` (slot 38) have at
      most one side with non-zero low three bits (`m := ((e & 0x07070707) + 0x07070707) & 0x08080808`, `m & (m − 1) =
      0`), `t := 8` (Street).
   3. For j = 0…8 with the type `k :=` (3, 1, 10, 0, 6, 9, 8, 12, 2)[j] (table `0x00AA76B8`): when bit k of t is set
      (`0x00AA7540`), with `code := (z·256 + x)·256 + (j + 1)` and `e := N.GetEdgeMaskForType(k)`:
      - `e = 0`: unless `N.HasAllNetworkFlags(0x100000)` (slot 8, an intersection piece), set all four side codes.
      - `e ≠ 0`: set **all four** when any of: `e & 0x04040404 ≠ 0`; with `c := e & ~(e << 1)`,
        `c & 0x02000200 = 0x02000200`; `c & 0x00020002 = 0x00020002`; `c & 0x02000200 ≠ 0` and `e & 0x00FF00FF = 0`;
        `c & 0x00020002 ≠ 0` and `e & 0xFF00FF00 = 0`. Otherwise set side i's code for each non-zero byte i of e.
      A later j overwrites an earlier one, so Highway has the highest priority. The code's low byte names the type
      (1 Street, 2 Rail, 3 OneWayRoad, 4 Road, 5 Avenue, 6 Monorail, 7 LightRail, 8 GroundHighway, 9 Highway); bytes 1
      and 2 hold the source cell.
   4. Release N.

DirtRoad, Pipe, PowerLine and Subway never get a code.

### 3.2 Spreading the wide networks

Two passes over x < W, z < H, the first with P = the cell `(x − 1, z)`, the second with P = `(x, z − 1)`; for x = 0
(resp. z = 0) P is a zeroed scratch cell whose contents are never used. In each cell C the 14 moves of the pass's table
run in order; a move copies the whole code `src → dst` when `(src & 0xFF) > (dst & 0xFF)` and the source type
propagates (codes 5, 8 and 9: the table `0x00AA7670`).

| Pass | Moves (C = this cell, P = the previous one; digits are sides) |
|---|---|
| 1 (`0x00AA7680`) | C0→P0, C0→P1, C0→P2, C0→P3, C1→C0, C1→P1, C1→P2, C3→C0, C3→P2, C3→P3, C2→C1, C2→C3, C2→C0, C2→P2 |
| 2 (`0x00AA7638`) | C1→P0, C1→P1, C1→P2, C1→P3, C0→C1, C0→P0, C0→P3, C2→C1, C2→P2, C2→P3, C3→C0, C3→C2, C3→C1, C3→P3 |

### 3.3 The triangles

The vertex vector is emptied (end := begin). For each cell, x < W outer, z < H inner, with the corner heights
`y(i, j) := GetAltitudeAtVertex(x+i, z+j) + max(GetWaterDepthAtVertex(x+i, z+j), 0)` (terrain slots 14 and 33) and
the corners `v0 = (16x, y(0,0), 16z)`, `v1 = (16x+16, y(1,0), 16z)`, `v2 = (16x+16, y(1,1), 16z+16)`,
`v3 = (16x, y(0,1), 16z+16)`:

**All four side codes zero (a zone cell).** The zone type of the cell (zone manager `0x00B43D14` slot 5, its slot 25
rows, a signed byte) chooses the colour:

| Zone | Colour |
|---|---|
| 0 (none) | nothing drawn |
| 11 (airport) | palette 9 |
| 12 (seaport) | palette 10; but nothing when the cell's lot (lot manager slot 6 `GetLot(x, z, false)`) has a building (lot slot 48) not in group `0x1509` |
| 15 (plopped) | the lot's building in group `0x1006` (park): palette 11; else in `0x1300` (rail): palette 12; else, or no lot or building: palette 13 |
| any other value, negative included | palette 13 |

A non-zero colour adds the triangles `(v0, v2, v1)` and `(v0, v3, v2)`, all four corners in that colour.

**Otherwise (a network cell).** The centre `v4 = (16x+8, h, 16z+8)` with `h := (y(0,1) + y(1,0))·0.5` when the terrain's
`IsTriangulationFlipped(x, z)` (slot 70), else `(y(1,1) + y(0,0))·0.5`. For each side i with a non-zero code:

1. The occupant at the code's source cell: occupant manager slot 32
   `GetFirstOccupantByStandardCityCell(&o, byte 1, code >> 16, F)`. None: nothing for this side.
2. `N := o.QueryInterface(0xA821EF94)`. Add the triangle of table `0x00AA7620` for side i: `(v3, v4, v0)`,
   `(v0, v4, v1)`, `(v1, v4, v2)`, `(v2, v4, v3)` for sides 0–3, each vertex coloured `palette[code − 1]`. After each
   vertex is appended, its y becomes `N.GetAltitude(x, z)` (network occupant slot 56, at the vertex's own x and z)
   when that is higher.
3. Release N and o.

A side whose code is zero draws nothing, and a network cell never shows its zone colour.

Finally free G, release F and destroy L (`0x00413E60`; the elements were released during the walk).

---

## 4. The custom view

### 4.1 InitCustomView (`0x0060C0A0`)

Always returns true.

1. No layout view yet: create one (`0x5C` bytes), hold it in `+0x0C`, `Init`, `RebuildMap`, and add it to the renderer
   (slot 32 `AddViewObject(view, 5, 1000)`).
2. No path view yet (`+0x10`):
   1. Create a path view (`0x2C` bytes, overlay-views.md §1), hold it, `Init`.
   2. `R :=` the traffic simulator's ferry routes (`0x00B43DA8` slot 30 `GetAllFerryRoutes(&R)`, a list whose node data
      at `+0x08` is a compressed route).
   3. For each route: `SetThickness(18.0)`; `NewPath(0xFFFFFFFF, 2.0, 0)`; `SetPathTexture(0xABE9EB36, 0.25)`. Read the
      route's cells (`cSC4CompressedRouteReader`, pathfinder.md §4.15); each cell `(cx, cz)` becomes the point
      `(16cx, max(GetAltitude(p), GetLerpedWaterLevel(p)), 16cz)` (`0x0060A820`; terrain slots 13 and 32 at
      `p = (16cx, 16cz)`; NaN keeps the altitude).
   4. Smooth the points twice (§4.2) and add every point of the result to the path (`AddPathVertex`).
   5. Add the path view to the renderer (`AddViewObject(view, 5, 1000)`), then free R and the point buffers.

### 4.2 The smoothing pass

From the points `P[0…n−1]` it builds a new list: `P[0]`; then, for `i = 0, 2, 4, …` while `i + 3 < n` (that is
`((n − 4) >> 1) + 1` points when `n ≥ 4`), the cubic Bézier midpoint of `P[i…i+3]`; then `P[n − 1]`. The midpoint is
evaluated in Horner form with binary32 steps, each component separately:

```
C = 3·(P1 − P0)
B = 3·(P2 + P0) − 6·P1
A = (P3 − P0) − 3·(P2 − P1)
Q = P0 + 0.5·(C + 0.5·(B + 0.5·A))
```

(constants 3.0 `0x00A85074`, 6.0 `0x00A90900`, 0.5 `0x00A84D2C`). The value is `(P0 + 3P1 + 3P2 + P3) / 8` up to
rounding. A pass roughly halves the number of points; with fewer than four points only the first and last are kept
(a single point is doubled). An empty route is not guarded *[quirk]*: the first pass reads the first point of an
empty buffer.

### 4.3 ShutdownCustomView (`0x0060A8B0`)

When a path view is held: remove it from the renderer (slot 33 `RemoveViewObject`), `Shutdown`, release, clear `+0x10`.
When a layout view is held: remove it from the renderer, clear its initialised byte, release, clear `+0x0C`. Return true.

---

## 5. Quirks

1. The OR filter of §3.1 is only used for the per-side occupant lookups, not for the city-wide occupant query.
2. A spread code keeps its source cell, so the triangle drawn in the neighbour cell takes its height clamp from the
   source cell's occupant.
3. An empty ferry route is not guarded (§4.2).
4. The layout view's grid is always 256 × 256 cells; the loops stop at the city's own cell counts.

## 6. Conformance tests

1. A straight north–south road piece at (5, 7) (edge word `0x02000200` for type 0): all four side codes are
   `(7·256 + 5)·256 + 4` = `0x00070504`; the cell draws four triangles in palette 3.
2. A road–street piece whose road and street edge words each have one connected side: treated as Street only (code
   low byte 1).
3. Highway (code 9) in cell C side 0 and Road (code 4) in the previous cell's side 0: pass 1 copies the highway code
   over the road code; an Avenue (5) never replaces a Highway (9).
4. An empty cell in zone 11 adds two triangles in palette 9; in zone 0 nothing.
5. Smoothing of the points (0,0,0), (3,0,0), (6,0,0), (9,0,0), (12,0,0): one pass gives (0,0,0), (4.5,0,0),
   (12,0,0); the second pass gives (0,0,0), (12,0,0).
6. The palette property with 3 values replaces palette entries 0–2 only; with 20 values, the first 14.
