| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin4: cSC4View3DWin window spec (view3d-window.md) | `re-view3dwin4` / (RE spec lane) | in progress | see Handoff |

## Handoff

- Goal: new docs/specs/view3d-window.md for owner cSC4View3DWin (149 functions in decomp/cSC4View3DWin.c): lifecycle, frame, view state, render, events; not restating ui-windows.md §3-4 (layout, slot table, catalog menus) or view3d.md §5 (paint path).
- Done: spec v1 (§1 interfaces/QI/4 vtables, §2 ctor, §3 Init + cheat table, §4 Shutdown, §5 notification routing, §6 window DoMessage), questions VW1-VW4, ABI rows build-runs/abi-rows-re-view3dwin4.tsv (vtables 0xABCE2C/0xABCE1C).
- Next: add DoPreCityInit/DoPostCityInit (0x007F41B0/0x007ED0F0) and AddGUI/RemoveGUI lifecycle section; then finish.
- Facts: REA unavailable (no app open). Decomp `this` in DoMessage 0x007F53C0 is obj+4, in 0x007F3E50 obj+0xC: offsets in spec are absolute, verified on asm. Full asm dumps via build-runs/asmall.sh.
