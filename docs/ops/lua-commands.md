# Maxis command scripts: luacmd27 calibration status

**2026-10-25: Lua entry and a city mutation are proved on 1.1.641.0.**
`luacmd27-proof-r7` printed through Lua Extensions, reached city view, changed the
city name, read the changed name back, restored it, and saved the slot copy.
The nine older candidate scripts remain unqualified and are not suite rows.
The registered `lua-lots-empty` and `lua-lots-grown` rows exercise the installed
catalog's 175 categorized ploppable lots, then partially demolish the plan.
Their measured results and limitations are recorded below; a zero command status
alone does not prove that a lot exists in the save.

## Launching a candidate

Put this harness key in the request's ini text (absent by default):

```ini
lua_script = engine/tests/scenarios/scripts/lua-calibrate.lua
lua_extensions = 1
close_dialogs = 1
```

`tools/ingame_run.ps1` requires `-Slot`, rejects absolute/outside-repository paths
and missing/non-Lua files, and copies the script below
`<slot>/LuaScripts/<repo-relative path>`. It also copies the shared `.lua` helpers
and prepends `FC4_ROOT`, pointing to that slot's `LuaScripts` directory. The game
receives `-LuaScript:"<absolute slot copy>"`. Scripts load the library with
`dofile(FC4_ROOT .. '/engine/tests/scenarios/lua/fc4.lua')`. Other arbitrary
dependencies are not automatically staged. The original script is unchanged.
The plugin currently warns that the harness-only `lua_script` key is unknown;
that warning does not say whether the game executed the command-line script.

`lua_extensions = 1` copies the user's unchanged `SC4LuaExtensions.dll` and
`.dat` from `<main>/build-runs/thirdparty/sc4-lua-extensions/` into the private
slot's Plugins folder. It requires `-Slot`, is absent by default, and the next
slot run's normal plugin cleanup removes it when disabled. Nothing is staged
in Documents or the install's Plugins. These LGPL-3.0 binaries are not committed.
After a run, the harness harvests `SC4LuaExtensions.log` and Lua lines from other
slot logs into `<main>/build-runs/game-results/<harness-label>.lua.log`, also
copied beside the plugin log in the invoking worktree's `build-runs/`. Queue
harness labels have the `q-<root>-<request-label>` prefix. `fc4.log` prefers the
extension's `print`; a harvested print line proves execution of its script.

## Verified in round 2

Use `lua_script = engine/tests/scenarios/lua/proof.lua` for the short probe.
It is a calibration probe, not an added suite scenario. Its clean plugin verdict
had zero foreign exceptions, watchdog stalls, RTC, overruns and ref warnings.
Raw console, plugin and `.lua.log` evidence uses the labels below in `build-runs/`.
Probes r4-r8 used DLL SHA-256 prefix `13f21803` explicitly.

| Command / facility | Measurement |
|---|---|
| Lua startup / `print` | `luacmd27-entry-r4.lua.log`: `FC4_LUA r4 entered startup`. |
| `GetAppState()` | Returns **status, state**, not state alone: `0,0` at startup and `0,2` in city view (`entry-r6`). |
| `SetScriptAutoYield(-1, true)` | Status `0`; a loop polling the **second** GetAppState return let the city load. Disable with `false` before an uninterrupted mutation sequence if needed. |
| `GetCityName()` | `0, 'Konradshohe'` (`proof-r7`). |
| `SetCityName('FC4 Lua proof')` | Status `0`; next GetCityName returned `0, 'FC4 Lua proof'`; restoring the original also returned `0`. This is a verified city effect. |
| `SaveCityQuick(true)` | Status `0`, completed without confirmation (`entry-r6`, `proof-r7`). Slot save changed; Documents source stayed unchanged. |
| `GetCitySize()` | `0,128,128` for Konradshohe. Placement coordinate units/inclusivity remain uncalibrated. |
| `SetPopupModalDialogsEnabled(false)`, `SimSpeedFast()` | Status `0` only; no separate UI/speed-effect qualification claimed. |
| `GameDelay('frames',10)` | Polling while status is `1` returned and printed the after-delay marker (`date-r8`); exact elapsed frames are uncalibrated. |
| `io.open` | Present and wrote `FC4_LUA io.open file proof` to the slot's `LuaScripts/io-proof-r8.txt`. Print remains the harvested script log. |
| `GetSimulationDate` | **Blocked**: lowercase single/multiple fields, CamelCase fields, embedded quoted fields and no fields all returned `0` with no payload (`date-r8`). The date helper refuses this result; dated scenarios are not qualified. |
| `WaitForMessage2(3417684805, token)` | Returned `1`; with the documented registration/yield sequence the city loaded but the post-region wait never completed (`entry-r5`). The helper uses state polling instead. |

