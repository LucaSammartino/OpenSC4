| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwintitle7: the general window's title bar `cGZWinGenTitleBar`, widget-wingen.md §13 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwingen7) | **ready** (docs only). widget-wingen.md §13: fields and interfaces, the title-bar flags and `SetTitleBarFlag`, `Init` with the system-button strip and its white placeholder, `create_and_add_sys_btn`, `Shutdown`, minimum size, `AutoSize`, `SetArea`, the font, the shortened display caption, painting, button placement, the 6-slot `cIGZWinGenTitleBar` table, quirks and cases; owner claimed: `cGZWinGenTitleBar`: specified +3,168 bytes (+23 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 23 functions were read in the decompile; `GZPaint`'s text call, `place_controls` and the ellipsis step of
`calc_display_caption` were checked on the instructions. The vtables `0x00ADC398` (window, compared with the flat
rectangle's `0x00AE20A0`), `0x00ADC348` and `0x00ADC330` were read. Written on 2026-10-23.

### What changed

- widget-wingen.md: scope line for the third owner, new §13, §12 points to §13.6.

### Quirks found

- A title bar without a font ignores `AutoSize`.
- A failed `ChildAdd` of a system button leaves a released pointer in its field.
- Flag `0x1000` and the `+0xFC` caption colour cannot be reached from the general window.
- The caption is cleared when even the ellipsis does not fit.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinGenTitleBar` → widget-wingen.md, after the `cGZWinGen` row.
- Specified bytes 4,635,039 → 4,638,207 (+3,168, 83.0% → 83.1%, `cov_local.py`).

### Not covered

- The font object's slots (37, 42, 48, 51) and the control manager's button factory (slot 10), cited by number.
