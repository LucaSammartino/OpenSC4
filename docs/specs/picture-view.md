# Picture view, image resampling and the texture tile packer (behaviour spec)

Scope: the owners `cWinBMPView` (20 functions, 4,918 bytes), `cWinBMPViewGVExtra` (7, 706), `fn:Resample`
(7, 1,696), `fn:GenerateKernelArray` (1, 181), `fn:ConvertKernelToInteger` (1, 127) and `cRZTileSetup` (5, 3,054) as
tools/coverage.py assigns them.

Related: widget-bmp.md (`cGZWinBMP`, the picture window this view extends, and four of the view's setters),
widget-toolkit.md (window slots), widget-winmgr.md (message posting, cursor position), snapshots.md (the album dialog
that uses `cWinBMPViewGVExtra`), gimex.md (the image file factory), region-screen.md (the city thumbnail shift that
uses the 2-tap helpers of §7.4), gz-graphics.md (the SimGL buffer cache that uses the tile packer).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the view's interface table `0x00ABA700`, its
zoom, scroll, centre and metrics bodies, the paint, the file loader and writer, the resampler and its kernels
(`0x007BF7B0`, `0x007AA0E0`, constants `0x00A80AB0`…`0x00ABA6B8`), the kernel generators, the GVExtra bodies and
the tile packer's `Init` were read in the disassembly. Written 2026-10-24 (re-picview7).

## 1. The picture view (`cWinBMPView`) *[compat]*

A `cGZWinBMP` (widget-bmp.md) that shows its picture zoomed and scrolled. Object size `0x140`; vtables `0x00ABA770`
(window), `0x00ABA740` (the `cIGZWinBMP` face at `+0xD8`), `0x00ABA700` (the view interface at `+0xFC`, IID
`0x841471B9`; `QueryInterface` `0x007BFEE0`, thunk `0x007B6780`).

| Offset | Field | Default (constructor `0x007C0120`) |
|---|---|---|
| `+0x100` | zoom keys allowed (byte; setter window slot 153) | 1 |
| `+0x101` | scrolling allowed (byte; slot 154) | 1 |
| `+0x102` | fit large pictures to the window when loading (byte; slot 152) | 0 |
| `+0x104` / `+0x108` | minimum / maximum zoom | 0.125 / 16.0 |
| `+0x110` | centred mode (slot 151, widget-bmp.md §5) | 0 |
| `+0x114` | zoom | 1.0 |
| `+0x118`…`+0x124` | the picture's rectangle in window coordinates (§2) | 0 |
| `+0x128` / `+0x12C` | scroll position x / y | 0 |
| `+0x130` / `+0x134` | right-drag anchor point | 0 |
| `+0x138` / `+0x13C` | last auto-scroll time / interval in ms (`clock()`) | 0 / 20 |

The view interface (`+0xFC`):

| Slot | Address | Behaviour |
|---|---|---|
| 0 `SetZoom(z)` | `0x007C0830` | `z` clamped to [min, max]; unchanged → true. Else the image point under the window centre (slot 7 at (window width / 2, height / 2)) is kept: the zoom is stored, metrics (§2), centre on that point (slot 5), metrics; invalidate when the zoom or scroll changed. True |
| 1 `ZoomIn(f)` | `0x007BFC30` | `SetZoom(zoom × f)` |
| 2 `ZoomOut(f)` | `0x007BFC50` | false when `f` is below the minimum zoom *[quirk: the factor is compared with the zoom limit]*; else `SetZoom(zoom / f)` |
| 3 `SetZoomLimits(min, max)` | `0x007BFBE0` | `min` raised to 0.25, `max` lowered to 256; no re-clamp of the current zoom. True. The constructor's 0.125 is below this floor |
| 4 `SetScroll(x, y)` | `0x007C0930` | stores, metrics, invalidate. True |
| 5 `CenterOn(ix, iy)` | `0x007C0960` | false when the **unscaled** picture is smaller than the window both ways *[quirk]*, or the point is outside the picture. Else normal mode: scroll := `(trunc(ix·zoom − winW/2), trunc(iy·zoom − winH/2))`; centred mode: `(trunc(ix·zoom), trunc(iy·zoom))`; metrics, invalidate, true |
| 6 `CenterOnWindowPoint(x, y)` | `0x007BFC80` | slot 7, then slot 5 with its result (even when slot 7 failed) |
| 7 `WindowToImage(x, y, &ix, &iy)` | `0x007BFCC0` | `ix = trunc((x − rect.left) / zoom)`, `iy` likewise; true when a picture exists and `0 ≤ ix < W`, `0 ≤ iy < H` |
| 8 `Scroll(dx, dy)` | `0x007C0A70` | `±1` means a step of the picture's width (height) / 32, at least 1 pixel, in that direction; other values are pixels; `(0, 0)` does nothing. Adds to the scroll, metrics, invalidate when it moved. True |
| 9 `SetAutoScrollInterval(ms)` | `0x007B62D0` | `+0x13C` |
| 10 `GetBMP(&out)` | `0x007B62E0` | the picture pointer, borrowed |
| 11 `SetBMP(p)` | `0x007B6790` | the `cIGZWinBMP` face's `SetBMP` |
| 12 `SetBMPFromFile(path)` | `0x007C1670` | §6.2 |
| 13 `Free(p)` | `0x007BFDD0` | frees memory with the global heap |
| 14 / 15 `GetImageWidth` / `GetImageHeight` | `0x007B62F0`, `0x007B6310` | picture slots 9 / 10; 0 without a picture |

Window overrides: `Init` (`0x007C0790`: the base `Init`, metrics, then the shared image factory of §6); `SetArea`
(window slot 55, `0x007C07D0`: the base, then metrics); `Shutdown` (`0x007BF9D0`) and the destructors (`0x007C01D0`,
deleting `0x007C0730`) release the shared factory (§6), then the base.

## 2. Metrics (`CalculateAllMetrics`, `0x007C0390`) *[compat]*

With `W`, `H` the picture size and `winW`, `winH` the window size (window slots 41, 42); nothing when `W` or `H` is 0.

1. The zoom is clamped to [min, max]. `sW = W·zoom` (float), likewise `sH`.
2. Per axis (x shown): when `trunc(sW) ≤ winW` the scroll is 0 (normal) or `W / 2` (centred). Otherwise, normal mode:
   a scroll above `sW − winW` becomes `trunc(sW − winW)`, and a negative scroll 0; centred mode: above `sW − winW/2`
   becomes `trunc(sW − winW/2)`, and below `winW / 2` becomes `winW / 2` (unsigned compare).
3. The rectangle: normal mode `(−scrollX, −scrollY, trunc(sW) − scrollX, trunc(sH) − scrollY)`. Centred mode: left
   `= winW/2 − trunc(sW)/2`, top likewise, right `= trunc(left + sW)`, bottom likewise, then all shifted by
   `(W/2 − scrollX, H/2 − scrollY)`, with the **unscaled** half size *[quirk: correct only at zoom 1]*.

## 3. Input *[compat]*

- `GZOnKeyDown` (`0x007BFA30`): numeric-keypad `+` / `−` (`0x6B`, `0x6D`) zoom in / out by 2 when zoom keys are
  allowed; the arrow keys scroll by one step (`Scroll(±1, 0)`, `(0, ±1)`) when scrolling is allowed. **Every key
  returns true** *[quirk: the view consumes all keys]*.
- `GZOnMouseDownL` (`0x007BFB30`): when scrolling is allowed, slot 6 at the click point; true.
- `GZOnMouseDownR` (`0x007BFB60`): captures the mouse; on success records the anchor and posts message
  `0xFE342D2A` to itself (window manager slot 14). `GZOnMouseUpR` (`0x007BFBA0`): when capturing, records the point
  and releases the capture. `GZOnMouseWheel` (`0x007BFBD0`): true, nothing else.
- `DoMessage` (`0x007BFDE0`): `0xFE342D2A` is the drag auto-scroll: when more than the interval has passed since the
  last step, the cursor (window manager slot 34) is compared with the anchor with an 8-pixel dead zone and
  `Scroll(±1 or 0, ±1 or 0)` follows; while the capture lasts the message is posted again (also when no step was
  due); true. Messages 7–14 go to `cGZWin::DoMessage`, the rest to `cGZWinBMP::DoMessage`.

## 4. Painting (`GZPaint`, `0x007C0220`) *[compat]*

1. Without bitmap flag 2 (widget-bmp.md §2), and when there is no picture or the picture rectangle is narrower or
   shorter than the window: the draw context is locked (`0x8001`), filled with the window's fill colour for the
   draw area (window slot 103, context slot 20) and unlocked.
2. With a picture, under the `0x8010` lock: source `(0, 0, W, H)`, destination the picture rectangle offset by the
   draw area's origin (`+0x24`, `+0x28`). At zoom exactly 1.0 the context's blit (slot 29) with the draw area as clip;
   otherwise `RZDrawBltStretch(context, picture, src, dst, drawArea)`.

## 5. Writing a picture (`WriteBMPToFile(buffer, format, name)`, `0x007C0EF0`) *[compat]*

`format` 0 or 2 → `.jpg`, 1 → `.png`, 3 → `.gif`, 4 → `.bmp`; the extension is appended when `name` does not end
with it (case-sensitive). When it was appended, **an existing file at the original, extension-less name is
deleted** *[quirk]* and `name` (a `cIGZString`, its slot 4) receives the new name. A new image factory (gimex.md §3)
saves the buffer with quality 100 (factory slot 8). Returns whether the save succeeded. A failed factory allocation is
not tested *[quirk: crash]*.

## 6. Loading a picture *[compat]*

### 6.1 `LoadBufferFromFile(out, path)` (`0x007BFD60`)

The views share one image factory (class `0xA2FFB5D3`, IID `0x02FFBB60`, the Gimex factory of gimex.md §3) cached at
`0x00B4E1DC`, fetched from GZCOM on demand. `out := null`; factory slot 4 `Create(path, &buffer, 9)` (colour type 9,
A8R8G8B8); true when a buffer came back. Any view's `Shutdown` releases and clears the shared pointer; the others
fetch it again on their next load.

### 6.2 `SetBMPFromFile(path)` (slot 12)

A failed load sets no picture (`SetBMP(null)`) and returns false. When the fit flag (`+0x102`) is set and the picture
is wider or taller than the window (either ratio `window / picture` below 1), a new buffer of **exactly the window's
size** (graphic system slot 3, then the buffer's `Init(winW, winH, 9, 32)`) receives the picture resampled into it
(§7.1) and replaces it: the aspect ratio is not kept *[quirk]*. The picture is then set with `SetBMP`.

## 7. Resampling

### 7.1 `Resample(dst, src)` (`0x007C1500`) and the worker (`0x007C10F0`) *[compat]*

Both buffers are locked (`0x8040`; the result is false when either lock fails). Source rows are converted to
A8R8G8B8 and result rows back to the destination's format (`cRZRGBColorConverter`, rz-draw.md). Per axis (x shown,
`dW`, `sW` destination and source widths):

- `r = dW / sW`, `s = min(r, 1)`; taps `n = (ceil(4 / s) + 1) & ~1`; step `= trunc(65536 / r)` (16.16 fixed point);
  start `= step/2 − n·0x8000 + 0x8000`, the source position of the first tap for output 0.
- Kernels: `GenerateKernelArray(16, n, s, table, CatmullRom)` (§7.2): 16 phases of `n` integer weights.
- Rows: for each output row at `posY` (start, then `+= stepY`), the vertical window is the source rows
  `posY >> 16` … `+ nY − 1`, each clamped to [0, H − 1], horizontally filtered (§7.3) into a ring of `nY` rows
  (every output row refilters the whole window; the result is the same); the vertical pass (§7.3) uses phase
  `(posY >> 12) & 15`.
- Each channel (alpha included) is accumulated as `Σ value·weight`, then `(sum + 0x2000) >> 14` clamped to [0, 255].

### 7.2 Kernels *[compat]*

- Catmull-Rom (`0x007BF7B0`, `x = |t|`): `1.5x³ − 2.5x² + 1` below 1, `−0.5x³ + 2.5x² − 4x + 2` below 2, else 0.
- Tent (`0x007AA0E0`): `1 − x` below 1, else 0.
- `GenerateKernelArray(phases, n, s, out, k)` (`0x007C0B80`): for phase `p` the taps are `k(x)` at
  `x = −(n/2 − 1)·s − p·s/phases + i·s`, `i = 0 … n − 1`, normalised to sum 16384 (`GenerateKernel` `0x007AA860`), then
  converted to integers.
- `ConvertKernelToInteger(first, last, out, total)` (`0x007BFF20`): truncates each weight; the excess
  `E = Σ trunc − total` is spread over the taps (each gets `E / remaining` rounded away from zero, `E` reduced by it)
  but **added** to the weights instead of subtracted *[quirk: the integer weights miss `total` by twice the
  truncation error]*.

### 7.3 Row passes (`fn:Resample`) *[compat]*

- Horizontal N-tap (`0x007BF820(out, src, outW, srcW, table, n, pos, step, scratch)`): output pixel `j` uses the
  phase `(pos >> 12) & 15` and the source pixels from `pos >> 16`; outside [0, srcW) the edge pixel is repeated
  (through `scratch`); `pos += step`.
- Vertical N-tap (`0x007BFFC0(out, rows, width, weights)`): per column `Σ row[k][x]·weight[k]`.
- `0x007C0E70` builds the ring: a vector of `n` empty row vectors.

### 7.4 The 2-tap sub-pixel shift helpers *[compat]*

Used by the region screen's thumbnail shift (region-screen.md; its body `0x007AE160` and `GenerateKernel` are that
owner's), with tent weights summing to 16384:

- Horizontal (`0x007AA110(out, src, count, srcW, weights, pos)`): output `j` mixes source `pos >> 16` and the next,
  `pos += 0x10000`; pixels outside [0, srcW) count as **0** (transparent black), unlike §7.3.
- Vertical (`0x007AABB0(out, rows, count, weights)`): mixes two rows.

Both round and clamp as §7.1.

## 8. The album picture (`cWinBMPViewGVExtra`) *[compat]*

A `cWinBMPView` with a small and a large size (snapshots.md §3.2), vtable `0x00AB9980`. Fields: fit flag `+0x140`,
large mode `+0x141` (constructor `0x007B6870`: 1), large rectangle `+0x144`…`+0x150` (0, 0, 16, 16), small rectangle
`+0x154`…`+0x160` (0, 0, 32, 32), a double-click timer `+0x168`. Destructor `0x007B68F0`.

- `SetSizeInOut(large)` (`0x007B63D0`): unchanged → true. Else stores it, pulls the window to the front (slot 23),
  sets the view's fit flag (large: `+0x140`; small: 0 when Scroll Lock is on, else `+0x140`), centred mode 1, zoom
  keys and scrolling off, the window area (slot 55) to the large or small rectangle, and sends
  `(0x9127484A, ID, large, 0)` to the parent (window slot 144). True.
- `SetShouldFitImageAtSizeIn(b)` (`0x007B6380`): stores `+0x140` and applies it as above for the current mode.
- `GZOnMouseDownL` (`0x007B6970`): a second click within 500 ms toggles the size (and resets the timer); otherwise
  the timer restarts and the view's left click follows.
- `Init` (`0x007B6940`): the view's `Init`, bitmap flag 2 (no background fill) on, then `SetSizeInOut(1)`, which does
  nothing because the constructor already set large mode *[quirk]*. `DoMessage` (`0x007B6500`) is the view's.

## 9. The texture tile packer (`cRZTileSetup`) *[compat]*

Splits a `W × H` image into square textures plus small power-of-two textures for the leftover strips. Used by the
SimGL buffer cache with tile size 256, minimum texture 16 and alignment 1 (gz-graphics.md). The result is a deque of
textures (20 bytes each: a vector of placements, texture width, texture height); a placement is
`{srcX, srcY, dstX, dstY, w, h}` (24 bytes). `GetTileCount` (`0x00A6CBC0`) is the number of textures; the deque's
clear is `0x00A6CE10`, its push-back slow path `0x00A6CD60`.

`Init(&size, minTex, tile, align)` (`0x00A6CF00`):

1. Clears the result; `W' = W` rounded up to `align`, `H'` likewise; `area = W'·H'`; `nx = W' / tile`,
   `ny = H' / tile`.
2. Grid: for each row then column, one `tile × tile` texture holding one placement `{(tx·tile, ty·tile), (0, 0)}` of
   size `tile` clipped to the image (`min(tile, W − x)`, `min(tile, H − y)`). `area −= tile²·nx·ny`.
3. Leftover strips: the right strip `(nx·tile, 0, W', ny·tile)` when `nx·tile < W` and `ny > 0`; the bottom strip
   `(0, ny·tile, W', H')` when `ny·tile < H`. The largest placement count per texture (`+0x34`) starts at 1.
4. Packing, with a free-rectangle list for the current texture and two strip lists A and B (A empty, B the strips):
   while B is not empty, swap A and B, then while A is not empty:
   - With no free rectangle, a new texture starts at `w = h = minTex`, `a = w·h`, and repeats: stop unless
     `a < area` and `a < tile²`; double `w`; stop unless `2a < area`; double `h`; `a ×= 4`. So it is square or twice
     as wide as tall. Its free list is the whole texture.
   - The first strip of A goes into the free rectangle with the largest overlap `min(fw, sw)·min(fh, sh)` (the first
     one on ties): one placement from the strip's top-left to the free rectangle's top-left, of the overlap size
     clipped to the image. `+0x34` keeps the maximum count; `area −=` the overlap.
   - The free rectangle is replaced by its remainders below the piece (full width) and right of it (piece height);
     the strip by its remainders below (back into A) and right (into B). Remainders are inserted at the front of their
     list, merged with a rectangle sharing a full edge when one exists (`0x00A6CC10`).

## 10. Function inventory

`cWinBMPView`: `0x007B6780`, `0x007BF9D0`, `0x007BFA30`, `0x007BFB30`, `0x007BFB60`, `0x007BFBA0`, `0x007BFBD0`,
`0x007BFD60`, `0x007BFDD0`, `0x007BFDE0`, `0x007C0120`, `0x007C01D0`, `0x007C0220`, `0x007C0390`, `0x007C0730`,
`0x007C0790`, `0x007C0EF0`, `0x007C10F0`, `0x007C1500`, `0x007C1670`.
`cWinBMPViewGVExtra`: `0x007B6380`, `0x007B63D0`, `0x007B6500`, `0x007B6870`, `0x007B68F0`, `0x007B6940`,
`0x007B6970`.
`fn:Resample`: `0x007AA0E0`, `0x007AA110`, `0x007AABB0`, `0x007BF7B0`, `0x007BF820`, `0x007BFFC0`, `0x007C0E70`.
`fn:GenerateKernelArray`: `0x007C0B80`. `fn:ConvertKernelToInteger`: `0x007BFF20`.
`cRZTileSetup`: `0x00A6CBC0`, `0x00A6CC10`, `0x00A6CD60`, `0x00A6CE10`, `0x00A6CF00`.
