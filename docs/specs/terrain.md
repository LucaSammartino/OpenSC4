# Terrain (`cSTETerrain`, `cSTESurfaceWater`, `cSTEVolcanoLava`): behaviour spec

Status: round 1 (re-specs14a, 2026-10-15). Written from the Windows 1.1.641 executable (terrain code
`0x007405D0`–`0x0074FEA0`, surface water `0x0073CD50`–`0x0073FE70`, lava `0x0075CE60`–`0x0075FAE0`). Every numeric
rule below was checked against the instruction bytes unless it is tagged *[decomp only]*. The save-record facts were
also checked against a corpus of 502 saves in the user's regions.

Scope: the parts of the terrain that the **simulation** reads or changes: the height field and its queries, slope,
water presence and depth, the change messages and their `data4` kinds, the brushes that the disasters and tools
apply, the lava as far as the volcano disaster needs it, the saved records, and the surface-water object that the
traffic, zone and watercraft code query. Drawing (textures, cliffs as art, waves, decals, steam, lighting, the 3D
terrain view) is out of scope; where a function mixes both, only the simulation-visible effects are given.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced, or that a replacement must survive.

Shared material cited and not repeated here:
- streams, records and object references: foundations §2;
- class store, load order and the component table (terrain is component 49): §3.5, §3.6;
- messages, `MessagePost` (queued) and `MessageSend` (synchronous): foundations §5;
- x87 precision (every float operation below rounds to a 24-bit significand, written `r24`): foundations §18;
- the volcano and earthquake disasters, the fire manager and its slots: disasters.md §4, §5, §7, §8, §10;
- the network manager's terrain handler: network-manager.md §7.6;
- the economy modifier of the nation: demand.md §8.2, and [nation.md](nation.md).

---

## 1. Identity and ownership

| Item | Value |
|---|---|
| Terrain class | `cSTETerrain`, CLSID `0xE98F9525` (`GetGZCLSID` `0x007405D0`). Created by the city as component 49 (foundations §3.6), returned by city slot 84 (`GetTerrain`, `+0x150`) |
| Terrain IIDs | `0x00000001`, `0xAB953253` and `0xC70BD37E` (`cISTETerrain`, the component IID) → this; `0x452294AA` (`cIGZMessageTarget2`) → this + 4; `0xE4FDA3D4` (`cIGZSerializable`) → this + 8 (QueryInterface `0x00741E90`) |
| Terrain vtable | `cISTETerrain` at `0x00AB3D18`, slots 0–97 (§15.1) |
| Surface water class | `cSTESurfaceWater`, CLSID `0x0B8DA7DB`, vtable `0x00AB3B00` (slots 0–14, §15.2). Owned by the terrain (terrain `+0x24`) and returned by city slot 83 (`GetSurfaceWater`, `+0x14C`) |
| Lava class | `cSTEVolcanoLava`, object size `0x178`. Not a city component: the volcano disaster instance creates one per eruption (§11) and releases it at the end |
| Terrain view | the 3D terrain view (terrain `+0xE8`, class `0xC9B84E10`), saved through a reference in the terrain's class record (§13.2). Rendering only |

The terrain object holds the vertex array (altitudes, normals, flags). The surface water object holds the per-vertex
water array (depths, velocities, sediment). Most water queries are terrain slots that read the surface water's array
directly (§5); the surface water's own slots add the cell queries and navigability (§10).

---

## 2. Grid, coordinates and state

### 2.1 Grid *[compat]*

- A **cell** is 16 m × 16 m (`CellWidth`, slot 6, constant 16.0). The city has `cellsX × cellsZ` cells (64, 128 or
  256 per side; city slots 93 and 94, read in `Init`).
- **Vertices** sit on cell corners: `vertsX = cellsX + 1`, `vertsZ = cellsZ + 1`. Vertex `(x, z)` is at world
  position `(16x, 16z)`. Its **index** is `x + z × vertsX` (`GetVertIndex`, slot 9, no bounds check).
- `GetVertex(index, &x, &z)` (slot 10): `x = index mod vertsX`, `z = index div vertsX` (unsigned).
- Cell `(cx, cz)` has the corners `(cx, cz)`, `(cx + 1, cz)`, `(cx, cz + 1)`, `(cx + 1, cz + 1)`.
- World positions are metres: `0 ≤ x ≤ sizeX = 16 × cellsX`, same for z. y is the altitude in metres.

### 2.2 Per-vertex records *[compat]*

| Array | Record | Fields read by the simulation |
|---|---|---|
| terrain vertices (`+0x6C`) | `0x24` bytes | `+0x00` altitude (float); `+0x04..+0x0C` normal (render); `+0x22` uint16 flags (§7) |
| surface water (`water +0x18`) | `0x24` bytes | `+0x00` **depth** (float, metres, relative to the ground: water level = altitude + depth); `+0x10` velocity vector; `+0x1C` sediment (float) |

The depth is negative on dry land: almost every height setter writes `depth := seaLevel − altitude` (§6), so a land
vertex above the sea has a negative depth. **"Water" always means depth > 0**, "land" means depth ≤ 0.

### 2.3 Terrain object fields *[compat where a slot returns them; otherwise internal]*

| Offset | Meaning |
|---|---|
| `+0x0C` | initialised flag (byte; slot 5 returns it) |
| `+0x18` | terrain type (int32; slots 41/42; saved in the misc record; 0 in every corpus save) |
| `+0x1C` | the city |
| `+0x24` | the surface water object |
| `+0x28`, `+0x2C` | `cellsX`, `cellsZ` (slots 7 and 8 return them) |
| `+0x30`, `+0x34` | `cellsX − 1`, `cellsZ − 1` |
| `+0x38`, `+0x3C` | `vertsX`, `vertsZ` |
| `+0x40..+0x4C` | the whole-city vertex rectangle `(0, 0, cellsX, cellsZ)` |
| `+0x50`, `+0x54` | `sizeX = 16 × cellsX`, `sizeZ` (metres, float) |
| `+0x58` | city size type (city slot 87) |
| `+0x5C` | vertex count `vertsX × vertsZ` |
| `+0x6C` | the vertex array |
| `+0x70`, `+0x74` | cached minimum and maximum altitude (slots 21, 22; §4.6) |
| `+0x78` | tuning `0x48CD7B20` (shipped 308); saved |
| `+0x7C` | sea level (slot 29) |
| `+0x84..+0xA0` | the 8 neighbour index offsets (for the lava and the flood code) |
| `+0xBC` | the default brush level (§9.2) |
| `+0xC0` | "accumulating texture updates" flag (slots 78–80; render) |
| `+0xD8` | tick counter for the reconciliation post (§12.1) |
| `+0xDD` | water-surface display state (slot 83 returns it, slot 84 sets it) |
| `+0xE8` | the terrain 3D view |
| `+0xEC` | **view-ready** flag, set by `DoPostCityInit` (slot 40, `0x00749540`). `RedisplayTerrain` does nothing until it is set (§8.1) |

### 2.4 Tuning *[compat]*

Read once per process by `ReadTuningParameters` (`0x0074B6A0`) from exemplar `0x6534284A / 0x88CD66E9 / 1`
("Terrain Properties"):

| Property | Meaning | Code default | Shipped |
|---|---|---|---|
| `0x48CD7B26` | maximum allowed height (slot 24; the raise clamp, §6) | 200 | 2500 |
| `0x48CD7B27` | minimum allowed height (slot 23; the lower clamp) | 5 | 20 |
| `0x48CD7B21` | sea level of a new terrain (clamped ≥ minimum allowed height) | — | 250 |
| `0x48CD7B2A` | flat height of a new terrain (clamped ≥ minimum) | — | 300 |
| `0x48CD7B28` | height scale for a new terrain from an image | — | 3 |
| `0x48CD7B70`..`73` | sea-level range for `SetSeaLevel`: minimum, maximum, and two editor values | — | −10, 200, 5, 40 |
| `0x48CD7B65` | cliff slope threshold (render) | — | 0.5 |
| `0x48CD7B20` | stored at `+0x78` and saved | — | 308 |

