| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzsmallwin7: small game windows and the remaining region dialogs (credits, clock, splash, height-field loader, missing plugin packs, transparent window, panel buttons, game delay, tertiary frame, situation face, load/delete region, delete city, the dialog base) — small-windows.md, advisor-face.md §8, region-city-dialogs.md §9 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzsc4text7) | **ready** (docs only). Owners claimed: specified +8,363 bytes (+67 functions), including the two popup-safety free functions already described by advisors.md | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the decompile; the mangled bodies (credits `InitDialog`, the clock's update and opener,
the splash `DoMessage`, the height-field `Init`, the load-region attach and OK, the delete-region attach) were checked
on the instructions; the credits table `0x00AB4AEC`, the HTML size array `0x00AB4AD0`, the filter strings and the
dialog vtables were read. Written on 2026-10-24.

### What changed

- New docs/specs/small-windows.md (§1–§9).
- advisor-face.md §8 (`cSC4SituationUIImageWinProc`); region-city-dialogs.md §9 (base bodies, load-region,
  delete-region and delete-city dialogs) and its §8 "not covered" sentence.

### Quirks found

- The splash skips on a click but not on a key; it clears the screen on its first two paints.
- The load-region dialog's Cancel returns 0, the same as region 0.
- The clock's opener returns false when it only re-shows the clock.
- The height-field OK button is ignored while a folder is selected.

### Coverage

- tools/coverage-map.tsv: 17 new rows (before `cSC4*WinProc`, `cSC4Win*` and `cSC4*Dialog`).
- Specified bytes 4,769,889 → 4,778,252 (+8,363, 85.4% → 85.6%, `cov_local.py`).

### Not covered

- The legend and query dialogs of the query tool (`cSC4LegendDialogWinProc`, `cSC4QueryDialogWinProc`): left to the
  upstream city-tools lanes (CT7). The tooltip manager and progress screen: upstream ui-feedback-owners.md.
