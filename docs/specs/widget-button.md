# `cGZWinBtn`: the toolkit button (behaviour spec)

Scope: the `cGZWinBtn` owner as tools/coverage.py assigns it on `5c0ff782` (87 functions, 7,818 bytes; inventory in
§8). It is the framework's push button, toggle, image button and check/radio button. It is used by the game's
windows and by UI scripts (class factory `CreateNewObject`).

Related:
- widget-toolkit.md: the `cGZWin` base, its slots, flags, painting (§14), `SendMsg`/`PostMsg` (§7) and hit tests
  (§5.2);
- ui-windows.md §2 (the `cIGZWin` slots);
- drawing-fonts.md (fonts, `RZDraw*`);
- callout-box.md and the game windows that use buttons.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled or read in the decompile, with ambiguous stack arguments
checked on the instructions. The vtables `0x00ADDD50` (the `cIGZWinBtn` interface) and `0x00ADDAF0` (the window)
were read. Written 2026-10-23 (re-gzwinbtn7).

Conventions:
- Offsets are from the object start: the interface pointer is at `+0`, the `cGZWin` part at `+4`. So a `cGZWin`
  field at `+n` (widget-toolkit.md §2) is at `+4 + n` here, for example the flags `+0xCC`, the draw area `+0x28`,
  the draw context `+0x70` and the notification target `+0x50`.
- "Window slot n" is a `cIGZWin` slot of the `+4` part; "slot n" alone is a `cIGZWinBtn` slot.
- `W`, `H` are the window's width and height (window slots 41, 42). "Relayout" is `on_layout(W, H)` (§4).
- "Notify(code, d)" is window slot 146 `PostMsg(notification target, 3, code, the button's ID, d)` (the ID from
  slot 48).

## 1. Identity and fields *[compat]*

- **Factory** `CreateNewObject` (`0x009B1E0A`): 0x1B4 bytes, constructor `0x009B1C27`; it returns the window part
  (`+4`), or null.
- **Interfaces**: `cIGZWinBtn` (IID `0x8810`, 53 slots, §2) at `+0`; the window (vtable `0x00ADDAF0`) at `+4`.
- **`QueryInterface`** (`0x009AFFD3`): `0x8810` gives the object start, AddRef'd; any other IID goes to the
  `cGZWin` one.
- **Destructors**: the body `0x009B1DA8`:
  1. restores both vtables;
  2. `Shutdown`;
  3. destroys the flash timer and the tip text;
  4. releases the image and the font;
  5. frees the caption copy;
  6. runs the `cGZWin` destructor.

  The deleting destructor is `0x009B1E2E`; the window part's thunk is `0x009B1D98`.

| Offset | Content | Constructor |
|---|---|---|
| `+0xDC` | **button flags** (§1.1) | `0x6A0` |
| `+0xE0` | a string (freed by the destructor; not read here) | empty |
| `+0xF4`, `+0xF6` | text position (x, y), shorts, inside the draw area | 0 |
| `+0xF8`, `+0xFA` | text size (width, height), shorts | 0 |
| `+0xFC` | text alignment: 0 centre, 1 left, 2 right | 0 |
| `+0x100` | the matching draw-text alignment: 1, 0, 2 | — |
| `+0x102`–`+0x105` | margins left, top, right, bottom (bytes) | 0 |
| `+0x108` | the font (counted) | null |
| `+0x10C` | the font ID | 0 |
| `+0x110`–`+0x124` | three colour pairs: normal, hover, disabled; `0x8000000` = unset | `0x8000000` ×6 |
| `+0x128`–`+0x130` | three sound IDs: press, release, toggle | 0 |
| `+0x134` | style (§1.2) | 0 |
| `+0x138` | frame mode: 0 one image, 1 four frames, 2 eight frames | 0 |
| `+0x13C`–`+0x148` | one frame's rectangle in the image | 0 |
| `+0x14C` | the current frame's left edge | 0 |
| `+0x150`, `+0x154` | where the image is drawn, relative to the draw area | 0 |
| `+0x158` | the image (counted) | null |
| `+0x15C` | the tip text (a string) | empty |
| `+0x170` | the shown tip's ID | 0 |
| `+0x174` | tip flags | 0 |
| `+0x178`, `+0x17A` | the tip point (shorts) | 0 |
| `+0x17C`–`+0x184` | three tip colours | 0, `0xFFFFE0`, 0 |
| `+0x188` | the flash count | 0 |
| `+0x18C` | the flash timer (`cRZCallbackTimer`) | stopped |

