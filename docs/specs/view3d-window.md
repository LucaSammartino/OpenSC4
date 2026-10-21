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

## 2. Construction (`0x007F0A70`) *[compat]*

Order of effects:

1. The four vptrs are first set to pure-call stubs (`0x00ABC9C0`, `0x00A81174`, `0x00A86864`), the `cGZWin` base is
   constructed at `+0x0C` (`0x0099D938`), then the four final tables of §1 are stored.
2. All owned pointers are nulled (renderer, camera control, model maker, view manager slots, the catalog-menu fields
   `+0x1F8`…`+0x208`, the panel window procs, the occupant filter at `+0x320`, the progress screen).
3. Non-zero defaults (absolute offsets, read on the instructions of the ctor; meanings *[inferred]* where marked):

| Offset | Default | Meaning |
|---|---|---|
| `+0x1F4` | `0xFFFFFFFF` | "no selection" sentinel *[inferred]* |
| `+0x210`, `+0x218`, `+0x220` | `1`, `4`, `7` | three small counters/limits *[inferred]* |
| `+0x278` … `+0x2C8` | `0x8C329937`, `0x31`, `0x29`, `0x299237BF`, `0x6A33E1BA`, `0x2A33E1A3`, `0xEA353064`, `0x2A312F97`, `0x0A5242DF` (each followed by a zero dword) | a table of resource/ID pairs (`{id, 0}`) used by the GUI *[inferred]* |
| `+0x324` | `1` | (written twice: 0 then 1 at `0x007F0FB4`) |
| `+0x334` | `1.0f` | a scale factor *[inferred]* |
| `+0x340`, `+0x344` | `0.2f`, `0.2f` | two tuning terms *[inferred]* (not the ones focus loss zeroes) |
| `+0x368`, `+0x370` | `5`, `0x15200003` | *[inferred]* |
| `+0x380` | a `cSC4RenderProperties` sub-object, constructed in place | render properties (owner class cSC4RenderProperties) |
| `+0x400` | `-1` | *[inferred]* |
| `+0x408` | `1` | *[inferred]* |

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
   key-configuration file binds keys to. *[inferred]* `0x68` is the record count (104); the table was not walked
   to its end (VW3).
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
   (`0x007F3510`), reached through the window message `0x230E27AC` (§6). Per-cheat effects are not specified here
   (VW1).
4. `cGZWin::Init` runs (`0x007E9C27`); its bool result is stored at `+0xE4` and is `Init`'s return value.
5. The window names itself `"cSC4View3DWin"` (`0x007E9C2E`), sets its window ID class `0x9A47B417` (`0x007E9C62`,
   the class ID by which `GetChildAs(0x9A47B417, 0xFA47B3F9, …)` finds it, landfill.md §3.2), sizes itself to its
   parent's client area, and becomes visible/enabled through `cGZWin` slots.
6. Cursor: through the cursor manager, cursor `0xC16C4E1B` is set as the window's cursor (`0x007E9CB2`).
7. The coverage-radius helper at `+0x3D8` is initialised, a byte at `+0x150` cleared, and a `cRZTimer` (the scroll /
   frame timer *[inferred]*) is started (`0x007E9CDD`).
8. Two GZCOM lookups: object `0x42E3EA4B` and class `0x42E967BE` / IID `0xE2C1B3C4` (stored at `+0x114`); the latter
   is then given the window (slot 3). *[inferred]* these are the camera/view-input helpers; identity open (VW2).
9. `+0x119` := 1.
10. If the app's current city record (app slot 39 `+0x9C`, field `+0x204`) is non-zero, the progress screen is
   created (`CreateProgressScreen`, `0x007E5BB0`) — a city is being loaded.
11. `0x00562DD0` is called last.

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
| `0xAC083D7D` | mission target reached | `UpdateMissionProgress(data1, 1)` |
| `0x4A44667F` | progress report *[inferred]* | if the message carries interface `0x089B7DC8` and a progress screen exists, forward it to the progress screen (`0x00777640`) |

The four types `0x6A935C34`, `0x6A935CA4`, `0x6A935CA8`, `0x6A935CAB` close the 33-entry set; they are command
messages and take the first rule.

### 5.1 Pause and mission notifications

For `0x4A7FB7E2`, `0xAA7FB7E0` and `0xABBBCA64`: if a "paused" overlay object is present it is updated from the
simulator's pause state (overlay object `0xC998BB81`, shown when paused at speed 1000 *[inferred]*); then
`UpdateSpeedButtons` and `UpdateAlertBorder`. For `0xABBBCA64` only, `UpdateMissionProgress(data1-derived, data1)`
follows (the mission activation flag from city-situation.md).

### 5.2 View-update notifications

`0xA6B79602` and `0xA6B79612` act only while the app's "city view hidden" byte (`+0x39C` of the object at
`0x00B43CC4`+`0xC`) is zero. `0xA6B79602` first runs `UpdateZoomButtons` and `AddBackgroundImageToRenderer`; both
types then run `DoTemporarySimHiddenPause`, and if the cursor-text flag is set the cursor text is re-placed at the
current mouse position.

## 6. Window messages (`cIGZWin::DoMessage`, `0x007F3E50`) *[compat]*

Every message is first offered to `nsSC4TutorialSystem::FilterUICommandMessage` (tutorial.md). Then:

| Window message | Effect | Result |
|---|---|---|
| `0x230E27AC` (cheat) | `ProcessCheatCode(data1 = cheat ID, data2 = text)` | its result |
| 3 (command), data1 `0x49D993D4` | the window takes keyboard focus, then `InvokeCatalogItemCommand(data2, data3)` | true |
| 3, data1 `0x287259F6` (button) | not handled here (the filter, §6.3, owns buttons) | base `cGZWin` |
| `0x13` (mouse enter) | hover byte `+0x118` := 1 (`0x007F402F`), `Update24HourClockHover`, base | base |
| `0x14` (mouse leave) | hover byte `+0x118` := 0, `Update24HourClockHover`, base | base |
| `0x0D` | `Update24HourClockHover`, base | base |
| `0x10` (focus lost) | only when global `0x00B43CEC` is non-zero: the four scroll-velocity dwords `+0x348`…`+0x354` are zeroed (`0x007F4087`–`0x007F4099`; the handler's `this` is `+0x0C`), `RecalcScrollVelocity`, `ScrollStop` (slot 34), and a `cGZWin` slot 142 call (`GZOnMouseExit`) | true |
| `0x1B` with sub-type 5 / 6 | key down / up: `ProcessKeyDown` / `ProcessKeyUp`; if neither consumes it, the key goes to the key-binding helper (`+0x114`, slot 9) and, if bound, a command is posted through the command dispatcher `0x00B43CB0` | false only when unbound |
| `0xE4A85D91` (link) | `cSC4HTMLLinkHandler::DoURL(data2)` | true when DoURL **failed** *[quirk]* |
| `0x6A5BDFCD`, `0x6A720993`, `0xEA5BDFBC` | `HandleAdviceLinkMessage`, then base | base |
| anything else | base `cGZWin::DoMessage` | base |

### 6.1 Focus loss stops scrolling

A replacement must stop any edge or key scroll when the city window loses focus, but only under the global flag
`0x00B43CEC` (*[inferred]* pause-on-focus-loss / windowed mode; application.md `GP` cheat).

### 6.3 The message filter (`0x007F58E0`)

The filter sub-object (`+0x08`) is installed on child windows (InvokeSecondaryMenu, ui-windows.md §4.3). It first
runs the tutorial filter, then handles window message `3` (and `0x12`): a `0x287259F6` button press goes to
`HandleButtonActivated` (`0x007F4690`); IDs `0x287259F7`, `0x8988BC94`, `0x2988BC85` and `0x4988BC6A` have their
own branches (not specified here, VW4).

## 6a. City init: what the window builds per city *[compat]*

The window exists before any city; the city-scoped objects are built by the PreCityInit notification and torn down
before the next city.

**`DoPreCityInit` (`0x007F41B0`)**, in order:

1. Reads the window's client width and height (`cGZWin` slots 41/42, `+0xA4`/`+0xA8`).
2. Creates the object of GZCOM class `0xE9C6262A` (constructor `0x007FF930`) and stores it at `+0xEC`
   (`0x007F4224`). *[inferred]* a view-side helper; identity open (VW5).
3. Creates the **camera control** (GZCOM class `0xC9C628EC`, `cSC4CameraControl`, `0x007CC990`), stores it at
   `+0xF0` (`0x007F427C`), sets its viewport to the client size (`SetViewportSize`) and calls
   `cSC4CameraControl::Init` with the start focus **(512.0, 0, 512.0)** (`0x007F42A0`–`0x007F42B0`, two `0x44000000`
   floats). The real city size is applied later (PostCityInit step 2).
4. If no occupant filter exists at `+0x320`, a 16-byte one is created and stored there (`0x007F4307`).
5. Creates the **renderer** (GZCOM class `0xE9C622D8`, IID `0x681BD529`, `cSC43DRender`, `0x007C9B10`) and stores
   it at `+0xF4`, the field `GetRenderer` (slot 4, `0x0043FD00`) returns. It is wired to the camera control and the
   `+0xEC` helper through its slots 6, 7, 12 (view3d.md §3.2).
6. Looks up GZCOM `0xA2FFB5D3`/`0x02FFBB60`, then `AddBackgroundImageToRenderer` (`0x007F1640`).
7. Registers script hooks (`cSLString` with `0x6A231EAA`/`0xAA765647`, `0xAA76563A`) and reports progress to the
   progress screen (`0x00777640`).

The model maker is held at `+0xF8` (`GetModelMaker`, slot 5, `0x007F0FE0`).

**`DoPostCityInit` (`0x007ED0F0`)**, in order: updates the progress message; sets the camera control's city size
(`cSC4CameraControl::SetCitySize`, `0x007ED17F`) from the city; refreshes `Update24HourClock` and
`UpdateAmbientLightLevel`; enables the two child windows `0x4992046A` and `0xE9920494`
(`nsGZWinUtils::SetWindowEnabled`); refreshes the simulation clock, population and day/night buttons; posts the bond
message (`cSC4BudgetSimulator::PostBondMessage`); refreshes the mayor rating bar; then runs either
`DoCityEstablished` or `DoCityObliterated` (`0x007ED333`/`0x007ED33A`) according to the city's established state
*[inferred: the branch condition was not read]*; finally it builds a `cSC4ResponseCurve` from properties
`0x6A6564F4`/`0x6A6564F6` (`0x007ED5B5`–`0x007ED5DB`).

## 7. Conformance tests

- VW-T1: QI on the five IIDs of §1 returns the documented sub-object offsets; any other IID returns false.
- VW-T2: after `Shutdown`, no cheat ID of §3 is registered, and no view input control (normal or persistent) remains.
- VW-T3: `0xAA5CF1E3` while modal popups are unsafe is queued once; a second identical message does not add a copy.
- VW-T4: a link message whose URL opens returns false from the window `DoMessage` (§6 quirk).

## 8. Open questions

See [questions/view3d-window.md](questions/view3d-window.md): VW1 (per-cheat effects of `ProcessCheatCode`), VW2
(identity of the two GZCOM helpers in `Init`), VW3 (length of the command-name table), VW4 (filter branches), VW5 (the `+0xEC` helper and the established/obliterated branch).
