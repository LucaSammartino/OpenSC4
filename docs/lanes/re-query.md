| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-query: the query tool's dialog fillers, other dialogs, building values and route mode, folded into city-tools-input.md §8 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-render3d) | **ready** (docs and RE tables only). city-tools-input.md §8.12.3's table named and typed, §8.12.4 (what all ~100 entries show), §8.13 (the query sound, zone-based and neighbour dialogs, the dialog's win proc), §8.14 (building values, subjective ratings), §8.15 (route mode, outline) added, two corrections (§8.6, §8.12.1); the query tool claimed as specified; CT8, CT10, CT12 answered, CT9 narrowed, CT14–CT15 filed; 177 names (104 new, 73 renamed). Specified coverage +76,933 B / +217 functions | CT14 (the neighbour dialog, the MySim icon, the debug writers), CT15 (route mode), CT9 |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as a separate `query-tool.md` and held back when `main` gained
city-tools-input.md §8 (re-querytool4, three rounds, parked with the fillers and builders open, CT12); this is that work
folded into §8.

### What changed (docs/specs/city-tools-input.md)

- §8.12.3: the control table's fillers carry their names and kinds.
- §8.12.4 (new): the entry kinds (number, percent, money, text, meter, slider with live write-back, load meter,
  per-unit cost, levels) and what each id shows, from which simulator slot; the goofy stat.
- §8.13 (new): `PlayOccupantQuerySound`; `DoZoneBasedQueryDialog` (airport, seaport, landfill windows and their
  handler `SetOrGetControlValue`); `DoNeighborConnectionQueryDialog` (summary); `cSC4QueryDialogWinProc` (end on
  `0x287259F6`, live sliders on `0x887113A3`).
- §8.14 (new): capacity, occupancy, the mayor rating percent, the wealth level, the subjective ratings and their seven
  rating functions.
- §8.15 (new): route mode, outline.
- Corrections, read in the bytes: §8.6 step 5, `DescribePick` runs unless Shift **found an automaton** (the pointer
  `0x004D4F43` tests), not whenever Shift is up; §8.12.1 step 3, the pick filter is `cSC4QueriableOccupantFilter`
  (vtable `0x00A90B18`, occupant-manager.md §13.2.6), not an accept-all filter.
- §4 quirks 6–8, §6 tests 11–14, §7: the query tool and `cSC4QueryDialogWinProc` are now claimed as specified.

### Names and coverage

- re/symbols/query-tool.tsv: 177 names (104 new; 73 replace auto names). The `names_windows.tsv` delta (−73 +177) is
  identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: `cSC4ViewInputControlQuery`, `cSC4QueryDialogWinProc`, `cSC4QueryAutomatonFilter`,
  `cSC4QueryStoplightFilter` → city-tools-input.md (the route filter is already owned by `cSC4*OccupantFilter`). Also
  fixed two rows that matched no owner (coverage.py names a file-static function `fn:<name>`):
  `fn:ConvertV5FlagsToCurrentFlags` (re-ugbase's) and `fn:AddOptionToSet` (graphics rules).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-render3d 3,353,863 bytes in 15,045
functions; with this lane 3,430,796 bytes in 15,262 functions (+76,933 B, +217 functions; denominator unchanged).
`docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/city-tools-input.md: CT8 (Shift and Ctrl), CT10 (`ToggleTractTracking`, an
  empty body), CT12 (the remainder).
- Narrowed: CT9 (city slots 43 and 40 named, the automaton filter's types; the two called slots stay open).
- Filed: CT14, CT15.
