# The city-screen window `cSC4View3DWin`: lifecycle, interfaces and event dispatch (behaviour spec)

Status: M6 behaviour spec, written 2026-10-21 (re-view3dwin4 lane). Every fact was read on the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`) through `tools/re_query.py` (export) and the class's
`decomp/` file; numeric facts were checked on the instructions and carry addresses. REA was unavailable to this lane
("No app is open"), so no fresh pseudocode was taken. Implementers work from this spec, the LGPL gzcom-dll SDK
headers and general knowledge only, never from decompiled code.

Tags: *[compat]* observable, must match; *[inferred]* consistent with the evidence, not proven; *[quirk]* original
behaviour reproduced as is.

Scope. This spec owns the **window object's lifecycle and event routing**: its four interface sub-objects and their
tables, construction, `Init`/`Shutdown`, the cheat and command-name registrations, the MessageServer2 subscription
set and how each notification is routed, and the `cIGZWin::DoMessage` (window message) routing. It does **not**
restate:

- the object layout around the catalog menus, the save record or the 61-slot `cISC4View3DWin` table: see
  [ui-windows.md](ui-windows.md) §3–§4;
- the paint path (`GZPaint`, slot 88 of the window table) and the draw-relevant fields: see [view3d.md](view3d.md) §5;
- the terrain's 3D view, the DX7 driver and the particle/effects layer (terrain.md §15.5, renderer-driver.md,
  view3d.md §8).

*Extended 2026-10-21 (re-view3dwin), from the instruction bytes:* the fields (§6d), the view input control stack
(§6e), scrolling, zoom and views (§6f), every command of the command table (§6g), the panels (§6h), the buttons,
menus and catalog popups (§6i), the status displays (§6j), the save record's body and the global effect triggers
(§6k), helpers (§6l) and quirks (§6m).

## 1. Identity and interfaces *[compat]*

One `cSC4View3DWin` object carries four vtable pointers. Construction (`0x007F0A70`) writes them at
`0x007F0A9D`–`0x007F0AB1`:

| Offset | Interface | Vtable | Slots | How it is reached | Evidence |
|---|---|---|---|---|---|
| `+0x00` | `cISC4View3DWin` (IID `0xFA47B3F9`), also answers IID `1` (`cIGZUnknown`) | `0x00ABCE40` | 61 | `QueryInterface` returns `this` | QI `0x007E5630`; ui-windows.md §3.3 |
| `+0x04` | `cIGZMessageTarget2` (IID `0x452294AA`) | `0x00ABCE2C` | 4 | QI returns `this+4` | slots 0–2 are thunks (`this-4`) to QI/AddRef/Release; slot 3 is the notification handler `0x007F53C0` (§5) |
| `+0x08` | window message filter (IID `0xC6AE7085`) *[inferred: `cIGZWinMessageFilter`]* | `0x00ABCE1C` | 4 | QI returns `this+8` | slots 0–2 thunks (`this-8`, e.g. `0x007F10D0`); slot 3 `0x007F58E0` (§6.3) |
| `+0x0C` | `cIGZWin` (IID `0x22BA0121`), a `cGZWin` base | `0x00ABCBC0` | 151 | QI returns `this+0xC`; also `AsIGZWin` (slot 3 of `0x00ABCE40`) | ctor `0x007F0AB1`; ui-windows.md §3.2 |

`QueryInterface` (`0x007E5630`) answers exactly these five IIDs (`1`, `0xFA47B3F9`, `0x452294AA`, `0xC6AE7085`,
`0x22BA0121`) and returns false for anything else; it does **not** fall back to `cGZWin`'s own QI, so asking a
city-screen window for any other window-family IID (e.g. a `cIGZWinGen` IID) fails. Every success AddRefs through
`+0`. `AddRef`/`Release` (`0x007E56D0`/`0x007E56E0`) forward to the `cGZWin` sub-object's count: the object has one
reference count, the window's.

The three small tables are not in `docs/abi/vtables.tsv`; this lane read them as raw data (`0x00ABCE1C` =
`007F10D0 007F10E0 007F10F0 007F58E0`; `0x00ABCE2C` = `007F10A0 007F10B0 007F10C0 007F53C0`, followed by a zero
dword). Proposed rows: `build-runs/abi-rows-re-view3dwin4.tsv`.

### 1.1 The window table `0x00ABCBC0`: slots the class overrides *[compat]*

All other slots of the 151 are inherited `cGZWin` code.

| Slot | Offset | Address | Role |
|---|---|---|---|
| 0–2 | `+0x0`–`+0x8` | `0x007F1100`/`0x007F1110`/`0x007F1120` | thunks (`this-0xC`) to QI / AddRef / Release |
| 3 | `+0xC` | `0x007F3E50` | window `DoMessage` (§6) |
| 4 | `+0x10` | `0x007E9900` | `Init` (§3) |
| 5 | `+0x14` | `0x007F1320` | `Shutdown` (§4) |
| 88 | `+0x160` | `0x007F6200` | `GZPaint` (view3d.md §5.1) |
| 129 | `+0x204` | `0x007E6600` | `GZOnCharacter` |
| 134–139 | `+0x218`–`+0x22C` | `0x007E68D0` `0x007E6A00` `0x007E8170` `0x007E82E0` `0x007E9460` `0x007E6B50` | mouse down L/R, up L/R, move, wheel (city-tools-input.md) |
| 142 | `+0x238` | `0x007E96E0` | `GZOnMouseExit` |
| 143 | `+0x23C` | `0x007E5BA0` | `GZOnCommand` |
| 148 | `+0x250` | `0x007F1130` | deleting destructor thunk (`this-0xC` → `0x007F3E30`) |

`GZOnCommand` (VW16) accepts two 32-bit arguments (command and caller data), ignores both and
returns **false** with no effects. Its shared body is `0x0093877E` (`0x007E5BA0` is a jump to it;
the return discards eight argument bytes). It neither invokes `ProcessCommand` nor forwards a message.
The command notification and window-message routes in §§5–6 remain separate routes.

## 2. Construction (`0x007F0A70`) *[compat]*

Order of effects:

1. The four vptrs are first set to pure-call stubs (`0x00ABC9C0`, `0x00A81174`, `0x00A86864`), the `cGZWin` base is
   constructed at `+0x0C` (`0x0099D938`), then the four final tables of §1 are stored.
2. All owned pointers are nulled (renderer, camera control, model maker, view manager slots, the catalog-menu fields
   `+0x1F8`…`+0x208`, the panel window procs, the occupant filter at `+0x320`, the progress screen).
3. Non-zero defaults (absolute offsets, read on the instructions of the ctor; meanings *[inferred]* where marked):

| Offset | Default | Meaning |
|---|---|---|
| `+0x1F4` | `0xFFFFFFFF` | the current panel: none (§6h) |
| `+0x210`, `+0x218`, `+0x220` | `1`, `4`, `7` | the zone catalogs' remembered items (the second dwords of the states at `+0x20C`, `+0x214`, `+0x21C`): low residential, low commercial, agriculture (§6i) |
| `+0x278` … `+0x2C8` | `0x8C329937`, `0x31`, `0x29`, `0x299237BF`, `0x6A33E1BA`, `0x2A33E1A3`, `0xEA353064`, `0x2A312F97`, `0x0A5242DF` (each followed by a zero dword) | the remembered items of the transport and landscape catalogs (§6i) |
| `+0x324` | `1` | the view level (§6f); written twice: 0 then 1 at `0x007F0FB4` |
| `+0x334` | `1.0f` | the scroll stop step (§6f) |
| `+0x340`, `+0x344` | `0.2f`, `0.2f` | the edge and keyboard scroll speeds (§6f) |
| `+0x368`, `+0x370` | `5`, `0x15200003` | the zoom maximum; the current mayor sub-panel (§6f, §6h) |
| `+0x380` | a `cSC4RenderProperties` sub-object, constructed in place | render properties (owner class cSC4RenderProperties) |
| `+0x400` | `-1` | the contour layer id: none (§6f) |
| `+0x408` | `1` | "not loaded": cleared by `DoLoad` (§6k) |

4. Two empty intrusive lists are allocated (12-byte and 60-byte heads, each pointing at itself) and a 100-bucket
   `hash_set<uint32>` is constructed.
5. The `cRZAutoMsg2Notification` member is told its message set: **33 message types** from the table at
   `0x00ABC070` (the call `AddMessageTypes(0x00ABC070, 0x84)` at `0x007F0FBE`; `0x84` bytes = 33 ids *[inferred]*
   that the argument is a byte length). The set is in §5.
6. `SL::OnView3DCreate(this)` (`0x00602120`) is called last: the scripting layer learns the window exists before
   `Init`.

## 3. `Init` (window slot 4, `0x007E9900`) *[compat]*

`Init` runs once: it returns at once if the `cGZWin` init flag is already set. Otherwise, in order:

1. **Notification subscription**: the auto-notification member registers the object (as `cIGZMessageTarget2`).
2. **Command names**: the framework is asked (`0x007E9959`, slot 5 with class `0xCB90253A`, IID `0xEB903A32`) for a
   command-name registry and given the table at `0x00B0DD98` with the argument `0x68` (slot 7). The table holds
   records `{command id, name, 0}`; the first nine are `0x6A935C34 ToggleGodPanel`, `0x6A935CA4 ToggleMayorPanel`,
   `0x6A935CA8 ToggleMySimPanel`, `0x6A935CAB ToggleOptionsPanel`, `0x6A935CAD SimSpeedPause`, `0x6A935CB0
   SimSpeedSlow`, `0x6A935CB2 SimSpeedMedium`, `0x6A935CB5 SimSpeedFast`, `0x6A935CB7 ZoomIn`. These are the names a
   key-configuration file binds keys to. `0x68` is the record count: 104 records, all
   listed with what each does in §6g (answers VW3); the registry is the command server (command-system.md §5).
3. **Cheat registration** (only when the cheat manager global `0x00B43CAC` exists): the window registers itself as a
   cheat listener (slot 5), then registers these cheat strings with their IDs (slot 3, `(id, string)`), in this
   order:

| ID | Cheat text | Registered |
|---|---|---|
| `0x8973A69C` | `GOL` | always |
| `0xCA7BC5B5` | `SizeOf` | always |
| `0xAA7A2746` | `TastyZots` | always |
| `0x6A7A276E` | `Zoneria` | always |
| `0x8AB730AE` | `TerrainQuery` | always |
| `0x88F3D4BB` | `RenderProp` | always |
| `0x88F3D4BC` | `RP` | always |
| `0x6976AFA0` | `Recorder` | always |
| `0x4A972F37` | `WatchMeMove` | always |
| `0x4A82253E` | `You don't deserve it` | always |
| `0x4AC096C6` | `LotPlop` | only if the app object `0x00B43C94` slot 21 (`+0x54`) returns true (`0x007E9B85`) |
| `0x4AC096CA` | `BuildingPlop` | same condition |
| `0x0BFD8064` | `toll` | same condition |

   The pairing is read from the instruction order (`0x007E998C`–`0x007E9C06`: each string load is followed by its
   ID push). Cheat text is matched by the cheat manager, not here; what each cheat does is `ProcessCheatCode`
   (`0x007F3510`), reached through the window message `0x230E27AC` (§6); per-cheat effects are §3.1.
4. `cGZWin::Init` runs (`0x007E9C27`); its bool result is stored at `+0xE4` and is `Init`'s return value.
5. The window names itself `"cSC4View3DWin"` (`0x007E9C2E`), sets its window ID class `0x9A47B417` (`0x007E9C62`,
   the class ID by which `GetChildAs(0x9A47B417, 0xFA47B3F9, …)` finds it, landfill.md §3.2), sizes itself to its
   parent's client area, and becomes visible/enabled through `cGZWin` slots.
6. Cursor: through the cursor manager, cursor `0xC16C4E1B` is set as the window's cursor (`0x007E9CB2`).
7. The coverage-radius helper at `+0x3D8` is initialised, a byte at `+0x150` cleared, and a `cRZTimer` (the scroll /
   frame timer *[inferred]*) is started (`0x007E9CDD`).
8. **The key accelerator** (answers VW2): the key-configuration resource (type `0xA2E3D533`, group `0x6A231EAA`,
   instance `0x6A9362EF`, interface `0x42E3EA4B`) is loaded; when it is, GZCOM class `0x42E967BE` (interface
   `0xE2C1B3C4`) is made into object `+0x120` (the handler's `+0x114`) and the resource's slot 3 is called with it
   *[inferred: the key map fills the accelerator]*. Window `DoMessage` gives it the keys no control took (§6).
9. `+0x119` := 1.
10. If the app's current city record (app slot 39 `+0x9C`, field `+0x204`) is non-zero, the progress screen is
   created (`CreateProgressScreen`, `0x007E5BB0`) — a city is being loaded.
11. `0x00562DD0` is called last.

### 3.1 What each cheat does (`ProcessCheatCode`, `0x007F3510`) *[compat]*

**How it is called.** The window `DoMessage` (§6) handles message `0x230E27AC`: it takes the message's string
(`data2`, string slot 5), splits it into a command line (`cRZCmdLine`, `0x007F40D4`) and calls
`ProcessCheatCode(cheat ID = data1, command line)` on the object's `+0` sub-object (`0x007F40E2` passes `this − 0xC`).
The `bool` result is the window message's result (`0x007F40EE`), so `false` means "not consumed". In the command line,
argument 0 is the cheat word itself; `argc` (command-line slot 3, `+0xC`) counts it, and argument *i* is command-line
slot 16 (`+0x40`). The dispatch is a compare tree on the unsigned cheat ID (`JA`/`JZ`/`JC`, `0x007F3517`–`0x007F3888`);
an ID not in the table returns `false` (`0x007F39C0`).

**Installing an input control** (four cheats) means: allocate the control, hold one reference, call the window's own
`SetCurrentViewInputControl(control, 2)` (slot 25, `+0x64`; the same call the catalog makes, city-tools-input.md §3.1),
release the reference, return `true`. If the allocation fails the call is still made with a null control.

| Cheat | ID | Effect | Returns |
|---|---|---|---|
| `GOL` | `0x8973A69C` | installs a new `cSC4ViewInputControlGameOfLife` (0x88 bytes, ctor `0x004BC310`) | true |
| `SizeOf` | `0xCA7BC5B5` | needs `argc ≥ 2`. `v := atof(argument 1)` (`RZAtofA`, stored as a float). If `v < 0.9f` (`0x00A88218`) then `v := 0.9f`, else if `v > 64.0f` (`0x00A9552C`) then `v := 64.0f` (equal to a bound is kept). Calls the camera control's (`+0xF0`) `0x007CD6E0` with `v` (below) | true; false when `argc < 2` |
| `TastyZots` | `0xAA7A2746` | new occupant filter (below), flags: `+0xD` := **not** old `+0xD`, `+0xE` := old `+0xE`, `+0xC` := old `+0xC`; `SetCurrentViewFilterInfo` | **false** *[quirk]* |
| `Zoneria` | `0x6A7A276E` | new occupant filter, flags: `+0xC` := **not** old `+0xC`, `+0xD` := old `+0xD`, `+0xE` := 0 (not copied) *[quirk]*; `SetCurrentViewFilterInfo` | **false** *[quirk]* |
| `TerrainQuery` | `0x8AB730AE` | byte `+0x126` := 1. If `argc ≥ 2` and argument 1, lower-cased (`MakeLower`), equals `"off"` (`0x00A82B94`), `+0x126` := 0. `+0x126` is what `GetTerrainQueryEnabled` (slot 39, `0x007E5900`) returns | true |
| `RenderProp`, `RP` | `0x88F3D4BB`, `0x88F3D4BC` | `argc ≥ 3`: `SetRenderProperties(argument 1, argument 2)` (`0x007ECDA0`). `argc = 2`: `SetRenderProperties(argument 1, "")`. `argc < 2`: shows the text `"Cheat code usage: renderprop <property name> [0|1] [true|false]"` (`0x00ABCF70`) through the window's slot 45 (`+0xB4`; an empty stub in this build, `0x00648F10`, so nothing is shown) | true; false when `argc < 2` |
| `Recorder` | `0x6976AFA0` | `AddWinAnimationRecorder(true)` (`0x007F0840`, below) | true |
| `WatchMeMove` | `0x4A972F37` | byte `+0x124` := 1. Nothing clears it here (it allows the `PauseAnimation` and `StepAnimation` commands, §6g) | true |
| `You don't deserve it` | `0x4A82253E` | byte `+0x446` := 1. Nothing clears it here (every reward building is then available in the building menus, §6i) | true |
| `LotPlop` | `0x4AC096C6` | installs a new `cSC4ViewInputControlPlaceLot` (0x158 bytes, ctor `0x004C0430`) built with arguments `(0, 0, 0, 0)` | true |
| `BuildingPlop` | `0x4AC096CA` | the same control built with `(1, 0, 0, 0)`: the first argument is the only difference | true |
| `toll` | `0x0BFD8064` | installs a new `cSC4ViewInputControlTollBooth` (0x38 bytes, ctor `0x004D9E80`) | true |

No cheat reads its arguments beyond those listed: extra words are ignored.

`SetRenderProperties(name, value)` (`0x007ECDA0`): the name is looked up as a bool, an int and a float property
(render-properties slots 12, 13, 14); a bool is parsed, or **toggled** when the value is empty; an int or float is
parsed; on success `0xC9F775BB` is posted (render-properties.md).

**The two filter cheats** (`TastyZots` `0x007F391F`, `Zoneria` `0x007F365D`). Both build a fresh 16-byte
`cSC4RealisticOccupantFilter` (vtables `0x00A88084`/`0x00A8806C`, flag bytes `+0xC`/`+0xD`/`+0xE` all 0; `TastyZots`
through `0x004666C0`, `Zoneria` inline with the same stores) and take the current filter from `+0x320` (the field
`GetCurrentViewFilterInfo`, slot 18, returns). The old filter's flags are copied (as in the table) only when it answers
`QueryInterface(0xE4FDA3D4)` and that interface's slot 5 returns `0x8A7A1C2C` (the realistic filter's type,
occupant-manager.md filter table); otherwise the new filter keeps all three flags at 0. The new filter is handed to
`SetCurrentViewFilterInfo` (slot 19, `+0x4C`) and the cheat's references are released. Each use therefore toggles one
flag: `TastyZots` toggles `+0xD`, `Zoneria` toggles `+0xC` and clears `+0xE`. What the flags keep, when set, is
occupant-manager.md §13.2.3: `+0xC` the base textures of empty lots, `+0xD` the occupants of group `0x5001`, `+0xE`
the signposts and text decals. Both branches end on the shared `return false` (`0x007F39A8` →
`0x007F39C0`), so the cheat message counts as unconsumed although the filter changed *[quirk]*. If the 16-byte
allocation fails and the old filter is realistic, the original writes through a null pointer (crash); a replacement
may skip the cheat instead.

