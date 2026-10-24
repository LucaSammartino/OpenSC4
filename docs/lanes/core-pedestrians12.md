| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians12: pedestrian owners | Codex / core-pedestrians12 | in progress | Implement core pedestrian and one-shot owners, test and gate |

## Handoff
- Goal: close cSC4Pedestrian and cSC4PedestrianOneShot in engine/core/src/pedestrians using automata records.
- Done: inspected pedestrian specification and existing automata module; detached reserved worktree is clean.
- Next: read automata pedestrian behaviour sections and existing traffic core APIs, then implement.
- PN1 initial-parameter operation is now specified in pedestrian PR conformance; play-count consumer remains open. PN2 belongs to fire crew outside these owners.
- No game requests; no ABI or plugin wiring planned.
