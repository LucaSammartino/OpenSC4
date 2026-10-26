| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| views-cities29: view-city shadows (engine) | Codex; .claude/worktrees/views-cities29; detached HEAD, reserved branch views-cities29 | WIP: code d14c99b30, question 755e5aa37; ABI unchanged, no seam reservations; Rust 7901 passed, clippy 0 diagnostics, CTest 125/125; views four-key control 46/46 exact, house 46/46, census 493/493; VC29-PC open | 1. Resolve VC29-PC. 2. Reproduce and fix numeric differences under the broad view-run timing. 3. Revalidate and enqueue kind engine. |

- Changed: consume lot slot 14's inclusive rectangle regardless of unspecified AL; regression covers overlap exclusion and adjacency through the production host fallback (23 insertions, 1 deletion).
- Zone acceptance: views-cities29-5 month 2197-04-01 lists calls=32, mismatches=0, lots=377/377 (baseline 377/420): C:/Users/tlori/FreeCity4/build-runs/game-results/views-cities29-5.log.
- Four-key view control: clean/ref_warnings=0; residential 25/25, zonedev 15/15, lotdev 3/3, regionalcity 3/3 exact; view script scenario-complete and both lighting actions delivered: same .log, .lua.log and .verdict.
- Numeric acceptance remains unproved: faster four-shadow control changes timing. Original broad log enters CW=0x027F before the reported balance/slope/EQ differences; cores implement the specified 24-bit arithmetic. No precision policy or tolerance changed.
- VC29-PC: docs/specs/questions/foundations.md asks whether Lua re-entry must establish simulation PC=24 or pass PC=53 into these cores; blocks residential, lotdev and regionalcity numeric items.
- House acceptance: clean/end=1/ref0, actions 5/5, all four keys exact, 46/46 shadow lines: C:/Users/tlori/FreeCity4/build-runs/suite/views-cities29-house-6/{summary.tsv,per_key.tsv,reach29-house.log}.
- Census acceptance: all four keys 6/6 scenarios exact, 493/493 shadow lines, six clean/end=1/ref0: C:/Users/tlori/FreeCity4/build-runs/suite/views-cities29-census-7/{summary.tsv,per_key.tsv,*.log}.
- Census cases: berlin-spandau, berlin-tegel, london-kensington, london-fulham, timbuktu-big-city, timbuktu-rush-hour. Suite exits 1 for six action refusals: funds 1, blocked 2, network placement 3; details in that directory's scenario JSON files. Shadow results remain exact.
- Tests: focused plugin_ini_enable_zonedev_test 1/1; qbuild all once PASS (Rust 7901, clippy 0 diagnostics, CTest 125/125). Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Distinct failed attempts: requested --set all/--keys command failed measured selection; suite views-cities29-2 exited during demolition shadow (17/18 actions, end=0); views-cities29-3/-4 harness path errors, no game result. These do not supply acceptance credit.
- Enqueue acceptance: NOT met; WIP merge-queue entry only. No merge-train enqueue until VC29-PC and broad numeric validation are resolved.
