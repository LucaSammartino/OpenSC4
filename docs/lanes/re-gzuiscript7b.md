| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzuiscript7b: the UI-script COM director and the control set (`cGZUIScriptCOMDirector`, `cGZScriptableControlSet`), ui-script-service.md §7–§8 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzuiscript7) | **ready** (docs only). §7: the four classes the director registers and its start-up registration of the service; §8: the control set's layout, interfaces and 13 + 5 slots; owners claimed: specified +2,131 bytes (+33 functions) | UIS1 (hit dispatch vs the legacy control) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 33 functions were read in the decompile; the set's `QueryInterface`, the hit dispatch and the legacy control's
area slots were checked on the instructions. The vtables `0x00AD5248` and `0x00AD5234` were read. Written on
2026-10-24.

### What changed

- ui-script-service.md §7 (the director) and §8 (the control set); docs/specs/questions/ui-script-service.md (UIS1);
  QUESTIONS.md regenerated.

### Quirks found

- The set's hit dispatch calls an element's area slots as "get rectangle" and "handle point", which for the legacy
  control are `SetArea(rect)` and the one-argument `GetArea(rect)` (UIS1).
- `Dispatch` stops calling elements after the first false but keeps iterating.

### Coverage

- tools/coverage-map.tsv: new rows `cGZUIScriptCOMDirector` and `cGZScriptableControlSet` → ui-script-service.md,
  before `cGZUIScript*`.
- Specified bytes 4,749,082 → 4,751,213 (+2,131, 85.0% → 85.1%, `cov_local.py`).

### Not covered

- The interpreter and scope classes; `cGZWinCustom` beyond its constructor.
