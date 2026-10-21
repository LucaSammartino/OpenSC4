| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow-fix4: apply BW1-BW4/BW6-BW9 answers to core budget_window | fc4-lane agent, .claude/worktrees/core-budgetwindow-fix4 | in progress | see Handoff |

## Handoff
- goal: implement answered BW1-BW4, BW6-BW9 (spec sections 2-6, 9) in engine/core/src/budget_window/; BW5 stays open.
- done: baseline qbuild all 3944 tests, clippy 0 (ctest cargo_test failed only because files changed mid-run); code+tests ccac0724 (quick PASS, fc4-core 2121), BW10 filed.
- next: qbuild all gate, then final report + merge-queue json.
