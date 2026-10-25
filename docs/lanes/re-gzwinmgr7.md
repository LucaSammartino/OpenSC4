| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinmgr7: the window manager (`cGZWinMgrW32`, `cGZWinMgrBase`, `cGZWin32COMDirector`), new spec widget-winmgr.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzcombo7) | **ready** (docs only). widget-winmgr.md §1–§10: fields, lifecycle, the 64-slot `cIGZWinMgr` table (modal slots referenced to modal-dialogs.md), posted messages and broadcasts, deferred destruction, `GZWinMsgPost` mapping, the three input targets, mouse and key routing, capture, focus, removal, the keyboard state, the Win32 COM director, quirks, three cases; WT5 and WT6 answered; owners claimed: `cGZWinMgrW32`, `cGZWinMgrBase`, `cGZWin32COMDirector`: specified +8,618 bytes (+110 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 110 functions were read in the decompile; both constructors, the capture, focus, removal, mouse and destroy paths,
`ProcessPostedMessages`, `ChildDeleteAll` and the keyboard-state slots were checked on the instructions. The vtables
`0x00AE46E8`, `0x00AE46B4`, `0x00AE469C`, `0x00AE466C`, `0x00AE467C`, `0x00AE468C` and `0x00AC5268` were read, and the
`cIGZWinMgr` rows of docs/abi/interfaces.tsv followed. Written on 2026-10-24.

### What changed

- New docs/specs/widget-winmgr.md.
- WT5 and WT6 answered (moved to questions/answered/widget-toolkit.md); widget-toolkit.md §4.2, §8.1, the case list
  and §11 updated; QUESTIONS.md regenerated.

### Quirks found

- While the busy count is above 0 (a broadcast is running) `DestroyWindow` leaves the window attached, so
  `ChildDeleteAll` would not terminate then (WT5).
- `SetFocus(null)` is refused, so focus is never cleared by it; windows without flag `0x8000` cannot take the focus
  (partly answers WT3: `0x8000` is read by the manager).
- Mouse events outside the modal window are dropped without updating the window under the mouse (byte `+0x1A9` 0).
- `ProcessPostedMessages` and `DestroyCondemnedWindows` use process-wide counters.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinMgrW32`, `cGZWinMgrBase`, `cGZWin32COMDirector` → widget-winmgr.md, before
  the `cGZWin*` catch-all.
- Specified bytes 4,718,096 → 4,726,714 (+8,618, 84.5% → 84.6%, `cov_local.py`).

### Not covered

- The canvas and its message interface, the cursor manager, the `0x2AC4516B` object made by the director, and the
  input event objects beyond the getters used.
