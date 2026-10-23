| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-fix5: isolate netocc shadow | Codex / glue-netocc-fix5 | in progress | Resolve requested merge, reproduce CTest failures, fix and verify city through 2032. |

## Handoff
- Goal: merge glue-netocc-shadow5 and repair plugin exit/write-count regressions; retain exact 73/73 city shadow.
- Requested merge started; conflicts in FC4Config.cpp and FC4Observer.cpp need additive resolution.
- Next: resolve conflicts, commit merged source, run qbuild verify to reproduce.
- Detached HEAD is intentional; never switch or update refs.
- No game request outstanding.