**`SizeOf` and the camera control (`0x007CD6E0`).** The camera control stores the new value in its float `+0xF0` only
when the value differs from the current one **and the current one** lies strictly between 0.0 (`0x00A81054`) and
1000.0 (`0x00A8FAF0`): the bounds test the old value, not the new one. When it stores and its `+0xC` is non-null, it
recomputes its zoom/rotation and position parameters, updates the view utilities (global `0x00B43DD8`), refreshes the
scroll bounds when `+0x14C == +0x150`, and posts message `0xA6B79621` (PostViewRefresh) carrying its `+0x100` and
`+0x110` vectors. With the cheat's clamp the stored value is always in [0.9, 64].

Example: `SizeOf 100` stores 64.0; `SizeOf 0.5` stores 0.9; `SizeOf abc` stores 0.9 (`atof` gives 0); `SizeOf`
alone does nothing and returns false.

**`Recorder` (`AddWinAnimationRecorder(bool add)`, `0x007F0840`).** Looks for a child window with ID `0xA802B4EB`
(`cIGZWin` slot 34). With `add` true and no such child: creates a `cSC4WinAnimationRecorder` (0x218 bytes, ctor
`0x00796380`), sets its output directory to the user-data directory (`SL::UserDataDirectory`) + `"RecordedAnimations"`
+ `"\"` (`SetAnimationOutputDirectory(dir, 1)`, `0x00795100`), gives it window ID `0xA802B4EB` and adds it as a child
(`cIGZWin` slot 14). With `add` true and the child present: nothing. With `add` false: removes the child if present
(slot 15). Always returns true. The cheat only ever passes true, so a second `Recorder` is a no-op.

## 4. `Shutdown` (window slot 5, `0x007F1320`) *[compat]*

1. Unregisters the cheat listener and **all thirteen** cheat IDs of §3, including `LotPlop`, `BuildingPlop` and
   `toll` even when they were never registered *[quirk]* (`0x007F1383`–`0x007F141A`). A replacement's cheat manager
   must tolerate unregistering an unknown ID.
2. Calls its own `RemoveAllViewInputControls` (slot 27, `+0x6C`) and `RemoveAllPersistentViewInputControls` (slot 30,
   `+0x78`): no input control survives the window.
3. Releases the helper objects, then `RemoveGUI` (`0x007EF200`), then `cSC4ProgressScreen::Shutdown` if a progress
   screen exists, then unsubscribes from the command-name registry (`0xCB90253A`).

## 5. Notifications (`cIGZMessageTarget2::DoMessage`, `0x007F53C0`) *[compat]*

The handler always returns true. First rule: if the message answers `QueryInterface(0x4B99446A)` (a command
message), the type and its two data values are passed to the window's own `ProcessCommand` (slot 7, `+0x1C`) and
nothing else happens. This is how the nine-plus command names of §3 step 2 reach the window. Otherwise by type:

| Type | Name (from other specs) | Effect |
|---|---|---|
| `0x26D31EC0` | PreCityInit | `DoPreCityInit` (`0x007F41B0`) |
| `0x26D31EC1` | PostCityInit | `DoPostCityInit` (`0x007ED0F0`) |
| `0x26D31EC2` | PreCityShutdown | `0x007E9780` |
| `0x26D31EC4` | CityEstablished | `DoCityEstablished` |
| `0x26D31EC5` | CityObliterated | `DoCityObliterated` |
| `0x26C63341` | load | `DoLoad(data)` (ui-windows.md §3.3a) |
| `0x26C63343` | (pre-load/clear) | `RemoveAllViewInputControls(0)` (slot 27) |
| `0x26C63344` | save | `DoSave(data)` |
| `0x0AB99380` | city name changed *[inferred]* | `UpdateCityName` |
| `0x0772FAD4`, `0x08DAC3E7`, `0x08DAC3E8` | FundsChanged and the two budget dirty messages | `UpdateBudget` |
| `0x06CDB65B` | view level changed | if the renderer (`+0xF4`, `GetRenderer`) exists, its `cISC43DRender` slot 20 (`+0x50`) (`0x007F5455`) |
| `0x44778231` | 24-hour clock tick | `Update24HourClock(data, data)` |
| `0x44778233` | VisualTimeTick | `UpdateAmbientLightLevel` |
| `0x66956814` | simulation date tick *[inferred]* | `UpdateSimulationClock`, then `UpdatePopulation` |
| `0x66956816` | mayor rating changed *[inferred]* | `UpdateMayorRatingBar` |
| `0x4A7FB7E9` | speed changed | `UpdateSpeedButtons` |
| `0x4A7FB7E2` (HiddenPause change), `0xAA7FB7E0` (SimPauseChange), `0xABBBCA64` | pause / mission state | §5.1 |
| `0xA6B79602`, `0xA6B79612` | view update / ViewTChange3 | §5.2 |
| `0xA6B79621` | PostViewRefresh | `DoTemporarySimHiddenPause` |
| `0xCA1B1E64` | DisasterPeriodStarted | if the secondary menu kind (`+0x200`, ui-windows.md §3.2) is `0x49923239`, close the secondary menu (slot 59) (`0x007F5861`); then as `0xAA1B2002` |
| `0xAA1B2002` | DisasterPeriodEnded | child window `0x8991EE08`, its slot 68 (`+0x110`) with `(2, type == 0xAA1B2002)`; then `UpdateAlertBorder` |
| `0xAA5CF1E3` | EndGameForBudgetFailure | if modal popups are safe now (`SC4GetPopupModalDialogsSafe`): `DoBudgetFailure`; otherwise the message is queued once (a list of deferred messages; a second copy of the same type is not added) |
| `0xAC083D7D` | mission target reached | `UpdateMissionProgress(message.IGZUnknown, 1)` |
| `0x4A44667F` | progress report *[inferred]* | if the message carries interface `0x089B7DC8` and a progress screen exists, forward it to the progress screen (`0x00777640`) |

The four types `0x6A935C34`, `0x6A935CA4`, `0x6A935CA8`, `0x6A935CAB` close the 33-entry set; they are command
messages and take the first rule.

### 5.1 Pause and mission notifications

For `0x4A7FB7E2`, `0xAA7FB7E0` and `0xABBBCA64`: if a "paused" overlay object is present it is updated from the
simulator's pause state (overlay object `0xC998BB81`, shown when paused at speed 1000 *[inferred]*); then
`UpdateSpeedButtons` and `UpdateAlertBorder`. For `0xABBBCA64` only,
`UpdateMissionProgress(message.IGZUnknown, message.data1)` follows: the attachment is the advice object,
and data1 is the activation flag from city-situation.md. See §6l (VW18).

### 5.2 View-update notifications

`0xA6B79602` and `0xA6B79612` act only while the app's "city view hidden" byte (`+0x39C` of the object at
`0x00B43CC4`+`0xC`) is zero. `0xA6B79602` first runs `UpdateZoomButtons` and `AddBackgroundImageToRenderer`; both
types then run `DoTemporarySimHiddenPause`, and if the cursor-text flag is set the cursor text is re-placed at the
current mouse position.

## 6. Window messages (`cIGZWin::DoMessage`, `0x007F3E50`) *[compat]*

Every message is first offered to `nsSC4TutorialSystem::FilterUICommandMessage` (tutorial.md). Then:

| Window message | Effect | Result |
|---|---|---|
| `0x230E27AC` (cheat) | `ProcessCheatCode(data1 = cheat ID, command line split from data2)` (§3.1) | its result |
| 3 (command), data1 `0x49D993D4` | the window takes keyboard focus, then `InvokeCatalogItemCommand(data2, data3)` | true |
| 3, data1 `0x287259F6` (button) | not handled here (the filter, §6.3, owns buttons) | base `cGZWin` |
| `0x13` (mouse enter) | hover byte `+0x118` := 1 (`0x007F402F`), `Update24HourClockHover`, base | base |
| `0x14` (mouse leave) | hover byte `+0x118` := 0, `Update24HourClockHover`, base | base |
| `0x0D` | `Update24HourClockHover`, base | base |
| `0x10` (focus lost) | only when global `0x00B43CEC` is non-zero: the four scroll-velocity dwords `+0x348`…`+0x354` are zeroed (`0x007F4087`–`0x007F4099`; the handler's `this` is `+0x0C`), `RecalcScrollVelocity`, `ScrollStop` (slot 34), and a `cGZWin` slot 142 call (`GZOnMouseExit`) | true |
| `0x1B` with sub-type 5 / 6 | key down / up: `ProcessKeyDown` / `ProcessKeyUp`; if neither consumes it, the key goes to the key-binding helper (`+0x120`, slot 9; `0x007F3F39` reads it as `+0x114` from the handler's `this` = `+0x0C`; the key record supplies key code `+4` and modifiers `+8`, §6c) and, if bound, a command is posted through the command dispatcher `0x00B43CB0` | false only when unbound |
| `0xE4A85D91` (link) | `cSC4HTMLLinkHandler::DoURL(data2)` | true when DoURL **failed** *[quirk]* |
| `0x6A5BDFCD`, `0x6A720993`, `0xEA5BDFBC` | `HandleAdviceLinkMessage`: open the `+0x300` sub-panel and pass the link, open the graphs at the link's graph, or open the budget for links `0x4A5BDFB8`/`0xCA5BDFB0`, else pass it to the budget window; then base | base |
| anything else | base `cGZWin::DoMessage` | base |

### 6.1 Focus loss stops scrolling

A replacement must stop any edge or key scroll when the city window loses focus, but only under the global flag
`0x00B43CEC` (*[inferred]* pause-on-focus-loss / windowed mode; application.md `GP` cheat).

### 6.3 The message filter (`0x007F58E0`)

The filter sub-object (`+0x08`) is installed on child windows (InvokeSecondaryMenu, ui-windows.md §4.3). It first
runs the tutorial filter, then handles window message `3` (and `0x12`): a `0x287259F6` button press goes to
`HandleButtonActivated` (`0x007F4690`); `0x287259F7` toggles a panel tab named by its ID (`0x8988BC94`,
`0x2988BC85`, `0x4988BC6A`, `0xC988BC79`): §6h (answers VW4).

## 6a. City init: what the window builds per city *[compat]*

The window exists before any city; the city-scoped objects are built by the PreCityInit notification and torn down
before the next city.

**PreCityShutdown is narrower than city teardown (VW15).** `0x007E9780` first clears the deferred-message
list at `+0x14C`, releasing its message references, then calls view slot 27 with `true`
(`RemoveAllViewInputControls`, including the current control), then slot 30
(`RemoveAllPersistentViewInputControls`). It does not call `RemoveGUI`, release the camera/renderer/model maker,
remove notifications, clear paint-enabled `+0x15C`, or reset the pause timer. Those effects must not be added to
this notification; window `Shutdown` supplies its separate teardown. The final operation is a tail call to slot 30.

**`DoPreCityInit` (`0x007F41B0`)**, in order:

1. Reads the window's client width and height (`cGZWin` slots 41/42, `+0xA4`/`+0xA8`).
2. Creates the **S3D camera** (GZCOM class `0xE9C6262A`, `cS3DCamera`, constructor `0x007FF930`; camera.md) and
   stores it at `+0xEC` (`0x007F4224`) (answers VW5).
3. Creates the **camera control** (GZCOM class `0xC9C628EC`, `cSC4CameraControl`, `0x007CC990`), stores it at
   `+0xF0` (`0x007F427C`), sets its viewport to the client size (`SetViewportSize`) and calls
   `cSC4CameraControl::Init` with the start focus **(512.0, 0, 512.0)** (`0x007F42A0`–`0x007F42B0`, two `0x44000000`
   floats). The real city size is applied later (PostCityInit step 2).
4. If no occupant filter exists at `+0x320`, a 16-byte realistic filter with all three flags set
   (occupant-manager.md §13.2.3) is created and stored there (`0x007F4307`).
5. Creates the **renderer** (GZCOM class `0xE9C622D8`, IID `0x681BD529`, `cSC43DRender`, `0x007C9B10`) and stores
   it at `+0xF4`, the field `GetRenderer` (slot 4, `0x0043FD00`) returns. It is wired to the camera control and the
   `+0xEC` helper through its slots 6, 7, 12 (view3d.md §3.2).
6. Looks up GZCOM `0xA2FFB5D3`/`0x02FFBB60`, then `AddBackgroundImageToRenderer` (`0x007F1640`).
7. Registers script hooks (`cSLString` with `0x6A231EAA`/`0xAA765647`, `0xAA76563A`) and reports progress to the
   progress screen (`0x00777640`).
8. The animation tick manager (GZCOM `0xA9C73857`, interface `0x86AD10ED`, `+0xE8`) and the model maker
   (`0xA9C741B3`, `+0xF8`); `SL::OnView3DInit`; `ReadTunableValues` (§6f); the second progress message; `AddGUI`
   (§6b). `+0xE5` is 1 from the start to the end of `DoPostCityInit` (§6d).

The model maker is held at `+0xF8` (`GetModelMaker`, slot 5, `0x007F0FE0`).

**`DoPostCityInit` (`0x007ED0F0`)**, in order: updates the progress message; sets the camera control's city size
(`cSC4CameraControl::SetCitySize`, `0x007ED17F`) from the city; refreshes `Update24HourClock` and
`UpdateAmbientLightLevel`; enables the two child windows `0x4992046A` and `0xE9920494`
(`nsGZWinUtils::SetWindowEnabled`); refreshes the simulation clock, population and day/night buttons; posts the bond
message (`cSC4BudgetSimulator::PostBondMessage`); refreshes the mayor rating bar; then runs either
`DoCityEstablished` or `DoCityObliterated` (`0x007ED333`/`0x007ED33A`) according to the city's established state
(city slot 22; answers VW5); finally it builds a `cSC4ResponseCurve` from properties
`0x6A6564F4`/`0x6A6564F6` (`0x007ED5B5`–`0x007ED5DB`). Between them (re-view3dwin): the three **RCI demand graphs** (window interface
`0x89E1574C`): `0x09D27EB0` R$, R$$, R$$$ (`0x1010`, `0x1020`, `0x1030`) in (0, 200, 0); `0x29D27EC0` CS$, CS$$, CO$$,
CS$$$, CO$$$ (`0x3110`, `0x3120`, `0x3320`, `0x3130`, `0x3330`) in (0, 0, 200); `0x49D27ED0` I-D, I-M, I-HT
(`0x4200`, `0x4300`, `0x4400`) in (200, 200, 0) (demand.md §2; agriculture `0x4100` has no bar); six gauges
`0x6A5E6EDC`–`0x6A5E6EE1` of the `+0x30C` panel with two pictures (`0x14015580`, `0x14015584`) and that curve
*[summary]*. Last: `+0xE5` := 0, a `cSC4ViewInputControlCitySituations` added as a persistent control, focus to the
view, the tip of the day when not shown yet and present, and the progress screen shut down and released.

## 6b. The city-screen GUI: `AddGUI` (`0x007EDEB0`) and `RemoveGUI` (`0x007EF200`) *[compat]*

**Pairing.** `AddGUI` has one caller, the end of `DoPreCityInit` (`0x007F4665`, after the progress message and before
`UpdateProgressMessage`); it has no guard against a second run. `RemoveGUI` has one caller, `Shutdown` (`0x007F14B5`,
§4). *[inferred]* the window object lives for one city, so the two run once each per city.

In this section "script window *I*" means `SC4CreateWindowFromScript` with key `{type 0, group 0x96A006B0, instance
I}` and the city window (`+0x0C`) as parent. The two variants are `(key, parent, window ID)` → window (`0x005F9390`)
and `(key, parent, window ID, IID, out)` (`0x005F9480`), the second querying the new window for the IID into *out*.

**`AddGUI`, in order** (fields are absolute object offsets; `this` = object `+0`):

1. **Main panel**: script window `0xC973B411`, ID `0x0987B48F`, stored at `+0x18C` (`0x007EDEF2`); queried for
   `cIGZWinGen` (`0x5386D516`) into `+0x190` (`0x007EDF00`); the window's message filter (`+0x08`) is added to it (`cIGZWin` slot 116,
   `+0x1D0`); it is placed at the bottom of the parent (y = parent height − its height + 11, `0x007EDF87`–`0x007EDFAC`; x = 5,
   plus a third of the width when the width is above three times the height);
   flag `0x800` is set (slot 68); `ClearAcceptFocusFlagRecursive` is applied to it.
   **Then a hidden button at `+0x194`** (`0x007EDFE3`–`0x007EE193`): GZCOM class `0x22ECFC47`, IID `0x22BA0121`,
   into `+0x194`; when that succeeds and the object answers IID `0x8810` (a button interface, held only for this
   step): image `{0x856DDBAC, 0x46A006B0, 0x53244588}` with colour key (255, 0, 255) (city window `MakeFillColor`,
   slot 108), set with mode 2 (button slot 12); button slot 5 `(0x20, 1)`; flag 4 on; tooltip string
   `{0x6A231EAA, 0x8A51545D}` (button slot 35); notification target = the main panel (`SetNotificationTarget`, slot
   86); ID `0x43`; moved to (12, city-window height − 28) (`GZWinMoveTo`, slot 56); added to the city window, then
   hidden (slot 70); `ClearAcceptFocusFlagRecursive`.
2. **Five tab panels**, a loop `i = 0..4` (jump table `0x007EF1D8`):

| i | Script instance | Window ID | Extra |
|---|---|---|---|
| 0 | `0x69E3D347` | `0xC991EDA8` | — |
| 1 | `0x2BC90671` | `0x69E40A1F` | its child `0xE9889775` (as `cIGZWinGen`) is also kept in the lists `+0x198` and `+0x1B0` |
| 2 | `0xAA1F1F57` | `0xCA1F1D9C` | its child `0xABB26B0E` is also kept in `+0x198` and `+0x1BC` |
| 3 | `0x49889894` | `0xC98F49F1` | a `cSC4MySimWinProc` is created, stored at `+0x2E4` and `Init`ed (`0x007EE3D6`) |
| 4 | `0xAA53E3EA` | `0x0A78827A` | — |

   Each panel window is appended (one reference) to the window list `+0x198` (begin/end/capacity `+0x198`/`+0x19C`/
   `+0x1A0`) and to panel *i*'s own list at `+0x1A4 + 12·i`; its `cIGZWinGen` pointer goes to `+0x1E0 + 4·i`; the
   message filter is added to it; it is given the object at `+0x2D8` (generator slot 7). Flag `0x800` and
   `ClearAcceptFocusFlagRecursive` are applied as in step 1, and the panel is aligned to the main panel
   (`AlignWindowSetToWindow`). Before the loop, two helper objects are made: `+0x2D8` (`0x00774550`) and a
   `cSC4NewsTickerWinProc` at `+0x2DC`.
3. **Text fields** (`GetChildAs`, slot 37, text IID `0x212CDC1F`): main panel child ID 1 → `+0x310`; city-window
   children `0x09E418FE` → `+0x314` and `0xC9E41918` → `+0x318`.
4. **News ticker**: script window `0x2A2AED99`, ID `0xCA2AEDC0`, as `cIGZWinGen`, handed to
   `cSC4NewsTickerWinProc::Init` (`0x007EE6C1`), then aligned and appended to `+0x198`.
5. **Four more script windows**, each queried as `cIGZWin` (`0x22BA0121`), aligned, and kept in its own field:

| Script instance | Window ID | Field | Winproc created after it |
|---|---|---|---|
| `0x4BC906B5` | `0x6A64E3C0` | `+0x30C` | — |
| `0x2BC9060F` | `0x00004200` | `+0x300` | — |
| `0x6BC9065A` | `0x0A4A8176` | `+0x304` | `cSC4GraphsViewWinProc` at `+0x2F8` (`Init` `0x0076CA60`) |
| `0x6BC61F19` | `0xABC619D2` | `+0x308` | `cSC4BuildingSelectWinProc` at `+0x2FC` (`Init` `0x00769420`) |

6. `cSC4BudgetUIWinProc` (0x1A0 bytes) at `+0x2E8`, `Init`ed; a `cSC4WinCalloutBox` (0x1B4 bytes) at `+0x3D8`; a
   `cGZWinBMP` at `+0x3D4` with its surface `+0x3D0`; an image at `+0x3E0`; an object at `+0x31C` (`0x00762E70`);
   an unstored overlay window and a winproc for window `0xEA8CAD14` (`0x00769E90`). (Round 2 listed `+0x1F4` here:
   corrected, `AddGUI` only clears it, step 7.)
   In detail (`0x007EED41`–`0x007EF108`):
   - **Callout box `+0x3D8`**: style IDs `0xA85F1A83` (its slot 11) and `0xE9C86C9D` (slot 10), each looked up
     through `RZ::WCM` and kept by the box; hidden (`HideWindow` on its `cIGZWin` at box `+4`); background image
     `{0x856DDBAC, 0x46A006B0, 0x14416190}` set with corners `(-1, -1)` (box slot 4; −1 selects 8) only when the
     image loads; the image `{0x856DDBAC, 0x46A006B0, 0x14416160}` is kept at `+0x3E0` (assign: AddRef new,
     release old); the box is added to the city window (`ChildAdd`, slot 14).
   - **`cGZWinBMP` (0xFC bytes) `+0x3D4`**, its `cIGZWinBMP` (object `+0xD8`): `SetWinBMPFlag(6, 1)`; window flags
     `0x200000` and `4` on; hidden. A drawing surface is requested from the graphic system at `0x00B43C9C` (`CreateBuffer`, slot 3,
     out = `+0x3D0`, the old one released first); when it succeeds the surface is set up 128 × 32 (its slot 3,
     `(0x80, 0x20, {1, 0})`), becomes the BMP's image (`SetBMP`), and the image area is the surface rectangle moved
     to the origin, `(0, 0, w, h)` (`SetImageArea`). The BMP window is added to the city window either way.
   - **`+0x31C`**: a 0x18-byte object (`0x00762E70`), AddRef, its slot 5 (`Init`).
   - **A full-size overlay window** (not stored): GZCOM `GetClassObject(0xCA5D3294, IID 0xCA5D3290)`; when it
     succeeds its `cIGZWin` gets `SetArea(0, 0, client width, client height)` (width/height from the city window's
     rectangle `+0xB4`..`+0xC0`), ID `0x6A5E44B6`, flag `0x10000` **off**, flags `0x200000` and `4` on, and is added
     to the city window; the class-object reference is released.
   - **Window `0xEA8CAD14`**: the city window looks it up recursively as `cIGZWinGen` (`GetChildAsRecursive`, slot 37,
     IID `0x5386D516`); when found it gets a new 0x14-byte object (`0x00769E90`) through its slot 7 *[inferred]* a
     winproc; nothing is stored in this object.
7. When the city is **not** established (city slot 22 false, `0x007EF11D`): `+0x1F4` := 0, `ShowPanel(0)`
   (`0x007E8B90`), and `nsGZWinUtils::SetWindowEnabled(main panel, 0x99887766, false)` and `(…, 0x8B96B73E, false)`.
   For an established city none of this runs. The rest (`0x007EF159`–`0x007EF1B6`) frees AddGUI's locals.

   Not created by `AddGUI`: `+0x2EC` and `+0x2F0` (zeroed in the constructor, released by `RemoveGUI`); no
   `cSC4View3DWin` method stores them directly (VW6).

**`RemoveGUI`, in order.** "Destroy *w*" means: the window manager (`cGZWin` member `+0x10` of the object, i.e. the
city window's manager) `DestroyWindow(w)` (manager slot 23), then `w->Shutdown()` (slot 5), then release, then the
field := null. Each step is skipped when its field is null.

1. Destroy the main panel `+0x18C`; release `+0x190`; destroy `+0x194`.
2. For every window in the list `+0x198`: remove the message filter `+0x08` from it (slot 117, `+0x1D4`), then
   `DestroyWindow` and `Shutdown` (the list's references are dropped when the list is cleared).
3. Shut down and release the winprocs: news ticker `+0x2DC` (`0x00772A70`), MySim `+0x2E4`, then release `+0x2D8`,
   `+0x2EC` (after its slot 5), `+0x2F0`.
4. Destroy `+0x30C`, `+0x300`, `+0x304`, `+0x308`; shut down and release Graphs `+0x2F8`, BuildingSelect `+0x2FC`,
   Budget UI `+0x2E8` (`0x00764B70`).
5. Destroy the three text fields `+0x310`, `+0x314`, `+0x318` (through their `cIGZWin` interface, slot 3).
6. The callout box `+0x3D8` is removed from the city window (window slot 16, `+0x40`) and released; release `+0x3D0`,
   `+0x3D4`; clear `+0x3DC` (`0x007EDE50`); release `+0x3E0`; `+0x31C` gets its slot 6 then is released.

A replacement must remove the message filter from every tracked panel before destroying it. The parent-owned
unstored overlay survives until base window teardown; see §6m (VW14).

## 6c. Input handlers: keys, characters and the mouse *[compat]*

All offsets are from the start of the object. The mouse and character handlers are `cIGZWin` slots (§1.1), so the
code's `this` is `+0x0C` and its displacements are 0xC smaller; `ProcessKeyDown`/`ProcessKeyUp` are called from the
window `DoMessage` with `this` = object start (`0x007F3F14`).

**State the handlers use.**

| Field | Meaning | Written by |
|---|---|---|
| `+0x11C` | modifier flags of the last input event | every key and mouse handler, first thing (`0x007E66DB`, `0x007E68D9`, ...) |
| `+0x134`/`+0x138` | the view input control **stack** (begin/end of a vector of control pointers; the top is the last element; `GetCurrentViewInputControl` returns it, null when empty, no reference added, `0x007E65E0`) | `SetCurrentViewInputControl` / `Remove...` |
| `+0x140`/`+0x144` | the **persistent** view input controls (vector; `AddPersistentViewInputControl` appends, `0x007EBED1`) | `Add/RemovePersistentViewInputControl` |
| `+0x348`, `+0x34C` | keyboard scroll velocity pair; `KillKeyboardScrolling` (`0x007E7F90`) zeroes both and calls `RecalcScrollVelocity` | key scroll commands |
| `+0x350`, `+0x354` | edge-scroll direction X / Z, each −1, 0 or +1 | `GZOnMouseMove`, `GZOnMouseExit`, focus loss (§6) |
| `+0x358`, `+0x35C` | client x, z of the last right-button press | `GZOnMouseDownR` |
| `+0x360` (byte) | "moved since the right press" flag: 0 at the press, 1 once the pointer is more than 3 pixels away on either axis | `GZOnMouseDownR` (0), `GZOnMouseMove` (1) |

Control slots used below are the view-input-control table of city-tools-input.md §2: 13 `OnCharacter`, 14/15
`OnKeyDown`/`OnKeyUp(key, modifiers)`, 16–19 `OnMouseDownL/DownR/UpL/UpR(x, z, modifiers)`, 20 `OnMouseMove`, 21
`OnMouseWheel(x, z, modifiers, delta)`, 22 `OnMouseExit`, 11 `IsSelfScrollingView`, 25 `AmCapturing`. Every visit
of a control is bracketed by AddRef and Release. Lists are walked **from the last element to the first**.
Two visiting modes appear:

- *first-taker*: stop at the first control that returns true;
- *broadcast*: call every control, ignore the results.

**Dispatch order per handler.**

| Handler (address) | Before the controls | Controls, in order | After | Returns |
|---|---|---|---|---|
| `ProcessKeyDown(key, mods)` (`0x007E66D0`) | store mods | stack first-taker (slot 14), then persistent **broadcast** (slot 14) — the persistent pass runs even when a stack control took the key | — | true when any stack control **or any persistent control** returned true (`0x007E6747`, `0x007E67A6`) |
| `ProcessKeyUp(key, mods)` (`0x007E67D0`) | same, slot 15 | same | — | same (`0x007E6847`, `0x007E68A6`) |
| `GZOnCharacter(ch)` (`0x007E6600`) | — | stack first-taker (slot 13); only when none took it, persistent first-taker with a bounds quirk (below) | — | always true |
| `GZOnMouseDownL(x, z, mods)` (`0x007E68D0`) | store mods; tutorial filter (below), mode `-1` | persistent first-taker (slot 16), then stack first-taker | — | always true |
| `GZOnMouseDownR(x, z, mods)` (`0x007E6A00`) | store mods; **take mouse capture** (`GZSetCapture`) | persistent first-taker (slot 17), then stack first-taker | if nobody took it: start a mouse scroll (below) | always true |
| `GZOnMouseUpL(x, z, mods)` (`0x007E8170`) | store mods; scroll re-check (below); tutorial filter, mode `1` | persistent first-taker (slot 18), then stack first-taker | — | always true |
| `GZOnMouseUpR(x, z, mods)` (`0x007E82E0`) | store mods; **release capture** if the window holds it; click-to-centre (below) | persistent first-taker (slot 19), then stack first-taker | — | always true |
| `GZOnMouseMove(x, z, mods)` (`0x007E9460`) | store mods; tutorial filter, mode `0`; drag flag; `UpdateCursorTextPositions(x, z)` (`0x007E9090`) | persistent **broadcast** (slot 20), then stack first-taker | edge scrolling (below) | always true |
| `GZOnMouseWheel(x, z, mods, delta)` (`0x007E6B50`) | store mods | **stack first-taker first** (slot 21); if one took it, return | else persistent broadcast, then `ZoomIn` (view slot 9) when `delta > 0`, `ZoomOut` (slot 10) when `delta ≤ 0` (`0x007E6C1F`) | always true |
| `GZOnMouseExit()` (`0x007E96E0`) | edge directions := 0, `RecalcScrollVelocity`; `UpdateCursorTextPositions(−10000, −10000)` | slot 22 on the **top stack control only** (no AddRef), then persistent broadcast | — | always true |

So the persistent controls see mouse buttons before the tool on the stack, but keys and the wheel after it.
`GZOnMouseDownL` does not take capture and `GZOnMouseUpL` does not release it; only the right button does.

**Tutorial filter** (`DownL`, `UpL`, `Move`). When the city (`0x00B43CEC`) exists and its tutorial system (city
slot 82) reports `IsTutorial` (slot 8), the handler calls `FilterLeftMouseClick(&x, &z, mode)` (slot 16; mode `-1`
down, `1` up, `0` move; tutorial.md). False: the handler returns true at once, no control sees the event (and on a
move nothing below runs, not even the drag flag). True: the event continues with the possibly rewritten x, z. On a
move, the result of `IsEdgeScrollingAllowed` (tutorial slot 9) then gates edge scrolling; without a tutorial edge
scrolling is allowed.

**Mouse scroll start** (`DownR`, `0x007E6AD5`–`0x007E6B3B`). Only when no control took the press: store x, z at
`+0x358`/`+0x35C`, clear the drag flag `+0x360`, create a `cSC4ViewInputControlCityScroll` (0x48 bytes, constructor
`0x004B7DC0`), AddRef it, make it the current control with `SetCurrentViewInputControl(control, 0)` (view slot 25),
call its `SetMouseScroll(x, z)` (`0x004B82F0`), then Release it (the stack keeps its own reference).

**Scroll re-check on left release** (`0x007E8188`–`0x007E81B4`). When the window holds capture and the current
control exists, is not self-scrolling (slot 11 false) and reports `AmCapturing` (slot 25 true):
`RecalcScrollVelocity`. Nothing else changes.

**Click-to-centre on right release** (`0x007E8303`–`0x007E8428`). Only when the drag flag `+0x360` is 0 and
`mods & 7` is 0 (no modifier key):

1. `Pick(x, z, 0, &occupant)` on the renderer `+0xF4` (renderer slot 65). On a hit: ask the occupant (slot 8) for its
   bounding box into a box preset to min (1, 1, 1), max (0, 0, 0); take the box centre; call the terrain
   (`0x00B43CF4`) `GetAltitude` and `GetWaterLevel` at the centre's x, z and **discard both results** *[quirk]*; then
   jump the camera control `+0xF0` to the box centre (`0x007CD810`, the routine the Lua `camera_jump` uses).
2. No occupant: `PickTerrain(x, z, &point, 0, 1)` (renderer slot 60), point preset to the value at `0x00B4E43C` in
   all three components; on a hit jump the camera to `point`; on a miss nothing.
3. A picked occupant reference is released.

Then the controls see the release as in the table. A right *drag* (more than 3 pixels on either axis since the press)
never recentres.

**Drag flag** (`0x007E94BD`–`0x007E94E9`). On every move that passes the tutorial filter: if |`+0x358` − x| > 3
or |`+0x35C` − z| > 3, `+0x360` := 1. It is set whether or not a button is held; only `DownR` clears it.

**Edge scrolling** (`0x007E95C3`–`0x007E96CD`), after the controls, with `c` = the current control:

1. Not allowed (tutorial said no, or the preference byte `+0xF09` of the object at `0x00B43CD8` is 0): directions :=
   0, `RecalcScrollVelocity`.
2. Else, when `c` is null or not self-scrolling, **and** (global `0x00B43C9C` slot 12 returns true, or `c` exists and
   `AmCapturing`): X := −1 if x ≤ 1, +1 if x ≥ width − 1, else 0 (width = window slot 41); Z the same with z and
   height (slot 42); `RecalcScrollVelocity`. `0x00B43C9C` is the graphic system (service `0xC416025C`,
   interface `cIGZGraphicSystem` `0x73283C`, stored at `0x00602384`), and slot 12 is `IsFullScreenMode` (SDK order).
3. Otherwise: if `0x00B43C9C` slot 12 is true, keep the directions; else, if either direction is non-zero, zero both
   and `RecalcScrollVelocity`.

**`GZOnCharacter` bounds quirk** (`0x007E665B`–`0x007E66A7`). The persistent pass starts at index (persistent count −
1) but checks each index against the **stack's** count: it stops as soon as the index is not below the stack size.
With more persistent controls than stack controls the pass does nothing; with fewer, it is a normal top-down
first-taker pass. A replacement that wants identical character routing must keep this bound *[quirk]*.

## 6d. Fields *[compat]*

*Added 2026-10-21 (re-view3dwin).* Offsets are from the object start (`+0`); the input handlers and `GZPaint`
receive `this + 0x0C`. They extend ui-windows.md §3.2 and view3d.md §5.2, and give the meanings §2 marks inferred.

| Offset | What | Set by |
|---|---|---|
| `+0xE4` | byte: initialised (`cGZWin::Init` result) | `Init` |
| `+0xE5` | byte: city initialising (1 in `DoPreCityInit` until the end of `DoPostCityInit`) | §6a |
| `+0xE8` | animation tick manager | §6a |
| `+0xEC`, `+0xF0`, `+0xF4`, `+0xF8` | the S3D camera (`cS3DCamera`, camera.md), the camera control, the 3D renderer, the model maker | §6a |
| `+0x100`, `+0x160` | paint timer; hidden-pause timer | ctor, `Init` |
| `+0x118` | byte: the mouse is inside the window | §6 |
| `+0x11C` | the last modifier flags of a key or mouse event | §6c |
| `+0x120` | the key accelerator (held) | `Init` step 8 |
| `+0x124` | byte: animation debugging allowed (cheat `WatchMeMove`) | §3.1 |
| `+0x125`, `+0x126` | byte 1: the God-mode keyboard override is allowed (§6g); byte: terrain query on (`TerrainQuery`, slot 39) | ctor, §3.1 |
| `+0x128` | automatic message subscriptions (`cRZAutoMsg2Notification`) | ctor, `Init` |
| `+0x134`…`+0x13C` | vector: the **tool stack** (§6e) | §6e |
| `+0x140`…`+0x148` | vector: the **persistent** controls | §6e |
| `+0x14C` | list of deferred messages (re-sent from `GZPaint` when popups are safe) | §5 |
| `+0x150` | an object made by GZCOM class `0xA2FFB5D3` (interface `0x02FFBB60`) *[inferred: image loader]* | §6a |
| `+0x154` | the simulator (held) | §6a |
| `+0x158` | background image mode: 0 none, 1 2D, 2 3D | §6l |
| `+0x15C` | byte: painting enabled | `Init`, §6a |
| `+0x178`, `+0x17C` | byte: hidden pause active; the auto-resume delay in ms (default 2000) | §6f |
| `+0x18C`, `+0x190`, `+0x194` | the control panel window, its `cIGZWinGen`, the restore-UI button | §6b |
| `+0x1A4` + 12·n | vector of the buttons of panel n | §6b |
| `+0x1F4` | the current panel (−1 none, 0–4) | §6h |
| `+0x1F8`…`+0x208` | the catalog menus (ui-windows.md §4) | §6i |
| `+0x20C`…`+0x2CC` | 8-byte remembered catalog states, one per menu (the `{id, 0}` pairs §2 lists: zone menus 1, 4, 7; transport and landscape items) | ctor, §6i |
| `+0x2D4`…`+0x2D6` | bytes: the mayor, MySim and panel-4 sub-windows are shown | §6h |
| `+0x2D8`, `+0x2DC` | the news-ticker list holder; the news ticker | §6b |
| `+0x2E4`, `+0x2E8`, `+0x2F8`, `+0x2FC` | MySim, budget, graphs and building-select window procedures | §6b |
| `+0x300`…`+0x30C` | four scripted mayor sub-panels | §6b |
| `+0x310`, `+0x314`, `+0x318` | the date, funds and population labels | §6j |
| `+0x31C` | the advisor popup tracker (advisors.md) | §6b |
| `+0x320`, `+0x324` | the view filter (held, `cISC4OccupantFilter`); the view level (default 1) | §6f |
| `+0x328`, `+0x32C`, `+0x330` | scroll angle, scroll speed, byte scrolling | §6f |
| `+0x334`, `+0x338`, `+0x339`, `+0x33C` | the stop step (1.0), byte proportional stop, byte stopping, the step in use | §6f |
| `+0x340`, `+0x344` | the edge and keyboard scroll speeds (0.2 each) | §6f |
| `+0x348`, `+0x34C`; `+0x350`, `+0x354` | keyboard x, y directions; edge x, y directions (−1, 0, 1) | §6c, §6f |
| `+0x358`, `+0x35C`, `+0x360` | the right-press point; byte dragged | §6c |
| `+0x364`, `+0x368` | the zoom limits (0 and 5) | §6f |
| `+0x36C`, `+0x370`, `+0x374` | the mayor-rating bar position; the current mayor sub-panel (`0x15200003`); byte none shown | §6h, §6j |
| `+0x378`, `+0x37C` | the rating arrow's home position | §6j |
| `+0x384`…`+0x3AC` | render properties saved while scrolling | §6f |
| `+0x3B0`…`+0x3E0` | the cost indicator and the cursor-text callout | §6j |
| `+0x3E4` | the coverage-radius helper (slot 49) | §6l |
| `+0x3FC`, `+0x400` | byte contours shown; the contour layer id (−1) | §6f |
| `+0x404`, `+0x408` | the progress screen; byte "not loaded" (1, cleared by `DoLoad`) | §6k |
| `+0x40C` | the mission status panel | §6j |
| `+0x428`…`+0x438` | hash map trigger → count (the global effect triggers), element count at `+0x438` | §6k |
| `+0x43C`, `+0x440`, `+0x444`, `+0x445` | the last selected building and lot, and their "set" bytes | slots 51–54 |
| `+0x446` | byte: every reward building available (cheat) | §3.1 |

## 6e. The view input control stack *[compat]*

*Added 2026-10-21 (re-view3dwin).* §6c gives how events visit the controls; these slots change the lists.

- `SetCurrentViewInputControl(c, mode)` (slot 25, `0x007EBD70`): `c.SetWindow(view)`; the old top is held and
  `SendViewInputControlChangeMessage(old, c)` runs (below); mode 1 pops the top (slot 26, false), mode 2 pops all
  (slot 27, false), mode 0 stacks; the top that is now there (if any) gets `OnMouseExit` and `Deactivate`; then, if
  `c.Init()` succeeds, `c` is pushed (held), `Activate`d, and its cursor (slot 7), when it has one, becomes the
  window's cursor. Returns true. Tools are normally set with mode 2; the query and demolish tools with mode 0.
- `RemoveCurrentViewInputControl(silent)` (slot 26, `0x007E9390`): false with an empty stack. Otherwise the capture is
  released, the top popped, `Deactivate`d, `Shutdown` and released; unless `silent`, the new top's cursor (or the
  default cursor `0xC16C4E1B`) is set and, with a new top, the change message is sent. Returns true.
- `RemoveAllViewInputControls(silent)` (slot 27): slot 26 until it returns false.
- `AddPersistentViewInputControl(c)` (slot 28): when `c.Init()` succeeds: `SetWindow`, held, appended, `Activate`.
  `RemovePersistentViewInputControl(c)` (slot 29): `Deactivate`, `Shutdown`, release and erase; true when found.
  `RemoveAllPersistentViewInputControls` (slot 30): the same from the back; true.
- `SendViewInputControlChangeMessage(old, new)` (`0x007E8090`): posts message `0x09C8B8EA` (a 0x30-byte message class
  with one extra held pointer) with void1 = `old`, void2 = `new`, and `new` also held in the message.

## 6f. Scrolling, zoom and views *[compat]*

*Added 2026-10-21 (re-view3dwin).*

**Paint timing and inverted zoom limits (VW11).** The constructor writes **0.0f** to `+0xFC`
(`0x007F0AE1`); the period is in **microseconds**, like `GetElapsedTime` on `+0x100`.
A zero period is always due. For a nonzero period, drawing is due only when the unsigned elapsed tick is
**strictly greater** than the period; equality skips the draw. The timer restarts only on a due draw.
The update duration is elapsed microseconds times the binary64 constant `1e-6`, capped at binary64 `0.5`,
then narrowed to binary32 for owner updates (`0x007F623A`–`0x007F62AF`). The deferred-message tail still runs
when disabled or not due (§6n). `SetZoom` uses signed comparisons: below minimum selects minimum; only
otherwise does above maximum select maximum (`0x007E60EE`–`0x007E60FC`). Thus minimum 4, maximum 2 maps
request 3 to 4 and request 4 to 2. Limits are never sorted. `SetZoomLimits` stores the supplied minimum,
applies its existing preference cap to the stored maximum, and chooses a corrective request by the same
minimum-first rule against the **supplied** maximum (`0x007E61A9`–`0x007E61C0`), passing it through `SetZoom`.
It returns true even for inverted limits.

**Scrolling.**
- `RecalcScrollVelocity` (`0x007E6C60`): `kb` := either keyboard direction is non-zero; `f` := 1.2 to the power of
  the camera's zoom (camera `+0x108`). When the preference byte `+0xF09` (edge scrolling) is set, the edge
  directions are **cleared after being read** *[quirk: so a later recomputation that is not caused by a mouse move
  forgets the edge]*. Per axis, a non-zero keyboard direction replaces the edge one. With no direction: `ScrollStop`
  (slot 34) unless already stopping. Otherwise `SetScrolling(true, angle, (kb ? +0x344 : +0x340) · f)` with `angle =
  atan2(y, x)` taken from the table (1, 0) 0, (1, 1) π/4, (0, 1) π/2, (−1, 1) 3π/4, (−1, 0) π, (−1, −1) 5π/4, (0, −1)
  3π/2, (1, −1) 7π/4 (y grows downward on the screen).
- `SetScrolling(on, angle, speed)` (slot 33): starting calls `DoScrollBegin`, stopping `DoScrollEnd`; then
  scrolling := on, the angle and speed are stored and "stopping" is cleared.
- `ScrollStop` (slot 34): while scrolling, stopping := 1 and the step := `+0x334`, times the speed when the
  proportional byte is set.
- **Per paint** (view3d.md §5.1 steps 5–8): `dt` = the timer's microseconds × 10⁻⁶, at most 0.5; while scrolling the
  camera's `ScrollByViewportAngle(angle, dt · speed · 25)`; while stopping, speed −= step, and at ≤ 0 `DoScrollEnd`
  and scrolling := 0. A true result of the camera call starts a hidden pause.
- `DoScrollBegin` / `DoScrollEnd` post `0xCAD61E00` / `0xEAD61E16` (mouse scroll begin / end). When the render setting
  byte `+0x3DC` is set, `Begin` saves the application's render properties, turns properties 9, 10, `0x20` and `0xD`
  off and sets property 9 to property 10's value, and calls `0x00B43DDC` slot 7 with 0.1; `End` restores them, calls
  slot 8 and refreshes the renderer (slot 20) *[summary]*.
- `KillKeyboardScrolling` (slot 35): keyboard directions := 0, recompute.

**Zoom and rotation.**
- `SetZoomLimits(min, max)` (slot 11): store; a `max` above 4 becomes 4, or 5 when the preference byte `+0xF10` is set
  (5 without preferences). If the camera's virtual zoom is outside [min, the **unclamped** max], `SetZoom`.
- `SetZoom(z)` (slot 8) clamps to the limits; `ZoomIn` / `ZoomOut` (slots 9, 10) need the virtual zoom below the
  maximum / above the minimum; `CanZoomIn` / `CanZoomOut` (12, 13) are those tests; `SetRotation`, `RotateLeft`,
  `RotateRight`, `SetZoomAndRotation` (14–17) forward. Each forwards to the renderer (slots 35–41) and, when it
  returns true, starts a hidden pause.
- `UpdateZoomButtons`: buttons `0x4992046A` (in) and `0xE9920494` (out) enabled by `CanZoomIn` / `CanZoomOut`.

**View filter and view level.**
- `GetCurrentViewFilterInfo(&f)` (slot 18) gives `+0x320` held. `SetCurrentViewFilterInfo(&f)` (slot 19) **sends**
  `0x06CDB65A` (void1 the old filter, void2 the new), replaces the filter and sends `0x06CDB65B` (which also refreshes
  the renderer, §5).
- `SetCurrentViewLevelInfo(&n)` (slot 21): when n differs, level := n, then sends `0xC6CDBF1A` and `0xC6CDBF1B`
  (foundations.md §5.2). `SetCurrentViewLevel(n)` (slot 23) calls slot 21 when n differs; slots 20 and 22 read it.

**Hidden pause.** Each successful zoom or rotation, each camera scroll step that reports a move, and message
`0xA6B79621` start a hidden pause: if not active and there is a simulator, active := 1 and the simulator's
`HiddenPause`; the timer `+0x160` restarts. `GZPaint` resumes (simulator `HiddenResume`, timer reset) once the timer
exceeds `+0x17C` (2000 ms).

**Contours and terrain.** `ToggleContourDisplay`: flips `+0x3FC`; on, the terrain view's layer list (slot 18) adds
layer `0x07F94205` and keeps its id; off, it removes it. `ChangeTerrainType`: the terrain type (terrain slot 42)
through the change-terrain dialog, then set (slot 41). `PickTerrain` / `PickOccupant` (slots 36, 37) forward to the
renderer (the occupant through interface `0x26BB767E`; with no renderer the point is zeroed and the result false).

**Tuning.** `ReadTunableValues` (`0x007EF820`, from `DoPreCityInit`) reads the exemplar (`0x6534284A`, `0x690F693F`,
`0xE90F6C84`): `0x4975DD20` → the stop step `+0x334`, `0x4975DD25` → proportional stop `+0x338`, `0x49E5522F` → the
edge speed `+0x340`, `0xA9E581FB` → the keyboard speed `+0x344`. Then every entry of the ini section `"Render
Properties"` and every command-line `RenderProp` switch (`name,value` or `name value`) goes through
`SetRenderProperties` (§3.1).

## 6g. Commands (`ProcessCommand`, slot 7, `0x007F1C10`) *[compat]*

*Added 2026-10-21 (re-view3dwin); answers VW3.* The 104 records of the table `0x00B0DD98` that `Init` registers
(§3 step 2) and what `ProcessCommand` does with each. `ProcessCommand` **always returns false**. Tools are set with
`SetCurrentViewInputControl(tool, 2)` unless noted; "established" means the command does nothing in an unestablished
city. Ids are given in full or as the last digits of `0x6A935xxx`.

| Commands (id) | Behaviour |
|---|---|
| `ToggleGodPanel` `6A935C34`, `ToggleMayorPanel` `…CA4`, `ToggleMySimPanel` `…CA8`, `ToggleOptionsPanel` `…CAB` | flip the panel button `0x2988BC85` / `0xC988BC79` / `0x4988BC6A` / `0x8988BC94` of the control panel (button interface `0x8810`: slot 4(1) true → slot 7 off, else slot 6 on). The God-mode override: `ToggleGodPanel` opens panel 0 (God mode) instead of 4 in an established city when `+0x125` is set and Shift, Ctrl and Alt are all down |
| `SimSpeedPause` `…CAD` | the simulator paused (slot 15) ? `Resume` : `Pause` |
| `SimSpeedSlow` `…CB0`, `SimSpeedMedium` `…CB2`, `SimSpeedFast` `…CB5` | `SetSimSpeed(0 / 1 / 2)`, resume if paused, `UpdateSpeedButtons` |
| `ZoomIn` `…CB7`, `ZoomOut` `…CBA`, `Zoom0`…`Zoom4` `…CBD`/`CBF`/`CC2`/`CC4`/`CC7`, `Zoom5` `2BCBD5F1` | slot 9, slot 10, `SetZoom(0…5)` |
| `RotateCW` `…CC9`, `RotateCCW` `…CCC` | `RotateLeft` (slot 15), `RotateRight` (slot 16) |
| `ScrollLeft` `…CD8`, `ScrollUp` `…CDD`, `ScrollRight` `…CE0`, `ScrollDown` `…CE2` | only when **both** keyboard directions are 0: x := −1 / y := −1 / x := 1 / y := 1, recompute |
| `Scroll*Stop` `2A948275`/`272`/`26E`/`26B` | both keyboard directions := 0, recompute |
| `Scroll*Once` `2A9484DA`/`4F2`/`4FF`/`50B` | only when both keyboard directions are 0: `ScrollByViewportAngle(π / 3π/2 / 0 / π/2, 1)` |
| `CenterOnCursor` `…CE5` | the cursor position in the view (window manager slot 34); terrain pick; view target |
| `ToggleGridVisibility` `…CE8` | post `0x277A9364` |
| `Cancel` `…CF1` | non-empty stack: slot 27(false); close the tertiary, then the secondary menu |
| `QueryTool` `…CF3`, `TrafficQueryTool` `…CF4`, `DemolishTool` `…D36` | established; a top tool that does not stack (slot 12) is popped; nothing if the top is already the tool (ids `0xC7AF928E`, `0xC7AF928F`, `0x46DDB5F1`); else a query tool (0, 50) / (1, 50) or the demolish tool, mode 0 |
| zone tools `…CF7`…`…D2E` | `InvokeZoningTool(z)`: DeZone 0, low/medium/high residential 1/2/3, commercial 4/5/6, agriculture 7, medium/high industry 8/9, landfill 14 (§6i) |
| network tools | established; network tool type: Road 0, Rail 1, Highway 2, Street 3, Pipes 4, PowerLines 5, Avenue 6, Subway 7, LightRail 8; ids `8BE098F4` 9, `4BE098F7` 10, `6BE098FA` 11, `4BE098FD` 12 (network-tools.md) |
| place-lot tools | `InvokeCatalogItemCommand(0x6A2B245F, lot, false)`: bus stop `0x05000001`, passenger depot `0x6A8A358F`, freight depot `0x6A3855D4`, subway station `0x05000002`, small/large police `0x69DE426B`/`0x01000005`, jail `0x01000003`, small/large fire `0xA9DD1E4C`/`0x01000007`, clinic `0x2A2735A0`, hospital `0xEA273867`, elementary/high school `0x89DD094E`/`0x4A2AF441`, college `0x4A2829D1`, light-rail station `0x4BAFBA9F`, ferry `0x0B96AE64`, police kiosk `0x6BB942D7` |
| `PlaceLotTool` `EC3E82F8` | the lot id is the command's parameter 0 (its value pointer, slot 73); 0 does nothing |
| `DispatchFireTool` `…D39`, `DispatchPoliceTool` `CA935DBD`; ids `8C0B0E46`, `EC0B0E6A` | established; dispatch tool 1, 0; 5, 6 |
| `EnableDayAndNight` `…E27`, `EnableDayOnly` `…E1E`, `EnableNightOnly` `…E21` | visual time mode 0 / 1 / 2 (§6j) |
| `OpenPlayOptionsDialog` `…E2A`, `…Graphic…` `…E2D`, `…Audio…` `…E34` | the game / video / audio preferences dialog, modal; focus back to the view |
| `OpenPhotoAlbumDialog` `…E45` | the snapshot album for the current city, modal; the previous focus restored (snapshots.md) |
| `OpenSnapshotDialog` `…E4B` | a snapshot taker over the view |
| `SaveCity` `…E36`, `SaveCityQuick` `…E37` | `RequestSaveCity(not parameter 0, quick)`; quick for `SaveCityQuick` or with Alt down |
| `QuitCity` `…E39`, `QuitGame` `…E3C` | `RequestGoToRegionView(not parameter 0)`; `RequestQuit(not parameter 0, parameter 1)` |
| `ToggleUIVisibility` `…E3F` | `MinimizeUI(the control panel is visible)` (§6h) |
| `ToggleTerrainContourDisplay` `2A94A04B`, `OpenObliterateCityDialog` `2A94A5B6` | §6f; established: `ObliterateCity` (§6h) |
| `SetExpandedToolTips` `6AA9FE51` | flips the preference byte `+0xF0A` |
| `SignTool` `0B980581`, `LabelTool` `0B980587`, `DemolishSignsAndLabels` `ABCDEE11` | the sign, label and sign/label demolish tools (view-input-controls.md) |
| `ToggleSignsAndLabels` `6B98058A` | a sign, label or sign-demolish tool on top (ids `0x8B3CE416`, `0x8B4CE5A0`, `0xCBCC987E`) is removed when signs are shown; then the prop developer's toggle (prop-developer.md §4.11) |
| `FullScreenRefresh` `6A9391A0` | renderer slot 20 |
| `PauseAnimation` `6A9753EA`, `StepAnimation` `…3ED` | only after `WatchMeMove`: the animation tick manager paused/unpaused, or one step |
| `AnimationRecorder` `CAB22C64` | `AddWinAnimationRecorder(true)` (§3.1) |
| `RotateLotCW`/`CCW` `…CD5`/`CD1`, `OpenCheatCodeDialog` `2A9496C9`, `RegionBitmapLoad` `6A9757C2` | not handled here |

Parameter 0 and 1 are read through the command's parameter set without a null test *[quirk]*.

## 6h. Panels and the control panel *[compat]*

### Complete command-name registry (VW9, §6g supplement)

`Init` registers exactly these **104 records in this order**, from `0x00B0DD98`, stride 12 bytes;
every record's third word is **0** (`0x007E9959`–`0x007E9969`). Preserve even the commands that
`ProcessCommand` does not handle here; registration is not evidence of a local dispatch case. The ID/name
mapping also supplies the bus-stop (`0x6A935DC9`) and pipe (`0x6A935DEE`) command-server IDs.

| Order | ID | Name |
|---|---|---|
| 1 | `0x6A935C34` | `ToggleGodPanel` |
| 2 | `0x6A935CA4` | `ToggleMayorPanel` |
| 3 | `0x6A935CA8` | `ToggleMySimPanel` |
| 4 | `0x6A935CAB` | `ToggleOptionsPanel` |
| 5 | `0x6A935CAD` | `SimSpeedPause` |
| 6 | `0x6A935CB0` | `SimSpeedSlow` |
| 7 | `0x6A935CB2` | `SimSpeedMedium` |
| 8 | `0x6A935CB5` | `SimSpeedFast` |
| 9 | `0x6A935CB7` | `ZoomIn` |
| 10 | `0x6A935CBA` | `ZoomOut` |
| 11 | `0x6A935CBD` | `Zoom0` |
| 12 | `0x6A935CBF` | `Zoom1` |
| 13 | `0x6A935CC2` | `Zoom2` |
| 14 | `0x6A935CC4` | `Zoom3` |
| 15 | `0x6A935CC7` | `Zoom4` |
| 16 | `0x2BCBD5F1` | `Zoom5` |
| 17 | `0x6A935CC9` | `RotateCW` |
| 18 | `0x6A935CCC` | `RotateCCW` |
| 19 | `0x6A935CD5` | `RotateLotCW` |
| 20 | `0x6A935CD1` | `RotateLotCCW` |
| 21 | `0x6A935CD8` | `ScrollLeft` |
| 22 | `0x2A9484DA` | `ScrollLeftOnce` |
| 23 | `0x2A948275` | `ScrollLeftStop` |
| 24 | `0x6A935CDD` | `ScrollUp` |
| 25 | `0x2A9484F2` | `ScrollUpOnce` |
| 26 | `0x2A948272` | `ScrollUpStop` |
| 27 | `0x6A935CE0` | `ScrollRight` |
| 28 | `0x2A9484FF` | `ScrollRightOnce` |
| 29 | `0x2A94826E` | `ScrollRightStop` |
| 30 | `0x6A935CE2` | `ScrollDown` |
| 31 | `0x2A94850B` | `ScrollDownOnce` |
| 32 | `0x2A94826B` | `ScrollDownStop` |
| 33 | `0x6A935CE5` | `CenterOnCursor` |
| 34 | `0x6A935CE8` | `ToggleGridVisibility` |
| 35 | `0x6A935CF1` | `Cancel` |
| 36 | `0x6A935CF3` | `QueryTool` |
| 37 | `0x6A935CF4` | `TrafficQueryTool` |
| 38 | `0x6A935CF7` | `ZoneLowResidentialTool` |
| 39 | `0x6A935D0B` | `ZoneMediumResidentialTool` |
| 40 | `0x6A935D0E` | `ZoneHighResidentialTool` |
| 41 | `0x6A935D11` | `ZoneLowCommercialTool` |
| 42 | `0x6A935D17` | `ZoneMediumCommercialTool` |
| 43 | `0x6A935D1A` | `ZoneHighCommercialTool` |
| 44 | `0x6A935D1D` | `ZoneAgriculturalTool` |
| 45 | `0x6A935D25` | `ZoneMediumIndustrialTool` |
| 46 | `0x6A935D2B` | `ZoneHighIndustrialTool` |
| 47 | `0x6A935D28` | `ZoneLandfillTool` |
| 48 | `0x6A935D2E` | `DeZoneTool` |
| 49 | `0x6A935D36` | `DemolishTool` |
| 50 | `0x6A935D39` | `DispatchFireTool` |
| 51 | `0xCA935DBD` | `DispatchPoliceTool` |
| 52 | `0x6A935DC0` | `RoadTool` |
| 53 | `0x6A935DC2` | `StreetTool` |
| 54 | `0x6A935DC5` | `HighwayTool` |
| 55 | `0x6B730F78` | `AvenueTool` |
| 56 | `0x6A935DC9` | `PlaceBusStopTool` |
| 57 | `0x6A935DDA` | `RailTool` |
| 58 | `0x2B79E72B` | `LightRailTool` |
| 59 | `0x6A935DDD` | `PlacePassengerDepotTool` |
| 60 | `0x6A935DDF` | `PlaceFreightDepotTool` |
| 61 | `0x2BFE11C9` | `PlaceLightRailStationTool` |
| 62 | `0x6A935DE3` | `SubwayTool` |
| 63 | `0x6A935DE6` | `PlaceSubwayStationTool` |
| 64 | `0x6A935DEB` | `PowerLinesTool` |
| 65 | `0x6A935DEE` | `PipesTool` |
| 66 | `0x6A935DF1` | `PlaceSmallPoliceStationTool` |
| 67 | `0x6A935DF4` | `PlaceLargePoliceStationTool` |
| 68 | `0x2BFE0BEA` | `PlacePoliceKioskTool` |
| 69 | `0x6A935DF7` | `PlaceJailTool` |
| 70 | `0x6A935DFA` | `PlaceSmallFireStationTool` |
| 71 | `0x6A935DFD` | `PlaceLargeFireStationTool` |
| 72 | `0x6A935E0A` | `PlaceClinicTool` |
| 73 | `0x6A935E0D` | `PlaceHospitalTool` |
| 74 | `0x6A935E14` | `PlaceElementarySchoolTool` |
| 75 | `0x6A935E11` | `PlaceHighSchoolTool` |
| 76 | `0x6A935E17` | `PlaceCollegeTool` |
| 77 | `0x2BFE1095` | `PlaceFerryTerminalTool` |
| 78 | `0xEC3E82F8` | `PlaceLotTool` |
| 79 | `0x6A935E1E` | `EnableDayOnly` |
| 80 | `0x6A935E21` | `EnableNightOnly` |
| 81 | `0x6A935E27` | `EnableDayAndNight` |
| 82 | `0x6A935E2A` | `OpenPlayOptionsDialog` |
| 83 | `0x6A935E2D` | `OpenGraphicOptionsDialog` |
| 84 | `0x6A935E34` | `OpenAudioOptionsDialog` |
| 85 | `0x6A935E45` | `OpenPhotoAlbumDialog` |
| 86 | `0x6A935E4B` | `OpenSnapshotDialog` |
| 87 | `0x6A935E36` | `SaveCity` |
| 88 | `0x6A935E37` | `SaveCityQuick` |
| 89 | `0x6A935E39` | `QuitCity` |
| 90 | `0x6A935E3C` | `QuitGame` |
| 91 | `0x6A935E3F` | `ToggleUIVisibility` |
| 92 | `0x2A9496C9` | `OpenCheatCodeDialog` |
| 93 | `0x2A94A04B` | `ToggleTerrainContourDisplay` |
| 94 | `0x2A94A5B6` | `OpenObliterateCityDialog` |
| 95 | `0x6AA9FE51` | `SetExpandedToolTips` |
| 96 | `0x0B980581` | `SignTool` |
| 97 | `0x0B980587` | `LabelTool` |
| 98 | `0x6B98058A` | `ToggleSignsAndLabels` |
| 99 | `0xABCDEE11` | `DemolishSignsAndLabels` |
| 100 | `0x6A9391A0` | `FullScreenRefresh` |
| 101 | `0x6A9753EA` | `PauseAnimation` |
| 102 | `0x6A9753ED` | `StepAnimation` |
| 103 | `0x6A9757C2` | `RegionBitmapLoad` |
| 104 | `0xCAB22C64` | `AnimationRecorder` |

*Added 2026-10-21 (re-view3dwin).*

**The message filter's branches** (`0x007F58E0`, §6.3; answers VW4). A window message first passes the tutorial's
`FilterUICommandMessage` (false: eaten); a window whose slot 67 test is true lets it pass; type `0x12` is eaten; only
type 3 (commands) is handled: `0x287259F6` is a button press → `HandleButtonActivated` (§6i); `0x287259F7` is a toggle
of a panel tab, by the tab's ID: `0x8988BC94` options (panel 3), `0x4988BC6A` MySim (2), `0xC988BC79` mayor (1, and in
an unestablished city the establish dialog first, below: declined → the button is toggled back off), `0x2988BC85` God
(`DoPanelCommandGUI(ToggleGodPanel)`).

`DoPanelCommandGUI(cmd)` (`0x007EC2D0`) and the filter do the same: read the tab's state; close the secondary menu;
**on**: hide the current panel and untoggle its tab, current := n, `ShowPanel(n)`; **off** while n is current:
current := −1, `HidePanel(n)`. God is panel 0 in an unestablished city and 4 in an established one. When a mayor,
MySim or panel-4 sub-window is shown, the news ticker's slot 5 is called (shown).

- `ShowPanel(n)` (`0x007E8B90`): panel n's buttons get flag 1 on (slot 68); 1 (mayor): `ShowMayorSubPanel(+0x370)`,
  mayor shown; 2: the advisor window hidden, MySim shown; 4: panel-4 flag, MySim hidden; 0: MySim hidden. When none of
  the three flags is set, the news ticker is hidden (slot 6).
- `HidePanel(n)` (`0x007E8CA0`): closes the secondary menu; buttons without flag `0x1000` get flag 1 off; 2 hides
  MySim; 4 clears its flag; 1 hides every open mayor sub-panel (advisor window, budget, `+0x300`, graphs, `+0x30C`,
  building select) and sets "none shown" when none was open.
- `ShowMayorSubPanel(id)` (`0x007E6F60`): with "none shown" clear, shows the one named and hides the others: advisor
  `0x49EDF9B7` (advisor system slots 25 / 26 / 27), budget `0x41`, `0x99887755` (`+0x300`), graphs `0x15200002`,
  `0x15200003` (`+0x30C`), building select `0xABC54125`; `+0x370` := id. Id 0 only records "none shown".
- `MinimizeUI(b)` (slot 55): hides (b) or shows the control panel, the current panel, the `+0x2EC` window and the news
  ticker, closes the secondary menu, shows the restore button `+0x194` instead, and sets window `0xEA8CAD14` visible =
  not b.

**Establishing and obliterating.**
- `DoEstablishCityDialog` (`0x007EB320`): a scripted dialog (`0x96A006B0`/`0x6A414973`) centred horizontally, at a
  third of the free height, with the difficulty from the preferences (`+0x46C`), edit fields `0xAA4149BF` (city) and
  `0xAA4149D2` (mayor). On OK (result 0): the difficulty, the city name (at most 511 characters, also kept in the
  preferences at `+0x6C`), "name changed", the mayor name (preferences `+0x26C`), the preferences saved, the city
  established (city slot 23). Returns whether it was accepted.
- `DoCityEstablished` (`0x26D31EC4`): buttons `0x99887766`, `0x8B96B73E`, `0x4988BC6A` enabled; the top tool popped;
  funds and city name updated.
- `ObliterateCity`: the scripted dialog `0x2A41436C` (`0x27DF05BE`); on `0x6A41BBB5` the MySim button is disabled,
  panel 4 hidden if current, and an obliterate tool (`cSC4ObliterateCity`) set. `DoCityObliterated` (`0x26D31EC5`):
  the MySim button disabled, panel 4 hidden if current, the God tab untoggled.
- `DoBudgetFailure` (`0x007E6E00`): modal-dialogs.md §4.1.

## 6i. Buttons, menus and catalog popups *[compat]*

*Added 2026-10-21 (re-view3dwin).*

**`HandleButtonActivated`** (`0x007F4690`). Buttons that execute a command (command server slot 20, id, no
parameters): `0x6A4FBB31`/`0x4A4FBB60`/`0x8998BBDF` speeds 0/1/2, `0xC998BB81` pause, `0x4992046A`/`0xE9920494` zoom
in/out, `0x2A4FBB08`/`0x8A4FBAEA` rotate CW/CCW, `0x99887766` query, `0x8B96B73E` traffic query, `0xE999C820` demolish,
`0xE99237B4` bus stop, `0x2992F72C` pipes, `0x0B954123` sign, `0xCB95412C` label, `0xEB9541EB` toggle signs,
`0xABA7ED0A` sign demolish, `0xAA5543A1` obliterate, `0xCA58EBB7`/`0x098F4F6C`/`0xA98F4F88` play/graphic/audio options,
`0x44` toggle UI.

Menu buttons: zoning (`0x30`, `0x29920899`, `0xA998AF42`, `0xC998AF00`, `0x0991EE13`), landscape (below), utilities
(`0x32`, `0x35`, `0x39`, `0x40`), transport (`0x29`, `0x31`, `0x299237BF`, `0x2B730C5B`, `0x2B79E398`, `0x6999BF56`,
`0xA99234A6`, `0xE99234B3`, `0xEA6E379D`), vehicles (`0xABB27A7A`, `0xABB27B91`, `0xEBF9E342`, `0xEBFAC717`–`719`,
`0xEBFAC720`), disasters (`1`, `0x33`, `0x0992FD36`, `0x09DE87AE`, `0x4BFC8C2F`, `0x69A30679`, `0x6BFC8C55`,
`0xE9DE87A7`), day/night (`0xCA35CB74/76/78`), mayor sub-panels (`0x15200002`, `0x15200003`, `0x49EDF9B7`,
`0x99887755`, `0xABC54125`), the RCI graphs (`0x09D27EB0`, `0x29D27EC0`, `0x49D27ED0`, `0xAA9211B3`: show), the
mission panel (`0x4BB92C1F`–`21`), and these secondary menus: `0x6991EE42` (`0x899302FC`/`0x0992FD17`), `0x0991EE39`
(`0xC9930681`/`0x699306ED`), `0xA994824D` (`0xE99237FF`/`0xC99237A0`), `0xA9ED5617` (`0xAA356502`/`0xCA35CBED`),
`0xE991EE2F` (`0x4992F764`/`0xE992F711`); `0x69B9324A` opens the disaster catalog (catalog view `0x29B6C670`, kind
`0x4A2B2464`); `0xAB9537B7` the tertiary menu `0xCB95403E`/`0xAB954023` (margin −75).

**Building menus**: buttons `3` parks (group `0x1006`), `0x34` rewards (`0x150B`), `0x37` police and jail (`0x1500`,
`0x1501`), `0x38` fire (`0x1502`), `0x42` education (`0x1503`–`0x1506`), `0x09930709` landmarks (`0x150A`),
`0x89DD5405` health (`0x1507`): the same kind toggles the menu closed; otherwise a catalog view of the buildings of
those occupant groups (`CreateBuildingMenu`, catalog `0xC8DBCCBA`; civic-buildings.md for which are enabled) is
opened as a tertiary menu with background `0x14215ED4` and the menu's remembered state.

**Menus.**
- **Zoning** (`0x007EC740`): `0x30` the dezone tool; `0x0991EE13` the secondary menu `0xE9949936`/`0x69923479`;
  `0x29920899`, `0xA998AF42`, `0xC998AF00` zone catalogs (groups `0x8A3F1505`, `0x8A3F14DC`, `0x8A3F1502`, kind
  `0x4A2B2466`) remembered at `+0x20C`, `+0x214`, `+0x21C` (defaults 1, 4, 7: low residential, low commercial,
  agriculture). `InvokeZoningTool(z)` needs an established city; zone kinds above 15 are refused.
- **Landscape** (`0x007ED720`): flora catalog `0x4A22EA06`; landscape catalogs `0x29923215`, `0x152231F3`,
  `0xC99231DA`, `0x6A3D638C` (kind `0x4A2B2462`); raise / lower everything `0x0AA44504` / `0x0AA44505` (terrain
  `ChangeTerrainLevel(1 / 0, default amount)`); `0x0AA44502` / `0x0AA44503` post `0x470BD214` with data1 0 / 1 (VW7);
  brush tools `0x0A32DDD2` and `0x899231E6` (the terrain's brush 6; mode = which one), `0xEA32DDE5` (`0x4A524342`),
  `0xEA32DDBB` (`0xEA33E2FD`), `0x4A32DDC7` (`0x0A33E37B`); `0x4A551A6B` reconciles the edges with the neighbours
  (terrain slot 89); secondary menus `0x36`/`0x8A32DDDB`, `0x49E95D2B`, `0x8991EE08`. The global terrain sound plays
  first for the raise/lower/`0x470BD214` items.
- **Global terrain tool** (`RunGlobalTerrainTool(n)`, catalog kind `0x6A5AC478`): plays the terrain sound, then 0
  change terrain type, 1 create flat areas, 2 create shore cliffs, 3 / 4 raise / lower everything, 5 / 6 post
  `0x470BD214` with 0 / 1, 7 the levelling tool.
- **Utilities**: `0x32` the power-line tool; `0x35` power plants (groups `0x1400`, `0x1402`, plus `0x6A650FFE` when
  `0x00B43D90` slot 8 accepts `0xE0D0722E`; network list `0x4003`); `0x39` water (`0x1401`, `0x4004`); `0x40` landfill
  (`0x1402`, `0x1401`, zone `0x2A49D7CA`); background `0x14215ED3`.
- **Transport** (catalog `0x07BDDF1C`, background `0x14215ED2`): `0x29` rail (`0x1300`, networks `0x4001`), `0x31`
  highways (`0x4005`), `0x299237BF` subway (`0x130A`, `0x4002`), `0x6999BF56` and `0x2B730C5B` roads (`0x4000`),
  `0xA99234A6` (`0x1519`), `0xE99234B3` airports (`0x1508`).
- **Vehicles**: `0xEBFAC717`/`718`/`719` the earned-vehicle catalogs of the three automata lists (`0x00B43D44`,
  `0x00B43D4C`, `0x00B43D50`; items from `AddAutomataToItemList`, kind `0x0BB2E43E`); `0xABB27A7A` the secondary menu
  `0x6BB27447`/`0x8BB27C12` with button `0xEBFAC720` showing the city situations' CSI visibility, which that button
  sets (situation manager slots 29/30); `0xEBF9E342` asks `0x00B43D34` slot 7 (`0x1E`) *[inferred]*.
- **Disasters** (`DoDisasterCopingMenu`): `1` the tertiary menu `0x09DE3002`/`0x09DE8798` (margin −25); `0x0992FD36`
  the disaster layer's slot 11 (go to the next disaster); `0x33`, `0x69A30679` the dispatch tool (0 / 1);
  `0x4BFC8C2F`, `0x6BFC8C55` dispatch 6 / 5.
- **Day/night** (`DoDayNightMenu`): day/night notifications on; `0xCA35CB74/76/78` visual time mode 0/1/2; the
  preference byte `+0` := the mode; `UpdateDayNightButtons` toggles exactly the button of the current mode.

**Catalog popups** (`0x007EAEB0`, the six-argument `InvokeTertiaryMenu` of ui-windows.md §4.4). The building, catalog
and vehicle menus share one tertiary popup: the same kind closes it; otherwise the item grid gets 44 × 44 cells with
spacing 5 and `clamp(items, 1, 8)` rows (6 when the view is under 601 pixels high), inside a callout
(`0xA85F1A83`/`0xE9C86C9D`) and a `cSC4WinTertiaryMenuFrame` (id `0x8A6E61E0`) with the menu's background picture. The
remembered state (scroll position, selected item) is restored; when a tutorial is not running and an item was
selected, the item's tool is **invoked again** unless its view level does not match (subway needs level 2, pipes level
3, everything else level 1) *[compat]*. The disaster catalog (button `0x69B9324A`) uses the same popup one level up
(`0x007E7270`: the secondary menu fields, 1–6 rows). The constructor's defaults: transport `0x8C329937`, `0x31`,
`0x29`, `0x299237BF` at `+0x278`…`+0x290`; landscape `0x6A33E1BA`, `0x2A33E1A3`, `0xEA353064`, `0x2A312F97`,
`0x0A5242DF` at `+0x2A8`…`+0x2C8`.

`InvokeCatalogItemCommand(kind, item, close)` (`0x007E9FA0`), also reached by window command `0x49D993D4` (§6):

| Kind | Action |
|---|---|
| `0x4A2B2467` network | established; the network tool for the item (as §6g; road `0x8C329937`, rail `0x29`, highway `0x31`, street `0xEA6E379D`, pipe `0x2992F72C`, power `0x32`, avenue `0x2B730C5B`, subway `0x299237BF`, light rail `0x2B79E398`, `0x4BE099DE` 9, `0x0BE099C1` 10, `0x2BE099A9` 11, `0x0BE0998F` 12); items `0x6A47xxxx` the intersection tool with `xxxx` |
| `0x4A2B2466` zone | established, item < 16: `InvokeZoningTool` |
| `0x6A2B245F` lot | established: the lot tool for the item; when the mouse is over the view it gets one move |
| `0x4A2B2464` disaster | `InvokeDisaster` (below) |
| `0x4A2B2462` / `0x8A313C52` | the terrain-scrubber tool (item, mode 1 / 0) |
| `0xEA5AD08D` | the flora tool |
| `0x6A5AC478` | `RunGlobalTerrainTool(item)` |
| `0x0BB2E43E` | the automata group whose item matches, started through the city situations (slot 23) |

`close` closes the tertiary menu first (the secondary one for `0x6A5AC478`).

**`InvokeDisaster(id)`** (`0x007E9DB0`): `0xA9DE8784` is the torch tool. Otherwise the disaster exemplar
(`0x6534284A`, `0x29B6C670`, id) gives an effect name (`0x4A08C700`) or an effect group (`0x4A08C702`), with a disaster
manager (`0x4A08C701`): an effect-disaster tool is set with them; with neither property nothing happens
(disasters.md).

## 6j. Status displays *[compat]*

*Added 2026-10-21 (re-view3dwin).*

- **Date** (`+0x310`, `0x007EA6E0`): the simulator's date with its year replaced by the years since the start date,
  wrapped into 1…10000 above 10000 *[summary: the formatting calls]*.
- **Funds** (`+0x314`): the budget's total funds as money text, dark red (`0x77`, 0, 0) when negative, else (`0x44`,
  `0x55`, `0x66`); a red cost indicator is re-evaluated; the budget window refreshed when visible.
