# `cSC4WinAdviceList`: the advice list control (behaviour spec)

Scope: the `cSC4WinAdviceList` owner as tools/coverage.py assigns it (18 functions; inventory in §6). The control is
the HTML list of advice rows in three places:
- the advisor window (advisor-window.md §5.2, `+0x5C`);
- the news ticker's expanded list (news-ticker.md §4, `+0x38`);
- the news ticker's flipper (news-ticker.md §5, `+0x3C`).

Each row has an icon that toggles it open, a title, a body shown when open, and a close icon. Clicks come back as
`sc4://action/toggle?item=n` and `sc4://action/close?item=n` links. The HTML control inside it (`cGZWinHTML`) is out
of scope.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled; the HTML strings were read from the data section
(`0x00AB564C`–`0x00AB5868`). Written 2026-10-23 (re-advlist7).

Conventions:
- `L` is the control (its base, a `cGZWin`).
- `I` = `L+0xD8` is its list interface.
- `H` is the HTML control, with interface `0x4A11FD4A`; its slot 3 is its window.

## 1. Identity and fields *[compat]*

| Offset | vtable / field |
|---|---|
| `+0x00` | `cGZWin`, vtable `0x00AB58B0`. Overrides: 0 `QueryInterface`, 3 `DoMessage` (`0x00793400`), 4 `Init` (`0x00793190`), 5 `Shutdown` (`0x007930E0`), 55 `SetArea` (`0x00793100`), 148 the deleting destructor (`0x00793D50`) |
| `+0xD8` | the list interface, vtable `0x00AB5880`, IID `0xCA1492A2` (§2) |
| `+0xDC` | `H`. A borrowed pointer: `Init` releases its own reference, so the child list keeps `H` alive *[quirk]* |
| `+0xE0` | the row list: a `std::list` header node; nodes of 0x78 bytes (next, previous, row) |
| `+0xE4` | the background picture ID (0 = none) |

**A row** (0x70 bytes; constructor `0x007932B0`, copy `0x00793710`, destructor `0x00793380`):

| Offset | Field |
|---|---|
| `+0x00` | the close icon (HTML) |
| `+0x14` | the icon while open |
| `+0x28` | the icon while closed |
| `+0x3C` | the body |
| `+0x50` | the title |
| `+0x64` | byte: open |
| `+0x68` | the advice (counted) |
| `+0x6C` | byte: static (no toggle links) |

The other parts:
- **`QueryInterface`** (`0x00793080`, thunk from `I` `0x00793D10`):
  - IIDs 1 and `0xCA1492A2` give `I`;
  - `0x22BA0121` gives `L`;
  - anything else gives false. There is no fallback to the base class *[quirk]*.
- **Destructor**: clear the rows; free the list header; `~cGZWin`; free the object (global heap) when bit 0 of its
  argument is set.

## 2. The list interface (`0x00AB5880`)

| Slot | Address | Behaviour |
|---|---|---|
| 3 | shared | `L` |
| 4 | `0x00793DA0` | `AddEntry(openIcon, closedIcon, body, title, open, advice, closeIcon, static, background)` |
| 5 | `0x00793810` | `RebuildHTML(keepScroll)` (§3) |
| 6 | shared | `(id)`: background := `id` |
| 7 | `0x00793800` | `ClearList` (`0x007936D0`): destroy every row, empty the list |
| 8 | `0x00793150` | four arguments passed to `H` slot 32, when `H` exists (`SetGutters`) |
| 9 | `0x00793170` | arguments passed to `H` slot 9 (`ScrollToAnchor`) |
| 10 | `0x00793180` | `H` slot 28 `(8)` (`DisableVerticalScrollbar`) |

`AddEntry` appends a row at the end of the list:
- the five strings are copied from the `cIGZString*` arguments (string slot 13);
- `open` and `static` are bytes;
- `advice` is kept with a reference;
- `background` replaces the control's background ID, the same field as slot 6.

So the advisor window's rows (advisor-window.md §5.2) are: the two state icons, the body and title HTML, the
"open" flag (`CheckFlags(0x20)`), the advice, the close icon, not static, and background `0x14416264`.

## 3. `RebuildHTML(keepScroll)` (`0x00793810`)

1. With `keepScroll`, save `H`'s scroll position (`H` slot 6 `(&x, &y)`).
2. `cell` := `</TD><TD WIDTH="%d">` with `H`'s window width − 61.
3. Start the page:
   - no background: `<HTML><HEAD></HEAD><BODY BGCOLOR="#dee8e3"><FONT FACE="Arta" SIZE=3><TABLE BORDER=0>`;
   - otherwise the same with `BODY BACKGROUND="sc4://HTML/46a006b0/%x"` (the background ID in hexadecimal).
