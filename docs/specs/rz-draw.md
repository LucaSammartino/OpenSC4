# `RZDraw*`: the 2D drawing helpers (behaviour spec)

Scope: the free-function owners `fn:RZDrawDrawCircle` (6,749 bytes), `fn:RZDrawBltEdge` (2 functions, 1,746),
`fn:RZDrawBltTiled` (1,101), `fn:RZDrawBltStretch` (829), `fn:RZDrawRectangleOutline` (263),
`fn:RZDraw3DRectangleOutline` (210), `fn:RZDrawRaisedRect` (2, 184), `fn:drr_get_color` (160),
`fn:DrawRectangleOutline` (146), `fn:DrawLineAntialiased` (1,584) and `fn:DrawTriangle` (106), plus the span
lists (`cRZSpanList`, 13 functions, 2,529 bytes, with gz-graphics.md; `fn:SC4CreateSpanList`, 911) and the colour
converter (`cRZRGBColorConverter` and `::Format`, 5 functions, 2,586 bytes), as tools/coverage.py assigns them. They draw
into a buffer (`cIGZBuffer`) or a draw context; the windows of the toolkit call them for frames, edges, bars and
outlines.

Related: gz-graphics.md (buffers, draw contexts, interface `0xAB300B2B`), widget-toolkit.md (the window's buffer
`+0x68` and draw context `+0x6C`), widget-bmp.md and scripted-window-director.md §4 (edge frames), widget-wingen.md
(raised rectangles), demand-trend-bars.md (tiled bar rows).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the outlines, the raised rectangle and its
colour helper were followed on the instructions; the long bodies (the circle, the tiled and stretched blits, the
line) are given at the level marked *[summary]*. Written 2026-10-24 (re-rzdraw7).

Rectangles are `(L, T, R, B)`. "Fill" is the draw context's slot 35 with the current colour (slot 21); "blit" is a
buffer's slot 29 `(picture, &src, &dst, flags)` or a draw context's slot 38 `(picture, &src, &dst)`.

## 1. Outlines and raised rectangles *[compat]*

- **`RZDrawRectangleOutline(context, rect, colour, t)`** (`0x008D9360`): context slot 10 (begin); line thickness
  `t` as a float (slot 25; unsigned conversion); colour (slot 21); four fills: top `(L, T, R + t − 1, T + t)`, left
  `(L, T + t, L + t, B − t + 1)`, right `(R + t − 1, T + t, R + 1, B − t + 1)`, bottom `(L, B − t + 1, R + 1, B + 1)`;
  slot 11 (end). With fills half-open on the right and bottom, `t = 1` outlines the inclusive rectangle `[L, R] ×
  [T, B]` **except its top-right pixel** `(R, T)` *[quirk]*; for `t > 1` the right edge has no width and the top edge
  runs `t − 1` pixels past R *[quirk]*.
- **`RZDraw3DRectangleOutline(context, rect, light, dark)`** (`0x008D9470`): colour `light`: top `(L, T, R, T + 1)`
  and left `(L, T, L + 1, B)`; colour `dark`: bottom `(L + 1, B − 1, R, B)` and right `(R − 1, T + 1, R, B)` (half-open
  rectangle, one pixel wide).
- **`drr_get_color(r, g, b, delta)`** (`0x008D8760`): each channel `+ delta`, clamped to 0–255; the result is
  `0xFF000000 | r << 16 | g << 8 | b`.
- **`RZDrawRaisedRect(context, rect, r, g, b, raised)`** (`0x008D9870`): fills `rect` with `0xFFrrggbb`, then
  `RZDraw3DRectangleOutline(rect, drr_get_color(d), drr_get_color(−d))` with `d` = +56 when `raised`, −56 otherwise
  (light top-left for a raised button, dark when pressed). The form `0x008D9900` takes a packed `0x00rrggbb`.
- **`DrawRectangleOutline(buffer, rect, colour, t)`** (`0x00931180`): four lines with the buffer's slot 28
  `(x1, y1, x2, y2, colour, t)`: `(L, T, R − 1, T)`, `(L, B − t, R − 1, B − t)`, `(R − t, T, R − t, B − 1)`,
  `(L, T, L, B − 1)`; true only when all four succeed (the first failure stops).

## 2. Blits *[compat]*

