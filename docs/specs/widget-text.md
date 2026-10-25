# `cGZWinText`: the text label (behaviour spec)

Scope: the `cGZWinText` owner as tools/coverage.py assigns it on `bb641878` (21 functions, 2,554 bytes; inventory
in §6). It is the toolkit's static text: the window caption drawn in a font, with margins, nine alignments, word
wrap, two colours, an optional fill and outline. The window-control manager creates it as a label
(widget-ctrlmgr.md §3.2); UI scripts create it as `GZWinText` (legacy-control.md §9.10).

Related: widget-toolkit.md (the `cGZWin` base: caption `+0x50`, flags §3, colour conversions slots 127/128);
widget-ctrlmgr.md (system fonts).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the draw call's arguments and the width rule of
`get_min_text_area` were checked on the instructions; the small `<shared>` slots of the interface were
disassembled so that the interface is complete. Vtables: `cIGZWinText` `0x00AE0118` (29 slots) and the window
`0x00ADFEB8`, compared with `cGZWin`'s. Written 2026-10-24 (re-gzwintext7).

Conventions: offsets from the window start; the interface (IID `0x212CDC1F`) is at `+0xD8`. "Slot n" is a
`cIGZWinText` slot, "window slot n" a `cIGZWin` slot. "Redraw" is window slot 92. `mx`, `my` are the margins.

## 1. Identity and fields *[compat]*

- **Factory**: class `0x592` (window-control manager, `0x114` bytes, interface returned).
- **`QueryInterface`** (window slot 0, `0x009C160F`): `0x212CDC1F` → `+0xD8` (null for null), AddRef'd; else
  `cGZWin`'s. Interface slot 0 (`0x007BE700`) is the thunk.
