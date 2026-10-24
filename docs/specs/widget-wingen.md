# `cGZWinGen` and `cGZWinFlatRect`: the general window and its colour bars (behaviour spec)

Scope: two owners as tools/coverage.py assigns them on `bb641878`:
- `cGZWinGen` (56 functions, 7,049 bytes; inventory in §10): the **general window** every UI-script window and every
  game dialog is built on. It has an optional title bar, an optional side bar, colour bars and dividers, gutters, a
  background image, drag-to-move and drag-to-resize, and a pluggable window procedure;
- `cGZWinFlatRect` (17 functions, 1,509 bytes, §9): the flat rectangle used for the colour bars, the dividers, the
  side bar and as the base of the title bar;
- `cGZWinGenTitleBar` (23 functions, 3,168 bytes, §13): the title bar, with its caption and three system buttons.

Related:
- widget-toolkit.md: the `cGZWin` base (fields §2, flags §3, children §4, hit tests §5, `SendMsg`/`PostMsg` §7,
  `EnumChildren` §18);
- ui-windows.md §2 (the `cIGZWin` slots) and §5 (windows created from scripts);
- legacy-control.md §9.7 (FlatRect script properties) and §9.20 (SC3WinGen script properties: the source of the
  flag names in §1.1);
- modal-dialogs.md §2 (the end message `0xF`, window flag `0x1000` = "is the current modal window");
- widget-button.md (the title bar's system buttons are buttons).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the stack-argument tricks, the paint calls,
the size clamps and the cursor switch were checked on the instructions. The vtables `0x00ADC678` (window),
`0x00ADC5F8` (`cIGZWinGen`), `0x00AE20A0` (flat-rect window) and `0x00AE2050` (`cIGZWinFlatRect`) were read and
compared slot by slot with the `cGZWin` table `0x00ADC8D8`. Written 2026-10-23 (re-gzwingen7).

Conventions:
- Offsets are from the window start (the `cGZWin` part is at `+0`, widget-toolkit.md §2). The `cIGZWinGen`
  interface is at `+0xD8`; its methods subtract `0xD8` first. "Window slot n" is a `cIGZWin` slot; "gen slot n" a
  `cIGZWinGen` slot.
- `W`, `H` are window slots 41 and 42. `gx`, `gy` are the gutter bytes `+0x114` and `+0x115`.
- **Win-proc call** `P(type, d1, d2, d3)`: with a window procedure at `+0x10C`, build the message
  `{type, d1, d2, d3}` and call the procedure's slot 3 `(window, &msg)`; if that returns false and the procedure is
  still set, call its slot 4 `(window, type, d1, d2, d3)`. The call is **handled** if either returned true. With
  no procedure nothing happens and the call is not handled.
- Rectangles are `{l, t, r, b}`. The draw area is `+0x24` (widget-toolkit.md §17.1).

## 1. Identity and fields *[compat]*

- **Class** `0x4386D516` (`GetCLSID`, gen slot 4, `0x0099908D`); script class names `GZWinGen` and `SC3WinGen`
  (legacy-control.md §4.1).
- **`QueryInterface`** (`0x00998F83`): `0x5386D516` gives `+0xD8` (null for a null object), AddRef'd, true;
  anything else goes to `cGZWin::QueryInterface`. Gen slot 0 (`0x0079C580`) is the thunk that subtracts `0xD8`.
- **Window overrides** (all other slots are `cGZWin`'s): 0 `QueryInterface`, 3 `DoMessage`, 4 `Init`, 5 `Shutdown`,
  55 `SetArea(l, t, r, b)`, 62 `IsPointInWindowScreenCoordinates`, 74 `SetCaption`, 85 `SetCursor`, 88 `GZPaint`,
  121 `IsPointInWindowWindowCoordinates`, 130 key down, 131 key up, 134 left button down, 136 left button up,
  138 mouse move, 143 `GZOnCommand`, 148 destructor, and an extra slot 151 `AsIGZWin` (`0x009990A5`).
- **Constructor** (`0x0099B6A5`): `cGZWin` constructor, then the fields below; then `SetWinGenFlag(0x10, 1)`,
  `(4, 1)`, `(1, 1)`. So a new window has generator flags `0x15`: moveable, default keys, close button visible.
- **Destructor** (`0x0099AF6E`; deleting form `0x0099B485`, bit 0 frees): restore both vtables, `Shutdown`,
  destroy the user-data map, release the held cursor, `~cGZWin`.

| Offset | Size | Meaning | Constructor value |
|---|---|---|---|
| `+0xD8` | 4 | `cIGZWinGen` vtable | `0x00ADC5F8` |
| `+0xDC` | 4 | generator flags (§1.1) | 0, then `0x15` |
| `+0xE0` | 4 | drag mode while the left button is held (§6) | 0 |
| `+0xE4` | 4 | the hover mode that last chose the cursor (§6.3) | 0 |
| `+0xE8`, `+0xEC` | 4+4 | minimum width and height (§4.1) | 0 |
| `+0xF0` | 4 | the title bar (`cGZWinGenTitleBar`, a child window), or null | null |
| `+0xF4` | 4 | background blit type: 0 tiled, 1 plain, 2 edge (§5) | **2** |
| `+0xF8` | 4 | background image (`cIGZBuffer`, counted), or null | null |
| `+0xFC..+0x108` | 16 | **user area**: the part inside gutters, title bar and side bar (§4.2) | not set (`Init` computes it) |
| `+0x10C` | 4 | window procedure (counted), or null | null |
| `+0x110` | 4 | user dword (gen slots 19–23) | 0 |
| `+0x114`, `+0x115` | 1+1 | gutters `gx`, `gy` | **4**, **4** |
| `+0x118` | 4 | the cursor set from outside (counted, §6.3) | null |
| `+0x11C` | — | user-data map: key (dword) → counted `cIGZUnknown` | empty |

### 1.1 Generator flags (`+0xDC`)

The names come from the script properties that set them (legacy-control.md §9.20).

| Bit | Script property | Effect here |
|---|---|---|
| `0x1` | `moveable` | the non-user area (or the title bar) drags the window (§6) |
| `0x2` | `sizeable` | the gutters resize the window (§6) |
| `0x4` | `defaultkeys` | Esc and Alt+F4 close the window (§7) |
| `0x10` | `closevisible` | title-bar flag 1 |
| `0x20` | `gobackvisible` | title-bar flag 4 |
| `0x40` | `minmaxvisible` | title-bar flag 2 |
| `0x100` | `closedisabled` | title-bar flag `0x100` |
| `0x200` | `gobackdisabled` | title-bar flag `0x400` |
| `0x400` | `minmaxdisabled` | title-bar flag `0x200` |
| `0x1000` | `titlebar` = 0 | no title bar |
| `0x2000` | fill = 0 | no background fill or image |
| `0x4000` | outline = 0 | no outline |
| `0x8000` | `paint` = 0 | `GZPaint` draws nothing |
| `0x10000` | `sidebar` | a side bar along the left edge |

Note that bit `0x1000` of the **window** flag word (`cGZWin` `+0xC8`) is a different flag: "current modal window"
(modal-dialogs.md §2.2). §7 tests that one.

## 2. The `cIGZWinGen` slots *[compat]*

Vtable `0x00ADC5F8`, 32 slots. Slots 1 and 2 are the shared AddRef and Release thunks (`0x007BE710`,
`0x007BE720`); slot 3 is the shared `AsIGZWin` thunk `0x0095BA6F`. Every setter returns true unless stated.

| Slot | Address | Behaviour |
|---|---|---|
| 4 `GetCLSID` | `0x0099908D` | `0x4386D516` |
| 5 `IsWinGenFlag(mask)` | `0x00999055` | `(flags & mask) != 0` |
| 6 `SetWinGenFlag(mask, on)` | `0x0099B5BF` | §3 |
| 7 `SetWinProc(proc)` | `0x009998C8` | §3.1 |
| 8 `GetWinProc` | `0x009DB800` | `+0x10C`, no AddRef (a shared 4-byte getter) |
| 9 `GetWinProcAs(iid, out)` | `0x00999093` | no procedure → false; else the procedure's `QueryInterface(iid, out)` |
| 10 `GetUserArea(rect*)` | `0x00999065` | copies `+0xFC..+0x108` out |
| 11 `SetUserArea(const rect*)` | `0x00999079` | copies it in; nothing is recomputed or notified |
| 12 `SetBackgroundImage(image)` | `0x009990EC` | if different: AddRef the new one, release the old one, store |
| 13 `SetBackgroundImage(type, image)` | `0x009990B0` | if the image or the type differs: as slot 12, and store the type |
| 14 `GetBackgroundImage(out)` | `0x00999131` | `out` null → nothing; else `*out = +0xF8` and **AddRef it unchecked** (crashes when there is no image) |
| 15 `GetBackgroundImage(outType, outImage)` | `0x0099911A` | both out, no AddRef |
| 16 `SetBlitType(type)` | `0x00999159` | stores `+0xF4` |
| 17 `GetBlitType(out)` | `0x0099914B` | `*out = +0xF4` |
| 18 `SetUserData(key, unknown)` | `0x0099A90B` | slot 24 (key); then, if `unknown` is non-null, insert `key → unknown` (AddRef'd). No defined return value (AL is whatever the last call left) |
| 19 `SetUserDword(value)` | `0x00945EAC` | `+0x110 := value` (script property `userdata`) |
| 20 `SetUserDword(key, value)` | `0x00999169` | key 0 → slot 19; other keys → false |
| 21 `GetUserData(key, iid, out)` | `0x0099A5ED` | find `key`; absent → false; else the stored object's `QueryInterface(iid, out)` |
| 22 `GetUserDword` | `0x00999165` | `+0x110` |
| 23 `GetUserDword(key, out)` | `0x00999180` | key 0 → `*out = slot 22()`, true; other keys → false, `out` untouched |
| 24 `RemoveUserData(key)` | `0x0099A643` | erase the entry if present (its reference released); no defined return value |
| 25 `SetGutters(x, y)` | `0x009991C7` | forwards to slot 26 |
| 26 `SetGutters(byte x, byte y)` | `0x0099919B` | stores `+0x114`, `+0x115`; **nothing is laid out again** |
| 27 `GetGutters(l, t, r, b)` | `0x009991D7` | `gx`, `gy`, `gx`, `gy` |
| 28 `GetGutters(x, y)` | `0x009991AE` | `gx`, `gy` |
| 29 `AddColorBar(id, autoSize, style, const rect*)` | `0x009999BA` | §8.1 |
| 30 `AddDividerH(id, y)` | `0x00999AFE` | §8.2 |
| 31 `RemoveColorBar(id)` | `0x00999204` | child with that ID (window slot 34) → `ChildDelete` (window slot 16), true; none → false |

The script loader reaches slots 7, 13, 19, 26, 29 and 30 (legacy-control.md §9.20: `bluebar` is slot 29 with
`autoSize` 0 and style 0, `divider` is slot 30).

## 3. `SetWinGenFlag(mask, on)` (gen slot 6)

`on` is tested as a byte.
1. `flags := on ? flags | mask : flags & ~mask`.
2. With a title bar, the six title-bar flags are mapped and passed on with
   `cGZWinGenTitleBar::SetTitleBarFlag(bit, on)`: `0x10` → 1, `0x20` → 4, `0x40` → 2, `0x100` → `0x100`, `0x200` →
   `0x400`, `0x400` → `0x200`. For `0x20` … `0x400` the function then returns true at once; for `0x10` it goes on to
   step 3, which does nothing for it.
3. Only for the masks `0x1000` and `0x10000` exactly (a combined mask does nothing more):
   - `0x1000` on → destroy the title bar (§4.4); off → create it (§4.3);
   - `0x10000` on → create the side bar (§4.5); off → destroy it;
   - then `calc_user_area(W, H, &userArea)` (§4.2) and `on_layout` (§4.6).
4. Return true.

Without a title bar the title-bar flags are only stored; `create_title_bar` copies them over (§4.3).

### 3.1 `SetWinProc(proc)` (gen slot 7, `0x009998C8`)

If `proc` equals the current procedure: nothing. Otherwise:
1. With an old procedure: `P(0xA2BF8AD6, 0, 0, 0)` (detached), then release it.
2. Store `proc`. If non-null: AddRef it, `P(0xA2BF8AD5, 0, 0, 0)` (attached), then `P(0xA2BF8AD8, 0, 0, 0)`.
3. Return true.

The window argument of these calls is the window (`+0`), not the interface.

### 3.2 Window-procedure messages

| Type | Sent by | d1, d2, d3 | Use of "handled" |
|---|---|---|---|
| `0xA2BF8ACD` | `GZPaint`, first | draw area, 0, 0 | handled → skip the window's own drawing |
| `0xA2BF8ACE`, `0xA2BF8ACF` | `GZPaint`, last, in this order | draw area, 0, 0 | ignored |
| `0xA2BF8AD0` | `Init` | 0, 0, 0 | not handled → `Init` returns **false** |
| `0xA2BF8AD1` | `Shutdown` | 0, 0, 0 | ignored |
| `0xA2BF8AD2` | `SetArea`, `on_layout` | limits `{minW, minH, maxW, maxH}` (in: 0, 0, `0x7FFFFFFF`, `0x7FFFFFFF`), the proposed user area, 0 | handled → the user area is clamped to the limits (§4.1) |
| `0xA2BF8AD3` | `SetArea`, `on_layout` | user area `+0xFC`, the window's area (window slot 48), 0 | ignored |
| `0xA2BF8AD4` | slot 121 | `(x << 16) | (y & 0xFFFF)`, pointer to a result byte (0), 0 | handled → the byte is the answer (§6.4) |
| `0xA2BF8AD5`, `0xA2BF8AD6` | `SetWinProc` | 0, 0, 0 | ignored |
| `0xA2BF8AD8` | `SetWinProc`; the message server | — | §3.3 |

All other messages reach the procedure through `DoMessage` (§3.3).

### 3.3 `DoMessage(msg)` (window slot 3, `0x0099A276`)

1. Type `0xA2BF8AD8` (the layout request): if `msg.d1` equals the window ID (`+0x10`), `on_layout` and return true;
   otherwise return false. The window does **not** pass this message to its procedure here.
2. Types 7 … 14 (the input messages): with a procedure, hold a counted reference to the window and pass the
   message to it (slot 3 with `msg`, then slot 4 with its four dwords); release; then **always** continue with
   `cGZWin::DoMessage`.
3. Any other type: with a procedure, the same call; if handled, release and return true. Otherwise (or without a
   procedure) `cGZWin::DoMessage`.

`Init` subscribes the window to `0xA2BF8AD8` on the message server (server slot 5); `Shutdown` unsubscribes
(server slot 6). So posting `{0xA2BF8AD8, id}` lays out the window with that ID.

## 4. Layout *[compat]*

### 4.1 `SetArea(l, t, r, b)` (window slot 55, `0x0099A0AF`)

1. `w := r − l`, raised to the minimum width `+0xE8` when not larger; `h := b − t`, likewise with `+0xEC`.
2. If `w == W` and `h == H` (the size does not change): `cGZWin::SetArea(l, t, l + w, t + h)` and go to step 6.
   Gutters, controls and the user area are **not** recomputed on a move.
3. With a procedure: `calc_user_area(w, h, &ua)`; send `0xA2BF8AD2` with the default limits and `&ua`. If handled,
   clamp: when `ua` is narrower than `minW` its right edge becomes `ua.l + minW`, else when wider than `maxW` it
   becomes `ua.l + maxW`; the same for the height with `minH`/`maxH`. Then `calc_win_area(&ua, &wa)` and
   `w := wa.r − wa.l`, `h := wa.b − wa.t`. The minimum of step 1 is not applied again.
4. `cGZWin::SetArea(l, t, l + w, t + h)`; `place_controls` (§4.7); `calc_user_area(w, h, &userArea)`.
5. (Falls through.)
6. With a procedure: `P(0xA2BF8AD3, &userArea, window slot 48 (), 0)`.

`cGZWin::SetArea(const rect&)` (window slot 54) calls slot 55, so every resize, including the drags of §6, passes
through this function.

### 4.2 `calc_user_area(w, h, out)` (`0x0099922F`) and `calc_win_area(in, out)` (`0x009992AE`)

`calc_user_area`, for a window of size `w × h`:
- `out.l = gx`, or `2·gx + 8` with flag `0x10000` (the side bar is 8 pixels wide);
- `out.t = gy`, or with a title bar `title.GetB() + gy` (title window slot 46);
- `out.r = w − gx`;
- `out.b = h − gy`, but never above `out.t` (`max(h − gy, out.t)`).

`calc_win_area` is the inverse, for a user area `in`:
- `out.l = in.l − gx`, and `− gx − 8` more with flag `0x10000`;
- `out.t = in.t − gy`, or with a title bar `in.t − 2·gy − title.H` (title window slot 42);
- `out.r = in.r + gx`, `out.b = in.b + gy`.

Both return true. The pair is exact when the title bar sits at `y = gy`, which `create_title_bar` arranges.

### 4.3 `create_title_bar` (`0x0099A96E`)

Nothing if there is one. Otherwise, with the window-control manager (`RZ::WCM`, may be absent):
1. `c1 := WCM.GetColor(0xBB5B2EC6)` and `c2 := WCM.GetColor(0xBB5B2EC7)` (manager slot 32; 0 when absent).
2. Allocate `0x11C` bytes and construct a `cGZWinGenTitleBar`; store it in `+0xF0`; AddRef (an allocation failure
   is not checked).
3. Title window: `SetID(0x42B7C351)` (slot 64), `GZWinMoveTo(gx, gy)` (slot 56), `SetFlag(0x800, 0)`.
4. For `0x10`, `0x40`, `0x20`, `0x100`, `0x400`, `0x200` in this order: `SetWinGenFlag(f, IsWinGenFlag(f))`, which
   copies the flag to the new title bar (§3).
5. Title's flat-rect interface (title `+0xD8`): `SetOutlineFlags(0xF)` (slot 12), `SetAutoSizeMode(5)` (slot 18);
   title window slot 106 `SetFillColor(c1)`; flat-rect slot 8 with `c2 & 0xFFFF` (all four edge colours, §9.2).
6. `ChildAdd(title)` (window slot 14). On failure: release the title bar, clear `+0xF0`, return **false**.
7. `SetCaption(GetCaption())` (window slots 74 and 73), which copies the caption to the title bar.
8. Title `AutoSize({gx, gy, W − gx, H − gy})` (flat-rect slot 17; mode 5 stretches it to the full inner width).
9. Minimum size: `cGZWinGenTitleBar::GetMinSize(&minW, &minH)` into `+0xE8`/`+0xEC`, then `+ 2·gx` and `+ 2·gy`. If
   `W` or `H` is below it: `SetSize(minW, minH)` (window slot 53).
10. With a side bar (child `0x42B7C352`): `SetFlag(0x800, 1)` on it, `ChildToFront(side)` (window slot 26),
    `SetFlag(0x800, 0)`; then the side bar's `SetArea(gx, title.GetB() − 1, gx + 8, H − gy)`. *Quirk*: the call
    pushes `H − gy` and `gx + 8` before it calls the argument-less `GetB`, so they stay on the stack as the side
    bar's right and bottom.
11. Return true.

### 4.4 `destroy_title_bar` (`0x0099933C`)

With a title bar: `ChildRemove(title)` (window slot 15), release it, clear `+0xF0`; minimum size `:= (2·gx, 2·gy)`;
with a side bar, `SetArea(gx, gy, gx + 8, H − gy)` on it. Return true. The title bar is released, not shut down;
`Shutdown` (§5.3) also calls its slot 5.

### 4.5 Side bar (`create_side_bar` `0x00999EB9`, `destroy_side_bar` `0x009993D7`)

Create: nothing if a child `0x42B7C352` exists (window slot 19). Otherwise its rectangle is `l = gx`, `r = gx + 8`,
`t = title ? title.GetB() − 1 : gy`, `b = H − gy`; `AddColorBar(0x42B7C352, 4, 0xD, &rect)` (auto-size 4 = bottom,
style `0xD` = left, right and bottom edges, §9.1). Failure → false. Then the bar goes to the back (`SetFlag(0x800, 1)`,
`ChildToBack` (window slot 27), `SetFlag(0x800, 0)`), and so does the title bar, the same way. Return true.

Destroy: the child `0x42B7C352` (window slot 34), if any, is deleted (window slot 16). Return true.

### 4.6 `on_layout` (`0x00999D43`)

Without a procedure, return false. Otherwise:
1. `ua := userArea`; send `0xA2BF8AD2` with the default limits and `&ua`.
2. If handled: clamp `ua` as in §4.1 step 3; `calc_win_area(&ua, &wa)`; if `wa`'s width differs from `W` or its
   height from `H`, `SetSize(wa width, wa height)` (window slot 53, which reaches §4.1 and its own `0xA2BF8AD3`) and
   return true.
3. `P(0xA2BF8AD3, &userArea, window slot 48 (), 0)`; return true.

So a window procedure sizes its window by answering `0xA2BF8AD2` with the limits of the user area it wants
(equal minimum and maximum give a fixed size).

### 4.7 `place_controls` (`0x00999BDD`)

`EnumChildren(0xC2AFA76F, cb, &packed)` (window slot 32) with `packed = (gx << 16) | gy`; returns that result
(false when there are no children). The callback (`0x00999B84`) gives every flat-rect child (colour bars, dividers,
side bar, title bar) `AutoSize({gx, gy, W − gx, H − gy})` (flat-rect slot 17, §9.3), where `W`, `H` are the
general window's.

## 5. Lifecycle and painting

### 5.1 `Init` (window slot 4, `0x0099AFA8`)

Once (the `cGZWin` init flag): `cGZWin::Init`, then:
1. With the window-control manager and no background image: `image := WCM.GetPicture(0xE2B14588)` (manager slot 7;
   registered at start-up, application.md AP13); if non-null, `SetBackgroundImage(0, image)` (gen slot 13, blit type
   0 = tiled).
2. Without flag `0x1000`: `create_title_bar` (its result is ignored).
3. `place_controls`; `calc_user_area(W, H, &userArea)`.
4. Subscribe to `0xA2BF8AD8` (message server slot 5).
5. Window manager slot 29 (`SetCapture`) with null, then `SetFocus()` (widget-toolkit.md §6).
6. `P(0xA2BF8AD0, 0, 0, 0)`: not handled → return **false** (the window stays initialised).
7. `on_layout`; return true.

A second call returns true and does nothing.

### 5.2 `Shutdown` (window slot 5, `0x0099A804`)

Not initialised → true. Otherwise, holding a counted reference to itself:
1. Hide (window slot 70).
2. Unsubscribe from `0xA2BF8AD8`.
3. Title bar: `ChildRemove`, its `Shutdown` (slot 5), release, clear.
4. Release the background image, clear `+0xF8`.
5. `P(0xA2BF8AD1, 0, 0, 0)`; then `SetWinProc(null)` (so the procedure also gets `0xA2BF8AD6`).
6. Clear the user-data map; return `cGZWin::Shutdown`.

### 5.3 `GZPaint` (window slot 88, `0x009995E7`)

1. Flag `0x8000` → return true (nothing drawn, no procedure calls).
2. `P(0xA2BF8ACD, drawArea, 0, 0)`; handled → step 5.
3. **No background image:**
   - flag `0x4000` (no outline): unless `0x2000`, fill the draw area with the fill colour (draw context slot 21 with
     `GetFillColorRGB` (window slot 126), then slot 35 with the draw area). Then step 5.
   - otherwise, unless `0x2000`: `RZDrawRaisedRect(context, drawArea, fill colour, 1, 1)`, then step 5. *Quirk*: the
     two 1s are pushed before the argument-less `GetFillColorRGB` call and become the last two arguments.
   - otherwise (flag `0x2000`, no `0x4000`): step 4.
4. **With an image**, unless `0x2000`, by blit type (`+0xF4`), with `src` = the image's own rectangle (buffer slot
   12):
   - 0: draw context slot 39 `(image, src, drawArea, 0, 0)` *[inferred: tiled]*;
   - 1: draw context slot 38 `(image, src, drawArea)` (the plain copy, as in widget-toolkit.md §14);
   - 2: `RZDrawBltEdge(context, image, {0, 0, imageW / 3, imageH / 3}, drawArea, 1)` (buffer slots 9, 10; corner
     pieces one third of the image);
   - other: nothing.
   As in step 3, the two zeros of type 0 and the draw area of types 0 and 1 are pushed before the argument-less
   buffer slot 12 call.

   Then, unless flag `0x4000`: `RZDraw3DRectangleOutline(context, drawArea, 0xFFFFFFFF, 0xFF000000)`. Step 3's last
   case also comes here.
5. With a procedure: `P(0xA2BF8ACE, drawArea, 0, 0)`, then `P(0xA2BF8ACF, drawArea, 0, 0)`. Return true.

## 6. Moving and resizing *[compat]*

### 6.1 `calc_size_mode(x, y)` (`0x00999C15`), window coordinates

0 when the point is inside the user area (inclusive edges). Otherwise:
1. With flag 2 (sizeable), by the gutter the point is in (`x < gx` left, `x > W − gx` right, `y < gy` top,
   `y > H − gy` bottom): left 2, right 3, top 4, bottom 5, top-left 6, top-right 7, bottom-left 8, bottom-right 9;
   0 when in none (for example beside the side bar).
2. Then, with flag 1 (moveable): without a title bar the result is **1** (move) for every point outside the user
   area, so it **replaces** any edge of step 1. With a title bar, the point is tested (inclusive) against the
   title bar's area (title window slot 48, which is in this window's coordinates): inside → 1, outside → step 1's
   result. *Quirk*: `x`, `y` are pushed before the argument-less `GetArea` and become the test's arguments.

### 6.2 Buttons and drag (`0x0099A361`, `0x00999039`, `0x0099A3A7`)

- **Left down** `(x, y)`: `+0xE0 := calc_size_mode(x, y)`. If not 0: `GZSetCapture`; on failure `+0xE0 := 0`, on
  success the mouse-down offset `+0xA0`, `+0xA4 := (x, y)`. Return true always.
- **Left up**: if `+0xE0` ≠ 0: clear it, `GZReleaseCapture`, return true; else false.
- **Move** with `+0xE0` ≠ 0: `dx = x − +0xA0`, `dy = y − +0xA4`; `rect := window area` (window slot 48, parent
  coordinates):

  | Mode | Change | Offset update |
  |---|---|---|
  | 1 move | all four edges by `(dx, dy)` | — |
  | 2 left | `l += dx` | — |
  | 3 right | `r += dx` | `+0xA0 += dx` |
  | 4 top | `t += dy` | — |
  | 5 bottom | `b += dy` | `+0xA4 += dy` |
  | 6 top-left | `l += dx`, `t += dy` | — |
  | 7 top-right | `t += dy`, `r += dx` | `+0xA0 += dx` |
  | 8 bottom-left | `l += dx`, `b += dy` | `+0xA4 += dy` |
  | 9 bottom-right | `r += dx`, `b += dy` | both |

  Then `cGZWin::SetArea(rect)` (so §4.1 with its minimum and limits) and, for mode 1,
  `cGZWin::InvalidateAllParents`, else window slot 92 (`InvalidateSelfAndParents`). Other modes return at once.
  Return true. Left and top edges move the window, so the window-relative point stays where it was; right and
  bottom edges move away from it, hence the offset update.

Note the left and top edges are not clamped against the minimum size by moving the opposite edge: §4.1 keeps
`l` and `t` and lengthens the window to the right and bottom instead, so dragging the left edge past the minimum
pushes the window to the right.

### 6.3 Cursor (`0x0099A3A7` without a drag; `SetCursor` `0x00999B56`)

Mouse move without a drag: `m := calc_size_mode(x, y)`. Only with **both** flags 1 and 2, when `m` differs from
`+0xE4` and there is a cursor manager (`cGZWin::GetCursorManager`): `+0xE4 := m`, then choose a cursor by ID through
the cursor manager's slot 9:

| `m` | Cursor ID | When the ID gives null |
|---|---|---|
| 2, 3 | 10 | try `0x11`; still null → nothing |
| 4, 5 | 8 | same |
| 6, 9 | 9 | same |
| 7, 8 | 7 | same |
| 1 | 1 | nothing |
| 0 | the held cursor `+0x118` if set, else ID 1 | nothing |

The chosen cursor goes to `cGZWin::SetCursor(cursor, 1)`. Return true.

`SetCursor(cursor, arg)` (window slot 85) stores `cursor` in `+0x118` (counted) and passes it to
`cGZWin::SetCursor` only while `+0xE4` is 0. So the cursor someone else sets is kept while the pointer is over an
edge and restored when it moves back into the user area.

### 6.4 Hit tests (`0x0099955D`, `0x00998FD7`)

- **Window coordinates** (window slot 121): with a procedure, `P(0xA2BF8AD4, (x << 16) | (y & 0xFFFF), &b, 0)` with
  `b = 0`; handled → return `b != 0`. Otherwise `cGZWin::IsPointInWindowWindowCoordinates(x, y)`.
- **Screen coordinates** (window slot 62): `ScreenToWindowCoordinates(&x, &y)` (window slot 59) on its own
  arguments, then slot 121. So the procedure's answer also decides the screen test, and with it the toolkit's
  `GetChildWindowFromPoint` walks.

## 7. Keys, commands and closing *[compat]*

- **Key down** `(vk, modifiers)` (window slot 130, `0x00999832`): with flag 4 (default keys), Esc (`0x1B`) or F4
  (`0x73`) with modifier bit 4 (Alt) → `kill_self(0x42B7C353)`, return true. Otherwise, while the window is the
  current modal window (window flag `0x1000`) → return false. Otherwise return `SendMsg(notification target, 5, vk,
  modifiers, 0)` (window slot 144).
- **Key up** (window slot 131, `0x00999004`): modal → false; else `SendMsg(notification target, 6, vk, modifiers, 0)`.
- **`GZOnCommand(type, id)`** (window slot 143, `0x00999896`): type `0x287259F6` (a button press) with
  `id = 0x42B7C353` or `0x42B7C356` → `kill_self(id)`, true. Everything else → false. `0x42B7C353` is the title bar's
  close button.
- **`kill_self(result)`** (`0x009993F7`): when the window is the current modal window (window flag `0x1000`), post
  the modal end message: window manager slot 9 `GZWinMsgPost(0xF, window, result)` (modal-dialogs.md §2.4).
  Otherwise `PostMsg(parent, 1, window, 0, 0)` (window slot 146), the `result` dropped. Returns true.

So closing a modeless general window asks its parent (message type 1 with the window); closing a dialog ends its
modal loop with the button ID as the result.

## 8. Colour bars and dividers

### 8.1 `AddColorBar(id, autoSize, style, rect)` (gen slot 29, `0x009999BA`)

1. A child with `id` already present (window slot 19) → false.
2. `GZCOM()->CreateInstance(0xC2AFA76E, 0xC2AFA76F, &bar)` (a `cGZWinFlatRect`); failure → **true**, nothing
   added.
3. Its window (`bar` slot 3): `SetID(id)`, `SetArea(rect)` (slot 55), `SetFlag(0x800, 0)`, `SetFillColor(c1)`
   (window slot 106); `bar` slot 8 (`c2 & 0xFFFF`, edge colours), slot 18 (`autoSize`), slot 12 (`style`). `c1`, `c2`
   are `WCM.GetColor(0xBB5B2EC6)` and `(0xBB5B2EC7)`, 0 without the manager. `0xBB5B2EC6` is set to (124, 144, 255)
   at start-up (application.md AP13): the "blue bar". `GetColor` returns a native (display-format) colour
   (widget-ctrlmgr.md §2); the `& 0xFFFF` keeps its low 16 bits, which is exact only in a 16-bit display format.
4. `ChildAdd(bar window)` (window slot 14). On failure: the window manager's `DestroyWindow(bar window)` (manager
   slot 23), release `bar`, return false. On success release `bar` (the child list keeps its own reference) and
   return true.

