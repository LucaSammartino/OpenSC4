# Window toolkit odds and ends: the cursor manager, tree-view dragging, the dialog base, script writer helpers (behaviour spec)

Scope: small owners of the GZ window library as tools/coverage.py assigns them: `cRZCursorManager` (§1, 18
functions, 914 bytes), `cGZWinTreeViewDrag` (§2, 7, 544), `cGZWinDlg` (§3, 3, 163), `fn:unscript` and the
`fn:script_tag_*` writers (§4, 12, 999), `fn:GetGZWinFont` (§5, 2, 242) and three constructors (§6:
`cGZWinTreeView::GZWinTreeView_TreeNode`, `cGZWin::ChildList`, `cGZWinTextEdit::UndoRedoAction`; 204 bytes).

Related: widget-winmgr.md (the window manager creates the cursor manager), widget-toolkit.md (`cGZWin` slots, key
equivalents §16, cursors §15), widget-controls.md (the tree view, the text edit), widget-dialogs.md (the message box
and generic dialog, which derive from `cGZWinDlg`), legacy-control.md (the UI script writer that uses §4 and §5).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owners was read in the decompile; the cursor manager's vtable
`0x00AC5290`, the tree-view drag's mouse move, `cGZWinDlg::GZOnKeyDown` and `GetGZWinFont` were checked on the
instructions. Written 2026-10-24 (re-gzwinmisc7).

## 1. The cursor manager (`cRZCursorManager`) *[compat]*