The constructor also clears window flag `0x800` and sets the fill colour `0xFFC0C0C0`.

### 1.1 Button flags (`+0xDC`)

| Bit | Meaning | Default |
|---|---|---|
| `0x1` | on (checked) | 0 |
| `0x2` | toggle on mouse **down** | 0 |
| `0x4` | has a caption | 0 |
| `0x10` | do not paint the flat face | 0 |
| `0x20` | size to fit at `Init`; no edge stretching | 1 |
| `0x40` | wrap the caption | 0 |
| `0x80` | shift the caption 1 pixel while pressed | 1 |
| `0x100` | has a tip | 0 |
| `0x200`, `0x400` | tip options (bits 1 and 2 of the tip request) | 1, 1 |
| `0x800` | the right button toggles too | 0 |
| `0x1000` | pressed | 0 |
| `0x2000` | hovered | 0 |
| `0x4000` | Space held | 0 |
| `0x1000000` | (repaint only) | 0 |

### 1.2 Styles (`+0x134`)

| Style | Frame mode | Frames | Look |
|---|---|---|---|
| 0 | — | — | not set yet: `Init` picks style 2 (§3) |
| 1 | 0 | 1 | one image, no state frames |
| 2 | 1 | 4 | push button: disabled, normal, pressed, hover |
| 3 | 1 | 4 | as 2, and "on" shows the pressed frame |
| 4 | 2 | 8 | check/radio: an image beside the caption |

## 2. The `cIGZWinBtn` slots *[compat]*

| Slot | Address | Behaviour |
|---|---|---|
| 0–2 | `0x009AFFD3`, `0x009D6090`, `0x009D6098` | QI, AddRef, Release |
| 3 | `0x009CFA36` | the window part (`this + 4`, null-safe) |
| 4 | `0x009A62BC` | `(flags & mask) ≠ 0` |
| 5 | `0x009B14DF` | `SetBtnFlag(mask, on)` (§2.1) |
| 6 / 7 / 9 | `0x009B1BC2` / `0x009B1BCC` / `0x009B1BD6` | flag 1 on / off / to the argument |
| 8, 10 | `0x009B048D` | flag 1 (the same body twice) |
| 11 | `0x009B002D` | the style |
| 12 | `0x009B1179` | `SetStyle(style, image, rect*)` (§2.2) |
| 13 | `0x009B0497` | change the style, keeping the image and its whole rectangle (frame width × the old frame count) |
| 14 / 15 / 16 | `0x009B0559` / `0x009B0568` / `0x009B005E` | frame width / height / rectangle (copied out, true) |
| 17 | `0x009B0338` | `SetImage(image)`: `SetStyle(current style, image, null)`, invalidate |
| 18 | `0x009B10BF` | the image (out), true |
| 19 | `0x009B12EF` | `SetFont(font)`: when it changes, hold it; unset colours of each pair take the font's two default colours (font slots 12, 13); relayout |
| 20 / 21 | `0x009B0507` / `0x009B0045` | font ID set / get. Setting stores `+0x10C`; when the window is initialised or a font is held, `SetFont(WCM slot 5 (ID, or 0x68963C4C when 0))`. Get: false for 0 |
| 22 / 23 | `0x009B0174` / `0x009B014F` | colour *i* from packed RGB (window slot 127) / native; a change invalidates (window slot 92) |
| 24 / 25 | `0x009B0FF1` / `0x009B01AB` | colour *i* as packed (window slot 128) / native; false when unset |
| 26 | `0x009B01CC` | unset colour *i* |
| 27 / 28 | `0x009B1291` / `0x009B0034` | text alignment set (also `+0x100`; relayout) / get |
| 29 / 30 | `0x009B0F9E` / `0x009B0075` | margins (l, t, r, b) / (x, y) = (x, y, x, y); relayout |
| 31 / 32 | `0x009B0116` / `0x009B008D` | margins out (four / left and top) |
| 33 | `0x009B0776` | size to fit: the preferred size (slot 34) becomes the window size (window slot 53) unless both are 0 |
| 34 | `0x009B07B7` | preferred size (§4.3) |
| 35 | `0x009B01DF` | `SetTipText(s)`: empty → flag `0x100` off and slot 37; else copy it, flag `0x100` on |
| 36 / 37 | `0x009B022D` / `0x009B1C0C` | tip text out / clear it (hide the tip first) |
| 38 / 39 | `0x009B0243` / `0x009B0252` | tip flags set / get |
| 40 / 41 | `0x009B0263` / `0x009B0280` | tip point set / get |
| 42 / 43 | `0x009B029F` / `0x009B02C3` | tip colour *i* := `r << 16 \| g << 8 \| b` / out |
| 44 | `0x009B1058` | `Flash(count, period)` (§5.3) |
| 45 / 46 | `0x009B02EF` / `0x009B0303` | sound ID *i* set / get |
| 47–50 | `0x009AFFF5`, `0x009AFFFD`, `0x009B14E4`, `0x009B00AC` | `SetID`, `GetID`, `SetCaption`, `GetCaption` |
| 51 | `0x009B00B4` | `Notify(0x287259F6, flag 1)`: "clicked" |
| 52 | `0x009B00E9` | `Notify(0x287259F0, 0)`: "right-clicked" |

