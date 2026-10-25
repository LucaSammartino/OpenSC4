| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzoutline7: the outline control `cGZWinOutline`, new spec widget-outline.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzdragdrop7) | **ready** (docs only). widget-outline.md §1–§5: fields, the 19-slot interface, polygons (setter, rotation, bounding box) and the scan-line span table, rectangle and polygon painting with the bevel colours and fill, quirks, two cases; the unattributed setter/span/SetArea/destructor code is described without being claimed; owner claimed: specified +2,808 bytes (+35 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 35 owner functions and the 11 unattributed ones next to them were read in the decompile; the shared slots and the
paint's result were checked on the instructions. The vtables `0x00AE1A78` and `0x00AE1AC8` were read. Written on
2026-10-24.

### What changed

- New docs/specs/widget-outline.md.

### Quirks found

- `SetPoints` rotates the points to start at the top-most one.
- Horizontal polygon edges add a crossing only at local extremes; the fill uses the window's fill colour.
- The polygon setter, span builder, `SetArea` and destructor are unattributed in the owner table (`0x009CBD16`…
  `0x009CC1F8`).

### Coverage

- tools/coverage-map.tsv: new row `cGZWinOutline` → widget-outline.md, before the `cGZWin*` catch-all.
- Specified bytes 4,740,322 → 4,743,130 (+2,808, 84.9%, `cov_local.py`).

### Not covered

- The draw-context slots (WGN1).
