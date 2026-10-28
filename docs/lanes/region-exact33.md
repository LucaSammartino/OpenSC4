| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| region-exact33 — engine: regionalcity, neighbors | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/region-exact33; detached HEAD, reserved branch region-exact33; code 51f302734, 7edfd67ec; question 8191c9b4d; ABI/seam reservations none | wip; 0 owners fully exact; regionalcity 1/3, neighbors 1/2; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; game clean/ref_warnings=0; shadow n/a; NB-EXACT33-1 | 1. Resolve NB-EXACT33-1 ordering contract. 2. Diagnose regionalcity first differences, including first shifted ID. 3. Rerun each owner alone and none on its same scenarios. |

- Fixed regional-city load/cache access violation: pass the demand simulator an STLport-layout map with B-allocated header/nodes, then walk and free it through existing game-map helpers.
- Fixed neighbor commodity statistics to round each arithmetic operation at PC24 before storing doubles; added a denominator-rounding regression at 2^24.
- Final gate: qbuild all PASS; 8410 Rust tests, clippy zero diagnostics, native build PASS, CTest 146/146. Focused core tests 5718 passed; owned-file fmt 2/2 passed.
- **regionalcity not exact:** qcheck-7 exit 1, 1/3. `owner27-views` exact.
- `owner27-resources`: first DIGEST 2006-04-01 income/expense; 27 differing records, 2 only relabelled. Object IDs shift from 1, class 2A5F877D; inspect that shift before later records. First reported content is 0990C075/299B2D1B/0, plumbing / cSC4PlumbingSimulator.
- Resources evidence: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-021925-0d3b259a/result.
- `paradis-karanes`: first DIGEST 2016-07-01 income/expense; 11 differing records, first 2A134F42/299B2D1B/0. Cause remains for the next round.
- Karanes evidence: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-022258-67ac4b8c/result.
- **neighbors not exact:** qcheck-6 exit 1, 1/2. `paradis-ragako` exact after the PC24 fix (previously its power/water cost-per-unit doubles differed).
- `maxisland-indereast`: only CA027EDB/CA027EE1/0 differs; first byte 11881, 139 bytes. All seven complete local-deal records are identical as a multiset; the difference is order alone. Commodity values now match exactly.
- Indereast actual other-side city groups: (1,6), (0,7), (0,5); reference: (0,7), (0,5), (1,6). Current reverse-position order follows neighbors §9.1/§14.1; NB-EXACT33-1 requests its fixed-schedule contract.
- Indereast evidence: C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-021402-82bbf8a5/result.
- Final original-only qcheck-8 exit 0, **5/5 exact**: maxisland-indereast, owner27-resources, owner27-views, paradis-karanes, paradis-ragako, with the final DLL.
- Latest replacement runs inspected: golden-ef979c64df19444b97bcfbb5196c2028 (Indereast), golden-aaf072f155af4cfd87d0a2860d469287 (resources), golden-329e24533d7c436d96d24672a9adb54e (Karanes): clean, ref_warnings=0, no foreign exception/stall/RTC/overrun.
- Direct replacement smoke labels region-exact33-cache-2, region-exact33-views-1, region-exact33-karanes-1: all clean at their end dates, ref_warnings=0. These establish crash recovery, not exactness.
- region-exact33-cache-1 was rejected before launch because script was outside [ini]; cache-2 corrected that placement. Initial qcheck-1/-2/-3 lacked references; later comparisons supersede them. qcheck-5 predates the PC24 fix and is superseded by qcheck-6.
- Logs contain the documented startup x87 warnings; golden requests also contain ignored `pacing` input. Final original-only comparison remains exact. No shadows run, C ABI exports unchanged, no ABI version bump.
- Question: NB-EXACT33-1 in docs/specs/questions/neighbors.md. No behavior spec was edited; glue-regionalcity-replace23 was not merged.
- Next: retain WIP until an owner's complete cover is exact; resolve ordering from the spec, then investigate regional-city deal/statistic queries at the earliest DIGEST and the first shifted save ID.
