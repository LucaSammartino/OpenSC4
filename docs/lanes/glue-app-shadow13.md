| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-app-shadow13 (application shadow) | Codex / `glue-app-shadow13` | in progress | Implement supported application observations, gate, run to 2032 |

## Handoff
- Goal: default-off app_shadow comparing native observations with the core application model.
- Done: added private Rust transport, persistent model, 18 path/filename comparisons and message-derived mode; default-off plugin observer wiring.
- AP17 filed for missing capture contracts: startup inputs, timer elapsed values/effects, pause/speed, app frame/tick counters. Existing AP3/AP7 retained; no whole-owner claim.
- Formatting passed for capi/src/app_shadow.rs. Focused q.py test could not start Cargo (Git Bash CreateFileMapping Win32 error 5); evidence build-runs/q/20261024-184945-32108.log.
- Resume review found app_shadow missing from the known-key list; appended it in ae3cc0a3c, preventing the misleading config warning. The running recovery gate predates this C++ edit; run final all after collecting it, preserving both results.
- Previous qbuild all was terminated with the transport; session 26241 is unknown and targeted process inspection confirms no surviving lane qbuild. Incomplete evidence retained as build-runs/qbuild-test-transport-interrupted.log.
- Recovery gate: cargo 7407 passed / 0 failed / 0 ignored (51 result lines); clippy exit=0 diagnostics=0. Logs build-runs/qbuild-test.log, qbuild-clippy.log, app-gate-console.log.
- Recovery full gate PASS: native qbuild-20261024-200020-build-ctest passed 265/265 build edges and 81/81 CTests. Exec session 55208 collected to exit=0.
- Recovery evidence preserved by build-runs/run-app-final-gate.py under build-runs/app-recovery-gate/; final all runs on committed stable source (RUST_TEST_THREADS=2).
- Game app-shadow13-2032-r2a completed clean to 2032-01-05, ref_warnings=0; run 631 records app 0/72 and app-startup 0/1. Exactly fields 14 Exception Reports and 15 Scripts differ; other 17 fields match. Startup published Julian was unset.
- Added expected-path and stored-input diagnostics; startup now waits for a published positive Julian. Formatting fix applied in place. This source change requires final all after diagnosis.
- Gate exec session 35565 is still running; collect it before any new native verify. No game request outstanding.
- AP18 filed for observed lazy slot65/66 base/suffix/pre-cache-input disagreement. Supported app/app-startup now compare 17 fields; app_shadow_path_cache=1 separately enables app-path-cache diagnostics, default off. Literal core unchanged; no resync.
- Next: collect gate, normal-speed diagnostic run to 2032, resolve or question the two lazy cache differences, gate final source and rerun measured scope.
- Clean-room: no analysis sources, tools/, or game binaries read.
