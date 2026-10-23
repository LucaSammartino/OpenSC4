# Spec inventory M6+: the non-simulation subsystems a standalone FreeCity4 must replace (ranked)

Status: **planning document**, clean-room input, in the style of [INVENTORY.md](INVENTORY.md). It lists what the
original program contains besides the simulation: the framework, the renderer, the user interface, audio, the region
view, the launcher and the video layer. For each part it gives the classes, their class IDs (CLSIDs), a code size,
what the part reads from the simulation and from the shipped data, which existing specs already cover it, and a
proposed spec order. It ends with the smallest milestone sequence to a standalone executable (§10). It holds no
behaviour beyond short identifying facts; addresses are citations. Written 2026-10-15 by the re-inventory6 lane.

Tags as elsewhere: *[compat]* observable and must match; *[choice]* a FreeCity4 decision; *[inferred]* consistent
with the evidence but not proven; *[unsettled]* open.

---

## 0. Sources and how sizes are measured

- **Code size** is the summed function body bytes from the analysis index of the Windows exe (35,637 functions,
  6.36 MB of code in all). A class's figure counts its named functions plus every unnamed function that lies
  between two functions of the same class (same object file). 2.49 MB of code is in unnamed functions; about
  1.7 MB of that cannot be attributed this way and is reported by **address range** where a range is clearly one
  library (§0.1). The figures are a guide to the reading effort, not to the size of a port.
- **Class IDs** come from three places: the exe's built-in ID→name table (`re/class_id_table.tsv`); the class
  registrations made by every COM director's constructor (a director registers `(class ID, factory)` pairs; read
  for every director, §2.2); and `GetGZCLSID` bodies (`mov eax, id; ret`).
- **Strings** were cross-referenced to their using functions. **RTTI** does not help here: the exe carries RTTI
  only for 60 STLport classes; no game or framework class has a type descriptor. **Vtables** are cited from
  [docs/abi/vtables.tsv](../abi/vtables.tsv) (806 vtables).
- **Shipped data** was surveyed by resource type and group over the install's packages (`SimCity_1.dat` …
  `SimCity_5.dat`, `EP1.dat`, `Intro.dat`, `Sound.dat`, `English\SimCityLocale.DAT`, `Sku_Data`). Only counts,
  record magics and exemplar names were read; nothing is copied here.

### 0.1 Unnamed libraries identified by address range

| Range | Size | What it is | Evidence |
|---|---|---|---|
| `0x0083A000`–`0x00864000` | 157 KB, 666 functions | 2D pixel blitters (MMX: unpack, multiply, pack), template-expanded per pixel format | no strings at all; instruction mix dominated by MMX packing ops; reached only through function tables |
| `0x00864000`–`0x00879000` | 74 KB | EA's statically linked sound library (SNDSYS, AEMS module banks, mixer, filters) | fragments named `SNDSYSI_100hzserver`, `_SNDAEMSI_addsystem`, `_MIXI_interleave`, `SFILTER_timestretchinit`; creates its own thread (`CreateThread` at `0x00869B30`) |
| `0x0087C4C0`–`0x00880000` | 16 KB | OpenGL graphics driver | `OPENGL32.dll` imports (67 functions); window class string `GDriverClass--OpenGL` used from `0x0087ECD0` |
| `0x00881970`–`0x00890000` | 50 KB | DirectX 7 graphics driver (`nSGLDX7::cGDriver`, texture and primitive managers) | `DDRAW.DLL` loaded by name (`0x00888600`), `Gonzo: D3D Driver`, `GDriverClass--DirectX` |
| `0x008D8000`–`0x008EB000` | 72 KB | software graphics driver and the `RZDraw*` 2D primitives | `GDriverClass--Software`, `Software stats` (`0x008E13D0`); the software driver ID is used at `0x008DA491` |
| `0x0080F760`–`0x00812000` | ≈ 11 KB | the intro video player (`cSC4MadCowPlayerWin`, EA "MAD" video) | the `MADk` frame tag compared at `0x00810A2F` and `0x008111B0` |
| `lib` (scattered) | 191 KB | third-party: Lua 5, libpng, zlib, the C runtime | Function-ID names (`_png_*`, `_inflate_*`, `_lua*`) |

---

## 1. Overview

Code size in KB (§0). "Covered" names specs that already describe part of the subsystem. "Standalone" is the
first milestone of §10 that needs the part (S0 headless … S6 full).

| Part | Code | Main classes | Covered by | Standalone |
|---|---|---|---|---|
| F1 application, command line, ini, launcher | 87 | `cSC4App`, `cWinSC4App`, `cRZCmdLine`, `cRZIniFile`, `cSC4ConfigRuleManager` | foundations §3.7 (save/load sequence) | S0 (part) |
| F2 GZCOM core, framework, services | 16 | `cGZCOM`, `cRZCOMDllDirector`, `cGZFramework`, `cGZApp`, `cRZSystemService` | public SDK headers; simulator §5.1, §9 | S0 |
| F3 message and command servers, cheats | 38 | `cGZMessageServer2`, `cGZMessageQueue2`, `cGZCommandServer`, `cGZCheatCodeManager` | foundations §5 (messages); simulator §9 (cheats) | S0 (messages) |
| F4 timers, dates, animation tick | 13 | `cRZTimer`, `cRZCallbackTimer`, `cSC4AnimationTickManager`, `cSC4AnimationCoordinator` | simulator §5.5–§5.7 | **S0** (§8) |
| F5 persistence, resources | 91 | `cGZPersistResourceManager`, `cGZDBSegment*`, `cGZPersist*`, `cRZFile`, `cRZVariant` | **data-formats §2–§5**, foundations §2–§3, §9–§10 | S0 (done in `engine/formats`) |
| F6 language, strings, detokenizer | 40 | `cRZLanguageManager`, `cRZLanguageUtility`, `cSCStringDetokenizer` | data-formats §6, lua-host §10 | S4 |
| F7 web, HTML help, HTTP | 104 | `cHTMLDocument`, `cRZHTTPClient`, `cGZWWWSystem`, `cGZWinHTML` | — | not needed |
| F8 developer tools and editors | 207 | `cSC4WinLotConfigurationEditor`, `cSCPropertyView`, `cGZWinLuaScript*`, `cSC4WinAnimationRecorder` | — | not needed |
| V1 graphics system and drivers | 119 + ranges | `cGZGraphicSystem`, `nRZSimGL`, `nSGLDX7::cGDriver`, `cGZCanvasW32`, `cGZDrawContext*` | — | S1 (replaced, §3.1) |
| R1 3D render core | 75 | `cSC43DRender`, `SC4DrawContext`, `cSC4RenderProperties`, `SC4QuadGrid` | — | S1 |
| R2 camera, zoom, rotation | 37 | `cSC4CameraControl`, `cS3DCamera`, `SC4ViewUtilities` | graphics-formats §4.2 (zoom mask) | S1 |
| R3 model and texture system | 24 | `cS3DModelInstance*`, `cS3DTextureBindingFactory`, `cS3DMaterialManager`, `cS3DModelProcessor` | **graphics-formats §2–§4, §8.5** | S1 |
| R4 model maker and background loader | 41 | `cSC4ModelMaker`, `cSC4ModelBackgroundLoader` | graphics-formats §4.2; occupant-manager §5 (messages) | S1 |
| R5 terrain and water drawing | 65 | `cSTETerrainView3D`, `cSTEOverlayManager`, `cSTEWaterRipple` | terrain.md (data), zones §7.1.1 (highlight textures) | S1 |
| R6 lighting, night, shadows | 21 | `cSC4LightingManager` (+ overlay shadows) | clock24.md (time of day) | S3 |
| R7 effects and particles | 144 | `cSC4EffectsManager`, `cSC4ParticlesEffect`, `cSC4EffectsCollection` | disasters §11 (game actions only) | S3 |
| R8 overlay views and labels | 43 | `cSC4DispatchVehicleView`, `SC4PathInfoView`, `cSC4NetworkLayoutView`, `cSC4GlyphTextureManager` | — | S4 |
| U1 window system, UI scripts | 241 | `cGZWin*`, `cGZLegacyControl`, `cGZUIScriptService`, `cGZWinCtrlMgr` | modal-dialogs.md (modal loop) | S4 |
| U2 fonts, cursors | 21 | `cGZFont`, `cGZFontSys`, `cRZCursorManager`, `cSLCursor` | — | S2 |
| U3 graphs | 12 | `cGZGraph`, `cGZPieGraph` | — | S5 |
| U4 city view and tools | 259 | `cSC4View3DWin`, `cSC4ViewInputControl*` (22 tools), `cSC4NetworkToolUI` | network-tools.md, zones §7 (tool back ends) | S2–S4 |
| U5 info windows, data views, budget, news | 233 | `cSC4SubBudgetWindow`, `cSC4WinMapView`, `cSC4GraphsViewWinProc`, `cSC4NewsTickerWinProc` | advisors.md, budget.md (data) | S5 |
| U6 dialogs and preferences | 38 | `cSC4VideoPreferencesWinProc`, `cSC4AudioPreferencesWinProc` | modal-dialogs.md | S5 |
| U7 shell screens, intro video | 5 + range | `cWinSC4App`, `cSC4WinIntroVideoScreen`, `cSC4WinSplashScreen`, `cSC4MadCowPlayerWin` | — | S6 |
| G1 region view, region save/load | 50 | `cSC4Region`, `cSC4WinRegionScreen`, `cSC4WinRegionView` | nation.md, foundations §11 (regional-city record), neighbors §9 | S5 |
| A1 audio | 138 + 74 | `cGZSndSys`, `cHitMan`, `cFreshPlayer`, `cRZRadio`, `cSC4Audio*` | — | S5 |

