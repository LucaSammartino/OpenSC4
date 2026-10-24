# Widget toolkit: the `cGZWinGrid` spreadsheet window (behaviour spec)

Status: round 1 of at most 3 (lane re-wingrid4, 2026-10-21). Owner scope is exactly the 198 functions
(32735 bytes) the run-4 owner inventory attributes to `cGZWinGrid`; the appendix (§A) lists every one with its
spec section or `pending`. No blanket credit: a function counts as specified only when its row names a section.
Base-window behaviour (`cGZWin`) is in `widget-toolkit.md` and is not repeated here.

Evidence note: REA was unavailable (no binary open in the shared session, 2026-10-21), so everything below comes
from `decomp/cGZWinGrid.c`, `tools/re_query.py asm` and raw reads of the vtable at 0x00ADD578. Method names for
slots come from the Mac build's `__ZTV10cGZWinGrid` (re/symport/vtables.pkl), matched slot for slot; where the
match is uncertain the row says so.

## 1. Identity

- Class `cGZWinGrid`: a scrolling grid ("spreadsheet") window used for list boxes, tables and drop-downs. It
  implements `cIGZWinGrid` (interface id 0xDAA6B9BE, the only id its own `QueryInterface` 0x009A5CB5 answers;
  every other id goes to `cGZWin::QueryInterface`) and embeds a full `cGZWin` at +0x4.
- Two vtables: `cIGZWinGrid` view at 0x00ADD578 (142 slots, §3) written at offset 0, and the `cGZWin` view
  `vftable_1` at 0x00ADD7B0 (151 slots, docs/abi/vtables.tsv) written at +0x4. Functions reached through the
  second view receive `this` = grid+4; every offset in this spec is from the grid start (offset 0).
- Constructor 0x009ABC95 takes one argument, the initial grid flag word (§2, +0xDC).

## 2. Object layout *[compat]*

Values are what the constructor 0x009ABC95 stores. Size at least 0x2C4.

| Offset | Type | Initial | Meaning |
|---|---|---|---|
| +0x000 | vptr | 0x00ADD578 | `cIGZWinGrid` view |
| +0x004 | `cGZWin` | base ctor | embedded window; its vptr becomes 0x00ADD7B0 |
| +0x0DC | u32 | ctor arg | grid flag word (§4) |
| +0x0E0 | u32 | 0 | *[unknown]* |
| +0x0E4 | u8 | 1 | layout-dirty: set by every setter that changes geometry, then the recalculation helper 0x009A5FAA runs |
| +0x0E8 | s32 | 0 | recalculation pause counter (§5.4) |
| +0x0EC | u32 | 0 | *[unknown]* |
| +0x0F0..+0x0FC | 4 x s32 | 5,5,5,5 | grid gutters (left, top, right, bottom order as passed) |
| +0x100..+0x10C | 4 x s32 | 1,1,1,1 | cell gutters, same order |
| +0x110 / +0x114 | s32 | 20 / 80 | column-header height / row-header width (§5.1 GetCellArea) |
| +0x118 / +0x11C | s32 | 1 / 1 | grid-area width / height (§5.1) |
| +0x120 / +0x124 | s32 | 1 / 1 | cell-area width / height (§5.1) |
| +0x128 | s32 | 0 | horizontal scroll position (§5.1 DoMessage) |
| +0x12C / +0x130 / +0x134 | s32, f32, f32 | 0, 1.0, 1.0 | vertical scroll position; visible column / row counts, possibly fractional (hit test, arrow keys and the step scrolls use them, §5.9) |
| +0x138 / +0x13C | s32 | -1 / -1 | column / row cell limit; -1 = unlimited (§5.3) |
| +0x140 / +0x144 | s32 | 100 / 20 | default column width / default row height (§5.10) |
| +0x148 | u32 | 0 | outline colour (§5.2) |
| +0x14C..+0x158 | s32 | 0,0,1,1 | *[unknown]* |
| +0x15C / +0x160 | s32 | -1 / -1 | last cell set by SetCellText/Bitmap/Window (§5.11) |
| +0x164 | IUnknown* | null | background image (GZPaint step 3, §5.12) |
| +0x168 | IUnknown* | null | owner-draw notification target (§5.6) |
| +0x170 | `cRZTimer` | ctor(…,4) | double-click timer (§5.9 mouse-down) |
| +0x188 | u32 | 0 | extra-data mode: 1 = plain value, 2 = object (§5.5) |
| +0x18C / +0x190 | u32 | 1 / 0x7FFFFFFF | selection rules: mode, maximum count (§5.3) |
| +0x194 | std::list | empty | selection list of cell coordinates |
| +0x198 | u32 | 2 | *[unknown]* |
| +0x1E4 | sparse matrix | | per-cell format-extra matrix; cell entry +0x14 = cell extra data (§5.5) |
| +0x208 | IUnknown* | null | default font, ref-counted (§5.2) |
| +0x20C..+0x220 | 6 x u32 | | default font colours, index 0..5 (§5.2) |
| +0x238 / +0x244 | 3 x s32 each | 0,-1,-1 / 0,-1,-1 | last click hit and last hover hit {region, col, row} (§5.9) |
| +0x250 / +0x25C | map<u32,u32> | empty | row / column extra data (§5.5) |
| +0x268 | u32 or IUnknown* | 0 | grid extra data (§5.5) |
| +0x26C / +0x270 | IUnknown* | null | vertical / horizontal scroll bar (§5.10) |
| +0x274 / +0x278 | IUnknown* | null | scroll-bar images |
| +0x27C..+0x298 | | 0 | *[unknown]* |
| +0x29C..+0x2A4 | 3 x u32 | 0 | sound ids (§5.6) |
| +0x2A8 / +0x2B4 | map<u32,cRZString> | empty | column / row headings (§5.5) |
| +0x2C0 | IUnknown* | | released in Shutdown *[purpose pending]* |

## 3. The `cIGZWinGrid` slot table *[compat]*

142 slots at 0x00ADD578. Slot number = byte offset / 4. Rows marked *own* are in this owner's inventory; the
others point at code attributed elsewhere (shared one-line stubs or functions outside the 198) and are listed
only so the slot numbering is complete.

| Slot | Target | Name | | Slot | Target | Name |
|---|---|---|---|---|---|---|
| 0 | 009A5CB5 own | QueryInterface | | 71 | 009AF0F6 own | GetRowHeight |
| 1 | 0094D34F | AddRef | | 72 | 009A683D | GetColumnHeaderArea |
| 2 | 009A7650 own | Release | | 73 | 009A68E2 | GetRowHeaderArea |
| 3 | 009AF121 own | Init | | 74 | 009A6986 | EnableHeaders |
| 4 | 009ABED4 own | Shutdown | | 75 | 009A69B7 | GetHeaderSizes |
| 5 | 009CFA36 | AsIGZWin | | 76 | 009A69D4 | SetHeaderSizes |
| 6 | 009AFCCB | Refresh | | 77 | 009A69F9 | SetHeadingFormat |
| 7 | 009A619F own | SetArea | | 78 | 009AD986 own | GetColumnHeading |
| 8 | 009A76DA own | GetGridArea | | 79 | 009AD9B3 own | SetColumnHeading |
| 9 | 009A7734 own | GetCellArea | | 80 | 009AD931 own | GetRowHeading |
| 10 | 009A6284 | SetFlag | | 81 | 009AD95E own | SetRowHeading |
| 11 | 009A81BE own | SetFlags | | 82 | 009AB87B | ClearAllRowHeadings |
| 12 | 009A629F | ClearFlag | | 83 | 009AB889 | ClearAllColumnHeadings |
| 13 | 009A62BC | IsFlagSet | | 84 | 009AA5CE | GetCellFormat |
| 14 | 009683E0 | GetFlags | | 85 | 009AC519 own | SetCellFormat |
| 15 | 009A62F0 own | SetDefaultFont | | 86 | 009AB8FE | ClearCellFormat |
| 16 | 009A6333 own | GetDefaultFont | | 87 | 009AA7E3 | GetColCellFormatExtra |
| 17 | 009A633A own | SetDefaultFontColor | | 88 | 009AA7FB | GetRowCellFormatExtra |
| 18 | 009A6357 own | GetDefaultFontColor | | 89 | 009A6C14 | GetColRowCellFormatExtra |
| 19 | 009AC269 | SetDefaultFontColorByColumns | | 90 | 009AC741 own | SetCellFont |
| 20 | 009AC2E9 | SetDefaultFontColorByRows | | 91 | 009AC866 own | SetCellColors |
| 21 | 009A6376 own | SetDefaultTextWrapping | | 92 | 009AC960 own | SetCellWrapping |
| 22 | 009A63B4 own | GetDefaultTextWrapping | | 93 | 009ACA52 own | SetCellAlignment |
| 23 | 009A63BF own | GetGutters | | 94 | 009ACB56 own | SetCellBorder |
| 24 | 009A63F4 own | SetGutters | | 95 | 009AA59C | GetCellType |
| 25 | 009A642D own | GetCellGutters | | 96 | 009AB897 own | ClearCell |
| 26 | 009A6462 own | SetCellGutters | | 97 | 009A6A39 | ClearCellRect |
| 27 | 009A648F own | SetOutlineUse | | 98 | 009A6AF1 | ClearAllCells |
| 28 | 0076B620 | GetOutlineColor | | 99 | 00938789 | DeleteCell |
| 29 | 009A64B3 own | PauseRecalculations | | 100 | 009AE5AC own | DeleteColumns |
| 30 | 009AA3E1 own | ActLikeAListbox | | 101 | 009AE6DF own | DeleteRows |
| 31 | 009AE48F own | ScrollCellsVertically | | 102 | 009AD9DB own | CopyColumns |
| 32 | 009AD817 own | ScrollCellsHorizontally | | 103 | 009ADB72 own | CopyRows |
| 33 | 009A64CB own | ScrollPagesVertically | | 104 | 009A6B50 | IsCellVisible |
| 34 | 009A64DB own | ScrollPagesHorizontally | | 105 | 009AFC65 | MakeCellVisible |
| 35 | 009A64EE own | ScrollToColumn | | 106 | 009AE813 own | GetCellRect |
| 36 | 009A6563 own | ScrollToRow | | 107 | 009A6BA5 | GetFirstVisibleCell |
| 37 | 009A65D8 own | ScrollToCell | | 108 | 009A6BC2 | SetFirstVisibleCell |
| 38 | 009A65FB own | GetSelectionRules | | 109 | 009A6BF7 | GetVisibleColumnAndRowCount |
| 39 | 009A6618 own | SetSelectionRules | | 110 | 009A7C69 own | DoCursorHitTest |
| 40 | 009A96FD | GetSelectionCount | | 111 | 009AA813 own | GetCellTextValue |
| 41 | 009A9711 | GetSelection | | 112 | 009AE8CB own | SetCellTextValue |
| 42 | 009A87DC own | GetSelection (2nd overload) | | 113 | 009AA8AA | GetCellBitmap |
| 43 | 009A9765 own | SetSelection | | 114 | 009AEB66 own | SetCellBitmap |
| 44 | 009AA477 own | AddCellToSelection | | 115 | 009AA8E2 | GetCellWindow |
| 45 | 009A8809 | GetCellIsSelected | | 116 | 009AECB7 own | SetCellWindow |
| 46 | 009A8869 | ClearSelection | | 117 | 009A8F16 own | IsWindowACellWindow |
| 47 | 009A6639 own | EnableFixedRanges | | 118 | 009ADD09 own | ResizeColumnToFitCellWindows |
| 48 | 009A667E own | SetCellLimits | | 119 | 009ACE2F own | SetCellExtraData |
| 49 | 009A6697 | GetCellLimits | | 120 | 009AAAD1 own | GetCellExtraData |
| 50 | 009A97CE | GetCellUsedRange | | 121 | 009AB198 own | SetColumnExtraData |
| 51 | 009A66B4 | GetCellVisibleRange | | 122 | 009AB1DC own | GetColumnExtraData |
| 52 | 009A66FC | IsColumnWithinBounds | | 123 | 009AB1FA own | SetRowExtraData |
| 53 | 009A6722 | IsRowWithinBounds | | 124 | 009AB23E own | GetRowExtraData |
| 54 | 009A6748 | IsCellWithinBounds | | 125 | 009A6D2D own | SetExtraData |
| 55 | 009A6778 | GetMostRecentlyAddedCell | | 126 | 009A6D43 own | GetExtraData |
| 56 | 009A679D | SetBackgroundColor | | 127 | 009A6D4A own | GetCellExtraDataObject |
| 57 | 009A67B1 | SetBackgroundBitmap | | 128 | 009A6D87 own | SetCellExtraDataObject |
| 58 | 009A67DE | SetGridColors | | 129 | 009A6E0F own | GetExtraDataObject |
| 59 | 005F8AC0 | GetGridColors | | 130 | 009A6E22 own | SetExtraDataObject |
| 60 | 009A6814 | SetDefaultTextAlignment | | 131 | 009ACED6 own | SortColumn |
| 61 | 009AC369 | SetDefaultTextAlignmentByColumns | | 132 | 009AD1E8 own | SortRow |
| 62 | 009AC3D2 | SetDefaultTextAlignmentByRows | | 133 | 009A9892 own | FindCellText |
| 63 | 00423850 | GetDefaultTextAlignment | | 134 | 009A6E90 own | SetOwnerDrawNotification |
| 64 | 009A6823 | SetDefaultColumnWidth | | 135 | 009A6EBD own | GetBufferToDrawTo |
| 65 | 009AC43B own | SetColumnWidth | | 136 | 009A6EF7 own | SetScrollBarImageVertical |
| 66 | 009A6830 | SetDefaultRowHeight | | 137 | 009A6F67 own | SetScrollBarImageHorizontal |
| 67 | 009AC4AA own | SetRowHeight | | 138 | 009A6EDA own | SetSoundID |
| 68 | 00459870 | GetDefaultColumnWidth | | 139 | 009A5F70 own | MakeDropDown |
| 69 | 00472E70 | GetDefaultRowHeight | | 140 | 009A5F99 | scalar deleting destructor |
| 70 | 009AE35C own | GetColumnWidth | | 141 | 009A824E own | PlaySound (§5.8) |