- **Population** (`+0x318`): `0x00B43DA4` slot 16 as a number. **City name**: the city's name into the panel's window 2.
- `UpdateSpeedButtons` (`0x007E59B0`): buttons `0x6A4FBB31`, `0x4A4FBB60`, `0x8998BBDF` toggled for the speed (0, 1, 2);
  each shows the lower half of its picture while the simulator is paused.
- `Update24HourClock(hour, minute)` (`0x007EC0B0`): an hour above 23 is read from the clock; label `0x29EE06D8` gets the
  time text; the clock picture `0x6A64E353` shows frame `2·hour + (minute > 29)` of the strip (`0x46A006B0`,
  `0x13D14CA3`), frames 37 pixels wide; then the hover text. Hover (`0x007EA810`): while button `0xAA75CA06` has state
  `0x2000`, window `0xAA771FD7` is shown with caption `0xAA771FC7` = the time text; else it is hidden.
- `UpdateAmbientLightLevel`: the lighting manager (renderer slot 9) gets `hour + minute / 60` of the **visual** clock;
  a true result refreshes the renderer.
- `UpdateMayorRatingBar` (`0x007E8510`): `r` = the aura simulator's mayor rating (city slot 51, slot 8, a signed byte),
  `i = clamp((clamp(r + 100, 0, 200) · W) / 200, 0, W − 1)` with W the width of the gradient picture (`0x46A006B0`,
  `0x14015549`); the bar shows column i *[summary]*; the arrow (`0xCA5A415E` down, `0x6A5A4156` up, children of
  `0xE9889775`) moves `7 · d` pixels from its home with `d = clamp(i − previous, −3, 3)`; previous := i.