Third-party code (Lua, libpng, zlib, CRT, STLport: about 300 KB) is replaced by Rust crates or system libraries and
needs no spec, except Lua (lua-host.md).

---

## 2. Framework

### 2.1 Application start-up, command line, ini files, launcher (F1)

- **Classes.** `cSC4App` (class `0xCA263481`, 55 KB; vtables from `0x00A86A68`) owns start-up, the city life cycle
  (`NewCity` `0x004548B0`, `LoadCity` `0x004555F0`, `SaveCity` `0x00455E20`, `CloseCity` `0x00455330`,
  `DoCityObliteration` `0x00454E50`) and the directories. `cWinSC4App` (`0x004E1060` `Init`) owns the top-level
  screens: it creates, in turn, the intro video screen, the splash screen, the region screen and the city screen
  (`CreateIntroVideoScreen` … `CreateCityScreen`, `0x004E0C20`–`0x004E0DD0`). There is no separate main menu: the
  region screen is the game's menu.
- **Start-up order** (names from the exe): `PreFrameWorkInit` (`0x00452360`: debug level, logging, the single-instance
  mutex `SimCity 4 Mutex` unless `AllowMultipleInstances`, `Restart`, cursor mode `blit`/`off`, the `Plugins` and
  `Sku_Data` folders, `GZLog`), `AddSystemServices` (`0x00450350`), `AddCodePlugins` (`0x00451260`: DLL plug-ins
  from the plug-in folders, each with an optional `.ini` holding `Enabled` and `Admin`), `SetupResources`
  (`0x004572B0`, §2.5), `LoadRegistry` (`0x004584A0`: `Resources.ini`, keys `packedexemplars`, `packedtextures`,
  `DBSegments`, `DBSegmentDirectoryFiles`), `PostFrameWorkInit` (`0x00453240`: Lua `-LuaScript:` start-up script,
  `NetCommandGenerator`). The app messages are PreAppInit / PostAppInit / PreAppShutdown / PostAppShutdown
  `0x28E9397A`–`0x28E9397D` and PostAppServicesInit `0x2B96B3EA` / `…Complete` `0xAB96B05F`.
- **Command line** (`ReadCommandLine` `0x0044EED0`; switch strings at `0x00A86DF8`–`0x00A86E98`):
  `ExceptionHandling` (`off`/`disabled`/`enabled`/`verbose`), `IME`, `r<W>x<H>x<D>` (resolution, parsed with
  `%ux%ux%u`), `f` (full screen), `w` (windowed), `g`, `d:<driver>` (`opengl`, `soft`, `dx`/`directx`; §3.1),
  `audio`, `sound`, `radio`, `music` (each `off`/`disabled`/`enabled`), `sim`, `RDD` (real-time debug display),
  `UserDir`, `CPUCount`, `CPUPriority` (`low`), `AutoZoomChange`, `CustomResolution`. Others are read elsewhere:
  `intro:off` (`cWinSC4App::Init` `0x004E11CC`), `IgnoreMissingModelDataBugs` and `backgroundLoader:off`
  (`cSC4ModelMaker::DoPostCityInit` `0x00494EAC`, `0x00494DF1`). The GOG play tasks start
  `Apps\SimCity 4.exe -CPUCount:1 -f` (and `-w` for the windowed task), working directory `Apps`.
- **Ini and preference files.** `Apps\SimCity 4.ini` (`[Directories]` `Data`, `PlugIn`); the user preferences file
  `SimCity 4.cfg` (`GetAppPreferencesFileName` `0x0044A940`, `ReadPreferences` `0x0044BC50`,
  `SavePreferences` `0x0044BDB0`); `FontStyle.ini` (`Font Styles`, `Font Aliases`); `region.ini` per region
  (§6); `album.ini`; `CookieData.ini`, `FileCache.ini` (web, not needed). The install path comes from the registry
  (`HKLM\SOFTWARE\Maxis\SimCity 4\Install Dir`) and the language from `…\SimCity 4\1.0\Language` (set by the
  GOG `language_setup.exe`).
- **User folders** under `Documents\SimCity 4`: `Plugins`, `Regions`, `Albums`, `HTTPCache`, `Exception Reports`,
  `TestScripts`, `Scripts`.
- **Graphics rules.** `cSC4ConfigRuleManager` (`0x0A1D8285`, 10 KB) parses `Graphics Rules.sgr`
  (`ParseConfigFile` `0x007CFF4B`; the install also ships `Video Cards.sgr`) and maps the detected card and memory
  to option groups that set the render properties (§3.2).
- **Video preferences**: `ApplyVideoPreferences` `0x0044A1C0`, `IsStartupResolutionOK` `0x0044BA80`; the software
  fallback message ("Could not initialize the hardware driver …") is shown from `cSC4App::Init`.
