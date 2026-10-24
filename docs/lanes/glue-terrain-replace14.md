| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain replacement interface audit) | `Codex` / `glue-terrain-replace14` (engine lane) | in progress | (1) Audit terrain contracts and plugin layout; (2) fix and gate; (3) measure game runs. |

## Handoff
Goal: make terrain replacement pass the house replace run through 2032 and retain exact shadows.
Done: 2e7504714 corrects slot 66 per terrain.md §15.5.6; null and adjustment seam checks added.
Done: companion factory correction requests IID 0x6771477D and stores base; audited acquisition/release; replacement seam wired into terrain native test.
Next: collect running all gate (session 67957, began before companion changes), then run all on final stable source; game trial and original terrain shadow to 2032.
Question: TR44 requests the reported crash receiver/slot and slot 54 render-object identity/lifecycle.
Facts: clean-room prohibitions exclude RE sources and tools/ even though task requests them; use specs only.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-terrain-replace14-r1-manifest.json.
