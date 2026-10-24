| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-traffic-drift10: isolate train188 traffic drift | `Codex` / `glue-traffic-drift10` (engine lane) | in progress | Gate merged main, then one final turbo house run with DBPF enabled |

## Handoff
- Goal: eliminate traffic perturbation from default-off DBPF observer, then verify full current house profile.
- Detached HEAD: commit normally; do not switch branches or publish refs.
- Candidate source 3d2a34205 defers global deque construction until DbpfShadowObserve opt-in. Prior pre-DBPF bisects 478/479 had exact traffic. No comparison weakening or ABI export change; no questions.
- Prior gate cargo 6997/0, clippy 0, corrected CTest 80/80. Stale native ABI object failed evidence preserved in build-runs/evidence-run4/candidate-all-failed-{build,ctest}.log; local FC4_COMPILER_CACHE=OFF corrective build passed.
- Run 480 traffic-drift10-fix-normal-a clean to 2032-01-27, ref_warnings=0, traffic 72/72 exact, resyncs=0. Other totals exact except landvalue 72/73, unsettled pass-end slope sample; retained in its run record.
- Round 2 collected existing traffic-drift10-fix-turbo-a without resubmitting: run 491 clean to 2032-01-27, ref_warnings=0, traffic 72/72 exact, resyncs=0, real_seconds=951.438; all house totals exact except propmgr 219/220 (2027-01-01 queries mismatch=1, resultsDiffering=0).
- Both runs used candidate DLL SHA256 3bd60f90ba90c42cbfce9eb3715c723e2b6672bca78e0606f2e3abb6174a7754; dbpf shadow off.
- Merge dff0e55d6 incorporated main 655671626 as explicitly requested. Resolved FC4DbpfNative.cpp by retaining main monthly rotation/incremental reads/coverage/reset logic and applying nullable pending pointer to new DbpfShadowPending getter too.
- Net engine diff versus merged main 655671626 is ONLY deferred DBPF deque allocation and nullable queue accesses. Main has advanced during the long gate; do not mistake newer region files absent here for lane deletions.
- Final-source qbuild all PASS: Rust 7094 passed/0 failed, clippy diagnostics=0, native build 162/162 edges (674.8s), CTest 81/81 (1220.2s), operation qbuild-20261024-113122-build-ctest. Actual terminal qbuild PASS collected; no duplicate build. Logs build-runs/qbuild-{test,clippy}.log and build-runs/fc4verify-{build,ctest}.log.
- DLL SHA256 6e89375a905e324663b2eabce86d734c29c0ba574505cce6cb00c3ef68683451. Source unchanged since merge; subsequent commits docs only.
- NEXT: submit ONE unique request traffic-drift10-r2-dbpf-turbo-2032-a to 2032-01-27 using main build-runs/regress-train192-shadows.ini (dbpf_shadow=1), omit autoload_city; seconds=1800 timeout=2100. Label verified unused, no request outstanding before submission. Then wait with build-runs/wait-verdict.ps1, never qwait or resubmit completed unique label.
- Final run must have traffic/propmgr and every other house shadow exact plus 0 traffic resyncs/ref warnings, else queue wip with evidence. Next free run number currently 492; recheck docs/runs before assigning.
- Manifest C:/Users/tlori/FreeCity4/build-runs/run4-glue-traffic-drift10-r2-manifest.json.