## 4. The grid flag word (+0xDC) *[compat]*

Flags are changed through the interface's own SetFlag/ClearFlag/IsFlagSet slots (10, 12, 13). Bits whose
meaning is measured so far:

| Bit | Meaning | Evidence |
|---|---|---|
| 0x200 | draw cell outline | SetOutlineUse 0x009A648F sets/clears byte +0xDD bit 1 |
| 0x800 | default text wrapping | SetDefaultTextWrapping 0x009A6376, GetDefaultTextWrapping 0x009A63B4 |
| 0x2000 | fixed column range | EnableFixedRanges 0x009A6639 |
| 0x4000 | fixed row range | EnableFixedRanges 0x009A6639 |
| 0x40000 | drop-down mode | MakeDropDown 0x009A5F70 |

## 5. Owner methods

### 5.1 Lifecycle, area and wheel

- **Constructor(flags)** 0x009ABC95: builds the embedded `cGZWin`, then every field of §2; the three sparse
  matrices at +0x19C, +0x1C0 and +0x1E4, the maps at +0x22C, +0x250, +0x25C, +0x2A8, +0x2B4 and the selection
  list +0x194 start empty.
- **Init()** 0x009AF121 always returns true. Only when the embedded window is not yet initialised: run
  `cGZWin::Init`, store at +0xE0 the language manager's slot-7 value (1 when there is no language manager),
  then SetDefaultFormatting 0x009A5CEE (§5.15) and RecalculateAll 0x009AEE63 (§5.10). A second Init is a no-op.
- **Shutdown()** 0x009ABED4: DisposeOfAllCellData (pending), then Release and null each of +0x164, +0x168,
  +0x208, +0x2C0, +0x26C, +0x270, +0x274, +0x278 that is non-null (in that order), then `cGZWin::Shutdown`.
- **Release()** 0x009A7650: the reference count is the window's (grid +0x10). Above 1 it is decremented and the
  new count returned; at 1 or below FinalRelease 0x009A5CD7 runs and 0 is returned. FinalRelease calls Shutdown
  (slot 4) and then the deleting destructor (window-view slot 148, argument 1).
- **Destructor** 0x009AC24D / body 0x009ABF83: restores both vptrs, calls Shutdown again (harmless: everything
  is already null), destroys the heading maps, the extra-data maps, the +0x22C map, the three matrices, the
  selection list and the +0x170 object, then `~cGZWin`; frees the memory when bit 0 of the argument is set.
- **SetArea(l, t, r, b)** 0x009A619F (window view): `cGZWin::SetArea`, then, only if the window is initialised,
  recompute grid area and cell area and cache their sizes: +0x118 = grid width, +0x11C = grid height, +0x120 =
  cell width, +0x124 = cell height; mark dirty and recalculate.
- **GetGridArea(&rect)** 0x009A76DA: rect = (left gutter, top gutter, window width - right gutter, window
  height - bottom gutter), using gutters +0xF0, +0xF4, +0xF8, +0xFC; returns true.
- **GetCellArea(&rect)** 0x009A7734: the grid area, then top += +0x110 when flag 0x8 (column headers shown),
  left += +0x114 when flag 0x10 (row headers shown), right -= the vertical scroll bar's width when +0x26C exists,
  bottom -= the horizontal scroll bar's height when +0x270 exists. So +0x110 is the column-header height
  (default 20) and +0x114 the row-header width (default 80); this answers half of WG2.
- **SetFlag(flag, on)** 0x009A6224 (window view): for flag 2 (window visibility) the same SetFlag(2, on) is
  first forwarded to both scroll bars when present; then `cGZWin::SetFlag`.
- **SetFlags(newFlags)** 0x009A81BE (grid flag word): equal to the current word: nothing. Otherwise store it;
  if bit 0x2 or 0x4 changed, call RemoveScrollBar(!(new & 0x4), !(new & 0x2)) 0x009A7178, mark dirty and
  recalculate; if bit 0x40000 (drop-down) changed, take mouse capture when it became set and the window is not
  already capturing, or release capture when it became clear and the window is capturing. The return register
  is not set (callers must not use it). So 0x2 / 0x4 are the scroll-bar enable bits.
- **DoMessage(msg)** 0x009A7665 (window view): for a type-3 message with id 0x887113A3 (scroll-bar position
  change) from a scroll bar: sub-id 0x1A7C4511 with a vertical bar (+0x26C) present reads the bar's value
  (slot 6) into +0x12C; sub-id 0x1A7C4510 with the horizontal bar (+0x270) reads it into +0x128. If the value
  changed, set the recalc request byte (+0xE4... see note) and recalculate. Every message, handled or not, then
  goes to `cGZWin::DoMessage`. So +0x128 / +0x12C are the horizontal / vertical scroll positions.
  Note: the store is to grid +0xE4 (decomp `this+0xE0` in the window view), the same layout-dirty byte.
- **GZOnMouseWheel(…, delta)** 0x009A5F7D: ScrollCellsVertically(delta / -100) (signed division, truncating
  toward zero; wheel up = negative = scroll up), returns true.
- **Scroll by step** 0x009A5F23 (up?) / 0x009A5F48: ScrollCellsVertically / ScrollCellsHorizontally by the
  float visible row / column count +0x134 / +0x130 (default 1.0; one page, see §5.9) converted with truncation, negated when the argument is true.
- **GZOnSetFocus** 0x009A7E0C / **GZOnKillFocus** 0x009A7E2F: when grid flag bit 0x1 is set, recalculate
  (0x009A5FAA) first. SetFocus then runs `cGZWin::GZOnSetFocus`; KillFocus returns false directly (the shared
  stub 0x0093878E).
- **GZOnCharacter** 0x009A5F1E: returns false (same stub); the grid ignores character input.

### 5.2 Fonts, colours, gutters and outline

- **SetDefaultFont(font)** 0x009A62F0, returns nothing useful (the register is left as is). If `font` equals the
  stored +0x208 pointer, nothing happens. Otherwise: AddRef the new font if non-null, Release the old one if
  non-null, store the new pointer, set layout-dirty (+0xE4 = 1) and run the recalculation helper 0x009A5FAA.
- **GetDefaultFont()** 0x009A6333 returns +0x208 without AddRef.
- **SetDefaultFontColor(colour, index)** 0x009A633A: index is unsigned; index >= 6 returns false and stores
  nothing; otherwise stores `colour` at +0x20C + 4*index and returns true. No redraw, no dirty flag.
- **GetDefaultFontColor(&colour, index)** 0x009A6357: same bound; on success writes the stored value and
  returns true.
- **SetDefaultTextWrapping(on)** 0x009A6376 always returns true. If `on` equals the current state of flag
  0x800 (as returned by IsFlagSet) it does nothing else; otherwise sets or clears 0x800, sets layout-dirty and
  recalculates. **GetDefaultTextWrapping()** 0x009A63B4 returns IsFlagSet(0x800).
- **GetGutters(&a,&b,&c,&d)** 0x009A63BF / **SetGutters(a,b,c,d)** 0x009A63F4 read / write the four grid
  gutters +0xF0..+0xFC in argument order; both return true; SetGutters sets layout-dirty and recalculates.
- **GetCellGutters / SetCellGutters** 0x009A642D / 0x009A6462 do the same on +0x100..+0x10C, but
  SetCellGutters does **not** mark dirty or recalculate (quirk: the change shows only after the next
  recalculation triggered by something else).
- **SetOutlineUse(on, colour)** 0x009A648F sets or clears flag bit 0x200 directly in the word (no SetFlag call)
  and always stores `colour` at +0x148; returns nothing meaningful.

### 5.3 Scrolling shortcuts, selection rules and limits

- **ScrollPagesVertically(n)** 0x009A64CB calls ScrollCellsVertically(n*4); **ScrollPagesHorizontally(n)**
  0x009A64DB calls ScrollCellsHorizontally(n*4). A "page" is a fixed 4 cells, not the visible count
  (quirk, keep). The return value is whatever the cell scroll returned.
- **ScrollToCell(col, row)** 0x009A65D8 calls ScrollToColumn(col) then ScrollToRow(row) and returns true
  regardless of their results.
- **GetSelectionRules(&mode, &max)** 0x009A65FB writes +0x18C and +0x190, returns true.
  **SetSelectionRules(mode, max)** 0x009A6618 stores both, then calls ClearSelection (slot 46); returns true.
  Defaults: mode 1, max 0x7FFFFFFF.
- **EnableFixedRanges(cols, rows)** 0x009A6639: `cols` true sets flag 0x2000; false clears it **and** resets
  the column limit +0x138 to -1. Same for `rows` with 0x4000 and +0x13C. Returns true.
- **SetCellLimits(cols, rows)** 0x009A667E stores +0x138/+0x13C unchecked and returns true; no recalculation.
- **PauseRecalculations(pause)** 0x009A64B3: true increments the counter +0xE8, false decrements it (no floor at
  zero); returns the new counter value.

### 5.4 Widths and heights

- **GetColumnWidth(first, count)** 0x009AE35C returns the sum of GetColumnWidthInternal(first+i) for
  i = 0..count-1; count <= 0 returns 0. **GetRowHeight(first, count)** 0x009AF0F6 is the same over
  GetRowHeightInternal. Both are signed 32-bit sums with no overflow check.

### 5.5 Extra data and headings

- **SetExtraData(value)** 0x009A6D2D stores `value` at +0x268 and sets the mode +0x188 to 1; no AddRef, no
  release of a previous object (quirk: replacing an object with a plain value leaks one reference).
  **GetExtraData()** 0x009A6D43 returns +0x268.
- **SetExtraDataObject(obj)** 0x009A6E22 sets mode 2, AddRefs `obj` (if non-null), Releases the previous +0x268
  if non-null (even if it was a plain value: quirk, a plain value stored earlier is called as an object) and
  stores `obj`; returns true. **GetExtraDataObject(iid, &out)** 0x009A6E0F returns false if +0x268 is null,
  otherwise tail-calls the object's QueryInterface(iid, &out).
