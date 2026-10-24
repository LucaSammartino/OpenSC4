| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gztips7: the tip of the day dialog (`cSC4TipOfTheDayWinProc`), tip-of-the-day.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzbars7) | **ready** (docs only). Opening, the tip scan, commands, showing a tip and closing; owner claimed: specified +1,982 bytes (+12 functions) | TOD1 (reader of the check box preference) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 12 functions were read in the decompile; `DoModelessWin`, `EndDialog`, `GoToNextTip`, `InitDialog`, the step and
the exemplar enumeration were checked on the instructions, and the opener in the 3D view (`0x007ED66F`–`0x007ED683`)
was read. Written on 2026-10-24.

### What changed

- New docs/specs/tip-of-the-day.md and docs/specs/questions/tip-of-the-day.md (TOD1); QUESTIONS.md regenerated.

### Quirks found

- A last-shown tip that no longer exists makes the dialog start at the second tip.
- `EntriesPresent` scans once per run; the presence scan still visits every exemplar key.
- The check box preference is written but not read by the opener (TOD1).

### Coverage

- tools/coverage-map.tsv: new row `cSC4TipOfTheDayWinProc` → tip-of-the-day.md, before `cSC4*WinProc`.
- Specified bytes 4,759,492 → 4,761,474 (+1,982, 85.2% → 85.3%, `cov_local.py`).

### Not covered

- The dialog's script layout (`0xCA3BDD94`) and the tip exemplars' other properties.
