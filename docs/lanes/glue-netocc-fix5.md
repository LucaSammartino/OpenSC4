| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-fix5: isolate netocc shadow | Codex / glue-netocc-fix5 | in progress | Resolve requested merge, reproduce CTest failures, fix and verify city through 2032. |

## Handoff
- Goal: merge glue-netocc-shadow5 and repair plugin exit/write-count regressions; retain exact 73/73 city shadow.
- Merged requested branch in fd035d87; resolved config/observer conflicts additively.
- Original merged source qbuild verify PASS: build 221/221, CTest 69/69 (193.5 s), request qbuild-20261023-230531-build-ctest.
- Train157 failures did not reproduce in this fresh build; do not claim a confirmed source-level crash cause.
- Variant support is now a netocc-only capture subclass; shared WriteCapture constructor/data layout and variant rejection match main. No new global objects.
- Netocc requires its ini key; absent/off key stores no city pointer and never captures, including when the old environment variable is set.
- Isolation committed as 7ca6b5fe; final qbuild all running (shell session 19033); do not duplicate it.
- Final workspace tests PASS 6653/0/0; clippy PASS, zero diagnostics.
- Native request qbuild-20261023-231436-build-ctest outstanding; collect same shell operation, then city through 2032.
- Detached HEAD is intentional; never switch or update refs.
- No game request outstanding.
