# `cSC4WinMiniMap`: the city minimap window (behaviour spec)

Scope: the whole `cSC4WinMiniMap` owner as tools/coverage.py assigns it (38 functions, 10,345 bytes on `5c0ff782`),
its nested `cSC4WinMiniMap::cTZMap` (1 function), and the free functions `GZWinIsVisibleRecursive` and
`GZCOM_cSC4WinMiniMapCLSID`. Inventory in §13. It also describes the window's constructor `0x007A8920` (attributed
to `cSC4ScriptedWindowCOMDirector`), the shared rotation-step helper `0x007A65D0`, the palette ramp
`nSC4UI::InterpolateRampToPalette` (`0x0078D520`) and the network filter `cSC4SurfaceNetworkOccupantFilter` (already
covered by the `cSC4*OccupantFilter` row); none of those owners is claimed here.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled, and the shipped tuning was read from `SimCity_1.dat` with
`tools/dbpf.py`. The two lane-arithmetic blends (§4.2, §5.3) were checked against their closed forms by emulating
the instructions on random inputs (`build-runs/lanes7/lerp_check.py`, `water_check.py`, 0 mismatches). Written
2026-10-23 (re-minimap7).

Conventions: offsets are from the window object `W` (its primary `cGZWin` base). Methods reached through the
`+0xD8` or `+0xDC` interfaces run with `this` = `W+0xD8` / `W+0xDC`; their field offsets are converted to `W` offsets
here. Globals:
- `city` `0x00B43CEC`: slots 89/90 (`+0x164`/`+0x168`) the city's world size x/z; slots 93/94 the cell counts.
- `terrain` `0x00B43CF4` (`cISTETerrain`).
- `render` `0x00B43DD0` (`cISC43DRender`): slot 16 the camera control, whose word `+0x10C` is the view rotation 0–3.
- `occupants` `0x00B43D0C`, `zones` `0x00B43D14`, `msgsrv` `0x00B43CCC`, the main window `0x00B43DC8`.

## 1. Identity and lifetime *[compat]*

- Class ID `0xCA318388` (`GZCOM_cSC4WinMiniMapCLSID` `0x007A6580`). Interfaces:

| Offset | vtable | Interface |
|---|---|---|
| `+0x00` | `0x00AB83B8` | `cGZWin` (window slots; overrides below) |
| `+0xD8` | `0x00AB838C` | the minimap interface `0xCA318385` (10 slots) |
| `+0xDC` | `0x00AB837C` | message target (slot 3 `DoMessage`) |

- `QueryInterface` (`0x007A6650`): `0xCA318385` → `W+0xD8`, AddRef'd, true; anything else → `cGZWin::QueryInterface`.
  The thunks `0x007A8A40`/`0x007A8A50` subtract `0xD8`/`0xDC`.
- The `+0xD8` slots: 0–2 QueryInterface/AddRef/Release, 3 `AsIGZWin`, 4 `SetOverlay`, 5 `SetDefaultZoneColor`,
  6 `SetDefaultZoneColorToTrans`, 7 `SetZoneColor`, 8 `SetFlags`, 9 `Update`.
- `cGZWin` slots overridden: 0 `QueryInterface`, 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 88 `GZPaint`, 134
  `GZOnMouseDownL`, 136 `GZOnMouseUpL`, 138 `GZOnMouseMove`, 141/142 `GZOnMouseEnter`/`Exit` (one body), 148 the
  deleting destructor.
- **Constructor** (`0x007A8920`):
  - `cGZWin::cGZWin`, then the three vtables;
  - the flags `+0xE0` := 7, all other fields 0;
  - the zone colour table (`+0x108`/`+0x10C`) = 16 zero words.
- **Destructor** (`0x007A8A60`; deleting `0x007A8B20` frees when bit 0 of its flag is set):
  - frees the network bits `+0x960`, the map pixels `+0x114` and the zone table `+0x108`;
  - releases the overlay `+0xF4` and the surface `+0xF0`;
  - `cGZWin::~cGZWin`.