- `UpdateAlertBorder`: the border `0x6A5E44B6` shows picture `0x14315E60` during a disaster (disaster layer slot 10),
  else `0x14315E62` during a city situation (slot 6), else `0x14315E61` while paused, else none.
- **Cursor text** — `SetCursorText(id, priority, text1, text2, flag)` (slot 40): empty texts clear entry `id`;
  otherwise the entry is replaced or inserted by priority. `ClearCursorText(id)` (slot 41) erases it. The callout shows
  the first entry, following the cursor (slot 50 projects a 3D point first) *[summary]*.
- **Cost indicator** — `SetCostIndicator(amount, cost)` (slot 42): red when `cost` exceeds the funds; mode 1, alpha 255
  red or 150, white text; redrawn when anything changed. Slot 43 is `(amount, amount)`. `ClearCostIndicator(fade)`
  (slot 44): with `fade`, mode 2 (white) frozen at the last cursor position for 1500 ms, its alpha falling with the
  time left; else hidden. Colours: normal (255, 255, 40), red (255, 40, 40), fading (255, 255, 255), with a black
  outline. It follows the cursor: its right edge 15 pixels left of it, its top 15 pixels above it.
- **Mission status panel** — `ShowMissionStatusPanel(id)` (slot 56) creates the scripted panel `id` at the view's
  right edge (8 pixels in) when none is shown; `HideMissionStatusPanel` (57) deletes it; `UpdateMissionStatusPanel`
  (58) writes the time left of the active situation as `MM:SS` (red when under 60 seconds and above 20, or on odd
  seconds below that); the progress text comes from the situation's target or LTEXT `0x6A231EA4`/`0xAC2BD618`.

