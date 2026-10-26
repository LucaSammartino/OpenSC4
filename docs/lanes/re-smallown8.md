| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-smallown8: `nsTexUtil`, `SC43DGridElement`, `cSC4LimitTimerCalibrator`, `LimitTimer`, plus rows for `tControllerTime` and `cGZWinCustom`, small-owners-b9.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-luaext8) | **ready** (docs only). Six owners claimed: specified +1,750 bytes (+19 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The uncited bodies were read in the disassembly; the cited ones (`tControllerTime` in automata.md §3.1, `LimitTimer`
in simulator.md §5.5 and pathfinder.md, `cGZWinCustom` in widget-custom.md) were checked against it. Written on
2026-10-25.

### What changed

- docs/specs/small-owners-b9.md (new): §1 `nsTexUtil` (power of two, texture release, `SetupTexture`'s direct and
  resize paths); §2 the `SC43DGridElement` record, constructor and pool destructors; §3 the calibrator's `Tick`;
  §4 `LimitTimer`; §5 conformance tests.

### Coverage

- tools/coverage-map.tsv: rows for `nsTexUtil`, `SC43DGridElement`, `LimitTimer` and `tControllerTime` filled;
  new rows `cSC4LimitTimerCalibrator` (before `cSC4*`) and `cGZWinCustom` (before `cGZWin*`).
- Specified bytes 5,130,906 → 5,132,656 (+1,750, 91.9%, `cov_local.py` with owners-c1).
