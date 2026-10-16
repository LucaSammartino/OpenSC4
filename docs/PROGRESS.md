# Progress

The detailed milestone checklist: what is done, how it was verified in game, and what is next. The landing page is
the [README](../README.md); milestones are defined in [ARCHITECTURE.md](ARCHITECTURE.md#roadmap) and the in-game run
log is [manual-checks.md](manual-checks.md). Keep this file current at every merge.

Last updated 2026-10-14 (run 103). "Core" means the Rust logic, with its conformance vectors, in `engine/core`. "Glue" means
the plugin code in `engine/plugin` that runs the core inside the real game. Every glue except the simulator and the
budget is off by default (enable = <key>).

**M0: toolchain**
- [x] CMake project, Rust core, C ABI, conformance runner (658 vector files)
- [x] `FreeCity4.dll` loads in SC4 1.1.641 and replaces `cSC4Simulator`
- [x] Unattended in-game runs: autoload a city, autotest verdict, monthly observer (`tools/ingame_run.ps1`)
- [x] Hands-free game control: advisor popups and the save box closed automatically, scripted UI actions (`-Actions`), the test city restored after a save (`tools/sc4ctl.ps1`)
- [x] Popups off without side effects: `popup_dialogs = 0` now waits for ConnectionsReady (d161d20). Set earlier, it silently kept power, plumbing and neighbours disabled (runs 58–64; foundations §11.6)

**M1: the clock**
- [x] Simulator core and spec
- [x] Runs in-game: days, weeks and months advance; limit-timer calibration
- [x] Save → reload verified in-game (runs 50/51: date, funds and all 18 departments survive)
- [x] Pause and all three speeds verified in-game (runs 50–53, driven by `tools/sc4ctl.ps1`; speeds call `SetSimSpeed(0/1/2)`)
- [x] Emergency pause verified in-game (run 54: a God-mode disaster calls `EmergencyPause()`/`EmergencyResume()` on our simulator)
- [x] Clock differential against the original on the same save: **identical** (runs 56/57, `clock_probe` + `tools/clock_diff.ps1`). Over 240 days on Konradshohe (2026-01-28 to 09-24) the probe agent's day/week/month/year deliveries and the posted SimNewDay/Week/Month/Year messages match entry for entry, and both follow the spec's calendar rules

**M2: scalar sims**
- [x] Budget, demand, ordinance and weather cores
- [x] Budget, demand, ordinance and weather glue (budget in-game with no heap corruption, runs 17–18)
- [x] x87 precision: diagnosed **and worked around** — the game runs 24-bit all session; our DLL's presence raises it to 53-bit (foundations §18.2). Since 63e88f1 the plugin re-asserts 24-bit at `PostAppInit`, and the control word reads `0x007F` for the rest of the session. The *trigger* is still not located (foundations §19.4); the re-assert makes it moot for differential work
- [x] Budget differential: **ours matches the original exactly** (run 39, the same-process shadow differential, merged 6ca4b65). On the same lots at the same moment, all 12 tax types, total tax, income, expense and net were identical: 5 of 5 month-ends on Spandau (run 39) and 35 of 35 on Konradshohe, 2026-02 to 2028-12 with about 700 lots (run 41), with 0 mismatches. Separate-run comparisons (runs 35/36) could not settle per-type tax, because the original is not deterministic from run to run; the shadow (`budget_shadow = 1` with `disable = budget`) removes that
- [x] Budget and demand match the original (M2 exit). **Budget:** identical on every month-end, 5 of 5 on Spandau (run 39) and 35 of 35 on Konradshohe (run 41). **Demand:** bit-identical, 160 of 160 days and 6 of 6 modifier checks (run 44), after the one difference the shadow found was traced to the exe (tax ratio = maxRate·0.1f × a float32 reciprocal of neutral; demand.md §8.1) and fixed in 42f7649. Both checks are same-process shadow differentials (`budget_shadow`, `demand_shadow`), opt-in
- [x] Demand glue reads the building-profile capacity maps and fills `GetLocalPopulationSummary` through the game's own STLport map (merged ae82e65); run 61's soak had found demand frozen without them

**M3: grid sims**
- [x] Cores: SimGrid, land value, pollution, crime, aura, flammability, power, plumbing, protection, flora
- [x] Land value glue
- [x] Power glue (merged 63e88f1; runs 27/28 clean, production 5997→6000, 409 cells powered, 0 unpowered)
- [x] Plumbing and flammability glue (merged 9f741de), **now run in game** (runs 33–38): the water pass on a real network (2 producers, 403 pipes, watered = building cells), fire rolls about every 40 days, and the cross-subsystem `IsCellWatered` read is live. Run 37 caught a use-after-free on fire ignition (a borrowed manager was released), fixed in 95db886; run 38 went through a real fire and a full Shutdown with 0 reference warnings
- [x] **Pollution glue** (merged d743db2; run 67 clean over 73 months) and its same-process **grid shadow: identical to the original on all eight grids, cell by cell, for 94 months** (run 68). Its one difference, a `dirtyWater` flag, was fixed in 4619a6f; exact since run 73
- [x] **Protection glue** (fire + police, merged 29b3fcb) and its **grid shadow: identical to the original, cell by cell, for 97 months** (run 70). The fire grid read 0×0 after a load; fixed in f7a9f16 (run 73: fire cycles running)
- [x] **Crime glue** (merged 941c34f) and its **grid shadow: 96 of 96 months identical**, hotspot included (run 70b); the replacement runs clean for 97 months
- [x] **Aura glue** and its shadow (merged 7c3f3a2; run 75b: the replacement runs clean; the shadow is exact since run 95)
- [x] **Flora glue** and census shadow (merged f8ebce0) and **landfill developer glue** and shadow (merged c2ada06); in game since run 80
- [x] **All ten replacements together track the original for 36 game years** (runs 77/78 on Konradshohe: the same funds curve and the same end in 2062)
- [x] Shadows for power (290093b), plumbing (571a47e), land value and flammability (9bb91f2): in game since runs 80–83
- [x] x87 return channels fixed (5ee5c52): our methods no longer leave values on the game's x87 stack (run 74's corruption); 0 leftovers in every run since, the 900 s soaks included
- [x] **Shadow scoreboard (run 103, `regress.ps1 -Tag r6c`), exact:** pollution, protection, crime, aura, flora,
  flammability, land value, residential, power (463/463), plumbing (179/179; exact since run 103, after apply6c),
  landfill (89/89), neighbours, budget, demand; 0 INCOMPLETE lines. Zones differ only by ZG17
