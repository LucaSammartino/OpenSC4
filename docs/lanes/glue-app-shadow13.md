| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-app-shadow13 (application shadow) | Codex / `glue-app-shadow13` | in progress | Implement supported application observations, gate, run to 2032 |

## Handoff
- Goal: default-off app_shadow comparing native observations with the core application model.
- Done: added private Rust transport, persistent model, 18 path/filename comparisons and message-derived mode; default-off plugin observer wiring.
- AP17 filed for missing capture contracts: startup inputs, timer elapsed values/effects, pause/speed, app frame/tick counters. Existing AP3/AP7 retained; no whole-owner claim.
- Formatting passed for capi/src/app_shadow.rs. Focused q.py test could not start Cargo (Git Bash CreateFileMapping Win32 error 5); evidence build-runs/q/20261024-184945-32108.log.
- Previous qbuild all was terminated with the transport; session 26241 is unknown and targeted process inspection confirms no surviving lane qbuild. Incomplete evidence retained as build-runs/qbuild-test-transport-interrupted.log.
- Next: full gate via build-runs/run-app-gate.py with RUST_TEST_THREADS=2 (limits shared-machine contention), then unique app_shadow game request to 2032 and record every comparison count. No game request outstanding.
- Clean-room: no analysis sources, tools/, or game binaries read.
