# `cGZWinCombo`: the combo box (behaviour spec)

Scope: the `cGZWinCombo` owner as tools/coverage.py assigns it on `bb641878` (55 functions, 4,820 bytes;
inventory in §7). The combo box keeps a list of strings and a selected index, shows the selection in a child text
edit, and drops a modal list box below itself to choose another. Its script properties are in legacy-control.md
§9.13; it is created as class `0x59B` (widget-comdirector.md §2, `0x13C` bytes, constructor argument 1). Callers
read it through IID `0x412CE496` and react to message `{3, 0xE8711255}` (budget-window.md, resource-catalog.md).

Related:
- widget-toolkit.md: the `cGZWin` base (fields §2: absolute area `+0x14`…`+0x20`, draw area `+0x24`, notification
  target `+0x4C`, draw context `+0x6C`; flags §3: `0x2` enabled, `0x4000` initialised; §17.8 colours);
- text-edit.md: the child text edit (`cIGZWinTextEdit` `0x231A1C57`, vtable `0x00ADFA68`; its bits 1 editable,
  `0x200` single line, `0x400` notify on Return, and the messages `0x9B1552D5`–`D7`);
- widget-controls.md §5: the list box used for the drop-down (`L` slots);
- widget-ctrlmgr.md: `SystemFont` (slot 5), `SystemBMP` (slot 7), `GetColor` (slot 32).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `SetEditable`, the window `SetFlag`, the key
handler, the paint, the changed message and the text-edit slots were checked on the instructions (interface
methods run on `+0xD8`). The vtables `0x00AE2970` (window) and `0x00AE2BE8` (`cIGZWinCombo`) and the text-edit
table `0x00ADFA68` were read. Written 2026-10-24 (re-gzcombo7).

Conventions:
- Offsets are from the window start; the interface is at `+0xD8`. "Window slot n" is a `cIGZWin` slot (or one the
  combo adds, §3); "slot n" alone a `cIGZWinCombo` slot; "text slot n" the text edit's interface.
- "The text edit" is the child with ID `0x53430D98` fetched as `0x231A1C57` (window slot 36). `fw`, `fh` are the
  arrow cell's width and height (`+0x11C`, `+0x120`). "Redraw" is window slot 92. `W`, `H` are window slots 41, 42.
- "Changed" = `SendMsg(+0x4C, 3, 0xE8711255, GetID(), 0)` (window slot 144; the `0` is a push left before the
  argument-less `GetID`, window slot 63; window slot 156, `0x009CF071`).

## 1. Identity and fields *[compat]*

- **`QueryInterface`** (`0x009CE74A`): `0x412CE496` → `+0xD8` (null for null) AddRef'd; a null out pointer gives
  false without writing; else `cGZWin::QueryInterface`. Slot 0 of the interface (`0x009CF875`) is the thunk.
- **Window overrides**: 0, 3 `DoMessage`, 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 68 `SetFlag`, 88 `GZPaint`, 106
  `SetFillColor(native)`, 130 key down, 134 left down, 148 destructor (135–138 are the shared "false" stub); it adds
  slots 151–156 (§3).
- **Constructor** (`0x009CF772`, `(editable)`): `cGZWin` constructor, the vtables, an empty string list, the values
  below; then `SetFillColor(WCM.GetColor(0xA8963FB2))` (its own, so a text edit would follow).

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | combo flags (bit 1: widen the drop-down to the longest string) | 0 |
| `+0xE0` | the strings (`std::list<cRZString>`) | empty |
| `+0xE4` | selected index | −1 |
| `+0xE8` | dropped down (byte) | 0 |
| `+0xEC` | number of strings | 0 |
| `+0xF0` | font (counted) | null |
| `+0xF4`, `+0xF8`, `+0xFC` | font colours 0 normal, 1 disabled, 2 highlighted | `0xDEADBEEF` (unset) |
| `+0x100` | highlight colour | `0xDEADBEEF` |
| `+0x104` | highlight fill alpha (float) | 1.0 |
| `+0x108` | drop-down outline colour | 0 |
| `+0x10C` | editable (byte) | the argument |
| `+0x110` | arrow image (a `cIGZBuffer`, counted) | null |
| `+0x114`, `+0x118` | arrow source x, y | 0, 0 |
| `+0x11C`, `+0x120` | `fw`, `fh` | 16, 8 |
| `+0x124`, `+0x128` | arrow position in the window | 0, 0 |
| `+0x12C` | source x of the arrow frame shown | 0 |
| `+0x130` | arrow colour without an image (packed, "combo-down colour") | `0xDEADBEEF` |
| `+0x134` | gutter between the text and the arrow (byte) | 1 |
| `+0x135`, `+0x136` | text gutters x, y (bytes) | 3, 0 |
| `+0x137` | strings changed since the widest was measured (byte) | 0 |
| `+0x138` | widest string (pixels) | 0 |