- [x] **Landfill replacement game tiles** (apply9b, merged 969527d): the tile exemplar is set through the holder at occupant + 0x20 (LG27); in the full city `gameTiles=12 gameTileFailures=0` (run 107)
- [x] **Grid snapshots match within tolerance (M3 exit): met in run 103.** Every grid shadow exact, and the replace run (sixteen replacements) clean

**M4: growth**
- [x] Specs: building development, growth, tract, lot and zone developers
- [x] Cores: building development simulator, growth developer, tract developer, **lot developer** (merged 9889f65; 5,254 lines, 36 unit tests, 18 conformance vectors), **zones** (merged f56e52a; ~10,900 lines, 30 conformance vectors covering all 29 §15 tests)
- [x] **M4 cores complete (5 of 5)**; zones found and fixed a real `SimpleParcellize` Q=2 x-line bug during self-review (`zn15_31`)
- [x] Zones follow-up (merged 8ee3253): spec answers from the exe applied; `DetermineLotSize` filter in the core; tests 30–32 as vectors `zn15_32`–`zn15_35`
- [x] Landfill tests 1–23 (`lf11_*`) and the **neighbours core** with tests 1–12 (`nb18_*`) (merged 026fd9e)
- [x] **Zones glue, phase 1** (the zone manager replacement and its shadow; regress rzones: 16 replacements together run clean, run 97). Run 99 exercised the PlaceZone commit path and the dezone handler in game: clean. The shadow is exact since apply9b (292/292, run 107)
- [x] **Neighbours glue** and its shadow (merged 50e2dc5, fixed 5e61237): the shadow matches the original every month (92/92); the replacement runs clean with the other fourteen (regress rneigh2). The first in-game run crashed until our property holder kept the game's own property objects (foundations §10.2)
- [x] **Residential simulator core** (merged 737bcc5): all 24 residential §15 tests as unit tests and `rs15_*` vectors, the save record; REH building work is left to host hooks
- [x] **Residential education and health core** (REH §2–§7, merged e520128): all 24 REH §11 tests as unit tests and `reh11_*` vectors (36 new vectors)
- [x] **Residential glue** and its shadow (merged bc7ffa0): in game since run 86. The replacement runs clean for 39 game years (run 89) and saves and reloads (runs 88, 91); the shadow is exact since run 95; the original's EQ/HQ path (run 90b) and ours (run 89) decline the same way
- [x] Growth, building development and tract developer spec questions answered in the specs (8fdea4e)
- [x] Daily growth budget calibrated in the original (growth probe merged 393a529; run 93: 14,339 days, the 3 ms timer never cuts, so the default is unlimited)
- [ ] Seeding rules (foundations §6.3)
- [x] **Development glue** (building development simulator, growth developers and `development_shadow`; merged c787ecf, fixed in devglue2 38524aa): the catalogue loads 744/744 types and 281 families, the shadow is exact (913/913 in regress m850, run 104), and the replacement grows like the original at equal pacing. Our unit is about 3x slower per game day; the DG4/DG5/DG6/DG11 answers are applied (apply10a, merged b398049: BDS foundation and retaining wall slots 23/26/29, the budget item cost sum; regress run 110 PASS)
- [x] Tract developer glue (merged on main 76bf685; regress mtract PASS, run 109)
- [x] **Lot developer glue** (merged 5212df2, ABI 54; regress lotglue PASS, run 116): `enable = lotdev`, off by default, clean in game to 2032 alone and with `development` (203 buildings started, 402 abandonments, ref_warnings=0); `lotdev_shadow` clean with every record round trip identical, but not yet exact: slots 25 / 26 flatness differ on lots with an altitude range 1.0-1.68 (LD29). Open: LD29, the night batches never run under turbo (LD28), retaining walls, burnt / rubble props, network nodes and business unit creation unspecified (LD12-LD26), adding `lotdev` / `lotdev_shadow` to the regress lists
- [x] **Zone developer and lot manager glue**, zones phase 2 (Stage A merged 9a9b0df, regress zones2 PASS, run 112; **Stage B merged 3cb9b43**, ABI 55; regress zones2b PASS, run 117): `enable = zonedev`, off by default; game lots are created from the game's lot pool and constructor (zones.md §8.5.1), slot 21 CanCreatePloppedLot, UpdateZoneType and the §5.2.1-§5.2.2 street joins are implemented; clean in game for about one turbo year (68 lots built, 0 refused, ref_warnings=0) and `zonedev_shadow` exact in every lane run (70/70; 365/365 in regress). Open: no run has zoned, dezoned or plopped (a scripted zone-and-plop run would cover the dezone, the joins and slot 21); `zonedev` is not in the regress replace list; UpdateHighlightTextures omitted (allowed by the spec); ZG22 (slot 19 `*ok`) and ZG24 (the highlight region with a zone tool) answers not yet applied
- [x] Specs: commercial and industrial simulators, civic buildings, park manager, history warehouse, 24-hour clock (re-specs9, a3599a0)
- [x] Cores for them: **history warehouse and 24-hour clock merged** (0757437: every §12 test, `hw12_*` and `ck12_*` vectors; glue merged, below); **commercial, industrial, civic and parks merged** (ab3c1c3: `ci13_*`, `cb12_*`, `pk12_*`; glue merged, below)
- [x] **History warehouse and 24-hour clock glue** (merged efac89a, ABI 52; regress histclock PASS, run 113): `enable = history, clock24`, both off by default; `history_shadow` and `clock24_shadow` exact in every lane run (224/224 and 207/207 at normal speed to 2043); the save sweep (`engine/formats/tests/save_records.rs`) reads and rewrites both records of all 417 stock saves (history version 1 converts to version 2 stably)
- [x] **Commercial, industrial, civic and park glue** (merged 58b6d11, ABI 53; regress cmglue PASS, run 115): `enable = commercial, industrial, civic, parks`, all off by default; `commercial_shadow`, `civic_shadow` and `parks_shadow` exact in every lane run (commercial 18/18, civic 73/73, parks 73/73 to 2032); the four with the 17 regress replacements clean to 2032; civic `Read` follows CB6; the save sweep (`capi/src/save_sweep_tests.rs`) reads and rewrites the four records of all 417 stock saves. Open: a save/reload run with all four, and adding them to the regress lists
- [ ] The growth statistics distribution matches across seeded runs (M4 exit)

