| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrids4: layout-compatible cSC4SimGrid<T> family (enable=simgrids) | glue-simgrids4 / .claude/worktrees/glue-simgrids4 | in progress | see Handoff |

## Handoff

- Goal: C++ cFC4SimGrid<T> for the 10 element types + "bool" factory (CLSIDs 0x49B9E601..0x49B9E60B), game
  layout (vector<vector<T>> {begin,end,cap}, buffers from allocator B = FC4GameList.h StlNodeAllocator),
  count-0 creation, §4 slots, §5 record; registered only under `enable = simgrids`. No capi/ABI change.
- Done: (none yet)
- Next: write engine/plugin/cFC4SimGrid.{h,cpp}, register in FC4Subsystems.cpp, ctest, game run.
- Facts: spec docs/specs/simgrid.md §2-§6; rule §6.4.2 lifted only because ours is layout-compatible (§6.3.2).