Command wrappers discard the first return only after checking it is status `0`,
then return the payload. `Before` without `Return` is incomplete evidence.
Startup `entry-r4` enabled auto-yield before registering the message wait and
stalled without a city for 180 s; it is distinct from the corrected probes.
`entry-r6` proved a save but reported `watchdog_stalls=1`; the later shared-helper
proof `proof-r7` was clean with **0** stalls. The source save SHA-256 was
`864e0a8817e2b450f76bd0bc` (prefix); the saved r6 slot copy was
`0ee812d930a706e4f714525a` (prefix), 1,323,907 bytes, written at 19:57:58.

Plain `luacmd27-plain-r4` and scripted r5/r6/r7 all showed **Could not initialize
Direct Draw**, accepted the software-driver fallback and eventually loaded.
Their launch switches were `-CPUCount:1 -w -intro:off -UserDir:"<slot11>\\"`,
plus `-LuaScript:"<slot script>"` only for scripts. No command-line difference
eliminated DirectDraw failure. The first measured change that enabled progress
after Lua entry was the wait algorithm; no renderer workaround is claimed.
After the final build, `luacmd27-plain-ext-r9` and `luacmd27-proof-r10` repeated
the comparison in **slot 11**, both with DLL **e27dd1c0**, Lua Extensions, the
same ini and launch switches, differing only in the `-LuaScript` argument.
Both displayed the same two DirectDraw/software-fallback dialogs and finished
clean with zero diagnostic counts. The scripted run again printed
`city-name-effect verified` and `proof-complete`; its name read-back was
`FC4 Lua proof`. Thus the final built DLL also has a proved Lua/city-command run.

The no-script house controls used the literal regress shadows profile, turbo,
`close_dialogs=1`, end date 2026-06-01 and the same DLL. Both reached the target
and quit cleanly with zero exception/stall/RTC/overrun/ref counts:

| Control | Final date | Days/s | Exact shadow lines | Findings |
|---|---|---|---|---|
| `luacmd27-house-noext-r6`, slot 12 | 2026-06-15 | 0.93 | 867/873 | occmgr 0/5; landvalue 5/6 |
| `luacmd27-house-ext-r8`, slot 11 | 2026-06-16 | 0.89 | 879/884 | occmgr 0/5; landvalue exact |

**Control accepted by coordinator review (20:20, 2026-10-25).** The different
exactness is within identical-build run variance measured by golden27 and
train248; it is not evidence against the extension. First mismatches are
`occmgr`: noext log line **3557**, ext log line **3559** (2026-02-01), and
`landvalue`: noext log line **5760** (2026-06-07, intrinsic/total mismatch at
tract 22,58). No engine fix was attempted. The existing `run_summary.ps1` reader
provided these counts; raw evidence, `summary.tsv`, `per_key.tsv` and the full
per-key JSON are in `build-runs/suite/luacmd27-r2/`. Empty proof-run shadow counts
give no exactness credit. The 180-second house attempt r5 failed before city load
and is retained separately from the completed 600-second controls.

Round-two gate: `python -m pytest tools/tests -q --tb=short` printed **6 failed,
344 passed in 89.54s**. All six failures now expose `spawnSync cmd.exe EPERM`
with null process status; no other failure remains. The launcher/evidence tests
passed **10/10**. The once-only final `qbuild.py --wt <WT> all` printed Cargo
**7844 passed**, clippy **0 diagnostics**, native **46/46 edges**, CTest
**117/117**, `qbuild: PASS`. No binaries, saves or ABI changes are committed.

Use reserved slots 11/12 for this lane. Never write to the original Regions
folder. All nine candidate rows use region `FC4 Test`, city `Konradshohe`, a
2028-02-01 end date, and set `full`; none is a smoke test. The round trip also
uses `Tegel` in the same region. Both saves exist in Documents, and only their
slot copies may change. `fc4.finish()` waits to 2028-01-01, requests a quick save,
and logs completion, leaving a month before the harness's end date.

## Historical round-one attempts

Evidence is in this worktree's ignored `build-runs/`, not in Documents:

| Run label | Measured result |
|---|---|
| `luacmd27-startup-1` | Stopped before launch: CIM access denied made the existing conservative fallback report other slots as busy. |
| `luacmd27-startup-2` | Game attached; no city within 120 s; no proof file. This early probe had the wrong decimal message ID and is superseded. |
| `luacmd27-startup-3` | Correct message ID, CreateGZLog entry marker and I/O probes; Hardware Driver Error: `Could not initialize Direct Draw`; no city within 120 s. |
| `luacmd27-control` | No Lua switch; no city within 90 s, stopped after `PreFrameWorkInit` too. |
| `luacmd27-startup-software` | Debugger-injection experiment did **not** append the renderer argument; no city within 120 s. Not a software-renderer result. |
| `luacmd27-startup-software-2` | Temporary launch-block experiment **did** append `-d:software`; still DirectDraw initialization failure; no city within 100 s. Temporary edit removed. |

The temporary `build-runs/run-lua-probe.ps1` invokes this worktree's actual
`ingame_run.ps1`. Because CIM is denied in the worker, its local function supplies
Win32_Process PID/command-line metadata using read-only
`NtQueryInformationProcess(ProcessCommandLineInformation)`. It preserves the
slot occupancy check and does not stop or change queue games. This workaround
and the renderer experiment are not committed harness features.

The shadows attempts use the **literal `shadows` ini profile from
`tools/regress.ps1`**, plus `lua_script`, turbo, end date and a 1,800-second
post-load timer. `build-runs/suite/luacmd27-r1/` holds per-scenario console and
plugin logs, the exact profile, `summary.tsv` and `per_key.tsv`. An initialization
failure is `failed-start`, not a clean simulation. `actions_ok=actions_failed=0`
means no action was reached. `worst_shadow=unmeasured`, zero compare counts and
zero per-key scenario counts give no exactness credit. No replace run is claimed.

Measured initial suite outcomes: **4 failed starts** (`lua-calibrate`,
`lua-demolish`, `lua-ui`, `lua-emergency`), **4 blocked slot-lock attempts**
(`lua-civics`, `lua-networks`, `lua-terrain`, `lua-roundtrip`), and **1 metadata
race before launch** (`lua-cheats`, a process was terminating). A corrected
metadata wrapper skips only `STATUS_PROCESS_IS_TERMINATING`; the separate
`luacmd27-lua-cheats-shadows-r2` retry launched, reported DirectDraw failure and
no city within 90 s. Its evidence is retained separately under
`build-runs/suite/luacmd27-r2/`; initial results are not overwritten.

The staged Documents DLL was **46e586f2** (SHA-256 prefix). In each of the four
initial launched suite logs it ignored **43 ini keys**, including `lua_script`
and many current shadows. These requests used the right profile text but cannot
qualify that profile. Repeat with a freshly built matching DLL after the host
graphics problem is resolved. None of these attempts entered a city or produced
`fc4-lua.log`, a startup marker, or a snapshot proving Lua entry. `ConnectionsReady`
appears only in a turbo configuration description, not as a city-load event.

After the required build gate, a **fresh DLL 13f21803** (SHA-256
`13f2180366df269c61d55c1c45319233bdd8ec59c4690a96ba17007b9e39aa64`)
was used explicitly with `-Dll` for **`luacmd27-lua-calibrate-shadows-r3`**.
It recognized every requested shadows key (only harness-only `lua_script` was
unknown), but again printed **`no city load within 90 s`**, stopped after
`PreFrameWorkInit`, and produced **0 markers and no Lua text log**. Its 165-line
plugin log, console, ini and separate summary/per-key tables are retained in
`build-runs/suite/luacmd27-r3/`. This corrected-binary failure is separate from
the older-DLL and metadata-wrapper attempts; no command or shadow success is
claimed for it.

Python validation: `python -m pytest tools/tests -q` printed **6 failed, 341 passed
in 95.17s**. The six failures are unchanged `WorkflowDryRun` tests in
`test_laneguard.py`: their mock preflight produced `ERROR` with an empty line.
The underlying cause is unresolved. All **13 new tests** passed (focused runs:
launcher **7 passed in 2.25s**, catalog **6 passed in 0.05s**).
The one required `qbuild.py --wt <WT> all` invocation passed: **7,844 Cargo
tests**, **0 clippy diagnostics**, native build **302/302 edges**, CTest
**117/117**, final **`qbuild: PASS`**. The full Python gate remains failed.

**Round-one I/O was unknown; round two proved io.open above.** The first probes tried `io.open`, then Lua 4's
`openfile/write/closefile`. Neither an entry marker, text log nor snapshot was
produced before the graphics failure. This does not prove the I/O library is
missing. `fc4.log` uses those backends when available; otherwise the documented
`CreateGZLog` command creates an empty file whose bounded filename carries the
log message. A `Before` without `Return` is incomplete evidence, not automatically
a crash. A return value alone does not prove an action changed the city.

