| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-impl4: wire fc4_network_rules into our network tool | fc4-lane / .claude/worktrees/glue-netrules-impl4 | in progress | see Handoff |

## Handoff

Goal: per network-rules.md §2.4 (NR13), run fc4_network_rules_* as a component of our network tool
(FC4NetToolHost InsertOccupantsForSolvedCells, enable = netmgr), no netrules key; prove in game.

Done: worktree at main 07cb3113.

Next: read FC4NetToolHost.cpp InsertOccupantsForSolvedCells (line ~258) and capi network_rules.rs API.

Facts: none yet. No game requests outstanding.
