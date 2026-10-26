# The window manager `cGZWinMgrW32` (behaviour spec)

Scope: the owners `cGZWinMgrW32` (65 functions, 5,106 bytes), `cGZWinMgrBase` (8, 1,435) and `cGZWin32COMDirector`
(37, 2,077) as tools/coverage.py assigns them on `bb641878` (inventory in §10). Together they are the GZ window
manager: the framework service that owns the main window, the set of live windows, capture, focus and the window
under the mouse, routes mouse and keyboard input, queues posted messages, defers window destruction, runs modal loops
and tracks the keyboard. The modal loop itself (`DoModalWin`, `ModalMessageLoop`, `EndModal`, the begin/end notices)
is specified in modal-dialogs.md §2 and is only summarised here.

Related:
- modal-dialogs.md §2: service `0xA417445E`, IID `0x5A4`, slots 41, 42, 43, 44, 54, 62, 63;
- widget-toolkit.md: the `cGZWin` side (registration §8.1, message records and the dispatch table §7.3, cursor
  messages and "set window under mouse" §15.1, the deferred plot queue §14.2);
- docs/abi/interfaces.tsv: the `cIGZWinMgr` rows (vtable `0x00AE46E8`), whose argument counts this spec follows.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; both constructors, the capture, focus, removal,
mouse and destroy paths, `ProcessPostedMessages` and the keyboard-state slots were checked on the instructions. The
vtables `0x00AE46E8` (`cIGZWinMgr`, 64 slots), `0x00AE46B4` (system service), `0x00AE469C`, the three message targets
`0x00AE466C`, `0x00AE467C`, `0x00AE468C` and the keyboard state `0x00AC5268` were read. Written 2026-10-24
(re-gzwinmgr7).

Conventions:
- `M` is the manager object; offsets are from its start (the `cIGZWinMgr` pointer). "Slot n" is a `cIGZWinMgr`
  slot; "window slot n" a `cIGZWin` slot. A message is the 4-dword record `{type, d1, d2, d3}` of widget-toolkit.md
  §7 (mouse records: `d1` = x, `d2` = y, the high 16 bits of `d3` the buttons and modifiers).
- "Valid" means `IsWindowValid` (slot 24). Modifier bits: 1 Shift, 2 Control, 4 Alt (§7); mouse buttons `0x10`
  left, `0x20` right, `0x40` middle.

## 1. Identity and fields *[compat]*

- **Class** `0x84174448`, made by the Win32 COM director (§8): `0x1DC` bytes, constructor `0x009DAE9F`; the factory
  returns the system-service interface (`M + 4`).
- **Interfaces**: `+0x00` `cIGZWinMgr` (`0x00AE46E8`); `+0x04` the system service (`cRZSystemService`, ID
  `0xA417445E`, vtable `0x00AE46B4`: slot 8 `Init`, 9 `Shutdown`, 10 `OnTick`); `+0x08` a third COM table
  (`0x00AE469C`).

| Offset | Meaning | Constructor value |
|---|---|---|
| `+0x20` | posted-message queue (list of `{target, message}`, 0x1C-byte nodes) | empty |
| `+0x24` | its lock (`cRZCriticalSection`) | constructed |
| `+0x2C` | initialised (byte) | 0 |
| `+0x30` | capture window (counted) | null |
| `+0x34` | focus window | null |
| `+0x38` | current modal window (modal-dialogs.md) | null |
| `+0x3C` | window under the mouse | null |
| `+0x40` | main window | null |
| `+0x44` | the set of live windows (hash set keyed by the pointer value) | empty |
| `+0x58` | the framework | null |
| `+0x5C` | the keyboard state (§7) | constructed |
| `+0x164` | cursor manager | null |
| `+0x168`, `+0x16C` | last cursor position x, y | 0 |
| `+0x178` | last mouse buttons and modifiers | 0 |
| `+0x17C` | modal nest count | 0 |
| `+0x180` | plotting enabled (byte) | 1 |
| `+0x184`, `+0x188` | deferred plot queue and its "enabled" byte (widget-toolkit.md §14.2) | empty, 0 |
| `+0x18C` | busy count | 0 |
| `+0x190` | condemned windows (destroy later) | empty |
| `+0x194` | next instance ID | **1** |
| `+0x198` | deferred modal ends (map window → result) | empty |
| `+0x1A4` | key filters (list, counted) | empty |
| `+0x1A8` | a capture may be taken over while one is held (byte) | 1 |
| `+0x1A9` | outside a modal window, still track the window under the mouse (byte) | 0 |
| `+0x1AA`…`+0x1AE` | bytes | 1, 1, 1, 0, 1 |
| `+0x1BC`, `+0x1C4`, `+0x1CC` | three message targets `{vtable, M}` (§6) | `0x00AE466C`, `0x00AE467C`, `0x00AE468C` |
| `+0x1D4` | the canvas (from the graphics system) | null |
| `+0x1D8` | the canvas's message interface (posting and notifications) | null |

