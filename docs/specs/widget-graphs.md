# `cGZGraph` and its subclasses: the graph widgets (behaviour spec)

Scope: the graph owners as tools/coverage.py assigns them on `bb641878`:
- `cGZGraph` (64 functions, 3,758 bytes, §1–§6): the base graph window: background, plot area, two axes with
  ticks and labels, title, drawing helpers, dynamic decoration objects;
- `cGZBarGraph` (5 functions, 467 bytes), `cGZLineGraph` (2, 188), `cGZScatterGraph` (3, 169) (§7): what those
  owners hold (constructors, hit test, redraw);
- `cGZPieGraph` (78 functions, 7,833 bytes, §8): the pie graph, plus point-series code of the scatter graph that
  folding put in this owner (lane re-gzpie7).

Most of the drawing (axes, grids, ticks, labels, title, series) is in folded functions that tools/coverage.py
attributes to `<shared>` and that are out of this scope; they are cited by their window slot (§5.2).

Related:
- widget-toolkit.md (the `cGZWin` base; flag `0x10000` = private buffer);
- graphs-window.md §4 (the city graphs window: it creates line, bar and pie graphs and styles them through the
  `G<n>` slots of §2, the same numbering);
- span-render-8.md 10.7 and `docs/abi/interfaces.tsv` (`cIGZBuffer` slots: 6 `Lock`, 7 `Unlock`, 8 the lock
  query).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the mouse-move call, the constants
`0x00B17C1C`, `0x00A84D2C` and `0x00A80AA8` and the vtables were checked on the bytes. Vtables read: `cGZGraph`
window `0x00ADE188` (178 slots) and `cIGZGraph` `0x00ADE450` (55 slots); bar `0x00ADE648`/`0x00ADE568`/`0x00ADE910`;
line `0x00ADF2E8`/`0x00ADF208`/`0x00ADF5B0`; scatter `0x00ADEA90`/`0x00ADE9B0`/`0x00ADED58`; pie
`0x00ADEEC0`/`0x00ADEDE0`/`0x00ADF188`. Written 2026-10-23 (re-gzgraph7).

