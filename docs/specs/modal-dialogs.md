# Modal dialogs: the modal loop and ending a dialog from a plugin — behaviour spec

Status: round 1, written from the Windows 1.1.641 executable. The Mac named pseudocode was used for names only.
Every fact below was checked on Windows unless it is tagged *[inferred]*.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

ABI rows: `docs/abi/interfaces.tsv`, interfaces `cIGZWinMgr` (vtable `0x00AE46E8`), `cIGZWin` (`cGZWinGen`,
`0x00ADC678`), `cIGZMessageServer` (`0x00AD1CD8`) and `cIGZMessageTarget(AudioEventHandler)` (`0x00A94154`). Their
notes start with `re-modal:`.

---

## 1. Purpose and scope

A modal dialog runs a nested message loop inside whatever handler opened it. The simulation date stops while it is
open whenever the dialog takes a hidden pause, which most do. An unattended run must therefore close every modal
dialog itself. This spec says how the loop ends and which interface calls a plugin can make to end it, so the test
harness no longer needs screenshots and clicks. It also lists the modal dialogs that can appear in unattended play
and what suppresses each one.

The advisor popup, its gates and its pause are in advisors.md §6.8.1 and are not repeated here. §6 below corrects
two details there. The dialogs' UI layouts are out of scope.

## 2. The window manager and the modal loop *[compat]*

### 2.1 Getting the manager

The window manager is framework service `0xA417445E`, interface `cIGZWinMgr`, IID `0x5A4` (the game's own accessor
is `0x00913C46`, which calls framework slot 5 `GetSystemService`). The class is `cGZWinMgrW32`.

### 2.2 `DoModalWin` (slot 41, `+0xA4`, `0x009DBDA3`)

Argument: the dialog window. Returns a `uint32` in EAX: the result given to `EndModal`, or one of two special
values.

1. **Refuse** with `0x80000000` if any of these holds: the window is not valid (slot 24); there is no parent and no
   main window to attach it to; the legacy message server is missing. A window with no parent is attached to the
   main window first.
2. **Begin notice.** Send legacy message `0x62BA7B42` synchronously (§2.5).
3. **Enter.** Give the window the focus (slot 37), show it and bring it to the front if needed, remember the
   previous modal window and make this one current (manager `+0x38`), set window flag `0x1000` (`cIGZWin` slot 68)
   and add one to the **nest count** (manager `+0x17C`).
4. **Loop** (slot 63, §2.3) until this window's end arrives.
5. **Leave.** Subtract one from the nest count, clear flag `0x1000`, make the previous modal window current again.
   If an end for that outer window arrived while this one ran, re-post it now (§2.4). Detach the window from the
   parent it was attached to in step 1, if any, restore the focus, and run one more frame.
6. **End notice.** Send legacy message `0x62BA7B43` (§2.5).
7. Return the result.

`DoModalWin` neither hides nor destroys the window. That is the owner's job after the call returns (§2.6).

### 2.3 The loop (`ModalMessageLoop`, slot 63, `0x009DA84F`)

Each pass drains the Win32 queue, then runs **one framework frame** (`cIGZFrameWork` slot 16, `+0x40`), which ticks
every service on the framework tick list. Every fourth pass it also yields the CPU (`Sleep(0)`).

While draining:
- the Win32 message `0x700` is the **end message** (GZ window message type `0xF`, mapped to `WM_USER + 0x300` by
  `0x009DA71A`). Its `wParam` is a window and its `lParam` a result. If the window is the current modal window, the
  result is kept and the loop stops after one more frame. Otherwise the pair is stored for later (§2.4);
- `WM_QUIT` is re-posted for the outer loop and the result becomes `0x80000001`;
- everything else is translated and dispatched as usual.

So the whole game keeps running inside the loop: the simulator, the clock, the manager's own tick and every plugin
that is on the tick list. Only the code that opened the dialog is blocked.

### 2.4 `EndModal` (slot 54, `+0xD8`, `0x009DB038`)

Arguments: the window and a `uint32` result. It posts the end message through `GZWinMsgPost` (slot 9) as type `0xF`
with data1 = window and data2 = result. That reaches the canvas window as Win32 `0x700`. It returns a bool in AL
only. **The SDK does not declare it**: `cIGZWinMgr` stops at slot 53.

