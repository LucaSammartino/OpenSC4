# SimCity 4 Deluxe 1.1.641 (Windows) - Prior Art for Reverse Engineering

Compiled 2026-10-12. Target: `SimCity 4.exe` 1.1.641 (GOG/Steam "digital" build), image base `0x00400000`, 32-bit x86, MSVC 7.1 (VC2003) + Intel C++ 6 for some libs, STLport 4.6.

Where this file disagrees with `docs/specs/`, the spec wins.

Companion files (generated, same folder):
- `prior_art_vtables.md` - slot/offset tables for 44 GZCOM interfaces (from gzcom-dll headers). Superseded by `docs/abi/`; it repeats SDK errors, so do not use it for slot lookups.
- `prior_art_exemplar_props.md` - ~430 simulator / RCI / growth / budget tuning exemplar property IDs.

Confidence legend: **[V]** verified from source code that patches/calls the 641 exe (usually with byte-signature checks or version gating); **[S]** from SDK headers that are exercised by working plugins; **[W]** wiki/forum knowledge; **[?]** uncertain / my inference.

---

## 0. Target identification

| Item | Value | Source |
|---|---|---|
| GOG 1.1.641.0 exe SHA-256 | `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b` (matches `re/decomp_702a80.md` for `bin\SimCity4.exe`) | [KillaVolt README](https://github.com/KillaVolt/SimCity-4-Deluxe-GOG-Decompiled), [PortCellar notes](https://github.com/dotcom07/PortCellar/blob/main/modules/simcity-4/docs/windows-1.1.641.md) |
| Version sentinel used by every community DLL | byte at `0x006E5000`: `0x8B`=610/613, `0xFF`=638, `0x24`=640, `0x0F`=**641** (fallback when VERSIONINFO is unavailable) [V] | [sc4-dll-utilities SC4VersionDetection.cpp](https://github.com/0xC0000054/sc4-dll-utilities) |
| Embedded PDB | `c:\SimCity4\SC4000Projects\Ep1\out\ReleaseSRT\SimCity 4.pdb`, GUID `46e5cc8b-14c3-4e10-826f-ca9752664c89`, age 7. Not on MS symbol server. | [KillaVolt docs/coverage.md](https://github.com/KillaVolt/SimCity-4-Deluxe-GOG-Decompiled/blob/main/docs/coverage.md) |
| RTTI | Only STL classes carry RTTI (your `rtti_classes.txt` shows `_STL@@` types); game classes must be named via Mac symbols / CLSIDs. [?] | local |

Static libraries identified by Ghidra FID DBs in sc4-ghidra-symbols: libcmt (VC2003 RTM 13.10.3077), STLport 4.6 (vc7, static mt), Lua 5.0, libpng 1.0.5 + zlib 1.1.3 (ICC6). [V]

---

## 1. Must-have resources (ranked)

| # | Resource | What it gives you | URL |
|---|---|---|---|
| 1 | **0xC0000054/sc4-ghidra-symbols** | `SimCity4.gdt` Ghidra type archive: interfaces **and concrete classes + vftable structs** extracted from the **debug-symbol-bearing Mac ports** (PPC and Intel). Contains e.g. `cSC4Simulator`, `cSC4TrafficSimulator`, `cSC4DemandSimulator`, `cSC4Demand`, `cSC4PathFinder`, `cSC4SimGrid`, `cSC424HourClock`, `cSC4TractDeveloper`, `cSC4ZoneManager`, `vftable_cSC4*`. Also FID DBs (libcmt, STLport, Lua5, libpng, zlib), DirectX7/Lua5/Gimex GDTs, and a script to set vtable funcs to `__thiscall`. | https://github.com/0xC0000054/sc4-ghidra-symbols |
| 2 | **nsgomez/gzcom-dll** (actively maintained, last commit 2026-10-11; 291 headers) | The de-facto SDK: all `cISC4*`/`cIGZ*` interface headers incl. every sub-simulator, `GZServPtrs.h` (service IDs), `GZCLSIDDefs.h` (259 CLSIDs machine-extracted from the Mac symbol table), several struct layouts (`cSC4TripData`, `SC4Preferences`, `cSC4LotConfigurationObject`...). memo33/gzcom-dll is an older fork (2024, 122 headers). | https://github.com/nsgomez/gzcom-dll |
| 3 | **0xC0000054/sc4-gzids** | `allGZIDs.h`: ~800 message IDs, CLSIDs, IIDs, service IDs, command IDs (from OrSpeeder's list). | https://github.com/0xC0000054/sc4-gzids |
| 4 | **KillaVolt/SimCity-4-Deluxe-GOG-Decompiled** (Sept 2026) | Ghidra project + full pseudocode for this exact exe hash; a **named Mac "Rev A" Intel reference decompilation (67,188 functions, 94,951 method/address records)**; 358 Windows-to-Mac function pair leads via shared strings. See caveats in section 9. | https://github.com/KillaVolt/SimCity-4-Deluxe-GOG-Decompiled |
| 5 | **NAMTeam/nam-dll** | Most sim-relevant patches: **traffic pathfinder / traffic-sim internals** (addresses + partial structs `cSC4PathFinder`, `cSC4PathInfo`, `TrafficSimCellConnections`, `TravelTypeAttr`). | https://github.com/NAMTeam/nam-dll |
| 6 | **0xC0000054/sc4-cpu-options** (SimTickBudget.cpp, from caspervg/scd3d11) | `cSC4Simulator::OnTick` address and its tick-budget loop. | https://github.com/0xC0000054/sc4-cpu-options |
| 7 | **0xC0000054/sc4-lot-network-scoring-test** | `cSC4TractDeveloper` field layout (0x130 bytes) and `CandidateLot` struct; ScoreCandidateLots hook points. | https://github.com/0xC0000054/sc4-lot-network-scoring-test |
| 8 | SC4Devotion wiki (archived) | Savegame subfile type IDs (= class IDs), SimGrid format, Date subfile, Exemplar property list, Traffic Simulator guide. Live wiki is behind Cloudflare; use web.archive.org. | https://web.archive.org/web/2024/https://wiki.sc4devotion.com/index.php?title=Savegame |

Other repos surveyed (mostly UI/render, fewer sim addresses): 0xC0000054/{sc4-query-ui-hooks, sc4-resource-loading-hooks, sc4-more-demand-info, sc4-regional-supply-demand, sc4-city-date-sync, sc4-clear-pollution, sc4-growify, sc4-no-kick-out, sc4-park-and-ride-ordinance, sc4-data-view-extensions, sc4-bulldoze-extensions, sc4-advanced-water-management, sc4-extended-terrain, sc4-lua-extensions, SC4MessageViewer, sc4-gzcom-dll-template}, nsgomez/sc4fix, nsgomez/sc4macinjector, memo33/{submenus-dll, sc4-3d-camera-dll, sc4-thumbnail-fix-dll}, caspervg/{sc4-render-services, sc4-zoning-extensions, sc4-season-jumper, sc4-effects-extensions, scd3d11, sc4-imgui-advanced-lotplop}, sebamarynissen/sc4 (JS savegame parser). No repository named "sc4-imgui" or "buildingdevelopmentsim" exists; ImGui lives in caspervg/sc4-render-services.

---

## 2. Mac version with debug symbols - YES

- **The Aspyr Mac ports (PowerPC and Intel x86) shipped with debug symbols.** 0xC0000054 extracted class/interface info from both into `SimCity4.gdt` [V: repo README]. KillaVolt counted 564,265 Intel and 69,550 PPC symbol/debug records in the "Mac Rev A" build; no source-line info. [V]
- Source for the Mac binary: the Mac "Rev A" patch in the Simtropolis "SC4 Maxis files" archive (https://community.simtropolis.com/sc4-maxis-files/ - 403 to bots; exact file title not verified). [W]
- The Intel Mac build (Steam/Origin, app 24780) exports Itanium-mangled C++ symbols; SC4MacInjector resolves `__ZN13cGZCOMLibrary4LoadEv` and `__Z5GZCOMv` by name. Mac App Store version is a Wineskin wrapper of the Windows exe (useless for symbols). [V: https://github.com/nsgomez/sc4macinjector]
- Caveat: Mac uses GCC/CodeWarrior ABI. Single-inheritance member offsets usually match MSVC but **vtable layouts differ** (Itanium has no MSVC overload reordering; GCC emits 2 destructor slots; multiple-inheritance thunks differ). Use Mac names/types as hints, map Windows functions by strings/structure (KillaVolt's `compare_build_strings.py` does this).

---

## 3. Hard-coded 1.1.641 addresses (Windows VA, image base 0x400000)

### 3.1 Simulation core / loop

| VA | What | Source | Conf |
|---|---|---|---|
| `0x00703E70` | `cSC4Simulator::OnTick` | sc4-cpu-options/src/SimTickBudget.cpp | [V] |
| `0x00703EF1` | budget clamp inside OnTick: `cmp eax,0Fh; jg; mov eax,0Fh; mov [esi+70h],eax` (bytes `83 F8 0F 7F 05 B8 0F 00 00 00 89 46 70`) | same | [V] |
| `0x00703EFE` | rejoin point (`mov al,[esi+24h]`) | same | [V] |
| `0x005EEFF0` | `cSC4PropOccupant::SetSimulatorDateRange(interval,duration,month,day)`, thiscall, `ret 10h`; prologue `83 EC 10 53 8B 5C 24 18` | caspervg/sc4-season-jumper SeasonalPropVisibilityFix.cpp | [V] |

`cSC4Simulator::OnTick` structure (from SimTickBudget.cpp comments) [V]:
```
budget_ms = 1000 / speedDivisor - elapsed;   // floor 15 ms
this->+0x70 = max(budget_ms, 15);
do { SimulateTimePassage(); SimulateAgents(); SimulateIdleTime(); }
while (cRZTimer::GetElapsedTime(this + 0x190) < this->+0x70);
```
Vanilla budget is 33-66 ms per frame (sc4-cpu-options README). `cISC4Simulator::SetMaxMillisecondsPerTick` (vtbl +0x7C) likely feeds this. [?]

### 3.2 Traffic simulator / pathfinder (NAM DLL)

| VA | What | Conf |
|---|---|---|
| `0x006D7EBE`/`0x006D7EC5`, `0x006D7EF0`/`0x006D7EF7` | `cSC4PathFinder::FindNearestStandardDest` search-rect setup (west / north neighbor cases) | [V] |
| `0x006D79BF` -> `0x006D79C5` (NC branch) / `0x006D7A42` (lot branch) | pathfinder `AtGoal` neighbor-connection check; reads `[esi+0x48]` originatingEdge | [V] |
| `0x006D95B1` -> `0x006D95B6` | `cSC4PathFinder::FindPath`: sets `checkPreviousTripConnection` (byte at `[esp+13h]`), normally car-only | [V] |
| `0x0071AB60` -> `0x0071ABAB` | traffic sim `DoConnectionsChanged` (per-cell connection rebuild) | [V] |
| `0x0070EAE7` -> `0x0070EAF1` / skip `0x0070EB59` | `FullSetPathNodeConnection` multi-level travel-mode check | [V] |
| `0x0053E350` | `cSC4PathInfo::GetPathsNearEndPoint` (thiscall) | [V] |
| `0x00AB3174` | `kNetworkManagerTypeToTransitNetworkType[]` (uint32 per eNetworkType; road=0x03, rail=0x04) | [V] |
| `0x00B09628` | `sTravelTypeAttrs[9]` (stride 0x14; `networkFlags` at +0xC, `pathType` at +0x10) | [V] |
| `0x004D09D1` -> `0x004D0B4A` | `GetNetworkOccupantTrafficInfo` (query UI route text) | [V] |
| `0x00B43D0C` | **global `cISC4OccupantManager*`** (pointer-to-pointer) | [V] |

### 3.3 Growth / zoning / lots

| VA | What | Source | Conf |
|---|---|---|---|
| `0x00705863` | `cSC4TractDeveloper::ScoreCandidateLots` inject (network weighting); continue `0x00705901` | sc4-lot-network-scoring-test | [V] |
| `0x007050E0` -> `0x007050F2` | `cSC4TractDeveloper` FindNetworks | same | [V] |
| `0x004BE6DC`, `0x004BE70B` | GetLotFacingStreetCount network-mask push / test (`0x449`, `0x408`) | nam-dll DirtRoadAccess.cpp | [V] |
| `0x006C1726`, `0x006C1BD1` | ferry-terminal / CalculateRoadAccess network-mask pushes (`0x449`) | same | [V] |
| `0x007E6DA0` | InvokeZoningTool (`cISC4View3DWin*`, ZoneType) | caspervg/sc4-zoning-extensions | [V] |
| `0x00732BF0` (call site `0x00733954`) | zone developer DetermineLotSize | same | [V] |
| `0x00730D30` | zone developer LayInJogStreets | same | [V] |
| `0x0072C0E0` | zone developer ClearHighlight | same | [V] |
| zone developer field offsets | internal cSC4NetworkTool at `+0x48`, street interval at `+0x94` | same | [V] |
| `0x004C0430` | `cSC4ViewInputControlPlaceLot` ctor (size 0x158) | sc4-query-ui-hooks | [V] |

### 3.4 Network tool (useful for network-aware sims)

`cSC4NetworkWorldCache::GetCell` `0x00647A20`; `cSC4NetworkTool::GetCellInfo` `0x00633220`; `InsertEqualityConstraint` `0x00636000`; `InsertSmoothnessConstraint` `0x00636030`; `GetCrossSections` `0x00636360`; `cSC4VertexHtConstraintSatisfier::InsertSlopeConstraint` `0x0065FF60`; `GetFirstNetworkTypeFromFlags` `0x00623E60`; `sNetworkTypeInfo[]` `0x00B452C8` (stride 0x114); tile-conflict rules list `0x00B466D0`; `GetIntersectionRule` `0x00625380`; `DoAutoComplete` `0x006097E0`; `InsertIsolatedHighwayIntersection` `0x0062CD50`; `cSC4NetworkTool` ctor `0x0062C5F0` (caspervg says size 0x368; NAM's partial struct is 0x270 - likely different (sub)class [?]); `DrawNetworkLine` `0x0063AF40`; `PlaceAllSegments` `0x0063BDC0`. [V: nam-dll NetworkStubs.h, caspervg ZoneDeveloperHooks.cpp]

### 3.5 Misc (UI, render, persistence)

| VA | What | Source |
|---|---|---|
| `0x005D3DE0` | `cSC4COMSerializer::SaveClassObjects` (Prop Pox fix hook); `0x005D32D0` map `operator=`; `0x0090CF63` free | nsgomez/sc4fix (641 branch) |
| `0x0087B3D1` | call to `FreeLibrary` in `cGZCOMLibrary::Free` | sc4fix |
| `0x0065EE3E`, `0x0096DA1D` | puzzle-piece / transit-enabled crash fix sites | sc4fix |
| `0x0070244A/57/62` | FPS-limit bytes | sc4-disable-fps-limits |
| `0x004D3C30` DoQueryDialog; `0x004D5E30` GetBuildingOccupantTipInfo; `0x004CF8A0` GetNetworkOccupantTipInfo; `0x004D7464` cSC4ViewInputControlQuery::DoMessage; `0x0090F574` RZString Sprintf | | sc4-query-ui-hooks |
| `0x007A0F30` ... `0x007A62E0` | cSC4WinMapView (data views: Sint8/Sint16 grid draw paths `0x7A3240`, `0x7A331D`) | sc4-data-view-extensions |
| `0x005FD480` cSCPropertyHelp::GetPropertyValue; `0x0078EF30` nSC4UI::CreateCatalogItemList | | memo33/submenus-dll |
| `0x005FB7B0` SCLuaHelp::LoadResourceScript; `0x008AED10` Lua ScriptArgSetup; `0x005FC020` Lua helper (cISCLua) | | sc4-lua-extensions, KillaVolt |
| `0x007CB530` renderer Draw; `0x007C7370` DrawStaticView; `0x007C7830` DrawDynamicView; `0x007FFF10` cS3DCamera::Project ... (~60 render/camera thunks) | | caspervg/sc4-render-services DrawService.cpp, S3DCameraService.cpp |
| `0x007CCF80` cSC4CameraControl::UpdateCameraPosition (yaw at +0x118) | | memo33/sc4-3d-camera-dll |
| `0x00A92570` water-building efficiency jump table; `0x004C3CA0` SetPerformanceMeterValue | | sc4-advanced-water-management |
| `0x00B4BC64` terrain texture prefix `SC4String`; `0x0074FC20` cSTETerrain::StaticShutdown | | sc4-extended-terrain |
| `0x00A901D8..0x00A901FC` cSC4ViewInputControlDemolish vtable slots | | sc4-bulldoze-extensions |

**No public source hard-codes the cISC4App / cISC4City / cISC4Simulator globals.** Plugins obtain them through the framework: `RZGetFrameWork()->GetSystemService(srvid, iid)` -> `cISC4App::GetCity()` (vtbl +0x94) -> `cISC4City::GetSimulator()` etc. Find the globals in Ghidra via the service registration of `kSC4SimulatorSysServiceId 0x46956659` / app srvid `0x66`. [?]

---

## 4. IDs: service IDs, interface IIDs, class IDs

### 4.1 System services (`cRZSysServPtr<T, IID, SRVID>`, GZServPtrs.h) [S]

| Interface | IID | Service ID |
|---|---|---|
| cISC4Simulator | `0x8695664E` | `0x46956659` (= kSC4SimulatorSysServiceId) |
| cISC4App | `0x26CE01C0` | `0x00000066` |
| cIGZMessageServer2 | `0x652294C7` | `0x04FA845B` |
| cIGZMessageServer | `0xC1085722` | `0x64063387` |
| cIGZPersistResourceManager | `0x656B8EFC` | `0x056B906E` |
| cISC4AnimationTickManager | `0x86AD10ED` | `0x86AD10EC` |
| cISCExemplarCache | `0xA52EA0C8` | `0x2533DF21` |
| cISC4GameDataRegistry | `0xE898ED03` | `0xEBE0E860` |
| cIGZLuaScriptServer | `0x4BD10129` | `0x4BD10133` |
| cIGZCommandServer | `0xCB90253A` | `0xEB903A32` |
| cIGZAllocatorService | `0x3AE4BEAB` | `0x3AE4BEA3` |
| 24-hour clock (sys service) | - | `0xA7E7F92A` (kSC424HourClockSysServiceId) |
| cGZMessageQueueManager2 | - | `0x64FA8434` |

(Full list of ~37 services in gzcom-dll `GZServPtrs.h`.)

### 4.2 Interface IIDs (GZIID) [S]

| Interface | IID |
|---|---|
| cIGZUnknown | `0x00000001` |
| cISC4City | `0x66BA4DB5` |
| cISC4App | `0x26CE01C0` |
| cISC4Simulator | `0x8695664E` |
| cISC4ZoneManager | `0x67652433` |
| cISC4LotManager | `0x681905BD` |
| cISC4PowerSimulator | `0xE9E7BFF5` |
| cISC4Occupant / BuildingOccupant / NetworkOccupant | `0x26BB767E` / `0x87DDA3A9` / `0xA821EF94` |
| cISC4Lot | `0xE818FCED` |
| cIGZMessageTarget2 / cIGZMessage2Standard | `0x452294AA` / `0x65297976` |
| cIGZFrameWorkHooks / cIGZCOMDirector | `0x03FA40BF` / `0xA21EE941` |
| cIGZSerializable | `0xE4FDA3D4` |
| cISCPropertyHolder / cISCProperty / cISCResExemplar | `0x25216283` / `0x452160EC` / `0xA52160F5` |
| cISC4DBSegment / IStream / OStream | `0x499899DA` / `0x4999A8F5` / `0x6999A8FB` |
| cIGZPersistDBSegment / SerialRecord | `0x656B8F0C` / `0xC56B8F08` |
| cISCLua / cIGZLua5 | `0xEA10C4A2` / `0x6B2811A6` |

**No public IIDs for the other sub-simulator interfaces** (traffic, demand, residential, BDS, pollution, ...). They are reached through `cISC4City` getters, not QueryInterface, so community never needed them. They have since been extracted from the exe: `docs/specs/foundations.md` §3.6 lists every city component's IID and CLSID.

### 4.3 Class IDs (GZCLSID) - also the savegame subfile Type IDs [S][W]

| Class | CLSID / savegame type | Class | CLSID |
|---|---|---|---|
| cSC4App | `0xCA263481` | cSC4City | `0x8990C372` |
| **cSC4Simulator** ("Date subfile") | `0x2990C1E5` | cSC424HourClock | `0xA7E7F929` |
| cSC4TrafficSimulator | `0x6990C1AA` | cSC4TrafficNetworkMap | `0x6A0F82B2` |
| cSC4DemandSimulator | `0x2990BFF2` | cSC4Demand | `0x8990C12D` |
| cSC4ResidentialSimulator | `0x4990C013` | cSC4CommercialSimulator | `0x8990C080` |
| cSC4IndustrialSimulator | `0xA990C022` | cSC4BuildingDevelopmentSimulator | `0x8990BE7A` |
| cSC4TractDeveloper | `0x2990C142` | cSC4GrowthDeveloper | `0x0990C138` |
| cSC4LotDeveloper | `0xA990BFE0` | cSC4ZoneDeveloper | `0x498F9B01` |
| cSC4ZoneManager | `0x298F9B2D` | cSC4LotManager | `0xA990625B` |
| cSC4OccupantManager | `0x098F964D` | cSC4LandValueSimulator | `0xE990C18E` |
| cSC4PollutionSimulator | `0x8990C065` | cSC4CrimeSimulator | `0x8990C09A` |
| cSC4PoliceSimulator | `0x6990C04F` | cSC4FireProtectionSimulator | `0x2990C05A` |
| cSC4FlammabilitySimulator | `0xE990C045` | cSC4PowerSimulator | `0x0990C005` |
| cSC4PlumbingSimulator | `0x0990C075` | cSC4BudgetSimulator | `0xE990BE01` |
| cSC4AuraSimulator | `0x0990BDD8` | cSC4NeighborsSimulator | `0xC990C0A4` |
| cSC4OrdinanceSimulator | `0xA990C08E` | cSC4WeatherSimulator | `0x2990C1BC` |
| cSC4FloraSimulator | `0xC990C034` | cSC4CivicBuildingSimulator | `0x0A413481` |
| cSC4MySimAgentSimulator | `0x29E37DEC` | cSC4WaterPathFinder | `0xABDF0343` |
| cSC4SimGrid{Bool8,Uint8,Sint8,Uint16,Sint16,Uint32,Sint32,Uint64,Sint64,Float32,Float64} | `0x49B9E601`..`0x49B9E60B` (base `0x49B9E600`) | cSC4NetworkManager | `0xC990BD46` |

Searching the exe's `.rdata` for these DWORDs locates each class factory / serializer, which is the fastest way to label the concrete simulator classes.

---

## 5. Messages (cIGZMessageServer2 / cIGZMessage2Standard) [S]

Simulator clock messages - note `msgID = 0x66956810 + cISC4Simulator::eAgentType`:

| ID | Name | AgentType |
|---|---|---|
| `0x66956810` | kSC4MessageSimBegin (also listed as SimOneShot) | 0 |
| `0x66956811` | kSC4MessageSimEmergencyTime | 1 |
| `0x66956812` | kSC4MessageSimNewMinute | 2 |
| `0x66956813` | kSC4MessageSimNewHour | 3 |
| `0x66956814` | **kSC4MessageSimNewDay** | 4 |
| `0x66956815` | kSC4MessageSimNewWeek | 5 |
| `0x66956816` | **kSC4MessageSimNewMonth** | 6 |
| `0x66956817` | kSC4MessageSimNewYear | 7 |
| `0x66956818` | kSC4MessageSimSpecificDay | 8 |
| `0x66956819` | kSC4MessageSimSpecificTime | 9 |
| `0x6695681A` | kSC4MessageSimIdle | 10 |
| `0xAA7FB7E0` / `0x4A7FB7E2` / `0x4A7FB807` | SimPauseChange / SimHiddenPauseChange / SimEmergencyPauseChange | |
| `0x4A7FB7E9` | kSC4MessageSimSpeedChange | |
| `0xEC320E96` | kMessageSetSimSpeed | |
| `0x6716A4E5` / `0x6716A4E6` | SeasonChange / WeatherCycleCompleted | |
| `0xC53D10AA` | **ReloadTunableValues** (Data1=group, Data2=instance of tuning exemplar, e.g. traffic `0xE7E2C2DB`/`0xC9133286`) [V: sc4-park-and-ride-ordinance] | |
| `0x426840A0` | kSC4MessageActiveDemandChanged | |
| `0x26D31EC0..C5` | Pre/PostCityInit, Pre/PostCityShutdown, CityEstablished, CityObliterated | |
| `0x26C63340..45` | PreLoad, Load, PostLoad, PreSave, Save, PostSave | |
| `0x28E9397A..7D` | Pre/PostAppInit, Pre/PostAppShutdown | |

Sub-simulator "cycle complete"/"map changed" messages: Aura `0x2A09B21A`, FireProtection `0x68F54CE0`, Flammability `0x08F415FE`, Police `0xA8DAF510`, Ordinance `0xC8EA5018`, CrimeMapChanged `0x88EA4DFF`, LandValueMapChanged `0x87CC75B3`, TrafficMapChanged `0x69247DC7`, PowerMapChanged `0x6A07C2EE`, Pollution Air/Water/Garbage/Radiation change `0xFA6DD0BB`/`0x5A6DD1AA`/`0xDA6DD2CB`/`0xDA6DD35D`, ZoningChange `0x07699455`, LotDeveloperNotification `0x4C04DE69`, FundsChanged `0x0772FAD4`. Full list: sc4-gzids `allGZIDs.h`.

Message `PostCityInit` payload: Void1 = city ptr, IGZUnknown = city, Data2 = 1 for a loaded city, Data3 = 0 [V: park-and-ride].

---

## 6. Interface method orders (key excerpts; full tables in `prior_art_vtables.md`)

Offsets = byte offsets into the vtable (slot*4). Slots 0-2 are QueryInterface/AddRef/Release. Overloads: see MSVC caveat in the appendix.

### cISC4City - simulator getters [S]
| off | method | off | method |
|---|---|---|---|
| 0x084 | GetLotManager | 0x0C8 | **GetSimulator** |
| 0x088 | GetOccupantManager | 0x0CC | GetAuraSimulator |
| 0x090 | GetZoneManager | 0x0D0 | GetBudgetSimulator |
| 0x098 | GetNetworkManager | 0x0D4 | GetBuildingDevelopmentSimulator |
| 0x0A0 | GetTrafficNetwork | 0x0D8 | GetCommercialSimulator |
| 0x0DC | GetCrimeSimulator | 0x0E0 | **GetDemandSimulator** |
| 0x0E4 | GetFireProtectionSimulator | 0x0E8 | GetFlammabilitySimulator |
| 0x0EC | GetFloraSimulator | 0x0F0 | GetIndustrialSimulator |
| 0x0F4 | GetLandValueSimulator | 0x0F8 | GetNeighborsSimulator |
| 0x0FC | GetOrdinanceSimulator | 0x100 | GetPlumbingSimulator |
| 0x104 | GetPoliceSimulator | 0x108 | GetPollutionSimulator |
| 0x10C | GetPowerSimulator | 0x110 | GetResidentialSimulator |
| 0x114 | **GetTrafficSimulator** | 0x118 | GetWeatherSimulator |
| 0x11C | GetMySimAgentSimulator | 0x124 | GetCivicBuildingSimulator |
| 0x12C | GetZoneDeveloper | 0x13C | GetLotDeveloper / 0x140 GetTractDeveloper |
| 0x158 | Get24HourClock | 0x164/0x168 | SizeX/SizeZ ; 0x16C/0x170 CellWidthX/Z ; 0x174/0x178 CellCountX/Z |

### cISC4App [S]
`+0x94 GetCity`, `+0x90 GetRegionalCity`, `+0x8C GetRegion`, `+0x98 GetPreferences`, `+0x84 GetCheatCodeManager`, `+0x58 SetDebugFunctionalityEnabled`.

### cISC4Simulator [S]
```
0x0C Init            0x10 Shutdown          0x14 GetSimStartDate(cIGZDate&)
0x18 GetSimDate(y,m,d,doy,wd)  0x1C GetSimDate()   (verified compiled order: simulator.md §5.6, foundations §16-§17)
0x20 GetSimDateNumber  0x24 Pause  0x28 HiddenPause  0x2C EmergencyPause
0x30 Resume  0x34 HiddenResume  0x38 EmergencyResume
0x3C IsPaused 0x40 IsHiddenPaused 0x44 IsEmergencyPaused 0x48 IsAnyPaused
0x4C AddAgent(cIGZMessageTarget2*, eAgentType, const cIGZString& name, eAgentFlags)
0x50..0x5C RemoveAgent x2 / RemoveAllAgents x2 (overloads)
0x60 EnumerateAgentsByName 0x64 GetAgentEnabled 0x68 SetAgentEnabled
0x6C GetSimSpeed 0x70 SetSimSpeed 0x74 GetSimTime 0x78 SetSimTime
0x7C SetMaxMillisecondsPerTick 0x80 GetAnimationTimeDilation 0x84 SetCityEstablished
eAgentFlags: 1 = EnabledForUnestablishedCities, 2 = EnabledForEstablishedCities
```
Sub-simulators register as **agents** on the main simulator: `AddAgent(target, AgentTypeSimNewMonth, "name", flags)`; the simulator then delivers the matching `kSC4MessageSim*` message to `target->DoMessage`. `EnumerateAgentsByName` + `GetAgentEnabled/SetAgentEnabled` let you list and toggle the built-in sim agents by name at runtime, which is a cheap way to map agent name -> sub-simulator -> tick frequency.

### cISC4DemandSimulator / cISC4Demand [S]
```
DemandSimulator: 0x0C Init 0x10 Shutdown 0x14 GetSimulatorType 0x18 GetDemand(demandID, demandIndex)
  0x1C UpdateOccupantEffects 0x20 CalculateJobsPerUnitOfDemand(float[4],id,idx)  (writes jobs[1..3], jobs[0] untouched; demand.md §5)
  0x24 GetJobsBySensus(type) 0x28 GetNeutralTaxRate 0x2C GetLocalPopulationSummary(map&)
Demand: 0x14 SimulationBegin 0x18 GetId 0x1C SetId 0x20 QuerySupplyValue 0x24 QueryDemandValue
  0x28 QueryNewSupply 0x2C QueryNewDemand 0x30 QueryActiveDemandValue 0x34 QueryEconomyModifier
  0x38 QueryActiveDemandMax 0x3C QueryActiveDemandMin 0x40 AddToSupplyValue 0x44 AddToDemandValue
  0x48 SetSupplyValue 0x4C SetDemandValue 0x50/0x54 SetActiveDemandMax/Min
  0x58/0x5C Get/SetEconomyModifier 0x60/0x64 Get/SetTaxModifier 0x68/0x6C Get/SetDemandCap(SC4Percentage)
  0x70/0x74 Get/SetRegionUse 0x78 EndOfCycle 0x7C..0x84 Debug lock (no-ops)
```
Demand IDs: R$ `0x1010`, R$$ `0x1020`, R$$$ `0x1030`, Cs$ `0x3110`, Cs$$ `0x3120`, Cs$$$ `0x3130`, Co$$ `0x3320`, Co$$$ `0x3330`, I-R `0x4100`, I-D `0x4200`, I-M `0x4300`, I-HT `0x4400`. Demand index `0x20000` = city totals (used with GetDemand). [V: sc4-more-demand-info, sebamarynissen/sc4 enums.ts]

### cISC4TrafficSimulator (first slots) [S]
`0x18 CreatePathFinder`, `0x1C SetupPathFinderForLot`, `0x20 GetAirPollutingTrafficMap`, `0x24 GetCommercialTrafficMap`, `0x28 GetCongestionMap`, `0x30 GetTrafficEdgeDensity(x,z,TravelType,eveningCommute)`, `0x34 GetTripLengthMap`, `0x38 GetTripScale`, `0x5C GetCapacity`, `0x60 GetTravelTimeRatio`, `0x68 GetTravelStrategyPercentages`, `0x6C GetFreightScalingFactor`, `0x90 AreLotsConnected`, `0x9C..0xA8 subnetwork queries`. TravelType: Walk 0, Car 1, Bus 2, PassengerTrain 3, FreightTruck 4, FreightTrain 5, Subway 6, ElevatedTrain 7, Monorail 8. `TrafficEdgeDensity` is 0x42 bytes, `usage` at +0x20.

### cISC4SimGrid<T> [S]
`0x0C Init 0x10 Shutdown 0x14 GetInstanceID 0x18 SetInstanceID 0x1C GetCellValue(x,z) 0x20 GetAverageValueInCellRect 0x24 SetTractSize 0x28 GetTractSize 0x2C GetTractShift 0x30/0x34 GetTractCountX/Z 0x38/0x3C GetTractWidthX/Z 0x40/0x44 GetOneOverTractWidth 0x48 TractIsInBounds 0x4C PositionToTract 0x50/0x54 TractCorner/CenterToPosition 0x58 GetTractValue 0x5C GetAverageValueInTractRect 0x60/0x64 GetGridData (const/non-const) 0x68 SetTractValue 0x6C SetTractValues`.

Grids exposed: Pollution `GetPollutionGrid(type)` -> `SimGrid<short>`; LandValue intrinsic/total `SimGrid<uint8>` + slope `SimGrid<float>`; Crime `SimGrid<uint8>`; Residential EQ/HQ `SimGrid<float>`, population `SimGrid<uint16>`, proximity `SimGrid<uint8>` per wealth; Traffic air-polluting / commercial / congestion / trip-length `SimGrid<uint8>`; ZoneManager zone grid `SimGrid<int8>`; GrowthDeveloper desirability `SimGrid<int16>`, capacity / max-capacity `SimGrid<uint16>`, population `SimGrid<int16>`.

---

## 7. Known concrete structures

### cSC4TractDeveloper (0x130) [V: sc4-lot-network-scoring-test]
```
+0x00 vtable  +0x04 cIGZMessageTarget2 vptr  +0x08 cIGZSerializable vptr
+0x0C initialized  +0x10 refCount  +0x14 pMS2  +0x18 pCity
+0x1C pBuildingDevelopmentSim  +0x20 pLandValueSim  +0x24 pLotConfigurationManager
+0x28 pLotDeveloper  +0x2C pLotManager  +0x30 pPlumbingSim  +0x34 pPowerSim
+0x38 pTrafficSim  +0x3C pZoneManager
-- tunables (BDS exemplar 0x6534284A/0xE7E2C2DB/0xE8DA7677) --
+0x40 float overbuild (0x47E2C500)       +0x44 float reoccupyThresholds[2] (0x47E2C501)
+0x4C float redevelopSizeThreshold        +0x50 float demolitionCosts[2]
+0x58 float newLotCosts[2]                +0x60 float occupancyVariation
+0x64 float minVacancyRange[2]            +0x6C u8 kickOutLowerWealth (0x47E2C540)
+0x70 SC4Vector<u32> availableStyles      +0x7C int yearsBetweenStyles
+0x80 cISC4GrowthDeveloper* pCurrentDeveloper
+0x118 SC4Vector<u32> activeStyles  +0x124 currentStyleIndex  +0x128 yearsPassed  +0x12C u8 changeStylesEveryNYears
```
`CandidateLot` (0xA4): developmentType, pExistingLot, lotLocationX/Z, facing, lotSizeX/Z(u8), lotSet (std::set<cISC4Lot*>), totalNonVacantCapacity, totalVacantCapacity, growthStage, wealth, **float score @0x34 (lower = higher priority)**, SC4Rect lotBounds @0x38, connection vectors street/road/avenue/rail @0x70/0x7C/0x88/0x94. Maxis bug: commercial network scoring unused (typo); street weighting never implemented.

### Simulator object fragments
- `cSC4Simulator`: `+0x24` byte flag read after budget; `+0x70` tick budget (ms); `+0x190` cRZTimer. [V]
- `cSC4PathFinder` (partial): `+0x0C cISC4TrafficSimulator*`, `+0x18 SC4Rect start`, `+0x28 SC4Rect nearestStandardDest`, `+0x48 SC4Rect originatingEdge` (neighbor routes), `+0x80 tripDestinationFilter (0/2/3)`, `+0x84 destinationFlags`, `+0xB8 SC4Point cityCellCount`. [V: nam-dll NetworkStubs.h]
- `cSC4PathInfo`: `+0x1C SC4HashMap<u32, tPath>` keyed `(pathType<<24)|(entry<<8)|exit`; `tPath = {SC4Vector<cS3DVector3> coords; float length}`. PathType: Car 1, Sim 2, Train 3, Subway 4, LightRail 6, Monorail 7. [V]
- `TrafficSimCellConnections` (0x2E, per cell): `u16 entryExitConnectionsPerTravelType[9]` (4x4 entry/exit bits), `u16 exitTravelTypePerSidePerEntryTravelType[9]`, `u16 priorCarConnectionsPerEntrySide[4]`, `u8 wormholeFlags` (1 subway station, 2 tunnel, 4 ferry). [V]
- `TravelTypeAttr` (0x14): `?, float, float, u32 networkFlags, u32 pathType`. TransitNetwork enum: Pedestrian 0, Car 1, Train 2, Subway 3, LightRail 4, Monorail 5. [V]
- `cSC4TripData` (0x2C, commute record in lot subfile): pathData1/2 vectors, `commuterCount @0x18`, transitType @0x1D, `destinationTileX/Z (int16) @0x20` (-1 = W/N neighbor, cellCount = E/S neighbor, `0xDFA8` = lost path), `float tripLength @0x24`. [S]
- `cSC4NetworkCellInfo` (0xBC), `cSC4NetworkTool` (partial, 0x270), `nSC4Networks::cIntRule` (0xFC) - see nam-dll NetworkStubs.h.
- `SC4Preferences` (0x1734) with `simulatorPreferences` at +0x470 (one float). [S]

---

## 8. Simulation behaviour (community knowledge)

### 8.1 Loop & time [V/W]
- Main loop: framework `OnTick` -> `cSC4Simulator::OnTick (0x703E70)` -> repeated `SimulateTimePassage / SimulateAgents / SimulateIdleTime` until the per-frame ms budget (derived from current speed) is consumed. Sub-simulators are **agents** hooked to minute/hour/day/week/month/year/idle events (section 6).
- Speeds: Main Simulator exemplar `0x89590E6D` "Simulator Speeds" = **milliseconds per sim-day for slow, medium, fast**; `0x89590E6E` animation time dilation per speed (paused, slow, med, fast, emergency/drive). `SetSimSpeed(0)` + `Pause()` is used to pause; DLLs use `SetSimSpeed(2)` as "fastest" [V caspervg]. The integer is a speed index: 0 = turtle, 1 = llama, 2 = cheetah; pause is a separate counter (simulator.md §6).
- **24-hour clock** (`cSC424HourClock`, CLSID `0xA7E7F929`, srvid `0xA7E7F92A`) is a *visual* day/night clock decoupled from sim days: `0x897E709F` time magnification (real time -> clock time), `0x09B73421/22` night begin/end hour. Interface exposes visual vs logical hour/minute, time magnification min/max, IsNight.
- **Sim date**: `cIGZDate` counts **Julian Day Numbers**; new cities start at JDN 2451545 = 1 Jan 2000 (simulator.md §2.1; the wiki's `0x00256858` = 2451544 is off by one). Date subfile (type `0x2990C1E5`, cSC4Simulator, version 4, 0x5B bytes) stores hours/day (24), day-of-year, week, month, year, JDN. [W: Date Subfile page]
- Sub-simulator periods known from tuning: Crime `0xE8EC2720` "Crime Update Period" (days); Police update period (obsolete); most decay/strike/pollution delta properties are "per month"; traffic simulator recomputes commutes **about every four game months** (destination finder then pathfinder, sequential). [W]

### 8.2 Grids [W/S]
- SimGrid savegame record: dataID, resolution (1,2,4,...,64 tiles per cell), resolution power, xSize, zSize, values column-major. Known data IDs: `0x41800000` zone overview (Sint8), `0x49D5BC86` power, `0x49D5BC40` air pollution (Int16), `0x69D5C40E`/`0x69D5C3AC` traffic floats. City tile = 16 m; city cell counts 64/128/256.
- Grid resolution is not read the same way by every simulator (flammability is always 1 cell, protection uses a fixed 2); see each simulator's spec and `docs/specs/data-formats.md` §7.4 for the readers. "Tracts" for the destination finder are 4x4 tiles (RippleJet). **No authoritative public table of per-grid resolutions was found** - read them from a savegame's SimGrid subfiles (sebamarynissen/sc4 can parse them) or from the tuning exemplars.

### 8.3 Traffic simulator [W: "A Guide to the Operation of the Traffic Simulator", by z]
- Two phases: **destination finder** (assigns jobs; scans tracts W->E, N->S columns; aggressive pre-pruning that estimates reachability, heavily rewarding subway/rail/highway proximity) then **pathfinder** (modified **A\***, confirmed by Maxis). "Nearest Destination Attractiveness" `0x4A678060` is the A\* heuristic weight; Maxis default 0.09, "perfect" ~0.005797; lower = more exploration/CPU.
- Commute time limit `0xA92356BA` (minutes; Maxis default 6); speeds in km/h per network per travel type `0x491332E6..EF`; congestion curve `0xA92356B9` (capacity ratio -> speed multiplier); capacity `0xA92356B3`; trip start costs `0xA92356BC`, `0xEA8C3CDB` (mass-transit pref), `0xCAD64136` (car pref); travel strategy % by wealth `0x4953E8A3..A6` (MassTransit/Car/Fastest); freight scaling `0x49A2E8BE`; trip-length map is 0-255 where 255 = max commute time (`0xCA76013B` display multiplier). Commute result feeds desirability (short/medium/long; neighbor commutes always "long"; unemployed count as very long).
- Visible traffic is automata only; the traffic sim is effectively a desirability generator.

### 8.4 Demand / growth [W/S]
- RCI demand is **data-driven by Demand exemplars** (one per demand ID) linked by properties: `0x47BB3F10/11` active-demand min/max, `0x47BB3F20/21` initial supply/demand, `0x47BB3F30` Drives, `0x47BB3F31` Satisfies, `0x47BB3F32` Contributes, `0x47BB3F40` Limits growth, `0x47BB3F41` Caps demand (by % depleted), `0x47BB3F50` Sums, `0x47BB3F60` Region use.
- Demand Simulator tuning: neutral tax rate vs population `0x4A1F38B5`, tax modifier vs variance `0x4A1F38B6`, days of tax penalty `0x4A1F38B7`, cap relief for connections/airports/seaports/trips `0x4A1F38C5..CA`, average salary per demographic `0x8A63F2A1`. National economy demand modifiers vs month `0x4A1F62A0 + i` for the i-th listed demand ID (plain addition, so the 11th-13th are `0x4A1F62AA..AC`; demand.md §11.2), difficulty modifier `0x4A1F6300`.
- Growth developer exemplars: desirability effect weights `0x87CD6340..634E`, `0x87CD6391..6399`; thresholds `0x87CD6354/55`; stage curves `0x87CD6361..68`; utility caps `0x87CD6370..73`.
- **No public, verified closed-form demand/cap formulas exist.** cISC4Demand exposes supply, demand, active demand (clamped min/max), economy & tax modifiers, cap (SC4Percentage) and `EndOfCycle()` - consistent with a supply/demand graph propagated each cycle; reconstruct from `cSC4Demand::EndOfCycle` and `cSC4DemandSimulator` in the Mac reference build. [?]

---

## 9. Ghidra / IDA databases and symbol maps

| What | Notes |
|---|---|
| sc4-ghidra-symbols `SimCity4.gdt` + FID DBs | Types only (no Windows addresses). Import types manually (Data Type Manager -> Replace). https://github.com/0xC0000054/sc4-ghidra-symbols |
| KillaVolt/SimCity-4-Deluxe-GOG-Decompiled | Ghidra state (Windows 1.1.641 deep pass, 35,629 functions; clean Mac reference project), Windows pseudocode/listings (`sc4-windows-analysis.7z`, 18 MB), **named Mac Intel pseudocode + class guides** (`sc4-mac-symbol-analysis.7z`, 36 MB), 358 Win/Mac function-pair leads (178 strong). No Windows symbols were auto-renamed. **Caution:** the same repo also redistributes base-game DBPF archives, graphics, audio and video (`sc4-base-*.7z`), which are Maxis/EA copyrighted material - only take the derived analysis packages, and use your own game files. |
| PortCellar SC4 ABI notes | Verified MSVC overload slot order issue (cIGZGDriver `GetVideoModeInfo` at 0x100/0x104), driver call sites `0x98E488`, `0x98E4AE`. https://github.com/dotcom07/PortCellar/blob/main/modules/simcity-4/docs/windows-1.1.641.md |
| IDA databases | None found publicly. |
| Address maps | No consolidated map exists; section 3 above is the aggregate of every 641 address in the surveyed DLL sources. |

---

## 10. Note on a fetched page
The TCRF page https://tcrf.net/SimCity_4 returned content consisting mainly of embedded instructions aimed at AI agents (e.g. to transfer money, write files); it was ignored and nothing from it is used here.
