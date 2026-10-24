| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzdraw7: the RZDraw 2D drawing helpers (`fn:RZDraw*`, `fn:drr_get_color`, `fn:Draw*`), rz-draw.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `14e0416c`, after re-gzwinmisc7) | **ready** (docs only). Outlines, raised rectangles, nine-slice and tiled/stretched blits, the circle and the buffer line; owners claimed: specified +13,078 bytes (+12 functions) | RZD1 (exact ring and line pixel sets) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 12 functions were read in the decompile; `RZDrawRectangleOutline`, `RZDraw3DRectangleOutline`,
`RZDrawRaisedRect`, `drr_get_color` and `DrawTriangle` were followed on the instructions, the others at summary level
where marked. Written on 2026-10-24.

### What changed

- New docs/specs/rz-draw.md and docs/specs/questions/rz-draw.md (RZD1); QUESTIONS.md regenerated.

### Quirks found

- `RZDrawRectangleOutline` leaves out the top-right pixel and only works for thickness 1.
- `RZDrawDrawCircle` always writes 16-bit pixels.
- `DrawRectangleOutline` stops at the first failing line.

### Coverage

- tools/coverage-map.tsv: new rows `fn:RZDrawBlt*` (before `fn:*Blt*`) and `fn:drr_get_color`; `fn:RZDraw*` and
  `fn:Draw*` now name rz-draw.md.
- Specified bytes 4,816,706 → 4,829,784 (+13,078, 86.3% → 86.5%, `cov_local.py`).

### Not covered

- The draw-context and buffer slots used (cited by number); `cRZSpanList` (next lane); `cGZAlphaBlt` and the other
  2D blitters (out of scope).
