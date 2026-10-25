| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzballoon7: the MySim thought balloon (`cSC4MySimBalloonWinProc`), mysim-balloon.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gztips7) | **ready** (docs only). Layout, start-up, the self-posted tick, placement, the show/hide cycle, picture refresh, sound and painting; owner claimed: specified +2,857 bytes (+10 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 10 functions were read in the decompile; the constructor (`0x00430B60`, a `cSC4MySim` function), `DoMessage`, the
paint and the sound start were checked on the instructions; the vtables `0x00A8576C` and `0x00A8575C` were read.
Written on 2026-10-24.

### What changed

- New docs/specs/mysim-balloon.md.
- tip-of-the-day.md §2: the parent global `0x00B43CC0` is no longer called the main window (the main game window is
  `0x00B43DC8`, advisors.md).

### Quirks found

- The tick message re-posts itself on every delivery; a failed refresh in the hidden state retries on every tick.
- The second balloon getter's advice is accepted without pictures; null pictures are dereferenced by the size loop and
  the paint.
- The window is only resized when a picture does not fit; offsets use unsigned halves.
- An off-screen MySim parks the balloon at (−W, −H).

### Coverage

- tools/coverage-map.tsv: new row `cSC4MySimBalloonWinProc` → mysim-balloon.md, before `cSC4*WinProc`.
- Specified bytes 4,761,474 → 4,764,331 (+2,857, 85.3%, `cov_local.py`).

### Not covered

- The constructor and destructor (owner `cSC4MySim`, mysims.md); the balloon advice's picture and sound slots.