`GetDefaultTerrainRaiseLowerAmount` (slot 47) returns the constant 20.0. The lava has its own exemplar (§11.1).

---

## 3. Bounds

| Slot | Function | Rule *[compat]* |
|---|---|---|
| 11 | `LocationIsInBounds(x, z)` (`0x0074C610`) | `0 ≤ x ≤ sizeX` and `0 ≤ z ≤ sizeZ` (both ends inclusive). NaN is out |
| — | vertex bounds | `x < vertsX` and `z < vertsZ`, compared **unsigned** (negative is out) |
| — | cell bounds | `x < cellsX` and `z < cellsZ`, unsigned |

---

## 4. Height queries

### 4.1 `GetAltitude(x, z)` (slot 13, `0x00741260`) *[compat]*

The altitude of the ground at a world position, by linear interpolation on one of the two triangles of the cell.

1. If not `LocationIsInBounds(x, z)`: return 0.0.
2. `ix := floor(r24(x × 0.0625))`, `iz := floor(r24(z × 0.0625))`. The original computes the floor with the
   "1.5 × 2^23" trick: round `t − 0.5` to the nearest integer (ties to even), then add 1 if `t` equals that
   integer + 1. The result is exactly `floor(t)` for every in-bounds `t`.
3. Clamp `ix ≤ cellsX − 1` and `iz ≤ cellsZ − 1` (so `x = sizeX` uses the last cell).
4. `fx := r24(x − 16·ix)`, `fz := r24(z − 16·iz)` (metres into the cell, 0..16).
5. Heights: `h00 = (ix, iz)`, `h10 = (ix + 1, iz)`, `h01 = (ix, iz + 1)`, `h11 = (ix + 1, iz + 1)`.
6. The diagonal always runs from `(ix, iz)` to `(ix + 1, iz + 1)`:
   - if `fx ≤ fz` (the triangle `h00, h01, h11`):
     `h := h00 + r24(r24(r24((h11 − h01) × fx) + r24((h01 − h00) × fz)) × 0.0625)`;
   - otherwise (`fx > fz`, or a NaN compare; the triangle `h00, h10, h11`):
     `h := h00 + r24(r24(r24((h11 − h10) × fz) + r24((h10 − h00) × fx)) × 0.0625)`.

   Every subtraction is rounded too (`r24`), and the final addition is rounded.

*[quirk]* The query **ignores** the triangulation-flip flags of §7 (`0x0800`, `0x1000`, `0x2000`): a flipped cell is
drawn with the other diagonal, but its altitude is still read across `(ix, iz)–(ix + 1, iz + 1)`.

### 4.2 The other height reads *[compat]*

| Slot | Function | Rule |
|---|---|---|
| 12 | `GetAltitude(index)` (`0x007413C0`) | `index ≥ vertexCount` (unsigned) → 0.0; else the vertex altitude |
| 14 | `GetAltitudeAtVertex(x, z)` (`0x007413E0`) | vertex out of bounds → 0.0; else the altitude |
| 15 | `GetAltitudes(rect*, out, rowStride, colStride)` (`0x00742A00`) | copies the altitudes of an inclusive vertex rectangle *[decomp only]* |
| 17 | `GetCellCornerAltitudes(x, z, out[4])` (`0x007415F0`) | only if the **cell** is in bounds; order `(x, z)`, `(x, z + 1)`, `(x + 1, z + 1)`, `(x + 1, z)`. Out of bounds: `out` is untouched |
| 19 | `GetAltitudeAtNearestGrid(x, z)` (`0x00741420`) | `LocationIsInBounds` else 0.0; vertex `(round(x / 16), round(z / 16))`, rounding to nearest, ties to even; no further clamp |
| 20 | `GetAltitudeCheckSum(rect*)` (`0x007414A0`) | the sum of the altitudes over an inclusive vertex rectangle; 0.0 for an invalid rectangle |
| 21, 22 | `MinHeight`, `MaxHeight` | the cached `+0x70`, `+0x74` (§4.6) |
| 23, 24 | `MinAllowedHeight`, `MaxAllowedHeight` | the tuning (20, 2500) |

### 4.3 Slope: `GetFaceNormal(x, z, out)` (slot 26, `0x00742B10`) *[compat]*

1. Only if `0 ≤ x < sizeX` and `0 ≤ z < sizeZ` (upper ends **exclusive**). Otherwise `out` is untouched.
2. `ix, iz, fx, fz` as in §4.1 (no clamp is needed here). The triangle test is `fz < fx` (the same split as §4.1).
3. Unnormalised normal, each step rounded:
   - `fz < fx`: `nx := r24(0 × d) − 16(h10 − h00)` and `nz := 16(h10 − h00) − 16(h11 − h00)`;
   - otherwise: `nx := 16(h01 − h00) − 16(h11 − h00)` and `nz := r24(0 × d) − 16(h01 − h00)`;
   with `d = r24(h11 − h00)`, and `ny := 256`. Algebraically: `nx = −16(h10 − h00)`, `nz = −16(h11 − h10)` in the
   first case, and `nx = −16(h11 − h01)`, `nz = −16(h01 − h00)` in the second.
4. `s := 1 / sqrt(nx² + nz² + 65536)` (sum in that order) and `out := (nx·s, 256·s, nz·s)`.

The slope angle of the face is `acos(out.y)`. Slots 27 (`GetLerpedNormal`) and 28 (`GetVertexNormal`) return the
smoothed render normals; no simulation code in this spec reads them.

### 4.4 Worked example *[compat]*

Cell (0, 0) with `h00 = 100`, `h10 = 104`, `h01 = 108`, `h11 = 120`:

| Query | Triangle | Result |
|---|---|---|
| `GetAltitude(4, 12)` | `fx ≤ fz` | 100 + (12·4 + 8·12) / 16 = **109** |
| `GetAltitude(12, 4)` | `fx > fz` | 100 + (16·4 + 4·12) / 16 = **107** |
| `GetAltitude(8, 8)` | `fx ≤ fz` | 100 + (12·8 + 8·8) / 16 = **110** (the other formula also gives 110) |
| `GetAltitude(0, 0)` | `fx ≤ fz` | **100** |
| `GetAltitude(−0.001, 0)` | out of bounds | **0** |
| `GetFaceNormal(12, 4)` | `fz < fx` | n = (−64, 256, −256) / sqrt(4096 + 65536 + 65536) |

### 4.5 Edge case: the last row and column

For a 64-cell city, `GetAltitude(1024, 1024)` uses cell (63, 63) with `fx = fz = 16` and returns `h11` of that
cell, the corner vertex (64, 64). `GetFaceNormal(1024, z)` is out of bounds (untouched output).

### 4.6 The cached minimum and maximum *[compat]*

`+0x70` and `+0x74` are recomputed over **all** vertices only by `Create` (new terrain), `CreateFlatAreas` and
`RedisplayTerrain` with the terrain flag (after the view is ready, §8.1). Load reads them from the misc record
(§13.1). *[quirk]* So they go stale: in 67 of the 502 corpus saves the stored values differ from the real minimum
and maximum of the stored altitudes. A replacement must return the cached values, not a fresh scan.

---

## 5. Water queries on the terrain

All read the surface water's depth array. Water level = altitude + depth.