- **`Init`** (`0x007A8BD0`, slot 4): already initialised → true. Otherwise:
  1. Get the framework service `0xC416025C` (IID `0x0073283C`). Into `+0xF0`, its slot 3 creates the drawing
     surface: false ends with false.
  2. The surface's slot 3 `(n, n, {9, 32})`, with `n` = `+0xE4`: failure releases it and returns false.
  3. When the surface locks (slot 6, `0x8001`): clear it to colour (100, 100, 127, 0) (slots 30 and 18), then unlock
     (slot 7).
  4. `cGZWin::Init`: failure calls `Shutdown` and returns false.
  5. Subscribe `+0xDC` on `msgsrv` (slot 5) to `0x89171496`, `0x678E6E3A`, `0x278E6003`, `0x07699455`,
     `0xA6B79602`, `0xA6B79612`, `0x26D31EC1`, `0x26D31EC2` and `0xC53D10AA`.
  6. Clear both palettes (`+0x160`, `+0x560`, 0x400 bytes each).
  7. Zone colours: `SetDefaultZoneColor(0xE57300)`; then `SetZoneColor` 1–3 `0x00EC00`, 4–6 `0x7979FF`, 7–9 `0xC7BC00`.
  8. `LoadTunableValues`, `OnCityInit`. True.
- **`Shutdown`** (`0x007A75D0`, slot 5), when initialised:
  - `UnhookOccupantMessages`, then unsubscribe the nine messages of `Init`;
  - the surface's slot 4, then release it and clear `+0xF0`;
  - `cGZWin::Shutdown`. Always true.

## 2. Object layout

| Offset | Field |
|---|---|
| `+0x24`, `+0x28` | (window) the origin the map is drawn at (§8) *[inferred]* |
| `+0x34`…`+0x40` | (window) its area x0, y0, x1, y1 |
| `+0x68`, `+0x6C` | (window) its draw context and its paint buffer |
| `+0xA8`…`+0xB4` | (window) a second rectangle, used for the marker (§8.3) *[inferred: the client area]* |
| `+0xE0` | flags (§10.2) |
| `+0xE4` | `n`: the map size in pixels, a power of two |
| `+0xE8`, `+0xEC` | the city's cell counts x, z (0 without a city) |
| `+0xF0` | the drawing surface (`n` × `n`, 32-bit) |
| `+0xF4` | the overlay surface, counted (§10.3) |
| `+0xF8` | float: the tick accumulator (§10.1) |
| `+0xFC` | byte: attached to a city |
| `+0xFD` | byte: the map needs rebuilding (some 16×16 block is dirty) |
| `+0xFE` | byte: the surface needs refilling |
| `+0xFF` | byte: the window needs a repaint |
| `+0x100` | byte: the left button is held (dragging) |
| `+0x101` | byte: the network messages are subscribed |
| `+0x102` | byte: the marker's blink phase |
| `+0x104` | `s`: the scale shift (§3) |
| `+0x108` | the zone colour table: 16 ARGB words (vector) |
| `+0x114`, `+0x118`, `+0x11C` | the map (`cTZMap`): pixels, width, height |
| `+0x120` | 16 words: the dirty bits of the 16×16-cell blocks (word = block z, bit = block x) |
| `+0x160` | the terrain palette, 256 ARGB words |
| `+0x560` | the water palette, 256 ARGB words |
| `+0x960` | vector of words: two network bits per cell (§6) |

## 3. City attach and size *[compat]*

- **`SetArea(x0, y0, x1, y1)`** (`0x007A8E30`, slot 55): `cGZWin::SetArea`, then `n` := the largest power of two not
  above `min(x1 − x0, y1 − y0)`. With map pixels allocated, `OnCityInit` is called, which does nothing while
  attached *[quirk]*.
