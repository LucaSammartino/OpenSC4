# Reverse-engineering workspace

This file covers the **reverse-engineering workspace** that produces the specs in [specs/](specs/). It is for spec
writers only (the in-game harness entry under Tooling is for the coordinator); implementers must not use it (see the
clean-room rules in [CONTRIBUTING.md](../CONTRIBUTING.md)). It contains no game binaries or assets, and decompiled
output is never committed.

## Early RE notes (spec writers only; superseded by docs/specs where they overlap)

| Topic | Doc |
|---|---|
| The engine core: agent scheduler, clock, message protocol | [docs/sim-engine.md](sim-engine.md) |
| Sim speed, tunables, save format, budget, neighbours, weather | [docs/subsystems/clock-budget-neighbors.md](subsystems/clock-budget-neighbors.md) |
| R/C/I demand graph, population census, residential EQ/HQ | [docs/subsystems/demand-rci.md](subsystems/demand-rci.md) |
| Growth: desirability, tract selection, lot scoring, abandonment | [docs/subsystems/development.md](subsystems/development.md) |
| Traffic: trip generation, A* routing, congestion feedback | [docs/subsystems/traffic.md](subsystems/traffic.md) |
| Grids: SimGrid, land value, pollution, power, water, crime, aura, fire/police, flora | [docs/subsystems/grid-sims.md](subsystems/grid-sims.md) |

Each subsystem doc labels its claims **[confirmed]** (read in code) or **[inferred]**, and gives Windows
addresses alongside the matching named Mac Rev A function.

## Data

- [re/symbols_windows.tsv](../re/symbols_windows.tsv) and [re/symbols/*.tsv](../re/symbols/): hand-verified
  Windows address → name mappings, each with its evidence.
- `re/names_windows.tsv` (generated): every name applied to the Windows exe, with source and confidence.
- [re/class_id_table.tsv](../re/class_id_table.tsv): the exe's built-in ID→name table (648 class, message,
  service and command IDs).
- [re/tuning/](../re/tuning/): simulator tuning exemplars and the Demand exemplar graph, extracted from the game data.
- [re/prior_art.md](../re/prior_art.md): community knowledge (hard-coded addresses, interface IDs, vtable orders).

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
- In-game harness (coordinator): `tools/ingame_run.ps1`, `tools/regress.ps1`, `tools/run_summary.ps1`, `tools/sc4ctl.ps1`;
  see [docs/ops/running-the-game.md](ops/running-the-game.md). Implementers work from the coordinator's run results and do not open `tools/`.

## Rebuilding locally

This section rebuilds the RE workspace. To build the engine and the plugin DLL, see
[engine/README.md](../engine/README.md#build-and-test).

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
