| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-traffic-drift10: defer DBPF allocation to eliminate traffic drift | `Codex` / `glue-traffic-drift10` (engine lane) | READY: fix 3d2a34205, main merge dff0e55d6; cargo 7094/0, clippy 0, CTest 81/81; runs 480/492 traffic 72/72, zero resyncs; run 493 traffic 72/72, propmgr 220/220, DBPF 73/73, all 52 house categories exact, ref_warnings=0; no questions or ABI export changes | - |

## Change and cause

Train188 added a namespace-scope `std::deque<HeldSample>` for the DBPF shadow. Even its empty constructor allocates during DLL initialization, before the default-off observer checks its setting. The bisect and fixed runs implicate that startup allocation as the traffic perturbation: runs 478 (pre-GZMsg/DBPF) and 479 (GZMsg, pre-DBPF) both had exact traffic; deferring the queue restores exact traffic on the full train188 engine.

The fix constructs a function-local queue only after `dbpf_shadow` opt-in and keeps a trivial null pointer before that. Tick, Shutdown and the newer Pending getter handle an absent queue. Existing enabled sampling, comparisons and acquired-reference release behavior remain intact. No traffic comparison was weakened and no C ABI export changed.

As requested, merge dff0e55d6 incorporated main 655671626. The conflict resolution retains main's incremental DBPF reads, monthly rotation, coverage counters and shutdown resets while applying nullable queue access. Relative to that merged main, the engine change is only FC4DbpfNative.cpp (16 additions, 9 deletions). Main advanced while the gate ran; newer region work remains on main and was never removed by this lane.

## Validation

- Final stable engine source: dff0e55d6. Later commits change documentation only.
- Required qbuild all: PASS, operation qbuild-20261024-113122-build-ctest, collected to its actual terminal result. Rust 7094 passed/0 failed, clippy 0 diagnostics, native 162/162 edges (674.8s), full CTest 81/81 (1220.2s). Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log.
- Earlier candidate gate had a stale native ABI object: CTest 79/80, capi_smoke linked Rust 98/header 97 despite both source constants being 98. Failed logs preserved at build-runs/evidence-run4/candidate-all-failed-{build,ctest}.log. Corrective native rebuild passed 80/80; final merged-source gate above passed 81/81. No ABI constants were changed by this lane.
- [Run 480: normal](../runs/traffic-drift10-fix-normal-a-500.md): clean through 2032-01-27, traffic 72/72 exact, zero resyncs/ref warnings. Earlier landvalue 72/73 unsettled pass-end slope sampling difference retained in its record.
- [Run 492: existing turbo](../runs/traffic-drift10-fix-turbo-a-504.md): collected traffic-drift10-fix-turbo-a-504 without resubmission. Clean through 2032-01-27, traffic 72/72 exact, zero resyncs/ref warnings. Propmgr 219/220 (queries 2027-01-01, resultsDiffering=0), every other house total exact. This historical difference is retained. Provisional run 491 was renumbered to free 492 after main allocated train193 run 491.
- [Run 493: final house turbo](../runs/traffic-drift10-r2-dbpf-turbo-2032-a-506.md): clean through 2032-01-27 using main's train192 house profile with dbpf_shadow=1. Traffic 72/72 exact, zero between-delivery resyncs; propmgr 220/220; DBPF 73/73; every house total exact: 52/52 categories, 10344/10344 lines. Landvalue 73/73; automata and automataowners each 72/72. Foreign exceptions, watchdog stalls, RTC, overruns and reference warnings all zero. No positive shadow mismatch or traffic resync found in the full log search.
- Final run real_seconds=808.329, wall=829s, slot=1; DLL SHA256 6e89375a905e324663b2eabce86d734c29c0ba574505cce6cb00c3ef68683451. Request label traffic-drift10-r2-dbpf-turbo-2032-a-506 was unique and submitted once; wait used wait-verdict.ps1 rather than qwait. No game/build operation remains outstanding.

## Questions and scope

No new spec questions. DBPF exactness here has main's existing public resource-manager packed-file scope; EX56 coverage limits are unchanged. No game assets or forbidden analysis material were read or committed. Ready for the coordinator to publish the detached HEAD to the reserved lane branch and merge the fix.
