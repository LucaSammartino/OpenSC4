# `cGZWinPieMenu`: the radial menu (behaviour spec)

Scope: the `cGZWinPieMenu` owner as tools/coverage.py assigns it on `bb641878` (15 functions, 2,651 bytes) **and** the
50 pie-menu functions that the coverage tool attributes to `cGZWinCOMDirector` (addresses `0x009D40BE`–`0x009D5CB6`,
listed in §8). It is the toolkit's pie menu: items placed on up to eight compass directions around a centre, chosen
by direction and distance, with an optional title, background image, item windows and a pluggable item renderer.

Related: widget-toolkit.md (the `cGZWin` base, capture, `SendMsg`); modal-dialogs.md §2 (`DoModalWin`, the end
message `0xF`); widget-ctrlmgr.md (fonts, pictures).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the small `<shared>` getters and setters of the
interface were disassembled; the angle constants (π, 3π, π/4, 1/2π) were read. Vtables: `cIGZWinPieMenu`
`0x00AE3420` (57 slots) and the window `0x00AE3508`. Written 2026-10-24 (re-gzwinpie7).

Conventions: offsets from the window start; the interface (IID `0x1AAD4A4A`) is at `+0xD8`. "Slot n" is a
`cIGZWinPieMenu` slot, "window slot n" a `cIGZWin` slot. "Notify(code, d)" is `SendMsg(notification target, 3, code,
d, 0)` when the target is set.

## 1. Fields *[compat]*