## 2. Lifecycle *[compat]*

- **`Init`** (system-service slot 8, `0x009DAA1C`), once: the base `Init` (`0x009DB1BC`): when there is no main
  window, `new cGZWin(M, &keyboard state)` becomes it and gets window slot 4; the framework is fetched and the manager
  is added to it (framework slot 9); the cursor manager is created (GZCOM `(0x4352FEF3, 0xA34C498A)`, else `new
  cRZCursorManager`) and initialised (its slot 3). Then, without a canvas: the graphics system's slot 14 gives it
  (held); its `QueryInterface(0x3AC7A210)` and that object's slot 3 give `+0x1D8` (held), on which the three targets
  are registered (slot 5 `(target, type)`) for `0x1A10630A`, `0xFA0C6390` and `0x7A104750`. Initialised := 1; true.
- **`Shutdown`** (slot 9, `0x009DA7A1`): release the canvas; unregister the three targets (slot 6) and release
  `+0x1D8`; the base `Shutdown` (`0x009DB279`): the five window references `+0x30`…`+0x40` are cleared, the main window
  gets window slot 5 first; the cursor manager gets its slot 4 and is released; the manager is removed from the
  framework (slot 10). Initialised := 0; true.
- **`OnTick`** (slot 10, `0x009DAF46`): `ProcessPostedMessages` (slot 16); true.
- **Destructors**: `0x009DAE38` (restore the vtables, `Shutdown`, destroy the deferred-end map) then the base body
  `0x009DC172` (destroy the lists, the window set, the lock and the queue; release the five references); deleting
  forms `0x009DAF08`, `0x009DC527`, thunks `0x009DAE79`, `0x009DC21D`.

## 3. The `cIGZWinMgr` slots *[compat]*

Slots 1, 2 are the reference-count thunks; the others not listed are in modal-dialogs.md (41–44, 54, 62, 63).

