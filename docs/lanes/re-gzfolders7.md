| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzfolders7: the folder tabs `cGZWinFolders`/`cGZWinFolderRow` and the text ticker `cGZWinTextTicker`, new specs widget-folders.md and widget-ticker.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gztipmgr7) | **ready** (docs only). widget-folders.md: both interfaces, multi-row layout, selection, painting with the selected row in front, tab drawing; widget-ticker.md: fields, message records, the interface, scrolling and character-by-character drawing, clicks; owners claimed: `cGZWinFolders`, `cGZWinFolderRow`, `cGZWinTextTicker`: specified +3,720 bytes (+34 functions); the 29 functions of these classes in the `cGZWinCOMDirector` owner are specified there too | claim `cGZWinCOMDirector` (director functions left) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All owner functions and the 29 attributed to `cGZWinCOMDirector` (28 folder functions and the ticker constructor)
were read in the decompile; the vtables `0x00AE3FE4`, `0x00AE3AF4` and `0x00AE3A44` were read and the `<shared>` slots
disassembled. Written on 2026-10-24.

### What changed

- New docs/specs/widget-folders.md and docs/specs/widget-ticker.md.

### Quirks found

- A folder wider than the window makes the folder layout fail.
- The folders' mouse handler returns false.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinFolders`, `cGZWinFolderRow` → widget-folders.md and `cGZWinTextTicker` →
  widget-ticker.md, before the `cGZWin*` catch-all.
- Specified bytes 4,677,934 → 4,681,654 (+3,720, 83.8%, `cov_local.py`).

### Not covered

- The font and buffer slots, cited by number.
