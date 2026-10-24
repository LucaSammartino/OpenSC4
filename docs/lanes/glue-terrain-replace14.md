| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain replacement interface audit) | `Codex` / `glue-terrain-replace14` (engine lane) | in progress | (1) Audit terrain contracts and plugin layout; (2) fix and gate; (3) measure game runs. |

## Handoff
Goal: make terrain replacement pass the house replace run through 2032 and retain exact shadows.
Done: corrected slot 66 per terrain.md §15.5.6 to return base+0x0C; added null and adjustment seam checks.
Next: run stable-source full gate, then replacement trial and original terrain shadow to 2032.
Facts: clean-room prohibitions exclude RE sources and tools/ even though task requests them; use specs only.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-terrain-replace14-r1-manifest.json.