- **`OnCityInit`** (`0x007A8B40`), only when not attached:
  1. Set `+0xFC`; `InitTZMap`.
  2. Keep the cell counts in `+0xE8`/`+0xEC`. Resize the network bit vector to `(2·cx·cz + 31) >> 5` words; new
     words are 0, old ones kept (§11).
  3. Register `+0xDC` with the animation tick manager (slot 5, `(target, 0)`); `HookOccupantMessages`.
- **`InitTZMap`** (`0x007A7840`), when the city has positive cell counts `cx`, `cz`:
  1. `cTZMap::Init(n, n)` (`0x007A7570`): reallocate the pixel block `n × n × 4` unless the size is unchanged.
  2. `s := 0`. While `cx > n`, halve `cx` and add 1 to `s`; while `cx < n`, double it and subtract 1. So `s` is the
     power-of-two scale from cells to pixels, from the x count only.
  3. Mark every block dirty (§5.1).
- **`OnCityShutdown`** (`0x007A77E0`), when attached:
  - `UnhookOccupantMessages`; unregister from the tick manager (slot 6);
  - free the map pixels; clear `+0xFC` and the cell counts.

  The network bits are not cleared.

## 4. Tuning and palettes *[compat]*

### 4.1 `LoadTunableValues` (`0x007A7450`)

The exemplar (`0x6534284A`, `0x690F693F`, `0x8A356F2C`, "Mini map"). When present:
- property `0xEA3573AF` → the terrain palette `+0x160`, through `InterpolateRampToPalette`; then every terrain
  entry's alpha byte is set to 0;
- property `0x6B733233` → the water palette `+0x560` (alpha kept).

Then every block is marked dirty. Shipped ramps (key, ARGB):
- terrain: (0, `FF536FD7`), (58, `FF4B83E7`), (62, `FFBBB198`), (64, `FFCDBB8D`), (68, `FF65963D`), (72, `FF70AA8E`),
  (88, `FF9A8A62`), (255, `FFFFFFFF`);
- water: (0, `006DACC4`), (32, `FF3D66B4`).

### 4.2 `InterpolateRampToPalette(out, count, ramp, words)` (`0x0078D520`, cdecl, owner `nSC4UI`)

`ramp` is `words` words: pairs (key, colour), keys rising.
1. Entries below the first key take the first colour.
2. Then for each index `i`, the segment is the pair `j` with `key_j ≤ i < key_{j+1}`. Indexes from the last key on
   take the last colour.
3. Inside a segment, `t = ((i − key_j) << 8) / (key_{j+1} − key_j)` (unsigned integer division). Each of the four
   bytes is `floor((a·256 + (b − a)·t + 128) / 256)`, with `a`, `b` the two colours' bytes.

## 5. The terrain and zone map (`UpdateTerrainAndZoneMap`, `0x007A7FF0`) *[compat]*

### 5.1 Dirty blocks

The map is built in 16×16-cell blocks. Block `(bx, bz)` is dirty when bit `bx` of word `bz` of `+0x120` is set.
- `InvalidateTZMap` (`0x007A66C0`) sets all 16 words to all ones, `+0xFD` and `+0xFE`.
- `InvalidateTZMapRect(&rect)` (`0x007A71F0`, rect = cell x0, z0, x1, z1): sets the bits of blocks
  `(x0>>4 … min(x1>>4, 15)) × (z0>>4 … min(z1>>4, 15))`, then `+0xFD` and `+0xFE`. Lower bounds are not clamped
  *[quirk]*.

### 5.2 Per block

Nothing without the terrain. With `bcx = terrain slot 7 >> 4` and `bcz = terrain slot 8 >> 4` blocks (cells beyond a
multiple of 16 are not drawn), the sea level `h0` = terrain slot 29, and the light `L` (§5.4), each dirty block at
cells `(x0, z0)` is drawn into a 16×16 pixel block:

1. **Fetch:** the 16×16 heights (terrain slot 15 on the cell rect `(x0, z0, x0+15, z0+15)`, row length 16) and the
   16×16 water values (terrain slot 16, the same rect).
