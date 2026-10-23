# Application object (cSC4App): lifecycle, services, messages

Subject: the game's application object `cSC4App` (SimCity 4 Deluxe 1.1.641). It owns startup order,
the framework services it registers, the app-wide lifecycle messages, the idle and focus-pause logic,
the command-line and startup-resolution rules, and the per-user directory layout. Written by RE lane
re-application4 (2026-10-21) from the instructions (Ghidra export via tools/re_query.py; REA was not
available this round). Not covered yet (see section 12): SaveCity(path)/RequestSaveCity/RequestQuit bodies,
window/font/resource setup, code-plugin loading.

Coverage status: partial owner spec. After round 2 the sections below cover about 52 of cSC4App's 322
functions (vtable surface, lifecycle, city request/load/close/obliteration path, OnIdle, part of
ReadCommandLine); section 12 lists what is still open. This is not a complete owner spec. Sections 13-15
(preferences, plugins and resources, message pumps) and the corrections marked re-app were added 2026-10-22.

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
| 4, 5 | +0x10, +0x14 | RunMessageServerPump / RunMessageServer2Pump | pump the v1 / v2 message servers (section 15) |
| 6 to 12 | +0x18..+0x30 | RequestNewCity, RequestLoadCity, RequestCloseCity, RequestSaveCity, RequestQuit, RequestQuitFromRegion, RequestGoToRegionView | request entry points (all seven specified, sections 11.1-11.3, 11.10, 11.12-11.14) |
| 13, 14, 15, 16 | +0x34..+0x40 | LoadCity, CloseCity, SaveCity, SaveCity (second overload) | sections 11.9, 11.4, 11.16, 11.11 |
| 17 | +0x44 | SavePreferences | called by Init's failure path and by the autosave timer (section 13.1) |
| 18 | +0x48 | EnableFullGamePauseOnAppFocusLoss(b) | writes b to the preferences byte +0x4A4 and to +0x5D0 |
| 21 / 22 | +0x54 / +0x58 | Get/SetDebugFunctionalityEnabled | byte +0x650 |
| 23 / 24 | +0x5C / +0x60 | Get/SetPopupDialogsEnabled | byte +0x651 |
| 27 | +0x6C | GetAppName(out) | copies string +0x68 into out |
| 28 | +0x70 | GetAppIniFileName(out) | app name + `.ini`, only when the name is non-empty |
| 29 | +0x74 | GetAppIniFilePath(out) | section 14.3; candidate under slot 51, otherwise slot 58 plus INI name |
| 30 | +0x78 | GetAppPreferencesFileName(out) | app name + `.cfg`, same condition |
| 31 | +0x7C | GetAppPreferencesFilePath(out) | slot 58 base plus preferences filename; no existence test (`0x0044C090`) |
| 35 / 36 | +0x8C / +0x90 | GetRegion / GetRegionalCity | region getter forwards to the region object's slot +0x20 |
| 38 | +0x98 | GetPreferences | pointer +0x3B0 |
| 39 | +0x9C | GetNewCitySpecification | address of +0x3B4 |
| 40 to 44 | +0xA0..+0xB0 | GetDebugConsole, GetGimexFactory, GetStringDetokenizer, GetWinLocationSaver, GetRenderProperties | pointers +0x5BC, +0x5C0, +0x5C4, +0x5C8, +0x630 |
| 45, 48 | +0xB4, +0xC0 | GetGlyphTextureManager | address of the embedded manager at +0x1E8 (two slots, same function) |
| 46 / 47 | +0xB8 / +0xBC | (unnamed) / GetLuaInterpreter | pointers +0x660 / +0x664 |
| 49 | +0xC4 | GetTutorialRegistry | address of the embedded registry at +0x1CC |
| 50 | +0xC8 | (named GetCDAppDirectory in the export) | returns the byte +0x668; it is a flag, not a path (AP3) |
| 51 to 66 | +0xCC..+0x108 | directory getters | section 9 |
| 67 / 68 | +0x10C / +0x110 | AddDynamicLibraryByName / ByPath | section 14.1 |

The folded getters return a 32-bit field without retaining it or invoking a foreign simulator.
Their export labels describe other users of the same instructions, not app behavior:

| Slot | Address | Field relative to cISC4App / whole app | App meaning where established |
|---|---|---|---|
| 25 | `0x0080BC00` | +0x38 / +0x44 | app mode |
| 26 | `0x0068D1B0` | +0x40 / +0x4C | stored value; semantic name unconfirmed |
| 32 | `0x006EFA20` | +0x308 / +0x314 | feature-manager pointer; Init stores constructor `0x0046F8B0` result at `0x0044CABF` relative to its +0x10 face |
| 33 | `0x006EFA30` | +0x30C / +0x318 | cheat-code-manager pointer; Init stores constructor `0x00921970` result at `0x0044C76E` relative to its +0x10 face |
| 34 | `0x00698CF0` | +0x48 / +0x54 | region pointer |
| 37 | `0x0081F910` | +0x50 / +0x5C | city pointer |

Slots 19 and 20 are the app's own `ApplyVideoPreferences` (`0x0044A1C0`) and `GetAutoVideoPreferences`
(`0x0044A330`) (vtable words `0x00A86B3C`, `0x00A86B40`; section 13.3; corrected 2026-10-22, re-app).

## 3. Startup order

**Entry** (`SC4AppStart`, `0x0044C170`, called by `_main` `0x0044C2B0`; added 2026-10-22, re-app) *[summary]*: the
framework is a `cSC4Framework` when the byte `0x00B21428` is set, else a `cGZFramework` (0x150 bytes each); the
application (0x670 bytes, name `"SimCity 4"`) is made and held; `sSetFrameWork`, `sSetApplication(app + 0x2C)`;
`cGZFramework::Main(argc, argv)` runs everything below and the main loop; then the application is released and the
framework's slot 40 is called with 1. When the byte `0x00B07EE0` is clear afterwards, `SC4DisplayResourceError`
shows the text at `0x00B21420`.

The framework drives the app through its `cIGZApp` and `cGZApp` faces. The observable order:

1. `PreFrameWorkInit` (`0x00452360`), in this order: read the `DebugLevel` and `logging` options; build
   the version object; split the executable path into directory and name; resolve the user data
   directory (`SC4GetUserDataDirectory`); create the render properties object; read
   `AllowMultipleInstances` and `Restart` (when another instance runs, the instance object at `+0x648` answers 0 to its
   slot 3; then start-up fails unless `AllowMultipleInstances` is given, except that with `Restart` it first polls
   that slot again, sleeping 200,000 between polls, for up to 30 units of a timer of unit 5 *[inferred: seconds]*,
   and goes on once the other instance is gone; `0x0045265D`–`0x004527AC`, re-app); **AddCOMDirectors** (section 4.1); **AddSystemServices**
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
   After a driver passes its check, `0x0044AC30(driver, &prefs +0xF0C)`: when a byte of the driver's
   description (driver slot 3, then slot 64, byte `+0x30`) is set, `+0xF13` := 1; otherwise, when `+0xF13` was
   set, a message box (`SimCity 4`, `Could not initialize the hardware driver. Using the software driver
   instead.`) is shown, `+0xF13` := 0 and `+0xF0C` := 1 (re-app).
4. Driver-dependent settings: a GZCOM object of class `0x0A1D8285` (the config-rule manager in
   re/class_id_table.tsv) is requested with interface `0x0A1D8284`; a driver query with argument 13
   decides a preference byte under +0xF0C (result 1 writes 1, else 0). Exact targets: AP5.
5. **Send** `kSC4MessagePreAppInit` `0x28E9397A` on message server 2 (constructed directly, data =
   the app interface).
6. Register the app on message server 1 for `0xC7AA61C6` (the quit request, section 6.1). Then subscribe the
   v2 target on message server 2 to `0xFA0C6390`, `0x7A104750`, `0xBA44EB51`, `0x49A780B2`, `0xEA65224E`,
   `0xCBB5BB45`, `0x8BB5BB46`, `0xEA8AE29A` and `0x26D31EC2` (section 6.2; `0x0044C679`–`0x0044C74F`, re-app).
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
allocator service (`cGZAllocatorService`); `0x276C784B` (unnamed); file system (`cGZFileSystem`, `0xE0203660`); then
`FindAndPossiblyFixIniFiles` (section 14.3); SC4 message server (v1) over a GZ message server (`0xC1085722`); SC4
message server 2 over a GZ message server 2 (`0x652294C7`); log service (`nGZLog::cService`, with a null output
stream); command server (`0xCB90253A`); **`cSC4GameDataRegistry`** (`0xE898ED03`); Lua script server (`0x4BD10129`);
language manager (`0xA3024279`); `0x94AF390C` (unnamed, constructor `0x0081F4A0`); `0xA499932C` (it takes the
graphics driver interface `0xC47B747C`; *[inferred]* the graphics system); `0x66B217A1` (unnamed); font system
(`cGZFontSys`, `0x20673960`); `0x84174448` (unnamed); window-control manager (`0x22C2EB1F`); clipboard manager
(`0x07EC0010`); `0x2AC44AE7` (unnamed, constructor `0x00996271`); IME proxy (`0x2AC44AE9`); persist-resource manager
(`0x656B8EFC`); registry (`cGZRegistry`); S3D texture-binding factory (`0xFAC0E219`); `0xFAC0E21A` (unnamed);
script-service resource factory (`0xFA3562FA`); WWW system (`0x449B1912`); two unnamed services (constructors
`0x0078D050`, `0x005C22F0`); audio AEMS event system. Each is queried for the system-service interface `0x287FB697`
and added. (Order and IDs re-read 2026-10-22, re-app: the earlier text had one unnamed service where the bytes have
three before the font system and one after it; the unnamed ones are AP10.)