Created by the window manager (widget-winmgr.md: GZCOM `(0x4352FEF3, 0xA34C498A)`, else this class) and kept at its
`+0x164`. Fields: `+0x04` initialised; `+0x08` reference count; a map ID → cursor (counted); a stack (deque) of
saved cursors at `+0x18`; `+0x40` the current cursor (counted). Constructor `0x00892792`; deleting destructor
`0x008921CA`.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x00891E1F` | 1 and `0xA34C498A` → this, with a reference; else false |
| 2 `Release` | `0x00891E08` | the last reference calls slot 16 (delete) |
| 3 `Init` | `0x008927BB` | once: five system cursors (`cRZSystemCursorW95`, 0x30 bytes) of kinds 1, 3, 15, 16 and 17, each added under the ID equal to its kind; kind 1 also becomes current |
| 4 `Shutdown` | `0x008921E6` | when initialised: no current cursor, every cursor released, the map emptied |
| 5 `AddCursorFromResource(res, id)` | `0x00892970` | a new `cRZCursor(res)` (0x30 bytes) through slot 6 |
| 6 `AddCursor(cursor, id)` | `0x0089288B` | an entry under `id` is replaced (the old one released); a null cursor only removes it; true |
| 7 `RemoveCursor(id)` | `0x00892149` | release and erase; true when it existed |
| 8 `GetCurrentCursor` | `0x009DB37A` (shared) | `+0x40`, no reference |
| 9 `GetCursor(id)` | `0x0089217F` | the cursor or null, no reference |
| 10 `SetCursor(cursor)` | `0x00891E9F` | current := cursor, then its slot 5 `(1)` (no null check); true |
| 11 `SetCursorByID(id)` | `0x00891E70` | slot 9; when found, its slot 5 `(1)` and current := it; true when found |
| 12 `PushCursor(id)` | `0x0089299E` | the current cursor is kept; slot 11 `(id)`; when that succeeds the kept cursor is pushed on the stack, else released |
| 13 `PopCursor` | `0x00892205` | with a non-empty stack: its top is popped and becomes current (slot 10); true; else false |
| 14 `ShowCursor` / 15 `HideCursor` | `0x00891DFB` / `0x00891DEE` | Win32 `ShowCursor(1)` until the display count is ≥ 0 / `ShowCursor(0)` until it is < 0 |

Helpers: `set_current_cursor` (`0x00891E44`, counted assignment), `release_all_cursors` (`0x0089219E`).

## 2. The tree-view drag window (`cGZWinTreeViewDrag`) *[compat]*

A modal helper window that carries the picture of a tree-view row while it is dragged. Constructor
`0x009CFD89(tree, node, picture, dx, dy)`: `+0xD8` the tree view, `+0xDC` the node, `+0xE0` the picture, `+0xE4`,
`+0xE8` the grab offset.

- **`Init`** (`0x009CFDCE`): `cGZWin::Init`; the window takes the picture's bounds (picture slot 12) moved to the
  cursor (window manager slot 33, then the parent's `ScreenToWindow`, slot 59) minus the grab offset (slot 54);
  it captures the mouse. Returns the base result.
- **Move** (`0x009D141A`): the point is converted to the parent's coordinates (slot 61), the window moves to it minus
  the grab offset (slot 56) and is shown (slot 69); then its top-left, converted into the tree view's coordinates,
  goes to `cGZWinTreeView::NotifyDrag(node, x, y, 0)`.
- **Left release** (`0x009CFEF0`): ends the modal run with 0. **`Shutdown`** (`0x009CFE60`) releases the capture.
- **`GZPaint`** (`0x009CFE70`): the draw context's slot 5 with the draw area, then slot 38 copies the picture's
  bounds to the same rectangle moved to the draw area's origin (`0x009CFED9` offsets a rectangle).

## 3. The dialog base (`cGZWinDlg`) *[compat]*

Three bodies of the base of the message box and the generic dialog (widget-dialogs.md): the deleting destructor
`0x009CDB7E`, the `QueryInterface` thunk `0x009CDB9A`, and **`GZOnKeyDown(vk, modifiers)`** (`0x009DA676`): the
direct child whose key equivalent matches (`IsChildKeyEquivalent`, slot 82, widget-toolkit.md §16) gets the focus
(window manager slot 37) and the key (`SendMsg(child, 5, vk, modifiers, 0)`, slot 144); the result is that send's;
false when no child matches.

## 4. Script writer helpers (`fn:unscript`, `fn:script_tag_*`) *[compat]*

The UI script writer (legacy-control.md, "value helpers") appends `name=value ` items with `cRZString::Sprintf`; the
name is the tag's text (`+4`). Besides the helpers that spec lists (hex, quad, colour, resource key, text, yes/no):

| Helper | Address | Format |
|---|---|---|
| `script_tag_int` | `0x0095BDDF` | `"%s=%d "` |
| `script_tag_pair` | `0x0095BECF` | `"%s=(%d,%d) "` |
| `script_tag_symbol` | `0x0095BF79` | `"%s=%s "` (the second name's text) |
| `script_tag_textkey` | `0x0095BD57` | `"%s=(%u,%u,%u)(%u,%u,%u)(%u,%u,%u)(%u,%u,%u) "` (four colours) |
| (in `unscript`) | `0x0095BCED` | `"%s=(%u,%u,%u)(%u,%u,%u) "` (two colours) |
| `script_tag_color` / `_quad` / `_imagekey` / `_hex` | `0x0095BC92` / `0x0095BF21` / `0x0095BE7D` / `0x0095BE2E` | legacy-control.md |

The other `unscript` bodies: `0x0095BC74` compares two tag records (equal when both `+4` and `+8` match);
`0x009A935D` and `0x009A903E` begin and advance an iterator over a bucketed table of tags, skipping empty buckets.

## 5. `GetGZWinFont(window, id, out)` (`0x0095BB82`) *[compat]*

Writes a window's font parameter into the script: the window's parameter `id + 0xFAA3BE85` (window slot 113) is read
as a variant; type 6 (a number) is a font ID: the window-control manager's font of that ID (its slot 37) gives `"%s"`
of the font's name (font slot 5), or `"%d"` of the ID when there is none; type `0x800E` (text) is copied out as it is
(variant slot 55). True for those two types, false otherwise or without the parameter. The second form
(`0x0095BC21`) takes a font ID directly and writes the same `"%s"` name or `"%d"` ID.

## 6. Constructors *[compat]*

- `cGZWinTreeView::GZWinTreeView_TreeNode` (`0x009D2B42`): `+0` = 0, `+4` = −3, an empty string at `+8`, everything
  else (to `+0x50`) zero.
- `cGZWin::ChildList` (`0x0099D922`): an empty list and a zero count (`+4`) (widget-toolkit.md §4.1).
- `cGZWinTextEdit::UndoRedoAction` (`0x009BE9CE`): a byte and four dwords zero, an empty string at `+0x14`.

## 7. Quirks

- `SetCursor(null)` dereferences null (slot 10 calls the cursor's slot 5 without a check).
- `HideCursor` / `ShowCursor` loop on the Win32 display counter, so they always end hidden / shown whatever other
  code did before.
- `PushCursor` with an unknown ID leaves the stack untouched.

## 8. Function inventory

§1: `0x00891DEE`, `0x00891DFB`, `0x00891E08`, `0x00891E1F`, `0x00891E44`, `0x00891E70`, `0x00891E9F`, `0x00892149`,
`0x0089217F`, `0x0089219E`, `0x008921CA`, `0x008921E6`, `0x00892205`, `0x00892792`, `0x008927BB`, `0x0089288B`,
`0x00892970`, `0x0089299E`. §2: `0x009CFD89`, `0x009CFDCE`, `0x009CFE60`, `0x009CFE70`, `0x009CFED9`, `0x009CFEF0`,
`0x009D141A`. §3: `0x009CDB7E`, `0x009CDB9A`, `0x009DA676`. §4: `0x0095BC74`, `0x0095BCED`, `0x009A903E`,
`0x009A935D`, `0x0095BD57`, `0x0095BDDF`, `0x0095BECF`, `0x0095BF79`, `0x0095BC92`, `0x0095BF21`, `0x0095BE7D`,
`0x0095BE2E`. §5: `0x0095BB82`, `0x0095BC21`. §6: `0x009D2B42`, `0x0099D922`, `0x009BE9CE`.