2. **Height colour:** `i = clamp(64 + floor(0.2 × (h − h0)), 0, 255)` and `c = terrain palette[i]`.
3. **Normal:** `N` = (0, 1, 0) below the sea level. At or above it, the terrain's normal (slot 28 at the cell) with
   its y scaled by 0.4, then normalised (`1/sqrt`).
4. **Brightness:** `d = N·L` and `I = round_even(|d| + d + 127.75)`; with `|L| = 63.75`, `I` lies in 128…255.
   Each colour byte becomes `(c × I + 128) >> 8`; the alpha byte is 0.
5. **Water:** `w = water palette[clamp(floor(water value), 0, 255)]`. When its alpha `a` is not 0, each byte
   becomes `floor((s·256 + (w − s)·a + 128) / 256)` (verified on the bytes), with `s` the shaded byte; alpha 0.
6. **Zones** (flags bit `0x1`): the zone grid is `zones` slot 5, then its slot 25: a vector of byte rows, one per
   cell x. Each pixel `(x, z)` takes `zone table[signed byte (x0+x, z0+z)]` when that colour is not 0. The test
   "pixel < `0x01000000`" is always true, because every pixel has alpha 0 here.
7. **Resample into the map** at pixel `(z0 >> s, x0 >> s)` (computed as `((z0·16) >> (s+4))`), with row length `n`:

| `s` | Function | Result |
|---|---|---|
| 0 | `Downsample1` (`0x007A6A70`) | the 16×16 block copied |
| 1 | `Downsample4` (`0x007A6AD0`) | 8×8: each the 2×2 average `(sum + 2) >> 2` per byte, alpha 0 |
| 2 | `Downsample16` (`0x007A6BD0`) | 4×4: each the 4×4 average `(sum + 8) >> 4` per byte, alpha 0 |
| −1 | `Upsample4` (`0x007A6E60`) | 32×32: each pixel repeated 2×2 |
| −2 | `Upsample16` (`0x007A6EE0`) | 64×64: each pixel repeated 4×4 |
| other | — | the block is skipped |

Then `+0xFD` is cleared and all dirty bits are cleared.

### 5.3 Rounding detail

`floor(x)` above is the exe's sequence: `k = round_even(x − 0.5)` (adding `1.5 × 2^23` and storing binary32), then
`k + 1` when `x` equals `k + 1`. `round_even` is the same addition without the 0.5. The products and sums are
evaluated in x87 extended precision.

### 5.4 The light

`L = 63.75 × (lx, 0.57735026, lz)`, with `(lx, lz)` by the view rotation:

| Rotation | `(lx, lz)` |
|---|---|
| 0 | (−0.57735, −0.57735) |
| 1 | (+0.57735, −0.57735) |
| 2 | (+0.57735, +0.57735) |
| 3 | (−0.57735, +0.57735) |

So the light keeps its screen direction as the map turns. The constants 0.5 (ambient), 127.75 (`0.5 × 255 + 0.25`)
and 63.75 are function statics (`0x00B4E1B4`, `0x00B4E1B0`, `0x00B4E1AC`, set once).

## 6. Networks (in `Update`, flags bit `0x2`) *[compat]*

A filter `cSC4SurfaceNetworkOccupantFilter` (12 bytes, vtables `0x00AB8340`/`0x00AB8358`) admits occupants that
answer `0xA821EF94` and whose network-type mask (slot 12) has a bit of `0x1F4F`. It answers QI `1` and `0x46A80026`.

For each cell `(x, z)` of the city, the bit pair at index `b = 2·(cx·z + x)` of `+0x960` holds "checked" (bit `b`)
and "has a network" (bit `b+1`):
1. When not checked: clear `b+1`; when `occupants` slot 32 `(&o, x, z, filter)` finds one, set `b+1` and release
   `o`; set `b`.
2. With `b+1` set, the map pixel `((z·16) >> (s+4), (x·16) >> (s+4))` := `0xFFCFCFC5`. At `s` < 0 this marks one
   pixel of the cell's 2×2 or 4×4 *[quirk]*.

