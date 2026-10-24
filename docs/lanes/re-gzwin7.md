| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwin7: the rest of the base window `cGZWin`, folded into widget-toolkit.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-mysimtools7) | **ready** (docs only). widget-toolkit.md §18: `create_private_buffer`, the 30 accessors and window-manager forwards, the child queries (`GetChildCount`, both `ChildExists`, `GetChildWindowFromCursorPoint`), the container helpers and the resource catalog view's `GZOnMouseExit`; §2 names `+0x0C` and `+0xA0`/`+0xA4`; WT2 answered; owner claimed: `cGZWin`: specified +12,861 bytes (+157 functions) | WT1, WT3, WT5, WT6 stay open |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The 47 owner functions that widget-toolkit.md did not cite by address were disassembled; the other 110 were already
specified there (§1–§17). Written on 2026-10-23.

### What changed

- widget-toolkit.md:
  - new §18;
  - §2 rows `+0x0C` (reference count) and `+0xA0`/`+0xA4` (mouse-down offset, set by `cGZWinGenDlg` and
    `cGZWinMsgBox`);
  - §12 records the whole owner.
- WT2 answered; the entry moved to questions/answered/widget-toolkit.md. QUESTIONS.md regenerated.

### Quirks found

- `SetFillColor(native)` uses the graphics system without a null test.
- `QueryInterface` leaves `out` untouched on a miss.
- `GZOnMouseExit` at `0x0079A1B0` belongs to the resource catalog view.

### Coverage

- tools/coverage-map.tsv: new row `cGZWin` → widget-toolkit.md, before the `cGZWin*` catch-all.
- Specified bytes 4,605,786 → 4,618,647 (+12,861, 82.5% → 82.7%, `cov_local.py`, on `bb641878` + re-batch7 and the
  re-batch8 lanes before).

### Not covered

- The meanings of the draw-context, buffer and graphics-system slots, still cited by number.
