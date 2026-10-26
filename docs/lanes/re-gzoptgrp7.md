| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzoptgrp7: the option group `cGZWinOptGrp`, new spec widget-optgrp.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzspinner7) | **ready** (docs only). widget-optgrp.md §1–§7: fields, the 30-slot `cIGZWinOptGrp` interface, flags and colour refreshes, selection and the click route, adding buttons, manual and automatic minimum sizes, the column layout, `SetArea`, fonts on demand, label creation, lifecycle, painting, quirks, three cases; owner claimed: `cGZWinOptGrp`: specified +4,946 bytes (+49 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 49 functions were read in the decompile; `SetValue`, `OptBtnAdd`, `OptBtnMoveTo`, `SetOutlineColors`, the outline
paint and the automatic size's column loop were checked on the instructions. The vtables `0x00AE1300` and
`0x00AE1288` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-optgrp.md.

### Quirks found

- The outline colours are stored and painted as the low 16 bits of the native-to-packed conversion.
- `SetFontColor` writes the colour before checking the index.
- `SetValue` with an unknown ID unchecks the selected button and keeps its ID.
- The automatic minimum and the layout break columns differently (the minimum counts the overflowing button in the
  previous column; the layout keeps a running widest button and uses the window height).
- `ClearFontColor` resets to the font default; the button class ID setter is never used.
- `0x0099BE32`, a `cGZWin` helper, is assigned to this owner.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinOptGrp` → widget-optgrp.md, before the `cGZWin*` catch-all.
- Specified bytes 4,708,330 → 4,713,276 (+4,946, 84.3% → 84.4%, `cov_local.py`).

### Not covered

- The draw-context slots (WGN1).