4. For each row `n` (0, 1, … in list order), with `S` = its static byte:
   1. `<TR><TD WIDTH="18">`.
   2. Unless `S`: `<A NAME="item%d" HREF="sc4://action/toggle?item=%d">` with `n` twice.
   3. The open or closed icon.
   4. Unless `S`: `</A>`.
   5. `cell`.
   6. Unless `S`: `<A HREF="sc4://action/toggle?item=%d"><SPAN STYLE="text-decoration: none">`.
   7. The title.
   8. Unless `S`: `</SPAN></A>`.
   9. When open: `<BR>` and the body.
   10. `</TD><TD WIDTH="18"><A NAME="item%d" HREF="sc4://action/close?item=%d">`, the close icon, `</A></TD></TR>`.
5. End: `</TABLE></FONT></BODY></HTML>`.
6. `H` slot 22 `(page, "sc4://")` sets the document with that base.
7. With `keepScroll`, restore the scroll position (`H` slot 7 `(x, y)`).

The close link is written even for static rows.

## 4. `DoMessage(msg)` (`0x00793400`)

Messages other than `0xE4A85D91` (a link, URL `cRZString*` at `msg+8`) go to `cGZWin::DoMessage`. For a link:
1. The click sound: the sound service (framework service `0x23CECBA0`, IID `0x106077D2`), when present, gets slot 11
   `(1, 0x2A5C322B, 0, 0, 0)`. It plays for every link.
2. `sc4://action/toggle?` + `item=%d`, with `n` read:
   1. Row `n` is found by stepping `n` nodes from the first, with no bound check *[quirk]*.
   2. Its open byte is inverted.
   3. Post `0x426841A1` (message server slot 4): data1 := the advice's slot 11 (its advisor ID), data2 := the
      advice's slot 6.
   4. The advice gets `SetFlags(0x20, open)` (slot 14).
   5. `RebuildHTML(1)`.

   Result false.
3. `sc4://action/close?` + `item=%d`, with `n` read:
   1. Find row `n` the same way.
   2. The advice gets `SetFlags(8, 1)`.
   3. Send `0xCA09FA6C` at once (message server slot 3), with data1 `0xCA09FA6C` and the advice in the message's
      slot 33 field.
   4. `RebuildHTML(1)`. The row itself is not removed here; the owners remove it on `0xCA09FA6C`
      (news-ticker.md §3).

   Result false.
4. When `item=%d` does not parse, nothing more; false.
5. Any other URL: `cSC4HTMLLinkHandler::DoURL(url, 0)` on an empty local handler. The result is true when `DoURL`
   returned false, else false.

## 5. `Init`, `Shutdown` and `SetArea`

- **`Init`**: true at once when initialised.
  1. `cGZWin::Init`; false when it fails.
  2. Create `H` with GZCOM `CreateInstance(0xAA12E5F5, 0x4A11FD4A)`; false when that fails.
  3. `H`'s window gets `SetArea(0, 0, width, height)` of `L`; `H` slot 28 `(0x40)`.
  4. `L` adds `H`'s window as a child (slot 14). `+0xDC` := `H`; the creation reference is released.
  5. True.
- **`Shutdown`**: `ClearList` (interface slot 7), then tail-call `cGZWin::Shutdown`.
- **`SetArea(l, t, r, b)`**: `cGZWin::SetArea`, then `H`'s window gets `SetArea(0, 0, r − l, b − t)` when `H`
  exists.

| Case | Setup | Expected |
|---|---|---|
| AL-C1 | two rows, row 1 open, not static, background 0 | page with two `<TR>`; row 1 has `<BR>` and its body; links `item0` and `item1` |
| AL-C2 | link `sc4://action/toggle?item=1` | row 1 closed; `0x426841A1` posted; advice `SetFlags(0x20, 0)`; rebuilt with the scroll kept; false |
| AL-C3 | link `sc4://action/close?item=0` | advice `SetFlags(8, 1)`; `0xCA09FA6C` sent; false |
| AL-C4 | static row | no toggle anchors; the close anchor stays |

## 6. Function inventory (owner `cSC4WinAdviceList`)

All 18 functions:
- §1: `0x00793080`, `0x00793D10`, `0x00793D50`, `0x007932B0`, `0x00793710`, `0x00793380`;
- §2: `0x00793DA0`, `0x007936D0`, `0x00793800`, `0x00793150`, `0x00793170`, `0x00793180`, `0x00793280` (a list
  iterator advance by `n`, forwards or backwards);
- §3: `0x00793810`;
- §4: `0x00793400`;
- §5: `0x00793190`, `0x007930E0`, `0x00793100`.

Not covered: the HTML control (out of scope); `H` slots 6, 7, 9, 22, 28 and 32 are named from their callers only.
