# `cSC4WinRegionView`: the region playfield (behaviour spec)

Scope: three owners as tools/coverage.py assigns them on `5c0ff782` (inventory in §11):

| Owner | Functions | Bytes |
|---|---|---|
| `cSC4WinRegionView` | 34 | 11,566 |
| `cSC4TiledScrollingCanvas` | 19 | 3,248 |
| `cSC4RegionSignpostAnimatorWinProc` | 5 | 401 |

Three unattributed functions of the view are described too (§11). The window that owns the view, makes the city
images and handles the mouse is the region screen (region-screen.md). The city records come from the region
(region-model.md, region.md) and the regional city (regional-city.md).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled. The decompile of this owner loses register arguments,
so the bodies were read from the disassembly. Written 2026-10-23 (re-regionview7).

Conventions:
- `V` is the view.
- `C` is its canvas.
- `e` is a city entry (§2.3).
- The renderer is `0x00B43DD0` (`cISC43DRender`).
- The application is `0x00B43C94`. Its slot 34 is the region manager and its slot 49 the tutorial registry.
- `GetCity(x, z)` is region slot 11 (region-model.md §1.3), reached as the manager's slot 11 `(V+0xDC)` and then that
  object's slot 11 `(e.x, e.z)`. It returns a pointer to the city's holder, `rc` = the regional city in it.
- Surface slots (the GZ 2D buffers):

| Slot | Use |
|---|---|
| 6, 7 | lock and unlock with a code |
| 9, 10 | width, height |
| 12 | its rectangle |
| 18 | `Fill(colour, null)` |
| 21, 22 | `GetPixel(x, y)`, `SetPixel(x, y, c)` |
| 29 | `Blit(src, &srcRect, &dstRect, clip)` |
| 30, 31 | `rgb → colour`, `colour → (r, g, b)` |
| 34, 35 | bits, pitch |

## 1. The view object *[compat]*

| Offset | vtable | Slots overridden |
|---|---|---|
| `+0x00` | `0x00AB9658` (a `cGZWin`) | 0 QI (`0x00682050`, shared); 4 `Init` (`0x007B6060`); 5 `Shutdown` (`0x007B53A0`); 55 `SetArea` (`0x007B6240`); 148 deleting destructor (`0x007B5E70`); 149 `is_point_transparent` (`0x007B2440`) |
| `+0xD8` | `0x00AB9644` | the tile renderer interface given to the canvas: 0 QI thunk (`0x007B4140`, `this − 0xD8`), 1, 2 (shared), 3 `RenderTile` (`0x007B4150`) |

The constructor (`0x007B4090`, attributed to the region screen) runs `cGZWin`'s, sets both vtables and zeroes the
fields below. Exceptions: `+0x111`, `+0x113` and `+0x116` start at 1, and `+0xDC`, `+0xF8`, `+0xFC` are left as
`cGZWin` leaves them. Those three are set by the owner (region-screen.md §1).

