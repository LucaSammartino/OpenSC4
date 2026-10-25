# `cGZWinOutline`: the outline control (behaviour spec)

Scope: the `cGZWinOutline` owner as tools/coverage.py assigns it on `bb641878` (35 functions, 2,808 bytes;
inventory in §5). The outline control draws a rectangle or polygon outline in one or two colours (a bevel), optionally
filled. Its polygon setter, span builder, `SetArea` and destructor are **unattributed** code (`0x009CBD16`…`0x009CC1F8`,
no owner) and are described here for completeness without being claimed. Script name `GZWinOutline` (class
`0x4303E5B6`, IID `0xE303E5AE`, legacy-control.md's class table).

Related: widget-toolkit.md (the embedded `cGZWin`; fill colour slots); widget-comdirector.md (the class factory);
widget-wingen.md §5.3 (draw-context slots 21, 34, 35).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile, the unattributed ones too; the shared slots and
the paint's result were checked on the instructions. The vtables `0x00AE1A78` (`cIGZWinOutline`, 19 slots) and
`0x00AE1AC8` (window) were read. Written 2026-10-24 (re-gzoutline7).

Conventions: offsets are from the object start; the interface is at `+0` and the window `W` (a `cGZWin`) at `+4`.
"Slot n" is a `cIGZWinOutline` slot, "window slot n" a slot of `W`. The draw area is `W`'s `+0x24`…`+0x30` (`l`,
`t`, `r`, `b`); `H` is `b − t`. Colours 1 and 2 are packed `0xFFRRGGBB`; colour 0 is `W`'s fill colour.

## 1. Identity and fields *[compat]*

- **Factory** `CreateNewObject` (`0x0099893D`): `0x110` bytes, constructor `0x009CBBFF`; returns `W` (`+4`), null on
  failure.
- **`QueryInterface`** (`0x009CB247`): `0xE303E5AE` → the interface, AddRef'd; else `W`'s. Window slot 0 is the thunk
  `0x009CBD0E`.
- **Constructor**: `cGZWin` at `+4`, the vtables, the values below; colours from the control manager's default RGB
  colours (its slot 31): 1 ← `0xA8963FAD`, 2 ← `0x08963FAF`, 0 ← `0x68963F6F`.

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | flags: `0x2`, `0x4` bevel choice, `0x10` polygon, `0x20` fill | 1 |
| `+0xE0`, `+0xE4` | colours 1 and 2 | defaults above |
| `+0xE8`, `+0xEC` | point count and points (`(x, y)` pairs, heap) | 0, null |
| `+0xF0` | spans valid (byte) | 0 |
| `+0xF4`…`+0x100` | the points' bounding box (min x, min y, max x, max y) | 0 |
| `+0x104`…`+0x10C` | the span table: one sorted list of x crossings per row | empty |

