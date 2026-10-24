# `cGZWinMsgBox` and `cGZWinGenDlg`: the toolkit dialogs (behaviour spec)

Scope: the owners `cGZWinMsgBox` (27 functions, 3,756 bytes) and `cGZWinGenDlg` (30 functions, 3,231 bytes) as
tools/coverage.py assigns them on `bb641878`. The message box is the toolkit's standard alert (text, optional caption,
one to three standard buttons, modal end with the button ID); the generic dialog is a framed window with a
procedural builder for controls (`AddBtn`, `AddText`, …) used by game dialogs (cheat-dialog.md).

Related: widget-toolkit.md (the `cGZWin` base, `set_mouse_down_offset` §18, flags §3); modal-dialogs.md (`DoModalWin`,
the end message `0xF`); widget-ctrlmgr.md (the factories, fonts, colours, texts); widget-text.md, widget-button.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the dialog builder's argument sizes and
`AddListBox` were checked on the instructions, the button literals and the border table `0x00B17C24` read.
Vtables: message box `0x00AE2638` (`cIGZWinDlg`, 4 slots, then the window) and `0x00AE2604` (`cIGZWinMsgBox`,
13 slots); generic dialog `0x00AE01E8` (22 slots, then the window `0x00AE0240`). Written 2026-10-24 (re-gzdialogs7).

## 1. The message box *[compat]*

**Object** (constructor `0x009CDBA5`, `0x118` bytes): the `cGZWinDlg` base (`0x009CDB5A`: `cGZWin`, then the
`cIGZWinDlg` interface at `+0xD8`), the `cIGZWinMsgBox` interface (IID `0x57C`) at `+0xDC`, then:

| Offset | Meaning | Initial |
|---|---|---|
| `+0xE0` | message text (`cRZString`) | empty |
| `+0xF4` | style; the low 16 bits choose the buttons (§1.2) | 0 |
| `+0xF8` | caption background colour | `MakeFillColor(124, 144, 255)` |
| `+0xFC`, `+0x100` | default button, cancel button (counted) | null |
| `+0x104`, `+0x108` | caption and message text windows (`cIGZWinText`, counted) | null |
| `+0x10C` | background image | null |
| `+0x110`, `+0x114` | outline colours | (`Init`: `0xFFFFFFFF`, `0xFF808080`) |

**`QueryInterface`** (`0x009CD88C`): `0x57C` → `+0xDC`, AddRef'd; else `cGZWinDlg`'s. Thunks `0x009CDC2B`, `0x009CDC36`,
`0x009CDC41` (`+0xD8`) and `0x009CDC4C`, `0x009CDC57`, `0x009CDC62` (`+0xDC`). Destructor body `0x009CDC6D` (Shutdown,
the text, the `cGZWinDlg` body), deleting form `0x009CDFEF`.

`cIGZWinDlg` slot 3 (`0x009DA6C5`, `<shared>`): `EndDialog(result)` = window manager `GZWinMsgPost(0xF, window,
result)`.

### 1.1 The `cIGZWinMsgBox` slots

| Slot | Address | Behaviour |
|---|---|---|
| 3 | `0x009CDAAB` | the window (`this − 0xDC`) |
| 4 `SetMessage(text, style)` | `0x009CDAF1` | text, style; clears `+0xF8` and `+0xFC`…`+0x10C` |
| 5 `SetMessage(text, caption, style)` | `0x009CDAB2` | text; window caption := `caption` (slot 74); style; clears `+0xFC`…`+0x10C` |
| 6 | `0x009CDB21` | window slot 5 (`Shutdown`) |
| 7 / 8 `SetCaptionColor(r, g, b)` / `(native)` | `0x009CDB36` / `0x009CDB2C` | `+0xF8` |
| 9 / 10 / 11 / 12 | `0x009D8564`, `0x009D858F`, `0x009CDC23`, `0x009CDC27` (shared getters) | default button, cancel button, caption window, message window |