**M5: traffic**
- [x] Specs: traffic simulator, traffic network, pathfinder
- [x] Traffic network core (merged dd240c9: 50 unit tests, 20 conformance vectors for all twelve §11 tests; §4.14 and §6 persistence wait for the traffic simulator)
- [x] Pathfinder core (merged 36ababe: 28 `pf11_*` vectors)
- [x] **Traffic simulator core** (merged a11eb7d: all 15 §11 tests as unit tests and 29 `ts11_*` vectors)
- [x] **Traffic glue, phase 1** (merged 850b567, `enable = traffic`): our traffic simulator and pathfinder replace the original in game; clean alone, across saves and for 84 months at normal speed, and in a full city with 17 other replacements (run 105, one reference warning under investigation)
- [x] **Traffic phase 2** (merged 40d9eab, regress run 119 PASS): the TS35–TS49 answers, the O(1) source iterator, and the traffic shadow (`traffic_shadow = 1`): with `traffic_shadow_hypotheses = 7` every delivery matches the original on Konradshohe for 7 game months (261/261, resyncs=0); TS55–TS57 decide whether those readings become the spec
- [ ] Calibration; commute and volume maps match (M5 exit): answer TS50–TS66, make the confirmed readings the core's behaviour, run the shadow on Spandau and past 2026-08 (on Konradshohe to 2032 under turbo it is not exact: run 121, 43 of 72 month lines, TS63/TS64); `traffic` and `traffic_shadow` are in regress since run 121

