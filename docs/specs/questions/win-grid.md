# win-grid: open questions
<!-- Questions about [win-grid.md](../win-grid.md). Append new entries at the END; when one is answered, move it to answered/win-grid.md. Rules: README.md. -->

<!-- from: re-wingrid4 (2026-10-21) -->
- **WG2 - section 2:** (+0x110/+0x114 are header sizes, §5.1) which fields are +0x140/+0x144 (100/20): default row height and column width, or minimum/maximum sizes? [setters 0x009A6823/0x009A6830 are outside the inventory]
<!-- from: re-wingrid4 (2026-10-21) -->
- **WG3 - section 5.2:** SetCellGutters does not mark layout dirty while SetGutters does; does any caller rely on the delayed effect? [game: change cell gutters on a visible list box]
<!-- from: re-wingrid4 (2026-10-22) -->
- **WG4 - section 5.8:** SetCellExtraDataObject 0x009A6D87 stores the grid extra data (+0x268) into the cell instead of the AddRef'd object; is this ever reached in game (any caller of cIGZWinGrid slot 128)? [static: callers of slot 0x200; keep the quirk until shown unused]
