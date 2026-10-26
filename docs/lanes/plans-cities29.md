| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| plans-cities29 / nettool_plan, powerline_plan (engine) | Codex / .claude/worktrees/plans-cities29 / detached HEAD; reserved branch plans-cities29; commits c279c8150 through aced409df; ABI unchanged; no seam reservation | continue; full gate PASS: Rust 7896, clippy 0, CTest 124/124; r3b game runs clean, ref_warnings=0, shadows inexact; NG15 filed | 1. Coordinator review reading-scope incident. 2. Resolve dense-city inputs and NG15. 3. Fresh broad sample and house regress, then enqueue only when accepted. |
- Reading-scope incident: recovery opened unrelated legacy build-runs/scan.py and rd.py, containing analysis helper code; neither was run or used as a network behavior source. Coordinator review required before landing.
- Changes retained: ordinary surface-run planes, specified earthwork precision, powerline zero-pole wrapper validity; this round adds dense-origin blocker counts and multi-cell terrain/corner diagnostics without changing checks.
- Full gate PASS: build-runs/qbuild-test.log (7896/0), qbuild-clippy.log (0 diagnostics), fc4verify-build.log (49/49), fc4verify-ctest.log (124/124); native label qbuild-20261026-133325-build-ctest. Focused geometry 60/60; fmt 9 owned Rust files passed.
- Corrected diagnostic DLL bc6e69f5: MAIN/build-runs/game-results/plans-cities29-r3b-{aquaton,lamburg}.{log,verdict}, both clean/ref_warnings=0; Aquaton N/P=2/3,2/3; Lamburg N/P=0/8,0/8. No requests remain outstanding.
- Dense origin evidence: Aquaton dry=3713 empty=12626 clear=970; Lamburg dry=3552 empty=583 clear=535 (later 533); first occupant type=74758926. Fixed 17-cell L footprint has no origin; this is availability coverage, not unreadable terrain.
- Old DLL 096d8930 runs r3-{spandau,aquaton,lamburg} are separate evidence: all clean/ref_warnings=0; N/P=4/7,7/7; 2/3,2/3; 0/45,0/45 respectively. Spandau window ends 2002-03-01; new multi-cell terrain logging was absent.
- NG15 in docs/specs/questions/network-geometry.md blocks the nonflat multi-cell Spandau refusal/cost case: exact cross-section pairs, diagonal flag and constraint ordering require specification; inferred mapping retained.
- Census3 -> earlier plans-cities29-2 exact-line counts below are N/P; the latter includes partial/invalid runs and an older DLL, so these are diagnostic counts, not suite acceptance. Logs: MAIN/build-runs/run7/census3 and WT/build-runs/suite/plans-cities29-2.
- berlin-konradshohe: 4/4,4/4 -> 4/4,4/4.
- berlin-tempelhof: 1/1,1/1 -> 1/1,1/1.
- timbuktu-rush-hour: 3/5,5/5 -> 4/4,4/4.
- timbuktu-big-city: 1/4,4/4 -> 1/1,1/1 (stalled).
- maxisland-aquaton: 2/3,2/3 -> 2/3,2/3.
- maxisland-downtown-weburg: 0/3,2/3 -> 0/1,0/1 (stalled).
- maxisland-lamburg-villa: 0/4,0/4 -> 0/1,0/1 (exited).
- gen27-konradshohe-s2-churn: 10/25,25/25 -> 20/20,20/20 (invalid).
- gen27-spandau-s1-default: 4/25,25/25 -> 4/11,11/11 (invalid).
- gen27-tegel-s0-default: 5/25,23/25 -> 1/1,1/1 (exited).
- gen27-tegel-s1-churn: 4/25,25/25 -> 1/1,1/1 (exited).
- london-sydenham: 4/4,4/4 -> 4/4,4/4.
- empty-small: 4/4,4/4 -> 4/4,4/4.
- Missing: all distinct census cases classified/resolved, reach29 family reruns, a fresh >=12-scenario exact sample across region/gen27/Lua, and fresh house regress. No credit flip or merge entry requested.
- Next: reconstruct the specified occupied/demolition inputs needed by dense cities, or establish an evidence-based availability correction; preserve every vector and comparison field.
- Next: rerun Spandau on the diagnostic DLL after the build verdict, collect public terrain/corners and resolve NG15; recheck earthwork residual cases using e08638e14's restored product rounding.
- Next: run the required broad/house checks on the corrected source, rerun all only after a fix, then produce the engine merge-queue entry and request coordinator enqueue. MAIN=C:/Users/tlori/FreeCity4; WT is this worktree.
