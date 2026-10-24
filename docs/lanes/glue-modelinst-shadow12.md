| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (model instance shadow load fix) | `glue-modelinst-shadow12` / engine lane | in progress | Diagnose shadow11 pre-city load blocker; apply MI14; gate and measure to 2032. |

## Handoff
- Goal: merge shadow11, fix menu load blocker, apply MI14 and measure shadows through 2032.
- Done: shadow11 merged in 2b8dc5bcf, preserving main's lighting additions.
- Diagnosis: pre-load diff only registers default-off ini key; fixture starts in city-load notification. MI14 proves missing constructor flag corrupts the x86 stack and allocation-start cast selects the container instead of instance interface.
- Applied MI14 constructor flag 1 and +0xA0 standalone interface adjustment in e8703c22d.
- Native verify PASS: qbuild-20261024-155017-build-ctest; build 263/263, CTest 81/81, logs build-runs/fc4verify-{build,ctest}.log.
- Game run 520 modelinst-mi14-loadfix-2032-r1: clean to 2032-01-02, ref_warnings=0; static 73/73 exact, 144 fields/line. DLL fa30f030. File-watcher session 74491 was collected to terminal timeout despite the valid completed verdict; no game rerun was made.
- Live static GetTransform destination counter/padding observer implemented (first 4096 natural calls/city, no added native calls); source stable, owned Rust fmt and diff --check pass. No source words/kind, basic/animated/processor/sprite or whole-owner claim.
- Live observer committed in 939c4bfa5. Focused cargo test session 51921 collected to terminal: 2 passed, 0 failed, compile 34m43s; PowerShell wrapper exit 1 is from stderr progress becoming NativeCommandError, not a failed test. Full evidence: build-runs/modelinst-focused.log.
- Required stable-final-source all gate session 36931 collected to terminal PASS: cargo 7,375 passed / 0 failed / 0 ignored (48 summaries); clippy PASS (-D warnings), diagnostics=0. Native qbuild-20261024-182011-build-ctest PASS: build 58/58 (58.3 s), CTest 81/81 (157.6 s), total 220 s. Full logs in build-runs/qbuild-*.log and fc4verify-*.log. Only docs commits changed the tree during gate; engine source is 939c4bfa5.
- Wind-down checkpoint at 99,525/150,000 measured context: no further implementation this round; finish existing gate and measure committed scope, then report/queue. Full final-source gate and live game measurement still pending.
- Next: request final modelinst-mi14-live-2032-r2 game run to 2032, collect its terminal result, record exact static/live counts and queue ready if clean. No outstanding native operation.
- Initial q.py Bash transport failure is preserved at build-runs/q/20261024-161127-6204.log; direct cargo was used instead. fmt required an absolute owned-file path (relative invocation failed before checking source).
- Keep shadows default-off; use queued native builds and game runs; no direct game launch.
