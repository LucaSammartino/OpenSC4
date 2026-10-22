| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-persist-runtime4 (portable persistence runtime) | `core-persist-runtime4` / (engine lane, detached HEAD) | in progress | Audit six runtime owners; independent vectors; implement; final gate |

## Handoff
- Goal: first portable implementation of persistence sections 2-7; six new owners, 131 functions / 13,721 B; existing KeyList separate (22 / 1,071 B).
- Done: f8f00f3c audits all six owner totals and commits independent literal vectors before implementation.
- Done: 31438dbe implements record/typed scalar-array/string/key algorithms and shared stream views.
- Done: ee08730a adds six portable owner groups, variants, sealed factory hooks and image algorithms; QFS codec only in formats.
- Done: 7921d6fe defines Cursor partial-read primitive, file access/close/module-target tests, buffer-QI cleanup and padded row vectors. Native ownership/format gaps remain explicit in AUDIT.md.
- Done: 849dc582 records baseline evidence and PR13. Source is frozen at 7921d6fe; later commits contain only audit/questions/lane documentation.
- Independent vectors: field vectors committed before implementation; v1/v2 literal fixtures also recorded then. Additional lifetime/BMP/variant/edge tests were added after their bodies, not a test-first claim for those groups.
- Baseline all: 20 suites / 5080 passed / 0 failed test cases, then formats doctest E0433 (new adapter source/old core artifact, mixed source); clippy passed, CMake pending. No stable full gate yet.
- Next: run corrected build-runs/runtime-gates.py WITHOUT --after-baseline (quick/fmt/all serially); finish per-owner audit and measure the 17+1 tests.
- Questions PR9-PR13 appended; IDs chosen beyond existing manager PR8; PR1-PR3 retained.
- R2 fresh native process snapshot 2026-10-22T22:36:44Z: no old runtime-gates/qbuild/fc4verify/compiler/test attributable to this lane, all checker candidate commands accessible. WMI denied; native query succeeded. No cause of disappearance inferred.
- Expected additions: 17 core runtime tests and 1 formats integration test, pending actual measurements. No blanket owner or implemented-byte credit.
- IMPORTANT: those 18 new tests have NOT yet had a coherent quick/all execution. Never promote the old 5080 baseline count to runtime coverage. No merge queue entry while gates are pending.
- Review: foreign helpers are limited to raw I/O, buffer allocation/ref/QI/locks/pixels/conversion, OS opening and existing QFS bytes; sealed production factory bodies remain ours. All six owners are partial; no byte subtotal or KeyList completion claim.
- Oracle audit recorded in engine/core/src/persistence_runtime/AUDIT.md: constructed/synthetic conversion fixtures are not native, shipped-data or complete foreign-subclass evidence.
- Baseline retained intact in build-runs/runtime-gates/baseline-mixed-r1-retained/: 20 suites/5080 passed cases then E0433 failure, clippy finished, CMake 214/214, CTest only 20/68 with no final verdict/exit. Interrupted/unmeasured combined gate. Original helper/status and process snapshot retained there.
- Source correspondence rechecked: git diff 7921d6fe..4c99691e -- engine shows ONLY AUDIT.md (23 lines); broad engine trees 146feb7d versus c7cdd186 differ by that documentation. No production-source change since 7921d6fe.
- Local gitignored helper corrected to absolute fmt file paths; archived original first. Do not use --after-baseline on the incomplete dead baseline. No queue until complete required checks.
- R2 owner/test review: sections 2-7 and actual 17+1 tests rechecked; bounded primitives/sealed hooks and specific vectors recorded in AUDIT.md. No production changes. Serial quick running from daec994c, source unchanged since 7921d6fe; native snapshot 22:41:45Z shows qbuild/fc4verify/cargo/sccache descendants, no duplicate checker.
- No C ABI/plugin/manager/cache changes authorized; no game request planned for pure core.
- PR1 native-order supplied explicitly; PR2 constructed fixtures are not shipped coverage; PR3 padded v1 row copy is a FreeCity4 choice.
- R1 guard denial honored with committed handoff/no queue. This is R2 of original max3, not a reset; manifest C:/Users/tlori/FreeCity4/build-runs/run4-core-persist-runtime4-r2-manifest.json.
