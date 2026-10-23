| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-fix5: isolate netocc shadow | Codex / glue-netocc-fix5 | in progress | Resolve requested merge, reproduce CTest failures, fix and verify city through 2032. |

## Handoff
- Goal: merge glue-netocc-shadow5 and repair plugin exit/write-count regressions; retain exact 73/73 city shadow.
- Merged requested branch in fd035d87; resolved config/observer conflicts additively.
- Original merged source qbuild verify PASS: build 221/221, CTest 69/69 (193.5 s), request qbuild-20261023-230531-build-ctest.
- Train157 failures did not reproduce in this fresh build; do not claim a confirmed source-level crash cause.
- Next: isolate variant support in a netocc-only capture subclass, restoring shared WriteCapture layout; run final qbuild all.
- Detached HEAD is intentional; never switch or update refs.
- No game request outstanding.