## 5. Shutdown

### 5.1 Shutdown (`0x0044A520`)

1. **Send** `kSC4MessagePreAppShutdown` `0x28E9397C`.
2. Release the app-owned helpers created by Init (tutorial registry, tooltip manager and the heap
   objects of step 8 of section 3.1, freed with GlobalHeapFree); unregister the three cheats from the
   cheat-code manager (`+0x318`), release it and clear the pointer; unsubscribe the nine messages of section
   3.1 step 6 from message server 2, then `0xC7AA61C6` from message server 1 (`RZ::MS`, `0x00913B97`)
   (`0x0044A5E1`–`0x0044A843`; AP5 part answered, re-app).
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

Runs on the `+0x0C` face; always returns false. Steps (traced in full in round 2, answers AP2):
1. Gate: when timer +0x360 shows 1000 ms or less, skip to step 5. Otherwise restart it.
2. Idle check. Let `t300` = elapsed time of timer +0x390.
   - `t300 >= 300`: restart the input-idle timer +0x378 and use the value 60; go to step 3.
   - `t300 < 300`: let `idle` = elapsed of +0x378. When `idle` > threshold +0x3A8, set the input-idle
     timer's elapsed time to `threshold - (+0x3AC)` (not "subtract"; it is set to that value) and use
     the value `idle`; go to step 3. Otherwise go to step 4 without a message.
3. Restart +0x390. When message server 2 (`0x00B43CCC`) exists, queue GameInIdleState
   (`0xBA44EB51`) with the value as its first data word.
4. Autosave timer +0x320 above +0x338: restart it, SavePreferences (iface slot 17). Timer +0x340
   above +0x358: restart it, resource manager (`0x00B43CA8`) slot +0xAC when present.
5. Tail, on every call (also when the gate skipped): when the app-paused byte +0x5D1 is set, sleep the
   current thread with argument 250000 (`0x3D090`; the unit of `cRZThread::SleepCurrentThread` is
   not checked here).

Consequence (worked): +0x390 is touched only by the constructor (start), the destructor and step 3
(scan of all faces' offsets in 0x00448000-0x00459FFF). Since step 1 lets a pass through at most once
per 1000 ms and step 3 restarts +0x390 on every message, `t300` is always at least 1000 on a pass, so
the original takes the first branch every time: each pass restarts the input-idle timer and queues
GameInIdleState with 60, about once per second; the threshold branch (+0x3A8/+0x3AC) is effectively
unreachable. A clone that keeps the 300 ms branch gets the same observable rate.

### 7.2 Focus pause

`OnAppActivate(active)` (primary +0x30): only when pause-on-focus-loss (+0x5D0) is on, and only when
the paused state would change, calls `FullGamePause(!active)`. `FullGamePause(p)` (primary +0x2C):
when p differs from +0x5D1, store it and call `SC4GamePause(p, 1, 1, 1, 1)`.

## 8. Command line and startup resolution

`ReadCommandLine` (`0x0044EED0`) reads these options (value words compared lower-case; `on`/`enabled`
and `off`/`disabled` are synonyms where a switch takes them): `ExceptionHandling` (also `verbose`),
`IME`, `f`, `w`, `g`, `d` (driver: `soft`, `dx`/`directx`, `opengl`), `l`, `audio`, `sound`, `radio`,
`music`, `sim`, `RDD`, `UserDir`, `CPUCount`, `CPUPriority` (value `low` recognised), `AutoZoomChange`;
a resolution value is parsed with the pattern `%ux%ux%u` (width x height x depth).

Per-option effects traced in round 2 ("prefs" = the preferences object at +0x3B0; on = `on` or
`enabled`, off = `off` or `disabled`, value lower-cased first):

| Option | Effect |
|---|---|
| `IME` | off: prefs byte +0x4EF := 0; on: := 1 |
| `r` (character switch `0x72`) | value `WxHxD` via `%ux%ux%u`, defaults from prefs; after FixStartupResolutionValuesIfNeeded (section 8 below) prefs +0xF1C width, +0xF20 height, +0xF24 depth (exact acceptance checks at `0x0044F21E`..`0x0044F253` not detailed) |
| `f` / `w` | prefs byte +0xF17 := 1 (full screen) / := 0 (windowed) |
| `g` | value starting with `p` or `P`: prefs byte +0x4A4 := 1 (the pause-on-focus-loss byte, section 2 slot 18) |
| `d` | app +0x65C (driver id) := 0 first; then by **substring** of the value, in this order: `opengl` -> `0xC4554841`, `soft` -> `0x7ACA35C6`, `dx` -> `0xBADB6906`, `directx` -> `0xBADB6906`; any other value is parsed as a hexadecimal class ID (`0x0090FCEF`) (`0x0044F46D`–`0x0044F56B`; corrected 2026-10-22, re-app: there is no `gl` test and the match is a substring search). *[quirk]* `-d:xdx` selects DirectX |
| `audio` or `sound` | both names are looked up before one shared value test (`audio` first, `sound` only when `audio` is absent); on: prefs bytes +0x04 and +0x69 := 1; off: both := 0; any other value: no change |
| `radio` or `music` | same pattern; on: prefs dword +0x10 := 1; off: := 0; other: no change |
| `ExceptionHandling` | value lower-cased, then three independent tests on the exception-handler object embedded at app +0x1E8 (the object section 2 lists under GetGlyphTextureManager; that export name does not fit this use): `off`/`disabled` -> its enable(0) (`0x0091DF5C`); `on`/`enabled` -> enable(1); `verbose` -> enable(1) and option byte 6 := 1 (`0x0091DEFE`, byte +0x118+index). enable(1) when not yet enabled sets byte +0x120, installs a process-wide unhandled-exception filter (import `0x00A80264`, *[inferred]* SetUnhandledExceptionFilter) and keeps the previous filter at +0x128; enable(0) when enabled restores the previous filter |
| `l` | the language/locale value as typed: a leading `:` is removed, then a leading `"`, then a trailing `"`; the result is cut to at most 63 characters and copied (NUL-terminated) into the prefs char buffer +0x4AC. Example: `-l:"French"` gives `French` |
| `UserDir` | value lower-cased for the test only. `off`/`disabled`: the user base folder (section 9, slot +0xE8, string at +0x118/+0x11C) := the folder held at +0xA0/+0xA4 (the base GetPluginDirectory starts from). Otherwise, when the path test `0x00918080` accepts the value as typed (it starts with a letter and passes further checks, not traced), the user base folder := the value; a rejected value changes nothing |
| `CPUPriority` | only the value `low` (exactly 3 characters after lower-casing) acts: the process priority class is set to `0x40` (idle) through two imports (`0x00A800A0` then `0x00A8009C`, *[inferred]* GetCurrentProcess / SetPriorityClass); any other value: nothing |
| `AutoZoomChange` | off/disabled -> render properties (app +0x630, section 2 GetRenderProperties) slot +0x24 with (`0x25`, 0); on/enabled -> (`0x25`, 1); other: nothing |
| `sim` | value parsed as an English-locale float into prefs float +0x470 |
| `RDD` | on: prefs byte +0x498 := 1; off: := 0 |
| `CPUCount` | value parsed with `atol` and passed to `0x005F8CD0`: n := max(n, 1), a mask of the n lowest bits, the process affinity read (`GetProcessAffinityMask`) and set to that mask (re-app) |

All options of `ReadCommandLine` are now traced (AP1 answered 2026-10-21). Value matching: `off` and `on`
are compared as exact 3- and 2-byte strings after lower-casing, `disabled`/`enabled` as whole strings.

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

Complete slots 51–58 (*[compat]*, AP15): the plain getters assign the stored NUL-terminated
path to the caller's output string; they do not create, validate, normalize or cache a folder.
Offsets in this table identify stored character pointers, not embedded string headers.

| Slot | Interface offset | Address | Path source (whole-app offset) / arguments |
|---|---|---|---|
| 51 | +0xCC | `0x00449DE0` | data, +0x7C; out |
| 52 | +0xD0 | `0x00449E00` | CD data, +0x90; out |
| 53 | +0xD4 | `0x0044BF90` | plugin, +0xA4; out, signed language index; negative means base only |
| 54 | +0xD8 | `0x0044C010` | CD plugin, +0xB8; out, signed language index; same language suffix rule |
| 55 | +0xDC | `0x00449E20` | SKU-specific, +0xCC; out |
| 56 | +0xE0 | `0x00449E40` | user data, +0xE0; out; ordinary copy, no missing implementation |
| 57 | +0xE4 | `0x00449E60` | stored path +0x108; out; retain offset-based identity |
| 58 | +0xE8 | `0x00449E80` | stored user base +0x11C; out; distinct from slot 56 |

The slot 57/58 semantic aliases remain AP3; callers must not exchange them merely because
the export calls slot 58 `GetRegionsDirectory`. The table fixes dispatch independently of those names.

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

**Failure contract (AP15, instruction audit `0x00454ED3`–`0x00455051`):** this is a destructive
rebuild, with no rollback on failure to make or write the temporary file. Partial close and
deletion of the old nonempty path still occur; a failed temporary read skips NewCity and the
replacement save. The read segment's initialization return is ignored: path assignment and
read-open are still attempted. Read-open failure still shuts down and releases the local segment
and attempts temporary-file removal, but does not call its close method. On read-open success,
the template pointer is cleared after NewCity even if no city results; close is called, and a
replacement save is attempted only if a city exists. No template segment is registered as the
ordinary city resource segment by this routine. Save failure is not a rollback trigger.

Zoom and rotation start at zero. Missing source view leaves those zero values; a destination
view, if present, receives both zero values even after failed rebuilding. Missing destination
view skips both setters. These are independent singleton checks, not a requirement that both exist.

`SaveCityForObliteration` (`0x00454510`) releases its segment after failed initialization;
after successful initialization it also shuts it down, even when write-open fails. Close is
called only following successful write-open. Its return is **always true on normal completion**
(`0x004545BC`), regardless of initialization/open/write result. Consequently step 1's result is
not a reliable write-success indicator. If temporary-name generation fails, the outer return
byte is uninitialized; do not assign it a deterministic false value.

An open city is a precondition for the temporary writer: its successful open path dereferences
the city without a null check (`0x00454588`–`0x0045458E`). Thus no-city obliteration has no general
safe false/no-op contract. The outer routine guards only the old-path read and city flag update;
if the writer reaches that dereference without a city, the native call is invalid. Allocation
failure likewise has no recoverable false contract. Portable implementations may reject such
invalid invocations explicitly *[choice]*; conformance must not claim that rejection is native behavior.

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
4. Send message `0x26C63340` PreLoad (data: the segment and the path string). Keep the segment as the city
   segment (+0x654) and add it to the resource manager (slot +0x70).
5. If the segment's slot +0x1C test passes, open the load stream (type `0x49DD6F4C`, group
   `0x49DD6F4E`, instance 1); it must be open with version word 1, then Read (below) must succeed.
   Then: the city gets slot +0x18 with the regional city's slot +0x6C value, slot +0x2C(path) and
   slot +0x3C(0); the window owner destroys the current screen and creates the city screen; send
   `0x26C63341` Load then `0x26C63342` PostLoad (same data, names from re/class_id_table.tsv); section 11.7 with new = 0; success.
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
result of SaveCity(path, flag) (iface slot 15, `0x00455E20`, section 11.16).

