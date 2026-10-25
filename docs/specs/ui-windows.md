# UI window and dialog layer: the window object, `cSC4View3DWin`, its catalog menus and the dialog close path (behaviour spec)

Status: round 1, written from the Windows 1.1.641 executable (SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`) by
the re-specs19 lane. Ghidra 12.1.4 (REA bridge) plus `tools/x86dis.py`/capstone were used; both agree on every address
cited. The Mac Intel named pseudocode (`external/extracted/sc4-mac-symbol-analysis`) was used for **names only**; no Mac
struct layout is applied to a Windows `this`, and every Windows offset below was read from Windows code.

Tags: *[compat]* observable, must match; *[choice]* a FreeCity4 decision; *[inferred]* consistent with the evidence but
not proven; *[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

**Sources and their limits.** `docs/abi/vtables.tsv` and `docs/abi/interfaces.tsv` were used as the slot-name and
argument-count inventory; the return channels and the object offsets in §3–§5 were re-derived from Windows assembly for
this spec and are cited per claim. Where a name in `re/names_windows.tsv` is an auto-port from the Mac build it is marked
*[inferred]* unless a Windows body was read.

---

## 1. Purpose and scope

Two things are specified here that no existing spec covers:

1. **The window object model as the game uses it** — what a `cSC4View3DWin` *is* in memory, how a game window relates to
   the framework window `cIGZWin`, and the complete `cISC4View3DWin` slot table with argument lists and return channels
   (§3). tutorial.md cites two of those 61 slots; this is the whole interface.
2. **The catalog-menu open/close pair** (`InvokeSecondaryMenu` / `InvokeTertiaryMenu`, `CloseSecondaryMenu` /
   `CloseTertiaryMenu`, §4) — the path the tutorial's `endTutorial` uses, and the only place the view builds a scripted
   child window.

Also settled on the way through (§5–§8): how a scripted window is created (`SC4CreateWindowFromScript`), what the
dialog layer already says and where this spec corrects it, the catch-all coverage owners named in the task brief, and
what the view does about the effects draw pass.

**Out of scope.** The modal loop itself (modal-dialogs.md §2), the advisor popup's gates and pause (advisors.md §6.8.1),
the UI script grammar and each control's vocabulary (INVENTORY-M6 §4.1, still unmade as a design decision), the MXF font
format, and every panel/budget/query window's own behaviour. Those are separate specs; this one says what they stand on.

---

## 2. The window object model *[compat]*

### 2.1 Three interfaces, one object

The framework window is `cIGZWin` (IID `0x22BA0121`), implemented by `cGZWinGen` (vtable `0x00ADC678`, 152 slots) for
windows built from a UI script. `cIGZWinGen` (IID `0x5386D516`) is the generator interface on the same object; its slot
3 `AsIGZWin` (`0x009990A5`) returns the `cIGZWin` view of it. The window manager is `cIGZWinMgr` (IID `0x5A4`), class
`cGZWinMgrW32`, vtable `0x00AE46E8`, framework service `0xA417445E` (modal-dialogs.md §2.1).

A game window such as `cSC4View3DWin` is **not** a `cGZWinGen`. It *contains* one. The two are reached through two
different vtables on the same allocation (§3.2). Every code path in §4 that manipulates a child window goes through the
`cIGZWin` sub-object, never through the game interface.

### 2.2 The `cIGZWin` slot set that the window layer actually uses *[compat]*

Full tables live in `docs/abi/interfaces.tsv` (`cIGZWin` 148 rows, `cIGZWinMgr` 64 rows, notes prefix `re-modal:`). The
slots this spec calls, with Windows targets and argument/return channels re-checked here:

| Call | Interface (IID) | Slot | Offset | Target | Arguments | Return |
|---|---|---|---|---|---|---|
| `QueryInterface` | `cIGZWin` (`0x22BA0121`) | 0 | `+0x0` | `0x00998F83` | `uint32 iid, void** out` | bool, **`AL` only** (upper EAX garbage) |
| `AddRef` | `cIGZWin` | 1 | `+0x4` | `0x008FB729` | — | uint32 (new count) in EAX |
| `Release` | `cIGZWin` | 2 | `+0x8` | `0x008FB72E` | — | uint32 (new count) in EAX |
| `GetParentWin` | `cIGZWin` | 11 | `+0x2C` | `0x0099B7EE` | — | `cIGZWin*` or null |
| `ChildAdd` | `cIGZWin` | 14 | `+0x38` | `0x0099EA66` | `cIGZWin* child` | bool, `AL` only |
| `ChildRemove` | `cIGZWin` | 15 | `+0x3C` | `0x0099EA6B` | `cIGZWin* child` | bool, `AL` only |
| `ChildDelete` | `cIGZWin` | 16 | `+0x40` | `0x0099B833` | `cIGZWin* child` | **none** (tail call, §4.1) |
| `GetChildWindowFromID` | `cIGZWin` | 34 | `+0x88` | `0x0099DE74` | `uint32 id` | `cIGZWin*` or null |
| `GetFlag` | `cIGZWin` | 67 | `+0x10C` | `0x0099BDBB` | `uint32 mask` | 0/1 in EAX (`(flags & mask) != 0`) |
| `SetFlag` | `cIGZWin` | 68 | `+0x110` | `0x0099DB6B` | `uint32 mask, bool` | the whole new flag word in EAX (single exit `0x0099DCD9` loads `+0xC8`; see widget-toolkit.md §3.1) |
| `ShowWindow` | `cIGZWin` | 69 | `+0x114` | `0x0099D1AA` | — | none useful |
| `HideWindow` | `cIGZWin` | 70 | `+0x118` | `0x0099D1EA` | — | none useful |
| `GetParam` | `cIGZWin` | 113 | `+0x1C4` | `0x0099D388` | `uint32 key, void** out` | bool, `AL` only |
| `AddMessageFilter` | `cIGZWin` | 116 | `+0x1D0` | `0x0099D832` | `cIGZWinMessageFilter*` | none useful |
| `RemoveMessageFilter` | `cIGZWin` | 117 | `+0x1D4` | `0x0099D865` | `cIGZWinMessageFilter*` | none useful |
| `GZOnCommand` | `cIGZWin` | 143 | `+0x23C` | `0x00999896` | `uint32 command, uint32 data` | none useful |
| `PullToFront` | `cIGZWin` | 23 | `+0x5C` | `0x0099B856` | — | none useful |

All are thiscall and pop their own arguments. **`ChildDelete` is a tail call, not a normal call** (§4.1) — that matters
for a replacement that wants to observe a child being destroyed.

### 2.3 `ChildDelete` is a forward *[compat]*

`0x0099B833` is four instructions:

```
0099b833  mov ecx, dword ptr [ecx + 4]    ; ECX = this+4
0099b836  mov eax, dword ptr [ecx]        ; its vtable
0099b838  jmp dword ptr [eax + 0x5c]      ; tail-call slot 23
```

So `cIGZWin::ChildDelete(child)` does not run on the window at all: it forwards, on the object held at the window's
`+4`, to **that object's slot 23**, which `docs/abi/interfaces.tsv` records as `cIGZWinMgr::DestroyWindow`
(`0x009DC4AC`, deferred to the condemned list while the manager's busy count is non-zero). *[inferred]* that the field
at window `+4` is the window manager pointer; the forward and its destination slot are measured, the field's identity
is not. What would settle it: the `cGZWin` constructor, or a watch on `+4` across `SetWindowManager` (slot 7,
`0x0099CA0B`).

Consequence: **`ChildDelete` destroys the child.** `ChildRemove` (slot 15) only detaches it. The catalog-menu close
path in §4 deliberately does Hide → Release → ChildDelete, so the window is detached, hidden and destroyed in one step.

---

## 3. `cSC4View3DWin`: identity, layout and the full interface

### 3.1 Identity

| Fact | Value | Evidence |
|---|---|---|
| Class name | `cSC4View3DWin` | `re/names_windows.tsv` (its own methods) |
| Class ID | `0x9A47B417` | INVENTORY-M6 §4.4 |
| Interface | `cISC4View3DWin` | INVENTORY-M6 §4.4; tutorial.md TU2 |
| Primary vtable | `0x00ABCE40`, **61 slots** (0–60) | `docs/abi/vtables.tsv` |
| Window sub-object | `cIGZWin` at **`+0x0C`** | `AsIGZWin` `0x005F8400`, §3.2 |
| Code size | 62 KB | INVENTORY-M6 §4.4 |
| Save record | `0xA9C8647C / 0xE9C8647E / 1`, `DoSave` `0x007EA5B0`, `DoLoad` `0x007EBFB0` | INVENTORY-M6 §4.4; save-records.md |
| Created by | `cWinSC4App::CreateCityScreen` (`0x004E0C20`–`0x004E0DD0`) | INVENTORY-M6 §2.1 |

### 3.2 Object layout *[compat]*

The decisive instruction is `cSC4View3DWin::AsIGZWin` at `0x005F8400`, which is slot 3 of `0x00ABCE40`:

```
005f8400  test ecx, ecx
005f8402  je   0x5f8408
005f8404  lea  eax, [ecx + 0xc]     ; return this + 0x0C
005f8407  ret
005f8408  xor  eax, eax
005f840a  ret
```

**`AsIGZWin()` returns `this + 0x0C`.** So the `cIGZWin` sub-object of a `cSC4View3DWin` starts at offset `+0x0C`, and
the primary vptr at `+0` is the `cISC4View3DWin` one. This is measured, not inferred: the whole body is five
instructions and was read on Windows.

| Offset | What | Evidence |
|---|---|---|
| `+0x00` | `cISC4View3DWin` vptr → `0x00ABCE40` | `CloseSecondaryMenu` `0x007E5D2D` does `mov eax,[esi]; call [eax+0xf0]` and `0x00ABCE40` slot 60 is `CloseTertiaryMenu` |
| `+0x08` | a `cIGZWinMessageFilter` sub-object *[inferred]* | `InvokeSecondaryMenu` `0x007E5CA1` does `lea eax,[edi+8]; call [edx+0x1d0]` — `AddMessageFilter` takes the **address** of the field, so the filter is inline at `+8`, not a pointer. Its vtable is not read here |
| `+0x0C` | `cIGZWin` / `cGZWinGen` sub-object (vptr → `0x00ABCBC0`; `0x00ADC678` is the `cGZWin` base table the base constructor writes first, view3d-window.md §1) | `AsIGZWin` `0x005F8404` |
| `+0x1F8` | secondary catalog-menu window, `cIGZWin*` or null | written by `InvokeSecondaryMenu` (stores `esi`), cleared at `0x007E5D5E`; read at `0x007E5D23` |
| `+0x1FC` | tertiary catalog-menu window, `cIGZWin*` or null | written by `InvokeTertiaryMenu` `0x007E5E70`, cleared at `0x007E5F1C`; read at `0x007E5E94` |
| `+0x200` | secondary menu kind | `InvokeSecondaryMenu` `0x007E5C1E` compares it to **arg2**; cleared at `0x007E5D68` |
| `+0x204` | tertiary menu kind | written by `InvokeTertiaryMenu` `0x007E5E76` (from `ebx` = **arg2**), cleared at `0x007E5F26` |
| `+0x208` | pointer to a two-dword catalog state, or null | `CloseTertiaryMenu` `0x007E5EA2`–`0x007E5EE7` writes `+0` and `+4` through it; only the tertiary path uses it |
| `+0x300` | object answering `QueryInterface(0x67C5BEDE)` (the view manager) | `GetViewManager` `0x007E60B1`–`0x007E60C2` |

Offsets `+0x1F8`…`+0x208` are confirmed **from both directions** — written by the `Invoke*` open path and cleared by the
`Close*` teardown path — and are safe to code against. `+0x08` and `+0x300` are single-site readings and are marked
accordingly.

### 3.3a Save behaviour *[compat]*

The window layer persists exactly one thing: the **View3D record**, written by `cSC4View3DWin::DoSave` (`0x007EA5B0`)
and read by `DoLoad` (`0x007EBFB0`). Its identity is **group `0xA9C8647C`, type `0xE9C8647E`, instance `1`**,
with a further integer `4` pushed alongside — the record header is built at `0x007EA5CD` (helper `0x005D9F00`) from
five stack arguments `(stream, 0xA9C8647C, 0xE9C8647E, 1, 4)`, then the body is written through `0x005D9670`, which
returns `AL` and is tested at `0x007EA5DB`; a false result skips the whole body. The first two things written are
**references**, not values: `this+0xF0` and `this+0xF4` (the latter written as `obj + 8` when non-null, else null),
pushed through `0x005D99A0`. Those are the camera control / renderer / model maker / simulator references
INVENTORY-M6 §4.4 and save-records.md describe; the rest of the body (four references in all, then the global
effect triggers) is view3d-window.md §6k.

What is *not* saved, and matters for a replacement:

- **No UI state.** The catalog menus (`+0x1F8`…`+0x208`), the current tool, the cursor text, the cost indicator and
  every panel's open/closed state are outside the record. A city saved with a catalog popup open loads with it closed.
- **No dialog state.** A modal dialog is a nested loop with no record of its own; saving is impossible mid-dialog in
  practice because `cSC4AutoGamePause` holds the simulator, and even if a save ran it would write a clean record.
- **The record is a reference record.** `DoLoad` resolves the references by class id — `0xA9C741B3` is
  `cSC4ModelMaker`, resolved at `0x007EC01C` (INVENTORY-M6 §4.4 / save-records §3.2). A replacement that does not
  implement those classes must give an exact opaque round trip, per save-records.

`4` is the record version and `1` the instance: `DoLoad` accepts versions below 5 (view3d-window.md §6k, re-view3dwin
2026-10-21).

### 3.3 `cISC4View3DWin` slot table, vtable `0x00ABCE40` *[compat]*

Argument lists are `arg_dwords` (dwords the **callee** pops, exclusive of `this`); `ret` is the return channel:
`eax` = a value in EAX that callers use, `eax-or-void` = EAX is not a usable status, `al` = a bool in AL only (upper
EAX undefined), `x87` = `st(0)`. `ret_confidence` is `medium` unless the body was read for this spec.

| Slot | Offset | Target | Method | pops | args | ret |
|---|---|---|---|---|---|---|
| 0 | `+0x0` | `007e5630` | `QueryInterface(uint32 iid, void** out)` | 8 | 2 | `al` (bool) |
| 1 | `+0x4` | `007e56d0` | `AddRef()` | 0 | 0 | `eax` (count) |
| 2 | `+0x8` | `007e56e0` | `Release()` | 0 | 0 | `eax` (count) |
| 3 | `+0xC` | `005f8400` | `AsIGZWin()` | 0 | 0 | `eax` = `this + 0x0C`, or null for a null `this` |
| 4 | `+0x10` | `0043fd00` | `GetRenderer()` | 0 | 0 | `eax` (object or null) |
| 5 | `+0x14` | `007f0fe0` | `GetModelMaker()` | 0 | 0 | `eax` (object or null) |
| 6 | `+0x18` | `007e60b0` | `GetViewManager()` | 0 | 0 | `eax` (object or null) |
| 7 | `+0x1C` | `007f1c10` | `ProcessCommand(uint32, uint32, uint32)` | 12 | 3 | `eax-or-void` |
| 8 | `+0x20` | `007e60e0` | `SetZoom(int32)` | 4 | 1 | `eax-or-void` |
| 9 | `+0x24` | `007e62c0` | `ZoomIn()` | 0 | 0 | `eax-or-void` |
| 10 | `+0x28` | `007e6320` | `ZoomOut()` | 0 | 0 | `eax-or-void` |
| 11 | `+0x2C` | `007e6150` | `SetZoomLimits(int32, int32)` | 8 | 2 | `eax-or-void` |
| 12 | `+0x30` | `007e5780` | `CanZoomIn()` | 0 | 0 | `eax` (0/1) |
| 13 | `+0x34` | `007e57a0` | `CanZoomOut()` | 0 | 0 | `eax` (0/1) |
| 14 | `+0x38` | `007e61d0` | `SetRotation(int32)` | 4 | 1 | `eax-or-void` |
| 15 | `+0x3C` | `007e6220` | `RotateLeft()` | 0 | 0 | `eax-or-void` |
| 16 | `+0x40` | `007e6270` | `RotateRight()` | 0 | 0 | `eax-or-void` |
| 17 | `+0x44` | `007e6380` | `SetZoomAndRotation(int32, int32)` | 8 | 2 | `eax-or-void` |
| 18 | `+0x48` | `007e57c0` | `GetCurrentViewFilterInfo(void* out)` | 4 | 1 | `eax-or-void`, `[inferred]` out-param |
| 19 | `+0x4C` | `007e63f0` | `SetCurrentViewFilterInfo(const void*)` | 4 | 1 | `eax-or-void` |
| 20 | `+0x50` | `007e57e0` | `GetCurrentViewLevelInfo(void* out)` | 4 | 1 | `eax-or-void`, `[inferred]` out-param |
| 21 | `+0x54` | `007e6500` | `SetCurrentViewLevelInfo(const void*)` | 4 | 1 | `eax-or-void` |
| 22 | `+0x58` | `007e57f0` | `GetCurrentViewLevel()` | 0 | 0 | `eax` (int) |
| 23 | `+0x5C` | `007e5800` | `SetCurrentViewLevel(int32)` | 4 | 1 | `eax-or-void` |
| 24 | `+0x60` | `007e65e0` | `GetCurrentViewInputControl()` | 0 | 0 | `eax` (object or null) |
| 25 | `+0x64` | `007ebd70` | `SetCurrentViewInputControl(cSC4ViewInputControl*, bool)` | 8 | 2 | `eax-or-void` |
| 26 | `+0x68` | `007e9390` | `RemoveCurrentViewInputControl(bool)` | 4 | 1 | `eax-or-void` |
| 27 | `+0x6C` | `007e5820` | `RemoveAllViewInputControls(bool)` | 4 | 1 | `eax-or-void` |
| 28 | `+0x70` | `007ebea0` | `AddPersistentViewInputControl(cSC4ViewInputControl*)` | 4 | 1 | `eax-or-void` |
| 29 | `+0x74` | `007e7fb0` | `RemovePersistentViewInputControl(cSC4ViewInputControl*)` | 4 | 1 | `eax-or-void` |
| 30 | `+0x78` | `007e8030` | `RemoveAllPersistentViewInputControls()` | 0 | 0 | `eax-or-void` |
| 31 | `+0x7C` | `007e56f0` | `GetKeyAccelerator()` | 0 | 0 | `eax` (object or null) |
| 32 | `+0x80` | `007e5700` | `SetKeyAccelerator(cIGZWin* or cRZWinKeyAccelerator*)` | 4 | 1 | `eax-or-void` |
| 33 | `+0x84` | `007f64f0` | `SetScrolling(int32, int32, int32)` | 12 | 3 | `eax-or-void` |
| 34 | `+0x88` | `007e5740` | `ScrollStop()` | 0 | 0 | `eax-or-void` |
| 35 | `+0x8C` | `007e7f90` | `KillKeyboardScrolling()` | 0 | 0 | `eax-or-void` |
| 36 | `+0x90` | `007e5840` | `PickTerrain(...)` | 16 | 4 | `eax-or-void` |
| 37 | `+0x94` | `007e5890` | `PickOccupant(...)` | 12 | 3 | `eax-or-void` |
| 38 | `+0x98` | `006f7dc0` | `DisplayTerrainPickDebugString(...)`: no effect, reads neither argument (view3d-window.md §6l, VW18) | 8 | 2 | folded empty method; no return contract |
| 39 | `+0x9C` | `007e5900` | `GetTerrainQueryEnabled()` | 0 | 0 | `eax` (0/1) |
| 40 | `+0xA0` | `007f5cf0` | `SetCursorText(...)` | 20 | 5 | `eax-or-void` |
| 41 | `+0xA4` | `007ef7a0` | `ClearCursorText(...)` | 4 | 1 | `eax-or-void` |
| 42 | `+0xA8` | `007ec800` | `SetCostIndicator(int32, int32)` | 8 | 2 | `eax-or-void` |
| 43 | `+0xAC` | `007f0ff0` | `SetCostIndicator(...)` (overload) | 4 | 1 | `eax-or-void` |
| 44 | `+0xB0` | `007ec900` | `ClearCostIndicator(...)` | 4 | 1 | `eax-or-void` |
| 45 | `+0xB4` | `00648f10` | `[inferred]` a debug dump hook | 4 | 1 | `eax-or-void` |
| 46 | `+0xB8` | `007eb850` | `GetGlobalEffectTriggerState(uint32)` | 4 | 1 | `eax` (state) |
| 47 | `+0xBC` | `007ecce0` | `AssertGlobalEffectTrigger(uint32)` | 4 | 1 | `eax-or-void` |
| 48 | `+0xC0` | `007eb8a0` | `RetractGlobalEffectTrigger(uint32)` | 4 | 1 | `eax-or-void` |
| 49 | `+0xC4` | `007f1010` | `GetCoverageHelper()` | 0 | 0 | `eax` (object or null) |
| 50 | `+0xC8` | `007e90f0` | `SetCursorTextPosition3D(...)` | 4 | 1 | `eax-or-void` |
| 51 | `+0xCC` | `007f1060` | `GetLastSelectedBuilding()` | 0 | 0 | `eax` (object or null) |
| 52 | `+0xD0` | `007f1080` | `GetLastSelectedLot()` | 0 | 0 | `eax` (object or null) |
| 53 | `+0xD4` | `007f1020` | `SetLastSelectedBuilding(...)` | 4 | 1 | `eax-or-void` |
| 54 | `+0xD8` | `007f1040` | `SetLastSelectedLot(...)` | 4 | 1 | `eax-or-void` |
| 55 | `+0xDC` | `007e91e0` | `MinimizeUI(bool)` | 4 | 1 | `eax-or-void` |
| 56 | `+0xE0` | `007ecf60` | `ShowMissionStatusPanel(...)` | 4 | 1 | `eax-or-void` |
| 57 | `+0xE4` | `007e77f0` | `HideMissionStatusPanel()` | 0 | 0 | `eax-or-void` |
| 58 | `+0xE8` | `007eb970` | `UpdateMissionStatusPanel()` | 0 | 0 | `eax-or-void` |
| **59** | `+0xEC` | `007e5d20` | **`CloseSecondaryMenu()`** | 0 | 0 | none (void) |
| **60** | `+0xF0` | `007e5e90` | **`CloseTertiaryMenu()`** | 0 | 0 | none (void) |

**Matches** tutorial.md TU2 and `docs/abi/interfaces.tsv` rows 3157–3158 for slots 59 and 60. **Corrects:**
`docs/abi/vtables.tsv` records slot 60's `ret` as `via-call` / `ret_confidence low`; the body at `0x007E5E90` ends in a
plain `ret` after a `pop ecx; pop esi`, so the return channel is **none (void)**, `ret_confidence` **high**.

Slot 3's target `0x005F8400` is recorded in `docs/abi/vtables.tsv` under the folded name
`icf::cSC4PropOccupantAutomataController::AsOccupant`. **Corrects:** that address is `cSC4View3DWin::AsIGZWin`
(`re/names_windows.tsv:4466` lists five folded names including it) and its body returns `this + 0x0C`. The Mac build
also calls it `AsIGZWin`.

---

## 4. The catalog menus: secondary and tertiary

The 3D view can open two levels of **catalog popup** over the city — the tool/build catalog (secondary) and a
sub-catalog (tertiary). They are ordinary scripted child windows of the view's `cIGZWin` sub-object, tracked in the
fields of §3.2, and the tutorial's `endTutorial` closes them by calling slots 59 and 60 (tutorial.md TU2).

### 4.1 `CloseTertiaryMenu` (slot 60, `0x007E5E90`) *[compat]*

thiscall, pops 0, returns nothing. Body `0x007E5E90`–`0x007E5F32` (163 bytes).

```
if (this->tertiaryWin == null) return;                     // +0x1FC, 0x007E5E94
if (this->catalogState != null) {                          // +0x208, 0x007E5EA2
    void* p = null;
    if (tertiaryWin->QueryInterface(0x8A2CAD8B, &p) && p != null) {   // slot 0, 0x007E5EBD
        catalogState->field0 = p->slot17();                // 0x007E5ECD, 0x007E5ED6
        catalogState->field1 = p->slot9();                 // 0x007E5EDE, 0x007E5EE7
        p->Release();                                      // slot 2, 0x007E5EF0
    }
}
tertiaryWin->HideWindow();                                 // slot 70, 0x007E5EFB
tertiaryWin->Release();                                    // slot 2,  0x007E5F09
this->AsIGZWin()->ChildDelete(tertiaryWin);                // slot 16 on this+0x0C, 0x007E5F19
this->tertiaryWin   = null;                                // +0x1FC, 0x007E5F1C
this->tertiaryKind  = 0;                                   // +0x204, 0x007E5F26
```

Measured details:

- The two saved values are written **through `this+0x208`**, an 8-byte structure, and only when that pointer is
  non-null. The order is slot 17 first, then slot 9. Their meaning (a scroll position, a selected item, a filter) is
  **not established**; the interface `0x8A2CAD8B` is unidentified. *[inferred]* it is the catalog widget's own state
  interface. What would settle it: name `0x8A2CAD8B` in `re/class_id_table.tsv`, or read the two slot bodies.
- `QueryInterface` here is the **2-argument** form (`iid, &out`) on slot 0, not the SDK's 3-argument COM form. The
  `out` pointer is written by the callee.
- The order Hide → Release → `ChildDelete` is fixed. `ChildDelete` is a tail forward (§2.3) that ends in
  `cIGZWinMgr::DestroyWindow`, so the child is destroyed, not merely detached. The `Release` before it balances the
  reference the view took when it stored the pointer.
- Both `+0x1FC` and `+0x204` are cleared **unconditionally** at the end, even on the early-out path through
  `0x007E5EF3`. Only the `null` check at the top returns without clearing.

### 4.2 `CloseSecondaryMenu` (slot 59, `0x007E5D20`) *[compat]*

thiscall, pops 0, returns nothing. Body `0x007E5D20`–`0x007E5D73` (84 bytes).

```
if (this->secondaryWin == null) return;                    // +0x1F8, 0x007E5D23
this->CloseTertiaryMenu();                                 // slot 60 first, 0x007E5D2F
secondaryWin->HideWindow();                                // slot 70, 0x007E5D3D
secondaryWin->Release();                                   // slot 2,  0x007E5D4B
this->AsIGZWin()->ChildDelete(secondaryWin);               // slot 16 on this+0x0C, 0x007E5D5B
this->secondaryWin  = null;                                // +0x1F8, 0x007E5D5E
this->secondaryKind = 0;                                   // +0x200, 0x007E5D68
```

**Closing the secondary menu always closes the tertiary one first.** That is the whole reason slot 59 calls slot 60: a
tertiary catalog is only reachable from a secondary one, so tearing the secondary down would otherwise orphan it. This
matches tutorial.md TU2 ("calls slot 60 first") and is the behaviour that makes "a tutorial that ends mid-tool does not
leave a detached catalog popup open" true.

Note the asymmetry: `CloseSecondaryMenu` does **not** touch `this+0x208`. The catalog state is only written back by the
tertiary path. A replacement must not "tidy up" by also writing it on the secondary path.

### 4.3 `InvokeSecondaryMenu` (slot ?, `0x007E5C10`) *[compat]*

Not a `cISC4View3DWin` slot — it is an ordinary member function of `cSC4View3DWin`. thiscall + 4 arguments
(`ret 0x10` at `0x007E5C3F`, the callee pops 16 bytes), no useful return.

```
this->RemoveAllViewInputControls(0);                       // slot 27, 0x007E5C1B
if (this->secondaryKind != 0) {                            // +0x200, 0x007E5C1E
    if (this->secondaryKind == arg2) {                     // 0x007E5C2C
        this->CloseSecondaryMenu();                        // slot 59 — toggle the same menu closed
        return;
    }
    this->CloseSecondaryMenu();                            // slot 59 — close a different menu first
}
win = SC4CreateWindowFromScript(..., this->AsIGZWin(), arg2, ...);   // 0x005F9390, parent = this+0x0C
if (win == null) goto end;
cIGZWinGen* gen = null;
win->QueryInterface(0x5386D516, &gen);                     // slot 0 — cIGZWinGen
if (gen != null) {
    win->AddMessageFilter(&this->filterAt0x08);            // slot 116, 0x007E5CA7
    gen->SetWinGenFlag(4, 0);                              // gen slot 6, 0x007E5CB7
    gen->SetWinGenFlag(1, 0);                              // gen slot 6, 0x007E5CC4
    gen->Release();                                        // slot 2
}
nSC4UI::AlignWindowToWindow(win->GetID(), 0xAAAA, argN, 0);          // 0x0078DB20
win->PullToFront();                                        // slot 23, 0x007E5CF3
win->ShowWindow();                                         // slot 69, 0x007E5CFA
```

Measured facts and their limits:

- **Toggle semantics.** Invoking the menu that is already open closes it and returns; invoking a different one closes
  the open one first, then opens the new one. Both branches call slot 59. This is why the tutorial's
  `CloseSecondaryMenu` is safe to call at any time.
- **`RemoveAllViewInputControls(0)` runs first**, before any test on the current menu. So merely *trying* to open a
  catalog menu drops the current tool. *[inferred]* the argument is a "force"/"silent" flag; slot 27's body was not
  read.
- **The parent of the new window is `this + 0x0C`**, i.e. the view's own `cIGZWin` sub-object — so the catalog popup is
  a *child window of the view*, which is what makes `ChildDelete` on that sub-object the right teardown.
- **`0x96A006B0` is written into the outgoing frame** at `0x007E5C61`. That is the UI script **group** used for every
  scripted window (INVENTORY-M6 §4.1: "Windows are built from text resources of type `0x00000000`, group `0x96A006B0`").
  This is the group argument of `SC4CreateWindowFromScript`, not a window ID.
- `SetWinGenFlag` is called for flags **4** and **1**, both cleared. *[inferred]* from the Mac name
  (`cGZWinGen::SetWinGenFlag(long, __bool8)`) that the slot pair is `(flag, value)`; the Windows target is
  `0x0099xxxx` inside `cGZWinGen`. What would settle it: the flag bit meanings (they are `cIGZWinGen` generator flags,
  distinct from `cIGZWin` `GetFlag`/`SetFlag` slot 67/68).
- `nSC4UI::AlignWindowToWindow` at `0x0078DB20` (name from `re/names_windows.tsv:7942`, Mac score 0.987) is called
  once, with `0xAAAA` as its second argument and `win->GetID()` (slot 63) as its first. Its third argument is
  **arg4 of the caller**, which at the two known call sites is `1` and a resource-shaped id — so it is a *target
  window id*, not a coordinate (§4.4). *[inferred]* `0xAAAA` is an anchor/alignment code. The fourth argument is `0`.
- The argument **order** into `SC4CreateWindowFromScript` is not fully settled: the callee takes 3 dwords (§5.1) and
  the caller cleans 12 bytes, but one of the three dwords it writes is an out-pointer local and the register saves
  interleave with the pushes. *[inferred]* the order is `(out_ptr, parent, instance)` from the push sequence
  (`push eax` / `push edx` / `push ebx`, right-to-left). What would settle it: read the three writes as one
  decompiled frame in a compiler-aware listing.

### 4.4 `InvokeTertiaryMenu` (`0x007E5D80`) *[compat]*

Byte-for-byte the same shape as §4.3 with four differences, all measured from `0x007E5D80`–`0x007E5E82`:

| | Secondary | Tertiary |
|---|---|---|
| Menu-kind field tested | `+0x200` (`0x007E5C1E`) | `+0x204` (`0x007E5D8E`) |
| Close slot on toggle/replace | 59 (`0x007E5C34`, `0x007E5C44`) | 60 (`0x007E5DA4`, `0x007E5DB4`) |
| Callee pops | `ret 0x10` = **4** args (`0x007E5C3F`) | `ret 0x14` = **5** args (`0x007E5DAF`) |
| Stored on success | `+0x1F8` = window | `+0x1FC` = window (`0x007E5E70`), `+0x204` = arg2 (`0x007E5E76`) |
| State pointer written on close | none | `+0x208` (§4.1) |

**The fifth argument, now identified (answers UIW-2 in part).** `InvokeTertiaryMenu` has exactly two callers
(`xrefs` to `0x007E5D80`, both inside `cSC4View3DWin`), and both push five values right-to-left:

| Call site | arg1 | arg2 | arg3 | arg4 | arg5 |
|---|---|---|---|---|---|
| `0x007E718A` | `0x9DE3002` | `0x9DE8798` | a variable (`[esp+0xC]`) | `1` | `-25` (`0xFFFFFFE7`) |
| `0x007F50A7` | `0xCB95403E` | `0xAB954023` | a variable (`[esp+0x34]`) | `0xAB9537B7` | `-75` (`0xFFFFFFB5`) |

So arg1 is a script-instance id, arg2 is the **menu kind** (it is what `ebx` holds and what is stored to `+0x204` at
`0x007E5E76`), arg3 is caller data, and **arg5 is a small negative integer** — the only argument the secondary form
lacks. arg5 does **not** reach `AlignWindowToWindow`, which is passed `arg4` (§4.3); it is consumed earlier in the body
or is unused. *[inferred]* it is a pixel margin/offset for the popup placement, which would explain the −25 / −75. What
would settle it: disassemble `0x007E5DC0`–`0x007E5E40`, the span between the create and the align call.

**`InvokeTertiaryMenu` names two different functions.** `re/names_windows.tsv` carries the name at **both**
`0x007E5D80` (the 5-argument one analysed above) and `0x007EAEB0`. `0x007EAEB0` is a second implementation with the
same toggle shape — it tests `+0x204` at `0x007EAEB6`, compares against `[esp+0x74]`, and calls slot 60 on both the
toggle and replace branches — but with a `0x6C` frame and **`ret 0x18` = 6 arguments**. The two callers of §4.4 call
`0x007E5D80` only. `0x007EAEB0` is the **catalog-view popup** with seven callers of its own (the building, catalog
and vehicle menus); view3d-window.md §6i specifies it, and `0x007E7270` is the matching secondary variant. Quirk 12.

**`AlignWindowToWindow` takes a window id to align to.** The tertiary tail (`0x007E5E40`–`0x007E5E57`) passes
`win->GetID()` (slot 63), `0xAAAA`, `arg4`, `0` — and `arg4` is `1` at one site and a resource-shaped id at the other,
which fits a **target window id** (with `1` as a special "screen"/"parent" id) far better than a coordinate.
*[inferred]* the signature is
`nSC4UI::AlignWindowToWindow(uint32 winID, uint32 anchor, uint32 targetWinID, uint32 flags)`.

---

## 5. Creating a scripted window

### 5.1 `SC4CreateWindowFromScript` (`0x005F9390`) *[compat]*

Three overloads exist (`0x005F9390`, `0x005F93F0`, `0x005F9480`); the one the view calls is the first. Decompiled
whole (Ghidra):

```c
undefined4 FUN_005f9390(undefined4 param_1, undefined4 param_2, undefined4 param_3)
{
  int *svc = 0;
  int *com = FUN_008793ec();                    // the framework / cGZCOM accessor
  if (com != 0)
      com->slot5(0x5A356E15, 0xFA3562FA, &svc); // GetSystemService(service, iid, &out)
  if (svc != 0) {
      u = svc->slot4(param_1, param_2, param_3); // cGZUIScriptService::CreateWindowFromScript
      FUN_004495c0();
      return u;
  }
  FUN_004495c0();
  return 0;
}
```

- It is a **3-argument** thunk. Its return is the window pointer in EAX, or `0` when the script service is missing.
- The service is looked up as service id `0x5A356E15`, interface id `0xFA3562FA`. The UI script service class is
  `cGZUIScriptService`, class id `0xFA3562FB` (INVENTORY-M6 §4.1) — the interface id is one less than the class id.
  Independently confirmed by `cRZSysServPtr<cIGZUIScriptService, 4197802746ul, 1513451029ul>`
  (`0x00449750`; `4197802746 = 0xFA3562FA`, `1513451029 = 0x5A356E15`).
- `FUN_004495C0` runs on both exits; *[inferred]* it releases a lock or a COM reference taken by `FUN_008793EC`.
- The work is `cGZUIScriptService` **slot 4**, three arguments, forwarded unchanged, **returning the window in EAX**.

**Name-collision trap — read this before using `re/names_windows.tsv`.** The name table attaches
`cGZUIScriptService::CreateWindowFromScript` to **two** different functions (`re/names_windows.tsv:12614`, `:12622`).
They are slots 3 and 4 of the service table `0x00AD5158` (corrected 2026-10-24, re-gzuiscript7; ui-script-service.md
§2–§3):

| Address | Signature (measured) | Returns | What it actually does |
|---|---|---|---|
| `0x0094B08B` (slot 3) | 5 dwords (`ret 0x14`), thiscall | **bool in `AL`** | `w = this->slot4(p1, p2, p3);` if `w`: `ok = w->QueryInterface(p4, p5); w->Release();` return `ok` |
| `0x0094B210` (slot 4) | 3 dwords: the key pointer (reads `+0`, `+4`, `+8` of it), parent, id | **`cIGZWin*` in EAX** | the actual factory; `SC4CreateWindowFromScript` calls it |

So `0x0094B08B` is a **QI wrapper that consumes the window it created** — it returns only whether the query worked,
and releases the window before returning. A caller that wants the window must call slot 4, which is what
`SC4CreateWindowFromScript` does. Confusing the two is the difference between getting a window and getting `AL`.

**Answers modal-dialogs.md open question 1, in part.** The question asks whether "the window ID passed to
`SC4CreateWindowFromScript` (`0x005F9390`, third argument) becomes the window's `GetID`". What is measured here:

- the function takes three arguments and passes all three to `cGZUIScriptService` slot 4 unchanged;
- the view's call site (`0x007E5C6D`) writes **`0x96A006B0`, the UI script group**, into the outgoing frame next to the
  parent window pointer and the instance value, so at least one of the three is a *group* and at least one is an
  *instance*, not a free-form ID;
- whether the resulting window's `GetID` (slot 63) equals any of the three is **still not measured**. *[inferred]* the
  instance argument becomes the window's ID, because `GetChildWindowFromID` (slot 34) must be able to find scripted
  children and `cGZLegacyControl` names controls by instance. What would settle it: read **slot 4** of
  `cGZUIScriptService` (not `0x0094B08B`) and see whether it calls `SetID` (slot 64).

Until that is checked, the dialog closer's rule stands: key on its own records, not on IDs (modal-dialogs.md §3.2).

### 5.2 The window tree of a city screen *[inferred]*

Assembled from §3.2, §4.3 and modal-dialogs.md; no single code path enumerates it.

```
canvas window (the Win32 client surface)
└── cSC4View3DWin  (a cIGZWin at its +0x0C, parent = the canvas)
    ├── the panels, menus and catalogs the view builds in AddGUI (0x007EEFE5)
    │   └── secondary catalog menu  (view+0x1F8)   ── child of view+0x0C
    │       └── tertiary catalog menu (view+0x1FC) ── child of view+0x0C
    └── modal dialogs (advisor popup, confirmations) ── attached to the
        main window by DoModalWin when they have no parent (modal-dialogs.md §2.2 step 1)
```

The two facts that are measured and that an implementer needs: a catalog menu's parent is the **view's own window
sub-object** (`0x007E5C50`, `0x007E5DC0`), and a dialog with no parent is attached to the **main window**
(modal-dialogs.md §2.2). Everything else in the diagram is composition inferred from those two.

---

## 6. Dialogs, the pause hook and the close path

This section is a **cross-reference plus corrections**. The behaviour is specified in modal-dialogs.md (the modal loop
and how a plugin ends a dialog) and advisors.md §6.8.1 (the advisor popup's gates, queue and pause). Read those two
first. What this spec adds is where that behaviour sits in the window tree, and four corrections.

### 6.1 What makes an advisor popup hold the city *[compat]*

The chain, each link cited:

1. The city view creates one `cSC4AdvisorPopupTracker` in `cSC4View3DWin::AddGUI` (`0x007EEFE5`), a 24-byte object at
   `0x00762BA0`–`0x007638E0` that is **not** a GZCOM class (advisors.md §6.8.1).
2. The tracker's `DoMessage` (`0x00763450`) queues an advice and, on its own tick, checks five gates and
   `SC4GetPopupModalDialogsSafe` (`0x005F9000`).
3. It creates the dialog from the UI script group `0x96A006B0`, instance `0x4A5A89D4` (mood < 3) or `0x4A5A89D5`
   (mood ≥ 3) — the same group the catalog menus use (§4.3).
4. It takes a scoped **hidden** pause: `cSC4AutoGamePause(sound no, animation no, simulator yes, clock yes)`
   (`0x005F9320`/`0x005F9360`) → `SC4GamePause` (`0x005F91C0`) → `cISC4Simulator` slot 10 `HiddenPause()` and the
   city clock's `HiddenPause(1)`. `IsPaused()` stays false; `IsHiddenPaused()` becomes true (advisors.md §6.8.1).
5. It runs `DoModalWin` (manager slot 41, `+0xA4`, `0x009DBDA3`), which blocks the handler but keeps every service
   ticking (modal-dialogs.md §2.2–§2.3).
6. The tracker's window procedure `DoWinProcMessage` (`0x00762D30`) is attached to the dialog and ends it (§6.2).

So "advisor popups call `HiddenPause()` on the simulator" is true because of step 4, and "the date stops while one is
open" is true because of steps 4–5 together. The `hidden` heartbeat field is the popup signal
(`docs/manual-checks.md` run 48).

### 6.2 Why Esc/Enter do nothing and a Close button does *[compat]*

`cSC4AdvisorPopupTracker::DoWinProcMessage` at `0x00762D30` handles **no keyboard messages**. Its first act on every
message is to read the advice the dialog is showing via the window property **`0xEA9F13C0`** (`cIGZWin` slot 113
`GetParam`, `0x0099D388`), then test that advice's flags:

| Advice state | Action |
|---|---|
| `CheckFlags(2)` false (no longer active) | end the dialog (`cSC4AdvisorPopupTracker::EndDialog`, `0x00762CD0`) |
| `CheckFlags(2)` true **and** `CheckFlags(8)` true (expire requested) | end the dialog |
| `CheckFlags(2)` true and `CheckFlags(8)` false | leave it open |
| a command message (type 3, data1 `0x287259F6`) | end it — **any** button ID (modal-dialogs.md §3.3) |
| a link message `0xE4A85D91` | follow the link, then end it |
| types `0xA2BF8AD5` / `0xA2BF8AD6` | swallowed (they are the win-proc attach/detach notices) |

This is exactly what the harness observes: **Esc and Enter produce no message this procedure looks at**, so nothing
happens; a click on Close is a **command message** with data1 `0x287259F6`, so the dialog ends. `docs/manual-checks.md`
run 48 records the observation ("Esc and Enter did not close them; clicking Close did"); this is the code that makes it
true.

**Corrects / clarifies the brief's premise about coordinates.** The close path is **message-based, not
coordinate-based**. There is no pixel test anywhere in `0x00762D30`, and no window geometry (`800x600`) or client
coordinate (`526,410`) appears in `docs/`, `engine/` or `tools/`. If a run was driven by clicking at a client point,
that point is whatever the *Close button's* hit rectangle is in the UI script instance `0x4A5A89D4`/`0x4A5A89D5`, and
what the game sees is the command message, not the coordinates. **DIFFERS** from any claim that the game closes an
advisor popup "at (526,410)". *[inferred]* (526,410) is the Close button's client position in the 800×600 layout and
was reached by a harness click; it is not an address-level fact. What would settle it: the UI script instance
`0x4A5A89D4`'s button rectangle, or a run that clicks at (526,410) and logs the posted command's data2 (the button ID).

### 6.3 `close_dialogs = 1` and the correct wording of "default button"

`docs/harness.md:124-125` says the closer "end[s] each game modal dialog **by its default button**".
**Corrects:** modal-dialogs.md §3.2 step 2 and `engine/README.md` both implement **button ID 0**, which is not
necessarily the default. For a confirmation, 0 is "no" (yes is `0x8A8DFD4E`); for a scripted dialog 0 is "cancel"; two
dialog families **ignore** button 0 entirely (terrain edge reconciliation, missing plugin packs) and need the
`EndModal` fallback. `docs/PROGRESS.md:119` repeats the same loose wording ("presses each modal dialog's default
button"). A spec-conformant description is: *the closer posts command `0x287259F6` with button ID 0, and falls back to
`EndModal(window, 0)` if no end notice arrives*.

### 6.4 The pause hook is not a popup counter *[compat]*

`cSC4AutoGamePause` is used by more than dialogs: `cSC4App::FullGamePause` (`0x00449380`), `cSC4App::PartialCloseCity`
(`0x00454A60`), `cGZMiscCommandSystemSupport::DoMessage` (`0x005C8B30`),
`SC4WriteCityRegionViewThumbnail` (`0x005DDEC0`), `cSC4View3DWin::DoTemporarySimHiddenPause` (`0x007E5B40`, camera
moves), city-situation popups and saves (clock24.md, advisors.md §6.8.1). The hidden count is a **nesting count**, so a
harness must not read it as "how many advisor popups are open". `cISC4Simulator` slot 16 `IsHiddenPaused`
(`0x00703D60`) is shared with the clock and answers "count at `+0x34` > 0".

### 6.5 Message and `DoMessage` contracts for the window layer *[compat]*

Every message a window or a dialog sees goes through one of two entry points, in this order (modal-dialogs.md §2.6):

| Entry | Interface / slot | Signature | Returns |
|---|---|---|---|
| `DoMessage` | `cIGZWin` slot 3 (`+0xC`), `0x0099A276` on `cGZWinGen` | `(cGZMessage& msg)` thiscall, pops 4 | **bool, `AL` only** |
| `DoWinProcMessage` | `cIGZWinProc` (the object a window's `SetWinProc` attached) | `(cIGZWin* win, cGZMessage& msg)` — **arg1 is the window**, not the message: `0x00762D32` loads `arg1` and uses it as `this` for `GetParam` (§6.2) | bool |
| `GZOnCommand` | `cIGZWin` slot 143 (`+0x23C`), `0x00999896` | `(uint32 command, uint32 data)` thiscall, pops 8 | none useful |
| `SendMsg` / `PostMsg` | `cIGZWin` slots 144–147 (`+0x240`…`+0x24C`) | `(cIGZWin* target, type, d1, d2, d3)` or `(cIGZWin* target, cGZMessage const&)` | `PostMsg` (slot 147) returns the manager's slot 15 result, not a status |

For a scripted window the order is: the window procedure first (`DoWinProcMessage`, then `DoWinMsg`), then the window's
own `DoMessage`, and a type-3 message reaches `GZOnCommand` (modal-dialogs.md §2.6).

The message vocabulary this layer switches on:

| Message | Kind | data1 | data2 | data3 | Who handles it | Action |
|---|---|---|---|---|---|---|
| `0x287259F6` | command (type 3) | `0x287259F6` | **button ID** | toggle state | every dialog's handler | end the dialog; for the advisor popup **any** button ID ends it |
| `0xE4A85D91` | link | — | — | — | advisor tracker `0x00762D30` | follow the link (`cSC4HTMLLinkHandler::DoURL`), then end |
| `0xA2BF8AD5` / `0xA2BF8AD6` | attach / detach notices | — | — | — | advisor tracker `0x00762D30` | **swallowed**. Not key down/up (modal-dialogs.md §6) |
| `0x62BA7B42` | legacy, sent | window ID | the window | nest count **before** | subscribers of the legacy message server | modal begin notice (modal-dialogs.md §2.5) |
| `0x62BA7B43` | legacy, sent | window ID | the window | nest count **after** | same | modal end notice |
| GZ type `0xF` → Win32 `0x700` | posted | — | the window (`wParam`) | the result (`lParam`) | `ModalMessageLoop` | end the modal loop (modal-dialogs.md §2.3) |

`cGZMessage` is passed by reference. `PostMessageToTarget` (manager slot 14/15) queues under the manager's lock and the
manager delivers from its own tick (slot 16), skipping windows that are no longer valid; `SendMessageToTarget` (slots
12/13) delivers synchronously. Neither returns a status in EAX. **There is no keyboard message in the advisor popup's
procedure at all** — that is why Esc and Enter do nothing (§6.2).

---

## 7. The draw pass from the window side

**Matches** effects.md §7 and EF2, settled 2026-10-16 by re-answers29, and **re-confirmed here**: `xrefs` to
`cSC4EffectsManager::Update(SC4DrawContext*)` (`0x00593120`) returns **exactly one** address, `0x00A9F25C`. That is
slot 3 of `cSC4EffectsManager::vftable_1` (vtable `0x00A9F250`, 5 slots — `re/labels_windows.tsv:268`), the manager's
**draw interface**, beside the main `cISC4EffectsManager` vtable `0x00A9F264`. The chain effects.md gives is therefore
complete and has no second route.

**Corrects the task's framing.** `0x005922A0` is **not** a 3D-view slot. It is
`cSC4EffectsManager::DeactivateUnusedVisualEffects` (`re/names_windows.tsv:3578`), called only from `0x00593120`,
which is only dispatched through `cSC4EffectsManager::vftable_1` slot 3. The 3D view's own draw-related slots are
nothing like it; the view's connection is that its paint is what causes the renderer to run the pass (below).

The draw interface, measured from `docs/abi/vtables.tsv`:

| Slot | Offset | Target | Method | pops | args | ret |
|---|---|---|---|---|---|---|
| 0 | `+0x0` | `0058fe20` | `QueryInterface(uint32, void**)` | 8 | 2 | `al` (bool) |
| 1 | `+0x4` | `0058fd60` | `AddRef()` | 0 | 0 | `eax` (count) |
| 2 | `+0x8` | `0058fd70` | `Release()` | 0 | 0 | `eax` (count) |
| 3 | `+0xC` | `00593120` | `Update(SC4DrawContext*)` | 4 | 1 | `eax-or-void` |
| 4 | `+0x10` | `00735290` | `Pick(...)` | 16 | 4 | `eax-or-void` |

What the **window layer** owes that chain: the draw pass runs because the city view paints. `cGZWinGen::GZPaint` is
slot 88 (`+0x158`, `0x009995E7`); `cGZWin::Plot` is slot 89, `PlotComposite` slot 123, `PlotPresent` slot 124
(`0x0099C498`); `GetDrawContext` is slot 93 (`0x0099BEF9`) and `GetBufferToDrawTo` slot 94. The renderer's own passes
are `cSC43DRender::DrawStaticView` (`0x007C7370`) and `DrawDynamicView` (`0x007C7830`). A city view that is never
painted never reaches `0x005922A0`, so a headless run has to synthesise the sweep exactly as effects.md EF2 says.

*[inferred]* that the view's paint path reaches the effects draw interface through the 3D renderer's `DrawDynamicView`
rather than by calling it directly; the window-layer slots and the effects-side chain are measured, the link between
`Plot`/`DrawDynamicView` and `vftable_1` slot 3 is not. What would settle it: the caller list of
`cSC43DRender::DrawDynamicView`'s draw-target loop, or a runtime breakpoint on `0x00593120`.

---

## 8. The catch-all coverage owners

The task brief asks whether `cSCResProperty`, `fn:getDefaultValue`, `WriteEntry`/`InterpretEntry` and `cGroupLayer`
belong to the UI window layer. `tools/coverage-map.tsv` already assigns three of the five away from UI; this section
records the verdict **with the caller evidence**, so the coverage map is not reopened by guesswork.

| Owner | coverage-map row | Verdict | Evidence |
|---|---|---|---|
| `fn:InterpretEntry` | `data formats (DBPF, exemplars)` → `data-formats.md` `formats/src/exemplar.rs` | **Not UI.** Exemplar text-form reader | Two bodies: `0x0067DFD0` (212 B, parses `ParentCohort=Key:{g,i,t}`) and `0x0067E780` (2319 B, parses `0xID:{"name"}=Type:reps:{values}`). Both `pop 0` (cdecl). Callers: only the two unnamed helpers `FUN_0067E140` / `FUN_0067F0D0`, reached only from `cSCResExemplarFactory::SerialRead` `0x0067F5B3` and `cSCResExemplarCohortFactory::SerialRead` `0x0067FB49` |
| `fn:WriteEntry` | same | **Not UI.** Exemplar text-form writer | Three bodies: `0x0067C590` (70 B, `"%s=Key:{0x%08x,0x%08x,0x%08x}\r\n"`), `0x0067C5E0` (51 B, `"%s=0x%08x\r\n"`), `0x0067CDD0` (2446 B, the typed writer). Callers: only `cSCResExemplarFactory::SerialWrite` `0x006801E8`/`0x0068022C`/`0x006802DD` and `cSCResExemplarCohortFactory::SerialWrite` `0x006806E2`/`0x00680839` |
| `cSCResProperty` | same | **Not UI.** A 56-byte (`0x38`) resource-value COM object built by the exemplar factories | 5 methods: `AddRef` `0x005FD060`, `Release` `0x005FD070`, `QueryInterface` `0x005FD080`, ctor `0x005FDE80`, dtor `0x005FE9E0`. **No `GetGZCLSID`.** Layout: `+0x00` primary vptr, `+0x04` `cRZVariant` sub-object (vftable_2 `0x00AA69E8`, 60+ slots), `+0x08..0x1F` variant body, `+0x20` `cGZPersistResource`. Size confirmed twice: the pool stride in `FUN_005FD6B0` is `0x38`, and `cSCResExemplarFactory.c` passes `0x38` to the allocator |
| `cGroupLayer` | `audio` → `audio` | **Not UI, and not a UI "layer" at all.** The sound group layer under the ambience layer | **Confirmed.** 10 methods, all 40-byte (`0x28`) spatial sound-group operations: `ClearEmptyGroups` `0x004F0A30`, `RemoveInstance` `0x004F0AC0`, `GetPositions` `0x004F0B40`, `_AddPositionToGroup` `0x004F0C20`, `Clear` `0x004F0DE0`, `InsertInstance` `0x004F0E80`, `Intersect` `0x004F0EA0`, `_AddInstanceToGroup` `0x004F10C0`, `InsertInstance` `0x004F1210`, `UpdateInstance` `0x004F12B0`. Every external caller is `cSC4AudioAmbienceLayer` (`BeginUpdate`/`EndUpdate`/`HandleCell`/`HandleOccupant`/`InsertOccupant`/`MoveOccupant`/`RemoveOccupant`/`Update`); it is created only by `cSC4AudioAmbienceLayer::_CreateLayers` `0x004E4F60`. **It has no vtable row in `docs/abi/vtables.tsv`** — it is not a `cGZWin` descendant, unlike every real window class in §14 |
| `fn:getDefaultValue` | `resource and exemplar editor` → `ui` | **Editor-only, not the window layer.** | One address only: `0x00670940`, 1177 B, `__cdecl` 4 dwords (`pop 0`), returns `AL`. Coerces a text default into a property through the variant setters (`+0x2C` `SetValBool`, `+0x38` `SetValUint8`, …). **Exactly one caller**: `cSCPropertyView::propertiesSectionCallback` `0x00672880`. Neighbours in the same object file are `getUnquotedString` `0x00670E10` and `convertStringToFloatValue<float>` `0x00670300` |

**DIFFERS** from the brief's framing ("if they belong to the UI layer, say so and give them real sections"): **four of
the five do not**, and the fifth belongs to the F8 developer tools that INVENTORY-M6 §2.7 lists as "not needed for a
standalone game". The three exemplar ones already have a real home in data-formats.md §4.3; `cGroupLayer` belongs with
audio (INVENTORY-M6 §5). None of them needs a section in a UI window spec.

**Corrects:** nothing in `docs/specs/` claims these are UI classes — the coverage map is already right. What *is* owed
is an old HANDOFF item ("split the catch-all owners … into map rows", the coverage2 lane;
docs/history/handoff-2026-10.md), which is a coverage-tooling task, not a spec
task. Two small gaps in **data-formats.md §4.3** are worth noting for whoever picks that up: it never names
`InterpretEntry`/`WriteEntry`/`cSCResProperty` (addresses only), and it does not record that the `ParentCohort`
**writer** `0x0067C590` prints the key as `instance, type, group` while the **reader** `0x0067DFD0` reads
`group, instance, type` — a real asymmetry.

---

## 9. Interface slot tables (ABI rows)

Rows for `cISC4View3DWin` slots 59 and 60 are **already** in `docs/abi/interfaces.tsv` (lines 3157–3158) and must not
be regenerated by hand here. This lane appends the *new* rows it derived — the complete `cISC4View3DWin` table, the
`AsIGZWin` correction, `SC4CreateWindowFromScript`'s service slot, and the two `Invoke*` members — to
`build-runs/pending-abi/re-specs19-rows.tsv` in `docs/abi/interfaces.tsv`'s exact 12-column layout. **Do not edit
`interfaces.tsv` itself**, and do not regenerate it (`docs/abi/README.md:58-62`: a regeneration drops the hand-written
`re-modal:` notes).

---

## 10. Quirks

1. **Closing the secondary menu closes the tertiary one first** (`0x007E5D2F` calls slot 60 before doing anything
   else). *[quirk]* Not a bug, but a replacement that tears down only the named menu will leave the other one alive
   and, because `ChildDelete` destroys it, the leftover is unreachable.
2. **`ChildDelete` is a tail forward through `this+4` to the manager's `DestroyWindow`** (§2.3). It is not
   overridable on the window, so a `cIGZWin` subclass cannot intercept its own child's destruction by overriding slot
   16. *[quirk]*
3. **`InvokeSecondaryMenu` drops the current tool before it decides anything** (`RemoveAllViewInputControls(0)` at
   `0x007E5C1B` is the first call). Opening a catalog menu that is already open still costs the player their tool.
   *[quirk]*
4. **`AsIGZWin` returns `this + 0x0C` even when `this` is non-null but the window is not initialised** — the only null
   check is on `this` itself (`0x005F8400`). A caller that stores the result must not treat "non-null" as "valid
   window"; use `cIGZWinMgr::IsWindowValid` (slot 24) as modal-dialogs.md §3.2 says. *[quirk]*
5. **The tertiary close writes catalog state through a raw pointer at `+0x208` with no lifetime management** visible in
   the close path. If that pointer is stale the write is a wild store; the close path never validates it. *[quirk]*
6. **`CloseTertiaryMenu` clears both `+0x1FC` and `+0x204` even when the `QueryInterface` for `0x8A2CAD8B` fails** —
   the window is destroyed either way and the state pointer is left untouched. A replacement that "helpfully" nulls
   `+0x208` would differ. *[quirk]*
7. **Advisor popups are closed by any button ID, not by a specific Close control** (modal-dialogs.md §3.3). A harness
   that sends button 0 is correct for them and wrong for two other dialog families that ignore it. *[quirk]*
8. **`SC4CreateWindowFromScript` returns 0 when the script service is missing** and still runs `FUN_004495C0` on that
   path — the cleanup is not conditional on success. *[quirk]*
9. **`docs/abi/vtables.tsv` misnames `0x005F8400`** as five folded occupant `AsOccupant` methods; it is
   `cSC4View3DWin::AsIGZWin`. Name-collision folding in `re/names_windows.tsv` is a known source of wrong names —
   always confirm with a body. *[quirk]* of the tooling, but it produced a wrong ABI row.
10. **`cGZUIScriptService::CreateWindowFromScript` names two functions and neither is the one you want.**
    `0x0094B08B` returns a bool in `AL` and **releases the window it just made**; `0x0094B210` takes a struct. The
    function that returns the window is the unnamed **slot 4**. Writing a shim against the name rather than the slot
    gives you a success flag and a freed window. *[quirk]*
11. **`cGroupLayer` is an audio class named like a UI one.** It has no window vtable at all. A name-driven layering
    pass that files `*Layer*` under UI will misfile it. *[quirk]* of the naming.
12. **`InvokeTertiaryMenu` names two functions with different arity.** `0x007E5D80` takes 5 dwords (`ret 0x14`) and
    `0x007EAEB0` takes 6 (`ret 0x18`); both toggle `+0x204` and call slot 60. Both have callers (view3d-window.md
    §6i). Writing a binding against the name rather than the address gets the wrong ABI. *[quirk]*
13. **Nothing in the window layer is saved** except the View3D reference record (§3.3a). A catalog popup open at save
    time is closed at load time — by design, not by an accident a replacement should "fix". *[quirk]*

---

## 11. Conformance tests

An implementer should be able to make all of these pass without asking a question. "The view" is a
`cISC4View3DWin`-shaped object; "the window" is its `AsIGZWin()` result. Where a test needs the original, it is marked
**[game]**; the rest run against a replacement.

1. **`AsIGZWin` offset.** For any non-null view, `AsIGZWin() == (char*)view + 0x0C`. For a null view it returns null.
   *(Body `0x005F8400`.)*
2. **Two vtables, one allocation.** `*(void**)view == 0x00ABCE40` and `*(void**)AsIGZWin() == 0x00ABCBC0` (or the
   replacement's equivalents), and the two are distinct objects within one allocation.
3. **Slots 59 and 60 return nothing.** Neither `CloseSecondaryMenu()` nor `CloseTertiaryMenu()` produces a value; a
   caller must not read EAX.
4. **Secondary close is a no-op when nothing is open.** With `+0x1F8 == 0`, `CloseSecondaryMenu()` must not call
   `CloseTertiaryMenu`, `HideWindow`, `Release` or `ChildDelete`, and must leave `+0x200` alone.
5. **Tertiary close is a no-op when nothing is open.** With `+0x1FC == 0`, `CloseTertiaryMenu()` must return without
   touching `+0x204`.
6. **Secondary close cascades.** With both menus open, `CloseSecondaryMenu()` must close the tertiary menu first
   (observe order: `CloseTertiaryMenu` before `HideWindow` on the secondary window).
7. **Teardown order.** For either close path the order must be `HideWindow` → `Release` → `ChildDelete`, then the
   field and the kind are cleared.
8. **Both fields cleared.** After `CloseTertiaryMenu()`, `+0x1FC == 0` **and** `+0x204 == 0`.
9. **Secondary does not write catalog state.** `CloseSecondaryMenu()` must not write through `+0x208`.
10. **Tertiary writes two dwords.** With `+0x208 != 0` and the window answering `QueryInterface(0x8A2CAD8B)`, the close
    path writes the slot-17 result to `state+0` and the slot-9 result to `state+4`, in that order, then releases the
    interface.
11. **Tertiary skips the save on a failed query.** If `QueryInterface(0x8A2CAD8B)` returns false, `+0x208` is not
    written but the window is still hidden, released and deleted.
12. **Invoke toggles.** With the same menu already open, `InvokeSecondaryMenu(..., same, ...)` closes it and does not
    create a new window. Test the parallel case for the tertiary menu.
13. **Invoke replaces.** With a different menu open, `Invoke*` closes the open one first (slot 59 / 60 observed before
    the create call), then creates the new one.
14. **Invoke drops the tool first.** `RemoveAllViewInputControls` is called before the open-menu test, even when the
    result is a toggle-close.
15. **Parent is the view's window.** The created catalog window's parent (`GetParentWin`, slot 11) is
    `AsIGZWin()` — i.e. `view + 0x0C`.
16. **Script group.** The create call passes group `0x96A006B0` (the UI script group), not a window ID.
17. **Null create is safe.** If `SC4CreateWindowFromScript` returns 0, `Invoke*` must not write `+0x1F8`/`+0x1FC`, must
    not call `AddMessageFilter`, and must still leave the previous menu closed.
18. **Generator flags.** On a successful create, the new window answers `QueryInterface(0x5386D516)` and flags 4 and 1
    are set to 0 on the resulting `cIGZWinGen`; the interface is released afterwards.
19. **Alignment and show order.** `nSC4UI::AlignWindowToWindow` (`0x0078DB20`) is called once per create, as
    `AlignWindowToWindow(win->GetID(), 0xAAAA, <caller arg4>, 0)` — its third argument a target window id (observed `1`
    and `0xAB9537B7`), not a coordinate — then `PullToFront` (slot 23) then `ShowWindow` (slot 69), and only then are
    the fields written: `+0x1F8`/`+0x1FC` = the window and `+0x200`/`+0x204` = arg2. A test that reads the fields must
    see the window already shown.
20. **Advisor popup closes on a button command** (any ID) and on the advice going inactive; **Esc and Enter must not
    close it.** *(This is the `docs/manual-checks.md` run 48 observation, expressed as a test.)* **[game]**
21. **`DoWinProcMessage` gets the window as arg1.** A win proc must be able to `GetParam` on its first argument and
    reach the dialog's properties (`0xEA9F13C0` for the advice). A shim that passes the message instead will fail.
22. **Message delivery order on a scripted window.** The window procedure (`DoWinProcMessage`) runs before the
    window's own `DoMessage`, and a type-3 command reaches `GZOnCommand` (slot 143) after both.
23. **The hidden pause is a nesting count, not a popup count.** While an advisor popup is open `IsHiddenPaused()` is
    true and `IsPaused()` is false, and the count returns to its previous value when the dialog ends. A camera move
    (`0x007E5B40`) raises the same count with no dialog open, so a harness must not equate the two. **[game]**
24. **Effects end messages need a paint.** With the city view never painted, `0x005BFD70` is never reached and no
    effect end message is posted; with a paint pass it is. **[game]** *(effects.md EF2; this test is the window-side
    consequence.)*
25. **Only the View3D record is saved, and it is a reference record.** `DoSave` writes group `0xA9C8647C` /
    type `0xE9C8647E` / instance `1` and nothing else from the window layer: a city saved with a catalog popup open
    (`+0x1F8` or `+0x1FC` non-null) loads with both null. A round trip must keep the camera-control / renderer /
    model-maker / simulator references resolvable by class id.

---

## 12. Dependencies

- **modal-dialogs.md** — the modal loop, `DoModalWin`/`EndModal`, the begin/end notices, the dialog-family close table.
  This spec does not restate them; it states what the window tree looks like underneath.
- **advisors.md §6.8.1** — the advisor popup tracker, its queue, its five gates and the hidden pause. §6.1–§6.2 above
  add the window-side explanation and the property/flag detail.
- **tutorial.md TU2** — slots 59/60 as the `endTutorial` teardown. **Matches.**
- **effects.md §7 / EF2** — the draw pass chain `0x005922A0` → `0x00593120` → `0x00A9F25C`. **Matches.**
- **INVENTORY-M6 §4.1, §4.4** — the UI script resources (group `0x96A006B0`), the control manager, and the class list
  §13.
- **data-formats.md §4.3** — the exemplar text form (`InterpretEntry`/`WriteEntry`), which §8 places outside this layer.
- **save-records.md** — the View3D record `0xA9C8647C / 0xE9C8647E / 1`; this spec does not touch its contents.

## 13. Replaceability

- The **window object model** (§2) is a small, stable surface: `cIGZWin`'s child/flag/param slots and the manager's
  modal slots. A replacement needs those semantics, not the 152-slot `cGZWinGen` surface. *[choice]* implement
  `cIGZWin`'s slots 0–3, 11, 14–18, 23, 34, 63–70, 113, 116–117, 143, 146–147 and nothing else until a panel needs it.
- **`cISC4View3DWin`** (§3.3) is the interface the rest of the game and FreeCity4's own plug-in use to drive the city
  screen. Slots 4–6 (renderer, model maker, view manager) are the ones the simulation-adjacent code reaches for;
  slots 8–35 are camera and tool control; 40–55 are presentation; 55–60 are the UI panels and the catalog menus. A
  headless host needs only slots 59–60 (to satisfy `endTutorial`) plus whatever its tool layer uses.
- The **catalog menus** (§4) are pure UI. They can be replaced by any popup that is a child of `AsIGZWin()` and is
  torn down by Hide → Release → ChildDelete, as long as the toggle semantics of §4.3–§4.4 are kept (a tool is dropped
  on every invoke).
- The **UI script interpreter** (`cGZUIScriptService`, `cGZLegacyControl`) is the unmade decision of INVENTORY-M6 §4.1.
  Nothing in §2–§4 depends on interpreting the shipped scripts: the only thing the window layer needs is
  "create a window whose parent is X and whose instance/group are G/I", which is §5.1.

## 14. UI window class inventory

Specified in detail in this document: `cSC4View3DWin` / `cISC4View3DWin`, `cGZWinGen` / `cIGZWin` / `cIGZWinGen`,
`cGZWinMgrW32` / `cIGZWinMgr` (cross-referenced), `cSC4AdvisorPopupTracker`, `cSC4AutoGamePause` / `SC4GamePause`,
`SC4CreateWindowFromScript` / `cGZUIScriptService` (as far as §5.1), `nSC4UI` (`AlignWindowToWindow`),
`cSC4EffectsManager` (draw-pass side only).

Everything below is **named and sized, not specified** — it is the ranked reading list an implementer needs before S4.
`fns` = functions attributed to the class in `re/names_windows.tsv`; `bytes` = summed function-body bytes from
`decomp/index.tsv`; `vtables` from `docs/abi/vtables.tsv` (`—` = no attributed vtable row). These are reading-effort
figures in the sense of INVENTORY-M6 §0, not port sizes.

### 14.1 The toolkit (needed to describe any window)

| Class | Vtables | fns | bytes | What |
|---|---|---|---|---|
| `cGZWin` | `—` | 161 | 12 804 | The **base window**: parenting and child list, coordinates, hit testing, mouse/keyboard routing, accelerators, cursors, `Plot`/blit/fill, packed-RGB conversion. The 152-slot `cIGZWin` surface of §2.2 is this class |
| `cGZWinGen` | `00adc678` | 55 | 6 960 | The **general/script-built window**: caption, gutters, colour bars and dividers, title and side bars, `place_controls`, `calc_user_area`, `SetWinProc`, background bitmap |
| `cGZWinCtrlMgr` | `00adc1ec`, `00adc1d4`, `00adc128` | 40 | 6 602 | Control factory and registry: `CreateButton`/`CheckBox`/`Label`/`WinBMP`/`Slider`/`Scrollbar`/`Spinner`/`LineInput`/`OptGrp`, font/BMP/colour defaults, `RegisterCtrlCreator`. Set up by `cSC4App::SetupWinCtrlMgr` `0x0044D810` |
| `cGZWinMgrBase` / `cGZWinMgrW32` | `00ae48d8` / `00ae46b4` | 9 / 4 | 1 469 / 526 | Window manager: `OnTick`, `GZMsgBox`, `DoModalWin` (modal-dialogs.md §2), `GZWinMsgPost`, `GZShowCursor` |
| `cGZLegacyControl` | `00ad5360`, `00ad5324` | 56 | **54 189** | The **UI-script control vocabulary** — `InterpretPropertiesGZWin*` / `ControlVocabularyGZWin*` for every widget type (`Btn`, `BMP`, `Combo`, `ListBox`, `Slider`, `LineInput`, `Spinner`, `Outline`, `Folders`, `TextTicker`, …) plus `InterpretPropertiesSC3WinGen`. This is the single biggest piece of the "interpret the shipped UI scripts" decision |
| `cGZUIScriptService` | `00ad5158` + 5 more | 28 | 2 775 | UI-script interpreter: `ExecuteScriptFile`/`Resource`/`String`, `CreateWindowFromScript` (§5.1), `ScriptBegin`/`Tag`/`End`, `PushParent`/`PopParent`, `SetLegacyWinProc` |
| `cGZWinGenDlg` | `00ae0240`, `00ae01e8` | 28 | 3 128 | Procedural dialog builder: `AddListBox`/`AddBtn`/`AddCombo`/`AddLineInput`/`AddSlider`/`AddSpinner`/`AddText`/`AddTextEdit`/`AddCheck`/`AddOptionGroup`/`AddLineGraph`/`AddBarGraph`/`AddScatterGraph`, `SetCommandCallback` |
| `cGZWinDlg` / `cGZWinMsgBox` | `00ae2398` / `00ae2648` | 3 / 12 | 152 / 3 397 | Minimal dialog base; the message box (`Init`, `create_and_add_button`, `GZPaint`, `GZOnCommand`) |
| `cGZFont` / `cGZFontSys` | `00ac55d8` / `—` | 70 / 13 | 10 823 / 4 455 | Font object (glyph cache, `CharWidth`/`GetAdvanceWidth`, code page, antialiasing/gamma, `CalculateCharsToFitInWidth`, `FormatForEllipsis`) and the font system (`AddAllFontsFromDirectory`, `FontAcquire`, `AddFont`, `AddSubstitution`). **The MXF format is still a spec gap** (INVENTORY-M6 §4.2) |

Framework widgets, needed to describe the toolkit but not game UI: `cGZWinBMP` (24 fns / 1 378 B), `cGZWinBtn`
(39 / 5 326), `cGZWinCombo` (52 / 4 612), `cGZWinGrid` (61 / 20 377), `cGZWinLineInput` (59 / 5 876), `cGZWinListBox`
(33 / 5 485), `cGZWinOptGrp` (44 / 4 659), `cGZWinScrollbar2` (34 / 6 185), `cGZWinSlider` (29 / 2 925),
`cGZWinSpinner` (48 / 4 348), `cGZWinText` (13 / 1 879), `cGZWinTextEdit` (136 / **19 459**), `cGZWinTextTicker`
(11 / 1 761), `cGZWinToolTip` (10 / 1 996), `cGZWinTreeView` (34 / 10 706), `cGZWinPieMenu` (14 / 2 640),
`cGZWinDragAndDropManager` (30 / 3 116), `cRZWinKeyAccelerator` (16 / 801), `cRZCursorManager` (18 / 914).

### 14.2 Screens and the city view (S2–S4)

| Class | Vtables | fns | bytes | What |
|---|---|---|---|---|
| `cSC4View3DWin` | `00abce40`, `00abcbc0` | **149** | **62 066** | The city screen. §3 |
| `cWinSC4App` | `00a92f28` | 13 | 2 113 | The top-level app window (INVENTORY-M6 §2.1) |
| `cSC4WinIntroVideoScreen` | `00ab78f0` | 12 | 1 845 | Maxis-logo and splash video at boot (`SetupMaxisLogoScreen`, `SetupSplashScreen`, `DoUserStop`) |
| `cSC4WinSplashScreen` | `00ab9eb8`, `00ab9ea4` | 12 | 1 277 | Script-driven splash/title (`SetupScriptWindow`, `TransitionTimerCallback`) |
| `cSC4WinRegionScreen` | `00ab9260`, `00ab923c` | 36 | 20 760 | The region view screen (`LoadRegion`, `LaunchCity`/`LaunchRegionalCity`, `LoadGUI`, `MakeWholeRegion`, signposts, `GZOnCommand`) |
| `cSC4WinRegionView` | `00ab9644` | 24 | 7 925 | The region playfield widget (`RenderTile`, `GetCityAtPoint`, `SelectCity`, `InitSignpost`, `SetScrollPos`) |
| `cSC4ProgressScreen` | `—` | 5 | 1 874 | Progress/wait screen; created only from `cSC4View3DWin::CreateProgressScreen` `0x007E5BDB` |
| `cSC4WinGameDelay` | `00aa56c0` | 3 | 161 | Invisible delay window (`SC4GameDelay` `0x005F9140`; modal-dialogs.md §3.4) |

### 14.3 Panels, data views and dialogs (S5)

| Class | Vtables | fns | bytes | What |
|---|---|---|---|---|
| `cSC4SubBudgetWindow` | `00ab5494` | 32 | **71 235** | Neighbour/ordinance/individual funding dialog, built procedurally (`SetupWindowBig`/`Indiv`, `add_comboBox`, `updateOrdinanceBudget`) |
| `cSC4BudgetUIWinProc` | `00ab4a40`, `00ab4a2c`, `00ab4a1c` | 21 | 17 952 | Budget window (`SetTaxes`, `InitBalanceSheet`, `DisplayLoanWindow`, `DoSubBudgetWindow`) |
| `cSC4BudgetUIExpenseWinProc` / `…IncomeWinProc` | `00ab499c` / `00ab49e0` | 3 / 3 | 479 / 733 | Budget tabs |
| `cSC4WinMapView` | `00ab7e60`, `00ab7ee0` | 22 | 19 792 | The large data-map overlay (`DisplayLegend`, `DisplayStandardUint8Grid`/`Sint8Grid`, `SetDataView`, `ShowLargeWindow`) |
| `cSC4WinSnapshotAlbum` | `00ab9e54` | 40 | 22 565 | Snapshot album browser |
| `cSC4WinLotConfigurationEditor` | `—` | **92** | **51 054** | Lot-configuration editor (F8 — not needed) |
| `cSC4VideoPreferencesWinProc` | `00ab55f8` | 8 | 14 965 | Video preferences modal |
| `cSC4AudioPreferencesWinProc` | `00a94264` | 20 | 11 020 | Audio preferences modal |
| `cSC4MySimWinProc` | `—` | 24 | 14 731 | MySim (advisor avatar) window |
| `cSC4AdvisorUIAdvisorWinProc` | `00a83548`, `00a83538` | 21 | 9 712 | Advisor window |
| `cSC4WinMiniMap` | `00ab837c` + 2 | 28 | 7 889 | Minimap |
| `cSC4WinCalloutBox` | `00ab6770` | 7 | 6 879 | Tutorial/label callout box |
| `cSC4GraphsViewWinProc` | `00ab4fe0` | 13 | 8 864 | Graphs panel (`ShowGraphType`, `SetTimeScale`, `RefreshGraph`, `SetExpanded`) |
| `cSC4WinSnapshotTaker` | `00ab9bf8` | 18 | 6 010 | Snapshot capture |
| `cSC4TutorialUIWinProc` | `00a86378` | 17 | 6 695 | Tutorial overlay |
| `cSC4NewsTickerWinProc` | `00ab5150` | 8 | 6 754 | News ticker (`DisplayNewsFlipperLine`, `DoFlipperMessage`) |
| `cSC4CheatCodeEntryDialog` | `00ab7078`, `00ab7020` | 10 | 2 308 | Cheat-code entry |
| `cSC4WinTrendBar` | `00aba430`, `00aba68c` | 12 | 2 492 | Trend/graph bar |
| `cSC4WinAdviceList` | `00ab5880`, `00ab58b0` | 13 | 2 663 | Advisor message list |
| `cSC4WinCatalogView` | `00ab6d88` | 14 | 2 064 | Lot/prop catalog view |
| `cSC4ToolTipManager` | `00abcb60` | 7 | 1 658 | Tooltips |
| `cSC4WinAlertBorder` / `cSC4WinRCI` | `00ab5b48` / `00ab8628` | 3 / 5 | 304 / 945 | Alert border flash; RCI demand bar |
| `cSC4MissingPluginPacksDialog` | `—` | 1 | 435 | Missing-content warning (`DoDialog`, from `cSC4MissingPluginDataReporter::DoPostCityInit` `0x0048C84B`); [reporter caller contract](missing-plugin-reporter.md), dialog internals remain outside that contract |

Region dialogs and the remaining winprocs (`cSC4NewRegionDialog`, `cSC4DeleteCityDialog`, `cSC4DeleteRegionDialog`,
`cSC4ImportCityDialog`, `cSC4LoadRegionDialog`, `cSC4PickSimWinProc`, `cSC4QueryDialogWinProc`
`00a90b58`, `cSC4LegendDialogWinProc` `00a90980`, `cSC4CreditsWinProc`, `cSC4EstablishCityWinProc`,
`cSC4BuildingSelectWinProc`, `cSC4GamePreferencesWinProc`, `cSC4TipOfTheDayWinProc`, `cSC4PanelUIWinProc`,
`cSC4TimeDisplayMiniWindowWinProc`, `cSC4DialogWinProcBase`, `cSLWinLocationSaver`) are each under 3 KB except
`cSC4GamePreferencesWinProc` (2 655 B) and are listed so the set is complete.

**Not to be specified** (INVENTORY-M6 §2.7, F7/F8): `cGZWinHTML`, the IME family, `cGZWinFileBrowser`, the Lua debugger
windows, `cSC4WinAnimationRecorder`, `cSC4WinProcSpy`, the Game of Life easter egg, and the 2D blitters.

## 15. Open questions

1. **UIW-1:** What the two values written through `view+0x208` on a tertiary close mean, and what interface
   `0x8A2CAD8B` is (§4.1). Settle by naming the interface or reading the slot-17 and slot-9 bodies.
2. **UIW-2:** The **fifth** argument of `InvokeTertiaryMenu` is a small negative integer (−25 at `0x007E7179`, −75 at
   `0x007F5093`), the only argument the secondary form lacks; it does not reach `AlignWindowToWindow`. What it is
   consumed by (§4.4). Settle by disassembling `0x007E5DC0`–`0x007E5E40`.
3. **UIW-3:** Whether the window's `GetID` becomes the instance argument of the create call (this is
   modal-dialogs.md open question 1, still open). §5.1 now establishes that `SC4CreateWindowFromScript` calls
   **slot 4** of `cGZUIScriptService` — not the method the name table calls `CreateWindowFromScript` (`0x0094B08B`,
   a 5-argument QI wrapper that returns `AL` and releases the window). Settle by reading slot 4 and checking for a
   `SetID` (slot 64).
4. **UIW-4:** What the `cIGZWinGen` generator flags 4 and 1 mean when cleared (§4.3).
5. **UIW-5:** What `0xAAAA` means to `nSC4UI::AlignWindowToWindow` (§4.3). Its third argument is now known to be a
   target window id (§4.4); the anchor code is still unnamed.
6. **UIW-6:** Whether the view's paint path reaches `cSC4EffectsManager::Update` directly or through the 3D renderer's
   draw pass (§7).
7. **UIW-7:** The identity of the field at `cIGZWin + 4` that `ChildDelete` forwards through (§2.3) — presumably the
   window manager.
8. **UIW-8:** Whether (526,410) is the Close button's client position in UI script instance `0x4A5A89D4`, and what
   button ID that click posts (§6.2).
9. **UIW-9:** The vtable and full slot set of `cSC4View3DWin`'s message-filter sub-object at `+0x08` (§3.2).
10. **UIW-10:** The MXF font format (INVENTORY-M6 §4.2) and the UI script grammar (§4.1). The MXF format is now
    drawing-fonts.md §14 (re-fonts, 2026-10-23); the UI script grammar is still unspecified and on the critical
    path for S4.

---

## 16. The x87 precision trigger (foundations §18–§19)

The task brief asked whether the unknown "what raises the main thread to 53-bit during app init" lands here. It does,
and the answer is worth stating plainly: **nothing in the shipped exe raises it after `PreFrameWorkInit`.** The
trigger is not in this binary.

### 16.1 What the exe does, measured

The complete chain that writes the precision on the way into the game, every step read from the bytes:

```
__start 0x009F15DC (PE AddressOfEntryPoint)
  -> 0x009F171A  call __cinit(1)
  -> 0x009F1E04  __cinit: call [0x00B18B88]     ; = __fpmath, first, before the C++ static-init tables
  -> 0x009EEEE5  __fpmath: P5 fdiv test, then call __setdefaultprecision
  -> 0x009F6C30  __setdefaultprecision: push 0x30000 (_MCW_PC); push 0x10000 (_PC_53)
  -> 0x00A02F9B  __controlfp
  -> 0x00A02F69  __control87:  fldcw [ebp+0xc]  at 0x00A02F93   <-- the write
```

So the CRT raises the thread to 53-bit **before `WinMain`**. Then:

```
0x009F175B  call WinMain (0x0044C2B0; ret 0x10)
  -> cSC4App::PreFrameWorkInit 0x00452360
     -> 0x0045236C  call 0x005F85E0     ; the FIRST call after the prologue
        -> 0x005F85E0: push 0; call 0x008929E0; pop ecx; ret   ; helper(0) = 24-bit
```

`0x005F85E0` has **exactly one xref in the whole image**, `0x0045236C`, and it *lowers* precision. The CRT is
statically linked (MSVC 7.1 / VS .NET 2003 Release: PE linker version 7.10, string `Microsoft Visual C++ Runtime
Library` at `0x00AEADA0`), so there is no `_controlfp` import; the 13 imported DLLs are all system libraries with no
CRT surface at all.

### 16.2 Every control-word writer in the image

A function-granularity sweep finds **137 `fldcw`, 84 `fnstcw`, 2 `fnclex`, 1 `fldenv`**, and **no `fninit`, `finit`,
`frstor`, `fxsave`/`fxrstor`**. (That absence matters: `fninit` would give `0x037F` — 64-bit — not the measured
`0x027F`.) The writers that change **precision** are exactly these:

| Writer | Address (`fldcw`) | Modes written | Notes |
|---|---|---|---|
| `__control87` | `0x00A02F69` (`0x00A02F93`) | whatever `_controlfp` computes | 1 caller, `__controlfp` |
| `__controlfp` | `0x00A02F9B` | — | 1 caller, `__setdefaultprecision` |
| `__setdefaultprecision` | `0x009F6C30` | `_PC_53` | 1 caller, `__fpmath` |
| RZ helper | `0x008929E0` (`0x00892A05`) | 0 = 24-bit, 1 = 53-bit, 2 = 64-bit | 246 call sites; below |
| `__CIcos` / `__CIsin` / `__CItan` | `0x009F58F0` / `0x009F59A0` / `0x009F5A50` (`0x009F591C` / `0x009F59CC` / `0x009F5A7C`) | **forces `0x027F` = 53-bit** | §16.3 |
| `__ctrlfp` | `0x009FF2E3` (`0x009FF2FE`) | 64-bit (`0x133F`/`0x173F`/`0x1B3F`) or a restore | 22 callers |

The RZ helper's 246 call sites decode as: **1** `push 0` (the `PreFrameWorkInit` assert), **120** `push 1` (each
paired with a getter call to `0x00892A0A`), **124** register restores, 1 unresolved (`0x008B76CE`). That matches
foundations §18.2's "24→53→24 around every Lua call, and if already 53 the guard skips both".

The one **unguarded** 53-bit raise that never restores is the additive text parser `0x0067BA40` (`nSCRes` family):
its only setter call is `push 1` at `0x0067BA65`, at entry, with no restore on the way out. That is foundations §18.1
row 4's leak, confirmed. But `nSCRes::ParseFloat` (`0x0067BBA0`) has 185 call sites in 58 owner functions and
**every one is an effect/particle/camera `*::Parse` or `nSCRes::ParseColor`/`ParseColorAlpha`/`ParseVector3`/
`cSetCommand::Parse`** — text resource parsing, not application init.

**So: the shipped exe has no path that raises precision between `PreAppInit` and `PostAppInit`.** The open item in
foundations §18.2 / §19.4 is therefore correctly framed as a question about **FreeCity4.dll**, and static reading of
`SimCity4.exe` cannot close it. What would settle it is the minimal-DLL test foundations §19.4 already names,
instrumented: breakpoints on **every** writer in the table above **and the same breakpoints in `FreeCity4.dll` and
every module it loads**, taken between `PreAppInit` and `PostAppInit`, logging module, EIP and the value loaded. If
none fires and the control word still becomes `0x027F`, the writer is inside a newly loaded DLL's own CRT, which a
`LoadLibrary`/`LdrLoadDll` breakpoint plus `GetThreadContext` polling at framework-service boundaries would then
pinpoint. *[inferred]* the most likely cause is a DLL that `FreeCity4.dll`'s presence causes to load, whose CRT runs
`_fpmath(1)` on attach; `engine/plugin/FC4Checked.cpp:75` (`LoadLibraryExW(L"dbghelp.dll", …)`) is the one
plugin-side `LoadLibrary` visible in a quick scan. Nothing in this subsection is a runtime measurement; `CW = 0x027F`
is the project's own run measurement (foundations §18.2), not this lane's.

### 16.3 **Corrects:** foundations §18.1 on `sin`/`cos`/`tan`

**DIFFERS.** `docs/specs/foundations.md:2016-2018` says the C library's x87 math helpers "keep the caller's precision
(they only force round-to-nearest and masked exceptions while they run)". `__CIcos` (`0x009F58F0`), `__CIsin`
(`0x009F59A0`) and `__CItan` (`0x009F5A50`) do **not**:

```
fnstcw [esp]
cmp    word ptr [esp], 0x27f
je     skip
fldcw  word ptr [0x00AEA388]      ; .rdata word = 0x027F = _PC_53
```

They force **53-bit** whenever the caller is not already at 53-bit, then restore through `__math_exit` (`0x009FABBB`)
or the restore stub `0x009FABAE` (`cmp [esp],0x27f / je / fldcw [esp]`). So precision is *not* preserved across them,
though it is not permanently raised either. The functions that **do** preserve the caller's precision are
`lib::__load_CW` `0x009FAB25` (`(arg & 0x300) | 0x7f`) and its four callers `0x009EFCCC`, `0x009F00BC`,
`0x009F03DC`, `0x009F1D20` — which is what foundations' `fsqrt` note is describing.

**This is the one place the trigger question does touch the UI layer.** The only callers of `__CIcos`/`__CIsin`/
`__CItan` in the whole image are `cGZPieGraph::GetLineFromCenterOfDiskToEdge` (`0x009B2E3B`, `0x009B2E5B`,
`0x009B2EA3`, `0x009B2EC3`) and `FUN_009b27bd` (`0x009B2851`) — **pie-graph drawing**, a UI widget (INVENTORY-M6 §4.1
lists `cGZPieGraph` under U3 "graphs"). A pie graph drawn while the thread is at 24-bit will see `sin`/`cos` computed
at 53-bit and then truncated back to 24-bit on restore, so the drawing itself is safe, but a test that compares
intermediate values across those calls is not. Neither path is on the init route, so neither can be the app-init
trigger.

Two tooling notes for whoever picks this up: `decomp/_global.c` mislabels `0x009EEF04` as `_lua_gettop` — the bytes
there are the classic `_ftol`; and there is no `_fpreset`/`fninit` equivalent anywhere in the image.
