# FreeCity4

An **open-source (GPL-3.0-or-later) reimplementation of the SimCity 4 engine**. It is built the way OpenRCT2 was:
original code replaces the game piece by piece, verified against the real game, until it no longer needs the
original executable. Target: **SimCity 4 Deluxe 1.1.641** (GOG Windows exe, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`). You need your own copy of the game.

- How it works and the roadmap: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- How to contribute (clean-room rules): [CONTRIBUTING.md](CONTRIBUTING.md)
- Specs for engine authors: [docs/specs/](docs/specs/)

## Progress

Last updated 2026-10-12. "Core" means the Rust logic, with its conformance vectors, in `engine/core`. "Glue" means
the plugin code in `engine/plugin` that runs the core inside the real game. Milestones are defined in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#roadmap).

**M0: toolchain**
- [x] CMake project, Rust core, C ABI, conformance runner (364 vector files)
- [x] `FreeCity4.dll` loads in SC4 1.1.641 and replaces `cSC4Simulator`
- [x] Unattended in-game runs: autoload a city, autotest verdict, monthly observer (`tools/ingame_run.ps1`)

**M1: the clock**
- [x] Simulator core and spec
- [x] Runs in-game: days, weeks and months advance; limit-timer calibration
- [ ] Save → reload verified in-game
- [ ] Pause, emergency and all speeds verified in-game (manual checks)
- [ ] Differential log against the original on the same save

**M2: scalar sims**
- [x] Budget, demand, ordinance and weather cores
- [x] Budget, demand, ordinance and weather glue (budget in-game with no heap corruption, runs 17–18)
- [ ] x87 precision: the game expects 24-bit; something in our process resets it (guard on a branch, not yet merged)
- [ ] Budget differential: tax by type (types 4, 6, 9 differ) and non-tax income
- [ ] Budget and demand panels match the original after N months (M2 exit)

**M3: grid sims**
- [x] Cores: SimGrid, land value, pollution, crime, aura, flammability, power, plumbing, protection, flora
- [x] Land value glue (off by default)
- [ ] Power glue (in-game without crashing, but production stays 0; on a branch)
- [ ] Plumbing, protection, flora, pollution, crime, aura and flammability glue
- [ ] Grid snapshots match within tolerance (M3 exit)

**M4: growth**
- [x] Specs: building development, growth, tract, lot and zone developers
- [x] Cores: building development simulator, growth developer, tract developer
- [ ] Cores: lot developer, zones
- [ ] Open spec questions answered ([docs/specs/QUESTIONS.md](docs/specs/QUESTIONS.md))
- [ ] Seeding rules (foundations §6.3) and the daily growth budget calibrated in the original
- [ ] Glue, and the growth statistics distribution matches across seeded runs (M4 exit)

**M5: traffic**
- [x] Specs: traffic simulator, traffic network, pathfinder
- [ ] Cores, glue, calibration; commute and volume maps match (M5 exit)

**M6+: beyond simulation**
- [ ] Specs: save records, game data formats (DBPF, exemplars), tuning catalogue (in progress)
- [ ] Formats, rendering, UI, audio, region; a standalone executable

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

Optional, only to re-run the symbol port or to read the named Mac code: get the Mac "Rev A" Intel executable
(Aspyr's Mac port, which kept its debug symbols) from the
[SC4 Maxis files](https://community.simtropolis.com/sc4-maxis-files/) archive on Simtropolis, run
`.\tools\setup_ghidra.ps1 -Mac <path>`, then `.\tools\build.ps1` (with matching).

## Caveats

- Decompiled output is Ghidra pseudocode, not original source.
- The Windows build used the **Intel C++ Compiler 6.0** (`-Qvc6`, VC6 runtime). Class layouts differ from the Mac/GCC
  build, so Mac struct layouts are never applied to Windows `this` pointers.
- Automatic names are reported with a confidence level. Bodies of 12 bytes or less are usually folded getters and are marked `low`.