## 6k. The save record's body and the global effect triggers *[compat]*

**Status ordering and fade details (VW13, §6j supplement).** Priorities are unsigned, greatest first;
a new ID is inserted **before** entries of equal priority (`0x007F5D61`–`0x007F5D6A`). Updating an existing
ID changes its strings/style in place, retaining both its old priority and position. The first entry drives
the callout. `ClearCostIndicator(true)` unconditionally selects fading mode and white text, copies the last
cursor position and resets the remaining time to binary32 1500 ms (`0x007EC900`), even when already hidden.
It redraws without changing the existing surface alpha or explicitly showing a hidden BMP. On each due paint,
remaining time loses binary64 `seconds * 1000`, is stored as binary32, and the still-live x87 value determines
expiration and alpha (`0x007E8F62`–`0x007E8FE5`). At remaining time <= 0 the BMP is hidden and mode becomes 0.
Otherwise alpha is truncation toward zero of remaining time times **binary32 0.17** (`0x00ABCB58`), giving
255 at the start, 127 at 750 ms and 0 shortly before expiration. This does not preserve the active indicator's
150 alpha; a normal indicator can brighten on its first fading paint. Its frozen Y position moves upward by
truncation toward zero of `(1500 - remaining) * binary32 0.13333334` (`0x00ABCB5C`), subtracted from the
saved Y before the final integer conversion; X remains fixed. At half-time it is about 100 pixels higher.
`ClearCostIndicator(false)` immediately hides and resets the remaining time. The simulation date formatter
subtracts the configured start year; only differences **above 10000** wrap to the range 1–10000
(`0x007EA778`–`0x007EA7B0`). Zero and negative differences are passed to the locale formatter unchanged;
they are not clamped to year 1.

