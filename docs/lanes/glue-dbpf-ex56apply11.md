| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-ex56apply11 | Codex / glue-dbpf-ex56apply11 | in progress | Apply EX56 traversal contract; gate and measure to 2032. |

## Handoff
- Goal: extend default-off DBPF shadow to registered city and other specified open files with monthly rotation.
- Done: merged re-dbpf-ex56 by fast-forward to 4761ef152; Git also reported packed-refs.lock permission denial (exit 0).
- Done: 8003d6ddb implements city-path discovery/fallback, explicit class counters, EX56 payload return/null/prefix/missing checks, file-stamp and pre-save generation invalidation. EX57 asks for transient lifecycle observation contracts.
- First all operation completed FAIL: cargo 7119/0/0 across 45 suites; clippy 0 diagnostics; native build PASS 260/260 (625.4 s), CTest 80/81 (792.5 s), ret_channel_check failed on unreviewed fallback Open. Tree-change warning reflects PreSave teardown correction after worker start. Evidence: build-runs/evidence-run4/ex56-first-{build,ctest}.log and build-results/qbuild-20261024-123328-build-ctest.verdict.
- Review diagnostic: fallback factory temporary needed RELEASE_CREATED on successful teardown; fixed while first native request was still queued. Preserve first operation evidence distinctly; final stable-source gate/verify still needed.
- Save-transition review: release all pending QI views at PreSave, before game owners can unregister old city segments; later ticks report invalidated samples without touching released views.
- Ret-channel diagnostic fixed with existing SDK-vs-streaming ABI explanation; focused checker PASS 0 flagged/78 reviewed. q.py wrapper failed Git Bash shared-memory sandbox setup; direct PowerShell Python check succeeded.
- Corrected-source explicit qbuild verify PASS: request qbuild-20261024-132817-build-ctest; build 47/47 edges (4.0 s), CTest 81/81 (98.8 s), total 106.6 s. Logs archived under build-runs/evidence-run4/ex56-verify-{build,ctest}.log; unique build-results verdict retained. Operation 36086 ended exit 0.
- Final all active: session 70829, native request qbuild-20261024-141831-build-ctest. Rust 7119 passed/0 failed/0 ignored, clippy 0 diagnostics; native result pending. Collect SAME operation, no duplicate build.
- Next: finish all gate, then queued native verify; house profile turbo to 2032, then focused save/re-registration observation.
- House milestone: dbpf-ex56-house-492 clean, dbpf 73/73 exact to 2032-01-03, 916.234 real seconds / 932 wall seconds, ref_warnings=0. Record docs/runs/dbpf-ex56-house-492.md; DLL ec17f769. Eleven files (10 packed roots + 1 multi child), city directly registered and compared 7 times/187 records each; no fallback. Initial 111826 records; final cumulative 876020 records, rotation 11/11 files and 111826/111826 records. Max slice 828 ms, refusals=0.
- Next game: submit build-runs/dbpf-ex56-save-493.pending (normal speed to 2032, full save 2029-06-15), then wait via build-runs/wait-dbpf-verdict.ps1 (no qwait). Only one game request outstanding; house is terminal.
- Helper: python build-runs/summarize-dbpf-run.py LABEL prints bounded exact counts and city/save evidence. Run numbers 492/493 were free in this checkout when requests were prepared.
- Required: all gate plus queued verify; house profile turbo to 2032, exact shadow and clean shutdown under 1800 seconds.
- No game request currently outstanding; focused save request not yet submitted. No C ABI exports changed. EX57 blocks transient lifecycle coverage; report all scope limits before ready. Do not edit tools/ (provided house profile already has dbpf_shadow=1); coordinator regenerates question index.