- **`RZDrawBltEdge` (nine-slice frame)**: the picture's source rectangle `src` is the top-left cell of a 3 × 3 grid
  of equal cells (`cw = src` width, `ch` its height); the grid starts at `src`'s top-left. Into `dst` it copies the
  four corners at `dst`'s corners, tiles the top and bottom edge cells along `(L + cw … R − cw)` and the left and right
  edge cells along `(T + ch … B − ch)` (each only when that span is not empty), and tiles the centre cell over the
  interior when the last argument (`fill`) is true. Two forms:
  - `0x008D9550(buffer, picture, &src, &dst, fill)`: corners with the buffer's blit, edges and centre with
    `RZDrawBltTiled`;
  - `0x008D8800(context, picture, &src, &dst, fill)`: through a draw context (begin and end, slots 10 and 11); the
    picture's colour key (its interface `0x86D72B57`, slot 4) becomes the context's transparent colour (slot 21;
    −1 without one); corners with context slot 38, edges and centre with the tiled blit slot 39.
- **`RZDrawBltTiled(buffer, picture, &src, &dst, clip, ox, oy)`** (`0x008D8BC0`) *[summary]*: repeats `src` over
  `dst`, the first tile aligned to `dst`'s top-left shifted by `(ox, oy)` (taken modulo the tile size), every tile
  clipped to the buffer, to `dst` and, when given, to `clip`; each piece is one buffer blit. A **1 × 1** source is
  read as a colour (picture slots 25, 21, converted with the buffer's colour slots) and filled instead, unless that
  pixel is transparent. When the buffer offers a draw context (`0xAB300B2B`), the context's tiled blit (slots 15,
  39) does the work instead.
- **`RZDrawBltStretch(buffer, &dst, &src, clip)`** (`0x008D9020`) *[summary]*: with a draw context, its stretched
  blit (slot 38 after setting the source, slots 4, 9, 5); otherwise a nearest-neighbour software copy: both rectangles
  must be non-empty and `dst` inside the buffer; the steps are `(dstW << 16) / srcW` and `(dstH << 16) / srcH`, sampled
  at the pixel centres (half a step), clipped to the buffer and `clip`; each pixel is read (slot 21), split and packed
  into the destination format (slots 33, 32) and written (slot 22) under lock `0x800`.

## 3. Circles and lines *[summary]*

- **`RZDrawDrawCircle(buffer, L, T, R, colour, colour16, t)`** (`0x008D6CF0`): a circle of radius
  `r = (R − L) / 2` centred at `((L + R) / 2, T + r)`, drawn directly into a **16-bit** buffer (pixels slot 34, row
  pitch slot 35) with the 16-bit value `colour16`. When `t ≥ r` it is a filled disc (`FillEllipse`). It draws nothing
  (returns false) when `t` = 0, a coordinate (or `T + R − L`) lies outside ±4096, or `R < L`. `t = 1`: the 8-way
  symmetric midpoint circle (decision `1 − r`, increments 3 and `5 − 2r`), each point clipped to the buffer. Larger
  `t`: the ring between that circle and the inner one of radius `r − t`, filled row by row and column by column per
  octant; the outer circle's x per row is kept in a 16,384-entry table on the stack. Pixels outside the buffer are
  skipped. Returns true. The exact pixel set of the thick ring is RZD1.