- **GetCellExtraData(col, row)** 0x009AAAD1 looks the cell up in the format-extra matrix at +0x1E4; returns the
  entry's +0x14 field, or 0 if there is no entry.
- **GetCellExtraDataObject(col, row, iid, &out)** 0x009A6D4A sets the mode to 1, calls GetCellExtraData (slot
  120), then sets the mode to 2 regardless; if the value is null returns false, else returns its
  QueryInterface(iid, &out).
- **SetColumnExtraData(col, value)** 0x009AB198: sets mode 1, asks IsColumnWithinBounds(col) (slot 52); if
  false returns false; else stores `value` in the column map +0x25C (inserting the key) and returns true.
  **SetRowExtraData** 0x009AB1FA is the same with IsRowWithinBounds (slot 53) and the row map +0x250.
- **GetColumnExtraData(col)** 0x009AB1DC / **GetRowExtraData(row)** 0x009AB23E return the map value; a missing
  key is **inserted** with value 0 (std::map operator[]) and 0 is returned. No bounds check.
- **SetColumnHeading(col, text)** 0x009AD9B3 / **SetRowHeading(row, text)** 0x009AD95E assign `text` (a
  cIGZString, via its slot 13 copy) to the column map +0x2A8 / row map +0x2B4 entry, creating it; return true.
- **GetColumnHeading(col, &out)** 0x009AD986 / **GetRowHeading(row, &out)** 0x009AD931 copy the map entry into
  `out` (creating an empty entry if missing) and return true. No bounds checks on any of the four.

### 5.6 Notifications, buffer, sounds, drop-down

- **SetOwnerDrawNotification(target)** 0x009A6E90: AddRef new (if non-null), Release old +0x168 (if non-null),
  store; returns true.
- **GetBufferToDrawTo(&buffer, &rect)** 0x009A6EBD writes the window's buffer pointer (grid +0x6C, i.e. cGZWin
  +0x68) and copies the 16-byte rectangle at grid +0x28 (cGZWin +0x24) into `rect`; returns true.
- **SetSoundID(index, id)** 0x009A6EDA: signed index; index >= 3 returns false; otherwise stores at
  +0x29C + 4*index and returns true. A negative index is **not** rejected (quirk: writes before the array).
- **MakeDropDown()** 0x009A5F70 sets flag 0x40000 and returns true.

### 5.8 Format matrices, explicit scrolling, cell windows, sounds

The three sparse matrices hold 40-byte format records (copy rule 0x009A5C4A): +0x19C column formats keyed
(col, 0), +0x1C0 row formats keyed (0, row), +0x1E4 per-cell formats keyed (col, row). Record +0x14 is the cell
extra data, +0x24 (s16) the explicit size, +0x26 (s16) a cached actual size (both -1 = unset).

- **SetColumnWidth(first, count, width)** 0x009AC43B: a negative width becomes 0. For each column in
  [first, first+count) set the column record's +0x24 to the width truncated to 16 bits, inserting a default
  record when the column has none. Returns true. **No** dirty mark or recalculation (the caller recalculates).
  **SetRowHeight(first, count, height)** 0x009AC4AA: the same over the row matrix +0x1C0.
- **ScrollToColumn(col)** 0x009A64EE, always returns true. If `col` equals the horizontal position +0x128,
  nothing. Otherwise clamp the upper end only: when flag 0x8000 is set and the column limit +0x138 is > 0, the
  maximum is limit-1; when flag 0x8000 is set and the limit is <= 0 (or -1), no clamp; when flag 0x8000 is
  clear, the maximum is the used range's last column (GetCellUsedRange, slot 50) and `col` above it becomes it.
  Negative values are stored unclamped. Store into +0x128 and recalculate (no dirty mark).
  **ScrollToRow(row)** 0x009A6563: the same with +0x12C, row limit +0x13C, flag 0x10000 and the used range's
  last row.
- **GetSelection(&col, &row)** 0x009A87DC (slot 42): the first entry of the selection list (+0x194); false and
  outputs untouched when the selection is empty.
- **IsWindowACellWindow(win, id, &col, &row)** 0x009A8F16: walk the cell-window map +0x22C in key order; an
  entry matches when `win` is non-null and is the entry's window, or `win` is null and the entry window's ID
  (window slot 63) equals `id`. On the first match write the entry's column and row to the non-null out
  pointers and return true; otherwise false.
- **SetCellExtraDataObject(col, row, obj)** 0x009A6D87: false when IsCellWithinBounds (slot 54) fails.
  Otherwise AddRef `obj`, Release the cell's previous extra data if non-null, then call SetCellExtraData(col,
  row, **grid extra data +0x268**) and return true; the mode ends at 2. Quirk *[compat, see WG4]*: the object
  passed in is AddRef'd but not stored; the cell receives the grid-level extra-data value instead.
- **SetScrollBarImageVertical(image, unused)** 0x009A6EF7: AddRef new, Release old +0x274, store. When the
  image is non-null and the vertical bar exists, hand it to the bar (bar slot 17) with a source rectangle: the
  stored rectangle +0x27C if it is non-empty (test 0x007C1870), else the image's own area (image slot 12).
  The second argument is ignored. Returns true. **SetScrollBarImageHorizontal** 0x009A6F67: same with +0x278,
  the horizontal bar +0x270 and rectangle +0x28C.
- **PlaySound(index)** 0x009A824E (slot 141; answers WG1): sound id = +0x29C + 4*index (index unchecked). If
  the id is 0 or there is no sound system, false. Otherwise ask the sound system (slot 13) for an instance of
  class 0x5B523BA4 with arguments (5, id, 0, 0, 0); if that fails or yields null, false; else play it (slot 8),
  release the instance and return the play result.

### 5.7 Forwarding thunks and container helpers

Pure forwarding (no behaviour of their own): the `cGZWin`-view thunks subtract 4 from `this` and jump to the
grid method: 0x005F8F70 QueryInterface, 0x005F8F90 Release, 0x005F8FA0 Init, 0x005F8FB0 Shutdown,
0x005F8FC0 SetArea, 0x009ABECC deleting destructor (to 0x009AC24D).

Container helpers attributed to this owner (template instances; an implementation may use any container with
the same observable result):

| Address | Behaviour |
|---|---|
| 0x0047D840 | scalar deleting destructor of a small record {vptr, cRZString at +0x8}: resets the vptr to 0x00A80810, destroys the string if present, frees the record when bit 0 of the argument is set; returns the record |
| 0x0066E2F0 | `std::map<u32,…>::lower_bound`: first node whose key (+0x10) is not below the key; returns the end node when none |
| 0x007BFFA0 | greatest common divisor by Euclid on signed ints: `gcd(a, 0) = a`; otherwise repeat `(a, b) = (b, a mod b)` (C remainder, sign of the dividend) until b = 0 |
| 0x008940CD | `std::list<CellCoordinates>::insert(pos, first, last)` wrapper |
| 0x009A5C20 / 0x009A5C9F | return a node to a free-list pool (heads at 0x00B17BF8 / 0x00B17C18); null is ignored |
| 0x009A62CF | window-view SetFillColor(colour): `cGZWin::SetFillColor`, then recalculate when grid flag 0x100 is set; the return value is not defined by the grid |
| 0x009A6E59 | sort comparator for two cell values {type +0x0, payload +0xC}: when both types are equal, both payloads non-null and the type is 2 (text), returns the case-insensitive compare (`_stricmp`) of the payloads' strings (+0x4); every other pair compares equal (0) |
| 0x009A75A3 | equality of two 3-word records on words 1 and 2: true when word 1 is 0 in both (word 2 ignored), otherwise true only when words 1 and 2 both match |
| 0x009A75CF | temporary-buffer allocation for stable sorting: request capped at 0x1FFFFFFF 4-byte elements; on allocation failure halve the request and retry until it reaches 0 (buffer then null) |
| 0x009A7610 | placement copy of a 28-byte record (no-op when the destination is null) |
| 0x009A7626 / 0x009A8574 | `copy_backward` over 4-byte elements; returns the new start of the destination |
| 0x009A8591 | `swap_ranges` over 4-byte elements |
| 0x009A8446 / 0x009A8473 | recursive node erase of a `map<u32, cRZString>` subtree (two identical instances, used for the heading maps) |
| 0x009A5C4A | copy-construct a 40-byte cell record: copies the first 36 bytes from the source, then the two 16-bit fields at +0x24 and +0x26 are -1 (not copied) |
| 0x009A88DF / 0x009A8908 | `clear()` of a heading map (+0x2A8 / +0x2B4): when non-empty, erase every node (0x009A8446 / 0x009A8473), reset the header links and the size to 0 |
| 0x009A8FBE / 0x009A8FD5 | destructor of the same two maps: clear, then free the header node |
| 0x009A89AF | red-black tree node insert for the cell-window set +0x22C (`_Rb_tree::_Insert`): new node linked as the left child when the parent is the header, or the caller forces left, or the new key is below the parent's key (unsigned compare of the value's word +0xC against node +0x1C); rebalance (0x0040D5E0), size + 1, return the node |
| 0x009A93A1 / 0x009A999D | `insert_unique` into the cell-window set: walk down by the unsigned key (value word +0xC = node +0x1C); an equal key returns (existing node, false), otherwise insert and return (new node, true); 0x009A999D is the wrapper that copies the pair out |
| 0x009A90F8 | `equal_range` on a `map<u32, void*>`: (lower_bound 0x0066E2F0, upper_bound) |
| 0x009A9A09 / 0x009A9A4C | begin / end iterator of one line of a sparse matrix: find the outer key (0x009A94A6); missing: iterator {matrix, null, null}; present: {matrix, inner map, first node} / {matrix, inner map, header} |

**Stable-sort machinery** (SortColumn / SortRow, §5.16, sort an array of 4-byte value pointers with a stable
merge sort; an implementation may use any stable sort with the same predicate, the result is identical):

| Address | Behaviour |
|---|---|
| 0x009A8888 | predicate object {compare function, pointer to the value-pointer array, ascending byte}: less(i, j) = compare(v[i], v[j]) < 0 when the byte is non-zero, compare(v[j], v[i]) < 0 when it is zero (descending); compare is 0x009A6E59 or a caller-supplied one |
| 0x009A8A38 | temporary buffer for (first, last): count = (last - first) / 4, allocated by 0x009A75CF |
| 0x009A8BA4 / 0x009A9D2E | unguarded linear insert / linear insert (an element less than the first goes to the front by copy_backward) |
| 0x009A8BD4 / 0x009A963E | `lower_bound` with the predicate, and its wrapper |
| 0x009A8C1E / 0x009A9665 | `upper_bound` with the predicate, and its wrapper |
| 0x009A8C68 / 0x009A9D7C | in-place `rotate(first, middle, last)` by gcd cycles (gcd 0x007BFFA0; equal halves use swap_ranges); returns first + (last - middle) |
| 0x009A968C | `merge` of two sorted runs into an output range; on ties the element of the first run goes first (stable); then copies what remains |
| 0x009A96E2 | `copy_backward` wrapper (0x009A8574) |
| 0x009A9D95 | one merge-sort pass: merge adjacent runs of `step` elements pairwise into the output, then the tail |
| 0x009A9E59 | `merge_backward` (merge from the ends, used when the buffer holds the second run) |
| 0x009A9EE3 | buffered rotate: when the shorter side fits the buffer, rotate by copying through it; otherwise the in-place rotate |

### 5.9 Selection, hit testing and mouse input

Round 2 (2026-10-22), static only (REA had no app open): decomp/cGZWinGrid.c, decomp/_unnamed/region_009a0000.c
and `re_query.py asm`. Window-view slots below are `cGZWin` slots (vtable 0x00ADD7B0); grid slots are
`cIGZWinGrid` slots (§3). "Recalculate" is the helper 0x009A5FAA (§5.1).

**Selection list.** +0x194 is a `std::list` of (column, row) pairs in insertion order; the last element is the
most recently added cell. The selection mode +0x18C decides what a *removal* matches: mode 0 = column (every
entry with the same column), mode 1 = row (every entry with the same row), any other value = the exact cell.
Additions never look at the mode.

