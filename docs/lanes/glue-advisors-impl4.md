| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-impl4: GZCOM glue for the whole advisor hierarchy (enable=advisors), implementation round 1 | glue-advisors-impl4 / .claude/worktrees/glue-advisors-impl4 | in progress | see Handoff |

## Handoff
- Goal: replace the advisor hierarchy (advisors.md §14.2) behind `enable = advisors`; window via original
  ctor+Init (§12.1.1) through FC4Compat; host callbacks, GZCOM classes, game-layout maps (+0x4C/+0x70).
- Done: worktree from main 604bcc05.
- Next: read spec §12.1.1, §12 slot tables, capi advisors.rs host struct; decide this round's stage.