## Argument contracts to calibrate

The following are **public interface facts and candidate arguments**. Only the
subset in the round-two measurement table above has now been reached; all other
commands remain unqualified. No command-attributed crash has been demonstrated.

| Commands used | Candidate arguments / unresolved measurement |
|---|---|
| `WaitForMessage2`, `CancelWaitForMessage2` | Message **3417684805** (0xCBB5BB45), unique string token; poll while return is 1. |
| `SetScriptAutoYield` | `-1, true`; current script, allow the game to advance. |
| `CreateGZLog` | Absolute slot path, empty category string, `false`; empty-file logging fallback. |
| `GetAppState` | Expected 0 unspecified, 1 region, 2 city. Wrappers refuse city/region calls in another state. |
| `GameDelay` | `'frames', n` or `'time', n`; public documentation says **milliseconds**, not seconds. |
| `SetPopupModalDialogsEnabled` | Boolean; manually opened dialog closing is still unresolved. |
| `GetCityName`, `GetCitySize` | No arguments; size documented as two cell counts. Need actual values and matching digest. |
| `GetSimulationDate` | `'year', 'month', 'day of month'`; must return three numbers before the bounded date loop can advance. |
| `GetDateAndTime`, `GetRandomInteger` | Date/time string fields; integer min/max candidate 1,10. Capture actual arity and returns. |
| `PlaceZone`, `PlaceNetwork` | Type followed by four rectangle/endpoint numbers. Units/inclusivity must be checked against size, digest and saved data. All 13 network types are candidates. |
| `PlaceLot` | Numeric exemplar IID, cell x,z, facing 0..3, `false,false`; compare numeric vs `'0x03000001'` string form. Facing documented N/E/S/W. Success/return convention unknown. |
| `PlaceBuilding` | Numeric or hex-string building IID, x,z; **position units unknown**, do not assume cells or metres. |
| `Demolish` | Four coordinates followed by `'Sim'`, `'None'` or `'Explode'`. Sim is a privilege flag; None/Explode are effects, not three equivalent enums. Multiple rectangles are not yet exercised. |
| `GetCellAltitude`, `SetCellAltitude` | Cell x,z; setter single-cell or x1,z1,x2,z2,height. Read-before/write/read-after/restore candidates. Height units uncalibrated. |
| `GetSeaLevel`, `SetSeaLevel` | No arguments / float; candidate +1 then restore. Units/effects uncalibrated. |
| `GetViewTarget`, `SetViewTarget` | Documented x,y,z; set the returned values back. Units uncalibrated. |
| `TakeSnapshot` | Absolute slot `.png` path, optional rectangle not yet exercised; file creation/dimensions unresolved. |
| `SaveCity`, `SaveCityQuick` | `true` suppresses confirmation per reference; check slot save timestamp/content. |
| `QuitCity` | `false` avoids confirmation per reference; then wait for state 1. |
| `LoadCity`, `LoadRegion` | Name, `'any','full'` / region name. Loading and Lua continuation across transitions unverified. |
| `ExecuteCheat` | One player-documented string; acceptance and city effects unverified. |
| `GZWinIsWindowPresent`, `GZWinClickButton` | Window/parent or button/parent numeric IDs. 0,0 and 1,0 are **negative ID probes**, not known working UI controls. |
| `GZWinEnableUserInput` | `true`; actual effect unverified. |
| `OpenAudioOptionsDialog`, `OpenCheatCodeDialog`, `OpenGraphicOptionsDialog`, `OpenPlayOptionsDialog`, `OpenPhotoAlbumDialog`, `OpenSnapshotDialog`, `OpenObliterateCityDialog` | No-argument candidates, snapshot, then `Cancel`. **Closing must be calibrated** before claiming every dialog was visited; never confirm obliteration. |
| `ToggleGodPanel`, `ToggleMayorPanel`, `ToggleMySimPanel`, `ToggleOptionsPanel`, `ToggleGridVisibility`, `ToggleUIVisibility` | Call twice; restoration is a candidate, not a measured result. |
| `QueryTool`, `TrafficQueryTool`, `Cancel` | Select/cancel; actual map queries need a map input event. |
| `Zoom0`..`Zoom5`, `ScrollRightOnce`, `ScrollLeftOnce`, `ScrollUpOnce`, `ScrollDownOnce` | No-argument view candidates. |
| `EnableDayOnly`, `EnableNightOnly`, `EnableDayAndNight`, `SimSpeedFast` | No-argument candidates. |
| `RoadTool`, `StreetTool`, `AvenueTool`, `HighwayTool`, `RailTool`, `LightRailTool`, `SubwayTool`, `PowerLinesTool`, `PipesTool` | Select then `Cancel`; tool selection alone does not place a network. |
| `PlaceBusStopTool`, `PlaceSubwayStationTool`, `PlaceLightRailStationTool`, `PlaceFerryTerminalTool`, `PlacePassengerDepotTool`, `PlaceFreightDepotTool` | Select then `Cancel`; actual placements are separate `PlaceLot` candidates. |
| `ToxicSpill`, `DispatchFireTool`, `DispatchPoliceTool` | No-argument candidates; dispatch tools are selected/cancelled only. **No actual dispatch or map click is claimed.** |

