| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| services-exact33 engine: terrain startup and water-grid contract | Codex / services-exact33 / reserved services-exact33 (detached HEAD) | wip; code 28c6d1a2e, 9e0fca9e4, 43ff9bcba; ABI exports unchanged; existing seam 59 extended; cargo 8409, clippy clean, CTest 146/146; terrain qcheck 0/2, none 2/2; manual shutdown corrupt/ref=0; questions none | 1. Fix terrain's first Jinae difference and shutdown lifetime. 2. Rerun terrain and none. 3. Continue the other ten owners, smallest difference first. |

Code: restore terrain misc/altitude/vertex records during Read, retain the saved view, publish saved dimensions before Init, and write all seven terrain records.
Code: copy draw vertices only within each change notification's rectangle, avoiding full-city copies for small levelling updates.
Code: return a retained game-created uint16 SimGrid from GetBodyOfWaterMap, rebuild labels at established-city PostCityInit, and provide the city to host queries.
Regression: existing terrain seam tests non-default saved dimensions before Init, retention through Init, and byte-exact seven-record round trips; focused CTest 1/1 passed.
Final gate: qbuild all PASS; cargo 8409 passed/0 failed, clippy 0 diagnostics, CMake 5/5, CTest 146/146. Logs are in this worktree's build-runs. No Rust file or C ABI export changed.
qcheck-1: exit 2, both references unavailable during pin refresh. qcheck-2: terrain exit 1, 0/2 exact. qcheck-3: none exit 0, paradis-jinae and ui29-volcano both exact. No owner newly exact; no replacement credit claimed.
Terrain not exact, paradis-jinae: first delivery 2047-04-06 #26, reference vt 0x00AB127C; first DIGEST 2047-05-01 income/expense; 114 records differ, 59 relabel-only, IDs shift at 519/class 896E75AF; first content 096E75F5/299B2D1B/0.
Terrain not exact, ui29-volcano: first delivery 2026-01-28 #95, reference vt 0x00AB2214; first DIGEST 2026-02-01 income/expense; 117 records differ, 46 relabel-only, IDs shift at 7/class A98F9D28; first content 098F964D/299B2D1B/0 (occmgr).
Both replacement qcheck runs are clean/ref_warnings=0. Results: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-015850-ae981585/result and compare-20261028-020038-63394f09/result under the same golden-runs root.
Manual services33-terrain-load-1 stalled with budget unintentionally enabled; excluded as a terrain-only check. services33-terrain-load-2 stalled with only terrain enabled; watchdog samples identified full-city SyncDrawVertices work.
Manual services33-terrain-load-3 passed loading after the rectangle fix, then crashed in WaterPathFinder::FindPath at 0x007251A7 because the body-map result was a raw array instead of a grid object.
Manual services33-terrain-load-4 reached 2047-06-06 after the grid fix, then crashed during shutdown in OverlayManager::MarkChanged at 0x00737D92 reading null, after terrain Shutdown destroyed the core. All manual runs had ref_warnings=0; the shutdown crash remains unresolved.
Manual services33-terrain-original-1 completed the same Jinae script clean/ref_warnings=0. Manual quit/shutdown failure and clean qcheck results are separate observations; neither clean verdict proves exactness.
The ten owners below were not rerun or changed; their first differences are the coordinator's supplied starting results, not fresh comparisons against the new pin.
Power not exact: paradis-berner delivery 2039-10-07 #5, vt 0x00AB2214; first content 098F964D (occmgr), 11 differing records.
Pollution not exact: ui29-bevopolis first A9BD882D, 1 record; paradis-stohess first 8A22BEBE, 3 records.
Landvalue not exact: ui29-weburg first 8BA98EDA, 1 record.
Budget not exact: maxisland-wrenton first E990BFFC (DepartmentBudget), 1 record/ID relabelling; reach29-growth was exact in the supplied baseline.
Ordinance not exact: reach29-growth first 00D0723D (CommunityCPRTraining), 20 records.
Parks not exact: paradis-zoe first 2977AA47, 4 records.
Clock24 not exact: timbuktu-rush-hour first 6A0F82B2, 5 records.
Demolition not exact: gen28-tegel-s2-default delivery 2029-08-30 #21, vt 0x00AB1B88; DIGEST 2029-09-01 funds/income/expense; first content 096E75F5, 120 records.
Dispatch not exact: maxisland-norte-delano first 098F964D (100 records); ui29-volcano first 2977AA48 (4); timbuktu-big-city first 896E75AF (1).
Tutorial not exact: timbuktu-rush-hour delivery 2010-05-03 #5, vt 0x00AB2214; first content 098F964D, 100 records; ui29-weburg was exact in the supplied baseline.
Next: inspect Jinae's first differing delivery before later shifted save IDs, compare the queried terrain/water answers with the spec, and fix the manual overlay teardown lifetime; rerun qcheck after each fix. Then resume the remaining owners. Questions filed: none. Queue is WIP, never ready.
