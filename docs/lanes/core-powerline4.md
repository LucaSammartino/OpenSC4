| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-powerline4 (cSC4PowerLineTool core: slot 53 FSM, slot 81 reserved tiles, cost, pole position, pole reuse) | `core-powerline4` / (engine lane) | ready: core `7ef2c0b8`; gate cargo 3997/0, clippy 0, ctest 65/65; no ABI, no game run (core only); questions NT18-NT20, NO21 | 1. answer NT18 (DeterminePolePositions/PlacePoles bodies), then implement them behind `PowerLineHost`; 2. a glue lane binds the core to slot 53/81 |

## Report

**What changed** (engine/core only; no plugin, CAPI or network_rules edits):

- `engine/core/src/network_tools/power_line.rs` (new):
  - `do_network_specific_placement`: slot 53, network-tools.md §9 steps 1-6. It clears the pole and line lists,
    sets valid := dragged, runs DeterminePolePositions + PlacePoles(1) when there are at least 2 steps, sets
    valid := 0 when there is no pole record, runs the regional-connection gate, refreshes the draft preview with
    slot 82(0), releases the pole infos, and returns true.
  - `check_for_reserved_tiles`: slot 81, the network-geometry §11 NT3 table. It uses `last`/`prev` and takes the
    uninitialised first "previous" result as a parameter (the quirk).
  - `PowerLineTool::cost_of_solution`: reuses `network_geometry::extras::power_line_cost`.
  - `PowerLineParams::load`: the exemplar properties `0x4A5F5953`, `0x0A5F596B`, `0x098B25C8` and `0x098B27D0`.
  - `compute_pole_position`: network-occupants §5.2.
  - `create_power_pole`: §4.5 rule (reuse the pole found at the cell, else create one).
  - Failure codes `0xF0000011`-`0xF0000013`.
- `engine/core/src/network_tools/tests_power_line.rs`: 17 conformance tests:
  - the FSM call orders for real/draft and neighbour on/off;
  - the one-step drag, no poles, one pole, and the failure and partial-drag gates;
  - reserved tiles for clear, building, zoned (middle and trailing), road (middle, last after clear, last after
    zoned, road after road) and the one-cell road quirk;
  - the cost with the shipped values (12 wire cells and 4 poles give 84), the pole position and its guard, and
    pole reuse.

**Gate:** baseline was cargo 3980 and ctest 65/65. After the change: cargo 3997 passed / 0 failed, clippy 0
diagnostics, ctest 65/65 (all passed).

**Owner completeness:** this lane covers only the parts of cSC4PowerLineTool that the specs give in full: slot 53,
slot 81, GetCostOfSolution, the parameters, ComputePolePosition and the reuse rule of CreatePowerPole.

What is still missing, and why:
- No spec gives a body for DeterminePolePositions (`0x00650840`), PlacePoles (`0x00651560`),
  Attempt/PlaceNeighborConnections, or the slot 82 preview refresh. They are `PowerLineHost` hooks (NT18).
- Slots 3, 4, 10 and 16 (Init, Shutdown, UpdateOnZoneChange, `0x0064F3F0`) are listed without behaviour.
- GetFailureReasonString `0x0064F570` is not implemented.

**Questions filed:**
- NT18: the pole placement algorithm, and where slot 81 is called.
- NT19: the parameter defaults, and the meanings of `+0x3A4`/`+0x3A8`.
- NT20: the Attempt/Place pairing, the trailing zoned or road cell, and results above 3.
- NO21: the `-FLT_MAX` guard in ComputePolePosition.
