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
- Native build+CTest queued as qbuild-20261024-200020-build-ctest; collect exec session 55208 to its terminal result. No game request outstanding.
- Next: collect native gate, preserve recovery evidence and run final all on stable source, then unique app_shadow game request to 2032 and record every comparison count.
- Clean-room: no analysis sources, tools/, or game binaries read.
