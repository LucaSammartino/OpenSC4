| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-wingrid4: cGZWinGrid owner spec (198 functions, 32735 B) in docs/specs/win-grid.md | `re-wingrid4` / (RE lane) | in progress (round 2 of 3), docs only, no ABI change; 127/198 functions, 11433/32735 B specified (remaining 71 fn, 21302 B, listed in spec §A as `pending`); WG1 answered (DIFFERS: slot 141 is PlaySound), WG2-WG4 open | 1. Round 2: §5.9 selection/mouse/keys (AddCellToSelection, SetSelection, ActLikeAListbox, GZOnMouseDownL/UpL/Move, GZOnKeyDown/Up, DoArrowKey, DoCursorHitTest). 2. Round 3: cell data, scroll bars, layout/RecalculateAll, sort, copy/delete, GZPaint. |

## Handoff

Round 2 in progress (2026-10-22), static only (REA: no app open). Goal: §5.9 selection/mouse/keys, then helpers.
Done: §5.9 selection, hit test, notifications table, mouse down/up/move, key-up (commit "round 2 part 1").
Coverage now: 100/198 functions, 8546/32735 B (appendix.py). Next: GZOnKeyDown 0x009A8D6D, DoArrowKey
0x009A85B6, then FUN_ helpers 0x009A7E52, 0x009A8888..0x009A8C68 (region_009a0000.c has their decomp),
then sizing/data. Tools: casm.py now fetches whole functions (--max 2000). Facts: +0x130/+0x134 are
used as visible column/row counts by the hit test (ceil); hit records +0x238 (click) / +0x244 (hover).

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