*Quirk*: slots 4 and 5 overwrite the counted fields without releasing them.

### 1.2 `Init` (`0x009CE00B`)

Once:
1. `s` = style & `0xFFFF`. Button-row width `b`: 100 for `s` = 0, 208 for 1–3, 316 for 4–5, else 200; dialog width
   `w := max(b, 200)`.
2. With a caption: a label (control manager slot 13, or a `cGZWinText` constructed directly) with the caption;
   text slot 16 `(1)`; fill := `+0xF8`; text flag 1 (clicks pass to the dialog, so the caption drags it); added; fit
   to text; `w := max(w, its width + 16)`.
3. The message: a label with the text; alignment 4 (centred), wrap on, fill off; added (failure → destroy, false).
4. Fit: message width := `w`, fit to text; repeat with `w += 16` until twice the text height is below `w`.
5. Buttons, in a row at `y` = message height + 24 (+ caption height), from `x = (w − b)/2 + 8`, the next ones at
   `+16`, then `+8`, past the previous button's width:

   | `s` | Buttons (default first, cancel last) |
   |---|---|
   | 0 | OK (both default and cancel) |
   | 1 | OK, Cancel |
   | 2 | Retry, Cancel |
   | 3 | Yes, No |
   | 4 | Yes, No, Cancel |
   | 5 | Abort, Retry, Ignore |
   | other | `Init` fails |

   `w` grows to hold the last button.
6. `SetArea(0, 0, w + 8, bottom + 8)`; caption and message widths `W − 16` at `(8, 8)` and below the caption;
   private buffer on; centred in the main window; fill (124, 144, 255); outline colours `0xFFFFFFFF`, `0xFF808080`; ID
   `0xA999B756`; `cGZWin::Init`.

**`create_and_add_button(id)`** (`0x009CDCA2`): the caption is the control manager's default text for `id` (slot 35)
or the English literal; the colour is red (192, 0, 0), green (0, 192, 0) or yellow (192, 192, 0):

| ID | Literal | Colour |
|---|---|---|
| `0x53018146` | `Abort` | red |
| `0x53018147` | `Cancel` | red |
| `0x53018148` | `Ignore` | yellow |
| `0x53018149` | `No` | red |
| `0x5301814A` | `OK` | green |
| `0x5301814B` | `Retry` | green |
| `0x5301814C` | `Yes` | green |

The button comes from control manager slot 9 `(id, "", 0, 0, 0)` or is constructed (style 2, ID `id`); its fill is the
colour; it is sized to fit (slot 33), loses button flag `0x20`, gets width 100 and is added; failure (or an unknown ID)
→ destroyed (window manager slot 23), null. The returned button is counted.

### 1.3 Behaviour

- **`Shutdown`** (`0x009CD8B9`): release the two buttons and two text windows; hide; private buffer off;
  `cGZWin::Shutdown`.
- **`GZPaint`** (`0x009CD92E`): no background → fill with the fill colour; else the image tiled (draw context slot 39);
  then `RZDraw3DRectangleOutline(draw area, +0x110, +0x114)`.
- **Keys** (`0x009CD9A8`): Enter → `GZOnCommand(0x287259F6, OK)`; Esc or Alt+F4 → `GZOnCommand(0x287259F6, Cancel)`;
  others nothing. True.
- **Mouse**: down sets the mouse-down offset, flag `0x400` (dragging) and capture (`0x009CD9E2`); move while dragging
  offsets the window by the movement (window slot 57, `0x009CDA45`); up clears both (`0x009CDA10`).
- **`GZOnCommand(type, id)`** (`0x009CDA7F`): `0x287259F6` → window manager `GZWinMsgPost(0xF, window, id)` (the modal
  loop ends with the button ID); returns whether the type matched.

`0x00736559` (29 bytes) is not code: it is the pointer value of the `"Yes"` literal (`0x00AE28C0`) read as an address.