### 8.2 `AddDividerH(id, y)` (gen slot 30, `0x00999AFE`)

`AddColorBar(id, 3, 0xE, {l, y, W − gx, y + 8})` with `l = gx − 1`, or `gx + 7` with the side bar. Auto-size 3
stretches it to the right inner edge; style `0xE` draws its top, right and bottom edges. Returns slot 29's result
(the last call before `ret`; the decompile shows void).

## 9. `cGZWinFlatRect` *[compat]*

A `cGZWin` with four edge colours, outline flags and an auto-size mode. Class `0xC2AFA76E`, interface
`cIGZWinFlatRect` `0xC2AFA76F` at `+0xD8` (vtable `0x00AE2050`, 20 slots).

- **`QueryInterface`** (`0x009CD1D2`): `0xC2AFA76F` → `+0xD8` (null for null), AddRef'd, true; else `cGZWin`'s.
- **Window overrides** (vtable `0x00AE20A0`): 0, 3 `DoMessage`, 88 `GZPaint`, 103/104/106/107 (5-byte jumps to the
  `cGZWin` fill-colour functions, same behaviour), 130/131 keys, 148 destructor (`0x009CD6FC`; body `0x009CD1B2`,
  bit 0 frees).

| Offset | Meaning |
|---|---|
| `+0xDC` | auto-size mode (byte) |
| `+0xDD` | outline flags (byte) |
| `+0xE0`, `+0xE4`, `+0xE8`, `+0xEC` | edge colours 0 left, 1 top, 2 right, 3 bottom, packed `0x00RRGGBB` |