| Slot | Address | Behaviour |
|---|---|---|
| 3 `GetMainWindow()` | `0x009DB37A` (shared) | `+0x40` |
| 4 `SetMainWindow(w)` | `0x009DAB04` | if `w` differs: when initialised, `SetFocus(null)` and `ReleaseCapture(capture)`; replace (counted). When initialised and `w` is set: with a canvas, `w.SetArea(0, 0, canvas width, height)` (canvas slot 10; window slot 55); window slot 4; `SetFocus(w)`. True |
| 5 `GetMainWindowAs(iid, out)` | `0x009DB37E` | the main window's `QueryInterface`; false without one |
| 6 `PlotAll()` | `0x009DB9E3` | with `+0x180` set: empty the plot queue, enable it, queue the main window (slot 7), flush (slot 8), disable |
| 7 / 8 | `0x009DC225` / `0x009DBD68` | queue a window for plotting / flush (widget-toolkit.md §14.2) |
| 9 `GZWinMsgPost(type, d1, d2)` | `0x009DA90E` | §5 |
| 10 `GZWinMsgPost(type, x, y, mods)` | `0x009DAF53` | slot 9 `(type, mods, y << 16 \| (x & 0xFFFF))` |
| 11 | `0x009DAF79` | slot 9 with the same arguments |
| 12 / 13 `SendMessageToTarget` | `0x009DB38E` / `0x009DAF7E` | valid target → its `DoMessage` (window slot 3) at once and its result; else false |
| 14 / 15 `PostMessageToTarget` | `0x009DC286` / `0x009DC24E` | append `{target, message}` to the queue under the lock (§4) |
| 16 `ProcessPostedMessages()` | `0x009DC2D1` | §4 |
| 17 `DispatchMouseMessage(msg)` | `0x009DB3CE` | §6.1 |
| 18 `DispatchKeyMessage(msg)` | `0x009DB553` | §6.2 |
| 19 | `0x009DA98D` | `+0x1D8`'s slot 18 and its result; 0 without it |
| 20 `RegisterWindow(w)` | `0x009DC586` | widget-toolkit.md §8.1; false when already registered |
| 21 `UnregisterWindow(w)` | `0x009DC451` | remove from the set; whether it was there |
| 22 `OnWindowRemoved(w)` | `0x009DB8A4` | §6.3 |
| 23 `DestroyWindow(w)` | `0x009DC4AC` | §4.1 |
| 24 `IsWindowValid(w)` | `0x009DC087` | null → false; else whether the set holds the pointer (never dereferenced) |
| 25 `GetWindowFromPoint(x, y)` | `0x009DB957` | the main window's `GetChildWindowFromPoint` (window slot 39); 0 without one |
| 26 `SetMouseWindow(w)` | `0x009DB756` | widget-toolkit.md §15.1 (messages `0x14` / `0x13` to the old / new window) |
| 27 `GetCanvas()` | `0x009DA848` | `+0x1D4` |
| 28 `GetCapture()` | `0x009DB663` (shared) | `+0x30` |
| 29 `SetCapture(w)` | `0x009DB667` | §6.3 |
| 30 `ReleaseCapture(w)` | `0x009DB6F3` | §6.3 |
| 31 `GZShowCursor(on)` | `0x009DA82A` | Win32 `ShowCursor(on & 0xFF)`, then the graphics system's slot 30 `(on)` and its result |
| 32 `GetCursorManager()` | `0x00993797` (shared) | `+0x164` |
| 33 `GetCursorPos(out x, out y)` | `0x009DB006` | `+0x168`, `+0x16C` |
| 34 `GetCursorPosInWindow(w, out x, out y)` | `0x009DAFE0` | slot 33, then `w`'s `ScreenToWindow` (window slot 59) on them |
| 35 `SetCursorPos(x, y)` | `0x009DA9A4` | store; through the framework's interface `0x23EA70A1` (its slot 5, the main HWND): `ClientToScreen`, Win32 `SetCursorPos` |
| 36 `GZGetFocus()` | `0x009DB800` (shared) | `+0x34` |
| 37 `SetFocus(w)` | `0x009DB804` | §6.3 |
| 38 `GetKeyboardState()` | `0x009DB021` | `&+0x5C` |
| 39 `AddKeyFilter(f)` / 40 `RemoveKeyFilter(f)` | `0x009DC3E1` / `0x009DBBB9` | append (counted) unless present / remove |
| 45, 46 | `0x009378BC` (shared) | false |
| 47, 49, 51 / 48, 50, 52 / 53 | shared | false / nothing (one argument) / nothing |
| 55 `InvalidateAllWindows()` | `0x009DC47B` | window slot 91 on every registered window |
| 56 `DestroyWindowAbsolute(w)` | `0x009DB0FD` | §4.1 |
| 57 `DestroyCondemnedWindows()` | `0x009DBA4F` | §4.1 |
| 58 `IsWindowOrParentInCondemnedList(w)` | `0x009DBAA4` | true when `w` or one of its ancestors (window slot 11) is condemned |
| 59 / 60 | `0x009DB163` / `0x009DB16E` | busy count + 1 (returns it) / − 1 (not below 0; reaching 0 runs slot 57) |
| 61 `RunKeyFilters(msg)` | `0x009DBA20` | each filter's slot 3 `(0, msg)` until one returns true; whether one did |

## 4. Posted messages and deferred destruction *[compat]*

**`ProcessPostedMessages`** (slot 16). A process-wide counter (`0x00BAC13C`) holds the number of messages still to
deliver. When it is below 1 (no delivery running), it is set, under the lock, to the queue's current length; messages
posted while delivering wait for the next call. Then, while it is above 0: take one, decrement, pop the front record
under the lock, and deliver it:
- a valid target → its `DoMessage`;
- a null target → **broadcast**: busy count + 1 (slot 59), every registered window's `DoMessage`, busy count − 1
  (slot 60);
- an invalid non-null target → dropped.

A nested call (from a handler) continues the same batch. The return value is not meaningful.

### 4.1 Destroying windows

- **`DestroyWindow(w)`** (slot 23): busy count above 0 → unless `w` or an ancestor is already condemned (slot 58),
  append `w` to the condemned list; true. `w` stays attached to its parent until then. Busy count 0 →
  `DestroyWindowAbsolute(w)`.
