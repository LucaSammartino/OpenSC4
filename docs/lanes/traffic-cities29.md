| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| traffic-cities29: traffic across cities (engine) | Codex; traffic-cities29 worktree; detached HEAD, reserved branch traffic-cities29 | continue; code 3d30e734a, 4c91cf56a, 3bab274bc; ABI unchanged; focused shadow tests 13/13; fmt 4/4; full gate interrupted before tests; game untested; TC29-STATS-1/2 | 1. Retry gate after build capacity recovers. 2. Run sample below. 3. Resolve or question remaining cases; house regress; enqueue only after acceptance. |

Fix: replay WorkOnSimCycle's intersection prelude when the original's deadline stops after it with zero phase progress; rebuild capacities, retain all record/grid checks.
Test: zero-progress stops before/after the prelude; added capacity assertions. Native binary built during the first code milestone: 13/13 traffic shadow tests pass (tool stdout); initial wrong filter ran 0 tests and is not counted.
Formatting: qbuild fmt passes all four owned Rust files. Latest capacity assertions still need the full gate's fresh build.
Gate: qbuild all invoked ONCE; waited for a cargo slot (3 active runs) through 2421 s, then interrupted with Ctrl-C; exec session 90849 collected terminal exit 1. No cargo/clippy/CMake/CTest PASS/FAIL stages ran. Retry after shared build capacity recovers.
Infrastructure: the queued gate blocked later shell/edit calls; interrupting it released them. No game request submitted and no rebuilt DLL claimed. Context wind-down: guard measured 130861/200000 after the first handoff commit.
Questions: TC29-STATS-1 (working switch-stat entry/reset/publication), TC29-STATS-2 (published trailing floats between deliveries), docs/specs/questions/traffic-simulator.md. Computation unchanged, gaps marked in state.rs.
Before counts below are inexact traffic lines / all traffic result lines; after counts are PENDING, not zero. Evidence: main build-runs/run7/census3/<scenario>.shadows.txt; parser build-runs/traffic-census.py.
berlin-konradshohe 7/7; berlin-spandau 7/7; berlin-tegel 7/7 -> PENDING.
gen27-konradshohe-s0-default 0/24; gen27-spandau-s1-default 49/49; gen27-tegel-s0-default 49/49 -> PENDING.
lua-lots-empty 3/3; lua-lots-grown 5/5 -> PENDING.
paradis-dreyse 3/3; paradis-ehrmich 3/3; paradis-reiss 3/4; timbuktu-big-city 7/7 -> PENDING.
Broad sample baseline: 12 scenarios, 1 exact, 11 inexact. New broad sample and house regress NOT RUN; other distinct census cases remain untriaged until replay fix is measured.
Next: after a completed full gate, run main tools/suite.py run --set all with the twelve scenarios above, --profile shadows --dll this worktree --keys traffic --tag traffic-cities29-r2-1 --slots 3, using the shared queue.
Next: compare counts, inspect remaining cases against specs, file narrow questions rather than guesses; house regress and clean/ref_warnings=0 proof remain required. No merge-queue entry or enqueue while continuing.
