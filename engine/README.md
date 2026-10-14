<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# FreeCity4 engine

Clean-room engine code for FreeCity4. It is written only from [`docs/specs/`](../docs/specs/) and the LGPL
gzcom-dll SDK. Read [CONTRIBUTING.md](../CONTRIBUTING.md) before you change anything here.

| Path | What | Language |
|---|---|---|
| [`core/`](core/) | `fc4-core`: the portable simulation core. No GZCOM or OS types, no `unsafe`. | Rust |
| [`capi/`](capi/) | `fc4-capi`: a stable C ABI over the core (`include/fc4.h`), built as a static library | Rust + C header |
| [`conformance/`](conformance/) | Language-neutral JSON test vectors (the migration contract) and their Rust runner | JSON + Rust |
| [`formats/`](formats/) | `fc4-formats`: readers for the game's data files (DBPF packages, QFS/RefPack, exemplars and cohorts, LTEXT), the `fc4-dat` inspection tool, and a sweep test over the installed game (`FC4_GAME_DIR`). Groundwork for the standalone engine (M6); not used by the plugin. | Rust |
| [`plugin/`](plugin/) | `FreeCity4.dll`: GZCOM director with a subsystem table, the `cISC4Simulator` adapter, the budget classes (`cISC4BudgetSimulator`, `cISC4DepartmentBudget`, `cISC4LineItem`), the land value, weather, ordinance, demand and power simulators, the standard ordinances and the demand nodes (off by default), stream and property helpers, and our own standard message class (foundations §5.1; the SDK's `cRZMessage2Standard` is not used). Windows, x86 only. | C++17 |
| [`tests/`](tests/) | `capi_smoke` (drives the C ABI from C++ with the plugin's compiler and CRT), `message_test` (foundations §7 item 12) and `plugin_load_test` (loads the DLL like a host and drives the simulator and the budget) | C++17 |
| `third_party/gzcom-dll/` | gzcom-dll SDK (LGPL-2.1+), git submodule | C++ |

```
SimCity 4.exe ── GZCOM ──► FreeCity4.dll (plugin/, C++)
                              cFC4Director  ─ registers the classes of FC4Subsystems.cpp
                              cFC4Simulator ─ cISC4Simulator, cIGZSerializable,
                                              cIGZSystemService, cIGZMessageTarget2
                              cFC4BudgetSimulator, cFC4DepartmentBudget, cFC4LineItem
                                     │  fc4.h (C ABI)
                                     ▼
                           fc4_capi.lib (capi/, Rust staticlib)
                                     │
                                     ▼
                           fc4-core (core/, Rust): simulator, budget, streams, class-object
                           store, variants and property holders, curves, RNGs
```

The C ABI is the seam between the two languages. A future all-Rust GZCOM layer can replace `plugin/` and keep
`fc4.h` unchanged.

## Prerequisites

- Rust (stable, 1.75 or newer) with the `i686-pc-windows-msvc` target:
  `rustup target add i686-pc-windows-msvc`.
- For the DLL: Visual Studio 2022 Build Tools (MSVC x86, Windows SDK). CMake 3.20+ and Ninja ship with them.
- The SDK submodule: `git submodule update --init`.

## Build and test

Rust only (any OS):

```sh
cd engine
cargo test                                   # host target
cargo test --target i686-pc-windows-msvc     # 32-bit Windows, the plugin's target
cargo clippy --workspace --all-targets -- -D warnings   # must be clean, tests included
```

Run cargo from `engine/`, so `engine/.cargo/config.toml` applies. It links the C runtime statically
(`+crt-static`) for the Windows targets.

The plugin, from a shell with the x86 MSVC environment
(`"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars32.bat"`):

```bat
cmake -S engine -B build-x86 -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build-x86
ctest --test-dir build-x86 --output-on-failure
```

CMake runs `cargo build --release -p fc4-capi --target i686-pc-windows-msvc`, with its output in
`build-x86/cargo/`, then links `fc4_capi.lib` into `build-x86/FreeCity4.dll`. Everything uses the static release
CRT (`/MT`), so the DLL needs no runtime DLLs. CTest runs `cargo test` for the same target, plus the C++
`capi_smoke` test. On other platforms or architectures CMake builds and tests only the Rust parts and skips the
plugin.

## Installing and checking in game

1. Copy `build-x86/FreeCity4.dll` to the game's plugin folder, for example
   `Documents\SimCity 4\Plugins\` or `<game dir>\Plugins\`.
2. Start the game. `FreeCity4.log` appears next to the DLL, or in `%TEMP%` if that folder is not writable.
   - On start you should see `FreeCity4 … loaded (… replacement mode)` and `PostAppInit (simulator class 0x2990C1E5
     provided by FreeCity4)`.
   - When a city opens, you should see `cFC4Simulator created`, `Read: …`, `Init: …` and then `PostCityInit: hidden pause released`. While the city runs you
     should see one `SimNewMonth …` line per game month.
   - Diagnostics: `FC4_LOG_LEVEL` = `0` (off), `1` (lifecycle only) or `2` (default, verbose). Verbose mode traces
     `DllMain`, every director call (`InitializeCOM`, `OnStart`, `EnumClassObjects` with each class ID reported,
     every `GetClassObject` with the clsid, the iid, the result and whether our factory ran), plus simulator
     `QueryInterface` (the first 200 calls), `Init`, `Read`/`Write`, `OnTick` (the first tick and every 1000th),
     `DoMessage` (the first 50), agent registration and pause calls. Lines marked `[v]` are verbose. Every line is
     written straight to the OS. The log is opened with full sharing, so a viewer that holds it open cannot block
     writes. The path never changes once chosen. Failed writes are retried once and then reported on the next line.
   - Crash tracing: the class-object path (`GetClassObject`, the factory, `QueryInterface`) runs under a Win32
     structured-exception guard. A first-chance vectored handler also logs every exception raised inside
     `FreeCity4.dll`'s image, with code, address and offset from the DLL base, and passes it on unhandled.
     Exceptions outside our image are reported at most twice per faulting address (16 addresses), as
     `module+offset`, with the return addresses on the stack and the last interface calls. The handler never
     blocks: it takes the log lock only with a time-limited try-lock and otherwise writes through a spare
     handle. Stack overflows are only counted. `FC4_DIAG_SELFTEST=1` raises one deliberate access violation at
     load to prove that both work.
   - Heartbeat (always on, also at `FC4_LOG_LEVEL=1`): one line per speed or pause change and per month start,
     and a `heartbeat (periodic)` line every `FC4_HEARTBEAT_MS` (default 10000) with the julian date, speed,
     the pause/hidden/emergency flags, and since the last heartbeat the `OnTick` count, agent deliveries,
     interface calls and verbose lines suppressed by caps.
   - Hang watchdog: a background thread watches markers the simulation thread publishes without locks (inside
     `OnTick`, the agent delivery in progress with level, message and agent name, core `fc4_*` calls, our
     interface methods). If the simulation thread stays inside our code for more than `FC4_WATCHDOG_STALL_MS`
     (default 5000), or no `OnTick` arrives for `FC4_WATCHDOG_NOTICK_MS` (default 15000) while a city runs and
     is not hidden-paused, it logs `WATCHDOG: … stalled` with that activity, the last 64 interface calls, and a
     sample of the thread: suspended only to copy EIP/ESP and its stack, then resumed before anything is
     formatted. The sample lists the return addresses into `SimCity 4.exe` and `FreeCity4.dll` as
     `module+offset`, plus process and thread CPU time (a thread that waits gains no CPU, a spinning one does).
     It repeats every `FC4_WATCHDOG_SAMPLE_MS` (default 5000), at most 10 samples per stall, and ends with
     `WATCHDOG: stall over after …`. `FC4_NO_WATCHDOG=1` disables the thread.
   - Scheduler tracing: the first 300 agent deliveries are logged with level, message, agent and result. Per
     agent, the log counts deliveries, "not finished" results and the current streak of consecutive ones, and
     notes when a streak passes 100, 1,000, 10,000, and so on. Every `FC4_STATUS_EVERY` ticks (default 500) it
     writes a status block: the day-barrier conditions (`DAY_GATE_OPEN`, interval timer, blocking levels,
     counters), and for each level its cursor (agent name or END), its size and its top five false streaks. The
     first 200 broadcast posts are logged, and every SimNewDay, SimNewWeek, SimNewMonth and SimNewYear post. Date
     reads are logged too: the first 20 calls to each `GetSimDate` variant and `GetSimDateNumber`, and the first
     200 method calls on our `cIGZDate` objects.
   - At `PostAppInit`, verbose mode also asks `GZCOM()->GetClassObject(0x2990C1E5)` once. The answer shows whether
     the game's registry resolves the class to our factory. The object it creates is released at once. Set
     `FC4_NO_PROBE=1` to skip this check.
   - Limit-timer calibration (spec §5.5): the simulator keeps the game's two CPU-cycle coefficients
     (`CyclesToMillisecs`, `MillisecsToCycles`) up to date. Subsystems such as the lot developer use them to budget
     their work. Writing them needs the version-specific addresses in `plugin/compat/`, and only happens after the
     host has been identified as the exact 1.1.641 executable. The log shows each check (`compat: …`) and the
     published values (`calibration …`), both at `Init` and in every status block. `FC4_DISABLE_CALIBRATION=1`
     turns off the writes (for A/B tests).
   - Budget: `budget Init: …` when a city opens (funds, departments, bonds, tax rates), `budget Read/Write`,
     cheats and bonds as they happen, and three `budget settlement: …` lines each month: funds before and after
     with income and expense, the tax of each of the 12 groups with its rate, and the bonds (count, borrowed,
     monthly payments). `FC4_BUDGET_FIX_CS3=1` enables the intended CS$$$ tax weight (budget spec §13.1; off by
     default, as in the original).
3. **A/B against the original:** `FC4_DISABLE` takes a comma- or space-separated list of subsystems to leave to
   the game (`simulator`, `budget`, or `all`), e.g. `FC4_DISABLE=budget`. `FC4_DISABLE_SIMULATOR=1` still works.
   The DLL still loads, but it does not register those classes, so the game's own ones run. The simulator and the
   budget only talk through the SDK interfaces, so either can be switched off alone.
4. **Bisection (`FreeCity4.ini`):** a `key = value` file next to `FreeCity4.dll` (`#` or `;` after whitespace
   starts a comment, `[sections]` are ignored). A key in the ini wins over the environment variable.
   - `disable = budget` means the same as `FC4_DISABLE`. It is read once, when the game enumerates the classes,
     so a change needs a game restart.
   - Some subsystems are **off by default** and only registered with `enable = …` (or `FC4_ENABLE`); `disable`
     still wins (`enable = landvalue, weather` takes a list). Without it the log says
     `subsystem '…': off by default` and the game's class runs, so budget bisection runs are unaffected:
     - `landvalue`: the land value simulator (`cSC4LandValueSimulator`, landvalue spec; its grids are the game's
       own `cSC4SimGrid` objects, created through GZCOM, simgrid spec §6.3);
     - `weather`: the weather simulator (`cSC4WeatherSimulator`, weather spec; the map view reads its moisture
       grid through an `SC4Vector`-compatible mirror, §11.2);
     - `ordinance`: the ordinance simulator and the 21 standard ordinance classes (`cSC4OrdinanceSimulator`,
       ordinance spec; replaced together, §10.2; plugin ordinances are accepted through `cISC4Ordinance`);
     - `demand`: the demand simulator and the demand node class (`cSC4DemandSimulator`, `cSC4Demand`, demand
       spec; the graph comes from the game's demand exemplars). The building profile's capacity maps (§7.2) are
       the game's STLport maps (docs/abi/allocators.md, layouts.md): `plugin/FC4GameMap.h` walks them read-only
       (count, depth, link, key-order and step checks; a map that fails them, or faults, reports nothing) on any
       executable. `GetLocalPopulationSummary` (§5) empties and fills the caller's STLport map with the game's own
       subtree free and `operator[]` (docs/abi/direct-calls.md), only on the identified executable and under the
       structured-exception guard; elsewhere the map is left untouched (logged once). Switches (default `1`,
       environment `FC4_DEMAND_<KEY>`): `demand_capacity`, `demand_direct_calls`. Log lines:

       ```
       demand: capacity map read: building type 0x…, N map(s), M entries: 0x1010=12, …        (once)
       demand: GetLocalPopulationSummary: N entries copied into the game's map (K old entries cleared, count now N;
         main|another thread)                                                                 (once)
       demand month: … export cities; capacity: reads=… entries=… units=… (occupancy: reads=… units=…) refused=…;
         summary: calls=… copied=… untouched=… failed=…                                       (each month)
       ```

       `reads` counts building profiles read (census and §7.2), `occupancy` the §7.2 reads alone (StateChanged),
       `refused` the profiles whose maps failed the checks; the month counters restart at each month line.
     - `power`: the power simulator (`cSC4PowerSimulator`, power spec; its powered grid is the game's own
       `cSC4SimGrid<uint8>`, created through GZCOM). Not yet done (logged once): power lines are not detached on
       terrain changes (the line's `GetConnectedPoles` is not in docs/abi), and the Nuclear Free Zone does not close
       the 3D view's placement tool. The pole's `RemoveConnection` is a direct call, made only on the identified
       executable. Its game calls have their own switches (default `1`, environment `FC4_POWER_<KEY>`):
       `power_occupants`, `power_poles`, `power_lots`, `power_neighbors`, `power_budget`, `power_grid_writes`,
       `power_posts`, `power_terrain`, `power_zone_tool`, `power_view`, `power_radiation`,
       `power_occupant_query`, `power_demolish`, `power_direct_calls` (see `plugin/FC4Config.h`).
     - `pollution`: the pollution and garbage simulator (`cSC4PollutionSimulator`, pollution spec; its eight grids
       are the game's own `cSC4SimGrid<int16>`, created through GZCOM). The getters are answered from any thread
       (the game asks from a second thread during a day or a month). Not yet done (logged once): the visual effects
       of §4.10 (`cISC4VisualEffect` / `cS3DTransform` are not in docs/abi). The log has a status line at `Init`,
       `Read` and every month (`pollution month: init=… cells=… lists wtp/inc/wte/rec=… avg air/water/garbage=…
       supply=… capacity=… last month produced/…/landfilled=… cost=…; grids airSrc nonzero/tracts sum [min..max],
       …`). Its game calls have their own switches (default `1`, environment `FC4_POLLUTION_<KEY>`):
       `pollution_occupants`, `pollution_lots`, `pollution_neighbors`, `pollution_budget`,
       `pollution_grid_writes`, `pollution_posts`, `pollution_history`, `pollution_ordinance`,
       `pollution_traffic`, `pollution_zoning`, `pollution_power`, `pollution_landfill`, `pollution_population`,
       `pollution_props`, `pollution_terrain` (see `plugin/FC4Config.h`).
     - `protection`: the fire and police simulators together (`cSC4FireProtectionSimulator`,
       `cSC4PoliceSimulator`, protection spec; replaced together, §13.2). Their coverage grids are the game's own
       `cSC4SimGrid<uint8>` / `<int16>`, created through GZCOM. The fire plane and police helicopter dispatch
       sources are the game's classes, created through GZCOM; their `Init` / `Shutdown` are direct calls, made only
       on the identified executable (otherwise no air source, logged). The getters the game calls from its second
       thread are served under a per-object lock; a thread that cannot get it within 20 ms gets the last month-end
       values (or a neutral one) and is counted as `fallbacks=`. The game calls have their own switches (default
       `1`, environment `FC4_PROTECTION_<KEY>`): `protection_occupants`, `protection_budget`,
       `protection_dispatch`, `protection_automata`, `protection_grid_writes`, `protection_posts`,
       `protection_history`, `protection_air_sources` (see `plugin/FC4Config.h`). Log lines:
       `cFC4FireProtectionSimulator(…) created`, `protection Init (fire): init=1 … stations=… department=…
       grid=… air=…`, once per game month `protection month (fire|police): … stations=… coveredCells=…
       maxCoverage=… avg=… pct=… dispatches=… units=… strike=… reentrant=… fallbacks=… posts=…` (police adds
       `protection month (police jails): jails=… capacity=… population=…`), `protection Read/Write (…) -> 1`,
       `protection Shutdown (…)`, `cFC4…Simulator destroyed (N fallbacks)`.
     - `crime`: the crime simulator (`cSC4CrimeSimulator`, crime spec; its crime and crime-effect grids are the
       game's own `cSC4SimGrid<uint8>`, created through GZCOM; police coverage comes through whatever police
       simulator the city has). The getters are answered from any thread (they read a snapshot); the prop
       requester's calls run on the main thread only (a call from another thread is answered 0 and logged once).
       The random generator is seeded with `derive_seed(S, 0x6C09C3F6)`, `S` the session seed: the local wall
       clock in seconds at the first crime simulator of the process, or `crime_seed = N` (env `FC4_CRIME_SEED`,
       decimal or `0x…`) for a repeatable run. The log has a status line at `Init` and `Read` (`crime Init:
       init=… state=… types=… S=… decay=…`) and one every month from the first (`crime month yyyy-mm:
       crimes=N prevented=P arson=A/T incidents=I grid sum/max=… (tracts=…) average=… hotspot=(x,z)=v props
       requested=R selected=K/Q jailed=J released=L criminal base=B to commit=C attempts=… (no lot …) police
       grid=yes|no ordinance=… posts=…/… state=… types=… days=… months=…`). Its game calls have their own
       switches (default `1`, environment `FC4_CRIME_<KEY>`): `crime_lots`, `crime_police`, `crime_residential`,
       `crime_demand`, `crime_ordinance`, `crime_occupants`, `crime_fire`, `crime_props`, `crime_posts`,
       `crime_grid_writes`, `crime_zoning` (see `plugin/FC4Config.h`).
   - Each budget behaviour that touches game objects has a switch. The switches are re-read at every city load
     (budget `Read`/`Init`). Each defaults to `1`; `0` makes the behaviour a no-op with a neutral result. The
     matching environment variable is `FC4_BUDGET_<KEY>`. `budget_default = 0` turns every unset switch off.

     | Key | When off |
     |---|---|
     | `lot_scan` | `ForEachLot` sees no lots, so property tax is 0 |
     | `property_writes` | `AddPropertyVariant` and property copies are skipped but reported as written |
     | `property_reads` | properties read as absent |
     | `message_posting` | no budget messages are posted |
     | `history` | no history records or queue lengths |
     | `building_name` | empty line-item names |
     | `exemplar_loads` | the tuning exemplar is not loaded (code defaults) |
     | `disaster` | RiskyMoney never starts the earthquake |
     | `cheats` | the budget's cheats are not registered |
     | `holder_refs` | the budget core holds no references on game objects (references taken while the switch was on are still given back) |
     | `ltext` | LTEXT lookups return "not found" |

     The log shows `config (…): budget switches OFF (n of 11): …` at each load. At budget `Shutdown` it shows how
     many calls each switch skipped.
   - `refaudit = 0` turns the reference audit off. `refaudit_lines = N` sets its verbose line cap (default 2000).
   - **Autotest (unattended bisection):** `autotest_seconds = N` (env `FC4_AUTOTEST_SECONDS`, read at game start).
     N seconds after the first framework tick that follows a city load (`PostCityInit`) the DLL writes one line,
     flushes the log and ends the game with `TerminateProcess(…, 0)`:

     ```
     AUTOTEST VERDICT: clean|corrupt|stalled (foreign_exceptions=…, watchdog_stalls=…, rtc=…, overruns=…,
       ref_warnings=…, julian=…, switches_off=…, disabled=…)
     ```

     - `clean`: at the deadline, with no foreign exception, RTC failure or out-parameter overrun.
     - `corrupt`: a first-chance exception outside `FreeCity4.dll` (written at once, after its report; the
       process ends 5 s later so a stall can still be logged), or RTC failures / overruns at the deadline.
     - `stalled`: the watchdog saw the main thread stuck for more than 10 s (inside our code, or no tick while
       a city runs). The watchdog thread writes it and terminates. Under autotest the no-tick threshold is
       lowered to `FC4_WATCHDOG_STALL_MS`.
     - `switches_off` lists the budget switches that are off, `disabled` the subsystems left to the game.
     - `autotest_unpause = 1` or `2` (env `FC4_AUTOTEST_UNPAUSE`): after the load, resume the city's simulator
       (`Resume` while `IsPaused`) and set that speed, through the simulator's own interface (ours or the
       original), at the first tick and again after 3 s and 10 s.
     - `autotest_end = quit` (env `FC4_AUTOTEST_END`; default `terminate`): at the deadline, quit through the game
       instead of `TerminateProcess`. The main thread's next tick calls `cISC4App::RequestQuit(false, false)` (slot
       10: no dialog, no save), the city and the app shut down as on a normal quit, and the verdict is written at
       `PostSystemServiceShutdown`, so it counts reference warnings from every subsystem's `Shutdown`. Its `julian`
       is the last day played. A quit that has not reached that hook in 90 s is `stalled`; a process still alive
       30 s after the verdict is terminated.
   - **Clock probe** (`clock_probe = 1` or `2`, env `FC4_CLOCK_PROBE`; off by default; `plugin/FC4ClockProbe.h`):
     the M1 clock differential. From game start it logs every message the simulator posts (`CLOCK post day|week|
     month|year j=… c=…`, `CLOCK post pause|hidden|emergency|speed d1=…`), and at every city load it registers an
     agent at the day..year levels and logs each delivery (`CLOCK agent day|week|month|year j=… c=…`). With `2`
     the week agent reports "not finished" once per week, so §5.3's repeat delivery shows as `… again`.
     `tools/clock_diff.ps1 -A <original log> -B <our log>` checks both streams against simulator.md §2.3/§5.2 and
     compares the two runs from the first day both saw (runs 56/57: identical over 240 days).
   - **Monthly digest** (also at `FC4_LOG_LEVEL=1`; `digest = 0` turns it off): at every `SimNewMonth`, one line
     for diffing the original against FreeCity4 on the same save, read only through public interfaces:

     ```
     DIGEST julian=… (yyyy-mm-dd) funds=… population R/C/I=…/…/… demand
       R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT=…/… budget income/expense=…/…
     ```

     R is `cISC4ResidentialSimulator::GetPopulation`; C and I are the summed `QuerySupplyValue` of the
     commercial and industrial demands (jobs); demand values are `QueryDemandValue` of
     `cISC4DemandSimulator::GetDemand(id, 0)` (`-` when the game has no such demand).
   - **Observe** (`observe = 1`, env `FC4_OBSERVE`; default on in checked builds, off in release builds): at every
     `SimNewMonth`, and in every mode, one more line with the month's money:

     ```
     observe month yyyy-mm: date=yyyy-mm-dd julian=… funds=… income=… expense=… tax[0..11]=…/… dept_expense=0xID:…,…
       demand R$/…/I-HT=…/… population R/C/I=…/…/…
     ```

     `income` / `expense` are the budget's total monthly income and expense, `tax[t]` is `GetTaxIncome(t)` for the
     budget's 12 tax types by index (their zone and wealth names are not verified, so the line does not name
     them), `dept_expense` is `GetTotalExpenses` of every department of every budget group (sorted by ID; `-`
     without the game's allocator). At the first tick after every city load it also writes
     `observe snapshot (after load): date=… julian=… funds=… fundings=0xID:pct,…` (the date from the
     simulator, `pct` the stored `SC4Percentage`). Only slots checked against `docs/abi` are called; the list is
     at the top of the observe section of `plugin/FC4Observer.cpp`.
   - **Save autotest** (`autotest_save_after_months = N`, env `FC4_AUTOTEST_SAVE_AFTER_MONTHS`): at the first tick
     after the N-th `SimNewMonth` since the city load, write `observe snapshot (before save): …`, save the city in
     place with `cISC4App::SaveCity(bool)` (a full save; `autotest_save_fast = 1` for a fast one), log
     `autotest: save after N month(s): SaveCity(full) returned 1 in … ms`, then write the verdict and end the
     process like the autotest. It needs no `autotest_seconds`; with `autotest_unpause` (which does) the city
     runs by itself, so set `autotest_seconds` beyond the N months. A second run that loads the saved city
     (autoload) logs `observe snapshot (after load): …` to compare with the line before the save.
   - **Popup dialogs off** (`popup_dialogs = 0`, env `FC4_POPUP_DIALOGS`; not set = the game is left alone): for
     unattended runs, where an advisor popup holds the city (a hidden pause of the simulator) until someone clicks
     Close. FreeCity4 calls `cISC4App::SetPopupDialogsEnabled(false)` (slot 24) when **ConnectionsReady**
     (`0x6AC284F3`) arrives, a few frames after each city load, and sets it true again at the next city load
     (advisors spec §6.8.1, option 1; foundations §11.6). Not earlier: with popups off, the terrain edge
     reconciliation is skipped and the regional city never posts ConnectionsReady, so power, plumbing and the
     neighbours simulator stay disabled all session (runs 58–64 did this at `PostAppInit`; run 64 vs 65 shows it).
     Advisor popups are then dropped without pausing, and the generic notification,
     confirmation and scripted modal dialogs return without asking (false / 0; the quit, go-to-region, obliterate
     and budget-failure prompts use the scripted one, and what they then do is not in the spec). City-situation
     popups still show and pause. The log proves it
     took effect by reading the getter (slot 23) before and after:
     `popup dialogs: disabled (cISC4App::SetPopupDialogsEnabled(false); was enabled=1, now=0) at ConnectionsReady`.
     `popup_dialogs = 1` calls it with `true`; any other value logs `config: WARNING popup_dialogs='…' … is not
     0/1` and leaves the game alone. Manual fallback without FreeCity4: untick Options → Play Options → "Urgent
     Advice Dialogs" (saved in the user's preferences; advisor popups only).
   - **Budget shadow** (`budget_shadow = 1`, env `FC4_BUDGET_SHADOW`; off by default; `budget_shadow_lots = N`,
     default 6): a same-process differential of property tax. It needs **our simulator and the original budget**
     (`disable = budget`); with any other combination it logs `budget shadow: requested … but INACTIVE` at game
     start. Our simulator wraps its `SimNewMonth` delivery to the original budget's month agent: just before it,
     FreeCity4's own `ComputeTax` (`fc4_budget_shadow_tax`, the core code our budget runs) scans the live lots with
     the original's tax rates; just after it, the original's `GetWeightedAssessedTaxValue(t)`, `GetTaxIncome(t)`,
     `GetTaxIncome()`, year-to-date income/expense and funds are read and compared. Read-only: the scan's host
     table has no write callback, and only getters are called on the game. One line per month:

     ```
     budget shadow yyyy-mm-dd: tax orig/ours=a/b,…(12 types; ! = differs) ; total orig/ours=…; income orig/ours=…;
       expense orig/ours=…; net orig/ours=…; lots=N; mismatches=M [tax 3,4 total income expense]; months=K clean=C
     ```

     A type mismatches when its assessed sum or its `GetTaxIncome(t)` differs. `ours` income is our tax plus the
     ordinance, neighbour-deal and department income the settlement books; `ours` expense is ordinance + neighbour
     deals + the departments' expenses after the original's settlement + bond payments (so income/expense check
     the settlement's composition; only the tax is computed by our core). At the January 1st settlement the
     year-to-date values are reset, so income/expense read `-` there and only `net` (the funds delta) compares.
     For each mismatched type: `budget shadow … type T: assessed orig/ours=…/… (orig-ours=D); tax …; ours:
     counted=…, not-livable=…`, then up to N lots of that type, (x,z), size, status, purpose, building/lot wealth,
     value, land value, assessed and tax, those whose assessed value equals |D| first, then not-livable ones.
   - **Demand shadow** (`demand_shadow = 1`, env `FC4_DEMAND_SHADOW`; off by default; `demand_shadow_nodes = N`,
     default 8, 0..64): a same-process differential of RCI demand. It needs **our simulator and the original demand
     simulator** (do not enable `demand`); otherwise it logs `demand shadow: requested … but INACTIVE` at game start.
     Our simulator wraps two deliveries to the original demand simulator's agent (`cSC4DemandSimulator`):
     - `SimNewDay` (demand §10.2: `DoIdle(5)`, then the propagation `DoDemandIndexLinks`, the only step that changes
       committed and active demand): just before it every node of the graph is read through `cISC4Demand` getters
       (supply, demand, pending supply/demand, active, min/max, tax/economy modifiers, cap, region use); just after
       it the same, plus `GetNeutralTaxRate()` (only when `0x1001`, `0x3001` and `0x4001` exist). FreeCity4's own
       propagation (`fc4_demand_shadow_propagate`, the core code our demand simulator runs, on a graph built once from
       the same demand exemplars) then runs over the states read before, and every node's six values are compared
       bitwise. All propagation state is in the nodes, so seeding ours from the original each day keeps the inputs
       identical. If the `DoIdle(5)` slice set the 12 total nodes' modifiers (census state 0, §8.1), ours uses the
       original's new values (noted). If a census finished inside it (census nodes moved although they only fold
       their own pending values), their pending values are inferred as post − pre and ours runs again (noted, and
       counted as `inferred=`: an ulp there is not conclusive).
     - `SimIdle`: the 12 total nodes' tax/economy modifiers are read before and after; when one changed (census
       state 0), FreeCity4's §8.1 (`fc4_demand_shadow_modifiers`) from the census sums read before is compared. The
       CO/I economy bonuses are not readable and are taken as 0 (§15 Q5).

     Read-only: the core's host table (`DemandReadOnlyHost`) has the exemplar reads and the §8.1 getters only (no
     node creation, reference counting, posts or history writes), and only getters are called on the original;
     both halves run under the structured-exception guard. Lines (always written):

     ```
     demand shadow yyyy-mm-dd: demand orig/ours=a/b,…(R$ … I-HT local demand, as the digest; ! = bits differ);
       active orig/ours=…(the 12 total nodes); neutral orig/ours=…; nodes=N; mismatches=M [0xKEY,…] max|orig-ours|=…
       maxulps=…; passes=P transfers=T; updates=K clean=C [inferred=I] [; notes]
     demand shadow yyyy-mm-dd node 0xKEY (ru R): supply orig/ours=…[!(orig-ours=d, u ulp)]; demand …; pending supply …;
       pending demand …; active …; cap … | before: …          (up to N nodes, 48 lines per update)
     demand shadow yyyy-mm-dd modifiers (SimIdle|NewDay): tax orig/ours=…(12); econ orig/ours=…(12); neutral=…;
       mismatches=M; checks=K clean=C
     demand shadow yyyy-mm-dd modifiers R$ (0x21010): tax …; econ …; before …; ours: max rate … ratio … curve …
     ```
   - **Pollution shadow** (`pollution_shadow = 1`, env `FC4_POLLUTION_SHADOW`; off by default;
     `pollution_shadow_cells = N`, env `FC4_POLLUTION_SHADOW_CELLS`, default 4, 1..64): a same-process differential
     of the eight pollution grids. It needs **our simulator and the original pollution simulator** (do not enable
     `pollution`); otherwise it logs `pollution shadow: requested … but INACTIVE` at game start. Our simulator wraps
     each `SimNewDay` and `SimNewMonth` delivery to the original's agent (`cSC4PollutionSimulator`): just before it,
     the original's own `Write` (`cIGZSerializable`) goes into a memory stream, and FreeCity4's core loads that
     record, copies the eight live grids it refers to and runs its own day or month on the copies, reading every
     other input (city geometry, tuning, terrain, zones, air traffic, ordinances, deals, power, population,
     landfill, occupant properties, budget items) through a read-only host table; just after it, the record is
     captured again and our grids are compared with the live ones cell by cell, with the saved fields, and with
     the occupant properties and deal usage. Nothing is written to the game (the shadow's host table has no write
     callbacks; its grid table only reads). Not readable read-only, hence not compared (said in the start line):
     the radiation sum (not saved), the posted messages, history and budget values, the visual effects and the
     worst tracts. Lines (always written; longer ones continue on `pollution shadow (cont.)` lines):

     ```
     pollution shadow yyyy-mm-dd day|month: …    a delivery with a difference (at most 8 a month), a rerun with
                                                 the original's tract order, or INCOMPLETE (why); not compared
     pollution shadow month to yyyy-mm-dd: deliveries=N (days D, months M) clean=C incomplete=I reruns=R
       uncached=U; cells=X mismatches=Y; airSrc y/x max|d|=…, waterSrc …, garbageSrc …, radSrc …, airVal …,
       waterVal …, garbageVal …, radVal …; first differing cells: …; fields: …; props compared=… mismatched=…;
       deals compared=… mismatched=… [; incomplete because: …]
     ```

     Success is `clean` equal to `deliveries` and `mismatches=0` on every month line.
   - **Protection shadow** (`protection_shadow = 1`, env `FC4_PROTECTION_SHADOW`; off by default): a same-process
     differential of the fire and police coverage grids. It needs the **original** fire and police simulators (do
     not enable `protection`; otherwise `protection shadow: requested … but INACTIVE`). At every `SimNewMonth` (the
     observer's notification), per simulator, the original's adjusted grid (slot 5), department, strike flag,
     station list, chance of strike, total dispatch count and per-station efficiency and dispatch radius are read
     through its public getters; FreeCity4's core (`fc4_protection_shadow_run`, over a read-only host table: no
     callback that changes the game) rebuilds the unadjusted grid from those stations and the dispatch manager's
     active dispatches (§5.1) and derives the adjusted grid (§2), which is compared tract by tract. The original's
     adjusted grid lags its unadjusted grid by up to one daily pass (§5.3, about 51 fire / 41 police days), so an
     exact match is expected only when our grid was the same at two month ends in a row and no dispatch is active
     (`expectExact=1`); a mismatch then also logs a `WARNING` line. The original's unadjusted int16 grid and its
     cycle accumulators have no getter (logged at game start), so they are not compared. Lines (always written):

     ```
     protection shadow (fire|police) yyyy-mm-dd: cells=N mismatches=M max|diff|=D stations=S dispatches=K stable=s
       expectExact=0|1 first=(x,z):orig/ours,…(8); efficiency mismatches=a/S [(first …)]; radius mismatches=b/S;
       strike=0|1; chance orig/ours=…/…[ MISMATCH]; totalDispatch orig/ours=…/…; avg=… pct=… (original)
     protection shadow (fire|police) yyyy-mm-dd: WARNING M cells differ although the grid should be exact (…)
     protection shadow (fire|police) yyyy-mm-dd: skipped (…)
     ```

     Success: `mismatches=0` on every line with `expectExact=1`, `efficiency mismatches=0/S`, `radius
     mismatches=0/S`, equal chance and total dispatch.
   - **Crime shadow** (`crime_shadow = 1`, env `FC4_CRIME_SHADOW`; off by default; `crime_shadow_cells = N`, env
     `FC4_CRIME_SHADOW_CELLS`, default 8, 0..64): a same-process differential of the crime grid. It needs **our
     simulator and the original crime simulator** (do not enable `crime`); otherwise it logs `crime shadow:
     requested … but INACTIVE` at game start. The shadow follows the original's pipeline (crime spec §5) from our
     simulator's deliveries to its agent (`cSC4CrimeSimulator`): a new or loaded city starts in state 6,
     `SimNewMonth` sets 6 → 0, each `SimNewDay` / `SimIdle` runs one step. It wraps the delivery that runs state 4
     (decay, commit this month's crimes, average): just before, the crime grid (every tract) and the hotspot are
     read; just after, the grid, hotspot, monthly count, average, the incident list (into a vector of ours) and
     each incident's lot rectangle and the zone type at every tract corner. FreeCity4's state-4 grid work (the core
     code our crime simulator runs) is then replayed on the grid read before with the original's incidents, and
     compared cell by cell, with the hotspot, count and average. Only getters are called on the original, its grid,
     the lot and zone managers and lots; both halves run under the structured-exception guard. **Not compared**
     (said in every line): the number of crimes to commit (it needs `ResetUnjailedCriminals`, which has side
     effects), which attempts the police stopped and the type each crime got (the original's random draws), the
     lots tried (the lot manager's generator) and the crime-effect grid (no interface exposes it). Lines (always
     written):

     ```
     crime shadow: tracking the original crime simulator …     (each city)
     crime shadow yyyy-mm-dd: state 4 compared on CxZ tracts (S=s; …): cells mismatched=M max|orig-ours|=D
       [cells (x,z) orig/ours: (x,z) a/b pre p, …]; hotspot orig/ours=(x,z)=v/(x,z)=v (before: …, replayed from …);
       count orig/ours=a/b; average orig/ours=a/b; incidents=I (…); mismatches=N; months=K clean=C; not compared: …
     crime shadow yyyy-mm-dd: state 4 NOT OBSERVED in this delivery …   (the mirror is out of step: resynchronised
                                                                          at the next SimNewMonth)
     crime shadow yyyy-mm-dd: INCOMPLETE (why); not compared
     ```

     Success is `mismatches=0` on every month line (`clean` equal to `months`) and no `NOT OBSERVED` or
     `INCOMPLETE` line. A `!` marks a differing hotspot, count or average.
   - The autotest clock, the digest, the observe lines, the save autotest and the tick feed for the watchdog
     come from the **observer**
     (`plugin/FC4Observer.cpp`), a system service and message target registered at `PostAppInit` in every mode,
     including `FC4_DISABLE=all`.
   - **x87 control word** (always on; foundations §18.2): the game runs its main thread at 24-bit x87 precision and
     FreeCity4 never changes it. `fpu <where>: x87 CW=0x… (PC 24-bit, RC nearest, masks 0x3F) MXCSR=0x…` is logged at
     DLL attach (before and after our initialisation), at every framework hook (`PreAppInit` is the guard's
     reference: the first hook after the game's `PreFrameWorkInit`), at city load, at each observe month line,
     around our budget settlement, and at the first tick and every tick where the control word changed (with the
     tick number). The guard writes `fpu: WARNING the main thread's control word changed (…)` the first time the
     main thread differs from the reference, and `fpu: WARNING the main thread is not at 24-bit x87 precision (…)`
     the first time its precision is not 24-bit. A run without 24-bit precision is not a valid reference for
     float-sensitive values.
   - **Power recompute report** (`power_report = 1`, env `FC4_POWER_REPORT`; default on in checked builds, off in
     release builds; our power simulator only): after each recompute cycle
     `power recompute N: cells …x… widths … ; wire …x… words … bits set …; consumption cells …; plants …, floods …;
     production … consumption … demand … powered … unpowered …`, then one `plant 0x…: visited … position (…) seed
     (x, z) cells [x1,z1..x2,z2] capacity … funding … condition … efficiency … produced … connected …` line per
     plant and one `flood from (x, z) wired …: reached … cells, rows …; supply … import … available …; consumers …
     need …; powered …` line per flood. The first 40 cycles are logged, then the first after each month.
   - **Reference audit:** every `AddRef`/`Release` we do on a game object (vtable outside our DLL), and every
     reference the game hands us (QueryInterface, out-parameters, lot lists, save references), is logged at
     verbose level under its own cap. Each line gives the object, its class (`GetGZCLSID`), its vtable as
     module+offset, the count returned and our call site.
     - Always logged: `WARNING: released last reference to game object …` and `WARNING: over-release …`. A site that
       frees a temporary the game created for us (a new object returned through an out-parameter with one reference,
       e.g. the demand glue's resource key list) releases it with `FC4_RELEASE_CREATED` after `FC4_ACQUIRED`: that
       expected free is counted in the summary (`temporaries created for us`), not warned.
     - At each simulator or budget `Shutdown`: the net count per class and the objects we still hold.
5. **Checked build:** `cmake -S engine -B build-x86-checked -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
   -DFC4_CHECKED=ON`.
   - The plugin is built with `/Od /RTC1`, so an argument-count or callee-pop mismatch on any vtable call is
     caught at the call.
   - RTC failures go to the log, not a dialog: the kind, the call site as `FreeCity4.dll+offset` with our
     function and line, the frames, the activity and the last interface calls. Copy `FreeCity4.pdb` next to the
     DLL for the names.
   - Small buffers we pass to game methods (`GetSize`, `GetLocation`, `GetSimDateParts`, QueryInterface and
     `GetResource` out-pointers, the lot list, `ReadGZSerializable`) are surrounded by canaries. A write past
     them logs `OUT-PARAM OVERRUN`.
   - `FC4_CHECKED_SELFTEST=1` provokes one of each at start-up.

## Conventions

- Every source file starts with an `SPDX-License-Identifier: GPL-3.0-or-later` header.
- Spec gaps are decided in code and tagged `[choice]`. Behaviour the SDK cannot express yet is tagged
  `TODO(spec §n)`.
- Panics never cross the C ABI. Release builds use `panic = "abort"`, and each `fc4_*` entry point also catches
  unwinding.
- Every spec behaviour needs a conformance vector. See [`conformance/README.md`](conformance/README.md).
- Game interfaces are called through vtable slots, so slot order is part of the ABI (foundations §16). An
  interface with overloaded methods is declared in [`plugin/FC4Interfaces.h`](plugin/FC4Interfaces.h) in the
  spec's verified slot order with a unique name per method, never taken from the SDK header: MSVC regroups and
  reverses overloads, so with overloads the layout follows the compiler, not the declaration.
  `tests/vtable_layout_test.cpp` checks every slot of every interface we implement against the spec tables, and
  `plugin_load_test` calls each overload of the real objects through its slot with the game's argument count and
  checks that the stack balances. New code must not implement or call overloads of an interface until its spec
  has a verified slot table. Declared from the foundations §17 tables: `cFC4IGZDate` (implemented by
  `cFC4Date`), `cFC4IGZVariant` (implemented by `cFC4Variant`, the only variant we hand to the game; never pass
  the SDK's `cRZBaseVariant`, whose slots 8 and 60 differ from the game's), `cFC4ISCPropertyHolder`,
  `cFC4ISC4LotManager`, `cFC4ISC4Lot`, and `cFC4ISC4NeighborsSimulator` (from the ABI table: slots 32/33
  return 32 bits). Interfaces still used through SDK declarations: `cIGZString`, `cISC4LineItem`,
  `cISC4BuildingDevelopmentSimulator`, `cISC4DisasterManager`, `cIGZPersistResourceManager`,
  `cISC4HistoryWarehouse`, `cISC4OrdinanceSimulator`, `cISCResExemplar`.
- **ABI conformance** (`tests/abi_conformance_test.cpp`, CTest `abi_conformance_test`): every declaration above
  is checked against [`docs/abi/interfaces.tsv`](../docs/abi/interfaces.tsv), read at run time. Per slot:
  argument bytes (each argument rounded up to 4, int64/double 8, plus 4 for a hidden result pointer) ==
  `callee_pop`; float/double return ⇔ `ret = x87`; int64 return ⇒ `edx:eax`, and `edx:eax` (medium
  confidence) ⇒ int64. Per interface: slots contiguous and as many as the table's. The slots are decoded from
  the compiler's vcall thunks, so the test checks what MSVC actually laid out. After adding a method or an
  interface, run `python tests/gen_abi_entries.py` (from `engine/`) and rebuild; `--dump` prints every slot
  next to its table row. SDK methods that disagree with the game but are never called by the plugin are
  waived (`kWaivers`); the waiver lapses, and the test fails, as soon as a file in `plugin/` calls one.
  Reviewed slot-count differences (class-private trailing virtuals) are in `kReviewedSlotCounts`. Interfaces
  the plugin only calls, and which `interfaces.tsv` lists under another name, are checked against a game
  class's vtable in `docs/abi/vtables.tsv` instead (entries named `vt:<class>`, e.g. `vt:cSC4City::vftable`
  for `cISC4City`); there the class may have more slots than the interface declares.
- **Test seam:** `FreeCity4.dll` exports `FC4TestSeam` ([`plugin/FC4TestSeam.h`](plugin/FC4TestSeam.h)) for
  tests that load it without a game: a sink that sees every budget message as posted (budget §10: void1 =
  IGZUnknown = the sender), a fake city for the observer, and direct observer ticks and messages. The game
  never calls it. `plugin_autotest_test` runs three child processes (clean, corrupt, stalled) that end through
  the autotest's `TerminateProcess`.
- Return types are part of the ABI too: SDK declarations can be wrong (budget erratum:
  `cISC4DepartmentBudget` slot 26 returns a float POINTER in EAX, not a float in ST0; `cIGZDate` slots 33–36
  return a `uint32` day number and leave the date alone; `cISC4Lot` slot 61 returns `SC4Percentage` by value
  through a hidden result pointer; from the ABI table: `cIGZVariant` slot 41 and `cISC4Lot` slot 15 return in
  ST0, `cISC4BudgetSimulator` slots 20–23 in EAX only). Such interfaces are declared in `FC4Interfaces.h`, and the slot tests check
  the x87 stack and the stack balance: only slots the spec says return a float may leave a value there.
