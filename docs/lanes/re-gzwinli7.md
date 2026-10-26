| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwinli7: the single-line text field `cGZWinLineInput`, new spec widget-lineinput.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinsb7) | **ready** (docs only). widget-lineinput.md §1–§9: identity, fields and defaults, the 38-slot `cIGZWinLineInput` table, size, `SetText` with its guard, the changed/completed notifications, characters, keys, selection, scrolling, mouse and focus, sounds, `Init`/`Shutdown`, painting with caret, highlight and outlines, quirks, five cases; owner claimed: `cGZWinLineInput`: specified +5,854 bytes (+58 functions) | WLI1, WLI2 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 58 functions were read in the decompile. The key dispatch, the Backspace path and `DeleteChar`, the colour
accessors, the x87 alpha tests of `GZPaint`, the outline paths and the non-editable click were checked on the
instructions. The vtables `0x00AE0F18` and `0x00AE0FB0` were read and compared with the `cGZWin` table. Written on
2026-10-23.

### What changed

- New docs/specs/widget-lineinput.md.
- New docs/specs/questions/widget-lineinput.md (WLI1, WLI2); QUESTIONS.md regenerated.

### Quirks found

- Backspace passes a 0-based position to the 1-based `DeleteChar`: it erases one byte too early and never the
  first character.
- The constructor leaves the three sound IDs unwritten.
- A highlight alpha strictly between 0 and 1 draws no highlight.
- The outline type 1 converts its colour twice and takes its width from a leftover push.
- The constructor and a non-editable click use the control manager and the parent without null checks.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinLineInput` → widget-lineinput.md, before the `cGZWin*` catch-all.
- Specified bytes 4,644,527 → 4,650,381 (+5,854, 83.2% → 83.3%, `cov_local.py`).

### Not covered

- The font and sound-system slots, cited by number.