- **AddCellToSelection(col, row, select)** 0x009AA477 (slot 44). Ask GetCellIsSelected(col, row) (slot 45). When
  the answer equals `select` (byte compare), nothing changes. Otherwise, with `select` false, erase every list entry
  that matches by the mode rule above (all of them, not just the first); with `select` true, append (col, row)
  only when the current list length is below the maximum +0x190 (signed compare). Returns true when at least one
  entry was erased or the cell was appended, else false. The recalculate helper runs on **every** call, including
  the no-change ones. No bounds check: any coordinates can be appended.
- **SetSelection(col, row)** 0x009A9765 (slot 43). Maximum +0x190 <= 0: false. When the cell is already selected
  and the list holds exactly one entry: false (nothing to do). Otherwise ClearSelection (slot 46),
  AddCellToSelection(col, row, true), recalculate and return true.
- **ActLikeAListbox(columns, maxSelected, keepEmpty)** 0x009AA3E1, always returns true. Writes the fields directly
  (no SetFlags, so no scroll-bar or capture side effects): row limit +0x13C = -1, column limit +0x138 = `columns`,
  flag word = (word & 0xFFFC2BA6) | 0x2BA6, mode +0x18C = 1 (row selection), maximum +0x190 = `maxSelected`.
  So bits 0x1, 0x8, 0x10, 0x40, 0x400, 0x1000, 0x4000, 0x8000, 0x10000, 0x20000 are cleared and 0x2, 0x4, 0x20,
  0x80, 0x100, 0x200, 0x800, 0x2000 set (column headers 0x8 and row headers 0x10 off, both scroll-bar bits on,
  fixed column range on). With `keepEmpty` true the word also gets 0x1000 and nothing else happens. With it false:
  `maxSelected` < 1 runs ClearSelection; otherwise, when the selection is empty (GetSelection slot 42 fails), the
  cell (0, 0) is appended straight to the list (no GetCellIsSelected, no recalculate). *[The meaning of flag 0x1000
  is not measured yet; "keepEmpty" names only the observed effect on the selection.]*

**Hit test.** Region codes: 0 = none, 1 = outside the grid area, 2 = row header, 3 = column header, 4 = header
corner, 5 = cell area.

- **DoCursorHitTest(x, y, &region, &cell)** 0x009A7C69 (slot 110). Zero the region and both cell coordinates.
  Point tests use the inclusive rectangle test 0x0043FDE0.
  1. Not inside GetGridArea (slot 8): region 1, return true (cell stays (0, 0)).
  2. Inside GetCellArea (slot 9): region 5.
  3. Otherwise only when **both** header flags 0x8 and 0x10 are set: "in column header" = top gutter +0xF4 <= y <
     +0x110, "in row header" = left gutter +0xF0 <= x < +0x114 (quirk: the bound is the header size itself, not
     gutter + size). Both: region 4, return true, cell (0, 0). Column header only: region 3. Row header only:
     region 2. Neither, or either header flag clear: region 0, return false.
  4. For regions 5, 3, 2 find the cell: for column i from the horizontal position +0x128 while i < +0x128 +
     ceil(+0x130) (float, ceil 0x009F0A20, then truncation), take GetCellRect(i, 0) (slot 106); the first i with
     x < rect.right is the column. Rows the same with +0x12C, ceil(+0x134), GetCellRect(0, j) and y < rect.bottom.
     No match leaves 0. Region 3 then forces row 0; region 2 forces column 0. Return true.
  So +0x130 / +0x134 act as the visible column / row counts (possibly fractional); §2 calls them scroll steps
  because the step-scroll functions use them (so those scroll by one visible page).

**Hit records.** +0x238 {region, col, row} = last clicked hit (constructor 0, -1, -1); +0x244 {region, col, row} =
last hover hit (same initial values). Grid notifications go to the window's notification target (cGZWin +0x4C,
grid +0x50) through window-view SendMsg (slot 145, a 16-byte message {id, source = grid interface, data, 0}; slot
144 takes the same fields as arguments):

| Id | Sent by | Data |
|---|---|---|
| 0x9A1552D2 | mouse-down, when the clicked hit differs from the previous one | pointer to the hit cell (col, row) |
| 0x9A1552D3 | every single click (slot 144); drop-down mouse-up on a hit | pointer to the record +0x238 |
| 0x9A1552D4 | double click | pointer to the record +0x238 |
| 0x9A1552D6 | mouse-move entering a new header or cell | pointer to a copy {region, col, row} |
| 0x9A1552D7 | drop-down: mouse-down outside the window | 0 |

**Drop-down routing.** When the window is capturing and grid flag 0x40000 is set, mouse-down / mouse-up /
mouse-move first check the point is inside the window (window-view slot 121); then for the vertical bar +0x26C,
then the horizontal bar +0x270: convert the point into the bar's coordinates (window-view slot 61 with the bar's
window, bar slot 3), and when 0 <= x < bar width (bar slot 41) and 0 <= y < bar height (slot 42), pass the bar to
the window manager's slot 29 (the window held at manager +0x30, which sends message 0x12 to the old and new
holder), hand the bar a mouse message {type, x, y, modifiers} (type 7 down, 10 up, 13 move) through its slot 3,
re-take capture for the grid and return the bar's result.

- **GZOnMouseDownL(x, y, modifiers)** 0x009A9F6A, returns true unless forwarded to a bar. Modifier bits: 0x1, 0x2,
  0x4 (read as Shift, Control, Alt *[inferred from the range / toggle behaviour]*).
  1. Window flag 2 clear (window-view slot 67): return true.
  2. Drop-down capture: inside the window, route to a bar as above; a point inside the window but on neither bar:
     return true. Outside the window: send 0x9A1552D7 and return true.
  3. Double click: the timer +0x170 (`cRZTimer`). If it is running and its elapsed time is < 500, this is a double
     click and the timer is Reset; otherwise Restart it (single click).
  4. Hit test (slot 110). Region 2 (row header): empty the selection list directly (0x009A841F, no
     GetCellIsSelected, no message), then AddCellToSelection(c, row, true) for every column c of GetCellUsedRange
     (slot 50) from left to right inclusive (unsigned loop). Region 3 (column header): the same for every row of the
     used range at the clicked column. Region 5 (cell):
     - Shift without Alt and without Control: when the list is empty append the cell straight to the list;
       otherwise take the rectangle spanned by the cell and the **last** list entry (normalised), and
       AddCellToSelection(c, r, true) for every cell of it, columns outer, rows inner (the existing selection is
       kept).
     - Control without Shift and without Alt: toggle, AddCellToSelection(cell, !GetCellIsSelected(cell)).
     - No modifier: ClearSelection, then AddCellToSelection(cell, true).
     - Any other combination (Alt, Shift+Control): no selection change.
     Regions 0, 1, 4: no selection change.
  5. Compare (region, col, row) with the record +0x238; when different, the click is never a double click and
     "changed" is set. Store the new hit in +0x238.
  6. Double click: send 0x9A1552D4. Single click: send 0x9A1552D3 (slot 144); when changed, also send 0x9A1552D2
     and recalculate. Return true.