## 7. `Update` (`0x007A8640`, `+0xD8` slot 9) and the blits *[compat]*

1. When the window is not visible up its parent chain (`GZWinIsVisibleRecursive`, `0x007A6590`: every window to the
   root answers slot 71), free the map pixels.
2. Nothing more without map pixels or without `+0xFE`.
3. Then one of:
   - **Overlay mode** (an overlay is set and flags bit `0x8`): lock the overlay (`0x8040`), then
     `RotateBlt(surface, overlay pixels, overlay pitch / 4, n, rotation)`.
   - **Otherwise**:
     1. when `+0xFD`: `UpdateTerrainAndZoneMap`;
     2. with flags bit `0x2`: the networks (§6);
     3. with an overlay: `RotateBltDual(surface, map, map width, n, overlay, rotation)`; else
        `RotateBlt(surface, map, map width, n, rotation)`.
4. `+0xFE` := 0; the window's slot 92 (a repaint request); `+0xFF` := 0; `HookOccupantMessages`.

**Steps** (`0x007A65D0`, shared): `(a, b)` for rotation 0 = (1, P), 1 = (P, −1), 2 = (−1, −P), 3 = (−P, 1); other
values as 0.

**`RotateBlt(dst, src, P, n, r)`** (`0x007A66F0`): lock `dst` (`0x8040`). Each pixel is
`dst[y][x] = src[o + x·a + y·b] | 0xFF000000`, with `o = −((a < 0 ? n−1 : 0)·a + (b < 0 ? n−1 : 0)·b)`. Unlock.

**`RotateBltDual(dst, src, P, n, ov, r)`** (`0x007A67F0`):
1. Lock `ov`; steps for `src` (pitch P) and for `ov` (its pitch / 4).
2. Lock `dst`. Each pixel takes the map pixel `t` and the overlay pixel `v` at the same rotated position. With
   `k = 256 − (v >> 24)`, each colour byte is `min(255, v_c + ((t_c·k + 128) >> 8))`, and the alpha is `0xFF`.
3. Unlock both. Without the `dst` lock, `ov` is still unlocked.

## 8. Painting (`GZPaint`, `0x007A79B0`, slot 88) *[compat]*

### 8.1 The map

1. When attached and without map pixels: `InitTZMap`, then `Update`.
2. When the paint buffer (`+0x6C` slot 46, `(&info, 0)`) locks:
   - **Format 7 (4444):** lock the surface (`0x8040`). `DitherRGB8ToARGB4` writes the surface into the buffer at
     the window area `(+0x34, +0x38)`, size `(+0x3C − +0x34) × (+0x40 − +0x38)`.
   - **Otherwise:** lock the surface (`0x8010`). The draw context `+0x68` slot 29 blits it to the rectangle
     `(+0x24, +0x28)`–`(+0x24 + n, +0x28 + n)` (surface slot 12 gives the source).
   - Unlock the surface and the buffer (slot 47).

**`DitherRGB8ToARGB4(dst, dstPitch, src, srcPitch, w, h)`** (`0x007A6FB0`):
- the matrix `M` (`0x00AB82D8`, ×2^24) is `[0 8 2 10; 12 4 14 6; 3 11 1 9; 15 7 13 5]`;
- `d = M[h' & 3][w' & 3] << 24`, where `w'`, `h'` are the **remaining** column and row counts (they count down);
- each channel `q = (c × 0xF1F1F1 + d) >> 28` (4 bits); the 16-bit pixel is `0xF000 | R<<8 | G<<4 | B`.

### 8.2 The view frustum (flags bit `0x4`)

1. From the camera (`render` slot 14), take the eye rays through the screen corners (0, 0), (W−1, 0), (W−1, H−1)
   and (0, H−1). `W` and `H` are the main window's slots 41/42.
