| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-traffic-drift10: isolate train188 traffic drift | Codex; glue-traffic-drift10 | in progress | Inspect train changes, isolate observer perturbation, gate and two house runs |

## Handoff
- Goal: fix intermittent traffic shadow drift introduced by train188, without weakening comparisons.
- Worktree reserved, detached HEAD at 91265c0c9; do not switch branches.
- Diagnostic engine tree projected exactly to merge 28472c808 (construction included, later GZMsg/DBPF excluded); git diff --quiet 28472c808 -- engine confirmed equality.
- Diagnostic checkpoint d16290ae6: engine exactly at construction merge; later changes temporarily removed.
- Native verify qbuild-20261024-093759-build-ctest PASS: build 255/255, CTest 80/80. Logs build-runs/fc4verify-{build,ctest}.log. Tree-changed note is the checkpoint commit during build, engine contents unchanged.
- Run 478 traffic-drift10-b28472-a clean through 2032-01-01: traffic 71/71 exact, 0 resyncs; automata/owners 73/73 including load; landvalue 72/72, all checked house shadow lines exact. 805.234 real seconds, DLL SHA256 65670de3d51694f5c69d59fae5037c2fbc9c17fdef1c78942341f3a3366847b4.
- Next: project engine forward to GZMsg merge 435df94b5 (before DBPF), verify, run same house profile. One clean bisect run does not exclude intermittent regression.
- Required final validation: qbuild all, coordinator verify, two clean house runs through 2032 with zero traffic resyncs.
- No game request outstanding. Run evidence build-runs/game-results/traffic-drift10-b28472-a.{log,verdict}; run record docs/runs/traffic-drift10-b28472-a.md. Diagnostic source overlay only, no fix or questions yet. Restore engine to 91265c0c9 via apply_patch after bisect.
- House profile copied from main build-runs/regress-train188-shadows.ini, excluding autoload path; turbo to 2032-01-01.
- Original failures: train188 on 2031-02-21 trip splits/3 trip-length cells plus 1 ULP car sum; recheck188 on 2028-06-19 destination map/freeDest after a suspended search. Both resync next day.
- DBPF observe gates before service lookups when key off; tick returns immediately on empty pending. Construction change retains field QI through replay and canonicalizes references before indexing.
