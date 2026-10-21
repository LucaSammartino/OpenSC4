| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-slider-ownership4: BW15 `+0x2C` SliderInfo department refcount from native bytes | Claude / .claude/worktrees/re-budget-slider-ownership4 | in progress | byte-audit slider-row append in on_init group branch |

## Handoff
- Goal: BW15 in docs/specs/budget-window.md §4 (after BW14 block, ~line 225): +0x2C SliderInfo creation/push/grow AddRef count, clear/shutdown/dtor Release order vs +0x38; conformance tests in §9.
- Done: worktree, lane file.
- Next: re_query asm of on_init slider-row loop after 0x00788242 (non-empty branch), find push_back into +0x2C.
