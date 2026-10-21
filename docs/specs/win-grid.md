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
| +0x110 / +0x114 | s32 | 20 / 80 | default row height / default column width *[inferred from values; setters are outside the inventory]* |
| +0x118 / +0x11C | s32 | 1 / 1 | grid-area width / height (§5.1) |
| +0x120 / +0x124 | s32 | 1 / 1 | cell-area width / height (§5.1) |
| +0x128 | s32 | 0 | *[unknown]* |
| +0x12C / +0x130 / +0x134 | f32 | 0.0, 1.0, 1.0 | *[unknown floats]* |
| +0x138 / +0x13C | s32 | -1 / -1 | column / row cell limit; -1 = unlimited (§5.3) |
| +0x140 / +0x144 | s32 | 100 / 20 | *[unknown]* |
| +0x148 | u32 | 0 | outline colour (§5.2) |
| +0x14C..+0x158 | s32 | 0,0,1,1 | *[unknown]* |
| +0x15C / +0x160 | s32 | -1 / -1 | *[unknown, cell coordinate pair]* |
| +0x164 | IUnknown* | null | released in Shutdown *[purpose pending]* |
| +0x168 | IUnknown* | null | owner-draw notification target (§5.6) |
| +0x170 | sub-object | ctor(…,4) | embedded object *[unknown]* |
| +0x188 | u32 | 0 | extra-data mode: 1 = plain value, 2 = object (§5.5) |
| +0x18C / +0x190 | u32 | 1 / 0x7FFFFFFF | selection rules: mode, maximum count (§5.3) |
| +0x194 | std::list | empty | selection list of cell coordinates |
| +0x198 | u32 | 2 | *[unknown]* |
| +0x1E4 | sparse matrix | | per-cell format-extra matrix; cell entry +0x14 = cell extra data (§5.5) |
| +0x208 | IUnknown* | null | default font, ref-counted (§5.2) |
| +0x20C..+0x220 | 6 x u32 | | default font colours, index 0..5 (§5.2) |
| +0x238..+0x24C | | 0,-1,-1,0,-1,-1 | *[unknown]* |
| +0x250 / +0x25C | map<u32,u32> | empty | row / column extra data (§5.5) |
| +0x268 | u32 or IUnknown* | 0 | grid extra data (§5.5) |
| +0x26C / +0x270 | IUnknown* | null | the two scroll bars (§pending) |
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
| 70 | 009AE35C own | GetColumnWidth | | 141 | 009A824E own | *[Mac name GZOnCharacter does not fit; pending]* |

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

(round 1: pending detail beyond §2; see §A)

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
| 0x009A5C4A | copy-construct a 40-byte cell record: copies the first 36 bytes from the source, then the two 16-bit fields at +0x24 and +0x26 are -1 (not copied) |

## 6. Conformance (planned vectors)

