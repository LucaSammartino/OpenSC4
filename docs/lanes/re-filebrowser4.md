| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser4: cGZWinFileBrowser owner spec (100 functions, 25579 B) in docs/specs/win-filebrowser.md | `re-filebrowser4` / (RE lane) | in progress | see Handoff |

## Handoff

- Round 2 audit: preserved tip 6e85917e contains no spec or code changes; only this lane file.
- Milestone: drafted win-filebrowser.md interfaces, path updates, parent navigation, row lookup,
  and FB-C01..09. Numeric branches checked against bytes. Primary table 00ADD190 has 107 slots;
  00ADD33C is a separate comparator pointer. Primary interface absent from ABI table.
- Candidate inventory is unverified: 100 functions / 25,579 bytes includes reach/neighbour helpers.
- Current next step: query primary/secondary vtables, audit largest bodies and lifecycle; publish
  docs/specs/win-filebrowser.md with exact inventory and explicit partial-coverage limits.
- PowerShell fallback: q.py cannot start Bash (E_ACCESSDENIED); use capped native commands.
- Named public WT1/2/3/5/6 and HD7 concern other owners; do not claim answers from FileBrowser.
- Detached HEAD is intentional; do not switch branches or update shared refs. Manifest:
  C:/Users/tlori/FreeCity4/build-runs/run4-re-filebrowser4-r2-manifest.json.

- Goal: spec ONLY cGZWinFileBrowser (owner inventory from `tools/coverage.py --owners`: 100 functions, 25579 B,
  subsystem widget toolkit) in docs/specs/win-filebrowser.md, with an §A inventory naming a section per function.
- Done: worktree created; inventory extracted (build-runs/fb.tsv, fbnames.txt).
- Next: read vtable 0x00a8d000 region + decomp/cGZWinFileBrowser.c (3101 lines) by function; write spec sections.
- Facts: ui-windows.md line ~892 lists cGZWinFileBrowser as "Not to be specified" (INVENTORY-M6 F7/F8); this lane
  was assigned it anyway by the coordinator. legacy-control.md §9.3 already covers the property interpreter
  0x00954212 (not owned here). coverage-map.tsv needs a row `cGZWinFileBrowser -> win-filebrowser.md` (tools edit,
  proposed only; this lane is docs-only).


## Coordinator quota pause (2026-10-22)
Provider weekly limit; resets 18:00 Europe/London. This round stopped. Existing work is preserved as WIP, not ready or verified. Resume only after the provider limit clears and fresh slots/check --resume pass; the round still counts.
