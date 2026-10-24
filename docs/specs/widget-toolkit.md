# Widget toolkit: the `cGZWin` base window (behaviour spec)

Status: first slice (lane re-widgets4, 2026-10-21). Covers the **base window object `cGZWin`**: identity, object
layout, construction and teardown, the flag word, the child list and z-order, coordinates and hit testing, the
message dispatcher and message filters, focus/notification target, and the show/hide fade, and the keyboard accelerator table `cRZWinKeyAccelerator` (§13). Everything here is
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
| `+0x0C` | 4 | reference count (section 8.1; `AddRef` `0x0094C47C`, §18) | **0** | ctor; `0x0094C47C`, `0x0099B7A3` |
| `+0x10` | 4 | window ID (`GetID`/`SetID`) | 0 | `0x0099BE66`, `0x0099BE5C` |
| `+0x14..+0x20` | 16 | **absolute (screen) rectangle** L,T,R,B | 0 | `CalcAbsoluteArea` `0x0099DCE4` |
| `+0x24..+0x30` | 16 | **draw area** (§17.1) | 0 | ctor; `SetAreaToDrawTo` `0x0099CF6A` |
| `+0x34..+0x40` | 16 | **draw origin rectangle** (§17.1); not initialised by the constructor | — | `SetAreaToDrawTo` |
| `+0x44` | 4 | pointer to the child-list block (§4.1) | new block | ctor |
| `+0x48` | 4 | parent window | null | `SetParentWin` `0x0099B7F2` |
| `+0x4C` | 4 | notification target (also the focus fallback, §6) | null | `SetNotificationTarget` `0x0099BE42` |
| `+0x50` | 0x14 | caption (`cRZString`); `GetCaption` returns its address | empty | `0x0099BE50` |
| `+0x64` | 4 | private buffer | null | `GetPrivateBuffer` `0x009D419D` |
| `+0x68` | 4 | buffer to draw to | null | `GetBufferToDrawTo` `0x0099BEFD` |
| `+0x6C` | 4 | draw context | 0 | `GetDrawContext` `0x0099BEF9` |
| `+0x70` | 1 | (byte) | 1 | ctor |
| `+0x71` | 1 | (byte) | 0 | ctor |
| `+0x74` | 4 | cursor object, fetched lazily by `UpdateCursor` (§15.2) | null *[inferred]* | `0x0099C46B` |
| `+0x78` | 4 | keyboard accelerator | null | `SetKeyboardAccelerator` `0x0099B955`, `DoMessage` |
| `+0x7C` | — | parameter map (key → variant), `GetParam`/`SetParam` | empty | `Shutdown` releases each value |
| `+0x88`, `+0x8C` | 4+4 | two message-filter lists | empty | ctor, `Shutdown`, `DoMessage` (`+0x88`) |
| `+0x90` | 1 | "filters are being run" flag | 0 | `DoMessage` |
| `+0x94` | 4 | fade-in period (clock ticks) | **500** | ctor; `Set/GetFadeEffectPeriod` `0x0099C25F`/`0x0099C276` |
| `+0x98` | 4 | second fade period | **500** | same; never read by the show/hide paths (§8) |
| `+0x9C` | 4 | key equivalent | 0 | `0x0099BE80`/`0x0099BE87` |
| `+0xA0`, `+0xA4` | 4+4 | mouse-down offset (x, y), written only by `set_mouse_down_offset` (§18) | not set | `0x0099BDA4` |
| `+0xA8..+0xB4` | 16 | **area** L,T,R,B in parent coordinates | 0 | `GetR` `0x0099BC5A` reads `+0xB0`; `GetW` `0x0099C81B` = `+0xB0 − +0xA8` |
| `+0xB8..+0xC4` | 16 | **blit rectangle** (§17.1) | 0 | ctor; `recalc_blt_area` `0x0099C7BA` |
| `+0xC8` | 4 | **flag word** (§3) | **`0x8903`** | ctor; `GetFlag` `0x0099BDBB` |
| `+0xCC` | 4 | instance ID | 0 | `0x0099BE79`/`0x0099BE6A` |
| `+0xD0` | 4 | shade colour (4 bytes) | `FF FF FF FF` | `GetShadeColor` `0x0099CA9E` |
| `+0xD4` | 4 | fill colour (3 bytes used by `GetFillColor` `0x0099BFC9`) | 0 | ctor |

The `cGZWin` part ends at `+0xD8`. The constructor's field list covers everything above except `+0x34..+0x40` (always
written by `SetAreaToDrawTo` before use, §17.1) and `+0xA0..+0xA4`, which it leaves untouched: only the dialog
classes' mouse-down handlers set them (§18; WT2 answered 2026-10-23).

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
that test `AL` see the low byte of the flags. Every branch leaves through that single exit. (ui-windows.md §2.2 row 68 now says the same; WT4 answered
2026-10-21.)

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
`DestroyWindow`, until the list is empty; returns true only if every destroy returned true. Immediate destruction
requests normally detach through `ChildDeleteAbsolute`; a busy manager only queues the request and does not detach
it (section 8.1). Consequently a nonempty list with a persistently busy manager can make this loop nonterminating:
the same front child is requested repeatedly, even though each request returns true. This corrects the former
synchronous-detach assumption associated with WT5 (`0x0099DD76–A6`, `0x009DC4B3–DA`).

**Parenting boundary (WT12):** `SetParentWin` has no manager-validity, self, or ancestor-cycle test. On an
unparented window, setting the parent to itself or to an arbitrary non-null pointer stores that pointer and
returns true; no reference is acquired for it. A later reparent through an old invalid pointer can dereference
it. `ChildAdd` rejects a null/unregistered child via the manager and rejects an existing list member, but has no
separate self/ancestor-cycle rejection (`0x0099E207–63`). Self or cyclic additions can reach recursive geometry,
buffer or lifecycle work without returning; there is no specified benign false result for them. The ordinary
tree operations require live registered windows with acyclic parent relations. A portable API may enforce that
domain, but such validation is additional behavior. Direct SetParentWin does not insert into the new parent's
list, and setting null does not itself remove an old list entry (`0x0099B7F2–32`).

### 4.3 Reordering

| Call | Slot | Behaviour |
|---|---|---|
| `ChildToFront(child)` | 25/26 (`0x0099E3A8`) | If `child` is valid and in the list: **first, if this window has a parent, `parent->ChildToFront(this)`** (so the whole ancestor chain is raised). Then, if `child` has flag `0x800`: move it to the front (unless already there) and return true; without `0x800` return false (the ancestors were still raised). |
| `ChildToBack(child)` | 27 (`0x0099E457`) | If valid, present (`ChildExists`, slot 20) and flag `0x800`: move it to the back, return true. No ancestor effect. |
| `PullToFront()` | 23 (`0x0099B856`) | `parent->ChildToFront(this)`; false (low byte 0) without a parent. |
| `SendToBack()` | 24 (`0x0099B86B`) | `parent->ChildToBack(this)`; false without a parent. |
| `ChildStepFront(child)` | 28 (`0x0099E4B7`) | Valid (manager slot 24) and present: swap it with the window directly in front of it (one step toward the front); already front-most → true, no change. Not present or invalid → false. **No `0x800` check and no ancestor effect** (round 3). |
| `ChildStepBack(child)` | 29 (`0x0099E525`) | Mirror image: move it behind the window directly behind it; already back-most → true, no change. Same checks. |
| `ChildMoveRelative(a, b, before)` | 31 (`0x0099E594`) | Both `a` and `b` must be children (`ChildExists`, slot 20) else false. `before` ≠ 0: put `a` directly **in front of** `b`; `before` = 0: directly **behind** `b`. If `a` is already there (or `a` = `b`), nothing moves. True. No `0x800` check. `MoveRelativeTo(a, b)` (slot 30) is the child-side form, §17.4. |
| `IsWinInChildChain(w)` | 22 (`0x0099E03E`) | Depth-first over the children in list order: true at the first child equal to `w` or whose own slot 22 is true; false otherwise (a window is not in its own chain). |

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

**Mutation and nesting (WT10).** The active list is live, not a snapshot: each callback is followed by reading
its node's current successor. Appending during a callback can therefore expose the new filter later in the
**same** pass, even when the appending callback was originally last. A nested message or paint pass starts at
the current list head and can see those additions too. Duplicate registrations are allowed and each owns one
reference. RemoveMessageFilter (slot 117, `0x0099D865`) finds the first equal pointer; no match returns false.
Outside filter iteration it erases that one entry and Releases its reference, returning true. While `+0x90` is
set it instead appends that pointer to `+0x8C`, acquiring a separate owning reference, and returns true without
changing the active list. Thus pending filters can still receive this pass and nested passes. The flag is a saved
Boolean, restored after each pass, not a nesting counter. **DoMessage never drains pending removals**, even on
outermost return. Only a PlotComposite filter pass entered with the flag clear drains them, after restoring
the flag and before testing whether paint was consumed; an early visibility/fade rejection never reaches that
pass. Each pending entry removes one first matching active registration, then the pending list is cleared and
its references Released. Shutdown also clears both lists when initialized. Evidence: live traversal
`0x0099CD18–38`, `0x0099E69E–C1`; append/remove `0x0099D832–F2`; paint-only drain `0x0099E6C3–0x0099E733`.

Deferred removal is not deduplicated: repeated removal of a singleton while iterating reports true repeatedly
and retains a reference for every request. The drain has an original quirk: its guard compares the pending-node
address with the active-list sentinel, rather than checking the find result (`0x0099E6FB–0x0099E70D`). If a
queued pointer has no remaining active occurrence, it attempts to erase the active sentinel. This is list
corruption, not a supported harmless repeat; no stable subsequent callback/return behavior is promised. Safe
mutation vectors below keep at least one active occurrence per queued removal. Clearing lists through Shutdown
inside a callback likewise is not covered by deferred-removal safety.

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

