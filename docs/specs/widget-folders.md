# `cGZWinFolders` and `cGZWinFolderRow`: the folder tabs (behaviour spec)

Scope: the owners `cGZWinFolders` (9 functions, 637 bytes) and `cGZWinFolderRow` (9 functions, 1,029 bytes) as
tools/coverage.py assigns them on `bb641878`, **and** their 28 interface functions that the tool attributes to
`cGZWinCOMDirector` (listed in §4). Together they are the toolkit's multi-row folder tabs: a list of named folders
laid out as tabs on as many rows as needed, the selected row brought to the front. UI scripts create them as
`GZWinFolders` (class `0x078E8055`, legacy-control.md §9.6).

Related: widget-toolkit.md (the `cGZWin` base, `SendMsg`); widget-ctrlmgr.md (system font, picture `0xE8963EDF`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the vtables `0x00AE3FE4` (folders interface)
and `0x00AE3AF4` (row interface, followed by the row window's vtable) were read and the constant `0x00AE3A80` (4) was
checked. Written 2026-10-24 (re-gzfolders7).

Conventions: both classes start with their interface at `+0` and the window at `+4`; offsets are from the object
start (a `cGZWin` field `+n` is at `+4 + n`). "Slot n" is a slot of the class's interface. `cap(H) = max(H >> 3, 4)`
for a tab image of height `H`; a tab for name `s` is `textWidth(s) + 10 + 2·cap` wide (font slot 48).

## 1. Folder tabs (`cGZWinFolders`, IID `0xA13351B8`)

**Object** (constructor `0x009D77CA`, `0xFC` bytes): `+0xDC` folder list (records: name, …, `+0x2C` ID),
`+0xE0` row list (rows, counted), `+0xE4` font, `+0xE8` tab image (`+0xEC` half its width, `+0xF0` its height),
`+0xF4` "layout needed" (byte, initially 1), `+0xF8` row count; the window area starts as `(16, 16, 32, 32)`.
**`QueryInterface`** (`0x009D6CD4`): `0xA13351B8` → the object, AddRef'd; else `cGZWin`'s. Thunks `0x009D7848`
(window → object) and `0x009D7850` (deleting destructor); destructor `0x009D7BB9` (body `0x009D7BD5`).

| Slot | Address | Behaviour |
|---|---|---|
| 3 | `0x009CFA36` (shared) | the window (`+4`) |
| 4 `AddFolder(name, id)` | `0x009D7D14` | append; layout needed; true |
| 5 `SelectFolderByID(id)` | `0x009D762C` | layout when needed; each row's slot 11; the row that has it moves to the front of the row list and the search stops (true); rows without it get slot 7 (deselect) |
| 6 `SelectFolderByName(name)` | `0x009D75BB` | the same with row slot 12 |
| 7 | `0x009D76FB` (library) | the folder count |
| 8 / 9 `GetSelectedName(out)` / `GetSelectedID(out)` | `0x009D725D` / `0x009D7278` | the front row's slot 9 / 10; false without rows |
| 10 `SetFont(font)` | `0x009D6DD1` | if different: hold it, layout needed, redraw |
| 11 `SetImage(image)` | `0x009D6E18` | if different: hold it, half width and height (0 for null), layout needed, redraw |
| 12 `Layout` | `0x009D7858` | §1.1 |
| 13 `GetFolderCount(out)` | `0x009D74BD` | |
| 14 `GetFolderName(i, out)` | `0x009D7293` | false when `i` is past the end |

### 1.1 `Layout` (slot 12)

Only when layout is needed and the window is initialised (else true / false):
1. Remove every row (its window slot 5, then this window's `ChildDelete`); row count 0.
2. Widths: for each folder its tab width; `max` and `total`. `max > W` → **false**.
3. `n = max(ceil(total / W), 1)` rows (`0x009F0A20`). Each new row (`0xFC` bytes, `0x009D774A`): AddRef, its font
   (row slot 6), the tab image (`0x009D6C36`), `SetArea(0, 0, W, image height)`, added as a child, z-movable,
   appended to the row list, counted.
4. Each folder goes to the current row with row slot 4 (`AddTab`); a row that refuses passes to the next, and when
   all refuse a new row is made the same way.
5. Every row's slot 5 (`Justify`); window slot 52; redraw; layout done; true.

### 1.2 Window behaviour

- **`Init`** (`0x009D6CF6`): `cGZWin::Init`; no font → slot 10 with the system font `0x68963C4C` (with the font
  service and the control manager); no image → picture `0xE8963EDF` (AddRef, half width, height); layout when needed.
  True.
- **`Shutdown`** (`0x009D6D9C`): release the image and the font; `cGZWin::Shutdown`.
- **`GZPaint`** (`0x009D7459`): layout when needed; walking the row list **from its end**, row `i` is moved to
  `(0, (image height − 4)·i)` and painted (its window slot 88). So the front row (the selected one) is the lowest.
- **Left down** (`0x009D74F5`): when `y < (H − 4)·rows + 4`: row `r = rows − y/(H − 4) − 1` (at least 0) counted
  from the front; every other row is deselected; row `r` moves to the front; `SendMsg(notification target, 3,
  0xE87111BE, GetID(), 0)` (folder changed); redraw. Returns false.

## 2. Folder rows (`cGZWinFolderRow`, IID `0xA13351A3`)

**Object** (constructor `0x009D774A`): `+0xDC` tab image (`+0xE0` half width, `+0xE4` height), `+0xE8` tab list
(0x30-byte records: `+0x00` name, `+0x14..+0x20` rectangle, `+0x2C` ID), `+0xF0` font, `+0xF4` selected index (−1),
`+0xF8` tab count, `+0xEC` = 5. **`QueryInterface`** (`0x009D6BDF`): `0xA13351A3` → the object; thunks `0x009D77BA`,
`0x009D77C2`; destructor `0x009D7B87`.

| Slot | Address | Behaviour |
|---|---|---|
| 3 `CanFit(name)` | `0x009D736D` | true when the name is narrower than the row and there are no tabs yet; otherwise whether `total + width(name)` fits in `W` |
| 4 `AddTab(name, id)` | `0x009D7C19` | needs a font; when it fits, append `{name, rect = (total, 0, total + width, image height), id}`, count it, true; else false |
| 5 `Justify` | `0x009D73F8` | while the tabs are narrower than `W`, widen them one pixel at a time from the first, shifting the later ones |
| 6 `SetFont(font)` | `0x009D6C9F` | |
| 7 `Deselect` | `0x009D77B2` | selected := −1 |
| 8 `GetSelected` | `0x00994E85` (shared) | |
| 9 / 10 `GetSelectedName(out)` / `GetSelectedID(out)` | `0x009D7203` / `0x009D7232` | false without a selection |
| 11 / 12 `SelectByID(id)` / `SelectByName(name)` | `0x009D71B1` / `0x009D716F` | the first match becomes selected, true; none → false (the name test is the string's slot 10 with case-insensitivity flag 1) |

Also `0x009D6C36` (`SetImage`, called directly by the folders), `0x009D71E4` (the summed tab widths), `0x009D72C2` (the
empty list), `0x009D76CE`, `0x009D7729`, `0x009D7B62`, `0x009D7BFF` (tab record copy and append).

- **`Init`** (`0x009D6E93`): `cGZWin::Init`; no font → the font service's slot 9 `(9, 15, 0, 0, 1)`; no image → picture
  `0xE8963EDF` (half width, height). True. **`Shutdown`** (`0x009D6C01`): release both, `cGZWin::Shutdown`.
- **Left down** (`0x009D6F24`): the tab whose `l ≤ x ≤ r` becomes selected; then the point is converted to the parent's
  coordinates (window slot 61) and `SendMsg(parent, 7, x, y, flags)` passes the click on (to the folders, §1.2).
- **`GZPaint`** (`0x009D72DD`): every tab except the selected one with `DrawTab(tab, 0, first, last)`, then the
  selected one with `DrawTab(tab, 1, …)`, so it is on top.
- **`DrawTab(tab, selected, first, last)`** (`0x009D6F8A`): the tab is drawn from the image in three parts — left cap
  (draw context slot 38), the middle stretched (slot 39) and the right cap — using the unselected or the selected half of
  the image; a selected tab overlaps its neighbours by 2 pixels on each side that is not an end. Then the name, centred
  vertically, with the row font (font slot 52, horizontally centred).

## 3. Quirks and cases

Quirks: a folder wider than the window makes the whole layout fail; the folders' mouse handler returns false.

Cases:
1. Folders "A", "BB", "CCC" with widths 40, 50, 60 in a 120-wide window: total 150 → 2 rows; row 0 takes 40 + 50, "CCC"
   does not fit and goes to row 1; both rows are justified to 120.
2. A click on the top row of two: it becomes the front row, the other row is deselected, and `0xE87111BE` is sent.

## 4. Function inventory

Owners: `cGZWinFolders` (9): `0x009D6CD4`, `0x009D6CF6`, `0x009D6D9C`, `0x009D7459`, `0x009D74F5`, `0x009D7848`,
`0x009D7850`, `0x009D7BB9`, `0x009D7BD5`. `cGZWinFolderRow` (9): `0x009D6BDF`, `0x009D6C01`, `0x009D6E93`, `0x009D6F24`,
`0x009D6F8A`, `0x009D72DD`, `0x009D77BA`, `0x009D77C2`, `0x009D7B87`.

Attributed to `cGZWinCOMDirector` (28): `0x009D6C36`, `0x009D6C9F`, `0x009D6DD1`, `0x009D6E18`, `0x009D716F`, `0x009D71B1`,
`0x009D71E4`, `0x009D7203`, `0x009D7232`, `0x009D725D`, `0x009D7278`, `0x009D7293`, `0x009D72C2`, `0x009D736D`, `0x009D73F8`,
`0x009D74BD`, `0x009D75BB`, `0x009D762C`, `0x009D76CE`, `0x009D7729`, `0x009D774A`, `0x009D77B2`, `0x009D77CA`,
`0x009D7858`, `0x009D7B62`, `0x009D7BFF`, `0x009D7C19`, `0x009D7D14`.

Not covered: the font slots and the tab image layout beyond the two halves.
