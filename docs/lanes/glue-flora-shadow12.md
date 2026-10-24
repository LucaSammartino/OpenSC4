| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-flora-shadow12 (flora owner shadows) | Codex / glue-flora-shadow12 | in progress | Audit owner comparisons, extend shadows, gate and run house city to 2032 |

## Handoff
- Goal: exact same-process evidence for all three flora owners.
- Worktree reused at detached HEAD; initial tree clean.
- Implemented separate static-record and occupant-rule owner lines under existing default-off flora_shadow.
- Static owner excludes registry order checks; occupant owner excludes simulator scalar checks and random appearances/disappearances.
- Missing captured static record now reports INCOMPLETE rather than predicting unchanged.
- Focused capi shadow test passed (1/1), including injected float/state differences and missing-record rejection; explicit fmt passed.
- Source milestone: 69aeea81b.
- Stable-source qbuild all: workspace tests 7212 passed, 0 failed; clippy 0 diagnostics. Logs build-runs/qbuild-test.log and qbuild-clippy.log.
- Native build/CTest outstanding inside SAME qbuild all operation (session 80862): qbuild-20261024-144618-build-ctest. Do not enqueue another verify.
- Next: collect its terminal result; submit unique game request to 2032 after DLL exists.
- Forbidden tools/ coverage maps and regress profile will remain coordinator-owned.
- No game request outstanding. File-event wait helper: build-runs/wait-flora-verdict.ps1 (no qwait).
