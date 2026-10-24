| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-catview7: the catalog list `cSC4WinCatalogView` and `cSC4CatalogItem`, new spec catalog-view.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-uihelpers7) | **ready** (docs only). catalog-view.md §1–§6: fields of the view and the item, the 23-slot view interface (scroll, selection, flash, callout), the window overrides (`Init`, `Shutdown`, `DoMessage`, the four-frame `GZPaint`), the mouse (move, click command `0x49D993D4`, wheel) and the highlight callout, quirks, five conformance cases; questions CV1–CV2; TUW6 answered; owners claimed: `cSC4WinCatalogView`, `cSC4CatalogItem`: specified +3,797 bytes (+42 functions) | CV1–CV2 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 42 claimed functions were disassembled; both vtables and the item's getters (shared bodies) were read from the
data section. Written on 2026-10-23. ui-windows.md only listed the class; no upstream lane covers these owners.

### What changed

- New docs/specs/catalog-view.md.
- Read on the bytes:
  - the layout (item vector, scroll, highlight, selection, cell layout, callout, flash timer);
  - every interface slot, including the one-column geometry and the callout point (`w + 16`, row middle);
  - `SetScrollPos` with the parent's arrow update;
  - the icon strip of four frames (disabled, normal, selected, highlighted or flashing);
  - the click command and its sounds, and the wheel step (`delta / 300`);
  - the callout text (`name - money(cost)`).
- tutorial-window.md: TUW6 answered (interface `0x49D98264` is this view; slot 18 is "every item except"); §5.1, §5.3
  and §8 updated; the entry moved to questions/answered/tutorial-window.md.
- docs/specs/questions/catalog-view.md: CV1–CV2 (new prefix CV); QUESTIONS.md regenerated.

### Quirks found

- An item without an icon is painted out of row.
- Slot 18 changes every item except the named one.
- Removing all items keeps the indexes.
- The flash tick does not repaint.
- The row test truncates toward zero above the first row.

### Coverage

- tools/coverage-map.tsv: `cSC4WinCatalogView` → catalog-view.md, before the `cSC4Win*` catch-all; the
  `cSC4CatalogItem` row now cites catalog-view.md.
- Specified bytes 4,529,362 → 4,533,159 (+3,797, 81.1% → 81.2%, `cov_local.py`, on `5c0ff782` + the fourteen lanes
  before).

### Not covered

- CV1 (the parent's arrow interface) and CV2 (the buffer lock).
