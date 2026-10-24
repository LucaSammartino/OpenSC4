| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzdialogs7: the message box `cGZWinMsgBox` and the generic dialog `cGZWinGenDlg`, new spec widget-dialogs.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzcomdir7) | **ready** (docs only). widget-dialogs.md §1–§4: the message box (fields, both interfaces, layout and the six button sets, the button factory with its literals and colours, keys, dragging, modal end) and the generic dialog (fields, the 22-slot builder, framing styles, title, dragging, command routing), quirks, two cases; owners claimed: `cGZWinMsgBox`, `cGZWinGenDlg`: specified +6,987 bytes (+57 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 57 functions were read in the decompile; the builder's argument sizes (from each `ret`) and `AddListBox` were
checked on the instructions; the button literals (`0x00ABFF54`, `0x00AE28B4`, `0x00AE28C0`) and the border table
`0x00B17C24` were read. Written on 2026-10-24.

### What changed

- New docs/specs/widget-dialogs.md.

### Quirks found

- The message box's `SetMessage` slots overwrite counted fields without releasing them.
- Enter always ends a message box with the OK ID, whatever its buttons.
- The builder gives each control its own address as ID; `SetBackground(null)` crashes.
- `cGZWinGenDlg::Init` without the control manager returns true without initialising.
- One "function" of the owner (`0x00736559`) is a string pointer, not code.

### Coverage

- tools/coverage-map.tsv: new rows `cGZWinMsgBox`, `cGZWinGenDlg` → widget-dialogs.md, before the `cGZWin*` catch-all.
- Specified bytes 4,691,414 → 4,698,401 (+6,987, 84.0% → 84.1%, `cov_local.py`).

### Not covered

- The `cGZWinDlg` base beyond its constructor and `EndDialog`, and the created controls' own slots.
