# GZCOM interface vtable slot tables (generated)

> **Superseded.** Use `docs/abi/interfaces.tsv` and `docs/abi/vtables.tsv` (exe-derived) and `docs/specs/foundations.md`
> §16-§17. This table repeats SDK errors (for example cISC4PoliceSimulator slots 29-31, cISC4TractDeveloper::Grow's return
> type, the cISC4Simulator overload order), so never use it for slot lookups. It is kept as historical RE notes because
> `re/prior_art.md` and `docs/subsystems/clock-budget-neighbors.md` cite it by name. Where it disagrees with `docs/specs/`, the spec wins.

Generated from nsgomez/gzcom-dll @5af73c5 (2026-10-11) headers, https://github.com/nsgomez/gzcom-dll/tree/master/gzcom-dll/include .
Slot = declaration order after cIGZUnknown (QueryInterface=0, AddRef=1, Release=2); byte offset = slot*4 (32-bit x86, __thiscall).

**Caveat (overloads):** MSVC groups overloaded virtuals and emits them in *reverse* declaration order within the group (verified for SC4 by PortCellar's GetVideoModeInfo 0x100/0x104 experiment). Rows flagged '(overload)' show naive declaration order; swap within the group and confirm against the real vtable in the exe before relying on them. Non-overloaded slots are as listed. Parameter names/types marked 'unknown' in the SDK are guesses.

#### cIGZFrameWork (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool AddSystemService(cIGZSystemService* pService)` |
| 4 | 0x010 | `bool RemoveSystemService(cIGZSystemService* pService)` |
| 5 | 0x014 | `bool GetSystemService(uint32_t srvid, uint32_t riid, void** ppService)` |
| 6 | 0x018 | `bool EnumSystemServices(void* enumerator, cIGZUnknown* pUnknown, uint32_t dwUnknown)` |
| 7 | 0x01C | `bool AddHook(cIGZFrameWorkHooks* pHooks)` |
| 8 | 0x020 | `bool RemoveHook(cIGZFrameWorkHooks* pHooks)` |
| 9 | 0x024 | `bool AddToTick(cIGZSystemService* pService)` |
| 10 | 0x028 | `bool RemoveFromTick(cIGZSystemService* pService)` |
| 11 | 0x02C | `bool AddToOnIdle(cIGZSystemService* pService)` |
| 12 | 0x030 | `bool RemoveFromOnIdle(cIGZSystemService* pService)` |
| 13 | 0x034 | `int32_t GetOnIdleInterval(void)` |
| 14 | 0x038 | `bool SetOnIdleInterval(int32_t nInterval)` |
| 15 | 0x03C | `bool OnTick(uint32_t dwTimeElapsed)` |
| 16 | 0x040 | `bool OnIdle(uint32_t unknown1)` |
| 17 | 0x044 | `bool IsTickEnabled(void)` |
| 18 | 0x048 | `void ToggleTick(bool bTick)` |
| 19 | 0x04C | `void Quit(int32_t nQuitReason)` |
| 20 | 0x050 | `void AbortiveQuit(int32_t nQuitReason)` |
| 21 | 0x054 | `cIGZCmdLine* CommandLine(void)` |
| 22 | 0x058 | `bool IsInstall(void)` |
| 23 | 0x05C | `cIGZCOM* GetCOMObject(void)` |
| 24 | 0x060 | `FrameworkState GetState(void)` |
| 25 | 0x064 | `void* GetDebugStream(void)` |
| 26 | 0x068 | `int32_t DefaultDebugStream(void)` |
| 27 | 0x06C | `int32_t DebugStream(void)` |
| 28 | 0x070 | `bool SetDebugStream(void* pIGZDebugStream)` |
| 29 | 0x074 | `bool SetDebugLevel(int32_t nLevel)` |
| 30 | 0x078 | `int32_t GetDebugLevel(void)` |
| 31 | 0x07C | `cIGZOStream* StdOut(void)` |
| 32 | 0x080 | `cIGZOStream* StdErr(void)` |
| 33 | 0x084 | `cIGZIStream* StdIn(void)` |
| 34 | 0x088 | `bool GetStream(tStreamType type, uint32_t riid, void** ppvObj)` |
| 35 | 0x08C | `bool SetStream(tStreamType type, cIGZUnknown* pUnknown)` |
| 36 | 0x090 | `bool SetApplication(cIGZApp* const pIGZApp)` |
| 37 | 0x094 | `cIGZApp* const Application(void)` |
| 38 | 0x098 | `void ReportException(char const* szExcText)` |
| 39 | 0x09C | `cIGZExceptionNotification* ExceptionNotificationObj(void)` |

#### cIGZMessageServer2 (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool MessageSend(cIGZMessage2* pMessage)` |
| 4 | 0x010 | `bool MessagePost(cIGZMessage2* pMessage, bool bHighPriority)` |
| 5 | 0x014 | `bool AddNotification(cIGZMessageTarget2* pTarget, uint32_t dwMessageID)` |
| 6 | 0x018 | `bool RemoveNotification(cIGZMessageTarget2* pTarget, uint32_t dwMessageID)` |
| 7 | 0x01C | `bool GeneralMessagePostToTarget(cIGZMessage2* pMessage, cIGZMessageTarget2* pTarget)` |
| 8 | 0x020 | `bool CancelGeneralMessagePostsToTarget(cIGZMessageTarget2* pTarget)` |
| 9 | 0x024 | `bool OnTick(uint32_t unknown1)` |
| 10 | 0x028 | `uint32_t GetMessageQueueSize(void)` |
| 11 | 0x02C | `cIGZMessageServer2* SetAlwaysClearQueueOnTick(bool bToggle)` |
| 12 | 0x030 | `uint32_t GetRefCount(void)` |
| 13 | 0x034 | `cIGZMessage2* CreateMessage(uint32_t clsid, uint32_t msgid, void** ppData)` |

#### cIGZMessageTarget2 (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool DoMessage(cIGZMessage2* pMessage)` |

#### cIGZMessage2 (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Create(uint32_t dwTypeIID, void** ppUnknown) const` |
| 4 | 0x010 | `uint32_t GetType(void) const` |
| 5 | 0x014 | `void SetType(uint32_t dwType)` |
| 6 | 0x018 | `bool operator==(cIGZMessage2 const& msgOther) const` |
| 7 | 0x01C | `bool operator<(cIGZMessage2 const& msgOther) const` |

#### cIGZMessage2Standard (base: cIGZMessage2; first own slot = 8)

| idx | vtbl off | method |
|---|---|---|
| 8 | 0x020 | `bool GetHasData1(void) const` |
| 9 | 0x024 | `intptr_t GetData1(void) const` |
| 10 | 0x028 | `void* GetVoid1(void) const` |
| 11 | 0x02C | `void SetData1(intptr_t ulData)` |
| 12 | 0x030 | `void SetVoid1(void* pData)` |
| 13 | 0x034 | `bool GetHasData2(void) const` |
| 14 | 0x038 | `intptr_t GetData2(void) const` |
| 15 | 0x03C | `void* GetVoid2(void) const` |
| 16 | 0x040 | `void SetData2(intptr_t ulData)` |
| 17 | 0x044 | `void SetVoid2(void* pData)` |
| 18 | 0x048 | `bool GetHasData3(void) const` |
| 19 | 0x04C | `intptr_t GetData3(void) const` |
| 20 | 0x050 | `void* GetVoid3(void) const` |
| 21 | 0x054 | `void SetData3(intptr_t ulData)` |
| 22 | 0x058 | `void SetVoid3(void* pData)` |
| 23 | 0x05C | `bool GetHasData4(void) const` |
| 24 | 0x060 | `intptr_t GetData4(void) const` |
| 25 | 0x064 | `void* GetVoid4(void) const` |
| 26 | 0x068 | `void SetData4(intptr_t ulData)` |
| 27 | 0x06C | `void SetVoid4(void* pData)` |
| 28 | 0x070 | `bool GetHasString(void) const` |
| 29 | 0x074 | `bool GetString(uint32_t dwMaxLen, void** ppStringOut) const` |
| 30 | 0x078 | `void SetString(cIGZString* pString)` |
| 31 | 0x07C | `bool GetHasIGZUnknown(void) const` |
| 32 | 0x080 | `cIGZUnknown* GetIGZUnknown(void) const` |
| 33 | 0x084 | `void SetIGZUnknown(cIGZUnknown* pObj)` |
| 34 | 0x088 | `bool GetHasExtra(void) const` |
| 35 | 0x08C | `uint32_t GetExtra(uint32_t dwLen, void** ppOut) const` |
| 36 | 0x090 | `void SetExtra(cIGZSerializable* pSerializable)` |

#### cISC4App (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool OnIdle(uint32_t unknown1)` |
| 4 | 0x010 | `bool RunMessageServerPump(uint32_t dwMinMessages, uint32_t dwMaxMessages, uint32_t dwMaxTime)` |
| 5 | 0x014 | `bool RunMessageServer2Pump(uint32_t dwMinMessages, uint32_t dwMaxMessages, uint32_t dwMaxTime)` |
| 6 | 0x018 | `bool RequestNewCity(cRZAutoRefCount<cISC4RegionalCity>* pCity)` |
| 7 | 0x01C | `bool RequestLoadCity(void)` |
| 8 | 0x020 | `bool RequestCloseCity(bool bShowConfirmPrompt)` |
| 9 | 0x024 | `bool RequestSaveCity(bool bShowNotif, bool bFastSave)` |
| 10 | 0x028 | `bool RequestQuit(bool bShowDialog, bool bSaveFirst)` |
| 11 | 0x02C | `bool RequestQuitFromRegion(bool bShowDialog)` |
| 12 | 0x030 | `bool RequestGoToRegionView(bool bShowDialog)` |
| 13 | 0x034 | `bool LoadCity(SC4String const& szSaveFilePath, cRZAutoRefCount<cISC4RegionalCity>* pRegionalCity)` |
| 14 | 0x038 | `bool CloseCity(void)` |
| 15 | 0x03C | `bool SaveCity(bool bFastSave)` **(overload: MSVC slot order may differ)** |
| 16 | 0x040 | `bool SaveCity(cIGZString const& szSaveFilePath, bool bFastSave)` **(overload: MSVC slot order may differ)** |
| 17 | 0x044 | `bool SavePreferences(void)` |
| 18 | 0x048 | `bool EnableFullGamePauseOnAppFocusLoss(bool bEnable)` |
| 19 | 0x04C | `bool ApplyVideoPreferences(SC4VideoPreferences const& sPreferences)` |
| 20 | 0x050 | `bool GetAutoVideoPreferences(SC4VideoPreferences& pPreferencesOut)` |
| 21 | 0x054 | `bool GetDebugFunctionalityEnabled(void)` |
| 22 | 0x058 | `void SetDebugFunctionalityEnabled(bool bEnabled)` |
| 23 | 0x05C | `bool GetPopupDialogsEnabled(void)` |
| 24 | 0x060 | `void SetPopupDialogsEnabled(bool bEnabled)` |
| 25 | 0x064 | `int32_t GetAppState(void)` |
| 26 | 0x068 | `cIGZWin* GetMainWindow(void)` |
| 27 | 0x06C | `bool GetAppName(cIGZString& szNameOut)` |
| 28 | 0x070 | `bool GetAppIniFileName(cIGZString& szPathOut)` |
| 29 | 0x074 | `bool GetAppIniFilePath(cIGZString& szPathOut)` |
| 30 | 0x078 | `bool GetAppPreferencesFileName(cIGZString& szPathOut)` |
| 31 | 0x07C | `bool GetAppPreferencesFilePath(cIGZString& szPathOut)` |
| 32 | 0x080 | `cISC4FeatureManager* GetFeatureManager(void)` |
| 33 | 0x084 | `cIGZCheatCodeManager* GetCheatCodeManager(void)` |
| 34 | 0x088 | `cISC4Nation* GetNation(void)` |
| 35 | 0x08C | `cISC4Region* GetRegion(void)` |
| 36 | 0x090 | `cISC4RegionalCity* GetRegionalCity(void)` |
| 37 | 0x094 | `cISC4City* GetCity(void)` |
| 38 | 0x098 | `SC4Preferences* GetPreferences(void)` |
| 39 | 0x09C | `SC4NewCitySpecification* GetNewCitySpecification(void)` |
| 40 | 0x0A0 | `intptr_t GetDebugConsole(void)` |
| 41 | 0x0A4 | `cIGZGimexFactory* GetGimexFactory(void)` |
| 42 | 0x0A8 | `cISCStringDetokenizer* GetStringDetokenizer(void)` |
| 43 | 0x0AC | `cISLWinLocationSaver* GetWinLocationSaver(void)` |
| 44 | 0x0B0 | `cISC4RenderProperties* GetRenderProperties(void)` |
| 45 | 0x0B4 | `SC4String* GetExceptionReportsDirectory2(void) const` |
| 46 | 0x0B8 | `intptr_t GetGlyphTextureManager(void)` |
| 47 | 0x0BC | `cISCLua* GetLuaInterpreter(void)` |
| 48 | 0x0C0 | `SC4String* GetExceptionReportsDirectory3(void) const` |
| 49 | 0x0C4 | `intptr_t GetTutorialRegistry(void)` |
| 50 | 0x0C8 | `bool IsRunFirstTimeAfterInstall(void)` |
| 51 | 0x0CC | `bool GetAppDirectory(cIGZString& szPathOut)` |
| 52 | 0x0D0 | `bool GetCDAppDirectory(cIGZString& szPathOut)` |
| 53 | 0x0D4 | `bool GetDataDirectory(cIGZString& szPathOut, int32_t index)` |
| 54 | 0x0D8 | `bool GetCDDataDirectory(cIGZString& szPathOut, int32_t index)` |
| 55 | 0x0DC | `bool GetPluginDirectory(cIGZString& szPathOut)` |
| 56 | 0x0E0 | `bool GetCDPluginDirectory(cIGZString& szPathOut)` |
| 57 | 0x0E4 | `bool GetSkuSpecificDirectory(cIGZString& szPathOut)` |
| 58 | 0x0E8 | `bool GetUserDataDirectory(cIGZString& szPathOut)` |
| 59 | 0x0EC | `bool GetUserPluginDirectory(cIGZString& szPathOut)` |
| 60 | 0x0F0 | `bool GetRegionsDirectory(cIGZString& szPathOut)` |
| 61 | 0x0F4 | `bool GetMySimDirectory(cIGZString& szPathOut)` |
| 62 | 0x0F8 | `bool GetAlbumDirectory(cIGZString& szPathOut)` |
| 63 | 0x0FC | `bool GetHTTPCacheDirectory(cIGZString& szPathOut)` |
| 64 | 0x100 | `bool GetTempDirectory(cIGZString& szPathOut)` |
| 65 | 0x104 | `bool GetExceptionReportsDirectory(cIGZString& szPathOut)` |
| 66 | 0x108 | `bool GetTestScriptDirectory(cIGZString& szPathOut)` |
| 67 | 0x10C | `bool AddDynamicLibraryByName(cIGZString const& sName, cIGZString* pBasePath, bool bIgnoreINI)` |
| 68 | 0x110 | `bool AddDynamicLibraryByPath(cIGZString const& sPath, bool bIgnoreINI)` |
| 69 | 0x114 | `bool RegisterShutdownCallbackFunction(ShutdownCallback pfCallback, void* pUnknown)` |
| 70 | 0x118 | `bool UnregisterShutdownCallbackFunction(ShutdownCallback pfCallback, void* pUnknown)` |

#### cISC4City (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `uint32_t GetCitySerialNumber(void) const` |
| 6 | 0x018 | `void SetCitySerialNumber(uint32_t dwSerial)` |
| 7 | 0x01C | `uint32_t GetNewOccupantSerialNumber(void)` |
| 8 | 0x020 | `bool GetOriginalLanguageAndCountry(uint32_t& dwLanguage, uint32_t& dwCountry) const` |
| 9 | 0x024 | `bool GetLastLanguageAndCountry(uint32_t& dwLanguage, uint32_t& dwCountry)` |
| 10 | 0x028 | `void GetCitySaveFilePath(cIGZString& szPath) const` |
| 11 | 0x02C | `void SetCitySaveFilePath(cIGZString const& szPath)` |
| 12 | 0x030 | `void GetCityName(cIGZString& szName) const` |
| 13 | 0x034 | `void SetCityName(cIGZString const& szName)` |
| 14 | 0x038 | `bool GetCityNameChanged(void) const` |
| 15 | 0x03C | `void SetCityNameChanged(bool bToggle)` |
| 16 | 0x040 | `void GetMayorName(cIGZString& szName) const` |
| 17 | 0x044 | `void SetMayorName(cIGZString const& szName)` |
| 18 | 0x048 | `void GetCityDescription(cIGZString& szDescription) const` |
| 19 | 0x04C | `void SetCityDescription(cIGZString const& szDescription)` |
| 20 | 0x050 | `uint32_t GetBirthDate(void) const` |
| 21 | 0x054 | `void SetBirthDate(uint32_t dwDate)` |
| 22 | 0x058 | `bool GetEstablished(void) const` |
| 23 | 0x05C | `void SetEstablished(bool bEstablished)` |
| 24 | 0x060 | `DifficultyLevel GetDifficultyLevel(void) const` |
| 25 | 0x064 | `void SetDifficultyLevel(DifficultyLevel dwLevel)` |
| 26 | 0x068 | `void GetWorldPosition(float& fLongitude, float& fLatitude)` |
| 27 | 0x06C | `void SetWorldPosition(float fLongitude, float fLatitude)` |
| 28 | 0x070 | `float GetWorldBaseElevation(void)` |
| 29 | 0x074 | `void SetWorldBaseElevation(float fElevation)` |
| 30 | 0x078 | `WorldHemisphere GetWorldHemisphere(void)` |
| 31 | 0x07C | `cISC4Demolition* GetDemolitionUtility(void) const` |
| 32 | 0x080 | `cISC4HistoryWarehouse* GetHistoryWarehouse(void) const` |
| 33 | 0x084 | `cISC4LotManager* GetLotManager(void) const` |
| 34 | 0x088 | `cISC4OccupantManager* GetOccupantManager(void) const` |
| 35 | 0x08C | `cISC4PropManager* GetPropManager(void) const` |
| 36 | 0x090 | `cISC4ZoneManager* GetZoneManager(void) const` |
| 37 | 0x094 | `cISC4LotConfigurationManager* GetLotConfigurationManager(void) const` |
| 38 | 0x098 | `cISC4NetworkManager* GetNetworkManager(void) const` |
| 39 | 0x09C | `cISC4DispatchManager* GetDispatchManager(void) const` |
| 40 | 0x0A0 | `intptr_t GetTrafficNetwork(void) const` |
| 41 | 0x0A4 | `cISC4PropDeveloper* GetPropDeveloper(void) const` |
| 42 | 0x0A8 | `cISC4NetworkLotManager* GetNetworkLotManager(void) const` |
| 43 | 0x0AC | `cISC4AutomataManager* GetVehicleManager(void) const` |
| 44 | 0x0B0 | `cISC4AutomataManager* GetPedestrianManager(void) const` |
| 45 | 0x0B4 | `cISC4AutomataManager* GetAircraftManager(void) const` |
| 46 | 0x0B8 | `cISC4AutomataManager* GetWatercraftManager(void) const` |
| 47 | 0x0BC | `cISC4AutomataControllerManager* GetAutomataControllerManager(void) const` |
| 48 | 0x0C0 | `cISC4AutomataScriptSystem* GetAutomataScriptSystem(void) const` |
| 49 | 0x0C4 | `cISC4CitySituationManager* GetCitySituationManager(void) const` |
| 50 | 0x0C8 | `cISC4Simulator* GetSimulator(void) const` |
| 51 | 0x0CC | `cISC4AuraSimulator* GetAuraSimulator(void) const` |
| 52 | 0x0D0 | `cISC4BudgetSimulator* GetBudgetSimulator(void) const` |
| 53 | 0x0D4 | `cISC4BuildingDevelopmentSimulator* GetBuildingDevelopmentSimulator(void) const` |
| 54 | 0x0D8 | `cISC4CommercialSimulator* GetCommercialSimulator(void) const` |
| 55 | 0x0DC | `cISC4CrimeSimulator* GetCrimeSimulator(void) const` |
| 56 | 0x0E0 | `cISC4DemandSimulator* GetDemandSimulator(void) const` |
| 57 | 0x0E4 | `cISC4FireProtectionSimulator* GetFireProtectionSimulator(void) const` |
| 58 | 0x0E8 | `cISC4FlammabilitySimulator* GetFlammabilitySimulator(void) const` |
| 59 | 0x0EC | `cISC4FloraSimulator* GetFloraSimulator(void) const` |
| 60 | 0x0F0 | `cISC4IndustrialSimulator* GetIndustrialSimulator(void) const` |
| 61 | 0x0F4 | `cISC4LandValueSimulator* GetLandValueSimulator(void) const` |
| 62 | 0x0F8 | `cISC4NeighborsSimulator* GetNeighborsSimulator(void) const` |
| 63 | 0x0FC | `cISC4OrdinanceSimulator* GetOrdinanceSimulator(void) const` |
| 64 | 0x100 | `cISC4PlumbingSimulator* GetPlumbingSimulator(void) const` |
| 65 | 0x104 | `cISC4PoliceSimulator* GetPoliceSimulator(void) const` |
| 66 | 0x108 | `cISC4PollutionSimulator* GetPollutionSimulator(void) const` |
| 67 | 0x10C | `cISC4PowerSimulator* GetPowerSimulator(void) const` |
| 68 | 0x110 | `cISC4ResidentialSimulator* GetResidentialSimulator(void) const` |
| 69 | 0x114 | `cISC4TrafficSimulator* GetTrafficSimulator(void) const` |
| 70 | 0x118 | `cISC4WeatherSimulator* GetWeatherSimulator(void) const` |
| 71 | 0x11C | `cISC4MySimAgentSimulator* GetMySimAgentSimulator(void) const` |
| 72 | 0x120 | `cISC4DisasterLayer* GetDisasterLayer(void) const` |
| 73 | 0x124 | `cISC4CivicBuildingSimulator* GetCivicBuildingSimulator(void) const` |
| 74 | 0x128 | `cISC4ParkManager* GetParkManager(void) const` |
| 75 | 0x12C | `cISC4ZoneDeveloper* GetZoneDeveloper(void) const` |
| 76 | 0x130 | `cISC4SeaportDeveloper* GetSeaportDeveloper(void) const` |
| 77 | 0x134 | `cISC4AirportDeveloper* GetAirportDeveloper(void) const` |
| 78 | 0x138 | `cISC4LandfillDeveloper* GetLandfillDeveloper(void) const` |
| 79 | 0x13C | `cISC4LotDeveloper* GetLotDeveloper(void) const` |
| 80 | 0x140 | `cISC4TractDeveloper* GetTractDeveloper(void) const` |
| 81 | 0x144 | `cISC4AdvisorSystem* GetAdvisorSystem(void) const` |
| 82 | 0x148 | `cISC4TutorialSystem* GetTutorialSystem(void) const` |
| 83 | 0x14C | `cISTESurfaceWater* GetSurfaceWater(void) const` |
| 84 | 0x150 | `cISTETerrain* GetTerrain(void) const` |
| 85 | 0x154 | `cISC4EffectsManager* GetEffectsManager(void) const` |
| 86 | 0x158 | `cISC424HourClock* Get24HourClock(void) const` |
| 87 | 0x15C | `CitySizeType GetCitySizeType(void) const` |
| 88 | 0x160 | `bool SetSize(float fX, float fZ)` |
| 89 | 0x164 | `float SizeX(void) const` |
| 90 | 0x168 | `float SizeZ(void) const` |
| 91 | 0x16C | `float CellWidthX(void) const` |
| 92 | 0x170 | `float CellWidthZ(void) const` |
| 93 | 0x174 | `uint32_t CellCountX(void) const` |
| 94 | 0x178 | `uint32_t CellCountZ(void) const` |
| 95 | 0x17C | `void PositionToCell(float fX, float fZ, int& cX, int& cZ) const` |
| 96 | 0x180 | `void CellCornerToPosition(int cX, int cZ, float& fX, float& fZ) const` |
| 97 | 0x184 | `void CellCenterToPosition(int cX, int cZ, float& fX, float& fZ) const` |
| 98 | 0x188 | `bool LocationIsInBounds(float fX, float fZ) const` |
| 99 | 0x18C | `bool CellIsInBounds(int cX, int cZ) const` |
| 100 | 0x190 | `bool CellCornerIsInBounds(int cX, int cZ) const` |
| 101 | 0x194 | `void ToggleSimulationMode(void)` |
| 102 | 0x198 | `bool IsInCityTimeSimulationMode(void)` |
| 103 | 0x19C | `void EnableSave(void)` |
| 104 | 0x1A0 | `void DisableSave(void)` |
| 105 | 0x1A4 | `bool IsSaveDisabled(void)` |
| 106 | 0x1A8 | `void UIIncreaseLockCount(void)` |
| 107 | 0x1AC | `void UIDecreaseLockCount(void)` |
| 108 | 0x1B0 | `int32_t UIGetLockCount(void)` |
| 109 | 0x1B4 | `bool SaveObliterated(cIGZPersistDBSegment* pSegment)` |

#### cISC4Simulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool GetSimStartDate(cIGZDate& sDate)` |
| 6 | 0x018 | `cIGZDate* GetSimDate(void)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `void GetSimDate(uint32_t* year, uint32_t* month, uint32_t* day, uint32_t* dayOfYear, uint32_t* weekDay)` **(overload: MSVC slot order may differ)** |
| 8 | 0x020 | `int32_t GetSimDateNumber(void)` |
| 9 | 0x024 | `bool Pause(void)` |
| 10 | 0x028 | `bool HiddenPause(void)` |
| 11 | 0x02C | `bool EmergencyPause(void)` |
| 12 | 0x030 | `bool Resume(void)` |
| 13 | 0x034 | `bool HiddenResume(void)` |
| 14 | 0x038 | `bool EmergencyResume(void)` |
| 15 | 0x03C | `bool IsPaused(void)` |
| 16 | 0x040 | `bool IsHiddenPaused(void)` |
| 17 | 0x044 | `bool IsEmergencyPaused(void)` |
| 18 | 0x048 | `bool IsAnyPaused(void)` |
| 19 | 0x04C | `bool AddAgent(cIGZMessageTarget2* pAgent, eAgentType agentType, cIGZString const& szAgentName, eAgentFlags flags)` |
| 20 | 0x050 | `bool RemoveAgent(cIGZMessageTarget2* pAgent, eAgentType agentType)` **(overload: MSVC slot order may differ)** |
| 21 | 0x054 | `bool RemoveAgent(cIGZMessageTarget2* pAgent)` **(overload: MSVC slot order may differ)** |
| 22 | 0x058 | `bool RemoveAllAgents(void)` **(overload: MSVC slot order may differ)** |
| 23 | 0x05C | `bool RemoveAllAgents(eAgentType agentType)` **(overload: MSVC slot order may differ)** |
| 24 | 0x060 | `bool EnumerateAgentsByName(SC4Vector<SC4String>& sAgents)` |
| 25 | 0x064 | `bool GetAgentEnabled(cIGZString const& szAgentName)` |
| 26 | 0x068 | `bool SetAgentEnabled(cIGZString const& szAgentName, bool bEnabled)` |
| 27 | 0x06C | `int32_t GetSimSpeed(void)` |
| 28 | 0x070 | `bool SetSimSpeed(int32_t lSpeed)` |
| 29 | 0x074 | `int32_t GetSimTime(void)` |
| 30 | 0x078 | `bool SetSimTime(int32_t lTime)` |
| 31 | 0x07C | `bool SetMaxMillisecondsPerTick(uint32_t dwTime)` |
| 32 | 0x080 | `float GetAnimationTimeDilation(void)` |
| 33 | 0x084 | `bool SetCityEstablished(bool bEstablished)` |

#### cISC424HourClock (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `int32_t SetVisualTimeMode(int32_t nTimeMode)` |
| 6 | 0x018 | `int32_t GetVisualTimeMode(void)` |
| 7 | 0x01C | `float GetTimePercentage(void)` |
| 8 | 0x020 | `int32_t SetTimePercentage(float& fPercentage)` |
| 9 | 0x024 | `float GetVisualTimePercentage(void)` |
| 10 | 0x028 | `uint32_t GetHour(void)` |
| 11 | 0x02C | `uint32_t SetHour(uint32_t dwHour)` |
| 12 | 0x030 | `uint32_t GetVisualHour(void)` |
| 13 | 0x034 | `uint32_t GetMinute(void)` |
| 14 | 0x038 | `uint32_t SetMinute(uint32_t dwMinute)` |
| 15 | 0x03C | `uint32_t GetVisualMinute(void)` |
| 16 | 0x040 | `uint32_t GetSeconds(void)` |
| 17 | 0x044 | `uint32_t SetSeconds(uint32_t dwSeconds)` |
| 18 | 0x048 | `uint32_t GetVisualSeconds(void)` |
| 19 | 0x04C | `uint32_t GetEventHour(bool bUnknown)` |
| 20 | 0x050 | `uint32_t GetTimeMagnification(float& fMagnificationOut)` |
| 21 | 0x054 | `uint32_t SetTimeMagnification(float fMagnification)` |
| 22 | 0x058 | `uint32_t GetTimeMagnificationMinMax(float& fMinOut, float& fMaxOut)` |
| 23 | 0x05C | `bool SetTimeMagnificationMinMax(float fMin, float fMax)` |
| 24 | 0x060 | `bool Pause(bool bUnknown)` |
| 25 | 0x064 | `bool IsPaused(void)` |
| 26 | 0x068 | `bool HiddenPause(bool bUnknown)` |
| 27 | 0x06C | `bool IsHiddenPaused(void)` |
| 28 | 0x070 | `bool IsNight(void)` |
| 29 | 0x074 | `void EnableDayNightTransitionNotifications(bool bToggle)` |

#### cIGZDate (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Set(uint32_t dwDay, uint32_t dwYear)` **(overload: MSVC slot order may differ)** |
| 4 | 0x010 | `bool Set(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)` **(overload: MSVC slot order may differ)** |
| 5 | 0x014 | `bool Set(cRZTime const& sTime)` **(overload: MSVC slot order may differ)** |
| 6 | 0x018 | `bool Set(uint32_t dwDay)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `bool Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear)` **(overload: MSVC slot order may differ)** |
| 8 | 0x020 | `bool Copy(cIGZDate const& sCopyFrom)` |
| 9 | 0x024 | `bool Clone(cIGZDate** ppCopyTo)` |
| 10 | 0x028 | `bool Between(cIGZDate const& sStart, cIGZDate const& sEnd)` |
| 11 | 0x02C | `int32_t CompareTo(cIGZDate const& sOther)` |
| 12 | 0x030 | `uint32_t WeekDay(void)` |
| 13 | 0x034 | `uint32_t Year(void)` |
| 14 | 0x038 | `uint32_t DayOfYear(void)` |
| 15 | 0x03C | `uint32_t DayOfMonth(void)` |
| 16 | 0x040 | `uint32_t FirstDayOfMonth(void)` **(overload: MSVC slot order may differ)** |
| 17 | 0x044 | `uint32_t FirstDayOfMonth(uint32_t dwMonth)` **(overload: MSVC slot order may differ)** |
| 18 | 0x048 | `uint32_t Hash(void)` |
| 19 | 0x04C | `bool IsValid(void)` |
| 20 | 0x050 | `bool Leap(void)` |
| 21 | 0x054 | `bool MaxDate(cIGZDate const& sOther, cIGZDate& sMaxOut)` |
| 22 | 0x058 | `bool MinDate(cIGZDate const& sOther, cIGZDate& sMinOut)` |
| 23 | 0x05C | `uint32_t Month(void)` |
| 24 | 0x060 | `uint32_t Previous(uint32_t dwDayOfPriorWeek, cIGZDate& sOut)` |
| 25 | 0x064 | `uint32_t YearLastTwoDigits(void)` |
| 26 | 0x068 | `uint32_t DayNumber(void)` |
| 27 | 0x06C | `bool operator<(cIGZDate const& sOther)` |
| 28 | 0x070 | `bool operator<=(cIGZDate const& sOther)` |
| 29 | 0x074 | `bool operator>(cIGZDate const& sOther)` |
| 30 | 0x078 | `bool operator>=(cIGZDate const& sOther)` |
| 31 | 0x07C | `bool operator==(cIGZDate const& sOther)` |
| 32 | 0x080 | `bool operator!=(cIGZDate const& sOther)` |
| 33 | 0x084 | `cIGZDate& operator-(cIGZDate const& sOther)` |
| 34 | 0x088 | `cIGZDate& operator+(cIGZDate const& sOther)` |
| 35 | 0x08C | `cIGZDate& operator-(int32_t nDays)` |
| 36 | 0x090 | `cIGZDate& operator+(int32_t nDays)` |
| 37 | 0x094 | `cIGZDate& operator++(void)` |
| 38 | 0x098 | `cIGZDate& operator--(void)` |
| 39 | 0x09C | `cIGZDate& operator+=(int32_t nDays)` |
| 40 | 0x0A0 | `cIGZDate& operator-=(int32_t nDays)` |
| 41 | 0x0A4 | `bool DateString(cIGZString& sDateOut, uint32_t dwFormatID)` |
| 42 | 0x0A8 | `bool DayWithinMonth(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)` |
| 43 | 0x0AC | `uint32_t DaysInYear(uint32_t dwYear)` |
| 44 | 0x0B0 | `uint32_t Jday(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)` |
| 45 | 0x0B4 | `bool LeapYear(uint32_t dwYear)` |

#### cISC4DemandSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `uint32_t GetSimulatorType()` |
| 6 | 0x018 | `cISC4Demand* GetDemand(uint32_t demandID, uint32_t demandIndex)` |
| 7 | 0x01C | `void UpdateOccupantEffects(SC4Percentage unknown1, SC4Percentage const& unknown2, SC4Percentage const& unknown3)` |
| 8 | 0x020 | `void CalculateJobsPerUnitOfDemand(float* jobsArray, uint32_t demandID, uint32_t demandIndex)` |
| 9 | 0x024 | `uint32_t GetJobsBySensus(uint32_t type)` |
| 10 | 0x028 | `float GetNeutralTaxRate()` |
| 11 | 0x02C | `void GetLocalPopulationSummary(std::map<uint32_t, int32_t>& map) const` |

#### cISC4Demand (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool SimulationBegin()` |
| 6 | 0x018 | `uint32_t GetId() const` |
| 7 | 0x01C | `bool SetId(uint32_t id)` |
| 8 | 0x020 | `float QuerySupplyValue() const` |
| 9 | 0x024 | `float QueryDemandValue() const` |
| 10 | 0x028 | `float QueryNewSupply() const` |
| 11 | 0x02C | `float QueryNewDemand() const` |
| 12 | 0x030 | `float QueryActiveDemandValue() const` |
| 13 | 0x034 | `float QueryEconomyModifier() const` |
| 14 | 0x038 | `float QueryActiveDemandMax() const` |
| 15 | 0x03C | `float QueryActiveDemandMin() const` |
| 16 | 0x040 | `bool AddToSupplyValue(float value)` |
| 17 | 0x044 | `bool AddToDemandValue(float value)` |
| 18 | 0x048 | `bool SetSupplyValue(float value)` |
| 19 | 0x04C | `bool SetDemandValue(float value)` |
| 20 | 0x050 | `bool SetActiveDemandMax(float value)` |
| 21 | 0x054 | `bool SetActiveDemandMin(float value)` |
| 22 | 0x058 | `float GetEconomyModifier() const` |
| 23 | 0x05C | `bool SetEconomyModifier(float value)` |
| 24 | 0x060 | `float GetTaxModifier() const` |
| 25 | 0x064 | `bool SetTaxModifier(float value)` |
| 26 | 0x068 | `SC4Percentage* GetDemandCap() const` |
| 27 | 0x06C | `bool SetDemandCap(const SC4Percentage& demandCap)` |
| 28 | 0x070 | `uint32_t GetRegionUse()` |
| 29 | 0x074 | `bool SetRegionUse(uint32_t param_1)` |
| 30 | 0x078 | `float EndOfCycle()` |
| 31 | 0x07C | `void DebugLockValue(float value)` |
| 32 | 0x080 | `void DebugUnlockValue()` |
| 33 | 0x084 | `bool DebugIsValueLocked()` |

