# Widget toolkit: the `cGZWin` base window (behaviour spec)

Status: first slice (lane re-widgets4, 2026-10-21). Covers the **base window object `cGZWin`**: identity, object
layout, construction and teardown, the flag word, the child list and z-order, coordinates and hit testing, the
message dispatcher and message filters, focus/notification target, and the show/hide fade. Everything here is
measured from `bin/SimCity4.exe` (GOG 1.1.641) unless tagged *[inferred]*. Widgets (`cGZWinBtn`, `cGZWinListBox`, …),
`cGZWinGen`, `cGZLegacyControl`, the UI-script interpreter, graphs and HTML are **not** specified here yet (§12).

Related: `ui-windows.md` §2 (the `cIGZWin` slots the game layer calls, `ChildDelete` forward), §14.1 (toolkit class
inventory); `modal-dialogs.md` §2 (window manager, modal loop). Full slot tables: `docs/abi/interfaces.tsv`
(`cIGZWin`, 148 rows; `cIGZWinMgr`). This spec does not repeat those tables; it gives the behaviour behind them.

Evidence note: REA (`procedure_pseudo_code`) was unavailable for this lane (no binary open in the shared session);
everything below comes from `decomp/cGZWin.c`, `tools/re_query.py asm` and raw image reads, and every numeric claim
cites an instruction address.

## 1. Identity

| Item | Value |
|---|---|
| Class | `cGZWin` (framework base window; abstract in practice, always a base of `cGZWinGen`, widgets, game windows) |
| Interface | `cIGZWin`, IID `0x22BA0121` (ui-windows.md §2.1) |
| Own vtable | `0x00ADC8D8` (written by both constructors and by the destructor). Not yet in `docs/abi/vtables.tsv` (§11 WT1) |
| Constructors | `0x0099D938` (no arguments: window manager taken from `RZ::WM()`), `0x0099DA5A` (`winMgr`, `keyboard` explicit) |
| Destructor | `0x0099E1A2` |
| Functions | 167 named `cGZWin::*` in `re/names_windows.tsv` |

Virtual-slot numbers below are `cIGZWin` slots (byte offset = slot × 4). Slots 148–151 exist only on concrete
vtables (for example `cGZWinGen`, 152 slots); slot **149** is the internal `is_point_transparent` (`0x0099BBBE`).

## 2. Object layout *[compat]*

Offsets of the `cGZWin` part of the object. Initial values are the constructor's (`0x0099D938`).

