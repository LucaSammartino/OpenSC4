| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-region27 / RegionalCity replacement (engine) | Codex / replace-region27 / detached HEAD | Ready: 47e43b854, 09959aa61, 84a9dcb2b; ABI unchanged; cargo 7877/0, clippy 0, CTest 120/120; house2 clean through 2032, shadow 73/73, ref_warnings=0; Tegel neighbour roads and save clean; questions none | - |

## Handoff
rc23-wip was already merged before this round (ancestry verified); main conflict resolution is 07ffb56e4.
47e43b854 supplies the native COM owner, host-owned record elements, cache/preferences/read callbacks and connection reconciliation outside the Rust state lock.
09959aa61 fixes sparse/empty border-mask linkage using ZoneRegionBuffer and registers default-off `regionalcity` (CLSID 0x6A037C24).
84a9dcb2b identifies the record's acquired connection references as specified last-holder releases after simulator shutdown (regional-city sections 4.1, 7.2); over-release auditing remains enabled.
Focused quick passed: cargo 664/0; native 49/49; regionalcity owner and border contract executables passed.
Full qbuild all PASS: cargo 7877 passed/0 failed/0 ignored; clippy 0 diagnostics; native 55/55; CTest 120/120.
Gate request: qbuild-20261026-060323-build-ctest; logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
(a) region27-r4-house2: house replacements + regionalcity + mysims clean through 2032-01-01, ref_warnings=0; both full saves ok=1.
Evidence (a): C:/Users/tlori/FreeCity4/build-runs/game-results/region27-r4-house2.log and .verdict (verdict line 2; saves log lines 12183, 17684).
(b) Same run, key on: regionalcity shadow 73/73 exact, 0 resynced; each observation decoded 85 records and compared 16093 fields.
Evidence (b): C:/Users/tlori/FreeCity4/build-runs/game-results/region27-r4-house2.log:17770; .verdict shadow totals; factory registration log line 175.
(c) region27-r4-tegel: Berlin Tegel clean through 2029-09-01, ref_warnings=0; 2 existing road connections to neighbours; full save ok=1; shadow 2/2 exact.
Evidence (c): C:/Users/tlori/FreeCity4/build-runs/game-results/region27-r4-tegel.log:7886 (roads), :8829 (save), and .verdict (line 2).
Supplement: region27-r4-spandau clean, ref_warnings=0, full save ok=1, shadow 2/2; no neighbour road, so Tegel supplies criterion (c).
Superseded region27-r4-house reached 2032 with shadow 73/73 but ref_warnings=6; the specified cached-connection final releases were corrected in 84a9dcb2b and remeasured in house2.
Corrected game runs used DLL SHA256 prefix cd709950, through the shared main game queue; no request outstanding.
No C ABI exports changed; no questions filed; coverage and the regress replace list were not edited.
regionalcity is ready for the replace list. No implementation work remains.