An end for a window that is not the innermost modal window is kept in a map (manager `+0x198`, slot 62
`AddDeferredEndModal`). When the inner loop ends, the outer window's end is re-posted, so the outer dialog closes
next. So ending a dialog while a nested one is open is safe.

### 2.5 Begin and end notices

Both notices go through the **legacy message server** (framework service `0x64063387`, `cIGZMessageServer`, IID
`0xC1085722`), slot 3 `MessageSend`, which calls every subscriber at once. They are not MessageServer2 messages.

| Message | Sent | data1 | data2 | data3 |
|---|---|---|---|---|
| `0x62BA7B42` begin | before the window becomes modal (§2.2 step 2) | the window's ID (`cIGZWin` slot 63) | the window (`cIGZWin*`) | the nest count **before** this dialog |
| `0x62BA7B43` end | after the loop and the cleanup (§2.2 step 6) | the window's ID | the window | the nest count **after** this dialog |

A subscriber registers with slot 5 `AddNotification(target, type)`, one call per type; the target must implement
`cIGZMessageTarget` (IID `0xE98B2F57`), whose slot 3 `DoMessage(cGZMessage&)` receives the message. The server keeps
and AddRefs the pointer. The game's own subscriber is the audio event handler (`cSC4AudioEventHandler::Init`,
`0x004EFFD2`), which plays the dialog sounds *[inferred]*.

### 2.6 How a button ends a dialog

A button press is a **command message**: type 3, data1 = `0x287259F6` ("a button was pressed"), data2 = the button's
ID, data3 = its toggle state. The button (`0x009B00B4`) posts it to its notification target (the dialog window
*[inferred]*) through `cIGZWin::PostMsg` (slot 146 → 147), which is window manager slot 15 `PostMessageToTarget`.
The manager delivers posted messages from its own tick (slot 16 `ProcessPostedMessages`): it skips a window that is no
longer valid and otherwise calls the window's `DoMessage` (`cIGZWin` slot 3). For a dialog built from a UI script
(`cGZWinGen`, `0x0099A276`) the message goes first to the window procedure the owner attached, then to the window's
`GZOnCommand` (slot 143).

Each dialog family reacts in its own way (§3.3). The common pattern: the handler calls `EndModal(window, button ID)`,
`DoModalWin` returns the button ID, and the owner destroys the window (manager slot 23, `DestroyWindow`; deferred
while the manager is busy delivering messages).

## 3. Ending the current dialog from a plugin

### 3.1 Where plugin code runs

A plugin service on the framework tick list keeps getting `OnTick` while a dialog is open, once per loop pass
(§2.3). These ticks are **nested** inside the handler that opened the dialog. A legacy-server subscriber gets the
notices synchronously inside `DoModalWin`. Both run on the main thread, so the calls below need no locking.

### 3.2 The calls *[choice]*

**Track.** Subscribe to `0x62BA7B42` and `0x62BA7B43` (§2.5). On a begin notice, record the window (data2), its ID
(data1) and its depth (data3), and the current frame. On an end notice, drop the record for that window. **Do not
act inside the begin notice**: the window is not yet modal and `DoModalWin` keeps using it after the notice returns.

**Close.** On a later `OnTick`, for the innermost recorded window, when the closer is enabled and a grace period has
passed (a few frames, so the dialog has painted and a log line can name it):

1. Check that the window is still the current modal window: `IsWindowValid(window)` (slot 24) is true,
   `GetModalNestCount()` (slot 43) equals the recorded depth + 1, and the window's `GetFlag(0x1000)` (`cIGZWin`
   slot 67) is true. `IsWindowValid` only looks the pointer value up in the manager's window set, so it is safe on a
   stale pointer; call it first.
2. **Primary: press a button.** `PostMessageToTarget(window, 3, 0x287259F6, 0, 0)` (slot 14): button ID 0. This is
   exactly what a click does, so the owner's cleanup runs. Two load-time dialogs ignore button 0 (§3.3); step 3
   ends them.
3. **Fallback: end the loop.** If no end notice for that window has arrived after a timeout (some tens of frames),
   check step 1 again and call `EndModal(window, 0)` (slot 54). Log it: for the advisor popup this leaves the window
   alive (§3.3).

