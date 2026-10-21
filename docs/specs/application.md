# Application object (cSC4App): lifecycle, services, messages

Subject: the game's application object `cSC4App` (SimCity 4 Deluxe 1.1.641). It owns startup order,
the framework services it registers, the app-wide lifecycle messages, the idle and focus-pause logic,
the command-line and startup-resolution rules, and the per-user directory layout. Written by RE lane
re-application4 (2026-10-21) from the instructions (Ghidra export via tools/re_query.py; REA was not
available this round). Not covered yet (see section 11): city load/save/new/close bodies, window/font/
resource setup, code-plugin loading.

Coverage status: partial owner spec. The sections below cover about 35 of cSC4App's 322 functions
(the vtable surface and the lifecycle path); the large setup bodies listed in section 11 are open.

## 1. Object layout (multiple inheritance)

One object, seven vtable pointers, installed by the constructor `0x0044ACD0` after `cGZApp::cGZApp`
runs on the `+0x10` sub-object. Offsets below are from the object start (the primary pointer) unless
a row says "iface", which means relative to the `cISC4App` sub-object at `+0x0C`
(iface offset = object offset - 0x0C).

| Offset | vtable | Slots | Role |
|---|---|---|---|
| +0x00 | `0x00A86C2C` | 15 | primary: QueryInterface, AddRef, Release, message-target v1 `DoMessage` (+0x0C), then app internals (section 1.1) |
| +0x04 | `0x00A86C1C` | 4 | message target v2: `DoMessage` `0x00451ED0` at +0x0C (section 6.2) |
| +0x08 | `0x00A86C0C` | 4 | third COM face, slot +0x0C = `0x004559E0` (purpose not traced, question AP4) |
| +0x0C | `0x00A86AF0` | 71 | `cISC4App`, the public app interface other subsystems call (section 2) |
| +0x10 | `0x00A86ABC` | 13 | `cGZApp` system-service face: +0x18/+0x1C IsServiceRunning/SetServiceRunning, +0x20 `Init` `0x0044C320`, +0x24 `Shutdown` `0x0044A520`, +0x2C `OnTick`-style thunk to `OnIdle` |
| +0x14 | `0x00A86AA4` | 6 | string-ish helper face (not traced) |
| +0x2C | `0x00A86A68` | 15 | `cIGZApp` framework hooks: +0x18 `PreFrameWorkInit` `0x00452360`, +0x1C `PostFrameWorkInit` `0x00453240`, +0x24 `PreFrameWorkShutdown` `0x004498C0`, +0x38 `LoadRegistry` `0x004584A0` |

### 1.1 Primary vtable (`0x00A86C2C`)

| Slot | Offset | Address | Behaviour |
|---|---|---|---|
| 3 | +0x0C | `0x00451C30` | v1 message handler (section 6.1) |
| 6 | +0x18 | `0x0044BC50` | ReadPreferences |
| 7 | +0x1C | `0x0044EED0` | ReadCommandLine (section 8) |
| 8 | +0x20 | `0x0044D810` | SetupWinCtrlMgr |
| 10 | +0x28 | `0x00453A30` | SetupFontSet |
| 11 | +0x2C | `0x00449380` | FullGamePause(pause) (section 7.2) |
| 12 | +0x30 | `0x00449340` | OnAppActivate(active) (section 7.2) |
| 13 | +0x34 | `0x004548B0` | NewCity |
| 14 | +0x38 | `0x004544C0` | Quit (section 5.2) |

Slots 4, 5 (`0x00449420`, `0x00449470`) and 9 (an empty function) are not traced.

### 1.2 Fields named by this spec