## 2. The `cIGZWinCombo` slots *[compat]*

Vtable `0x00AE2BE8`, 42 slots; 1–3 are the shared AddRef, Release and `AsIGZWin` thunks. Setters return true
unless noted.

| Slot | Address | Behaviour |
|---|---|---|
| 4 | `0x0092EC14` | tail call to slot 5 |
| 5 `GetTextEdit()` | `0x009CE781` | window slot 151; none → create it (152) and look again; else null |
| 6 `SetFlag(f)` / 8 `ClearFlag(f)` | `0x009CE973` / `0x009CE98B` | when it changes, slot 7 with the new word |
| 7 `SetFlags(v)` | `0x009CE9A5` | store; when bit 1 changes, mark the strings changed |
| 9 `IsFlagSet(f)` / 10 `GetFlags()` | `0x00999055` / `0x009D7E63` (shared) | |
| 11 `SetFont(font)` | `0x009CEC0B` | release the old font **before** AddRef'ing the new; text slot 9 `(font)` |
| 12 `SetFontColor(i, c)` | `0x009CEAA2` | `i` ∉ 0–2 → false. Store; if changed: redraw and text slot 12 `(0, 2 or 4 for i = 0, 1, 2; c)` |
| 13 `GetFontColor(i, out)` / 14 `GetFontColor(i)` | `0x009CEB16` / `0x009CEB0B` | unset → false / the raw value (no range check) |
| 15 `ClearFontColor(i)` | `0x009CEB34` | slot 12 `(i, 0xDEADBEEF)`: the sentinel also reaches the text edit as a colour *[quirk]* |
| 16 `SetHighlightColor(c)` / 17 get | `0x009CEC8A` / `0x009CECB8` | store; text slot 12 `(5, c)` / unset → false |
| 18 `SetHighlightFillAlpha(a)` / 19 get | `0x009CECD1` / `0x009CED04` | clamped to 0–1 |
| 20 `GetEditable()` / 21 `SetEditable(on)` | `0x009CEC4A` / `0x009CEC4E` | the byte / text slot 76 `SetFlag(1)` or 78 `ClearFlag(1)`, then store |
| 22 `SetDropDownOutlineColor(c)` / 23 get | `0x009CED12` / `0x009CED1E` | store only |
| 24 `RemoveString(i)` | `0x009CF6EC` | `i ≤ count − 1` and found: erase, `count −= 1`, mark changed; true. Selection untouched |
| 25 `RemoveString(s)` | `0x009CEE2C` | slot 32 then slot 24 |
| 26 `RemoveAllStrings()` | `0x009CF742` | empty the list, count 0, selection −1, mark changed (the text is left) |
| 27 `InsertString(s, at)` | `0x009CF975` | `at` = 0 → front; `0 < at < count` → before string `at`; `at` ≥ count or −2 → back; any other negative → false. `count += 1`, mark changed |
| 28 `SetSelection(i, notify)` | `0x009CF8C7` | §2.1 |
| 29 `GetSelection(out i, out s)` | `0x009CEDE5` | the index, slot 31 into `s`; true even for −1 |
| 30 `GetSelectionIndex()` | `0x009CEDE1` (shared) | the index |
| 31 `GetStringFromIndex(i, out s)` | `0x009CEDFF` | window slot 153; found → `s`'s slot 14 `(stored string)` (copy); the result |
| 32 `GetIndexFromString(out i, s)` | `0x009CF343` | first stored string for which `s`'s slot 10 `(stored, 1)` is true |
| 33 `SetComboDownArrow(image, rect)` | `0x009CF122` | §3.2 |
| 34 `GetComboDownArrow(out image, out rect)` | `0x009CF1FB` | the image (not AddRef'd) and `(x, y, x + 4·fw, y + fh)`; true when there is an image |
| 35 `SetComboDownColor(c)` / 36 get | `0x009CEA7A` / `0x009CEA94` | store and redraw / read |
| 37 `SetBtnGutter(g)` / 38 get | `0x009CEB45` / `0x009CEB79` | store; layout `(W, H)` (window slot 154) |
| 39 `SetTextGutters(x, y)` / 40 get | `0x009CEB87` / `0x009CEBF4` | store; with a text edit: text slots 40 `(x)`, 42 `(y)`, then `SetSize(W, H)` (window slot 53) |
| 41 `GetMinMaxInfo(out)` | `0x009CF2DD` | §3.2 |

### 2.1 `SetSelection(i, notify)` (slot 28)

`i` < 0 or ≥ count → false. Store `i`; with a text edit, a copy of string `i`, trimmed (`cRZString::Trim`), goes to
text slot 49 `SetText(chars, length, 0)`. Redraw; `notify` → changed. True.

## 3. Window behaviour *[compat]*

### 3.1 Added window slots

| Window slot | Address | Behaviour |
|---|---|---|
| 151 | `0x009CEE5E` | the text edit or null (the reference is released at once) |
| 152 | `0x009CEE91` | create the text edit, below |
| 153 `GetString(i, out p)` | `0x009CF38C` | the stored `cRZString` at `i` (`i` ≤ count − 1; a negative `i` is not found) |
| 154 `Layout(w, h)` | `0x009CEFF6` | below |
| 155 `DropDown()` | `0x009CF3C8` | §4 |
| 156 | `0x009CF071` | changed |

**Creating the text edit**: true when it exists. Else GZCOM `(0x231A1C58, 0x231A1C57)`, or `new cGZWinTextEdit
(0x290, 0x11FF)` AddRef'd. Its window: ID `0x53430D98` (window slot 64), `MoveTo(0, 0)`, `SetFillColor(native)` with
the combo's fill colour as native (window slots 106, 103). Text slots 76 `(0x200)`, `(1)`, `(0x400)` (single line,
editable, notify on Return), 9 `(font)`, 40 `(+0x135)`, 42 `(+0x136)`, 37 `(1, WCM.GetColor(0xA8963FAD))` (outline).
`ChildAdd` (window slot 14) gives the result; release; then `SetEditable(+0x10C)`.