- **Shipped data read:** the ini and `.sgr` files above; nothing in the packages.
- **Simulation dependencies:** the app creates and owns the city (`cSC4City`, `0x8990C372`) and posts the city
  messages. **Spec gap:** the city's own fields and the record `0x89D9B9EE` it writes (save-records Q4) and the city
  life cycle (`cSC4App::LoadCity` → `cSC4City::CreateCityComponents` `0x0045AA10` → `Init` `0x0045BD20` → the
  PreCityInit / PostCityInit / PostCityInitComplete messages `0x26D31EC0`, `0x26D31EC1`, `0xEA8AE29A`) are not
  specified beyond foundations §3.6–§3.7.

### 2.2 GZCOM, the framework and the COM directors (F2)

- `cGZCOM` (16 KB with the framework) holds the class registry; `cGZFramework` (`sInit` `0x0087AF13`, `sRun`
  `0x0087957A`) runs the services. The game is itself a set of COM directors, the same mechanism FreeCity4.dll
  uses. `cSC4App::AddCOMDirectors` (`0x004493B0`) adds the SC4 director (`SC4GetCOMDirector` `0x00460F80`), the
  audio director and the web director. The SC4 director's constructor (`0x00460090`) adds 14 sub-directors and
  registers about 190 classes itself.
- **Directors and what they register** (count of classes; notable class IDs):