| Field | Type | Meaning | Set by |
|---|---|---|---|
| +0x44 | u32 | app mode: 0 none, 1 region view, 2 city view | ctor 0; section 6.2 |
| +0x48 | u8 | quitting latch | ctor 0; Quit |
| +0x50 | ptr | window/screen owner (`cWinSC4App`), gets `DestroyCurrentScreen` on Quit | |
| +0x54 / +0x58 | ptr | region / regional city | |
| +0x68 | string | application name (base of the .ini/.cfg file names) | |
| +0x320, +0x338 | timer, u32 ms | preferences autosave timer and period | OnIdle |
| +0x340, +0x358 | timer, u32 ms | second periodic timer and period | OnIdle |
| +0x360 | timer | 1-second idle-check gate | OnIdle |
| +0x378 | timer | time since last user input (restarted by mouse messages) | section 6.2 |
| +0x390 | timer | short activity gate (300 ms) | OnIdle |
| +0x3A8, +0x3AC | u32 ms | idle threshold and the amount subtracted after each idle message | |
| +0x3B0 | ptr | `cSC4Preferences` object (`GetPreferences`) | |
| +0x3B4 | struct | new-city specification (`GetNewCitySpecification` returns its address) | |
| +0x5D0 | u8 | pause-on-focus-loss enabled | `EnableFullGamePauseOnAppFocusLoss` |
| +0x5D1 | u8 | game currently fully paused by the app | FullGamePause |
| +0x5D4, +0x600 | object | the two plugin-directory change watchers | PostFrameWorkInit; section 6.1 |
| +0x650 / +0x651 | u8 | debug functionality / popup dialogs enabled | setters |

## 2. The `cISC4App` interface (`0x00A86AF0`, 71 slots)

Getters return the field and do nothing else unless the row says otherwise. Offsets are object offsets.

| Slot | Offset | Name | Behaviour |
|---|---|---|---|
| 3 | +0x0C | OnIdle | section 7.1 |
| 4, 5 | +0x10, +0x14 | RunMessageServerPump / RunMessageServer2Pump | pump the v1 / v2 message servers (not traced) |
| 6 to 12 | +0x18..+0x30 | RequestNewCity, RequestLoadCity, RequestCloseCity, RequestSaveCity, RequestQuit, RequestQuitFromRegion, RequestGoToRegionView | request entry points (bodies open, section 11) |
| 13, 14, 15, 16 | +0x34..+0x40 | LoadCity, CloseCity, SaveCity, SaveCity (second overload) | open, section 11 |
| 17 | +0x44 | SavePreferences | called by Init's failure path and by the autosave timer |
| 18 | +0x48 | EnableFullGamePauseOnAppFocusLoss(b) | writes b to the preferences byte +0x4A4 and to +0x5D0 |
| 21 / 22 | +0x54 / +0x58 | Get/SetDebugFunctionalityEnabled | byte +0x650 |
| 23 / 24 | +0x5C / +0x60 | Get/SetPopupDialogsEnabled | byte +0x651 |
| 27 | +0x6C | GetAppName(out) | copies string +0x68 into out |
| 28 | +0x70 | GetAppIniFileName(out) | app name + `.ini`, only when the name is non-empty |
| 30 | +0x78 | GetAppPreferencesFileName(out) | app name + `.cfg`, same condition |
| 35 / 36 | +0x8C / +0x90 | GetRegion / GetRegionalCity | region getter forwards to the region object's slot +0x20 |
| 38 | +0x98 | GetPreferences | pointer +0x3B0 |
| 39 | +0x9C | GetNewCitySpecification | address of +0x3B4 |
| 40 to 44 | +0xA0..+0xB0 | GetDebugConsole, GetGimexFactory, GetStringDetokenizer, GetWinLocationSaver, GetRenderProperties | pointers +0x5BC, +0x5C0, +0x5C4, +0x5C8, +0x630 |
| 45, 48 | +0xB4, +0xC0 | GetGlyphTextureManager | address of the embedded manager at +0x1E8 (two slots, same function) |
| 46 / 47 | +0xB8 / +0xBC | (unnamed) / GetLuaInterpreter | pointers +0x660 / +0x664 |
| 49 | +0xC4 | GetTutorialRegistry | address of the embedded registry at +0x1CC |
| 50 | +0xC8 | (named GetCDAppDirectory in the export) | returns the byte +0x668; it is a flag, not a path (AP3) |
| 51 to 66 | +0xCC..+0x108 | directory getters | section 9 |
| 67 / 68 | +0x10C / +0x110 | AddDynamicLibraryByName / ByPath | open |