| Slot | Function | Rule *[compat]* |
|---|---|---|
| 29 | `GetSeaLevel()` | `+0x7C` |
| 30 | `GetWaterLevel(x, z)` (`0x007449B0`) | vertex `(floor(x / 16), floor(z / 16))` (no bounds clamp): `depth + altitude` |
| 31 | `GetWaterLevel(index)` (`0x00744980`) | `depth + altitude` |
| 32 | `GetLerpedWaterLevel(x, z)` (`0x007444E0`) | bilinear interpolation of `depth + altitude` over the four corners of the cell *[decomp only]* |
| 33, 92 | `GetWaterDepthAtVertex(x, z)` / `(index)` | the depth (may be negative) |
| 18 | `GetMaxSurfaceLevel(x, z)` (`0x00744620`) | cell in bounds, else 0.0. The maximum over the corners `(x, z)`, `(x, z + 1)`, `(x + 1, z + 1)`, `(x + 1, z)` of `altitude + max(depth, 0)` |
| 34 | `CellHasWater(x, z)` (`0x00743FA0`) | cell out of bounds → false. True if **any** of the corners `(x, z)`, `(x + 1, z)`, `(x, z + 1)`, `(x + 1, z + 1)` has `depth > 0`. A NaN depth is not water |
| 35 | `CellHasLand(x, z)` (`0x00744030`) | cell out of bounds → false. True if any corner has `depth ≤ 0`. A NaN depth is not land |
| 36 | `IsLandVertex(x, z)` (`0x00747680`) | vertex out of bounds → false; `depth ≤ 0` |
| 37 | `IsWaterVertex(x, z)` (`0x007476D0`) | vertex out of bounds → false; `depth > 0` |
| 38 | `IsSeaVertex(x, z)` (`0x00741B30`) | vertex out of bounds → false; `altitude < seaLevel` |
| 53 | `NumberOfWaterVertices(rect*)` | counts `depth > 0` in an inclusive rectangle *[decomp only]* |
| 55 | `GetSedimentValue(x, z)` | the sediment field |

The boolean slots return their result in AL only (the upper bytes of EAX are not cleared on every path); callers test
AL. A cell can be both "has water" and "has land" (a shore cell).

---

## 6. Height setters and the water bookkeeping *[compat]*

Every setter skips a **locked** vertex (flag `0x4000`, §7). The clamps use the tuning (min 20, max 2500).

| Function | Rule |
|---|---|
| `RaiseVertex(i, d)` (`0x00744CA0`) | `alt := r24(alt + d)`; if `alt > max`: `alt := max`. `depth := r24(sea − alt)` |
| `LowerVertex(i, d)` (`0x00744AF0`, exported as `RecomputeWaterLevel`) | `alt := r24(alt − d)`; if `alt < min`: `alt := min`. `depth := r24(sea − alt)` |
| `SetAltitudeAtVertex(i, h)` (slot 50, `0x00744E60`) | `level := depth + alt`; `alt := h`. If the old depth ≤ 0 (land): if `sea ≤ h`, **stop** (the depth keeps its old value); else `level := sea`. Then `depth := level − h`. No clamp |
| `SetAltitude(x, z, h)` (slot 51, `0x00744F90`) | `i := (cellsX + 1) × z + x` (no bounds check). If `alt < h` and the vertex is not locked: `alt := r24(r24(h − alt) + alt)`, clamp to max, `depth := sea − alt`. If `alt > h`: `LowerVertex(i, alt − h)`. Equal or NaN: nothing |
| `SetAltitude(rect*, h, redisplay, data4)` (slot 49, `0x00744F10`) | `SetAltitudeAtVertex` for every vertex of the inclusive rectangle, z outer, x inner. If `redisplay`: first **reset the water** of the rectangle (`depth := sea − alt` where `alt < sea`, else `depth := 0`; `0x00744CF0`), then `RedisplayTerrain(1, 1, rect, data4)` |
| `LockAltitude(rect*, lock)` (slot 52) | sets or clears `0x4000` on every vertex of the inclusive rectangle |
| `SetSeaLevel(h)` (slot 45, `0x00743550`) | accepted only if `−10 ≤ h ≤ 200` (tuning `0x48CD7B70`/`71`). Sets `+0x7C`, resets every vertex's water to `depth := sea − alt` (other water fields := 0), posts `0x89171496`, then `RedisplayTerrain(1, 1, whole city, 0)` |
| `SetTerrainType(t)` (slot 41, `0x00749A00`) | if `t` differs: store it, post `0xC922EB91` with data1 = `t`, then `RedisplayTerrain(0, 0, whole city, 0)` (only the texture message, §8.1) |

*[quirk]* `SetAltitudeAtVertex` on a land vertex raised to or above the sea leaves the stale (negative) depth, so the
vertex stays land; lowered below the sea it becomes sea water at the sea level. A water vertex keeps its water
**level** and only its depth changes (it becomes land when raised above that level).

*[quirk]* `RaiseVertex` and `LowerVertex` overwrite the depth with `sea − alt` even when `d = 0`, so a lake vertex
(water above the sea) touched by either loses its lake water. The lava does this (§11.4).

---

## 7. Vertex flags (`+0x22`, uint16) *[compat]*

| Bit | Meaning |
|---|---|
| `0x0008` | cliff mark (render; set by the cliff texturing) |
| `0x0010` | shore or cliff render mark (seen in saves) |
| `0x0020` | even triangle hidden (cell flag 1) |
| `0x0040` | odd triangle hidden (cell flag 2); `0x20 | 0x40` = the whole cell hidden |
| `0x0200` | cell flag 4 |
| `0x0400` | mark set by brush component kind 7 (§9.4) |
| `0x0800` | triangulation flipped because of a cliff |
| `0x1000` | internal triangulation flip (slot 93) |
| `0x2000` | external triangulation flip (slot 94); also cell flag 8 |
| `0x4000` | **locked**: every height setter skips the vertex (slot 52) |

`IsTriangulationFlipped(x, z)` (slot 70) tests `0x3000`. `GetTerrainCellFlags` / `SetTerrainCellFlags` (slots 82,
81) pack the cell flags 1, 2, 4, 8 as the bits `0x20`, `0x40`, `0x200`, `0x2000` of the cell's `(x, z)` vertex. The
hide slots 71–77 work on the same bits. Values seen in the corpus: `0x0`, `0x8`, `0x10`, `0x18`, `0x200`, `0x208`,
`0x260`, `0x268`, `0x1800`, `0x1808`, `0x1810`, `0x1818`, `0x2000`; never `0x4000` (locks are transient).

---

## 8. `RedisplayTerrain` and the change messages

### 8.1 `RedisplayTerrain(terrainFlag, waterFlag, rect*, data4)` (slot 68, `0x00749850`) *[compat]*

`rect` is an **inclusive vertex rectangle** `(x0, z0, x1, z1)` (left, top, right, bottom).

1. If the view-ready flag (`+0xEC`) is clear: return. Nothing is posted.
2. If `terrainFlag`: recompute the cached minimum and maximum (§4.6), recompute the normals and cliff marks in the
   rectangle grown by 1 (render), and post **`0x278E6003`** (terrain heights changed) with
   `data1 = (x0 << 16) + x1`, `data2 = (z0 << 16) + z1`, `data3 = 0`, `data4 = data4`.
3. If `waterFlag`: post **`0x678E6E3A`** (water heights changed) with the same four values.
4. Always (when the view is ready): post `0x870BC918` (texture redisplay; data1/data2 pack a render rectangle, data3
   the minimum and maximum altitude truncated to integers). Only the renderer reads it.

