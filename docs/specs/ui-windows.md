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
| `SetFlag` | `cIGZWin` | 68 | `+0x110` | `0x0099DB6B` | `uint32 mask, bool` | 0/1 in EAX |
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
| `+0x0C` | `cIGZWin` / `cGZWinGen` sub-object (vptr → `0x00ADC678`) | `AsIGZWin` `0x005F8404` |
| `+0x1F8` | secondary catalog-menu window, `cIGZWin*` or null | written by `InvokeSecondaryMenu` (stores `esi`), cleared at `0x007E5D5E`; read at `0x007E5D23` |
| `+0x1FC` | tertiary catalog-menu window, `cIGZWin*` or null | written by `InvokeTertiaryMenu` `0x007E5E70`, cleared at `0x007E5F1C`; read at `0x007E5E94` |
| `+0x200` | secondary menu kind | `InvokeSecondaryMenu` `0x007E5C1E` compares it to **arg2**; cleared at `0x007E5D68` |
| `+0x204` | tertiary menu kind | written by `InvokeTertiaryMenu` `0x007E5E76` (from `ebx` = **arg2**), cleared at `0x007E5F26` |
| `+0x208` | pointer to a two-dword catalog state, or null | `CloseTertiaryMenu` `0x007E5EA2`–`0x007E5EE7` writes `+0` and `+4` through it; only the tertiary path uses it |
| `+0x300` | object answering `QueryInterface(0x67C5BEDE)` (the view manager) | `GetViewManager` `0x007E60B1`–`0x007E60C2` |

Offsets `+0x1F8`…`+0x208` are confirmed **from both directions** — written by the `Invoke*` open path and cleared by the
`Close*` teardown path — and are safe to code against. `+0x08` and `+0x300` are single-site readings and are marked
accordingly.

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
| 38 | `+0x98` | `006f7dc0` | `DisplayTerrainPickDebugString(...)` | 8 | 2 | `[inferred]` name folded with 8 others |
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
- `FUN_004495C0` runs on both exits; *[inferred]* it releases a lock or a COM reference taken by `FUN_008793EC`.
- The work is `cGZUIScriptService` **slot 4**, three arguments, forwarded unchanged.

**Answers modal-dialogs.md open question 1, in part.** The question asks whether "the window ID passed to
`SC4CreateWindowFromScript` (`0x005F9390`, third argument) becomes the window's `GetID`". What is measured here:

- the function takes three arguments and passes all three to `cGZUIScriptService` slot 4 unchanged;
- the view's call site (`0x007E5C6D`) writes **`0x96A006B0`, the UI script group**, into the outgoing frame next to the
  parent window pointer and the instance value, so at least one of the three is a *group* and at least one is an
  *instance*, not a free-form ID;
- whether the resulting window's `GetID` (slot 63) equals any of the three is **still not measured**. *[inferred]* the
  instance argument becomes the window's ID, because `GetChildWindowFromID` (slot 34) must be able to find scripted
  children and `cGZLegacyControl` names controls by instance. What would settle it: read
  `cGZUIScriptService::CreateWindowFromScript` (slot 4 of the service) and see whether it calls `SetID` (slot 64).

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
records the verdict so the coverage map is not reopened by guesswork.

| Owner | coverage-map row | Verdict for this spec | Evidence |
|---|---|---|---|
| `fn:InterpretEntry` | `data formats (DBPF, exemplars)` → `data-formats.md` `formats/src/exemplar.rs` | **Not UI.** Exemplar text form reader | `tools/coverage-map.tsv:247`; `InterpretEntry` `0x0067DFD0` is named in that row's comment |
| `fn:WriteEntry` | same | **Not UI.** Exemplar text form writer | `tools/coverage-map.tsv:248` |
| `cSCResProperty` | same | **Not UI.** The resource property object created by the exemplar factories | `tools/coverage-map.tsv:251` |
| `cGroupLayer` | `audio` → `audio` | **Not UI, and not a UI "layer" at all.** The sound group layer under the ambience layer | `tools/coverage-map.tsv:284-285`: "every caller is `cSC4AudioAmbienceLayer`". Its methods are `ClearEmptyGroups` `0x004F0A30`, `RemoveInstance` `0x004F0AC0`, `GetPositions` `0x004F0B40`, `_AddPositionToGroup` `0x004F0C20`, `Clear` `0x004F0DE0`, `InsertInstance` `0x004F0E80` (`re/names_windows.tsv:2274-2280`) |
| `fn:getDefaultValue` | `resource and exemplar editor` → `ui` | **Editor-only, not the window layer.** Part of the `cSCPropertyView` object file (`0x0066F8E0`–`0x00671080`), along with `fn:getUnquotedString` and `fn:convertStringToFloatValue` | `tools/coverage-map.tsv:389-392` |