Slots 19, 20 (ApplyVideoPreferences, GetAutoVideoPreferences), 25, 26, 32 to 34, 37 resolve to code
shared with other classes (folded getters) and are not described here.

## 3. Startup order

The framework drives the app through its `cIGZApp` and `cGZApp` faces. The observable order:

1. `PreFrameWorkInit` (`0x00452360`), in this order: read the `DebugLevel` and `logging` options; build
   the version object; split the executable path into directory and name; resolve the user data
   directory (`SC4GetUserDataDirectory`); create the render properties object; read
   `AllowMultipleInstances` and `Restart` (a `Restart` start waits in a sleep loop on a timer before
   continuing; the exact wait is not traced, AP5); **AddCOMDirectors** (section 4.1); **AddSystemServices**
   (section 4.2); query `SystemParametersInfo` / `GetVersion` and the `cursormode` option; date checks
   against the user `Plugins` and `Sku_Data` folders; launch the exception-report uploader;
   **AddCodePlugins**; open the `GZLog` (options `append`, `buffered`) and the `GZShutdownLog`;
   set the environment variables `RZSTD_OUTPUT_FILE_PATH` and `RZSTD_OUTPUT_PREFIX` (prefix
   `StdOut: `); **FindAndRegisterAllAvailableLanguages**; **DoPreferencesChanged(0)**.
2. `cGZApp::Init` = `cSC4App::Init` (`0x0044C320`), section 3.1.
3. `PostFrameWorkInit` (`0x00453240`): create the window, tool and misc command-system supports;
   the network command generator (option `NetCommandGenerator`, value `enabled`); create two user
   directories if missing (each: test, create, test again, then start a change watcher on it: these are
   the watchers at +0x5D4 / +0x600, section 6.1); `InitAlphaSystem`; then the Lua start script: the
   `LuaScript` option or a `-LuaScript:` command-line switch names a script, `FindTestScript` resolves
   it, and the switch is erased from the command line once consumed.

### 3.1 Init

Init returns a bool (true = continue start-up).

1. Fetch the window manager, window-control manager and system-service registry from the framework.
2. Read the install directory from `HKEY_LOCAL_MACHINE\SOFTWARE\Maxis\SimCity 4\Install Dir`. If that
   read fails Init returns **false** with no message.
3. Obtain the graphics driver and give it to the 3D renderer (`cSC43DRender::StaticSetGDriver`). If no
   driver can be obtained or it fails its check: set preferences byte +0xF13 to 0 and +0xF0C to 1
   (switch to the software driver), call SavePreferences, show a message box titled `SimCity 4`
   with the text `SimCity 4 will run with the software driver when next launched.`, return **false**.
4. Driver-dependent settings: a GZCOM object of class `0x0A1D8285` (the config-rule manager in
   re/class_id_table.tsv) is requested with interface `0x0A1D8284`; a driver query with argument 13
   decides a preference byte under +0xF0C (result 1 writes 1, else 0). Exact targets: AP5.
5. **Send** `kSC4MessagePreAppInit` `0x28E9397A` on message server 2 (constructed directly, data =
   the app interface).
6. Register the app on message server 1 for `0xC7AA61C6` (the quit request, section 6.1).
7. Create the cheat-code manager and register three app cheats: `GP` `0x2AB5C431`,
   `CreateException` `0x2B60E45B`, `ForceIdle` `0x8B6D4F8D` (section 6.1).
8. Create the glyph texture manager, the string detokenizer, `SetupResources`, the language manager
   hookup, the persist-resource manager, the render-property vector, the tooltip manager,
   `SL::OnAppInit1`, the tutorial registry (`cSC4TutorialRegistry::Init`).