## 2. The slots *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 1 / 2 | shared | `W`'s reference count |
| 3 `SetFlags(f)` / 4 `GetFlags()` | `0x009CBD01` / `0x009683E0` (shared) | `+0xDC` (no redraw) |
| 5 `SetColor(i, r, g, b)` | `0x009CB390` | 0 → `W`'s `SetFillColor(r, g, b)`; 1, 2 → `0xFF000000 \| r<<16 \| g<<8 \| b`; other `i`: nothing |
| 6 `GetColor(i, out r, g, b)` | `0x009CB3FC` | 0 → `W`'s; 1, 2 → the bytes; other `i`: nothing |
| 7 / 8 | `0x009CB449` / `0x009CB451` | colour 0 as native / set it from native (`W`'s fill-colour slots) |
| 9 / 12 `SetColorNative(1 / 2, c)` | `0x009CB459` / `0x009CB497` | the graphics system's slot 36 splits `c`, then slot 5 |
| 10 / 11 `GetColorNative(1 / 2)` | `0x009CB4D5` / `0x009CB500` | the graphics system's slot 34 `(r, g, b)` |
| 13 `SetPoints(n, points, fit, arg)` | `0x009CC050` (unattributed) | §3 |
| 14 `FitToPoints(exact)` | `0x009CB269` | `w := max x`, `h := max y` of the bounding box; unless `exact`, at least `W`'s current size; at least 0; `W.SetSize(w, h)` (window slot 53) |
| 15 `GetBoundingBox(out)` | `0x009CB2EE` | `+0xF4`…`+0x100` |
| 16 `GetPointCount()` | `0x009CB303` | the count with flag `0x10`, else 0 |
| 17 `GetPoints(out)` | `0x009CB317` | with flag `0x10` the points; else the draw area's corners `(l, t), (l, b), (r, b), (r, t)` |
| 18 `AsIGZWin()` | `0x009CFA36` (shared) | `W` (null for null) |

Window overrides: 0, 3 (`cGZWinFlatRect`'s `DoMessage`, `0x009CD718`), 55 `SetArea` (`0x009CC1F8`: `cGZWin::SetArea`,
then the spans are rebuilt), 88 `GZPaint` (`0x009CB57C`, §4), 106 (`0x009CBD16`, a thunk to `W`'s own native fill
setter), 148 destructor (`0x009CBD1E` → `0x009CBEA4`, body `0x009CBE2C`: free the points, clear and destroy the span
table, `~cGZWin`).

## 3. Polygons and spans *[compat]*

**`SetPoints(n, points, fit, arg)`** (slot 13): the old points are freed and `n` new ones stored, **rotated** to start
at the point with the smallest y (ties: smallest x); flag `0x10` is set; the bounding box is recomputed. `fit` →
`FitToPoints(arg)`. Then the spans are cleared (`0x009CBE10`) and rebuilt, and `W` is invalidated (window slot 91).

**Span building** (`0x009CBEEE`), only when not valid and `H` ≠ 0: the table gets `H` rows (`0x009CBEC0`). For each
row `y` from 0 and each edge from point `i` to point `i + 1` (the last to the first):
- an edge lying on the row (both ends at `y`) adds `min(x_i, x_{i+1})` only when the points before and after it
  (`i − 1` and `i + 2`, wrapping) are both above or both below the row;
- an edge whose end is on the row adds nothing; one whose start is on the row adds `x_i`;
- an edge the row crosses strictly adds `x_i + (y − y_i)·(x_{i+1} − x_i) / (y_{i+1} − y_i)` (integer division).

Each row's list is then sorted ascending (`0x009CB832`, merge sort of linked lists) and the spans are valid.

## 4. Painting (`GZPaint`, window slot 88) *[compat]*

Nothing for an empty draw area. The bevel colours are `a` (bottom/right edges) and `c` (top/left): flag `0x2` → `a`
= colour 2, `c` = colour 1; otherwise `a` = colour 1 and `c` = colour 2 with flag `0x4`, else colour 1.
- **Rectangle** (no `0x10`): flag `0x20` → fill the draw area with the fill colour (draw-context slots 21, 35). Then
  with `c`: `(l, t)–(r − 1, t)` and `(l, t)–(l, b − 1)`; with `a`: `(r − 1, t)–(r − 1, b − 1)` and `(l, b − 1)–(r −
  1, b − 1)` (draw-context slot 34).
- **Polygon** (`0x10`): flag `0x20`, at least 3 points and valid spans → for every row, each pair of crossings `(x0,
  x1)` with `x0 ≠ x1` becomes the line `(l + x0, t + y)–(l + x1, t + y)` in the fill colour. Then, when any of the
  flags 1, 2, 4 is set, every edge `(x0, y0)–(x1, y1)` is drawn offset by `(l, t)`: in `c` when `y0 < y1`, or `y0 =
  y1` and `x1 < x0`, else in `a`.

True.

## 5. Quirks and cases; inventory

Quirks:
1. `SetPoints` reorders the points (rotation to the top-most point).
2. Horizontal polygon edges add a crossing only at local extremes; vertices are counted once by the edge that starts
   there.
3. The polygon fill uses the window's fill colour, not colours 1 or 2.

Cases:
1. Defaults (flags 1), a 10×10 area at (0, 0): top/left and right/bottom all in colour 1.
2. Flags `0x14` (polygon, bevel 4: `c` = colour 2, `a` = colour 1), triangle `(5, 0), (9, 9), (0, 9)`: `(5, 0)→(9,
   9)` (y grows) and `(9, 9)→(0, 9)` (flat, x shrinks) in colour 2; `(0, 9)→(5, 0)` in colour 1. No fill (`0x20`
   clear).

Inventory (owner `cGZWinOutline`, 35): `0x0072BA00` (list helper), `0x0099893D` factory, `0x009CB247` QI,
`0x009CB269`, `0x009CB2EE`, `0x009CB303`, `0x009CB317`, `0x009CB390`, `0x009CB3FC`, `0x009CB449`, `0x009CB451`,
`0x009CB459`, `0x009CB497`, `0x009CB4D5`, `0x009CB500` (slots), `0x009CB52B` (list merge), `0x009CB57C` `GZPaint`,
`0x009CB802`, `0x009CB832` (list sort), `0x009CB959`, `0x009CB9C1`, `0x009CB9DE`, `0x009CBA0D`, `0x009CBA35`,
`0x009CBA64`, `0x009CBA81`, `0x009CBAAA`, `0x009CBAC7`, `0x009CBAE2`, `0x009CBAFB`, `0x009CBB19`, `0x009CBB4C` (the
span table's vector and list helpers), `0x009CBBFF` constructor, `0x009CBD01` slot 3, `0x009CBD0E` QI thunk.

Unattributed (described, not claimed): `0x009CBD16`, `0x009CBD1E`, `0x009CBD26`, `0x009CBE10`, `0x009CBE2C`,
`0x009CBE66`, `0x009CBEA4`, `0x009CBEC0`, `0x009CBEEE`, `0x009CC050`, `0x009CC1F8`.