## 2. The generic dialog (`cGZWinGenDlg`) *[compat]*

**Object** (constructor `0x009C2CEC (style)`): `cGZWin`, the interface (IID `0x20FEDAA6`, also 1) at `+0xD8`:

| Offset | Meaning | Initial |
|---|---|---|
| `+0xDC`, `+0xE0` | control font, title font (counted) | null |
| `+0xE8` | background image (counted) | null |
| `+0xF0..+0xFC` | border widths l, t, r, b | not set (`Init`, style 0: 6, 23, 6, 8) |
| `+0x100`, `+0x104` | command callback and its context | null |
| `+0x108` | style: 0 framed (9-slice image, title), 1 plain image, 2 fill colour | the argument |

**`QueryInterface`** (`0x009C218F`): `0x20FEDAA6` or 1 → `+0xD8` (null-safe), AddRef'd; else `cGZWin`'s. Thunk
`0x009C2B9B`. Destructor `0x009C2B7B`, deleting form `0x009C2D48`.

### 2.1 The builder slots

Every `Add*` takes the control's rectangle first, adds the control as a child and returns its window. "ID := self"
means the control's ID becomes its own window or interface address (no caller ID exists). Controls get the
control font and the fill colour `GetColor(0x68963F6F)` where noted.

| Slot | Address | Arguments | Control |
|---|---|---|---|
| 3 `SetBackground(img)` | `0x009C23FD` | image | release the old one, store, AddRef the new one (**not null-checked**) |
| 4 `AddListBox` | `0x009C2422` | rect | a `cGZWinListBox` (direct); ID := self; font; fill; **no failure check** |
| 5 `AddBMP` | `0x009C24AA` | rect, image | control manager slot 15 `(0, image)`; ID := self |
| 6 `AddBtn` | `0x009C251E` | rect, caption | control manager slot 9 `(0, caption, 0, 0, 0)`; ID := self |
| 7 `AddCombo` | `0x009C2594` | rect, flag | a `cGZWinCombo (flag)` (direct); ID := self; font; fill; no failure check |
| 8 `AddLineInput` | `0x009C261E` | rect | control manager slot 19 `(0, 1)`; font; fill; ID := self |
| 9 `AddSlider` | `0x009C26B9` | rect, a, b, c, orientation | control manager slot 16 `(0, orientation, a, b, c)`; ID := self |
| 10 `AddScrollbar` | `0x009C2732` | rect, a, b, c, orientation | control manager slot 17, the same |
| 11 `AddSpinner` | `0x009C27AF` | rect, a, b, c | control manager slot 18 `(0, 2, a, b, 1, c)`; ID := self; no failure check |
| 12 `AddText` | `0x009C2815` | rect, caption, alignment, filled | a label; font; alignment; text flag 1; `SetFilled(filled == 0)`; fill; then moved to the back (behind the other controls) |
| 13 `AddTextEdit` | `0x009C2900` | rect, text | a `cGZWinTextEdit (0x11FF)`; caption := text; font; fill; its text set from `text`; no failure check |
| 14 `AddCheck` | `0x009C29C0` | rect, caption | control manager slot 10 `(4, 0, caption, 0, 0, 0)` |
| 15 `AddOptionGroup` | `0x009C2A2D` | rect, caption | control manager slot 14 `(0, caption, rect)` |
| 16 / 17 / 18 | `0x009C2A97` / `0x009C2AE3` / `0x009C2B2F` | rect | a line (`0x260`), bar (`0x268`) or scatter (`0x254`) graph (widget-graphs.md) |
| 19 `GetBorderWidths(out[4])` | `0x0079BBE0` | | |
| 20 `GetStyle` | `0x006AAC50` (shared) | | `+0x108` |
| 21 `SetCommandCallback(fn, ctx)` | `0x0079BC10` | | |

With a failure check, a control that cannot be added is destroyed (window manager slot 23) and null returned.

