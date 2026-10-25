| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetdlg8: the budget income and expense dialogs that `ShowBudgetWindow` opens, budget-dialogs.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-layoutview8) | **ready** (docs only). Two owners claimed: specified +4,032 bytes (+19 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly (the decompile misnames the 64-bit multiply and divide helpers); the
expense row table and the constants were read in the PE. Written on 2026-10-25.

### What changed

- docs/specs/budget-dialogs.md (new): §1 opening by the panel flag, the script windows, closing and layouts; §2 the
  income dialog (tax rows 1/4/10, refresh, slider moves, totals written to the budget); §3 the expense dialog (the
  seven-department row table, init, changes, totals, applying the funding); §4 quirks; §5 conformance tests.
- docs/specs/budget.md: the `ShowBudgetWindow` row points to the new spec.

### Coverage

- tools/coverage-map.tsv: rows for both window procedures before the `cSC4BudgetUI*` catch-all.
- Specified bytes 5,101,168 → 5,105,200 (+4,032, 91.4%, `cov_local.py` with owners-c1).
