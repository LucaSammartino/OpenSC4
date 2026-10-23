| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-network-routines4 (portable placement demolition helper) | Codex, finished by Claude / `core-network-routines4` | ready: network-routines §§2–8 helper in fc4-core with the eight §8 vectors as tests; ABI unchanged; no game runs | Land via the merge train. Wiring into network_rules/plugin is a separate lane. |

## Report
- `engine/core/src/network_occupants/placement_demolition.rs` implements
  `ComputeDemolitionRegionForPlacementCells` (docs/specs/network-routines.md §§2–8): signed bounds from the
  input cells, half-open lot rects appended per input (duplicates kept), a cleared `CellRegion` whose stored
  highs keep the extra clear border, an x-outer/z-inner scan that sets a bit and the zone-type mask bit on
  each zoned cell whose filtered occupant lookup misses, the post-scan `0xFC00` rejection, cost cleared just
  before the preview, and the eleven-argument DemolishRegion preview whose AL is returned unchanged.
- `ExclusionFilter` is the temporary filter: occupant included iff QI `0xA9764C8C` returns zero, every
  non-null QI output released; type/holder predicates always true; QI views for IID 1/0x46A80026/0xE4FDA3D4.
- `RegionOutput` keeps a displaced previous region rather than dropping it (the helper never destroys it).
- Host trait exposes only the foreign primitives; demolition itself, lot/occupant/zone services stay foreign.
- Tests: `tests_placement_demolition.rs`, 11 tests covering §8 vectors 1–8 plus admission and max-size cases.
- Gate: cargo workspace 5725 passed / 0 failed (summed over 27 `test result:` lines); clippy -D warnings 0
  diagnostics; CMake build pass; ctest 68/68 on rerun (first run: `plugin_ini_turbo_test` failed on a
  wall-clock window under machine load; this lane touches no plugin code). fmt clean on the 3 .rs files.
- Not done: no C ABI, plugin or network_rules wiring; native region/filter layout and the allocation-failure
  branches (including the null-filter preview) are not modelled; out-of-city input or malformed lot rects
  return `Unsupported`, a portable validation policy and not native behaviour (spec NRT1). No new questions.