**DIFFERS** from the brief's framing ("if they belong to the UI layer, say so and give them real sections"): **four of
the five do not**, and the fifth belongs to the F8 developer tools that INVENTORY-M6 §2.7 lists as "not needed for a
standalone game". The three exemplar ones already have a real home in data-formats.md §4.3; `cGroupLayer` belongs with
audio (INVENTORY-M6 §5). None of them needs a section in a UI window spec.

**Corrects:** nothing in `docs/specs/` claims these are UI classes — the coverage map is already right. What *is* owed
is HANDOFF.md's item 4 ("split the catch-all owners … into map rows"), which is a coverage-tooling task, not a spec
task. This spec records the verdict so a later coverage lane does not have to re-derive it.

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

---

## 11. Conformance tests

An implementer should be able to make all of these pass without asking a question. "The view" is a
`cISC4View3DWin`-shaped object; "the window" is its `AsIGZWin()` result. Where a test needs the original, it is marked
**[game]**; the rest run against a replacement.

1. **`AsIGZWin` offset.** For any non-null view, `AsIGZWin() == (char*)view + 0x0C`. For a null view it returns null.
   *(Body `0x005F8400`.)*
2. **Two vtables, one allocation.** `*(void**)view == 0x00ABCE40` and `*(void**)AsIGZWin() == 0x00ADC678` (or the
   replacement's equivalents), and the two are distinct objects within one allocation.
3. **Slot 59 is void.** `CloseSecondaryMenu()` returns no value; a caller must not read EAX.
4. **Slot 60 is void.** `CloseTertiaryMenu()` returns no value.
5. **Secondary close is a no-op when nothing is open.** With `+0x1F8 == 0`, `CloseSecondaryMenu()` must not call
   `CloseTertiaryMenu`, `HideWindow`, `Release` or `ChildDelete`, and must leave `+0x200` alone.
6. **Tertiary close is a no-op when nothing is open.** With `+0x1FC == 0`, `CloseTertiaryMenu()` must return without
   touching `+0x204`.
7. **Secondary close cascades.** With both menus open, `CloseSecondaryMenu()` must close the tertiary menu first
   (observe order: `CloseTertiaryMenu` before `HideWindow` on the secondary window).
8. **Teardown order.** For either close path the order must be `HideWindow` → `Release` → `ChildDelete`, then the
   field and the kind are cleared.
9. **Both fields cleared.** After `CloseTertiaryMenu()`, `+0x1FC == 0` **and** `+0x204 == 0`.
10. **Secondary does not write catalog state.** `CloseSecondaryMenu()` must not write through `+0x208`.
11. **Tertiary writes two dwords.** With `+0x208 != 0` and the window answering `QueryInterface(0x8A2CAD8B)`, the close
    path writes the slot-17 result to `state+0` and the slot-9 result to `state+4`, in that order, then releases the
    interface.
12. **Tertiary skips the save on a failed query.** If `QueryInterface(0x8A2CAD8B)` returns false, `+0x208` is not
    written but the window is still hidden, released and deleted.
13. **Invoke toggles.** With the same menu already open, `InvokeSecondaryMenu(..., same, ...)` closes it and does not
    create a new window. Test the parallel case for the tertiary menu.
14. **Invoke replaces.** With a different menu open, `Invoke*` closes the open one first (slot 59 / 60 observed before
    the create call), then creates the new one.
15. **Invoke drops the tool first.** `RemoveAllViewInputControls` is called before the open-menu test, even when the
    result is a toggle-close.
16. **Parent is the view's window.** The created catalog window's parent (`GetParentWin`, slot 11) is
    `AsIGZWin()` — i.e. `view + 0x0C`.
17. **Script group.** The create call passes group `0x96A006B0` (the UI script group), not a window ID.
18. **Null create is safe.** If `SC4CreateWindowFromScript` returns 0, `Invoke*` must not write `+0x1F8`/`+0x1FC`, must
    not call `AddMessageFilter`, and must still leave the previous menu closed.
19. **Generator flags.** On a successful create, the new window answers `QueryInterface(0x5386D516)` and flags 4 and 1
    are set to 0 on the resulting `cIGZWinGen`; the interface is released afterwards.
20. **Alignment call.** `nSC4UI::AlignWindowToWindow` (`0x0078DB20`) is called once per create, as
    `AlignWindowToWindow(win->GetID(), 0xAAAA, <caller arg4>, 0)`, before `PullToFront` and `ShowWindow`. Its third
    argument is a target window id (observed `1` and `0xAB9537B7`), not a coordinate.
21. **Show and store order.** `PullToFront` (slot 23) then `ShowWindow` (slot 69), and only then are the fields
    written: `+0x1F8`/`+0x1FC` = the window and `+0x200`/`+0x204` = arg2. A test that reads the fields must see the
    window already shown.
22. **Advisor popup closes on a button command** (any ID) and on the advice going inactive; **Esc and Enter must not
    close it.** *(This is the `docs/manual-checks.md` run 48 observation, expressed as a test.)* **[game]**
23. **Advisor popup takes a hidden pause, not a pause.** While one is open, `IsHiddenPaused()` is true and
    `IsPaused()` is false, and the count returns to its previous value when the dialog ends. **[game]**
24. **The hidden count is not a popup count.** A camera move (`0x007E5B40`) raises it with no dialog open; a harness
    must not equate the two.
25. **Effects end messages need a paint.** With the city view never painted, `0x005BFD70` is never reached and no
    effect end message is posted; with a paint pass it is. **[game]** *(effects.md EF2; this test is the window-side
    consequence.)*

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

## 14. Class list covered

Specified here in detail: `cSC4View3DWin` / `cISC4View3DWin`, `cGZWinGen` / `cIGZWin` / `cIGZWinGen`,
`cGZWinMgrW32` / `cIGZWinMgr` (cross-referenced), `cSC4AdvisorPopupTracker`, `cSC4AutoGamePause` / `SC4GamePause`,
`SC4CreateWindowFromScript` / `cGZUIScriptService` (as far as §5.1), `nSC4UI` (`AlignWindowToWindow`),
`cSC4EffectsManager` (draw-pass side only).

Named and placed, but not specified: `cSC4WinGameDelay`, `cSC4WinIntroVideoScreen`, `cSC4WinSplashScreen`,
`cSC4WinMapView`, `cSC4WinRegionScreen`, `cSC4WinRegionView`, `cSC4ProgressScreen`, `cSC4BudgetUIWinProc` and its
expense/income sub-procs, `cSC4SubBudgetWindow`, `cSC4GraphsViewWinProc`, `cSC4NewsTickerWinProc`,
`cSC4AdvisorUIAdvisorWinProc`, `cSC4AdvisorUIImageWinProc`, `cSC4TutorialUIWinProc`, `cSC4MySimBalloonWinProc`,
`cSC4QueryDialogWinProc`, `cSC4LegendDialogWinProc`, `cSC4DialogWinProcBase`, `cSC4VideoPreferencesWinProc`,
`cSC4AudioPreferencesWinProc`, `cSC4CheatCodeEntryDialog`, `cSC4MissingPluginPacksDialog`, `cGZWinGenDlg`,
`cGZWinDlg`, `cGZWinMsgBox`, `cGZWinCtrlMgr`, `cGZLegacyControl`, `cGZFont`, `cGZFontSys`. Each is at
INVENTORY-M6 §4.5–§4.7's level (a class, a size, a role) and needs its own section before an implementer can build it.

## 15. Open questions

1. **UIW-1:** What the two values written through `view+0x208` on a tertiary close mean, and what interface
   `0x8A2CAD8B` is (§4.1). Settle by naming the interface or reading the slot-17 and slot-9 bodies.
2. **UIW-2:** The **fifth** argument of `InvokeTertiaryMenu` is a small negative integer (−25 at `0x007E7179`, −75 at
   `0x007F5093`), the only argument the secondary form lacks; it does not reach `AlignWindowToWindow`. What it is
   consumed by (§4.4). Settle by disassembling `0x007E5DC0`–`0x007E5E40`.
3. **UIW-3:** The exact argument order of the `SC4CreateWindowFromScript` call in `InvokeSecondaryMenu`, and whether
   the window's `GetID` becomes the instance argument (§4.3, §5.1 — this is modal-dialogs.md open question 1, still
   open). Settle by reading `cGZUIScriptService::CreateWindowFromScript`.
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
10. **UIW-10:** The MXF font format (INVENTORY-M6 §4.2) and the UI script grammar (§4.1) — both still unspecified, and
    both are on the critical path for S4.
