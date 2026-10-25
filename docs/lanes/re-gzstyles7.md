| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzstyles7: the building styles panel (`cSC4BuildingSelectWinProc`), building-styles-panel.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzballoon7) | **ready** (docs only). Layout, `Init`/`Shutdown`/`ShowWindow`, commands, style add/remove and the last-style lock; owner claimed: specified +2,178 bytes (+13 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 13 functions were read in the decompile; `Init`, `ShowWindow` and the style add/remove were checked on the
instructions, and the cGZWin slots 43–46 and 56 were confirmed on vtable `0x00ADC8D8`. Written on 2026-10-24.

### What changed

- New docs/specs/building-styles-panel.md.

### Quirks found

- `Init` passes x and y swapped when it lines up the two windows beside the "every N years" radio button.
- `QueryInterface` answers only `0x22E85D8E` (not IID 1).
- The last style button that is on is disabled, so the panel cannot empty the active style list.

### Coverage

- tools/coverage-map.tsv: new row `cSC4BuildingSelectWinProc` → building-styles-panel.md, before `cSC4*WinProc`.
- Specified bytes 4,764,331 → 4,766,509 (+2,178, 85.3% → 85.4%, `cov_local.py`).

### Not covered

- The panel's script layout; the tract developer itself (tract-developer.md).