*Added 2026-10-21 (re-view3dwin).* The View3D record (ui-windows.md §3.3a; `0xA9C8647C`, `0xE9C8647E`, instance 1)
is version **4**: references to the camera control, the renderer (its `+8` sub-object), the model maker (`+4`) and
the simulator, then the trigger count and the (trigger, count) pairs in bucket order. `DoLoad` accepts versions below
5, reads the four references by interface (`0xC9C628EC`, `0xE9C622D8`, `0xA9C741B3`, `0x8695664E`) and the pairs, and
clears `+0x408`. (This answers save-records.md Q4 for the View3D record.)

- `AssertGlobalEffectTrigger(t)` (slot 47): count + 1, or a new entry with 1 and **send** `0xAA375FE8` (data1 = t,
  data2 = 1). `RetractGlobalEffectTrigger(t)` (48): count − 1 while above 1; else erase and send (t, 0).
  `GetGlobalEffectTriggerState(t)` (46): present. **Corrects** prop-developer.md §4.10: the message is sent, not
  posted.

## 6l. Helpers reached only from the city view *[compat]*

### Mission, advice links and small interface contracts (VW18)

**Mission notifications.** There is no conversion from data1 to a mission identity. Message slot 9
(`+0x24`) returns data1; slot 32 (`+0x80`) is the **zero-argument GetIGZUnknown**, not an indexed lookup
(foundations.md, message-slot addendum). For `0xABBBCA64`, the owner passes the borrowed attachment as
argument 1 and data1 unchanged as argument 2 (`0x007F57C5`–`0x007F57D9`, `0x007F58B9`). For
`0xAC083D7D` the arguments are the same attachment and literal 1 (`0x007F58AA`–`0x007F58BA`). The
attachment getter is a simple borrowed return (`0x009D858F`, message table `0x00ACEC00` slot 32);
the owner neither AddRefs nor Releases it for this call. `UpdateMissionProgress` requires an existing
mission panel and situation manager, otherwise it does nothing. With an attachment and a nonzero flag,
it gets the active situation from the manager and uses its target text/progress; it does not dereference
the attachment to obtain those strings. A nonnull attachment with flag zero leaves those captions alone.
A null attachment selects the localized fallback progress caption (§6j). This includes joyride activation,
which has data1 = 1 but no advice attachment. The attachment thus gates the update, rather than naming
the source of the displayed progress (`0x007EBB20`–`0x007EBCAD`).

**Advice-link payload.** `HandleAdviceLinkMessage` takes the window-message record: its first word is
the message type and its second word is the link selector (window-message data1). Data2/data3 do not
select the destination. Before acting it requires the resource manager `+0x18C` and resource
`0xC988BC79`/`0x8810`; if either is missing it does nothing. A successful resource is activated with
`(1,1)` and released afterwards. The enclosing window handler still falls through to base `DoMessage`.

| Type | Link selector and result |
|---|---|
| `0x6A5BDFCD` | Select mayor subpanel `0x99887755`; send two button-command messages to `+0x300`: button `0x4A32CA92`, then button equal to the selector. Both use type 3, data1 `0x287259F6`, data3 0. |
| `0x6A720993` | Select mayor subpanel `0x15200002`; call the graphs procedure's `ShowGraphType(selector)` when present. |
| `0xEA5BDFBC`, selector `0x4A5BDFB8` | Select budget (`+0x370 = 0x41`); dispatch button `0xAA3AC40E` to the budget procedure's window. |
| `0xEA5BDFBC`, selector `0xCA5BDFB0` | Select budget; dispatch button `0xAA3AC40F` to that window. |
| `0xEA5BDFBC`, any other selector | Forward it as the button ID to the budget window, with no panel-selection change. |

The budget dispatch is through the city window's child-message dispatcher (window slot 147), with the
same type-3/button envelope and zero caller data. A null budget window skips dispatch, but the budget
procedure is required to retrieve it. For each recognized destination, if `+0x374 == 0`, hide the active
city dialog, hide the other visible mayor panes, and show the selected pane: budget, graphs, or `+0x300`.
Graphs/building-select visibility tests guard their hides; optional absent panes are skipped. If `+0x374`
is nonzero, skip that hide/show group. In both cases record the selected subpanel in `+0x370`, clear
`+0x374`, and perform the destination-specific dispatch above. This is selection, not a toggle of the
same link (`0x007E7830`–`0x007E7C78`).

**Borrowed getters and debug slot.** `GetRenderer` (`0x0043FD00`) and `GetModelMaker` (`0x007F0FE0`)
return their stored pointer without AddRef. The keyboard-accelerator getter, view-interface slot 31,
uses the inherited `0x0099B951` body and likewise returns its stored pointer without AddRef. Null stays
null; callers must acquire their own reference if retaining any of these across the owner's lifetime.
View-interface slot 38, named `DisplayTerrainPickDebugString`, is the folded empty method `0x006F7DC0`:
it consumes two arguments and has **no observable effect**. It reads neither argument and promises no
return value. There is no debug string, terrain query, caption update or renderer action to implement
for that slot.

**Background mode on zoom refresh.** The zoom notification uses the stored `+0x158` mode, not a host
supplied mode and not a fresh preference on every zoom. Mode 0 tries 2D first and latches 1 on success;
only if no 2D picture was accepted does it try 3D and latch 2. Mode 1 attempts only 2D; mode 2 attempts
only 3D. A failed reload preserves the mode and does not call the renderer to clear its old background.
The renderer receives a boolean **mode == 2** with an accepted picture, then the loader's picture
reference is released (`0x007F17A8`, `0x007F190F`–`0x007F193A`, `0x007F1B70`–`0x007F1BAE`).
The 2D basename is `Background2D`; the 3D basename is `Background3D` followed by the single byte
`'0' + camera zoom` (the integer at camera `+0x108`, `0x007F192E`–`0x007F193A`, `0x007F19BF`). Thus
zoom 3 in 3D mode reloads `Background3D3` with a supported image suffix; it does not switch to 2D.

### Coverage helper and auxiliary ownership (VW18)

The embedded helper at `+0x3E4` starts with category mask zero and an empty occupant-keyed hash table
(`0x007D26F0`). It requests 100 buckets (the shared prime policy supplies 193). `RecalcCoverage`
(`0x007D26E0` jump to `0x007D20E0`) takes **no explicit arguments**: no cursor position, viewport bounds,
radius or visibility flag. It rebuilds from the stored category mask and whole service-occupant lists.
This owner invokes it at Init; query/place-lot controls invoke it when their coverage needs change.
It is not an extra per-paint owner operation.

| Mask bit | Occupants queried | Radius query and effect name |
|---|---|---|
| 1 | police simulator's list (slot 21) | slot 20, factor binary32 0.5; `PlopMode_Police_Existing` |
| 2 | fire simulator's list (slot 22) | slot 21, factor binary32 0.5; `PlopMode_Fire_Existing` |
| 4 | health/education simulator's health list (slot 49, second argument 0) | slot 45, factor binary32 `0x3F333333` (about 0.7); `PlopMode_Health_Existing` |
| 8 | its education list (slot 47, second argument 0) | slot 45, same factor; `PlopMode_Education_Existing` |

Each radius query receives the occupant, two scratch outputs and a float radius output. On success use
that radius; on failure use binary32 **32.0**, not the factor. Missing simulators skip their categories.
Factors/fallback are instruction-verified at `0x007D21D1`–`0x007D21FF`, `0x007D2310`–`0x007D233E`,
`0x007D2448`–`0x007D2479` and `0x007D2583`–`0x007D25B4`. Returned occupant references are released after
processing. `0x3AE4BEA3`/`0x3AE4BEAB` are the framework class/interface request for the temporary list
allocator (`0x007D224A`–`0x007D227A`), **not two coverage-effect IDs**. Effect creation uses the names above.

Recalculation first clears every cached entry's seen flag. For each queried occupant, obtain its spatial
object from the occupant manager (slot 14); a missing spatial object skips it and leaves it unseen. Read
the spatial object's position (slot 15) and look up the cache by occupant pointer. A new entry is made
only after effect creation succeeds. Its transform has identity orientation, the spatial position and
uniform scale **2 × radius**, computed from binary32 radius and stored as binary32. Translation is marked
present; scaling is marked present unless the doubled radius equals 1. Start the effect with argument 0,
retain its reference in the entry, cache the radius and mark seen. There is no viewport/occupant-visibility
test, spatial range intersection or spatial-set insertion/removal in this path (`0x007D1E40`–`0x007D20C7`).

An existing entry is marked seen, but receives a new transform only when its requested radius differs
from the **original cached radius** and its effect pointer is nonnull. The comparison does not refresh
the cached radius. Equal radius therefore does not even update a moved occupant's position. After the
service lists, unseen effects are stopped with argument 0 only if their running test succeeds, and their
references are released and nulled. **Their hash entries remain.** A later sighting of such an entry does
not recreate its null effect. These are literal cache quirks, not a requirement to repair the cache
(`0x007D1EC9`–`0x007D1F84`, `0x007D2666`–`0x007D2690`).

`SetCategories(mask)` stores the entire mask, stops running effects and clears **all** cached entries,
even if the mask is unchanged; it does not itself recalculate (`0x007D1D10`). `Clear` (`0x007D1DB0`) also
stops running effects and clears/releases the cache, preserving the category mask. Destruction releases
cache entries/bucket storage; the owner's Shutdown explicitly clears first. Category classification
(`0x007D1AB0`) ORs bit 1 for a police occupant, bit 2 for a fire occupant, bit 4 when property
`0x091B42A2` can be read and bit 8 when `0x691B42B3` can be read; the property values need not be nonzero.

The remaining constructor list below is a dependency inventory, not extra per-frame work for this owner.
Demolish is created on its tool action and follows the input-control push/pop lifetime (§6e/§6g).
Budget, building-select, graphs and panel procedures attach to their scripted windows in AddGUI and
detach/release on their specified GUI removal paths (§6b); visible tests do not acquire references.
The advisor tracker starts empty/inactive (`0x00762E70`) and a panel procedure starts unattached
(`0x00774550`). The establish key filter belongs to its dialog (Tab/Enter forwarding), and progress-screen
paint/progress callbacks belong to the progress window, which receives forwarded progress messages only
while present (§5). Window-set helpers operate on existing children, not newly owned view objects:
`0x0078D720` finds the deepest eligible child containing the supplied point by repeated child enumeration
(`0x0078D670`: visible, lacks flag `0x200000`, contains point). No match returns null; the result is borrowed.

One extra AddGUI setup action in that inventory is concrete: `0x0079EA40` requests graph resource
`0x4203`/`0xCA318385` from its control's manager. If found, it sends slot 8 `(-1,0)`, slot 6, then slot 7
for series 1–9 with packed colours `0x00FF00`, `0x44CC22`, `0x1E911E`, `0x5B81D0`, `0x3D6EB8`,
`0x3C42AD`, `0xD0D030`, `0xC6B733`, `0xBC9F37`, and releases the resource; absence skips setup.
The stop helper `0x0047A780` acts only when its object's active byte `+8` is set: call slot 4, call slot 5(0)
only on success, clear that byte, return true. It is used for the dormant shutdown objects (§6m), not a
new required allocation or a call on every helper in the inventory.

**VW12: trigger representation and SizeOf parsing (§§3.1, 6k supplement).** Trigger keys and counts are
unsigned 32-bit words. Increment wraps modulo 2³² without sending a new assertion; a present zero-count key
still reports present and retracting it erases it and sends the off message. Retracting an absent key does
nothing (`0x007ECCE0`, `0x007EB8A0`). Save traversal (`0x007EA64A`–`0x007EA6A9`) starts at the first nonempty
bucket, visits each linked node from head to tail, then searches higher bucket indices. Bucket index is the
unsigned key modulo the **current bucket count**, not sorted key or insertion order. The constructor requests
100 buckets; the prime table at `0x00A829B0` rounds that up to **193** (53,97,193,389,…).
The serialized pair count is the number of present keys, even when a wrapped count is zero.
New keys prepend to their bucket, so inserting 1 then 194 into the initial table saves 194 before 1.
Before inserting a new key, grow if the resulting number of keys exceeds the bucket count: choose the
first table prime at least that number (the 194th distinct key grows 193 to 389). Rehash scans old buckets
in increasing order and their chains head-to-tail, prepending each node to its new bucket; insert the new
key at its bucket head afterwards (`0x00750C35`–`0x00750C89`, `0x004F0955`–`0x004F0A0F`). Thus growth
can reverse colliding-node order; saving does not normalize it. Count changes on existing keys do not grow.
`RZAtofA` (`0x0090FC5B`) delegates to the bundled CRT `strtod`, discarding the end pointer. It uses the runtime
locale's decimal-point byte (`0x00B19C60`), rather than requiring a literal dot. Its numeric recognizer
`0x00A0342B` accepts decimal digits, sign, decimal separator and decimal exponent (including D/d); it has
no hexadecimal-float, NaN or Inf recognition. Thus in the C locale `0x1p4`, `nan` and `inf` parse as zero
and SizeOf supplies its lower clamp 0.9; `5junk` supplies 5.0. With comma as the locale decimal separator,
`1,5` supplies 1.5 while `1.5` supplies 1.0. Do not use the host's modern hex/NaN/Inf grammar.

**VW17: remaining menu identities (§6i supplement).** Script-menu arguments below are script instance,
menu kind, alignment target (the originating window) and **button ID as caller data**. They are not caller
data zero. All use the existing script group `0x96A006B0`; script-menu backgrounds come from the script,
not a synthesized catalog background.

| Button | Literal action |
|---|---|
| `0x36`, `0x8A32DDDB` | secondary script `0xAAA44448`, kind `0xCA35CBED`; caller data is the respective button ID |
| `0x49E95D2B` | secondary script `0xE9923283`, kind `0x49923239` |
| `0x8991EE08` | secondary script `0x09923283`, kind `0x49923239` |
| `0x2B79E398`, `0xEA6E379D` | route to transport: catalog `0x07BDDF1C`, background `0x14215ED2`, network filter 0, empty group sets and **null remembered-state pointer** |
| `0xABB27B91` | routes to vehicle handler but matches none of its actions: no menu/tool effect |
| `0x09DE87AE`, `0xE9DE87A7` | route to disaster handler but match none of its actions: no menu/tool effect |

The last three still reach HandleButtonActivated's common tutorial follow-up. The transport cases still use
their button ID as tertiary kind; invoking that same kind closes it. These are intentional literal quirks,
not missing resource identities (`0x007F4690`, `0x007F3C10`, `0x007EF610`, `0x007E7130`). Utilities use catalog
`0xC8DBCCBA`; zoning uses `0x2A3858E4`. Landscape/flora catalog identity is `0x6A01FC2A`; landscape background
`0x14215ED0`, zoning `0x14215ED1`, utilities `0x14215ED3`. Flora uses the same landscape background.
Landscape remembered pairs are:

| Pair base | Button | Group | Initial item |
|---|---|---|---|
| `+0x2A0` | `0xC99231DA` | `0x6A33D682` | 0 |
| `+0x2A8` | `0x152231F3` | `0xEA33D694` | `0x6A33E1BA` |
| `+0x2B0` | `0x6A3D638C` | `0x8A3F131B` | `0x2A33E1A3` |
| `+0x2B8` | `0x29923215` | `0x8A3143B7` | `0xEA353064` |
| `+0x2C0` | no owner menu consumes it | — | `0x2A312F97` |
| `+0x2C8` | flora `0x4A22EA06` | `0x8A2602BB` | `0x0A5242DF` |

The second word of each pair starts at zero; helpers receive a pointer to that second word. Flora kind is
`0xEA5AD08D`; the other landscape kind is `0x4A2B2462`. Do not invent a fifth live landscape catalog for the
unused `+0x2C0` pair. Bus-stop/pipe IDs are in the full registry (§6g supplement).