**`Layout(w, h)`**: without an arrow image `fh := h`. Arrow position `(w − fw, (h − fh)/2)`; the text edit window gets
`SetSize(w − fw − gutter, h)` (`+0x134`); redraw.

### 3.2 Size and arrow

**`GetMinMaxInfo(out)`**: minimum `(16, 8)` without an arrow image, else `(fw, fh)`; with a text edit the width gains
`gutter + 1` and the height is at least 1; maximum `0x7FFFFFFF` both ways. True.

**`SetArea(l, t, r, b)`** (window slot 55, `0x009CF097`): `w := max(r − l, minW)`, `h := max(b − t, minH)`. If `(w,
h)` differs from `(W, H)`: `Layout(w, h)` and `r := l + w`, `b := t + h`. Then `cGZWin::SetArea(l, t, r, b)`. When
the clamped size equals the current size, the **unclamped** rectangle is applied *[quirk]*.

**`SetComboDownArrow(image, rect)`** (slot 33): release the old image, store, AddRef the new. No image: source `(0,
0)`, `fw := 16`, `fh := H`. Image without `rect`: source `(0, 0)`, `fw :=` buffer slot 9 (width) / 4, `fh :=` buffer
slot 10 (height). With `rect`: source `(l, t)`, `fw := (r − l)/4`, `fh := b − t`. Then `SetSize(max(W, fw), max(H,
fh))`. The shown frame `+0x12C` is not updated here (the next enable change or drop sets it).

