# FreeCity4

An **open-source (GPL-3.0-or-later) reimplementation of the SimCity 4 engine**. It is built the way OpenRCT2 was:
original code replaces the game piece by piece, verified against the real game, until it no longer needs the
original executable. Target: **SimCity 4 Deluxe 1.1.641** (GOG Windows exe, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`). You need your own copy of the game.

- How it works and the roadmap: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- How to contribute (clean-room rules): [CONTRIBUTING.md](CONTRIBUTING.md)
- Specs for engine authors: [docs/specs/](docs/specs/)

## Progress

Last updated 2026-10-14 (third session). "Core" means the Rust logic, with its conformance vectors, in `engine/core`. "Glue" means
the plugin code in `engine/plugin` that runs the core inside the real game. Milestones are defined in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#roadmap).

**M0: toolchain**
- [x] CMake project, Rust core, C ABI, conformance runner (416 vector files + 1 runner self-test = 417 tests)
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
- [x] Land value glue (off by default)
- [x] Power glue (merged 63e88f1; runs 27/28 clean, production 5997→6000, 409 cells powered, 0 unpowered)
- [x] Plumbing and flammability glue (merged 9f741de), **now run in game** (runs 33–38): the water pass on a real network (2 producers, 403 pipes, watered = building cells), fire rolls about every 40 days, and the cross-subsystem `IsCellWatered` read is live. Run 37 caught a use-after-free on fire ignition (a borrowed manager was released), fixed in 95db886; run 38 went through a real fire and a full Shutdown with 0 reference warnings
- [x] **Pollution glue** (merged d743db2; run 67 clean over 73 months) and its same-process **grid shadow: identical to the original on all eight grids, cell by cell, for 94 months** (run 68). The one difference is a `dirtyWater` flag on 104 days with identical grids (PG14, being traced in the exe)
- [x] **Protection glue** (fire + police, merged 29b3fcb) and its **grid shadow: identical to the original, cell by cell, for 97 months** (run 70). The fire grid read 0×0 after a load; fixed in f7a9f16 (run 73: fire cycles running)
- [x] **Crime glue** (merged 941c34f) and its **grid shadow: 96 of 96 months identical**, hotspot included (run 70b); the replacement runs clean for 97 months
- [x] **Aura glue** and its shadow (merged 7c3f3a2; run 75b: the replacement runs clean; the shadow is close but not exact, AU13–AU15)
- [x] **Flora glue** and census shadow (merged f8ebce0) and **landfill developer glue** and shadow (merged c2ada06); in game since run 80: landfill shadow exact (94/94), flora registry identical with one tree in some months (FQ15)
- [x] **All ten replacements together track the original for 36 game years** (runs 77/78 on Konradshohe: the same funds curve and the same end in 2062)
- [x] Shadows for power (290093b), plumbing (571a47e), land value and flammability (9bb91f2): in game since runs 80–83. Power and plumbing grids identical cell by cell on every replayed pass; their bookkeeping lines are off by inherited counts (PW8, PS8: being applied); land value exact (57/57); flammability 69/70 (FL6)
- [x] x87 return channels fixed (5ee5c52): our methods no longer leave values on the game's x87 stack (run 74's corruption); 0 leftovers in every run since, the 900 s soaks included
- [ ] Grid snapshots match within tolerance (M3 exit)

**M4: growth**
- [x] Specs: building development, growth, tract, lot and zone developers
- [x] Cores: building development simulator, growth developer, tract developer, **lot developer** (merged 9889f65; 5,254 lines, 36 unit tests, 18 conformance vectors), **zones** (merged f56e52a; ~10,900 lines, 30 conformance vectors covering all 29 §15 tests)
- [x] **M4 cores complete (5 of 5)**; zones found and fixed a real `SimpleParcellize` Q=2 x-line bug during self-review (`zn15_31`)
- [x] Zones follow-up (merged 8ee3253): ZQ2/ZQ3/ZQ5/ZQ6 answered from the exe and applied; `DetermineLotSize` filter in the core; tests 30–32 as vectors `zn15_32`–`zn15_35`. ZQ7–ZQ10 filed
- [x] Landfill tests 1–23 (`lf11_*`) and the **neighbours core** with tests 1–12 (`nb18_*`) (merged 026fd9e)
- [ ] Zones glue (phase 1, the zone manager: lane running)
- [x] **Residential simulator core** (merged 737bcc5): all 24 residential §15 tests as unit tests and `rs15_*` vectors, the save record, RS1–RS6 applied; REH building work is left to host hooks. RS7–RS17 filed
- [x] **Residential education and health core** (REH §2–§7, merged e520128): all 24 REH §11 tests as unit tests and `reh11_*` vectors (36 new vectors), RS7–RS17 applied, EH1–EH14 filed
- [x] **Residential glue** and its shadow (merged bc7ffa0): in game since run 86. The replacement runs clean for 39 game years (run 89) and saves and reloads (runs 88, 91); the shadow matches the original on 12 of 15 daily states, every grid and scalar (the proximity states differ, RG23); the original's EQ/HQ path (run 90b) and ours (run 89) decline the same way
- [x] GD/BD/TD spec questions answered in the named specs (8fdea4e)
- [ ] Seeding rules (foundations §6.3) and the daily growth budget calibrated in the original (the growth probe lane is running)
- [ ] Glue, and the growth statistics distribution matches across seeded runs (M4 exit)

**M5: traffic**
- [x] Specs: traffic simulator, traffic network, pathfinder
- [x] Traffic network core (merged dd240c9: 50 unit tests, 20 conformance vectors for all twelve §11 tests; §4.14 and §6 persistence wait for the traffic simulator). Twenty spec questions filed as TN1–TN20
- [x] Pathfinder core (merged 36ababe: 28 `pf11_*` vectors, PF1–PF15 filed)
- [x] **Traffic simulator core** (merged a11eb7d: all 15 §11 tests as unit tests and 29 `ts11_*` vectors; TS1–TS34 filed)
- [ ] Traffic glue, calibration; commute and volume maps match (M5 exit)

**M6+: beyond simulation**
- [x] Specs: game data formats (DBPF, exemplars, tuning catalogue, 601f3a9), lot/occupant save records (0460f3a), city save records validated against 417 saves (e69009f)
- [x] `engine/formats` matches spec §11 for DBPF, QFS and LTEXT (merged b5bdf8a; the sweep over real game data shows 0 failures across 22 .dat files and 1,004 saves)
- [x] `exemplar.rs` matches data-formats §4, §5 and the §11 exemplar rows (merged 64a25d2): the install sweep loads 9,347 of 9,347 exemplars and re-encodes 9,345 binary records byte-identically. EX1–EX20 filed
- [ ] Rendering, UI, audio, region; a standalone executable

**Cross-cutting**
- [x] Interface/ABI reference for implementers: allocators, layouts, direct calls ([docs/abi/](docs/abi/))
- [ ] SDK errata (289 findings) re-checked against upstream gzcom-dll and reported (needs the owner's OK)
- [ ] In-game manual checklist ([docs/manual-checks.md](docs/manual-checks.md))

The rest of this README covers the **reverse-engineering workspace** that produces those specs. That material is
for spec writers only. It contains no game binaries or assets, and decompiled output is never committed.

## Start here

| Topic | Doc |
|---|---|
| The engine core: agent scheduler, clock, message protocol | [docs/sim-engine.md](docs/sim-engine.md) |
| Sim speed, tunables, save format, budget, neighbours, weather | [docs/subsystems/clock-budget-neighbors.md](docs/subsystems/clock-budget-neighbors.md) |
| R/C/I demand graph, population census, residential EQ/HQ | [docs/subsystems/demand-rci.md](docs/subsystems/demand-rci.md) |
| Growth: desirability, tract selection, lot scoring, abandonment | [docs/subsystems/development.md](docs/subsystems/development.md) |
| Traffic: trip generation, A* routing, congestion feedback | [docs/subsystems/traffic.md](docs/subsystems/traffic.md) |
| Grids: SimGrid, land value, pollution, power, water, crime, aura, fire/police, flora | [docs/subsystems/grid-sims.md](docs/subsystems/grid-sims.md) |

Each subsystem doc labels its claims **[confirmed]** (read in code) or **[inferred]**, and gives Windows
addresses alongside the matching named Mac Rev A function.

## Data

- [re/symbols_windows.tsv](re/symbols_windows.tsv) and [re/symbols/*.tsv](re/symbols/): hand-verified
  Windows address → name mappings, each with its evidence.
- `re/names_windows.tsv` (generated): every name applied to the Windows exe, with source and confidence.
- [re/class_id_table.tsv](re/class_id_table.tsv): the exe's built-in ID→name table (648 class, message,
  service and command IDs).
- [re/tuning/](re/tuning/): simulator tuning exemplars and the Demand exemplar graph, extracted from the game data.
- [re/prior_art.md](re/prior_art.md): community knowledge (hard-coded addresses, interface IDs, vtable orders).

## Tooling

- `tools/symport/`: matches Windows functions to the debug-symbol Mac build.
  - Stage A: rare 32-bit constants and strings.
  - Stage B: call-graph propagation.
  - Stage C: vtable slot alignment, including the GCC→MSVC/ICC destructor and overload rules.
  - Stage D: ordered callee alignment.
  - It also detects identical-code-folded functions.
- `ghidra/scripts/`: headless scripts that apply names (with evidence as plate comments) and export
  per-namespace decompilation.
- `tools/setup_ghidra.ps1`: builds the Ghidra projects from your own executables and exports the analysis tables.
- `tools/build.ps1`: the full pipeline (match → merge → apply → export to `decomp/`).
- `tools/dbpf.py`, `tools/extract_tuning.py`: read-only DBPF/exemplar reader and tuning dumper.
- `tools/x86dis.py`, `tools/class_vtables.py`: quick disassembly and vtable discovery from a CLSID.

## Rebuilding locally

Everything is built from your own copy of the game. Nothing decompiled is downloaded or committed.

1. Install Ghidra 12.1.x (`C:\tools\ghidra_12.1.4_PUBLIC`), JDK 21+, Python with `pefile` and `capstone`.
2. Copy `SimCity 4.exe` to `bin\SimCity4.exe`.
3. Clone [0xC0000054/sc4-ghidra-symbols](https://github.com/0xC0000054/sc4-ghidra-symbols) to
   `external\sc4-ghidra-symbols` (type archives and Function ID databases).
4. Run `.\tools\setup_ghidra.ps1 -Windows`. It imports the exe into `ghidra\project\FreeCity4`, runs
   auto-analysis with the Function ID databases attached, and exports the analysis tables to `ghidra\export\windows`.
5. Run `.\tools\build.ps1 -SkipMatch`. It applies the committed names (`re\names_windows.tsv`), labels and types,
   then exports the decompilation to `decomp\`.

To verify a project built elsewhere without touching `ghidra\project` or `decomp\` (for example a scratch project
from `setup_ghidra.ps1 -ProjectRoot`):

```bash
.\tools\build.ps1 -SkipMatch -ProjectRoot <projectDir> -DecompOut <outDir>
```

Expected output of step 5 on a fresh `-Windows` project (verified 2026-10-13, Ghidra 12.1.4):
`renamed=15859 created=8 missing=76 failed=191 thiscallConverted=10145`, `labels=856 vtablesTyped=853 failed=0`,
`classes=1023 methods=9258`, `fieldsRenamed=235 filled=105`, and 1815 decompilation files. The counts are benign:
`missing` are auto/library names whose address has no function in this project; the 191 `failed` are all
"a symbol named `_lua_…` already exists at this address" — the Function ID databases already applied the identical
name, so none is a real conflict (0 of the 191 differ from the name we tried to set). All 906 hand/community names
in `re\names_windows.tsv` apply cleanly.

Optional, only to re-run the symbol port or to read the named Mac code: get the Mac "Rev A" Intel executable
(Aspyr's Mac port, which kept its debug symbols) from the
[SC4 Maxis files](https://community.simtropolis.com/sc4-maxis-files/) archive on Simtropolis, run
`.\tools\setup_ghidra.ps1 -Mac <path>`, then `.\tools\build.ps1` (with matching).

## Caveats

- Decompiled output is Ghidra pseudocode, not original source.
- The Windows build used the **Intel C++ Compiler 6.0** (`-Qvc6`, VC6 runtime). Class layouts differ from the Mac/GCC
  build, so Mac struct layouts are never applied to Windows `this` pointers.
- Automatic names are reported with a confidence level. Bodies of 12 bytes or less are usually folded getters and are marked `low`.
