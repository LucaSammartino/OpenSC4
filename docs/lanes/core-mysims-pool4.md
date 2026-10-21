# core-mysims-pool4 — MySimPool / MySimDispatch job search to mysims §4.7.1 / §4.12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-pool4 (rework mysims dispatch job search §4.7.1 and pool §4.12 cores) | `core-mysims-pool4` / `.claude/worktrees/core-mysims-pool4` | in progress | see Handoff |

## Handoff

- Goal: engine/core/src/mysims/dispatch.rs job search = §4.7.1 (persistent pathfinder, flags 0x11, filter 0,
  dest flags by wealth OR-accumulated, density 1, one FindPath, end-cell lot/neighbour resolution);
  pool.rs = §4.12 (templates, exemplar enumeration, MySims.dat, Import, cache reset, display path).
  Conformance vectors from the worked examples. No plugin/CAPI/CMake changes.
- Done: (none yet)
- Next: rewrite dispatch.rs validate/set_current_commute_destination, then pool.rs, then tests + vectors.
- Facts: only core tests (mysims/dispatch_tests.rs) use mysims::dispatch / mysims::pool; capi does not.
