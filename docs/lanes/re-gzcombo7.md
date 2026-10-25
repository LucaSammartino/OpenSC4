| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzcombo7: the combo box `cGZWinCombo`, new spec widget-combo.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzoptgrp7) | **ready** (docs only). widget-combo.md §1–§7: fields, the 42-slot `cIGZWinCombo` interface, selection and the changed message, the added window slots, the child text edit, layout, minimum size and `SetArea`, the arrow image, keys, messages, enable handling, lifecycle and defaults, painting, the modal drop-down list, quirks, three cases; owner claimed: `cGZWinCombo`: specified +4,820 bytes (+55 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 55 functions were read in the decompile; `SetEditable`, the window `SetFlag`, the key handler, the paint, the
changed message and the text-edit slots were checked on the instructions. The vtables `0x00AE2970`, `0x00AE2BE8` and
the text edit's `0x00ADFA68` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-combo.md.

### Quirks found

- `SetArea` applies the unclamped rectangle when the clamped size equals the current size.
- The window `SetFlag(2, …)` has no null test for the text edit.
- `ClearFontColor` sends the unset sentinel to the text edit as a colour.
- String insertion and removal do not move the selected index.
- `SetFont` and `SetComboDownArrow` release the old object before AddRef'ing the new one.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinCombo` → widget-combo.md, before the `cGZWin*` catch-all.
- Specified bytes 4,713,276 → 4,718,096 (+4,820, 84.4% → 84.5%, `cov_local.py`).

### Not covered

- The list box's own slots (widget-controls.md §5, WC1) and the draw-context slots (WGN1).