Constructor `0x009D5990` (`cGZWin`'s, then these; everything not listed is 0):

| Offset | Meaning | Initial |
|---|---|---|
| `+0xDC` | selection (−1 none) | −1 |
| `+0xE0`, `+0xE4`, `+0xE8` | item colours: normal, highlighted, pressed (native) | 0 (`Init` defaults) |
| `+0xEC`, `+0xF0` | font, background image (counted) | null |
| `+0xF4` | title (`cRZString`) | empty |
| `+0x108..+0x114` | background image rectangle (x, y, w, h) | |
| `+0x118..+0x124` | title rectangle (x, y, w, h) | |
| `+0x128`, `+0x12C` | centre (parent coordinates) | |
| `+0x130`, `+0x134` | centre offset inside the window | |
| `+0x138`, `+0x13C` | window position; `+0x140`, `+0x144` its size | |
| `+0x148` | open (byte); `+0x149` tracking; `+0x14A` sticky (stays open after a click) | 0 |
| `+0x150` | the window was moved to fit (use the screen cursor, §5); `+0x151` pressed look | 0 |
| `+0x154` | sector count (1, 2, 4 or 8) | |
| `+0x158` | first sector (0 = north, clockwise in eighths) | 0 |
| `+0x15C` | dead-zone radius (0 = automatic) | 0 |
| `+0x160`, `+0x164` | last distance and angle from the centre (floats) | 0 |
| `+0x168` | margin around the background image | **8** |
| `+0x16C` | a second click commits (byte) | **1** |
| `+0x170` | maximum radius (0 = none) | 0 |
| `+0x174` | cancel beyond the maximum radius (byte) | 0 |
| `+0x178` | close on commit (dword, ≥ 0) | **1** |
| `+0x17C`, `+0x180` | item distance on the axes, on the diagonals | **40**, **20** |
| `+0x184` | running modally (byte) | 0 |
| `+0x188` | item renderer (counted; optional) | null |
| `+0x18C..+0x194` | items: `vector` of 0x30-byte records | empty |

**Item record** (0x30 bytes, `0x009D57DF`): `+0x00` caption (`cRZString`), `+0x14` item window (counted, optional),
`+0x18` enabled (byte, initially 1), `+0x1C` angle (float), `+0x20..+0x2C` rectangle x, y, w, h (window
coordinates). Copy and container helpers: `0x009D5814`, `0x009D582B`, `0x009D5874`, `0x009D589D`, `0x009D58C5`,
`0x009D590A`, `0x009D5923`, `0x009D5955`, `0x009D5973`, `0x009D5B56`, `0x009D5B89`, `0x009D5C7E`.

## 2. The `cIGZWinPieMenu` slots *[compat]*

Slots 1–3 are the shared AddRef, Release and `AsIGZWin`; slot 0 (`0x009D5ADB`) the QI thunk. "(s)" marks a
`<shared>` function.

| Slot | Address | Behaviour |
|---|---|---|
| 4 | `0x009D40BE` | returns π (a constant) |
| 5 `RemoveAllItems` | `0x009D5C62` | erase every item; selection := −1 |
| 6 `AddItem(caption, window)` | `0x009D5CB6` | append a record (window AddRef'd, enabled); true |
| 7 `SetSelection(i, commit)` | `0x009D4988` | §4.3 |
| 8, 18 `GetSelection` | `0x009D7E63` (s) | `+0xDC` |
| 9 `Layout` | `0x009D4F8D` | §3 |
| 10 `LayoutAndPlace` | `0x009D41CD` | slot 9, then window position := centre − offset, `SetArea` with the size; slot 9's result |
| 11 `GetItemCount` | `0x009D4A6C` | |
| 12 `HitTest(x, y)` | `0x009D4A81` | §4.1 |
| 13 `SetItemEnabled(i, b)` | `0x009D4C30` | for a valid `i` |
| 14 / 15 | `0x009D44A0` / `0x009D44A7` | last distance / angle |
| 16 / 17 | `0x00993586` (s) / `0x009D44AE` | dead-zone radius get / set |
| 19 / 20 | `0x009D4185` / `0x009D418C` | maximum radius get / set |
| 21 / 22 | `0x0099BE80` / `0x0099BE87` (s) | `+0x174` get / set |
| 23 `IsOpen` | `0x009D449C` | `+0x148` |
| 24–29 | `0x009D4199`…`0x009D41AD` (s) | window x, y, centre x, y, centre offset x, y |
| 30 / 31 | `0x009D4F11` / `0x009D4F4F` | the selected item's centre x / y (0 without a selection) |
| 32 `SetTitle(s)` / 33 `GetTitle(out)` | `0x009D41C5` / `0x009D41B1` | |
| 34 `SetBackgroundImage(img)` / 35 `SetFont(font)` / 36 `GetFont` | `0x009D4718` / `0x009D470C` / `0x009D4714` (s) | |
| 37 / 39 / 41 | `0x009D40F2` / `0x009D4116` / `0x009D413A` | normal / highlighted / pressed colour from `(r, g, b)` (window slot 108) |
| 38 / 40 / 42 | `0x009B2970` / `0x00894F6D` / `0x0099BE5C` (s) | the same colours, native |
| 43 / 44 / 45 | `0x009D415E` (s) / `0x009D416B` / `0x009D4178` | first sector / margin / `+0x16C` |
| 46 | `0x009D5AE6` | `cGZWin::SetArea(rect)` |
| 47 `Run(x, y, sticky, close)` | `0x009D44BB` | slot 48, then slot 49's result, then slot 50 |
| 48 `Track(x, y, sticky, close)` | `0x009D44F4` | modal := 0; release any other capture (window manager slots 28, 30); capture; slot 51; slot 7 `(selection, 0)` |
| 49 `RunModal` | `0x009D454E` | modal := 1; window manager `DoModalWin(this)` (slot 41) |
| 50 | `0x009D456C` | when `+0x178 ≥ 0`: slot 52 |
| 51 `Open(x, y, sticky, close)` | `0x009D4C69` | §4.2 |
| 52 `Close` | `0x009D4E9C` | §4.2 |
| 53 `SetItemDistances(axis, diagonal)` | `0x009D457E` | `+0x17C`, `+0x180` |
| 54 `End(result)` | `0x009D4454` | not modal → slot 50; modal → window manager `GZWinMsgPost(0xF, window, result)`, open and tracking := 0 |
| 55 | `0x009D4490` | slot 54 with the selection |
| 56 `SetRenderer(r)` | `0x009D4595` | release the old one **before** AddRef'ing the new one |

## 3. Layout (slot 9, `0x009D4F8D`)

1. Window slot 4 (`Init`). Background size `W × H` (the image's slots 9, 10, or 0); its rectangle origin `(−W/2, −H/2)`.
2. Dead zone 0 → 8 without an image, else `max(max(W, H) / 2, 1)`.
3. Bounding box := the image rectangle grown by the margin.
4. Title size: the renderer's slot 4 `(menu, &title, &rect)`, or without a renderer (or when it returns false) the
   font's slot 47 measure grown by 3 on each side; centred on the origin; the box grows to hold it.
5. Sector count: no items 1; up to 2 items 2; up to 4 items 4; more 8.
6. Each item `i`: size from its window (its W, H) or the renderer's slot 3 `(menu, i, &item, &rect)` or the caption
   measure grown by 3. With `n` sectors and `step = 8 / n`, the first `n` items get direction
   `d = (first sector + i·step) & 7` and angle `(2 − d)·π/4` (made non-negative); with `a`, `b` the axis and diagonal
   distances and `w × h` the item size, its top-left is:

   | d | 0 N | 1 NE | 2 E | 3 SE | 4 S | 5 SW | 6 W | 7 NW |
   |---|---|---|---|---|---|---|---|---|
   | x | `−w/2` | `b` | `a` | `b` | `−w/2` | `−(b + w)` | `−(a + w)` | `−(b + w)` |
   | y | `−(a + h)` | `−(b + h)` | `−h/2` | `b` | `a` | `b` | `−h/2` | `−(b + h)` |

   Further items stack: `m = i − n`; even `m` goes above the item at the first direction (`m` = 0) or above item
   `i − 2`, with angle 3π; odd `m` goes below the item at `(first − 4) & 7` (`m` = 1) or below item `i − 2`, angle π;
   both centred horizontally. The box grows to hold every item.
7. With the box `{l, t, r, b}` around the centre: `l −= 4`, `t −= 4`; the centre offset := `(−l, −t)`; the window
   size := `(r − l + 4, b − t + 4)`; the image, title and item rectangles are moved by the offset. Invalidate (window
   slot 91). True.

## 4. Behaviour

### 4.1 `HitTest(x, y)` (slot 12), window coordinates

No items → −1. `(dx, dy)` = the point relative to the centre; distance `ρ` (stored). With `ρ ≥ 1`: angle
`θ = atan2(dx, −dy)` (stored). An enabled item whose rectangle holds the point wins. Otherwise: `ρ ≤` dead zone →
angle := 0, −1; a maximum radius with `ρ` beyond it → −1; else sector `s = n/(2π)·(θ − first·π/4)` wrapped into
`[0, n)` and truncated; `s ≥` item count or a disabled item → −1, else `s`.

### 4.2 `Open(x, y, sticky, close)` (slot 51) and `Close` (slot 52)

Open: centre := `(x, y)`; sticky, pressed look := `sticky ≠ 0`; close := `close`; open and tracking := 1; moved := 0;
layout; show (`SetFlag(1, 1)`). Window position := centre − offset, kept inside the parent (window slot 11's width
and height): a clamped position sets "moved" and moves the centre with it. `SetArea`. Each item window is placed at
its rectangle (window coordinates + position), enabled per its record and added to the parent. Selection := −1. Then
the cursor position (window manager slot 33), relative to the window, is passed to `GZOnMouseMove`.

Close: each item window is removed from the parent (its slot 15); open := 0.

### 4.3 `SetSelection(i, commit)` (slot 7)

`i ≠ −1` is clamped to the items. Store; Notify(`0x1AAD67D5`, window) (changed). Without `commit`: invalidate. With
it: Notify(`0x1AAD67D6`, window), or `0x1AAD67D7` for −1 (cancelled); and with close (`+0x178 ≠ 0`) slot 55 (end
with the selection).

### 4.4 Input (owner functions)

- **Mouse move** (`0x009D436D`), while tracking: with "moved", the screen cursor relative to the window position
  is used instead of `(x, y)`. `s := HitTest`; a change → `SetSelection(s, 0)`. With cancel-beyond (`+0x174`), a
  maximum radius, no item and `ρ` beyond it → `SetSelection(−1, 1)`. Notify(`0x1AAD67D4`, window). True.
- **Left down** (`0x009D42A2`): pressed look := 0; not sticky, or `+0x16C` = 0 → mouse move and sticky := 1;
  otherwise → left up (a second click commits). True.
- **Left up** (`0x009D42F4`): pressed look := 0; mouse move; while tracking: sticky → `SetSelection(selection, 1)`;
  not sticky: released outside the dead zone and not "moved" → the same commit; otherwise sticky := 1 (the menu stays
  open after a click). Moved := 0. True.
- **Right down / up** (`0x009D4368`): false.
- **Key down** (`0x009D4242`): Esc → selection := −1, Notify(`0x1AAD67D7`, `0x1B`), slot 55, true. Other keys →
  Notify(`0x1AAD67D9`, vk), false.
- **Character** (`0x009D420F`): Notify(`0x1AAD67D8`, the character, sign-extended).

### 4.5 Painting (`GZPaint`, `0x009D549D`)

1. Background image at its rectangle (draw context slot 38).
2. Title: the renderer's slot 8 `(menu, &title, title rectangle, pressed colour, fill colour)` may draw it; otherwise
   a non-empty title is drawn with window slot 152 (`DrawFramedLabel`, index −1) in the pressed colour. Then the
   renderer's slot 10 with the same arguments.
3. Each enabled item without a window: colour = pressed look ? pressed : highlighted for the selection, normal
   otherwise; the renderer's slot 5 `(menu, i, &title, item rectangle, colour, fill)` may draw it, otherwise
   `DrawFramedLabel`; then the renderer's slot 7.
4. Each item window is invalidated (window slot 91). True.

**`DrawLabelFrame(colour, x, y, w, h)`** (window slot 151, `0x009D4720`): a filled rounded rectangle: the rectangle
inset by 2, then two rings of 1-pixel strips outside it, each shortened at the corners (by 1, then 3).
**`DrawFramedLabel(text, i, length, font, colour, fill, x, y, w, h)`** (window slot 152, `0x009D4899`): frame with
`fill`; the font's colour set to `colour` (font slot 14) and restored after; text offset (3, 3), which the renderer
may change (its slot 9 for the title, slot 6 for items); font slot 51 draws the text at the offset inside the frame.
Most of its arguments arrive through pushes made before argument-less getters (window slots 94 and 126).

### 4.6 Lifecycle

**`Init`** (`0x009D45D8`), once: no font → slot 35 with the system font `0x68963C4C`; unset colours → normal
(185, 185, 208), highlighted (0, 255, 255), pressed (255, 255, 255) (window slot 108); no fill colour →
(118, 118, 118); an **existing** background image is replaced with the control manager's picture `0xA8963EE8`
(*quirk*: a missing one stays missing). `cGZWin::Init` is always called. **`Shutdown`** (`0x009D46E7`): release the
image and the font, `cGZWin::Shutdown`.

## 5. Quirks

1. `Init` only replaces a background image that is already set (§4.6).
2. `SetRenderer` releases before AddRef'ing, so setting the same renderer with one reference frees it.
3. Layout calls `Init` every time.
4. `SetSelection` notifies "changed" even when the selection did not change.

## 6. Cases

1. Four items, first sector 0: directions N, E, S, W at the axis distance 40.
2. Three items: four sectors; the third item goes south (`d = 4`).
3. A click inside the dead zone opens a sticky menu; the next click on an item commits it and, with close set, ends
   the menu with that index.

## 7. Notifications

`0x1AAD67D4` moved, `0x1AAD67D5` selection changed, `0x1AAD67D6` chosen, `0x1AAD67D7` cancelled, `0x1AAD67D8` character,
`0x1AAD67D9` key (all type 3).

## 8. Function inventory

Owner `cGZWinPieMenu` (15): `0x009D40C5` QI, `0x009D420F` character, `0x009D4242` key down, `0x009D42A2` left down,
`0x009D42F4` left up, `0x009D4368` right buttons, `0x009D436D` mouse move, `0x009D45D8` `Init`, `0x009D46E7` `Shutdown`,
`0x009D4720` `DrawLabelFrame`, `0x009D4899` `DrawFramedLabel`, `0x009D4988` slot 7, `0x009D549D` `GZPaint`,
`0x009D5ADB` QI thunk, `0x009D5C46` deleting destructor.

Attributed to `cGZWinCOMDirector` (50): `0x009D40BE`, `0x009D40F2`, `0x009D4116`, `0x009D413A`, `0x009D416B`,
`0x009D4178`, `0x009D4185`, `0x009D418C`, `0x009D41B1`, `0x009D41C5`, `0x009D41CD`, `0x009D4454`, `0x009D4490`,
`0x009D449C`, `0x009D44A0`, `0x009D44A7`, `0x009D44AE`, `0x009D44BB`, `0x009D44F4`, `0x009D454E`, `0x009D456C`,
`0x009D457E`, `0x009D4595`, `0x009D470C`, `0x009D4718`, `0x009D4A6C`, `0x009D4A81`, `0x009D4C30`, `0x009D4C69`,
`0x009D4E9C`, `0x009D4F11`, `0x009D4F4F`, `0x009D4F8D` (§2–§4); `0x009D57DF`, `0x009D5814`, `0x009D582B`, `0x009D5874`,
`0x009D589D`, `0x009D58C5`, `0x009D590A`, `0x009D5923`, `0x009D5955`, `0x009D5973`, `0x009D5B56`, `0x009D5B89`,
`0x009D5C7E` (item records, §1); `0x009D5990` constructor; `0x009D5AE6` slot 46; `0x009D5C62` slot 5; `0x009D5CB6` slot 6.

Not covered: the renderer interface (its slots cited by number) and the font slots.
