# win-grid: open questions
<!-- Questions about [win-grid.md](../win-grid.md). Append new entries at the END; when one is answered, move it to answered/win-grid.md. Rules: README.md. -->

<!-- from: re-wingrid4 (2026-10-21) -->
- **WG1 - section 3:** slot 141 (0x009A824E) is named GZOnCharacter by the Mac slot match, but GZOnCharacter is a cGZWin-view method (0x009A5F1E); what interface method is slot 141? [static: read its body; Mac vtable has 156 slots vs 142 here]
<!-- from: re-wingrid4 (2026-10-21) -->
- **WG2 - section 2:** which fields are +0x110/+0x114 (20/80) and +0x140/+0x144 (100/20): default row height and column width, or minimum/maximum sizes? [setters 0x009A6823/0x009A6830 are outside the inventory]
<!-- from: re-wingrid4 (2026-10-21) -->
- **WG3 - section 5.2:** SetCellGutters does not mark layout dirty while SetGutters does; does any caller rely on the delayed effect? [game: change cell gutters on a visible list box]