`build-runs/suite/luacmd27-r1/commands.tsv` expands the individual candidate
command names per scenario. All are `not-reached` when initialization fails.
Uncalled reference commands (including `QuitGame`, custom-window creation/text,
the remaining civic tools, pause/slow/medium and flora) are **untested**, not
failed or verified. There is no evidence yet for working UI button IDs.

## Lot data inventory

```text
python tools/lotconfigs.py --output engine/tests/scenarios/lotconfigs.tsv
```

Measured: **1,909 lots, 22 packages, 0 warnings**. Sources are read-only DBPF
game data in the installation and Plugins, including `.dat`, `.SC4Lot` and
`.SC4Desc`. No save files or executable/disassembly inputs are used. The catalog
records full lot IID/group, internal exemplar name, dimensions, building IDs,
OccupantGroups and relative source. Binary/QFS and text exemplars, inherited
cohorts, building families and plugin overrides are supported. Plugin ordering
is deterministic; arbitrary plugin load-order conflicts remain a calibration
concern, and the IID-only command cannot select between group collisions.

Categories come from building OccupantGroups (power 0x1400, water 0x1401,
garbage 0x1402; civic 0x1500..0x150B; transit 0x1003, parks 0x1006).
Multi-purpose lots retain multiple categories. **1,734 `other` rows** are
retained rather than guessed from names; the **175 categorized rows** include
all requested civic classes. `utilities` is the umbrella for power/water/garbage;
this installation has no additional generic-utility-only lot. Catalog presence
does not prove that a lot can be plopped (rewards, water/shoreline and funding
constraints still apply). Regenerate the TSV after changing game data/plugins.

## Public sources and cheat strings

Interface facts only, retrieved 2026-10-25:

- [0xC0000054 command documentation](https://github.com/0xC0000054/sc4-game-command-documentation)
  ([App](https://github.com/0xC0000054/sc4-game-command-documentation/blob/main/commands/Maxis/AppCommands.md),
  [City](https://github.com/0xC0000054/sc4-game-command-documentation/blob/main/commands/Maxis/CityCommands.md),
  [Region](https://github.com/0xC0000054/sc4-game-command-documentation/blob/main/commands/Maxis/RegionCommands.md)).
- [Public Lua startup example](https://github.com/0xC0000054/sc4-lua-test-scripts/blob/main/test-scripts/AutoLoadCityTestScript.lua):
  `sc4app.<Name>(...)`, startup ordering, message/token polling and auto-yield.
  These repositories are GPL-3.0; their interfaces informed independently
  written candidates. No disassembly-derived implementation was used.
- [IGN SimCity 4 PC Cheats](https://www.ign.com/wikis/simcity-4/PC_Cheats):
  `fightthepower`, `hellomynameis <name>`, `howdryiam`, `sizeof <1..100>`,
  `stopwatch`, `tastyzots`, `weaknesspays`, `whatimeizit <time>`,
  `whererufrom <name>`, `you don't deserve it`, `zoneria`, `recorder`, `gol`,
  `dollyllama`. The candidate omits stopwatch/recorder to avoid leaving a clock
  or recording running; it supplies explicit names/time/zoom for parameterized
  cheats. IGN's spelling is `whatimeizit` (one t). Other cheat pages returned
  HTTP 403 and supplied no additional facts.

No TCP listener was enabled. No Documents data, game saves or ABI exports were
changed. Next: resolve the host graphics initialization failure; prove entry
and logging; calibrate each call, dialog/button ID and map input; then replace
these `not-reached` statuses with measured effects and per-scenario shadows.