### 2.1 `SetBtnFlag(mask, on)` (`set_btn_flag`, `0x009B13DD`)

1. Nothing (true) when the flags do not change.
2. Store the new flags, then by mask:
   - `0x1`: `Notify(0x287259F7, on)`, relayout;
   - `0x1000`: `Notify(0x287259F8, on)`, relayout;
   - `0x2000`: `Notify(0x287259F9, on)`, relayout;
   - `0x4`, `0x20`, `0x40`: relayout;
   - `0x1000000`: invalidate.
3. Return true.

Any other mask is stored silently. A combined mask matches none of the rows.

### 2.2 `SetStyle(style, image, rect)` (`0x009B1179`)

1. Style 0 returns false. Otherwise store the style.
2. **Style 1**: frame mode 0, return true. The image and rectangle are ignored.
3. **Styles 2 and 3** (frame mode 1, 4 frames) and **style 4** (frame mode 2, 8 frames; also clears flag `0x80`):
   1. Hold the image (when it differs).
   2. The frame rectangle:
      - from `rect`: its left and top, width = rect width / frames, height = rect height;
      - without one, from the image: (0, 0), width = image width (slot 9) / frames, height = image height
        (slot 10).
   3. Relayout; return its result.
4. No image and no rectangle: frame mode 0, return false. Any other style also returns false.

## 3. Lifecycle

**`Init`** (window slot 4, `0x009B038A`), once:
1. The base `Init`.
2. With a caption, no font and a window colour manager (`RZ::WCM()`): `SetFont(WCM slot 5 (font ID or
   0x68963C4C))`.
3. Style still 0: `SetStyle(2, image, image slot 12)` with the WCM's default button image (its slot 7,
   `0xE8963EF1`); without it `SetStyle(2, null, null)`.
4. Flag `0x20`: size to fit (slot 33).
5. Each of the three sounds still 0 takes the WCM's default (its slot 29 with the index).
6. Return true.

**`Shutdown`** (window slot 5, `0x009B10D0`), when initialised:
1. Stop the flash timer and hide the tip.
2. Window slot 70 (`Hide`).
3. Release the image and the font.
4. The base `Shutdown`.

**Other window overrides:**
- `SetArea(l, t, r, b)` (slot 55, `0x009B1397`): the width and height are clamped to ≥ 0, the base `SetArea`, then
  `on_layout(w, h)`.
- `SetFlag(flag, on)` (slot 68, `0x009B112D`): the base `SetFlag`. Flag 2 (enabled) also relayouts; flags 1 and 2
  invalidate. Returns the base result.
