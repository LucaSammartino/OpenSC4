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
- qbuild all terminal PASS: native build 262/262 edges, CTest 81/81; logs fc4verify-build.log and fc4verify-ctest.log. Native queue qbuild-20261024-144618-build-ctest completed. Tree-change note is docs-only commit 3d1728678; engine source unchanged since 69aeea81b.
- Next: collect flora12-20261024-r1 house-city turbo verdict to 2032, inspect exact owner lines and ref warnings.
- Forbidden tools/ coverage maps and regress profile will remain coordinator-owned.
- Game request about to submit: flora12-20261024-r1; default Konradshohe, enable=simulator, flora_shadow=1, turbo to 2032-01-01. File-event wait helper: build-runs/wait-flora-verdict.ps1 (no qwait).