| Offset | Size | Meaning | Initial | Evidence |
|---|---|---|---|---|
| `+0x00` | 4 | vtable | `0x00ADC8D8` | ctor |
| `+0x04` | 4 | window manager (`cIGZWinMgr*`) | `RZ::WM()` | ctor; `SetWindowManager` `0x0099CA0B` writes it |
| `+0x08` | 4 | keyboard object | manager slot 38 result | ctor; `SetKeyboard` `0x0099BF01` |
| `+0x0C` | 4 | (unnamed) | 0 | ctor |
| `+0x10` | 4 | window ID (`GetID`/`SetID`) | 0 | `0x0099BE66`, `0x0099BE5C` |
| `+0x14..+0x20` | 16 | **absolute (screen) rectangle** L,T,R,B | 0 | `CalcAbsoluteArea` `0x0099DCE4` |
| `+0x24..+0x30` | 16 | rectangle (draw area, *[inferred]*) | 0 | ctor |
| `+0x44` | 4 | pointer to the child-list block (§4.1) | new block | ctor |
| `+0x48` | 4 | parent window | null | `SetParentWin` `0x0099B7F2` |
| `+0x4C` | 4 | notification target (also the focus fallback, §6) | null | `SetNotificationTarget` `0x0099BE42` |
| `+0x50` | 0x14 | caption (`cRZString`); `GetCaption` returns its address | empty | `0x0099BE50` |
| `+0x64` | 4 | private buffer | null | `GetPrivateBuffer` `0x009D419D` |
| `+0x68` | 4 | buffer to draw to | null | `GetBufferToDrawTo` `0x0099BEFD` |
| `+0x6C` | 4 | draw context | 0 | `GetDrawContext` `0x0099BEF9` |
| `+0x70` | 1 | (byte) | 1 | ctor |
| `+0x71` | 1 | (byte) | 0 | ctor |
| `+0x78` | 4 | keyboard accelerator | null | `SetKeyboardAccelerator` `0x0099B955`, `DoMessage` |
| `+0x7C` | — | parameter map (key → variant), `GetParam`/`SetParam` | empty | `Shutdown` releases each value |
| `+0x88`, `+0x8C` | 4+4 | two message-filter lists | empty | ctor, `Shutdown`, `DoMessage` (`+0x88`) |
| `+0x90` | 1 | "filters are being run" flag | 0 | `DoMessage` |
| `+0x94` | 4 | fade-in period (clock ticks) | **500** | ctor; `Set/GetFadeEffectPeriod` `0x0099C25F`/`0x0099C276` |
| `+0x98` | 4 | second fade period | **500** | same; never read by the show/hide paths (§8) |
| `+0x9C` | 4 | key equivalent | 0 | `0x0099BE80`/`0x0099BE87` |
| `+0xA8..+0xB4` | 16 | **area** L,T,R,B in parent coordinates | 0 | `GetR` `0x0099BC5A` reads `+0xB0`; `GetW` `0x0099C81B` = `+0xB0 − +0xA8` |
| `+0xB8..+0xC4` | 16 | (rectangle, zero) | 0 | ctor |
| `+0xC8` | 4 | **flag word** (§3) | **`0x8903`** | ctor; `GetFlag` `0x0099BDBB` |
| `+0xCC` | 4 | instance ID | 0 | `0x0099BE79`/`0x0099BE6A` |
| `+0xD0` | 4 | shade colour (4 bytes) | `FF FF FF FF` | `GetShadeColor` `0x0099CA9E` |
| `+0xD4` | 4 | fill colour (3 bytes used by `GetFillColor` `0x0099BFC9`) | 0 | ctor |

The `cGZWin` part ends at `+0xD8`. The constructor's field list covers everything above except `+0x34..+0x40` and
`+0xA0..+0xA4`, which it leaves untouched (open, WT2).

## 3. The flag word (`+0xC8`) *[compat]*

`GetFlag(mask)` (slot 67, `0x0099BDBB`) returns `(flags & mask) != 0`. Default `0x8903` = bits `0x1`, `0x2`, `0x100`,
`0x800`, `0x8000`.

| Bit | Name used here | Default | What reads or reacts to it |
|---|---|---|---|
| `0x1` | visible | set | `IsVisible` (slot 71) = `GetFlag(1)`; hit testing skips invisible children (§5.3) |
| `0x2` | enabled | set | `IsEnabled` (slot 72) = `GetFlag(2)` |
| `0x20` | fade on show | clear | `ShowWindow`/`HideWindow`/`SetFlag` start a fade (§8) |
| `0x800` | z-order movable | set | `ChildToFront`/`ChildToBack` refuse to move a child without it (§4.3) |
| `0x4000` | initialised | clear | `get_init_flag` `0x0099BC31` = `GetFlag(0x4000)`; set by `Init`, cleared by `Shutdown` |
| `0x10000` | has private buffer | clear | buffer created/destroyed on visibility and size changes (§3.1) |
| `0x20000` | private buffer transparency | clear | §3.1 |
| `0x80000` | per-pixel hit test | clear | hit testing asks `is_point_transparent` (§5.2) |
| `0x400000` | (cleared on any visibility change) | clear | `SetFlag` `0x0099DCD2` |
| `0x8000000` | (triggers re-propagation of draw buffer/area) | clear | §3.1 |

Bits `0x100` and `0x8000` are set by default; nothing in `cGZWin` itself tests them (open, WT3).

### 3.1 `SetFlag(mask, on)` (slot 68, `0x0099DB6B`)

First sets (`on` ≠ 0, tested as a **byte**) or clears every bit of `mask` in the flag word. Then, **only if `mask`
is exactly one of these values**, runs a side effect:

| `mask` | Side effect |
|---|---|
| `0x1` (visible) | If bit `0x10000` is set: `PrivateBuffer(…)` (slot 100) is re-evaluated; otherwise the private buffers of the children are updated with the new state. If hiding and this window holds the manager's focus (manager slot 36 `GZGetFocus` returns this window), focus moves to the notification target (`SetFocusTo(+0x4C)`, §6). If there is a parent, the parent's `InvalidateSelfAndParents` (slot 92). Finally clears bit `0x400000`. |
| `0x2` (enabled) | When disabling: if this window holds mouse capture, release it; if it holds focus, focus moves to `+0x4C`. Enabling has no side effect. |
| `0x20` | Starts a fade: on → `set_fade_state(alpha 0, period +0x94)`; off → `set_fade_state(alpha 0xFF, period 0)` (§8). |
| `0x10000` | `PrivateBuffer(on && IsVisible())`: creates the buffer only for a visible window, destroys it otherwise. |
| `0x20000` | Only if bit `0x10000` is set: apply transparency to the private buffer, then `InvalidateSelfAndParents`. |
| `0x8000000` | `SetBufferToDrawToRecursive` (slot 96) then `SetAreaToDrawToRecursive` (slot 98). |

A combined mask (for example `0x3`) changes the bits but runs **no** side effect.

**Return value: the whole new flag word in EAX** (`0x0099DCD9` loads `[ESI+0xC8]` before `RET 8`), not 0/1. Callers
that test `AL` see the low byte of the flags. (ui-windows.md §2.2 lists the return as "0/1 in EAX"; that is wrong
for this target — WT4 records the correction.)

### 3.2 `ShowWindow` / `HideWindow` (slots 69/70, `0x0099D1AA`/`0x0099D1EA`)

`ShowWindow`: if not already visible, `SetFlag(1, true)`; then, if bit `0x20` is set, `set_fade_state(0, +0x94)`.
`HideWindow`: if visible, `SetFlag(1, false)`; then, if bit `0x20` is set, `set_fade_state(0xFF, 0)`. Both are no-ops
when the window is already in the requested state (no fade restart).

## 4. Children and z-order *[compat]*

### 4.1 The child list is copy-on-write

`+0x44` points at a block `{list head, reader count}` holding `cIGZWin*` entries. Every iteration increments the
reader count first (`lock_child_list_read` `0x0099B94A`) and decrements it at the end (`unlock_child_list_read`
`0x0099D3D2`). A mutation calls `lock_child_list_write` (`0x0099E08F`): **if the reader count is non-zero**, it
copies the list into a new block, AddRefs every child in the copy, and installs the copy at `+0x44`; the mutation
then edits the copy. When the last reader of a block that is no longer installed unlocks, that block Releases each
child it holds and is freed.

Consequence: a handler that adds, removes or reorders children while its parent is iterating them (hit testing,
painting, `CalcAbsoluteArea`) is safe, and the running iteration keeps seeing the old order and the old set.

**List order is front-to-back**: the head is the frontmost child. New children are inserted at the head
(`push_front`), so the most recently added child is on top; hit testing walks from the head (§5.3).

### 4.2 Adding and removing

`ChildAdd(child)` (slot 14) and `ChildRemove(child)` (slot 15) are one-instruction forwards to
`child_add_absolute` `0x0099E207` and `child_remove_absolute` `0x0099E2BD`.

**Add**, all steps in order:
1. Ask the manager whether `child` is a live registered window (manager slot 24 `IsWindowValid`). If not: return false.
2. If `child` is already in the list: return false (no change).
3. `child->SetParentWin(this)` (slot 12). If that returns false: return false.
4. `child->Init()` (slot 4). If it returns false: `child->Shutdown()` (slot 5), `child->SetParentWin(null)`, return false.
5. Insert `child` at the **front** of the list; AddRef it; `child->CalcAbsoluteArea()` (slot 90);
   `child->SetBufferToDrawToRecursive()` (slot 96). Return true.

