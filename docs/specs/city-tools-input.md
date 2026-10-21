# City tools and input: the view input control framework and the place-lot tool (behaviour spec)

Status: behaviour spec, written 2026-10-21 (re-citytools4 lane). Every fact was checked on the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`); every address is a Windows address. Layouts and numeric
details were checked against the instructions. Implementers work from this spec, the public LGPL gzcom-dll SDK
headers and general knowledge only; they must **not** consult decompiled or disassembled code.

Tags: *[compat]* observable, a replacement must match it; *[choice]* a FreeCity4 decision; *[inferred]* consistent
with all the evidence, not proven; *[quirk]* original behaviour that looks unintended but is reproduced.

Outline: §1 scope; §2 the base class `cSC4ViewInputControl` (identity, layout, slot table, lifecycle, cursor,
capture, stack queries, result sound); §3 the place-lot tool `cSC4ViewInputControlPlaceLot` (construction, Init,
Shutdown, messages, key, mouse); §4 quirks; §5 open questions; §6 conformance tests; §7 coverage.

## 1. Scope

A *view input control* is the object the city's 3D view (`cSC4View3DWin`, ui-windows.md §3) forwards mouse and key
input to while a tool is active. The 3D view keeps a stack of them (`SetCurrentViewInputControl` slot 25,
`RemoveCurrentViewInputControl` slot 26, persistent controls slots 28-30; ui-windows.md §3.3). The stack itself is
the 3D view's (not specified here). This spec covers:

- the shared base `cSC4ViewInputControl` (17 functions), which every tool class derives from;
- the place-lot tool (plop a building or lot from the catalog), its lifecycle and input dispatch.

Not here: the network tools' back end (network-tools.md; `cSC4NetworkToolUI::DoPlacement` §8.2 there), the
dispatch tool (dispatch.md), the back ends of zone, demolish, disaster, flora tools (their specs), the window and
widget layer (ui-windows.md).

## 2. `cSC4ViewInputControl` (base)

### 2.1 Identity *[compat]*

- Primary vtable `0x00AA6658` (30 slots). `QueryInterface` (`0x005FB100`) answers IID `0x00000001` (IGZUnknown) and
  `0xC6BB8345` (the view-input-control interface), each with `AddRef`; any other IID returns false.
- Reference counting is the shared GZ pattern: `AddRef` increments `+0x08`; `Release` (`0x004B9E20`) decrements
  while the count is above 1, and on the last reference calls `FinalRelease` (slot 29, `0x005FB0E0`), which calls
  `Shutdown` (slot 4) and then the deleting destructor (slot 26) with flag 1.
- Derived constructors run the base constructor first, then install their own vtables (§3.1). The base
  constructor body was not re-read for this spec (Q CT4).

### 2.2 Layout *[compat]* (the derived tools read these offsets directly)

| Offset | Type | Meaning |
|---|---|---|
| `+0x00` | vtable | |
| `+0x04` | bool | initialised: set true by `Init`, false by `Shutdown` |
| `+0x08` | uint32 | reference count |
| `+0x0C` | uint32 | control ID (slot 5 `GetID` returns it; slot 6 `SetID(id)` stores it) |
| `+0x10` | uint32 | cursor resource ID |
| `+0x14` | cIGZCursor* | cursor object (held: AddRef on store, Release on replace); slot 7 returns it |
| `+0x18` | cIGZWin* | the window the control works in (slot 10 `SetWindow`, stored without AddRef) |
| `+0x1C` | cISC4View3DWin* | the city 3D view, taken from the global 3D-view pointer `0x00B43DC4` at `Init` (held) |
| `+0x20` | cIGZWinMgr* | the window manager (`RZ::WM()`), taken at `Init` (held) |

### 2.3 Slot table (vtable `0x00AA6658`) *[compat]*

Names follow the SDK's `cISC4ViewInputControl` order; "stub" rows are shared one-instruction bodies.

| Slot | Off | Target | Method | Pop | Base behaviour |
|---|---|---|---|---|---|
| 0 | +0x00 | `005fb100` | QueryInterface(iid, out) | 8 | §2.1 |
| 1 | +0x04 | `004bdf00` | AddRef | 0 | `++[+0x08]` |
| 2 | +0x08 | `004b9e20` | Release | 0 | §2.1 |
| 3 | +0x0C | `005fb1d0` | Init() | 0 | §2.4, returns true |
| 4 | +0x10 | `005fafd0` | Shutdown() | 0 | §2.4, returns true |
| 5 | +0x14 | `00517f70` | GetID() | 0 | returns `+0x0C` |
| 6 | +0x18 | `00989960` | SetID(id) | 4 | stores `+0x0C` |
| 7 | +0x1C | `0040ce70` | GetCursor() | 0 | returns `+0x14` (no AddRef) |
| 8 | +0x20 | `005fb070` | SetCursor(cursorId) | 4 | §2.5 |
| 9 | +0x24 | `005fb020` | SetCursor(cIGZCursor*) | 4 | §2.5 |
| 10 | +0x28 | `005fb0b0` | SetWindow(win) | 4 | stores `+0x18` |
| 11 | +0x2C | `00648ef0` | IsSelfScrollingView() | 0 | stub: false |
| 12 | +0x30 | `00648f00` | ShouldStack() | 0 | stub: true |
| 13 | +0x34 | `005deb40` | OnCharacter(ch) | 4 | stub: false |
| 14 | +0x38 | `0062c980` | OnKeyDown(vkey, modifiers) | 8 | stub: false |
| 15 | +0x3C | `0062c980` | OnKeyUp(vkey, modifiers) | 8 | stub: false |
| 16-19 | +0x40..+0x4C | `005be2e0` | OnMouseDownL / DownR / UpL / UpR (x, z, modifiers) | 12 | stub: false |
| 20 | +0x50 | `005be2e0` | OnMouseMove(x, z, modifiers) | 12 | stub: false |
| 21 | +0x54 | `00735290` | OnMouseWheel(x, z, modifiers, delta) | 16 | stub: false |
| 22 | +0x58 | `00648ef0` | OnMouseExit() | 0 | stub: false |
| 23 | +0x5C | `00407990` | Activate() | 0 | stub: nothing |
| 24 | +0x60 | `00407990` | Deactivate() | 0 | stub: nothing |
| 25 | +0x64 | `005fb170` | AmCapturing() | 0 | §2.6 |
| 26 | +0x68 | `005fb1b0` | deleting destructor(flags) | 4 | resets vtable, frees when `flags & 1` |
| 27 | +0x6C | `005fb0c0` | EndInput() | 0 | §2.7 |
| 28 | +0x70 | `005fb260` | PlayResultSound(soundId) | 4 | §2.8 |
| 29 | +0x74 | `005fb0e0` | FinalRelease() | 0 | §2.1 |

Input handlers return a bool: true means "handled" (the 3D view stops looking for another handler)
*[inferred]* (Q CT1). `x`, `z` are window-client pixel coordinates passed through to `PickTerrain` (3D view
slot 36).

Further non-virtual helpers: `IsKeyToBePassed` (`0x00505BE0`) returns true; `SetCapture` (`0x005FB130`) and
`ReleaseCapture` (`0x005FB150`), `IsOnTop` (`0x005FB190`), and the non-deleting destructor `0x005FAFC0`
(vtable reset only).

### 2.4 Init and Shutdown *[compat]*

`Init` (`0x005FB1D0`), only when not yet initialised:

1. If no cursor object is held and the cursor ID is non-zero: ask the window manager's cursor manager
   *[inferred name]* (WM slot 32 `+0x80`, then its slot 9 `+0x24`(id)) for the cursor and hold it (AddRef if non-null).
2. If a cursor is held and a window is set: window slot 85 (`+0x154`, set cursor)(cursor, true).
3. Hold the 3D view from `0x00B43DC4` (AddRef when non-null; null is stored as is).
4. Hold the window manager (`RZ::WM()`, AddRef; no null test).
5. Set initialised. Returns true either way.

`Shutdown` (`0x005FAFD0`), only when initialised: release and null, in this order, the window manager `+0x20`,
the 3D view `+0x1C`, the cursor `+0x14`; clear initialised. Returns true. The window `+0x18` is not released (it was
never held) and the cursor ID is kept, so a later `Init` reloads the same cursor.

### 2.5 SetCursor *[compat]*

- `SetCursor(cIGZCursor* c)` (slot 9): nothing when `c` equals the held cursor. Otherwise AddRef `c` (if non-null),
  Release the old one, store `c`; then if `c` is non-null, the control is initialised and a window is set, call the
  window's set-cursor slot (`+0x154`)(c, true). Setting null releases the old cursor without telling the window.
- `SetCursor(id)` (slot 8): look the ID up through the window manager's cursor manager (as §2.4 step 1). Only if
  the manager exists and returns a cursor: store `id` at `+0x10` and call slot 9 with that cursor. An unknown ID
  leaves both the ID and the cursor unchanged.

### 2.6 Capture and the stack top *[compat]*

- `SetCapture()`: with a window set, window manager slot 29 (`+0x74`)(window); returns that result; no window:
  false. `ReleaseCapture()`: the same with slot 30 (`+0x78`).
- `AmCapturing()` (slot 25): with a window set, true when window manager slot 28 (`+0x70`, current capture
  window) equals the window; no window: false.
- `IsOnTop()`: true when the 3D view's `GetCurrentViewInputControl` (slot 24) returns this control. No null test
  on the 3D view *[quirk]*: calling it before `Init` (or with no city view) faults in the original; a
  replacement returns false *[choice]*.

### 2.7 EndInput *[compat]*

`EndInput()` (slot 27): if the 3D view is held, call its `RemoveCurrentViewInputControl(false)` (slot 26). It
removes whatever control is on top of the stack, not necessarily this one *[compat]*; tools call it on themselves
while they are on top.

### 2.8 PlayResultSound *[compat]*

`PlayResultSound(soundId)` (slot 28): when `soundId != 0` and the sound system (`cIGZSndSys`, global
`0x00B43CB8`, audio.md) exists, call its slot 11 (`+0x2C`) with `(1, soundId, 0, 0, 0)`. No other effect. The base
class does not call it itself; derived tools call it after an action.

## 3. The place-lot tool `cSC4ViewInputControlPlaceLot`

### 3.1 Identity and creation *[compat]*

- Object size `0x158`. Two vtables: primary `0x00A90738` (the view-input-control interface, §2.3 layout) and a
  message-target vtable `0x00A90728` at `+0x28` whose slot 3 is `DoMessage` (`0x004C2F20`).
- Overrides: `QueryInterface` (`0x004BDF10`), `Init` (`0x004C13E0`), `Shutdown` (`0x004C0300`), `OnKeyDown`
  (`0x004C3060`), `OnMouseDownL` (`0x004C3120`), `OnMouseMove` (`0x004C3300`), `OnMouseExit` (`0x004C36C0`),
  destructor (`0x004C0070`). Every other slot is the base's (§2.3): right button, mouse up, wheel, `OnKeyUp`,
  `OnCharacter` return false; `ShouldStack` true.
- Constructor `0x004C0430`: `(buildingType, lotConfigId, buildingExemplar, monopolyFlag)` stored at `+0x2C`,
  `+0x30`, `+0x34` and the byte `+0x50` respectively *[inferred names, from their use in Init]*. Created by
  `cSC4View3DWin::InvokeCatalogItemCommand` (`0x007EA49B`), by two cheat codes in `cSC4View3DWin::ProcessCheatCode`
  (`0x007F35D9`, `0x007F363B`, all arguments 0) and by `cGZToolCommandSystemSupport::DoMessage` (`0x005C5441`).
  The catalog path installs it with the 3D view's `SetCurrentViewInputControl(tool, 2)` (slot 25).
- Constructor defaults worth matching: rotation `+0x38` = `+0x3C` = 0, last rotation shown `+0x40` = -1, last mouse
  `+0x44`/`+0x48` = 0, last placed rectangle `+0x78..+0x84` = (0, 0, -1, -1), cursor world position
  `+0x88`/`+0x8C`/`+0x90` = (0, -FLT_MAX, 0), placeable `+0x94` = false, feedback flag `+0x95` = false, toll-booth
  mode `+0x150` = false, cost-indicator values `+0x138` = -100000 (int64 low/high `+0x138`/`+0x13C` = -100000, -1),
  plops left `+0x154` = 1000.

### 3.2 Init (`0x004C13E0`) *[compat]*

Only when not initialised (the initialised byte `+0x04`). Steps:

1. Control ID `+0x0C` := `0x88F154FB`; cursor ID `+0x10` := `0x88E70B76`; then the base `Init` (§2.4).
2. Subscribe its message target (`+0x28`) on the message server (`0x00B43CCC`, slot 5 `+0x14`) to, in order:
   `0x277A9364`, `0x99EF1142` (occupant inserted), `0x99EF1143` (occupant removed), `0x49D5B83F`, `0xA6B79602`,
   `0xC9F775BB`.
3. Hold the lot-configuration manager (`0x00B43D18`) at `+0x4C` and the building-development service
   (`0x00B43D68`) at `+0x68` if not already held.
4. Choose what to plop:
   - **No building type** (`+0x2C == 0`): if no lot configuration was given, build a monopoly filter
     (`cSC4LotConfigurationFilterMonopoly(+0x50)`), let the lot-configuration manager fill the candidate set
     `+0x54` (manager slot 13 `+0x34`), take the first candidate, and let the user pick
     (`DoUserLotConfigurationTypePick`, `0x004BEEF0`). The lot configuration ID `0xEBF9A440` sets toll-booth mode
     `+0x150`.
   - **Building type given**: when no exemplar was given, start from the 3D view's last selected building
     (slot 51), fall back to the first entry of the building-type list (service slot 11 `+0x2C`), let the user
     pick (`DoUserBuildingTypePick`, `0x004BF2F0`) and store the choice back (3D view slot 53). A null choice ends
     here. Then fill the candidate set with `cSC4LotConfigurationFilterByBuildingType(exemplar)`; when the count at
     `+0x64` is zero (an empty set *[inferred]*) `Init` calls the tool's own `Shutdown` and returns false
     *[compat]*. Otherwise start from the 3D
     view's last selected lot (slot 52), let the user pick a lot configuration, store it back (slot 54).
5. Resolve the building exemplar (directly, or the first building of the chosen lot configuration) and read two
   dwords at record offsets `+0x30`/`+0x34` from the building-development service (slot 40 `+0xA0`) into
   `+0x128`/`+0x12C` (Q CT3).
6. Build the preview model (`GetBuildingPreviewModel`, `0x004C0680`), init the local grid helper `+0xAC`, create
   the ring effect `+0xD8` from the effects manager (`0x00B43D1C`) if absent, and register the coverage view
   `+0x140` with the view manager (slot 4 `+0x10`) when it is non-zero.
7. Return true.

### 3.3 Shutdown (`0x004C0300`) *[compat]*

Only when initialised: if the coverage overlay was raised (`+0x118`), clear it and recalculate the 3D view's
coverage helper (slot 49); unregister `+0x140` from the view manager (slot 5); clear the cursor text with key
`0xE9C877FA` (3D view slot 41); clear the cost indicator (slot 44, false); set the coverage helper's categories to
0 and recalculate; shut down the local grid helper; release the configuration effects (`0x004C0090`); release the
building-development service and the lot-configuration manager; unsubscribe the six messages of §3.2 step 2; then
the base `Shutdown`.

### 3.4 Messages (`DoMessage`, `0x004C2F20`) *[compat]*

Always returns false (the message is not consumed).

| Message | Action |
|---|---|
| `0x277A9364` | show or hide the local-grid effect: show when the city's (`0x00B43CF4`) slot 66 object reports slot 20 false |
| `0x49D5B83F` | `DoPlopRingEffectFinished` (`0x004BE0A0`) |
| `0x99EF1142`, `0x99EF1143` | unless `+0x118` is set: recalc the 3D view's coverage helper |
| `0xA6B79602`, `0xC9F775BB` | stop and release the two preview effects `+0xDC`, `+0xE0` (stop with slot 4(0) when slot 5 says running), then `SetVisualUserFeedback(+0x94, +0x95, +0x88)` to rebuild them |

### 3.5 OnKeyDown(vkey, modifiers) (`0x004C3060`) *[compat]*

Handled only while the tool is on top (§2.6) and the 3D view has a key accelerator (slot 31). The accelerator's
slot 10 (`+0x28`)(vkey, modifiers, 1, &command) translates the key; commands:

- `0x6A935CD5`: rotation := (rotation + 1) mod 4;
- `0x6A935CD1`: rotation := (rotation + 3) mod 4 (one step back).

The new rotation is written to both `+0x3C` and `+0x38`; then the tool re-runs its own `OnMouseMove` at the last
mouse position with modifiers `0x100` (§3.7, manual-rotation path) and calls `SetVisualUserFeedback`; returns
true. Any other key, or no translation: false.

### 3.6 OnMouseDownL(x, z, modifiers) (`0x004C3120`) *[compat]*

1. Pick the terrain under `(x, z)` (3D view slot 36, `PickTerrain(x, z, &pos, 0)`, `pos` preset to 0).
2. Only if a point was hit **and** the last move said placeable (`+0x94`): `PlaceLot(pos, rotation +0x3C, true,
   &rect)` (`0x004C1740`).
3. On success: read a float from the city (`0x00B43CF4`, slot 6 `+0x18`; the cell size *[inferred]*) and store the placed rectangle in world
   units: `+0x78` = `x0*size`, `+0x7C` = `z0*size`, `+0x80` = `(x1+1)*size`, `+0x84` = `(z1+1)*size`, each
   truncated to an unsigned 32-bit integer (§4 Q4); `SetVisualUserFeedback(true, false, pos)`; stop and release
   the effects `+0xDC` and `+0xE0`; clear the cost indicator (slot 44, true); placeable := false. Then
   plops-left `+0x154` decrements and when it drops below 1 the tool ends itself (`EndInput`, slot 27).
4. On failure (no hit, not placeable or `PlaceLot` refused): play the error sound through `cIGZSndSys` slot 11
   with `(0xEA451903, 0xAA62C9D5, 0, 0, 0)`.

Returns the `PlaceLot` result (false when it was not called).

### 3.7 OnMouseMove(x, z, modifiers) (`0x004C3300`) *[compat]*

1. Save the old feedback (placeable `+0x94`, flag `+0x95`, position `+0x88..+0x90`); clear the cursor-text buffer
   `+0x98`; remember `(x, z)` in `+0x44`/`+0x48`; flag `+0x95` := false.
2. Pick the terrain. **No hit:** load the string `(0x0A419226, 0xCA711914)` as cursor text, placeable := false,
   clear the cost indicator (false).
3. **Hit:** the candidate rotation is `+0x38`; a 4-int cell rectangle starts at (-1000, -1000, -1000, -1000).
   - modifiers bit `0x100` set: `GetLotCenterAndCellCoverage` (`0x004BE750`) with that rotation;
   - else toll-booth mode: `PickTollBoothLotAndOrientation` (`0x004BFB30`), switching the lot configuration
     (`ChangeLotConfiguration`, `0x004C1360`) when it picks another one, and taking its rotation into `+0x3C`;
   - else: `GetAutomaticFacingAndCellCoverage` (`0x004BEB70`) for the current lot configuration (manager slot 8);
     on success its rotation goes to `+0x3C`.
   If that step succeeded, `CanPlaceLot` (`0x004C1110`) decides placeable and writes the cost into `+0x138`
   (int64). Placeable: the stored rectangle `+0x78..+0x84` resets to (0, 0, -1, -1). Not placeable: rotation `+0x3C`
   := `+0x38`. Cost > 0: show it (3D view slot 43 `+0xAC`(low dword)); cost <= 0: clear the indicator.
   If the step failed, `CanPlaceLot` still runs (for its reason text) and the result is not placeable.
4. If the cursor text changed (`0x00454BB0` compare), show it with key `0xE9C877FA` (3D view slot 40 `+0xA0`).
5. When placeable, flag, any coordinate of the position (by more than `1e-4`, float `0x00A90688`) or the shown
   rotation (`+0x40` vs `+0x38`) changed: `SetVisualUserFeedback(placeable, flag, pos)` and store the new values.
6. Window manager slot 19 (`+0x4C`) (Q CT2); return true.

### 3.8 OnMouseExit (`0x004C36C0`) *[compat]*

Flag `+0x95` := false; `SetVisualUserFeedback(+0x94, false, +0x88)`; empty the cursor-text buffer; clear the cursor
text (`0xE9C877FA`) and the cost indicator (false); return true.

## 4. Quirks

1. `IsOnTop` and the place-lot handlers use the 3D view pointer without a null test (§2.6).
2. `EndInput` removes the stack top, whichever control it is (§2.7).
3. The place-lot tool ends itself after 1000 successful plops in one activation (§3.6) *[compat]*.
4. The world rectangle of §3.6 is computed in x87 precision as `x*size - 2^32` (min corners) and
   `(x+1)*size + 2^32` (max corners) and truncated to int64 (`_ftol2`, `0x009EEF04`); only the low dword is
   stored, so the 2^32 bias cancels and the stored values equal `trunc(x*size)` mod 2^32 *[compat]*.
5. On the automatic-facing path, when there is no lot configuration or automatic facing fails, the gate for the
   `CanPlaceLot` call is the **previous** placeable value (a stack slot reused from step 1) *[quirk, inferred]*.

## 5. Open questions

See docs/specs/questions/city-tools-input.md (CT1-CT5).

## 6. Conformance tests

1. Base: `Init` twice holds the 3D view once; `Shutdown` releases window manager, 3D view, cursor in that order and
   keeps the cursor ID; `SetCursor(unknownId)` changes nothing.
2. `QueryInterface(0xC6BB8345)` and `(1)` succeed with AddRef; any other IID fails.
3. Place-lot: `Init` sets ID `0x88F154FB` and cursor `0x88E70B76`, and subscribes the six messages in the §3.2
   order; `Shutdown` unsubscribes the same six.
4. Rotation keys: four presses of `0x6A935CD5` return the rotation to its start; `0x6A935CD1` then `0x6A935CD5`
   is the identity.
5. A failed left click plays `(0xEA451903, 0xAA62C9D5, 0, 0, 0)`; a successful one does not.

## 7. Coverage

`cSC4ViewInputControl` (all 17 functions) is specified by §2. `cSC4ViewInputControlPlaceLot` is specified for its
lifecycle and input dispatch (§3); the bodies of `PlaceLot`, `CanPlaceLot`, the facing/coverage helpers, the
preview model and the visual feedback are not, so it is **not** claimed as specified.
