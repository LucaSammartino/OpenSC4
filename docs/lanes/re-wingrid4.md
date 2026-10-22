| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-wingrid4: cGZWinGrid owner spec (198 functions, 32735 B) in docs/specs/win-grid.md | `re-wingrid4` / (RE lane) | wip after round 2 of 3, docs only, no ABI change; 127/198 functions, 11433/32735 B specified (remaining 71 fn, 21302 B, listed in spec §A as `pending`); WG1 answered (DIFFERS: slot 141 is PlaySound), WG2-WG4 open | Round 3 (last): cell data, scroll bars, layout/RecalculateAll, sort, copy/delete, GZPaint. |

## Handoff (round 3)

Round 3 in progress (2026-10-22), static only. §5.10 scroll bars/RecalculateAll/sizes, §5.11 cell data/windows/
copy/MakeRoom (f5f77b85), §5.12 GZPaint (9fcde01a), §5.13 rect/delete/copy committed; WG2 answered. Coverage 156/198, 20021/32735 B; remaining 42 functions, 12714 B.
Next: GZPaint 0x009AF168 + FUN_009A7E52 (§5.12), Sort/Copy/Delete rows+columns, SetCellFormat family,
SetDefaultFormatting, ScrollCells*, GetCellRect, ResizeColumnToFitCellWindows, remaining FUN_ helpers.
Tools: build-runs/cdec.py <decompfile> <addr> (compact decomp), rdf.py VA[:f|d|i|s] (read constants),
appendix.py (status.tsv -> §A). Round 3 is the last: park if not complete.

## Report (round 2)

Round 2 done (2026-10-22), static only (REA: no app open, every call failed "No app is open").
Done: 227181c2 §5.9 selection (AddCellToSelection, SetSelection, ActLikeAListbox), DoCursorHitTest,
notification-id table, GZOnMouseDownL/UpL/Move, GZOnKeyUp; 0cffe427 GZOnKeyDown, DoArrowKey, 25 container
and stable-sort helpers in §5.7, layout rows corrected (+0x130/+0x134 visible counts, +0x170 timer, +0x238/+0x244
hit records). Coverage (appendix.py): 127/198 functions, 11433/32735 B; remaining 71 functions, 21302 B.
Next (round 3, last): FUN_009A7E52 (876 B, GZPaint helper, decomp in decomp/_unnamed/region_009a0000.c:4070)
with GZPaint 0x009AF168; scroll bars (AddScrollBar, RemoveScrollBar, SetScrollBarValues/Positions,
TestForPossibleScrollBarAdditionOrRemoval); layout (RecalculateAll, MakeRoom*, CalculateCellHeight,
Get*Internal, SetSizeCachedActual*); cell data (SetCellTextValue/Bitmap/Window, CopyCellData x2, ClearCell,
Dispose*); sort; copy/delete rows/columns; SetCellFormat family; SetDefaultFormatting; FUN_ 0x009A5FE3,
0x009A60BB, 0x009A6C45, 0x009AA91A.. (PENDING=1 python build-runs/appendix.py lists them).
Facts: helper decomp for FUN_ addresses lives in region_009a0000.c (grep "^// ---- 009a...."); casm.py fetches
whole functions (--max 2000); window-view decomp offsets are grid offsets minus 4. WG2 (+0x140/+0x144) and
WG4 still open (no new byte/caller evidence this round); WG3 open. Flag 0x1000 meaning (ActLikeAListbox) and
window flag 2 (input gate) not measured.

## Report (round 1)

Round 1 (2026-10-21/22). REA had no app open; everything is static (decomp/cGZWinGrid.c, re_query.py asm,
raw read of the cIGZWinGrid vtable 0x00ADD578, Mac slot names from re/symport/vtables.pkl).

What changed:
- New docs/specs/win-grid.md: identity (§1), object layout from the constructor (§2), the full 142-slot
  cIGZWinGrid table with own/other marks (§3), the grid flag word bits measured so far (§4), owner methods
  §5.1 lifecycle/areas/flags/focus/wheel, §5.2 fonts/colours/gutters/outline, §5.3 scroll shortcuts/selection
  rules/limits, §5.4 width/height sums, §5.5 extra data and headings, §5.6 notifications/buffer/sounds/drop-down,
  §5.7 forwarding thunks and container helpers, §5.8 format matrices/scroll-to/cell windows/PlaySound,
  §6 six planned conformance vectors, §A the 198-row inventory generated with per-function section or `pending`.
- New docs/specs/questions/win-grid.md (WG2, WG3, WG4) and answered/win-grid.md (WG1); QUESTIONS.md regenerated.

Measured coverage (spec §A, generated): specified 92 of 198 functions, 5288 of 32735 bytes; remaining 106
functions, 27447 bytes. Forwarding (no own behaviour): 6 cGZWin-view thunks; the rest are owner algorithms
or template helpers.

Notable quirks recorded: ScrollPages* = 4 cells; SetCellGutters does not recalc; SetExtraData leaks a
previous object; SetCellExtraDataObject stores the grid extra data instead of the passed object (WG4);
SetSoundID accepts negative indices; PauseRecalculations has no floor; Get*ExtraData/Get*Heading insert
missing keys.

Next (round 2): regenerate build-runs tools if lost (see below), then specify selection and input
(0x009AA477, 0x009A9765, 0x009AA3E1, 0x009A9F6A, 0x009A7826, 0x009A79E6, 0x009A8D6D, 0x009A77ED, 0x009A85B6,
0x009A7C69) and their FUN_ helpers near 0x009A88DF-0x009A9EE3; round 3: cell data (SetCellTextValue,
SetCellBitmap, SetCellWindow, CopyCellData, ClearCell, Dispose*), scroll bars (Add/Remove/SetScrollBar*),
layout (RecalculateAll, MakeRoom*, CalculateCellHeight, Get*Internal, SetSizeCachedActual*), sort, copy/delete,
SetCellFormat family, SetDefaultFormatting, GZPaint (2813 B).

Tools (gitignored, in build-runs/): inv.py -> inv-named.tsv (names + slots); vtdump.py 00add578 00add7b0 >
vt0.txt; macmap.py -> slotmap.tsv; casm.py addr... (one-line compact asm); status.tsv (address<TAB>section)
and appendix.py (regenerates spec §A, prints coverage; PENDING=1 lists pending rows).
