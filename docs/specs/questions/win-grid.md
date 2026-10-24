# win-grid: open questions
<!-- Questions about [win-grid.md](../win-grid.md). Append new entries at the END; when one is answered, move it to answered/win-grid.md. Rules: README.md. -->

<!-- from: re-wingrid4 (2026-10-21) -->
- **WG3 - section 5.2:** SetCellGutters does not mark layout dirty while SetGutters does; does any caller rely on the delayed effect? [game: change cell gutters on a visible list box]
<!-- from: re-wingrid4 (2026-10-22) -->
- **WG4 - section 5.8:** SetCellExtraDataObject 0x009A6D87 stores the grid extra data (+0x268) into the cell instead of the AddRef'd object; is this ever reached in game (any caller of cIGZWinGrid slot 128)? [static: callers of slot 0x200; keep the quirk until shown unused]

<!-- from: core-widgets5 round 2 (2026-10-23) -->
- **WG5 - sections 3, 5.1, 5.9, 5.10 and 5.11 dependency bodies:** what are the bounds/empty-used-range rules, exact GetCellIsSelected matching predicate, relative-scroll clamping, and effective cell-format precedence/merge rules? These slots are named but lack body contracts needed by the newly specified owner methods. [literal provisional core: nonnegative indices, limits enforced only with fixed-range bits, -1 unlimited; empty range zeros; selected means exact pair; relative scroll adds then uses ScrollTo's upper clamp; font precedence cell/row/column/default; no closure claimed for these dependencies]

<!-- from: core-widgets5 round 2 (2026-10-23) -->
- **WG6 - sections 5.1, 5.12 and 5.14:** please settle the pending SetDefaultFormatting initializer, horizontal text positioning for alignments 3/4/5, and format-copy size-word paragraph. Section 5.7 says 009A5C4A resets both size words to -1 but 5.14 still marks these writes pending; should 5.7 govern SetCellFormat too? [zero-initialized unresolved format words are provisional; do not claim Init/paint/format-copy closure until specified]
