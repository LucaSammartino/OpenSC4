| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain3 (enable = terrain glue over the TR21 3D view contract, terrain.md §15.5) | `glue-terrain3` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: make `enable = terrain` reach a clean verdict to 2032 using terrain.md §15.5.11; regress with terrain added; coverage replaced.
- Prior state: glue18 (cFC4Terrain.cpp) builds, faults after DoPostCityInit in the 3D view (eip=0x41800000), run glue18-repl-9.
- Done: worktree created from main ea097498.
- Next: read §15.5.5/.7/.11, compare with cFC4Terrain.cpp, fix view init.
