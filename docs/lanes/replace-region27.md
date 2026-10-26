| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-region27 / RegionalCity replacement (engine) | Codex / replace-region27 / detached HEAD | WIP: merge 45f373821; bridge c42c7a0a9; compile fix 108812220; public ABI unchanged; cargo 7859/0, clippy 0 diagnostics, quick native 47/47; game acceptance unmeasured; RY9 filed | 1. Finish full gate and record-bridge baseline. 2. Resolve RY9/RR3. 3. Implement remaining native cache/connection hosts and regionalcity COM factory. 4. Measure acceptance and enqueue ready. |

Round 2 resumed from c42c7a0a9; the earlier merge is already present.
Native compile fix 108812220: supply FC4_ACQUIRED provenance; quick native build completed 47/47 edges (build-runs/fc4verify-build.log).
(a) House replace list plus regionalcity through 2032: pending; ref_warnings pending.
(b) regionalcity shadow with replacement on: pending; required 73/73.
(c) Berlin neighbour crossing/deal and save in replace mode: pending.
Full qbuild all: cargo 7859/0 and clippy 0 diagnostics; native build/CTest queued (qbuild-20261026-004545-build-ctest).
Formatting: eight owned Rust bridge files passed; an initial relative-path invocation found no files and is superseded.
Baseline region27-r2-records2032a rejected (script outside [ini]); corrected region27-r2-records2032b queued in MAIN shared queue.
RY9 filed at docs/specs/questions/regional-city.md: ConnectionHost native border calls remain RR3 in regional-city-records §4.
Next: finish the gate and baseline shadow run; stop the border-host/COM-registration item until RY9 is specified.
