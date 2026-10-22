# win-grid: answered questions

<!-- from: re-wingrid4 (2026-10-21) -->
- **WG1 - section 3:** slot 141 (0x009A824E) is named GZOnCharacter by the Mac slot match; what is it? **ANSWERED 2026-10-22 (re-wingrid4):** DIFFERS — it is PlaySound(index) (Mac name list order differs at the tail); rule in win-grid.md §5.8.
<!-- from: re-wingrid4 (2026-10-21) -->
- **WG2 - section 2:** (+0x110/+0x114 are header sizes, §5.1) which fields are +0x140/+0x144 (100/20): default row height and column width, or minimum/maximum sizes? [setters 0x009A6823/0x009A6830 are outside the inventory]
  **ANSWERED 2026-10-22 (re-wingrid4):** MATCHES — +0x140 is the default column width and +0x144 the default row height (GetColumnWidthInternal 0x009AD684, GetRowHeightInternal 0x009AE387, CalculateCellHeight 0x009AD6FA); spec §5.10.
