| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-uihelpers7: the UI helper functions `nSC4UI`, new spec ui-helpers.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-cheatdlg7) | **ready** (docs only). ui-helpers.md §1–§8: the property readers, the exemplar token formatter, the catalog item, list and view builders, window alignment (marker `0xAAAA` captions), child sets and the focus flag, the three generic modal dialogs and their window procedure, the set helpers, quirks, five conformance cases; questions UIH1–UIH3; owner claimed: `nSC4UI`: specified +6,802 bytes (+25 functions) | UIH1–UIH3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 25 claimed functions were disassembled; the variant type jump table (`0x0078D950`) was read from the data
section. Written on 2026-10-23. Other specs only cited these helpers by name (ui-windows.md, modal-dialogs.md,
neighbors.md, advisor-window.md); minimap.md §4.2 already specifies `InterpolateRampToPalette`. No upstream lane
covers this owner.

### What changed

- New docs/specs/ui-helpers.md.
- Read on the bytes:
  - the variant type codes and the three property readers;
  - the `d:`/`m:` token formats (`%.2f`, the language utility's integer format, the two money formatters);
  - the catalog item fields;
  - the item list filter (group, menu value, required property) and its ordering (property `0x8A2602B9`, default
    1000, stable);
  - both `AlignWindowToWindow` forms, including the `-XY` caption code (`tmb` × `lcr`) and the hexadecimal target
    caption;
  - `GetWindowChildrenSet`, `AlignWindowSetToWindow` and `ClearAcceptFocusFlagRecursive`;
  - the shared dialog sequence (popup gate, pause, placement at a third of the height, window procedure, location
    store, focus restore) and the IDs of the three dialogs.
- docs/specs/questions/ui-helpers.md: UIH1–UIH3 (new prefix UIH); QUESTIONS.md regenerated.

### Quirks found

- No bound check for non-negative array indexes in the integer reader.
- `AlignWindowToWindow` uses the found window without a null check.
- The `-XY` alignment ignores the main window's own origin.
- A nameless exemplar leaves the token callback registered.

### Coverage

- tools/coverage-map.tsv: `nSC4UI` → ui-helpers.md, before the `nSC4UI*` row.
- Specified bytes 4,522,560 → 4,529,362 (+6,802, 81.0% → 81.1%, `cov_local.py`, on `5c0ff782` + the thirteen lanes
  before).

### Not covered

- `cSC4WinCatalogView` and `cSC4CatalogItem` (their own owners); the detokenizer and the money formatters.
- UIH1–UIH3.
