| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzcomdir7: the window library's director `cGZWinCOMDirector`, new spec widget-comdirector.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzfolders7) | **ready** (docs only). widget-comdirector.md §1–§5: the director (child director, `GetDirectorID`, `OnStart` adding two system services), its 30-class table with factories, object sizes and returned interfaces, the tree-view and accelerator constructors, and a map of where the owner's 88 widget methods are specified (pie menu, folders, ticker, tooltip manager); owner claimed: `cGZWinCOMDirector`: specified +9,760 bytes (+115 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 115 functions of the owner were read in the decompile; this lane writes the 27 director functions and closes the
owner with the lanes re-gzwinpie7, re-gztipmgr7 and re-gzfolders7, which specified the other 88. A script check
confirmed that every address of the owner appears in one of the five specs. Written on 2026-10-24.

### What changed

- New docs/specs/widget-comdirector.md.

### Quirks found

- None new; the owner is a grab-bag by reach, mapped in §4.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinCOMDirector` → widget-comdirector.md, before the `cGZWin*` catch-all.
- Specified bytes 4,681,654 → 4,691,414 (+9,760, 83.8% → 84.0%, `cov_local.py`).

### Not covered

- The child director's class `0x84174448`, named by ID only.
