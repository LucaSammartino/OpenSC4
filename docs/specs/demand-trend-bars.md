# `cSC4WinRCI` and `cSC4WinTrendBar`: the demand bar and the trend bar (behaviour spec)

Scope: the owners `cSC4WinRCI` (10 functions, 1,266 bytes) and `cSC4WinTrendBar` (15 functions, 2,595 bytes) as
tools/coverage.py assigns them. Both are scripted windows (classes `0xC7A0E17E` and `0xAA5C2F86`,
scripted-window-director.md §2, which also gives their constructors). The RCI bar draws one zone's active demand; the
trend bar draws a recorded city statistic as a coloured bar with up or down arrows.

Related: demand.md (demand nodes, ActiveDemandChanged), history.md (the history warehouse and its record message),
widget-toolkit.md (`cGZWin`, the draw area `+0x24` and area `+0xA8`), response curves (`cSC4ResponseCurve`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owners was read in the decompile; both `GZPaint` bodies, the RCI
update and message handler, and the trend bar's value methods were checked on the instructions. Written 2026-10-24
(re-gzbars7).

Offsets are from the start of the object. Both classes put an interface at `+0`, the `cGZWin` at `+4` and a message
target at `+0xDC`; their `cGZWin` methods see `this` = object + 4.

## 1. The RCI demand bar (`cSC4WinRCI`) *[compat]*

### 1.1 Layout and interfaces

| Offset | Field |
|---|---|
| `+0x00` | bar interface, vtable `0x00AB8884` |
| `+0x04` | `cGZWin`, vtable `0x00AB8628`: `Init` `0x007A9340`, `Shutdown` `0x007A9390`, `GZPaint` `0x007A9500`, deleting destructor thunk `0x007A92E0` |
| `+0xDC` | message target, vtable `0x00AB8614`: `QueryInterface` thunk `0x007A92F0`, `DoMessage` `0x007A9810` (shared) |
| `+0xE0` | up to 16 demand IDs |
| `+0x120` | ID count |
| `+0x124` / `+0x128` / `+0x12C` | sums of active-demand minimum, maximum and value (integers) |
| `+0x130` | the bar colour |

The constructor zeroes the count, the first two IDs and the value sum only; the two limit sums and the colour are set
by the first update and by slot 4 *[quirk]*.

**`QueryInterface`** (`0x007A9240`, thunk `0x007A92D0`): `0x89E1574C` and 1 → `+0`; `0x452294AA` → `+0xDC`;
`0x22BA0121` → `+4`; each with a reference. **Any other IID returns false** without falling back to `cGZWin`.

**Bar interface** (`+0`): 0–2 the usual; 3 the window; 4 `SetColor(c)` (`0x007A92C0`): `+0x130` := c, no redraw;
5 `SetDemandIDs(ids, n)` (shared `0x007A97D0`): copies n IDs to `+0xE0` with **no bound** (n > 16 overwrites the count
and the sums *[quirk]*), count := n, then the update (§1.2) without a redraw; 6 and 7 are `_purecall` *[quirk]*.

### 1.2 Update (`0x007A93E0`)

Without the demand simulator (`0x00B43D74`) it returns false and changes nothing. Otherwise the three sums are
reset to 0 and, for each positive ID, `node = sim.GetDemand(id, 0x20000)` (slot 6); for each node found:
`min += node.QueryActiveDemandMin()` (slot 15), `max += node.QueryActiveDemandMax()` (slot 14),
`value += node.QueryActiveDemandValue()` (slot 12), each addition truncated to an integer (`fiadd` + `_ftol2`).
Returns true when any sum differs from before.

### 1.3 Lifetime and messages

- **`Init`** (once): `cGZWin::Init`; subscribe `+0xDC` to ActiveDemandChanged (`0x426840A0`) on the message server
  (`0x00B43CCC`, slot 5). **`Shutdown`**: unsubscribe (slot 6), `cGZWin::Shutdown`.
- **`DoMessage`**: for `0x426840A0` whose data1 (message slot 9) is one of the IDs, the update runs; when it returns
  true the window is invalidated (slot 92). Always returns false.
- **Destructor** (`0x007A9300`): vtables restored, `cGZWin` destroyed, freed when bit 0 of the argument is set.

### 1.4 Drawing (`GZPaint`)

The draw context (`+0x6C` of the window) gets the colour (its slot 21). With `W`, `H` the window's size (area
`+0xA8`), `a` the value sum and `L` the limit on its side (the maximum sum when a > 0, the minimum sum when a < 0):

```
r = ln|a| / ln|L|  +  a / L                    (L and a have the same sign)
x = (r · half + 1) · 0.5                       (half = W/2 or H/2, integer division; x not rounded)
```

- a = 0: nothing is drawn.
- W > H (horizontal, half = W/2): a > 0 fills (trunc(half + 1 + x), 0, half + 1, H) — left and right **swapped**
  (RCI1); a < 0 fills (trunc(half − x), 0, half, H).
- otherwise (vertical, half = H/2): a > 0 fills (0, trunc(half − x), W, half); a < 0 fills
  (0, half + 1, W, trunc(half + 1 + x)).

Each coordinate is truncated once, after the addition or subtraction (`_ftol2`).

The fill is the context's slot 35. So the bar grows right or up from the centre for positive demand, left or down for
negative, with a length that averages a logarithmic and a linear share of the half size. Returns true.

## 2. The trend bar (`cSC4WinTrendBar`) *[compat]*

### 2.1 Layout and interfaces

