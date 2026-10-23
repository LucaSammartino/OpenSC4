| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-powerpoles: the power pole and power line occupants, folded into network-occupants.md §4 and §5 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-ugbase) | **ready** (docs and RE tables only). network-occupants.md §4 and §5 rewritten from the instruction bytes with object offsets, every pole slot named, the full tuning and the pole and line records; the wire geometry left to power-line-tool.md; NO2 and NO13 answered, NO36–NO37 filed; power.md's pole slot 13 named; 77 names (34 new, 43 renamed). Specified coverage +2,334 B / +12 functions | NO36 (the pole's saved `+0x110`), NO37 (the pole's floor, walls, draw passes, bounds, pick and cell walk in detail) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), read
with the local disassembler and the Ghidra export, and the shipped "Utilities" exemplar for the tuning values. Written
on 2026-10-21 as a separate `power-poles.md` and held back when `main` gained network-occupants.md §4–§5 and
power-line-tool.md; this is that work folded into network-occupants.md.

### What changed

- **network-occupants.md §4** (the pole): identity and QI (including the `0x26A68448` forwarding), sub-objects and
  fields with object offsets; all 19 slots with pop sizes and the other members; the tuning (eight properties, two
  of them read by `StaticInit`), the static set-up, both `Init` forms, the message target, `SetDefaultExemplar` and
  `Shutdown`; the 13-row record and its shared-table quirk; the connection record, the attachment tables A and B,
  removing connections, the cells under a wire; position and floor height, model and zoom, distress, drawing.
  `AddConnection`, the direction helper, the sag and the Bézier sampling are left to power-line-tool.md §5.2–§5.5,
  which already gives them in more detail.
- **network-occupants.md §5** (the line): identity, object offsets, slots and the record as read in the bytes
  (`0x0064A310`: 3 × f32, 2 × s32, a u8, two refs).
- **Corrections**: view offsets in the earlier pole and line tables and records; `+0xBC` is the model instance (not a
  delegate of unknown kind); `+0xC0`/`+0xC4` the view rotation and zoom; slots 5, 9, 13, 16, 17 named
  (`GetLineCells`, `InsertLines`, `IsPreview`, `GetLineCellsTo`, `GetModelInstances`); slot 10 returns the tuning
  `0x0A2ECED8` (`0x0064CE15`, `0x0064DD60`), not anything derived from `0xE9A18D3F` (test 17 fixed); quirk 11: a
  connection's pole and lines are read with one IID each.
- Quirks 21–25, tests 35–39 added; §12's NO2 updated.
- **power.md** §5.1 and §13 Q1: pole slot 13 is `IsPreview`.

### Names and coverage

- re/symbols/power-poles.tsv: 77 names (34 new; 43 replace auto names: `AddConnection` / `AttachLines` moved from the
  power line tool, and seven functions named for `cSC4LandfillOccupant` / `cSC4LotRetainingWallOccupant`, whose
  vtables were mislabelled as power line). The `names_windows.tsv` delta (−43 +77) is identical to
  `merge_names.py`'s; no upstream manual name is replaced.
- tools/coverage-map.tsv: `cSC4PowerPoleOccupant::tConnection`, `fn:GZCOM_CreatecSC4PowerPoleOccupant`,
  `fn:GZCOM_CreatecSC4PowerLineOccupant` → network-occupants.md.

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-ugbase 3,235,510 bytes in 14,638
functions; with this lane 3,237,844 bytes in 14,650 functions (+2,334 B, +12 functions; denominator unchanged).
`docs/coverage-report.md` was not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/network-occupants.md: NO2 (the mesh container, forwarded to the model
  instance), NO13 (the line's offsets and record).
- Filed: NO36 (the saved `+0x110`), NO37 (the floor, walls and draw passes in detail).
