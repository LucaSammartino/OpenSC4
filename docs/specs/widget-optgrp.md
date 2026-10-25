# `cGZWinOptGrp`: the option group (behaviour spec)

Scope: the `cGZWinOptGrp` owner as tools/coverage.py assigns it on `bb641878` (49 functions, 4,946 bytes;
inventory in §7). The option group is a container of radio buttons (option buttons) with an optional label: it keeps
one selected button, lays the buttons out in columns, and sizes itself to them. Its script properties are in
legacy-control.md §9.20; the control manager's `CreateOptGrp` and default option group are in widget-ctrlmgr.md §3.3
and §5. One function of the owner (`0x0099BE32`) is a `cGZWin` helper (§6).

Related:
- widget-toolkit.md: the `cGZWin` base (fields §2: draw area `+0x24`, notification target `+0x4C`, draw context
  `+0x6C`; flags §3: `0x2` enabled, `0x4000` initialised; message types §7.3: type 3 → `GZOnCommand(+4, +8)`);
- widget-button.md §2: the option buttons (`cIGZWinBtn` `0x8810`: slots 3, 5 `SetBtnFlag`, 12 `SetStyle`, 19
  `SetFont`, 23 native colour, 26 unset colour, 47 `SetID`, 49 `SetCaption`; flags 1 checked, 4 has a caption; a
  click posts `0x287259F6` with the button's ID);
- widget-text.md §2: the label (`cIGZWinText` `0x212CDC1F`: slots 4, 5 `FitWindowToText`, 7, 10, 16, 18);
- widget-ctrlmgr.md §4–§5: `CreateCtrl` (slot 26) and the default controls `0xA8963E4B` (radio) and `0x48963E33`
  (label); `SystemFont` (slot 5).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `SetValue`, `OptBtnAdd`, `OptBtnMoveTo`,
`SetOutlineColors`, the outline paint and the automatic size's column loop were checked on the instructions (the
interface methods run on `+0xD8`, so the decompile's field names are offset). The vtables `0x00AE1300` (window) and
`0x00AE1288` (`cIGZWinOptGrp`) were read and compared with the `cGZWin` table `0x00ADC8D8`. Written 2026-10-24
(re-gzoptgrp7).

Conventions:
- Offsets are from the window start; the interface is at `+0xD8`. "Window slot n" is a `cIGZWin` slot; "slot n"
  alone a `cIGZWinOptGrp` slot; "button slot" and "label slot" the interfaces above.
- "The label" is the child with ID `0x02E3A3E5`, fetched as `0x212CDC1F` (window slot 36). "The buttons" are the
  children that answer `0x8810`, in child-list order; "for each button" is window slot 32 `EnumChildren(0x8810,
  callback, data)`.
- `gx`, `gy` are the gutters (`+0xEC`, `+0xED`). "Redraw" is window slot 92. `W`, `H` are window slots 41, 42.

## 1. Identity and fields *[compat]*

- **Factory** `CreateNewObject` (`0x009C7DFF`): `0x11C` bytes, constructor `0x009C7BB8`; returns the interface
  (`+0xD8`), null on failure. Class `0xA1336CC1`.
- **`QueryInterface`** (`0x009C7094`): `0xA1336CC0` → `+0xD8` (null for null) AddRef'd; with a null out pointer it
  returns false without writing; else `cGZWin::QueryInterface`. Slot 0 of the interface (`0x009C77B0`) is the thunk.
- **Window overrides**: 0, 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 68 `SetFlag`, 74 `SetCaption`, 88 `GZPaint`, 143
  `GZOnCommand`, 148 destructor; the mouse slots 134–138 are the shared "false" stub (`0x009378BC`).
- **Constructor**: `cGZWin` constructor, the vtables and the values below.

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | flags (§2.2) | `0xB` (main caption, item captions, outline) |
| `+0xE0` | button class ID (slot 23); not read by the owner | 0 |
| `+0xE4` | fill type (slot 28): 0 automatic, any other value manual sizing; 1 (the script's `manual`) also stops the layout from moving buttons | 0 |
| `+0xE8` | column height (automatic sizing) | 128 |
| `+0xEC`, `+0xED` | `gx`, `gy` (bytes) | 4, 4 |
| `+0xEE`, `+0xF0`, `+0xF2`, `+0xF4` | outline colours left, top, right, bottom (**16-bit**) | 0 |
| `+0xF8` | selected button ID | 0 |
| `+0xFC` | background image (a `cIGZBuffer`, counted) | null |
| `+0x100`, `+0x104` | label font, button font (counted) | null |
| `+0x108`…`+0x118` | font colours 0–4 (native): label normal, label disabled, button normal, button highlighted, button disabled | −1 (unset) ×5 |

## 2. The `cIGZWinOptGrp` slots *[compat]*

### 2.1 Table (vtable `0x00AE1288`, 30 slots)

Slots 1–3 are the shared AddRef, Release and `AsIGZWin` thunks.

| Slot | Address | Behaviour |
|---|---|---|
| 4 `IsFlagSet(f)` | `0x00999055` (shared) | `(flags & f) ≠ 0` |
| 5 `SetFlag(f, on)` | `0x009C7924` | thunk to `0x009C73B1`, §2.2 |
| 6 `SetGutters(gx, gy)` / 7 `GetGutters(out, out)` | `0x009C7380` / `0x009C7398` | store, then `AutoSize` / read; true |
| 8 `SetLabelFont(font)` | `0x009C7960` | store (counted); with flag 1, the label's slot 18 `(font)` and slot 5 `(0, 0)`; then §2.3's label refresh; true |
| 9 `SetBtnFont(font)` | `0x009C79E9` | store (counted); for each button, button slot 19 `(font)` (`0x009C7622`); §2.3's button refresh; true |
| 10 `SetFontColor(i, c)` | `0x009C7A3B` | colour `i := c` **before** the range check; `i` < 0 or > 4 → false; 0–1 → label refresh, 2–4 → button refresh; true |
| 11 `GetFontColor(i, out)` | `0x009C74DD` | `i < 5` and set → the colour, true; else false (no lower bound) |
| 12 `ClearFontColor(i)` | `0x009C74FE` | 0–1 with a label font, 2–4 with a button font: `SetFontColor(i, that font's slot 12)` (its default colour), true; else false |
| 13 `GetValue()` | `0x009D8564` (shared) | the selected ID |
| 14 `SetValue(id)` | `0x009C7288` | §2.4 |
| 15 `OptBtnAdd(id, caption, unused, image, rect)` | `0x009C82A2` | §2.5 |
| 16 `OptBtnDelete(id)` | `0x009C712F` | the child with that ID (window slot 34), if the manager knows it (`0x0099BE32`): `ChildDelete` (window slot 16); the selection is cleared when it was that ID; `InvalidateAllParents`; true. Else false |
| 17 `OptBtnMoveTo(id, x, y)` | `0x009C717D` | that child's window slot 56 `(x, y)`; false when not found |
| 18 `OptBtnSize(id, w, h)` | `0x009C71BE` | its window slot 53 `(w, h)`; false when not found |
| 19 `GetOptBtn(id, out)` | `0x009C71FF` | window slot 36 `(id, 0x8810, out)` and its result |
| 20 `SetBackBMP(image)` | `0x009C7357` | store (counted); no redraw; true |
| 21 `SetOutlineColors(l, t, r, b)` | `0x009C7415` | each `NativeToPacked` (window slot 128), **low 16 bits** stored; redraw; true |
| 22 `GetOutlineColors(out ×4)` | `0x009C7478` | each 16-bit value through `PackedToNative` (window slot 127); true |
| 23 `SetBtnCLSID(id)` | `0x00945E47` (shared) | `+0xE0 := id`; true |
| 24 `AutoSize()` | `0x009C792F` | if `GetMinMaxInfo(&m)`: `SetSize(m.minW, m.minH)` (window slot 53); true |
| 25, 26 | `0x0093878E` (shared) | false |
| 27 `GetMinMaxInfo(out)` | `0x009C83C2` | fill type 0 → automatic, else manual (§3) |
| 28 `SetAutoFillType(t, n)` / 29 `GetAutoFillType(out, out)` | `0x009C7328` / `0x009C7340` | `+0xE4 := t`, `+0xE8 := max(n, 1)` (no relayout) / read; true |

### 2.2 Flags (`+0xDC`) and `SetFlag(f, on)` (`0x009C73B1`)

| Bit | Meaning (legacy-control.md §9.20 names) |
|---|---|
| `0x1` | `maincaption`: the label exists |
| `0x2` | `itemcaption`: new buttons get button flag 4 (has a caption) |
| `0x4` | `drawfill`: fill the area when there is no background image |
| `0x8` | outline |
| `0x100000`, `0x400000` | label normal / disabled colour differs from the label font's default (§2.3) |
| `0x1000000`, `0x2000000`, `0x4000000` | button normal / highlighted / disabled colour differs from the button font's default |

`SetFlag` sets or clears `f`, then: `f` = 1 off → delete the label (window slots 34, 16); `f` = 1 on → create it
(§4); `f` = 4 or 8 → redraw. Other bits change nothing else (existing buttons keep their caption flag). True.

### 2.3 Colour refreshes

- **Label refresh** (`0x009C7630` then `0x009C76E9`), only with flag 1: without a label font, stop (false). With
  `d` = the label font's slot 12: flag `0x100000 := colour 0 ≠ d`, `0x400000 := colour 1 ≠ d`. Then, when the label
  exists: enabled (window slot 67 with 2) and `0x100000` → label slot 10 `(colour 0)`; disabled and `0x400000` →
  label slot 10 `(colour 1)`; redraw.
- **Button refresh** (`0x009C7686` then `0x009C7768`): without a button font, the flags are left; else `d` = its
  slot 12 and `0x1000000`, `0x2000000`, `0x4000000 :=` colours 2, 3, 4 `≠ d`. Then redraw and for each button
  (`0x009C7543`): for each of the three flags, set → button slot 23 `(2k, c)` and `(2k + 1, c)` with the colour
  (pairs 0 normal, 1 highlighted, 2 disabled); clear → button slot 26 `(2k)`, `(2k + 1)`.

### 2.4 Selection

**`SetValue(id)`** (slot 14): the button with the current ID (window slot 36 as `0x8810`), if any, gets
`SetBtnFlag(1, 0)` (unchecked). Then, if a button has the new ID: store it, `SetBtnFlag(1, 1)`, and
`PostMsg(+0x4C, 3, 0x88710F1C, GetID(), id)` (window slot 146; `id` is a push left before the argument-less
`GetID`); true. Otherwise false, the old button stays unchecked and the stored ID is kept *[quirk]*. Selecting the
current ID again posts again.

**`GZOnCommand(code, d)`** (window slot 143, `0x009C721D`): `code` = `0x287259F6` (a button was clicked; `d` is its
ID) → `SetValue(d)`; returns whether `code` was that.

### 2.5 `OptBtnAdd(id, caption, unused, image, rect)` (slot 15)

1. False for `id` = 0, when a child already has that ID (window slot 34), or when no button font can be found (§4).
2. `CreateCtrl(0xA8963E4B, 0x8810, &b)` on the control manager (a default radio button, widget-ctrlmgr.md §5); none
   → false.
3. With an image: button slot 12 `SetStyle(4, image, rect)`. Then button slots 47 `SetID(id)`, 49
   `SetCaption(caption)`, 19 `SetFont(button font)`, 5 `SetBtnFlag(4, flag 2 set)`.
4. `ChildAdd` (window slot 14) of its window part (button slot 3) gives the result; button refresh; lay out (§3)
   with `(W, H)`; redraw; release `b`.

## 3. Geometry *[compat]*

**Manual minimum** (`0x009C7C6C`): `minW`, `minH` = the largest right and bottom edge (window slots 45, 46) of any
child, plus `gx`, `gy`; maximum `0xFFFF` × `0xFFFF`.

**Automatic minimum** (`0x009C7E26`), with column height `ch = +0xE8 − gy`:
1. `minW := gx`, `minH := gy`, `y0 := gy`. With flag 1 and a label: label slot 5 `(0, 0)`; `minW := max(minW,
   label W + gx)`; `minH += label H`; `y0 := 2·gy + label H`.
2. `x := gx`, `cw := 0`, `y := y0`. For each button of size `bw × bh`: `cw := max(cw, bw)`; if `y + bh > ch`, a new
   column: `x += gx + cw`, `cw := bw`, `y := y0`. Then `y += bh`, `minW := max(minW, x + cw)`, `minH := max(minH, y)`.
3. `minW += gx`, `minH += gy`; maximum `0xFFFF` × `0xFFFF`. True.

The overflowing button's width is counted in the previous column's step *[quirk]*.

**Layout** (`on_layout(w, h)`, `0x009C8002`): with flag 1 and a label: label slot 5 `(0, 0)`, label window
`MoveTo(gx, gy)` (window slot 56), `y0 := 2·gy + label H`; else `y0 := gy`. Unless the fill type is 1: `x := gx`,
`cw := 0`, `y := y0`; for each button: if `y + bh > h − gy`, `x += gx + cw` and `y := y0`; `cw := max(cw, bw)` (never
reset); `MoveTo(x, y)`; `y += bh`. The layout's columns follow the window height and a running widest button, so
they need not match the automatic minimum *[quirk]*.

**`SetArea(l, t, r, b)`** (window slot 55, `0x009C81F7`): `w := max(r − l, 1)`, `h := max(b − t, 1)`. Unless both
equal the current `W`, `H`: fill type 0 → `+0xE8 := h`; then clamp `w`, `h` into `GetMinMaxInfo`'s range.
`cGZWin::SetArea(l, t, l + w, t + h)`, then the layout with `(w, h)`.

## 4. Lifecycle

- **Fonts on demand** (`0x009C7A8A` buttons, `0x009C7B29` label): when the font is missing, the control manager's
  `SystemFont(0xC8963C5C)`, or `SystemFont(0x68963C4C)`; unset colours of that kind take its slot 12; then
  `SetBtnFont` / `SetLabelFont`. True when a font is held afterwards.
- **Label creation** (`0x009C7D0A`): true at once when the label exists (window slot 19). Otherwise, with the label
  font and the control manager: `CreateCtrl(0x48963E33, 0x212CDC1F, &t)`; its window gets ID `0x02E3A3E5` (window
  slot 64); label slots 4 `(the group's caption)`, 18 `(label font)`, 7 `(2, 0)`, 16 `(0)` (not filled), 5 `(0, 0)`;
  `ChildAdd`; label refresh. The result is `ChildAdd`'s (false when anything is missing).
- **`Init`** (window slot 4, `0x009C817E`): true at once when already initialised. Both fonts on demand, then the
  label with flag 1, else false. Fill type 0 → `AutoSize`; else the layout with `(W, H)`. `cGZWin::Init`.
- **`SetCaption(s)`** (window slot 74, `0x009C723F`): the label's slot 4 `(s)` when it exists; then
  `cGZWin::SetCaption(s)`.
- **`SetFlag(f, on)`** (window slot 68, `0x009C77BB`): `cGZWin::SetFlag`; for `f` = 2 (enabled), every button's
  window gets `SetFlag(2, on)` (`0x009C710F`), then the label refresh. The base result is returned.
- **`Shutdown`** (window slot 5, `0x009C70CB`): `HideWindow` (window slot 70), release the background image,
  `SetLabelFont(null)`, `SetBtnFont(null)`, `cGZWin::Shutdown`; true.
- **Destructor** (body `0x009C7790`, deleting form `0x009C7C50`): restore the vtables, `Shutdown`, `~cGZWin`.

## 5. Painting (`GZPaint`, window slot 88, `0x009C7803`) *[compat]*

1. Background image: draw-context slot 39 `(image, image rect (buffer slot 12), draw area, 0, 0)`. Without one, flag
   4: fill the draw area with `GetFillColorRGB` (window slot 126; draw-context slots 21, 35).
2. Flag 8: four lines (draw-context slots 21, 34) with the 16-bit outline colours zero-extended: left `(l, t)–(l, b −
   1)`, top `(l, t)–(r − 1, t)`, right `(r − 1, t)–(r − 1, b − 1)`, bottom `(l, b − 1)–(r − 1, b − 1)`.
3. True. The label and buttons paint themselves as children.

## 6. Quirks and cases

Quirks:
1. The outline colours keep only the low 16 bits of the 32-bit conversion (§2.1), and are painted that way.
2. `SetFontColor` writes before checking the index, so an index above 4 overwrites the next fields (and a negative
   one the preceding ones).
3. `SetValue` with an unknown ID unchecks the selected button and keeps its ID (§2.4).
4. The automatic minimum and the layout break columns differently (§3).
5. `ClearFontColor` resets to the font's default colour instead of "unset".
6. `SetFlag(2, …)` (item captions) does not change existing buttons; the button class ID (slot 23) is never used.
7. `0x0099BE32` in this owner is `cGZWin`'s "is this window valid" helper: window manager (`+4`) slot 24 with the
   argument.

Cases:
1. Defaults (`gx = gy = 4`, column height 128) with flag 1 cleared (no label), three 100×20 buttons: automatic
   minimum: `ch` = 124; `y` = 4 → 24 → 44 → 64; `minW = 4 + 100 + 4 = 108`, `minH = 64 + 4 = 68`. `AutoSize` makes
   the group 108×68 (and, fill type 0, the column height 68); the layout puts them at `(4, 4)`, `(4, 24)`, `(4, 44)`.
2. The same with seven buttons and column height 128: the seventh (`y` = 124, `124 + 20 > 124`) starts column 2 at
   `x = 4 + 4 + 100 = 108`; `minW = 108 + 100 + 4 = 212`, `minH = 124 + 4 = 128`.
3. Clicking button 3: it posts `0x287259F6` with ID 3; the group's `SetValue(3)` unchecks the old button, checks
   button 3 and posts `{3, 0x88710F1C, group ID, 3}` to `+0x4C`.

## 7. Function inventory (owner `cGZWinOptGrp`, 49)

`0x0099BE32` window-valid helper, `0x009C7094` QI, `0x009C70CB` `Shutdown`, `0x009C710F` enable callback,
`0x009C712F` slot 16, `0x009C717D` slot 17, `0x009C71BE` slot 18, `0x009C71FF` slot 19, `0x009C721D`
`GZOnCommand`, `0x009C723F` `SetCaption`, `0x009C7288` slot 14, `0x009C7328` slot 28, `0x009C7340` slot 29,
`0x009C7357` slot 20, `0x009C7380` slot 6, `0x009C7398` slot 7, `0x009C73B1` `SetFlag` body, `0x009C7415` slot 21,
`0x009C7478` slot 22, `0x009C74DD` slot 11, `0x009C74FE` slot 12, `0x009C7543` button colour callback,
`0x009C7622` button font callback, `0x009C7630` label flags, `0x009C7686` button flags, `0x009C76E9` label colour,
`0x009C7768` button colours, `0x009C7790` destructor body, `0x009C77B0` QI thunk, `0x009C77BB` window `SetFlag`,
`0x009C7803` `GZPaint`, `0x009C7924` slot 5, `0x009C792F` slot 24, `0x009C7960` slot 8, `0x009C79E9` slot 9,
`0x009C7A3B` slot 10, `0x009C7A8A` button font on demand, `0x009C7B29` label font on demand, `0x009C7BB8`
constructor, `0x009C7C50` deleting destructor, `0x009C7C6C` manual minimum, `0x009C7D0A` label creation,
`0x009C7DFF` `CreateNewObject`, `0x009C7E26` automatic minimum, `0x009C8002` layout, `0x009C817E` `Init`,
`0x009C81F7` `SetArea`, `0x009C82A2` slot 15, `0x009C83C2` slot 27.

Not covered: the draw-context slots (WGN1 in widget-wingen.md).
