# `cGZWinScrollbar2`: the image scroll bar (behaviour spec)

Scope: the `cGZWinScrollbar2` owner as tools/coverage.py assigns it on `bb641878` (37 functions, 6,320 bytes;
inventory in §8). It is the framework's second scroll bar: arrows, track and thumb drawn from three images (or
drawn procedurally without them), with auto-repeat. It is a different class from the script control
`GZWinScrollbar` (`0x61325A2E`, legacy-control.md §9.16).

Related:
- widget-toolkit.md: the `cGZWin` base (fields §2, flags §3, capture §6, `SendMsg` §7);
- widget-wingen.md §5.3 (the same draw-context slots 38/39 and `RZDrawRaisedRect`);
- timers.md (the `cRZCallbackTimer` at `+0x12C`: slot 3 restart, 5 `Start(period, object)`, 7 `Stop`, 9
  `SetCallback`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `SetOrientation`, the value notification and
the thumb drag were checked on the instructions. The vtables `0x00AE4088` (`cIGZWinScrollbar2`) and `0x00AE40F0`
(window) were read and compared with the `cGZWin` table `0x00ADC8D8`. Written 2026-10-23 (re-gzwinsb7).

Conventions:
- Offsets are from the window start; the interface is at `+0xD8`. "Window slot n" is a `cIGZWin` slot; "slot n"
  alone a `cIGZWinScrollbar2` slot.
- `L` is the length along the bar: `W` when horizontal, `H` when vertical; `T` is the other dimension.
- `a` = arrow length `+0x100`; `track = L − 2a`; `range = max(max − min, 0)`; `page = +0xE8`;
  `step = max(page − overlap, 1)`.
- "Redraw" is window slot 92 (`InvalidateSelfAndParents`).

## 1. Identity and fields *[compat]*

- **Factory** `CreateNewObject` (`0x009D960B`): `0x154` bytes, constructor `0x009D9445`; registered for class
  `0x4B42DA5D` by the toolkit's class table (`0x00998CD3`).
- **`QueryInterface`** (`0x009D7D6A`): `0x4B42DA53` → `+0xD8` (null for null), AddRef'd, true; else
  `cGZWin::QueryInterface`. Slot 0 of the interface (`0x009D8057`) is the thunk.
- **Window overrides**: 0, 3 `DoMessage`, 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 88 `GZPaint`, 134 left down,
  136 left up, 138 mouse move, 142 mouse exit, 148 destructor, and slot 151 (`0x009D9841`).
- **Constructor**: `cGZWin` constructor, then the values below; it clears window flag `0x8000` (one of the
  `cGZWin` defaults, widget-toolkit.md §3) and calls its own `SetArea(0, 0, 32, 96)`.
- **Destructor** (body `0x009D8004`, deleting form `0x009D951B`): restore the vtables, destroy the timer, release
  the three images, `~cGZWin`.

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | value | 0 |
| `+0xE0`, `+0xE4` | minimum, maximum | 0, **256** |
| `+0xE8` | visible range (page size) | **16** |
| `+0xEC` | page overlap | 1 |
| `+0xF0` | increment (line) delta | 1 |
| `+0xF4`, `+0xF8`, `+0xFC` | arrows, container (track) and thumb images (counted) | null |
| `+0x100` | arrow length `a` | 0 |
| `+0x104` | track end-cap length | 0 |
| `+0x108` | track thickness | 0 |
| `+0x10C` | thumb end-cap length | 0 |
| `+0x110` | thumb thickness | 0 |
| `+0x114`, `+0x118` | minimum width and height (§2.1) | 0 |
| `+0x11C` | thumb offset in the track | 0 |
| `+0x120` | thumb length | 1 |
| `+0x124` | grab offset while the thumb is dragged | — |
| `+0x128` | dirty bits: 1 image sizes, 2 positions | 0 |
| `+0x129` | orientation: 0 horizontal, 1 vertical | **1** |
| `+0x12A` | part under the pointer (§3) | 0 |
| `+0x12B` | pressed part | 0 |
| `+0x12C` | auto-repeat timer (`cRZCallbackTimer`) | constructed |

## 2. The `cIGZWinScrollbar2` slots *[compat]*

Vtable `0x00AE4088`, 26 slots; 1–3 are the shared AddRef, Release and `AsIGZWin` thunks. Every setter returns true.

| Slot | Address | Behaviour |
|---|---|---|
| 4 `SetOrientation(o)` | `0x009D7DB1` | see below |
| 5 `GetOrientation(out)` | `0x009D7DD2` | the byte |
| 6 `SetValue(v, notify)` | `0x009D7DE1` | §2.2 |
| 7 `GetValue(out)` | `0x009D7E67` | |
| 8 `GetValue()` | `0x009D7E63` | returns the value (a shared 4-byte getter) |
| 9 `SetMinimumValue(v)` | `0x009D7E75` | if changed: store, `SetValue(value, 1)`, dirty 2, redraw |
| 10 `GetMinimumValue(out)` | `0x009C48C0` | (shared getter) |
| 11 `SetMaximumValue(v)` | `0x009D7EA6` | as slot 9 |
| 12 `GetMaximumValue(out)` | `0x009D7ED7` | |
| 13 `SetVisibleRange(v)` | `0x009D848E` | `v < 1` → 0; if changed: store, `SetValue(value, 1)`, dirty 2, redraw |
| 14 `GetVisibleRange(out)` | `0x009329A0` | |
| 15 `SetPageOverlapRange(v)` | `0x009D84D4` | `v < 1` → 0; store; nothing else |
| 16 `GetPageOverlapRange(out)` | `0x00932A04` | |
| 17 `SetIncrementDelta(v)` | `0x009D84F3` | `v < 1` → 0; store |
| 18 `GetIncrementDelta(out)` | `0x009D7EE5` | |
| 19, 21, 23 `SetArrowsImage`, `SetContainerImage`, `SetThumbImage` | `0x009D8512`, `0x009D853D`, `0x009D8568` | dirty 3, store (counted), redraw |
| 20, 22, 24 the getters | `0x009D8539`, `0x009D8564`, `0x009D858F` | the pointer, no AddRef |
| 25 `GetMinimumSize(out)` | `0x009D8593` | dirty 1 → `CalcImageSizes`; `out = (+0x114, +0x118)` |

**`SetOrientation(o)`**: for `o` = 0 or any value other than 1, if it differs from the current byte, store it and
set dirty 3. **`o` = 1 does nothing** (the test for 1 jumps straight to the return): a bar made horizontal can never
be made vertical again; only the constructor's default is vertical.

Note: setting the increment to 0 (or less) makes the arrows do nothing; the overlap 0 makes a page step equal the
page.

### 2.1 `CalcImageSizes` (`0x009D80AD`)

Clear dirty 1; minimum size `:= (0, 0)`.
- **No arrows, container or thumb image**: `a = 18`, thumb cap 4; vertical: minimum `(18, 54)`; horizontal:
  `(54, 18)`.
- **Vertical, with images** (every missing image contributes 0):
  - arrows `A`: minimum width `≥ A.W / 4` (four state columns); `a = A.H / 2` (two arrows stacked);
  - container `C`: minimum width `≥ C.W`; thickness `+0x108 = C.H / 2`, cap `+0x104 = C.H / 4`;
  - thumb `B`: minimum width `≥ B.W / 4`; `+0x110 = B.H / 2`, cap `+0x10C = B.H / 4`;
  - minimum height `= 2a + max(2 · track cap, 2 · thumb cap)`.
- **Horizontal**: the same with width and height exchanged (state rows, arrows side by side).

`W`, `H` of an image are buffer slots 9 and 10.

### 2.2 `SetValue(v, notify)` (slot 6)

1. `v := min(v, max − page)`, then `v := max(v, min)` (so the minimum wins when `max − page < min`).
2. Unchanged → true.
3. Dirty 2, redraw, store.
4. If `notify` (a byte) and the window is initialised: `SendMsg(notification target, 0x4B42DCC7, window, GetID(), v)`
   (window slot 144). *Quirk*: `v` is pushed before the argument-less `GetID` (window slot 63) and becomes the third
   data word.

## 3. Geometry *[compat]*

### 3.1 `SetArea(l, t, r, b)` (window slot 55, `0x009D8423`)

Dirty 1 → `CalcImageSizes`. Horizontal with a minimum height: `b := t + minimum height`; vertical with a minimum
width: `r := l + minimum width`. So the thickness always follows the images. `cGZWin::SetArea`; dirty 2.

### 3.2 `CalcPositions` (`0x009D8343`)

Dirty 1 → `CalcImageSizes`; clear dirty 2. `minThumb = 2 · thumb cap`.
- If `minThumb + 2 < track` and `page < range` and `range > 0`:
  `thumb length = max(page · track / range, minThumb)` and
  `thumb offset = ((range − page) / 2 + (value − min) · (track − length)) / (range − page)` (integer division; the
  first term rounds).
- Else if `minThumb + 2 < track`: length `= track`, offset 0 (everything visible).
- Else: length 0, offset 0 (no room for a thumb).

### 3.3 `PickPart(x, y)` (`0x009D85BC`), window coordinates

Positions first (dirty 2). Outside `0 ≤ x < W`, `0 ≤ y < H` → 0. Along the bar (`p = x` horizontal, `y` vertical):

| Condition | Part |
|---|---|
| `p < a` | 1 first arrow (left / top) |
| `p < a + offset` | 2 track before the thumb |
| `p < a + offset + length` | 5 thumb |
| `p < L − a` | 3 track after the thumb |
| otherwise | 4 second arrow (right / bottom) |

## 4. Input *[compat]*

- **Left down** (`0x009D918A`): `part := PickPart`; `GZSetCapture`, failure → return false. Then:
  - 1: `SetValue(value − line, 1)`; 2: `value − step`; 3: `value + step`; 4: `value + line`; each then starts the
    timer with 300 ms (timer slot 5, with `AsIGZWin`);
  - 5: grab offset `:= p − thumb offset`.

  Pressed part `:= part`, redraw. Returns **false** in every case.
- **Timer**: `Init` sets the callback (timer slot 9) to `0x009D7FDB`, which restarts the timer with a 70 ms period
  (timer slot 3) and posts `{0x8B443261, 0, 0, 0}` to the scroll bar itself (window slot 146). So a held arrow or track repeats after 300 ms,
  then every 70 ms.
- **`DoMessage`** (`0x009D7F1D`): type `0x8B443261`: only when the pointer is still over the pressed part
  (`+0x12A == +0x12B`), step once as in left down (parts 1–4); return true. Other types → `cGZWin::DoMessage`.
- **Mouse move** (`0x009D934D`):
  - capturing, pressed part 5, thumb length > 0: `pos = p − grab`, `free = L − 2a − length`; `free < 1` →
    `SetValue(0, 1)`; else `SetValue(pos · max(range − page, 0) / free, 1)`. *Quirk*: the minimum is not added, so
    for `min > 0` the thumb maps to `[0, …]` and is then clamped up to `min`;
  - capturing with pressed part 1–4: nothing (the timer repeats);
  - otherwise: `part := PickPart`; if it differs from `+0x12A`, store it and redraw.

  Returns false.
- **Left up** (`0x009D9303`): when capturing: release, pressed `:= 0`, `+0x12A := PickPart(x, y)`, redraw, stop the
  timer (timer slot 7). Returns false.
- **Mouse exit** (`0x009D7EF3`): when not capturing and `+0x12A` ≠ 0: clear it and redraw. Returns false.

## 5. Lifecycle

- **`Init`** (window slot 4, `0x009D9537`): when not initialised: timer callback (above); with no arrows **and** no
  container image, load three pictures from the window-control manager (`RZ::WCM`, slot 7, **not null-checked**):

  | Orientation | Arrows | Container | Thumb |
  |---|---|---|---|
  | vertical (1) | `0x0BD7935F` | `0xCBD79361` | `0x4BD79362` |
  | other | `0x4BD79360` | `0x8BD79361` | `0x0BD79363` |

  Then `CalcImageSizes`. Always ends with `cGZWin::Init`.
- **`Shutdown`** (window slot 5, `0x009D8062`): stop the timer, clear its callback (slot 9 with null), release the
  images, then `cGZWin::Shutdown`.

## 6. Painting (`GZPaint`, window slot 88, `0x009D86D9`) *[compat]*

Positions first. "Enabled" is window flag 2.

### 6.1 With any image

Each present image is drawn; an absent one is skipped. Image cells are taken in the image's own coordinates and
drawn with draw-context slot 38 `(image, src, dest)` for end pieces and slot 39 `(image, src, dest, 0, 0)` for the
stretch of the middle (WGN1 in widget-wingen.md).

- **Arrows** (horizontal; vertical exchanges the axes): the image is two arrows side by side, four state rows of
  `A.H / 4`. Row: disabled 0; else first arrow 2 when hovered (3 when also pressed), second arrow likewise, otherwise
  1. First arrow: cell `{0, row, a, row + A.H/4}` to `{0, 0, a, A.H/4}`; second arrow: cell `{a, row', 2a, …}` to
  `{W − a, 0, W, …}`.
- **Container**: left cap `{0, 0, cap, C.H}` to `{a, 0, a + cap, C.H}`; right cap `{C.W − cap, 0, C.W, C.H}` to
  `{W − a − cap, 0, W − a, C.H}`; middle `{cap, 0, C.W − cap, C.H}` tiled into `{a + cap, 0, W − a − cap, C.H}`.
- **Thumb**: four state rows of `B.H / 4`; row: disabled 0; pressed (part 5) 3; hovered 2; else 1. Left cap, right
  cap and tiled middle as for the container, placed from `a + offset` over the thumb length, with the thumb cap
  `+0x10C`.

### 6.2 Without images

Rectangles (horizontal): `A = {0, 0, a, H}`, `B = {W − a, 0, W, H}`; vertical: `A = {0, H − a, W, H}`,
`B = {0, 0, W, a}`. Track `= {a, 0, W − a, H}`, thumb `= {a + offset, 0, a + offset + length, H}` (vertical:
exchanged).
1. Arrows, `RZDrawRaisedRect(…, colour, raised, 1)`: disabled: both `0xFF808080`, raised. Hover on part 1: `A`
   normal (`0xFFC8C8FF`), `B` hovered (`0xFFDCDCFF`, raised) or pressed (`0xFFB4B4FF`, sunken). Hover on part 4:
   `B` normal, `A` hovered or pressed. Otherwise both normal.
2. Arrow glyphs, colour `0xFF646482` (draw-context slot 21), each a filled triangle (draw-context slot 44 with
   primitive 6 and three float points): `B` gets an upward triangle (apex at a quarter from its top), `A` a
   downward one.
3. Track: fill `0xFF9696BE` (slots 21, 35).
4. Thumb: disabled: fill `0xFFBEBEBE`; pressed: raised rect `0xFFB4B4FF`, sunken; hovered: `0xFFDCDCFF`, raised;
   else `0xFFC8C8FF`, raised.

*Quirk*: the triangles are not rotated and `A`/`B` follow the vertical layout, so a horizontal bar without images
shows up/down glyphs, and hovering its **left** arrow (part 1) highlights the **right** rectangle.

## 7. Quirks and cases

Quirks:
1. `SetOrientation(1)` is a no-op (§2).
2. Mouse handlers return false even when they act (§4).
3. Dragging the thumb ignores the minimum (§4).
4. `Init` does not check the control manager (§5).
5. `SetValue`'s notification takes `v` through a leftover push (§2.2).
6. The procedural horizontal look (§6.2).

Cases:
1. Defaults (min 0, max 256, page 16): `SetValue(250, 1)` stores 240 and sends `{0x4B42DCC7, window, id, 240}`.
2. Vertical, no images, `H = 96`: `a = 18`, `track = 60`, thumb length `max(16·60/256, 8) = 8`; at value 120:
   offset `(120 + 120 · 52) / 240 = 26`.
3. A click on part 3 with page 16, overlap 1: `SetValue(value + 15, 1)`; held, it repeats after 300 ms, then
   every 70 ms while the pointer stays on the track after the thumb.
4. `SetOrientation(0)` then `SetOrientation(1)`: the bar stays horizontal.

## 8. Function inventory (owner `cGZWinScrollbar2`, 37)

`0x009329A0` slot 14, `0x00932A04` slot 16, `0x009D7D6A` QI, `0x009D7DB1` slot 4, `0x009D7DD2` slot 5, `0x009D7DE1`
slot 6, `0x009D7E67` slot 7, `0x009D7E75` slot 9, `0x009D7EA6` slot 11, `0x009D7ED7` slot 12, `0x009D7EE5`
slot 18, `0x009D7EF3` mouse exit, `0x009D7F1D` `DoMessage`, `0x009D7FDB` timer callback, `0x009D8004` destructor
body, `0x009D8057` QI thunk, `0x009D8062` `Shutdown`, `0x009D80AD` `CalcImageSizes`, `0x009D8343`
`CalcPositions`, `0x009D8423` `SetArea`, `0x009D848E` slot 13, `0x009D84D4` slot 15, `0x009D84F3` slot 17,
`0x009D8512` slot 19, `0x009D8539` slot 20, `0x009D853D` slot 21, `0x009D8568` slot 23, `0x009D8593` slot 25,
`0x009D85BC` `PickPart`, `0x009D86D9` `GZPaint`, `0x009D918A` left down, `0x009D9303` left up, `0x009D934D` mouse
move, `0x009D9445` constructor, `0x009D951B` deleting destructor, `0x009D9537` `Init`, `0x009D960B`
`CreateNewObject`.

Not covered: the draw-context slots (WGN1 in widget-wingen.md).