The dialog normally closes within three frames: the command is delivered at the manager's next tick, the handler
posts the end message, and the loop sees it on its next pass.

**If no begin notice was seen** (the closer started while a dialog was open): `GZGetFocus()` (slot 36) gives the
focused window, which `DoModalWin` set to the dialog. Walk up with `GetParentWin()` (`cIGZWin` slot 11) to the first
window whose `GetFlag(0x1000)` is true. With nested dialogs this finds the innermost one only if it holds the focus.

`SendMessageToTarget(window, 3, 0x287259F6, id, 0)` (slot 12) delivers the same command synchronously. Do not use it
from inside a game message handler or the begin notice; from `OnTick` it is equivalent to step 2.

| Call | Interface (IID) | Slot | Offset | Arguments | Return |
|---|---|---|---|---|---|
| `AddNotification` | legacy `cIGZMessageServer` (`0xC1085722`, service `0x64063387`) | 5 | `+0x14` | target, message type | ignore |
| `DoMessage` (yours) | `cIGZMessageTarget` (`0xE98B2F57`) | 3 | `+0x0C` | `cGZMessage&` | bool |
| `IsWindowValid` | `cIGZWinMgr` (`0x5A4`, service `0xA417445E`) | 24 | `+0x60` | window | bool in AL only |
| `GetModalNestCount` | `cIGZWinMgr` | 43 | `+0xAC` | — | int32 |
| `PostMessageToTarget` | `cIGZWinMgr` | 14 | `+0x38` | window, 3, `0x287259F6`, button ID, 0 | none (EAX is not a status) |
| `EndModal` | `cIGZWinMgr` | 54 | `+0xD8` | window, result | bool in AL only |
| `GZGetFocus` | `cIGZWinMgr` | 36 | `+0x90` | — | `cIGZWin*` or null |
| `GetParentWin` | `cIGZWin` (`0x22BA0121`) | 11 | `+0x2C` | — | `cIGZWin*` or null |
| `GetFlag` | `cIGZWin` | 67 | `+0x10C` | `0x1000` | 0/1 in EAX |
| `GetID` | `cIGZWin` | 63 | `+0xFC` | — | uint32 |

All are thiscall and pop their arguments. **SDK errata:** slots 54–63 are missing from the SDK's `cIGZWinMgr`;
slot 13 is `SendMessageToTarget(cIGZWin*, cGZMessage&)` with two arguments, not one; the SDK's overloads land in
reverse order (slot 12 is the 5-argument `SendMessageToTarget`, slot 14 the 5-argument `PostMessageToTarget`), which
an MSVC build of the SDK header reproduces but another compiler may not. Call slot 54 by offset.

### 3.3 What the close command does, by dialog family

| Family | Handler | Command `0x287259F6` with button 0 | `EndModal(window, 0)` alone |
|---|---|---|---|
| Advisor popup (advisors.md §6.8.1) | tracker window procedure `0x00762D30` | **any** button ID ends it: the tracker stops the advisor head animation *[inferred]*, clears the advice property, ends the loop with 0 and destroys the window | the loop ends and the tracker resumes, but the window is never destroyed and the animation keeps running. **Avoid** |
| Generic notification, confirmation, scripted dialog (`nSC4UI`) | window procedure `0x0078DCE0` | ends with result = button ID; 0 is "no" for a confirmation (yes is `0x8A8DFD4E`) and 0 for a scripted dialog; the owner destroys the window | the same |
| Message box (`GZMsgBox`, slot 44; `cGZWinMsgBox`) | its `GZOnCommand` | ends with result = button ID | the same |
| City-situation popup | window procedure `0x0052CAB0` | ends it. Buttons 1, 6 and 7 do more (1 cancels the situation, 6 and 7 start the special or joyride action); do not send those | not checked |
| Terrain edge reconciliation (`cSTEEdgeReconcileInform*`, created with ID `0x6A4D0A59`) | `0x00734D30` | **ignored**: only button `0x6A4D0A6B` ends it | ends it as "no"; the owner destroys the window |
| Missing plugin packs (`cSC4MissingPluginPacksDialog`) | window procedure `0x0048C0A0` | **ignored**: only button `0xAA89FAF5` ends it | ends it; the owner destroys the window |

