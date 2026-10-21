| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-powerline4 (cSC4PowerLineTool core: slot 53 FSM, slot 81 reserved tiles, cost, pole position) | `core-powerline4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: Rust core of cSC4PowerLineTool from network-tools.md §9, network-geometry.md §11, network-occupants.md §4.5/§5.2.
  No plugin/CAPI edits; no change to network_rules glue.
- Done: lane file.
- Next: baseline `qbuild all`; add engine/core/src/network_tools/power_line.rs (state, FSM, reserved tiles, cost,
  pole position, CreatePowerPole reuse) + tests; questions NT18+ for unspecified DeterminePolePositions/PlacePoles.
- Facts: existing `network_geometry::extras::power_line_cost` already implements GetCostOfSolution (reuse it).