- **`DestroyWindowAbsolute(w)`** (slot 56): note whether `w` is valid; `OnWindowRemoved(w)` (slot 22); with a valid
  parent (window slot 11) the parent's window slot 17 `(w)` gives the result. Without one: a valid `w` gets AddRef,
  `Shutdown` (window slot 5), Release, true; otherwise false.
- **`DestroyCondemnedWindows()`** (slot 57): guarded by a process-wide re-entry counter (`0x00BAC138`; false when
  re-entered). With the busy count below 1, `DestroyWindowAbsolute` on each condemned window, then the list is
  emptied. True.

## 5. `GZWinMsgPost(type, d1, d2)` (slot 9) *[compat]*

For the mouse types 7–14 `d1` is converted to Win32 key flags (`0x009DA6EB`): 1 → `MK_SHIFT` (4), 2 → `MK_CONTROL`
(8), `0x10` → `MK_LBUTTON` (1), `0x20` → `MK_RBUTTON` (2), `0x40` → `MK_MBUTTON` (`0x10`). The type becomes a Win32
message (`0x009DA71A`): 5 `WM_KEYDOWN`, 6 `WM_KEYUP`, 7 `WM_LBUTTONDOWN`, 8 `WM_RBUTTONDOWN`, 10 `WM_LBUTTONUP`, 11
`WM_RBUTTONUP`, 13 `WM_MOUSEMOVE`, 14 `WM_MOUSEWHEEL`, 15 → `0x700` (modal end, modal-dialogs.md §2.3),
`0x4348B111` → `WM_ACTIVATEAPP` (`0x1C`), others 0. The triple `{message, d1, d2}` goes as a variant to `+0x1D8`'s
slot 19 (posted to the canvas window); its result, false without the interface.

## 6. Input routing *[compat]*

The canvas reports input through the three message targets (each target's slot 3; `M` at target `+4`):
- **`0x1A10630A`** (`0x009DABAA`, event IID `0xDA1062FA`, kind = its slot 9): kinds 0, 5, 7 → the main window is
  sized to the canvas (as slot 4); 2 (activated) and 3 (deactivated; `SetCapture(null)` first) →
  `PostMessageToTarget(null, {0x4348B111, kind = 2, 0, 0})` (a broadcast); 4 → `PlotAll`. Others: nothing.
- **`0xFA0C6390`** (`0x009DAC8A`, IID `0x1A0C615F`): `{slot 9, point x, point y (slot 10), slot 12 << 16 | slot 11 &
  0xFFFF}` → `DispatchMouseMessage`.
- **`0x7A104750`** (`0x009DAD22`, IID `0x1A104742`): `{slot 9, slot 10, slot 11, slot 12}` → `DispatchKeyMessage`.

### 6.1 `DispatchMouseMessage(msg)` (slot 17)

Store x, y in `+0x168`/`+0x16C` and the high half of `d3` in `+0x178`.
- **With a capture**: for a move (13): the capture's `ScreenToWindow(&x, &y)` (window slot 59, in place). Otherwise,
  if the capture is disabled (window slot 72): the window under the mouse becomes the main window's
  `GetChildWindowFromPoint(x, y)` and the result is false; if enabled: `ScreenToWindow`, and for 7 and 8 (left/right
  down) `PullToFront` (window slot 23) and `SetFocus(capture)`. Then `SetMouseWindow(capture)` and the capture's
  `DoMessage(msg)`.
- **Without**: the target is the modal window when the point is in it (window slot 62) and it is visible (window
  slot 71) and, except for a move, enabled; with no modal window, the main window (false without one). A modal window
  that does not qualify: with `+0x1A9` set, the window under the mouse is updated from the main window's
  `GetChildWindowFromPoint`; false either way. The target's `ProcessCursorMessage(msg)` (window slot 83,
  widget-toolkit.md §15.1) gives the result.

### 6.2 `DispatchKeyMessage(msg)` (slot 18)

1. Type 5 → keyboard state `KeyDown(d1)`, type 6 → `KeyUp(d1)` (§7).
2. Target: the focus window if enabled, else the main window if enabled, else false.
3. With `r = {0x1A, target, msg type, &msg}`: the key filters (slot 61); then `r` to the target and up its parents
   (each `DoMessage`, stopping at the first true); then `msg` itself to the target. Any true → true.