**Type 1 return (WT11):** after calling slot 16, DoMessage returns the **AL left by that call**, with no false
assignment or handled-value normalization (`0x0099CE8F–9A`, `0x0099CEF2–F8`). The base ChildDelete is a tail
forward to manager DestroyWindow (`0x0099B833–3A`), so its ordinary result byte propagates: for example a busy
manager queues the request and returns true, while an immediate request for an invalid window returns false
(section 8.1). A void-typed custom ChildDelete override that leaves an arbitrary AL has no defined Boolean result
from its declaration alone; the dispatcher exposes that byte. A portable callback boundary must supply that
byte explicitly if it aims to reproduce native return behavior.

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

**Interface and reference lifetime (WT12).** Base QueryInterface accepts IIDs `1`, `0x22BA0121` and
`0xE98B2F57`: it stores the same primary window pointer through the supplied output, calls virtual AddRef,
then returns true. Failure returns false with **output untouched**, without AddRef; there is no output-pointer
validation on success (`0x0099B774–A2`). Both constructors explicitly start the reference count at **zero**
(`0x0099D948`, `0x0099DA69`); manager registration does not establish a creation reference. Callers needing
ownership must acquire one. Base AddRef increments the 32-bit count with wrapping arithmetic and returns the
new word (`0x0094C47C–82`), without saturation or atomic synchronization. Base Release (`0x0099B7A3–BC`, also
reached through inherited wrappers) treats the word as **unsigned**: above 1 it stores and returns count minus
one; at **0 or 1** it calls virtual slot 148 (offset +0x250) with argument **1**, then returns zero. The final
branch does not first store a decremented count. In particular zero is a destruction request, not underflow to
`0xFFFFFFFF`, whereas AddRef at `0xFFFFFFFF` wraps to zero.

Slot 148 is the concrete deleting destructor: it completes derived teardown including the base destructor,
and frees allocation storage when argument bit 0 is set; with bit 0 clear it tears down without freeing.
Other argument bits do not select an array path in the verified concrete wrappers cGZWinGen `0x0099B485–A0`
and cGZWinTextEdit `0x009BFF52–6D`. The base vtable's own full inventory remains WT1; these wrappers and the
shared Release call establish the lifetime contract for ordinary concrete windows without inventing an
unpublished base deleting-wrapper address. Directly invoking a deleting destructor bypasses reference counting
and is not a supported way to preserve live references.

**Manager requests and retained child blocks (WT12).** DestroyWindow (`0x009DC4AC`) with a positive signed
manager busy depth (+0x18C) only queues a raw pointer at +0x190 unless the window or one of its ancestors is
already covered by the queue, and returns true. It does not AddRef, Shutdown, detach, or free in that branch
(`0x009DBAA4` tests queue coverage). With depth at most zero it calls the immediate routine `0x009DB0FD`:
an invalid window returns false; a valid window with a valid parent delegates to that parent's
ChildDeleteAbsolute; a valid root receives temporary AddRef, Shutdown, Release and returns true. For ordinary
parented windows ChildDeleteAbsolute likewise holds a temporary reference, shuts down and detaches the child,
then makes another manager request on the now-detached child and finally Releases the temporary reference
(`0x0099E344–A7`). Thus successful DestroyWindow is **not forced deletion regardless of references**.

An old read-locked COW child block retains its references after a replacement block is installed (section 4.1,
`0x0099E08F–D5`). A child detached and shut down by immediate manager destruction can remain allocated and
registered because that old block or a caller still owns it. The old iteration still contains the child pointer;
no tombstone is inserted into that block. The last unlock of a retired block Releases its entries
(`0x0099D3D2–0x0099D40A`); only final Release reaches destruction/unregistration when no other owner remains.
This is a reference-lifetime guarantee, not a guarantee that a retained child is still initialized or attached.
The parent object itself must outlive its active passes: base destruction frees its current child block without
waiting for its reader count (`0x0099E1CA–DE`). Neither self-destruction during a pass nor arbitrary dangling
parents become valid through COW.

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

Hide uses `set_fade_state(0xFF, 0)`: alpha 0xFF at once and period 0, so the window itself does not fade out;
`+0x98` is not used by `ShowWindow`, `HideWindow` or `SetFlag` directly. The visible fade-out comes from destroying the
private buffer: a temporary `cGZFadeTmpWin` takes it and fades it down over `+0x98` (§14.7).

## 9. Quirks (keep for compatibility)

1. `SetFlag` returns the whole flag word, not a bool (§3.1).
2. `SetFlag` side effects run only for the exact single-bit masks of §3.1; combined masks change bits silently.
3. `ChildToFront` raises the ancestors even when it refuses to move the child (§4.3).
4. Hit testing descends into children without checking the parent's bounds and ignores enabled state (§5.3).
5. `IsPointInWindowParentCoordinates` skips the per-pixel test the other two point tests apply (§5.2).
6. `GetChildWindowFromIDRecursive` prefers a descendant over the window itself (§5.3).
7. The wheel message passes x/y as full dwords, other mouse messages as sign-extended 16-bit values (§7.3).
8. The window never fades itself out; the fade-out is a temporary window that steals the private buffer and uses
   the second period `+0x98` as a negative period (§14.7).
9. A clean window (`+0x70` = 0) repaints nothing of its own or of its non-overlay descendants; only windows with a
   private buffer and a parent ever become clean (§14.3).
10. `PrivateBuffer(1)` sets bit `0x10000` but `PrivateBuffer(0)` never clears it (§14.5).
11. A default `SortChildren` (`moveAll` false) never moves the windows **with** bit `0x800` (the z-order-movable
    bit), though other windows may still be inserted ahead of them, so the result need not be sorted (§17.6).
12. `SetParam` releases the old value before AddRef'ing the new one; re-setting the same object whose only reference
    is the map's destroys it first (§17.5).
13. `EnumChildren` returns false for a window with no children; `FitRectToWindow` shrinks (does not shift) on the
    left/top edge but shifts on the right/bottom edge (§17.6, §17.7).
14. `SetAreaToDrawTo` normalises the draw area to `(0,0,W,H)` whenever the draw-system service exists, and does not
    refresh the blit rectangle for a window without its own buffer (§17.1).

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
11. Painting order: parent P with children A (front) and B (back), no private buffers: one `P.Plot()` calls
    `P.GZPaint`, then `B.Plot`, then `A.Plot`.
12. Overlay: P with children O (back, `0x8000000`) and C (front) overlapping O: `P.Plot()` plots neither directly; O
    then C are queued and drawn by the flush at the end of the outermost screen-level `Plot`, each with `0x4000000`
    set during the call and cleared by its `PlotPresent`.
13. Clean window: a private-buffered child with a parent, plotted once, then plotted again without `InvalidateSelf`:
    its `GZPaint` is not called the second time.
14. Fade-out: a `0x20|0x10000` window, plotted once while visible, then `SetFlag(1, false)`: a `cGZFadeTmpWin` sibling
    appears with the same area, its alpha falls 255 → 127 → 0 at elapsed 0/250/500 (period `+0x98` = 500) and it is
    destroyed by its first `Plot` after the fade ends.
15. Cursor routing: a disabled front child under the point does not receive a click (the window behind it does) but
    does receive a move.
16. `CheckKeyEquivalent(0x41, 0x10002)` on a window whose key equivalent is `0x00020041` is true (`modifiers` shifted
    by 16 drops bit 16).
17. Draw rectangles (service present): R `(0,0,800,600)` unbuffered, P `(100,50,500,450)` buffered, C
    `(10,20,110,70)` and D `(5,5,25,15)` unbuffered (R⊃P⊃C⊃D): draw areas P `(0,0,400,400)`, C `(0,0,100,50)`, D
    `(0,0,20,10)`; draw origin rectangles P `(0,0,400,400)`, C `(10,20,110,70)`, D `(15,25,35,35)`; P's blit rectangle
    `(100,50,500,450)` (§17.1).
18. `InvalidateSelfAndParents` on D under an overlay ancestor O (`0x8000000`) inside root R: D and every window up to
    and including O are dirty, R is not; on O itself only O is dirty (§17.3).
19. `SortChildren(null, 0, false)` with children (front to back) IDs 3, 1, 2, bit `0x800` clear on all: order 1, 2,
    3, true. With IDs 2, 1 and bit `0x800` set on the ID-1 window: order stays 2, 1 (the flagged window is never
    moved and the unflagged one is never less than its predecessor), true (§17.6).
20. `FitRectToWindow((−10,0,30,20), 4)` in a 100×100 window: `(4,0,30,20)`, returns 1; `CenterWindowInRect((0,0,10,10))`
    for a 13×10 window: area `(−1,0,12,10)` (§17.7).
21. `SetParam(5, A)`, `GetParam(5, &x)` → true and A's count +1; `SetParam(5, null)` → `GetParam(5, &x)` false (§17.5).

### 10.1 Filter, return and lifetime vectors (WT10–12)

Callbacks below mutate only once and otherwise return false. Reference observations exclude caller ownership.