The arrow image holds four frames side by side: 0 disabled, 1 normal, 2 dropped down (frame 3 is not used).

### 3.3 Input and messages

- **Left down** (window slot 134, `0x009CED2C`): `DropDown()`; true.
- **Key down** (window slot 130, `0x009CED39`, `(key, mods)`): Up → `SetSelection(index − 1, 1)`; Down with modifier
  bit `0x4` → `DropDown()`; Down → `SetSelection(index + 1, 1)`; true. Other keys: `SendMsg(+0x4C, 5, key, mods, 0)`
  and its result.
- **`DoMessage(msg)`** (window slot 3, `0x009CED9F`): type `0x9B1552D5` (Return in the text edit) → changed; types
  `0x9B1552D6`, `0x9B1552D7` (a press on a non-editable text edit) → `DropDown()` unless the combo is editable; true.
  Other types → `cGZWin::DoMessage`.
- **`SetFlag(f, on)`** (window slot 68, `0x009CE9C0`): with bit 2 in `f` while dropped down → false. Else the base
  `SetFlag`; for `f` = 2 the text edit's window gets `SetFlag(2, on)` (no null test: without a text edit this
  dereferences null *[quirk]*), the shown frame becomes `x + fw` (enabled) or `x` (disabled), redraw. The base result.
- **`SetFillColor(native)`** (window slot 106, `0x009CEA4B`): `cGZWin`'s, then the text edit window's with the same
  argument when there is a text edit.

### 3.4 Lifecycle and painting

- **`Init`** (window slot 4, `0x009CE7B3`): true when already initialised. With the control manager, unset parts get
  defaults: arrow `SetComboDownArrow(SystemBMP(0x68963ED8), null)`; arrow colour `GetColor(0x28963F93)`; font
  `SystemFont(0x68963C4C)`; colour 1 `GetColor(0x08963FB9)`; colour 2 `GetColor(0x28963FBC)`; highlight
  `GetColor(0x88963FB4)` (colour 0 has no default). Without a font → false. Else create the text edit, `SetFlag(2,
  GetFlag(2))`, `SetSelection(index, 0)`, `Layout(W, H)`, `cGZWin::Init`.
- **`Shutdown`** (window slot 5, `0x009CE936`): `HideWindow` (window slot 70), release the arrow image and the font,
  `cGZWin::Shutdown`.
- **Destructor** (body `0x009CF880`, deleting form `0x009CF8AB`): restore the vtables, `Shutdown`, destroy the
  string list, `~cGZWin`.