Conventions:
- Offsets are from the window start. `cIGZGraph` (IID `0x02CF2351`) is at `+0xD8`; each subclass has a data
  interface at `+0x22C`. "G n" is `cIGZGraph` slot n; "window slot n" a slot of the window vtable (slots 151–177
  are the graph's own virtuals).
- `a` is an axis index, 0 or 1 (x, y). "Unset" sentinels: `0x7FFFFFFF` for positions, `0x80000000` for colours
  ("keep the font's"), `FLT_MIN` (`0x00800000`, the global `0x00B17C1C`) for axis values.

## 1. Identity and fields *[compat]*

- **`QueryInterface`** (`0x009B1E73`): `0x02CF2351` → `+0xD8` (null for null), AddRef'd, true; else `cGZWin`'s.
  The `+0xD8` slot-0 thunk is `0x009B771E`.
- **Window overrides**: 0, 4 `Init`, 5 `Shutdown`, 55 (`<shared>`), 88 `GZPaint`, 134, 136, 138 mouse, 148
  destructor (`0x009B9621`; body `0x009B7729`); extra slots 151–177 (§5.2).
- The base part ends at `+0x22C`; subclasses are `0x260` (line), `0x268` (bar) and `0x28C` (pie) bytes.

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | (used by the drawing slots) | 1 |
| `+0xE0..+0xEC` | plot ("graph") area | unset ×4 |
| `+0xF0` | default fonts loaded (byte, §5.1) | 0 |
| `+0xF4` | title (`cRZString`) | empty |
| `+0x108..+0x114` | title position | unset ×4 |
| `+0x118`, `+0x11C` | title font (counted), colour | null, `0x80000000` |
| `+0x120`, `+0x124` | (used by the drawing slots) | 32, 1 |
| `+0x128` | background fill colour | `0xFFFFFFFF` |
| `+0x12C`, `+0x12D` | background fill on, outline on (bytes) | 1, 1 |
| `+0x130` | background outline colour | 0 |
| `+0x134`, `+0x13C` | plot-area fill on, outline on | 0, 1 |
| `+0x138`, `+0x140` | plot-area fill colour, outline colour | `0x12345678` (unset), 0 |
| `+0x144`, `+0x148` | background pattern, plot-area pattern (`cIGZBuffer`, counted) | null |
| `+0x14C` | (byte) | 0 |
| `+0x14D + a` | major ticks on | 1 |
| `+0x14F + a` | minor ticks on | 0 |
| `+0x151 + a` | major grid on | 1 |
| `+0x153 + a` | minor grid on | 0 |
| `+0x155 + a` | tick text drawn | 0, 1 |
| `+0x158` | axis line colour | 0 |
| `+0x15C + 4a`, `+0x164 + 4a` | axis minimum, maximum (float) | `FLT_MIN` |
| `+0x16C + 4a`, `+0x174 + 4a` | major, minor tick spacing (float) | `FLT_MIN` |
| `+0x17C + a` | major tick decimal places (byte) | `0x80` (automatic) |
| `+0x180 + 4a`, `+0x188 + 4a` | major, minor tick length | 4, 2 |
| `+0x190` | tick colour | 0 |
| `+0x194`, `+0x198` | (used by the drawing slots) | 1, 2 |
| `+0x19C + 4a` | axis zero position | `0x7FFFFFFF` |
| `+0x1A4 + 0x14a` | axis label (`cRZString`) | empty |
| `+0x1CC + 0x10a` | axis label position | unset ×4 |
| `+0x1EC`, `+0x1F0` | (used by the drawing slots) | 1, 0 |
| `+0x1F4 + 4a`, `+0x1FC + 4a` | axis label font (counted), colour | null, `0x80000000` |
| `+0x204 + 4a`, `+0x20C + 4a` | tick font (counted), colour | null, `0x80000000` |
| `+0x214 + 4a` | axis label rotation | 0 |
| `+0x21C` | native magenta (graphics system slot 34 with 255, 0, 255) | |
| `+0x220`, `+0x224` | hit-test data block, its size | null, 0 |
| `+0x228` | dynamic graph objects (a list, owned) | empty |

## 2. The `cIGZGraph` slots (G 0–54) *[compat]*

Vtable `0x00ADE450`. G 1, 2 are the shared AddRef/Release, G 54 the shared `AsIGZWin`. Setters store and return
nothing (no redraw); fonts are AddRef'd when they change (old one released) and a colour of `0x80000000` leaves
the stored colour unchanged.

| Slot | Address | Behaviour |
|---|---|---|
| 3 `SetBackgroundFill(on)` | `0x009B1F4B` | `+0x12C` (a shared byte setter) |
| 4 `SetBackgroundOutline(on)` | `0x009B1F41` | `+0x12D` |
| 5 `SetBackgroundFillColor(c)` / 6 `…OutlineColor(c)` | `0x009B1F55` / `0x009B1F5F` | `+0x128` / `+0x130` |
| 7 `SetBackgroundPattern(image)` | `0x009B1F8E` | `+0x144`; AddRef the new, release the old (no equality test) |
| 8 `SetGraphAreaOutlineAndFill(outline, fill)` | `0x009B1F69` | `+0x13C`, `+0x134` |
| 9 `SetGraphAreaOutlineColor(c)` / 10 `…FillColor(c)` | `0x009B1F7A` / `0x009B1F84` | `+0x140` / `+0x138` |
| 11 `SetGraphBackgroundPattern(image)` | `0x009B1FB5` | `+0x148`, as G 7 |
| 12 `SetGraphArea(rect*)` / 13 `GetGraphArea(out)` | `0x009B1F1D` / `0x009B1F2F` | `+0xE0..+0xEC` |
| 14 `CompleteRedraw(buffer)` | `0x009B2431` | §5.2 |
| 15 / 16 `SetAxisMinValue`/`MaxValue(a, f)` | `0x009B1FDC` / `0x009B1FEE` | floats |
| 17 `SetAxisZeroPosition(a, p)` | `0x009B2000` | |
| 18 `SetAxisLineColor(c)` | `0x009D415E` | `+0x158` |
| 19 `SetAxisLabel(a, s, resetPosition)` | `0x009BA932` | copy `s`; `resetPosition` (byte) → label position left := unset |
| 20 `SetAxisLabelPosition(a, rect*)` | `0x009B2012` | |
| 21 `SetAxisLabelFont(a, font, colour)` | `0x009B202F` | |
| 22 `SetAxisLabelRotation(a, r)` | `0x009B207B` | |
| 23 `SetMajorTickUsage(a, v)` | `0x009B208D` | `v ≠ 0` |
| 24 `SetMajorTickSpacing(a, f)` | `0x009B20A0` | |
| 25 `SetMajorTickDivisions(a, n, nice)` | `0x009B20B2` | §3 |
| 26 `SetMajorTickGridUsage(a, v)` | `0x009B211A` | `v ≠ 0` |
| 27 `SetMajorTickDecimalPlaces(a, n)` | `0x009B212D` | clamped to 0…9 |
| 28 `SetMajorTickLength(a, n)` | `0x009B214F` | |
| 29 `SetAxisTickFont(a, font, colour)` | `0x009B2174` | |
| 30 `SetAxisTickTextDrawing(a, v)` | `0x009B2161` | `v ≠ 0` |
| 31 `SetTickColor(c)` | `0x009B22A8` | `+0x190` |
| 32 `SetMinorTickUsage(a, v)` | `0x009B21C0` | **`v == 1`** (other non-zero values switch it off) |
| 33 `SetMinorTickSpacing(a, f)` | `0x009B21D3` | |
| 34 `SetMinorTickDivisions(a, n, nice)` | `0x009B21E5` | §3 |
| 35 `SetMinorTickGridUsage(a, v)` | `0x009B2283` | `v == 1` |
| 36 `SetMinorTickLength(a, n)` | `0x009B2296` | |
| 37 `SetTitle(s, resetPosition)` | `0x009BA967` | copy `s`; `resetPosition` → title position left := unset |
| 38 `SetTitlePosition(rect*)` | `0x009B22B5` | |
| 39 `SetTitleFont(font, colour)` | `0x009B22C7` | |
| 40 `DoHitTest(point, &result)` | `0x0093877E` | false (a shared stub; the bar graph overrides it, §7.1) |
| 41 `AllocateHitTestData(size)` | `0x009B25CC` | §4 |
| 42 `GetHitTestResultData` | `0x0076B620` | the block `+0x220` (a shared getter) |
| 43 `DrawLine(buffer, x1, y1, x2, y2, colour, width)` | `0x009B2602` | §4 |
| 44 `DrawRectangleOutline(buffer, rect*, colour, width)` | `0x009B265D` | the free function `DrawRectangleOutline` |
| 45 `Fill(buffer, colour, rect*)` | `0x009B3FB4` | §4 |
| 46 `Blt(image, buffer, a, b)` | `0x009B2678` | §4 |
| 47 `AddDynamicGraphObject(o)` / 48 `Remove…(o)` | `0x009B6A64` / `0x009B695C` | append to / remove from the `+0x228` list |
| 49–53 `AddMiscGraphTextInfo`, `…Line`, `…Rect`, `…Arrow`, `…Picture` | `0x009B963D`, `0x009B5A50`, `0x009B5A84`, `0x009B5AB1`, `0x009B361A` | §4 |

## 3. Tick divisions

- **`SetMajorTickDivisions(a, n, nice)`** (G 25): `s = (max_a − min_a) / n`, with `n` read as unsigned (2³² is
  added for a negative value). With `nice` (a byte): `k :=` window slot 162 `(a, s)`, raised to 4 when below 5, and
  `s :=` window slot 163 `(a, k)`. Store `s` as the major spacing.
- **`SetMinorTickDivisions(a, n, nice)`** (G 34): `s = major_a / n` (unsigned `n` as above). Window slot 162
  `(a, current minor spacing)` is called and its result dropped. With `nice`: `k := slot 162 (a, s)`; when
  `k < 2`, `v := slot 163 (a, 2)`; if `major_a × 0.5 < v` the minor ticks are switched off; store `v`. Otherwise
  (and without `nice`) store `s`.

Slots 162 and 163 are `<shared>` (a count and a "nice" spacing for the axis).

## 4. Helpers and decoration objects

- **Lock discipline** (G 43, 45, 46): if the buffer is not locked (buffer slot 8), lock it with `0x8004` (lines),
  `0x8001` (fills) or `0x8010` (blits); a failed lock → false; after the call, unlock only what was locked here.
  The result is the buffer call's: slot 28 `DrawLine(x1, y1, x2, y2, colour, width)`, slot 20
  `Fill(colour, &rectCopy, 0)`, slot 29 `Blt(image, a, b, 0)`.
- **`AllocateHitTestData(size)`** (G 41): when `size` exceeds the current size the old block is freed; then the
  size is stored and a new block allocated (global heap). *Quirk*: a size not larger than the current one
  allocates a new block without freeing the old one.
- **Decoration objects** (G 49–53): each allocates (global heap) and constructs its object — text info `0x48`
  bytes, line `0x20`, rectangle `0x2C`, arrow `0x2C`, picture `0x18` — and passes it to G 47 (also a null from a
  failed allocation), then returns it. The graph owns them: `Shutdown` deletes them. The line object (vtable
  `0x00ADE0D4`, `+0x18` = 0, `+0x1C` = 1) draws itself with `0x009B26CC`: lock `0x8004`, then the graph's G 43 with
  the six values at `+0x08..+0x1C` (`x1, y1, x2, y2, colour, width`), unlock.

## 5. Lifecycle, input and painting

### 5.1 Lifecycle

- **`Init`** (`0x009B1E4A`): once: `SetFlag(0x10000, 1)` (a private buffer), then `cGZWin::Init`.
- **Default fonts** (window slot 151, `0x009B7774`): through the font service (`RZ::FO()`, slot 11 `(name, 0,
  size, 0, 1)`), trying `"arial"` and then `"helvetica"`: no title font → size 24 into G 39; per axis, no label font
  → size 16 into G 21, no tick font → size 14 into G 29 (colours `0x80000000`); the local references are released.
  Then `+0xF0 := 1`.
- **`Shutdown`** (`0x009B5990`): delete every dynamic object (its slot 0 with 1) and clear the list; release the
  title font, both patterns, and the label and tick fonts of both axes; free the hit-test block; `+0xF0 := 0`;
  `cGZWin::Shutdown`. Thunk `0x009B621D`.
- **Destructor body**: vtables, `Shutdown`, the list, the two label strings, the title, `~cGZWin`.

### 5.2 Painting

`GZPaint` (`0x009B1EA0`) calls G 14 with null and returns true.

**`CompleteRedraw(buffer)`** (G 14, `0x009B2431`): `buffer` null → the window's buffer to draw to (`+0x68`); still
null → nothing. Default fonts not loaded → window slot 151. Then window slots 170 `()`, 166 `(0)`, 166 `(1)`, and
with the buffer: 152 (background, below), 153, 154, 155 `(·, 7)`, 156 `(·, 7)`, 157, 158, 159. In the base,
slots 153–177 are `<shared>` and 157 (the series drawing) is a 3-byte stub that each subclass replaces.

**Background** (window slot 152, `0x009B38A5`), over `{0, 0, W, H}` (from the draw area):
- with a background pattern: lock `0x8010`, G 46 `(pattern, buffer, &rect, &rect)`, unlock;
- else: fill on and lock `0x8001` → buffer slot 18 `(fill colour, &rect)`, unlock; outline on and lock `0x8008` →
  G 44 `(buffer, &rect, outline colour, 1)`, unlock.

### 5.3 Mouse

Left down: `GZSetCapture`, true. Left up: release when capturing, true. Move while capturing: window slot 121 with
`(x, y)`, result unused; true.

## 6. Quirks

1. `AllocateHitTestData` leaks the old block unless the new size is larger (§4).
2. `SetMinorTickUsage` and `SetMinorTickGridUsage` treat only 1 as on.
3. The mouse-move test has no effect (§5.3).
4. The setters do not invalidate; callers redraw.

## 7. The small subclass owners

### 7.1 `cGZBarGraph`

- **`QueryInterface`** (`0x009B2943`): `0x22CF2351` → `+0x22C`; else `cGZGraph`'s. Thunk `0x009B9EB2`.
- **Constructor** (`0x009B9DED`): `cGZGraph`'s, the vtables, `+0x230`…`+0x258` := 0, `+0x25C` := 1.0, `+0x260` := 1.0,
  `+0x264` := 0.75. Since `+0x234` (orientation) is 0: x-axis major ticks and grid off, y-axis on (`+0x14D` 0,
  `+0x14E` 1, `+0x151` 0, `+0x152` 1); x tick text on (`+0x155` 1).
- **`CompleteRedraw(buffer)`** (G 14, `0x009B2A0A`): as the base (buffer or `+0x68`, nothing when null), but first
  the data interface's slot 36.
- **`DoHitTest(point, &result)`** (G 40, `0x009B4E24`): for each series `s` (records of 12 bytes from `+0x238`, each
  a vector of values) and each value `i`: the data interface's slot 27 `(i, s, &rect)`; if the point is in `rect`:
  `AllocateHitTestData(12)`, block `+4 := s`, `+8 := i`, `*result := 1001` (`0x3E9`), true. None → false.

### 7.2 `cGZLineGraph` and `cGZScatterGraph`

- **Line constructor** (`0x009BA988`): `cGZGraph`'s, the vtables, `+0x230`…`+0x258` := 0, `+0x25C` := `0x80000000`;
  `+0x230` (orientation) 0 → the same axis defaults as the bar graph (without the tick-text change).
  **`RemoveCategoryNames`** (`0x009BBCB7`): clears the category-name vector.
- **Scatter `QueryInterface`** (`0x009B2A38`): `0x32CF2351` → `+0x22C`; thunk `0x009BA0A4`. **Constructor**
  (`0x009BA033`): `cGZGraph`'s, the vtables, `+0x230` := 1, `+0x234`…`+0x24C` := 0, `+0x250` := `0x80000000`;
  tick text on for both axes.

## 8. `cGZPieGraph` and the folded scatter-data code *[compat]*

The `cGZPieGraph` owner (78 functions, 7,833 bytes) holds the pie graph and, by folding, the point-series code the
scatter graph's vtables call (§8.6). Pie object `0x28C` bytes; data interface `cIGZPieGraph` (IID `0x42CF2351`) at
`+0x22C`.

### 8.1 Identity and fields

- **`QueryInterface`** (window slot 0, `0x009B2BEF`): `0x42CF2351` → `+0x22C`; else `cGZGraph`'s. Thunks
  `0x009BA307` (G 0), `0x009BA312` (data slot 0).
- **Overrides**: window 4 `Init` (`0x009B28C9` = `cGZGraph::Init`), 5 `Shutdown` (`0x009B6557`), 148 destructor
  (`0x009BAB87`, body `0x009BA349`), 151 default fonts (`0x009B946F`), 157 `DrawGraphData` (`0x009B6AE5`); G 12
  `SetGraphArea` (`0x009B2F92`), G 14 (`0x009B2935`, the base redraw).
- **Constructor** (`0x009BA216`): `cGZGraph`'s, the three vtables, then the values below; it also turns off every
  tick, grid and tick text of both axes (`+0x14C`…`+0x156` := 0).

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0x230` | style: 0 flat disc, 1 3D (masks) | 0 |
| `+0x234` | percentage texts on (byte) | 1 |
| `+0x238..+0x240` | the values (`vector<float>`) | empty |
| `+0x244` | percentage decimal places | 0 |
| `+0x248..+0x250` | series info (`vector`, 0x1C-byte records: `+0` colour, `+4` text colour, `+8` name) | empty |
| `+0x254` | minimum fraction shown with a text | 0.0 |
| `+0x258`, `+0x25C` | percentage font (counted), colour | null, `0x80000000` |
| `+0x260`, `+0x264` | shading mask and shape mask (`cIGZBuffer`, counted) | null |
| `+0x268..+0x274` | disc rectangle | `0x7FFFFFFF`, `0x7FFFFFFF`, `0x80000000`, `0x80000000` |
| `+0x278..+0x284` | the shape mask's opaque bounding box (min x, min y, max x + 1, max y + 1) | 0 |
| `+0x288` | 3D depth (pixels) | 0 |

`Shutdown`: release both masks and the percentage font, then `cGZGraph::Shutdown`. Destructor body: vtables,
`Shutdown`, the info and value vectors, the base body.

### 8.2 The `cIGZPieGraph` slots

Vtable `0x00ADF188`, 31 slots. 1/2 are AddRef/Release thunks (`0x009B9EC8`, `0x009B9ED3`, `this − 0x22C`), 3/4
the `Init`/`Shutdown` thunks (`0x009BA31D`, `0x009BA333`), 29 `AsIGZWin` (`0x009B9EF4`). "Index `−1`" means the last
or all, as stated.

| Slot | Address | Behaviour |
|---|---|---|
| 5 `SetStyle(s)` | `0x009B2C1C` | `+0x230` (a shared dword setter) |
| 6 `Set3DViewMaskBuffers(shade, shape)` | `0x009B2C77` | §8.3 |
| 7 `GetLineFromCenterOfDiskToEdge(f, &centre, &edge)` | `0x009B2DEB` | §8.3 |
| 8 `GetPositionForPercentageText(f0, f1, &out)` | `0x009B2F12` | §8.3 |
| 9 `GetPositionForPercentageText(i, &out)` | `0x009B69A6` | normalise (slot 10), `f0` = the sum of fractions before `i`, slot 8 `(f0, f0 + fᵢ, out)` |
| 10 `MakeNormalizedDataArray(&values, &out)` | `0x009B952D` | `out[i] = |vᵢ| / max(Σ|v|, 0.0001)` |
| 11 | `0x009BA0D0` | G 14 through the graph interface (`this − 0x154`) |
| 12 `DrawGraphData(buffer)` | `0x009BA33E` | thunk to window slot 157 |
| 13 | `0x009BA328` | G 12 `SetGraphArea` through the graph interface |
| 14 `SetPercentageTextDecimalPlaces(n)` | `0x009B2C26` | `min(n, 9)` (unsigned) |
| 15 `SetPercentageTextFont(font, colour)` | `0x009B28FB` | as the base font setters |
| 16 `SetMinPercentageToDisplay(f)` | `0x009B2C38` | below 0 → 0; above 1 it is read as a percentage (× 0.01) and then capped at 1 |
| 17 `SetSeriesCount(n)` | `0x009BBF9B` | resize the info vector to `n`; every record with an empty name takes the default record `i & 7` of the table at `[0x00BAC08C]` (8 records). The values are not resized |
| 18 `InsertSeries(i)` | `0x009BB7E1` | `−1`: append value 1.0 and a default info record; `i <` count: insert both at `i` |
| 19 `RemoveSeries(i)` | `0x009BB86A` | `−1`: drop the last value and info record; `i <` count: erase both |
| 20 `RemoveAllSeries` | `0x009B581D` | slot 19 `(−1)` count times |
| 21 `SetSeriesColor(colour, i, textColour)` | `0x009B583D` | `−1`: all records; `i <` info count: that one |
| 22 `SetSeriesName(name, i)` | `0x009BA3B2` | `−1`: all; else that one |
| 23 `SetSeriesInfo(info*, i)` | `0x009BA41A` | copy the record (colour, text colour, name) |
| 24 `InsertSeriesInfo(info*, i)` | `0x009BB8B8` | `−1`: append; `i <` count: insert |
| 25 `RemoveSeriesInfo(i)` | `0x009BB8F7` | `−1`: drop the last; `i <` count: erase |
| 26 `InsertData(v, i)` | `0x009B96D4` | only for `−1` or `i <` value count. `−1` → `i := count`; with `n = i + 1`, grow the info (slot 17) and the values to `n` when shorter; then slot 28 `(v, i)`. So an existing index is overwritten, not inserted |
| 27 `RemoveData(i)` | `0x009B65A0` | `−1` with exactly one value → 0; `i <` count: erase |
| 28 `SetData(v, i)` | `0x009B58AB` | `−1` with exactly one value → 0; `i <` count: store |
| 30 `AsIGZGraph` | `0x009BA2F4` | `this − 0x22C + 0xD8`, null-safe |

### 8.3 Geometry

- **`SetGraphArea(rect)`** (G 12): store the plot area, then the disc rectangle `:=` the mask bounding box offset by
  the plot area's left and top.
- **`Set3DViewMaskBuffers(shade, shape)`** (slot 6): hold both (AddRef new, release old). With `shade` set: no
  `shape` → return; else style := 1. With `shape` and `Lock(0x8080)`: the key colour is its pixel (0, 0) (buffer
  slot 21); the bounding box of every other pixel goes to `+0x278..+0x284` (max + 1). No such pixel: disc rectangle
  `{0, 0, W, H}` and depth 0. Otherwise the disc rectangle is the box offset by the plot area's left and top, and
  depth `:= shade.H − (max y + 1)`. Unlock.
- **`GetLineFromCenterOfDiskToEdge(f, &c, &e)`** (slot 7): `θ = 2π·f + π/2`; `c` = the disc centre (`(l + r) / 2`,
  `(t + b) / 2`, integer); `e.x = c.x + ftol(cos θ · w / 2)`, `e.y = c.y − ftol(sin θ · h / 2)` with `w`, `h` the
  disc's size. Style 1 computes the second term as `sin θ · w/2 · (h / w)`. Other styles leave `e` unwritten.
  So fraction 0 points straight up and the slices run counter-clockwise.
- **`GetPositionForPercentageText(f0, f1, &out)`** (slot 8): slot 7 at `(f0 + f1) / 2`; `k = 0.6`, or 0.8 when
  `f1 − f0 ≤ 0.1`; `out = c + ftol((e − c) · k)` per coordinate.

### 8.4 Drawing (`DrawGraphData(buffer)`, window slot 157)

Normalise the values (slot 10). Let `fᵢ` be the fractions, `s` the running start, `cᵢ`/`tᵢ` the record's colour and
text colour, and "radial(s, colour)" a G 43 line from the centre to slot 7's edge point (width 1); the last slice
closes on the first edge point.

**Style 0 (flat):**
1. Disc := the plot area inset by 1 on each side, made square (the longer side shortened).
2. Lock `0x8100`: `FillEllipse(buffer, disc, 0xEEEEEEEE)`, unlock.
3. Each slice: lock `0x8004`, radial(s) and radial(s + fᵢ) in `cᵢ`, unlock; `p :=` slot 8 `(s, s + fᵢ)`; unless `p`
   equals the slice's end point, lock `0x8400` and flood-fill from `p` with `cᵢ` (`0x008DA370`), unlock.
4. Lock `0x8200`: `RZDrawDrawCircle(buffer, disc, 0, 1)` (black outline), unlock.
5. Each slice again: the two radials in black (locked with `0x8200`, unlocked with `0x8004`); then, with texts on,
   a percentage font and `fᵢ ≥` the minimum: the text `sprintf("%.Nf%%", fᵢ · 100)` (`N` the decimal places, the
   format `0x00ADE17C` with its digit patched), centred on `p` (font slots 37 line height and 49 width), drawn with
   font slot 51 in `tᵢ` (`0x8000000` when unset).

**Style 1 (3D):**
1. Lock `0x8010`: blit the shape mask (G 46) at the disc rectangle `depth + 1` times, moving down one pixel each
   time (the extruded side); unlock.
2. Each slice: the two radials in `cᵢ`; for an edge point below the centre, also a vertical line of length `depth`
   from it (buffer slot 28 for the first, G 43 for the second); then the flood fill as in style 0.
3. `SaturationMaskBlt(buffer, shade, its rectangle, plot-area origin, 0)` (`0x009B58DE`: clip with
   `GenericBufferClip`, then `SaturationMaskBltNoClip`) shades the disc.
4. The percentage texts as in style 0 step 5, **without** the minimum check.

### 8.5 Fonts

Window slot 151 (`0x009B946F`): without a percentage font, the font service's `"arial"` (or `"helvetica"`) at 12 into
slot 15 (colour `0x80000000`); then the base default fonts (§5.1).

### 8.6 Folded scatter and line code in this owner

Called from the scatter graph's vtables (window `0x00ADEA90`, data `0x00ADED58`); the scatter keeps point series at
`+0x234` (12-byte records, each a `vector` of 8-byte `(x, y)` float points), series info at `+0x240` (0x24-byte
records: `+0x08` colour, `+0x0C` name, `+0x20` "connect the points" byte) and a marker font at `+0x24C`.

| Function | Reached as | Behaviour |
|---|---|---|
| `0x009B625F` | scatter window 5 | release the marker font, `cGZGraph::Shutdown` |
| `0x009B9226` | scatter window 151 (also data 29 via `0x009BA107`) | marker font `"windings"`, else `"dingbats"`, size 12, into data slot 3; then the base fonts |
| `0x009B63F8` | scatter window 157 (data 28 via `0x009BA0F1`) | with the marker font's slot 55 `(buffer)` true: each series, each point mapped by window slot 160 `(axis, value, 1)`; with "connect", a G 43 line from the previous point in the record colour; then data slot 30 `(buffer, &point, series)` (the marker); finally font slot 56 `(buffer)` |
| `0x009B6280` / `0x009B633C` | data 25 / 26 (via `0x009BA0DB`, `0x009BA0E6`) | the largest / smallest x (axis 0) or y (axis 1) over all points (starting from ∓`FLT_MAX`) |
| `0x009B92E4` | data 22 (via `0x009BA0FC`) | the plot rectangle (`<shared>` `0x009B799D`) made square by shortening its longer side |
| `0x009B932A` / `0x009B9385` | data 16 / 15 | insert point `(x, y)` into series `s` at index `i` (`i = −1` appends; `s = −1` with one series → 0) |
| `0x009B93A9` / `0x009B93F1` | data 20 / 19 | set point `i` of series `s` (`i = −1` appends) |
| `0x009B9415` | data 21 | append every point of a given vector to series `s` (`−1` with one series → 0) |
| `0x009BA183` / `0x009BA1EB` | data 10 / 12 | set a series name (`−1`: all) / copy a series info record |
| `0x009BA112` | scatter destructor body | vtables, `0x009B625F`, the two vectors, the base body |
| `0x009B29FC`, `0x009B9EDE`, `0x009B9EE9` | bar data 31/30 | forward to data slot 32; bar G 14 |

Container helpers of the same owner (vector copy, insert and erase for the 8-, 12-, 0x1C- and 0x24-byte element
types): `0x009B356F`, `0x009B5948`, `0x009B5965`, `0x009B679A`, `0x009B68F8`, `0x009B692A`, `0x009B6A0F`,
`0x009B6A2C`, `0x009B73E5`, `0x009B741E`, `0x009B9606`, `0x009B99EF`, `0x009B9D4B`, `0x009BA152`, `0x009BA38F`.

### 8.7 Pie quirks and cases

Quirks:
1. `InsertData` with an existing index overwrites; `SetData`/`RemoveData` with `−1` act only when there is exactly
   one value.
2. The 3D style shows every percentage text, ignoring the minimum.
3. The second radial pass locks with `0x8200` and unlocks with `0x8004`.
4. Slices are filled by flood fill from the text point, so a slice too thin to hold that point bleeds or stays
   unfilled.

Cases:
1. Values 1, 1, 2 (flat): fractions 0.25, 0.25, 0.5; texts `"25%"`, `"25%"`, `"50%"` at 60% of the radius; the
   first slice spans from straight up a quarter turn counter-clockwise.
2. `SetMinPercentageToDisplay(5)` stores 0.05; `(0.5)` stores 0.5; `(250)` stores 1.0.
3. `InsertData(7, −1)` on an empty pie: one value 7 and one info record, filled from default record 0.

## 9. Function inventory

`cGZGraph` (64): `0x009B1E4A` `Init`, `0x009B1E73` QI, `0x009B1EA0` `GZPaint`, `0x009B1EB0`/`0x009B1EBA`/
`0x009B1ED3` mouse, `0x009B1F1D`–`0x009B22C7` the setters of §2 (G 4–13, 15–39), `0x009B2431` G 14, `0x009B25CC`
G 41, `0x009B2602` G 43, `0x009B265D` G 44, `0x009B2678` G 46, `0x009B26CC` the line object's draw,
`0x009B293A` G 43 thunk, `0x009B361A` G 53, `0x009B38A5` slot 152, `0x009B3FB4` G 45, `0x009B5990` `Shutdown`,
`0x009B5A50`/`0x009B5A84`/`0x009B5AB1` G 50–52, `0x009B621D` `Shutdown` thunk, `0x009B695C`/`0x009B6A64`
G 48/47, `0x009B7526` constructor, `0x009B771E` QI thunk, `0x009B7729` destructor body, `0x009B7774` slot 151,
`0x009B9621` deleting destructor, `0x009B963D` G 49, `0x009BA932` G 19, `0x009BA967` G 37, `0x009D415E` G 18.

`cGZBarGraph` (5): `0x009B2943`, `0x009B2A0A`, `0x009B4E24`, `0x009B9DED`, `0x009B9EB2`. `cGZLineGraph` (2):
`0x009BA988`, `0x009BBCB7`. `cGZScatterGraph` (3): `0x009B2A38`, `0x009BA033`, `0x009BA0A4`.

`cGZPieGraph` (78): every address is listed in §8.1–§8.6 (including the folded scatter code and the container
helpers).

Not covered: the `<shared>` drawing slots 153–177, the data interfaces of the bar, line and scatter graphs, and the
decoration objects' own classes, cited by slot.