**Remove**: look the child up (manager validity is evaluated first but does not stop the lookup); if it is not in the
list return false. Otherwise, **only if the child is a valid window**, manager slot 22 is told about it (it drops the
manager's focus/capture references, *[inferred]* from `0x009DB8A4`), `child->SetParentWin(null)` and the list's
reference is Released; then the entry is erased. Return true. Remove does **not** shut the child down or destroy it.

`SetParentWin(p)` (slot 12, `0x0099B7F2`): if `p` equals the current parent, return true. If there is a current
parent **and** `p` is non-null, the current parent must first agree to `ChildRemove(this)`; if it refuses, return false
and change nothing; if the notification target was the old parent it is cleared. Then: if the notification target is
null it becomes `p`; the parent becomes `p`; return true. So the notification target defaults to the first parent and
follows re-parenting only when it was the old parent. Setting the parent to null never clears the notification target.

**Delete** (`ChildDeleteAbsolute`, slot 17, `0x0099E344`): if the child is valid, AddRef it and call its
`Shutdown`; remove it as above; hand it to the manager's `DestroyWindow` (slot 23, deferred while the manager is busy,
modal-dialogs.md); Release the extra reference. Returns true only if both the removal and the shutdown returned true.
`ChildDelete` (slot 16) is the forward in ui-windows.md §2.3.

`ChildDeleteAll` (slot 18, `0x0099DD6F`): repeatedly takes the **current front child** and passes it to the manager's
`DestroyWindow`, until the list is empty; returns true only if every destroy returned true. *[inferred]* the loop
terminates because `DestroyWindow` removes the child from this list; if it did not, the loop would not end (open,
WT5).

### 4.3 Reordering

| Call | Slot | Behaviour |
|---|---|---|
| `ChildToFront(child)` | 25/26 (`0x0099E3A8`) | If `child` is valid and in the list: **first, if this window has a parent, `parent->ChildToFront(this)`** (so the whole ancestor chain is raised). Then, if `child` has flag `0x800`: move it to the front (unless already there) and return true; without `0x800` return false (the ancestors were still raised). |
| `ChildToBack(child)` | 27 (`0x0099E457`) | If valid, present (`ChildExists`, slot 20) and flag `0x800`: move it to the back, return true. No ancestor effect. |
| `PullToFront()` | 23 (`0x0099B856`) | `parent->ChildToFront(this)`; false (low byte 0) without a parent. |
| `SendToBack()` | 24 (`0x0099B86B`) | `parent->ChildToBack(this)`; false without a parent. |

Reordering never invalidates or repaints by itself.

## 5. Geometry and hit testing *[compat]*

### 5.1 Two rectangles

The **area** (`+0xA8..+0xB4`) is L,T,R,B in the parent's coordinates; width = R − L, height = B − T. The **absolute
area** (`+0x14..+0x20`) is the same rectangle in screen coordinates.

`CalcAbsoluteArea` (slot 90, `0x0099DCE4`): start from the area; for each ancestor (walking `GetParentWin`, slot 11),
add that ancestor's `GetL` (slot 43) to L and R and its `GetT` (slot 44) to T and B (each ancestor's area relative to
its own parent, so the sum is the screen position with the root's area taken as screen coordinates). Store the
result, then recurse into every child.

`SetArea(l, t, r, b)` (slot 54, `0x0099C837`): stores the four values; if the size changed and bit `0x10000` is set,
re-creates the private buffer (`PrivateBuffer(1)`); then `CalcAbsoluteArea` and `SetAreaToDrawToRecursive`. It does
not invalidate. `GZWinMoveTo(x, y)` (slot 56) = `SetArea(x, y, x+W, y+H)`; `GZWinOffset(dx, dy)` (slot 57) adds
`dx`/`dy` to all four edges via `SetArea` (slot 55 on the vtable — a subclass override of `SetArea`, e.g.
`cGZWinGen::SetArea`, runs).

`WindowToScreenCoordinates(&x, &y)` (slot 60, `0x0099BD5E`) adds absolute L/T. `ScreenToWindowCoordinates` (slot 59,
`0x0099BD73`) subtracts them (it converts (0,0) to screen through slot 60 and subtracts).

### 5.2 Point tests

Rectangle containment is **half-open**: `L ≤ x < R` and `T ≤ y < B` (helper `0x00664C60`).