- `SetCaption(s)` (slot 74 and interface slot 49, `0x009B14E4`): the base, then flag 4 := not empty (with
  `update_text_size`), relayout.
- `SetKeyEquivalent` (slot 79, `0x009B0005`): the base, then the caption is set again (slots 50, 49).

## 4. Layout and painting *[compat]*

### 4.1 `on_layout(w, h)` (`0x009B0C08`)

1. Invalidate (window slot 92); `update_text_size` (§4.2).
2. `fw`, `fh` := the frame size; `phase` := the flash count's bit 0. "Enabled" is window flag 2.
3. **The frame index** (`+0x14C` = frame left + index × `fw`):

   | Frame mode | Rule |
   |---|---|
   | 0 | no frame: go to step 4 |
   | 1, style 2 | disabled 0; enabled: `phase` → 3, else pressed → 2, else hovered → 3, else 1 |
   | 1, style 3 | disabled 0; enabled: `phase` → 3, else pressed or on → 2, else hovered → 3, else 1 |
   | 2 (style 4) | disabled: on 6, off 7; enabled: `phase` or hovered → on 2, off 3; pressed → on 4, off 5; else on 0, off 1 |

   In frame mode 1 the image is drawn at the draw-area origin (`+0x150`, `+0x154` := 0).

   In frame mode 2 the image sits at the left (`+0x150` := 0) for alignments 0 and 2, or at the right
   (`+0x150` := `w − fw`) for 1, centred vertically (`+0x154` := (`h − fh`) / 2). The caption box then starts after
   it (left margin + `fw`) for alignments 0 and 2.
4. **The caption box** (only with a caption):
   - frame mode 0: (left margin, top margin, W − right margin, H − bottom margin);
   - frame mode 1: (left margin, top margin, `w` − right margin, `h` − bottom margin);
   - frame mode 2: (start, top margin, start + text width, top margin + text height).
5. **`update_text_pos(box)`** (`0x009B0B87`):
   - x by the alignment: centre `box.l + (box.r − box.l − textW) / 2`, left `box.l`, right `box.r − textW`;
   - y is always centred.

   Another alignment value sets the position to (0, 0).

Frame modes 3 and 4 have rows in the function but `SetStyle` never sets them *[inferred: unreachable]*.

### 4.2 `update_text_size` (`0x009B09B7`)

1. Without a font, take the font ID's font, else `0x68963C54`, else `0x68963C4C` (WCM slot 5).
2. Without a font or a caption, the size is (0, 0).
3. Not wrapping: the size of the caption's bounding box (font slot 47).
4. Wrapping (flag `0x40`): the available width is W − left − right margin (style 4: minus `fw` too). Then:
   - lines := font slot 43 for that width;
   - height := lines × the line height (font slot 37);
   - width := one line's width (font slot 47) when there is one line, else font slot 44.

### 4.3 Preferred size (slot 34, `0x009B07B7`)

The output is (w, h, maxW, maxH); 0x7FFFFFFF means unlimited. `tw`, `th` are the text size plus the margins.

| Style | w, h | maxW, maxH |
|---|---|---|
| 1 | `tw`, `th` | unlimited |
| 2, 3 | no caption: `fw`, `fh`; caption: `max(tw, fw)` (W when wrapping), `max(th, fh)` | unlimited |
| 4 | caption: `fw + tw`, `max(th, fh)`; no caption: `fw`, `fh` | caption: unlimited, h when h > 0; no caption: w, h when > 0 |
| 0, others | an image: `fw`, `fh` (at least the bare text size with a caption); no image: the bare text size with a caption, else W, H | w and h when > 0, else unlimited |

### 4.4 Painting (`GZPaint`, window slot 88, `0x009B167D`)

1. **The face.** Without an image, the flat face (`0x009B066C`):
   1. Nothing with flag `0x10`.
   2. `raised` := 1; style 2: not pressed; styles 3 and 4: neither pressed nor on.
   3. `RZDrawRaisedRect(draw context, draw area, the fill colour (window slot 126), raised, 1)`.
   4. Hovered: an outline 1 pixel inside the area, colour `0xFFFFFFFF`, width 1.

   With an image, the image face (`0x009B0575`):
   - The current frame is stretched with `RZDrawBltEdge(draw context, image, frame, draw area, 1)`, its edges at
     thirds, when:
     - flag `0x20` is clear or there is a caption,
     - the draw width differs from `fw`,
     - and the style is not 4.
   - Otherwise it is copied unscaled (draw context slot 38) to the draw area's origin + (`+0x150`, `+0x154`).