#### cISC4TrafficSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `uint32_t GetSimulatorType()` |
| 6 | 0x018 | `bool CreatePathFinder(cISC4PathFinder** pathFinder)` |
| 7 | 0x01C | `bool SetupPathFinderForLot(cISC4PathFinder* pathFinder, cISC4Lot* lot)` |
| 8 | 0x020 | `cISC4SimGrid<uint8_t>* GetAirPollutingTrafficMap() const` |
| 9 | 0x024 | `cISC4SimGrid<uint8_t>* GetCommercialTrafficMap() const` |
| 10 | 0x028 | `cISC4SimGrid<uint8_t>* GetCongestionMap() const` |
| 11 | 0x02C | `intptr_t GetBackgroundTraffic(int unknown1, int unknown2)` |
| 12 | 0x030 | `TrafficEdgeDensity* GetTrafficEdgeDensity(uint8_t x, uint8_t z, TravelType travelType, bool eveningCommute)` |
| 13 | 0x034 | `cISC4SimGrid<uint8_t>* GetTripLengthMap() const` |
| 14 | 0x038 | `float GetTripScale() const` |
| 15 | 0x03C | `float GetTripScaleForDisplay() const` |
| 16 | 0x040 | `bool IsRoadDamaged(int unknown1, int unknown2)` |
| 17 | 0x044 | `bool CheckRailAccident(int unknown1, int unknown2)` |
| 18 | 0x048 | `float GetLastMonthlyIncome()` |
| 19 | 0x04C | `bool SetMaxTripCapacity(cISCPropertyHolder* unknown1, uint32_t unknown2, uint32_t unknown3)` |
| 20 | 0x050 | `uint32_t GetMaxTripCapacity(cISCPropertyHolder* unknown1, uint32_t unknown2)` |
| 21 | 0x054 | `uint32_t GetTrafficArrived(cISCPropertyHolder* unknown1, uint32_t unknown2)` |
| 22 | 0x058 | `uint32_t GetDesiredLotInsertionPoint()` |
| 23 | 0x05C | `uint32_t GetCapacity(uint32_t networkType, int unknown2, int unknown3)` |
| 24 | 0x060 | `float GetTravelTimeRatio(long unknown1, long unknown2, TravelType travelType)` |
| 25 | 0x064 | `uint32_t GetConnectionCount(uint32_t networkType, int unknown2, int unknown3)` |
| 26 | 0x068 | `bool GetTravelStrategyPercentages(uint32_t wealthType, SC4Vector<SC4Percentage> unknown2)` |
| 27 | 0x06C | `float GetFreightScalingFactor()` |
| 28 | 0x070 | `bool GetTransitSwitches(ilist<cISC4Occupant*>& unknown1)` |
| 29 | 0x074 | `int32_t GetFerryRouteBetweenTiles(long unknown1, long unknown2, long unknown3, long unknown4)` |
| 30 | 0x078 | `bool GetAllFerryRoutes(SC4List<SC4Vector<uint8_t>>& unknown1)` |
| 31 | 0x07C | `bool GetFerryRoutesInUse(SC4List<FerryRouteInfo>& unknown1)` |
| 32 | 0x080 | `uint32_t GetFerryTerminalCount(uint32_t ferryType)` |
| 33 | 0x084 | `bool GetWaterRoute(long unknown1, long unknown2, long unknown3, long unknown4, SC4Vector<uint8_t>& unknown5)` |
| 34 | 0x088 | `intptr_t GetTrafficStats()` |
| 35 | 0x08C | `bool GetTransitSwitchQueryData(uint32_t unknown1, uint32_t unknown2, TransitSwitchQueryData& data)` |
| 36 | 0x090 | `bool AreLotsConnected(cISC4Lot* unknown1, cISC4Lot* unknown2) const` |
| 37 | 0x094 | `uint32_t GetConnectedOccupantCount(cISC4Lot* unknown1, uint32_t unknown2) const` |
| 38 | 0x098 | `uint32_t GetConnectedDestinationCount(cISC4Lot* unknown1, int unknown2) const` |
| 39 | 0x09C | `bool GetSubnetworksForLot(cISC4Lot* unknown1, SC4Vector<uint32_t>& unknown2)` |
| 40 | 0x0A0 | `bool GetSubnetworksInRectangle(SC4Rect<int> const& rect, SC4Vector<uint32_t>& unknown2)` |
| 41 | 0x0A4 | `bool GetSubnetworksInRegion(SC4CellRegion<long> const& cellRegion, SC4Vector<uint32_t>& unknown2)` |
| 42 | 0x0A8 | `bool GetOccupantCountForAllSubnetworks(uint32_t unknown1, SC4Vector<uint32_t>& unknown2)` |