All three are posted with `MessagePost(msg, false)` (queued, normal priority, through the helper `0x00743380`), in
this order. Receivers see them on the next message pump, never during the edit.

Slot 69 (`0x00743460`) takes two cell corners `(a, b)`, sorts them, sets `x1 := max + 1` clamped to `cellsX` (and
the same for z), and calls slot 68. It converts a cell rectangle to its vertex rectangle.

**Unpacking.** The consumers recover the **cells** touched by the vertex rectangle as `x0 − 1 … x1` and
`z0 − 1 … z1`, clamped to the city (building-development.md §4.11, network-manager.md §7.6, landfill.md §3.2,
flora.md, power.md, neighbors.md, traffic-network.md).

### 8.2 The `data4` kinds *[compat]*

`data4` is only `RedisplayTerrain`'s fourth argument. Every call site of slots 68 and 69 in the executable, by kind:

| `data4` | Who | Where |
|---|---|---|
| 0 | internal and non-height changes: `SetSeaLevel`, `SetTerrainType` (texture only), `CreateFlatAreas`, `CreateShoreCliffs`, `StopAccumulatingTextureUpdates` and the volcanic-ash ticks (texture only), the debug flood (`DoFloodFlowStep`, water only), the lot configuration editor | terrain `0x00743550`, `0x00749A00`, `0x0074A220`, `0x0074C0F0`, `0x00749A70`, `0x00749AF0`, `0x007467F0` |
| 1 | the terraforming tool in **god** mode (scrubber flag set, §9.6); `ChangeTerrainLevel` (`0x00746440`); the edge reconciliation with the neighbours (`LockEdgeVerticesAndSmoothen` `0x00748C90`); a view message `0x470BD214` (whole city, §12.1) | terrain, terrain scrubber |
| 2 | the terraforming tool in **mayor** mode (cost charged per application, §9.6) *[inferred: the mode names]*; the script / cheat command `ApplyTerrainBrush` (§9.5) | terrain scrubber, terrain `0x0074A470` |
| 3 | disasters and water: the earthquake (`0x00575C40`, after the fault brushes), the pipe-burst sinkhole (`0x0057C1A0`), every effect brush (volcano deformation, §9.5), the lava's solidification (`0x0075F4A0`), the surface water's animation (`AnimateWater` `0x0073FDD0`, **water flag only**), and any `tBrushParameters` left at its default (`0x004D8B90` sets 3) | §9.5, §11 |
| 4 | the network height constraint solver (`0x00644530`): road and rail grading. It sets each vertex whose change exceeds 0.001 m (slot 50), grows the rectangle by 1, and wraps `RedisplayTerrain(1, 1, rect, 4)` in `BeginLongTerrainOperation` / `EndLongTerrainOperation` | network tool |
| 5 | levelling: demolition `ModifyTerrainHeight` with reason 5 (`0x00467BB0`), the lot developer's area levelling through it, and the landfill developer (`HandleZoneOverlap` `0x006B96E0`) | demolition.md §4.7, landfill.md §4.5.3 |
| 6 | `cSC4NetworkTool::InsertTunnelPiece` (`0x00628390`): the tunnel cut | network tool |
| any | demolition `ModifyTerrainHeight` passes its caller's reason through | demolition.md §4.7 |

So the network manager's rule (network-manager.md §7.6) reads: kind 3 = a disaster or water change (pipes and
subways are destroyed too), kind 4 = its own grading (nothing is destroyed), kind 5 = levelling (the handler does
nothing), kind 6 = a tunnel cut (pieces with flag `0x80000` survive). The power simulator ignores 4 and 5, and the
landfill reacts only to 1, 2 and 3.

### 8.3 The long-operation messages *[compat]*

