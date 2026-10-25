| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzdraw7b: span lists and the RGB colour converter (`cRZSpanList`, `fn:SC4CreateSpanList`, `cRZRGBColorConverter`), rz-draw.md §4–§5 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `14e0416c`, after re-rzdraw7) | **ready** (docs only). The span list's vector, ellipse, draw and fill; the 32-bit run scanner; the colour formats and table converter; owners claimed: specified +6,026 bytes (+19 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All functions were read in the decompile; `cRZSpanList::Draw` was checked on the instructions and the ellipse
constants `0x00A84D2C` (0.5) and `0x00AA74E8` (0.25) were read. The polygon fill stays in upstream's gz-graphics.md
§3.2. `cRZTileSetup` is left out: its only callers are the SimGL buffer cache (3D driver, out of scope). Written on
2026-10-24.

### What changed

- rz-draw.md §4 (span lists) and §5 (colour converter); scope and inventory updated.

### Quirks found

- `Draw` stops at the first span at or below the clip bottom, so spans must be in row order.

### Coverage

- tools/coverage-map.tsv: `cRZSpanList` → gz-graphics.md, rz-draw.md; `cRZRGBColorConverter`,
  `cRZRGBColorConverter::Format` and `fn:SC4CreateSpanList` → rz-draw.md.
- Specified bytes 4,829,784 → 4,835,810 (+6,026, 86.6%, `cov_local.py`).

### Not covered

- `cRZTileSetup` (driver texture tiling).