| Call | Slot | Point in | Test |
|---|---|---|---|
| `IsPointInWindowWindowCoordinates` | 121 (`0x0099C8F5`) | window | `0 ≤ x < W`, `0 ≤ y < H`; then, if flag `0x80000` and inside, **not** `is_point_transparent(x, y)` |
| `IsPointInWindowParentCoordinates` | 122 (`0x0099C960`) | parent | area contains point. **No** transparency test even with `0x80000` |
| `IsPointInWindowScreenCoordinates` | 62 (`0x0099C97C`) | screen | absolute area contains point; then, if `0x80000`, convert to window coordinates (slot 59) and require not transparent |

`is_point_transparent` (slot 149, `0x0099BBBE`): false unless there is a private buffer that reports itself usable;
then it locks it (flag `0x800` on the buffer), asks the buffer whether the pixel is transparent, unlocks, and returns
that answer. A window without a private buffer is therefore fully opaque to hit tests even with `0x80000`.

### 5.3 Finding the window under a point

`GetChildWindowFromPoint(x, y)` (slot 39, `0x0099DF28`), point in **screen** coordinates: walk the children front to
back; for each **visible** child, call the child's `GetChildWindowFromPoint(x, y)` with the same screen point; the
first non-null answer wins. If no child answers, return this window if `IsPointInWindowScreenCoordinates(x, y)`
(slot 62), else null.

Consequences an implementation must keep:
- The parent's own bounds are **not** checked before descending: a child that extends outside its parent's area is
  still hit there.
- Enabled state is ignored (disabled windows are returned).
- Only the child's visibility is checked, not this window's own.

`GetChildWindowFromID(id)` (slot 34, `0x0099DE74`): first direct child, front to back, whose `GetID` (slot 63) equals
`id`; null if none. `GetChildWindowFromIDRecursive(id)` (slot 35, `0x0099DEC4`): for each child front to back, recurse;
the first non-null answer wins; only if no descendant matches is this window itself returned when its own ID equals
`id`. So **a descendant with the same ID beats the window itself**, and the search is depth-first, front-most first.

## 6. Focus, capture and the notification target *[compat]*

Focus and capture live in the window manager. `SetFocus()` (`0x0099BDE4`) = manager slot 37 with this window;
`GetFocus` = manager slot 36. `SetFocusTo(w)` (`0x0099C9DF`) sets manager focus to `w` only if `w` is a valid window,
returning that validity. The **notification target** (`+0x4C`) is where focus goes when the focused window is hidden
or disabled (§3.1), and defaults to the first parent (§4.2).

## 7. Message dispatch *[compat]*

### 7.1 Message record

`DoMessage(msg)` (slot 3, base `0x0099CCF0`) takes a pointer to a record whose first dword is the **type**.
Mouse types read `x` as the signed 16-bit value at `+4`, `y` as the signed 16-bit value at `+8`, and a 16-bit
button/modifier word at `+0xE` (zero-extended).

### 7.2 Filters first

Before dispatching, `DoMessage` saves the `+0x90` flag, sets it to 1, and calls each filter in the `+0x88` list, in
list order, as `filter->slot3(this, msg)`. The first filter that returns true stops the walk. The flag is restored.
If any filter returned true, `DoMessage` returns true without dispatching. `AddMessageFilter` (slot 116) appends to
the end of the list and AddRefs the filter; it always returns true.

### 7.3 Dispatch table (jump table at `0x0099CEF9`, 20 entries, type 1–20)

| Type | Handler slot | Handler | Arguments passed |
|---|---|---|---|
| 1 | 16 | `ChildDelete` | dword `+4` (the child) |
| 3 | 143 | `GZOnCommand` | dwords `+4`, `+8` |
| 4 | 129 | `GZOnCharacter` | low byte of `+4` |
| 5 | 130 | `GZOnKeyDown` | dwords `+4`, `+8` |
| 6 | 131 | `GZOnKeyUp` | dwords `+4`, `+8` |
| 7 | 134 | `GZOnMouseDownL` | x, y, buttons |
| 8 | 135 | `GZOnMouseDownR` | x, y, buttons |
| 10 | 136 | `GZOnMouseUpL` | x, y, buttons |
| 11 | 137 | `GZOnMouseUpR` | x, y, buttons |
| 13 | 138 | `GZOnMouseMove` | x, y, buttons |
| 14 | 139 | `GZOnMouseWheel` | **full dwords** `+4`, `+8` (not sign-extended shorts), buttons word `+0xE`, delta = signed 16-bit at `+0xC` |
| 16 | 133 | `GZOnKillFocus` | dword `+4` |
| 17 | 132 | `GZOnSetFocus` | dword `+4` |
| 18 | 140 | `GZOnCaptureChanged` | dwords `+4`, `+8` |
| 19 | 141 | `GZOnMouseEnter` | dword `+4` |
| 20 | 142 | `GZOnMouseExit` | dword `+4` |
| 2, 9, 12, 15, other | — | none | returns false |

