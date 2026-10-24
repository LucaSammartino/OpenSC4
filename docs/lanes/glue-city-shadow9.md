| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-city-shadow9: bounded city geometry shadow | Codex / glue-city-shadow9 | ready (measured slice; HG8 open) | Land measured geometry; resolve HG8 before whole-owner credit. |

## Result and scope

Default-off `city_shadow=1` measures a read-only geometry slice of the native city at load and each month.
Final measurement: **city 73/73**, exact through **2032-01-01** under the train187 house profile.
This is READY measured work under the task's partial-contract rule, **not whole cSC4City verification**.

The brief's implementation claim does not hold in this checkout: `core/src/save.rs` implements the simulator's
version-4 record. Foundations §3.6 specifies the city's 53 component references but explicitly defers the city's
other fields. Save-records §2 leaves its auxiliary record unspecified. HG8 in
`docs/specs/questions/foundations.md` requests the city scalar payload, update/getter contracts and a safe
serializable capture route. No invented city record parser or native-seeded persistence oracle was added.

## Implementation

- Source commit `237c3319a`: portable `city_geometry` core implements the published floor conversion from
  budget round-2 errata item 6, with the 16 m cell width from terrain §2.1. A private static-link C transport compares
  native samples against that production Rust core; no public C ABI export changes.
- Plugin samples `PositionToCell` at eight fixed caller-owned coordinate pairs and compares X/Z separately;
  both cell widths are compared separately. Every line covers **18 fields**, including negative edges, exact
  cell edges and coordinates outside the city. Expected values use only fixed probe inputs and spec constants.
  The native date is reporting metadata, not expected state.
- One `city shadow YYYY-MM-DD: ... mismatches=N` line at load and one per month; lowercase label `city`.
  Native getters remain borrowed, with no retained interfaces, AddRef/Release operations, native writes,
  mutators, replacement factory, hardware breakpoints or shadow resynchronization.
- No native `Read`/`Write` call: city `Write` also writes other records, and no safe read-only route is
  published. Unsupported state remains unimplemented/default-off under HG8.

## Verification

- Final audit found the floor rule citation was mislabeled §17; corrected to budget round-2 errata item 6.
  Implementation statements and the measured behavior are unchanged. Correction commit `e1a929dca` changes
  only the core's citation comment and documentation. No additional game run is required for this correction.
  Final stable-source `qbuild.py all` **PASS**: **6986 tests passed, 0 failed, 0 ignored**, clippy **0 diagnostics**;
  native queue `qbuild-20261024-091941-build-ctest` terminal PASS, build **205/205** (82.1 s),
  CTest **79/79** (172.1 s), native verification 258.0 s total. This final gate has no tree-change annotation.
- Owned-file formatting passes. Focused city-filter tests: **157 passed**, including both new tests for
  signed boundary flooring and counting every differing axis/width independently.
- Initial `qbuild.py all`: **PASS**, workspace **6986 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**.
  The same operation delegated native verification to queue `qbuild-20261024-083721-build-ctest` and was
  collected to terminal PASS: build **254/254**, full CTest **79/79**. No duplicate native verify was started.
  Its tree-change annotation refers only to a lane-documentation commit; implementation source stayed fixed.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
  Initial relative-path fmt failed because that wrapper needs absolute paths; corrected absolute-path check
  passed. `q.py` was unavailable due to Bash CreateFileMapping Win32 error 5; focused cargo used PowerShell.
  `git diff --check` passes. Detached commits succeed despite Git's packed-refs maintenance permission warning;
  no shared refs/config were changed.

## Real-city measurements

Both requests use Konradshohe, `sim_turbo=1`, and every switch from main
`build-runs/regress-train187-shadows.ini` plus `city_shadow=1`. The alias replaces the house autoload path.
Requests are under `build-runs/game-queue`; evidence is under `build-runs/game-results`.
Native plugin SHA256 prefix: `f346ce2d`. No request is outstanding.

- `glue-city9-237c3319a-2032a`: **city 42/42**, clean at 2029-06-05; 1 load + 41 months,
  **756/756 fields** exact. The 600-second limit ended this run before target, preserved as
  [run 469](../runs/glue-city9-237c3319a-2032a-479.md).
- `glue-city9-237c3319a-2032b`: **city 73/73**, clean at **2032-01-01**; 1 load + 72 months,
  **1314/1314 fields** exact, **0 duplicate monthly dates**, real_seconds=985.359. Extended duration cap
  1800 seconds, timeout 2100; ended at the requested date. See
  [run 470](../runs/glue-city9-237c3319a-2032b-480.md).