`BeginLongTerrainOperation` (slot 87, `0x00743B40`) posts `0xEA3B0AF1` and `EndLongTerrainOperation` (slot 88,
`0x007443A0`) posts `0x6A3B0AFD`, both with data1..data4 = 0, queued. There is **no nesting counter**: each call
posts one message, and the receivers (the network manager's `longTerrainOp`, network-manager.md §7.6) keep their own
state. The earthquake and the network grading use them.

### 8.4 Other terrain messages

| Message | Posted by | Data |
|---|---|---|
| `0x89171496` sea level changed | `SetSeaLevel` | 0 |
| `0xC922EB91` terrain type changed | `SetTerrainType` | data1 = the new type |
| `0x6AB97795` water display state | `SetWaterSurfaceDisplayState` (slot 84) | data1 = the state |
| `0xCA63D517` TerrainReconciliation | the animation tick (§12.1) | 0 (foundations §11.6) |

---

## 9. Brushes

### 9.1 Descriptions *[compat]*

Brushes are exemplars of type `0x6534284A`, group `0x6A01FC2A`, read once at `StaticInit`
(`ReadBrushDescriptions` `0x0074FDC0`) into a global map keyed by the exemplar **instance** (the brush ID). Each has
up to four **components**; component `i` (0–3) uses the properties `0x8A020202 + 0x100·i + k`:

| `k` | Meaning |
|---|---|
| `+3` | kind, 0–11 (a description with a kind > 11 is rejected) |
| `+0` | radius, 5 floats (one per brush level) |
| `+1` | strength, 5 floats |
| `−1` | bool: orient the bitmap along the stroke direction |
| `−2` | bitmap instance (a grey image, type `0x7AB50E44`, group `0x891B0E1A`) |
| `+2` | uint8 neutral grey value |
| `+4`..`+7` | texture, angle (`× 1/360`), and render values |

Whole-brush properties: a random-walk angle step (`× π/180`; when non-zero the start angle is random in [0, 2π)),
UI and cost values (render and tool only).

The shipped brushes the simulation uses:

| ID | Name | Component |
|---|---|---|
| `0x11` | EarthquakeFault | kind 0, radius 4 (all levels), strength 0.001, neutral 128, oriented, bitmap `0x0A0B485E` |
| `0x12` | EarthquakeFaultFlipped | the same with bitmap `0xCA131C51` |
| `0x14` | PipeSinkHole | kind 0, radius 16, strength 0.01, 0.008, 0.006, 0.0045, 0.0035, neutral 255, oriented, bitmap `0x2A05BB19` |
| `0xCA313035` | VolcanoBrush | two kind-0 components |

### 9.2 `ApplyBrush(brushID, params*)` (slot 57, `0x0074F0E0`) *[compat]*

`tBrushParameters` (`0x38` bytes):

| Offset | Field | Default (`0x004D8B90`) |
|---|---|---|
| `+0x00`, `+0x04` | from x, z (metres) | — |
| `+0x08`, `+0x0C` | to x, z (metres) | — |
| `+0x10` | level index 0–4; < 0 means the terrain default (`+0xBC`) | −1 |
| `+0x14` | redisplay (byte) | 1 |
| `+0x18` | strength scale | 1.0 |
| `+0x1C` | radius scale | 1.0 |
| `+0x20` | `data4` for `RedisplayTerrain` | 3 |
| `+0x24..+0x30` | out: the touched vertex rectangle | — |
| `+0x34` | target height (levelling components) | 100.0 |

1. Set the out rectangle to empty.
2. Look up `brushID`. Not found: return false.
3. Apply the components (§9.3).
4. If `redisplay` and the out rectangle is not empty: `RedisplayTerrain(1, 1, out, data4)`.
5. Return true.

### 9.3 Applying the components (`ApplyBrushComponents` `0x0074D730`) *[compat]*

1. The centre vertex is `vx := trunc(r24(to.x × 0.0625))`, `vz := trunc(r24(to.z × 0.0625))` (truncation, `_ftol`).
   If `vx ≥ vertsX` or `vz ≥ vertsZ` (unsigned): nothing is applied.
2. The **angle**:
   - oriented components: `angle := atan2(to.z − from.z, to.x − from.x) + π/2`, kept in the description. When
     `from = to`, the previous angle of that description is reused *[quirk: global state shared by every caller]*;
   - otherwise, with a random-walk step: `angle += step`, or `−= step` when `RandomUint32Uniform(5) = 0`, wrapped to
     [0, 2π);
   - otherwise 0.
3. For each component: `strength := strength[level] × params.strength`, `radius := radius[level] × params.radius`,
   and the kind's operation runs on the bitmap footprint (§9.4).

**Bitmap footprint** (`0x00742180`). With `W × H` the bitmap size, `k := 16 / (radius / W)`, `c := cos(angle)`,
`s := sin(angle)`:
- the footprint is a `radius × (H·radius / W)` metre rectangle centred on `(16vx, 16vz)`, rotated by the angle; its
  vertex rows and column spans come from `ComputeSpansForRectangle` (`0x00740BB0`);
- for each row `z` and the first vertex `x0` of its span: `U := s·(dz·k) + c·(dx·k) + W/2`,
  `V := c·(dz·k) − s·(dx·k) + H/2` with `dz = z − vz`, `dx = x0 − vx`; then per step in x, `U += c·k`, `V −= s·k`;
- the pixel is `(trunc(U), trunc(V))`. If `u < W` and `v < H` (unsigned) the operation runs with that grey value `b`;
  when it reports a change, the vertex joins the out rectangle.

*[quirk]* Truncation maps U in (−1, 0) to column 0, so the footprint reaches half a pixel further on the low side.

### 9.4 Component kinds *[compat for 0, 2, 5, 7; others summarised]*

| Kind | Operation |
|---|---|
| 0 | raise or lower (`0x00745C60`): `b > neutral` → `RaiseVertex(i, (b − neutral) × strength)`; `b < neutral` → `LowerVertex(i, (neutral − b) × strength)`; equal → no change |
| 1 | flora painting |
| 2 | smoothing (`0x00745DE0`; strength capped at 1.0; the out rectangle grows by 1) |
| 3 | (`0x00749130`) |
| 4 | animals (render) |
| 5 | level toward the target height `+0x34` (`0x00745BA0`) |
| 6 | water erosion |
| 7 | mark: sets flag `0x0400` (`0x00740820`) |
| 8 | lake (adds water) |
| 9 | rain (surface water) |
| 10 | spring (surface water) |

Kinds 1, 3, 4, 6, 8–11 are used only by the terraforming tool's god-mode brushes, which a harness does not drive.

### 9.5 Who applies brushes

- **Earthquake** (disasters.md §7, `StartDeformation` `0x00575C40`): level 0, redisplay off, `data4 = 3`. It calls
  `BeginLongTerrainOperation`, then, per 8 m step along the fault, `ApplyBrush(0x11, left)` and
  `ApplyBrush(0x12, right)`, and at the end `StopAccumulatingTextureUpdates` and
  `RedisplayTerrain(1, 1, leftRect, 3)`, `RedisplayTerrain(1, 1, rightRect, 3)`.
- **Pipe-burst sinkhole** (`0x0057C1A0`): from = to = the cell's north-west corner `(16x, 16z)`, level 0, redisplay
  on, strength 1, radius 1, `data4 = 3`, target 100 → `ApplyBrush(0x14)`: a 16 m footprint lowered by up to
  `(255 − b) × 0.01` m.
- **Effect brushes** (the effects manager's brush effect, `0x00581640`; [effects.md](effects.md) §8): the volcano's
  deformation is the effect `"volcano_control"` started by the volcano instance (`StartDeformation` `0x0057F4F0`).
  Each update applies the effect's brush `count` times from the previous to the current position, both inside the
  city (city slot 98), with the level from the effect, strength and radius drawn once from the effect's ranges,
  `data4 = 3`, the effect's target height, and **redisplay only on the last repetition**. The shipped values
  (effects.md §8.4, TR2): `volcano_control` picks one of nine creators (a select group, effects.md §5.3); each uses
  VolcanoBrush `0xCA313035` at level 1, strength 1.0, a fixed radius scale of 0.5–1.3 and target 100, as a stroke of
  30–130 applications per effect-second for 2–4 effect-seconds (`count` = the whole part of the accumulated rate,
  effects.md §8.3). Meteor craters are one-shot: CraterBrush `0x8A33E17A` applied twice in a single update (level 3,
  strength 2–6, radius 0.5–1.0).
- **Terraforming tool** (§9.6) and the cheat / Lua command `ApplyTerrainBrush` (`0x0C4A6AF6`): level −1, redisplay
  on, arguments (brushID, x0, z0, x1, z1[, strength, radius]) with both points required inside the city (slot 11),
  from = (x0, z0), to = (x1, z1), strength and radius default 1.0, `data4 = 2`, target height = `GetAltitude(x1,
  z1)` (checked in the bytes at `0x0074A933`–`0x0074AAB7`).

### 9.6 The terraforming tool

The tool (`cSC4ViewInputControlTerrainScrubber`) wraps strokes in `BeginBrushScrub` / `EndBrushScrub` (slots 59, 60)
and calls `ApplyBrush` per application. In god mode it sets `data4 = 1`; in mayor mode `data4 = 2` and each
application is charged `GetCostPerBrushApplication` (slot 62) against the budget. `EndBrushScrub` makes the surface
water rebuild its bodies of water (§10.3).

---

## 10. Surface water (`cSTESurfaceWater`)

### 10.1 State and life cycle *[compat]*

The water array (§2.2) is rebuilt, not loaded: the surface water `Init` (`0x0073FE70`) and the terrain `Create`
call `ResetWaterToSeaLevel`, which sets every vertex's `depth := sea − alt` and the other water fields to 0. Lakes
made by the god-mode brushes are therefore lost on reload (§13.3).

`Init` subscribes to `0x26D31EC1` (PostCityInit), `0x26D31EC4`, `0x26D31EC5`, `0x89D3E9F3` (network occupant
changed), `0x99EF1142`, `0x99EF1143` (occupant inserted, removed), and registers with the animation tick manager at
priority `0xC0`.

### 10.2 Slots *[compat]*

| Slot | Function | Rule |
|---|---|---|
| 5 | `GetWaterDepthAtVertex(i)` | the depth |
| 6 | `GetWaterDepthForCell(x, z)` (`0x0073D210`) | `i := terrain.GetVertIndex(x, z)` (no bounds check). `m :=` the minimum of the depths at `i`, `i + 1`, `i + 1 + vertsX`, `i + vertsX`, in that order, a NaN candidate never replacing the minimum. Return `m` if `m > 0`, else 0.0 |
| 13 | `HasDraftClearance(x, z)` (`0x0073CDB0`) | `GetWaterDepthForCell(x, z) > 5.0` |
| 8 | `IsNavigable(x, z, r, lot)` (`0x0073EBF0`) | §10.4 |
| 7 | `IsNavigable(position…)` (5 arguments) | converts the position to a cell (city slot 95) and calls slot 8 |
| 9 | `GetBodyOfWaterMap()` | the uint16 grid at `+0x4C` (SimGrid type `0x49B9E600`, ID `0x4BE3618B`), one value per cell, 0 = none |
| 10 | `AltitudeAtWhichFloodingWillOccur(i, start)` | the maximum of `start` and the water levels of the 8 neighbours with `depth > 0` *[decomp only]* |
| 11 | `GetWaterVelocity(x, z)` | a pointer to the vertex's velocity |
| 12 | `GetWaterType(x, z)` (`0x0073D7F0`) | vertex `i := GetVertIndex(x, z)`. `depth ≤ 0` → **0**; else `level := depth + alt`: `level > sea` (or NaN) → **2** (fresh water above the sea), otherwise **3** (sea water) |

### 10.3 Bodies of water *[compat]*

`UpdateBodiesOfWater` clears the map, then scans the cells with **x outer, z inner**. Each unlabelled cell that is
navigable (`IsNavigable(x, z, 0, 0)`) gets the next ID (1, 2, …) and a FIFO flood fill over the 4-neighbours, pushed
in the order west, east, north, south, labels every navigable cell it reaches.

It runs:
- on `0x26D31EC1` (only if the city is established, city slot 22) and on `0x26D31EC4`, after
  `UpdateNeighborConnections`;
- on SimNewDay `0x66956814` when the dirty flag (`+0x48`) is set;
- on `0x89D3E9F3` for a bridge piece (network flag `0x40000`) with no adjacent bridge piece;
- at `EndBrushScrub` when the city is established.

A power pole inserted or removed (`0x99EF1142` / `0x99EF1143`, pole slot 13 false) on a cell with `depth > 0` sets
the dirty flag.

### 10.4 `IsNavigable(x, z, r, lot)` (slot 8) *[compat]*

Visit the cells of the square `[x − r, x + r] × [z − r, z + r]` with the spiral iterator (from the centre outward;
when `max < min` the maximum is set to the minimum). For each cell:
1. Out of the city (city slot 99): return false.
2. Not `HasDraftClearance`: return false.
3. If `lot ≠ 0`: the lot at the cell centre (city slot 97, y = sea level; lot manager slot 5) must be null or `lot`.
4. If the traffic network map has a tile at the cell (`FindNetworkTile(x, z, 0x1F4F, 1)`) with network flag
   `0x40000` (**bridge**): if the tile's deck height (tile slot 67) is known and
   `deckHeight − (alt + depth at the cell's north-west vertex) ≥ 20.0`, return **true at once**; otherwise return
   false.
5. Otherwise, if the occupant manager has a power pole (type `0x2890D4DE`) whose cell is `(x, z)`: return false.

If every cell passes: true.

*[quirk]* Step 4 returns true without checking the cells still to be visited, so a high bridge anywhere in the square
makes the whole square navigable from that point of the spiral.

Consumers: traffic-network.md §4.10 (ferries and watercraft), zones.md (seaport and waterfront tests), the city
situation manager (`IsNavigable(r = 2)`, `GetWaterDepthForCell`), the watercraft automata.

---

## 11. Volcano lava (`cSTEVolcanoLava`)

### 11.1 Tuning *[compat]*

Exemplar `0x6534284A / 0x88CD66E9 / 5` ("Lava Properties"), read once (`0x0075DCE0`). The ones the flow uses:

| Property | Use | Shipped |
|---|---|---|
| `0x69987E61` | maximum flow rate, × 1e-6 per microsecond | 4 → 4e-6 |
| `0x69987E62` | level difference for the full rate | 12 |
| `0x69987E63` | solidified lava → terrain height factor | **0** |
| `0x69987E68` | solidification multiplier over water | 2 |
| `0x69987E69` | ignition level (`IgniteCell`) | 1.5 |
| `0x69987E6A` | burn level (`QuickBurnCell`) | 3 |
| `0x69987E6B` | animation frame period (µs; render) | 200000 |
| `0x69987E6C` | solidification threshold | 3 |

The others (`0x69987E60`, `64`–`67`, `6D`–`6F`) are render values (decals, steam).

### 11.2 Start *[compat]*

The volcano instance (disasters.md §10) creates the lava in state 3 (`InitializeVolcanoLava` `0x0057F360`, only when
`0x6A4BC8ED` is true), with production and solidification rates 0. In state 5 (`0x0057F3D0`) it starts the flow at
its location `(x, z)` (metres, clamped to `[0, size − 1]`) with radius `0x0A4BC752` (20 m) (`0x0075F8C0`):

- the source vertex `(floor(x / 16), floor(z / 16))`;
- the **source rectangle**: the vertex square of half-width `trunc(20 / 16) = 1` around it, clamped to
  `[0, cellsX] × [0, cellsZ]` (so 3 × 3 vertices);
- the active rectangle := the source rectangle;
- the burn rectangle: half-width 7 around the source vertex, clipped to `[0, cellsX − 1] × [0, cellsZ − 1]`, with a
  random iterator over it;
- it registers with the animation tick manager (priority `0xC0`).

Each volcano tick sets the production rate and the solidification rate (disasters.md §10). The lava level array
(one float per vertex) starts at 0. The lava is **not saved** (saving is disabled during a disaster).

### 11.3 The flow step (`DoLavaFlow(dt)` `0x0075F4A0`, every animation tick, dt in µs) *[compat]*

1. `solid := dt × solidificationRate`, `solidWater := solid × 2`; `solidifying := solid > 0`; then clamp both ≤ 1.
2. Every source vertex: `lava += dt × productionRate` (the active rectangle grows to include it).
3. For each vertex of the active rectangle (z outer, x inner) with `lava > 0`:
   1. **Flow.** On the city edge (`x = 0`, `x = cellsX`, `z = 0` or `z = cellsZ`): `amount := dt × 1e-7 × lava`,
      and it leaves the map. Otherwise find the neighbour with the lowest surface (`alt + lava`):
      `GetNeighborWithLowestLavaLevel` (`0x0075DA90`) takes the best drop among the 4 edge neighbours, then lets a
      diagonal neighbour win only when its drop exceeds `√2 ×` that best drop. No neighbour lower: no flow. Else
      `h := (surface − neighbourSurface) × 0.5`; `rate := h < 12 ? (4e-6 / 12) × h : 4e-6`; `h := min(h, lava)`;
      `amount := min(dt × h × rate, h)`.
   2. If `amount > 0`: if there is a neighbour, `SetThingsOnFire(neighbour, its old lava, amount)` (§11.5) and
      `neighbour.lava += amount`; then `lava −= amount`.
   3. **Solidification**, when `solidifying`: if `lava ≥ 3`: `f := solidWater` if the vertex has water
      (`depth > 0`), else `solid`; `lava −= f × lava`; `RaiseVertex(vertex, 0 × f × oldLava)`; the vertex joins the
      touched rectangle. Else if `lava ≥ 0.1`: `lava × 0.9`. Else `lava := 0`.
4. Grow the active rectangle by 1.
5. If the touched rectangle is not empty: `RedisplayTerrain(1, 1, touched, 3)` (texture-accumulation calls around it
   are render-only).

*[quirk]* With the shipped factor 0 the lava never raises the ground, but every solidifying vertex still gets
`RaiseVertex(…, 0)`, which rewrites its water depth to `sea − alt` (§6) and posts `0x278E6003` with kind 3 for the
touched rectangle every tick. So the network manager re-checks the networks under solidifying lava every tick.

### 11.4 Burning *[compat]*

While the production rate is positive, `BurnSurroundingStuff` (`0x0075F340`) draws `(7 × 7) / 6 = 8` cells per tick
from the burn rectangle's random iterator (an exhausted iterator is reset, and that draw is lost), and for each cell
that the flammability simulator calls flammable (slot 6, `IsCellFlammable`) calls the fire manager's
`IgniteCell(x, z, false, true, 0)` (fire interface slot 5).

### 11.5 `SetThingsOnFire(vertex, oldLava, amount)` (`0x0075DE60`) *[compat]*

- If `oldLava < 1.5 ≤ oldLava + amount`: mode **ignite**.
- Else if `oldLava < 3 ≤ oldLava + amount`: mode **burn**.
- Else nothing.

For the four cells around the vertex, `(x − 1 … x) × (z − 1 … z)`, that are flammable: ignite →
`IgniteCell(cx, cz, false, true, 0)`, burn → `QuickBurnCell(cx, cz)`; then `SetCause(3)` (fire interface slots 5, 7
and 9; disasters.md §5).

### 11.6 Messages

The lava subscribes to `0x26D31EC2` (PreCityShutdown → `Shutdown`) and `0x69E65264` (steam effect ended, render),
and handles the animation tick `0x86AD10EE` (→ `DoAnimationTick`: render frame, `DoLavaFlow`, burning, decals).

---

## 12. Messages and tick work

### 12.1 Terrain (`DoMessage` `0x0074A470`; foundations §11.6 cites an address inside it, `0x0074A590`) *[compat]*

| Message | Action |
|---|---|
| `0x26D31EC1` PostCityInit | `DoPostCityInit` (slot 40): creates the view and sets the view-ready flag |
| `0x86AD10EE` animation tick | `DoAnimationTick(dt)` (`0x0074A390`), below |
| `0x2A612643` (to the terrain itself) | the deferred edge reconciliation, foundations §11.6 (re-posted while a modal window runs, else `ReconcileEdgesWithNeighboringCities(0, 1, 1)`) |
| `0x470BD214` (from the 3D view window) | refresh the textures, then `RedisplayTerrain(1, 1, whole city, 1)` |
| `0xA6B79602` view update | render only |
| `0x66956816`, `0x6716A4E6` | subscribed, nothing |
| `0x0C4A6AF0`..`0x0C4A6AF6` | the script and cheat commands `GetAltitude`, `SetAltitude`, `GetCellAltitude`, `SetCellAltitude`, `GetSeaLevel`, `SetSeaLevel`, `ApplyTerrainBrush` |

`DoAnimationTick`:
1. If the view exists: count ticks up to 2; on the second tick post `0xCA63D517` (TerrainReconciliation) when the
   game preference `+0x4EC` is 0, else post `0x2A612643` to itself (foundations §11.6 has the full rule).
2. If the debug flood is on: one flood step.
3. Volcanic-ash regions (render).
4. Every 10 s of animation time (more than 9,999,999 µs accumulated), recompute the water proximity values of a
   pending rectangle (render).

### 12.2 Surface water

Animation tick: animate the water (the flood flow) and `RedisplayTerrain(0, 1, rect, 3)` for changed water. Messages:
§10.1, §10.3.

---

## 13. Saved records

### 13.1 Terrain records *[compat]*

`Save` (slot 97, `0x007477C0`) writes, in order, the misc record, the altitudes, the vertex properties and the four
edges. `Load` (slot 96, `0x00747800`) reads misc, altitudes, properties (success flag at `+0x68`); the edges are read
on demand by the region code.

| Record (type / group / instance) | Version | Payload |
|---|---|---|
| `0x8A91E7E3 / 0xE98F9525 / 3` misc | u16 2 | u32 `vertsX`; u32 `vertsZ`; i32 terrain type (`+0x18`; a version < 2 with a value ≥ 1 loads as 0); f32 cached minimum; f32 cached maximum; f32 `+0x78`. 26 bytes |
| `0xA9DD6FF4 / 0xE98F9525 / 1` altitudes | u16 2 | `vertsX × vertsZ` f32, row-major (index order) |
| `0x8A91E7E0 / 0xE98F9525 / 2` vertex properties | u16 1 | one u16 flag word per vertex (§7) |
| `0xA9DD6FF4 / 0xE98F9525 / 0x64 + side` edges | u16 1 | u32 `n`, then `n` f32: side 0 = the column `x = 0` (`n = vertsZ`), 1 = the row `z = 0` (`n = vertsX`), 2 = the column `x = cellsX`, 3 = the row `z = cellsZ` |

The loader takes `vertsX` and `vertsZ` from the record and sets the vertex count from them; it does not check them
against the city size. Corpus: 129 × 129 (302 saves), 257 × 257 (11), 65 × 65 (189); terrain type 0 and `+0x78` =
308 in every save.

### 13.2 Class record

The terrain's serializable record (type `0xE98F9525`, group `0x299B2D1B`): u16 version 2 (`Read` accepts ≤ 2) and
a reference to the terrain view (class `0xC9B84E10`, whose own record is u16 1 + one byte, 0 or 1 in the corpus;
render only). This settles the "unnamed `0xC9B84E10` (3 bytes)" of save-records.md §3.2.

### 13.3 Surface water

The surface water (CLSID `0x0B8DA7DB`) writes u16 1 and a reference (read with IID `0xC70BD37E`, the terrain).
`SaveWaterInfo` (record `0x8A91E7EF / 0xE98F9525 / 4`, u16 1, u32 count, per vertex 4 f32 + a vector + 1 f32)
exists in the code, but **no corpus save holds that record**: the water is rebuilt from the sea level on every load
(§10.1). *[compat]* A replacement must not save lakes either.

---

## 14. Nation

What `cSC4Nation` adds beyond neighbors.md and the economy modifier: see [nation.md](nation.md). In short, nothing
the simulation reads except `GetEconomyModifier` (demand.md §8.2, with two extra quirks in nation.md).

---

## 15. Slot tables *[compat]*

### 15.1 `cISTETerrain` (vtable `0x00AB3D18`)

Return channels and callee pops are in docs/abi/interfaces.tsv. Names that the export folded or got wrong are
corrected here (and proposed for that table).

| Slot | Function | Notes |
|---|---|---|
| 0–4 | QueryInterface, AddRef, Release, Init, Shutdown | |
| 5 | `IsInitialized()` | byte `+0x0C` (exported as a folded `IsInDelayedStop`) |
| 6 | `CellWidth()` | 16.0 |
| 7, 8 | `GetCellCountX()`, `GetCellCountZ()` | `+0x28`, `+0x2C` (exported as folded getters) |
| 9, 10 | `GetVertIndex`, `GetVertex` | §2.1 |
| 11 | `LocationIsInBounds` | §3 |
| 12–14, 17, 19–24 | height reads | §4 |
| 15, 16 | `GetAltitudes`, `GetWaterDepths` (rect copies) | |
| 18 | `GetMaxSurfaceLevel` | §5 |
| 25 | `GetTerrainGridIndex` | |
| 26–28 | normals | §4.3 |
| 29–38, 53, 55, 92 | water reads | §5 |
| 39 | `AreSeaVertices(rect*)` | |
| 40 | `DoPostCityInit` | §12.1 |
| 41 | `SetTerrainType` | §6 |
| 42 | `GetTerrainType()` | `+0x18` (exported as a folded `GetTractSize`) |
| 43, 44 | `CreateFlatAreas`, `CreateShoreCliffs` | data4 0 |
| 45, 46 | `SetSeaLevel`, `ChangeSeaLevel` | §6 |
| 47 | `GetDefaultTerrainRaiseLowerAmount` | 20.0 |
| 48 | `ChangeTerrainLevel` | data4 1 |
| 49–52 | setters, lock | §6 |
| 54 | returns `+0xE4` | render object (exported as a folded `GetSelectedCity`) |
| 56 | `StartFlood` | debug: +500 m of water on vertices 20..39 × 20..39 |
| 57 | `ApplyBrush` | §9.2 |
| 58–63 | brush and scrub helpers | §9.6 |
| 64–67 | waves, view, lighting | render |
| 68, 69 | `RedisplayTerrain` | §8.1 |
| 70–77, 81, 82 | flags, hiding | §7 |
| 78–80 | texture accumulation | render |
| 83 | `GetWaterSurfaceDisplayState()` | byte `+0xDD` (exported as a folded `IsPageFlippingEnabled`) |
| 84 | `SetWaterSurfaceDisplayState` | §8.4 |
| 85, 86 | plumbing display state | a global byte |
| 87, 88 | `Begin` / `EndLongTerrainOperation` | §8.3 |
| 89 | `ReconcileEdgesWithNeighboringCities` | foundations §11.6 |
| 90, 95 | `SaveObliterated`, `LoadObliterated` | |
| 91 | deleting destructor | |
| 93, 94 | `FlipInternalTriangulation`, `FlipExternalTriangulation` | |
| 96, 97 | `Load`, `Save` | §13 |

### 15.2 `cISTESurfaceWater` (vtable `0x00AB3B00`)

0 QueryInterface, 1 AddRef, 2 Release, 3 Init, 4 Shutdown, 5–13 as §10.2 (slot 9 is exported as a folded
`GetTransientAuraGrid`; it returns the body-of-water map at `+0x4C`), 14 deleting destructor.

### 15.3 Consumers' slots (for the implementer)

BDS 7, 8, 13; flora 5, 13, 26, 29; landfill 13, 19, 29, 49, 68; land value 12, 13, 23, 24, 31, 36; lot developer
6, 7, 8, 12, 13, 30, 31, 34, 37; demolition 14, 37, 49, 51, 68; network manager 13; pollution 13; protection 13, 29;
plumbing 7, 8, 34, 68, 69; traffic network 34; disasters 57, 69, 79, 87, 88; surface water 6, 8, 9, 12, 13 (traffic
network, zones, city situation, watercraft).

---

## 16. Quirks (summary)

1. `GetAltitude` ignores the triangulation flips (§4.1).
2. The cached minimum and maximum go stale and are saved stale (§4.6).
3. `RedisplayTerrain` posts nothing before `DoPostCityInit` (§8.1).
4. `SetAltitudeAtVertex` leaves a stale depth on land raised above the sea (§6).
5. `RaiseVertex`/`LowerVertex` erase lake water (§6); the lava does it with a zero raise every tick (§11.3).
6. The brush angle of an oriented brush is shared global state when `from = to` (§9.3).
7. Bitmap truncation reaches half a pixel further on the low side (§9.3).
8. `IsNavigable` accepts the rest of the square at the first high bridge (§10.4).
9. Lakes are not saved (§13.3).
10. The boolean slots return in AL only (§5).

---

## 17. Open questions

- **TR1.** The exact component operations of kinds 2, 3, 5, 6 and 8–10 (§9.4). Only the god-mode tool uses them; the
  harness does not. A trace of `0x00745DE0` and `0x00745BA0` settles 2 and 5.
- ~~**TR2.** The `volcano_control` effect's brush values (level, strength and radius ranges, repetition count). They
  live in the effects resource; the effects manager spec (INVENTORY rank 7) settles them.~~ **ANSWERED 2026-10-15
  (re-specs16):** effects.md §8.4 and §9.5 above: level 1, strength 1.0 (range 1–1), radius scale 0.5, 0.5, 0.6,
  0.6, 0.8, 0.8, 1.0, 1.0 or 1.3 (one of nine creators, ranges degenerate), rate 30–130 per effect-second for
  2–4 effect-seconds; `count` per update = `floor` of the accumulated `rate × dt`.
- **TR3.** `GetLerpedWaterLevel` (slot 32) and `AltitudeAtWhichFloodingWillOccur` are from pseudocode only; no
  simulation consumer was found. Check against the bytes if one appears.

---

## 18. Conformance tests (must exist in `engine/conformance`)

1. **GetAltitude** (§4.1, §4.4): a 2 × 2-cell terrain with the §4.4 heights; the five queries of the table give 109,
   107, 110, 100 and 0. `GetAltitude(32, 32)` on a 2 × 2 city returns the vertex (2, 2).
2. **Floor at cell boundaries**: `x = 16.0` uses `ix = 1`; `x = 15.999` uses `ix = 0`; `x = sizeX` uses the last
   cell with `fx = 16`.
3. **GetFaceNormal** (§4.3): the §4.4 cell at (12, 4) gives `(−64, 256, −256)·s`; at (4, 12) gives
   `(−16·12, 256, −16·8)·s = (−192, 256, −128)·s`; `x = sizeX` leaves the output untouched.
4. **CellHasWater / CellHasLand**: depths (−1, −1, −1, 0.5) at the four corners → both true; (0, 0, 0, 0) → water
   false, land true; a NaN corner with the rest −1 → water false, land true.
5. **GetWaterDepthForCell**: depths (2, 3, 0.5, 4) → 0.5; (2, −1, 3, 3) → 0.0; `HasDraftClearance` with (6, 6, 6,
   5) → false (5.0 is not > 5.0).
6. **GetWaterType**: depth −2 → 0; depth 3 with altitude 247 and sea 250 → 3 (level 250 = sea); depth 3 with
   altitude 260 → 2.
7. **SetAltitudeAtVertex** (§6): sea 250, a land vertex at 260 with depth −10 set to 270 → depth stays −10; set to
   240 → depth 10; a water vertex alt 240 depth 10 set to 245 → depth 5; set to 255 → depth −5.
8. **SetAltitude(x, z, h)**: alt 260 → `SetAltitude(…, 3000)` clamps to 2500 and depth = −2250; alt 30 →
   `SetAltitude(…, 10)` lowers by 20 and clamps to 20; a locked vertex never changes.
9. **SetSeaLevel**: 201 and −10.5 are rejected (no message); 100 posts `0x89171496` and then `0x278E6003` with
   data4 0 over the whole city.
10. **RedisplayTerrain packing** (§8.1): rect (3, 5, 10, 12), data4 4 → `0x278E6003` with data1 `0x0003000A`,
    data2 `0x0005000C`, data3 0, data4 4, then `0x678E6E3A` with the same, then `0x870BC918`; all queued. Before
    `DoPostCityInit`: nothing.
11. **Pipe-burst sinkhole**: a flat terrain at 300, sea 250; `ApplyBrush(0x14)` at the corner of cell (10, 10) with
    the §9.5 parameters lowers no vertex below `300 − 255 × 0.01`, raises none, touches only vertices inside the
    16 m footprint centred on (160, 160) (§9.3), and posts one `0x278E6003` with data4 3 whose rectangle is the
    touched vertex rectangle.
12. **Unknown brush**: `ApplyBrush(0x12345678, p)` returns false, leaves `p`'s out rectangle empty and posts nothing.
13. **Lava flow rule** (§11.3, the per-vertex transfer in isolation): flat ground, a vertex with lava 10 whose eight
    neighbours all hold 1.4, dt = 33333 µs. All drops are 8.6, so the first edge neighbour in the offset order wins
    (a diagonal would need more than `√2 × 8.6`). `h = 4.3`, `rate = 4e-6 × 4.3 / 12 ≈ 1.4333e-6`,
    `amount ≈ 33333 × 4.3 × 1.4333e-6 ≈ 0.2054`. The neighbour goes from 1.4 to ≈ 1.6054, crossing 1.5, so its four
    cells that are flammable get `IgniteCell(…, false, true, 0)` and `SetCause(3)`. With the neighbours at 0
    instead, `amount ≈ 0.2778` and nothing is ignited. A vertex on the city edge with lava 10 loses
    `33333 × 1e-7 × 10 ≈ 0.0333` and ignites nothing.
14. **Records** (§13.1): a 64-cell city round-trips the four record kinds with the sizes 26 bytes, `2 + 65·65·4`,
    `2 + 65·65·2` and `2 + 4 + 65·4`.
