| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-core-final4: resolve BW13 (on_init +0x10 create failure) and BW14 (+0x38 department refcount) | Claude / .claude/worktrees/re-budget-core-final4 | in progress | see Handoff |

## Handoff
- Goal: answer BW13/BW14 in docs/specs/budget-window.md section 4 from native bytes (on_init 0x007876B0, on_shutdown 0x0077E440, dtor).
- Done: worktree at main d91cb866.
- Next: read asm of on_init failure branch and 0x00787F8B clear loop.
