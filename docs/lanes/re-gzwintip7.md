| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwintip7: the tooltip window `cGZWinToolTip`, new spec widget-tooltip.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwintext7) | **ready** (docs only). widget-tooltip.md §1–§5: identity and fields, the 6-slot `cIGZWinToolTip` table, tip info, colour mapping, text shortening, the five tooltip messages, rendering into the 256 × 64 buffer, the 13 placement flags and the main-window clamp, painting, lifecycle, quirks, three cases; owner claimed: `cGZWinToolTip`: specified +2,269 bytes (+13 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 13 functions were read in the decompile; `SetFont`'s comparison, the colour mapper and the line separator were
checked on the bytes. The vtables `0x00AE4380` and `0x00AE4398` were read and compared with the `cGZWin` table.
Written on 2026-10-24.

### What changed

- New docs/specs/widget-tooltip.md.

### Quirks found

- `SetFont` compares both margins with the stored x margin.
- `CopyTipInfo` erases one character more than fits, and leaves a text that fits nowhere unchanged.
- Messages other than the five tooltip ones return false without the base dispatch.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinToolTip` → widget-tooltip.md, before the `cGZWin*` catch-all.
- Specified bytes 4,672,155 → 4,674,424 (+2,269, 83.7%, `cov_local.py`).

### Not covered

- The font and buffer slots, cited by number.