9. **Send** `kSC4MessagePostAppInit` `0x28E9397B` (PostSC4AppMessage with post = false).
10. Return **true**.

## 4. Services the app owns

### 4.1 COM directors (`AddCOMDirectors` `0x004493B0`)

Adds three directors to the framework's COM DLL director, in order: the SC4 game director
(`SC4GetCOMDirector`), the SC4 audio director (`GetSC4AudioCOMDirector`), the GZ WWW director.

### 4.2 System services (`AddSystemServices` `0x00450350`)

Created and registered in this order (each is constructed and added to the framework):
allocator service; (one unnamed service); file system; then `FindAndPossiblyFixIniFiles`; SC4 message
server (v1) over a GZ message server; SC4 message server 2 over a GZ message server 2; log service
(with a null output stream); command server; **`cSC4GameDataRegistry`**; Lua script server; language
manager; (one unnamed service); font system; window-control manager; clipboard manager; (one unnamed
service); IME proxy; persist-resource manager; registry; S3D texture-binding factory; script-service
resource factory; WWW system; (two unnamed services); audio AEMS event system.

## 5. Shutdown

### 5.1 Shutdown (`0x0044A520`)

1. **Send** `kSC4MessagePreAppShutdown` `0x28E9397C`.
2. Release the app-owned helpers created by Init (tutorial registry, tooltip manager and the heap
   objects of step 8 of section 3.1, freed with GlobalHeapFree); a call on message server 1
   (`RZ::MS`, presumably the unregistration matching Init step 6; AP5).
3. Clear the field +0x314.
4. **Send** `kSC4MessagePostAppShutdown` `0x28E9397D`.

`PreFrameWorkShutdown` (`0x004498C0`, called on the +0x2C face) stops the two directory watchers
+0x5D4 and +0x600; each is stopped only when its "running" test is true.

### 5.2 Quit (primary slot 14, `0x004544C0`)

Returns true the first time, false on every later call. First call: set the quitting latch +0x48;
call slot +0x44 of the service returned by the framework helper `RZ::GS` (`0x00913C1A`) twice
(meaning AP5); CloseCity (cISC4App +0x38); destroy the current
screen; then ask the framework to quit (framework slot +0x4C with argument 0).

## 6. Messages

### 6.1 Message target v1 (`0x00451C30`)

Always returns true. Message type at message +0, data1 at +4, data2 at +8.

| Type | Action |
|---|---|
| `0xC7AA61C6` (quit request) | cISC4App RequestQuit(0, 0) (slot +0x28) |
| `0x87163754` file-change notification | only when data1 is one of the two watchers (+0x5D4, +0x600): get the watched directory, `AddCodePlugins(dir)`, then post `0xA42396C6` (newly loaded plugins) |
| `0x230E27AC` cheat, data1 = cheat ID, data2 = cheat text | `GP` `0x2AB5C431`: with one argument `on`/`off` (lower-cased), set pause-on-focus-loss and apply it; `CreateException` `0x2B60E45B`: with one argument, raise a test exception (`0x005FA520`); `ForceIdle` `0x8B6D4F8D`: post `kMessageGameInIdleState` `0xBA44EB51` with data 60 |

### 6.2 Message target v2 (`0x00451ED0`)

Always returns true.

| Type | Action |
|---|---|
| `0x26D31EC2` PreCityShutdown, `0x8BB5BB46` PreRegionShutdown | app mode := 0 |
| `0xCBB5BB45` PostRegionInit | app mode := 1 |
| `0xEA8AE29A` PostCityInitComplete | app mode := 2 |
| `0x231BBF91` PreferencesChanged | DoPreferencesChanged(0) |
| `0x29A89B05` UnilateralQuit | Quit (section 5.2) |
| `0x0B627AD3`, `0x49A780B2` | LaunchExceptionReportUploader |
| `0xEA65224E` CityObliterationSceneEnded | DoCityObliteration |
| `0xBA44EB51` GameInIdleState | if the global manager at `0x00B43CA8` exists, call its slot +0xAC |
| `0xFA0C6390` mouse, `0x7A104750` | restart the input-idle timer +0x378 |