2. Hit each ray on the terrain (`render` slot 63, `PickTerrainAlways(origin, dir, &hit, 1)`, render3d.md §11.6).
3. For each hit:
   - `u = x / (sizeX − 1) − 0.5`, `v = z / (sizeZ − 1) − 0.5`;
   - rotation bit 0 → `(u, v) := (v, −u)`; bit 1 → both negated;
   - point = `(+0x24 + floor((u + 0.5)(n − 1)), +0x28 + floor((v + 0.5)(n − 1)))`.
4. Draw the four edges `p_i → p_(i+1) mod 4` in white (`+0x68` slot 30 `(255, 255, 255)`) with
   `DrawLineAntialiased(+0x68, x0, y0, x1, y1, colour, 1, 0)`, the coordinates truncated to integers.

### 8.3 The marker (flags bit `0x10`)

When `city` slot 49 then its slot 9 returns an object, its position (slot 8) `(x, z)` is mapped:
- rotation bit 0 → `(x, z) := (z, sizeZ − x)`; bit 1 → `(sizeX − x, sizeZ − z)`;
- `px = (+0xB0 − +0xA8) × x / sizeX` and `pz = (+0xB4 − +0xAC) × z / sizeZ`.

The blink byte `+0x102` flips on each paint. The colour is `0xFFC00000` when it becomes set, else `0xFFFF4040`
(paint buffer slot 21). The buffer's slot 44 `(3, points, 2)` draws `(px − 1, pz − 1)`–`(px + 1, pz + 1)`.

## 9. Input *[compat]*

- **`CenterOnMapPoint(x, y)`** (`0x007A72B0`, RET 8):
  1. `u = x / width − 0.5` and `v = y / height − 0.5` (window slots 41/42), stored as binary32.
  2. Rotation bit 0 → `(u, v) := (−v, u)`; bit 1 → both negated.
  3. `A = clamp(u + 0.5, 0, 1)` and `B = clamp(v + 0.5, 0, 1)`: values ≥ 1 or NaN become 1, values ≤ 0 become 0.
  4. The camera control's `SetViewTarget` (`0x007CD810`) gets `(A × sizeX, terrain slot 13 (x, z), B × sizeZ)`.
- `GZOnMouseDownL(x, y, …)` (`0x007A7720`): `CenterOnMapPoint`, set `+0x100`, capture the mouse; true.
- `GZOnMouseMove` (`0x007A7750`): `CenterOnMapPoint` while `+0x100` is set; true.
- `GZOnMouseUpL` (`0x007A6690`, RET 0xC): clear `+0x100`, release the capture; true.
- `GZOnMouseEnter`/`Exit` (`0x007A66B0`, RET 4): clear `+0x100`; true.

## 10. Messages and settings *[compat]*

### 10.1 `DoMessage(m)` (`0x007A8EA0`, `+0xDC` slot 3, RET 4)

By `m`'s type (slot 4):

| Type | Effect | Returns |
|---|---|---|
| `0x26D31EC1` (city init) | `OnCityInit`; `Update`; window slot 92 | true |
| `0x26D31EC2` (city shutdown) | `OnCityShutdown` | true |
| `0x07699455` | `InvalidateTZMapRect(*data2)`: data2 (slot 15) points at a cell rect | true |
| `0x278E6003`, `0x678E6E3A`, `0x89171496` | `InvalidateTZMapRect((d1 >> 16, d2 >> 16, d1 & 0xFFFF, d2 & 0xFFFF))`, with d1 = slot 9 and d2 = slot 14 | true |
| `0x86AD10EE` (animation tick) | §10.1.1 | true |
| `0x99EF1142`, `0x99EF1143` (network occupant added, removed) | §10.1.2 | true |
| `0xA6B79602` | `a` = slot 10, `b` = slot 15: `a[1] ≠ b[1]` → `InvalidateTZMap`; `a[0] ≠ b[0]` → `+0xFF` := 1 | true |
| `0xA6B79612` | `+0xFF` := 1 | true |
| `0xC53D10AA` | when slot 9 is `0x690F693F` and slot 14 `0x8A356F2C` (the tuning exemplar), `LoadTunableValues` | true |
| other | — | false |

