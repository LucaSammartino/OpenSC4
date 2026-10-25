# Window utilities, graph decorations, the query legend and query dialog remainder (behaviour spec)

Scope: the owners `nsGZWinUtils` (13 functions, 682 bytes), `MiscGraphTextInfo` (6, 516), `MiscGraphPicture` (5, 283),
`MiscGraphArrow` (2, 215), `MiscGraphRect` (2, 142), `cSC4LegendDialogWinProc` (3, 440), `cSC4QueryDialogWinProc`
(6, 842; the dialog's window procedure itself is city-tools-input.md's) and `fn:SC4DoPopupStringListMenu` (3, 75), as
tools/coverage.py assigns them.

Related: widget-toolkit.md (window slots: 11 parent, 32 `EnumChildren`, 35 `GetChildWindowFromIDRecursive`, 36/37
`GetChildAs` / `GetChildAsRecursive`, 67/68 `GetFlag` / `SetFlag`, 74 caption, 86 notification target, 92
invalidate), widget-graphs.md §4 (the graph's decoration list and its line object), city-tools-input.md (the query
tool, its dialog and route display), win-grid.md (`cGZWinGrid`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the button helpers, the arrow drawing, the
legend dispatch and `EndDialog` were read in the disassembly. Written 2026-10-24 (re-widgetmisc7b).

## 1. Window utilities (`nsGZWinUtils`) *[compat]*

All take a parent window and a child ID; "the child" is `GetChildWindowFromIDRecursive(id)` (or, for the typed
helpers, `GetChildAsRecursive(id, iid)`, released afterwards). They return false when there is no such child.

| Function (address) | Behaviour |
|---|---|
| `FindSiblingAs(win, id, iid, out)` (`0x009AFCDE`) | the parent's `GetChildAs(id, iid, out)` |
| `SetWindowVisible(parent, id, visible, invalidate)` (`0x009AFCFE`) | when the visible flag (1) differs: `SetFlag(1, visible)`, then invalidate when asked; true |
| `GetWindowVisible(parent, id)` (`0x009AFD4F`) | the visible flag; false without the child |
| `SetWindowEnabled(parent, id, enabled)` (`0x009AFD73`) | `SetFlag(2, enabled)`; true |
| `SetWindowCaption(parent, id, text)` (`0x009AFD9D`) | slot 74; true |
| `SetWindowNotificationTarget(parent, id, target)` (`0x009AFDC5`) | slot 86; true |
| `SetWindowNotificationTargetRecursive(win, target)` (`0x009AFE1C`) and `(parent, id, target)` (`0x009AFE37`) | `EnumChildren(0x22BA0121, cb, target)` on the window (or the child) with the callback `0x009AFDED`, which sets each child's target and recurses into it; the starting window itself is not changed *[quirk]* |
| `SetButtonToggleState(parent, id, on)` (`0x009AFE69`) | the child as a button (IID `0x8810`): toggle on (button slot 6) or off (slot 7); true |
| `GetButtonToggleState(parent, id)` (`0x009AFEBC`) | button slot 8 |
| `ExcludeButtonByRange(parent, first, count, selected)` (`0x009AFF08`) | for each ID in `[first, first + count)`: `SetButtonToggleState(parent, id, id == selected)` (radio behaviour) |
| `SetEditText(parent, id, text)` (`0x009AFF38`) | the child as a text edit (IID `0x231A1C57`), its slot 48 `(text, 0)`; true |

## 2. Graph decoration objects *[compat]*

The graph (widget-graphs.md §4) owns these and calls `DrawSelf(graph, context)`; each drawing step locks the context
for its kind and unlocks it (`0x8001` fills, `0x8004` lines, `0x8008` outlines, `0x8010` blits).

- **Text info** (`MiscGraphTextInfo`, 72 bytes; constructor `0x009B810F`): text string `+8`, rectangle `+0x1C`,
  alignment `+0x2C`, outline flag `+0x30` and colour `+0x34`, fill flag `+0x38` and colour `+0x3C` (−1), font `+0x40`,
  text colour `+0x44` (`0x80000000` = unset). `Init` (`0x009B8169`): without a font, the font system's 12-point
  `arial` (font system slot 11), else `helvetica`; true when a font exists. `DrawSelf` (`0x009B4040`): the fill
  (context slot 18), the 1-pixel outline (`DrawRectangleOutline`), then the font's text draw (font slot 52) with the
  text colour, or `0x08000000` when unset. `SetFont(font, colour)` (`0x009B2713`) replaces the font (counted); the
  colour is stored only when one was already set *[quirk: an unset colour cannot be set this way]*. Destructors
  `0x009B8140`, `0x009B966A`.
- **Rectangle** (`MiscGraphRect`; constructor `0x009B40EB`): rectangle `+8`, outline flag `+0x18` (1), width `+0x1C`
  (1), outline colour `+0x20`, fill flag `+0x24` (0), fill colour `+0x28` (−1). `DrawSelf` (`0x009B274F`): fill, then
  outline.
- **Arrow** (`MiscGraphArrow`; constructor `0x009B410B`): the line `+8`…`+0x14`, colour `+0x18`, width `+0x1C` (1),
  head flags `+0x20` (1) and `+0x21` (0), head size `+0x24` (8), head angle `+0x28` (π/4). `DrawSelf` (`0x009B27BD`)
  draws the line (graph slot 43). With a head flag set it makes a zero-length line one pixel long (the stored end x
  is incremented) and computes the length and angle, but **draws no head** *[quirk]*.
- **Picture** (`MiscGraphPicture`; constructor `0x009B286A`): image `+8` (counted; `SetImage` `0x009B28A2`), position
  `+0xC`, `+0x10`, "relative to the graph" flag `+0x14`. `DrawSelf` (`0x009B4151`): the image's area placed at the
  position (moved by the graph's area origin when relative) and blitted (graph slot 46). Destructors `0x009B2881`,
  `0x009B4135`.

## 3. The query tool's route legend (`cSC4LegendDialogWinProc`) *[compat]*

`DoWinProcMessage(win, msg)` (`0x004CB900`, always false):

- `0xA2BF8AD5` from the window with ID `0x0BB0F5E7`: button `0xABB0F65F` on, `0xEBB0F666` off.
- A button command (`3`, `0x287259F6`, ID): IDs `1`, `0x2BB0F616`, `0x2BB9131F`, `0x4BB0F628`, `0x4BB91326`,
  `0x4C081783`, `0x6BB91318`, `0xABB0F639`, `0xEBB0F630` refresh the route display (with the tool's path view:
  `UpdatePaths`, then `ShowAllRoutesThatIntersectCityRect` with the tool's city rectangle, `0x004CAC30`); `0xABB0F65F`
  and `0xEBB0F666` are a radio pair (the pressed one on, the other off), then the refresh; `0x4BB92C1F` calls slots 69
  and 70 of the tool's two views when both exist; `0xEBB91356` minimizes the legend (`SetLegendMinimized(1)`).

Deleting destructor `0x004C4570`.

## 4. Query dialog remainder (`cSC4QueryDialogWinProc`) *[compat]*

- `SetTarget(obj)` (`0x004C4530`): releases the held object (`+0x14`), stores and AddRefs `obj`; true when non-null.
- `EndDialog(result)` (`0x004C5690`): the window manager's slot 9 with `(15, dialog.slot3(result))`, then the dialog
  (`+0x10`) is released **twice** (once directly, once when the field is cleared) *[quirk]*, and the target released.
- Deleting destructor `0x004C68E0`; `0x004C6E00` forwards to the query tool's `SetStaticControlSimoleons` with the
  control ID plus `0x100`. The window procedure `0x004D3F40` and the attach helper `0x004C5620` are city-tools-input.md's.

## 5. Folded window bodies (`fn:SC4DoPopupStringListMenu` owner) *[compat]*

`0x009A6284` sets the bits of a mask in the window flags at `+0xDC` (through slot 11) when none was set;
`0x009A629F` clears them when any was set; `0x009AFCCB` recalculates a `cGZWinGrid` and sets its "dirty" byte `+0xE4`.

## 6. Function inventory

`nsGZWinUtils`: `0x009AFCDE`, `0x009AFCFE`, `0x009AFD4F`, `0x009AFD73`, `0x009AFD9D`, `0x009AFDC5`, `0x009AFDED`,
`0x009AFE1C`, `0x009AFE37`, `0x009AFE69`, `0x009AFEBC`, `0x009AFF08`, `0x009AFF38`.
`MiscGraphTextInfo`: `0x009B2713`, `0x009B4040`, `0x009B810F`, `0x009B8140`, `0x009B8169`, `0x009B966A`.
`MiscGraphPicture`: `0x009B286A`, `0x009B2881`, `0x009B28A2`, `0x009B4135`, `0x009B4151`.
`MiscGraphArrow`: `0x009B27BD`, `0x009B410B`. `MiscGraphRect`: `0x009B274F`, `0x009B40EB`.
`cSC4LegendDialogWinProc`: `0x004C4570`, `0x004CAC30`, `0x004CB900`.
`cSC4QueryDialogWinProc`: `0x004C4530`, `0x004C5620`, `0x004C5690`, `0x004C68E0`, `0x004C6E00`, `0x004D3F40`.
`fn:SC4DoPopupStringListMenu`: `0x009A6284`, `0x009A629F`, `0x009AFCCB`.