So the fallback ends the last two families. Sending their own button instead needs a way to recognise them
(§7 question 1). For the edge reconciliation question, `0x6A4D0A6B` means "reconcile" and anything else "do not";
in both cases the terrain still posts TerrainReconciliation (`0x0074EF08`), so ConnectionsReady still follows
(foundations.md §11.6), with data3 = 0 for "do not".

### 3.4 Hazards

- **Nested ticks.** The closer's `OnTick` runs inside the handler that opened the dialog (for example inside the 3D
  view's message handler). Posting is safe there; calling into the simulator or starting another modal is not.
- **`SC4GameDelay`** (`0x005F9140`) runs an invisible modal window (`cSC4WinGameDelay`) that ends itself; it is used
  by the `GameDelay` automation command and the debug plop-all commands. Ending it early only shortens the delay. The
  harness should not use those commands while the closer is on *[choice]*.
- **Win32 message boxes are out of reach.** `MessageBoxA` runs its own loop with no framework frames, so no plugin
  code runs while it is open. Callers: `SC4DisplayResourceError` and `0x005F8950` (both at start-up), the DirectX 7
  driver set-up, `cRZExceptionNotification::Notify`, the framework's "Exception encountered" box (`0x00879685`),
  `0x0098B610` (canvas) and the C runtime. The harness must detect these from outside (a window title or a hang
  timeout).
- **Stale pointers.** Never call through the recorded window pointer before `IsWindowValid` says it is still a
  window, and never `EndModal` a window that is not current: the deferred-end map would keep the pointer and could
  later end a different window allocated at the same address.

## 4. Modal dialogs in unattended play

"Gate a" is `cISC4App` slot 23 `GetPopupDialogsEnabled`, which FreeCity4 sets false at ConnectionsReady
(advisors.md §6.8.1 option 1; `engine/plugin/FC4Popups.cpp`). "Safe" is `SC4GetPopupModalDialogsSafe`
(`0x005F9000`: no mouse button held, no modal running, city UI lock count 0). "Pause" means `cSC4AutoGamePause`
(`0x005F9320`): a hidden pause of the simulator and the city clock, released when the dialog closes.

| Dialog | Trigger | Pause | Suppressor | With the suppressor on |
|---|---|---|---|---|
| Advisor popup | an advice activates (advisors.md §6.8.1) | simulator + clock | gate a (also gates b–e there) | dropped, no pause |
| Bankruptcy (budget failure) | `0xAA5CF1E3` at a month-end with funds ≤ the minimum (budget.md §5.6) | simulator + clock | **none for the outcome** | no dialog, but the city still closes (§4.1) |
| Save notifications | `cISC4App` slot 9 `RequestSaveCity(showNotification, fast)` | simulator + clock | gate a; also pass `showNotification` = false | none shown |
| Connection confirmation | a network reaches the city edge where no neighbour connection exists (`cSC4NeighborsSimulator::RequestCreateConnection`, `0x006D15E0`) | simulator + clock | gate a | answered "no": the connection is not created |
| Intersection question | the network tool would create intersections (`cSC4NetworkTool::QueryUserForPotentialIntersections`) | simulator + clock | gate a | answered "no" *[inferred]* |
| Missing plugin packs | city load, post-city-init, when the city uses content that is not installed | none | gate a, but it is still on at this point | — |
| Edge reconciliation (three variants) | city load: a question when the terrain edges differ from the neighbours', or a notice, depending on the caller's flags (`0x0074ED40`) | none | gate a, but it is still on at this point | the whole reconciliation is skipped (advisors.md §6.8.1) |
| Lua error box (`gAdviceMessageBox`, `0x004179A0`) | a Lua advice extension (`advice_info`, `camera_jump`, `camera_zoom`, `trigger_event`, `unlock_reward_building`) gets bad arguments | none | **none** | — |
| City-situation popup | `0xEBB67EF7` `kSC4MessageShowSituationPopup` and `0x2C16BDE7`, only during a U-Drive-It mission | simulator, clock **and sound** | **none** (it tests neither gate a nor "safe") | — |
| Win32 message box | start-up errors, driver errors, exceptions | — | none | the plugin cannot act (§3.4) |