Handler names for slots 129, 133, 135, 137, 140–142 come from widget overrides in `docs/abi/vtables.tsv`
(`cGZWinLineInput::GZOnCharacter`, `cGZWinBtn::GZOnMouseDownR`, …); the `cGZWin` defaults at those slots are shared
"return false" stubs.

**Key-down/up and the accelerator**: if no keyboard accelerator is set (`+0x78` null) the key handler's result is
returned directly. If one is set and the key handler returns false, `AccelerateKeyboardMsg(msg)` (slot 77) is called
and its result returned; if the key handler returned true, true is returned.

The return value is a byte (`AL`); for unhandled types the upper bytes of EAX hold `type − 1` with the low byte
cleared.

### 7.4 Send and post

`SendMsg(target, msg)` (slot 145, `0x0099BFEC`): if the manager says `target` is valid, returns
`target->DoMessage(msg)`; otherwise false. The five-argument `SendMsg` (slot 144, `0x0099CA2F`) builds a record
`{type, a, b, c, d}` on the stack and calls slot 145 with it. `PostMsg(a, b)` (slot 147, `0x0099C011`) passes both
arguments to manager slot 15 (`PostMessageToTarget(target, msg)`, the manager's deferred queue, modal-dialogs.md) and
always returns true.

## 8. Lifecycle and the fade effect *[compat]*

### 8.1 Construction, `Init`, `Shutdown`, destruction

Construction (§2) registers the window with the manager (manager slot 20). Registration *[compat, measured in the
manager `0x009DC586`]*: if the window is not yet in the manager's set it is added, and if its instance ID is 0 it is
given the manager's next serial (`SetInstanceID(counter++)`, counter at manager `+0x194`).

`Init()` (slot 4, `0x0099C2C3`): once only (guarded by flag `0x4000`): sets `0x4000`; if bits `0x10000` and `0x1` are
both set, creates the private buffer; then `SetBufferToDrawToRecursive`. Always returns true.

`Shutdown()` (slot 5, `0x0099D2FE`): once only (if `0x4000` set): clears `0x4000`; Releases and clears the keyboard
accelerator; empties both filter lists; calls `HideWindow` (slot 70), `ChildDeleteAll` (slot 18), `PrivateBuffer(0)`;
Releases every parameter value and empties the parameter map. Always returns true.

Destructor (`0x0099E1A2`): `ChildDeleteAll`, `Shutdown`, then manager slot 22 (drop focus/capture references) and
manager slot 21 (unregister), then frees the child-list block and members.

### 8.2 Fade-in

The fade works only on a window with both `0x20` and `0x10000` (it needs a private buffer). `set_fade_state(alpha,
period)` (`0x0099CAAD`) stores three window parameters — start alpha (key `0xE6E998FB`), start time `clock()` (key
`0xE6E998FC`), period (key `0xE6E998FD`) — and immediately applies colour `(0xFF, 0xFF, 0xFF, alpha)` to the private
buffer (through its interface `0x86D72B57`, slot 3).

`fade_advance` (`0x0099C026`), called only from `PlotComposite` (`0x0099E669`), so once per composite of the window,
and doing nothing while the period parameter is 0, computes in x87 double:

    a = start_alpha + (elapsed × 255.0) / period

where `elapsed = clock() − start_time` taken as **unsigned** (`2^32` added when negative, `0x00A80AB8`), `255.0` at
`0x00A90B88`, `period` divided as a 32-bit integer (`FIDIV`, `0x0099C114`), and `start_alpha` added last. If `a < 0`
or `a ≥ 256.0` (`0x00AA20C8`) the fade ends: the period parameter is set to 0 and the alpha becomes 0xFF (or 0 when
`a < 0`). Otherwise the alpha is `a` truncated to an integer (`0x009EEF04`). The alpha is then applied as above.

Worked example: period 500 (the default `+0x94`), `CLOCKS_PER_SEC` 1000 *[inferred]*: show at t=0 → alpha 0; t=100 →
51; t=250 → 127 (127.5 truncated); t=502 → 256.02 ≥ 256 → fade ends, alpha 0xFF.

Hide uses `set_fade_state(0xFF, 0)`: alpha 0xFF at once and period 0, so **there is no fade-out**; `+0x98` is never
used by `ShowWindow`, `HideWindow` or `SetFlag`.

## 9. Quirks (keep for compatibility)

1. `SetFlag` returns the whole flag word, not a bool (§3.1).
2. `SetFlag` side effects run only for the exact single-bit masks of §3.1; combined masks change bits silently.
3. `ChildToFront` raises the ancestors even when it refuses to move the child (§4.3).
4. Hit testing descends into children without checking the parent's bounds and ignores enabled state (§5.3).
5. `IsPointInWindowParentCoordinates` skips the per-pixel test the other two point tests apply (§5.2).
6. `GetChildWindowFromIDRecursive` prefers a descendant over the window itself (§5.3).
7. The wheel message passes x/y as full dwords, other mouse messages as sign-extended 16-bit values (§7.3).
8. No fade-out; the second fade period is dead state (§8.2).

## 10. Conformance tests

1. New window: flags `0x8903`, fade periods 500/500, shade `FFFFFFFF`, instance ID assigned by the manager's registration counter (start value open, WT6).
2. Add A then B to P: hit test at a point both cover returns B; `ChildToBack(B)` → returns A.
3. `ChildAdd` of a child already present returns false and leaves the order unchanged.
4. Child whose `Init` returns false: `ChildAdd` false, child's parent null afterwards.
5. `SetFlag(0x3, false)` on the focused window: both bits cleared, focus unchanged (no side effect).
6. `SetFlag(0x1, false)` on the focused window: focus moves to its notification target.
7. Child area extending outside parent: point outside the parent but inside the child returns the child.
8. Two windows with ID 7, parent and child: `GetChildWindowFromIDRecursive(7)` on the parent returns the child.
9. Fade: `0x20|0x10000` window shown at clock 0, period 500: alpha at clocks 100/250/502 = 51/127/255 and the period
   parameter is 0 after 502.
10. Message type 9 or 12 returns false without calling any handler; a filter returning true suppresses dispatch.

## 11. Open questions

Filed in `docs/specs/questions/widget-toolkit.md`: WT1 (own vtable rows), WT2 (unset fields), WT3 (bits `0x100`,
`0x8000`), WT4 (ui-windows §2.2 `SetFlag` return), WT5 (`ChildDeleteAll` termination), WT6 (instance-ID counter start).

## 12. Coverage of owner patterns (honest)

| Owner pattern (`tools/coverage-map.tsv`) | State after this slice |
|---|---|
| `cGZWin` (the class itself; matched by `cGZWin*`) | **specified** for layout, flags, children, z-order, geometry, hit testing, dispatch, filters, lifecycle, fade. Not yet: painting (`Plot`, `PlotComposite`, `PlotPresent`, private-buffer creation), cursors (`ProcessCursorMessage`, `UpdateCursor`), key equivalents/accelerators, parameter enumeration, `SortChildren`, `CenterWindowInRect`/`FitRectToWindow`, colour conversion |
| `cGZWin*` (everything else: `cGZWinGen`, widgets, managers) | not specified |
| `cGZLegacyControl`, `cGZUIScript*`, `cHTML*`, `cGZGraph`, `cGZ*Graph`, `fn:unscript`, `fn:script_tag_*`, `fn:GetGZWinFont`, `cSC4ScriptedWindowCOMDirector`, `cRZWinKeyAccelerator*`, `cRZCursorManager`, `cGZClipboardManager*`, `cGZCheatCodeManager` | not specified |