| Offset | Field |
|---|---|
| `+0xDC` | the key given to the region manager's slot 11 *[inferred: the current region; RGV1]* |
| `+0xE0` | the background image (§1.1) |
| `+0xE4` | the selected city entry |
| `+0xE8`, `+0xEC` | the scroll position (the absolute pixel at the view's top left) |
| `+0xF0`, `+0xF4` | the signpost script instance and window ID (`SetSignpostType`) |
| `+0xF8`, `+0xFC` | the grid colour and the selection colour |
| `+0x100`…`+0x108` | the vector of city entries (pointers; not owned) |
| `+0x10C` | the canvas (§3) |
| `+0x110` | byte, not read here (the region screen's flag) |
| `+0x111`, `+0x112` | bytes: signposts for every city; signposts for cities that have no record (`SetSignpostMode`) |
| `+0x113` | byte: grid |
| `+0x114`, `+0x115` | bytes: labels for other cities; label for the selected city (`EnableLabels`) |
| `+0x116` | byte: the tutorial arrow |
| `+0x118` | the map mode (`SetMapMode`) |
| `+0x11C`…`+0x124` | vector of 2 icons: airport, seaport (§1.1) |

### 1.1 `Init` (`0x007B6060`)

1. When already initialised: true.
2. `cGZWin::Init`; `SetID(0x2BA6BB97)` (slot 64).
3. **The canvas.**
   1. A 0x44-byte canvas is made (vtables `0x00AB9630`, `0x00AB9618`; the tile vector and `+0x40` zeroed) and put
      in `+0x10C`, counted.
   2. `C.Init(width, height, 256, 256, V+0xD8)`, with width and height from the window's slots 41 and 42 (§3.1).
      If that fails, return **false** and leave the view initialised.
   3. The renderer's slot 32 `(C, 0, 1000)` *[inferred: register the canvas as a view]*.
4. `SetFlag(0x10000, 0)` and `SetFlag(0x8000, 0)`.
5. **Images**, each loaded with `cSLBuffer`:
   - the background image `(0x856DDBAC, 0x6A1EED2C, 0x4A2805FF, 1, 0)` into `+0xE0`;
   - the icon vector is resized to 2. Each entry `i` of the table `0x00AB9594` (12 bytes: instance, index, name)
     is loaded with `(0x856DDBAC, 0x46A006B0, instance, 1, 0)` into `+0x11C[i]`, counted.

     | `i` | Instance | Name |
     |---|---|---|
     | 0 | `0xEBABB1B0` | `region_airport` |
     | 1 | `0xEBABB1B1` | `region_seaport` |

     The names are effect names (§5).
6. True.

### 1.2 The other lifetime functions

- **`SetArea(4 args)`** (`0x007B6240`): `cGZWin::SetArea`, then `C.Init(width, height, 256, 256, V+0xD8)` again.
  The result is not checked; a failed re-init leaves the canvas without tiles or renderer (§3.1).
- **`Shutdown`** (`0x007B53A0`), when initialised:
  1. `ClearCities` (§2.2); erase the icons.
  2. With a canvas: the renderer's slot 33 `(C)` *[inferred: unregister]*, `C.Shutdown`, release, null.
  3. Release `+0xE0`, null; `cGZWin::Shutdown`. True.
- **Destructor** (`0x007B5C40`):
  1. Both vtables, `Shutdown`.
  2. Destroy the icon vector, release the canvas, free the city vector's storage, release `+0xE0`.
  3. `cGZWin`'s destructor.

  The deleting destructor `0x007B5E70` frees when bit 0 of its argument is set.
- **`is_point_transparent(x, y)`** (`0x007B2440`, slot 149):
  1. The point goes to screen coordinates (slot 60).
  2. `0x0078D6E0` walks the children (IID `0x22BA0121`, callback `0x0078D670`) for one at that point.

  The view is transparent except where it has a child, so mouse input reaches the region screen.

## 2. Cities

### 2.1 Selection and settings *[compat]*

| Function | Address | Effect |
|---|---|---|
| `GetSelectedCity` | `0x0074C6E0` | `+0xE4` |
| `AddCity(e)` | `0x007B5D50` | push back; `InvalidateRegion(e)`; `UpdateCity(e)` |
| `ClearSelection` | `0x007B5DB0` | `+0xE4` := 0; `InvalidateRegion(old)` when there was one |
| `SelectCity(x, y)` | `0x007B5DD0` | `c = GetCityAtPoint(x, y)`; when `c` differs: `+0xE4` := `c`, `InvalidateRegion(c)` if non-null, then `InvalidateRegion(old)` if non-null |
| `SetSignpostType(inst, id)` | `0x007B5E20` | `+0xF0`, `+0xF4`; `UpdateSignpost` for every city |
| `SetSignpostMode(a, b)` | `0x007B2410` | `+0x111`, `+0x112`; nothing redrawn |
| `EnableTutorialArrow(a)` | `0x007B2430` | `+0x116`; nothing redrawn |
| `EnableLabels(a, b)` | `0x007B30B0` | `+0x114`, `+0x115`; `InvalidateAllRegions` |
| `EnableGrid(a)` | `0x007B30D0` | `+0x113`; `InvalidateAllRegions` |
| `SetMapMode(m)` | `0x007B30F0` | `+0x118`; `InvalidateAllRegions` |

- **`InvalidateAllRegions`** (`0x007B29E0`):
  - every canvas tile is marked dirty;
  - every city's `+0x34` := 0 (its image is rebuilt, §4);
  - the renderer's slot 20 *[inferred: request a redraw]*.
- **`InvalidateRegion(e)`** (`0x007B5CA0`):
  1. The rectangle is the city image's rectangle placed at `e`'s absolute position: its draw position plus the
     scroll.
  2. `C.InvalidateRect`, the renderer's slot 20, then `UpdateSignpost(e)`.

### 2.2 Positions and hit testing *[compat]*

- **`GetDrawPosition(e, &x, &y)`** (`0x007B3030`) gives a view position:
  - `x = floor(e.fx) − +0xE8`;
  - `y = trunc(float(floor(e.fy) − +0xEC) − h)`, with `h` the city image's height (slot 10).

  So `(e.fx, e.fy)` is the bottom left of the image in region pixels.
- **`GetRegionPlayfieldRect(e, &r)`** (`0x007B3110`): the draw position plus the scroll, and the image's size.
- **`GetCityAtPoint(x, y)`** (`0x007B3A80`) tests the cities from the **last** to the first:
  1. `(px, py)` = the point minus the city's draw position.
  2. When `(px, py)` is inside the image's rectangle (half-open), the city is hit when `lower_bound(e.hitRuns,
     (py << 16) + px + 1)` has an odd index (§4.1).

  Returns the hit city, or null.
- **`ClearCities`** (`0x007B3C10`):
  1. `+0xE4` := 0.
  2. For each city: each effect in its list `+0x70` is stopped (slot 4 `(0)`), then released, and the nodes are
     freed.
  3. The vector is emptied; the tiles are marked dirty; the renderer's slot 20.

### 2.3 The city entry (as read here)

| Offset | Field |
|---|---|
| `+0x08`, `+0x0C` | the city's tile `(x, z)` |
| `+0x10`, `+0x14` | floats: its position in region pixels |
| `+0x18` | byte: the size code (0 small, 1 medium, 2 large) |
| `+0x1C` | the city image |
| `+0x24` | the image for map mode 1 (may be null) |
| `+0x2C` | the composed image (null: the city is not drawn) |
| `+0x30` | the signpost window |
| `+0x34` | byte: the composed image is current |
| `+0x38` | the composed image's runs (§4.1) |
| `+0x44` | the hit-test runs (`GetCityAtPoint`) |
| `+0x50` | the grid coverage spans (§4.1) |
| `+0x5C` | the selection coverage spans |
| `+0x68`, `+0x6C` | the label anchor, relative to the draw position |
| `+0x70` | list of effects (12-byte nodes: next, previous, effect) |
| `+0x74`…`+0x7C` | vector of icon records: `x`, `y`, icon |

The region screen builds the entries, their images and the `+0x44`, `+0x50` and `+0x5C` lists (RGV5).

## 3. `cSC4TiledScrollingCanvas` *[compat]*

A 0x44-byte object.

| Offset | Field |
|---|---|
| `+0x00` | vtable `0x00AB9630`: 0 QI, 1 AddRef, 2 Release, 3 `Update` |
| `+0x04` | `cRZUnknown` (vtable `0x00AB9618`) |
| `+0x0C`…`+0x14` | vector of tiles (8 bytes: surface, dirty byte at `+4`) |
| `+0x18`, `+0x1C` | the origin tile (column, row) |
| `+0x20`, `+0x24` | the pixel offset in the origin tile |
| `+0x28`, `+0x2C` | the tile width and height |
| `+0x30`, `+0x34` | columns and rows |
| `+0x38`, `+0x3C` | the view width and height |
| `+0x40` | the tile renderer (counted) |

The QI (`0x007B23E0`) answers `0xC989F960` itself and passes other IIDs to `cRZUnknown`'s at `+4`.

### 3.1 `Init(W, H, tw, th, renderer)` (`0x007B5EF0`, RET `0x14`)

1. Store the sizes. Columns = `(W − 1) / tw + 2`, rows = `(H − 1) / th + 2`. Assign the renderer.
2. The format comes from the graphics system (`0x00B43C9C`): its slot 8, then slot 15, then the display depth at
   `+4`.
   - The format is 9 when the depth is above 16, else 4.
   - The bits per pixel come from the format: 2 → 8, 3–7 → 16, 8–9 → 32, other → 0.
3. Resize the tiles to `columns × rows` (`0x007B5E90`). For each tile:
   1. Release its surface.
   2. Make one with the graphics system's slot 3 `(&tile)`.
   3. Size it with the surface's slot 3 `(tw, th, {format, bits})`.

   If either step fails: `Shutdown` (`0x007B5350`), false.
4. The origin and the offset := 0. True.

**`Shutdown`** (`0x007B5350`):
- slot 4 on every tile surface, then the tiles are erased;
- the renderer is released;
- true.

The destructor is `0x007B5300` (the deleting one; the `+4` thunk is `0x007B3E70`).

### 3.2 `InvalidateRect(&r)` (`0x007B2620`)

The rectangle is in canvas (absolute) pixels. In tile coordinates relative to the origin, using floor division:
- `x0 = max(0, floor((r.x0 − ox·tw) / tw))`;
- `x1 = min(columns − 1, floor((r.x1 − ox·tw − 1) / tw))`;
- the same for `y` with `th` and rows.

Every tile in `[x0, x1] × [y0, y1]` is marked dirty.

### 3.3 `ScrollTo(x, y)` (`0x007B3E80`, RET 8)

1. `tx = floor(x / tw)`, `ty = floor(y / th)`. `dx = tx − ox`, `dy = ty − oy`.
2. Move the tiles:
   - **when `|dx| < columns` and `|dy| < rows`:** a new vector is filled: `new[r][c] = old[(r + dy) mod rows][(c + dx)
     mod columns]` (surface and dirty flag). A tile whose source wrapped round on either axis is marked dirty. The
     new vector replaces the old one.
   - **otherwise:** every tile is marked dirty in place.
3. The origin := `(tx, ty)`; the offset := `(x − tx·tw, y − ty·th)`.

### 3.4 `Render(ctx, dx, dy)` (`0x007B2770`, RET `0xC`)

1. `R = {dx, dy, dx + W, dy + H}`. Lock `ctx` with `0x8010` (slot 6); a failure returns false.
2. For each tile `k` in row-major order (column `c`, row `r`):
   1. Its rectangle is `x0 = tw·c − offsetX`, `y0 = th·r − offsetY`, size `tw × th`.
   2. A dirty tile is drawn first: the renderer's slot 3 `(surface, (ox + c)·tw, (oy + r)·th)`. Then it is clean.
   3. `ctx` slot 29 `(surface, surface slot 12, &tileRect, &R)` copies it.
3. Unlock (`0x8010`). True.

### 3.5 `Update(dc)` (`0x007B28B0`, slot 3: the renderer's callback)

1. The renderer's slot 26. `dev` = `dc+0x28` slot 8. `vp` = `dc+0x30` slot 76.
2. When `dev` gives IID `0xAB300B2B` (`q`):
   1. `s = q` slot 5. `Hs` = `dev` slot 10.
   2. `s` slot 3, then `s` slot 6 `(&{vp.x0, Hs − vp.y1, vp.x1, Hs − vp.y0})`.
   3. `Render(dev, 0, 0)`.
   4. `s` slot 9, `q` slot 6.

   *[inferred: draw the tiles with the 2D buffer, the clip flipped in y]*
3. The renderer's slot 24; `dc+0x30` slot 74 `(vp.x0, vp.y0, w, h)`; `SC4DrawContext::SetDefaultRenderStateUnilaterally
   (dc)`. True.

### 3.6 Tile vector helpers

| Function | Address |
|---|---|
| construct with `n` empty tiles | `0x007B3B60` |
| destroy (release each, free) | `0x007B3BD0` |
| copy | `0x007B31D0` |
| copy `n` | `0x007B3220` |
| copy backward | `0x007B3290` |
| uninitialized copy | `0x007B35F0` |
| uninitialized fill `n` | `0x007B3630` |
| erase | `0x007B3D30` |
| grow-insert (capacity `size + max(size, n)`) | `0x007B3D80` |
| insert `n` in place | `0x007B51D0` |
| resize | `0x007B5E90` |

Surfaces are counted; the dirty byte is copied with them.

## 4. Drawing a tile: `RenderTile(dst, wx, wy)` (`0x007B4150`, `+0xD8` slot 3, RET `0xC`) *[compat]*

`(wx, wy)` is the tile's absolute pixel origin. A city's tile position is `p = (floor(e.fx) − wx, floor(e.fy) − h −
wy)`, the draw position moved to the tile.

1. **Background.**
   - With `+0xE0`: `RZDrawBltTiled(dst, img, &img.rect, &dst.rect, −(+0xE8 + wx), −(+0xEC + wy), 0)`.
   - Otherwise: `dst` slot 18 `(rgb(0, 0, 0), null)`.
2. **Cities**, in vector order, so later ones draw over earlier ones:
   1. `S` = the image's rectangle and `D` = `S` at `p`. `cGZDrawContext::RecomputeClipRect` clips them against the
      tile; nothing visible → next city.
   2. A city with no composed image `+0x2C` is skipped.
   3. When `+0x34` is clear, the composed image is rebuilt:
      1. `src` = `+0x1C`, or `+0x24` when the map mode is 1 and that image exists.
      2. `CopyAndTint(src, +0x2C, grid ? &e+0x50 : null, +0xF8)` (§4.1).
      3. `BuildRuns(+0x2C, &e+0x38)`; `+0x34` := 1.
   4. `BlitImageSpans(&S, &D, dst; +0x2C, &e+0x38)`.
3. **Labels and icons**, for each city:
   1. The same clip test (nothing visible → next city). No `GetCity` holder → next city.
   2. **The label** is drawn when all three hold, with `tut` = the city is a tutorial city (rc slot 67):
      - `tut` or the city is established (rc slot 43);
      - the city is not the selected one, or `+0x115` is set;
      - `tut` or `+0x114` is set.

      How it is drawn:
      1. The fonts come from the control manager (service `0xC2C2EB0F`, IID `0x22C2EB1F`, slot 5): `f1` = font
         `0x8A8CC984 − 2b` and `f2` = font `0x8A8CC985 − 2b`, with `b` = the size code.
      2. `name` = rc slot 33. `mayor` = rc slot 35, emptied when the tutorial registry's item for the city (rc
         slot 65) has its byte `+0x10` set.
      3. Font slot 50 `(&text, style, &{0, 0, 4/3 × image width, f1 height}, 0)` runs on both strings, with the
         style from the manager's slot 35 `(0x5377BE31)` *[inferred: layout in that box]*.
      4. Both are drawn centred on the anchor `a` = `p` + `(e+0x68, e+0x6C)` (font slot 48 gives the width), colour
         `rgb(0xDE, 0xE8, 0xE3)`, flags `0x8000000`, through font slot 51:
         - the name at `y = a.y − f1.height`;
         - the mayor at `y = a.y − f1.height + f2.height` *[RGV4]*.
   3. **Icons**, when the map mode is not 0:
      1. A GZCOM object (class `0x0AE6320E`, IID `0x2AE63219`) gets slots 17 `(dst)`, 21 `(−1)`, 23 `(1)` and 22
         `(1)` *[RGV3]*.
      2. Each icon record is centred on its `(x, y)` minus `(wx, wy)` and drawn with the object's slot 38 `(icon,
         &iconRect, &destRect)`.
4. **The selection**: the selected city's coverage spans `+0x5C` are filled with `+0xFC` over its image rectangle
   at `p` (`FillCoverage`).

Quirk: the colour `rgb(0x3C, 0x53, 0x8C)` is made and thrown away.

### 4.1 The image helpers (register calling conventions)

Two list formats:
- **Runs** are pairs of words. The opening word is `(y << 16) | x0`; the closing one has `x1` in its low half.
  They are sorted, so a point is inside a run when `lower_bound(key + 1)` has an odd index.
- **Coverage spans** are `(y << 16) | x0`, `x1`, then `x1 − x0` coverage dwords (0–255).

The helpers:
- **`BuildRuns(img, &runs)`** (`0x007B3670`, cdecl). It also changes the image in place:
  1. An empty image returns; otherwise the list is cleared and the image locked (`0x8040`).
  2. Each pixel `c` that is not fully opaque (`c < 0xFF000000`) is composited, all four channels, every step
     `ch = ((under × (255 − top.a) + 128) >> 8) + top`:
     1. `t` = the right neighbour (0 at the row end) under the pixel below (0 on the last row).
     2. The new pixel = `t` under `c`, each channel clamped to 255.

     This spreads every edge by one pixel to the left and up.
  3. Pixels are classed by their new alpha: 0 → none, 1–254 → partial, 255 → opaque. A class change closes the
     open run and opens a new one when the class is not none. The row's end closes an open run.
  4. Unlock.
- **`BlitImageSpans(&S, &D, dst; img, &runs)`** (`0x007B2A30`; `S` in ECX, `D` in EAX, `dst` in EDI):
  1. Clip `S` and `D` together, first against the image's rectangle, then `D` against `dst`'s.
  2. Lock `dst` (`0x8080`) and the image (`0x40`).
  3. For each run, clamped to `S`'s columns and within `S`'s rows:
     - when its first pixel is opaque, the row segment is blitted (slot 29);
     - otherwise each pixel with alpha `a > 0` is blended over `dst`: `d = min(255, ((d × (255 − a) + 128) >> 8) +
       s)` for R, G and B (premultiplied).
  4. Unlock both.
- **`FillCoverage(&rect, dst; spans, colour)`** (`0x007B2DD0`; the rectangle in EAX, `dst` in ESI):
  - span coordinates are offset by the rectangle's origin and clipped to `rect ∩ dst`;
  - each pixel with coverage `a ≠ 0` gets `d += ((C − d) × a + 128) >> 8` per channel, kept to a byte;
  - lock `0x8080`, unlock `0x8040`.
- **`CopyAndTint(src, dst, spans, colour)`** (`0x007B3300`, `src` in EAX):
  1. Lock `dst` (`0x8080`) and `src` (`0x800`).
  2. Copy `min(srcW, srcW) × min(srcH, srcH)` rows. The minimum compares the source with itself, so `dst` is never
     bounds-checked *[quirk]*.
  3. Every covered pixel moves its RGB toward the colour by `(coverage & 0xFF) >> 1` (half strength) and keeps its
     alpha; the spans are not clipped.
  4. Only `dst` is unlocked; the `src` lock is never released *[quirk]*.

## 5. `UpdateCity(e)` (`0x007B5430`) *[compat]*

1. `+0x34` := 0. The icon records are cleared, and every effect in `+0x70` is stopped, released and freed.
2. All tiles are marked dirty, every city's `+0x34` := 0, and the renderer's slot 20 runs. So one city's update
   redraws everything *[quirk]*.
3. `rc` = `GetCity(e.x, e.z)`; none → return. The city's corner in metres is `(X, Z) = (e.x << 10, e.z << 10)`.
4. For `i` = 0 (airport) and 1 (seaport), when the icon `+0x11C[i]` exists:
   1. `rc` slot 72 `GetPointsOfInterest(i, &list)` gives 12-byte points `(px, py, pz)`.
   2. For each point, the region's isometric projection of `(X + px, py, Z + pz)`:
      - `x = round(0.0883883·(X + px) − 0.0366117·(Z + pz))`;
      - `y = round(0.0183058·(X + px) − 0.0828534·py + 0.0441942·(Z + pz))`.

      The constants are at `0x00B0DBBC`…`0x00B0DBD0`. The rounding adds `1.5 × 2^23` and subtracts its bits.
   3. Push `{x, y, icon}`.
   4. The effects manager (`0x00B43D1C`, slot 7) makes the effect named by the table (`region_airport`,
      `region_seaport`).
   5. When it exists:
      - it gets the transform record: flags 1, 1; rotation = identity; position `(X + px, py, Z + pz)`; scale 1
        (slot 7);
      - it is started (slot 3 `(0)`);
      - it is appended to `+0x70`.

## 6. Signposts (`UpdateSignpost(e)`, `0x007B59B0`) *[compat]*

1. **The old signpost.** If the city has one: remove it from the view (slot 16) and release it.
2. **The tutorial city.** The tutorial registry's next active item for the current region (`GetNextActive`) gives
   a regional city (`GetRegionalCity`), and its slot 5 gives the tutorial tile `(tx, ty)`, else `(−1, −1)`.
3. **Whether to make one.** The **arrow** case is: `+0x116` set, `(tx, ty)` = the city's tile, both ≥ 0, and the
   city not selected. Otherwise:
   1. Return when `+0xF0` is 0, or when `+0x111` is clear and the city is not selected.
   2. Then a signpost is made when the city has a holder whose city is established (rc slot 43), or when `+0x112`
      is set; otherwise return.
4. **The window.**
   1. The window manager (service `0xA417445E`, IID `0x5A4`) is fetched and its slot 36 value kept.
   2. `SC4CreateWindowFromScript(&{0, 0x96A006B0, inst}, V, id)` makes the window, with `(inst, id)` = `(+0xF0,
      +0xF4)`, or `(0xCA539343, 0x0A551C53)` in the arrow case. It is stored in `+0x30`.
5. **Filling it**, when it was made:
   1. `InitSignpost(e, holder)` (§6.1).
   2. Its children are walked with `0x007B2340`, which recurses. Each child gets `SetFlag(0x8000, 0)`, and when it
      gives IID `0x00008810` it also gets slot 86 with the view's slot 87 value *[inferred: the notification
      target; RGV6]*.
   3. Through `cIGZWinGen` (`0x5386D516`, slot 7) the window proc becomes a new `cSC4RegionSignpostAnimatorWinProc`
      (§7).
6. **The window manager again.** When its slot 24 `(kept)` is true, its slot 37 `(kept)`.

**`SetScrollPos(x, y)`** (`0x007B4A60`, RET 8):
1. Nothing when unchanged. Otherwise `+0xE8`, `+0xEC` := `(x, y)`.
2. Every signpost is moved (slot 56) to `((px + px + w) / 2 − sw / 2, (py + py + h) / 2 − sh)`:
   - `(px, py)` is the city's draw position and `w × h` its image;
   - `sw × sh` is the signpost (slots 41, 42);
   - every division truncates toward zero.
3. `C.ScrollTo(x, y)`.

### 6.1 `InitSignpost(e, holder)` (`0x007B4B80`, RET 8)

The signpost is placed as in `SetScrollPos`. The rating fraction starts at 0.5.

- **An established city** (a holder whose city passes rc slot 43). Children by ID (slot 35):

| Child | Content |
|---|---|
| `0x4A552000` | the city name (rc slot 33), slot 74 |
| `0x4A552001` | the mayor name (rc slot 35, emptied by `filter_tutorial_mayor_name`, below) |
| `0x6C06F4A0`, `0xAC06F4C4`, `0xCC06F4CF` | `SetFlag(1, d == 0)`, `(1, d == 1)`, `(1, d == 2)` with `d` = rc slot 15 `GetDifficultyLevel` |
| `0x4A552002` / `0x4A552006` | the budget (rc slot 50, a double, truncated to int64, `SC4MakeMoneyString`). Negative and `0x4A552006` present: `0x4A552002` hidden, the text in `0x4A552006`, shown. Otherwise `0x4A552006` hidden, the text in `0x4A552002`, shown |
| `0x4A552003`, `0x4A552004`, `0x4A552005` | rc slots 10, 11, 12 (population, commercial jobs, industrial jobs) as numbers (`RZ::LM` slot 38 `(0)` gives a formatter; its slot 18 `(int64, &text)`) |

  The rating fraction := `rating × 0.005 + 0.5`, with the rating from rc slot 14 (`GetMayorRating`) taken as a
  signed byte.
- **Otherwise:**
  1. `price` = 10000, or property `0x6A9C7718` of the resource `(0x6534284A, 0xE7E2C2DB, 0xC8E8F6C5)` when
     present.
  2. `0x4A552002` gets `SC4MakeMoneyString(price)`.
  3. With no holder at all, child `0x4A560003` gets `SetFlag(2, 0)` (disabled).

  The shipped resource at that key is the "Crime Simulator" exemplar, which has no such property. So the price
  shown is 10000 *[RGV2]*.
- **Both:**
  1. When the child `0x4A553000` gives IID `0x4A5D1208` (slot 37), it gets slot 4 `(rating fraction)`.
  2. When the image `cSLBuffer(0, 0x46A006B0, 0x14416327, 9, 0)` loads, it also gets slot 3 `(image)`.

**`filter_tutorial_mayor_name(rc, &name)`** (`0x007B3170`): for a tutorial city (rc slot 67) whose registry item
(rc slot 65) has `+0x10` set, `name` slot 15 `(0)` empties the name.

## 7. `cSC4RegionSignpostAnimatorWinProc` *[compat]*

A 0x14-byte window proc.

| Offset | Field |
|---|---|
| `+0x00` | vtable `0x00AB95F0` |
| `+0x04` | `cRZRefCount` (vtable `0x00AB9604`) |
| `+0x0C` | the start time |
| `+0x10` | the moving child (counted) |

- The constructor is `0x007B3560`, the destructor `0x007B35C0` (releases `+0x10`). The deleting destructor is
  `0x007B35A0`, with a thunk at `0x007B3590`.
- **`DoWinProcMessage(w, m)`** (`0x007B2500`), by `m[0]`:

| Message | Effect | Returns |
|---|---|---|
| `0xA2BF8AD5` | `+0xC` := `clock()`; `+0x10` := `w`'s child `0x4A630000` (slot 35) | true |
| `0xA2BF8AD6` | release `+0x10` | true |
| `0xA2BF8ACD` (tick) | with a child: the bob below | false |
| other | — | false |

The bob: `p = 6.28318 × ((clock() − +0xC) mod 1500) / 1500`. The child moves (slot 56) to `(0, trunc((parent height
− child height) × (1 − |sin p|)))`, the parent being the child's slot 11. So the arrow bounces twice every 1.5
seconds.

## 8. Quirks

- A failed `Init` of the canvas still leaves the view initialised. `SetArea` does not check its re-init.
- `UpdateCity` redraws every city.
- `CopyAndTint` takes both sides of its size minimum from the source, and never unlocks the source.
- `BuildRuns` changes the city's composed image in place.
- An unused colour in `RenderTile`.
- The unclaimed-land price is read from the Crime Simulator exemplar's key (RGV2).
- The mayor line's `y` uses the mayor font's height (RGV4).

## 9. Cases

| Case | Setup | Expected |
|---|---|---|
| RGV-C1 | canvas `W = 800`, `H = 600`, tiles 256 | 5 columns (`799 / 256 + 2`), 4 rows (`599 / 256 + 2`) |
| RGV-C2 | `ScrollTo(300, −10)` from origin `(0, 0)` | `tx = 1`, `ty = −1`; offset `(44, 246)`; column 4 and row 0 come from the other side and are dirty |
| RGV-C3 | `InvalidateRect({0, 0, 256, 256})` at origin `(0, 0)` | only tile `(0, 0)`: `x1 = floor(255 / 256) = 0` |
| RGV-C4 | runs `[0x00030002, 0x00030006]`, point `(px = 5, py = 3)` | key `0x00030006`, `lower_bound` index 1 (odd): hit; `px = 6` gives index 2: miss |
| RGV-C5 | rating −100 / 0 / 100 | fraction 0 / 0.5 / 1 |
| RGV-C6 | bob at `t − t0` = 375 ms | `p = π/2`, `1 − |sin p| = 0`: the child at `y = 0` |

## 10. Not covered

- The region screen (region-screen.md), which makes the entries and their images.
- The GZ surfaces, fonts and the blit object (RGV3).
- The effects (effects.md).

## 11. Function inventory

- `cSC4WinRegionView`, 34 functions (11,566 bytes):
  - §1: `0x007B4140`, `0x007B5C40`, `0x007B53A0`, `0x007B6060`, `0x007B6240`;
  - §2: `0x0074C6E0`, `0x007B29E0`, `0x007B3030`, `0x007B30B0`, `0x007B30D0`, `0x007B30F0`, `0x007B3110`,
    `0x007B3A80`, `0x007B5CA0`, `0x007B5D50`, `0x007B5DB0`, `0x007B5DD0`, `0x007B5E20`, `0x007B2410`, `0x007B2430`;
  - §3: `0x007B3E70`, `0x007B5300`, `0x007B5350`;
  - §4: `0x007B4150`, `0x007B2A30`, `0x007B2DD0`, `0x007B3300`, `0x007B3670`;
  - §5: `0x007B5430`;
  - §6: `0x007B59B0`, `0x007B4A60`, `0x007B4B80`, `0x007B3170`, `0x007B2340`.
- `cSC4TiledScrollingCanvas`, 19 functions (3,248 bytes):
  - §3: `0x007B23E0`, `0x007B2620`, `0x007B2770`, `0x007B28B0`, `0x007B3E60`, `0x007B3E80`, `0x007B5EF0`;
  - §3.6: `0x007B31D0`, `0x007B3220`, `0x007B3290`, `0x007B35F0`, `0x007B3630`, `0x007B3B60`, `0x007B3BD0`,
    `0x007B3D30`, `0x007B3D80`, `0x007B51D0`, `0x007B5E90`;
  - §2.2: `0x007B3C10` (`ClearCities`, a view function).
- `cSC4RegionSignpostAnimatorWinProc`, 5 functions (401 bytes), all in §7: `0x007B2500`, `0x007B3560`,
  `0x007B3590`, `0x007B35A0`, `0x007B35C0`.
- Unattributed (no owner, so not counted):
  - `0x007B2440` `is_point_transparent`;
  - `0x007B5E70` the view's deleting destructor;
  - `0x007B5FD5`, a fragment of the canvas `Init`.

  The constructor `0x007B4090` is counted under the region screen.
