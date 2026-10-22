| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser4: cGZWinFileBrowser owner spec (100 functions, 25579 B) in docs/specs/win-filebrowser.md | `re-filebrowser4` / (RE lane) | in progress | see Handoff |

## Handoff

- Goal: spec ONLY cGZWinFileBrowser (owner inventory from `tools/coverage.py --owners`: 100 functions, 25579 B,
  subsystem widget toolkit) in docs/specs/win-filebrowser.md, with an §A inventory naming a section per function.
- Done: worktree created; inventory extracted (build-runs/fb.tsv, fbnames.txt).
- Next: read vtable 0x00a8d000 region + decomp/cGZWinFileBrowser.c (3101 lines) by function; write spec sections.
- Facts: ui-windows.md line ~892 lists cGZWinFileBrowser as "Not to be specified" (INVENTORY-M6 F7/F8); this lane
  was assigned it anyway by the coordinator. legacy-control.md §9.3 already covers the property interpreter
  0x00954212 (not owned here). coverage-map.tsv needs a row `cGZWinFileBrowser -> win-filebrowser.md` (tools edit,
  proposed only; this lane is docs-only).
