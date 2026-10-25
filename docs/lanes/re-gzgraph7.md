| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzgraph7: the graph base `cGZGraph` and the bar, line and scatter owners, new spec widget-graphs.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinctrl7) | **ready** (docs only). widget-graphs.md §1–§7, §9: fields and defaults, the 55-slot `cIGZGraph` table (the `G<n>` numbering graphs-window.md uses), tick divisions, the locking draw helpers, hit-test data, decoration objects, default fonts, `CompleteRedraw`'s slot sequence, the background, mouse, the bar graph's hit test and redraw, the line and scatter constructors; owners claimed: `cGZGraph`, `cGZBarGraph`, `cGZLineGraph`, `cGZScatterGraph`: specified +4,582 bytes (+74 functions) | pie graph (§8) next |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 74 functions were read in the decompile; the vtables of the four classes (window, `cIGZGraph`, data interface)
were read with their owners, and the constants `FLT_MIN` (`0x00B17C1C`), 0.5 (`0x00A84D2C`) and 2³² (`0x00A80AA8`)
and the mouse-move call were checked on the bytes. Written on 2026-10-23.

### What changed

- New docs/specs/widget-graphs.md (§8 reserved for the pie graph).

### Quirks found

- `AllocateHitTestData` leaks the old block unless the new size is larger.
- `SetMinorTickUsage` and `SetMinorTickGridUsage` treat only 1 as on.
- The mouse-move hit test has no effect.

### Coverage

- tools/coverage-map.tsv: row `cGZGraph` now points to widget-graphs.md; new rows `cGZBarGraph`, `cGZLineGraph`,
  `cGZScatterGraph`, before the `cGZ*Graph` catch-all.
- Specified bytes 4,657,186 → 4,661,768 (+4,582, 83.4% → 83.5%, `cov_local.py`).

### Not covered

- The `<shared>` drawing slots 153–177 and the subclasses' data interfaces (out of scope as `<shared>` or not in
  these owners), cited by slot.
