# FreeCity4

Reverse engineering of **SimCity 4 Deluxe 1.1.641** (GOG Windows exe,
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`), starting from the
simulation engine and working toward a full, named decompilation.

You need your own copy of the game. This repo contains analysis, tools and derived notes. It does not
contain game binaries or assets.

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
- `tools/build.ps1`: the full pipeline (match → merge → apply → export to `decomp/`).
- `tools/dbpf.py`, `tools/extract_tuning.py`: read-only DBPF/exemplar reader and tuning dumper.
- `tools/x86dis.py`, `tools/class_vtables.py`: quick disassembly and vtable discovery from a CLSID.

## Rebuilding locally

1. Install Ghidra 12.1.x (`C:\tools\ghidra_12.1.4_PUBLIC`), JDK 21+, Python with `pefile` and `capstone`.
2. Copy `SimCity 4.exe` to `bin\SimCity4.exe`.
3. Clone [KillaVolt/SimCity-4-Deluxe-GOG-Decompiled](https://github.com/KillaVolt/SimCity-4-Deluxe-GOG-Decompiled)
   to `external\sc4-decomp` and pull its `sc4-ghidra-state` LFS package. Extract the
   `sc4-windows-analysis`, `sc4-mac-symbol-analysis` and `sc4-ghidra-state` packages into `external\extracted\`.
4. Clone [0xC0000054/sc4-ghidra-symbols](https://github.com/0xC0000054/sc4-ghidra-symbols) to `external\`.
5. Copy `external\extracted\sc4-ghidra-state\artifacts\ghidra\deep\BaseGameAnalysis.{gpr,rep}` to
   `ghidra\project\FreeCity4.{gpr,rep}` and set the owner in `project.prp` to your user name.
6. Run `.\tools\build.ps1`.

## Caveats

- Decompiled output is Ghidra pseudocode, not original source.
- The Windows build used the **Intel C++ Compiler 6.0** (`-Qvc6`, VC6 runtime). Class layouts differ from the Mac/GCC
  build, so Mac struct layouts are never applied to Windows `this` pointers.
- Automatic names are reported with a confidence level. Bodies of 12 bytes or less are usually folded getters and are marked `low`.