| Setup and action | Required observation |
|---|---|
| Filters A,B; A appends C during an outer DoMessage | Callback order A,B,C; C owns one list reference immediately |
| A removes B while filtering, then makes one nested DoMessage | Nested pass can still call B; outer pass can still call B; both returns leave B active and a queued owning reference |
| After the previous message, run an outer visible PlotComposite | Paint filter pass still offers B, then removes B and clears its queued reference before proceeding to paint |
| A removes itself during an outer paint pass and returns true | Paint is consumed, but A's active and pending references are still released by the drain before return |
| Remove absent pointer, then add B twice and remove B once outside a pass | Absent removal false; duplicate adds true with two owning entries; removal true leaves one entry/reference |
| Remove singleton B twice during a pass, then drain | Both removals return true; second drain lookup has no active B and reaches the sentinel-erasure quirk; no benign-repeat guarantee |
| Type 1, base ChildDelete, busy manager, initially unqueued child | Child queued without detachment; DoMessage returns true |
| Type 1 with instrumented slot 16 leaving AL 0x7E | Dispatcher returns byte 0x7E unchanged, not forced zero or normalized one |
| QI supported IID with output sentinel and count 0; then QI unsupported IID | First stores primary pointer and changes count to 1; second returns false, retaining pointer and count 1 |
| AddRef at count 0xFFFFFFFF; Release at count 0 or count 1 | AddRef wraps to 0; either Release invokes slot 148(1) and returns 0 without a pre-call count store |
| Release at count 0x80000000 | Stores/returns 0x7FFFFFFF; unsigned comparison does not treat it as a final release |
| Registered child C owned only by parent's list; hold old read block, detach/destroy C through immediate manager | New active list omits C; C is shut down, still allocated/registered and still in old block; last old-block unlock triggers final Release/destruction |
| Same retained block, but caller also owns C | Last old-block unlock releases that reference but does not destroy C; caller's final Release does |
| Unparented live W; direct SetParentWin(W), without subsequent traversal | Returns true and stores self as parent/notification target; no cycle rejection or reference acquisition |
| ChildAdd(null) or unregistered pointer; existing valid list member | Returns false; no child Init or insertion; cycles are a separate unsafe domain |

## 11. Open questions

Filed in `docs/specs/questions/widget-toolkit.md`: WT1 (own vtable rows), WT2 (unset fields), WT3 (bits `0x100`,
`0x8000`), WT5 (`ChildDeleteAll` termination), WT6 (instance-ID counter start).

## 12. Coverage of owner patterns (honest)

| Owner pattern (`tools/coverage-map.tsv`) | State after this slice |
|---|---|
| `cGZWin` (the class itself; matched by `cGZWin*`) | **specified** for layout, flags, children, z-order, geometry, hit testing, dispatch, filters, lifecycle, fade in and out, accelerator wiring (§13.4), painting (`Plot`, `PlotComposite`, `PlotPresent`, `PrivateBuffer`, `SetBufferToDrawTo`, §14), mouse routing and `UpdateCursor` (§15), key equivalents (§16), and (round 3, §17) the draw rectangles `+0x24`/`+0x34`/`+0xB8` (`SetAreaToDrawTo`, `SetAreaToDrawToRecursive`, `SetBufferToDrawToRecursive`, `recalc_blt_area`), `child_private_buffers`, the invalidation trio, coordinate converters and parent-chain forwarders (`WindowToWindowCoordinates`, `IsWinInParentChain`, `MoveRelativeTo`, `ChildToFront(id)`, `PullToFront`, `SetFocus`), `ChildStepFront`/`ChildStepBack`/`ChildMoveRelative`/`IsWinInChildChain` (§4.3), the `GZOnSetFocus`/`GZOnMouseWheel` defaults, parameters (`GetParam`/`SetParam`/`EnumParams`), `EnumChildren`, `SortChildren`, `CenterWindowInRect`/`FitRectToWindow`, colour conversion and the two private-buffer transparency helpers. WT14 closes concrete slot 150, the base-owned graphics calls and ownership boundary in §§14.5.1–14.5.3 and 14.6; graphics services remain separate owners, not unspecified base-window operations. **Whole owner since 2026-10-23** (re-gzwin7): §18 adds `create_private_buffer`, the small accessors and the last helpers (and answers WT2, the fields `+0xA0..+0xA4`), so all 157 functions of the owner are placed. The draw-context, buffer, graphics-system and colour-manager slots stay cited by number; WT1, WT3, WT5 and WT6 stay open |
| `cGZFadeTmpWin` (not an owner pattern of this lane; listed for completeness) | constructor, `StealBuffer`, `Plot` specified (§14.7); destructor not |
| `cGZWinGen` | Partial concrete generated-window contract in [widget-generator.md](widget-generator.md): inventory, lifetime, callback dispatch, client layout, capture and drawing boundary. Open helper details are explicitly listed there. |
| `cGZWinHTML` | Partial window-side contract in [widget-html-window.md](widget-html-window.md); document grammar remains in [html-document.md](html-document.md). |
| `cGZWinCustom` | Parent forwarding and fill drawing in [widget-custom.md](widget-custom.md); construction/secondary-interface attribution remains open. |
| `cGZWin*` (other widgets and managers) | Coverage must be assigned per owner, not to this wildcard. The byte-ranked audit and proposed destinations are in [the lane report](../lanes/re-gzwin-spec9.md). |
| `cRZWinKeyAccelerator*` | `cRZWinKeyAccelerator` (all 16 functions) **specified** in §13; `cRZWinKeyAcceleratorRes` (4 functions, the text resource parser `SerialRead` `0x00984587`) not specified |
| `cGZLegacyControl`, `cGZUIScript*`, `cHTML*`, `cGZGraph`, `cGZ*Graph`, `fn:unscript`, `fn:script_tag_*`, `fn:GetGZWinFont`, `cSC4ScriptedWindowCOMDirector`, `cRZCursorManager`, `cGZClipboardManager*`, `cGZCheatCodeManager` | not specified |

## 13. `cRZWinKeyAccelerator`: the keyboard accelerator table *[compat]*

### 13.1 Identity and layout

Vtable `0x00AE3770`, 16 slots (rows in `docs/abi/vtables.tsv`). `QueryInterface` (`0x009D5CEB`) accepts IID
`0xE2C1B3C4` and `1` (unknown) only. Reference counted (`+0x04`, `AddRef` is the shared `cRZRefCount::AddRef`);
`Release` (`0x009D5D73`) at count 1 calls `Shutdown` (slot 4) and then the deleting destructor (slot 15) instead of
decrementing to 0, and returns 0.

| Offset | Meaning |
|---|---|
| `+0x08` | initialised flag (byte) |
| `+0x0C` | ordered map: 32-bit **key code** → 16-byte message record (`cGZMessage`: type + three dwords), node value at node `+0x14` |
| `+0x18` | message server (`cIGZMessageServer`, service `0xC1085722`) held while initialised |

`Init` (slot 3, `0x009D5D88`): once; acquires and AddRefs the message server, sets the flag; returns true.
`Shutdown` (slot 4, `0x009D5D23`): once; Releases the server and clears the flag; returns true. The map is kept.

### 13.2 Key codes

A key code packs the virtual key in the low 16 bits and a modifier word in the high 16 bits:

    code = vk | ((modifiers & 0xFFFFFF8F) | (keyUp ? 0x1000 : 0)) << 16

(`0x009D5F89..0x009D5FA4`: modifier bits `0x70` are dropped; bit `0x1000` of the high word, i.e. `0x10000000` in the
code, marks a key-**up** accelerator; a key-down accelerator has it clear.) A lookup with `vk` = `0x10`, `0x11` or
`0x12` (Shift, Control, Alt alone) or `modifiers` = `0xFFFFFFFF` never matches.

### 13.3 Slots

| Slot | Address | Behaviour |
|---|---|---|
| 5 `RegisterAccelerator(code, const msg&)` | `0x009D600F` | inserts `code → msg` **only if `code` is not already present** (an existing binding is kept); always returns true |
| 6 `RegisterAccelerator(code, type)` | `0x009D5DBD` | slot 5 with the record `{type, 0, 0, 0}` |
| 7 `UnregisterAccelerator(code)` | `0x009D5F41` | erases; returns whether it was present |
| 8 `IsAcceleratorPresent(code, withModBits, withUpBit)` | `0x009D5E9D` | linear scan in key order; each stored code is compared after clearing bits `0x00070000` when `withModBits` is false and bit `0x10000000` when `withUpBit` is false; the probe `code` is not masked |
| 9 `TestForAccelerator(vk, modifiers, isDown, msg* out)` | `0x009D5F6B` | builds the code (§13.2) and looks it up; on a hit copies the 16-byte record to `out` and returns true |
| 10 `TestForAccelerator(vk, modifiers, isDown, uint32* outType)` | `0x009D5E1A` | slot 9, returns only the record's type dword |
| 11 `TestForAccelerator(const msg& keyMsg, msg* out)` | `0x009D5D44` | only for key messages: type 5 (key down) → slot 9 with `(keyMsg+4, keyMsg+8, true)`; type 6 (key up) → `isDown` false; other types false |
| 12 `TestForAccelerator(const msg& keyMsg, uint32* outType)` | `0x009D5DE4` | slot 11, returns only the type dword |
| 13 `GetKeysForAccelerator(type, uint32* codes, uint32* inoutCount)` | `0x009D5EDE` | walks the map in key order; writes the code of every entry whose record type equals `type` until `*inoutCount` (capacity on entry) entries are written; `*inoutCount` = number written; **returns the total number of matches**, which can exceed the capacity |
| 14 `ProcessAccelerator(const msg& keyMsg)` | `0x009D5E56` | slot 11; on a hit, if a message server is held, sends the bound record **synchronously** (`MessageSend`, server slot 3) and returns true (true even when no server is held); false on a miss |

### 13.4 Wiring into the window

`cGZWin::SetKeyboardAccelerator(acc)` (slot 76, `0x0099B955`) swaps the window's `+0x78` reference (Release old,
AddRef new, nothing when equal). `AccelerateKeyboardMsg(msg)` (slot 77, `0x0099B980`) tail-calls the accelerator's
`ProcessAccelerator(msg)` (slot 14), or returns false without one. `DoMessage` calls it only after a key-down/up
handler returned false (§7.3). `Shutdown` of the window Releases the accelerator (§8.1).