### 2.2 Behaviour

- **`Init`** (`0x009C21C5`), once, with the control manager (without it: true and nothing else, not even
  `cGZWin::Init`): style 0 → no background → picture `0xC8963ED3` (AddRef) and private buffer on; borders (6, 23, 6, 8).
  No background and style ≠ 2 → false. No control font → system font `0x68963C4C`; no title font → `0x28963C70`
  (both AddRef'd). `cGZWin::Init`.
- **`Shutdown`** (`0x009C229F`): hide; style 0 → private buffer off; release the background and both fonts;
  `cGZWin::Shutdown`.
- **`GZPaint`** (`0x009C2BA6`): style 0 `RZDrawBltEdge(context, background, {0, 0, 24, 24}, draw area, 1)` (24-pixel
  corners); style 1 a plain blit (draw context slot 38); style 2 the fill colour. Style 0 with a caption: the title font's
  slot 52 draws it centred in `{0, (23 − line height)/2, W, 23}`.
- **Mouse**: down sets the mouse-down offset and, with window flag `0x100`, flag `0x400` (dragging), then capture
  (`0x009C2307`); up clears dragging and releases (`0x009C2348`); move (`0x009C2379`): without flag `0x100` the point
  goes to the parent in its coordinates as `SendMsg(parent, 13, x, y, flags)`; with it, while dragging, the window is
  offset by the movement.
- **`GZOnCommand(type, data)`** (`0x009C2D64`): `0x287259F6` with `data` a child (window slot 20): that child's ID 0 →
  `GZWinMsgPost(0xF, this, 0)` (end the dialog); otherwise the callback `(0x287259F6, data, ctx)`. Other types: the
  callback `(type, data, ctx)`. Returns false.

## 3. Quirks and cases

Quirks: the message box setters leak the counted fields they clear (§1.1); the builder gives controls their own
address as ID; `SetBackground(null)` crashes; `Init` without the control manager reports success without
initialising.

Cases:
1. `SetMessage("Save?", 3)` and `Init`: buttons Yes (green, default) and No (red, cancel); Enter ends the modal loop
   with `0x5301814A`, which is not one of its buttons (the OK ID).
2. A generic dialog of style 0 with caption "Options": the 9-slice frame and the caption centred in the 23-pixel top
   border.

## 4. Function inventory

`cGZWinMsgBox` (27): `0x00736559` (not code, §1.3), `0x009CD88C`, `0x009CD8B9`, `0x009CD92E`, `0x009CD9A8`, `0x009CD9E2`,
`0x009CDA10`, `0x009CDA45`, `0x009CDA7F`, `0x009CDAAB`, `0x009CDAB2`, `0x009CDAF1`, `0x009CDB21`, `0x009CDB2C`,
`0x009CDB36`, `0x009CDB5A` (`cGZWinDlg` constructor), `0x009CDBA5`, `0x009CDC2B`, `0x009CDC36`, `0x009CDC41`,
`0x009CDC4C`, `0x009CDC57`, `0x009CDC62`, `0x009CDC6D`, `0x009CDCA2`, `0x009CDFEF`, `0x009CE00B`.

`cGZWinGenDlg` (30): `0x0079BBE0`, `0x0079BC10`, `0x009C218F`, `0x009C21C5`, `0x009C229F`, `0x009C2307`, `0x009C2348`,
`0x009C2379`, `0x009C23FD`, `0x009C2422`, `0x009C24AA`, `0x009C251E`, `0x009C2594`, `0x009C261E`, `0x009C26B9`,
`0x009C2732`, `0x009C27AF`, `0x009C2815`, `0x009C2900`, `0x009C29C0`, `0x009C2A2D`, `0x009C2A97`, `0x009C2AE3`,
`0x009C2B2F`, `0x009C2B7B`, `0x009C2B9B`, `0x009C2BA6`, `0x009C2CEC`, `0x009C2D48`, `0x009C2D64`.
