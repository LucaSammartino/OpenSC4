| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinctrl7: the window-control manager `cGZWinCtrlMgr`, new spec widget-ctrlmgr.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinli7) | **ready** (docs only). widget-ctrlmgr.md §1–§8: identity, fields and tables, the 43-slot `cIGZWinCtrlMgr` table, the control factories with their default fonts, pictures and colours, control creators, the `CreateDefaultCtrl` type table, `Init` with its 22 default colours, `Shutdown`, quirks, four cases; WGN2 answered (native colours); owner claimed: `cGZWinCtrlMgr`: specified +6,805 bytes (+48 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 48 functions were read in the decompile. `CreateOptGrp`'s area and colour pushes and the text-edit and
`0x68963E45` branches of `CreateDefaultCtrl` (including the 1.5 line-height factor at `0x00A880D4`) were checked on
the instructions. The vtables `0x00ADC128`, `0x00ADC1D4` and `0x00ADC1EC` were read. Written on 2026-10-23.

### What changed

- New docs/specs/widget-ctrlmgr.md.
- WGN2 answered: questions/answered/widget-wingen.md (new); widget-wingen.md §8.1 and its closing note updated.

### Quirks found

- `CreateOptGrp` exchanges the right and top edges of the rectangle it is given.
- `RegisterCtrlCreator` keeps the reference its interface query added.
- The resource getters return borrowed pointers; the fallback constructions do not check the allocation.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinCtrlMgr` → widget-ctrlmgr.md, before the `cGZWin*` catch-all.
- Specified bytes 4,650,381 → 4,657,186 (+6,805, 83.3% → 83.4%, `cov_local.py`).

### Not covered

- The slots of the controls it creates that have no spec yet (label, slider, old scroll bar, spinner, option
  group, bitmap, list box), cited by number.
