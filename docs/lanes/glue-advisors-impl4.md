| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-impl4: GZCOM glue for the whole advisor hierarchy (enable=advisors), implementation round 1 | glue-advisors-impl4 / .claude/worktrees/glue-advisors-impl4 | in progress | see Handoff |

## Handoff
- Goal: replace the advisor hierarchy (advisors.md §14.2) behind `enable = advisors`; window via original
  ctor+Init (§12.1.1) through FC4Compat; host callbacks, GZCOM classes, game-layout maps (+0x4C/+0x70).
- Done: worktree from main 604bcc05; AD4 filed 14f92796 (window direct-call signatures); map mirrors
  FC4AdvisorMaps.h + ctest advisor_maps_test c1b704d7; AD5 filed a6ff203a (9 UI Lua entries).
- Decision: system not enabled this round: slot 24 needs AD4 (no signature guesses), register_api needs AD5.
- Next: full gate (`qbuild all`) green incl. advisor_maps_test, then final report + merge-queue (state blocked/wip).
- Facts: no game request made (nothing enabled; plugin behaviour unchanged). Baseline gate started run in bg.