### 9.1 Outline flags and `SetOutlineFlags(f)` (slot 12, `0x009CD300`)

Bits: 1 left, 2 top, 4 right, 8 bottom (draw that edge), `0x10` raised, `0x20` sunken, `0x40` no fill (script words
in legacy-control.md §9.7).

`SetOutlineFlags(f)` stores the byte. If `f & 0x30`: read two colours from the window-control manager's slot 31
`GetColorRGB(id, &r, &g, &b)`: dark `0xA8963FAD` (default 0, 0, 0) and light `0x08963FAF` (default 255, 255, 255);
the defaults stay when the manager is absent. Raised (`0x10`, which wins over `0x20`): left and top light, right
and bottom dark. Sunken: left and top dark, right and bottom light. In both cases `flags |= 0xF`. Return true.

### 9.2 The other `cIGZWinFlatRect` slots

| Slot | Address | Behaviour |
|---|---|---|
| 3 `AsIGZWin` | `0x0095BA6F` | shared thunk |
| 4, 5, 6, 7 | `0x00999FF6`, `0x0099A001`, `0x0099A00C`, `0x0099A017` | thunks to window slots 106, 107, 103 (`SetFillColor(r, g, b)`), 104 (`GetFillColor`) |
| 8 `SetEdgeColors(native)` | `0x009CD433` | the graphics system (`RZ::GS`) slot 36 turns the native colour into `r, g, b`; then slot 9 |
| 9 `SetEdgeColors(r, g, b)` | `0x009CD46F` | all four edges `:= 0x00RRGGBB` |
| 10 `SetEdgeColor(edge, r, g, b)` | `0x009CD603` | edges 0–3; other values ignored |
| 11 `GetEdgeColor(edge, &r, &g, &b)` | `0x009CD671` | edges 0–3; other values leave the outputs |
| 12 `SetOutlineFlags(f)` | `0x009CD300` | §9.1 |
| 13 `GetOutlineFlags` | `0x009CD40A` | the byte |
| 14 `IsOutlineFlag(mask)` | `0x009CD40F` | `(flags & mask) != 0` |
| 15 `SetEdgeColors(n0, n1, n2, n3)` | `0x009CD492` | each native colour through `GS` slot 36 into its edge; nothing without the graphics system |
| 16 `GetEdgeColors(out0 … out3)` | `0x009CD563` | each edge through `GS` slot 34 `(r, g, b)` back to a native colour; nothing without the graphics system |
| 17 `AutoSize(const rect* area)` | `0x009CD7B6` | §9.3 |
| 18 `SetAutoSizeMode(byte)` | `0x009CD6E0` | stores `+0xDC` |
| 19 `GetAutoSizeMode` | `0x009CD6EC` | `+0xDC` |

