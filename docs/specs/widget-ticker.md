# `cGZWinTextTicker`: the scrolling text ticker (behaviour spec)

Scope: the `cGZWinTextTicker` owner as tools/coverage.py assigns it on `bb641878` (16 functions, 2,054 bytes), its
constructor (attributed to `cGZWinCOMDirector`, `0x009D6530`) and its `<shared>` interface slots. It is the toolkit's
news-ticker strip: queued messages scroll in from the right edge one character at a time, each with its own font,
colours and click action. UI scripts create it as `GZWinTextTicker` (class `0xE32F0B31`, legacy-control.md §9.19).
The SC4 news ticker window is a different class (news-ticker.md).

Related: widget-toolkit.md (the `cGZWin` base); timers.md (`cRZTimer`); text-edit-encoding.md (`GetCharLength`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the `<shared>` interface slots were
disassembled. Vtable `cIGZWinTextTicker` `0x00AE3A44` (15 slots). Written 2026-10-24 (re-gzticker7).

Conventions: the interface is at `+0`, the window at `+4`; offsets are from the object start. "Slot n" is a
`cIGZWinTextTicker` slot.

## 1. Fields *[compat]*

Constructor `0x009D6530` (`0x138` bytes): the window, a `cRZTimer` at `+0xE0`, zeros, two message lists, speed and
frequency, and an off-screen buffer from the graphics system.

| Offset | Meaning | Initial |
|---|---|---|
| `+0xE0` | frame timer (`cRZTimer`) | |
| `+0xF8` | pixels scrolled since the last character was drawn | 0 |
| `+0xFC` | the off-screen strip (`cIGZBuffer`) | created |
| `+0x104` | code page | `Init`: the framework's |
| `+0x108` | the message being drawn (its list node) | |
| `+0x10C` | next character of that message | 0 |
| `+0x110` | message ID counter | 0 |
| `+0x114` | default font (counted) | null |
| `+0x118`, `+0x11C`, `+0x120` | the current message's font, text colour, background colour | |
| `+0x124` | messages on screen (list) | empty |
| `+0x128` | queued messages (list) | empty |
| `+0x12C` | speed (pixels per frame) | **2** |
| `+0x130` | frequency (ms per frame) | **50** |

**Message record** (`0x4C` bytes, built by `0x009D60CD`): `+0x04` ID, `+0x08` flags (bit 1 underline), `+0x0C` text,
`+0x10` font, `+0x14`/`+0x1C` start and end x on the strip, `+0x24` text colour, `+0x28` background colour, `+0x34` a
message to send when clicked (16 bytes, type 0 = none), `+0x44` a callback `(id)` called when clicked.

## 2. The `cIGZWinTextTicker` slots

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x009D6066` | `QueryInterface`: `0x032F0B3C` → the object (AddRef'd) when `out` is non-null, returning that test; else `cGZWin`'s |
| 3, 4 `AddMessage(…)` | `0x009D6AC8`, `0x009D6A57` (shared) | without a default font → 0; else a new record with the next ID and the default font, appended to the queue; returns the ID |
| 5 `RemoveMessage(id)` | `0x009D63E5` | delete the queued record with that ID; false when none |
| 6 `Clear` | `0x009D6314` | delete every queued and on-screen record; fill the strip with the fill colour (window slot 103). True |
| 7 `SetFont(font)` | `0x009D60A0` (shared) | the default font |
| 8 / 9 / 10 / 11 | `0x009D65E6`, `0x009D65F3`, `0x00993946`, `0x009CFBBA` (shared) | set speed, set frequency, get speed, get frequency |
| 12 | `0x009CFA36` (shared) | the window |
| 13 `NextMessage` | `0x009D63B4` | take the first queued record and give it to slot 14 |
| 14 | `0x009D6B37` (shared) | put a record on screen |

## 3. Behaviour

- **`Init`** (window slot 4, `0x009D619B`), once: scroll count 0; restart the timer; height := the default font's
  line height (window slot 52); the strip is initialised to the window's size (buffer slot 3); on success its key
  colour is the control manager's `0x08963F6C` (buffer slot 23) and it is filled with the fill colour; the current node
  := the end of the on-screen list; cursor 18 from the cursor manager; code page. Returns `cGZWin::Init`'s result, or
  false when the strip cannot be made.
- **`Shutdown`** (`0x009D62B1`): delete the on-screen records, release the default font and the strip,
  `cGZWin::Shutdown`.
- **`GZPaint`** (`0x009D69FC`): when more than the frequency (ms) has passed, `DoDraw` and restart the timer; blit the
  strip to the draw area (draw context slot 38); invalidate (window slot 91) so it keeps moving. True.
- **`DoDraw`** (`0x009D673B`), with the strip `W × H` and `v` the speed:
  1. Lock `0x8010`: move the strip left by `v` (buffer slot 29); lock `0x8001`: fill the right `v` columns with the fill
     colour.
  2. Scroll count `+= v`; `shiftSpans` (`0x009D66A5`): every on-screen span moves left by `v`; spans whose end goes below
     0 are deleted (advancing first when it is the current one).
  3. No message on screen → `advanceMessage`. The current message finished (or none) → `advanceMessage`.
  4. Otherwise: if its background colour differs from the strip's key colour (buffer slot 26), fill the newly
     exposed columns with it; with the underline flag, a line along the bottom row in the text colour; with a positive
     scroll count, the next character (`GetCharLength`) is drawn at `x = W − count` (font slots 55, 51, 56) and, when its
     width (font slot 40) fits in the count, the character pointer advances and the count drops by that width.
- **`advanceMessage`** (`0x009D645B`): slot 13 (move the next queued message on screen). With messages on screen,
  the last one becomes current: its font and colours are taken, its first character, and its span is set to start
  20 pixels past the right edge and end after its text width; the scroll count starts at −20 (a 20-pixel gap).
  Without messages: no current character.
- **Left down** (`0x009D6664`): the on-screen message whose span holds `x` (`FindClickedMessage`, `0x009D6429`): its
  callback with its ID, and its message (when it has one) sent through the message server (slot 4). True.

## 4. Function inventory

Owner `cGZWinTextTicker` (16): `0x009D6066` QI, `0x009D619B` `Init`, `0x009D62B1` `Shutdown`, `0x009D6314` slot 6,
`0x009D63B4` slot 13, `0x009D63E5` slot 5, `0x009D6429` `FindClickedMessage`, `0x009D645B` `advanceMessage`,
`0x009D6600` QI thunk, `0x009D6618` deleting-destructor thunk, `0x009D6620` destructor, `0x009D6664` left down,
`0x009D66A5` `shiftSpans`, `0x009D671F` deleting destructor, `0x009D673B` `DoDraw`, `0x009D69FC` `GZPaint`.
Attributed to `cGZWinCOMDirector`: `0x009D6530` constructor.

Not covered: the font and buffer slots, cited by number; the record constructor `0x009D60CD` (`<shared>`).