#### cISC4TrafficSource (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `SC4List<cSC4TripData>* GetTripDataList(void)` |
| 4 | 0x010 | `bool UpdateTrafficVolume(void)` |
| 5 | 0x014 | `bool GetSourceCells(SC4Rect<uint32_t>& sCellRect)` |

#### cISC4ResidentialSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `cISC4SimGrid<uint8_t>* GetProximityMap(cISC4BuildingOccupant::WealthType eWealthType)` |
| 6 | 0x018 | `bool SchoolIsOnStrike(void)` |
| 7 | 0x01C | `bool HealthIsOnStrike(void)` |
| 8 | 0x020 | `bool EndSchoolStrike(void)` |
| 9 | 0x024 | `bool EndHealthStrike(void)` |
| 10 | 0x028 | `float ChanceOfSchoolStrike(void)` |
| 11 | 0x02C | `float ChanceOfHealthStrike(void)` |
| 12 | 0x030 | `float GetSchoolSystemRating(void)` |
| 13 | 0x034 | `float GetHealthSystemRating(void)` |
| 14 | 0x038 | `bool GetSchoolSystemTotals(intptr_t tQueryData, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` |
| 15 | 0x03C | `bool GetHospitalSystemTotals(intptr_t tQueryData, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` |
| 16 | 0x040 | `int32_t GetPopulation(void)` |
| 17 | 0x044 | `int32_t GetTotalCityEducationUpkeepCost(void)` |
| 18 | 0x048 | `int32_t GetTotalCityHealthUpkeepCost(void)` |
| 19 | 0x04C | `bool SetOccupantFundingPercentages(cISC4Occupant* pOccupant, SC4Percentage const& sSchoolFunding, SC4Percentage const& sHealthFunding, bool bUnknown)` |
| 20 | 0x050 | `bool GetOccupantFundingPercentages(cISC4Occupant* pOccupant, SC4Percentage& sSchoolFunding, SC4Percentage& sHealthFunding, bool bUnknown)` |
| 21 | 0x054 | `bool GetAverageEQGrid(cISC4SimGrid<float>*& pGrid, float* fMin, float* fMax)` |
| 22 | 0x058 | `bool GetAverageHQGrid(cISC4SimGrid<float>*& pGrid, float* fMin, float* fMax)` |
| 23 | 0x05C | `bool GetEQGrids(cISC4SimGrid<float>*& pGrid, cISC4SimGrid<float>* pUnknown1, cISC4SimGrid<float>* pUnknown2)` |
| 24 | 0x060 | `bool GetHQGrids(cISC4SimGrid<float>*& pGrid, cISC4SimGrid<float>* pUnknown1, cISC4SimGrid<float>* pUnknown2)` |
| 25 | 0x064 | `bool GetPopulationGrids(cISC4SimGrid<uint16_t>*& pGrid, cISC4SimGrid<uint16_t>* pUnknown1, cISC4SimGrid<uint16_t>* pUnknown2)` |
| 26 | 0x068 | `bool GetSchoolQueryData(cISC4Occupant* pOccupant, intptr_t pQueryData)` |
| 27 | 0x06C | `bool GetHospitalQueryData(cISC4Occupant* pOccupant, intptr_t pQueryData)` |
| 28 | 0x070 | `bool EstimateCurrentOccupantCapacity(cISC4Occupant* pOccupant, uint32_t& dwUnknown1, uint32_t& dwUnknown2)` |
| 29 | 0x074 | `int32_t GetCellLifeExpectancy(uint32_t dwCellX, uint32_t dwCellZ)` |
| 30 | 0x078 | `float GetCellWorkforcePercent (uint32_t dwCellX, uint32_t dwCellZ)` |
| 31 | 0x07C | `float GetGlobalWorkforcePercent(void)` |
| 32 | 0x080 | `float GetGlobalEQ(void)` |
| 33 | 0x084 | `float GetGlobalHQ(void)` |
| 34 | 0x088 | `float GetGlobalLE(void)` |
| 35 | 0x08C | `float GetCellEQ(uint32_t dwCellX, uint32_t dwCellZ)` |
| 36 | 0x090 | `float GetCellHQ(uint32_t dwCellX, uint32_t dwCellZ)` |
| 37 | 0x094 | `float GetCellEQByWealth(uint32_t dwCellX, uint32_t dwCellZ, cISC4BuildingOccupant::WealthType eWealthType)` |
| 38 | 0x098 | `float GetCellHQByWealth(uint32_t dwCellX, uint32_t dwCellZ, cISC4BuildingOccupant::WealthType eWealthType)` |
| 39 | 0x09C | `int32_t GetSchoolAverageGradeMap(void)` |
| 40 | 0x0A0 | `int32_t GetHospitalAverageGradeMap(void)` |
| 41 | 0x0A4 | `int32_t GetAverageAgeMap(void)` |
| 42 | 0x0A8 | `int32_t GetAverageNewAgeByWealth(cISC4BuildingOccupant::WealthType eWealthType)` |
| 43 | 0x0AC | `bool GetEQMinAndMaxCellCoords(uint32_t& dwMinCellX, uint32_t& dwMinCellZ, uint32_t& dwMaxCellX, uint32_t& dwMaxCellZ, float& fMin, float& fMax)` |
| 44 | 0x0B0 | `bool GetHQMinAndMaxCellCoords(uint32_t& dwMinCellX, uint32_t& dwMinCellZ, uint32_t& dwMaxCellX, uint32_t& dwMaxCellZ, float& fMin, float& fMax)` |
| 45 | 0x0B4 | `bool GetOccupantCoverage(cISC4Occupant* pOccupant, SC4Percentage const& sEffectiveness, float& fRangeX, float& fRangeZ)` |
| 46 | 0x0B8 | `int32_t GetSchoolBuildingCount(void)` |
| 47 | 0x0BC | `bool GetSchoolBuildings(SC4List<cISC4Occupant*>& sBuildings, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` |
| 48 | 0x0C0 | `int32_t GetHospitalBuildingCount(void)` |
| 49 | 0x0C4 | `bool GetHospitalBuildings(SC4List<cISC4Occupant*>& sBuildings, SC4Vector<uint32_t>* requiredOccupantGroups = nullptr)` |
| 50 | 0x0C8 | `int32_t GetMaxEQ(void)` |
| 51 | 0x0CC | `int32_t GetMaxHQ(void)` |
| 52 | 0x0D0 | `bool GetGlobalAutoBudgetForSchools(void)` |
| 53 | 0x0D4 | `bool SetGlobalAutoBudgetForSchools(bool bEnable)` |
| 54 | 0x0D8 | `bool GetGlobalAutoBudgetForHospitals(void)` |
| 55 | 0x0DC | `bool SetGlobalAutoBudgetForHospitals(bool bEnable)` |
| 56 | 0x0E0 | `bool GetAutoBudget(void)` |
| 57 | 0x0E4 | `bool SetAutoBudget(bool bEnable)` |
| 58 | 0x0E8 | `bool EstimateIdealFunding(cISC4Occupant* pOccupant, SC4Percentage& sFunding)` |
| 59 | 0x0EC | `void ToggleTractTracking(int32_t nUnknown1, int32_t nUnknown2)` |

#### cISC4CommercialSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `cISC4SimGrid<uint8_t>* GetProximityMap(cISC4BuildingDevelopmentSimulator::DeveloperType)` |

#### cISC4IndustrialSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |

#### cISC4BuildingDevelopmentSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool LocationIsOKForNewBuilding(uint32_t dwBuilding, float fPosX, float fPosZ)` |
| 6 | 0x018 | `bool AddNewBuildingOccupant(cISC4BuildingOccupant* pOccupant)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `bool AddNewBuildingOccupant(uint32_t dwUnknown, float* pfUnknown, int32_t nUnknown)` **(overload: MSVC slot order may differ)** |
| 8 | 0x020 | `bool AddNewBuildingOccupant(uint32_t dwUnknown, int32_t nUnknown1, int32_t nUnknown2, int32_t nUnknown3)` **(overload: MSVC slot order may differ)** |
| 9 | 0x024 | `int64_t GetCostOfOccupantDemolition(cISC4BuildingOccupant* pOccupant, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 10 | 0x028 | `size_t GetAllBuildingTypes(uint32_t* pArray, uint32_t& nCount)` **(overload: MSVC slot order may differ)** |
| 11 | 0x02C | `size_t GetAllBuildingTypes(SC4Vector<uint32_t>& sList)` **(overload: MSVC slot order may differ)** |
| 12 | 0x030 | `size_t GetAllBuildingFamilyTypes(SC4Vector<uint32_t>& sList)` |
| 13 | 0x034 | `bool GetBuildingKeyFromType(uint32_t dwType, cGZPersistResourceKey& sKey)` |
| 14 | 0x038 | `bool IsBuildingTypeValid(uint32_t dwType)` |
| 15 | 0x03C | `bool IsBuildingFamilyTypeValid(uint32_t dwFamilyType)` |
| 16 | 0x040 | `bool GetBuildingTypesByFilter(std::unordered_set<uint32_t>& sList, cISC4BuildingTypeFilter* pFilter)` |
| 17 | 0x044 | `bool GetBuildingTypesByFamily(std::unordered_set<uint32_t>& sList, uint32_t dwFamily)` **(overload: MSVC slot order may differ)** |
| 18 | 0x048 | `bool GetBuildingTypesByFamily(SC4Vector<uint32_t>& sList, uint32_t dwFamily)` **(overload: MSVC slot order may differ)** |
| 19 | 0x04C | `bool IsBuildingInFamily(uint32_t dwBuilding, uint32_t dwFamily)` |
| 20 | 0x050 | `bool GetAllFoundationTypes(SC4Vector<uint32_t>& sList)` |
| 21 | 0x054 | `bool GetFoundationKeyFromType(uint32_t dwType, cGZPersistResourceKey& sKey)` |
| 22 | 0x058 | `bool AddNewFoundationOccupant(cISC4FoundationOccupant* pOccupant)` **(overload: MSVC slot order may differ)** |
| 23 | 0x05C | `bool AddNewFoundationOccupant(uint32_t, int32_t, float*, float, float, int, float)` **(overload: MSVC slot order may differ)** |
| 24 | 0x060 | `bool DemolishFoundationOccupant(cISC4FoundationOccupant* pOccupant, uint32_t)` |
| 25 | 0x064 | `int64_t GetCostOfOccupantDemolition(void)` **(overload: MSVC slot order may differ)** |
| 26 | 0x068 | `bool LoadFoundationProperties(cISC4FoundationOccupant* pOccupant, uint32_t)` |
| 27 | 0x06C | `bool GetAllLotRetainingWallTypes(SC4Vector<uint32_t>& sList)` |
| 28 | 0x070 | `bool GetLotRetainingWallKeyFromType(uint32_t dwType, cGZPersistResourceKey& sKey)` |
| 29 | 0x074 | `bool LoadLotRetainingWallProperties(cISC4LotRetainingWallOccupant* pOccupant, uint32_t)` |
| 30 | 0x078 | `DeveloperType GetDeveloperType(cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth)` |
| 31 | 0x07C | `cISC4BuildingOccupant::WealthType GetWealth(DeveloperType developerType)` |
| 32 | 0x080 | `cISC4GrowthDeveloper* GetDeveloper(cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth) const` **(overload: MSVC slot order may differ)** |
| 33 | 0x084 | `cISC4GrowthDeveloper* GetDeveloper(DeveloperType developerType) const` **(overload: MSVC slot order may differ)** |
| 34 | 0x088 | `intptr_t GetDevelopers(void)` |
| 35 | 0x08C | `cISC4TractDeveloper* GetTractDeveloper(void)` |
| 36 | 0x090 | `cISC4LotDeveloper* GetLotDeveloper(void)` |
| 37 | 0x094 | `cIGZString* GetExemplarName(uint32_t dwExemplar)` |
| 38 | 0x098 | `cIGZString* GetBuildingName(uint32_t dwBuilding)` |
| 39 | 0x09C | `cISC4BuildingOccupant::BuildingProfile& GetBuildingProfile(uint32_t dwBuilding) const` |
| 40 | 0x0A0 | `cISC4BuildingOccupant::BuildingProfile* GetBuildingProfilePtr(uint32_t dwBuilding) const` |
| 41 | 0x0A4 | `intptr_t GetBuildingMap(void)` |
| 42 | 0x0A8 | `intptr_t GetBuildingMapKey(uint32_t dwBuilding)` **(overload: MSVC slot order may differ)** |
| 43 | 0x0AC | `intptr_t GetBuildingMapKey(cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth)` **(overload: MSVC slot order may differ)** |
| 44 | 0x0B0 | `intptr_t GetSpecialBuildings(void)` |
| 45 | 0x0B4 | `int64_t GetMaxCapacity(cISC4ZoneManager::ZoneType zone, cISC4BuildingOccupant::PurposeType purpose, cISC4BuildingOccupant::WealthType wealth)` |
| 46 | 0x0B8 | `bool IsSuitableForConstruction(intptr_t const& sConstructionProfile, int32_t& nReasonForFail)` |
| 47 | 0x0BC | `bool PlopBuilding(ConstructionProfile const& sConstructionProfile)` |
| 48 | 0x0C0 | `int16_t GetFreeConstructionResources(void)` |
| 49 | 0x0C4 | `int16_t GetTotalConstructionResources(void)` |
| 50 | 0x0C8 | `bool ClaimConstructionResources(uint16_t wResources)` |
| 51 | 0x0CC | `bool ReleaseConstructionResources(uint16_t wResources)` |
| 52 | 0x0D0 | `int32_t GetDayOfLastSuccessfulBuild(DeveloperType developerType)` |
| 53 | 0x0D4 | `float GetStagePopulationMultiplier(void)` |
| 54 | 0x0D8 | `bool CheatIgnorePlumbing(void)` |
| 55 | 0x0DC | `bool CheatIgnorePower(void)` |
| 56 | 0x0E0 | `bool CheatIgnoreCaps(void)` |
| 57 | 0x0E4 | `bool CheatDebugGrowth(void)` |
| 58 | 0x0E8 | `bool CheatCoffeeBar(void)` |

#### cISC4GrowthDeveloper (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(cISCPropertyHolder* pPropertyHolder)` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `TargetZoneType GetType()` |
| 6 | 0x018 | `uint32_t GetSubType()` |
| 7 | 0x01C | `cISC4BuildingOccupant::PurposeType GetPurpose()` |
| 8 | 0x020 | `cISC4BuildingOccupant::WealthType GetWealth()` |
| 9 | 0x024 | `void GetConstructionInfo(cISC4BuildingDevelopmentSimulator::ConstructionProfile& constructionProfile)` |
| 10 | 0x028 | `bool GrowthAndDeclineListsAreValid()` |
| 11 | 0x02C | `SC4Vector<GrowthItem>& GetGrowthList()` |
| 12 | 0x030 | `SC4Vector<GrowthItem>& GetDeclineList()` |
| 13 | 0x034 | `cISC4SimGrid<int16_t>* GetDesirabilityMap()` |
| 14 | 0x038 | `cISC4SimGrid<uint16_t>* GetCapacityMap()` |
| 15 | 0x03C | `cISC4SimGrid<int16_t>* GetPopulationMap()` |
| 16 | 0x040 | `cISC4SimGrid<uint16_t>* GetMaxCapacityMap()` |
| 17 | 0x044 | `uint32_t GetTotalCapacity()` |
| 18 | 0x048 | `cISC4Demand* GetSourceDemand()` |
| 19 | 0x04C | `uint8_t GetPowerSupplyCap()` |
| 20 | 0x050 | `uint8_t GetWaterSupplyCap()` |
| 21 | 0x054 | `uint8_t GetFireCoverageCap()` |
| 22 | 0x058 | `uint8_t GetParkCap()` |
| 23 | 0x05C | `int32_t GetDesirabilityThreshold()` |
| 24 | 0x060 | `float GetAbandonmentThreshold(int32_t densityType)` |
| 25 | 0x064 | `MonopolyPieceUse GetMonopolyUse()` |
| 26 | 0x068 | `intptr_t GetStylePercentages()` |
| 27 | 0x06C | `void StartMapUpdate()` |
| 28 | 0x070 | `bool IsMapUpdateFinished()` |
| 29 | 0x074 | `int32_t EstimateDesirability(int32_t cellX, int32_t cellZ)` |
| 30 | 0x078 | `void GetSubjectiveThresholds(int32_t index, int32_t& min, int32_t& max)` |

#### cISC4TractDeveloper (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool Grow(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile, SC4List<Outcome>& sOutcomes, bool& bUnknown)` |
| 6 | 0x018 | `bool Decline(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile)` |
| 7 | 0x01C | `bool UpdatePopulation(cISC4BuildingDevelopmentSimulator::GrowthProfile const& sProfile)` |
| 8 | 0x020 | `int32_t GetCurrentStyle(void) const` |
| 9 | 0x024 | `SC4Vector<uint32_t>& GetActiveStyles(void) const` |
| 10 | 0x028 | `bool SetActiveStyles(SC4Vector<uint32_t> const& sStyles)` |
| 11 | 0x02C | `bool IsUsingAllStylesAtOnce(void) const` |
| 12 | 0x030 | `cISC4TractDeveloper* UseAllStylesAtOnce(bool bFlag)` |
| 13 | 0x034 | `int32_t GetYearsBetweenStyleChanges(void) const` |
| 14 | 0x038 | `cISC4TractDeveloper* SetYearsBetweenStyleChanges(int32_t nYears)` |

#### cISC4LotDeveloper (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `int32_t GetSimulatorType(void)` |
| 6 | 0x018 | `bool StartConstruction(cISC4Lot* pLot, cISC4LotConfiguration* pConfig, uint32_t, bool)` |
| 7 | 0x01C | `bool ConstructDefaultLot(cISC4Lot* pLot, cISC4LotConfiguration* pConfig, uint32_t, bool)` |
| 8 | 0x020 | `bool AbandonBuilding(cISC4Lot* pLot, int32_t nVacateReason)` |
| 9 | 0x024 | `bool ReoccupyBuilding(cISC4Lot* pLot, int32_t nDeveloperType)` |
| 10 | 0x028 | `bool ChangeBuilding(cISC4Lot* pLot, uint32_t)` |
| 11 | 0x02C | `bool RemoveBuildingZotProp(cISC4Lot* pLot)` |
| 12 | 0x030 | `bool StartDemolishLot(cISC4Lot* pLot)` |
| 13 | 0x034 | `bool EndDemolishLot(cISC4Lot* pLot)` |
| 14 | 0x038 | `bool ExplodeLot(cISC4Lot* pLot, bool bBurnt)` |
| 15 | 0x03C | `float GetTerrainAltitudeByVertex(int32_t nX, int32_t nZ, bool)` |
| 16 | 0x040 | `float GetLowestOrHighestAreaAltitude(SC4Rect<int32_t> const&, bool, bool)` **(overload: MSVC slot order may differ)** |
| 17 | 0x044 | `float GetLowestOrHighestAreaAltitude(SC4Rect<float> const&, bool, bool)` **(overload: MSVC slot order may differ)** |
| 18 | 0x048 | `float GetAverageAreaAltitude(SC4Rect<float> const&, bool)` |
| 19 | 0x04C | `bool LevelAreaInternalVertices(SC4Rect<int32_t> const&, float, bool, bool, cISC4Lot*)` |
| 20 | 0x050 | `bool LevelBorderingVertices(SC4Rect<int32_t> const&, float, bool, cISC4Lot*)` |
| 21 | 0x054 | `bool AttemptAreaLevelling(SC4Rect<int32_t> const&, float, bool, bool, float, cISC4Lot*)` |
| 22 | 0x058 | `bool GetOccupantCellCoverage(cISC4Occupant* pOccupant, SC4Rect<int32_t>& sCoverage)` |
| 23 | 0x05C | `bool IsAreaDeveloped(void)` |
| 24 | 0x060 | `bool IsLotDeveloped(cISC4Lot* pLot)` |
| 25 | 0x064 | `bool IsAreaFlat(SC4Rect<int32_t> const& sBounds, bool)` |
| 26 | 0x068 | `bool IsLotFlat(cISC4Lot* pLot, bool)` |
| 27 | 0x06C | `float GetAreaAltitudeRange(SC4Rect<int32_t> const& sBounds, bool)` |
| 28 | 0x070 | `float GetLotAltitudeRange(cISC4Lot* pLot, bool)` |
| 29 | 0x074 | `float GetAreaSlope(SC4Rect<int32_t> const& sBounds, bool)` |
| 30 | 0x078 | `float GetLotSlope(cISC4Lot* pLot, bool)` |
| 31 | 0x07C | `float GetOccupantSlope(cISC4Occupant* pOccupant, bool)` |
| 32 | 0x080 | `int64_t GetLotDevelopmentCost(SC4Rect<int32_t> const& sBounds)` |
| 33 | 0x084 | `bool RemoveLotFromItsBusinessUnit(cISC4Lot* pLot)` |
| 34 | 0x088 | `bool UpdateTravelDesignates(cISC4BusinessUnit*& pUnit)` |
| 35 | 0x08C | `bool UpdateTravelPopulations(cISC4BusinessUnit* pUnit)` |
| 36 | 0x090 | `bool NeedsAnchorBuilding(SC4Rect<int32_t> const& sBounds, int32_t nPurpose)` |
| 37 | 0x094 | `bool UpdateLotElements(cISC4Lot* pLot, uint32_t)` |
| 38 | 0x098 | `cISC4Occupant* GetFirstBuilding(cISC4Lot* pLot)` |
| 39 | 0x09C | `bool GetPositionForLotConfigurationObject(cISC4Lot* pLot, cISC4LotConfiguration* pConfig, cSC4LotConfigurationObject* pObject, float*, long&)` |
| 40 | 0x0A0 | `bool GetPositionForBuilding(cISC4Lot* pLot, float*, long&)` |
| 41 | 0x0A4 | `bool GetLotBaseTextureOccupant(cISC4Lot* pLot, cISC4LotBaseTextureOccupant*& pOccupant, bool)` |
| 42 | 0x0A8 | `bool GetBoundaryLotTexture(cISC4LotDeveloper::BoundaryType* pType, uint32_t&, uint8_t&, bool&)` |
| 43 | 0x0AC | `void SetEmptyLotBaseTexture(cISC4Lot* pLot)` |
| 44 | 0x0B0 | `cISC4Lot* SetNormalLotBaseTexture(cISC4Lot* pLot, cISC4LotConfiguration* pConfig, bool)` |
| 45 | 0x0B4 | `bool UpdateLotBaseTextureOccupant(cISC4Lot* pLot, bool)` |
| 46 | 0x0B8 | `bool GetForceEmptyLotBaseTextures(void)` |
| 47 | 0x0BC | `void SetForceEmptyLotBaseTextures(bool bFlag)` |
| 48 | 0x0C0 | `intptr_t GetFoundationByOccupant(cISC4Occupant* pOccupant)` |
| 49 | 0x0C4 | `intptr_t GetFoundationByStandardCityCell(int32_t, int32_t nX, int32_t nZ)` |
| 50 | 0x0C8 | `intptr_t GetFoundationByPosition(int32_t, float fX, float fZ)` |
| 51 | 0x0CC | `intptr_t GetLotFoundation(cISC4Lot* pLot)` |
| 52 | 0x0D0 | `bool RemoveFoundation(int32_t, float, float)` |
| 53 | 0x0D4 | `bool RemoveLotFoundation(cISC4Lot* pLot)` |
| 54 | 0x0D8 | `bool AddLotFoundation(cISC4Lot* pLot, uint32_t, bool, bool, bool)` **(overload: MSVC slot order may differ)** |
| 55 | 0x0DC | `bool AddLotFoundation(cISC4Lot* pLot, cISC4LotConfiguration* pConfig, bool, bool, bool)` **(overload: MSVC slot order may differ)** |
| 56 | 0x0E0 | `bool IsLotObjectFoundationRequired(float*, float*)` **(overload: MSVC slot order may differ)** |
| 57 | 0x0E4 | `bool IsLotObjectFoundationRequired(cISC4Occupant* pOccupant)` **(overload: MSVC slot order may differ)** |
| 58 | 0x0E8 | `bool AddBuildingFoundation(cISC4BuildingOccupant* pOccupant, uint32_t)` **(overload: MSVC slot order may differ)** |
| 59 | 0x0EC | `bool AddBuildingFoundation(cISC4BuildingOccupant* pOccupant, cISC4LotConfiguration* pConfig)` **(overload: MSVC slot order may differ)** |
| 60 | 0x0F0 | `bool RemoveBuildingFoundation(cISC4Occupant* pOccupant)` |
| 61 | 0x0F4 | `bool UpdateLotFoundation(cISC4Lot* pLot)` |
| 62 | 0x0F8 | `bool UpdateLotRetainingWalls(cISC4Lot* pLot)` |
| 63 | 0x0FC | `bool CreateLotProp(cISC4Lot* pLot, uint32_t, uint32_t)` |
| 64 | 0x100 | `bool GetLotConfigurationObjectOccupant(cISC4Lot* pLot, uint32_t, cISC4Occupant*& pOccupant)` |

#### cISC4ZoneDeveloper (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `void HighlightParcels(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType, SC4Point<long>* unknown3, SC4CellRegion<long>** unknown4)` |
| 6 | 0x018 | `void DoParcellization(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType, bool unknown3)` |
| 7 | 0x01C | `void SetOptions(bool alternateOrientation, bool placeStreets, bool customZoneSize)` |
| 8 | 0x020 | `bool ExistsNetworkOfType(int32_t x, int32_t z, uint32_t networkFlags)` |
| 9 | 0x024 | `bool IsCellWater(int32_t x, int32_t z)` |

#### cISC4ZoneManager (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `cISC4SimGrid<int8_t>* GetZoneGrid() const` |
| 6 | 0x018 | `int32_t GetZoneCount(ZoneType type)` |
| 7 | 0x01C | `int32_t GetUndevelopedTileCount()` **(overload: MSVC slot order may differ)** |
| 8 | 0x020 | `int32_t GetUndevelopedTileCount(ZoneType type)` **(overload: MSVC slot order may differ)** |
| 9 | 0x024 | `int32_t GetDevelopmentFailureCount(int x, int z)` |
| 10 | 0x028 | `int32_t GetAbandonedTileCount()` |
| 11 | 0x02C | `bool PlaceZone(SC4CellRegion<int32_t>& cellRegion, ZoneType zoneType, bool placeZone, bool skipFundsCheck, bool checkAreaSlope, bool skipZoneSizeCheck, bool checkLotSize, int64_t* outZonedCellCount, int32_t* outErrorCode, intptr_t outLotsDemolishedSet)` |
| 12 | 0x030 | `ZoneType GetZoneType(int32_t x, int32_t z)` |
| 13 | 0x034 | `bool IsRCIZone(ZoneType type)` |
| 14 | 0x038 | `bool GetTextureForZone(ZoneType type, cGZPersistResourceKey& key)` |
| 15 | 0x03C | `uint32_t GetZoneDragColor(ZoneType type) const` |
| 16 | 0x040 | `uint8_t GetZoneDisplayAlpha() const` |
| 17 | 0x044 | `uint8_t GetDefaultZoneDisplayAlpha() const` |
| 18 | 0x048 | `void SetZoneDisplayAlpha(uint8_t value)` |
| 19 | 0x04C | `int32_t GetMinZoneSize(ZoneType type)` |
| 20 | 0x050 | `int32_t GetMaxZoneSize(ZoneType type)` |

#### cISC4SimGrid (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `uint32_t GetInstanceID(void) const` |
| 6 | 0x018 | `bool SetInstanceID(uint32_t dwInstanceID)` |
| 7 | 0x01C | `T GetCellValue(int32_t nCellX, int32_t nCellZ) const` |
| 8 | 0x020 | `T GetAverageValueInCellRect(int32_t nTopLeftX, int32_t nTopLeftZ, int32_t nBottomRightX, int32_t nBottomRightZ) const` |
| 9 | 0x024 | `bool SetTractSize(int32_t nSize)` |
| 10 | 0x028 | `int32_t GetTractSize(void) const` |
| 11 | 0x02C | `int32_t GetTractShift(void) const` |
| 12 | 0x030 | `int32_t GetTractCountX(void) const` |
| 13 | 0x034 | `int32_t GetTractCountZ(void) const` |
| 14 | 0x038 | `float GetTractWidthX(void) const` |
| 15 | 0x03C | `float GetTractWidthZ(void) const` |
| 16 | 0x040 | `float GetOneOverTractWidthX(void) const` |
| 17 | 0x044 | `float GetOneOverTractWidthZ(void) const` |
| 18 | 0x048 | `bool TractIsInBounds(uint32_t dwTractX, uint32_t dwTractZ) const` |
| 19 | 0x04C | `bool PositionToTract(float fPosX, float fPosZ, int32_t& nTractX, int32_t& nTractZ) const` |
| 20 | 0x050 | `bool TractCornerToPosition(int32_t nTractX, int32_t nTractZ, float& fPosX, float& fPosZ) const` |
| 21 | 0x054 | `bool TractCenterToPosition(int32_t nTractX, int32_t nTractZ, float& fPosX, float& fPosZ) const` |
| 22 | 0x058 | `T GetTractValue(int32_t nTractX, int32_t nTractZ) const` |
| 23 | 0x05C | `T GetAverageValueInTractRect(int32_t nTopLeftX, int32_t nTopLeftZ, int32_t nBottomRightX, int32_t nBottomRightZ) const` |
| 24 | 0x060 | `intptr_t GetGridData(void)` **(overload: MSVC slot order may differ)** |
| 25 | 0x064 | `intptr_t GetGridData(void) const` **(overload: MSVC slot order may differ)** |
| 26 | 0x068 | `void SetTractValue(int32_t nTractX, int32_t nTractZ, T value)` |
| 27 | 0x06C | `void SetTractValues(T value)` |

#### cISC4PollutionSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `cISC4SimGrid<short>* GetPollutionGrid(uint32_t dwPollutionType)` |
| 6 | 0x018 | `uint16_t GetPollutionValue(uint32_t dwPollutionType, uint32_t dwCellX, uint32_t dwCellZ)` |
| 7 | 0x01C | `bool GetAirValue(uint32_t dwCellX, uint32_t dwCellZ, int32_t& wValue)` |
| 8 | 0x020 | `bool GetWaterValue(uint32_t dwCellX, uint32_t dwCellZ, int32_t& wValue)` |
| 9 | 0x024 | `bool GetGarbageValue(uint32_t dwCellX, uint32_t dwCellZ, int32_t& wValue)` |
| 10 | 0x028 | `int32_t GetAverageAirValue(void)` |
| 11 | 0x02C | `int32_t GetAverageWaterValue(void)` |
| 12 | 0x030 | `int32_t GetAverageGarbageValue(void)` |
| 13 | 0x034 | `bool IsAirPolluted(uint32_t dwCellX, uint32_t dwCellZ)` |
| 14 | 0x038 | `bool IsWaterPolluted(uint32_t dwCellX, uint32_t dwCellZ)` |
| 15 | 0x03C | `bool IsGarbageAccumulated(uint32_t dwCellX, uint32_t dwCellZ)` |
| 16 | 0x040 | `bool IsRadioactive(uint32_t dwCellX, uint32_t dwCellZ)` |
| 17 | 0x044 | `bool GetWorstAirTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` |
| 18 | 0x048 | `bool GetWorstWaterTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` |
| 19 | 0x04C | `bool GetWorstGarbageTract(uint32_t& dwCellX, uint32_t& dwCellZ, int32_t& wValue)` |
| 20 | 0x050 | `uint32_t GetGarbageSupply(void)` |
| 21 | 0x054 | `int32_t GetLandfillTileCapacity(void)` |
| 22 | 0x058 | `int32_t GetGarbageCapacity(void)` |
| 23 | 0x05C | `bool IsGarbageUncollected(void)` |
| 24 | 0x060 | `int32_t GetTotalGarbageUncollected(void)` |
| 25 | 0x064 | `bool SetIsRadioactive(uint32_t dwCellX, uint32_t dwCellZ, bool bRadioactive)` |
| 26 | 0x068 | `bool SetAirValue(uint32_t dwCellX, uint32_t dwCellZ, int16_t wValue)` |
| 27 | 0x06C | `bool SetWaterValue(uint32_t dwCellX, uint32_t dwCellZ, int16_t wValue)` |
| 28 | 0x070 | `bool SetGarbageValue(uint32_t dwCellX, uint32_t dwCellZ, int16_t wValue)` |
| 29 | 0x074 | `bool AddToGarbageSupply(int32_t nGarbage)` |
| 30 | 0x078 | `bool SetGarbageCapacity(int32_t nCapacity)` |
| 31 | 0x07C | `int32_t GetMonthlyIncineratorCapacity(void)` |
| 32 | 0x080 | `int32_t GetMonthlyWasteToEnergyCapacity(void)` |
| 33 | 0x084 | `int32_t GetRecyclingCenterEffect(void)` |
| 34 | 0x088 | `int32_t GetGarbageAbductedPrevMonth(void)` |
| 35 | 0x08C | `int32_t GetGarbageProducedLastMonth(void)` |
| 36 | 0x090 | `int32_t GetGarbageImportedLastMonth(void)` |
| 37 | 0x094 | `int32_t GetGarbageExportedLastMonth(void)` |
| 38 | 0x098 | `int32_t GetGarbageRecycledLastMonth(void)` |
| 39 | 0x09C | `int32_t GetGarbageIncineratedLastMonth(void)` |
| 40 | 0x0A0 | `int32_t GetGarbageConvertedToEnergyLastMonth(void)` |
| 41 | 0x0A4 | `int32_t GetGarbageLandfilledLastMonth(void)` |
| 42 | 0x0A8 | `uint32_t GetIncineratorCount(void)` |
| 43 | 0x0AC | `uint32_t GetWasteToEnergyBuildingCount(void)` |
| 44 | 0x0B0 | `uint32_t GetRecyclingCenterCount(void)` |
| 45 | 0x0B4 | `uint32_t GetWaterTreatmentPlantCount(void)` |
| 46 | 0x0B8 | `bool GetGarbageBuildings(SC4List<cISC4Occupant*>& sList)` |
| 47 | 0x0BC | `uint32_t GetBuildingGarbageCapacity(cISC4Occupant* pOccupant)` |
| 48 | 0x0C0 | `float GetGarbageScalingFactor(void)` |
| 49 | 0x0C4 | `bool IsActiveLandfill(uint32_t dwCellX, uint32_t dwCellZ)` |
| 50 | 0x0C8 | `bool SetIsActiveLandfill(uint32_t dwCellX, uint32_t dwCellZ, bool bActive)` |
| 51 | 0x0CC | `int64_t GarbagePutInLandfill(int32_t nGarbage)` |
| 52 | 0x0D0 | `bool GetMaxCapacity(uint32_t dwProperty, cISC4Occupant* pOccupant, uint32_t& dwResult)` |
| 53 | 0x0D4 | `bool GetCurrentCapacity(uint32_t dwProperty, cISC4Occupant* pOccupant, uint32_t& dwResult)` |
| 54 | 0x0D8 | `bool GetRecyclingCenterMaxCapacity(cISC4Occupant* pOccupant, SC4Percentage& sCapacity)` |
| 55 | 0x0DC | `bool GetRecyclingCenterCurrentCapacity(cISC4Occupant* pOccupant, SC4Percentage& sCapacity)` |
| 56 | 0x0E0 | `bool GetCurrentOccupantCondition(cISC4Occupant* pOccupant, SC4Percentage& sCapacity)` |
| 57 | 0x0E4 | `bool GetCurrentOccupantEfficiency(cISC4Occupant* pOccupant, SC4Percentage& sCapacity)` |
| 58 | 0x0E8 | `bool GetIncineratorUsedCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 59 | 0x0EC | `bool GetWasteToEnergyUsedCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 60 | 0x0F0 | `bool GetIncineratorCurrentCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 61 | 0x0F4 | `bool GetWasteToEnergyCurrentCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 62 | 0x0F8 | `bool GetWaterTreatmentMaxCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 63 | 0x0FC | `bool GetWaterTreatmentCurrentCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 64 | 0x100 | `bool GetWaterTreatmentUsedCapacity(cISC4Occupant* pOccupant, uint32_t& dwCapacity)` |
| 65 | 0x104 | `bool CalcOccupantMaintenanceCost(cISC4Occupant* pOccupant, uint64_t& dwCost)` |
| 66 | 0x108 | `void AddRadiationToCity(cS3DVector3& sCenter, float fRadiusX, float fRadiusZ)` |
| 67 | 0x10C | `int32_t GetMaxAirPollutionLevelForUI(void)` |
| 68 | 0x110 | `int32_t GetMaxWaterPollutionLevelForUI(void)` |
| 69 | 0x114 | `int32_t GetMaxGarbagePollutionLevelForUI(void)` |
| 70 | 0x118 | `int32_t GetMaxRadiationPollutionLevelForUI(void)` |

#### cISC4LandValueSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `LandValueType GetLandValueType(int32_t x, int32_t z)` |
| 6 | 0x018 | `uint8_t GetLandValue(int32_t x, int32_t z)` |
| 7 | 0x01C | `cISC4SimGrid<uint8_t>* GetIntrinsicLandValueMap()` |
| 8 | 0x020 | `cISC4SimGrid<uint8_t>* GetTotalLandValueMap()` |
| 9 | 0x024 | `cISC4SimGrid<float>* GetSlopeMap()` |
| 10 | 0x028 | `uint8_t GetAverageLandValue()` |
| 11 | 0x02C | `float GetModeAltitude()` |

#### cISC4CrimeSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `cISC4SimGrid<uint8_t>* GetCrimeGrid()` |
| 6 | 0x018 | `uint32_t GetAverageCrime()` |
| 7 | 0x01C | `uint32_t GetMonthlyCrimeCount()` |
| 8 | 0x020 | `bool GetCrimeHotspot(uint8_t* unknown1, int32_t* unknown2, int32_t* unknown3)` |
| 9 | 0x024 | `cISC4Automaton* GetNearestCrimeSim(cS3DVector3 const& unknown1, float unknown2)` |
| 10 | 0x028 | `cISC4PropOccupant* GetNearestCrimeProp(cS3DVector3 const& unknown1, float unknown2)` |
| 11 | 0x02C | `bool GetCrimeIncidentArray(SC4Vector<tCrimeEvent>& vector)` |
| 12 | 0x030 | `bool SetVisualsCreated(uint32_t unknown1, bool unknown2)` |
| 13 | 0x034 | `bool GetVisualsCreated(uint32_t unknown1)` |

#### cISC4PoliceSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `cISC4SimGrid<short>* GetPolicePowerGrid(void)` |
| 6 | 0x018 | `intptr_t GetDepartmentBudget(void)` |
| 7 | 0x01C | `bool GetStationFunding(cISC4Occupant* pStation, int32_t& nFunding, SC4Percentage& sFundingPercent)` |
| 8 | 0x020 | `bool SetStationFunding(cISC4Occupant* pStation, SC4Percentage const& sFundingPercentage)` |
| 9 | 0x024 | `uint32_t GetJailDepartmentBudget(void)` |
| 10 | 0x028 | `bool IsOnStrike(void)` |
| 11 | 0x02C | `float GetChanceOfStrike(void)` |
| 12 | 0x030 | `bool BeginStrike(cISC4Occupant* pStation)` |
| 13 | 0x034 | `bool EndStrike(void)` |
| 14 | 0x038 | `uint32_t GetStrikeStartDate(void)` |
| 15 | 0x03C | `uint32_t GetStrikeLength(void)` |
| 16 | 0x040 | `bool GetStrikingStations(SC4List<cISC4Occupant*>& sList)` |
| 17 | 0x044 | `bool IsStationOnStrike(cISC4Occupant* pStation)` |
| 18 | 0x048 | `float GetAverageCoverage(void)` |
| 19 | 0x04C | `float GetPercentCoverage(void)` |
| 20 | 0x050 | `bool GetOccupantCoverage(cISC4Occupant* pStation, SC4Percentage const& sPercentage, float& fRadiusX, float& fRadiusZ)` |
| 21 | 0x054 | `bool GetPoliceStations(SC4List<cISC4Occupant*>& sList)` |
| 22 | 0x058 | `bool IsPoliceStation(cISC4Occupant* pBuilding)` |
| 23 | 0x05C | `float GetStationDispatchRadius(cISC4Occupant* pStation)` |
| 24 | 0x060 | `uint32_t GetPoliceStationCount(void)` |
| 25 | 0x064 | `float GetStationEfficiency(cISC4Occupant* pStation)` |
| 26 | 0x068 | `uint32_t GetTotalDispatchCount(void)` |
| 27 | 0x06C | `bool ResetUnjailedCriminals(void)` |
| 28 | 0x070 | `uint32_t GetJailCapacity(void)` **(overload: MSVC slot order may differ)** |
| 29 | 0x074 | `uint32_t GetJailPopulation(void)` |
| 30 | 0x078 | `uint32_t GetCriminalCount(void)` |
| 31 | 0x07C | `int32_t GetJails(SC4List<cISC4Occupant*>& sList)` |
| 32 | 0x080 | `bool IsJail(cISC4Occupant* pBuilding)` |
| 33 | 0x084 | `uint32_t GetArrestCount(void)` |
| 34 | 0x088 | `uint32_t GetJailCount(void)` |
| 35 | 0x08C | `uint32_t GetJailCapacity(cISC4Occupant* pJail)` **(overload: MSVC slot order may differ)** |
| 36 | 0x090 | `uint32_t GetJailedPercentage(void)` |
| 37 | 0x094 | `int64_t GetJailCurrentUpkeepCost(void)` |
| 38 | 0x098 | `bool ProcessMonthlyCrimeStats(void)` |
| 39 | 0x09C | `bool IsInJailbreak(void)` |
| 40 | 0x0A0 | `float BeginJailbreak(cISC4Occupant* pJail)` |
| 41 | 0x0A4 | `bool EndJailbreak(cISC4Occupant* pJail)` |
| 42 | 0x0A8 | `bool GetJailbreakBuildings(SC4List<cISC4Occupant*>& sList)` |
| 43 | 0x0AC | `uint32_t GetPoliceStationArrestCount(cISC4Occupant* pStation)` |
| 44 | 0x0B0 | `uint32_t GetPoliceStationCrimeCount(cISC4Occupant* pStation)` |

#### cISC4FireProtectionSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `cISC4SimGrid<int16_t>* GetFireProtectionGrid() const` |
| 6 | 0x018 | `cISC4DepartmentBudget* GetDepartmentBudget() const` |
| 7 | 0x01C | `bool IsFireStation(cISC4Occupant* pStation) const` |
| 8 | 0x020 | `bool GetStationFunding(cISC4Occupant* pStation, int32_t& nFunding, SC4Percentage& sFundingPercent) const` |
| 9 | 0x024 | `bool SetStationFunding(cISC4Occupant* pStation, SC4Percentage const& sFundingPercent)` |
| 10 | 0x028 | `float GetStationEfficiency()` |
| 11 | 0x02C | `bool IsOnStrike()` |
| 12 | 0x030 | `float GetChanceOfStrike()` |
| 13 | 0x034 | `bool BeginStrike(cISC4Occupant* pStation)` |
| 14 | 0x038 | `bool EndStrike()` |
| 15 | 0x03C | `uint32_t GetStrikeStartDate()` |
| 16 | 0x040 | `uint32_t GetStrikeLength()` |
| 17 | 0x044 | `bool GetStrikingStations(SC4List<cISC4Occupant*>& list)` |
| 18 | 0x048 | `bool IsStationOnStrike(cISC4Occupant* pStation)` |
| 19 | 0x04C | `float GetAverageCoverage()` |
| 20 | 0x050 | `float GetPercentCoverage()` |
| 21 | 0x054 | `bool GetOccupantCoverage(cISC4Occupant* pStation, SC4Percentage const& sPercentage, float& fRadiusX, float& fRadiusZ)` |
| 22 | 0x058 | `int32_t GetFireStations(ilist<cISC4Occupant*>& list)` |
| 23 | 0x05C | `uint32_t GetFireStationCount()` |
| 24 | 0x060 | `float GetStationDispatchRadius(cISC4Occupant* pStation)` |
| 25 | 0x064 | `uint32_t GetTotalDispatchCount()` |

#### cISC4FlammabilitySimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `cISC4SimGrid<uint8_t>* GetFlammabilityGrid()` |
| 6 | 0x018 | `bool IsCellFlammable(int x, int z)` |
| 7 | 0x01C | `bool GetFlammabilityHotspot(uint8_t* param_1, int* param_2, int* param_3)` |
| 8 | 0x020 | `uint8_t GetAverageFlammability()` |

#### cISC4PowerSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `void PositionToCell(float unknown1, float unknown2, int32_t& unknown3, int32_t& unknown4)` |
| 6 | 0x018 | `void CellToPosition(int32_t unknown1, int32_t unknown2, float& unknown3, float& unknown4)` |
| 7 | 0x01C | `void CellToStandardCityCell(int32_t unknown1, int32_t unknown2, int32_t& unknown3, int32_t& unknown4)` |
| 8 | 0x020 | `void StandardCityCellToCell(int32_t unknown1, int32_t unknown2, int32_t& unknown3, int32_t& unknown4)` |
| 9 | 0x024 | `uint32_t GetAnnualConsumption()` |
| 10 | 0x028 | `uint32_t GetAnnualConsumptionByPowerType(int32_t powerType)` |
| 11 | 0x02C | `int32_t GetMonthlyRequirement()` |
| 12 | 0x030 | `int32_t GetMonthlyProduction()` |
| 13 | 0x034 | `int32_t GetMonthlyImport()` |
| 14 | 0x038 | `int32_t GetMonthlyExport()` |
| 15 | 0x03C | `int32_t GetMonthlyDemand()` |
| 16 | 0x040 | `int32_t GetMonthlyConsumption()` **(overload: MSVC slot order may differ)** |
| 17 | 0x044 | `int32_t GetMonthlyConsumption(int32_t x, int32_t z)` **(overload: MSVC slot order may differ)** |
| 18 | 0x048 | `void GetMonthlyConsumptionMapPtr(SC4Vector<SC4Vector<int16_t>>*& vector)` |
| 19 | 0x04C | `bool GetSupplyValue(int32_t x, int32_t z)` |
| 20 | 0x050 | `void GetSupplyValueMapPtr(cISC4SimGrid<uint8_t>*& grid)` |
| 21 | 0x054 | `int32_t GetUnpoweredConsumerCount()` |
| 22 | 0x058 | `int32_t GetPoweredConsumerCount()` |
| 23 | 0x05C | `int32_t GetPowerPlantCount()` **(overload: MSVC slot order may differ)** |
| 24 | 0x060 | `int32_t GetPowerPlantCount(int32_t powerType)` **(overload: MSVC slot order may differ)** |
| 25 | 0x064 | `void GetPowerPlants(ilist<cISC4Occupant*>& list)` |
| 26 | 0x068 | `uint32_t GetPowerPoleCount()` |
| 27 | 0x06C | `int32_t GetOverworkedPowerPlantCount()` |
| 28 | 0x070 | `int32_t GetAgedPowerPlantCount()` |
| 29 | 0x074 | `int32_t GetDistressedPowerPlantCount()` |
| 30 | 0x078 | `void FindPowerPlants(cISC4Occupant** unknown1, SC4Percentage& unknown2, cISC4Occupant** unknown3, int32_t& unknown4)` |
| 31 | 0x07C | `bool IsOccupantPowered(cISC4Occupant* occupant)` |
| 32 | 0x080 | `void GetOccupantPowerConsumed(cISC4Occupant* occupant, uint32_t& powerConsumed)` |
| 33 | 0x084 | `void GetOccupantPowerProduced(cISC4Occupant* occupant, uint32_t& powerProduced, uint32_t& powerUsed)` |
| 34 | 0x088 | `void GetOccupantMaxPowerProduced(cISC4Occupant* occupant, uint32_t& maxPowerProduced)` |
| 35 | 0x08C | `cISC4SimGrid<uint8_t>* GetPoweredGrid() const` **(overload: MSVC slot order may differ)** |
| 36 | 0x090 | `cISC4SimGrid<uint8_t>* GetPoweredGrid()` **(overload: MSVC slot order may differ)** |
| 37 | 0x094 | `bool IsPowered(int32_t x, int32_t z)` |
| 38 | 0x098 | `bool IsWired(int32_t x, int32_t z)` |
| 39 | 0x09C | `bool IsOnStrike()` |
| 40 | 0x0A0 | `void GetChanceOfStrike(SC4Percentage& chance)` |
| 41 | 0x0A4 | `bool BeginStrike()` |
| 42 | 0x0A8 | `bool EndStrike()` |
| 43 | 0x0AC | `int32_t GetStrikeStartDate()` |
| 44 | 0x0B0 | `int32_t GetStrikeLength()` |
| 45 | 0x0B4 | `float GetCurrentPlantEfficiencyPercent(cISC4Occupant* occupant)` |
| 46 | 0x0B8 | `float GetCurrentPlantConditionPercent(cISC4Occupant* occupant)` |
| 47 | 0x0BC | `SC4Percentage* GetStrikeEfficiencyPercent()` |
| 48 | 0x0C0 | `SC4Percentage* GetFundingPercent()` |
| 49 | 0x0C4 | `bool GetMonthlyPowerGenerationCost(cISC4Occupant* occupant, int64_t& cost)` |
| 50 | 0x0C8 | `bool GetPowerPlantFundingPercentage(cISC4Occupant* occupant, SC4Percentage& fundingPercentage)` |
| 51 | 0x0CC | `bool SetPowerPlantFundingPercentage(cISC4Occupant* occupant, SC4Percentage const& fundingPercentage)` |

#### cISC4PlumbingSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `uint8_t GetSupplyValue(int32_t cX, int32_t cZ)` |
| 6 | 0x018 | `bool GetSupplyValueMapPtr(cISC4SimGrid<uint8_t>*& grid)` |
| 7 | 0x01C | `cISC4SimGrid<uint8_t>* GetWateredGrid()` |
| 8 | 0x020 | `bool AreWateredBuildingsInCell(int32_t cX, int32_t cZ)` |
| 9 | 0x024 | `bool IsBuildingWatered(cISC4Occupant* occupant)` |
| 10 | 0x028 | `uint32_t GetWaterProduction(cISC4Occupant* occupant)` |
| 11 | 0x02C | `bool IsCellWatered(int32_t cX, int32_t cZ)` |
| 12 | 0x030 | `uint32_t GetNumWaterSources()` |
| 13 | 0x034 | `uint32_t GetNumPollutedWaterSources()` |
| 14 | 0x038 | `int64_t GetWaterProductionCapacity()` |
| 15 | 0x03C | `int64_t GetWaterConsumed()` |
| 16 | 0x040 | `int64_t GetWaterImported()` |
| 17 | 0x044 | `int64_t GetWaterExported()` |
| 18 | 0x048 | `uint32_t GetNumPipes()` |
| 19 | 0x04C | `uint32_t GetNumDistressedPipes()` |
| 20 | 0x050 | `uint32_t GetNumBurstPipes()` |
| 21 | 0x054 | `uint32_t GetNumWateredBuildings()` |
| 22 | 0x058 | `uint32_t GetNumUnwateredBuildings()` |
| 23 | 0x05C | `int64_t GetWaterDemand()` |
| 24 | 0x060 | `bool GetWaterProducerInfo(cISC4Occupant* producer, uint32_t* currentCapacity, uint32_t* maxCapacity, SC4Percentage* efficiency, bool* shutDownByPollution, uint32_t* waterUsed)` |
| 25 | 0x064 | `bool GetWaterProducerConditionPercent(cISC4Occupant* producer, SC4Percentage& conditionPercent)` |
| 26 | 0x068 | `bool GetOccupantMonthlyUpkeepCost(cISC4Occupant* occupant, int64_t& monthlyUpkeepCost)` |
| 27 | 0x06C | `bool FindWaterBuildings(cISC4Occupant** unknown1, SC4Percentage& unknown2, cISC4Occupant** unknown3, long& unknown4)` |
| 28 | 0x070 | `bool GetWaterProducers(ilist<cISC4Occupant*>& list)` |

#### cISC4BudgetSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool SetTotalFunds(int64_t llFunds)` |
| 6 | 0x018 | `int64_t GetTotalFunds(void)` |
| 7 | 0x01C | `bool DepositFunds(int64_t llFunds)` |
| 8 | 0x020 | `bool WithdrawFunds(int64_t llFunds)` |
| 9 | 0x024 | `int64_t GetMinAllowableFunds(void)` |
| 10 | 0x028 | `int64_t GetYTDIncome(void)` |
| 11 | 0x02C | `int64_t GetEstIncome(void)` |
| 12 | 0x030 | `int64_t GetYTDExpenses(void)` |
| 13 | 0x034 | `int64_t GetEstExpenses(void)` |
| 14 | 0x038 | `uint32_t GetTotalMonthlyExpense(void)` |
| 15 | 0x03C | `int64_t GetTotalYearlyExpense(void)` |
| 16 | 0x040 | `int32_t GetTotalMonthlyIncome(void)` |
| 17 | 0x044 | `int64_t GetTotalYearlyIncome(void)` |
| 18 | 0x048 | `int64_t GetTaxIncome(void)` **(overload: MSVC slot order may differ)** |
| 19 | 0x04C | `int64_t GetTaxIncome(int32_t nTaxType)` **(overload: MSVC slot order may differ)** |
| 20 | 0x050 | `int64_t SetTotalMonthlyExpense(int64_t llExpense)` |
| 21 | 0x054 | `int64_t SetTotalYearlyExpense(int64_t llExpense)` |
| 22 | 0x058 | `int64_t SetTotalMonthlyIncome(int64_t llIncome)` |
| 23 | 0x05C | `int64_t SetTotalYearlyIncome(int64_t llIncome)` |
| 24 | 0x060 | `bool ShowBudgetWindow(void)` |
| 25 | 0x064 | `cISC4DepartmentBudget* CreateDepartmentBudget(uint32_t dwDepartmentID, uint32_t dwBudgetGroup)` |
| 26 | 0x068 | `bool RemoveDepartmentBudget(uint32_t dwDepartmentID)` |
| 27 | 0x06C | `cISC4DepartmentBudget* GetDepartmentBudget(uint32_t dwDepartmentID)` **(overload: MSVC slot order may differ)** |
| 28 | 0x070 | `cISC4DepartmentBudget* GetDepartmentBudget(cIGZString const& szDepartmentName)` **(overload: MSVC slot order may differ)** |
| 29 | 0x074 | `bool GetAllGroups(SC4Vector<BudgetGroupInfo>& sGroups)` |
| 30 | 0x078 | `bool SetGroupName(uint32_t dwGroupID, cIGZString& szName)` |
| 31 | 0x07C | `bool GetDepartmentBudgetsInGroup(uint32_t dwGroupID, SC4Vector<cISC4DepartmentBudget*>& sGroups)` |
| 32 | 0x080 | `void NeededFundingChanged(cISC4DepartmentBudget* pDepartmentBudget)` |
| 33 | 0x084 | `void FundingPercentageChanged(cISC4DepartmentBudget* pDepartmentBudget)` |
| 34 | 0x088 | `int64_t GetFunding(uint32_t dwUnknownID)` **(overload: MSVC slot order may differ)** |
| 35 | 0x08C | `int64_t GetFunding(cISC4DepartmentBudget* pDepartmentBudget)` **(overload: MSVC slot order may differ)** |
| 36 | 0x090 | `float GetTaxRate(uint32_t dwTaxGroup)` |
| 37 | 0x094 | `bool SetTaxRate(uint32_t dwTaxGroup, float fRate)` |
| 38 | 0x098 | `int64_t GetWeightedAssessedTaxValue(uint32_t dwTaxGroup)` |
| 39 | 0x09C | `int64_t GetBondIncrement(void)` |
| 40 | 0x0A0 | `bool IssueBond(uint32_t dwBondAmount)` |
| 41 | 0x0A4 | `int64_t GetTotalBorrowed(void)` |
| 42 | 0x0A8 | `int64_t GetCurrentBorrowingLimit(void)` |
| 43 | 0x0AC | `int64_t GetCurrentBondLimit(void)` |
| 44 | 0x0B0 | `int64_t GetCurrentMaxOutstandingBondsLimit(void)` |
| 45 | 0x0B4 | `int64_t GetTotalMonthlyBondPayments(void)` |
| 46 | 0x0B8 | `bool GetAllLoans(SC4Vector<LoanInfo>& sLoans)` |
| 47 | 0x0BC | `int32_t GetLoanTimeInMonths(void)` |
| 48 | 0x0C0 | `int64_t GetMonthlyPaymentForLoan(int64_t llLoanAmount)` |
| 49 | 0x0C4 | `int64_t GetFullCostOfLoan(int64_t llLoanAmount)` |
| 50 | 0x0C8 | `bool GetBudgetItemInfo(cISCPropertyHolder* pProperty, SC4Vector<BudgetItem>& sBudgetInfo)` |
| 51 | 0x0CC | `bool GetBudgetItemForPurpose(cISCPropertyHolder* pProperty, uint32_t dwPurpose, BudgetItem& sBudgetItem)` |
| 52 | 0x0D0 | `bool ChangeBudgetItemLine(cISCPropertyHolder* pProperty, uint32_t dwPurpose, uint32_t dwLine)` |
| 53 | 0x0D4 | `bool ChangeBudgetItemCost(cISCPropertyHolder* pProperty, uint32_t dwPurpose, int64_t llCost)` |
| 54 | 0x0D8 | `bool ChangeBudgetItemLocalFunding(cISCPropertyHolder* pProperty, uint32_t dwPurpose, SC4Percentage const& sFunding)` |
| 55 | 0x0DC | `bool AddBudgetItemToBudget(cISCPropertyHolder* pProperty, uint32_t dwPurpose)` |
| 56 | 0x0E0 | `bool RemoveBudgetItemFromBudget(cISCPropertyHolder* pProperty, uint32_t dwPurpose)` |
| 57 | 0x0E4 | `cISC4LineItem* GetLineItemFromBudgetItem(BudgetItem& pBudgetItem)` |
| 58 | 0x0E8 | `bool CopyBudgetItemProperties(cISCPropertyHolder* pOriginal, cISCPropertyHolder* pCopy)` |

#### cISC4AuraSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `int8_t GetAuraValue(int32_t x, int32_t z)` |
| 6 | 0x018 | `void AddTransientEffect(float unknown1, float param_2, float param_3, float param_4)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `void AddTransientEffect(float param_1, float param_2, uint32_t transientAuraEffect)` **(overload: MSVC slot order may differ)** |
| 8 | 0x020 | `int8_t GetMayorRating() const` |
| 9 | 0x024 | `int8_t GetMaxMayorRating(SC4Point<long>& location) const` |
| 10 | 0x028 | `int8_t GetMinMayorRating(SC4Point<long>& location) const` |
| 11 | 0x02C | `void BustStrike(eStrikeBuster type)` |
| 12 | 0x030 | `cISC4SimGrid<int8_t>* GetAuraGrid()` |
| 13 | 0x034 | `cISC4SimGrid<int8_t>* GetTransientAuraGrid()` |
| 14 | 0x038 | `cISC4SimGrid<int16_t>* GetParkMap() const` |
| 15 | 0x03C | `cISC4SimGrid<int16_t>* GetLandmarkMap() const` |

#### cISC4CivicBuildingSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool IsBuildingExcluded(uint32_t buildingID)` |
| 6 | 0x018 | `ConditionalBuildingStatus* GetConditionalBuildingStatus(uint32_t buildingID) const` |
| 7 | 0x01C | `bool UpdateConditionalBuildingStatus(uint32_t buildingID, ConditionalBuildingStatus* status)` |
| 8 | 0x020 | `uint32_t GetLandmarkCount()` |

#### cISC4NeighborsSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool RequestCreateConnection(ConnectionType type, int32_t x, int32_t z, bool unknown4)` |
| 6 | 0x018 | `cISC4NeighborConnection* CreateConnection(ConnectionType type, int32_t x, int32_t z, bool unknown4)` |
| 7 | 0x01C | `void DeleteConnection(ConnectionType type, int32_t x, int32_t z)` |
| 8 | 0x020 | `void DeleteOccupantConnection(cISC4Occupant* pOccupant)` |
| 9 | 0x024 | `void DeleteAllConnectionsOfType(ConnectionType type)` |
| 10 | 0x028 | `bool HasConnection(ConnectionType type, int32_t x, int32_t z) const` |
| 11 | 0x02C | `cISC4NeighborConnection* FindConnection(ConnectionType type, int32_t x, int32_t z)` |
| 12 | 0x030 | `cISC4RegionalCity* FindCityOnOtherSideOfConnection(int32_t x, int32_t z)` |
| 13 | 0x034 | `cISC4NeighborConnection* FindConnectionOnOtherSide(ConnectionType connectionType, int32_t x, int32_t z)` **(overload: MSVC slot order may differ)** |
| 14 | 0x038 | `cISC4NeighborConnection* FindConnectionOnOtherSide(cISC4NeighborConnection* pConnection)` **(overload: MSVC slot order may differ)** |
| 15 | 0x03C | `cISC4NeighborConnection* FindUnvisitedConnection(ConnectionType type, cISC4RegionalCity const* pRegionalCity)` |
| 16 | 0x040 | `void GetAllConnections(SC4List<cISC4NeighborConnection*>& list) const` |
| 17 | 0x044 | `void GetConnectionsOfType(ConnectionType type, SC4List<cISC4NeighborConnection*>& list) const` |
| 18 | 0x048 | `void GetConnectionsOfTypeToCity(ConnectionType type, cISC4RegionalCity const* pRegionalCity, SC4List<cISC4NeighborConnection*>& list) const` |
| 19 | 0x04C | `void GetConnectionsToCity(cISC4RegionalCity const* pRegionalCity, SC4List<cISC4NeighborConnection*>& list)` |
| 20 | 0x050 | `void GetAdjacentCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished)` |
| 21 | 0x054 | `void GetConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished)` |
| 22 | 0x058 | `void GetAllConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished)` |
| 23 | 0x05C | `cISC4NeighborConnection* FindConnectionSuitableForDeal(int32_t dealType, cISC4RegionalCity const* pRegionalCity)` |
| 24 | 0x060 | `bool ProposeDeal(int32_t dealType, cISC4RegionalCity* otherCity, cISC4NeighborDeal** outDeal)` |
| 25 | 0x064 | `bool ProposeDeals(SC4List<cISC4NeighborConnection*>& deals, bool unknown2)` |
| 26 | 0x068 | `bool AddDeal(cISC4NeighborDeal* pDeal)` |
| 27 | 0x06C | `void SetUIDisableFlag(bool flag)` |
| 28 | 0x070 | `void FindDealsAll(SC4List<cISC4NeighborConnection*>& deals)` |
| 29 | 0x074 | `void FindDeals(int32_t dealType, SC4List<cISC4NeighborConnection*>& deals)` **(overload: MSVC slot order may differ)** |
| 30 | 0x078 | `void FindDeals(int32_t dealType, cISC4RegionalCity* pRegionalCity, SC4List<cISC4NeighborConnection*>& deals)` **(overload: MSVC slot order may differ)** |
| 31 | 0x07C | `void MarkDealsDirty()` |
| 32 | 0x080 | `int64_t GetTotalMonthlyNeighborDealIncome()` |
| 33 | 0x084 | `int64_t GetTotalMonthlyNeighborDealExpense()` |
| 34 | 0x088 | `void UpdateCommodityStats(cISC4City* pCity)` |
| 35 | 0x08C | `int64_t GetDealCost(int32_t dealType, double unknown2, cISC4RegionalCity const* pRegionalCity)` |
| 36 | 0x090 | `int64_t GetDealMaxAmount(int32_t dealType, cISC4RegionalCity const* pRegionalCity)` |
| 37 | 0x094 | `cISCResExemplar* GetTuningExemplar()` |

#### cISC4OrdinanceSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool AddOrdinance(cISC4Ordinance& ordinance)` |
| 6 | 0x018 | `bool RemoveOrdinance(cISC4Ordinance& ordinance)` |
| 7 | 0x01C | `bool IsOrdinanceAvailable(uint32_t clsid)` |
| 8 | 0x020 | `bool IsOrdinanceOn(uint32_t clsid)` |
| 9 | 0x024 | `bool IsOrdinanceAvailableButOff(uint32_t clsid)` |
| 10 | 0x028 | `bool SetOrdinanceAvailable(uint32_t clsid, bool available)` |
| 11 | 0x02C | `uint32_t GetOrdinanceAvailableCount(void)` |
| 12 | 0x030 | `uint32_t GetOrdinanceAvailableArray(uint32_t* pCLSIDsOut, uint32_t& dwCountOut)` |
| 13 | 0x034 | `bool SetOrdinanceOn(uint32_t clsid, bool on)` |
| 14 | 0x038 | `uint32_t GetOrdinanceOnCount(void)` |
| 15 | 0x03C | `uint32_t GetOrdinanceOnArray(uint32_t* pCLSIDsOut, uint32_t& dwCountOut)` |
| 16 | 0x040 | `bool IsOrdinanceEnabled(uint32_t clsid)` |
| 17 | 0x044 | `bool SetOrdinanceEnabled(uint32_t clsid, bool enabled)` |
| 18 | 0x048 | `uint32_t GetOrdinanceIDArray(uint32_t* pCLSIDsOut, uint32_t& dwCountOut)` |
| 19 | 0x04C | `uint32_t GetOrdinanceIDByName(cIGZString const& name)` |
| 20 | 0x050 | `cISC4Ordinance* GetOrdinanceByID(uint32_t clsid)` |
| 21 | 0x054 | `cISC4Ordinance* GetOrdinanceByName(cIGZString const& name)` |
| 22 | 0x058 | `int64_t GetOrdinanceMonthlyExpense(void)` |
| 23 | 0x05C | `int64_t GetOrdinanceMonthlyIncome(void)` |
| 24 | 0x060 | `int64_t GetCumulativeOffset(uint32_t unknown1, uint32_t unknown2)` |
| 25 | 0x064 | `float GetCumulativeFactor(uint32_t unknown1, uint32_t unknown2)` |

#### cISC4WeatherSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool DoPostCityInit()` |
| 6 | 0x018 | `float CellWidthX()` |
| 7 | 0x01C | `float CellWidthZ()` |
| 8 | 0x020 | `uint32_t CellCountX()` |
| 9 | 0x024 | `uint32_t CellCountZ()` |
| 10 | 0x028 | `int32_t PositionToCell(float fX, float fZ, int& cX, int& cZ)` |
| 11 | 0x02C | `void CellToPosition(int32_t cX, int32_t cZ, float& fX, float& fZ)` |
| 12 | 0x030 | `void CellToStandardCityCell(int32_t unknown1, int32_t unknown2, int32_t& unknown3, int32_t& unknown4)` |
| 13 | 0x034 | `void StandardCityCellToCell(int32_t unknown1, int32_t unknown2, int32_t& unknown3, int32_t& unknown4)` |
| 14 | 0x038 | `Season GetCurrentSeason()` |
| 15 | 0x03C | `int32_t GetCurrentSeasonMonth()` |
| 16 | 0x040 | `void UpdateMoisture(SC4Rect<int32_t> const& rect)` |
| 17 | 0x044 | `uint8_t GetMoistureValue(float x, float z)` |
| 18 | 0x048 | `void GetMoistureValueMapPtr(SC4Vector<SC4Vector<uint8_t>>& vector)` |
| 19 | 0x04C | `uint8_t GetMoistureType(float x, float z)` |
| 20 | 0x050 | `void SetManualMoistureAddition(float unknown1, float unknown2, float unknown3, float unknown4, float unknown5)` |
| 21 | 0x054 | `float GetHumidity(float x, float z)` |
| 22 | 0x058 | `float GetAmbientWind(cS3DVector2& ambientWind)` |
| 23 | 0x05C | `float GetWindAtCell(int x, int z, cS3DVector2& ambientWind)` |

#### cISC4FloraSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool AddStaticFloraData(cSC4StaticFloraData* unknown1)` **(overload: MSVC slot order may differ)** |
| 6 | 0x018 | `bool AddStaticFloraData(cGZPersistResourceKey* unknown1, cISCResExemplar* unknown2)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `cSC4StaticFloraData* GetStaticFloraData(uint32_t type)` |
| 8 | 0x020 | `uint32_t GetAllFloraTypes(uint32_t* buffer, uint32_t& count)` **(overload: MSVC slot order may differ)** |
| 9 | 0x024 | `void GetAllFloraTypes(SC4Vector<uint32_t>& vector)` **(overload: MSVC slot order may differ)** |
| 10 | 0x028 | `void GetAllFloraFamilyTypes(SC4Vector<uint32_t>& vector)` |
| 11 | 0x02C | `bool IsFloraTypeValid(uint32_t type)` |
| 12 | 0x030 | `bool IsFloraFamilyTypeValid(uint32_t type)` |
| 13 | 0x034 | `uint32_t GetFloraTypesByFilter(intptr_t& hash_set, cISC4FloraTypeFilter* unknown2)` |
| 14 | 0x038 | `uint32_t GetFloraTypesByFamily(intptr_t& hash_set, uint32_t unknown2)` |
| 15 | 0x03C | `bool LocationIsOKForNewFlora(uint32_t unknown1, float unknown2, float unknown3, cISC4Occupant* pOccupant)` |
| 16 | 0x040 | `uint32_t GetBestFloraTypeForLocation(float unknown1, float unknown2, float& unknown3)` |
| 17 | 0x044 | `bool AddNewFloraOccupant(cISC4Occupant* pOccupant)` **(overload: MSVC slot order may differ)** |
| 18 | 0x048 | `cISC4Occupant* AddNewFloraOccupant(uint32_t unknown1, float* unknown2, bool unknown3)` **(overload: MSVC slot order may differ)** |
| 19 | 0x04C | `bool DemolishFloraOccupant(cISC4Occupant* pOccupant, int32_t unknown2)` |
| 20 | 0x050 | `bool DemolishFloraInCellArea(SC4Rect<int32_t> const& rect, int32_t unknown2)` |
| 21 | 0x054 | `bool DemolishFloraInBoundingBox(float const* unknown1, float const* unknown2, int32_t unknown3)` |
| 22 | 0x058 | `int64_t GetCostOfOccupantDemolition(cISC4Occupant* pOccupant, int32_t unknown2)` |
| 23 | 0x05C | `bool KillFloraOccupant(cISC4Occupant* pOccupant)` |
| 24 | 0x060 | `bool SaveObliterated(cIGZPersistDBSegment* pSegment)` |

#### cISC4MySimAgentSimulator (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init()` |
| 4 | 0x010 | `bool Shutdown()` |
| 5 | 0x014 | `bool GetActiveMySims(SC4Vector<cISC4MySim*>& sims)` |
| 6 | 0x018 | `void MoveIn(SC4String& personalName, bool isMale, cISC4MySim::ZodiacSign sign, SC4String& bio, uint32_t bitmapID, SC4String& bitmapImagePath, uint8_t slot)` |
| 7 | 0x01C | `void TrackLocation(cISC4MySim* pSim)` |
| 8 | 0x020 | `void HideLocation()` |
| 9 | 0x024 | `cISC4MySim* MySimLivesHere(cISC4BuildingOccupant* pBuilding)` |
| 10 | 0x028 | `void CancelMoveIn()` |
| 11 | 0x02C | `void SetActiveUIPanelSim(int32_t slot)` |
| 12 | 0x030 | `bool GetActiveUIPanelSim(int32_t& slot)` |

#### cISC4OccupantManager (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `bool SetCitySize(float fX, float fZ)` |
| 6 | 0x018 | `bool GetOccupantManagerCellSizes(float& fX, float& fZ)` |
| 7 | 0x01C | `bool GetWorldCellCount(int& nX, int& nZ)` |
| 8 | 0x020 | `bool GetOccupantManagerCellCount(int& nX, int& nZ)` |
| 9 | 0x024 | `bool GetOccupantManagerCellBounds(int& nTL, int& nTR, int& nBL, int& nBR)` |
| 10 | 0x028 | `bool WorldCellToOccupantManagerCell(int nX, int nZ, int& nManagerX, int& nManagerZ)` |
| 11 | 0x02C | `bool OccupantManagerCellToWorldCell(int nX, int nZ, int& nWorldX, int& nWorldZ)` |
| 12 | 0x030 | `bool WorldCellToStandardCityCell(int nX, int nZ, int& nCityX, int& nCityZ)` |
| 13 | 0x034 | `bool StandardCityCellToWorldCell(int nX, int nZ, int& nWorldX, int& nWorldZ)` |
| 14 | 0x038 | `bool WorldCellToPosition(int nX, int nZ, float& fX, float& fZ)` |
| 15 | 0x03C | `bool PositionToWorldCell(float fX, float fZ, int& nX, int& nZ)` |
| 16 | 0x040 | `bool OccupantManagerCellToPosition(int nX, int nZ, float& fX, float& fZ)` |
| 17 | 0x044 | `bool PositionToOccupantManagerCell(float fX, float fZ, int& nX, int& nZ)` |
| 18 | 0x048 | `bool StandardCityCellToPosition(int nX, int nZ, float& fX, float& fZ)` |
| 19 | 0x04C | `bool PositionToStandardCityCell(float fX, float fZ, int& nX, int& nZ)` |
| 20 | 0x050 | `bool InsertOccupant(cISC4Occupant* pOccupant, uint32_t dwMsgData)` |
| 21 | 0x054 | `bool RemoveOccupant(cISC4Occupant*, bool bShutOccupantDown, uint32_t dwMsgData)` |
| 22 | 0x058 | `bool RemoveOccupants(int32_t dwUnknown, bool bShutOccupantsDown, uint32_t dwMsgData)` **(overload: MSVC slot order may differ)** |
| 23 | 0x05C | `bool RemoveOccupants(cISC4OccupantFilter* pFilter, bool bShutOccupantsDown, uint32_t dwMsgData)` **(overload: MSVC slot order may differ)** |
| 24 | 0x060 | `bool MoveOccupant(cISC4Occupant* pOccupant, bool bUnknown)` |
| 25 | 0x064 | `bool IsCellEmpty(int32_t nX, int32_t nZ)` |
| 26 | 0x068 | `int32_t GetBoundingCells(int nUnknown1, int nUnknown2, int* nUnknown3[2], int& nUnknown4)` |
| 27 | 0x06C | `bool FindOccupant(cISC4Occupant* pOccupant, bool bUnknown, int* nX, int* nZ)` |
| 28 | 0x070 | `bool GetFirstOccupantByPosition(cISC4Occupant*& ppOccupant, float fX, float fZ, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 29 | 0x074 | `bool GetFirstOccupantByPosition(cISC4Occupant*& ppOccupant, float fX, float fZ, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 30 | 0x078 | `bool GetFirstOccupant(cISC4Occupant*& ppOccupant, int nX, int nZ, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 31 | 0x07C | `bool GetFirstOccupant(cISC4Occupant*& ppOccupant, int nX, int nZ, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 32 | 0x080 | `bool GetFirstOccupantByStandardCityCell(cISC4Occupant*& ppOccupant, int nX, int nZ, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 33 | 0x084 | `bool GetFirstOccupantByStandardCityCell(cISC4Occupant*& ppOccupant, int nX, int nZ, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 34 | 0x088 | `bool GetFirstOccupantByStandardCityCells(cISC4Occupant*& ppOccupant, SC4Point<int> const* northWestCell, SC4Point<int> const* southEastCell, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 35 | 0x08C | `bool GetFirstOccupantByStandardCityCells(cISC4Occupant*& ppOccupant, SC4Point<int> const* northWestCell, SC4Point<int> const* southEastCell, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 36 | 0x090 | `bool GetOccupantsByBBox(SC4List<cISC4Occupant*>& sOccupants, float const* fXCells, float const* fZCells, uint32_t dwType, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 37 | 0x094 | `bool GetOccupantsByBBox(SC4List<cISC4Occupant*>& sOccupants, float const* fXCells, float const* fZCells, cISC4OccupantFilter* pFilter, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 38 | 0x098 | `bool GetOccupantsByOccupantManagerCells(SC4List<cISC4Occupant*>& sOccupants, int const* fXCells, int const* fZCells, uint32_t dwType, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 39 | 0x09C | `bool GetOccupantsByOccupantManagerCells(SC4List<cISC4Occupant*>& sOccupants, int const* fXCells, int const* fZCells, cISC4OccupantFilter* pFilter, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 40 | 0x0A0 | `bool GetOccupantsByStandardCityCells(SC4List<cISC4Occupant*>& sOccupants, SC4Point<int> const* northWestCell, SC4Point<int> const* southEastCell, uint32_t dwType, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 41 | 0x0A4 | `bool GetOccupantsByStandardCityCells(SC4List<cISC4Occupant*>& sOccupants, SC4Point<int> const* northWestCell, SC4Point<int> const* southEastCell, cISC4OccupantFilter* pFilter, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 42 | 0x0A8 | `bool IterateOccupantsByBBox(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, float const* pfUnknown1, float const* pfUnknown2, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 43 | 0x0AC | `bool IterateOccupantsByBBox(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, float const* pfUnknown1, float const* pfUnknown2, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 44 | 0x0B0 | `bool IterateOccupants(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, int const* pnUnknown1, int const* pnUnknown2, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 45 | 0x0B4 | `bool IterateOccupants(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, int const* pnUnknown1, int const* pnUnknown2, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 46 | 0x0B8 | `bool IterateOccupantsByStandardCityCell(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, int const* pnUnknown1, int const* pnUnknown2, uint32_t dwUnknown)` **(overload: MSVC slot order may differ)** |
| 47 | 0x0BC | `bool IterateOccupantsByStandardCityCell(bool(*pfIterator)(cISC4Occupant*, void*), void* pData, int const* pnUnknown1, int const* pnUnknown2, cISC4OccupantFilter* pFilter)` **(overload: MSVC slot order may differ)** |
| 48 | 0x0C0 | `bool ReleaseOccupantList(SC4List<cISC4Occupant*>& sList)` |

#### cISC4LotManager (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `cISC4Lot* GetLot(int32_t nCellX, int32_t nCellZ, bool bUnknown)` **(overload: MSVC slot order may differ)** |
| 6 | 0x018 | `cISC4Lot* GetLot(cS3DVector3 const& sPoint)` **(overload: MSVC slot order may differ)** |
| 7 | 0x01C | `bool GetLotsInCellRect(SC4List<cISC4Lot*>& sLotList, int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nBottomRightCellX, int32_t nBottomRightCellZ, bool bUnknown)` |
| 8 | 0x020 | `bool GetLotsSurroundingLot(cISC4Lot const* pLot, SC4List<cISC4Lot*>& sList, int32_t nFlags)` **(overload: MSVC slot order may differ)** |
| 9 | 0x024 | `bool GetLotsSurroundingLot(int32_t nCellX, int32_t nCellZ, SC4List<cISC4Lot*>& sList, int32_t nFlags)` **(overload: MSVC slot order may differ)** |
| 10 | 0x028 | `bool GetLotsSurroundingRect(SC4Rect<long> const& sRect, SC4List<cISC4Lot*>& sList, int32_t nFlags)` |
| 11 | 0x02C | `bool GetVerticesAtEdgeOfLot(SC4Rect<long> const& sLotCells, int32_t nFlags, SC4Rect<long>& sVertices)` |
| 12 | 0x030 | `bool GetNeighborCellsAtEdgeOfLot(SC4Rect<long> const& sLotCells, int32_t nFlags, SC4Rect<long>& sCells)` |
| 13 | 0x034 | `cISC4Lot* GetRandomLot(void)` |
| 14 | 0x038 | `cISC4Lot* GetOccupantLot(cISC4Occupant* pOccupant)` |
| 15 | 0x03C | `intptr_t GetSafeLotIterator(void)` |
| 16 | 0x040 | `int32_t GetLotCount(void)` |
| 17 | 0x044 | `int32_t GetLotSavvyTerrainVertexAltitude(int32_t nPointX, int32_t nPointY, cISC4Lot* pLot, bool& bSucceeded)` |
| 18 | 0x048 | `int32_t GetLotSavvyTerrainAltitude(float fPointX, float fPointY, bool& bSucceeded)` |
| 19 | 0x04C | `int32_t GetLotSavvyCellAltitude(int32_t nCellX, int32_t nCellZ, bool& bSucceeded)` |
| 20 | 0x050 | `int32_t CanCreateLot(int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nLotWidth, int32_t nLotHeight, bool bZoneNotNeeded, bool bIgnoreConfig, cISC4LotConfiguration* pConfig)` |
| 21 | 0x054 | `int32_t CanCreatePloppedLot(SC4Rect<long> sBounds, int32_t nUnknown, cISC4LotConfiguration* pConfig)` |
| 22 | 0x058 | `bool CreateLotIfPossible(int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nLotWidth, int32_t nLotHeight, int32_t nUnknown, bool bZoneNotNeeded, bool bIgnoreConfig, cISC4LotConfiguration* pConfig)` |
| 23 | 0x05C | `bool CreateLot(int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nLotWidth, int32_t nLotHeight, int32_t nFacing, cISC4Lot*& pLot)` |
| 24 | 0x060 | `bool DeleteLot(cISC4Lot* pLot)` |
| 25 | 0x064 | `bool DeleteLots(int32_t nTopLeftCellX, int32_t nTopLeftCellZ, int32_t nBottomRightCellX, int32_t nBottomRightCellZ)` |
| 26 | 0x068 | `bool GetLotVertexAltitudeAdjustmentRange(int32_t nCellX, int32_t nCellZ, float& fRangeX, float& fRangeY)` |
| 27 | 0x06C | `cISC4SimGrid<short>* GetMaximumCapacityGrid(void)` |

#### cISC4NetworkManager (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `cISC4NetworkTool* GetNetworkTool(int32_t nNetworkType, bool bCreateUnique)` |
| 6 | 0x018 | `bool DestroyNetworkOccupants(SC4List<cISC4Occupant*> const& sOccupants)` |
| 7 | 0x01C | `void QueueRemovals(bool bShouldQueue)` |
| 8 | 0x020 | `bool RemoveOrReplaceOccupant(cISC4NetworkTool* pTool, cISC4Occupant* pOccupant, bool& bUnknownOut)` |
| 9 | 0x024 | `bool IsOccupantSubwayStation(cISC4Occupant* pOccupant)` |
| 10 | 0x028 | `bool FindPipeOccupant(intptr_t pPipeTool, int32_t nUnknown, intptr_t ppPipeOccupant)` |
| 11 | 0x02C | `bool FindPipeConnectionOccupant(intptr_t pPipeTool, int32_t nUnknown, intptr_t ppPipeConnOccupant)` |
| 12 | 0x030 | `bool GetAllIntersectionPieces(cISC4NetworkOccupant* pOccupant, SC4List<cISC4Occupant*>& sList, cISC4OccupantManager* pManager, cISC4NetworkOccupant* pUnknown)` |
| 13 | 0x034 | `bool GetAllIntersectionPieces2(cISC4NetworkOccupant* pOccupant, SC4List<cISC4Occupant*>& sList, cISC4OccupantManager* pManager, cISC4NetworkOccupant* pUnknown, cISC4OccupantFilter* pFilter)` |
| 14 | 0x038 | `uint32_t MapPreviewTexture(uint32_t dwUnknown1, uint8_t& cUnknown)` |
| 15 | 0x03C | `uint32_t MapShadowTexture(uint32_t dwUnknown1, uint8_t& cUnknown, uint32_t dwUnknown2, uint32_t dwUnknown3)` |
| 16 | 0x040 | `uint32_t MapPath(uint32_t dwUnknown1)` |
| 17 | 0x044 | `bool RegisterConstructionCrew(intptr_t pConstructionCrew)` |
| 18 | 0x048 | `bool UnregisterConstructionCrew(intptr_t pConstructionCrew)` |
| 19 | 0x04C | `cISC4OccupantFilter* GetWaterOccupantFilter(void)` |
| 20 | 0x050 | `cISC4OccupantFilter* GetSubwayOccupantFilter(void)` |
| 21 | 0x054 | `bool CreateConnectionNode(cISC4Occupant*& pOccupant, int32_t nNetworkType, SC4Point<int> const& sPoint, uint32_t dwUnknown1, uint32_t dwUnknown2, uint32_t dwUnknown3, uint8_t cUnknown)` |
| 22 | 0x058 | `bool AutoconnectLotNodes(cISC4Lot* pLot, SC4List<cISC4Occupant*> const& sNodes)` |
| 23 | 0x05C | `char const* GetEnglishNetworkName(int32_t nNetworkType)` |
| 24 | 0x060 | `bool AddToRetrofitList(cISC4NetworkOccupant* pOccupant)` |

#### cISC4RegionalCity (base: cIGZUnknown; first own slot = 3)

| idx | vtbl off | method |
|---|---|---|
| 3 | 0x00C | `bool Init(void)` |
| 4 | 0x010 | `bool Shutdown(void)` |
| 5 | 0x014 | `void GetPosition(int32_t& nX, int32_t& nZ) const` |
| 6 | 0x018 | `void SetPosition(int32_t nX, int32_t nZ, bool bDoRearrange)` |
| 7 | 0x01C | `void GetCitySize(int32_t& nX, int32_t& nZ) const` |
| 8 | 0x020 | `void SetCitySize(int32_t nX, int32_t nZ)` |
| 9 | 0x024 | `int32_t GetPopulation(void) const` **(overload: MSVC slot order may differ)** |
| 10 | 0x028 | `int32_t GetCommercialJobs(void) const` |
| 11 | 0x02C | `int32_t GetIndustrialJobs(void) const` |
| 12 | 0x030 | `float GetWorkforcePercentage(void) const` |
| 13 | 0x034 | `int8_t GetMayorRating(void) const` |
| 14 | 0x038 | `cISC4City::DifficultyLevel GetDifficultyLevel(void) const` |
| 15 | 0x03C | `float GetTaxRate(uint32_t dwTaxType) const` |
| 16 | 0x040 | `int32_t GetPopulation(uint32_t dwPopulationType) const` **(overload: MSVC slot order may differ)** |
| 17 | 0x044 | `int32_t GetExtrapolatedPopulation(uint32_t dwPopulationType) const` |
| 18 | 0x048 | `int32_t GetAllowableExtrapolation(uint32_t dwPopulationType) const` |
| 19 | 0x04C | `void ExtrapolateGrowth(uint32_t dwPopulationType, float fAddedPop)` |
| 20 | 0x050 | `cISC4RegionalCity* FindConnection(int32_t nUnknown1, int32_t nUnknown2, int32_t nUnknown3) const` |
| 21 | 0x054 | `void GetAllConnections(SC4List<cISC4NeighborConnection*>& sList) const` |
| 22 | 0x058 | `void ChangeSymmetricConnection(cISC4NeighborConnection* pConnection, bool bUnknown)` |
| 23 | 0x05C | `void SetupPreferences(SC4NewCityPreferences* pPreferences)` |
| 24 | 0x060 | `void UpdateCityCache(SC4NewCityPreferences* pPreferences)` **(overload: MSVC slot order may differ)** |
| 25 | 0x064 | `void SetupCity(cISC4City* pCity)` |
| 26 | 0x068 | `void UpdateCityCache(cISC4City* pCity)` **(overload: MSVC slot order may differ)** |
| 27 | 0x06C | `uint32_t GetCitySerialNumber(void) const` |
| 28 | 0x070 | `void SetCitySerialNumber(uint32_t dwSerialNumber)` |
| 29 | 0x074 | `bool GetOriginalLanguageAndCountry(int32_t& nLanguage, int32_t& nCountry) const` |
| 30 | 0x078 | `bool GetLastLanguageAndCountry(int32_t& nLanguage, int32_t& nCountry)` |
| 31 | 0x07C | `void GetCitySaveFilePath(cIGZString& sPath) const` |
| 32 | 0x080 | `void SetCitySaveFilePath(cIGZString const& sPath)` |
| 33 | 0x084 | `void GetCityName(cIGZString& sName) const` |
| 34 | 0x088 | `void SetCityName(cIGZString const& sName)` |
| 35 | 0x08C | `void GetMayorName(cIGZString& sName) const` |
| 36 | 0x090 | `void SetMayorName(cIGZString const& sName)` |
| 37 | 0x094 | `void GetUtilityAdvisorName(cIGZString& sName) const` |
| 38 | 0x098 | `void SetUtilityAdvisorName(cIGZString const& sName)` |
| 39 | 0x09C | `void GetCityDescription(cIGZString& sDescription) const` |
| 40 | 0x0A0 | `void SetCityDescription(cIGZString const& sDescription)` |
| 41 | 0x0A4 | `uint32_t GetBirthDate(void) const` |
| 42 | 0x0A8 | `void SetBirthDate(uint32_t dwBirthDate)` |
| 43 | 0x0AC | `bool GetEstablished(void) const` |
| 44 | 0x0B0 | `void SetEstablished(bool bEstablished)` |
| 45 | 0x0B4 | `void GetWorldPosition(float& fLongitude, float& fLatitude) const` |
| 46 | 0x0B8 | `void SetWorldPosition(float fLongitude, float fLatitude)` |
| 47 | 0x0BC | `float GetWorldBaseElevation(void) const` **(overload: MSVC slot order may differ)** |
| 48 | 0x0C0 | `void GetWorldBaseElevation(float fElevation)` **(overload: MSVC slot order may differ)** |
| 49 | 0x0C4 | `cISC4City::WorldHemisphere GetWorldHemisphere(void) const` |
| 50 | 0x0C8 | `float GetBudget(void) const` |
| 51 | 0x0CC | `void SetBudget(float fBudget)` |
| 52 | 0x0D0 | `float GetIncome(void) const` |
| 53 | 0x0D4 | `void SetIncome(float fIncome)` |
| 54 | 0x0D8 | `float GetExported(int32_t nCommodity) const` |
| 55 | 0x0DC | `void SetExported(int32_t nCommodity, float fExports)` |
| 56 | 0x0E0 | `float GetImported(int32_t nCommodity) const` |
| 57 | 0x0E4 | `void SetImported(int32_t nCommodity, float fImports)` |
| 58 | 0x0E8 | `float GetProduced(int32_t nCommodity) const` |
| 59 | 0x0EC | `void SetProduced(int32_t nCommodity, float fProduced)` |
| 60 | 0x0F0 | `float GetDemanded(int32_t nCommodity) const` |
| 61 | 0x0F4 | `void SetDemanded(int32_t nCommodity, float fDemanded)` |
| 62 | 0x0F8 | `float GetCostPerUnit(int32_t nCommodity) const` |
| 63 | 0x0FC | `void SetCostPerUnit(int32_t nCommodity, float fCostPerUnit)` |
| 64 | 0x100 | `float GetCommodityBalance(int32_t nCommodity) const` |
| 65 | 0x104 | `uint32_t GetTutorialGUID(void) const` |
| 66 | 0x108 | `void SetTutorialGUID(uint32_t dwGUID)` |
| 67 | 0x10C | `bool IsTutorial(void) const` |
| 68 | 0x110 | `bool UpdateLocalDeals(void)` |
| 69 | 0x114 | `bool SetLocalDeals(SC4List<cISC4NeighborDeal*>& sList)` |
| 70 | 0x118 | `bool GetLocalDeals(SC4List<cISC4NeighborDeal*>& sList)` |
| 71 | 0x11C | `bool UpdateImportExport(void)` |
| 72 | 0x120 | `void GetPointsOfInterest(uint32_t dwPointOfInterestType, SC4Vector<uint32_t>& sList)` |

