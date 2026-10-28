| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-exact33 — engine: zone manager exact | Codex / growth-exact33 / detached HEAD (reserved branch growth-exact33); code 1d923e9c1; ABI exports unchanged, no seam reservations | ready for zones only; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; zones and none exact 1/1 each, clean/ref_warnings=0; G33-CM1 filed | 1. Land zone save fix. 2. Resolve G33-CM1. 3. Continue the nine unresolved owners below. |

- Changed ZoneManager::Write to return GetError()==0, as zones §10.1 requires; setter returns no longer prevent record completion.
- Before: payload already identical, but false Write left the length/CRC header zero (6 differing bytes) and shifted the class-index ID once. After: every record exact.
- Regression covers false reference-setter returns with a good stream, identical bytes/all six references, and a nonzero stream error.
- Final full gate: qbuild all PASS; cargo 8410/0/0, clippy -D warnings 0 diagnostics, CMake 3/3 edges, CTest 146/146. Owned-file fmt passed.
- zones: NEWLY EXACT, qcheck-5 exit 0, cover=timbuktu-rush-hour; original zone manager is replaced. No shadow credit claimed.
- zones run: golden-6723ed9ebca448c4ac14baefcc63a8d2, compare-20261028-022640-8755633a/result; clean, ref_warnings=0, Write -> 1.
- --replace none: qcheck-6 exit 0, timbuktu-rush-hour exact; golden-eef94f8490e94038923b6b78ff931a6c, compare-20261028-022848-889eecb9/result; clean, ref_warnings=0.
- commercial: NOT EXACT, qcheck-1 and qcheck-3 exit 1 on timbuktu-rush-hour: same 49 differing vehicle bytes in 896E714A/299B2D1B/0, including checksums and four entries' float values; all other records/digests/sampled RNG agree. qcheck-2 none exact. No commercial code changed.
- Commercial evidence: compare-20261028-020554-ca3c0b7f/result and compare-20261028-020949-f9fa67d3/result, both clean/ref_warnings=0. Cause remains unspecified; G33-CM1 in docs/specs/questions/commercial-industrial.md requests the missing observable contract.
- The following owners were not changed or rerun; their first differences are the coordinator's supplied baseline, not new measurements:
- residential: NOT EXACT; gen28-konradshohe-s2-grow first delivery 2026-02-07 #21, ui29-bevopolis 2257-04-20 #3381; both first content occmgr 098F964D/299B2D1B/0.
- development: NOT EXACT; gen27-spandau-s2-default first delivery 2001-09-19 #21, first content 096E75F5/299B2D1B/0; reach29-growth was exact in the supplied baseline only.
- demand: NOT EXACT; gen28-konradshohe-s2-default first delivery 2026-05-02 #21, first content 096E75F5/299B2D1B/0.
- zonedev: NOT EXACT; gen28-tegel-s2-default first delivery 2030-04-30 #21, first content occmgr 098F964D/299B2D1B/0.
- lotdev: NOT EXACT; gen27-spandau-s2-default first shifted ID 4 class A98F9D28, first content occmgr 098F964D/299B2D1B/0; ui29-volcano one differing record C9BD5D4A/299B2D1B/0.
- lotcfg: NOT EXACT; gen28-tegel-s1-default first delivery 2029-08-30 #21, first content occmgr 098F964D/299B2D1B/0.
- propmgr: NOT EXACT; maxisland-north-forks first shifted ID 21 class 6A26D049, first content occmgr 098F964D/299B2D1B/0.
- exemplars: NOT EXACT; ui29-weburg first delivery 2197-04-12 #21, first content occmgr 098F964D/299B2D1B/0.
- Run directories above are under C:/Users/tlori/FreeCity4/build-runs/golden-runs; local qcheck-N.log, qbuild-test.log, qbuild-clippy.log and fc4verify-ctest.log are in this worktree's build-runs.
- No unlanded branches merged; no game files committed; no C ABI exports changed.