- **Window overrides**: 3 `DoMessage`, 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 74 `SetCaption` (`0x009C20B2`: base, then
  text position and redraw), 88 `GZPaint`, 106 `SetFillColor` (`0x009C16A9`: base, then redraw), 130/131 keys (the
  bitmap window's: `SendMsg(notification target, 5 or 6, vk, modifiers, 0)`), 148 destructor.
- **Constructor** (`0x009C19C8`): `cGZWin` constructor, the values below; clears window flag `0x8000`.
- **Destructor** (`0x009C1A6B`; deleting form `0x009C1BBD`): vtables, `Shutdown`, release the font, `~cGZWin`.

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xDC` | font (counted) | null |
| `+0xE0` | font ID (system font to load at `Init`) | 0 |
| `+0xE4`, `+0xE5` | margins `mx`, `my` (signed bytes) | 2, 2 |
| `+0xE6`, `+0xE7` | two bytes (slots 25/26), not read here | 0, 0 |
| `+0xE8..+0xF4` | text rectangle (window coordinates) | 0, 0, 8, 8 |
| `+0xF8` | text flags (§1.1) | 6 |
| `+0xFC` | outline colour | 0 |
| `+0x104` | alignment 0–8 | 0 |
| `+0x108`, `+0x10C` | colours 0 and 1, packed (`0xDEADBEEF` = unset) | unset |
| `+0x110` | horizontal alignment 0 left, 1 centre, 2 right | 0 |

### 1.1 Text flags (`+0xF8`)

| Bit | Meaning |
|---|---|
| `0x1` | pass messages to the parent (§3) |
| `0x2` | word wrap |
| `0x4` | filled (also makes the private buffer opaque, slot 7) |
| `0x8` | outline |
| `0x10000000` | the colours equal the font's own (drawn with the font's colours) |

## 2. The `cIGZWinText` slots *[compat]*

Slots 1–3 are the shared AddRef, Release and `AsIGZWin` thunks. "(s)" marks a `<shared>` function.

| Slot | Address | Behaviour |
|---|---|---|
| 4 `SetCaption(s)` | `0x007BE730` | thunk to window slot 74 |
| 5 `FitWindowToText(a, b)` | `0x009C1E18` | `get_min_text_area(&r, a, b)` (§4.2), then `SetSize(r.w + 2·mx, r.h + 2·my)` (window slot 53); true |
| 6 `IsTextFlag(mask)` | `0x009BC171` (s) | `(flags & mask) != 0` |
| 7 `SetTextFlag(mask, on)` | `0x009C1F89` | set/clear; mask 2: text position, redraw; mask 4: window `SetFlag(0x20000, !on)`, then `SetFlag(0x40000, GetFlag(0x20000))`, redraw |
| 8 `SetColorPacked(i, c)` | `0x009C1889` | `i` 0 or 1 (else false): store; no font → flag `0x10000000` off; else on exactly when both colours equal the font's two colours (font slots 12, 13, through window slot 128); redraw; true |
| 9 `SetColor(i, native)` | `0x009C1864` | slot 8 `(i, ConvertNativeToPackedRGB(native))` |
| 10 `SetColor(native)` | `0x009C16C3` (s) | slot 9 `(0, native)` |
| 11 `GetColorPacked(i, out)` | `0x009C1967` | false for `i` outside 0–1 or unset |
| 12 `GetColor(i, out)` | `0x009C1931` | as 11, converted with window slot 127 |
| 13 `GetColor(out)` | `0x009C16D1` (s) | slot 12 `(0, out)` |
| 14 `ResetColor(i)` | `0x009C198E` | slot 9 `(i, the font's colour i)` (font slot 12 or 13); `i` > 1 → false. The font is not checked for null |
| 15 `ResetColor()` | `0x009C16DF` (s) | slot 14 `(0)` |
| 16 `SetFilled(on)` / 17 `IsFilled` | `0x009C16E7` / `0x009C16F5` (s) | slot 7 / slot 6 with mask 4 |
| 18 `SetFont(font)` | `0x009C200E` | §4.1 |
| 19 `SetFontID(id)` | `0x009C16FD` (s) | store `+0xE0`; when initialised, `SetFont(WCM.SystemFont(id or 0x68963C4C))` if that is non-null; true |
| 20 `GetFontID(out)` | `0x009C1745` (s) | false when 0 |
| 21 `SetAlignment(a)` | `0x009C213E` (s) | `a` > 8 → false. If it changed: horizontal `+0x110 := a / 3` (0–2 → 0, 3–5 → 1, 6–8 → 2), store, text position, redraw. True |
| 22 `GetAlignment(out)` | `0x009C17D1` (s) | |
| 23 `SetMargins(x, y)` / 24 `GetMargins` | `0x009C2094` / `0x009C175B` (s) | bytes; setting recomputes the text position (no redraw) |
| 25 / 26 | `0x009C1772` / `0x009C1785` (s) | set / get the two bytes `+0xE6`, `+0xE7` |
| 27 `SetOutline(on, colour)` / 28 `GetOutline` | `0x009C179C` / `0x009C17B7` (s) | flag 8 and `+0xFC` |

So the alignment is `3 · horizontal + vertical`, with vertical 0 top, 1 middle, 2 bottom (§4.3).

## 3. Lifecycle and messages

- **`Init`** (`0x009C17DF`): when not yet initialised: `cGZWin::Init`; then, without a font and with the
  window-control manager: `SystemFont(font ID or 0x68963C4C)`, falling back to `0x68963C4C`, into slot 18. True.
- **`Shutdown`** (`0x009C183F`): initialised only: drop the font, `cGZWin::Shutdown`'s result; else true.
- **`DoMessage(msg)`** (`0x009C163C`): key messages (types 5, 6) go to `cGZWin::DoMessage`. Every other message:
  with text flag 1 and a parent, mouse messages (types 7–14) get the window's left and top added (window slots 43,
  44, in place) and the message goes to the parent's `DoMessage`, whose result is returned; otherwise **false**,
  without the base dispatch.

## 4. Layout and painting *[compat]*

### 4.1 `SetFont(font)` (slot 18)

If different: hold it; if non-null, give each unset colour the font's colour (slot 9 with font slot 12 / 13); re-store
both colours through slot 8 (re-evaluating flag `0x10000000`); text position; redraw. True.

### 4.2 `get_min_text_area(&out, a, b)` (`0x009C1BD9`)

`out` is `{0, 0, w, h}`; `text` is the caption, `lh` the font's line height (slot 37).
- No font: `{0, 0, 1, 1}`.
- No wrap (flag 2 clear): the font's slot 47 `(text, length, &out)` measures it.
- Wrap, `a` = 0 (fit the current width): `w = max(W − 2·mx, 1)`; `h = lines(w) · lh`, where `lines(w)` is font
  slot 43 `(text, length, w)`.
- Wrap, `a` ≠ 0, `b` = 0: `maxw = 60 · lh`; `h = lines(maxw) · lh`; `w = maxw`, or for a single line
  `min(maxw, text width)` (font slot 48).
- Wrap, `a` ≠ 0, `b` ≠ 0: `w` = the widest of the caption's lines split at `'\n'` (font slot 48 per line; at least 1);
  `h = lines(w) · lh`.

### 4.3 `calc_text_pos` (`0x009C1E6D`)

Empty caption → nothing. By the alignment:
- top (0, 3, 6): the window area (window slot 47) inset by `mx`, `my` on each side, then moved to `(mx, my)` (window
  coordinates);
- middle (1, 4, 7): `th` = the height of `get_min_text_area(·, 0, 0)`; `y = (H − th) / 2`;
- bottom (2, 5, 8): `y = H − my − th`;
- for middle and bottom the rectangle is `{mx, y, W − mx, y + th}`.

**`SetArea`** (window slot 55, `0x009C20D3`): `cGZWin::SetArea`, then the text position when the size changed.

### 4.4 `GZPaint` (window slot 88, `0x009C1A9A`)

1. Filled: fill the draw area with the fill colour (window slot 126; draw context slots 21, 35).
2. With a font and a non-empty caption: font slot 52 `(buffer to draw to, the text rectangle offset by the draw
   area's origin, text, length, horizontal alignment, c0, c1, 0, 0)` with `c0`, `c1` = colours 0 and 1 as native
   (window slot 127), or both `0x8000000` (the font's own) when flag `0x10000000` is set.
3. Outline: `RZDrawRectangleOutline(context, draw area, outline colour, 1)`. True.

## 5. Quirks and cases

Quirks:
1. `DoMessage` drops every non-key message unless flag 1 passes it to the parent.
2. `ResetColor` dereferences the font without a check.
3. `SetMargins` does not redraw.

Cases:
1. A new label with caption `"Hello"`, font line height 12, area 100 × 40, alignment 4 (centre, middle): text
   rectangle `{2, y, 98, y + th}` with `y = (40 − th) / 2`, drawn centred.
2. `SetAlignment(9)` returns false and changes nothing.
3. `FitWindowToText(1, 1)` with caption `"ab\ncdef"`: width = the width of `"cdef"`, height = 2 lines × line height,
   plus the margins.

## 6. Function inventory (owner `cGZWinText`, 21)

`0x007BE700` QI thunk, `0x007BE730` slot 4, `0x009C163C` `DoMessage`, `0x009C17DF` `Init`, `0x009C183F` `Shutdown`,
`0x009C1864` slot 9, `0x009C1889` slot 8, `0x009C1931` slot 12, `0x009C1967` slot 11, `0x009C198E` slot 14,
`0x009C19C8` constructor, `0x009C1A6B` destructor, `0x009C1A9A` `GZPaint`, `0x009C1BBD` deleting destructor,
`0x009C1BD9` `get_min_text_area`, `0x009C1E18` slot 5, `0x009C1E6D` `calc_text_pos`, `0x009C1F89` slot 7,
`0x009C200E` slot 18, `0x009C2094` slot 23, `0x009C20D3` `SetArea`.

Not covered: the font slots (12, 13, 37, 43, 47, 48, 52), cited by number.

## 7. The game's subclass `cSC4WinText` *[compat]*

Class `0xAA7CECFD` (scripted-window-director.md §2): a `cGZWinText` with its own vtables and two overrides, the
deleting destructor (`0x007BE780`: `~cGZWinText`, freed when bit 0 of the argument is set) and **`GZPaint`**
(`0x007BE7A0`), which adds an ellipsis to text that does not fit. Owner `cSC4WinText` (2 functions, 1,266 bytes;
added 2026-10-24, re-gzsc4text7).

`GZPaint`, with a buffer to draw to (`+0x68`; else nothing, true):

1. **Filled** (flag 4): the buffer is locked (slot 6, `0x8001`), filled (slot 20) with the window's native fill colour
   for the draw area (window slot 103 `(&+0x24, 0)`), unlocked (slot 7). (The base fills through the draw context.)
2. **Text**, with a font and a non-empty caption: `E` := the window-control manager's (service `0xC2C2EB0F`) slot
   35 string `0x5377BE31` (the ellipsis). The colours are as in §4.4.
   - **No wrap** (flag 2 clear): a copy of the caption is cut to the text rectangle with `E` by font slot 50
     `(&copy, E, &+0xE8, 0)`; the copy is drawn (font slot 52) in the text rectangle offset by the draw area's origin.
   - **Wrap** (flag 2): `lh` = the line height (font slot 35); at most `n = H / lh` lines (H the window height,
     signed division); line width `w = W − 2·mx`. Lines are cut from the caption with font slot 41
     `(start, remaining, w, 0, 2)` (the characters that fit, breaking at words); the cutting stops when the text is
     used up or n lines are taken. It is **truncated** when a cut returns 0 or when the last allowed line does not take
     the rest. The block is placed by the vertical alignment (alignment mod 3: 0 top, 1 middle with
     `(H − 2·my − lines·lh) / 2`, 2 bottom with the whole remainder) from `(drawArea.L + mx, drawArea.T + my)`, one line
     per `lh`, right edge `drawArea.R − mx`. Each line is drawn with font slot 52 and the horizontal alignment; when
     truncated, the last line instead becomes the characters that fit in `width − width(E)` (font slots 48 and 42)
     followed by `E`, and nothing at all is drawn for it when `E` alone is wider than the line.
3. **Outline** (flag 8): the buffer locked with `0x8008`, `DrawRectangleOutline(buffer, draw area, outline colour,
   1)`, unlocked. True.

Quirks: a window lower than one line takes no lines and draws nothing; a cut that returns 0 (a word wider than the
line) ends the text there and puts the ellipsis on the line before it.
