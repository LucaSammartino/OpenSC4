| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-region27 / RegionalCity replacement (engine) | Codex / replace-region27 / detached HEAD | WIP: rc23-wip merged at 45f373821; record/cache/deal bridge c42c7a0a9; ABI additions inherited; cargo 7859 passed and clippy clean in round 1; native build failed; game acceptance unmeasured; questions none | 1. Fix native build. 2. Complete regionalcity replacement. 3. Measure house, shadow and neighbour/save runs. 4. Full gate and enqueue. |

Round 2 resumed from c42c7a0a9; the earlier merge is already present.
Native blocker: engine/plugin/FC4RegionalCityElements.cpp:54; previous gate stopped at 309/311 edges.
(a) House replace list plus regionalcity through 2032: pending; ref_warnings pending.
(b) regionalcity shadow with replacement on: pending; required 73/73.
(c) Berlin neighbour crossing/deal and save in replace mode: pending.
Full qbuild all: pending after native fixes and acceptance runs.
Next: inspect the native error, fix it and run qbuild quick.