*Added 2026-10-21 (re-view3dwin).*

- The coverage helper at `+0x3E4` (`0x007D26F0` constructor; `0x007D1DB0` clear; `0x007D1CF0`
  destructor; recalculation `0x007D26E0`) shows the named coverage-radius effects described above.
- `AddBackgroundImageToRenderer` (`0x007F1640`): an accepted user background image, loaded through `+0x150`,
  becomes the renderer's background; dimensions must be equal with `(width & (width - 1)) == 0`.
  Mode and basename selection are specified above; `+0x158` keeps the selected mode.
- Window `0xEA8CAD14`'s procedure (`0x00769E90`…`0x00769F10`) subscribes to `0x26D31EC1` and `0xA6B79602` while
  attached (`0xA2BF8AD5`/`0xA2BF8AD6`) and toggles the child buttons after it to show the view's rotation.
- Constructors and small members of other UI classes that only the view calls: the demolish tool (`0x004B9070`), the
  advisor popup tracker (`0x00762E70`), the budget window procedure's teardown (`0x00764990`, `0x00764B70`), the
  building-select procedure (`0x00769750`; visible test `0x007696C0`), the establish dialog's key filter
  (`0x0076A390`–`0x0076A4E0`: Tab and Enter forwarded), the graphs procedure (`0x0076CFE0`, `0x0076D080`; visible byte
  `0x0076B6A0`), the panel procedure (`0x00774550`), the progress screen's paint and message (`0x00776C30`,
  `0x007773B0`, `0x00777640`), window-set helpers (`0x0078D670`–`0x0078D720`, interface `0x22BA0121`), a graph colour
  set-up (`0x0079EA40`) and `0x0047A780` (stop: slot 4, then slot 5(0) when it returned true).

## 6m. Quirks

### Dormant fields and panel extras (VW6, VW8)

`+0x2EC` and `+0x2F0` have **no creator in this owner**: constructor zeroes them
(`0x007F0FA2`/`0x007F0FA8`), `RemoveGUI` conditionally shuts down/releases the former and releases the latter,
then zeroes both (`0x007EF348`–`0x007EF377`). No indirect out-argument in `AddGUI` supplies either field.
Normal creation leaves them null; replacements need not invent two more GUI objects. The same is true of
absolute `+0x184`, `+0x188` and `+0x410` (constructor stores `0x007F0B92`, `0x007F0B98`, `0x007F0EE8`).
The first two are dormant stoppable helpers: shutdown, if populated, asks slot 4 whether running and calls
slot 5(false) only when true (`0x0047A780`), then releases and clears them. `+0x410` is a dormant owned reference
released during destruction (`0x007F119D`); it has no creation or active owner action. These are absence-of-use
contracts, not unidentified mandatory services. Apparent accesses at `+0x184` in the message filter are relative
to its `this+8` interface and refer to the main panel, not the absolute dormant field.

The five-panel loop has two extra child windows (`0x007EE39B`–`0x007EE610`): mayor panel index 1 adds
`0xE9889775`; MySim panel index 2 adds `0xABB26B0E`. A successful recursive `cIGZWinGen` lookup supplies its
window interface, adds an owned entry to the GUI destruction list and a borrowed entry to that panel's alignment
set, installs the common panel procedure and city message filter, enables window flag `0x800`, sends it to back
(window slot 24, `+0x60`), disables `0x800`, then recursively clears focus acceptance. This transient flag is
not a persistent panel setting. Only panel index 2 creates/initializes the MySim winproc and installs it on the
panel itself; other panels retain the common procedure. All newly created panel children are aligned as a set
to the city view and the panel is hidden. Failed extra-child lookup skips that extra only.

### Required and optional creation paths (VW14)

`DoPreCityInit` is void; it has no failure return or rollback transaction. Existing camera/control/renderer/
animation/model-maker references are reused. When absent, each tries GZCOM, and **factory failure triggers a
local concrete constructor fallback**, with AddRef after storing the result (`0x007F41B0`–`0x007F4690`).
Camera-control viewport/Init and model-maker/animation Init results do not abort the sequence. Local allocation
failure for these required objects, or the realistic filter, is followed by an unchecked dereference: it is a
fatal precondition failure, not a successful initialization with null dependencies. The graphic-system QI for
`0xC47B747C` gates the renderer configuration/Init block as a whole; failed QI skips that block, not AddGUI.
The background-loader factory is optional; progress-screen updates are guarded. `Init` likewise requires its
command registry service (it dereferences the acquisition result), rather than returning a clean failure on a
missing service; optional accelerator/resource acquisitions follow their explicit guards in §3.

`AddGUI` also has no Boolean success/rollback contract. A failed script creation skips that script window's
immediate setup and continues; main-panel `cIGZWinGen` QI and the five panels' QIs are assumed successful once
their script window exists (their results are dereferenced without testing). A missing required main panel
can still fail at later unconditional child lookups. The common panel proc, news ticker, MySim proc, budget
proc, callout, BMP and popup tracker are allocated as required helpers: null allocation is not a supported
null-safe skip. Graph/building proc setup is conditional on the corresponding window; the optional background
image, buffer creation, extra child lookups, full-size overlay factory and rotation-widget lookup have explicit
success guards. No synthetic false result, balanced rollback or deferred retry is implied for an unchecked
failure. A host may report a fatal initialization error instead of reproducing an access violation.

The full-size overlay `0x6A5E44B6` is **parent-owned** after ChildAdd; its temporary creation reference is
released and no owner handle is retained. `RemoveGUI` does **not** explicitly delete it. It can remain until
the city window's base teardown removes remaining children. Thus the old “no AddGUI window alive after
RemoveGUI” sentence applies to the explicitly tracked windows only; do not add a borrowed overlay handle or
an extra destroy call. RemoveGUI followed by base Shutdown leaves no surviving children.

### Terrain message meaning (VW7, §6i supplement)

`0x470BD214` is a **whole-map erosion request**, handled by `cSTETerrain::DoMessage`:
data1 0 runs the configured repeated water erosion followed by the terrain erosion pass; data1 1 runs just
the terrain erosion pass with the alternate tuning (`0x00747B20`, erosion worker `0x00746F40`). The message
handler uses its default tuning (second argument false), then refreshes the entire terrain rectangle
`(0,0,width,height)` with notification arguments `(1,1,rectangle,1)` (`0x00741660`, handler call sites in
terrain's `0x470BD214` branch). Other data1 values do no erosion but still refresh. The two landscape items
and global tool cases 5/6 **post** it with data1 0/1; it is neither an immediate city-window callback nor an
input-control install.

*Added 2026-10-21 (re-view3dwin)*, besides those tagged in §3–§6c:

1. `ProcessCommand` returns false even for the commands it handled (§6g).
2. Arrow-key scrolling starts only when no arrow is held; there is no diagonal keyboard scrolling, and any Stop
   command stops both axes (§6g).
3. Edge directions are cleared by every recomputation while edge scrolling is on (§6f).
4. `RotateCW` turns the camera left (slot 15) and `RotateCCW` right (§6g).
5. `SetZoomLimits` compares the zoom with the unclamped maximum (§6f).
6. The command parameters of the save and quit commands are read without a null test (§6g).

## 6n. The paint completion contract *[compat]*

**VW10.** `GZPaint` (`0x007F6200`–`0x007F6405`) always returns true. On a due enabled paint it updates
the cost indicator first, then scrolling/deceleration, temporary-pause expiration, draw-context time and renderer
Draw, in the order in [view3d.md](view3d.md) §5.1. Cursor-text positioning is driven by mouse/view changes,
not this tail; mission updates are driven by their notifications. The tail (`0x007F638C`–`0x007F63FE`) is a
**deferred-message queue**, not a child-window walk: if nonempty and modal dialogs are safe, take a temporary
reference to the first message, unlink that node and release its queue reference. Recheck modal-dialog enabled;
when enabled, repost this message through the message server (slot 4, message plus 0), otherwise discard it.
Release the temporary reference. This processes **at most one** message per paint, even when callbacks add more.
With an empty queue or unsafe modal state, do nothing. It runs even when painting is disabled or throttled.
There are no calls here to `0x005F98C0`, `0x007F5F00`, `0x007F6010`, child callbacks, or mission/cursor refreshes;
the former §5.1 description crossed the end of this function into neighboring functions.

## 7. Conformance tests

### Owner closure vectors (VW10–VW16)

- VW-T26: ordinary construction/AddGUI leaves dormant `+0x184`, `+0x188`, `+0x2EC`, `+0x2F0`, `+0x410`
  null. Mayor/MySim extras toggle flag 0x800 on, send to back, then off. A failed extra lookup skips only it.
- VW-T27: failed camera factory uses the concrete fallback and continues; failed graphic-system QI skips
  renderer wiring but reaches AddGUI. Required fallback allocation failure is fatal. RemoveGUI does not
  destroy overlay 0x6A5E44B6; subsequent base shutdown does.
- VW-T28: capture the Init registry: 104 records exactly in the table order, every third word zero,
  including unhandled records. Bus-stop/pipe buttons execute 0x6A935DC9/0x6A935DEE with zero parameters.
- VW-T29: post erosion data1 0 versus 1: only 0 runs repeated water erosion, both run the configured
  terrain erosion pass and whole-map refresh. Data1 2 refreshes without erosion.
- VW-T30: a present trigger count 0xFFFFFFFF asserted once becomes zero without another on message;
  retract then removes it and sends off. A second retract sends nothing. Distinct buckets save by increasing
  key modulo 193, not key order (e.g. keys 193 then 1 save in that order).
  Colliding new keys 1 then 194 save as 194 then 1; inserting the 194th distinct key grows to 389 buckets,
  rehashing by the rule in §6l before that new key is prepended.
- VW-T31: C-locale SizeOf `nan`, `inf`, `0x1p4`, `5junk` supplies 0.9, 0.9, 0.9, 5.0 respectively.
- VW-T32: buttons 0x36 and 0x8A32DDDB use the same script/kind but preserve their distinct caller-data IDs;
  0xABB27B91 has no vehicle action; 0x2B79E398 opens the generic transport catalog with null memory.
- VW-T33: activation message data1 7 with advice attachment A calls UpdateMissionProgress(A,7), with
  no indexed lookup or AddRef/Release of A. A null attachment with data1 1 uses the fallback caption;
  target-reached with data1 99 still calls UpdateMissionProgress(A,1).
- VW-T34: advice type 0x6A5BDFCD, data1 X, data2 Y sends buttons 0x4A32CA92 then X to +0x300.
  Type 0x6A720993 selects graph X. Budget selectors 0x4A5BDFB8/0xCA5BDFB0 send buttons
  0xAA3AC40E/0xAA3AC40F; another selector X is forwarded as X without changing panel selection.
  Missing advice resource causes none of these actions. With +0x374 nonzero, pane hide/show is skipped,
  but the selected ID is recorded, +0x374 is cleared and destination dispatch still occurs.
- VW-T35: repeated renderer/model-maker/accelerator getters preserve reference counts, including null
  results. Debug-string slot 38 with arbitrary argument words has no effects or argument dereferences.
- VW-T36: mode 0 and valid 2D/3D candidates selects 2D and latches mode 1; another zoom retries only
  2D. Mode 2 at zoom 3 requests Background3D3, passes renderer boolean true and keeps mode 2 on failure;
  failure issues no renderer clear. A fresh mode 0 with no accepted 2D and accepted 3D latches mode 2.
- VW-T37: mask 1 enumerates police only. Successful radius 12 creates PlopMode_Police_Existing at
  the spatial position with scale 24; a failed radius query uses 32 and scale 64. Missing spatial object
  creates nothing. Recalc has no explicit arguments and performs no viewport visibility test.
- VW-T38: create occupant A with radius 12; move A with radius still 12: no transform update. Radius 13
  updates to scale 26 on every recalc because the cached radius remains 12. Omit A: stop if running,
  release effect, retain key. Reintroduce A: no recreation until Clear or SetCategories removes that key.
  SetCategories with the unchanged mask still stops/releases and empties the cache, without recalculating.
- VW-T39: graph setup resource absent produces no series calls; present produces the nine colours
  above then one release. An inactive stop-helper object receives no virtual calls; an active one with
  slot 4 false receives no slot 5 call, clears active and returns true.

- VW-T19: with paint disabled and two deferred messages A, B, unsafe modal state drains neither. Safe/enabled
  paints drain and repost A with delivery argument 0, then B on the next paint; safe/disabled drains and drops
  one per paint. Each paint returns true. A not-due paint has the same tail behavior (§6n).
- VW-T20: period 1000 microseconds, elapsed 999/1000/1001: only 1001 restarts the timer and draws. Default
  period 0 draws even at elapsed 0 when painting is enabled; 900000 microseconds gives update duration 0.5 s.
- VW-T21: stored limits (4,2): `SetZoom(3)` requests renderer zoom 4; `SetZoom(4)` requests 2. Do not sort limits.
- VW-T22: insert A(priority 2), B(9), C(9): order C,B,A. Update A with priority 100: it stays last with priority 2.
- VW-T23: fade a red alpha-255 indicator, then update by 0.5 s: white alpha 170, remaining 1000 ms and Y about
  66 pixels upward; another 0.25 s gives alpha 127. At total 1.5 s it hides. Clearing a hidden indicator with
  fade still sets mode 2 and timer 1500, without a ShowWindow call.
- VW-T24: date year minus start year -1, 0, 10000, 10001: pass years -1, 0, 10000, 1 to the locale formatter.
- VW-T25: PreCityShutdown releases all queued messages, removes normal controls including current, then
  persistent controls; renderer, GUI, paint flag and notification registrations remain. Slot 143 with any
  two arguments returns false and produces no command callback.

- VW-T1: QI on the five IIDs of §1 returns the documented sub-object offsets; any other IID returns false.
- VW-T2: after `Shutdown`, no cheat ID of §3 is registered, and no view input control (normal or persistent) remains.
- VW-T3: `0xAA5CF1E3` while modal popups are unsafe is queued once; a second identical message does not add a copy.
- VW-T4: a link message whose URL opens returns false from the window `DoMessage` (§6 quirk).
- VW-T5: `SizeOf` with arguments `100`, `0.5`, `abc`, `5` hands the camera control 64.0, 0.9, 0.9, 5.0; `SizeOf` alone returns false and changes nothing (§3.1).
- VW-T6: `TastyZots` twice in a row restores the filter flag `+0xD`; each `TastyZots`/`Zoneria` message returns false; `TerrainQuery OFF` leaves `GetTerrainQueryEnabled` false, `TerrainQuery` alone makes it true (§3.1).
- VW-T7: with a stack control that takes key-down and a persistent control that does not, `ProcessKeyDown` returns true and the persistent control was still called once; with neither taking it, a persistent control returning true alone makes the result true (§6c).
- VW-T8: right press at (100, 100) on empty controls, move to (102, 103), release with mods 0: the camera jumps to the picked point; the same with a move to (104, 100) or with mods 1: no jump (§6c).
- VW-T9: wheel delta 0 with no control taking it calls `ZoomOut`; a stack control that takes the wheel suppresses zoom and the persistent controls are not called (§6c).
- VW-T10: with persistent controls P1, P2 and stacked tools T1, T2, a left press is offered to P2, P1, T2, T1 and stops at the first true; a key press is offered to T2, T1 (stopping), then to P2 **and** P1 always (§6c).
- VW-T11: `ScrollLeft` then `ScrollUp` scrolls left only; `ScrollUpStop` stops it (§6g).
- VW-T12: at zoom 3 with the default tuning the keyboard scroll speed is 0.2 · 1.2³ = 0.3456 and the angle for left is π (§6f).
- VW-T13: `SetZoomLimits(0, 9)` with the preference `+0xF10` clear gives a maximum of 4 (§6f).
- VW-T14: `ToggleGodPanel` in an established city opens panel 4, and panel 0 with Shift+Ctrl+Alt held (§6g).
- VW-T15: `QueryTool` twice leaves one query tool on the stack (§6g).
- VW-T16: global triggers: assert, assert, retract, retract sends exactly two messages, (t, 1) then (t, 0); a View3D record with triggers {(t, 2)} ends with count 1 and the pair (t, 2), and loads back (§6k).
- VW-T17: the 24-hour clock at 13:45 shows frame 27 (§6j).
- VW-T18: the cost indicator for a cost above the funds is red with alpha 255; `ClearCostIndicator(true)` fades it over 1500 ms (§6j).

## 8. Open questions

No open VW questions remain. VW1–VW5 were answered 2026-10-21; VW6–VW18 were answered 2026-10-23.
The dated questions and verdicts are in [answered/view3d-window.md](questions/answered/view3d-window.md).