4. Otherwise `r` becomes type `0x1B` and goes to the target and up its parents as far as the modal window
   (inclusive); then the key filters again with it. The result is whether anything returned true.

### 6.3 Capture, focus and removal

- **`SetCapture(w)`** (slot 29): nothing when `w` is the capture, or when a capture is held and `+0x1A8` is 0. Else
  the capture becomes `w`, and `SendMessageToTarget(x, {0x12, old, w, 0})` goes to the old window, then to `w` (each
  when not null). True.
- **`ReleaseCapture(w)`** (slot 30): only when `w` is the capture: `SetCapture(null)`, then a synthetic move
  `GZWinMsgPost(13, x, y, +0x178)` (slot 10) at the last position. Returns whether `w` was the capture.
- **`SetFocus(w)`** (slot 37): returns the focus. Nothing when `w` is the focus or not valid (so `SetFocus(null)`
  never clears the focus *[quirk]*), or when `w` lacks window flag `0x8000` (window slot 67): **`0x8000` marks a
  window that can take the focus**. Else `{0x10, w, 0, 0}` (kill
  focus) to the old focus, the focus becomes `w`, `{0x11, old, 0, 0}` (set focus) to `w`.
- **`OnWindowRemoved(w)`** (slot 22): if the capture is `w` or below it (window slot 21 on the capture), release it.
  If the focus is `w` or below it: focus to `w`'s notification target (window slot 87) or, without one, the main
  window; an invalid focus also moves to the main window. The window under the mouse is cleared when it is `w` or
  below it, or no longer valid.

## 7. The keyboard state (`M + 0x5C`, vtable `0x00AC5268`) *[compat]*

Fields: `+0` vtable, `+4` list of keys held (in press order), `+8` 256 bytes "held" by virtual-key code.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `Reset()` | `0x00891C01` | clear the table and the list |
| 1 `IsKeyComboDown(k, mods)` | `0x00891B0C` | `IsKeyDown(k)` and `GetModifiers() = mods` |
| 2 `IsKeyDown(k)` | `0x00891AD9` | the table byte (no bound check) |
| 3 `IsKeyDownNow(k)` / 6 `IsToggleKeySet(k)` | `0x00891B93` / `0x00891BA0` | `cRZKeyboard`'s live tests |
| 4 `IsModifierKey(k)` | `0x00891B32` | `k` ∈ {`0x10`, `0x11`, `0x12`} |
| 5 `GetModifiers()` | `0x00891B4E` | 1 Shift, 2 Control, 4 Alt (`VK_SHIFT`, `VK_CONTROL`, `VK_MENU` held) |
| 7 `KeyDown(k)` / 8 `KeyUp(k)` | `0x00891C52` / `0x00891BE1` | set and append / clear and remove, when it changes |

The constructor is `0x00891C2F`.

## 8. The Win32 COM director *[compat]*

`cGZWin32COMDirector` (constructor `0x009DA5BD`, director ID `0x041740BA` from `0x009DA5F8`) registers two classes:
`0x84174448` → the manager (`0x009DA575`, §1) and `0x2AC4516B` → a `0x9C`-byte object (`0x009DA599`, constructor
`0x00996271`, not covered here). The rest of the owner is the manager's constructor (`0x009DAE9F` and the base
`0x009DC5E8`), destructors, the message targets (§6) and container helpers (§9).

## 9. Quirks and cases

Quirks:
1. While the busy count is above 0 (during a broadcast) `DestroyWindow` leaves the window attached, so
   `cGZWin::ChildDeleteAll`, which repeats `DestroyWindow` on the front child until the list is empty, would not end
   if run then (widget-toolkit.md WT5).
2. A mouse event outside the modal window is dropped without updating the window under the mouse unless `+0x1A9` is
   set (it is 0 by default).
3. `SetFocus` silently refuses windows without flag `0x8000` (the slider, scroll bars and option buttons clear it).
4. `ProcessPostedMessages` uses process-wide counters, not per-manager ones.
5. `SetFocus(null)` is refused (null is not a valid window), so `SetMainWindow`'s `SetFocus(null)` does nothing.

Cases:
1. Left click on a button in the main window, no capture or modal: the `0xFA0C6390` event becomes `{7, x, y, …}` →
   `DispatchMouseMessage` → the main window's `ProcessCursorMessage`, which finds the button.