- **GZOnMouseUpL(x, y, modifiers)** 0x009A7826: acts only under drop-down capture with the point inside the window:
  bar routing (type 10) first; otherwise hit test, and on success store the hit in +0x238 and send 0x9A1552D3.
  Returns true (or the bar's result).
- **GZOnMouseMove(x, y, modifiers)** 0x009A79E6, returns true (or the bar's result). Not under drop-down capture:
  hit test; on success, when the region is 2, 3 or 5 and differs from the hover record +0x244 (region or either
  coordinate), send 0x9A1552D6 with a copy of the hit; store the hit in +0x244 whatever its region. Under drop-down
  capture: outside the window, nothing; bar routing (type 13); otherwise a hit in region 5 that differs from the
  click record +0x238 (region not 5 or another cell) runs ClearSelection then AddCellToSelection(cell, true), and
  the record becomes (5, col, row). So dragging in an open drop-down list moves the single selection.
- **GZOnKeyDown(key, modifiers)** 0x009A8D6D. Window flag 2 clear: return true and nothing else. Otherwise by
  virtual key:
  - End 0x23: when flag 0x8000 or 0x10000 is clear, read GetCellUsedRange (slot 50; otherwise the range stays all
    0). Column: flag 0x8000 clear: ScrollToColumn(used right); set: ScrollToColumn(column limit +0x138 - 1) only when
    the limit is neither -1 nor 0. Row: the same with flag 0x10000, used bottom, ScrollToRow and +0x13C.
  - Home 0x24: ScrollToCell(0, 0) (slot 37).
  - PageUp 0x21 / PageDown 0x22: modifier 0x4 clear: vertical step scroll 0x009A5F23(up = true for PageUp);
    set: horizontal 0x009A5F48. (Each scrolls by the visible count, see the hit-test note.)
  - Left/Up/Right/Down 0x25..0x28: DoArrowKey(key, modifiers).
  - Insert 0x2D: toggle flag 0x80 in the word directly.
  - Escape 0x1B: send message {type 5, 0x1B, modifiers, 0} to the notification target (result ignored).
  - Any other key: nothing.
  Then, for every key (including unhandled ones), send 0x9A1552D5 with data = the key code itself (not a pointer)
  and return true. Key-down never returns false, so the grid consumes all keys while enabled.
- **DoArrowKey(key, modifiers)** 0x009A85B6. Current cell = the last selection entry, or **(0, 1)** when the
  selection is empty (quirk: row 1, not 0). Read GetCellUsedRange (slot 50).
  - Left: column < 1: PlaySound(2) (slot 141) and stop. Else column - 1. Up: the same on the row.
  - Right: when flag 0x8000 is set and the column limit +0x138 is -1 or 0, column + 1 with no bound. Otherwise the
    bound is the limit (flag 0x8000 set) or used right + 1 (clear); when column >= bound - 1: flag set: PlaySound(2)
    and stop; flag clear: keep the column (no sound) and continue. Else column + 1.
    Down: the same with flag 0x10000, row limit +0x13C and used bottom.
  - Any other key: return at once.
  Then: when the selection is non-empty and modifier 0x1 (Shift) is clear, empty the list directly (0x009A841F).
  Send 0x9A1552D2 with a pointer to the new cell, **then** AddCellToSelection(col, row, true). Keep it visible:
  Left: column < +0x128: ScrollToColumn(column). Up: row < +0x12C: ScrollToRow(row). Right: when (float)column >=
  (float)+0x128 + +0x130: ScrollToColumn(truncate(column - +0x130 + 1.0)) (x87, single-precision operands; constant
  1.0f at 0x00A81228). Down: the same with +0x12C, +0x134 and ScrollToRow. Finally SetScrollBarValues 0x009A71FD,
  layout-dirty = 1, recalculate.
  Worked example: a 3-column list, used range right = 2, flag 0x8000 clear, selection {(2, 4)}: Right keeps (2, 4),
  sends 0x9A1552D2, clears and re-adds (2, 4) (AddCellToSelection returns true because the list was just emptied).
- **GZOnKeyUp(key, modifiers)** 0x009A77ED: Escape (0x1B) sends message {type 6, 0x1B, modifiers, 0} to the
  notification target (window-view slot 145) and returns its result; any other key returns true.

### 5.10 Scroll bars, layout recalculation and row/column sizes

Two scroll-bar windows: +0x26C is the **vertical** bar (allowed by flag 4, follows rows), +0x270 the
**horizontal** bar (allowed by flag 2, follows columns). "Used range" below is slot 50 GetCellUsedRange
{minCol, minRow, maxCol, maxRow}; flags 0x8000 / 0x10000 make the column / row extent come from the cell limit
instead of the used range.

- **AddScrollBar(vertical, horizontal)** 0x009A6FD7. For each requested bar that is allowed and absent: create
  it through the window manager (RZ::WCM, slot +0x44) with class 0x1A7C4511 style 2 (vertical) or 0x1A7C4510
  style 1 (horizontal), range 0..100. Creation failure clears the allowing flag (ClearFlag(4) or (2)) instead.
  If an image is set (+0x274 vertical, +0x278 horizontal) the bar gets it through bar slot +0x44 (image rect,
  image, 0); the rect is the stored 16-byte source rect (+0x27C vertical, +0x28C horizontal) unless that rect is
  empty (right <= left or bottom <= top, test 0x007C1870), then the image's own rect (image slot +0x30). The bar
  window is added as a child (ChildAdd) and the previously focused window gets focus back. Layout-dirty = 1 only
  when at least one bar was created. Returns 1.
- **RemoveScrollBar(vertical, horizontal)** 0x009A7178: for each requested present bar: ChildDelete its window,
  Release, null the field. Layout-dirty = 1 if any was removed. Returns 1.
- **SetScrollBarValues()** 0x009A71FD. Vertical bar, only when present and visible rows (+0x134) > 0.0: if the
  row limit (+0x13C) is -1 or 0 and flag 0x10000 is set: range (0, trunc(visibleRows)) via bar slot +0x1C, then
  slot +0x3C(0x7FFFFFFF, 0x7FFFFFFF). Otherwise total = row limit, or (flag 0x10000 clear) usedRange.maxRow + 1;
  max = max(0, total - trunc(visibleRows)); range (0, max); slot +0x3C(max, trunc(total / visibleRows + 1.0)),
  the division done in x87 on the float field. Then position = vertical scroll (+0x12C) via slot +0x14.
  Horizontal bar the same with visible columns (+0x130, test is "not equal 0.0"), column limit (+0x138), flag
  0x8000, usedRange.maxCol + 1 and the horizontal scroll (+0x128); the used range is fetched once and shared.
  Returns 1.
- **SetScrollBarPositions()** 0x009A7405. W, H = grid window GetW/GetH. Vertical bar of width bw:
  SetArea(W - bw, top, W, H - (horizontal bar height, if present)), top = 0, or with flag 8 the column-header
  height (+0x110) + grid gutter top (+0xF4), +1 more with flag 0x200. Horizontal bar of height bh:
  SetArea(left, H - bh, W - (vertical bar width, if present), H), left = 0, or with flag 0x10 the row-header width
  (+0x114) + 1 + grid gutter left (+0xF0), +1 more with flag 0x200. *[compat]* The extra +1 exists only on the
  horizontal side.
- **TestForPossibleScrollBarAdditionOrRemoval()** 0x009A82C6. Vertical (flag 4): when the row limit is -1/0 and
  flag 0x10000 is set, the bar is always wanted; otherwise needed = (row limit, or with flag 0x10000 clear
  usedRange.maxRow + 1), and the bar is removed when (float)needed <= visible rows, else added. Horizontal (flag
  2): the same rule with the column count (limit +0x138 or usedRange.maxCol + 1) against +0x130.
  *[compat]* The horizontal test still reads the **row** limit (+0x13C) and flag **0x10000** to decide
  "always wanted" (0x009A835E, 0x009A836D), while its count uses +0x138/maxCol. Finally, if either bar exists:
  SetScrollBarValues, SetScrollBarPositions. Returns 1.
- **RecalculateAll()** 0x009AEE63. A global depth counter (0x00BAC064) allows two nested levels; at depth >= 2
  it returns 1 without doing anything. Otherwise depth += 1 and, only if the pause counter (+0xE8) < 1 and the
  window is initialised: grid area (slot 8) gives +0x118 = width, +0x11C = height; cell area (slot 9) gives
  +0x120 / +0x124. Visible columns (+0x130) restart at 0.0: column bound = column limit (-1/0 = 0x7FFFFFFF),
  lowered to usedRange.maxCol + 1 when flag 0x8000 is clear and maxCol <= bound. From the horizontal scroll
  column, integer widths (GetColumnWidthInternal) are summed; while the running sum is <= cell-area width,
  +0x130 += 1.0f; at the first column that overflows, f = (areaW - prevSum) / (sum - prevSum) (x87, stored as
  float) is added only if |f| > 0.05f (0x00A880D8), then stop. Visible rows (+0x134) the same with the row limit,
  flag 0x10000, maxRow, vertical scroll, GetRowHeightInternal and cell-area height. Then
  TestForPossibleScrollBarAdditionOrRemoval; if the set of present bars changed, RecalculateAll recurses (the
  depth counter stops a third level). At depth 1 PlaceAllCellWindows (§5.11) follows. Finally depth -= 1 and
  layout-dirty = 0 (also when paused). Returns 1.
  Worked example: cell area 250 wide, columns 100, 100, 100 from column 0: sums 100, 200 count 2.0; 300 > 250
  gives f = 50/100 = 0.5, visible columns = 2.5. With area 205 f = 0.05 is not > 0.05f, so 2.0.
- **GetColumnWidthInternal(col)** 0x009AD684. Column size entries live in the format-extra matrix +0x19C at
  (col, 0); row entries in +0x1C0 at (0, row). Entry +0x24 (s16) is the explicit size, +0x26 (s16) the cached
  actual size, -1 = unset. Explicit if set, else cached if set; else if the column has no entry and no cells in
  the cell matrix (+0x1E4) the default width +0x140 (100) without caching; otherwise the default width is cached
  (SetSizeCachedActualForColumn(col, 1, w)) and returned. *[compat]* Column widths never measure content.
- **GetRowHeightInternal(row)** 0x009AE387. Base h = default row height +0x144 (20), or cell-area height / row
  limit (integer division) when a row limit other than 0/-1 is set. Explicit, else cached size if set. A row with
  no entry and no cells returns +0x144 (not the divided base) uncached. Otherwise h = max(base,
  CalculateCellHeight of every cell in the row), cached via SetSizeCachedActualForRow(row, 1, h) and returned.
- **CalculateCellHeight(col, row, cell)** 0x009AD6FA (cell looked up when null). Content height c = +0x144, then
  by cell type: window (1) the window's GetH; text (2) with non-empty text: font = the cell's effective format
  font (slot 84 GetCellFormat), lineH = font slot +0x94(0, 0); if lineH != 0: lines = 1 when flag 0x800 or
  0x20000 is set, else font slot +0xAC(text, length, GetColumnWidthInternal(col)); c = lines x lineH; bitmap
  (4) the image slot +0x28 height. Result = cell gutter top (+0x104) + cell gutter bottom (+0x10C) + c.
- **SetSizeCachedActualForRow(row, count, value)** 0x009ACC5A / **ForColumn(col, count, value)** 0x009ACD43.
  count 0x7FFFFFFF means "to the limit": count = 0x7FFFFFFF - start, clipped to (limit - 1) - start + 1 when a
  row (column) limit other than -1/0 exists; and if then value = -1, every entry of the matrix (all rows, not
  only from start) gets cached = -1 and nothing else happens. Otherwise for count consecutive indices from
  start: an existing entry gets +0x26 = (s16)value; a missing entry is inserted (default-constructed) and set
  only when value != -1.

WG2 answer: +0x140 is the default column width and +0x144 the default row height (and the base content height).

### 5.11 Cell data, cell windows and moving cells

A cell record (28 bytes, operator new via 0x005A6290) is {type, col, row, payload, format*, extra, cellFlags}:
type 1 window (payload = window), 2 text (payload = cRZString), 4 bitmap (payload = image); format* = an
optional 36-byte per-cell format whose first field is a ref-counted font; extra = cell extra data; cellFlags bit
0/1 = window keeps grid width/height. Records live in the cell matrix +0x1E4; window cells also live in an
ordered set of {type, col, row, window, ...} keyed records at +0x22C.

Coordinates 0x7FFFFFFF in the three setters mean "append": column = (last used column of that row) + 1, or 0 for
an empty row; row = (last used row of that column) + 1, or 0; both 0x7FFFFFFF take column = max used column of
the matrix (0 if empty) and then the row rule.

- **SetCellTextValue(col, row, text, length, mode, editable)** 0x009AE8CB. mode 0 = 1. The existing cell window
  is looked up (slot 115) unless a coordinate is 0x7FFFFFFF. Editable: if that window has interface 0x231A1C57
  its text is replaced (slot +0xC4(text, length, 0)); else the cell is cleared (slot 96) and a new
  cGZWinTextEdit (style 0x11FF) gets the text and is placed with SetCellWindow(col, row, w, 1, 1, 1) on the
  unresolved coordinates. Returns 1. Plain: an existing cell window is cleared first; coordinates are resolved;
  out of bounds (slot 54) clears that cell and returns 1. Null text clears the cell. Otherwise length -1 or 0
  means strlen; MakeRoom(col, row, 1, mode); a new type-2 record is inserted; layout-dirty = 1; recalculation
  hook 0x009A5FAA. In bounds, +0x15C/+0x160 = the cell written (last set cell). Returns 1.
- **SetCellBitmap(col, row, image, mode)** 0x009AEB66: the same resolve/bounds/clear rules; out of bounds returns
  0. AddRef image, MakeRoom(…, 1, mode), type-4 record, dirty, hook; +0x15C/+0x160 updated; returns 1.
- **SetCellWindow(col, row, win, keepW, keepH, mode)** 0x009AECB7: returns 0 when win is already a cell window
  (slot 117). Resolve; out of bounds clears and returns 0. Null win clears the cell. Else AddRef, MakeRoom,
  type-1 record with cellFlags bit 0 = keepW, bit 1 = keepH, insert, attach (0x009AA91A), Release, dirty, hook,
  last set cell, returns 1.
- **Attach** 0x009AA91A: inserts {1, col, row, win} into the +0x22C set, AddRef, win SendToBack (+0x60),
  win SetFlag(0x800, 0), ChildAdd(win), then **place** 0x009A6C45.
- **Place(col, row, win, keepW, keepH)** 0x009A6C45 / **PlaceAllCellWindows()** 0x009AA991 (every window record
  of the set): only when the grid window is initialised. A cell outside the visible range (slot 51) hides its
  window. Otherwise rect = GetCellRect (slot 106) shrunk by the cell gutters (left/top added, right/bottom
  subtracted); ShowWindow; GZWinMoveTo(left, top); keepW: SetW(right - left); keepH: SetH(bottom - top).
  *[compat]* Place additionally calls SetArea(rect) at the end; PlaceAllCellWindows does not.
- **DoWindowRemoval(col, row, win)** 0x009A9837: erase {1, col, row, win} from the set; if it was there, remove
  the child (ChildRemove) and Release. **DoCellWindowMove(cell)** 0x009A8F7A only scans the set for the window
  and returns: no effect.
- **DisposeOfCellData(col, row, cell, free)** 0x009AAB01 (cell looked up when null; none returns 0): window ->
  DoWindowRemoval; text -> delete the string; bitmap -> Release; payload = 0 (type other than 1/2/4 keeps it).
  Format: Release its font, free it. Extra: Release only in extra-data mode 2 (+0x188), then 0. free: free the
  record. The matrix entry is not removed. Returns 1.
- **ClearCell(col, row)** slot 96, 0x009AB897: deselect (AddCellToSelection(col, row, false)); remove the matrix
  entry and dispose it with free (dirty = 1); hook 0x009A5FAA when something was removed or deselected. Returns 1.
- **DisposeOfAllCellData()** 0x009ABAEF: disposes (with free) every record of the whole matrix, clears it, count
  +0x1F0 = 0. Returns 1.
- **CopyCellData(col, row, toCol, toRow, copy)** 0x009AD512: false when the source is out of bounds; true when
  source = destination. Out-of-bounds destination: copy returns true; move falls back to destination = source
  (net no change). Move: remove the source entry; none -> ClearCell(destination); else find or insert the
  destination record, dispose its content, take over all 7 fields with the new coordinates, free the old record.
  Copy: missing source -> ClearCell(destination); else find or insert the destination record and
  **record copy** 0x009AB46A. Returns 1.
- **Record copy(src, toCol, toRow, dst)** 0x009AB46A: dst content is disposed first (when dst exists); a window
  source then returns 0 (dst left empty). dst gets type, coordinates, cellFlags; text is duplicated (empty string
  for a null source text), a bitmap is AddRef'd and shared; a format is duplicated (36 bytes) with its font
  AddRef'd; extra data is not copied (0). Returns 1.
- **GetCellTextValue(col, row, out)** slot 111, 0x009AA813: a text cell assigns its text (out slot +0xC); otherwise
  a cell window with interface 0x231A1C57 supplies it (slot +0xB4); else false.
- **FindCellText(text, length, col0, nCols, row0, nRows, &col, &row, exactLength)** slot 133, 0x009A9892: null
  text = false; length -1/0 = strlen. Walks the cell matrix in its order; the first text cell with col0 <= col <
  col0 + nCols and row0 <= row < row0 + nRows (0x7FFFFFFF counts = unbounded) whose string contains text
  (substring find at 0; with exactLength only strings of exactly that length) writes its coordinates and returns
  true.
- **MakeRoom(col, row, n, mode)** 0x009ADE44 (mode 0 = 1; others than 1..5 return 0). All moves use CopyCellData
  (move) and reset the cached size of each touched column (modes 2/4) or row (3/5); each mode first resets row
  `row`'s cache. Mode 1, n > 0: dispose and free the record at (col, row) (entry stays, the caller overwrites it).
  Mode 2 (row, leftwards), with the row's used columns [min, max]: n > 0 moves c + n -> c for c = max(min - n, 0)
  .. col - 1; n < 0 moves c -> c + n for c = col - n .. max. Mode 4 (row, rightwards): n > 0 moves c - n -> c for
  c = min(max + n, column limit unless -1) down to col; n < 0 moves c -> c - n for c = col - 1 down to min. Modes
  3 and 5 are the same along the column with rows and the row limit. n = 0 does nothing but the cache reset.
  Layout-dirty = 1; returns 1. *[compat]* the limit clip uses the limit itself (not limit - 1) and treats 0 as a
  limit.
- **MakeRoomForRow(row, n, mode)** 0x009AE282: mode 0 = 3; only 1, 3, 5 (else 0); MakeRoom(c, row, n, mode) for
  each used column c (used range) that has cells. **MakeRoomForColumn(col, n, mode)** 0x009AE2EF: mode 0 = 2;
  only 1, 2, 4; MakeRoom(col, r, n, mode) for each used row having cells. Return 1.

### 5.12 Painting (GZPaint 0x009AF168)

GZPaint is the window-view override (its `this` is the grid + 4). All rectangles are window-local plus the window
origin. "No colour" is the sentinel 0x40000000 in every colour field below. Order of one paint:

1. Layout-dirty (+0xE4) set: RecalculateAll. Then +0xE8 (pause counter) is forced to 0 *[compat]*.
2. Cell area (slot 9) offset by the window origin; used range (slot 50). Visible bounds: columns from the
   horizontal scroll c0 (+0x128) to cEnd = c0 + trunc(+0x130), rows r0 (+0x12C) to rEnd = r0 + trunc(+0x134);
   cEnd is lowered to maxCol + 1 when flag 0x8000 is clear (and maxCol <= cEnd), rEnd to maxRow + 1 when flag
   0x10000 is clear.
3. Flag 0x100 (fill background): with no background image (+0x164 null) fill the whole window with the
   window colour (window-view slot 126) via the draw context; with an image, draw it over the window rect.
4. Grid lines, only when flag 0x20 (column lines, colour +0x14C) or 0x40 (row lines, colour +0x150) is enabled
   with a colour. Line extents: column lines run from the cell-area top to min(area bottom - 1, top + height of
   rows r0..limit/maxRow - 1) and row lines likewise across. For each column boundary from c0 to cEnd - 1 (x =
   right edge of the column): if an owner-draw target (+0x168) exists it gets message 0xFA785CB0 with the line
   geometry, the draw context and the enabled state (view slot 67); a false answer skips the default line. The
   default line uses width (float)(u32)+0x154 then resets the width to 1.0f. Row lines: on the first column pass
   only, the same per row from r0 to rEnd - 1 with message 0xFA785CB1, width +0x158.
5. Flag 8: column headings for every visible column (slot 51 range): heading = column heading map entry
   (inserted when missing); an empty heading with flag 0x100000 becomes "Column #%d" (column index). Rect =
   slot 72 (window-view +0x120) heading rect + origin; the header background is painted by 0x009A60BB with the
   header format at +0x2C0, then the text through the header font (+0x2C0 slot +0xD0) at the position from
   0x009A5FE3. Flag 0x10: row headings the same with "Row #%d" and window-view +0x124.
6. Flag 0x20000: a background pass over every visible cell: format (slot 84) and selected state (slot 45),
   background painted by 0x009A60BB.
7. Cell pass, column by column then row by row over the visible bounds: cell rect = running sums of
   GetColumnWidthInternal / GetRowHeightInternal from the cell-area origin, clipped to the cell area
   (0x0048ED90); format (slot 84); cell record (null when empty); selected state; enabled state (view slot 67).
   Without an owner-draw target, or when the target answers message 0xFA785CB2 true, the default cell drawer
   0x009A7E52 runs.
8. When both scroll bars exist (+0x26C and +0x270) and flag 0x100 is set: the
   corner square right of the horizontal bar / below the vertical bar is filled like step 3 (shrunk by 1 with
   flag 0x200).
9. Flag 0x400: outline of the cell area; flag 0x200: outline of the window rect; both in the outline colour +0x148
   (RZDrawRectangleOutline), width 1.

Paint record passed to the helpers and the owner-draw target: +0x0C cell rect*, +0x10 clip rect*, +0x18 draw
context, +0x20 format* (+0x00 font, +0x08 background, +0x10 text colour, +0x18 selected colour, +0x1D horizontal
alignment byte: 3 centre, 4 right; +0x1E vertical alignment: 6 centre, 7 bottom), +0x24 cell record*, +0x28
selected, +0x29 enabled.

- **Background helper** 0x009A60BB: rect = cell rect, left += +0x154 when column lines are on, top += +0x158
  when row lines are on, clipped. Not selected: background +0x08; selected: +0x18 unless flag 0x80000. No colour
  = nothing. Fill uses draw-context capability 0x8001 enabled around the fill (slot +0x50).
- **Default cell drawer** 0x009A7E52: the same background step unless flag 0x20000 (already painted in pass 6).
  Content rect = cell rect shrunk by the cell gutters (+0x100..+0x10C). Window cells draw nothing (the child
  paints itself). Text: alignment and colour from 0x009A5FE3 (vertical centre = (top + bottom) / 2 - lineH / 2;
  colour +0x18 when selected, else +0x08 or +0x10 by the enabled byte); with flag 0x20000 the right edge is
  taken from the paint record +0x14; with flag 0x80000 and a colour, each wrapped line (font slot +0xA4 breaks
  at the content width) is filled as a selection band; then font slot +0xD0 draws the string. Bitmap: image
  size from slot +0x2C; placed at the content left/top, centred (3/6) or right/bottom aligned (4/7); with flag
  0x20000 the clip right edge from the record +0x14; drawn with capability 0x8010 enabled (context slot +0x74).
  Horizontal text alignment is supplied to the font as a draw flag, not as a measured x coordinate: 3 gives
  flag 1 (centre), 4 gives flag 2 (right), and every other value, including 5, gives flag 0. The content's
  left/right coordinates are unchanged by 0x009A5FE3. See §5.15 for the vertical-alignment quirk.

### 5.13 Cell rectangles, deleting and copying rows and columns

- **GetCellRect(col, row, &rect)** slot 106, 0x009AE813: left = width sum of columns before col (slot 70), right =
  left + GetColumnWidthInternal(col); top/bottom the same with rows (slot 71) and GetRowHeightInternal. Both
  edges are then shifted by minus the width sum before the horizontal scroll column (skipped when it is 0) and
  minus the height sum before the vertical scroll row, then plus the cell-area left/top (slot 9). Off-screen
  cells get rectangles outside the area. Returns 1.
- **DeleteColumns(col, n, mode)** 0x009AE5AC: mode 0 = 2, anything but 1/2/4 becomes 1. n < 0 returns 0, n = 0
  returns 1. Every cell in columns col .. col + n - 1 is disposed (freed), then the matrix drops those columns
  (RemoveCol). Mode 1: only column col's cached width is reset (cells to the right keep their indices). Mode 2:
  cached widths from col to the limit are reset, then MakeRoomForColumn(col, -n, 2) pulls the right-hand cells
  left. Mode 4: cached widths 0 .. col reset, then MakeRoomForColumn(col, -n, 4). Layout-dirty = 1, hook
  0x009A5FAA, returns 1. **DeleteRows(row, n, mode)** 0x009AE6DF is the same along rows (mode 0 = 3; modes 1/3/5;
  RemoveRow; SetSizeCachedActualForRow; MakeRoomForRow).
- **CopyColumns(from, to, n, move)** 0x009AD9DB: nothing when from = to. The n column pairs are processed
  forwards (from, from + 1, ...) when the ranges do not overlap or the destination end lies inside the source;
  otherwise backwards from the last column. Per pair: for every used row of the source column (its own min..max)
  the row's cached height is reset and CopyCellData(src, row, dst, row, copy = !move); the column extra data
  (+0x25C map) entry is copied to dst and erased from src when moving; the column heading (+0x2A8) is assigned
  to dst and, when moving, erased from src (0x009AB66B). Layout-dirty = 1. Returns 1. **CopyRows** 0x009ADB72 is
  the same with rows, the row extra data map +0x250, row headings and the per-column width cache.

### 5.14 Per-cell formats

A format is 36 bytes whose first field is a ref-counted font (see the paint record, §5.12, for the colour and
alignment fields).

- **SetCellFormat(col, row, format)** 0x009AC519. (-1, -1): the default format at +0x208 (36 bytes) is replaced:
  AddRef the new font, Release the old one, copy; returns 1 without dirty or hook. (-1, row) / (col, -1): the
  row entry (+0x1C0 at (0, row)) / column entry (+0x19C at (col, 0)) is found or inserted, the font AddRef'd and
  the old one Released, and 40 bytes built from the format by 0x009A5C4A are copied over the entry
  with both size words +0x24/+0x26 reset to -1 by 0x009A5C4A (§5.7); returns 1 without dirty or hook. (col, row):
  out of bounds (slot 54) returns 0. No cell record: a new empty record (type 0) with a copy of the format is
  inserted (changed). Existing record: AddRef the new font; no format yet -> one is allocated (changed); else the
  old font is Released (changed when it was non-null); the 36 bytes are copied. A non-null new font also counts
  as a change. Change -> layout-dirty = 1; hook 0x009A5FAA always. Returns 1.
- **SetCellFont(col, row, font)** slot 90, 0x009AC741: out of bounds (slot 54) returns 0. AddRef font; a missing
  record is created (type 0, default-constructed format) and inserted; a record without format gets one; an old
  font is Released. The font is stored at format +0x00 (0x009AC7D9–0x009AC7E1, new record:
  0x009AC832). When the new font is non-null or an old font was released, set layout-dirty and call hook
  0x009A5FAA (0x009AC845–0x009AC856). Returns 1.

### 5.15 Dependency contracts and initialization (WG5, WG6, WG7, WG9)

**Bounds and used range.** IsColumnWithinBounds 0x009A66FC and IsRowWithinBounds 0x009A6722 return true
for **every signed index** when the corresponding limit is 0 or -1. Otherwise the predicate is
`0 <= index < limit`. They do not inspect fixed-range flags. IsCellWithinBounds 0x009A6748 requires both
predicates. Thus a limit of -2 admits no index; an unlimited limit admits -2. GetCellUsedRange 0x009A97CE
reports inclusive extrema of entries in the cell-data matrix +0x1E4, including format-only records, independently
of limits, visibility and selection. An empty matrix writes (0,0,0,0) and returns false; a nonempty matrix
returns true. Evidence: 0x009A6702–0x009A671E and 0x009A97D3–0x009A9830.

**Selection membership.** GetCellIsSelected 0x009A8809 does not check bounds. Mode +0x18C = 0 matches any
selection entry with the requested column, ignoring row; mode 1 matches row, ignoring column; every other mode
requires both coordinates. An empty selection returns false. This applies to the stored selection list as it
stands, without synthesizing new entries (0x009A8809–0x009A8865).

**Relative scrolling.** ScrollCellsHorizontally 0x009AD817 and ScrollCellsVertically 0x009AE48F always return
true. Add the signed delta to the stored position using 32-bit integer arithmetic. With an unlimited (0/-1)
limit and flag 0x8000 (horizontal) or 0x10000 (vertical), only clamp the result at zero. Otherwise choose a total
count: the axis limit, replaced by usedRange.maxAxis + 1 when flag **0x10000 is clear**, for either axis.
The horizontal method deliberately tests the vertical flag for this replacement (0x009AD859).
Clamp the position to at most count - 1, then at least zero. If it is positive, back it up until the sum of
internal sizes from it to the end reaches the cell-area pixel extent or position reaches zero. This fills the
viewport near the end, unlike ScrollToColumn/Row. A zero/negative pixel extent does not cause backing up.
Finally clamp at zero; only a changed position calls the hook and marks layout dirty. These methods do not call
ScrollTo. Evidence: 0x009AD88D–0x009AD90F and 0x009AE4F1–0x009AE59E.

**Effective formatting.** GetCellFormat 0x009AA5CE always returns true, without a bounds check. Begin with the
36-byte default format +0x208, then merge row format (slot 88), column format (slot 87), then the cell's format
if it exists. Thus precedence is **cell > column > row > default**, independently per field. A null font inherits;
each of the six colour words +0x04..+0x18 and border colour +0x20 inherits only at 0x40000000; wrapping byte
+0x1C inherits at 0xFF; the alignment bytes +0x1D/+0x1E and border byte +0x1F inherit at zero. Other values,
including colour zero and wrapping zero, override. The result is a borrowed-font value copy: no AddRef/Release,
and no cached size words. Evidence: 0x009AA5E0–0x009AA7DD; row is merged before column.

**SetDefaultFormatting** 0x009A5CEE fills sentinel fields, retaining explicit caller choices. Both normal
(+0x208) and header (+0x2C0) fonts, when null, acquire font 0x68963C4C from WCM and AddRef it if found.
Colours below are obtained through the window's RGB conversion, not by assuming a particular pixel layout.

| Format field | Sentinel | Normal replacement | Header replacement |
|---|---|---|---|
| +0x04, +0x08 | 0x40000000 | black, white | black, white |
| +0x0C, +0x10 | 0x40000000 | RGB(192,192,192), white | RGB(128,128,128), RGB(220,220,220) |
| +0x14, +0x18 | 0x40000000 | white, black | black, RGB(220,220,255) |
| +0x1C wrapping | 0xFF | 1 | 1 |
| +0x1D horizontal | 0 | 2 | 2 |
| +0x1E vertical | 0 | 5 | 6 |
| +0x1F border | 0 | 0 | 0 |
| +0x20 border colour | 0x40000000 | black | black |

Independently, grid outline +0x148 is reset to converted black, and both line colours +0x14C/+0x150 to
0xFF000000 on every call. No dirty flag or change hook is emitted by this initializer. Byte writes, colour
sentinels and literal line colours are confirmed at 0x009A5D4B–0x009A5E16 and 0x009A5EA0–0x009A5F1B.

**Text placement** 0x009A5FE3 uses the horizontal flags specified in §5.12. Let h be font slot +0x94's line
height. Vertical alignment 6 replaces top with `trunc((top+bottom)/2) - trunc(h/2)` and bottom with top+h.
Otherwise it leaves top unchanged when **horizontal** alignment is 5; for every other horizontal value it
sets top = bottom-h, regardless of the vertical byte. This odd cross-field test is literal
(0x009A603E–0x009A6075). The integer halves truncate toward zero separately. Selected text uses colour +0x14
and selection fill +0x18; otherwise enabled text/fill use +0x04/+0x08, disabled use +0x0C/+0x10
(0x009A6078–0x009A60B4). These correct the abbreviated paint-record colour description above.

**Editable editor allocation failure.** In SetCellTextValue's editable branch, a missing allocation or null
constructor result produces a null text interface and an immediate null dereference while setting its text;
there is no successful return or recoverable false result. Any old incompatible cell window was already
cleared. Do not infer a rollback or a retry from this path (0x009AE958–0x009AE99E). If the new editor exists
but its subsequent window QueryInterface fails, the method instead returns true without attaching it
(0x009AE9A4–0x009AE9B9); that is a separate failure case.

**Header rectangles** (slots 72/73, 0x009A683D / 0x009A68E2) use local window coordinates and return false
when their header flag is clear. For a particular column, obtain GetCellRect(col,0), retaining left/right,
then replace top/bottom with gutterTop and gutterTop+headerHeight. For a particular row, obtain
GetCellRect(0,row), retaining top/bottom, then replace left/right with gutterLeft and gutterLeft+headerWidth.
A failed GetCellRect propagates false. Index -1 asks for the whole band: column =
(gutterLeft, gutterTop, windowWidth-gutterRight, gutterTop+headerHeight); row =
(gutterLeft, gutterTop, gutterLeft+**headerHeight**, windowHeight-gutterBottom).
The whole-row band's use of header height +0x110, rather than width +0x114, is a quirk
(0x009A6936–0x009A694A). No scroll-bar subtraction, outline inset, or extra separator pixel is applied here.

**Paint record +0x14** is a pointer to the whole cell-area rectangle in drawing coordinates, not an integer
edge or the per-cell clip. The cell-area rect comes from slot 9, translated by window origin. A cell's +0x10
clip points to its intersection with that area. With flag 0x20000, text content.right and its clip.right are
replaced with (+0x14)->right; bitmaps replace clip.right only. Thus text can extend across later cells while
retaining the row's vertical clip. Evidence: 0x009AFA29–0x009AFA34, 0x009AFA78–0x009AFA87,
0x009A8054–0x009A805A and 0x009A80D6–0x009A80E2.

## 6. Conformance (planned vectors)

1. SetDefaultFontColor(c, 6) returns false and leaves all six colours unchanged; index 5 succeeds.
2. ScrollPagesVertically(2) scrolls exactly 8 rows from the top of a 100-row grid.
3. SetSelectionRules clears an existing selection (GetSelectionCount becomes 0).
4. EnableFixedRanges(false, true) after SetCellLimits(10, 10): GetCellLimits returns (-1, 10).
5. PauseRecalculations(true), (true), (false) return 1, 2, 1; a further (false), (false) return 0, -1.
6. GetColumnExtraData(7) on a fresh grid returns 0 and SetColumnExtraData(7, x) on an out-of-bounds column
   returns false.

The following vectors are static contracts, not claims of game execution:

| ID | Setup / action | Required observation |
|---|---|---|
| WG5-a | Column limit 0, row limit 3; test (-9,2), (-9,3); repeat with column limit 2 | true, false; then (-9,2) is false regardless of fixed-range flags |
| WG5-b | Empty matrix; GetCellUsedRange; then create only a format record at (4,7) | false and (0,0,0,0); then true and (4,7,4,7) |
| WG5-c | Selection contains (2,3); query (2,99), (99,3), (2,3) | mode 0: true/false/true; mode 1: false/true/true; mode 2: false/false/true |
| WG5-d | Ten rows of height 20, cell area height 60, flag 0x10000, limit 10, position 0; relative +9 | position 7, since rows 7..9 fill 60 pixels; ScrollToRow(9) instead gives 9 |
| WG5-e | Row font R and red text; column font C and inherited text; cell null font and blue background | resolved font C, red text, blue background; setting cell wrapping 0 overrides inherited wrapping 1 |
| WG6-a | Sentinel normal/header formats; run SetDefaultFormatting with an RGB conversion stub | normal wrapping/horizontal/vertical = 1/2/5, header = 1/2/6; normal disabled pair = grey192/white; header selected pair = black/RGB(220,220,255) |
| WG6-b | Replace axis format whose cached words were 12 and 34 | copied 36 format bytes, both size words become -1 |
| WG6-c | Content (10,20,110,60), h=9; horizontal 3/4/5, vertical 7 | horizontal flags 1/2/0; top = 51/51/20, left/right remain 10/110 |
| WG6-d | Same rect/h, vertical 6 | top = 40-4 = 36, bottom = 45 for every horizontal alignment |
| WG7-a | Existing incompatible child; editable text; inject null editor allocation | old cell cleared, then null-interface fault, no boolean result or rollback |
| WG9-a | W=300,H=200, gutters (3,5,7,11), header height 20,width 40; whole bands | column (3,5,293,25); row (3,5,23,189), deliberately width 20 |
| WG9-b | Cell rect (43,25,93,45); query column header / row header | column (43,5,93,25); row (3,25,43,45) |
| WG9-c | Per-cell clip right 93, whole translated cell-area right 293; flag 0x20000 | text content and clip right 293; bitmap clip right 293; bitmap destination remains cell aligned |

## 7. Open questions

See `questions/win-grid.md`.

## A. Owner inventory (198 functions)

<!-- BEGIN INVENTORY -->
| Address | Bytes | Name (slot) | Spec section |
|---|---|---|---|
| 0047d840 | 48 | FUN_0047d840 | 5.7 |
| 005f8f70 | 8 | __ZThn4_N10cGZWinGrid14QueryInterfaceEmPPv | 5.7 |
| 005f8f90 | 8 | __ZThn4_N10cGZWinGrid7ReleaseEv | 5.7 |
| 005f8fa0 | 8 | __ZThn4_N10cGZWinGrid4InitEv | 5.7 |
| 005f8fb0 | 8 | __ZThn4_N10cGZWinGrid8ShutdownEv | 5.7 |
| 005f8fc0 | 8 | __ZThn4_N10cGZWinGrid7SetAreaEllll | 5.7 |
| 0066e2f0 | 38 | FUN_0066e2f0 | 5.7 |
| 007bffa0 | 28 | FUN_007bffa0 | 5.7 |
| 008940cd | 30 | FUN_008940cd | 5.7 |
| 009a5c20 | 22 | FUN_009a5c20 | 5.7 |
| 009a5c4a | 37 | FUN_009a5c4a | 5.7 |
| 009a5c9f | 22 | FUN_009a5c9f | 5.7 |
| 009a5cb5 | 34 | QueryInterface | 1 |
| 009a5cd7 | 23 | FinalRelease | 5.1 |
| 009a5cee | 560 | SetDefaultFormatting | pending |
| 009a5f1e | 5 | GZOnCharacter | 5.1 |
| 009a5f23 | 37 | FUN_009a5f23 | 5.1 |
| 009a5f48 | 40 | FUN_009a5f48 | 5.1 |
| 009a5f70 | 13 | MakeDropDown [slot 139] | 5.6 |
| 009a5f7d | 28 | GZOnMouseWheel | 5.1 |
| 009a5fe3 | 216 | FUN_009a5fe3 | pending |
| 009a60bb | 228 | FUN_009a60bb | 5.12 |
| 009a619f | 133 | SetArea | 5.1 |
| 009a6224 | 96 | SetFlag | 5.1 |
| 009a62cf | 33 | FUN_009a62cf | 5.7 |
| 009a62f0 | 67 | SetDefaultFont [slot 15] | 5.2 |
| 009a6333 | 7 | GetDefaultFont [slot 16] | 5.2 |
| 009a633a | 29 | SetDefaultFontColor [slot 17] | 5.2 |
| 009a6357 | 31 | GetDefaultFontColor [slot 18] | 5.2 |
| 009a6376 | 62 | SetDefaultTextWrapping [slot 21] | 5.2 |
| 009a63b4 | 11 | GetDefaultTextWrapping [slot 22] | 5.2 |
| 009a63bf | 53 | GetGutters [slot 23] | 5.2 |
| 009a63f4 | 57 | SetGutters [slot 24] | 5.2 |
| 009a642d | 53 | GetCellGutters [slot 25] | 5.2 |
| 009a6462 | 45 | SetCellGutters [slot 26] | 5.2 |
| 009a648f | 36 | SetOutlineUse [slot 27] | 5.2 |
| 009a64b3 | 24 | PauseRecalculations [slot 29] | 5.3 |
| 009a64cb | 16 | ScrollPagesVertically [slot 33] | 5.3 |
| 009a64db | 19 | ScrollPagesHorizontally [slot 34] | 5.3 |
| 009a64ee | 117 | ScrollToColumn [slot 35] | 5.8 |
| 009a6563 | 117 | ScrollToRow [slot 36] | 5.8 |
| 009a65d8 | 35 | ScrollToCell [slot 37] | 5.3 |
| 009a65fb | 29 | GetSelectionRules [slot 38] | 5.3 |
| 009a6618 | 33 | SetSelectionRules [slot 39] | 5.3 |
| 009a6639 | 69 | EnableFixedRanges [slot 47] | 5.3 |
| 009a667e | 25 | SetCellLimits [slot 48] | 5.3 |
| 009a6c45 | 232 | FUN_009a6c45 | 5.11 |
| 009a6d2d | 22 | SetExtraData [slot 125] | 5.5 |
| 009a6d43 | 7 | GetExtraData [slot 126] | 5.5 |
| 009a6d4a | 61 | GetCellExtraDataObject [slot 127] | 5.5 |
| 009a6d87 | 136 | SetCellExtraDataObject [slot 128] | 5.8 |
| 009a6e0f | 19 | GetExtraDataObject [slot 129] | 5.5 |
| 009a6e22 | 55 | SetExtraDataObject [slot 130] | 5.5 |
| 009a6e59 | 55 | FUN_009a6e59 | 5.7 |
| 009a6e90 | 45 | SetOwnerDrawNotification [slot 134] | 5.6 |
| 009a6ebd | 29 | GetBufferToDrawTo [slot 135] | 5.6 |
| 009a6eda | 29 | SetSoundID [slot 138] | 5.6 |
| 009a6ef7 | 112 | SetScrollBarImageVertical [slot 136] | 5.8 |
| 009a6f67 | 112 | SetScrollBarImageHorizontal [slot 137] | 5.8 |
| 009a6fd7 | 417 | AddScrollBar | 5.10 |
| 009a7178 | 133 | RemoveScrollBar | 5.10 |
| 009a71fd | 520 | SetScrollBarValues | 5.10 |
| 009a7405 | 370 | SetScrollBarPositions | 5.10 |
| 009a75a3 | 44 | FUN_009a75a3 | 5.7 |
| 009a75cf | 65 | FUN_009a75cf | 5.7 |
| 009a7610 | 22 | FUN_009a7610 | 5.7 |
| 009a7626 | 42 | FUN_009a7626 | 5.7 |
| 009a7650 | 21 | Release | 5.1 |
| 009a7665 | 117 | DoMessage | 5.1 |
| 009a76da | 90 | GetGridArea [slot 8] | 5.1 |
| 009a7734 | 185 | GetCellArea [slot 9] | 5.1 |
| 009a77ed | 57 | GZOnKeyUp | 5.9 |
| 009a7826 | 448 | GZOnMouseUpL | 5.9 |
| 009a79e6 | 643 | GZOnMouseMove | 5.9 |
| 009a7c69 | 419 | DoCursorHitTest [slot 110] | 5.9 |
| 009a7e0c | 35 | GZOnSetFocus | 5.1 |
| 009a7e2f | 35 | GZOnKillFocus | 5.1 |
| 009a7e52 | 876 | FUN_009a7e52 | 5.12 |
| 009a81be | 144 | SetFlags | 5.1 |
| 009a824e | 120 | GZOnCharacter [slot 141] | 5.8 |
| 009a82c6 | 308 | TestForPossibleScrollBarAdditionOrRemoval | 5.10 |
| 009a8446 | 45 | FUN_009a8446 | 5.7 |
| 009a8473 | 45 | FUN_009a8473 | 5.7 |
| 009a8574 | 29 | FUN_009a8574 | 5.7 |
| 009a8591 | 37 | FUN_009a8591 | 5.7 |
| 009a85b6 | 550 | DoArrowKey | 5.9 |
| 009a87dc | 45 | GetSelection [slot 42] | 5.8 |
| 009a8888 | 60 | FUN_009a8888 | 5.7 |
| 009a88df | 41 | FUN_009a88df | 5.7 |
| 009a8908 | 41 | FUN_009a8908 | 5.7 |
| 009a89af | 137 | FUN_009a89af | 5.7 |
| 009a8a38 | 28 | FUN_009a8a38 | 5.7 |
| 009a8ba4 | 48 | FUN_009a8ba4 | 5.7 |
| 009a8bd4 | 74 | FUN_009a8bd4 | 5.7 |
| 009a8c1e | 74 | FUN_009a8c1e | 5.7 |
| 009a8c68 | 261 | FUN_009a8c68 | 5.7 |
| 009a8d6d | 425 | GZOnKeyDown | 5.9 |
| 009a8f16 | 100 | IsWindowACellWindow [slot 117] | 5.8 |
| 009a8f7a | 68 | DoCellWindowMove | 5.11 |
| 009a8fbe | 23 | FUN_009a8fbe | 5.7 |
| 009a8fd5 | 23 | FUN_009a8fd5 | 5.7 |
| 009a90f8 | 42 | FUN_009a90f8 | 5.7 |
| 009a93a1 | 134 | FUN_009a93a1 | 5.7 |
| 009a963e | 39 | FUN_009a963e | 5.7 |
| 009a9665 | 39 | FUN_009a9665 | 5.7 |
| 009a968c | 86 | FUN_009a968c | 5.7 |
| 009a96e2 | 27 | FUN_009a96e2 | 5.7 |
| 009a9765 | 105 | SetSelection [slot 43] | 5.9 |
| 009a9837 | 91 | DoWindowRemoval | 5.11 |
| 009a9892 | 267 | FindCellText [slot 133] | 5.11 |
| 009a999d | 35 | FUN_009a999d | 5.7 |
| 009a9a09 | 67 | FUN_009a9a09 | 5.7 |
| 009a9a4c | 61 | FUN_009a9a4c | 5.7 |
| 009a9d2e | 78 | FUN_009a9d2e | 5.7 |
| 009a9d7c | 25 | FUN_009a9d7c | 5.7 |
| 009a9d95 | 196 | FUN_009a9d95 | 5.7 |
| 009a9e59 | 138 | FUN_009a9e59 | 5.7 |
| 009a9ee3 | 135 | FUN_009a9ee3 | 5.7 |
| 009a9f6a | 1143 | GZOnMouseDownL | 5.9 |
| 009aa3e1 | 150 | ActLikeAListbox | 5.9 |
| 009aa477 | 293 | AddCellToSelection | 5.9 |
| 009aa813 | 151 | GetCellTextValue [slot 111] | 5.11 |
| 009aa91a | 119 | FUN_009aa91a | 5.11 |
| 009aa991 | 320 | PlaceAllCellWindows | 5.11 |
| 009aaad1 | 48 | GetCellExtraData [slot 120] | 5.5 |
| 009aab01 | 197 | DisposeOfCellData | 5.11 |
| 009aabc6 | 116 | FUN_009aabc6 | pending |
| 009aad55 | 31 | FUN_009aad55 | pending |
| 009aad74 | 63 | FUN_009aad74 | pending |
| 009aadb3 | 272 | FUN_009aadb3 | pending |
| 009aaec3 | 346 | FUN_009aaec3 | pending |
| 009ab01d | 110 | FUN_009ab01d | pending |
| 009ab08b | 131 | FUN_009ab08b | pending |
| 009ab10e | 138 | FUN_009ab10e | pending |
| 009ab198 | 68 | SetColumnExtraData [slot 121] | 5.5 |
| 009ab1dc | 30 | GetColumnExtraData [slot 122] | 5.5 |
| 009ab1fa | 68 | SetRowExtraData [slot 123] | 5.5 |
| 009ab23e | 30 | GetRowExtraData [slot 124] | 5.5 |
| 009ab25c | 61 | FUN_009ab25c | pending |
| 009ab310 | 55 | FUN_009ab310 | pending |
| 009ab37e | 178 | FUN_009ab37e | pending |
| 009ab46a | 275 | CopyCellData | 5.11 |
| 009ab57d | 68 | FUN_009ab57d | pending |
| 009ab5c1 | 41 | FUN_009ab5c1 | pending |
| 009ab613 | 88 | FUN_009ab613 | pending |
| 009ab66b | 73 | FUN_009ab66b | pending |
| 009ab7e6 | 37 | FUN_009ab7e6 | pending |
| 009ab80b | 23 | FUN_009ab80b | pending |
| 009ab822 | 23 | FUN_009ab822 | pending |
| 009ab897 | 103 | ClearCell | 5.11 |
| 009ab9ab | 26 | FUN_009ab9ab | pending |
| 009ab9c5 | 26 | FUN_009ab9c5 | pending |
| 009abaef | 116 | DisposeOfAllCellData | 5.11 |
| 009abc95 | 567 | cGZWinGrid | 5.1 |
| 009abecc | 8 | __ZThn4_N10cGZWinGridD0Ev | 5.7 |
| 009abed4 | 175 | Shutdown | 5.1 |
| 009abf83 | 172 | FUN_009abf83 | 5.1 |
| 009ac24d | 28 | ~cGZWinGrid | 5.1 |
| 009ac43b | 111 | SetColumnWidth [slot 65] | 5.8 |
| 009ac4aa | 111 | SetRowHeight [slot 67] | 5.8 |
| 009ac519 | 552 | SetCellFormat | 5.14 |
| 009ac741 | 293 | SetCellFont [slot 90] | pending |
| 009ac866 | 250 | SetCellColors [slot 91] | pending |
| 009ac960 | 242 | SetCellWrapping [slot 92] | pending |
| 009aca52 | 260 | SetCellAlignment [slot 93] | pending |
| 009acb56 | 260 | SetCellBorder [slot 94] | pending |
| 009acc5a | 233 | SetSizeCachedActualForRow | 5.10 |
| 009acd43 | 236 | SetSizeCachedActualForColumn | 5.10 |
| 009ace2f | 167 | SetCellExtraData [slot 119] | pending |
| 009aced6 | 786 | SortColumn | pending |
| 009ad1e8 | 810 | SortRow | pending |
| 009ad512 | 370 | CopyCellData | 5.11 |
| 009ad684 | 118 | GetColumnWidthInternal | 5.10 |
| 009ad6fa | 285 | CalculateCellHeight | 5.10 |
| 009ad817 | 282 | ScrollCellsHorizontally | pending |
| 009ad931 | 45 | GetRowHeading [slot 80] | 5.5 |
| 009ad95e | 40 | SetRowHeading [slot 81] | 5.5 |
| 009ad986 | 45 | GetColumnHeading [slot 78] | 5.5 |
| 009ad9b3 | 40 | SetColumnHeading [slot 79] | 5.5 |
| 009ad9db | 407 | CopyColumns | 5.13 |
| 009adb72 | 407 | CopyRows | 5.13 |
| 009add09 | 315 | ResizeColumnToFitCellWindows | pending |
| 009ade44 | 1086 | MakeRoom | 5.11 |
| 009ae282 | 109 | MakeRoomForRow | 5.11 |
| 009ae2ef | 109 | MakeRoomForColumn | 5.11 |
| 009ae35c | 43 | GetColumnWidth | 5.4 |
| 009ae387 | 264 | GetRowHeightInternal | 5.10 |
| 009ae48f | 285 | ScrollCellsVertically | pending |
| 009ae5ac | 307 | DeleteColumns | 5.13 |
| 009ae6df | 308 | DeleteRows | 5.13 |
| 009ae813 | 184 | GetCellRect | 5.13 |
| 009ae8cb | 667 | SetCellTextValue | 5.11 |
| 009aeb66 | 337 | SetCellBitmap [slot 114] | 5.11 |
| 009aecb7 | 428 | SetCellWindow [slot 116] | 5.11 |
| 009aee63 | 659 | RecalculateAll | 5.10 |
| 009af0f6 | 43 | GetRowHeight | 5.4 |
| 009af121 | 71 | Init | 5.1 |
| 009af168 | 2813 | GZPaint | 5.12 |

Specified 165 of 198 functions, 26103 of 32735 bytes; remaining 33 functions, 6632 bytes.
<!-- END INVENTORY -->