#### 10.1.1 The tick

- Visible (§7): `+0xF8` += the message's slot 19 word read as a float. When the sum exceeds 1000.0:
  1. reset it to 0 and `Update`;
  2. with flags bit `0x10`, `city` slot 49 then its slot 6 non-zero sets `+0xFF`.

  Then, when `+0xFF` is set: window slot 92 and clear `+0xFF`.
- Not visible: free the map pixels.

#### 10.1.2 Network changes

Only when the message's slot 19 word is `0xC772BF98`:
1. The occupant (slot 10) answers `0xA821EF94`; its slot 59 gives the cell `(x, z)`.
2. Inside the city, clear its "checked" bit.
3. Removal (`…43`): `InvalidateTZMapRect((x, z, x, z))`. Addition (`…42`): `+0xFE` := 1, since the network pixel is
   added on the next `Update`.

### 10.2 Flags and hooks

`SetFlags(mask, value)` (`0x007A7770`, `+0xD8` slot 8):
1. Unhook (as below) when hooked.
2. `flags := (flags & ~mask) ^ value`.
3. Mark every block dirty, set `+0xFD`/`+0xFE`, and return the flags.

| Bit | Meaning |
|---|---|
| `0x1` | zone colours (§5.2) |
| `0x2` | networks (§6) and their messages |
| `0x4` | the view frustum (§8.2) |
| `0x8` | overlay-only mode (§7) |
| `0x10` | the blinking marker (§8.3) |

The constructor sets `0x7`.

`HookOccupantMessages` (`0x007A7140`): when not hooked and attached, subscribe `0x99EF1142` and `0x99EF1143` (only
with flags bit `0x2`), then mark hooked. `UnhookOccupantMessages` (`0x007A71A0`): when hooked, unsubscribe them
(only with bit `0x2`) and mark unhooked. Since `SetFlags` unhooks before changing the bits, clearing bit `0x2` while
hooked still unsubscribes. `Update`'s tail hooks again with the new bits.

### 10.3 Zone colours and overlay

- `SetDefaultZoneColor(c)` (`0x007A70E0`): every entry := `c | 0xFF000000`, then entry 0 := 0.
- `SetDefaultZoneColorToTrans` (`0x007A70C0`): every entry := 0.
- `SetZoneColor(i, c)` (`0x007A7120`): for `i < 16`, entry `i` := `c ? c | 0xFF000000 : 0`.
- `SetOverlay(o)` (`0x007A7080`): replace `+0xF4` (AddRef the new one, release the old) and set `+0xFE`.

## 11. Quirks

- `SetArea`'s `OnCityInit` call is a no-op while attached, so a resize keeps the old pixel buffer size until
  re-attach.
- The scale shift uses the x cell count only.
- The network bits survive `OnCityShutdown`; a next city of the same size reuses the stale "checked"/"network" bits
  until those cells change.
- `InvalidateTZMapRect` clamps only the upper block bounds.
- The zone overwrite test is always true.
- Network pixels mark one pixel per cell when the map is upscaled.
- The 4444 paint path copies the window area's size, which can exceed the `n`-pixel surface when the window is not
  square.
- `CenterOnMapPoint` and the frustum use different scales (`size` vs `size − 1`).

## 12. Conformance cases

| Case | Setup | Expected |
|---|---|---|
| MM-C1 | `SetArea` 0, 0, 200, 150 | `n` = 128 |
| MM-C2 | city 256 cells, `n` 128 | `s` = 1; `Downsample4` |
| MM-C3 | city 64 cells, `n` 128 | `s` = −1; `Upsample4` |
| MM-C4 | height = sea level + 10 | index 66 |
| MM-C5 | cell below the sea | `N` = (0, 1, 0), `d` = 36.81, `I` = 201 |
| MM-C6 | shipped terrain ramp, index 60 | between keys 58 and 62: `t` = 128; each byte `floor((a·256 + (b−a)·128 + 128)/256)` |
| MM-C7 | rotation 1, `RotateBlt` | `dst[y][x] = src[(n−1) + x·P − y]` |
| MM-C8 | tick values 600, 500 | one `Update` after the second (1100 > 1000) |
| MM-C9 | `SetFlags(0x2, 0)` while hooked | unsubscribed; bits `0x5` left |
| MM-C10 | click at the window centre, rotation 0 | view target (0.5·sizeX, h, 0.5·sizeZ) |