- Both verdicts: **foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0**.
  Both waits were collected to actual verdicts; nine-minute wait timeouts did not cause request resubmission.
  No differences were observed, so no turbo-only difference requires a normal-speed diagnostic run.

## Explicitly uncompared owner methods

Only `PositionToCell`, `CellWidthX`, and `CellWidthZ` are compared. The following SDK methods remain uncompared:

- Lifecycle/metadata: `Init`, `Shutdown`, `GetCitySerialNumber`, `SetCitySerialNumber`,
  `GetNewOccupantSerialNumber`, `GetOriginalLanguageAndCountry`, `GetLastLanguageAndCountry`,
  `GetCitySaveFilePath`, `SetCitySaveFilePath`, `GetCityName`, `SetCityName`, `GetCityNameChanged`,
  `SetCityNameChanged`, `GetMayorName`, `SetMayorName`, `GetCityDescription`, `SetCityDescription`,
  `GetBirthDate`, `SetBirthDate`, `GetEstablished`, `SetEstablished`, `GetDifficultyLevel`,
  `SetDifficultyLevel`, `GetWorldPosition`, `SetWorldPosition`, `GetWorldBaseElevation`,
  `SetWorldBaseElevation`, `GetWorldHemisphere`.
- Component access: `GetDemolitionUtility`, `GetHistoryWarehouse`, `GetLotManager`, `GetOccupantManager`,
  `GetPropManager`, `GetZoneManager`, `GetLotConfigurationManager`, `GetNetworkManager`, `GetDispatchManager`,
  `GetTrafficNetwork`, `GetPropDeveloper`, `GetNetworkLotManager`, `GetVehicleManager`, `GetPedestrianManager`,
  `GetAircraftManager`, `GetWatercraftManager`, `GetAutomataControllerManager`, `GetAutomataScriptSystem`,
  `GetCitySituationManager`, `GetSimulator`, `GetAuraSimulator`, `GetBudgetSimulator`,
  `GetBuildingDevelopmentSimulator`, `GetCommercialSimulator`, `GetCrimeSimulator`, `GetDemandSimulator`,
  `GetFireProtectionSimulator`, `GetFlammabilitySimulator`, `GetFloraSimulator`, `GetIndustrialSimulator`,
  `GetLandValueSimulator`, `GetNeighborsSimulator`, `GetOrdinanceSimulator`, `GetPlumbingSimulator`,
  `GetPoliceSimulator`, `GetPollutionSimulator`, `GetPowerSimulator`, `GetResidentialSimulator`,
  `GetTrafficSimulator`, `GetWeatherSimulator`, `GetMySimAgentSimulator`, `GetDisasterLayer`,
  `GetCivicBuildingSimulator`, `GetParkManager`, `GetZoneDeveloper`, `GetSeaportDeveloper`,
  `GetAirportDeveloper`, `GetLandfillDeveloper`, `GetLotDeveloper`, `GetTractDeveloper`, `GetAdvisorSystem`,
  `GetTutorialSystem`, `GetSurfaceWater`, `GetTerrain`, `GetEffectsManager`, `Get24HourClock`.
- Other geometry: `GetCitySizeType`, `SetSize`, `SizeX`, `SizeZ`, `CellCountX`, `CellCountZ`,
  `CellCornerToPosition`, `CellCenterToPosition`, `LocationIsInBounds`, `CellIsInBounds`, `CellCornerIsInBounds`.
- Controls: `ToggleSimulationMode`, `IsInCityTimeSimulationMode`, `EnableSave`, `DisableSave`,
  `IsSaveDisabled`, `UIIncreaseLockCount`, `UIDecreaseLockCount`, `UIGetLockCount`, `SaveObliterated`.

Inherited `QueryInterface`, `AddRef`, `Release`, serializable `Read`, `Write`, `GetGZCLSID`, all persisted
scalar fields, all 53 serialized component references, auxiliary city records, and any additional unpublished
owner methods remain uncompared. `GetSimulator` is used only to date the load line, with no equivalence claim.

## Questions and next steps

- **HG8**, `docs/specs/questions/foundations.md`: publish missing city persistence/owner contracts and safe
  capture route. Extend the independent implementation and shadow after publication; do not credit the
  entire city row from this geometry measurement.
- Coverage tables and `tools/regress.ps1` were not read or edited, and main was not merged: those operations
  are excluded by this lane's clean-room/worktree rules. Coordinator may append `city_shadow` to the current
  single shadows-profile string after review, keeping the bounded scope visible until HG8 is resolved.
