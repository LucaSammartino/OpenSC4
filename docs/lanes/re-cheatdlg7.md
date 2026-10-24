| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-cheatdlg7: the cheat code dialog `cSC4CheatCodeEntryDialog`, new spec cheat-dialog.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-tutwin7) | **ready** (docs only). cheat-dialog.md §1–§7: identity and fields, `Init` and `Shutdown`, `DoMessage` (running a cheat, the command server fallback, the Ctrl/Shift keep-open rule, `EndDialog`), the keys and the history, the sorted cheat list menu, the message filter, the sounds, the name sort, quirks, four conformance cases; questions CCD1–CCD3; owner claimed: `cSC4CheatCodeEntryDialog`: specified +6,403 bytes (+33 functions) | CCD1–CCD3 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 33 claimed functions were disassembled:
- the 16 dialog functions;
- the 17 functions of the `std::sort` instantiation over its name list, which the coverage tool assigns to this
  owner by reach.

The vtables were read from the data section, and the creator (`0x004E0BB0`, `cWinSC4App`) was read to confirm the
modal lifetime. Written on 2026-10-23. No upstream lane covers this owner (ui-windows.md lists only its vtables).

### What changed

- New docs/specs/cheat-dialog.md.
- Read on the bytes:
  - the layout (text edit, sound IDs, history index, the global history vector);
  - `Init` (text edit rectangle, flags `0x17C1`, font, colours, message filter, size 312 × 28);
  - the Return handling (`DoDefaultCheatCodeProcessing`, then the command server, result 2 = failure);
  - the Ctrl/Shift rule that keeps the dialog open;
  - the Up/Down history walk;
  - F1 / right click: the sorted cheat list menu;
  - Esc and Alt+F4;
  - the filter that routes the edit's key-down and right-click messages to the dialog;
  - the sort (introsort, threshold 16, byte-order `std::string <`).
- docs/specs/questions/cheat-dialog.md: CCD1–CCD3 (new prefix CCD); QUESTIONS.md regenerated.

### Quirks found

- The two sound IDs are never set, so the dialog never plays a sound.
- Text that is not a known cheat still enters the history and closes the dialog, unless the command server
  returns 2.
- The history is global to the session.

### Coverage

- tools/coverage-map.tsv: `cSC4CheatCodeEntryDialog` → cheat-dialog.md, before the `cSC4*Dialog` catch-all.
- Specified bytes 4,516,157 → 4,522,560 (+6,403, 80.9% → 81.0%, `cov_local.py`, on `5c0ff782` + the twelve lanes
  before).

### Not covered

- `cGZCheatCodeManager`, `SC4DoPopupStringListMenu` and the base `cGZWinGenDlg`.
- Message type `0x11` and keyboard slot 3 (CCD1, CCD2).
