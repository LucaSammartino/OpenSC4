| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetwindow-answers4: answer budget-window BW5 and BW10 (RE spec lane) | fc4-lane agent, .claude/worktrees/re-budgetwindow-answers4 | ready: BW5 answered statically (bit 0x4 = Alt), BW10 a/b/c answered (DIFFERS), BW11 filed (runtime delivery of Alt+Enter/Alt+F4) | core lane: reconcile core-budgetwindow-fix4 with BW10 DIFFERS (+0x10 content window, +0x38 built for empty group, initial arrow hides) and add tests 14-17; game lane: BW11 |

## Report

Static RE only (re_query.py export + exe bytes via build-runs/va.py); no REA calls were needed.

### What changed (docs/specs/budget-window.md)
- §4 "Content window +0x10" (BW10a): on_init creates a GZCOM window (class 0x4386D516 / IID 0x5386D516), keeps its
  cIGZWin at +0x10 with a reference, sets generator and window flags, SetID(0x0423278E), ChildAdd to the dialog, in
  every mode before the selector branch (0x00787A9E-0x00787BD5). Core said "holds nothing": DIFFERS.
- §4 "Order and the empty group" (BW10b): +0x38 is filled per group department inside the counting loop, before
  the empty test; empty branch (SetEmptyWindow) skips +0x2C. Core said "builds neither": DIFFERS for +0x38.
- §5.3 "Initial arrow state" (BW10c): 0x451/0x453 hidden always, 0x452/0x454 hidden when count <= 9; empty branch
  builds no arrows. Core said "sets no arrow": DIFFERS.
- §4.0 (BW5): GZ Win32 translator FUN_0098D230 routes WM_KEYDOWN and WM_SYSKEYDOWN to the type-5 builder
  FUN_0098C400; mask from FUN_0098B6E0 via GetKeyState: 0x1 Shift, 0x2 Ctrl, 0x4 Alt (0x0098B702), 0x10/0x20/0x40
  mouse buttons, 0x40000000 repeat. Data words {VK, mask, repeat count}. MATCHES the Alt guess, now proven.
- §8 updated; §9 conformance tests 14-17 added.

### Questions
- BW5, BW10 moved to questions/answered/budget-window.md; QUESTIONS.md reindexed.
- BW11 filed (questions/budget-window.md): does Alt+Enter / Alt+F4 reach the dialog at runtime, and does Alt+F4
  also trigger app quit or system close? Hook loop 0x0098D2B6 and accelerators not traced.

### Not covered
- Ordinance and neighbour init arrow bounds (copies at 0x0077D620 and 0x00781AD1) not byte-checked.