The two load-time dialogs come before ConnectionsReady, so the closer must be on from the start of city load, not
only after the popup flag is set. Disasters, neighbour deals, ordinance offers, reward offers and advisor questions
all arrive as advices, so they are advisor popups; no separate modal dialog was found for them.

**Only when the user or the harness asks:** `RequestCloseCity(true)` (a confirmation), `RequestQuit` and
`RequestGoToRegionView(true)` (scripted dialogs; with gate a off they return 0, which is "cancel", so the city
stays), the obliterate prompt, the tutorials (not analysed) and any dialog opened from a menu.

### 4.1 Bankruptcy *[compat]*

This answers budget.md §14 question 9. The 3D view handles `0xAA5CF1E3` (`cSC4View3DWin::DoMessage`):
- if "safe" is true, it runs the budget-failure sequence (`0x007E6E00`) at once: hidden pause (simulator and clock),
  scripted dialog `0x0A5CF71D` (dialog ID `0x2A5CFB2C`), whose result is ignored, resume, then
  `cISC4App` slot 12 `RequestGoToRegionView(false)`: **the city closes to the region view without saving**. Gate a
  only suppresses the dialog, not the closing;
- otherwise it queues the message once and re-posts it when it becomes safe. *[inferred]* If gate a is off at that
  point the queued message is dropped. The budget re-posts `0xAA5CF1E3` at every month-end while funds stay at or
  below the minimum, so the city closes at a later month-end anyway.

No other subscriber to `0xAA5CF1E3` was found. The ID appears only in the budget's post, the 3D view's handler,
the 3D view's subscription list (`0x00ABC070`, read by its constructor) and a message-name table. A harness run
that goes bankrupt therefore ends with the region view; the harness should treat that as the run's end, not as a
hang.

## 5. Harness tests

1. **Advisor popup closed by command.** With popups enabled, force an advice popup; the closer posts the command.
   Expected: an end notice for the same window, `GetModalNestCount()` back to 0, the hidden-pause count back to its
   value before the dialog, and the date advancing again within a few frames.
2. **Confirmation answered "no".** With popups enabled, trigger a connection confirmation; the closer ends it with
   button 0. Expected: the connection is not created.
3. **Nested dialogs.** Two dialogs open (for example a message box over a popup). Expected: the inner one closes
   first, then the outer one, and the nest count returns to 0.
4. **Fallback.** Load a city whose edges differ from a neighbour's (or that uses a missing plugin pack). The dialog
   ignores button 0, the fallback `EndModal(window, 0)` ends it after the timeout, and the load continues to
   ConnectionsReady.
5. **Bankruptcy.** Funds forced to −100,000 before a month-end. Expected: the run reports "ended by bankruptcy"
   (region view requested), not a stall.
6. **Watchdog.** If the date has not advanced for N seconds and no modal is recorded, report a Win32 message box or
   a hang instead of waiting.

## 6. Corrections to advisors.md §6.8.1

- `0x287259F6` is the "button pressed" command code in data1, not a control ID. The button's ID is in data2, and
  **any** button ends the advisor popup.
- Message types `0xA2BF8AD5` and `0xA2BF8AD6` are the notices a window procedure receives when it is attached to and
  detached from a window (`cGZWinGen::SetWinProc`, `0x009998C8`), not key down and key up. Enter and Esc do nothing
  because the tracker's window procedure handles no keyboard messages.

## 7. Open questions

1. Whether the window ID passed to `SC4CreateWindowFromScript` (`0x005F9390`, third argument) becomes the window's
   `GetID`, so that the begin notice's data1 identifies the dialog (`0x6A4D0A59` for edge reconciliation,
   `0x2A5CFB2C` for the scripted dialogs). Until checked, the closer should key on its own records, not on IDs.
2. Whether the paint-time re-post of a queued bankruptcy uses the same queue as the message handler (§4.1).
3. Who posts `0x2C16BDE7` (the situation popup's "poster not found" path).
4. The tutorial dialogs.
5. The identity of the object the advisor tracker stops when its dialog ends (tracker `+0x14`).
6. What the remaining callers of `DoScriptedModalDialog` do with result 0.
7. Whether any shipped Lua script triggers the Lua error box.