2. `SetFocus(edit)` while `list` has the focus: `{0x10, edit, 0, 0}` to `list`, then `{0x11, list, 0, 0}` to `edit`.
3. A dialog closed by a broadcast handler: `DestroyWindow(dialog)` queues it; when the broadcast ends the busy count
   returns to 0 and `DestroyCondemnedWindows` destroys it.

## 10. Function inventory

Owner `cGZWinMgrW32` (65): `0x009DA6EB` key-flag map, `0x009DA71A` type map, `0x009DA7A1` `Shutdown`, `0x009DA82A`
slot 31, `0x009DA848` slot 27, `0x009DA84F` slot 63, `0x009DA90E` slot 9, `0x009DA98D` slot 19, `0x009DA9A4` slot 35,
`0x009DAA1C` `Init`, `0x009DAB04` slot 4, `0x009DAF53` slot 10, `0x009DAF79` slot 11, `0x009DAF7E` slot 13,
`0x009DAFA4` bubble to parents, `0x009DAFE0` slot 34, `0x009DB006` slot 33, `0x009DB021` slot 38, `0x009DB025`
slot 42, `0x009DB031` slot 43, `0x009DB038` slot 54, `0x009DB0FD` slot 56, `0x009DB163` slot 59, `0x009DB16E`
slot 60, `0x009DB37E` slot 5, `0x009DB38E` slot 12, `0x009DB3CE` slot 17, `0x009DB553` slot 18, `0x009DB646` record
copy, `0x009DB667` slot 29, `0x009DB6F3` slot 30, `0x009DB756` slot 26, `0x009DB804` slot 37, `0x009DB8A4` slot 22,
`0x009DB957` slot 25, `0x009DB9B1` set find, `0x009DB9E3` slot 6, `0x009DBA20` slot 61, `0x009DBA4F` slot 57,
`0x009DBAA4` slot 58, `0x009DBB21` queue node, `0x009DBB42` set next bucket, `0x009DBB7C` set node, `0x009DBB99` map
node, `0x009DBBB9` slot 40, `0x009DBC2D` set iterator, `0x009DBC43` list insert, `0x009DBC68` set insert,
`0x009DBCDB` set erase, `0x009DBD68` slot 8, `0x009DC087` slot 24, `0x009DC0AB` queue append, `0x009DC0C5` set
resize, `0x009DC225` slot 7, `0x009DC24E` slot 15, `0x009DC286` slot 14, `0x009DC2D1` slot 16, `0x009DC3E1` slot 39,
`0x009DC451` slot 21, `0x009DC47B` slot 55, `0x009DC4AC` slot 23, `0x009DC4E6` insert with resize, `0x009DC543`
insert wrapper, `0x009DC56D` slot 62, `0x009DC586` slot 20.

Owner `cGZWinMgrBase` (8): `0x0093056F` AddRef, `0x00930577` Release, `0x009DAF46` `OnTick`, `0x009DB04A` slot 44,
`0x009DB1BC` base `Init`, `0x009DB279` base `Shutdown`, `0x009DB72D` base `GZShowCursor`, `0x009DBDA3` slot 41.

Owner `cGZWin32COMDirector` (37): keyboard state `0x00891AD9`, `0x00891B0C`, `0x00891B32`, `0x00891B4E`,
`0x00891B93`, `0x00891BA0`, `0x00891BAD` (erase range), `0x00891BE1`, `0x00891C01`, `0x00891C2F`, `0x00891C52`;
director `0x009DA575`, `0x009DA599`, `0x009DA5BD`, `0x009DA5F8`; message targets `0x009DABAA`, `0x009DAC8A`,
`0x009DAD22`; manager construction and destruction `0x009DAE38`, `0x009DAE71` (QI thunk), `0x009DAE79`, `0x009DAE9F`,
`0x009DAF08`, `0x009DC172`, `0x009DC21D`, `0x009DC527`, `0x009DC5E8`; container helpers `0x0041D990`, `0x009DAD98`,
`0x009DADC5`, `0x009DADEE`, `0x009DAE05`, `0x009DAE81`, `0x009DB96B`, `0x009DBADD`, `0x009DBAF8`, `0x009DBC16`.

Not covered: the canvas and its message interface (graphics system), the cursor manager, the `0x2AC4516B` object,
and the event objects' own slots beyond the getters used in §6.
