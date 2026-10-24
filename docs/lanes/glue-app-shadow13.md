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
- Game request app-shadow13-2032-r2a submitted: turbo, end_date=2032-01-01, seconds=900, app_shadow=1. Wait via build-runs/wait-app-result.py (no qwait).
- Next: collect final full gate and game verdict, record every app/app-startup comparison count, finalize report and ready queue for measured work.
- Clean-room: no analysis sources, tools/, or game binaries read.