2. **The caption** (with flag 4 and a font, `0x009B1541`):
   - Colours: the disabled pair when not enabled, the hover pair when hovered, else the normal pair.
   - Offset: 1 pixel when flag `0x80` is set and the button is pressed.
   - Not wrapping: font slot 51 at (draw area + text position + offset), clipped to the draw area.
   - Wrapping: font slot 52 in the text box (its top not above the area), with the `+0x100` alignment.
3. Returns true.

## 5. Input *[compat]*

All handlers return true.

### 5.1 Mouse

| Slot | Event | Behaviour |
|---|---|---|
| 134 `GZOnMouseDownL` (`0x009B176C`) | left down | when not already pressed: press sound, capture, hide the tip, pressed := 1. With flag 2 (and not flag `0x800`): toggle sound, flip "on", Notify clicked. Invalidate |
| 135 `GZOnMouseDownR` (`0x009B17EF`) | right down | the same press; with flag 2: flag `0x800` → toggle sound, flip, clicked; otherwise right-clicked |
| 136 `GZOnMouseUpL` (`0x009B187C`) | left up | when pressed: release sound, pressed := 0. While capturing: when flag 2 is clear and the point is inside (window slot 121) and it was pressed: toggle sound, flip "on", Notify clicked. Release capture. Invalidate when it was pressed |
| 137 `GZOnMouseUpR` (`0x009B1923`) | right up | the same, but without flag `0x800` the click is right-clicked (no flip) |
| 141 `GZOnMouseEnter` (`0x009B1BE4`) | enter | when not pressed: hovered := 1, show the tip |
| 142 `GZOnMouseExit` (`0x009B19E0`) | exit | hovered := 0, hide the tip |
| 140 `GZOnCaptureChanged` (`0x009B19FF`) | capture moved | to another window: hovered := 0, pressed := 0, hide the tip |

So every completed left click flips the "on" flag, also for a plain push button: "on" alternates with each click,
and both `0x287259F7` (state) and `0x287259F6` (clicked) are posted *[quirk]*.

Sounds: `play_sound(i)` (`0x009B071A`): with a sound ID *i* and the sound system (`RZ::SS()`), its slot 13 makes a
sound (class `0x5B523BA4`, the ID) and slot 8 plays it.

### 5.2 Keys

- **`GZOnKeyDown(key, mods)`** (slot 130, `0x009B16B0`):
  - Enter (`0x0D`): Notify clicked.
  - Space (`0x20`), when not held: flag `0x4000` on, then `SendMsg(self, 7, 1, 1, mods)` (a left press at (1, 1)).
  - Any other key: `SendMsg(notification target, 5, key, mods, 0)`.
- **`GZOnKeyUp(key, mods)`** (slot 131, `0x009B1722`):
  - Space: flag `0x4000` off, `SendMsg(self, 10, 1, 1, mods)` (the release).
  - Any other key: `SendMsg(notification target, 6, key, mods, 0)`.

### 5.3 Flash and the tip

- **`Flash(count, period)`** (slot 44, `0x009B1058`):
  1. Stop the timer when it runs; `+0x188` := count; relayout.
  2. Start the timer with the callback `0x009B0311` and the period.

  The callback sends `0xC7EC278E` through window-manager slot 14 to the button's window.
- **`DoMessage`** (window slot 3, `0x009B1A3B`), on `0xC7EC278E`:
  1. Relayout.
  2. Then the count:
     - > 0 (unsigned): −1 becomes −2 and −2 becomes −1, so the phase keeps alternating forever; any other count
       decreases by 1;
     - 0: stop the timer.

  Other messages go to the base. The phase (bit 0) picks the hover frame (§4.1).
