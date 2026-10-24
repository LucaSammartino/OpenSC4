# `cGZWinToolTip`: the tooltip window (behaviour spec)

Scope: the `cGZWinToolTip` owner as tools/coverage.py assigns it on `bb641878` (13 functions, 2,269 bytes; inventory
in §5). It is the small overlay window that shows a tip text next to a tool: it renders the text once into an
off-screen 256 × 64 buffer, places itself around the tool's rectangle and blits that buffer when painted. Buttons
request it through the tooltip manager (widget-button.md §5.3).

Related: widget-toolkit.md (the `cGZWin` base; flag `0x8000000` = overlay, §3); widget-button.md §5.3; drawing
helpers `DrawRectangleOutline`; the font slots cited by number.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; `SetFont`'s comparison, the colour mapper and
the line separator (`0x00A916BC` = `"\n"`) were checked on the bytes. Vtables: `cIGZWinToolTip` `0x00AE4380`
(6 slots) and the window `0x00AE4398` (153 slots), compared with `cGZWin`'s. Written 2026-10-24 (re-gzwintip7).

Conventions: offsets from the window start; the interface (IID `0x22C010CF`) is at `+0xD8`. "Window slot n" is a
`cIGZWin` slot.

## 1. Identity and fields *[compat]*

- **`QueryInterface`** (`0x009D9624`): `0x22C010CF` → `+0xD8`; `0xE98B2F57` → the object itself; both AddRef'd; else
  `cGZWin`'s. Interface slot 0 (`0x009D9A38`) is the thunk.
- **Window overrides**: 3 `DoMessage`, 4 `Init`, 5 `Shutdown`, 62, 121 and 122 (a shared stub returning 0: the tip
  is never hit by the mouse), 88 `GZPaint`, 148 destructor (`0x009D9A18`; deleting form `0x009D9CD3`), and its own
  151 `update_offscreen_buffer`, 152 `MapColor`.

| Offset | Meaning |
|---|---|
| `+0xDC` | font (counted) |
| `+0xE0`, `+0xE1` | margins `mx`, `my` (signed bytes) |
| `+0xE4` | the off-screen buffer (`cIGZBuffer`, counted) |
| `+0xE8..+0xF4` | the tip rectangle in the buffer (`{0, 0, w, h}`) |
| `+0xF8..+0x104` | the anchor: the tool's rectangle |
| `+0x108`, `+0x10C`, `+0x110` | text, background and outline colours (native) |
| `+0x114` | placement flags (§3) |

## 2. Interface and messages

- **Interface slot 4 `SetFont(font, mx, my)`** (`0x009D97B0`): a different font is held (AddRef new, release old).
  The margins are stored when `mx` differs from the stored `mx`, or `my` differs from the stored **`mx`**
  (*quirk*: the stored `my` is never compared). Nothing is redrawn (a `get_init_flag` call with no effect). True.
- **Interface slot 5 `SetTipInfo(info*)`** (`0x009DA05E`), `info` = `{+4 flags, +8 text (string or null),
  +0x0C..+0x18 anchor rectangle, +0x1C/+0x20/+0x24 text, background and outline colours (packed)}`:
  1. No text → clear the caption. Otherwise the caption := the text, shortened by `CopyTipInfo(caption, 256, font)`.
  2. Anchor := the rectangle; placement := the flags, or the flags `| 0x8000000` when their high 16 bits are 0.
  3. With the buffer: the three colours through `MapColor`. True.
