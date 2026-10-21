| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrids4: layout-compatible cSC4SimGrid<T> family (enable=simgrids) | glue-simgrids4 / .claude/worktrees/glue-simgrids4 | in progress | see Handoff |

## Handoff

- Goal: C++ cFC4SimGrid<T> for the 10 element types + "bool" factory (CLSIDs 0x49B9E601..0x49B9E60B), game
  layout (vector<vector<T>> {begin,end,cap}, buffers from allocator B = FC4GameList.h StlNodeAllocator),
  count-0 creation, §4 slots, §5 record; registered only under `enable = simgrids`. No capi/ABI change.
- Done: 3bc0dae7 code (FC4SimGridLogic.h, cFC4SimGrid.{h,cpp}, simgrid_test, registration "simgrids").
  Baseline gate: cargo 3652 pass, clippy 0, ctest 62/62. SG1 filed (docs/specs/questions/simgrid.md).
  Game simgrids4-a1 (turbo to 2026-07-01, script save 2026-04-01): clean, ref_warnings=0; 136 grids created by
  class store, 136 read / 136 written / 136 released, grid_data_calls=3142408, iface_writes=314134,
  raw_changed=50 (grids changed only through raw GetGridData writes).
- Next: game simgrids4-a2 (turbo to 2032-01-01, save 2031-12-01), README entry, full gate, finish.
- Facts: spec docs/specs/simgrid.md §2-§6; rule §6.4.2 lifted only because ours is layout-compatible (§6.3.2).
