| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reachgen29 suite reach and lifecycle drivers; kind engine | Codex; reachgen29 worktree; detached HEAD, reserved branch reachgen29; code dbc3dd617 | WIP; suite-verified 7.324638% -> 7.893120% (+0.568482 points, +3 target unmet); pytest 498; cargo 7882; clippy 0; ctest 124/124; ABI unchanged; no seams/questions | 1. Repair mature-city lifecycle instrumentation. 2. Exercise remaining formats/overlays/player drivers. 3. Replay fixed census and promote held queue entry. |

- Census acceptance: froze newest completed sh28-census3 as reach29-before (179 runs); 82 exact-key owners had 278088 unexecuted implemented bytes.
- Ranking/delta acceptance: tools/suite_reach_gaps.py and 6 tests; full owner results in ../ops/reach29-owner-results.tsv, detailed findings in ../ops/reach29-results.md.
- Drivers acceptance: four suite.tsv reach29 rows, dated scripts and calibrated Lua zoom/scroll plus native lighting; reach_lifecycle=0 by default; no engine algorithms or ABI exports changed.
- Lifecycle qualification PARTIAL: empty Tempelhof clean/end=1, 19/19 actions, 1667 functions on 3146-entry exact-owner roster; full-roster exploratory empty run reached 10747 functions.
- Mature lifecycle BLOCKER: r1 and r2 each had 3 corrupt mature-city runs; same-DLL lifecycle-off controls all game-clean. This is instrumentation, not an attributed engine defect.
- Shared queue acceptance: 14 requests total, maximum 4 in flight; no Documents writes, saves or third-party DLLs committed.
- Qualified actions: reach29-control house/Tegel each clean/end=1, 5/5; growth clean/end=1, 19/19; reach29-views-qualified clean/end=1, 18/18; shadows remain nonexact.
- Exploratory distinction: reach29-r1, reach29-r2, rejected TakeSnapshot controls and reach29-views-final retained separately; snapshot returned status 2 and was removed.
- Measured before/after: python tools/coverage.py --suite reach29-before / reach29-complete printed suite-verified 409033 bytes (7.3%) / 440779 bytes (7.9%); same 5584344-byte denominator.
- Final union acceptance: before 179 plus 9 measured additions; failed rows retained. build-runs/reach29-final-delta holds metrics.json, owners.tsv, functions.tsv, mismatches.tsv, coverage.md.
- Per-owner result: 133820 newly executed bytes under baseline-exact keys; 117757 retain exact keys; newly failing keys remove 86011 old verified bytes; net +31746 bytes / +0.568482 points (target +3 NOT met).
- Largest exact gains: AutomataScriptSystem 22778; ResidentialSimulator 9627; BudgetSimulator 6959; AdvisorSystem 6416 bytes. All 82 owner verdicts and gains are committed in the TSV.
- Engine findings, not fixed: lotdev r2 views line 3806 (48 mismatches); pollution line 3152 (recycled 5156.035380870104/5155.97802734375); regionalcity line 3611 (14 mismatches).
- Lifecycle-off reproductions: control views lotdev 4161, pollution 3497, regionalcity 3967; neighbours 4427 has 4 water/garbage import/export mismatches. Full first lines are in mismatches.tsv.
- Tools gate: python -m pytest tools/tests -q printed 498 passed in 491.40s. Initial 20s junction-test timeout retried alone: 1 passed in 0.84s; intermediate full run: 497 passed.
- Required qbuild.py --wt <WT> all ran once: test exit=0 passed=7882; clippy exit=0 diagnostics=0; ctest PASS 124/124; qbuild PASS. Tree-change note was scenario/docs qualification; harness C++/Rust stayed unchanged.
- Real-input CLI: suite_reach_gaps --baseline reach29-before --after reach29-additions printed after_verified_bytes=440779, gain_points=0.5684821708691299; committed owner TSV matches all 82 measured rows.
- Commits: 048951ce5 ranking/tests; a1e998591 harness/scenarios; 522cb5329 reduced roster; dbc3dd617 qualified scripts/results. Queue held WIP, kind engine because plugin harness changed; supervisor publishes detached tip.
- Next: resolve early-trace corruption before enabling mature lifecycle; remaining formats/overlay gaps and the +3-point target are unfinished. Questions: none.
