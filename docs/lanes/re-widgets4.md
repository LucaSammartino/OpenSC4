| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets4 (widget toolkit spec: cGZWin base window) | `re-widgets4` / (RE lane) | in progress | - |

## Handoff
- Goal: docs/specs/widget-toolkit.md for the cGZWin base window (identity, layout, lifecycle, child list, flags, coordinates, hit test, message dispatch) from executable evidence.
- Done: widget-toolkit.md sections 1-12 (cGZWin layout, flags, children, geometry, dispatch, lifecycle, fade); questions WT1-WT6 (commit: spec 1).
- Next: optional second slice (painting PlotComposite 0x0099E62D, cursors 0x0099D40B, accelerators/key equivalents), then finish report + merge-queue.
- Facts: coverage-map rows 372-386 (cGZWin*, cGZLegacyControl, ...) all '-'. REA NO-GO (no binary open); used decomp + re_query asm + build-runs/rd.py image reads. ui-windows.md §2 and §14.1 already cover cIGZWin slot subset; do not rewrite.
