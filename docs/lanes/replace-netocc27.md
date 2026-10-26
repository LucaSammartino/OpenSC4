| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-netocc27: pole cell-vector and preview boundaries (engine) | Codex / replace-netocc27 / detached HEAD; final round; code 73c61480c, 25e056895; private C ABI additions | ready to merge partial work: final gate cargo 7879/7879, clippy 0, CTest 119/119; house-0614 clean to 2032, ref_warnings=0, shadow 72/72 months; native poles retained | 1. Concrete tool/mixed-endpoint bridge. 2. Model/render host. 3. Replacement tool script and power-month comparison before replace-list credit. |

- Main merged first without conflicts; NO44 is answered in §4.6.3/cases 75–80. No branch/ref maintenance or readiness credit change.
- Cell slots 5/16 now use allocator B, 12-byte headers/8-byte cells, native growth/capacity reuse, unchanged output on failure and the same header on reverse delegation.
- Host city dimensions replace zero shared cell dimensions; draft generated links skip construction; undo literally calls enable(B,B,true), covered with reversed endpoints.
- Focused CAPI 7/7, core power-line tool 36/36; formatting 3 owned Rust files, zero diffs.
- Preliminary failures remain distinct in build-runs/netocc-r3-first-results.txt: CAPI 6/7 before cell-size fix; quick native C2594 before explicit primary-view cast.
- Final-round qbuild all on HEAD d1970eab7 (code 25e056895), 2026-10-26: PASS; cargo 7879/7879, clippy zero diagnostics, native 49/49 edges, CTest 119/119.
- Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Game log G: C:/Users/tlori/FreeCity4/build-runs/game-results/netocc27-r3-house-0614.log.
- (a) Requested house list + mysims + netocc: clean to 2032-01-01, ref_warnings=0 (G:65259); poles retained native (G:20), so replacement-pole acceptance remains unmet.
- (b) Placement/extension/demolition/fire script and matching power month lines: NOT RUN; concrete tool bridge remains unimplemented.
- (c) Netocc monthly samples 72/72 with mismatches=0; overall 73/73 includes load (G:16066). Pole 73/73, records=219; line 73/73, records=1314 (G:16077–16078).
- NO45 explains normalization: load fixups=0; final month fixups=857, mismatches=0 (G:7260,16051); only documented Read fixups are normalized, not unexplained differences.
- Earlier request netocc27-r3-house-0613 was rejected before start for top-level close_dialogs; corrected request 0614 put it under [ini]. Both used parallel=1.
- ABI rows needed: architect-reviewed NO44 corrections for 00649910,0064E3E0,0064BE50,00649DD0,00649BA0,0064D510,00650140; legacy cell returns unresolved and attachment row says AL. No rows invented.
- ABI: two private static-link C transport entries added; internal host callback extended; fc4.h/version unchanged. No new questions filed.
- Still missing: native CreatePowerPole bridge, enable/Add/Remove/attachment/rebuild crossings (including native neighbours), own model-instance QI/host and delegate/floor/wall/wire submissions; zap callbacks exist but fire is unmeasured.
- Final-round limit respected: no new implementation started. Enqueue only the validated partial work, kind engine; ready means mergeable code, not completed pole replacement.
- Next: bridge the concrete native tool calls and rendering, then prove (a)–(c) with FC4 poles. Pipe classes not attempted; netocc is NOT ready for the replace list and neither pole replacement nor scripted power equality is claimed.