- **`DrawLineAntialiased`** (`0x00985690`, a buffer's line method): only thickness 1 is drawn here (other thicknesses
  go to the buffer's slot 28); the buffer must be locked (slot 8) or lockable (`0x8040`); only 16-, 24- and 32-bit
  buffers are drawn: horizontal and vertical lines as spans, diagonals step one pixel in each axis, and other slopes
  are stepped along the major axis *[summary; RZD1]*.
- **`DrawTriangle(target, x1, y1, x2, y2, x3, y3, colour, …)`** (`0x009328E0`): a span list (`cRZSpanList`) set to
  the three points as a simple polygon (`SetPolygonSimple(points, 3)`) is drawn with the last two arguments
  (`cRZSpanList::Draw`), then freed.

## 4. Span lists (`cRZSpanList`, `fn:SC4CreateSpanList`) *[compat]*

A span list is a vector of 8-byte spans `{int32 y, int16 x1, int16 x2}` (half-open `[x1, x2)`), in row order. The
polygon fill (`SetPolygonSimple`, `0x00932700`, with its edge stepping `0x00932450` and edge setup `0x00931220`) is
specified in gz-graphics.md §3.2 (the two-chain fill and its 16.16 edge accumulators). The rest:

- **Constructor** (`0x00932170`): an empty vector. Vector helpers: copy a range (`0x00931FF0`), fill n copies
  (`0x00932030`), reserve (`0x009320D0`), insert-and-grow (`0x00932180`), `AddSpan(x1, x2, y)` (`0x00932260`, a
  push-back), and a 12-byte-element vector of n zeroed entries for the edges (`0x00932070`).
- **`SetEllipse(L, T, R, B)`** (`0x009322D0`): the list is emptied; rows `y` from `ceil(T − 0.5)` to `ceil(B − 0.5)`;
  with centre `cx = (L + R)/2 − 0.5`, `cy = (T + B)/2 − 0.5` and `h = B − T`: `d = h² / 4 − (y − cy)²`; when `d > 0`,
  `half = √d · (R − L) / h` and the span `[ceil(cx − half), ceil(cx + half))` is added when it is not empty.
- **`Draw(buffer, colour)`** (`0x00931EF0`): clipped to the buffer's bounds (slot 12): spans above the top are
  skipped, drawing stops at the first span at or below the bottom, each span is cut to `[max(x1, L), min(x2, R))` and,
  when not empty, filled one row high with the buffer's slot 19 `(colour, x1, y, x2, y + 1, 0)`, all under lock
  `0x8001`.
- **`FillEllipse(buffer, L, T, R, B, colour)`** (`0x00932670`): under lock `0x8001`, `SetEllipse(L, T, R + 1, B + 1)`
  then `Draw` (the circle's filled case, §3).
- **`SC4CreateSpanList(buffer, out, shift, withValues)`** (`0x007AD400`): scans a 32-bit buffer under lock `0x800`, row
  by row; a run is a stretch of pixels whose byte `(pixel >> shift) & 0xFF` is not 0. For each run it appends the
  run's start and end as linear indices (`row · width + x`) to `out` (a vector of `uint32`), and with `withValues`
  the run's byte values after its end index. The output is used for region masks (region-screen.md).

## 5. The RGB colour converter (`cRZRGBColorConverter`) *[compat]*

Converts pixels between formats through four 256-entry lookup tables, one per source byte.

- **Formats** (`Format::Format(type)`, `0x009312C0`): `{bits, A, R, G, B masks}` for the colour types: 4 → 16-bit
  `0x7C00/0x03E0/0x001F`, no alpha; 5 → 16-bit 565 (`0xF800/0x07E0/0x001F`); 6 → 1555 (alpha `0x8000`); 7 → 4444
  (`0xF000/0x0F00/0x00F0/0x000F`); 8 → 32-bit `0xFF0000/0xFF00/0xFF`, no alpha; 9 → 8888 (alpha `0xFF000000`); others
  all zero.
- **`SetFormats(dst, src)`** (`0x009313B0`): for each source byte position k (0–3) and value v, the table entry is the
  sum over the four channels of `((v << 8k) & srcMask) >> srcShift`, scaled by `0xFFFFFFFF / (srcMask >> srcShift)`
  to 32 bits, shifted down to the destination channel's top bit and masked with `dstMask`; the first table also adds
  the full destination mask of every channel the source lacks (so a missing alpha becomes opaque). The bytes per pixel
  of both formats (bits / 8) are kept (`+0x1000`, `+0x1004`). `0x009312A0` is the shift of a mask's lowest set bit.
- **`Convert(dst, dstPitch, src, srcPitch, width, height)`** (`0x009317F0`): each destination pixel is the sum of the
  table entries of its source bytes, for every pair of 2-, 3- or 4-byte source and destination pixels.

## 6. Quirks

- `RZDrawRectangleOutline` misses the top-right pixel and is only consistent for `t = 1` (§1).
- `RZDrawDrawCircle` writes 16-bit pixels whatever the buffer's format.
- `DrawRectangleOutline` stops at the first failing line, leaving the outline partly drawn.

## 7. Function inventory

§4: `0x00931EF0`, `0x00931FF0`, `0x00932030`, `0x00932070`, `0x009320D0`, `0x00932170`, `0x00932180`,
`0x00932260`, `0x009322D0`, `0x00932670` (and `0x00931220`, `0x00932450`, `0x00932700`, gz-graphics.md), `0x007AD400`.
§5: `0x009312A0`, `0x009312C0`, `0x009313B0`, `0x009317F0`. §1–§3:

`0x008D6CF0` `RZDrawDrawCircle`; `0x008D8760` `drr_get_color`; `0x008D8800`, `0x008D9550` `RZDrawBltEdge`;
`0x008D8BC0` `RZDrawBltTiled`; `0x008D9020` `RZDrawBltStretch`; `0x008D9360` `RZDrawRectangleOutline`; `0x008D9470`
`RZDraw3DRectangleOutline`; `0x008D9870`, `0x008D9900` `RZDrawRaisedRect`; `0x00931180` `DrawRectangleOutline`;
`0x00985690` `DrawLineAntialiased`; `0x009328E0` `DrawTriangle`.
