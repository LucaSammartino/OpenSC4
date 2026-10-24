| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: dispatch shadow | `Codex` / `glue-mysimdispatch-shadow9` (engine lane) | in progress; contracts under inspection | 1. Implement settled dispatch comparisons. 2. Gate and measure to 2032. |

## Handoff
- Goal: default-off mysimdispatch_shadow; independently compare settled dispatch contracts through 2032 with house switches.
- Worktree reserved at detached HEAD 67a23d0fb; commit normally, supervisor publishes branch.
- Done: inspected settled contracts and public SDK; implemented default-off constructor/header-only shadow, private independent Rust comparator, monthly capture and field counts.
- Next: focused tests/format, stable full gate and queued native verify, house run to 2032. Actual live-trip replay is blocked by missing native callable/input-observation ABI (MS20).
- Compared scope: fresh native constructor state/previous/flags/minutes/travel, class/version; discoverable live serialized class/version only. Native outputs never initialize expected state. This does NOT verify the whole owner.
- Filed MS20 in docs/specs/questions/mysims.md. Coordinator must regenerate QUESTIONS index (tools forbidden here).
- MS18 remains partial: explicitly inventory uncompared methods; do not infer missing native signatures.
- Milestone cdaea5883: scoped shadow implemented. Focused fc4-capi comparator tests pass 2/2. q.py could not start Git Bash (mapping access denied); direct cargo worked. Applied explicit fmt fix and calendar-month deduplication.
- Gate terminal PASS: stable qbuild all cargo 6986/6986, clippy diagnostics=0; queued native qbuild-20261024-084135-build-ctest PASS build 254/254 edges, CTest 79/79. Logs build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Owned Rust fmt PASS.
- Run 469 mysimdispatch-shadow9-20261024-ctor01 terminal clean but UNACCEPTABLE: stopped 2028-04-19 at seconds=300, ref_warnings=73, foreign_exceptions=0, mysimdispatch 27/27 exact (7 constructor fields each, no live dispatch headers). Evidence retained in docs/runs/mysimdispatch-shadow9-20261024-ctor01-469.md and build-runs/game-results/.
- Diagnostic source correction 9e3c6df24: add missing FC4_ACQUIRED factory audit registration before FC4_RELEASE_CREATED; load date from public simulator instead of unset PublishedJulian. Corrected qbuild all terminal PASS: cargo 6986 passed, clippy diagnostics=0; native qbuild-20261024-085949-build-ctest PASS build 207/207 edges, CTest 79/79. No further source changes required by gate.
- New game request mysimdispatch-shadow9-20261024-ctor02, same house ini with seconds=1500 timeout=1800, turbo to 2032. Await build-runs/game-results/mysimdispatch-shadow9-20261024-ctor02.verdict with one FileSystemWatcher operation (build-runs/wait-mysimdispatch-verdict.ps1). Do NOT discard a completed verdict, resubmit, or launch a duplicate request. Reserve next run number 470 after checking local docs/runs.