1. SetDefaultFontColor(c, 6) returns false and leaves all six colours unchanged; index 5 succeeds.
2. ScrollPagesVertically(2) scrolls exactly 8 rows from the top of a 100-row grid.
3. SetSelectionRules clears an existing selection (GetSelectionCount becomes 0).
4. EnableFixedRanges(false, true) after SetCellLimits(10, 10): GetCellLimits returns (-1, 10).
5. PauseRecalculations(true), (true), (false) return 1, 2, 1; a further (false), (false) return 0, -1.
6. GetColumnExtraData(7) on a fresh grid returns 0 and SetColumnExtraData(7, x) on an out-of-bounds column
   returns false.

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
| 009a5cd7 | 23 | FinalRelease | pending |
| 009a5cee | 560 | SetDefaultFormatting | pending |
| 009a5f1e | 5 | GZOnCharacter | pending |
| 009a5f23 | 37 | FUN_009a5f23 | pending |
| 009a5f48 | 40 | FUN_009a5f48 | pending |
| 009a5f70 | 13 | MakeDropDown [slot 139] | 5.6 |
| 009a5f7d | 28 | GZOnMouseWheel | pending |
| 009a5fe3 | 216 | FUN_009a5fe3 | pending |
| 009a60bb | 228 | FUN_009a60bb | pending |
| 009a619f | 133 | SetArea | pending |
| 009a6224 | 96 | SetFlag | pending |
| 009a62cf | 33 | FUN_009a62cf | pending |
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
| 009a64ee | 117 | ScrollToColumn [slot 35] | pending |
| 009a6563 | 117 | ScrollToRow [slot 36] | pending |
| 009a65d8 | 35 | ScrollToCell [slot 37] | 5.3 |
| 009a65fb | 29 | GetSelectionRules [slot 38] | 5.3 |
| 009a6618 | 33 | SetSelectionRules [slot 39] | 5.3 |
| 009a6639 | 69 | EnableFixedRanges [slot 47] | 5.3 |
| 009a667e | 25 | SetCellLimits [slot 48] | 5.3 |
| 009a6c45 | 232 | FUN_009a6c45 | pending |
| 009a6d2d | 22 | SetExtraData [slot 125] | 5.5 |
| 009a6d43 | 7 | GetExtraData [slot 126] | 5.5 |
| 009a6d4a | 61 | GetCellExtraDataObject [slot 127] | 5.5 |
| 009a6d87 | 136 | SetCellExtraDataObject [slot 128] | pending |
| 009a6e0f | 19 | GetExtraDataObject [slot 129] | 5.5 |
| 009a6e22 | 55 | SetExtraDataObject [slot 130] | 5.5 |
| 009a6e59 | 55 | FUN_009a6e59 | pending |
| 009a6e90 | 45 | SetOwnerDrawNotification [slot 134] | 5.6 |
| 009a6ebd | 29 | GetBufferToDrawTo [slot 135] | 5.6 |
| 009a6eda | 29 | SetSoundID [slot 138] | 5.6 |
| 009a6ef7 | 112 | SetScrollBarImageVertical [slot 136] | pending |
| 009a6f67 | 112 | SetScrollBarImageHorizontal [slot 137] | pending |
| 009a6fd7 | 417 | AddScrollBar | pending |
| 009a7178 | 133 | RemoveScrollBar | pending |
| 009a71fd | 520 | SetScrollBarValues | pending |
| 009a7405 | 370 | SetScrollBarPositions | pending |
| 009a75a3 | 44 | FUN_009a75a3 | pending |
| 009a75cf | 65 | FUN_009a75cf | pending |
| 009a7610 | 22 | FUN_009a7610 | pending |
| 009a7626 | 42 | FUN_009a7626 | pending |
| 009a7650 | 21 | Release | pending |
| 009a7665 | 117 | DoMessage | pending |
| 009a76da | 90 | GetGridArea [slot 8] | pending |
| 009a7734 | 185 | GetCellArea [slot 9] | pending |
| 009a77ed | 57 | GZOnKeyUp | pending |
| 009a7826 | 448 | GZOnMouseUpL | pending |
| 009a79e6 | 643 | GZOnMouseMove | pending |
| 009a7c69 | 419 | DoCursorHitTest [slot 110] | pending |
| 009a7e0c | 35 | GZOnSetFocus | pending |
| 009a7e2f | 35 | GZOnKillFocus | pending |
| 009a7e52 | 876 | FUN_009a7e52 | pending |
| 009a81be | 144 | SetFlags | pending |
| 009a824e | 120 | GZOnCharacter [slot 141] | pending |
| 009a82c6 | 308 | TestForPossibleScrollBarAdditionOrRemoval | pending |
| 009a8446 | 45 | FUN_009a8446 | pending |
| 009a8473 | 45 | FUN_009a8473 | pending |
| 009a8574 | 29 | FUN_009a8574 | pending |
| 009a8591 | 37 | FUN_009a8591 | pending |
| 009a85b6 | 550 | DoArrowKey | pending |
| 009a87dc | 45 | GetSelection [slot 42] | pending |
| 009a8888 | 60 | FUN_009a8888 | pending |
| 009a88df | 41 | FUN_009a88df | pending |
| 009a8908 | 41 | FUN_009a8908 | pending |
| 009a89af | 137 | FUN_009a89af | pending |
| 009a8a38 | 28 | FUN_009a8a38 | pending |
| 009a8ba4 | 48 | FUN_009a8ba4 | pending |
| 009a8bd4 | 74 | FUN_009a8bd4 | pending |
| 009a8c1e | 74 | FUN_009a8c1e | pending |
| 009a8c68 | 261 | FUN_009a8c68 | pending |
| 009a8d6d | 425 | GZOnKeyDown | pending |
| 009a8f16 | 100 | IsWindowACellWindow [slot 117] | pending |
| 009a8f7a | 68 | DoCellWindowMove | pending |
| 009a8fbe | 23 | FUN_009a8fbe | pending |
| 009a8fd5 | 23 | FUN_009a8fd5 | pending |
| 009a90f8 | 42 | FUN_009a90f8 | pending |
| 009a93a1 | 134 | FUN_009a93a1 | pending |
| 009a963e | 39 | FUN_009a963e | pending |
| 009a9665 | 39 | FUN_009a9665 | pending |
| 009a968c | 86 | FUN_009a968c | pending |
| 009a96e2 | 27 | FUN_009a96e2 | pending |
| 009a9765 | 105 | SetSelection [slot 43] | pending |
| 009a9837 | 91 | DoWindowRemoval | pending |
| 009a9892 | 267 | FindCellText [slot 133] | pending |
| 009a999d | 35 | FUN_009a999d | pending |
| 009a9a09 | 67 | FUN_009a9a09 | pending |
| 009a9a4c | 61 | FUN_009a9a4c | pending |
| 009a9d2e | 78 | FUN_009a9d2e | pending |
| 009a9d7c | 25 | FUN_009a9d7c | pending |
| 009a9d95 | 196 | FUN_009a9d95 | pending |
| 009a9e59 | 138 | FUN_009a9e59 | pending |
| 009a9ee3 | 135 | FUN_009a9ee3 | pending |
| 009a9f6a | 1143 | GZOnMouseDownL | pending |
| 009aa3e1 | 150 | ActLikeAListbox | pending |
| 009aa477 | 293 | AddCellToSelection | pending |
| 009aa813 | 151 | GetCellTextValue [slot 111] | pending |
| 009aa91a | 119 | FUN_009aa91a | pending |
| 009aa991 | 320 | PlaceAllCellWindows | pending |
| 009aaad1 | 48 | GetCellExtraData [slot 120] | 5.5 |
| 009aab01 | 197 | DisposeOfCellData | pending |
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
| 009ab46a | 275 | CopyCellData | pending |
| 009ab57d | 68 | FUN_009ab57d | pending |
| 009ab5c1 | 41 | FUN_009ab5c1 | pending |
| 009ab613 | 88 | FUN_009ab613 | pending |
| 009ab66b | 73 | FUN_009ab66b | pending |
| 009ab7e6 | 37 | FUN_009ab7e6 | pending |
| 009ab80b | 23 | FUN_009ab80b | pending |
| 009ab822 | 23 | FUN_009ab822 | pending |
| 009ab897 | 103 | ClearCell | pending |
| 009ab9ab | 26 | FUN_009ab9ab | pending |
| 009ab9c5 | 26 | FUN_009ab9c5 | pending |
| 009abaef | 116 | DisposeOfAllCellData | pending |
| 009abc95 | 567 | cGZWinGrid | pending |
| 009abecc | 8 | __ZThn4_N10cGZWinGridD0Ev | 5.7 |
| 009abed4 | 175 | Shutdown | pending |
| 009abf83 | 172 | FUN_009abf83 | pending |
| 009ac24d | 28 | ~cGZWinGrid | pending |
| 009ac43b | 111 | SetColumnWidth [slot 65] | pending |
| 009ac4aa | 111 | SetRowHeight [slot 67] | pending |
| 009ac519 | 552 | SetCellFormat | pending |
| 009ac741 | 293 | SetCellFont [slot 90] | pending |
| 009ac866 | 250 | SetCellColors [slot 91] | pending |
| 009ac960 | 242 | SetCellWrapping [slot 92] | pending |
| 009aca52 | 260 | SetCellAlignment [slot 93] | pending |
| 009acb56 | 260 | SetCellBorder [slot 94] | pending |
| 009acc5a | 233 | SetSizeCachedActualForRow | pending |
| 009acd43 | 236 | SetSizeCachedActualForColumn | pending |
| 009ace2f | 167 | SetCellExtraData [slot 119] | pending |
| 009aced6 | 786 | SortColumn | pending |
| 009ad1e8 | 810 | SortRow | pending |
| 009ad512 | 370 | CopyCellData | pending |
| 009ad684 | 118 | GetColumnWidthInternal | pending |
| 009ad6fa | 285 | CalculateCellHeight | pending |
| 009ad817 | 282 | ScrollCellsHorizontally | pending |
| 009ad931 | 45 | GetRowHeading [slot 80] | 5.5 |
| 009ad95e | 40 | SetRowHeading [slot 81] | 5.5 |
| 009ad986 | 45 | GetColumnHeading [slot 78] | 5.5 |
| 009ad9b3 | 40 | SetColumnHeading [slot 79] | 5.5 |
| 009ad9db | 407 | CopyColumns | pending |
| 009adb72 | 407 | CopyRows | pending |
| 009add09 | 315 | ResizeColumnToFitCellWindows | pending |
| 009ade44 | 1086 | MakeRoom | pending |
| 009ae282 | 109 | MakeRoomForRow | pending |
| 009ae2ef | 109 | MakeRoomForColumn | pending |
| 009ae35c | 43 | GetColumnWidth | 5.4 |
| 009ae387 | 264 | GetRowHeightInternal | pending |
| 009ae48f | 285 | ScrollCellsVertically | pending |
| 009ae5ac | 307 | DeleteColumns | pending |
| 009ae6df | 308 | DeleteRows | pending |
| 009ae813 | 184 | GetCellRect | pending |
| 009ae8cb | 667 | SetCellTextValue | pending |
| 009aeb66 | 337 | SetCellBitmap [slot 114] | pending |
| 009aecb7 | 428 | SetCellWindow [slot 116] | pending |
| 009aee63 | 659 | RecalculateAll | pending |
| 009af0f6 | 43 | GetRowHeight | 5.4 |
| 009af121 | 71 | Init | pending |
| 009af168 | 2813 | GZPaint | pending |

Specified 53 of 198 functions, 1788 of 32735 bytes; remaining 145 functions, 30947 bytes.
<!-- END INVENTORY -->