### 6.3 PostSC4AppMessage (`0x00449BA0`, (type, post))

Builds a standard message 2 with the type and the app interface as data, then `post` true: message
server 2 `MessagePost(msg, false)`; `post` false: `MessageSend(msg)`. Every lifecycle call in this
spec passes false (synchronous).

## 7. Idle and focus pause

### 7.1 OnIdle (`0x00449C70`)

Runs at most once per second (gate timer +0x360, strictly more than 1000 ms). Then: when the input
idle time (+0x378) exceeds the threshold +0x3A8, the idle timer is set back by +0x3AC and
`kMessageGameInIdleState` is posted with the idle time; when the autosave timer +0x320 exceeds +0x338
it restarts and SavePreferences runs; when timer +0x340 exceeds +0x358 it restarts and the global
manager at `0x00B43CA8` gets slot +0xAC. The 300 ms gate (+0x390) path and the tail that tests the
paused flag are not fully traced (AP2).

### 7.2 Focus pause

`OnAppActivate(active)` (primary +0x30): only when pause-on-focus-loss (+0x5D0) is on, and only when
the paused state would change, calls `FullGamePause(!active)`. `FullGamePause(p)` (primary +0x2C):
when p differs from +0x5D1, store it and call `SC4GamePause(p, 1, 1, 1, 1)`.

## 8. Command line and startup resolution

`ReadCommandLine` (`0x0044EED0`) reads these options (value words compared lower-case; `on`/`enabled`
and `off`/`disabled` are synonyms where a switch takes them): `ExceptionHandling` (also `verbose`),
`IME`, `f`, `w`, `g`, `d` (driver: `soft`, `dx`/`directx`, `opengl`), `l`, `audio`, `sound`, `radio`,
`music`, `sim`, `RDD`, `UserDir`, `CPUCount`, `CPUPriority` (value `low` recognised), `AutoZoomChange`;
a resolution value is parsed with the pattern `%ux%ux%u` (width x height x depth). The effect of each
option on fields is not traced yet (AP1).

**IsStartupResolutionOK(w, h)** (`0x0044BA80`, called by FixStartupResolutionValuesIfNeeded):
true when the `CustomResolution` option is `on` or `enabled`. Otherwise true only for these
(minimum width, exact height) pairs, with h at least 600: (800, 600), (1024, 768), (1152, 864),
(1280, 1024), (1600, 1200), (1800, 1440), (2048, 1536).

**FixStartupResolutionValuesIfNeeded(&w, &h)** (`0x0044CD70`): when not OK, replace by area
a = w * h (unsigned): a < 480000 -> 800 x 600; 480000 <= a < 786432 -> 1024 x 768;
a < 995328 -> 1152 x 864; a < 1310720 -> 1280 x 1024; a < 1920000 -> 1600 x 1200;
a < 2592000 -> 1800 x 1440; a < 3145728 -> 2048 x 1536; otherwise 1024 x 768.
Worked examples: 1366 x 768 is kept (height 768 with width at least 1024 is OK). 1280 x 720 is not
OK (height 720 is not listed); a = 921600 lies in [786432, 995328), so it becomes 1152 x 864.
1024 x 600 is OK (height 600, width at least 800) and is kept.

## 9. Directories

Plain getters copy a stored path: data +0x7C, CD data +0x90, SKU-specific +0xCC, user data +0xE0,
slot +0xE4 +0x108, slot +0xE8 +0x11C. GetPluginDirectory (+0xD4) copies +0xA4 and, for a language
index of 0 or more, adds a language subfolder (default name `English`).

