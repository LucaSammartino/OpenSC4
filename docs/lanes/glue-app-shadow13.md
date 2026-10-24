| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-app-shadow13 (application shadow) | Codex / `glue-app-shadow13` | in progress | Implement supported application observations, gate, run to 2032 |

## Handoff
- Goal: default-off app_shadow comparing native observations with the core application model.
- Done: added private Rust transport, persistent model, 18 path/filename comparisons and message-derived mode; default-off plugin observer wiring.
- AP17 filed for missing capture contracts: startup inputs, timer elapsed values/effects, pause/speed, app frame/tick counters. Existing AP3/AP7 retained; no whole-owner claim.
- Formatting passed for capi/src/app_shadow.rs. Focused q.py test could not start Cargo (Git Bash CreateFileMapping Win32 error 5); evidence build-runs/q/20261024-184945-32108.log.
- Required qbuild all is running, exec session 26241; collect this exact operation. Cargo compilation in build-runs/qbuild-test.log; no game request yet.
- Next: collect full gate, run queued verify if not already included, then unique app_shadow game request to 2032 and record every comparison count.
- Clean-room: no analysis sources, tools/, or game binaries read.
