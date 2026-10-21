| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow4: pure-Rust cSC4SubBudgetWindow core (budget-window.md) | fc4-lane agent, .claude/worktrees/core-budgetwindow4 | in progress | see Handoff |

## Handoff

- Goal: host-independent core for cSC4SubBudgetWindow (+ the panel's DoSubBudgetWindow/DoMessage parts it needs),
  engine/core/src/budget_window/, typed host trait, tests from spec §9. No plugin/CMake/CAPI/ABI edits; no game run.
- Done: worktree created from main e6ab506d.
- Next: write engine/core/src/budget_window/{mod.rs,tests.rs}, add `pub mod budget_window;` at end of core lib.rs list.
- Facts: questions file docs/specs/questions/budget-window.md, next free id BW6.
