| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| region-exact33 - engine: regionalcity, neighbors | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/region-exact33; detached HEAD, reserved branch region-exact33; merge d3a11560b, code/comment tip 1bd6bd593, questions 9a8ede807; retained af415eb50, 51f302734, 7edfd67ec | ready; regionalcity 3/3 exact, neighbors 2/2 exact, none 5/5 exact; cargo 8412 passed, clippy 0 diagnostics, CTest 147/147, fmt 6/6; ten clean runs/ref_warnings=0; shadow n/a; ABI +2 retained C exports, no slot/seam change; questions none | - |

- Round 7 merged main's canonical adjacent-city schedule (fingerprint bde008a451b1c7fb). The existing x-then-z walk and reverse Init clone traversal already match the clarified contract; its compatibility comment is updated. No lane-authored FC4Schedule.cpp/h edits.
- Retained implementation: STLport population-summary ABI (51f302734), PC24 commodity operations (7edfd67ec), and current-game host binding for region-owned deals, including synchronous re-entry coverage (af415eb50).
- **regionalcity exact:** qcheck-27 exit 0, 3/3: owner27-resources, owner27-views, paradis-karanes, with regionalcity alone enabled.
- **neighbors exact:** qcheck-28 exit 0, 2/2: maxisland-indereast, paradis-ragako, with neighbors alone enabled.
- **Original-only controls exact:** qcheck-29 exit 0, 5/5 on the same five scenarios with --replace none.
- Previous Karanes byte-6734 and Indereast byte-11881 differences in CA027EDB/CA027EE1/0 were complete-deal permutations from native pointer walks. Canonicalizing the original's adjacency list resolves them; no first difference remains in either owner's covered scenarios.
- Full gate: qbuild all PASS, run once; 8412 Rust tests, clippy -D warnings with 0 diagnostics, native build 10/10 edges, CTest 147/147. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Owned-file fmt: 6/6 PASS. Initial focused qbuild quick: 6437 tests passed and plugin built; final full gate used the committed source.
- Tested DLL SHA256: ead76365c1e79ab611b7e07904f6f8f11f04f615323d885e154ce73fec026feb.
- Regionalcity labels: golden-b46e221f1c184534a4f8d11d24f47321 (resources), golden-063a2800fecb42b2a5ac62cb2eefc823 (views), golden-8d89039f52db42e5bcb4146115118f68 (Karanes).
- Neighbors labels: golden-450d03de47494a45b905c9687f76aa27 (Indereast), golden-fdebe09c5502404ea14bbf687bb5d361 (Ragako).
- Control labels: golden-c0d5deb677524ca19d8ecd0d41132ff1 (Indereast), golden-812acf58a08d4ee2bbff41e0dc8f1f96 (resources), golden-33554d989cfa4b27bcf0568dbe31465e (views), golden-23fe29aae2f8497294d52552fe047aa9 (Karanes), golden-f44f759c25cb408baa7070e6d5296e3f (Ragako).
- All ten runs: clean, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Replacement factory activation is confirmed in game.log. No shadows run; exactness comes from qcheck.
- Evidence: build-runs/qcheck-27.log, qcheck-28.log, qcheck-29.log; build-runs/check-round7-evidence.py lists each main build-runs/golden-runs result directory, comparison, label, and harness verdict.
- ABI relative to main: fc4_neighbor_deal_bind_host and fc4_neighbor_deal_clone_into remain added exports from earlier rounds; no ABI-version bump or GZ slot change. No additional export this round.
- Removed answered RY-EXACT33-1 (docs/specs/questions/regional-city.md) and NB-EXACT33-1 (docs/specs/questions/neighbors.md), including follow-ups. No new questions or behavior-spec edits.
- No remaining work for these owner scenarios or unresolved schedule request. glue-regionalcity-replace23 was not merged because no difference remains for it to remove.
