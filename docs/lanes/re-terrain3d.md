| re-terrain3d | docs | TR21 terrain 3D view contract (cSTETerrainView3D 0xC9B84E10) into docs/specs/terrain.md §15.5 | main | a2e6fdf5 | in progress |

## Handoff
Goal: RE cSTETerrainView3D (class 0xC9B84E10) and write the TR21 contract into docs/specs/terrain.md §15.5 so the terrain-replacement enable lane knows what the 3D view needs.
Done: (nothing yet)
Next: read terrain.md §15.5 + §1-§4, grep vtables.tsv/interfaces.tsv/layouts.md for cSTETerrainView3D + cSTETerrain, then RE slots.
Facts: worktree fresh at main a2e6fdf5. decomp/ lives in MAIN only (gitignored) — read from C:/Users/tlori/FreeCity4/decomp/. re_query.py from MAIN/tools/.