| Offset | Field |
|---|---|
| `+0x00` | bar interface, vtable `0x00ABA68C` |
| `+0x04` | `cGZWin`, vtable `0x00ABA430`: `Init` `0x007BEDD0`, `Shutdown` `0x007BEE50`, `GZPaint` `0x007BF0A0`, deleting destructor thunk `0x007BF5A0` |
| `+0xDC` | message target, vtable `0x00ABA420`: `DoMessage` `0x007BF490` |
| `+0xE0` | the history key (`SetDataSource`) |
| `+0xE4` | value, 0…1 |
| `+0xE8` | target, 0…1 |
| `+0xEC` | delta (float) |
| `+0xF0` | bar picture |
| `+0xF4` | arrow picture |
| `+0xF8` | response curve |

**`QueryInterface`** (`0x007BEC90`, thunk `0x007BF590`): `0xCA5C2F84` and 1 → `+0`; `0x452294AA` → `+0xDC`;
`0x22BA0121` → `+4`; any other IID false (no fallback).

**Bar interface:** 3 the window; 4 `SetArtwork(bar, arrows)` (`0x007BEEB0`): both assigned with reference counting;
the arrow picture gets magenta, RGB (255, 0, 255), as its colour key (graphic system slot 34, picture slot 23);
invalidate. 5 `SetDataSource(key, curve)` (`0x007BEF50`) and 6 `SetDataSource(key, x0, x1)` (`0x007BF670`, the curve
is the two points (x0, 0) and (x1, 1)): key := `+0xE0`, value := 0.5, the curve copied or set; then the city's history
warehouse (city slot 32) `GetPoint(key, today, &v)` (slot 8; today = the simulator's date number, its slot 8) and, when
it succeeds, `UpdateValue(v)`; invalidate.

### 2.2 Value methods

All clamp to [0, 1] (below 0 → 0, above 1 → 1) and, when the value or the delta changed and the window is visible
(slot 71), invalidate (slot 92).

- **`UpdateValue(x)`** (`0x007BED10`): value := curve(x); delta := value − previous value.
- **`UpdateTrend(x)`** (`0x007BF3C0`): target := curve(x); delta := target − value; then `InterpolateValue(0.01)`.
- **`InterpolateValue(f)`** (`0x007BEFF0`): step = f · delta; when |step| < |value − target|, value += step (clamped),
  else value := target.

The value therefore walks to a new target in steps of 1 % of the gap measured when the target was set.

### 2.3 Lifetime and messages

- **`Init`** (once): `cGZWin::Init`; subscribe `+0xDC` to the history record message `0x89EFA536` and SimNewDay
  `0x66956814`. **`Shutdown`**: unsubscribe both, then `cGZWin::Shutdown`.
- **`DoMessage`**: `0x89EFA536` with data1 (slot 9) equal to the key → `UpdateTrend((float) data2)` (message slot 14;
  the value just recorded, history.md §4.1). The city's warehouse and the date are fetched first and not used.
  `0x66956814` → `InterpolateValue(0.01)`. Always returns false.
- **Destruction** (`0x007BF530`, deleting `0x007BF650`): the curve's storage, the arrow and bar pictures released,
  then the `cGZWin`.

### 2.4 Drawing (`GZPaint`)

Needs both pictures (else returns false). `c` = arrow picture width ÷ 6 (unsigned), `ha` = its height; `hb` = bar
picture height, `wb` its width; L, T, R, B the draw area (`+0x24`).

- **Arrow counts** from the delta d: up u = 3 when d > 0.02, 2 when d > 0.01, 1 when d > 0.00001, else 0; down n = 3
  when d < −0.02, 2 when d < −0.01, 1 when d < −0.00001, else 0.
- The buffer to draw to (`+0x68`) is locked (slot 6, `0x8010`; nothing is drawn when the lock fails). Arrow top
  `y = T + (B − T − ha) / 2`.
- n > 0: arrow cells 0…n−1 (source (0, 0, n·c, ha)) go to (L + (3 − n)·c, y, L + 3c, y + ha) (buffer slot 29).
- u > 0: cells 3…3+u−1 (source (3c, 0, (3 + u)·c, ha)) go to (R − 3c, y, R − (3 − u)·c, y + ha).
- The bar: row `s = hb − 1 − trunc((hb − 1) · value + 0.5)` of the bar picture (source (0, s, wb, s + 1)) is tiled over
  (x, T, x + wb, B), x = L + (R − L − wb) / 2 (unsigned), with `RZDrawBltTiled(buffer, bar, &source, &dest, 0, 0, 0)`.
  Value 1 shows the top row, value 0 the bottom row.
- Unlock (slot 7). Returns true.

## 3. Quirks

- The RCI bar's horizontal positive rectangle has left > right (RCI1).
- `SetDataSource` resets the value to 0.5 before reading the latest point, so the first delta (and the arrows) show the
  jump from 0.5; the target is not set. Until the first record message each SimNewDay moves the value by 1 % of that
  delta, away from 0.5, toward 0 or 1.
- Neither bar's `QueryInterface` falls back to `cGZWin`'s; the RCI bar's slots 6–7 are pure.
- The RCI bar sums truncate after each node.

## 4. Function inventory

- `cSC4WinRCI`: `0x007A9240`, `0x007A92C0`, `0x007A92D0`, `0x007A92E0`, `0x007A92F0`, `0x007A9300`, `0x007A9340`,
  `0x007A9390`, `0x007A93E0`, `0x007A9500`.
- `cSC4WinTrendBar`: `0x007BEC90`, `0x007BED10`, `0x007BEDD0`, `0x007BEE50`, `0x007BEEB0`, `0x007BEF50`,
  `0x007BEFF0`, `0x007BF0A0`, `0x007BF3C0`, `0x007BF490`, `0x007BF530`, `0x007BF590`, `0x007BF5A0`, `0x007BF650`,
  `0x007BF670`.
