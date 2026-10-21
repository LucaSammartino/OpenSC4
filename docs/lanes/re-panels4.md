| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-panels4: windows and panels RE spec (budget window family) | `re-panels4` / `re-panels4` (RE lane) | in progress | see Handoff |

## Handoff

- Goal: spec the largest SC4-specific panel family by bytes: `cSC4SubBudgetWindow` (71 KB) + `cSC4BudgetUI*`
  (~19 KB) -> new docs/specs/budget-window.md. Not generic widgets (re-widgets4 owns those).
- Done: lane file.
- Next: inventory methods (re_query func/vtable), read lifecycle + refresh + event routines, write spec.
- Facts: ui-windows.md §14.3 lists sizes; coverage-map rows 361-362 (`cSC4SubBudgetWindow`, `cSC4BudgetUI*`).
