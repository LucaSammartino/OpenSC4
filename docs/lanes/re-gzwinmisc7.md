| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinmisc7: the bitmap window and the window toolkit's small owners (`cGZWinBMP`, `cRZCursorManager`, `cGZWinTreeViewDrag`, `cGZWinDlg`, `fn:unscript`, `fn:script_tag_*`, `fn:GetGZWinFont`, three constructors) — widget-bmp.md, widget-misc.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzregion7) | **ready** (docs only). Owners claimed: specified +4,473 bytes (+77 functions). With this lane every in-scope widget-toolkit owner has a spec; HTML, IME, Lua and developer windows stay out of scope | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All functions were read in the decompile; the vtables `0x00ADF66C` (cIGZWinBMP) and `0x00AC5290` (cursor manager),
the tree-view drag's mouse move, `cGZWinDlg::GZOnKeyDown` and `GetGZWinFont` were checked on the instructions.
Written on 2026-10-24.

### What changed

- New docs/specs/widget-bmp.md and docs/specs/widget-misc.md.
- widget-toolkit.md coverage-status rows updated.

### Quirks found

- `SetWinBMPFlag` does not redraw; the bitmap window's alpha pair is stored but not used by its paint.
- The cursor manager's `SetCursor(null)` dereferences null; `ShowCursor`/`HideCursor` loop the Win32 counter.
- A bitmap window that passes the mouse to its parent still runs `cGZWin`'s own move/press handling first.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinBMP`, `cGZWinTreeViewDrag`, `cGZWinDlg`,
  `cGZWinTreeView::GZWinTreeView_TreeNode`, `cGZWin::ChildList`, `cGZWinTextEdit::UndoRedoAction` (before `cGZWin*`);
  the rows `fn:unscript`, `fn:script_tag_*` (legacy-control.md, widget-misc.md), `fn:GetGZWinFont` and
  `cRZCursorManager` now name a spec.
- Specified bytes 4,782,755 → 4,787,228 (+4,473, 85.6% → 85.7%, `cov_local.py`).

### Not covered

- `cWinBMPView` beyond the four setters in this owner; the cursor classes `cRZCursor` and `cRZSystemCursorW95`.
