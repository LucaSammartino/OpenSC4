# Reverse-engineering workspace

RE/coordinator reference; engine lanes must not use it ([clean room](../CONTRIBUTING.md)).
The [RE prompt](../.claude/workflows/fc4-lane.js) owns scope, evidence rules and deliverables.

## Data

- [re/symbols_windows.tsv](../re/symbols_windows.tsv) and [re/symbols/*.tsv](../re/symbols/): hand-verified
  Windows address → name mappings, each with its evidence.
- `re/names_windows.tsv` (generated): every name applied to the Windows exe, with source and confidence.
- [re/class_id_table.tsv](../re/class_id_table.tsv): built-in class, message, service and command IDs.
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
- `tools/re_query.py`: read-only queries over the Ghidra export (function, asm, callers, callees, xrefs, strings,
  names, vtables); see [Querying in parallel lanes](#querying-in-parallel-lanes).
- `tools/x86dis.py`, `tools/class_vtables.py`: quick disassembly and vtable discovery from a CLSID.
- In-game harness: [coordinator reference](ops/running-the-game.md).

## Querying in parallel lanes

Run `python <MAIN>/tools/re_query.py` against `ghidra/export/windows`, `re/names_windows.tsv` and
`docs/abi/vtables.tsv`. Queries are read-only and need no server/lock. Tables cache atomically in the main
checkout's `build-runs/re-query-cache/`, refreshed when source size/mtime changes.

```bash
python <MAIN>/tools/re_query.py func 0x5B9FEB cSC4PathFinder::cSC4PathFinder
python <MAIN>/tools/re_query.py asm 00407bc0 --max 56 --skip 0
python <MAIN>/tools/re_query.py callers 009457c6  # also callees, xrefs [--from], vtable <class|addr>
python <MAIN>/tools/re_query.py --limit 20 strings 'demand'  # also search <name-regex> [--all]
```

Lists default to `--limit 40`; asm pages default to 56 lines (`--skip N`, `--bytes`). `--count` returns counts;
`--full` lifts caps. Addresses accept hex (with/without `0x`), `FUN_xxxxxxxx`, `SimCity 4.exe+0x...`, or a name (exact, else a
unique regex match). `vtable <function>` lists every vtable slot that points at that function. `asm` on bytes that
Ghidra did not decode falls back to capstone (`tools/x86dis.py`).

REA is only for missing pseudocode, using the prompt's two allowed tools. Lanes share one target; report errors
and fall back to export/decomp as instructed. Do not open, close or replace the shared target from a lane.

Coordinator recovery: one `rea.mjs mcp` server holds one target/JVM; separate server processes import independently.
Warm the target once; do not interrupt the roughly 3-minute initial import. `close_binary` closes it for all lanes,
and opening another file replaces it. "Ghidra client is already started" indicates a cancelled/timed-out startup
left runtime state (`%TEMP%/rea-ghidra-*`). `REA_GHIDRA_STARTUP_TIMEOUT_MS` defaults to 330 s.
At a quiet moment the coordinator may close once and reopen; never loop recovery calls.

## Rebuilding locally

Coordinator setup, from your own game copy:

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

Inspect `missing` names and `failed` renames: an identical name already applied by Function ID is benign;
a conflicting name requires review.

Optional, only to re-run the symbol port or to read the named Mac code: get the Mac "Rev A" Intel executable
(Aspyr's Mac port, which kept its debug symbols) from the
[SC4 Maxis files](https://community.simtropolis.com/sc4-maxis-files/) archive on Simtropolis, run
`.\tools\setup_ghidra.ps1 -Mac <path>`, then `.\tools\build.ps1` (with matching).

## Caveats

- Decompiled output is Ghidra pseudocode, not original source.
- The Windows build used the **Intel C++ Compiler 6.0** (`-Qvc6`, VC6 runtime). Class layouts differ from the Mac/GCC
  build, so Mac struct layouts are never applied to Windows `this` pointers.
- Automatic names are reported with a confidence level. Bodies of 12 bytes or less are usually folded getters and are marked `low`.
