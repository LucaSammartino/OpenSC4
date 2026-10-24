| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-traffic-drift10: isolate train188 traffic drift | Codex; glue-traffic-drift10 | in progress | Inspect train changes, isolate observer perturbation, gate and two house runs |

## Handoff
- Goal: fix intermittent traffic shadow drift introduced by train188, without weakening comparisons.
- Worktree reserved, detached HEAD at 91265c0c9; do not switch branches.
- Diagnostic engine tree projected exactly to merge 28472c808 (construction included, later GZMsg/DBPF excluded); git diff --quiet 28472c808 -- engine confirmed equality.
- Diagnostic checkpoint d16290ae6: engine exactly at construction merge; later changes temporarily removed.
- Native verify qbuild-20261024-093759-build-ctest PASS: build 255/255, CTest 80/80. Logs build-runs/fc4verify-{build,ctest}.log. Tree-changed note is the checkpoint commit during build, engine contents unchanged.
- Next: collect traffic-drift10-b28472-a verdict using build-runs/wait-verdict.ps1 (no qwait), inspect traffic and all house shadows. Then select next bisect engine tree.
- Required final validation: qbuild all, coordinator verify, two clean house runs through 2032 with zero traffic resyncs.
- Game request outstanding: traffic-drift10-b28472-a, turbo house to 2032-01-01, seconds 1200, timeout 1500. Initial wait failed before watching because game-results directory was missing; directory now exists, same request remains valid. Diagnostic source overlay only, no fix or questions yet. Restore engine to 91265c0c9 via apply_patch after bisect.
- House profile copied from main build-runs/regress-train188-shadows.ini, excluding autoload path; turbo to 2032-01-01.
- Original failures: train188 on 2031-02-21 trip splits/3 trip-length cells plus 1 ULP car sum; recheck188 on 2028-06-19 destination map/freeDest after a suspended search. Both resync next day.
- DBPF observe gates before service lookups when key off; tick returns immediately on empty pending. Construction change retains field QI through replay and canonicalizes references before indexing.
