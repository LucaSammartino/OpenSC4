| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetui7: the budget panel `cSC4BudgetUIWinProc` completed (BW18), owner claimed (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`) | **ready** (docs only). budget-window-bodies.md §11: construction order and the 18 balance-sheet labels (monthly/yearly strings, the yearly set never used), the rest of `Init` (72 notification targets, period toggles), the loan slider's limits on open and on redraw (the redraw drops the bond-limit cap), the loan rows and totals, the small slots and the dirty byte; budget-window.md §7 clarified; BW18 answered; `cSC4BudgetUIWinProc` row claimed: specified +18,290 bytes (+24 functions) | — |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`) and the
Ghidra export of the owner, whose bodies of `InitBalanceSheet`, `DisplayLoanWindow` and `UpdateLoanDisplay` are
stack-mangled, so every pair and branch below was read on the instructions. Written on 2026-10-23.

### What changed

- docs/specs/budget-window-bodies.md §11 (new):
  - §11.1 construction (`0x007658A0`) and `InitBalanceSheet` (`0x00764DF0`), with the label/string pairs checked
    at `0x00764E14`–`0x007652EB`;
  - §11.2 the rest of `Init` (`0x00766E10`);
  - §11.3 the title and slider limits of `DisplayLoanWindow` (`0x00765F5C`–`0x007660C6`) and the panel centring
    (`0x00766465`–`0x0076650F`);
  - §11.4 `UpdateLoanDisplay`'s slider and rows (`0x00766928`–`0x00766B99`);
  - §11.5 `IsVisible`, `Update`, the deleting destructor, and `DoMessage`'s dirty byte (`0x00768641`).
- The §8, §10 and inventory entries that said "summary, BW18" now point at §11. budget-window.md §7 states which
  frame its `+0x192` is in, and §8 points at §11.
- docs/specs/questions: BW18 moved to answered/budget-window.md with its answer; QUESTIONS.md regenerated.

### Quirks found

- The balance-sheet labels are set once, at construction, while the period is monthly. The yearly label strings
  are never shown.
- Redrawing the loan list sets the slider's maximum to the whole remaining borrowing room. Opening the window had
  capped it at the bond limit.
- A borrowed total above the limit gives the open path a negative slider maximum.

### Coverage

- tools/coverage-map.tsv: new row `cSC4BudgetUIWinProc` → budget-window.md, budget-window-bodies.md, before the
  `cSC4BudgetUI*` catch-all. Specified bytes 4,358,675 → 4,376,965 (78.0% → 78.4%, `cov_local.py`, on `5c0ff782`).

### Not covered

`cSC4SubBudgetWindow`'s BW17 audits (another owner, already claimed by its row).
