| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzdragdrop7: the drag-and-drop manager `cGZWinDragAndDropManager`, new spec widget-dragdrop.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzcheats7) | **ready** (docs only). widget-dragdrop.md §1–§6: interfaces and fields, the `DragData` record, the 14-slot interface, the six drag messages, the proxy filter, the drag window's input, threshold, image capture, placement and target search, quirks, three cases; owner claimed: specified +3,518 bytes (+48 functions) | WDD1 (threshold test) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 48 functions were read in the decompile; the proxy filter, the drag threshold, both proxy-registration slots and
the constructor were checked on the instructions. The vtables `0x00AE2018`, `0x00AE2008`, `0x00AE1FD4`, `0x00AE1FBC`
and `0x00AE1D60` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-dragdrop.md and docs/specs/questions/widget-dragdrop.md (WDD1); QUESTIONS.md regenerated.

### Quirks found

- The 3-pixel drag threshold does not use the grab point (WDD1).
- Registering a new drop source proxy returns false.
- `Drop` keeps the drag window's capture until the next move.
- Any key on a proxy window cancels a drag; on the drag window only Escape does.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinDragAndDropManager` → widget-dragdrop.md, before the `cGZWin*` catch-all.
- Specified bytes 4,736,804 → 4,740,322 (+3,518, 84.8% → 84.9%, `cov_local.py`).

### Not covered

- The `DragData` constructor/destructor (other owners) and the buffer slots of the image copy.
