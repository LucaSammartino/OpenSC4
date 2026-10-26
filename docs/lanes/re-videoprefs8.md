| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-videoprefs8: complete the video preferences window (`cSC4VideoPreferencesWinProc`), video-preferences.md §9, VP1–VP5 answered (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-dispatchview8) | **ready** (docs only). One owner claimed: specified +15,016 bytes (+9 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The three methods upstream's lane re-video-preferences4 left Partial were read instruction by instruction in the
disassembly (SetButtonsBasedOnPreferences `0079020d`–`00791201`, every HandleControlMessage branch and both jump
tables, ShowDialog `00791677`–`00791d1d`), plus the constructor `00791360` and its two callers. The captions come from
the shipped UI script (SimCity_1.dat, type 0, group `96A006B0`, instance `8A7E052F`, read-only with tools/dbpf.py).
Written on 2026-10-25.

### What changed

- docs/specs/video-preferences.md: new §9 (construction and callers; the whole redraw with every group, value test
  and baseline capture; every remaining event ID; the 35 notification-target bindings; the App and window-manager
  boundaries; the captions; conformance tests); the §8 ledger rows are now Supported.
- VP1–VP5 answered: moved to the new docs/specs/questions/answered/video-preferences.md; QUESTIONS.md regenerated.

### Coverage

- tools/coverage-map.tsv: new row `cSC4VideoPreferencesWinProc` → video-preferences.md, above `cSC4*WinProc`.
- Specified bytes 5,001,731 → 5,016,747 (+15,016, 89.6% → 89.8%, `cov_local.py` with owners-c1).

### Why now

Earlier batches skipped this owner as an active upstream lane (re-video-preferences4, finished 2026-10-23 with VP1–VP5
open). Upstream main `626043df` has STOP set and no lane on it.