**M6+: beyond simulation**
- [x] Specs: game data formats (DBPF, exemplars, tuning catalogue, 601f3a9), lot/occupant save records (0460f3a), city save records validated against 417 saves (e69009f)
- [x] `engine/formats` matches spec §11 for DBPF, QFS and LTEXT (merged b5bdf8a; the sweep over real game data shows 0 failures across 22 .dat files and 1,004 saves)
- [x] `exemplar.rs` matches data-formats §4, §5 and the §11 exemplar rows (merged 64a25d2): the install sweep loads 9,347 of 9,347 exemplars and re-encodes 9,345 binary records byte-identically
- [x] FSH texture and S3D model readers (graphics-formats §2–§4, merged 2db780b): `fc4-dat fsh|s3d`; the sweep reads every FSH and S3D in the install
- [x] Occupant model keys (graphics-formats §4.2, GF7; apply10b, merged 0f3ad91): the zoom mask, RKT0–RKT5 and the RKT4 record walk; `fc4-dat models`; the install sweep reproduces the spec's survey exactly
- [x] Disaster layer core and C ABI (disasters.md §2-§14; cores13, merged 333ecde, ABI 57): fire, riot, toxic spill, earthquake, pipe burst, volcano and effect managers, the riot removal lock (Q4, network-manager §10); every §18 test as a unit test and a `ds*` vector
- [ ] Disaster glue: wire `fc4_disasters_host` to the game's managers and shadow it against the original; answer DS1–DS24
- [ ] Rendering, UI, audio, region; a standalone executable

**Cross-cutting**
- [x] Interface/ABI reference for implementers: allocators, layouts, direct calls ([docs/abi/](abi/))
- [x] Test harness switches in the plugin (6147be6, tooling, off by default): `sim_turbo` (our clock without the msPerDay wait), `autotest_end_date`, `script = <file>` (dated zone / dezone / speed / save / digest actions through the game's interfaces); turbo runs about 100 game days/s in game and waits for ConnectionsReady (0786064)
- [x] Parallel game slots and a per-lane run queue (9e0fa74): up to 3 games at once, each lane tests its own DLL in game ([docs/harness.md](harness.md)); `regress -Parallel` in about 164 s
- [x] Game dialogs ended from inside the plugin (dba09eb, tooling, off by default): `close_dialogs = 1` presses each modal dialog's default button through the window manager ([docs/specs/modal-dialogs.md](specs/modal-dialogs.md))
- [ ] SDK errata (289 findings) re-checked against upstream gzcom-dll and reported (needs the owner's OK)
- [ ] In-game manual checklist ([docs/manual-checks.md](manual-checks.md)): two human checks remain, the budget window's look and the cheats (`RiskyMoney`, `WeaknessPays`, `Moolah`)
