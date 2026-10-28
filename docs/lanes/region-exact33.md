| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| region-exact33 — engine: regionalcity, neighbors | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/region-exact33; detached HEAD, reserved branch region-exact33; code af415eb50 (inherits 51f302734, 7edfd67ec); question 86e5b32b8; two additive C exports, no slot/seam reservations | wip; 0 owners fully exact; regionalcity 2/3, neighbors 1/2; cargo 8411 passed, clippy 0 diagnostics, CTest 146/146; game clean/ref_warnings=0; shadow n/a; RY-EXACT33-1, NB-EXACT33-1 | 1. Resolve both fixed-schedule city-order questions. 2. Apply the specified ordering contract. 3. Rerun each owner alone and none on its same scenarios. |

- Inherited fixes: regional-city population summary uses the game's STLport map ABI (51f302734); neighbor commodity statistics round each operation at PC24 (7edfd67ec).
- New fix: region-owned deals bind a copied host table to the current game's interfaces without requiring a replacement neighbors simulator. This restores Clone's side/type swap, pricing, amount changes, state checks and status reports. Clone identity exists before SetFlags posts a report.
- Added a regression for endpoint/type swaps, status identity, native pricing, state flags, unchanged source and synchronous MessageSend re-entry into getters/setters.
- Final gate: qbuild all PASS once; 8411 Rust tests, clippy zero diagnostics, native build PASS, CTest 146/146. Quick gate: 6437 tests and native build PASS. Owned-file fmt 4/4 PASS.
- **regionalcity not exact:** qcheck-11 exit 1, **2/3**: `owner27-resources` and `owner27-views` exact; `paradis-karanes` differs only in the regional-city record. No simulation-window/DIGEST difference remains.
- Resources' prior 27 differing records, including the first shifted ID 1/class 2A5F877D, are gone. qcheck-9 reproduced that baseline; qcheck-10 and qcheck-11 became exact after the host fix.
- Karanes' prior eleven differing records and income/expense mismatch are gone. Remaining `CA027EDB/CA027EE1/0`: 42 differing bytes, first byte 6734; its two complete 76-byte local deals at offset 6728 are identical as a multiset. Entire prefix and suffix match.
- Karanes actual other-side city order: (20,28), (16,32); reference: (16,32), (20,28). Native neighbors walks replacement regional-city pointers; a deterministic fixed-schedule contract for that walk is missing (RY-EXACT33-1).
- Karanes evidence: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-030636-9d489870/result.
- **neighbors not exact:** qcheck-12 exit 1, **1/2**: `paradis-ragako` exact; `maxisland-indereast` still differs only in `CA027EDB/CA027EE1/0`, first byte 11881, 139 bytes. No simulation-window/DIGEST difference.
- Indereast's seven complete deals are identical as a multiset; prefix and suffix match. Actual city groups (1,6), (0,7), (0,5); reference (0,7), (0,5), (1,6). Current position order follows neighbors §9.1/§14.1; NB-EXACT33-1 requests reconciliation with exactness.
- Indereast evidence: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-031059-6228e784/result.
- Final original-only qcheck-13 exit 0, **5/5 exact**, with the final DLL: maxisland-indereast, owner27-resources, owner27-views, paradis-karanes, paradis-ragako. Both remaining differences are replacement-specific.
- Replacement labels: golden-152f4a616c9d4268ba332fc3916c8e78 (resources), golden-eb2adeaded8d4625ae54e2dc34c9a0d0 (views), golden-a3e7aab37f2b40acbc07e6954da578a1 (Karanes), golden-e2fa95769f9a4005b744acab4840dd96 (Indereast), golden-e1d983a9ff8f49ce8c21573f3a781cc3 (Ragako).
- All five replacement runs and all five original-only controls: clean, ref_warnings=0, no foreign exception/stall/RTC/overrun. No shadows run. Startup x87/ignored pacing warnings remain; original-only stays exact.
- New C ABI exports: fc4_neighbor_deal_bind_host, fc4_neighbor_deal_clone_into. No ABI version bump or GZ interface-slot change.
- New question RY-EXACT33-1: docs/specs/questions/regional-city.md. Existing NB-EXACT33-1: docs/specs/questions/neighbors.md. No behavior spec changed; glue-regionalcity-replace23 was not merged.
- Earlier rejected-script/missing-reference attempts and qcheck-5/-6/-7/-8 are superseded by the final checks. This round's Bash-wrapper failure made no source edit; relative fmt arguments failed and were corrected to absolute paths.
- Next: hold WIP, with no owner credit, until the fixed-schedule ordering contracts resolve both remaining save differences; rerun each owner alone and original-only controls afterward.
