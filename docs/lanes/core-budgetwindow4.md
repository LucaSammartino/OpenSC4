| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow4: pure-Rust cSC4SubBudgetWindow core (budget-window.md) | fc4-lane agent, .claude/worktrees/core-budgetwindow4 | in progress | see Handoff |

## Handoff

- Goal: host-independent core for cSC4SubBudgetWindow (+ the panel's DoSubBudgetWindow/DoMessage parts it needs),
  engine/core/src/budget_window/, typed host trait, tests from spec §9. No plugin/CMake/CAPI/ABI edits; no game run.
- Done: baseline all PASS (3756 tests, clippy 0, ctest 63/63); core module engine/core/src/budget_window/ (mod, ids, dialog, panel, tests: 21 tests) + questions BW6-BW9 committed; quick PASS.
- Next: full gate `qbuild all`, then final report + merge-queue json.
- Facts: questions file docs/specs/questions/budget-window.md, next free id BW6.