Round 3 (2026-10-21), again from the instructions only (REA "No app is open"; decomp/ not used).
"City blocks saving" below means a city is open and the city's slot +0x1A4 returns true (the same test
as 11.3; its name is not known).

### 11.12 RequestSaveCity(notify, flag) (iface slot 9, `0x00455AE0`)

Scoped pause. Result starts false.
1. No city open: return false.
2. City blocks saving: show the generic notification dialog (group `0x6A231EAA`, instances
   `0xAA666AFD` first argument, `0xCA666B15` second) and return false. No save is attempted.
3. Read the city's file path (city slot +0x28). Empty path: return false, no dialog.
4. result := SaveCity(path, flag) (iface slot 15). Read the path again and take its file-name part
   (`cRZFile::GetPathName` with 1).
5. Save succeeded: when notify is true, show the generic notification dialog with first argument
   string `0x2A56CDA8` and second argument string `0x2A56CE36` followed by ` (` + file name + `)`.
   When notify is false, no dialog.
6. Save failed: always (whatever notify is) show the dialog with first argument `0x2A56CDA8` and
   second argument string `0x4A56CE71` followed by ` (` + file name + `)`.
7. Return result.

Worked example: city file `C:\...\Regions\Timbuktu\City - New City.sc4`, notify true, save OK: the
second string is the text of `0x2A56CE36` then ` (City - New City.sc4)` (assuming GetPathName(…, 1)
yields name plus extension; the flag's exact meaning belongs to the file-utility spec).

Callers seen: RequestCloseCity (11.2) passes (1, altDown); RequestGoToRegionView (11.3) and
RequestQuit (11.13) pass (0, altDown), so leaving a city never shows the success dialog.

### 11.13 RequestQuit(ask, saveWhenNotAsking) (iface slot 10, `0x004546B0`)

Scoped pause. No direct callers (reached through the interface).
1. City blocks saving (11.12): when ask is true, show the scripted modal dialog
   (`0x4A89B3F2`, `0x2A96ED21`, the same pair as 11.3); result `0xEA57DA59` goes to Quit below,
   anything else returns false. When ask is false go straight to Quit.
2. Otherwise (no city, or the city does not block saving):
   - ask true: scripted modal dialog (`0x0A55161D`, `0xAA921F4F`). Result `0x8A921F5B`: Save-and-quit.
     Result `0x8A921F65`: Quit. Anything else: return false.
   - ask false: saveWhenNotAsking true: Save-and-quit; false: Quit.
3. Save-and-quit: pump both servers (11 intro), read the Alt key (virtual key `0x12`), call
   RequestSaveCity(0, altDown) and ignore its result, then Quit. With no city open, RequestSaveCity
   simply returns false.
4. Quit: build a standard message of type `0x29A89B05` (UnilateralQuit) and hand it to message
   server 2 (global `0x00B43CCC`) slot +0x1C with the app's message-target face (object +0x04) as the
   target. Slot +0x1C (`0x0092EA00`) appends the (message, target) pair to the server's queue under its
   lock, so the quit is delivered later by the pump to the app's v2 target (section 6.2 -> Quit,
   section 5.2), not inside this call. Release the message, return true.

### 11.14 RequestQuitFromRegion(ask) (iface slot 11, `0x00454800`)

No direct callers. When ask is true: scoped pause around the scripted modal dialog
(`0x4A551B4C`, `0xAA921F4F`); result `0x8A921F70` returns false (cancel); any other result continues.
When ask is false there is no dialog. Then it queues UnilateralQuit exactly as 11.13 step 4 and returns
true. There is no save step (a region view has no open city to save).

### 11.15 Load-height-field dialog (`0x00455060`, export name `cSC4App::DoMessage`) and its window handler

The export name is misleading: `0x00455060` takes one string out-parameter and returns a bool; it is
the terrain import file picker.
1. Open an INI file object. When the global app (`0x00B43C94`) exists, its path is
   GetAppIniFilePath (iface slot 29, +0x74); read section `Settings`, key
   `Terrain Load/Save Location` (default empty) as the starting folder.
2. Create a `cSC4WinLoadHeightField` window (0xF0 bytes), set window flags `0x10000` and `0x100`
   (window slot +0x110 with (flag, 1) each), set its initial directory to the INI value.
3. When the window manager (`RZ::WM`) has a root window, centre the dialog in it:
   x = (root width - dialog width) / 2, y = (root height - dialog height) / 2 (signed, truncating).
4. Register it with the global at `0x00B43CE0` (slot +0x28 with (dialog, 1)) and run it modally
   (window manager slot +0xA4).
5. Non-zero modal result: write the dialog's current directory back to the same INI key; then, if the
   dialog has a selection, copy the selected path into the out string and the result becomes true.
6. Unregister (`0x00B43CE0` slot +0x18), release the dialog and the INI object; return the result.

Its only caller is the +0x08 face's slot +0x0C (`0x004559E0`, partial AP4 evidence). That handler
returns false unless the source window's slot +0xFC returns `0x27DF05BE` and the message words are
(3, `0x287259F6`, code). Code `0x20`: window manager slot +0x24 with (`0xF`, window, `0x20`), return
true. Code `0x30`: run the dialog above; on true copy the chosen path into the global app's
GetNewCitySpecification (iface slot 39, +0x3B4), keeping at most the first 511 characters (a longer path
is truncated by dropping its tail), return true. Any other code: return true. Meaning: the new-city
terrain button sends 0x30 to pick a height-field image for the next NewCity.

### 11.16 SaveCity(path, flag) (iface slot 15, `0x00455E20`, 2611 bytes)

