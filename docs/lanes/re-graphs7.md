| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-graphs7: the city graphs panel `cSC4GraphsViewWinProc` and its line graph `cSC4LineGraph`, new spec graphs-window.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-minimap7) | **ready** (docs only). graphs-window.md §1–§9: identity, lifetime and layout, the series record, the graph exemplars and the shipped graphs, the time scales, showing and the button table, `ShowGraph` (style, bar and line set-up, the legend), `RefreshGraph` (bar values and the log scale, line samples and operations, the y-axis step), the widget slots used, helpers, quirks, five conformance cases; questions GW1–GW4; owners claimed: `cSC4GraphsViewWinProc`, `cSC4LineGraph`: specified +11,692 bytes (+37 functions) | GW1–GW4 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 37 claimed functions were disassembled; the decompile of this owner reads `this` fields as globals, so every
body was taken from the disassembly. The 18 shipped graph exemplars were read from `SimCity_1.dat` with
`tools/dbpf.py`. Written on 2026-10-23. No upstream lane covers these owners.

### What changed

- New docs/specs/graphs-window.md.
- Read on the bytes:
  - the panel layout and the 0x30-byte series record;
  - the graph exemplar properties and the six time scales at `0x00B0DA50`;
  - the button and slider table of `DoWinProcMessage`;
  - the widget style colours and fonts, the bar and line plot areas, the legend layout and its checkboxes;
  - the bar log scale for operation 5, the line date window (`today − 4` rounded up to the step), the four line
    operations and the y-axis step choice (`(r + 9) / 10` per decade);
  - `cSC4LineGraph`: it overrides only the line draw (antialiased) and its interface's QueryInterface thunk.
- docs/specs/questions/graphs-window.md: GW1–GW4 (new prefix GW); QUESTIONS.md regenerated.

### Quirks found

- Pie graphs are made but never filled.
- The bar refresh makes an unused date.
- Only the low byte of a `0x4xxx` button id selects the graph.
- A bar series fills only its own group.
- Line samples before the city's start date are 0.
- One or two series get no legend checkboxes, so they cannot be hidden.

### Coverage

- tools/coverage-map.tsv: `cSC4GraphsViewWinProc` and `cSC4LineGraph` → graphs-window.md, before the
  `cSC4*WinProc` catch-all.
- Specified bytes 4,455,708 → 4,467,400 (+11,692, 79.8% → 80.0%, `cov_local.py`, on `5c0ff782` + the six lanes
  before).

### Not covered

- The graph widgets `cGZGraph`, `cGZLineGraph`, `cGZBarGraph`, `cGZPieGraph` (GW1).
- The history warehouse's data (history.md).
