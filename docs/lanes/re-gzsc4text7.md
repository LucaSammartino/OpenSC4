| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzsc4text7: the game's text label (`cSC4WinText`), widget-text.md §7 (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzadvface7) | **ready** (docs only). The subclass's paint: buffer fill, single-line cut with an ellipsis, word-wrapped lines with an ellipsis on the last, outline; owner claimed: specified +1,266 bytes (+2 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Both functions were read in the decompile and `GZPaint` (`0x007BE7A0`, 1,236 bytes) was followed on the instructions,
including its alignment tables `0x007BEC74`/`0x007BEC80` and the rectangle helper `0x00798090`. Written on 2026-10-24.

### What changed

- widget-text.md §7: `cSC4WinText`.

### Quirks found

- A window lower than one line draws no text.
- A word wider than the line ends the text and moves the ellipsis to the line before.

### Coverage

- tools/coverage-map.tsv: new row `cSC4WinText` → widget-text.md, before `cSC4Win*`.
- Specified bytes 4,768,623 → 4,769,889 (+1,266, 85.4%, `cov_local.py`).

### Not covered

- The font slots 35, 41, 42, 48, 50 and 52 (cited by number).
