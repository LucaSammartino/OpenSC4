| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-networkmodel4 (cSC4NetworkOccupantModelInstance core, network-occupants.md §10) | `core-networkmodel4` / (engine lane) | in progress | - |

## Handoff
- Goal: core-only cSC4NetworkOccupantModelInstance (spec network-occupants.md §10, test 24) in engine/core/src/network_occupants; no plugin/CAPI.
- Done: worktree at main 31d43eb5; model.rs + tests_model.rs + NO21-NO23 at 8372f704.
- Next: final qbuild all, then finish (report, merge-queue json). No game run (core only, no plugin).
- Facts: baseline gate cargo 4029/0, clippy 0, ctest 65/65.