| Director | Classes | Non-simulation class IDs |
|---|---|---|
| SC4 (`0x00460090`) | ≈ 190 | message server 2 `0x252294E9`, message queue manager 2 `0x452294DF`, Lua script server `0xCBD101A9`, Lua debugger windows (5), 3D renderer `0xE9C622D8`, camera `0xE9C6262A` *[inferred: its `GetGZCLSID` (`0x007FF010`) lies among `cS3DCamera`'s methods and the ID is used again at `0x007FF049`]*, camera control `0xC9C628EC`, animation tick manager `0xA9C73857`, model maker `0xA9C741B3`, FSH texture factory `0x3AB50E2A`, S3D model factory `0xFAD0F0B6`, terrain view `0xC9B84E10`, surface water `0x0B8DA7DB`, effects parser / manager / resource / factory `0xA956AD14` / `0x49822F75` / `0x2A5118AD` / `0xEA5118B5`, graphics rules `0x0A1D8285`, texture binding factory `0x9AC0E23C`, material manager `0x9AC0E23D`; the rest are simulation classes, occupants, filters, SimGrids and ordinances |
| scripted windows (`0x004662B0`) | 17 | advice list `0xCA1492AC`, minimap `0xCA318388`, aura bar `0xAA5D16A9`, alert border `0xCA5D3294`, map view `0x28C5A41F`, lot configuration chooser / editor `0xC918C6FF` / `0x4918C704`, building family chooser / building chooser `0x8918C708` / `0xE918C70C`, network lot chooser / editor `0x29AAD0F6` / `0x29AB0142`, RCI bar `0xC7A0E17E`, trend bar `0xAA5C2F86`, transparent window `0x89E1567C` |
| graphics drivers (`0x00466250`) | 3 | OpenGL `0xC4554841`, software `0x7ACA35C6`, DirectX `0xBADB6906` (§3.1) |
| GZ window (`0x00998B90`) | 29 | control manager `0xC2C2EB0F`, drag and drop `0xAC0874D8`, text edit `0x231A1C58`, button `0x22ECFC47`, bitmap `0x82FE68C4`, slider `0x21325208`, scrollbars `0x61325A2E` / `0x4B42DA5D`, line input `0x21335C5A`, option group `0xA1336CC1`, spinner `0x612CE0C4`, outline `0x4303E5B6`, grid `0xDAA6B9BF`, IME proxy / composition `0x2AC4516D` / `0x2AC45170`; message box, list box and combo box under the small numeric IDs `0x593`, `0x598`, `0x59B` (also `0x592`) |
| UI script (`0x0094AFF6`) | 4 | script service `0xFA3562FB`, legacy control `0xFA450242` |
| draw, font, graphic, Win32, gimex | 5, 3, 1, 2, 3 | draw system `0xAB2EA3AA`, draw font renderer `0x8B3255F9`, font system `0x20673960`, image codec factory `0xA2FFB5D3` |
| resource manager, resources W32, resource library | 15, 4, 25 | resource manager `0x656B8F27`, multi-package segment `0x090F8619`, exemplar / cohort factories `0x453429B3` / `0x053429C8`, exemplar cache `0x252EA0DF`; about 20 property-editor views (F8) |
| automata (`0x00514A00`) | 30 | all automata and vehicle classes (automata.md) and the city situation manager |
| audio (`0x004EC710`) | 90 | §5 |
| sound, Hit, freshness | 8, 10, 5 | sound system `0xE41CE9FD`, sound manager `0x7AF85B98`, Hit manager `0x7B1BD993`, freshness player / score / cell `0x3C1CF59E` / `0x5C1CF5A3` / `0x7DD9C9F2` |
| Lua, script, tools, web | 4, 2, 2, 1 | Lua host classes (lua-host.md); HTML window `0xAA12E5F5` |

- **System services** (`AddSystemServices` `0x00450350` and the directors' `OnStart`): framework service IDs in the
  ID table include the canvas `0xA3460DEB`, file system `0x2073215D`, graphic system `0xC416025C`, window manager
  `0xA417445E`, command server `0xEB903A32`, language manager `0x441E5070`, Lua script server `0x4BD10133`, radio
  `0x1D07B39F`, IME server / proxy `0x2AC45447` / `0x2AC45449`, real-time debug display `0x8414A2C2`, ambience and
  audio-scape managers `0xAA12F911` / `0x49E50BA1`, AEMS events `0xAC1FAE31`, animation tick manager `0x86AD10EC`,
  24-hour clock `0xA7E7F92A` and the simulator `0x46956659`.
- **What a standalone needs:** a service registry with tick and idle lists ordered by priority (the simulator's
  `OnTick` contract is simulator §5.1), the director hooks (OnStart, PreFrameWorkInit, PreAppInit, PostAppInit,
  PreAppShutdown, PostAppShutdown) and the class registry, only as far as FreeCity4's own parts use them.
  *[choice, for the coordinator]* Whether standalone FreeCity4 hosts the original's DLL plug-ins (GZCOM ABI) is a
  separate decision; nothing in §10 needs it.

### 2.3 Message server, command server and cheats (F3)

- The message server 2 (`cGZMessageServer2`, queue `cGZMessageQueue2`, 3 KB) is specified as far as the simulation
  uses it (foundations §5). Not yet written down: the queue order between posted and sent messages across
  frames, and `cSC4App::RunMessageServer2Pump` (`0x00449A70`, a time-bounded pump driven by a `cRZTimer`).
- The command server (`cGZCommandServer`, `0x0092A982` `Init`) maps command IDs to handlers. The game's command IDs
  (`kCommandID_*`, about 130 entries in the ID table from `0x6A935C34`) are the UI vocabulary: speeds, zoom 0–5,
  rotate, scroll, every tool, open dialogs, save, quit. Cheats go through `cGZCheatCodeManager` (3.5 KB) and the
  cheat dialog `cSC4CheatCodeEntryDialog`; the simulator's own cheats are in simulator §9.

### 2.4 Main loop, timers and the animation tick (F4)

- `cSC4App::OnIdle` (`0x00449C70`) runs the per-frame work between window messages, timed with `cRZTimer`
  (`0x0088FEFB`–`0x0089064D`, QueryPerformanceCounter based), and posts the idle message `0xBA44EB51`
  (`kMessageGameInIdleState`).
- `cSC4AnimationTickManager` (`0xA9C73857`, service `0x86AD10EC`, 3 KB; vtable `0x00A86760`) delivers
  `kSC4MessageAnimationTick` `0x86AD10EE` and `…Priority` `0x86AD10EF` to registered tickables, with minimum and
  maximum tick deltas, pause and single-step (`PauseAnimationSystem`, `StepAnimationSystem`). **The simulation
  depends on it** (§8): the lot developer, the prop developer (timed props), the terrain (`DoAnimationTick`
  `0x0074A390`, foundations §11.6), the fire disaster and the automata controller manager register with it. It is
  therefore part of S0, not of the renderer.

### 2.5 Persistence and resources (F5)

Specified in data-formats §2–§5 and foundations §2–§3, §9–§10, and implemented in `engine/formats`. For reference,
`cSC4App::SetupResources` registers one factory per resource type (`0x00457EC6`–`0x00457FE8`):

| Type | What | Factory class |
|---|---|---|
| `0xCA63E2A3` | Lua script | `0x0A63DF8C` |
| `0x2026960B` | LTEXT, and also the sounds in `Sound.dat` (§5) | (registered at `0x00457EDA`) |
| `0x856DDBAC` | PNG and other images | `0xA2FFB5D3` (image codecs) |
| `0x7AB50E44`, `0x7AB50E45` | FSH | `0x3AB50E2A` |
| `0x29A5D1EC`, `0x09ADCD75` | sprite animation tables ("ATC", "AVP"; automata sprites) | `0x69B6F01C` |
| `0x5AD0E817` | S3D | `0xFAD0F0B6` |
| `0x6534284A`, `0x05342861` | exemplar, cohort | `0x453429B3`, `0x053429C8` |
| `0xA83479D3` | prop placement properties | `0xA83479EA` |
| `0x296678F7` | SC4Path | `0x496678FE` |
| `0xEA5118B0` | effects directory | `0xEA5118B5` |
| `0xA2E3D533` | key accelerator configuration | `0x42E411C2` |
| `0x686AA4B0` | (no shipped record) | `0xC8696797` |

### 2.6 Language and strings (F6)

`cRZLanguageManager` and `cRZLanguageUtility` (18 KB) pick the language and its folder; LTEXT and locale selection
are in data-formats §6; the string detokenizer is in lua-host §10. Needed when UI text is shown (S4).

### 2.7 Not needed for a standalone game

- **Web, HTML help, HTTP** (F7, 104 KB): an HTML renderer (`cHTMLDocument` 35 KB) for the in-game help and news
  pages, an HTTP client, cookies and a file cache. *[choice]* Leave out; links can open the system browser.
- **Developer tools and editors** (F8, 207 KB): the lot and network-lot editors, the exemplar property views, the Lua
  debugger windows, the animation recorder (AVI frames through `MSVFW32`), the window-procedure spy, the debug
  console and the Game of Life easter egg. Not needed.

---

## 3. Renderer

### 3.1 Graphics system and drivers (V1)

- `cGZGraphicSystem` (service `0xC416025C`, 9.5 KB; `Init` `0x00990880`, `LoadAllDrivers` `0x009904D0`,
  `RenderBufferToScreen` `0x0098F110`) loads three drivers that implement one rendering interface (the "SimGL"
  layer, `nRZSimGL`, 9 KB of helpers): **OpenGL `0xC4554841`**, **software `0x7ACA35C6`** and **DirectX 7
  `0xBADB6906`** (the `d:` switch, §2.1; the software driver is the fallback and the preference
  `kForceSoftwareDriver`). Sizes in §0.1.
- *[correction for `re/names_windows.tsv`]* The function at `0x0087E0A0`, named `GZCOM_cSGLDriverDX7CLSID`, returns
  `0xC4554841`, which `ReadCommandLine` selects for `opengl` (`0x0044F4AA`) and which lies in the OpenGL driver's
  range. The DirectX driver's ID is `0xBADB6906` (`nSGLDX7::cGDriver::GetGZCLSID` `0x00886E01`). The `nSGLDX7`
  names were ported from the Mac build, which has one driver, and some land on OpenGL-driver code.
- The window and canvas (`cGZCanvasW32`, 9 KB), the screen buffer, the 2D draw contexts (`cGZDrawContext*`,
  27 KB, drawing to a buffer, to hardware or through SimGL) and the 2D blitters (§0.1) serve the UI.
- *[choice]* FreeCity4 does not port the driver layer: it renders through one modern API (for example wgpu or
  OpenGL 3) and draws the UI with it. Only the **observable** parts need specs: the render properties that change
  the image (§3.2) and the texture formats (graphics-formats §2).

### 3.2 3D render core (R1)

- `cSC43DRender` (class `0xE9C622D8`, 37 KB; vtables `0x00ABAB38`, `0x00ABAB60`): the viewport, the view frustum,
  a **quad grid** of view objects (`SC4QuadGrid`, `CreateQuadGrid`, `RebuildQuadGrid`), insert / remove / move /
  change of model instances (`InsertModel`, `RemoveModel`, `ProcessModelChangeQueue`), a **backing store** that
  caches the static scene per zoom (`CreateBackingStore`, `SaveBackingStore`, `RestoreBackingStore`) with dirty
  rectangles, separate **static and dynamic passes** (`DrawStaticView`, `DrawDynamicView`, with pre/post hooks),
  picking (`Pick`, `PickTerrain`), snapshots (`TakeSnapshot`), the background picture and screen shake. It writes a
  save record through the View3D record (§4.4).
- `cSC4RenderProperties` (19 KB) is a table of named properties (`kBackingStoreMaxZoom*`,
  `kBuildingVariety`, `kDrawLotTextures`, `kDrawLotWalls`, `kForceBilinear`, `kLODOffset`, `kMaxVehiclePct`,
  `kParticleDensity`, `kShadows*` …) set from the graphics rules (§2.1) and the preferences.
- Tuning: the render core reads exemplars of group `0x690F693F` (`ReadProperties` `0x007C5F72`; for example
  "Model Highlights" `0x4A639EF2`).
- *[choice]* The backing store and dirty rectangles are a 2003 performance device; a modern renderer that redraws
  every frame may leave them out. Draw **order** (static before dynamic; which layers are translucent) is what must
  be specified.
- **Simulation dependencies:** none directly; it is fed by the model maker (§3.5) and the terrain view (§3.4).

### 3.3 Camera, zoom and rotation (R2)

- `cSC4CameraControl` (class `0xC9C628EC`, 9 KB; vtable `0x00ABAD18`) holds the view target, zoom and rotation,
  scroll bounds clamped to the city size, and converts between world points and the camera plane. Its constructor
  (`0x007CC990`) fills a five-entry table that halves per step (100, 50, 25, 12.5, 6.25 at `+0xB8`…`+0xC8`) and two
  angles, −π/4 and π/6 (`+0x118`, `+0x11C`) *[inferred: the per-zoom scale and the default yaw and pitch;
  unsettled which zoom each entry belongs to]*. `cS3DCamera` (`0xE9C6262A`, 4.5 KB: `SetOrtho`, `Project`,
  `UnProject`, `GetEyeRay`) is the projection.
- Zoom levels 1–5 and four rotations (commands Zoom0–Zoom5, RotateCW/CCW); the model keys select a model per zoom
  and rotation (graphics-formats §4.2). Messages: CameraUpdated `0x4A9F2FFE`, ViewZRChange1–3
  `0xA6B79600`–`0xA6B79602`, ViewTChange1–3 `0xA6B79610`–`0xA6B79612`, PostViewInit `0xA6B79620`,
  PostViewRefresh `0xA6B79621`, PreViewShutdown `0xA6B79622`; their subscribers include the model maker, the
  terrain view and the lighting manager.
- `SC4ViewUtilities` (17.5 KB) and `SC4GeometryUtilities` (5 KB): screen↔cell conversions used by tools and picking.
- **Shipped data:** "UI Tuning Parameters" `0x6534284A/0x690F693F/0xE90F6C84` (scroll and view tuning, read by
  `cSC4View3DWin::ReadTunableValues` and the city scroll control).

### 3.4 Terrain and water drawing (R5)

- `cSTETerrainView3D` (class `0xC9B84E10`, 37 KB; vtable `0x00AB4480`; terrain.md §13.2 gives its 3-byte record):
  textured terrain rows and quads, side cliffs at the city edge, water layers and the water surface colour, the
  grid and contour display, cell levelling marks, the plumbing view, culling by a draw frustum. `StaticInit` loads
  the terrain textures (FSH, type `0x7AB50E44`). `cSTEOverlayManager` (14 KB) draws decals and shadows on the
  terrain; `cSTEWaterRipple` (8 KB) the shore ripples; `cSTESelectedTerrainArea` the selection highlight.
- **Shipped data:** "Terrain Properties", "Ripple Properties", "Wave Properties", "Lava Properties"
  (`0x6534284A/0x88CD66E9/1, 2, 4, 5`), the terrain texture FSH records, and the season (weather.md) for the
  texture choice *[inferred]*.
- **Simulation dependencies:** the terrain heights and water (terrain.md: the `fc4_terrain_*` core exists), the
  terrain change messages (TerrainHeightsChanged `0x278E6003`, WaterHeightsChanged `0x678E6E3A`), the zone tool's
  `MarkSelected` (zones §7.1.1), the plumbing grid for the pipe view.

### 3.5 Models, props and the model maker (R3, R4)

- The S3D/FSH object system (`cS3D*`, 24 KB): model instances (basic and animated sprite), the material manager,
  texture bindings keyed by resource key, the model processor (normals, lighting, mesh sort). The file formats are
  **specified and implemented** (graphics-formats §2–§4, `engine/formats`); the instance and material behaviour
  is in graphics-formats §3.12 and §8.5.
- `cSC4ModelMaker` (class `0xA9C741B3`, 29 KB; vtable `0x00A8EDE0`) turns occupants into model instances: it
  subscribes to the occupant messages (`0x99EF1140`–`0x99EF1146`, occupant-manager §5) and to zoom/rotation and
  view-filter changes, loads building, prop, foundation, network and underground models for the current zoom and
  rotation, creates occupant shadows and placeholders, hides terrain under occupants, and applies building
  state effects. `cSC4ModelBackgroundLoader` (8 KB) loads model resources on a **worker thread** around the visible
  area (tuning "Background Loader" `0x6534284A/0x7A4A8458/0x0B579C20`–`0x0B579C23`).
- Several occupant classes build their own models (network pieces, power poles and pylons, underground
  connections, retaining walls, lot base textures, signposts, text decals, construction and burnt buildings; §8).
- **Shipped data:** S3D `0x5AD0E817` (42,915 records, all group `0xBADB57F1`), FSH (40,549 records, mostly group
  `0x1ABE787D`), the model keys in exemplars (graphics-formats §4.2).
- **Simulation dependencies:** the occupant manager (insert / remove / move messages), the lot and building state
  (BuildingVisualStateChange `0x09D8E0B6`, BuildingCompletionChange `0x2977AE2F`, PropStateChange `0xE9C86DDB`),
  the network occupants (network-manager.md), the lot configuration (lot-configuration.md).

### 3.6 Lighting, night and shadows (R6)

- `cSC4LightingManager` (class `0x681BD52B`, 21 KB; vtable `0x00ABB530`): time of day and night (`GetTimeOfDay`,
  `IsNight`), global light, terrain and model colours, hill shadows, shadow direction and projection, a lighting
  tint. Day/night modes are the commands EnableDayOnly / NightOnly / DayAndNight.
- **Shipped data:** one lighting exemplar `0x6534284A/0xA9189CF0/0x00000001` (`Init` `0x007DB6FB`) and its
  property tables (`ReadProperties` reads the ID blocks `0x085489C8`–`0x085489D3` and `0x29189BEB`–`0x29189BF7`).
- **Simulation dependencies:** the 24-hour clock (clock24.md; its record is a city component).

### 3.7 Effects and particles (R7)

`cSC4EffectsManager` (`0x49822F75`) and its parser, collections and particle systems (144 KB) play the effects of
the effects directory (`0xEA5118B0`, one record, group `0xEA5118B1`). The few effects that act on the game (ignite
cells, toxic spills) are specified in disasters §11. Visual only otherwise; deferred to S3.

### 3.8 Overlay views and labels (R8)

Dispatch vehicle view, path and route views (query tool routes), train junction view, marked-cell view, network
layout and preview views, and `cSC4GlyphTextureManager` (text rendered into textures for labels and signs). 43 KB.
Needed with the tools (S4).

### 3.9 Automata drawing

Cars, people, planes and boats are drawn from the sprite animation tables (types `0x29A5D1EC` and `0x09ADCD75`,
1,084 and 2,253 records) and S3D models. Their simulation records and hooks are in automata.md; their drawing and
movement on screen are not specified. Deferred to S3.

---

## 4. User interface

### 4.1 Window system and UI scripts (U1)

- `cGZWin` and its controls (29 classes, §2.2; 241 KB with the grid 29 KB and the text editor 19 KB), the window
  manager (`cGZWinMgrBase`/`cGZWinMgrW32`; modal loop in modal-dialogs.md), the control manager
  `cGZWinCtrlMgr` (`0xC2C2EB0F`; system font, bitmap and colours set up by `cSC4App::SetupWinCtrlMgr`
  `0x0044D810`), drag and drop, clipboard, tooltips, IME (`IMM32`), key accelerators.
- **UI scripts.** Windows are built from text resources of type `0x00000000`, group `0x96A006B0` (271 records,
  each starting with a generated-file comment; 10 more in group `0x08000600`) by the UI script service
  (`cGZUIScriptService` `0xFA3562FB`: `ExecuteScriptResource`, `CreateWindowFromScript`) and the legacy control
  interpreter (`cGZLegacyControl` `0xFA450242`, 58 KB: one property vocabulary per control kind,
  `InterpretPropertiesGZWin*`). Game code creates windows by instance ID.
- **Images and text:** PNG UI images (`0x856DDBAC`, groups `0x46A006B0` 811 records and `0x6A386D26` 357 records
  for catalog icons), LTEXT (`0x2026960B`, main UI groups `0x6A231EA4` 2,442 and `0x6A231EAA` 1,009 records).
- **Key maps:** type `0xA2E3D533`, group `0x6A231EAA`, 4 records (text), read by `cWinSC4App::Init`,
  `cSC4View3DWin::Init` (instance `0x6A9362EF`), the region screen (`0x6A9362F0`) and the U-Drive-It control.
- *[choice, for the coordinator]* FreeCity4 can either **interpret the shipped UI scripts** (exact look, plug-in
  UI mods keep working; needs a spec of the script grammar and every control's vocabulary and drawing) or **draw
  its own UI** over the same commands (§2.3) and data. The minimal path (§10) needs neither until S4.

### 4.2 Fonts and cursors (U2)

- Fonts: `Fonts\*.mxf` in the install (19 files, magic `MXFN`, for example "Arial MT", "Arta", "CordiaUPC",
  "Courier LD"), read by `cGZFont` / `cGZFontSys` (`0x20673960`, 18 KB), with `FontStyle.ini` styles and aliases
  and fallbacks to system fonts for East-Asian languages (`ReadCommandLine` area strings `Gulim.ttc`, `mhei gb` …).
  The MXF format is drawing-fonts.md §14 (re-fonts, 2026-10-23).
- Cursors: type `0xAA5C3144` (444 records: 111 cursors in each of the groups 1, 4, 8 and `0x32`, matching the
  cursor modes `bw`, `color16`, `color256`, `fullColor`), read by `cSLCursor::Load` (`0x00602D81`) and
  `cRZCursorManager` (vtable `0x00AC5290`).

### 4.3 Screens: intro, splash, region and city (U7)

`cWinSC4App` switches between four screens (§2.1). The intro video (`cSC4WinIntroVideoScreen` `0x2A3832AA`) plays
the two "MADk" records of type `0x0A8B0E70` in `Intro.dat` with the MAD player (§0.1); the splash screen
(`0xAA38326E`) shows a scripted window; the region screen is §6; the city screen is the 3D view (§4.4). The
progress screen (`cSC4ProgressScreen`) covers loads and obliteration.

### 4.4 The city view and its tools (U4)

- `cSC4View3DWin` (class `0x9A47B417`, 62 KB; vtable `0x00ABCE40`) is the city screen: it owns the renderer, the
  camera control, the model maker and the terrain view (`DoPreCityInit` `0x007F41F6`…`0x007F45C5`), the panels
  and menus, the catalogs, the cost indicator, the date, funds and population displays, the alert border, the
  speed and day/night buttons, and the current **view input control** (the tool).
- **Save record** `0xA9C8647C / 0xE9C8647E / 1` (`DoSave` `0x007EA5B0`, `DoLoad` `0x007EBFB0`): references to the
  camera control, the 3D renderer, the model maker (`0xA9C741B3`) and the simulator, then lists
  (save-records §2; not specified, Q4).
- **Tools** (`cSC4ViewInputControl*`, 22 classes): query (72 KB, §4.5), place lot (21 KB), U-Drive-It (12 KB),
  zone, demolish, network and intersection tools, terrain scrubber and levelling, flora, sign / label, dispatch,
  effects and disasters, toll booth, torch, city scroll, MySim controls. The back ends are simulation-side and
  mostly specified (zones §7, network-tools.md, demolition.md, dispatch.md, disasters.md, mysims.md,
  city-situation.md); the tools themselves (cursor feedback, drag rules, cost preview, sounds) are not.
- **Tuning:** "Demolition Thresholds", the twelve U-Drive-It panels ("Drive … Panel") in group `0x690F693F`.

### 4.5 The query tool (U4)

`cSC4ViewInputControlQuery` (72 KB, the largest UI class) picks an occupant, opens the per-kind query window and
fills it from simulation getters: occupancy and capacity, land value, mayor rating, desirability equation,
neighbour connections and deals, airport and transit funding and traffic, routes through a cell. It is the best
single **consumer list** of simulation read interfaces for the UI; a spec of it should be a table of fields and the
getter each one reads.

### 4.6 Info windows, data views, graphs, news (U5)

- Budget window (`cSC4BudgetUIWinProc` and `cSC4SubBudgetWindow`, 91 KB), map view and data views
  (`cSC4WinMapView` `0x28C5A41F`, 26 KB: the "DataView: …" exemplars in group `0x690F693F`, about 60 records, one per
  map, naming the grid, colours and legend), minimap (`0xCA318388`, "Mini map" `0x8A356F2C`), graphs
  (`cSC4GraphsViewWinProc`; graph exemplars group `0xCA4AD545`: "Mayor Rating", "Funds", "Income & Expense" …),
  news ticker (`cSC4NewsTickerWinProc`, 8 KB: advisor and opinion messages, advisors.md), advisor windows,
  MySim windows, tutorial window, the trend bars ("… Trend Bar" exemplars `0x6A5E6EDC`–`0x6A5E6EE1`), the RCI bar,
  the snapshot album and taker.
- **Simulation dependencies:** the SimGrids of every simulator (data views), the history warehouse (graphs,
  history.md), the budget (budget.md), the advisors (advisors.md), demand (RCI bar).

### 4.7 Dialogs and preferences (U6)

Video (15 KB), audio (11 KB) and game preferences, tip of the day, credits, cheat entry, region and city dialogs
(new, delete, import, load), missing plug-ins. Modal behaviour: modal-dialogs.md.

---

## 5. Audio (A1)

Five layers, 138 KB of classes plus the 74 KB sound library:

| Layer | Classes | Data |
|---|---|---|
| framework sound system | `cGZSndSys` (`0xE41CE9FD`, vtable `0x00AD7180`), `cGZSndMgr` (`0x7AF85B98`), WAV and "UTalk" data sources | DirectSound (`DSOUND` ordinals 1, 2), `WINMM` mmio |
| EA sound library (SNDSYS / AEMS) | unnamed (§0.1), `cSC4AudioAEMSEventSystem` | AEMS banks: type `0x00000000`, group `0x8A5971C5` (43 records with magic `AB`, about 41 MB) |
| Hit (sequenced music and sound tracks) | `cHitMan` (`0x7B1BD993`), `cHitTrackPlayer` (15 KB), channels | track definitions `0x0B8D821A` (869 records, `TKDT`), Hit lists `0x7B1ACFCD` (214) |
| freshness (adaptive ambient score) | `cFreshPlayer`, `cFreshScore`, `cFreshCell` | freshness scores (loaded by the ambience layer as LTEXT-type keys, `0x004E6D74`) |
| radio | `cRZRadio` (`0x5CFBDBBF`), `cRZDiskRadioStation` (`0x5CFBDBD5`), `cGZPlayList` (`.m3u`, `.pls`) | `Radio\Stations\<Mayor|Region>\Music\*.mp3` (35 files) and a call-sign file (`kmay.id`, `krgn.id`) per station; `Res\Music\Radio\` |
| SC4 audio managers | sound manager `0x4A47DC0D`, ambience manager `0x6A12F51C` and layers `0x4A2835FE`, event handlers `0xBDB6171E` and `0x69E500BC` (fire), listener `0xCA4A66B9`, scape and sem managers; the audio director registers about 80 further classes from factories at `0x004EB260`–`0x004EC6F0` *[unsettled: per-event handlers or emanation models]* | sound effects: `Sound.dat`, type `0x2026960B` (1,512 of the records surveyed are EA XA ADPCM, a few RIFF WAV; about 120 MB); 12 records of type `0x9D796DB4` (no direct reader found) |

- **Simulation dependencies:** occupant insert / remove (ambience per occupant and alarms), automata sound messages
  (AutomataPlaySFX `0x8A385DD8`, AutomataStopSFX `0xAA396960`), dispatch, construction, disasters (fire),
  the listener follows the camera (ListenerPosChanged `0x8A4A69C7`). Audio changes nothing in the simulation.
- **Spec gaps:** EA XA decoding, the AEMS bank format, the Hit track format, the MP3 decoder (inside the sound
  library's codec set, `SND::CODASetNew`; *[inferred]*). *[choice]* A standalone can start with WAV/XA sound
  effects and the radio (MP3 through a crate) and leave AEMS and Hit for later.

**Corrects (re-specs20):** written up as [audio.md](audio.md). Four corrections to the table above.

- **IN6-3 ANSWERED.** The reader of the 12 records of type `0x9D796DB4` **does not exist as code**: the
  constant has zero byte occurrences in `SimCity4.exe`. The type is bound at runtime by
  `cHitIniFile::ProcessDBSegmentDirectoryFilesEntry` (`0x00936707`) from the audio configuration record's
  `[DBSegmentDirectoryFiles]` line `0xca4d19e3=0x9d796db4:tlo,0x2026960b:hit`, i.e. extension **`tlo`** in the
  TLO segment. **DIFFERS** from "no direct reader found".
- The sound-effects row's "1,512 of the records surveyed are EA XA ADPCM" is only the `XA\0\0` subset.
  **PARTLY**: `Sound.dat` holds **1680** records of type `0x2026960B`, with magics `XA\0\0` ×1512, `vers` ×71,
  `XAJ\0` ×43, `XAI\0` ×32, `RIFF` ×21, `HIT!` ×1 (measured). The ~120 MB figure **MATCHES**.
- The SC4 audio managers row omits two class IDs. **PARTLY**: the scape manager is `0x29E2797F` and the sem
  manager is `0xAA355871` (audio.md §1.2).
- **IN6-2 ANSWERED.** The "about 80 further classes … *[unsettled: per-event handlers or emanation models]*"
  are **neither**. Measured: the director's constructor (`0x004EC710`) makes 92 `AddCls` calls in total — 9
  named managers plus **83** leaf classes, and the factories run `0x004EB080`–`0x004EC6F0`, wider than the
  `0x004EB260`–`0x004EC6F0` range given. The 83 are small tagged objects in three fixed sizes (16 × 8 bytes,
  31 × 16 bytes, 36 × 20 bytes), each with QI/AddRef/Release, two always-true predicates and one unique
  method (audio.md §11). The emanation models and event handlers are separate named classes.

---

## 6. Region view and region save/load (G1)

- `cSC4Nation` (`0xC9DC0477`; nation.md covers only `GetEconomyModifier`) lists the regions (`LoadRegions`
  `0x00497CF0`, the `Downloads` folder, `ResetTutorialCities` with the tutorial cities embedded as type
  `0x6A5B7BF5`, 7 records). `cSC4Region` (vtable `0x00A8FA40`, 11 KB) loads one region: `region.ini`
  (`[Regional Settings]` with `Name`, `Terrain type`, `Water Min`, `Water Max`; `LoadConfig` `0x004A6CD0`),
  `config.bmp` (the city layout) and the `City - *.sc4` files (`LoadCitiesFromDirectory` `0x004A58D0`);
  `CreateRegion` (`0x004A67B0`) makes a new one.
- `cSC4WinRegionScreen` (`0xEA659793`, 21 KB; region tuning "Region UI Tuning Parameters"
  `0x6534284A/0x690F693F/0xAA383BFE`), `cSC4WinRegionView` (`0x2BA6BB97`, 10 KB: one tile per city drawn from the
  city's region-view thumbnail, the PNG records `0x8A2482B9/0x4A2482BB` in every save, written by
  `cSC4City::WriteRegionViewThumbnail` `0x00459B20`), clouds, signposts, scroll. `LaunchCity` (`0x007AF4B0`)
  enters a city. Region messages: PreRegionInit `0xABB5BB44`, PostRegionInit `0xCBB5BB45`, PreRegionShutdown
  `0x8BB5BB46`, PostRegionShutdown `0x8BB5BB4B`.
- The regional-city record and the deal flow between cities are specified (foundations §11, neighbors §9;
  `cSC4RegionalCity` `0x6A037C24`). Not specified: `config.bmp` → city rectangles, the region tile drawing,
  `region.ini`, new-region and delete-city flows, city obliteration (`cSC4ObliterateCity`, tuning
  "ObliterateCity" `0x7A4A8458/0xCA5A576F`).

---

## 7. Corrections and answers for existing specs

- **save-records Q5 (the unnamed classes)** is fully answered; save-records §3.2 and §12 are updated:
  - `0xA9C741B3` is `cSC4ModelMaker`: its `GetGZCLSID` is `0x0048EA60`, its factory `GZCOM_CreatecSC4ModelMaker`
    is registered by the SC4 director, and `cSC4View3DWin::DoLoad` resolves the View3D record's reference with this
    ID (`0x007EC01C`).
  - `0xEA0A5ACF` is `cSC4PipeConnectionOccupant` (`GetGZCLSID` `0x00648040`) and `0xCA15ED6B` is
    `cSC4SubwayConnectionOccupant` (`GetGZCLSID` `0x00652A70`); both derive from
    `cSC4UndergroundConnectionOccupant` (constructor `0x00654F00` called from `0x006483E0` and `0x00652C50`).
  - `0xC9B84E10` is `cSTETerrainView3D` (already answered in terrain.md §13.2).
- `0x0B8DA7DB` is `cSTESurfaceWater` (`GetGZCLSID` `0x0073CD41`; created by `cSC4City::CreateCityComponents`
  `0x0045AA78`). `0xE9C6262A` is the camera (§3.3).
- `re/names_windows.tsv`: `0x0087E0A0` returns the **OpenGL** driver's ID, not the DirectX driver's (§3.1).

---

## 8. Where the simulation reaches the view (headless limits)

A headless runner (S0) must replace or stub these calls. The 3D view is reached through one global (`0x00B43DD0`,
176 code references). Its users on the **simulation** side:

| User | Refs | Use *[inferred from the call sites; to be confirmed by the S0 spec]* |
|---|---|---|
| `cSC4ZoneDeveloper` | 2 | `Init` fails without the view (zones §1.2, §13; the known headless blocker) |
| `cSC4DispatchManager` | 2 | follows a dispatch with the camera / view state |
| `cSC4DisasterLayer` | 1 | `GoToNextDisaster` and disaster UI |
| `cSC4NetworkTool` | 2 | previews and cost display |
| `cSC4AutomataManager`, `cSC4Automaton`, `cSC4Vehicle`, `cSC4Helicopter`, `cSC4FireTruck`, `cSC4SimFireCrew` | 4 + 5 | the visible area decides where automata live |
| occupants: pylon (8), underground connection (8), construction (6), signpost (4), placeable object (4), retaining wall (2+1), power pole, burnt building, text decal, lot base texture, cell texture | ≈ 37 | model creation for the current zoom and rotation |
| `cSTEVolcanoLava`, `cSC4DynamicParticleEffect`, `cSC4EffectsManager` | 4 | effect placement |
| Lua extensions `camera_jump`, `camera_zoom` (`0x0040B01C`) | 2 | tutorial and advisor camera moves (lua-host §8) |

**Animation tick users** (§2.4): the lot developer, the prop developer, the terrain, the fire disaster and the
automata controller manager tick on the wall clock through `cSC4AnimationTickManager`, not on game days. A headless
runner must drive them (deterministic pacing: ARCHITECTURE "After the port").

**Threads.** Besides the main thread the original runs the sound library's mixer thread (`0x00869B10`) and
`cRZThread` workers (`0x009208CB`, `0x0092093B`), among them the model background loader *[inferred]*; one of them
is a candidate for the unidentified concurrent reader of foundations §19.2 *[unsettled]*.

---

## 9. Proposed spec order and lane split

Ranked by what a minimal standalone executable that loads a city, renders it statically and runs the simulation
needs first (§10). Sizes are the reading effort (§0).

| Rank | Spec (proposed file) | Covers | Reading | Needed by |
|---|---|---|---|---|
| 1 | **Standalone host** (`host.md`) | service registry, tick and idle order, director hooks as far as FreeCity4 uses them, the message pump between frames, timers, the animation tick manager and its users (§2.2–§2.4, §8) | ≈ 30 KB | S0 |
| 2 | **City object and life cycle** (`city.md`) | `cSC4City` own fields and its `0x89D9B9EE` record, `CreateCityComponents` order, `NewCity` / `LoadCity` / `SaveCity` / `CloseCity` message sequence, the View3D record as an opaque round trip (save-records Q4) | ≈ 25 KB | S0 |
| 3 | **Camera and projection** (`camera.md`) | `cSC4CameraControl`, `cS3DCamera`, the zoom and rotation tables, world↔screen, picking rays, view messages (§3.3) | ≈ 37 KB | S1 |
| 4 | **Terrain view** (`terrain-view.md`) | mesh, texture choice per terrain type, slope, height and season, city edge cliffs, water surface and colour, the terrain tuning exemplars (§3.4) | ≈ 50 KB | S1 |
| 5 | **Model maker and draw order** (`render-models.md`) | which model each occupant gets at a zoom and rotation, lot base textures, foundations and retaining walls, network pieces, placement transforms, draw passes and translucency, the render properties that change the image (§3.2, §3.5) | ≈ 70 KB | S1 |
| 6 | **Lighting and shadows** (`lighting.md`) | time-of-day colours, night lights, shadow projection, terrain shading (§3.6) | ≈ 35 KB | S3 |
| 7 | **UI script format, fonts and cursors** (`ui-scripts.md`, `fonts-cursors.md`) | the script grammar, each control's property vocabulary and look, MXF fonts, cursor records (§4.1–§4.2); decide interpret vs own UI first | ≈ 100 KB | S4 |
| 8 | **City view and tools** (`city-view.md`) | panels, menus, catalogs, tool behaviour on top of the specified back ends (§4.4) | ≈ 260 KB | S4 |
| 9 | **Query tool and info windows** (`query-and-views.md`) | the query field tables, data views (the DataView exemplars), graphs, budget window, news ticker (§4.5–§4.6) | ≈ 300 KB | S5 |
| 10 | **Region** (`region.md`) | region folder, `region.ini`, `config.bmp`, region view, new/delete/import, obliteration (§6) | ≈ 50 KB | S5 |
| 11 | **Audio formats and managers** (`audio.md`) | XA, AEMS banks, Hit tracks, the radio, the ambience and event rules (§5) | ≈ 210 KB | S5 |
| 12 | **Effects and automata drawing** (`effects.md`) | effect directory, particle systems, automata sprites and movement (§3.7, §3.9) | ≈ 250 KB | S3 |
| 13 | Shell, intro video (MAD), preferences, dialogs | §4.3, §4.7 | ≈ 50 KB | S6 |

**Parallel lanes** (none depends on another's text):

- **re-m6a: host and city** (ranks 1–2). Unblocks the headless runner; also closes save-records Q4.
- **re-m6b: camera and models** (ranks 3 and 5).
- **re-m6c: terrain view and lighting** (ranks 4 and 6).
- **re-m6d: UI data formats** (rank 7: UI scripts, MXF fonts, cursors, key maps). Pure data formats, like
  graphics-formats.md; `engine/formats` can read them before any UI exists.
- Later: city view and tools, info windows, region, audio, effects.

Not to be specified: F7 (web/HTML), F8 (developer tools), the backing store and dirty rectangles, the three
graphics drivers, the 2D blitters, the Game of Life easter egg.

---

## 10. Minimal standalone path

Each milestone is usable and testable on its own. The simulation cores already exist; the work is the host, the
city object and the presentation.

- **S0: headless runner.** `fc4-headless <install> <city.sc4> --days N [--save out.sc4]`. Loads the install's
  packages in the original order (data-formats §3.1), opens the save (foundations §3, save-records, lot-records),
  builds every city component from FreeCity4's cores (the 53 references of foundations §3.6; the classes still
  original in plug-in mode, save-records §3.2, get cores or exact opaque round trips), runs the simulator clock
  with deterministic pacing (the animation tick driven by the pacing, not the wall clock; no view: §8), and writes
  the save. *Needs:* specs 1–2; cores, or exact opaque round trips, for every class still original in plug-in mode
  (save-records §3.2 less the cores listed in PROGRESS; most are already specified). *Tests:* load → save is byte-identical on the 417-save
  sweep where nothing ran; a saved city loads in the original game; month-end budget and population against a
  same-process shadow run of the original.
- **S1: static render to an image.** `fc4-render <install> <city.sc4> --zoom Z --rotation R out.png`: terrain,
  water, lot textures, buildings, props, flora and networks at one zoom and rotation, fixed daylight, no shadows,
  no automata. *Needs:* specs 3–5, FSH/S3D readers (done). *Tests:* compare with the original's snapshot of the
  same city and camera (the in-game snapshot path `cSC43DRender::TakeSnapshot`, driven by the harness).
- **S2: a window and a camera.** Scroll, zoom 1–5, rotate, pick a cell; the simulation runs in the background at
  the three speeds; a minimal FreeCity4-drawn overlay shows date, funds and population. *Needs:* spec 3 complete,
  fonts or a system font.
- **S3: a living picture.** Lighting and night (spec 6), shadows, automata sprites, effects (spec 12) as far as the
  look needs.
- **S4: tools.** Query, zone and dezone, bulldoze, roads and other networks, plop a lot, through the specified
  back ends; the UI decision of §4.1 is taken before this milestone.
- **S5: the game's panels and the region.** Budget, data views, graphs, advisors and news, preferences, the region
  view with save and load across cities, audio.
- **S6: full replacement.** Intro and splash screens, every dialog, launcher integration (the GOG play task can
  point at FreeCity4's executable with the same `-CPUCount:1 -f` / `-w` arguments): FreeCity4 starts without
  `SimCity 4.exe` (ARCHITECTURE M6+ exit).

## 11. Open questions

- IN6-1: the exact role of each entry of the camera control's five-entry table and of the two default angles
  (§3.3).
- IN6-2: which of the about 80 unnamed audio classes are event handlers and which are emanation models (§5).
- IN6-3: the reader of the 12 records of type `0x9D796DB4` in `Sound.dat` (§5).
- IN6-4: which game thread is the concurrent reader of foundations §19.2 (§8).
- IN6-5: whether the season changes the terrain textures (§3.4).
