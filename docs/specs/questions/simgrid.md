# simgrid: open questions
<!-- Questions about [simgrid.md](../simgrid.md). Append new entries at the END; when one is answered, move it to answered/simgrid.md. Rules: README.md. -->

<!-- from: glue-simgrids4 (2026-10-21) -->
- **SG1 - section 6.3 (with sections 4 and 6.4.5):** a layout-compatible replacement of the grid class
  (`enable = simgrids`, engine/plugin/FC4SimGridLogic.h) keeps `vector<vector<T>>` in the STLport layout with
  every buffer from the STL node allocator B. Two points the spec leaves open: (a) does any original code outside
  `cSC4SimGrid<T>` ever change the vectors' structure through the `GetGridData` pointer (resize, assign, swap,
  `clear`, destroy, or keep a row pointer across a later `SetTractSize` / `Read`), or does it only read and write
  element values within `[begin, end)`? (b) Do original callers ever pass out-of-range indices to slots 7, 8, 22,
  23 or 26 in normal play (section 4: no bounds checks)? The replacement returns 0 for out-of-range reads and
  ignores out-of-range writes [choice]; if an original caller depends on reading a neighbouring row's memory,
  please name it.