- **Showing the tip** (`show_tip`, `0x009B1AC0`), only with flag `0x100` and a non-empty text:
  1. A request is built from the defaults of `0x009B035D` (11 dwords: 1, …, `0xFFFFE0`, …). It holds:
     - the text;
     - the options: bit 1 from flag `0x200`, bit 2 from `0x400`, then the tip flags OR'd in;
     - the place: the absolute rectangle, or with option `0x10000` the tip point turned into screen coordinates
       (window slot 60);
     - the three tip colours.
  2. It is sent through the message server (`RZ::MS()` slot 3) as message `0x22C010D0`.
  3. `+0x170` := the ID the receiver writes back.
- **Hiding the tip** (`hide_tip`, `0x009B101E`): message `0x22C010D8` with that ID.

## 6. Quirks

- Every completed click flips the "on" flag, whatever the style.
- `SetBtnFlag` with a combined mask stores it without notifying.
- A count of −1 or −2 flashes forever.
- Frame modes 3 and 4 are unreachable.

## 7. Cases

| Case | Setup | Expected |
|---|---|---|
| BTN-C1 | style 2, enabled, hovered, not pressed | frame 3 |
| BTN-C2 | style 4, on, pressed | frame 4 |
| BTN-C3 | style 4, off, disabled | frame 7 |
| BTN-C4 | push button, a full left click inside | toggle sound; posts `0x287259F7` (on := 1), then `0x287259F6` with 1 |
| BTN-C5 | `SetStyle(2, null, rect (0, 0, 160, 30))` | frame 40 × 30 |
| BTN-C6 | `Flash(3, p)` | the first three timer messages bring the count to 0; the fourth stops the timer (each one relayouts) |
| BTN-C7 | Space down then up | `SendMsg(self, 7, 1, 1)` then `(self, 10, 1, 1)` |

## 8. Function inventory (owner `cGZWinBtn`)

All 87 functions:
`0x009A62BC`, `0x009AFFD3`, `0x009AFFF5`, `0x009AFFFD`, `0x009B0005`, `0x009B002D`, `0x009B0034`, `0x009B0045`,
`0x009B005E`, `0x009B0075`, `0x009B008D`, `0x009B00AC`, `0x009B00B4`, `0x009B00E9`, `0x009B0116`, `0x009B014F`,
`0x009B0174`, `0x009B01AB`, `0x009B01CC`, `0x009B01DF`, `0x009B022D`, `0x009B0243`, `0x009B0252`, `0x009B0263`,
`0x009B0280`, `0x009B029F`, `0x009B02C3`, `0x009B02EF`, `0x009B0303`, `0x009B0311`, `0x009B0338`, `0x009B035D`,
`0x009B038A`, `0x009B048D`, `0x009B0497`, `0x009B0507`, `0x009B0559`, `0x009B0568`, `0x009B0575`, `0x009B066C`,
`0x009B071A`, `0x009B0776`, `0x009B07B7`, `0x009B09B7`, `0x009B0B87`, `0x009B0C08`, `0x009B0F9E`, `0x009B0FF1`,
`0x009B101E`, `0x009B1058`, `0x009B10BF`, `0x009B10D0`, `0x009B112D`, `0x009B1179`, `0x009B1291`, `0x009B12EF`,
`0x009B1397`, `0x009B13DD`, `0x009B14DF`, `0x009B14E4`, `0x009B1541`, `0x009B167D`, `0x009B16B0`, `0x009B1722`,
`0x009B176C`, `0x009B17EF`, `0x009B187C`, `0x009B1923`, `0x009B19E0`, `0x009B19FF`, `0x009B1A3B`, `0x009B1AC0`,
`0x009B1BC2`, `0x009B1BCC`, `0x009B1BD6`, `0x009B1BE4`, `0x009B1C0C`, `0x009B1C27`, `0x009B1D78`, `0x009B1D80`,
`0x009B1D88`, `0x009B1D90`, `0x009B1D98`, `0x009B1DA0`, `0x009B1DA8`, `0x009B1E0A`, `0x009B1E2E`.

The thunks `0x009B1D78`–`0x009B1DA0` adjust the window pointer by −4 into the interface methods.