The package contents and the message order inside the write are in foundations.md section 3.7
(PreSave `0x26C63343`, unnamed `0x26C63346`, version records, root record, Save `0x26C63344`, PostSave
`0x26C63345`, adjacent-city regional records). This section is the app-side file handling around it.
Result starts false; it is the return value.

1. Scoped helpers for the whole call: wait cursor (`cRZAutoCursor` 15), input-queue flush
   (`cSC4AutoFlushInputMessageQueue` 1000), game pause (0, 1, 1, 1) and one more scoped object
   (`0x005F88A0`, constructed with 1, not identified).
2. No city open: return false.
3. Target path: `SC4CreateSavePathForCity` (`0x005DC790`) derives the city's save path from the city
   (city slots +0x30 and +0x14 feed it). renamed := target differs from the path argument (exact,
   case-sensitive byte comparison of the two strings, `0x00454BB0`). *[inferred: which local holds the
   target; the name "renamed" is this spec's]*
4. Remember the city's current file path (city slot +0x28), then set the city's file path to the target
   (city slot +0x2C) so the saved records already carry it.
5. Temp file: `cRZFile::MakeTempPathName`. Failure: skip to step 8 with result false (no package).
6. Package: a new `cSC4DBSegment` (0x1DC bytes), Init (slot +0x10), path := temp (slot +0x2C), open for
   writing (slot +0x18 with (0, 1)). Init or open failure: result false, go to step 8.
7. Messages and writes as in foundations.md section 3.7 steps 1-5 (the PreSave data carry the package,
   the path strings and flag != 0). After PostSave, result := true; then the package is closed (segment
   slot +0x28) and result := the close result; the package is shut down and released.
8. Unregister the city DB segment (section 11.5).
9. Only when result is true, replace the file (city slot +0x3C is called first, no arguments, unnamed):
   - If a file exists at target: backup := target with `_` appended
     (`City.sc4` -> `City.sc4_`). If a backup file already
     exists, remove it; failure -> result false, stop replacing. Rename target -> backup; failure ->
     result false, stop.
   - Rename temp -> target. Success: remove the backup if one was made. Failure: remove the temp file,
     rename backup -> target when a backup was made, result false.
   - If renamed and the file at the old path (the argument) exists, remove it (a renamed city leaves
     no stale file).
10. Remove the temp file when its path is non-empty (harmless when already renamed).
11. City file path (slot +0x2C) := target when result is true, else the path remembered in step 4.
12. Register the city DB segment again (section 11.5).
13. Result true: through the framework service (`0x3AE4BEA3`, IID `0x3AE4BEAB`) get the set of
    regional cities next to this one; for each one that is not this city (compared through slot +0x7C)
    call `SC4WriteRegionalCity` (`0x005DC2E0`) so its file gets this city's new regional record
    (foundations.md section 11.1).
14. Return result.

Worked example (rename): the city was opened from `...\City - Old.sc4` and renamed to `New`; target
is `...\City - New.sc4` (assuming the derived name pattern), renamed is true. No file at target, so
no backup; temp is renamed to `City - New.sc4`, then `City - Old.sc4` is deleted, and the city's path
becomes the new name. If the temp rename failed, the old file stays and the path reverts.

Save-path naming supplement to section 11.16 (AP14): the helper
`SC4CreateSavePathForCity(base, cityName, cityID, out)` (`0x005DC400`) forms
`City - ` + cityName + `.sc4`, sanitizes that filename, and resolves it under base.
It does not create a file. If the candidate exists, a nonzero cityID permits reuse when
`SC4ReadRegionalCity` succeeds and the existing regional city's slot +0x6C value is either
that cityID or zero. Otherwise it restores the original candidate and inserts ` (2)` before
the last dot, then ` (3)`, and so on, testing existence and permitted reuse again each time.
Suffixes do not accumulate. The counter is unsigned 32-bit and the format is ` (%u)`;
the increment before the first suffix is explicit at `0x005DC6CB`. There is no maximum retry
count or error returned for an unreadable occupied candidate: it advances to another name.
The three-argument overload (`0x005DC790`) resolves the selected region's directory/name
from the global app's region pointer and then uses the same naming helper. These calls
require the corresponding region objects; there is no null-region fallback in that overload.
The sanitizer (`0x005F9BF0`) first uses `MakeFileNameValid`, then collapses repeated ASCII
spaces to one and trims the result. The filename filter removes each of these nine bytes:
`<`, `>`, `:`, backslash, slash, double quote, vertical bar, `*`, `?`; it does not replace them
with underscores, reject reserved device names, or reject other control bytes
(`0x009180D4`–`0x00918102`). Before filtering, the platform helper round-trips the string through
the current system encoding (conversion argument -1); its direct-string mode skips that
conversion. Thus non-ASCII results depend on the system code page and its substitution rules,
not a Unicode filename whitelist (`0x0091919F`–`0x009191E0`). The filtered result is capped at
256 bytes. Only if truncation was needed, bytes are removed from the end until the encoding
validity check (encoding argument -2) accepts the result or it is empty
(`0x00919219`–`0x00919261`). No extension-preservation repair follows this truncation: `.sc4`
can itself be cut off. ASCII space collapsing and trimming follow this filter/cap, before
collision suffix selection. These are filename rules, not validation of the base directory.

## 12. Open (not specified here)

The internals of the font-style parser and resource factories called by section 14.2,
the scoped object `0x005F88A0`, city
vtable slots +0x1A4 (blocks saving), +0x3C, +0x30, +0x14, the message `0x26C63346` name, and the rest of
cSC4App's 322 functions (this spec covers about 57). Not covered here: cWinSC4App,
cSC4Preferences, cSC4AutoGamePause. The bootstrap COM directors now have a
[director/registry dispatch contract](app-directors.md), including method inventories,
class tables and lifecycle vectors; registered-service internals remain in their linked owner specs.
The distinct cSC4GameDataRegistry owner now has a
[first bounded contract](game-data-registry.md), including its per-body evidence and remaining unknowns;
it is not included in this cSC4App function count. Questions: docs/specs/questions/application.md (AP3 to AP11).

## 13. Preferences (`cSC4Preferences`, `+0x3B0`) *[compat]*

*Added 2026-10-22 (re-app).* `cSC4Preferences` (constructor `0x00449070`) is a 0x1734-byte block made by the app's
constructor (`GetPreferences`, slot 38).

### 13.1 The file

`SimCity 4.cfg` (`GetAppPreferencesFileName`, in the user data directory) is a database package with **one record**,
type `0xA9DD6E06`, group `0x49DD6E08`, instance 1, version **0x15**, holding the block as raw bytes.

- **`ReadPreferences`** (primary slot 6, `0x0044BC50`): when the file opens and the record has version 0x15, the block
  is read into a fresh preferences object and copied over the app's; any other version leaves the defaults.
- **`SavePreferences`** (iface slot 17, `0x0044BDB0`): on the live block, `+0x4A8` := 1, the 64 bytes of the language
  name `+0x4AC` are cleared and the IME byte `+0x4EF` := 1 *[quirk: so `-l` and `-IME:off` are never saved, and the
  live block loses its `-l` name too]*; then the old file is deleted and the record (`0x1734` bytes) written
  (`0x0044BDEE`–`0x0044BEBA`). Callers: Init's driver failure (§3.1) and the autosave timer of `OnIdle` (§7.1).

### 13.2 Fields used by the app

`+0` visual time mode (city-view.md §7.2), `+4` and `+0x69` sound, `+0x10` music, `+0x46C` difficulty (city-view.md
§6.4), `+0x6C` / `+0x26C` the last city and mayor names, `+0x470` (`-sim`), `+0x498` (`-RDD`), `+0x4A4` pause on
focus loss (slot 18 also sets the app's `+0x5D0`), `+0x4AC` language, `+0x4EF` IME, `+0xF09` edge scrolling, `+0xF0A`
expanded tooltips, `+0xF0C` software driver, `+0xF10`, `+0xF13` hardware driver wanted, `+0xF16` city detail,
`+0xF17` windowed (zero requests fullscreen), `+0xF1C`…`+0xF24` resolution (§8).
`+0x470` supplies simulator speed through section 13.3; the meaning of `+0x498` remains AP11.

### 13.3 Re-applying them *[compat]*

`DoPreferencesChanged` (`0x0044CE40`, message `0x231BBF91`, §6.2) applies focus-pause, language,
graphics driver/video mode, simulator speed, then command-line cheats as detailed below. `ApplyVideoPreferences`
(iface slot 19, `0x0044A1C0`) writes twelve video settings and posts `0xC9F775BB`;
`GetAutoVideoPreferences` (slot 20, `0x0044A330`) reads fourteen.

The video calls use the video-settings interface stored at whole-app +0x634, rather than
the separately exposed render-properties pointer +0x630. Their argument is a video preference
record; offsets below are relative to that record, not the beginning of cSC4Preferences.
Apply writes unsigned bytes widened to 32 bits; GetAuto stores only the low byte of each
setting result. Neither operation converts those bytes to booleans. Both use this order:

| Order | Setting ID | Record byte offset |
|---|---|---|
| 1 | 0 | +0x01 |
| 2 | 1 | +0x02 |
| 3 | 2 | +0x03 |
| 4 | 3 | +0x04 |
| 5 | 6 | +0x07 |
| 6 | 4 | +0x05 |
| 7 | 5 | +0x06 |
| 8 | 8 | +0x09 |
| 9 | 7 | +0x08 |
| 10 | 9 | +0x0A |
| 11 | 10 | +0x0C |
| 12 | 15 | +0x1C |

GetAuto additionally sets byte +0x0B to 1 after setting 10 is read. After the twelve reads it
reads setting 11: values 0, 1, 2, 3 set dwords +0x10/+0x14 to 800/600, 1024/768, 1280/1024,
1600/1200 respectively; any other unsigned value leaves those dwords untouched. Finally setting
12 maps 0 to dword +0x18 = 16, 1 to 32, and otherwise preserves the old value. Apply does not
write settings 11 or 12 and does not modify the argument record. It queues message `0xC9F775BB`
after all twelve writes when message server 2 exists; absent server skips notification only.
Instruction evidence: `0x0044A1C6`–`0x0044A29D`, `0x0044A345`–`0x0044A47D`.

#### Preference application order (AP14) *[compat]*

`DoPreferencesChanged(full)` (`0x0044CE40`) applies effects in the following order. The
argument gates language and graphics work only; it does not gate the first or last two effects.

1. Apply pause-on-focus-loss from preferences byte +0x4A4, normalized to false/true, through
   app interface slot 18. The focus/pause contract remains section 7.
2. When full is true, clear preferences byte +0x4EB. If the language manager exists, try the
   nonempty +0x4AC language name as an English name, then as a local name only when English-name
   lookup fails. A resolved but unavailable name goes directly to the fallback below. On an
   available ID, select it and store the ID at preferences +0x4A8. Fallback order is the installed
   language registry value (Deluxe key first, original SC4 key second), then the current system
   language; each must pass CanSwitchToLanguage before selection. The final native fallback is
   defective: it probes the same last ID up to 44 further times, rather than enumerating IDs;
   on a successful repeated probe it stores the probe ordinal (1–44) without selecting a new
   language. If every probe fails, the old +0x4A8 survives. Empty names start at the fallback.
   Evidence: `0x0044CE8F`–`0x0044CF05`, `0x0044D034`–`0x0044D0C1`; registry helper `0x005FAEF0`.
3. Still only when full, if the graphics service exists, obtain current width/height/depth.
   Force preferences byte +0xF17 (windowed) to 1 if current depth is neither 16 nor 32, or if
   either current unsigned dimension is smaller than requested +0xF1C/+0xF20. Never clear
   +0xF17 here. Request the preferred dimensions, with preferred depth +0xF24 except that zero
   depth or fullscreen requests retain the current depth. Then enable fullscreen exactly when
   +0xF17 is zero. These two requests occur even when the sizes already agree; their return
   values do not abort later work (`0x0044CF2D`–`0x0044CFF2`).
4. Still only when full, query graphics interface `0xC47B747C`. If available, app driver override +0x65C selects the
   driver and updates preference byte +0xF13: software `0x7ACA35C6` → 0, DirectX `0xBADB6906`
   → 1, another nonzero ID → 255. A zero override instead selects software for +0xF13=0,
   DirectX for 1, and leaves the driver alone for every other byte. Release this interface
   after use (`0x0044D00A`–`0x0044D0FD`, `0x0044D189`–`0x0044D1B0`).
5. Independently of full, if both city and its simulator exist, set simulator speed using
   preferences float +0x470. Multiply the binary32 input by binary32 constant `0x3DA237C3`
   (exactly 0.079207919538021087646484375), convert toward zero to a signed integer, clamp
   to 0–7, and index `[5, 10, 15, 20, 30, 45, 60, 80]`. The multiply uses x87 precision with
   no intervening binary32 store; do not round the product to binary32 before conversion
   (`0x0044D130`–`0x0044D1C8`). Conversion uses `_ftol2`'s signed 64-bit result; interpret
   its low 32 bits as signed before the clamp. Out-of-range 64-bit/NaN conversion produces
   the integer-indefinite value whose low word is zero. Values outside signed 32-bit range
   but within signed 64-bit range therefore wrap before clamping, rather than saturating.
   Evidence: `0x009EEF0D`–`0x009EEF78`. Ordinary examples: -1 → 5, 50 → 20, 100 → 80.
6. Independently of full and city presence, obtain the cheat-code manager through app slot
   33. When nonnull, copy the framework command line, and process named `x` switches using
   named-switch lookup with unquoting enabled (command-line.md section 4.1). This accepts
   `-x:GP`, for example; bare `-x` is not such a switch. Supply each selected value to
   `DoDefaultCheatCodeProcessing(value, outCheatID)` through manager +0x58; its result does
   not stop subsequent requests. After each request, erase from the copy the first argument
   whose first case-sensitive occurrence of lowercase `x` is at byte index 1, then look for
   another named `x` switch. This erasure does not independently require a switch prefix or
   colon. An earlier unrelated `ax...` argument can therefore be erased first and cause the
   same cheat value to be processed twice. A matching uppercase `-X:GP` with no argument
   eligible for erasure can repeat without progress. The framework command line is never
   changed; another call makes a fresh copy and can run the same cheats again. Evidence:
   `0x0044D1CB`–`0x0044D310`, copy constructor `0x00914A30`, case-sensitive Find
   `0x0090F0DB`, cheat-manager table `0x00ACFCD0` +0x58 → `0x00921621`.

This call does not directly invoke configuration-rule evaluation, ApplyVideoPreferences,
SetupFontSet or SetupWinCtrlMgr and does not save the preference file. Missing language/graphics
services skip their own effects; they do not suppress simulator speed or cheat processing.

## 14. Plugins, resources and files *[compat]*

*Added 2026-10-22 (re-app).*

### 14.1 Code plugins

`AddCodePlugins(dir)` (`0x00451260`) loads every `*.dll` of the directory through GZCOM. A DLL with a `<name>.ini`
beside it whose `[Admin] Enabled` is case-insensitively `false` (`cRZString::IsEqual` with case-sensitive flag 0)
or exactly `0` is skipped
(`0x00451740`–`0x004518CB`). `AddDynamicLibraryByPath` (iface slot 68, `0x0044EAF0`) and `AddDynamicLibraryByName`
(slot 67, `0x00451A10`) apply the same test to one library. `PreFrameWorkInit` runs it for the plugin directories
(§3); a change notification from a directory watcher runs it again for that directory (§6.1).

The plugin comparison above operates on the INI reader's returned value: no extra trimming,
numeric conversion or truth-word parsing is performed. Thus `FALSE`, `False` and `0` disable;
`00`, `no` and an absent key do not. The case-insensitive branch lowercases both strings
(`0x0090ECCE`, `0x0090ECD6`); the separate `0` comparison is exact. By-name loading constructs
a path using `RZ::ADir`, then invokes interface slot 68 (`0x00451BD3`), forwarding the bypass
flag. It therefore resolves the path before the adjacent-INI test; it does not ask a foreign
resolver to decide Enabled. By-path loading returns false before reading the INI if GZCOM is
absent or the DLL does not exist. A nonzero bypass flag skips the Enabled test
(`0x0044EB27`–`0x0044EB2B`).

### 14.2 Resources *[compat]*

#### Resource registry syntax and malformed entries (AP13)

`LoadRegistry` (`0x004584A0`) reads `Sys\Resources.ini`. The packed options
**`packedexemplars`** and **`packedtextures`** are command-line switches,
not INI section names. Their values are lowercased before exact `on`/`off` comparisons; other
values leave the existing flags alone (`0x004585F2`–`0x004586EA`). `[DBSegments]` uses hexadecimal
segment IDs as entry names and `hexadecimal-class-ID;path` as values. The key and class token
are passed to `RZAtoUint32HexA`; the semicolon split is at `0x00456C1D`, class conversion at
`0x00456CA1`, and segment creation requests interface `0x656B8F0C`. Do not treat the entry name
as the class ID or parse either ID as decimal. `%DataDirectory%` and `%Language%` are the
case-sensitive substitution spellings. The directory-files section is registered separately
under `CN_DBSDF`, rather than passed through the same class/path callback.

Hexadecimal conversion skips leading character-class whitespace, accepts optional `0x`/`0X`,
and consumes at most eight hexadecimal digits. The first non-hexadecimal byte ends the
number; trailing bytes and extra digits are ignored. No digits means zero; a leading sign
is not accepted. Thus `12junk` means `0x12`, `123456789` means `0x12345678`, and `-1` means
zero (`0x0090FCEF`–`0x0090FD8F`). There is no callback-level rejection of zero segment/class
IDs. Only the first semicolon separates class and path; later semicolons belong to the path.
With no semicolon, the class token is empty and the entire value remains the path
(`SplitToken`, `0x0090E98C`). Such an entry attempts class zero; failure to create the
requested segment skips that entry, without rolling back previously accepted entries.
Initialization/path/open/registration failures similarly do not reject the whole INI.
`[DBSegmentDirectoryFiles]` stores its raw entry names and values under `CN_DBSDF`; the
app does not apply the class/path parser to that section.

Within `[DBSegments]`, `BEGIN_GROUP=PACKED_EXEMPLARS`, `NON_PACKED_EXEMPLARS`, `PACKED_TEXTURES`
or `NON_PACKED_TEXTURES` enables a corresponding inclusion condition: a packed group requires
its packed flag, a non-packed group requires its absence. These group names and the two marker
keys are case-sensitive. Groups are section entries, not extra INI sections. Conditions combine
while their bits are set. The app keeps a stack of group names, but only one bit per recognized
name: `END_GROUP` ignores its value and clears the bit for the most recently begun name.
Duplicate nesting therefore loses the outer condition at the first end. Unknown group names
still push/pop but impose no condition. Unclosed groups continue gating later entries; unmatched
`END_GROUP` accesses an empty stack unchecked, so there is no native safe-ignore/error-return
contract for that invalid input (`0x00458100`–`0x00458480`, especially `0x00458309`–`0x00458345`).
Missing Resources.ini and failed enumerations are not LoadRegistry failures: it returns true
(`0x00458784`–`0x004587AD`).

#### Database sources and order (AP13)

`SetupResources` (`0x004572B0`) submits sources in the following order, then registers
the factories below. Slots identify the directory contracts in section 9; do not infer paths
from the misleading export names.

| Order | Source | Selection / segment class |
|---|---|---|
| 1 | Registry `CN_DB` entries | Enumerate with CreateDBSegmentCallback; entry supplies class and segment ID |
| 2 | Slot 53, language index -1 | Nonrecursive `*.dat`, packed-file class `0x086D4A89` |
| 3 | Slot 53, language index 0 | Nonrecursive `*.dat`, same class |
| 4 | Slot 57 | Whole directory, directory-segment class `0x090F8619` |
| 5 | Slot 55 | Recursive `*` files with the filter below, packed-file class |
| 6 | Slot 55 | Whole directory, directory-segment class |
| 7 | Slot 59 / interface +0xEC | Recursive `*` files with the same filter, packed-file class |
| 8 | Slot 59 / interface +0xEC | Whole directory, directory-segment class |

The last directory segment is explicit at `0x00457DCF`–`0x00457E9C`; it is not replaced by
the recursive scan. The app attempts to create each missing recursive root, ignores creation's
result, and still attempts enumeration. Nonrecursive scans do not probe signatures before
opening. Recursive scans exclude extension `.dat`, case-insensitively, before testing other
files with `IsDatabaseFile(path, 1)`; only return -1 rejects a tested file. In particular `.dll`
is not the extension exclusion (`0x00457A42`–`0x00457AB2`, repeated at `0x00457CC1`–`0x00457D3C`;
literal `.dat` at `0x00A870D8`). This is the native filter even for a valid DBPF with a `.dat`
extension under a recursive root.

Enumeration preserves Windows FindFirst/FindNext order: there is no lexical sort. Recursive
enumeration emits the current directory's files first, then visits its enumerated child
directories depth first, including files at the supplied root. Paths are prefixed with their
directory (`cRZDirectory`, `0x0091B9C7`, `0x0091BD88`). The app does not deduplicate identical
roots or files between these passes. Registration order is a submission contract; lookup
precedence inside the foreign resource manager is not redefined here.

For scan-created segments, successful initialization and path assignment precede `Open(1,0)`
and resource-manager registration (+0x68). An initialization failure releases the local
segment; path/open/registration failure shuts it down and releases it. Successful registration
releases the local reference without shutting the registered segment down. One failed source
does not cancel later sources. This assumes the standard segment factories and resource service
exist: SetupResources does not provide a safe null-factory/service fallback.

#### Resource factories (AP13)

SetupResources registers the following complete type-to-factory-class list, in this order.
Every registration supplies a third argument of zero; failed individual registration results
are not checked. The final success test is resource-manager slot +0x84 >= 3, unsigned
(`0x00457EBF`–`0x00458002`). Factory IDs are identifiers, not implementation algorithms.

| Resource type | Factory class |
|---|---|
| `0xCA63E2A3` | `0x0A63DF8C` |
| `0x2026960B` | `0x00436EB4` |
| `0x686AA4B0` | `0xC8696797` |
| `0x856DDBAC` | `0xA2FFB5D3` |
| `0x7AB50E44` | `0x3AB50E2A` |
| `0x7AB50E45` | `0x3AB50E2A` |
| `0x29A5D1EC` | `0x69B6F01C` |
| `0x09ADCD75` | `0x69B6F01C` |
| `0x5AD0E817` | `0xFAD0F0B6` |
| `0x6534284A` | `0x453429B3` |
| `0x05342861` | `0x053429C8` |
| `0xA83479D3` | `0xA83479EA` |
| `0x296678F7` | `0x496678FE` |
| `0xEA5118B0` | `0xEA5118B5` |
| `0xA2E3D533` | `0x42E411C2` |

#### Font sources and language filtering (AP13)

`SetupFontSet` (`0x00453A30`) does nothing if the font service is absent. Otherwise it
adds all fonts from the slot-55 directory, then from `Fonts` under the stored data-directory
base. It performs the language-specific removal below after both additions; languages outside
the table do not perform this removal. Names are enumerated once, at most 64 records, and
lowercased before case-sensitive substring matching. Every matching substring requests removal
of that full font name, so one name can produce repeated removal requests. Font-service +0x44
removes the name's registration and cached fonts, releasing cached references; it does not add
an alias (`0x00894A3A`–`0x00894ADE`). The enumeration snapshot remains valid while removals run
(`0x00893E5B`–`0x00893ECA`).

All five languages remove names containing `arial mt`, `arta`, `courier ld`, or `stoclet`,
plus the substrings in their row. Spaces inside a substring are significant.

| Language ID | Additional removal substrings | Conditional additional removal |
|---|---|---|
| `0x0F` | `mhei gb ` (trailing space), `mhei b5`, `hy round gothic`, `cordiaupc` | None |
| `0x11` | `hg-gothic`, `mhei b5`, `hy round gothic`, `cordiaupc` | None |
| `0x12` | `hg-gothic`, `mhei gb`, `hy round gothic`, `cordiaupc` | None |
| `0x13` | `hg-gothic`, `mhei gb`, `mhei b5`, `hy round gothic` | `cordiaupc` only when system-directory `micross.ttf` exists and font-service +0x38 accepts it |
| `0x14` | `hg-gothic`, `mhei gb`, `mhei b5`, `cordiaupc` | `hy round gothic` only when system-directory `Gulim.ttc` exists and font-service +0x38 accepts it |

The conditional font is offered to the font service before enumerating names. These are
substring exclusions from the loaded set, not a language-to-font alias table. Evidence:
`0x00453A30`–`0x00454352`; path additions `0x00453AA3`/`0x00453B4E`, enumeration/removal
`0x004541A5`–`0x00454304`.

#### Window-control defaults and FontStyle.ini (AP13)

`SetupWinCtrlMgr` (`0x0044D810`) applies colours, font defaults/styles, control pictures,
cursors, then captions. The following colour table is in call order; triples are red,
green, blue integers supplied to window-control-manager +0x84, not packed ARGB words
(`0x0044D835`–`0x0044DA23`). Absent window-control manager skips this colour block.

| Control colour ID | RGB |
|---|---|
| `0x08963F6C` | 255, 0, 255 |
| `0x68963F6F` | 189, 188, 188 |
| `0xC8963F7A` | 255, 255, 255 |
| `0x68963F7D` | 0, 0, 0 |
| `0x28963FC5` | 128, 0, 0 |
| `0xA8963F82` | 0, 128, 0 |
| `0x48963F84` | 128, 128, 64 |
| `0x68963F8A` | 0, 0, 128 |
| `0xA8963F8C` | 128, 0, 128 |
| `0x68963F8F` | 0, 128, 128 |
| `0x08963F91` | 128, 128, 128 |
| `0x28963F93` | 192, 192, 192 |
| `0xE8963F95` | 255, 0, 0 |
| `0x28963F98` | 0, 255, 0 |
| `0xE8963F9A` | 255, 255, 0 |
| `0xC8963FA2` | 0, 0, 255 |
| `0x08963FA5` | 0, 255, 255 |
| `0x28963FA8` | 255, 255, 255 |
| `0xBB5B2EC6` | 124, 144, 255 |

When the font service exists, font setup first supplies 3 to its setting slot +0x1C.
Presence of service `0xEB2EA3CB` / interface `0x4B2EA3B3` permits creation of the optional
font helper class `0x8B3255F9` / interface `0xA8BC2A75`. On successful creation it is supplied
to font-service +0x18 and locally released; absence does not suppress subsequent font setup.
SetupFontSet is called before requesting the following two `Arta` fonts through
font-service +0x2C. Its argument tuple is `(name, 0, size, 2, 1)`; font construction and
rasterization remain font-service responsibilities. Each nonnull result receives the colour
returned by service `0xC416025C` / interface `0x0073283C`, slot +0x88 with arguments
`(0,0,0,0x08000000)`, then is registered under the listed IDs and locally released.

| Size | Window-control font IDs, in order |
|---|---|
| 12 | `0x68963C4C`, `0xC8963C5C`, `0x68963C54`, `0xA2CAE9DF` |
| 20 | `0x28963C70` |

A null font result skips its bindings only. Font-service absence skips this font and INI
block; it does not skip pictures/cursors/captions. Window-control-manager absence is not a
general safe no-op: successful font creation and later captions use that manager unchecked.
The colour service is likewise assumed present when a font needs its colour.
Evidence: `0x0044DA29`–`0x0044DC7B`.

FontStyle.ini is selected from slot 55 first, then slot 53 with language index 0, using the
first existing candidate. If neither exists, request a temporary path and export resource key
`(type=0, group=0x4A87BFE8, instance=0x2A87BFFC)` into it. Export result -1 clears the path;
an empty path skips INI enumeration. Temporary-path cleanup is attempted afterward; when
export failed and cleared the path, Remove receives the empty path, not a saved copy of the
original temporary name. Existing FontStyle.ini files are not removed (`0x0044DC80`–`0x0044DE92`).

Enumerate `Font Aliases` first. Each alias is forwarded to font-service +0x50
as `(entry value, entry name)`, with no app-side normalization (`0x004499D0`). Each style
value is passed to `CreateFontStyleFromSpecification` (`0x005C11B0`), which returns a font
and numeric style ID. A null result skips that entry. Otherwise register the font under that
ID with window-control-manager +0x18, attach a copy of the entry name using +0x9C, and, when
the auxiliary name table exists, add `(entry name, ID)` through its +0x0C. Release the local
font afterward. The entry name is not itself parsed as a hexadecimal style ID
(`0x0044D4D0`–`0x0044D563`). These are the app's parser inputs/outputs; the font specification
grammar belongs to the font parser, rather than the Resources.ini class/path grammar.

The `Font Styles` enumeration requires a nonnull object from service `0x5A356E15` /
interface `0xFA3562FA`, slot +0x20, and successful creation of name-table class `0xBA2E7954` /
interface `0x1A2BDEBB`. The app retains that object, supplies the new name table to its +0x34,
enumerates styles, then releases its name-table reference, clears the temporary global table
pointer, and releases the object. A null object or failed table creation skips styles, after
aliases have already been submitted. The service itself is assumed present (unchecked
dereference at `0x0044DDD9`). No previous table is restored (`0x0044DDB2`–`0x0044DE6E`).

Control pictures all use resource type `0x856DDBAC`, group `0x46A006B0`. Load each source in
table order, register nonnull pictures at window-control-manager +0x20 under each listed
control ID, and release the local resource before the next source. A failed load skips only
that source's bindings (`0x0044DEB7`–`0x0044E2A6`). Absent window-control manager skips pictures.

| Source instance | Control picture IDs |
|---|---|
| `0x46A006A2` | `0xA8963EB8`, `0xE8963EF1` |
| `0x144161EA` | `0x68963EBE`, `0xA8963EF3` |
| `0x0000000A` | `0xC8963EF6` |
| `0x82B99D9D` | `0x68963ED6` |
| `0x62B99D31` | `0x68963ED8` |
| `0x42E55FD4` | `0x48963EEE` |
| `0xE78FFC90` | `0xE8963EDF` |
| `0xC2B66DAA` | `0xE8963EE5` |
| `0x46A006A4` | `0xC8963ED3` |
| `0x46A006A6` | `0xE8963EC7` |
| `0x46A006A7` | `0x88963EC1` |
| `0x46A006A8` | `0xA8963EC4` |
| `0x46A006A5` | `0xA8963EEA` |
| `0xE2B14588` | `0xE2B14588` |
| `0x62B19CE9` | `0xE2B14589` |

#### Cursors and button captions (AP13)

Framework options `CustomCursors` and `cursors` are lowercased before comparison; they are
not Resources.ini sections. `CustomCursors=off` or `disabled` excludes only cursor instances
`1`, `3`, `15`–`18` (decimal), `0x43401C71`, `0x68024B0E`, `0x88024B02`, `0xC8024B17`,
`0xE8024B13`. It does not disable all cursors. Other values preserve inclusion of these IDs.

| Lowercased `cursors` value | Write to video setting 15 |
|---|---|
| `off`, `disabled` | 0 |
| `bw` | 1 |
| `color16` | 2 |
| `color`, `color256` | 3 |
| Other / absent | No write |

The comparison intended to select 4 uses the mixed-case literal `fullColor` after lowercasing,
so no ordinary spelling of that option selects 4. Preserve the previous setting for `fullColor`
as for other unknown values (`0x0044E402`–`0x0044E4F4`). Independently of whether an option
wrote setting 15, read that setting to select the resource group: 0→0, 1→1, 2→4, 3→8, 4→32;
other unsigned values map to `value + 0x4A708EFB`, wrapping to 32 bits
(`0x0044E4F7`–`0x0044E545`). Enumerate resource type `0xAA5C3144` in resource-manager order,
retain only that group and allowed instances, and construct a `cSLCursor` from each retained
key. Register each nonnull cursor by its instance ID through cursor-manager +0x18; a null
cursor result skips that key. No alternate-group fallback is attempted. The cursor manager
comes from window-manager +0x80 (`0x0044E549`–`0x0044E64F`).

Finally load localized strings with group `0x6A231EAA` and supply them to window-control-manager
+0x90 in the order below. Load results are not checked. The last caption is a literal UTF-8
ellipsis, not a localized string key (`0x0044E657`–`0x0044E8C7`).

| Caption control ID | String instance / literal |
|---|---|
| `0x5301814C` | `0x0A56B09E` |
| `0x53018149` | `0x8A56B0AF` |
| `0x5301814A` | `0x4A56B0B4` |
| `0x53018147` | `0x6A56B0BB` |
| `0x53018146` | `0x2A56B0BE` |
| `0x5301814B` | `0xAA56B0C3` |
| `0x53018148` | `0xCA56B0C8` |
| `0x5377BE31` | `…` (bytes `E2 80 A6`) |

### 14.3 Files

`GetAppIniFilePath(out)` (slot 29, `0x0044D570`) concatenates the stored data-directory
base (the same path returned by slot 51) and `GetAppIniFileName`. If that file exists it
assigns the candidate to out. Otherwise it assigns the stored slot-58 base plus the INI
filename, with no existence check on that fallback (`0x0044D615`–`0x0044D662`). The operation
uses string concatenation, not a filesystem search: no current-directory, CD, plugin or
environment search and no separator insertion. Normal stored bases already end in a separator.
Slot 31 similarly concatenates the slot-58 base and `GetAppPreferencesFileName` directly,
without even the candidate existence test (`0x0044C0D9`–`0x0044C104`). Both assign out, replacing
its previous contents; neither creates the file or updates directory caches.

- The ini file is `SimCity 4.ini` (`GetAppIniFileName`), looked up by `GetAppIniFilePath`;
  `FindAndPossiblyFixIniFiles` (`0x0044FE70`, §4.2) reads `[Directories] Data` and `PlugIn` from it and keeps them
  when the folders exist.
- The application's plugin directories are `Plugins` with the language name (`English` by default) inside the data
  and CD directories (§9); `DoLoadCityUI` produces literal `SavedCity` plus the global suffix in section 11.10,
  replacing the application-directory string initially fetched; it does not resolve a `SavedCity` subfolder.
  `[Settings] Terrain Load/Save Location` of the ini file remembers the height-map dialog's
  folder (§11.15).

## 15. Message pumps (iface slots 4 and 5) *[compat]*

*Added 2026-10-22 (re-app).* `RunMessageServerPump(min, max, ms)` (`0x004499F0`, message server `0x00B43CC8`) and
`RunMessageServer2Pump` (`0x00449A70`, message server 2) deliver queued messages one at a time (server slot 10 tells
whether one waits, slot 9 with 0 delivers it) until the queue is empty, `max` messages were delivered, or more than
`min` were and the elapsed time (a timer of unit 4 started on entry, *[inferred: milliseconds]*) exceeds `ms`.
Without the server nothing happens. The city life cycle uses (100, 500, 5000) (§11).

## 16. Conformance vectors for owner closure *[compat]*

These are spec-level vectors for the implementer; no engine files are supplied by this RE lane.
Paths below are literal Windows strings. Foreign primitive outcomes are fixture inputs.

| ID | Inputs / fixture | Expected observable result |
|---|---|---|
| AP12-1 | Existing DLL; adjacent INI returns Enabled values `false`, `False`, `FALSE`, `0` in four runs | No GZCOM library add in any run |
| AP12-2 | Same, values `00`, `no`, empty, missing key | Enabled filter permits add; no conversion of `00` to zero |
| AP12-3 | Disabled adjacent INI, bypass flag nonzero | Library add attempted; missing DLL still fails before INI lookup |
| AP12-4 | By-name request resolves to an existing path with Enabled=`FALSE` beside it | Slot 68 receives that path; its Enabled filter skips add |
| AP14-getters | Distinct u32 sentinels at whole-app +0x44, +0x4C, +0x314, +0x318, +0x54, +0x5C | Slots 25, 26, 32, 33, 34, 37 return respective sentinels with no AddRef |
| AP13-ini | Slot 51 base `C:\Game\`, slot 58 base `C:\User\`, app name `SimCity 4` | Existing `C:\Game\SimCity 4.ini` wins; absent candidate returns `C:\User\SimCity 4.ini` even if fallback absent |
| AP14-cfg | Same bases and name, out initially `old` | Slot 31 assigns `C:\User\SimCity 4.cfg`; no existence probe |
| AP14-video-write | Record bytes at offsets 1..10 equal their offsets, +0x0C=200, +0x1C=255 | Settings written in order 0,1,2,3,6,4,5,8,7,9,10,15 with corresponding unsigned byte values; one message after writes |
| AP14-video-read | Setting 0 returns 0x123, setting 11=2, setting 12=1 | Record +1=0x23, +0x0B=1, resolution 1280×1024, depth 32 |
| AP14-video-preserve | Resolution/depth seeded; setting 11=4, setting 12=2 | All three seeded dwords preserved; ordinary twelve byte reads still occur |
| AP14-name | Base `C:\Region\`, city `Oak`, ID=0; candidate and ` (2)` occupied | Result `C:\Region\City - Oak (3).sc4`; no file created |
| AP14-name-reuse | Candidate occupied; requested ID=7; readable existing city ID=7, then 0 in separate runs | Unsuffixed candidate reused in both runs; ID=8 instead causes ` (2)` candidate |
| AP15-directories | Eight distinct stored path fixtures for slots 51–58; language index -1 for 53/54 | Each copies its own table source; slot 56 does not raise Unsupported or alias slot 58 |
| AP15-temp-write | City open; temp segment initialization fails; read-open later fails | Temporary writer returns true and releases segment; outer routine still partial-closes, removes old path, attempts read, shuts down/releases read segment and removes temp; no NewCity |
| AP15-read-init | Read-segment initialization false; read-open false | Path and read-open still attempted; shutdown and release, no close; no template pointer installed |
| AP15-views | Source view absent; destination view present; rebuilding fails | Destination zoom=0 then rotation=0; missing destination instead makes neither setter call |
| AP15-invalid | No city, temporary writer successfully opens segment | Native city dereference is invalid; not a native false/no-op vector |
| AP13-hex | Keys/classes `12junk`, `123456789`, `-1`, ` 0Xff` | Numeric results 0x12, 0x12345678, 0, 0xFF; no whole-file rejection |
| AP13-separator | Value `086D4A89;a;b.dat`; separately value `a.dat` without semicolon | First class 0x086D4A89/path `a;b.dat`; second class 0/path entire value; failed class creation skips entry |
| AP13-groups | Packed exemplars off; begin PACKED_EXEMPLARS twice, end with arbitrary value, then ordinary entry | First end clears shared condition bit despite outer group; ordinary entry no longer excluded by that condition |
| AP13-unknown-group | Begin unknown name, ordinary entry, end arbitrary value | Unknown group imposes no condition; end pops that name |
| AP13-bad-end | END_GROUP without any BEGIN_GROUP | Invalid unchecked empty-stack access; no native safe-ignore contract |
| AP13-sources | Distinct roots and one accepted file per file pass | Registry; slot53(-1) files; slot53(0) files; slot57 directory; slot55 files then directory; slot59 files then directory; factories last |
| AP13-recursive-filter | Recursive root contains valid DBPF `a.DAT`, valid `b.dll`, invalid `c.txt` returning -1 | Skip a before signature probe; probe and attempt b; reject c; directory segment still registered afterward |
| AP13-order | Enumeration emits root files b,a; child directories z,y with files z1,y1 | File submissions b,a,z1,y1; no lexical sorting or cross-pass deduplication |
| AP13-segment-failure | First source Open false, second succeeds | First shutdown/release; second registration still attempted; success releases local reference without shutdown |
| AP13-factories | Valid resource manager records registrations, returns count 2 then 3 in separate runs | Fifteen ordered table registrations with third argument zero; false then true final result |
| AP13-font-filter | Language 0x0F; enumerated names `MHei GB`, `MHei GB Regular`, `Arta` | Bare MHei GB survives trailing-space substring; other two removed; source directories added before removals |
| AP13-font-limit | More than 64 registered names; matching name is only record 65 | App requests at most 64 and does not remove that name through this enumeration |
| AP13-font-conditional | Language 0x13; micross.ttf exists but font-service +0x38 fails | No conditional cordiaupc removal; common and other language exclusions still apply |
| AP13-font-defaults | Both Arta requests succeed | Request tuples (Arta,0,12,2,1), (Arta,0,20,2,1); bind four IDs then one, as table; local references released |
| AP13-font-ini | Neither FontStyle.ini candidate exists; temp export returns -1 | Export exact type/group/instance key; skip aliases/styles on empty path; cleanup Remove receives empty path |
| AP13-font-styles | Alias A=B; style parser returns null; next style returns font and ID 7 | Alias call (B,A); malformed style omitted; valid style registered as 7, named by its entry key, local font released |
| AP13-font-style-gate | Alias section present; service +0x20 returns null | Aliases submitted, Font Styles not enumerated |
| AP13-colours | Window-control manager present | Nineteen RGB writes in table order, including 0x68963F6F=(189,188,188) and final 0xBB5B2EC6=(124,144,255) |
| AP13-pictures | First picture load succeeds, second fails, third succeeds | First binds A8963EB8/E8963EF1; second binds neither; third binds C8963EF6; later sources still attempted |
| AP13-cursor-option | Existing setting15=4; option fullColor | No setting write; selected resource group32 from existing value |
| AP13-cursor-map | Options OFF, BW, COLOR16, COLOR256 in four runs | Setting15=0,1,2,3; resource groups0,1,4,8 respectively |
| AP13-cursor-filter | CustomCursors=disabled; selected group contains instances1 and2 | Skip1; construct/register2; cursor system remains active |
| AP13-cursor-custom | Setting15=5 and no recognized option | Select group0x4A708F00 with unsigned addition; no fallback |
| AP13-captions | All managers present | Seven localized captions in table order, followed by literal UTF-8 ellipsis for 0x5377BE31 |
| AP14-name-filter | ASCII city name ` A<>:*?  B `; empty destination | Filename `City - A B .sc4`; prohibited bytes deleted and ASCII spaces collapsed; space before extension survives |
| AP14-name-cap | ASCII city name long enough that full candidate exceeds256 bytes | Filtered filename capped256 bytes before suffix selection; no repair of a truncated .sc4 extension |
| AP14-savedcity | Nonempty application directory; global suffix .sc4 | RequestLoadCity stub uses literal SavedCity.sc4, replacing the initially fetched directory |
| AP14-preferences-partial | full=false, pause byte2, city/simulator present, speed50, command line has -x:GP | Pause true; no language/video work; simulator speed20; GP processed last from a command-line copy |
| AP14-preferences-window | full=true, current800x600x16, requested1024x768x32, windowed0 | Set windowed1; request1024x768x32 then fullscreen=false; driver selection follows |
| AP14-preferences-driver | full=true, driver override0x12345678, graphics interface present | Preference hardware byte255; select exactly that override; release queried interface |
| AP14-preferences-speed | Binary32 speed input bits0xD1000000 (-34359738368) | Exact product converts to -2721563392; low32 signed1573403904 clamps7, speed80 rather than5 |
| AP14-preferences-language | Named language unavailable, registry/system/repeated probes all unavailable | No language switch; preference language ID survives; full-only byte4EB cleared |
| AP14-cheat-copy | Original arguments p,-x:GP,-x:ForceIdle; call twice | Each call processes GP then ForceIdle; original argument list remains unchanged |
| AP14-cheat-erasure | Copied arguments p,axjunk,-x:GP | GP processed twice: unrelated axjunk erased first, then matching switch |
| AP14-cheat-case | Copied arguments p,-X:GP only | Named lookup matches but lowercase-x erasure cannot progress; native loop may repeat; no false claim of safe original-switch removal |
