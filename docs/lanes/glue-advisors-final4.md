| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-final4: advisor replacement (enable=advisors), final round | glue-advisors-final4 / .claude/worktrees/glue-advisors-final4 | in progress | see Handoff |

## Handoff
- goal: enable=advisors full replacement (system/advisor/advice/subject, window direct calls AD4, 47 Lua entries AD5/AD6), game trace 2032 ref0.
- done: be5b96d0 compat Direct entries (5 win proc AD4 + 11 Lua entries AD5/AD6), AdvisorSystemGlobal(), FC4AdvisorWindow.h (slots 23-28 + global slot), advisor_window_test.
- next: baseline qbuild all (background, build-runs/baseline.txt), then qbuild all on tip, final report. Full system/host (64 callbacks, 95 slots, Lua loader) does not fit this round (ctx 86k at call ~30).
- facts: final logical round (3/3); no 4th round.
