| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzpie7: the pie graph `cGZPieGraph`, widget-graphs.md §8 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzgraph7) | **ready** (docs only). widget-graphs.md §8: identity and fields, the 31-slot `cIGZPieGraph` table, geometry (masks, disc edge points, text positions), flat and 3D drawing with flood-filled slices and percentage texts, fonts, the scatter graph's point-series code folded into this owner, quirks and cases; owner claimed: `cGZPieGraph`: specified +7,833 bytes (+78 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 78 functions were read in the decompile. The disc-edge maths, the text-position factor, the normalisation, the
lock flags of `DrawGraphData` and the float constants (2π, π/2, 0.5, 0.6, 0.8, 0.1, 0.01, 1e-4, ±`FLT_MAX`, 100) and
the percentage format `"%._f%%"` were checked on the bytes. The pie vtables `0x00ADEEC0`, `0x00ADEDE0` and
`0x00ADF188` were compared with the base graph's; the scatter vtables show where the folded functions are reached.
Written on 2026-10-24.

### What changed

- widget-graphs.md: new §8, scope line and §9 updated.

### Quirks found

- `InsertData` with an existing index overwrites it; `SetData`/`RemoveData` with `−1` act only with one value.
- The 3D style ignores the minimum fraction for texts.
- The second radial pass locks with `0x8200` and unlocks with `0x8004`.
- Slices are filled by flood fill from the text point.

### Coverage

- tools/coverage-map.tsv: new row `cGZPieGraph` → widget-graphs.md, before the `cGZ*Graph` catch-all.
- Specified bytes 4,661,768 → 4,669,601 (+7,833, 83.5% → 83.6%, `cov_local.py`).

### Not covered

- The default series table at `[0x00BAC08C]` (data), the font and buffer slots, and the scatter data interface
  slots outside this owner, cited by number.