- **`MapColor(out*, packed)`** (window slot 152, `0x009D980E`): `*out :=` the buffer's slot 30 `MapRGB(packed >> 16,
  packed >> 8, packed)` (bytes). The buffer is not checked.
- **`CopyTipInfo(s, maxW, font)`** (`0x009D9F56`): `n` = the characters of `s` that fit in `maxW` (font slot 42).
  `n` = 0 or all of `s` → false, unchanged. Otherwise the ellipsis is the window-control manager's string
  `0x5377BE31`, or `"..."`; with `e` its width (font slot 48): `e < maxW` → keep what fits in `maxW − e`, erase from
  **one character before** that, `RTrim`, append the ellipsis; else clear `s`. True.
- **`DoMessage(msg)`** (`0x009D96C9`):

  | Type | Effect |
  |---|---|
  | `0xE2C7C838` | `update_offscreen_buffer`; on success `SetFlag(0x800, 1)`, `PullToFront` (window slot 23), `SetFlag(0x800, 0)`, show (69) |
  | `0xE2C7C839` | hide (70) |
  | `0xE2C7C83A` / `…3B` / `…3C` | with the buffer: `MapColor` of `msg.d1` into the text / background / outline colour, then `update_offscreen_buffer` |
  | other | **false**, without the base dispatch |

  All listed types return true.

## 3. Layout and painting *[compat]*

**`update_offscreen_buffer`** (window slot 151, `0x009DA106`). Empty caption, no buffer or no font → false.
1. Measure: split the caption at `"\n"`; for each line, font slot 47 gives its rectangle; `w` = the largest right
   edge, `h` = the sum of the line heights. Add `2·mx`, `2·my`; cap at the buffer's width and height.
2. Draw into the buffer: lock `0x8001`, fill the tip rectangle with the background colour (buffer slot 20), unlock;
   font slot 52 `(buffer, {mx, my, w − mx, h − my}, caption, length, 1 (centred), text colour, 0x8000000, 0, 0)`;
   lock `0x8004`, `DrawRectangleOutline(buffer, tip rectangle, outline colour, 1)`, unlock.
3. Place, from the anchor `{l, t, r, b}` and the tip size `w × h` (first matching flag):

   | Flag | x | y |
   |---|---|---|
   | `0x10000` | `l` | `t` |
   | `0x20000` | `l − w − 2` | `t` |
   | `0x40000` | `l − w − 2` | centred on the anchor |
   | `0x80000` | `l − w − 2` | `b − h` |
   | `0x100000` | `r + 2` | `t` |
   | `0x200000` | `r + 2` | centred |
   | `0x400000` | `r + 2` | `b − h` |
   | `0x800000` | `l` | `t − h − 2` |
   | `0x1000000` | centred | `t − h − 2` |
   | `0x2000000` | `r − w` | `t − h − 2` |
   | `0x4000000` | `l` | `b + 2` |
   | `0x8000000` (default) | centred | `b + 2` |
   | `0x10000000` | `r − w` | `b + 2` |
   | none | centred | `b + 2` |

   "Centred" is `l + (r − l − w) / 2` (or the same in y). Then keep it on the main window (window slot 10
   `GetMainWindow`): `x ≥ 2`, and `x + w ≤ R − 2` with `R` its right edge (its slot 45); the same in y with its
   bottom edge (slot 46).
4. `SetArea(x, y, x + w, y + h)` (window slot 55). True.

**`GZPaint`** (`0x009D96A6`): draw context slot 38 `(buffer, &tip rectangle, &draw area)`; true.

**`Init`** (`0x009D9CEF`), once: `SetFlag(0x8000000, 1)`, `cGZWin::Init`; a new buffer from the graphics system
(slot 3), initialised to 256 × 64 (buffer slot 3, with the graphics system's format); hide. True.
**`Shutdown`** (`0x009D9664`): the buffer's slot 4 and release; release the font; `cGZWin::Shutdown`.

## 4. Quirks and cases

Quirks: `SetFont` compares both margins with the stored `mx` (§2); `CopyTipInfo` erases one character more than
fits, and leaves a text unchanged when not even one character fits; non-listed messages return false.

Cases:
1. Anchor `{100, 50, 140, 70}`, tip 60 × 20, default flags: placed at `(90, 72)`.
2. Same with `0x100000`: `(142, 50)`.
3. A main window 640 wide and the anchor `{620, 50, 640, 70}`: the default `x = 600` is clamped to
   `640 − 2 − 60 = 578`.

## 5. Function inventory (owner `cGZWinToolTip`, 13)

`0x009D9624` QI, `0x009D9664` `Shutdown`, `0x009D96A6` `GZPaint`, `0x009D96C9` `DoMessage`, `0x009D97B0` slot 4,
`0x009D980E` `MapColor`, `0x009D9A18` destructor, `0x009D9A38` QI thunk, `0x009D9CD3` deleting destructor,
`0x009D9CEF` `Init`, `0x009D9F56` `CopyTipInfo`, `0x009DA05E` slot 5, `0x009DA106` `update_offscreen_buffer`.

Not covered: the font and buffer slots, cited by number.
