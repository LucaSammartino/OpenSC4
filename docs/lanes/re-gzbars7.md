| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzbars7: the RCI demand bar and the trend bar (`cSC4WinRCI`, `cSC4WinTrendBar`), demand-trend-bars.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzscriptwin7) | **ready** (docs only). Both bars' layout, interfaces, updates, messages and drawing; owners claimed: specified +3,861 bytes (+25 functions) | RCI1 (reversed fill rectangle) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 25 functions were read in the decompile; both `GZPaint` bodies, the RCI update and message handler, and the trend
bar's `UpdateValue`, `SetDataSource` and `DoMessage` were checked on the instructions. The vtables `0x00AB8884`,
`0x00AB8614`, `0x00ABA68C` and `0x00ABA420` and the threshold floats at `0x00ABA3EC`/`0x00ABA414` were read. Written
on 2026-10-24.

### What changed

- New docs/specs/demand-trend-bars.md: the RCI bar (§1), the trend bar (§2), quirks (§3), inventory (§4).
- New docs/specs/questions/demand-trend-bars.md (RCI1); QUESTIONS.md regenerated.

### Quirks found

- The RCI bar's horizontal positive fill rectangle has left > right (RCI1).
- `SetDemandIDs` copies with no bound (16 slots); the bar interface's slots 6–7 are pure; neither bar's
  `QueryInterface` falls back to `cGZWin`'s.
- The trend bar's `SetDataSource` starts from 0.5 and leaves the target unset, so daily ticks drift the value until
  the first history record.
- The trend bar's record handler fetches the warehouse and the date and does not use them.

### Coverage

- tools/coverage-map.tsv: new rows `cSC4WinRCI` and `cSC4WinTrendBar` → demand-trend-bars.md, before `cSC4Win*`.
- Specified bytes 4,755,631 → 4,759,492 (+3,861, 85.2%, `cov_local.py`).

### Not covered

- The draw context's slots 21, 29 and 35 and the arrow and bar artwork IDs (set by the scripts that create the bars).
