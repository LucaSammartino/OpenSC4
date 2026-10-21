| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules4: network-rules glue boundary | fc4-lane / .claude/worktrees/glue-netrules4 | blocked on NR13 (no supported interception boundary) | after NR13: wire fc4_network_rules into FC4NetToolHost piece choice (enable=netmgr) or a spec'd boundary |

## Report

Goal: find a supported real-game caller/factory boundary for cSC4TileEvaluator / cSC4TileRule / cSC4TileInfo /
nSC4Networks and wire the landed Rust network_rules as `enable = netrules` only if public spec/ABI supports it.

Findings (clean-room sources only):
- docs/abi has no row for any of the four owners (grep of interfaces.tsv and the SDK headers: no CLSID/IID/vtable).
  The nearest ABI rows are cISC4TrafficNetworkMap slots 13-16 (GetTileInfoAtCell ...), which read a tile-info
  record but do not create or solve it.
- docs/specs/network-rules.md presents them as internals of cSC4NetworkTool::SolveNetwork; §14 NR12 answer states the
  C entry points need not be exposed, only their behaviour.
- The only in-repo caller that could use the solver is our own tool, engine/plugin/FC4NetToolHost.cpp
  `InsertOccupantsForSolvedCells` (marked [choice]: piece id 0, solver not wired), reached under `enable = netmgr`.
  That is not interception of the original's owners, so it was not done without a spec answer.

Result: no code change; question NR13 filed in docs/specs/questions/network-rules.md. No game runs, no gate run
(docs-only change). No `enable = netrules` key added; no CLSID or address invented.

Next: once NR13 is answered, either (a) wire fc4_network_rules_* into FC4NetToolHost's per-cell piece choice and
prove it in-game under enable=netmgr, or (b) implement the boundary NR13 names.