### 9.3 `AutoSize(area)` (slot 17)

Mode 0 → true, nothing. Otherwise `rect := own area` (window slot 47), then by mode: 1, 2, 6 no change; 3 `r :=
area.r`; 4 `b := area.b`; 5 `l := area.l`, `r := area.r`; any other mode → **false**. Then `b := max(b, t)`,
`r := max(r, l)`, `SetArea(rect)` (window slot 55), true.

### 9.4 Painting and input

`GZPaint` (`0x009CD1FF`):
1. Unless flag `0x40`: fill the draw area with the fill colour (window slot 126; draw context slots 21 and 35).
2. If any flag other than `0x40` is set: with `x0 = l`, `y0 = t`, `x1 = r − 1`, `y1 = b − 1` of the draw area, draw
   1-pixel lines (draw context slot 21 with the edge colour, then slot 34 `(x, y, x, y)`): flag 1 left `(x0, y0)–(x0,
   y1)`, 2 top `(x0, y0)–(x1, y0)`, 4 right `(x1, y0)–(x1, y1)`, 8 bottom `(x0, y1)–(x1, y1)`, in this order.
3. Return true.

`DoMessage` (`0x009CD718`): without a parent → false. Otherwise, for types 7 … 14, add the window's left and top
(window slots 43, 44) to `d1` and `d2` (into the parent's coordinates, **in place**); then return the parent's
`DoMessage(msg)`. So a flat rect never handles anything itself: clicks on a colour bar or on the title bar
background reach the general window, which drags it (§6).

Keys (`0x009CD75C`, `0x009CD789`): `{5 or 6, vk, modifiers, 0}` to the notification target's `DoMessage`
(slot 3), the result returned. The target is not checked for null.

## 10. Quirks

1. `GetBackgroundImage(out)` (gen slot 14) AddRefs a null image (§2).
2. `SetUserData` and `RemoveUserData` have no defined return value; `AddColorBar` returns true when the flat rect
   cannot be created.
3. With flags 1 and 2 and no title bar, the move mode replaces every resize edge (§6.1), so such a window can be
   moved but not resized; the cursor still changes only with both flags (§6.3).
4. `SetGutters` lays nothing out again (§2); the next resize or flag change uses the new gutters.
5. Moving a window (same size) skips `place_controls` and the user-area recomputation (§4.1 step 2).
6. Four calls take arguments that were pushed before an argument-less getter (§4.3 step 10, §5.3 twice, §6.1).
7. `kill_self` drops its result for a modeless window (§7).
8. Esc and Alt+F4 close only with flag 4, and a modal window passes no other keys to its notification target.

## 11. Cases

1. A new window: flags `0x15`, gutters 4, blit type 2, no procedure. After `Init` on a 200 × 100 window with the
   control manager present: a title bar (ID `0x42B7C351`) at (4, 4), the background picture `0xE2B14588` with blit
   type 0, and a user area `{4, title.GetB() + 4, 196, 96}`.
2. `SetWinGenFlag(0x1000, 1)` on that window: the title bar is removed, the minimum size becomes 8 × 8 and the user
   area `{4, 4, 196, 96}`.
3. `SetWinGenFlag(0x10000, 1)` then: a flat-rect child `0x42B7C352` at `{4, 4, 12, 96}` (style `0xD`, auto-size 4)
   and the user area `{16, 4, 196, 96}`.
4. A procedure that answers `0xA2BF8AD2` with limits `{300, 200, 300, 200}` on a window with no title bar or side
   bar, gutters 4: `SetArea(0, 0, 50, 50)` gives a 308 × 208 window and the user area `{4, 4, 304, 204}`.
5. Left-down at (1, 50) on a sizeable, not moveable 200 × 100 window (gutters 4): mode 2. Moving to (11, 50)
   sets `l += 10`; the offset stays 1.
6. Esc on a window with flag 4 that is a modal dialog: `GZWinMsgPost(0xF, window, 0x42B7C353)`; the same on a
   modeless window posts `{1, window, 0, 0}` to the parent.
7. `AddDividerH(7, 40)` on a window 200 wide, gutters 4, no side bar: a flat rect `{3, 40, 196, 48}`, style `0xE`,
   auto-size 3.
8. A flat rect with flags `0x12` (raised, top): `SetOutlineFlags` turns them into `0x1F`; it paints the fill and all
   four edges, light top and left, dark right and bottom.

## 12. Function inventory

`cGZWinGen` (56): `0x0079C580` gen QI thunk, `0x00945EAC` slot 19, `0x00998F83` QI, `0x00998FB0` `SetCaption`
(base, then the title bar's slot 74), `0x00998FD7` slot 62, `0x00999004` key up, `0x00999039` left up,
`0x00999065`/`0x00999079` slots 10/11, `0x0099908D` slot 4, `0x00999093` slot 9, `0x009990B0` slot 13,
`0x009990EC` slot 12, `0x0099911A` slot 15, `0x00999131` slot 14, `0x0099914B` slot 17, `0x00999159` slot 16,
`0x00999169` slot 20, `0x00999180` slot 23, `0x0099919B` slot 26, `0x009991AE` slot 28, `0x009991C7` slot 25,
`0x009991D7` slot 27, `0x00999204` slot 31, `0x0099922F` `calc_user_area`, `0x009992AE` `calc_win_area`,
`0x0099933C` `destroy_title_bar`, `0x009993D7` `destroy_side_bar`, `0x009993F7` `kill_self`, `0x0099955D` slot 121,
`0x009995E7` `GZPaint`, `0x00999832` key down, `0x00999896` `GZOnCommand`, `0x009998C8` `SetWinProc`,
`0x009999BA` `AddColorBar`, `0x00999AFE` `AddDividerH`, `0x00999B56` `SetCursor`, `0x00999B84` the
`place_controls` callback, `0x00999BDD` `place_controls`, `0x00999C15` `calc_size_mode`, `0x00999D43` `on_layout`,
`0x00999EB9` `create_side_bar`, `0x0099A0AF` `SetArea`, `0x0099A276` `DoMessage`, `0x0099A361` left down,
`0x0099A3A7` mouse move, `0x0099A5ED` slot 21, `0x0099A643` slot 24, `0x0099A804` `Shutdown`, `0x0099A90B`
slot 18, `0x0099A96E` `create_title_bar`, `0x0099AF6E` destructor, `0x0099AFA8` `Init`, `0x0099B485` deleting
destructor, `0x0099B5BF` `SetWinGenFlag`, `0x0099B6A5` constructor.

`cGZWinFlatRect` (17): `0x0099A00C` slot 6 thunk, `0x009CD1FF` `GZPaint`, `0x009CD300` slot 12, `0x009CD40A`,
`0x009CD40F`, `0x009CD433`, `0x009CD46F`, `0x009CD492`, `0x009CD563`, `0x009CD603`, `0x009CD671` (slots 13, 14, 8,
9, 15, 16, 10, 11), `0x009CD6E0`/`0x009CD6EC` slots 18/19, `0x009CD6F1` interface QI thunk, `0x009CD6FC` deleting
destructor, `0x009CD718` `DoMessage`, `0x009CD7B6` slot 17. The slot 7 thunk `0x0099A017` is a separate
`fn:` owner, also described in §9.2.

`cGZWinGenTitleBar` (23): §13.6.

Not covered: the meaning of draw-context slots 38/39 (WGN1) and of the font slots the title bar uses (cited by
number). Graphics-system slots 34 and 36 are the RGB → native and native → RGB conversions (WGN2, answered).

## 13. `cGZWinGenTitleBar` *[compat]*

A `cGZWinFlatRect` (§9) with a caption and three system buttons. The general window creates one (§4.3) and keeps it
in `+0xF0`; it is a child window with ID `0x42B7C351`. It has no `DoMessage` of its own, so its mouse input goes to
the general window through the flat rectangle's forwarding (§9.4) and drags it (§6.1).

Window offsets as in §9; the title bar adds:

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0xD8` | `cIGZWinFlatRect` vtable, the title bar's copy `0x00ADC348` (slot 17 replaced, §13.3) | |
| `+0xF0` | `cIGZWinGenTitleBar` vtable `0x00ADC330` (IID `0x37C86704`) | |
| `+0xF4` | title-bar flags (§13.1) | 0 |
| `+0xF8` | title font (counted) | null |
| `+0xFC` | caption colour used with flag `0x1000` | 0 |
| `+0x100` | the **display caption** (a counted `cRZString`, possibly shortened) | a new empty string |
| `+0x104`, `+0x108` | caption position (x, y) in the title bar | 0, 0 |
| `+0x10C` | caption right limit: the left edge of the leftmost visible button | 0 |
| `+0x110`, `+0x114`, `+0x118` | the close, min/max and go-back buttons (`cIGZWinBtn`, counted) | null |

- **`QueryInterface`** (`0x00998E73`): `0x37C86704` → `+0xF0` (null for null), AddRef'd; anything else → the flat
  rectangle's. The `+0xF0` and `+0xD8` slot-0 thunks (`0x0099A022`, `0x00999FEB`) and the `+0xF0` AddRef and
  Release thunks (`0x0099A02D`, `0x0099A038`) adjust and forward.
- **Window overrides** (vtable `0x00ADC398`, compared with `0x00AE20A0`): 0, 1, 2 (its own reference count
  thunks), 4 `Init`, 5 `Shutdown`, 55 `SetArea`, 74 `SetCaption`, 88 `GZPaint`, 148 destructor.
- **Constructor** (`0x0099A67E`): the flat-rectangle constructor, the three vtables, the fields above; the display
  caption is a new `cRZString` (0x14 bytes from the global heap; an allocation failure stores null).
- **Destructor** (body `0x00999FB2`; deleting form `0x0099A59A`): restore the vtables, `Shutdown`, release the
  display caption, the flat-rectangle destructor body.

### 13.1 Flags and `SetTitleBarFlag(mask, on)` (`0x0099B50F`)

Bits: 1 close visible, 2 min/max visible, 4 go-back visible, `0x100` close disabled, `0x200` min/max disabled,
`0x400` go-back disabled, `0x1000` use the `+0xFC` caption colour (§13.4). The general window maps its own flags to
these (§3).

`SetTitleBarFlag` (not virtual; `on` tested as a byte):
- 1, 2, 4: if that button exists, its window gets `SetFlag(1 visible, on)` and the buttons are placed again
  (§13.5);
- `0x100`, `0x200`, `0x400`: if that button exists, `SetFlag(2 enabled, !on)`;
- then, for every mask, `flags := on ? flags | mask : flags & ~mask`. Returns true.

Nothing in the game sets `0x1000` or `+0xFC`: the general window passes only the six mapped bits.

### 13.2 `Init` (window slot 4, `0x0099AC7E`) and `Shutdown` (window slot 5, `0x0099944B`)

`Init`: already initialised → true. Without the window-control manager → **false**. Otherwise:
1. Font: manager slot 5 with `0xE2B14587`, or `0x68963C4C` when that is null; `SetTitleFont` (§13.4).
2. Image: manager slot 7 `GetPicture(0xE2B14589)` (the system-button strip). If null: a new buffer from the graphics
   system (slot 3), `Init(64, 64, …)` (buffer slot 3, with the format pair from graphics-system slot 9), and if that
   and `Lock(0x8001)` (slot 6) succeed, `Fill(0xFFFFFFFF, 0)` (slot 18) and `Unlock(0x8001)`: a white 64 × 64
   placeholder. Any failure here → release, return **false**.
3. The image holds the buttons in rows of `imageH / 4` (buffer slot 10 >> 2), each row the full image width
   (slot 9). With `rect = {0, 0, imageW, imageH / 4}`, in this order, release any old button and
   `create_and_add_sys_btn`:
   1. `0x42B7C353` into `+0x110` (close), row 0: visible = flag 1, enabled = not `0x100`;
   2. `0x42B7C355` into `+0x118` (go back), row 1: visible = flag 4, enabled = not `0x400`;
   3. `0x42B7C354` into `+0x114` (min/max), row 2: visible = flag 2, enabled = not `0x200`.

   `rect` moves down one row after each success. A failure stops the sequence: release the image and return
   **false** (the earlier buttons stay).
4. `cGZWin::Init`, release the image, return true.

The buttons are not placed here; that happens on the first size change (§13.5).

**`create_and_add_sys_btn(id, image, rect, slot)`** (`0x0099A70F`): without the manager or the image → false.
Manager slot 10 `(2, id, empty string, image, rect, 0)` creates a style-2 button (widget-button.md §1.2) into
`*slot`; null → false. Then `ChildAdd(button window)`. On success the button's notification target becomes the
title bar's **parent** (window slot 86 with `+0x48`), so its press reaches the general window's `GZOnCommand`
(§7), and `SetBtnFlag(4, 0)` (no caption); true. On failure the button is released but **`*slot` keeps the
pointer** (a later `Shutdown` releases it again); false.

`Shutdown`: initialised only. Release the three buttons (cleared), drop the display caption (the field becomes
null), hide (window slot 70), `SetTitleFont(null)`, `cGZWin::Shutdown`. Returns true.

### 13.3 Size (`GetMinSize` `0x00998F1D`, `AutoSize` `0x009994BF`, `SetArea` `0x0099B4B8`)

- **`GetMinSize(&w, &h)`** (not virtual): `w = 128`; `h = 8`, raised to the font's slot 37 `(0, 0)` + 8 (the line
  height) and to the close button's height + 2. Returns true.
- **`AutoSize(area)`** (flat-rect slot 17 of the title bar): without a font → **false**, nothing. Otherwise
  `GetMinSize`; the flat rectangle's `AutoSize(area)` (mode 5: full inner width); if `H` is below the minimum height,
  `SetH(min)` (window slot 52); caption position `x := 4`, `y := (H − line height) / 2`. True.
- **`SetArea(l, t, r, b)`** (window slot 55): `cGZWin::SetArea`, then `place_controls` (§13.5) only when the size
  changed.

### 13.4 Caption, font and painting

- **`SetTitleFont(font)`** (`0x00998EDC`): if different: AddRef the new one, release the old one, store, then
  `InvalidateSelfAndParents` (window slot 92). True.
- **`SetCaption(caption)`** (window slot 74, `0x0099B4A1`): `cGZWin::SetCaption`, then `calc_display_caption`.
- **`calc_display_caption`** (`0x0099B0EB`):
  1. The display caption becomes a copy of the caption (string slot 14 with window slot 73).
  2. Without a font: clear it, return false.
  3. With 0 or 1 characters: keep it, true.
  4. `avail := +0x10C − +0x104`. If `avail` ≤ 0, or the font fits no character of it into `avail` pixels (font
     slot 42 `(text, n, avail, 0)` returns 0): clear it, true. If all `n` fit: keep it, true.
  5. Otherwise shorten: the ellipsis is the manager's string `0x5377BE31` (manager slot 35), or `"..."` without it;
     `e :=` its width (font slot 48). If `e ≥ avail`: clear the caption. Else keep the characters that fit in
     `avail − e` (font slot 42), erase the rest, `RTrim`, append the ellipsis. True.
- **`GZPaint`** (window slot 88, `0x0099A043`): the flat rectangle's `GZPaint` (fill and edges); then, with a font,
  font slot 51 `(buffer to draw to, drawArea.l + x, drawArea.t + y, text, length, colour, 0x8000000, 0, 0)` with the
  display caption, where `colour` is `+0xFC` with flag `0x1000` and `0x8000000` otherwise *[inferred: "the font's
  own colour"]*. True.

### 13.5 `place_controls` (`0x0099B285`)

`x := W − 4`. For the close, go-back and min/max buttons in this order, when the button exists and its window is
visible (window slot 67 with 1): `x −= its width`; `MoveTo(x, (H − its height) / 2)` (window slot 56; the `y` is
pushed before the argument-less width getter and becomes the second argument); move it to the back
(`SetFlag(0x800, 1)`, `ChildToBack`, `SetFlag(0x800, 0)`); `x :=` its left edge. So the buttons line up from the
right edge, 4 pixels in, close outermost.

Then `+0x10C := x`; if `x` > 0 and it changed, `calc_display_caption`. True.

### 13.6 The `cIGZWinGenTitleBar` slots and the inventory

Vtable `0x00ADC330`, 6 slots: 0–2 the thunks above; 3 `GetDisplayCaption(out)` (`0x00999542`: the display caption,
AddRef'd, true; null → false); 4 `GetMargins(l, t, r, b)` (`0x00998EAA`: 4, 4, 4, 4); 5 `GetFontAs(iid, out)`
(`0x00998ECA`: without a font false, else the font's `QueryInterface`).

Owner functions (23): `0x00998E73` QI, `0x00998EAA`, `0x00998ECA`, `0x00998EDC` `SetTitleFont`, `0x00998F1D`
`GetMinSize`, `0x0099944B` `Shutdown`, `0x009994BF` `AutoSize`, `0x00999542`, `0x00999FB2` destructor body,
`0x00999FEB`/`0x0099A022` QI thunks, `0x0099A02D`/`0x0099A038` AddRef/Release thunks, `0x0099A043` `GZPaint`,
`0x0099A59A` deleting destructor, `0x0099A67E` constructor, `0x0099A70F` `create_and_add_sys_btn`, `0x0099AC7E`
`Init`, `0x0099B0EB` `calc_display_caption`, `0x0099B285` `place_controls`, `0x0099B4A1` `SetCaption`,
`0x0099B4B8` `SetArea`, `0x0099B50F` `SetTitleBarFlag`.

### 13.7 Title-bar quirks and cases

Quirks:
1. A title bar without a font ignores `AutoSize`, so it keeps the size it was created with.
2. A failed `ChildAdd` of a system button leaves a released pointer in its field.
3. Flag `0x1000` and the `+0xFC` colour are unreachable from the general window.
4. The caption is cleared, not shortened, when even the ellipsis does not fit.

Cases:
1. A strip image 60 × 64: rows of 16; close uses `{0, 0, 60, 16}`, go back `{0, 16, 60, 32}`, min/max
   `{0, 32, 60, 48}`.
2. A 200-wide title bar with only the close button visible, 14 × 14, H = 20: the button at (182, 3), and the
   caption limit 182.
3. Caption "Budget", avail 20, the font fits 3 characters in 20 and the ellipsis is 9 wide: the display caption is
   the characters that fit in 11, trimmed, + "...".
