| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-viewutil29: view utilities, engine | Codex; .claude/worktrees/close-viewutil29; reserved branch close-viewutil29, detached HEAD | continue: code 2c75c7ce9; ABI unchanged; all gate PASS: cargo 8082, clippy 0, CTest 135/135; gen27 clean and 25/25 exact, all 50 methods complete; full house retry active | 1. Collect close-viewutil29-house-r6b. 2. Parse all aggregate/method lines. 3. Enqueue ready only after a clean house verdict with zero reference warnings and exact viewutil. |

Main merged first (82e49a45b); resolved the question-file conflict to main's answered state. No new question filed; VU12/VU17/VU18 contracts are now consumed.
2c75c7ce9 fixes city-width's two separately PC24-rounded products before the sum, PC24 normalization, draw field/line loading, and city-clipped-box recomputation from selected corners.
The eight yaw mismatches were the retained earlier PC24 multiply fix; the seven width mismatches were operation order, not capture timing. No comparisons were suppressed or compensated.
Compared: all 50 entries in view-utilities.md §12, named in FC4ViewUtilShadow.cpp Method()/family tables; no body remains at calls=complete=0.
New cached bodies: 007F6910 draw-box and 007F7F60 city-frustum-box (both selectors). New provider/resource bodies: 007F9160 viewport-ground,007F91C0 MeshPick,007F9A40 ModelPick,007F9EA0 ground-corners,007F9F50 draw-build,0080B320 iterator-select.
Fixtures use the published mesh/vertex/index/group graph and private perspective camera. Model count/mesh/transform call order and arguments, return/out words, every owner word and camera preservation are compared.
Scope limits: camera update/direct ray still exercise missing-renderer paths; visible area is warm-cache; draw build is whole-viewport; destructor is non-deleting; generic numeric decoder retains its documented checked domain. Successful update/direct-ray, cold visible area, narrowed draw and heap deletion are unexercised.
Final-source qbuild all PASS, one invocation: cargo 8082/8082, clippy zero diagnostics, CMake 29/29 edges, CTest 135/135. No source changed afterward. All six owned Rust files passed fmt.
Gate logs: WT/build-runs/{qbuild-test.log,qbuild-clippy.log,fc4verify-build.log,fc4verify-ctest.log}; quick also passed 6128 tests and 35/35 native edges on fixed source.
DLL SHA256: 2A37DBDC911252049701D395EE21F70F925C73470A35E2F6DF21F2E2B93CA6D2; all r6 requests use it. No ABI export changes.
gen27-r6a: clean/ref_warnings=0 through 2028-01-28; 25/25 aggregate exact; all 50 methods have cumulative calls=complete>0, mismatches=0, resyncs=0.
gen27 evidence: MAIN/build-runs/game-results/close-viewutil29-gen27-r6a.{verdict,log}; script line 948 logs seed=1 mix=default complete. All 948 script lines reached the final marker.
Live VU12 observation: immediately-before-entry CW=007f, capture=f32; yaw (3,3)=408A3AE6, width64=42A73D74, width128=43273D74. Numeric lines are in both completed r6 logs.
house-r6a: stalled at 2027-03-01 (watchdog_stalls=1, foreign_exceptions=0, ref_warnings=0); viewutil 15/15 exact, all 50 complete, resyncs=0. Not accepted as clean.
house failure evidence: MAIN/build-runs/game-results/close-viewutil29-house-r6a.{verdict,log}; viewutil finishes at 20:58:45.619, later probes continue through 20:58:48, watchdog terminates at 20:58:49 after 10953 ms without OnTick.
PENDING: close-viewutil29-house-r6b, running in shared queue slot 8 since 2026-10-26 21:04:15. No verdict at the last qwait timeout. The qwait observer has exited; only the queue-owned game remains active.
Retry keeps the complete train283 house [ini] block and 2032-01-01 end date; adds documented autotest_stall_grace_ms=120000 (engine/README.md), seconds=7200, timeout=7350, parallel=1. No cases or shadows removed.
Next: python C:/Users/tlori/FreeCity4/.claude/lane-bin/qwait.py C:/Users/tlori/FreeCity4/build-runs/game-results/close-viewutil29-house-r6b.verdict; parse with WT/build-runs/analyze-viewutil6.py; require clean/ref_warnings=0, all aggregates exact and all 50 method records complete before ready enqueue.
Whole-owner verified credit is pending the clean house result. No new merge-queue entry or ready enqueue was written; do not promote the stalled run. No rebuild is needed unless source changes.