- **`GZPaint`** (window slot 88, `0x009CF241`): the arrow cell at the draw area's origin plus the arrow position, `fw
  × fh`. No image: `RZDrawRaisedRect(context, cell, +0x130, raised = not dropped down, 1)`. Image: draw-context slot
  38 `(image, (shown frame x, source y, + fw, + fh), cell)`. True. The text edit paints itself.

## 4. The drop-down (`DropDown`, window slot 155) *[compat]*

1. Already dropped down → false. `new cGZWinListBox(1)` (`0x168` bytes, a drop list); none → false. AddRef.
2. Shown frame `x + 2·fw`, dropped down := 1, redraw.
3. Width `w` := the combo's absolute width. With combo flag 1: when the strings changed, re-measure the widest
   (font slot 48 `(chars, length)` for each) and clear the mark; `w := max(w, widest + 8)`, then `w := min(w,
   main window W − 16)`, or 1024 without a main window (window slot 10).
4. The list window: ID `0x53430D99`; `SetArea(abs left, abs bottom, abs left + w, abs bottom + H)` (window slot 55);
   `SetFillColor(native)` with the combo's fill colour. `L` slots: 38 `(font)`; 39 `(i, colour i)` for each set font
   colour; 51 `(1)`; 55 `(outline, outline)` with `+0x108`; 42 `(highlight)`; 46 `(alpha)`; 57 `(0, 0)`; 59 `(gx,
   gy)` with the text gutters.
5. The list window's flag `0x10000` off; `L` slot 7 `(&string, −2, 0)` for each string in order; flag `0x10000` on.
   `L` slot 10 `(index, 0)`.
6. `r :=` the window manager's `DoModalWin` (WM slot 41) on the list window; `SetSelection(r, 1)` (an invalid `r`
   leaves the selection and sends nothing).
7. The list window's `Shutdown` (window slot 5) and Release; shown frame `x + fw`, dropped down := 0, redraw. True.

## 5. Quirks and cases

Quirks:
1. `SetArea` applies the unclamped rectangle when the clamped size equals the current one (§3.2).
2. `SetFlag(2, …)` before a text edit exists dereferences null (§3.3).
3. `ClearFontColor` passes `0xDEADBEEF` to the text edit as a real colour.
4. Inserting or removing strings does not move the selected index; `RemoveAllStrings` leaves the text.
5. `SetFont` and `SetComboDownArrow` release the old object before AddRef'ing the new, so passing the held object
   with a single reference frees it first.
6. `GetFontColor(i)` (slot 14) has no range check.

Cases:
1. Strings "A", "B", "C", selection 0: Down → `SetSelection(1, 1)`: text "B", `{3, 0xE8711255, id, 0}` to `+0x4C`. Down
   at index 2 → `SetSelection(3, 1)` → false, nothing sent.
2. Default arrow (no image), combo `SetArea(0, 0, 100, 20)`: minimum `(16 + 1 + 1, 8)` with a text edit; `Layout(100,
   20)`: `fh = 20`, arrow at `(84, 0)`, text edit `100 − 16 − 1 = 83` wide.
3. A press on a non-editable combo's text: the text edit sends `0x9B1552D6`; the combo drops its list below itself;
   choosing row 2 makes `SetSelection(2, 1)` and the changed message.

## 6. Notes

- The constructor's argument is the editable byte; the class factory passes 1 (widget-comdirector.md §2), so combos
  start editable until `SetEditable(0)` or the script's `editable` property (legacy-control.md §9.13).

## 7. Function inventory (owner `cGZWinCombo`, 55)

`0x0092EC14` slot 4, `0x009CE74A` QI, `0x009CE781` slot 5, `0x009CE7B3` `Init`, `0x009CE936` `Shutdown`,
`0x009CE973` slot 6, `0x009CE98B` slot 8, `0x009CE9A5` slot 7, `0x009CE9C0` window `SetFlag`, `0x009CEA4B`
`SetFillColor`, `0x009CEA7A` slot 35, `0x009CEA94` slot 36, `0x009CEAA2` slot 12, `0x009CEB0B` slot 14,
`0x009CEB16` slot 13, `0x009CEB34` slot 15, `0x009CEB45` slot 37, `0x009CEB79` slot 38, `0x009CEB87` slot 39,
`0x009CEBF4` slot 40, `0x009CEC0B` slot 11, `0x009CEC4A` slot 20, `0x009CEC4E` slot 21, `0x009CEC8A` slot 16,
`0x009CECB8` slot 17, `0x009CECD1` slot 18, `0x009CED04` slot 19, `0x009CED12` slot 22, `0x009CED1E` slot 23,
`0x009CED2C` left down, `0x009CED39` key down, `0x009CED9F` `DoMessage`, `0x009CEDE5` slot 29, `0x009CEDFF` slot 31,
`0x009CEE2C` slot 25, `0x009CEE5E` window slot 151, `0x009CEE91` window slot 152, `0x009CEFF6` window slot 154,
`0x009CF071` window slot 156, `0x009CF097` `SetArea`, `0x009CF122` slot 33, `0x009CF1FB` slot 34, `0x009CF241`
`GZPaint`, `0x009CF2DD` slot 41, `0x009CF343` slot 32, `0x009CF38C` window slot 153, `0x009CF3C8` window slot 155,
`0x009CF6EC` slot 24, `0x009CF742` slot 26, `0x009CF772` constructor, `0x009CF875` QI thunk, `0x009CF880` destructor
body, `0x009CF8AB` deleting destructor, `0x009CF8C7` slot 28, `0x009CF975` slot 27.

Not covered: the list box's own slots (widget-controls.md §5, WC1) and the draw-context slots (WGN1).
