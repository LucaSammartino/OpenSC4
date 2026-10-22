| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow-ownership4: budget-window.md BW16 row visibility, BW14/BW15 department ownership (+0x38/+0x2C), §9 tests 19-27 | fc4-lane agent, .claude/worktrees/core-budgetwindow-ownership4 | in progress | see Handoff |

## Handoff
- Goal: core only (engine/core/src/budget_window/). BW16 `line_item_shown` = flag bit0 && (id <= 100 || amount != 0);
  department refs: host `add_ref_dept`/`release_dept`; +0x38 pairs and +0x2C SliderInfo each own one ref
  (temp +1, copy +1, temp -1); B build loop makes one slider per shown B row (header + items, 9-row cap shared);
  on_shutdown releases +0x2C then +0x38; release_final releases +0x38 then +0x2C before +0x1C/+0x14/+0x10/+0x0C.
- Done: worktree created at main 511d0ab6.
- Next: baseline `qbuild all`, then code + tests 19-27.
- Facts: only tests.rs Rec implements BudgetWindowHost; no glue/CAPI user.


## Coordinator quota pause (2026-10-22)
Provider weekly limit; resets 18:00 Europe/London. This round stopped. Existing work is preserved as WIP, not ready or verified. Resume only after the provider limit clears and fresh slots/check --resume pass; the round still counts.
