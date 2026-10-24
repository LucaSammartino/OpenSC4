# win-grid: answered questions

<!-- from: re-wingrid4 (2026-10-21) -->
- **WG1 - section 3:** slot 141 (0x009A824E) is named GZOnCharacter by the Mac slot match; what is it? **ANSWERED 2026-10-22 (re-wingrid4):** DIFFERS — it is PlaySound(index) (Mac name list order differs at the tail); rule in win-grid.md §5.8.
<!-- from: re-wingrid4 (2026-10-21) -->
- **WG2 - section 2:** (+0x110/+0x114 are header sizes, §5.1) which fields are +0x140/+0x144 (100/20): default row height and column width, or minimum/maximum sizes? [setters 0x009A6823/0x009A6830 are outside the inventory]
  **ANSWERED 2026-10-22 (re-wingrid4):** MATCHES — +0x140 is the default column width and +0x144 the default row height (GetColumnWidthInternal 0x009AD684, GetRowHeightInternal 0x009AE387, CalculateCellHeight 0x009AD6FA); spec §5.10.

<!-- from: core-widgets5 round 2 (2026-10-23) -->
- ~~**WG5 - sections 3, 5.1, 5.9, 5.10 and 5.11 dependency bodies:** what are the bounds/empty-used-range rules, exact GetCellIsSelected matching predicate, relative-scroll clamping, and effective cell-format precedence/merge rules? These slots are named but lack body contracts needed by the newly specified owner methods. [literal provisional core: nonnegative indices, limits enforced only with fixed-range bits, -1 unlimited; empty range zeros; selected means exact pair; relative scroll adds then uses ScrollTo's upper clamp; font precedence cell/row/column/default; no closure claimed for these dependencies]~~
  **ANSWERED 2026-10-23 (re-wingrid-wg9):** DIFFERS — §5.15: bounds use limits directly, with 0/-1 admitting all signed indices; empty used range is zero/false; selection is column/row/pair by mode; relative scroll fills the viewport; merge precedence is cell/column/row/default, per field. Vectors WG5-a–e.

<!-- from: core-widgets5 round 2 (2026-10-23) -->
- ~~**WG6 - sections 5.1, 5.12 and 5.14:** please settle the pending SetDefaultFormatting initializer, horizontal text positioning for alignments 3/4/5, and format-copy size-word paragraph. Section 5.7 says 009A5C4A resets both size words to -1 but 5.14 still marks these writes pending; should 5.7 govern SetCellFormat too? [zero-initialized unresolved format words are provisional; do not claim Init/paint/format-copy closure until specified]~~
  **ANSWERED 2026-10-23 (re-wingrid-wg9):** DIFFERS — §5.15 gives sentinel-sensitive default initialization, font alignment flags 1/2/0 for 3/4/5, and the cross-field vertical-position quirk; §5.14 confirms size words reset to -1. Vectors WG6-a–d.

<!-- from: core-gzwingrid9 (2026-10-24) -->
- ~~**WG7 - section 5.11 editable text factory failure:** what result and cell state follow a failed cGZWinTextEdit creation? [settled path implemented with style 0x11FF, acquired text-interface release, created editor release, and original unresolved coordinates; on factory failure the already-cleared cell stays cleared and the core returns Unsupported::Open("WG7")]~~
  **ANSWERED 2026-10-23 (re-wingrid-wg9):** DIFFERS — §5.15: failed allocation/constructor leads to a null-interface dereference after prior clear, with no boolean result or rollback. Window QueryInterface failure after successful creation instead returns true without attachment. Vector WG7-a.

<!-- from: core-gzwingrid9 (2026-10-24) -->
- ~~**WG8 - sections 3, 5.7 and appendix A remaining pending owner methods:** please specify the still-pending owner bodies, especially SortColumn/SortRow, ResizeColumnToFitCellWindows, SetCellColors/Wrapping/Alignment/Border and default-comparator locale/encoding. Section 5.14 now specifies SetCellFont despite its appendix row remaining pending. [implement the sections that have bodies, expose stable sorting with an explicit comparator, and leave pending methods unsupported; no owner closure claimed for these rows]~~
  **ANSWERED 2026-10-23 (re-wingrid-wg9):** DIFFERS — §5.16 specifies all remaining owner bodies and helpers: sparse dense-rank sorting and selection quirk, monotonic signed-word width cache, unchecked colour index, inheritance-based partial formats, and CRT CTYPE-locale byte comparison. §5.14 confirms font store and dirty write. Vectors WG8-a–f.

<!-- from: core-gzwingrid9 (2026-10-24) -->
- ~~**WG9 - section 5.12 header rectangles and paint record +0x14:** what are the exact slot 72/73 header rectangles and the right-edge value read from paint record +0x14 in flag 0x20000 mode? [literal core uses the header band between gutter and cell-area boundary, and clip.right as the content-right value; these two details remain provisional]~~
  **ANSWERED 2026-10-23 (re-wingrid-wg9):** DIFFERS — §5.15: exact local rectangles include the whole-row header-height-as-width quirk; +0x14 points to the translated whole cell-area rectangle, whose right edge replaces text content/clip right and bitmap clip right. Vectors WG9-a–c.
