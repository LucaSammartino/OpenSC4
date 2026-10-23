| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetwin: the budget panel's balance-sheet drawing, formatting helpers, taxes and loan windows, folded into budget-window-bodies.md and budget-window.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-plop) | **ready** (docs and RE tables only). budget-window-bodies.md §4.2 rewritten (the complete control mapping), §4.3 (the two formatting helpers), §8 (construction, `Init`, notifications, the period and hide buttons), §9 (the taxes window) and §10 (the loan window) added; §4.1 and budget-window.md §5.2 corrected; `cSC4BudgetUIWinProc` claimed as specified; BW17 narrowed, BW18 filed; 42 names (4 new, 38 renamed). Specified coverage +18,290 B / +24 functions | BW18 (construction labels, `Init`, loan limits and rows), BW17 (the sub-budget remainder) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as a separate `budget-window.md` and held back when `main`
gained its own budget-window.md and budget-window-bodies.md (re-panels4, re-budgetwindow4 and the BW rounds, which
cover the sub-budget dialog in more depth than the held lane); this is the held lane's panel work folded in.

### What changed

- budget-window-bodies.md §4.2 (rewritten): `UpdateBalanceSheetInfo` writes every control: funds, income, expense and
  balance per tab, the sign colours, and the sixteen parts with their monthly and annual fields. Correction:
  `+0x198` is the period (set by the period buttons at `0x00768B40` / `0x00768C51` through the `+8` view) and the
  byte `+0x194` the tab; the earlier text had the two roles swapped (`0x007679E5`, `0x00767A29`, `0x00767E5F`). The
  yearly path sets no colour.
- §4.3 (new): `SetFormattedAmountInTextControl` and `PrintPercentageString` (`"%d%s%d"`, the locale's separator).
- §4.1, corrected: the group record's `+24` is the expense and `+32` the income (SDK `BudgetGroupInfo`, budget.md;
  the dispatch at `0x007675C0`–`0x00767780`), so `0x0A5A72D1` shows its income, `0xAA369059` income then expense, the
  other groups their expense. The offsets and destinations were right.
- §8 (new): construction and `Init` (summary), `HideWindow`, `RegisterMessageNotification`, the period buttons
  `0x4A8879CC`–`0x4A8879DD`, the hide buttons `0x4A5BB1B5` / `0x6A5BB0FB` (the panel's slot 7).
- §9 (new): the taxes window: start values, live `SetTaxRate`, the edit parse with its remapped nine-entry fallback
  table and the range test that never rejects, half-to-even rounding, Enter, OK and Cancel, the close after the
  modal loop.
- §10 (new): the loan window: the counters, the slider, scrolling, accept (a `LoanInfo` record, then
  `IssueNewLoans` issuing every bond added since opening), close.
- budget-window.md: §2.1 lists the hide and period buttons; §5.2 rewritten (primary offsets; the call after
  `UpdateLoanDisplay` is `IssueNewLoans`, not a refresh); §8 notes the narrowed BW17; tests 28–32.
- Kept upstream where it was right: the ordinance selector also has `0xAA3AC509` (`0x00768CB3`; the held spec missed
  it), and upstream's sub-budget sections stand as they are.

### Names and coverage

- re/symbols/budget-window.tsv: 42 names (4 new: `RestoreTaxRates`, `CloseTaxesWindow`, `IssueNewLoans` and
  `ApplyDealEdits`, the name upstream's text already used for `0x0077AB40`; 38 replace auto names). The
  `names_windows.tsv` delta (−38 +42) is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: `cSC4BudgetUIWinProc` → budget-window.md and budget-window-bodies.md (it fell to
  `cSC4BudgetUI*`, unspecified); the `cSC4SubBudgetWindow` row now cites both files.

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-plop 3,452,142 bytes in 15,300 functions;
with this lane 3,470,432 bytes in 15,324 functions (+18,290 B, +24 functions; denominator unchanged). The held
lane's +91,558 B included `cSC4SubBudgetWindow`, which upstream has since claimed. `docs/coverage-report.md` was not
regenerated (the tool accepts only the GOG exe).

### Questions

- Narrowed: BW17 (the panel's loan, tax, formatting and balance-sheet rendering bodies left its list).
- Filed: BW18 (the construction and `InitBalanceSheet` labels, the rest of `Init`, the loan limits and rows).
- The held lane's QB1–QB4 are settled or carried over: QB4 (the service `0x00B43CE0` is the window-location saver;
  slots 6 and 7 are `ShowWindow` and `HideWindow`) by upstream; QB2 and QB3 by upstream's BW rounds; QB1's styling
  part by §4.2, its layout part out of scope as upstream rules.

### Not covered

BW18's summary parts; the sub-budget remainder of BW17.
