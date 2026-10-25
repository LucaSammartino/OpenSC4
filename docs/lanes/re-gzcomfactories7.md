| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzcomfactories7: the GZCOM class creation functions and class-ID getters left to the `fn:GZCOM_*` catch-all, gzcom-factories.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-drawsys7) | **ready** (docs only). 46 owners claimed: specified +1,272 bytes (+46 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 46 functions were read in the disassembly (allocation size or pool, constructor and its arguments, the returned
face offset; the class-ID constants). The 15 other unspecified `fn:GZCOM_*` owners (Lua console views, effects
parser/manager, HTML window, font renderer, network layout custom view, resource-script file parser) are left to the
catch-all. Written on 2026-10-24.

### What changed

- New docs/specs/gzcom-factories.md.

### Quirks found

- The pooled creation functions do not test the pool refill for failure.

### Coverage

- tools/coverage-map.tsv: 46 `fn:GZCOM_…` rows before the `fn:GZCOM_*` catch-all.
- Specified bytes 4,929,212 → 4,930,484 (+1,272, 88.3%, `cov_local.py`).
