| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-citysit4: GZCOM replacement enable=citysit over the city-situation core | glue-citysit4 / .claude/worktrees/glue-citysit4 | in progress | see Handoff |

## Handoff
- Goal: enable=citysit replacement (cFC4CitySituation.*, landed by glue17 as the spec §8 minimal replacement over fc4_citysit) proven in game: clean through 2032, ref_warnings=0, trace of calls into the replacement; full gate.
- Done: worktree at main a2313c34.
- Next: baseline qbuild all; audit cFC4CitySituation.cpp for NULL host callbacks / trace; add trace; game run.
- Facts: shadow citysit_shadow is started with enabledKey("citysit") (FC4Subsystems.cpp:670).