Lazy user subfolders: on the first call (cached string empty) the getter asks slot +0xE8 for the base
path, appends the name and a `\`, and caches it; later calls copy the cache.

| Slot | Name | Folder |
|---|---|---|
| +0xEC | GetUserPluginDirectory | `Plugins\` |
| +0xF0 | (regions) | `Regions\` |
| +0xF4 | GetMySimDirectory | `MySim\` |
| +0xF8 | GetAlbumDirectory | `Albums\` |
| +0xFC | GetHTTPCacheDirectory | `HTTPCache\` |
| +0x100 | GetTestScriptDirectory | `Temp\` |
| +0x104 | GetExceptionReportsDirectory | `Exception Reports\` |
| +0x108 | (scripts) | `Scripts\`, base from slot +0xD4 with index -1 |

The export's names for slots +0xE4/+0xE8/+0xF0 are inconsistent with this use; slot +0xE8 is the
user base folder (AP3).

## 10. Implementation notes for a clean-room app

- Keep the order of section 3 and the synchronous sends of the four app messages: other subsystems
  (audio.md: PostAppInit no-op, PreAppShutdown stops all sounds) react to them.
- Init's two failure returns are distinct: registry failure is silent, driver failure saves preferences
  and warns.

## 11. City lifecycle (request, close, region view, obliteration)

Round 2 (2026-10-21), from the instructions only (REA unavailable, decomp/ fails on these bodies).
The `cISC4App` methods run on the `+0x0C` face; the offsets below are object offsets.

Fields used here: `+0x50` window owner (`cWinSC4App`), `+0x58` regional-city holder (a pointer to a
reference slot whose first word is the current `cISC4RegionalCity`, or null), `+0x5C` current city
(`cISC4City`, null when no city is open), `+0x5B8` template DB segment for NewCity (set only during
obliteration), `+0x654` the registered city DB segment.

Shared helpers:
- **Scoped pause.** Every request below that shows a dialog first creates a scoped `cSC4AutoGamePause`
  (arguments 0, 1, 1, 1) and releases it on every return path.
- **Pump.** "Pump both servers" means cISC4App slot 4 then slot 5, each with (100, 500, 5000).
- **PostCityMessage(city, type, d1, d2, post)** (`0x00449AF0`): when message server 2 (global
  `0x00B43CCC`) exists, builds a standard message of the given type whose data carries the city, d1 and
  d2; post true queues it (server slot +0x10 with 0), post false sends it at once (slot +0x0C).

### 11.1 RequestNewCity(regionalCity) (iface slot 6, `0x00454370`)

If a city is open, RequestCloseCity(true) runs first; if a city is still open afterwards the request
returns false. Otherwise the argument is stored as the regional city (+0x58) and the result is
NewCity (primary slot 13).

### 11.2 RequestCloseCity(ask) (iface slot 8, `0x00454C10`)

1. No city open: return false.
2. ask false: return CloseCity().
3. ask true: show the generic confirmation dialog with the two strings (group `0x6A231EAA`, instances
   `0x2A56CDA8` first argument, `0x2A56CD9B` second). "No": return CloseCity(). "Yes": read whether
   the Alt key (virtual key `0x12`) is down now, call RequestSaveCity(1, altDown); when that returns
   false, return false (the city stays open); otherwise return CloseCity().

### 11.3 RequestGoToRegionView(ask) (iface slot 12, `0x004543B0`)

When ask is true (scoped pause):
- If a city is open and the city's slot +0x1A4 returns true, show the scripted modal dialog
  (`0x4A89B3F2`, `0x2A96ED21`); result `0xEA57DA59` continues without saving, anything else returns
  false.
- Otherwise show the scripted modal dialog (`0x6A553AA4`, `0xAA921F4F`). Result `0x8A921F5B`: pump both
  servers, then RequestSaveCity(0, altDown) (result ignored) and continue. Result `0x8A921F65`: continue
  without saving. Anything else: return false.

Continue (also the whole path when ask is false): the RZ::GS service's slot +0x44 is called twice,
then CloseCity(); if no city is open afterwards the window owner gets `DestroyCurrentScreen` then
`CreateRegionScreen`. Returns true, also when the city did not close.

### 11.4 CloseCity() (iface slot 14, `0x00455330`) and PartialCloseCity (`0x00454A60`)

PartialCloseCity (returns whether a city was open):
1. If a city is open: `SC4GamePause(1, 0, 1, 1, 1)`; pump both servers; send PreCityShutdown
   (`0x26D31EC2`, d1 = d2 = 0, sent at once); city slot +0x10 (shutdown); post PostCityShutdown
   (`0x26D31EC3`, d1 = d2 = 0, queued); release the city and clear +0x5C; unregister the city segment
   (below).
2. In all cases, the resource manager (global `0x00B43CA8`) gets slot +0xAC when it exists.

CloseCity returns false when PartialCloseCity found no city (after the same resource-manager call).
Otherwise, when a regional city is set, it refreshes the region (GetRegion, iface slot 35):
- the regional city's slot +0x7C string is read; empty: `Region.RemoveCity(holder)`;
- non-empty: take `Region.GetAllCities` and, for each listed city whose tile rectangle overlaps or
  touches the closed city's rectangle (inclusive on both edges, so edge and corner neighbours count),
  reload it: the closed city's own entry with `ReloadCity(holder)`, any other with
  `ReloadCity(GetCity(x, y))` at its position. Positions and sizes come from regional-city slots
  +0x14 and +0x1C.

Worked example: the closed city occupies x 4..8 (x = 4, size 4) and a neighbour starts at x = 8 with
size 4: 8 lies in [4 - 4, 4 + 4], so it is reloaded; one starting at x = 9 is not.
CloseCity then returns true (and calls the resource manager's slot +0xAC once more).

### 11.5 City DB segment (`0x00454B20` register, `0x00454A10` unregister)

Register(path): create a `cSC4DBSegment`, initialise it, set its path, open it for reading; on
success keep it at +0x654 and add it to the resource manager (slot +0x70); on failure shut it down
and release it. Unregister: when +0x654 is set, remove it from the resource manager (slot +0x74),
shut it down, close it, release it and clear +0x654.

### 11.6 DoCityObliteration (`0x00454E50`, on CityObliterationSceneEnded, section 6.2)

Rebuilds the open city through a save and reload:
1. Busy cursor (id 15) for the whole call. Get a temporary file name; when that succeeds,
   SaveCityForObliteration(temp) writes the city into a new DB segment opened for writing (the city's
   slot +0x1B4 does the writing). Its result is the function's return value.
2. Remember the view zoom and rotation (`SC4ViewUtilities::GetZoomAndRotation`, global
   `0x00B43DD8`, when present).
3. Read the city's file path (city slot +0x28), call city slot +0x5C(0), PartialCloseCity, and delete
   the city's file when the path is non-empty.
4. Open the temp file as a read segment. On success: set it as the template segment (+0x5B8),
   NewCity(); on the new city call slot +0x5C(0) and give it the old path (slot +0x2C); clear +0x5B8;
   close the segment; when a city is open, SaveCity(path, 0) (iface slot 15) writes it back to the
   original path.
5. Restore zoom (3D view global `0x00B43DC4`, slot +0x20) and rotation (slot +0x38) when the view
   exists; shut down and release the segment; delete the temp file.

### 11.7 City init message sequence (NewCity and LoadCity)

Both paths end the same way, with `new` = 1 for NewCity and 0 for LoadCity:
1. send PreCityInit `0x26D31EC0` (d1 = new, d2 = 0) at once;
2. city slot +0x0C (init);
3. queue, in this order: `0x2B96B3EA` (d1 = d2 = 0), PostCityInit `0x26D31EC1` (d1 = new, d2 = 0),
   `0xAB96B05F`, PostCityInitComplete `0xEA8AE29A` (section 6.2 sets app mode 2 when it is delivered);
4. pump both servers through the global app pointer (`0x00B43C94`), then the resource manager's
   slot +0xAC.

### 11.8 NewCity() (primary slot 13, `0x004548B0`)

Returns false at once when a city is already open. Otherwise, with a busy cursor (id 15) and a scoped
input-queue flush (1000), the window owner destroys the current screen and creates the city screen;
a new `cSC4City` (0x20C bytes) becomes +0x5C (one reference); when a regional city is set it is told
about the city (regional-city slot +0x68 with the city); then section 11.7 with new = 1. Returns true.
The city's own init reads the template segment +0x5B8 when obliteration set it (11.6); NewCity does
not touch +0x5B8.

### 11.9 LoadCity(path, regionalCity) (iface slot 13, `0x004555F0`)

1. If a city is open, CloseCity() (result not checked). Store regionalCity in +0x58. Busy cursor (15)
   and input-queue flush (1000) for the whole call.
2. Create a DB segment for path and open it for reading. Open fails: release it and go to step 7
   with failure.
3. Read the save version (`SC4ReadAppLoadSaveVersion`); it must lie in 62..65 (`0x3E`..`0x41`)
   inclusive, else unregister (no-op) and fail.
4. Send message `0x26C63340` (data: the segment and the path string). Keep the segment as the city
   segment (+0x654) and add it to the resource manager (slot +0x70).
5. If the segment's slot +0x1C test passes, open the load stream (type `0x49DD6F4C`, group
   `0x49DD6F4E`, instance 1); it must be open with version word 1, then Read (below) must succeed.
   Then: the city gets slot +0x18 with the regional city's slot +0x6C value, slot +0x2C(path) and
   slot +0x3C(0); the window owner destroys the current screen and creates the city screen; send
   `0x26C63341` then `0x26C63342` (same data); section 11.7 with new = 0; success.
6. On any failure after step 4: release the half-read city (+0x5C cleared) and unregister the city
   segment.
7. Release the local segment reference; pump both servers and call the resource manager's
   slot +0xAC. Return success.

**Read(stream, segment)** (`0x00454620`, called directly by LoadCity): read a 16-bit version that
must be 2; read a serialised object of class `0x66BA4DB5` into +0x5C (the city); read a regional city
record and, on success, store it in the regional-city holder (+0x58), releasing the previous one.
Returns true only when all three succeed. **Write(stream, segment)** (`0x004545D0`, no caller in the
export) is the mirror: version 2, the city, then the regional city when the holder is set.

### 11.10 RequestLoadCity() (iface slot 7, `0x00456910`) and DoLoadCityUI (`0x00456860`)

When a city is open, RequestCloseCity(true) first; a city still open returns false. DoLoadCityUI
fills the path with the fixed name `SavedCity` followed by the string at global pointer
`0x00B07EF0`; it shows no browser and always returns true (a stub in this build). Then
LoadCity(path, null regional city); on failure the generic notification dialog shows strings
(group `0x6A231EAA`, instances `0xCA56CE8C` and `0x4A56CEA6`). Returns LoadCity's result.

### 11.11 SaveCity(flag) (iface slot 16, `0x00454DA0`)

No city open: false. Reads the city's file path (city slot +0x28); empty path: false; otherwise the
result of SaveCity(path, flag) (iface slot 15, `0x00455E20`, 2611 bytes, open).

## 12. Open (not specified here)

SaveCity(path, flag) `0x00455E20`, RequestSaveCity `0x00455AE0`, RequestQuit `0x004546B0`,
RequestQuitFromRegion `0x00454800`, the `0x00455060` DoMessage, SetupWinCtrlMgr, SetupFontSet, SetupResources, AddCodePlugins,
AddDynamicLibrary*, ReadPreferences/SavePreferences/DoPreferencesChanged bodies, the DB segment
callbacks, LoadRegistry. Questions: docs/specs/questions/application.md (AP1 to AP5).