Worked example: a window with an accelerator binding `code = 0x4E | (0x0 << 16)` (N key, no modifiers, key down) to
`{type T}`: a key-down message `{5, 0x4E, 0x0}` that the window's `GZOnKeyDown` does not handle sends `{T,0,0,0}`
through the message server and `DoMessage` returns true; the matching key-up `{6, 0x4E, 0x0}` builds code
`0x1000004E`, finds nothing, and returns false.

## 14. Painting: `Plot`, `PlotComposite`, `PlotPresent` and the private buffer *[compat]*

### 14.1 State used by painting

| Field / bit | Meaning for painting |
|---|---|
| `+0x64` | the window's own **private buffer** (off-screen surface; interface `0x86D72B57` gives its colour) |
| `+0x68` | **buffer to draw to**: own private buffer, else the nearest ancestor's, else the screen buffer (§14.6) |
| `+0x6C` | draw context of `+0x68` (from the buffer's interface `0xAB300B2B`, its slot 4) |
| `+0x70` | **dirty** byte: set by `InvalidateSelf` (slot 91, `0x0099BECC`), by construction (1) and by the two area setters; cleared by `PlotComposite` only for a window that has a private buffer **and** a parent |
| `+0x71` | 1 when `+0x68` is a private buffer (own or ancestor's), 0 when it is the screen buffer |
| `+0xB8..+0xC4` | blit rectangle used by `PlotPresent` when presenting into the parent's buffer (computed by `recalc_blt_area` `0x0099C7BA`, §17.1) |
| `0x4` | passed to draw-context slots 22/23 (a draw-mode switch, *[inferred]*) |
| `0x8` | draw/present with the shade colour `+0xD0` (else white `0xFFFFFFFF`) |
| `0x40000` | clear the private buffer to its transparent colour before each repaint (needs `0x20000` too) |
| `0x400000` | "private buffer holds a painted image": set by `PlotComposite` after `GZPaint` when a private buffer exists; cleared at the end of every visibility `SetFlag` (§3.1) |
| `0x4000000` | "present straight to the screen": set on a child just before it is queued on the manager's deferred plot list; cleared by its `PlotPresent` |
| `0x8000000` | **overlay** window: drawn through the manager's deferred queue, after the normal tree (§14.3) |

### 14.2 `Plot()` (slot 89, `0x0099BA07`)

`ok = PlotComposite() && PlotPresent()` (slots 123, 124; `PlotPresent` is skipped when `PlotComposite` returns false).
Then, if `+0x71` is 0 (this window draws to the screen), manager slot 8 (`0x009DBD68`) **flushes the deferred plot
queue**. Returns `ok`.

Manager deferred plot queue (list at manager `+0x184`, enabled by byte `+0x188`): slot 7 (`0x009DC225`) appends a
non-null window and returns true (false for a null window or a disabled queue); slot 8 pops windows from the front and
calls each one's `Plot` until the queue is empty (windows queued during the flush are flushed too); returns true
(false when disabled).

### 14.3 `PlotComposite()` (slot 123, `0x0099E62D`)

In order:

1. **Visibility / fade gate.** Bit `0x20` clear: return false when invisible. Bit `0x20` set: return false when
   invisible **and** no fade is running (`fade_in_progress` `0x0099C20D`: `0x20` and `0x10000` set and the period
   parameter non-zero); otherwise advance the fade (`fade_advance`, §8.2), also for an invisible window whose fade
   still runs.
2. **Paint filters.** A record `{type 0x16, 0, 0, 0}` is offered to the live `+0x88` filter list as in `DoMessage`
   (§7.2), but **only this paint path** drains `+0x8C` when entered non-nested. If a filter
   returns true: return **true** without painting.
3. **Clean-window shortcut.** If `+0x70` is 0: `EnumChildren(0x22BA0121, cb, winMgr)` (slot 32) with callback
   `0x0099B9AC`, then return true. Per child the callback skips an invisible child; for an overlay child
   (`0x8000000`) sets its `0x4000000` and queues it (manager slot 7); otherwise **recurses** with the same callback into
   that child's children. A clean window repaints none of its own pixels and none of its non-overlay descendants: its
   private buffer is reused and only overlays are refreshed.
4. **Prepare.** If a private buffer exists: with a parent, clear `+0x70`; if `0x20000` and `0x40000` are both set,
   `fill_private_buffer_with_transparent_color` (`0x0099BA3E`).
5. **Draw-context setup** (only when `+0x6C` is non-null; AddRef'd for the duration): context slots 3, 8 (`&+0x34`),
   6 (`&+0x34`), 4, 15, 10, 19; slots 22 and 23 with `m = GetFlag(4) && !GetFlag(0x10000)`; slot 21 with the shade
   `+0xD0` if `GetFlag(8) && !GetFlag(0x10000)`, else `0xFFFFFFFF`; slot 43.
6. **`GZPaint()`** (slot 88): the subclass's own drawing.
7. If a private buffer exists, set bit `0x400000`.
8. **Draw-context teardown** (when present): slots 42, 11, 9, then Release.
9. **Children, back to front.** The child list is read-locked (§4.1) and walked from the **tail** (backmost) to the
   head (frontmost): painter's order. Two modes:
   - *This window is an overlay* (`0x8000000`): every child, visible or not, gets `0x4000000` and is queued (manager
     slot 7); none is plotted directly.
   - *Normal window*: a local list of rectangles starts empty. For each **visible** child: an overlay child gets
     `0x4000000`, is queued, and its absolute area (slot 50) is added to the list. A non-overlay child, when the list
     is non-empty, has its absolute area tested against each listed rectangle (`0x0099C1E1`); on the first hit its area
     is added to the list, it gets `0x4000000` and is queued instead of plotted. Otherwise the child is plotted now
     (`Plot`, slot 89). Invisible children are skipped.
10. Unlock the child list, return true.

Consequence: a child that comes after (is in front of) an overlapping overlay sibling is deferred too, so overlays and
everything above them reach the screen in back-to-front order after the rest of the tree.

### 14.4 `PlotPresent()` (slot 124, `0x0099C498`)

Copies the private buffer to its destination; always returns true.

1. No private buffer: return true (the window drew straight into `+0x68` during `GZPaint`).
2. Destination and rectangle:
   - bit `0x4000000` set and a parent exists: the **screen buffer** (graphics system slot 8) with the **absolute**
     rectangle `+0x14..+0x20`; the bit is then cleared (`SetFlag(0x4000000, 0)`).
   - otherwise: the parent's buffer-to-draw-to (parent slot 94) with the blit rectangle `+0xB8..+0xC4`; with no parent
     or no parent buffer, the screen buffer (graphics-system pointer cached once in global `0x00BAC058`), still with
     `+0xB8..`.
3. If the destination answers interface `0xAB300B2B` (draw context from that interface's slot 5): context slots 4,
   8 (a zeroed 8-byte value), 10; slots 22 and 23 with `GetFlag(4)` (not masked by `0x10000`, unlike §14.3); slot 21
   with shade-or-white (here `GetFlag(8)` alone decides). Then, if the private buffer yields its colour
   `(c1, c2, c3, a)` through interface `0x86D72B57` slot 4, slot 21 again with `a<<24 | c1<<16 | c2<<8 | c3`, or, with
   bit `0x8`, with the shade dword whose top byte is replaced by `a` (`0x0099C63D`). This carries the fade alpha
   (§8.2) to the screen. Then context slot 38 blits `(privateBuffer, &+0x24, &rect)`, context slot 11, interface slot 6.
4. Otherwise: `dest.slot6(0x8010)` (lock); on success `dest.slot29(privateBuffer, &+0x24, &rect, 0)` and
   `dest.slot7(0x8010)` (unlock). No colour or alpha is applied on this path.

### 14.5 `PrivateBuffer(on)` (slot 100, `0x0099EA70`)

`on` is tested as a byte.

**Create** (`on` true): forced to the destroy path if **any ancestor is invisible** (walk of `GetParentWin`, slot 11,
testing `IsVisible`). Otherwise bit `0x10000` is **set**, then:
- a buffer whose size (its slot 12 rectangle, right−left by bottom−top) equals the area size (`+0xB0−+0xA8` by
  `+0xB4−+0xAC`) is kept;
- else `CreatePrivateBuffer(w, h)` (concrete slot 150): on failure the function returns **false** (the common tail
  still runs); on success, with `0x20000` set, `set_private_buffer_transparency(1)` (`0x0099BA92`), then `SetArea`
  (slot 55) with the current area.

**Destroy** (`on` false, or forced), only when a buffer exists:
- buffer slot 39 supplies the surface-readiness gate (plain buffer: initialized byte; draw-service wrapper:
  constant true, §14.5.2). Only when it returns true **and both existing flag
  bits** `0x20` and `0x400000` are set is a fade transfer considered. Neither bit is set here. In that case
  `0x400000` is cleared; if moreover the window has no parameter `0xE6E998FE` (it is not itself a fade-out
  window, §14.7) and every ancestor is visible, a **fade-out window** takes the buffer (§14.7);
- otherwise, when slot 39 returned true, the buffer's slot 4 is called (release the surface); when slot 39 returned
  false, slot 4 is not called;
- the window then Releases its reference and `+0x64` becomes null. Bit `0x10000` is **not** cleared. Returns true.

Common tail: `SetBufferToDrawToRecursive` (slot 96), `child_private_buffers(IsVisible())` (`0x0099D645`),
`InvalidateSelfAndParents` (slot 92).

### 14.5.1 Concrete `CreatePrivateBuffer(width, height)` (slot 150, WT14)

This is a destructive surface allocation/reinitialization request, not an atomic replacement. Width and height
are signed 32-bit values; the caller's area differences are native 32-bit subtractions. The method rejects the
request **only when both dimensions are ≤ 0**. It then Releases the old `+0x64`, if present, clears the field and
returns false, without calling surface Shutdown. A single nonpositive dimension is passed unchanged to the
selected service. Do not clamp dimensions or add a second base-window validity check
(`0x0099D0F5`–`0x0099D113`, signed greater-than tests).

For every other request the window acquires a temporary draw-system service (class `0xEB2EA3CB`, IID
`0x4B2EA3B3`; `0x00449720`–`0x00449747`). The following table specifies the window-owned effects in order.
An existing buffer is denoted B; service calls below use zero-based slots on the **named receiver**.

| Available service / old buffer | Allocation and ownership contract | Result |
|---|---|---|
| Draw system present, B present | B.slot4 Shutdown, B.slot2 Release, clear `+0x64`; then draw-system slot5 `(&+0x64, width, height, 1)` | Truth of slot5's byte result |
| Draw system present, no B | Draw-system slot5 `(&+0x64, width, height, 1)` | Truth of slot5's byte result |
| No draw system, B present | B.slot4 Shutdown; reuse the same owned object with B.slot3 Init `(width, height, 1, 0)` | True on Init success; otherwise B.slot2 Release, clear `+0x64`, false |
| No draw system, no B | Ask `RZ::GS()` for its graphics system. If present, its slot3 receives `&+0x64`; examine the output pointer, ignoring the call's byte result. A non-null output receives buffer slot3 Init `(width, height, 1, 0)` | False for absent graphics system or null output; otherwise the Init rule above |

The Init arguments are four dwords: width, height, colour type 1, bits per pixel 0. The last pair is an
eight-byte by-value format descriptor, not a pointer. The helper at `0x0044A490` is misleadingly labelled as a
vector operation in the export: it writes type 1 and the result of `0x00449500(1)`, which is zero. The window
does not choose a screen depth here (`0x0099D181`–`0x0099D198`). The backend resolves that format request.

All acquired draw-service references are released before returning. No AddRef is made on a successful output:
the output reference becomes the window's owned `+0x64`. On the draw-service path the slot's output effects
are retained even when it reports failure; the window neither restores B nor adds an extra Release of a failed
output. On the fallback path Init failure always Releases and nulls the candidate. Existing B is never Released
before fallback Init, and no extra Shutdown follows failed Init (`0x0099D11D`–`0x0099D1A9`).

Calling slot 150 alone does not set flags, configure transparency, reset geometry, refresh `+0x68`/`+0x6C`,
invalidate, or visit children. Those are `PrivateBuffer`'s responsibilities (§14.5). In particular it can leave
borrowed drawing pointers stale until the caller refreshes them. `PrivateBuffer(true)` avoids allocation when
the existing buffer's slot12 rectangle has the requested size; slot 150 itself has no same-size shortcut.
Failure from slot 150 skips transparency and SetArea, preserves the already-set `0x10000` bit, and still runs
the common tail. Successful allocation ignores the transparency helper's result and SetArea's result; the
common tail's results likewise do not replace the saved success byte (`0x0099EB22`–`0x0099EB6E`,
`0x0099EC65`–`0x0099EC95`). A size match skips transparency and SetArea too.

### 14.5.2 Graphics dependency boundary (WT14)

The base window owns the selection, arguments, call ordering, reference effects and failure handling in this
section and §§14.2–14.6, 17.1 and 17.8. Surface pixels, format resolution and graphics engines are service-owned.
An implementation using Host/NativeOp must expose these requests and their output/result effects; a local
bitmap allocation with invented success, colour or lock results is not equivalent. There is no further hidden
base-window allocator or drawing loop behind slot 150.

| Receiver and slots used by the window | Meaning at this boundary |
|---|---|
| Buffer 2, 3, 4, 39 | Release owned interface; Init dimensions/format; Shutdown surface; surface-readiness gate. Slot39 is not a dirty/content/visibility test. On the plain buffer (`0x00AC1400`) slot4 frees pixels and clears initialized byte, and slot39 returns that byte (`0x00825CE0`, `0x008264B0`). On the draw-service wrapper (`0x00AC1550`), slots4 and 39 both return true without changing state (`0x00648F00`), and slot3 always returns false (`0x00735290`). Thus fallback reuse of an existing wrapper fails Init and Releases it. |
| Buffer 6/7, 12, 20, 23/26/27, 29, 30/31 | Lock/Unlock; borrowed rectangle pointer; fill native colour; set/get/remove colour key; rectangle blit; RGB/native conversion. In particular slot12 returns a rectangle pointer (`0x008268C0`), whereas slot11 copies a rectangle to caller storage. §§14.4, 17.8 specify the arguments and result handling. |
| Buffer QI `0xAB300B2B`, interface slots 4/5/6 | Borrow current drawing context; begin presentation access; finish presentation. The queried interface is retained only for the operation and Released afterwards; the stored `+0x6C` is borrowed. On the wrapper, slot4 attaches the raw target and resumes the context if it is detached (`0x0082AF20`); slot5 requests buffer Lock `0x8010` and returns the borrowed context without checking Lock's result (`0x0082AF50`); slot6 requests Unlock `0x8010` (`0x0082ABE0`). These slots do not transfer an owning context reference. |
| Draw system 5; graphics system 3/8/34/36 | Create wrapped private surface with context; create fallback buffer object; obtain screen buffer; map RGB to/from native colour. These are distinct services. |
| Draw context 3/8/6/4/15/10/19 | PushTransform, OffsetAbsolute, ClipAbsolute, ClipReset, GetTransformStackDepth, PushState, ResetToDefaultState, in the paint setup order already specified in §14.3. |
| Draw context 22/23/21/43; 42/11/9 | SetAlphaBlend, SetBltModulate, SetColor, Resume; then Suspend, PopState, PopTransform at paint teardown. Slot15 is a getter whose value the window ignores, not an additional state mutation. |
| Draw context 38; colour manager 32 | Blt source and destination rectangles; resolve theme colour ID `0x08963F6C` for the private surface's colour key. |

For context behavior see [gz-graphics.md §§2–3](gz-graphics.md); the raw buffer route and storage owner are
described in [span-render-8.md §§10.4–10.7](span-render-8.md). The concrete draw-system slot5 at `0x0082B980`
creates a raw graphics-system buffer and initializes it before wrapping it with a drawing context. Raw object
creation failure or Init failure returns false, releases any temporary raw buffer, and does not write the caller's
output. Thus ordinary backend failure leaves the already-cleared window field null. Success transfers an owned
wrapper reference; the window does not separately own the wrapper's raw buffer or context. The native service
assumes the graphics system exists. Exhaustion of the later 0x4C-byte wrapper or 0xC8-byte context allocations
is **not** a recoverable false: the native code continues to dereference the null allocation
(`0x0082BA10`–`0x0082BAAC`). Keep that unsafe failure boundary distinct from the two reported failures; the
base window does not manufacture recovery from it.

### 14.5.3 Allocation and graphics conformance vectors (WT14)

Use a recording graphics host; B and N are distinct owned buffer objects. Unless stated otherwise, only slot
150 is invoked, so no flag, geometry, drawing-target or invalidation calls follow its recorded effects.

| Vector | Setup / request | Required observations |
|---|---|---|
| WT14-A | B present; `(0,-1)` | Release B once, null field, false; no service acquisition, Shutdown or Init. |
| WT14-B | Draw service present; B present; `(0,20)`; service succeeds with N | Shutdown B, Release B, clear field, service slot5 `(out,0,20,1)`, Release service; true and field N, no AddRef N. |
| WT14-C | Same as B; service returns false and leaves output untouched | False, field null, B lost; no Init fallback, no restore, no extra buffer Release. |
| WT14-D | Custom draw-service failure writes N then returns false | False with field N retained; slot150 adds no cleanup for that output. |
| WT14-E | No draw service; B present; `(30,40)`; Init succeeds | Shutdown B, Init B `(30,40,1,0)`; true, same B, no Release B or graphics-system creation. |
| WT14-F | Same as E, Init fails | Shutdown B, Init B, Release B, null field, false; no second Shutdown. |
| WT14-G | No draw service or B; graphics slot3 returns false but writes N; Init succeeds | Init N `(30,40,1,0)` still occurs; true, N owned without extra AddRef. |
| WT14-H | No draw service or B; graphics absent, or graphics slot3 leaves output null | False and null; no Init. |
| WT14-I | `PrivateBuffer(true)`, all ancestors visible, B size already matches, transparency requested | Set bit `0x10000`; no slot150, transparency helper or SetArea; common tail once; true. |
| WT14-J | `PrivateBuffer(true)`, resize required, allocation fails | False, `0x10000` remains set; no transparency/SetArea; target recursion, child-private-buffer pass with current visibility, then invalidation still occur. |
| WT14-K | `PrivateBuffer(false)`, initialized B, bit `0x20` clear, painted bit set | No fade transfer and no painted-bit clear from this path; Shutdown B, Release B, null; true then common tail. |
| WT14-L | Same as K but initialized test false | Release B and null; skip Shutdown and fade tests; true then common tail. |
| WT14-M | No own/ancestor buffer; graphics service absent; old cached target/context non-null | SetBufferToDrawTo clears target/context, leaves `+0x71=1`, then calls area recursion; no Release of old borrowed pointers. |
| WT14-N | Target QI returns false but writes a non-null interface | Do not call interface slot4; Release that interface; cached context remains null; still call area recursion. |

### 14.6 `SetBufferToDrawTo()` (slot 95, `0x0099C6F8`)

`+0x68` = own private buffer (slot 101), `+0x71` = 1. If none: walk the parents (slot 11); the first with a private
buffer supplies it; when **this** window has `0x8000000` the walk stops after the first parent (the child's bit is
tested, not the parent's). If still none: the graphics system's screen buffer (slot 8) and `+0x71` = 0. Then `+0x6C`
= the buffer's draw context (interface `0xAB300B2B`, slot 4) or 0, and finally vtable slot 98 is called.

`+0x68` and `+0x6C` are borrowed caches: overwriting either field does not Release the old pointer or AddRef
the new one. The temporary graphics-service handle and queried buffer interface are owning references and are
released locally. Context is cleared before QI; only a true QI result triggers interface slot4, but any non-null
QI output is Released even on false. With no graphics service, both pointers stay null and `+0x71` stays **1**;
with a service present it becomes **0** even if screen-buffer slot8 returns null. Slot98 runs in all these cases
(`0x0099C6F8`–`0x0099C7B9`). These choices do not promise safe painting with missing services.

### 14.7 Fade-out window `cGZFadeTmpWin` (ctor `0x0099EA30`, size `0xD8`)

A plain `cGZWin` subclass whose constructor stores parameter `0xE6E998FE` (a 0x14-byte variant). Created only by the
destroy path of §14.5: the new window gets `SetFlag(4, 1)`, then `StealBuffer(src)` (`0x0099CBB0`):
1. If `src` has a parent: parent `ChildAdd(tmp)` (slot 14), `tmp.SetArea(src.GetArea())`, `tmp.MoveRelativeTo(src,
   0)` (slot 30). Otherwise the main window (slot 10) adopts it and it gets `src`'s **absolute** area.
2. Flags: clear `0x40000`, set `0x230021` (visible, fade, private buffer, transparency, `0x200000`).
3. `+0x64` = `src`'s private buffer, AddRef'd; `SetBufferToDrawTo`.
4. `set_fade_state(0xFF, −p2)`, `p2` = `src`'s **second** fade period `+0x98` (`GetFadeEffectPeriod` `0x0099C276`
   returns `+0x94` then `+0x98`).

With a negative period `fade_advance` (§8.2) runs the alpha **down**: `a = 255 + elapsed×255/(−p2)`; the fade ends
when `a < 0` (alpha 0, period 0). Worked example, default `p2` 500: elapsed 250 → 127; 500 → 0; 501 → −0.51, fade ends.

`cGZFadeTmpWin::Plot` (`0x0099C291`): while the fade runs, `cGZWin::Plot` then `InvalidateSelfAndParents`; once it
has ended, it calls manager slot 23 `DestroyWindow(this)` (`0x0099C2BC`, call through `+0x5C`) and returns true.

This corrects §8.2: **there is a fade-out**, but only for a window with bit `0x20` whose private buffer is destroyed
(for example hidden by `SetFlag(1, false)` with `0x10000` set, §3.1) while it holds a painted image; it uses the second
period `+0x98`.

## 15. Mouse routing: `ProcessCursorMessage` and the cursor *[compat]*

### 15.1 `ProcessCursorMessage(msg)` (slot 83, `0x0099D40B`)

Routes one mouse message (record of §7.1; its x/y are read and written as **full dwords** at `+4`/`+8`, in screen
coordinates on entry) down the window tree to the window that takes it. There are no direct callers; it is reached
through the vtable (the manager for the root, and this function itself for the recursion). Return value: the
`DoMessage` result of the window that took it, or false.

1. **Children first, front to back.** The child list is read-locked and walked from the head (frontmost). The first
   child that is visible (slot 71), does **not** have bit `0x200000`, contains the point (slot 62
   `IsPointInWindowScreenCoordinates(x, y)`, the unconverted dwords) and is either enabled (slot 72) **or** the
   message is a move (type 13) takes over: the list is unlocked and `child.ProcessCursorMessage(msg)` is returned.
   A disabled child under the point therefore stops nothing for clicks: the walk goes on to the children behind it and
   then to the parent.
2. **This window.** Returns false if this window has bit `0x200000` or is invisible.
3. **Enabled window:** for types 7, 8 and 9 (left down, right down, and type 9, *[inferred: middle down]*)
   `PullToFront()` (slot 23) then `SetFocus()` (`0x0099BDE4`, non-virtual); then `ScreenToWindowCoordinates(&x, &y)`
   (slot 59) converts the record's coordinates **in place**; for a move (13) also `UpdateCursor()` (slot 84).
4. **Disabled window:** only a move goes on (coordinates converted in place as above, no `UpdateCursor`); any other
   type returns false.
5. Manager slot 26 (`0x009DB756`, "set window under mouse") with this window, then `DoMessage(msg)` (slot 3) and its
   result is returned.

Manager slot 26, `SetMouseWindow(w)`: nothing when `w` is already the current one (manager `+0x3C`). Otherwise a
current window that the manager no longer considers valid (manager slot 24) is forgotten; `w` itself is replaced by
null when not valid; the old window (if any) receives `{type 20 (mouse exit), w}` and the new one (if any)
`{type 19 (mouse enter), old}`, both through `DoMessage` directly (not queued); then `+0x3C` = `w` (reference-counted).

Worked example: root R with children A (front) and B (behind), both covering (10,10), A disabled. A left-down at
(10,10): A is skipped (disabled, not a move); B takes it (it has no children): B is pulled to the front, gets focus,
the record becomes B-local, R's manager marks B as the mouse window (A or R gets exit, B gets enter if it changed),
and B's `GZOnMouseDownL` sees local coordinates. A move at the same point instead goes to A (disabled windows still
get moves), converted, without `UpdateCursor`.

### 15.2 `UpdateCursor()` (slot 84, `0x0099C468`)

On first use, the window asks the manager for the cursor manager (manager slot 32) and stores the result of its slot 9
(argument 1) at `+0x74`; that object is not Released here. Then, if `+0x74` is non-null, calls its slot 5 with 1
(*[inferred: make this cursor current]*). Concrete windows set their cursor through slot 85 (`cGZWinGen::SetCursor`,
not specified here).

## 16. Key equivalents *[compat]*

| Slot | Function | Address | Behaviour |
|---|---|---|---|
| 78 | `GetKeyEquivalent()` | `0x0099BE80` | returns `+0x9C` |
| 79 | `SetKeyEquivalent(code)` | `0x0099BE87` | stores `+0x9C` |
| 81 | `MakeKeyEquivalent(key, modifiers)` | `0x0099BEB6` | `modifiers << 16 | (key & 0xFFFF)`: the full 32-bit `modifiers` is shifted, so bits above 15 of `modifiers` are lost and none leak into the key half |
| 80 | `CheckKeyEquivalent(key, modifiers)` | `0x0099BE94` | `MakeKeyEquivalent(key, modifiers)` through the vtable (so an override of slot 81 applies) compared with `+0x9C`; returns exactly 0 or 1 in EAX |
| 82 | `IsChildKeyEquivalent(key, modifiers)` | `0x0099D5F0` | walks the **direct** children front to back (read-locked) and returns the first one whose `CheckKeyEquivalent` is true, or null; not recursive, and visibility and enabled state are not checked |

The default key equivalent is 0, so `CheckKeyEquivalent(0, 0)` is true on every window that never set one, and
`IsChildKeyEquivalent(0, 0)` returns the frontmost child. Users seen: `cGZWinDlg` asks its own `IsChildKeyEquivalent`
and acts on the child found (widget behaviour, not specified here).

Worked example: `SetKeyEquivalent(MakeKeyEquivalent(0x0D, 0))` = `0x0000000D` on button B in dialog D;
`D.IsChildKeyEquivalent(0x0D, 0)` returns B if no child in front of B has the same code; `(0x0D, 1)` builds
`0x0001000D` and finds nothing.

## 17. Draw areas, invalidation, conversions, parameters and small helpers *[compat]*

Evidence for this section: export instructions of every function cited (`re_query.py asm`), round 3. REA not used.

### 17.1 The three drawing rectangles

| Field | Name here | Set by | Read by |
|---|---|---|---|
| `+0x24..+0x30` | **draw area** (L,T,R,B) | `SetAreaToDrawTo` (slot 97) | `GetAreaToDrawTo` (slot 99, returns its address); source rectangle of `PlotPresent` (§14.4); fill rectangle of `fill_private_buffer_with_transparent_color` (§17.8) |
| `+0x34..+0x40` | **draw origin rectangle** | `SetAreaToDrawTo` | draw-context slots 8 and 6 in `PlotComposite` step 5 (§14.3) |
| `+0xB8..+0xC4` | **blit rectangle** | `recalc_blt_area` (`0x0099C7BA`), called only by `SetAreaToDrawTo` | destination rectangle of `PlotPresent` into the parent's buffer (§14.4) |

"Offset" of a window below means its area's L/T (`GetL`/`GetT`, slots 43/44); "has a buffer" means
`GetPrivateBuffer` (slot 101) is non-null.

**`recalc_blt_area()`** (`0x0099C7BA`): blit rectangle = the window's area (`GetArea` copy, slot 47); then for each
ancestor starting at the parent: stop at the first ancestor that has a buffer, otherwise add its offset (L to L and R,
T to T and B) and go on up. So the blit rectangle is the window's area in the coordinates of its nearest buffered
strict ancestor, or in screen coordinates when there is none. The overlay bit `0x8000000` is **not** consulted.
Returns true.

**`SetAreaToDrawTo()`** (slot 97, `0x0099CF6A`), two cases:

1. **The window has its own buffer**: draw area = `(0, 0, W, H)`; `recalc_blt_area()`; draw origin rectangle = draw
   area, so also `(0, 0, W, H)`. Done.
2. **No own buffer**:
   a. Draw area = area; working rectangle `r` = area. Walk the ancestors from the parent: at an ancestor with a buffer
      stop; otherwise, if **this** window has `0x8000000`, set `r` to this window's absolute area (`+0x14`) and stop;
      otherwise add the ancestor's offset to `r` and continue. Draw area = `r`. (A window without a parent keeps its
      area.)
   b. Once per process the code caches whether the system service with ID `0xEB2EA3CB`, queried as IID `0x4B2EA3B3`, exists (the export names the
      pointer `cRZSysServPtr<cIGZDrawSystem, 0x4B2EA3B3, 0xEB2EA3CB>`; cache flag `0x00BAC060` bit 1, result byte
      `0x00BAC05C`). **If it exists, the draw area is moved to the origin** keeping its size: `(0, 0, W, H)` (helper
      `0x0082AE40`, "move rectangle so its top-left is the given point"). Step a's translation is then discarded.
   c. Draw origin rectangle = draw area; then walk from **this window** upwards: stop at the first window that has a
      buffer (this window has none, so its own offset is always added first); a window with `0x8000000` adds its
      **absolute** L/T (`GetAreaAbsolute`, slot 50) and stops; any other window adds its offset (`GetArea`, slot 48)
      and the walk moves to its parent.
   `recalc_blt_area` is **not** called in case 2: the blit rectangle keeps its last value (it is only used by windows
   with a buffer).

With the service present (the shipped game creates its draw system *[inferred]*), case 2 gives draw area
`(0, 0, W, H)` and draw origin rectangle = the window's rectangle in its nearest buffered ancestor's coordinates (or
in screen coordinates; an overlay contributes its absolute position). Without the service, step c adds the offsets a
second time on top of step a; an implementation that always provides the service never reaches that path.

Worked example (service present): root R at `(0,0,800,600)`, no buffer; R's child P at `(100,50,500,450)` with a
buffer; P's child C at `(10,20,110,70)`, no buffer; C's child D at `(5,5,25,15)`, no buffer.
- P: draw area = draw origin rectangle = `(0,0,400,400)`; blit rectangle `(100,50,500,450)` (R adds offset 0).
- C: draw area `(0,0,100,50)`; draw origin: C's offset `(10,20)`, then P has a buffer → `(10,20,110,70)`.
- D: draw area `(0,0,20,10)`; draw origin: D `(5,5)` + C `(10,20)` → `(15,25,35,35)`.

**`SetAreaToDrawToRecursive()`** (slot 98, `0x0099D5B7`): `SetAreaToDrawTo()` (slot 97), then slot 98 on every child
in list order under the child-list read lock (§4.1). **`SetBufferToDrawToRecursive()`** (slot 96, `0x0099D57E`): same
shape with `SetBufferToDrawTo` (slot 95, §14.6) and slot 96. Callers: `SetArea` (§5.1), `SetFlag(0x8000000)` (§3.1,
buffer first, then area), `PrivateBuffer` (§14.5).

### 17.2 `child_private_buffers(on)` (`0x0099D645`, non-virtual)

Walks the direct children in list order under the read lock; `failed` starts false. For each child:

| Child has `0x10000`? | `on` | Action |
|---|---|---|
| yes | true, child visible (slot 71) | `child.PrivateBuffer(1)` (slot 100); a false result sets `failed` |
| yes | true, child hidden | `child.child_private_buffers(0)` (recursion with **off**, result ignored); the child's own buffer is left alone |
| yes | false | `child.PrivateBuffer(0)`; a false result sets `failed` |
| no | either | `child.child_private_buffers(on)` (recursion, result ignored) |

Then, **only when `on`**, for every child whatever its row: `child.GZWinMoveTo(child.GetL(), child.GetT())` (slot 56),
which re-runs the child's `SetArea` with unchanged values and so recomputes its absolute area and draw areas (§5.1).
Returns true unless a direct child's `PrivateBuffer` returned false (failures deeper down are not reported). Callers:
`SetFlag(0x1)` on a window without `0x10000` (argument: the new visibility) and `PrivateBuffer` (§14.5).

### 17.3 Invalidation

| Call | Slot | Behaviour |
|---|---|---|
| `InvalidateSelf()` | 91 (`0x0099BECC`) | dirty byte `+0x70` = 1 (§14.1) |
| `InvalidateSelfAndParents()` | 92 (`0x0099BED1`) | `InvalidateSelf()` (slot 91); then, **unless this window has `0x8000000`**, `InvalidateAllParents()` |
| `InvalidateAllParents()` | `0x0099B7BD` (non-virtual) | for each ancestor from the parent upwards: its `InvalidateSelf()` (slot 91); stop after the first ancestor that has `0x8000000` (that overlay ancestor is still invalidated). The window itself is not touched |

Nothing is painted; the dirty bytes are consumed by the next `PlotComposite` (§14.3). An overlay window never dirties
its ancestors, because it is composited through the manager's deferred queue rather than into them.

### 17.4 Coordinate conversion, parent chain and small forwarders

| Call | Slot | Behaviour |
|---|---|---|
| `ScreenToWindowCoordinates(&x, &y)` | 59 (`0x0099BD73`) | converts `(0,0)` through slot 60 (virtual, so an override applies) and subtracts the result from `x` and `y` |
| `WindowToWindowCoordinates(other, &x, &y)` | 61 (`0x0099B8F5`) | `other` null: false, point untouched. Else this window's slot 60, then `other`'s slot 59 on the same point; true. Converts from this window's coordinates to `other`'s |
| `IsWinInParentChain(w)` | 21 (`0x0099B929`) | true if `w` is a strict ancestor (the walk starts at the parent; a window is not in its own chain) |
| `MoveRelativeTo(other, before)` | 30 (`0x0099B880`) | with a parent: `parent.ChildMoveRelative(this, other, before)` (slot 31, §4.3) and its result; without: false |
| `ChildToFront(id)` | 25 (`0x0099B83B`) | `ChildToFront(GetChildWindowFromID(id))` (slots 34 then 26): the ID form of §4.3 |
| `SetFocus()` | `0x0099BDE4` | returns the manager's slot 37 result for this window (§6) |
| `GZOnSetFocus(x)` (default handler) | 132 (`0x0099CF49`) | if this window is **not visible** (`GetFlag(1)` false), `SetFocusTo(+0x4C)` passes focus on to the notification target; returns false (not handled) either way |
| `GZOnMouseWheel(…)` (default handler) | 139 (`0x00938789`, shared stub, four dword arguments) | returns false |

### 17.5 Parameters (`+0x7C`)

An ordered map from a 32-bit key to a reference-counted object (slot 1 AddRef, slot 2 Release), iterated in
ascending unsigned key order.

- **`GetParam(key, &out)`** (`0x0099D388`): absent → false, `out` untouched. Present → `out` = the object, **AddRef'd
  for the caller** (the caller releases it), true.
- **`SetParam(key, obj)`** (`0x0099D7C8`): if the key is present, release its object (when non-null) and erase the
  entry; then, if `obj` is non-null, AddRef it and insert `(key, obj)`. Always true. `SetParam(key, null)` removes the
  parameter. The old value is released **before** the new one is AddRef'd (quirk 12).
- **`EnumParams(cb, ctx)`** (`0x0099D252`): for each entry in key order, `cb(this, key, obj, ctx)` (cdecl, byte-bool
  result; `obj` is **not** AddRef'd). A false result stops the walk and `EnumParams` returns false; otherwise true
  (also for an empty map).
- `Shutdown` releases every value (§8.1).

### 17.6 Enumerating and sorting children

**`EnumChildren(iid, cb, ctx)`** (slot 32, `0x0099D708`): `cb` null → false. Under the read lock; **no children →
false**. For each child in list order (the walk never stops early):
- `iid` = `0x22BA0121` (the window interface): `cb(this, child.GetID(), child, ctx)`; no AddRef.
- any other `iid`: `child.QueryInterface(iid, &p)` (slot 0); on failure the child is skipped (counts as success); on
  success `cb(this, child.GetID(), p, ctx)`, then the query's reference is released (through the child's Release).
Returns true only if **no** callback returned false. Callback: cdecl, four arguments, byte-bool result.

**`SortChildren(less, ctx, moveAll)`** (slot 33, `0x0099E0D6`): `less` null → default comparator `0x0099BC0E`
= `a.GetID() < b.GetID()` compared **unsigned** (slot 63). The list is taken for writing (copy-on-write, §4.1). Fewer
than two children → false, nothing changes. Otherwise a restarting insertion sort:
1. Walk adjacent pairs `(prev, cur)` from the front.
2. If `moveAll` is false and `cur` **has** flag `0x800` (`GetFlag(0x800)` true), `cur` stays where it is and the walk
   moves on (it still serves as `prev` and as a comparison position in step 3).
3. Else if `less(cur, prev, ctx)`: remove `cur`, re-insert it **before the first window `n` from the front** with
   `less(cur, n, ctx)` (every window is a candidate position; at the end if none), and restart at step 1.
4. When the walk reaches the end: true.
Equal windows keep their relative order. Sorting never invalidates or repaints. Note the polarity in step 2: it is
the windows **with** `0x800` (the bit `ChildToFront`/`ChildToBack` require, §4.3) that a default sort never moves
(quirk 11).

### 17.7 Placement helpers

- **`CenterWindowInRect(rect)`** (`0x0099C373` by reference; `0x0099C303` by pointer, which does nothing for null):
  `x = rect.L + (rect.W − W) / 2`, `y = rect.T + (rect.H − H) / 2`, division **truncating toward zero** (CDQ, SUB,
  SAR 1); then `SetArea(x, y, x+W, y+H)` through slot 55 (virtual). Example: rect `(0,0,101,50)`, window 50×20 → x =
  25, y = 15; rect `(0,0,10,10)`, window 13×10 → x = −1 (−3/2 truncates to −1).
- **`FitRectToWindow(rect, margin)`** (slot 58, `0x0099C3DF`): pulls `rect` (this window's coordinates) inside
  `(0, 0, W, H)`, in this order: L < 0 → L = `margin` (R unchanged, the rectangle shrinks); T < 0 → T = `margin`;
  R > W → L += W − R − `margin`, R = W − `margin` (shift); B > H → T += H − B − `margin`, B = H − `margin`. Returns 1
  if afterwards `R − L + margin ≤ W` and `B − T + margin ≤ H`, else 0. Example with W = H = 100, margin 4: rect
  `(−10,0,30,20)` → `(4,0,30,20)`, 1; rect `(90,0,120,20)` → `(66,0,96,20)`, 1.

### 17.8 Colours and private-buffer transparency

- `SetFillColorRGB(c)` / `GetFillColorRGB()` (slots 125/126): store/return `+0xD4` as a whole dword.
- **`ConvertPackedRGBToNative(packed)`** (slot 127, `0x0099BB08`): r = bits 16–23, g = bits 8–15, b = bits 0–7 (bits
  24–31 ignored); asks the buffer to draw to (`+0x68`, its slot 30) or, without one, the graphics system (`RZ::GS()`,
  its slot 34); 0 when neither exists.
- **`ConvertNativeToPackedRGB(native)`** (slot 128, `0x0099BB5A`): same two sources (buffer slot 31, graphics system
  slot 36) return r, g, b; result `0xFF000000 | r<<16 | g<<8 | b` (alpha forced to `FF`). With neither source the
  three bytes are never written (undefined in the original; return `0xFF000000` *[recommended]*).
- **`set_private_buffer_transparency(on)`** (`0x0099BA92`): no buffer → false. `on`: the window colour manager
  (`RZ::WCM()`; absent → false) is asked (its slot 32) for colour `0x08963F6C`, which becomes the buffer's
  transparent colour (buffer slot 23); off: buffer slot 27 removes it. True.
- **`fill_private_buffer_with_transparent_color()`** (`0x0099BA3E`): no buffer → false. Lock the buffer (slot 6,
  flags `0x8001`; failure → false); if the buffer has a transparent colour (slot 26), fill the draw area `+0x24` with
  it (slot 20, last argument 0); unlock (slot 7, `0x8001`); true even when there was no transparent colour.

## 18. The rest of the owner: the private buffer, accessors and helpers *[compat]*

*Added 2026-10-23 (re-gzwin7), read on the instructions.* With this section every function of the `cGZWin` owner
(157 on `5c0ff782`) is specified.

### 18.1 `create_private_buffer(w, h)` (`0x0099D0ED`, pops 8, AL)

1. Neither `w` > 0 nor `h` > 0: release the private buffer `+0x64`, clear it, return false.
2. **With the draw system** (`cRZSysServPtr<cIGZDrawSystem>`, `0x00449720`):
   1. Release the old buffer (its slot 4, then `Release`) and clear `+0x64`.
   2. The draw system's slot 5 (`&+0x64, w, h, 1`) makes the new one; its AL is the result.
3. **Without it**:
   1. When `+0x64` is null, the graphics system (`RZ::GS()`, slot 3 (`&+0x64`)) makes a buffer object; still null →
      false.
   2. An existing buffer first gets its slot 4.
   3. Then the buffer's slot 3 (`w, h`, an 8-byte format descriptor made from 1 by `0x0044A490`). True → true; false →
      release and clear `+0x64`, false.

### 18.2 Accessors and forwards

Slot numbers are `cIGZWin` slots; "WM" is the window manager `+0x04`.

| Address | Method | Behaviour |
|---|---|---|
| `0x0094C47C` | `AddRef` | `++[+0x0C]`, returns the new count |
| `0x0099B774` | `QueryInterface(iid, out)` | IIDs 1, `0x22BA0121` (`cIGZWin`) and `0xE98B2F57` give `this` (AddRef'd), true; any other: false, `out` untouched |
| `0x0099B8AB` / `0x0099B8D0` | `GetChildAs(id, iid, out)` / `GetChildAsRecursive` | the child by ID (slot 34 / slot 35), then its `QueryInterface(iid, out)`; no child: false |
| `0x0099B993` | `SetCursor(cursor, apply)` | `+0x74` := `cursor`; with `apply`, slot 84 (`UpdateCursor`, §15.2) |
| `0x0099BC3F` | `set_init_flag(b)` | `SetFlag(0x4000, b)` (slot 68) |
| `0x0099BDCE` / `0x0099BDD9` | `IsVisible` / `IsEnabled` | `GetFlag(1)` / `GetFlag(2)` (slot 67) |
| `0x0099BC61` / `0x0099C82A` | `GetB` / `GetH` | `+0xB4` / `+0xB4 − +0xAC` |
| `0x0099BC68` / `0x0099BC8F` | `SetW(w)` / `SetH(h)` | `SetArea(L, T, L + w, B)` / `SetArea(L, T, R, T + h)` (slot 55) |
| `0x0099BCB6` | `SetSize(w, h)` | `SetArea(L, T, L + w, T + h)` |
| `0x0099BD13` | `SetSize(point&)` | slot 53 `SetSize(p.x, p.y)` |
| `0x009D7D97` | `SetArea(rect&)` | slot 55 with the rect's four values |
| `0x0099BD27` | `GZWinOffset(dx, dy)` | `SetArea(L + dx, T + dy, R + dx, B + dy)` |
| `0x0099C8C5` | `GZWinMoveTo(x, y)` | `SetArea(x, y, x + W, y + H)` |
| `0x0099BCE1` / `0x0099BCEC` | `GetArea()` / `GetArea(out)` | `&+0xA8` / copies `+0xA8`–`+0xB4` |
| `0x0099BCE8` / `0x0099BD01` | `GetAreaAbsolute()` / `(out)` | `&+0x14` / copies `+0x14`–`+0x20` |
| `0x0099BDA4` | `set_mouse_down_offset(x, y)` | `+0xA0`, `+0xA4`. Its callers are the dialog classes `cGZWinGenDlg` (`0x009C2307`) and `cGZWinMsgBox` (`0x009CD9E2`) |
| `0x0099BDF3` | `GetFocus` | WM slot 36 |
| `0x0099BDFE` / `0x0099BE1E` | `GZSetCapture` / `GZReleaseCapture` | WM slot 29 / 30 with `this` |
| `0x0099BE0A` | `GZAmCapturing` | WM slot 28 = `this` |
| `0x0099BE2A` / `0x0099BE3A` / `0x0099BF14` | `GetMainWindow` / `GetWindowFromPoint` / `GetCursorManager` | WM slots 3 / 25 / 32 (tail calls, arguments passed on) |
| `0x0099BE54` | `SetCaption(s)` | the caption string `+0x50`, its slot 13 (assign) |
| `0x0099BADC` | `MakeFillColor(r, g, b)` | the buffer to draw to (`+0x68`) slot 30 or, without one, `RZ::GS()` slot 34 (tail calls); 0 when neither |
| `0x0099BF1F` | `SetFillColor(native)` | `RZ::GS()` slot 36 splits it into r, g, b (no null test on the graphics system *[quirk]*), then own slot 107 `(r, g, b)` |
| `0x0099BFAC` | `SetFillColor(r, g, b)` | `+0xD4` := `r << 16 \| g << 8 \| b` |
| `0x0099BF5E` / `0x0099BF6D` / `0x0099BFA5` | `SetFillColor(c)` (AL = 1) / `SetFillColorRGB(c)` / `GetFillColorRGB` | `+0xD4` as a dword |
| `0x0099CA91` | `SetShadeColor(c)` | `+0xD0` := `c` |
| `0x0099CA60` | `PostMsg(target, type, d1, d2, d3)` (slot 146, pops 20) | builds `{type, d1, d2, d3}` on the stack and calls slot 147 `(target, &block)` |

### 18.3 Children

- **`GetChildCount`** (`0x0099DDB3`): counts the child list under its read lock: the block's `+4` is incremented, and
  `unlock_child_list_read` (`0x0099D3D2`) releases it.
- **`ChildExists(win)`** (`0x0099DDD5`): false unless WM slot 24 (`IsWindowValid(win)`) holds; then whether `win` is in
  the list (under the lock).
- **`ChildExists(id)`** (`0x0099DE2A`): whether a child's `GetID` (slot 63) equals `id`.
- **`GetChildWindowFromCursorPoint(x, y)`** (slot 40, `0x0099DFA9`):
  1. In list order, children that are visible (flag 1) and do not have flag `0x200000` are asked, through this same
     slot 40, for the window at (x, y); the first non-null answer is the result.
  2. Otherwise, `this` when `IsPointInWindowScreenCoordinates(x, y)` (slot 62, §5.2) holds, else null.

  It differs from `GetChildWindowFromPoint` (slot 39, §5.3) only in skipping flag-`0x200000` children.

### 18.4 Small helpers

- `0x0099D2AB`: the parameter map's destructor (clears the tree with `0x0099D229`, frees its header).
- `0x0099D2E7`: a list constructor for the filter lists (`0x004F5A00`, count := 0).
- `0x0099D3B4`: a three-argument insert wrapper into `0x009CB802`.
- **`GZOnMouseExit`** (`0x0079A1B0`, pops 4) is an override that lives in the resource catalog view's vtable
  (`0x00AB6AA8`, a developer tool). When `+0x11C` is non-null: `+0x114 |= 1`, `+0x11C` := 0, slot 92 (invalidate).
  Returns true.
