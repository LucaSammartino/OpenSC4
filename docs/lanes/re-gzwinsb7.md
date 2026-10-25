| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinsb7: the image scroll bar `cGZWinScrollbar2`, new spec widget-scrollbar.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwintitle7) | **ready** (docs only). widget-scrollbar.md §1–§8: identity, fields and defaults, the 26-slot `cIGZWinScrollbar2` table, image sizes, `SetValue` and its notification, `SetArea`, thumb positions, part picking, mouse input with the 300/70 ms auto-repeat, `Init`/`Shutdown` with the default pictures, painting with and without images, quirks, four cases; owner claimed: `cGZWinScrollbar2`: specified +6,320 bytes (+37 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 37 functions were read in the decompile; `SetOrientation`, `SetValue`'s notification and the thumb drag were
checked on the instructions. The vtables `0x00AE4088` and `0x00AE40F0` were read and compared with the `cGZWin`
table; the class registration (`0x4B42DA5D`, `0x00998CD3`) was found from the factory. Written on 2026-10-23.

### What changed

- New docs/specs/widget-scrollbar.md.

### Quirks found

- `SetOrientation(1)` does nothing, so a horizontal bar cannot be made vertical again.
- Dragging the thumb ignores the minimum value.
- The mouse handlers return false even when they act.
- `Init` uses the window-control manager without a null check.
- Without images a horizontal bar draws up/down glyphs and highlights the opposite arrow.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinScrollbar2` → widget-scrollbar.md, before the `cGZWin*` catch-all.
- Specified bytes 4,638,207 → 4,644,527 (+6,320, 83.1% → 83.2%, `cov_local.py`).

### Not covered

- The draw-context slots 38/39/44 (WGN1 in widget-wingen.md).
