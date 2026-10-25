| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgetmisc7b: window utilities, graph decoration objects, the query route legend and the query dialog remainder, widget-misc2.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-smallhelpers7b) | **ready** (docs only). Eight owners claimed: specified +3,195 bytes (+40 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 40 functions were read in the decompile; the button helpers, the arrow drawing, the legend dispatch and
`EndDialog` in the disassembly. Looked at and left out as editor or developer-tool widgets: `cGZWinColorPicker2`
(only created by the exemplar editor's colour-sequence view) and `cSC4WinResizableOutline` (only the animation
recorder); `cGZWinCustom` is left to the upstream widget lane, whose report already proposes widget-custom.md for it.
Written on 2026-10-24.

### What changed

- New docs/specs/widget-misc2.md.

### Quirks found

- The graph arrow computes its head but never draws it, and lengthens a zero-length line in place.
- `MiscGraphTextInfo::SetFont` cannot set a colour that was never set.
- `cSC4QueryDialogWinProc::EndDialog` releases its dialog twice.
- `SetWindowNotificationTargetRecursive` leaves the starting window's own target unchanged.

### Coverage

- tools/coverage-map.tsv: `cSC4LegendDialogWinProc`, `cSC4QueryDialogWinProc` before `cSC4*WinProc`;
  `fn:SC4DoPopupStringListMenu` before `fn:SC4DoPopup*`; the `nsGZWinUtils` and four `MiscGraph…` rows of the
  attributed-owner block filled in place.
- Specified bytes 4,938,611 → 4,941,806 (+3,195, 88.5%, `cov_local.py`).
