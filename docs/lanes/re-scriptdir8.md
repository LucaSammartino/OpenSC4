| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scriptdir8: the script COM director, the interpreter's constructor and the script vocabulary, script-com-director.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-gzw32-8) | **ready** (docs only). One owner claimed: specified +1,474 bytes (+23 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the decompile and the constructors in the disassembly; the vtables were read in the PE.
Written on 2026-10-25.

### What changed

- docs/specs/script-com-director.md (new): §1 the singleton director and its two classes; §2 the interpreter's
  constructor (state, text buffer, the bottom vocabulary); §3 the vocabulary (case-insensitive name → object table,
  its eight slots); §4 conformance tests.

### Coverage

- tools/coverage-map.tsv: a `cGZScriptCOMDirector` row before the `cGZ*COMDirector` catch-all.
- Specified bytes 5,114,150 → 5,115,624 (+1,474, 91.6%, `cov_local.py` with owners-c1).