## 13. Function inventory

`cSC4WinMiniMap` (38 functions, 10,345 bytes), `cSC4WinMiniMap::cTZMap` (1, 87), `fn:GZWinIsVisibleRecursive`
(1, 51), `fn:GZCOM_cSC4WinMiniMapCLSID` (1, 6): all specified.

| Address | Bytes | Name | Section |
|---|---|---|---|
| `0x007A6580` | 6 | GZCOM_cSC4WinMiniMapCLSID | §1 |
| `0x007A6590` | 51 | GZWinIsVisibleRecursive | §7 |
| `0x007A6650` | 64 | QueryInterface | §1 |
| `0x007A6690` | 17 | GZOnMouseUpL | §9 |
| `0x007A66B0` | 12 | GZOnMouseEnter / Exit | §9 |
| `0x007A66C0` | 38 | InvalidateTZMap | §5.1 |
| `0x007A66F0` | 240 | RotateBlt | §7 |
| `0x007A67F0` | 629 | RotateBltDual | §7 |
| `0x007A6A70` | 96 | Downsample1 | §5.2 |
| `0x007A6AD0` | 255 | Downsample4 | §5.2 |
| `0x007A6BD0` | 652 | Downsample16 | §5.2 |
| `0x007A6E60` | 116 | Upsample4 | §5.2 |
| `0x007A6EE0` | 197 | Upsample16 | §5.2 |
| `0x007A6FB0` | 206 | DitherRGB8ToARGB4 | §8.1 |
| `0x007A7080` | 51 | SetOverlay | §10.3 |
| `0x007A70C0` | 30 | SetDefaultZoneColorToTrans | §10.3 |
| `0x007A70E0` | 56 | SetDefaultZoneColor | §10.3 |
| `0x007A7120` | 31 | SetZoneColor | §10.3 |
| `0x007A7140` | 83 | HookOccupantMessages | §10.2 |
| `0x007A71A0` | 73 | UnhookOccupantMessages | §10.2 |
| `0x007A71F0` | 190 | InvalidateTZMapRect | §5.1 |
| `0x007A72B0` | 408 | CenterOnMapPoint | §9 |
| `0x007A7450` | 286 | LoadTunableValues | §4.1 |
| `0x007A7570` | 87 | cTZMap::Init | §3 |
| `0x007A75D0` | 325 | Shutdown | §1 |
| `0x007A7720` | 40 | GZOnMouseDownL | §9 |
| `0x007A7750` | 30 | GZOnMouseMove | §9 |
| `0x007A7770` | 110 | SetFlags | §10.2 |
| `0x007A77E0` | 83 | OnCityShutdown | §3 |
| `0x007A7840` | 185 | InitTZMap | §3 |
| `0x007A79B0` | 1596 | GZPaint | §8 |
| `0x007A7FF0` | 1576 | UpdateTerrainAndZoneMap | §5 |
| `0x007A8640` | 733 | Update | §7 |
| `0x007A8A40` | 11 | QueryInterface thunk (`+0xD8`) | §1 |
| `0x007A8A50` | 11 | QueryInterface thunk (`+0xDC`) | §1 |
| `0x007A8A60` | 130 | destructor body | §1 |
| `0x007A8B20` | 30 | deleting destructor | §1 |
| `0x007A8B40` | 136 | OnCityInit | §3 |
| `0x007A8BD0` | 596 | Init | §1 |
| `0x007A8E30` | 105 | SetArea | §3 |
| `0x007A8EA0` | 918 | DoMessage | §10.1 |
