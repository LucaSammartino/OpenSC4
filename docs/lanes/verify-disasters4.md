| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-disasters4 (verify the non-exact disasters_shadow: fire 211/263, riot 2/354) | `verify-disasters4` / (engine lane) | No C ABI change. Fix `e4c645b5`: the disaster host's fire-coverage average (§5.4 step 4, §5.6 spread) passed grid slot 8 the bottom-right corner, but simgrid §4.1 says it takes a width and a height. Gate: cargo 3614/0, clippy clean, ctest 62/62 (baseline the same). Runs, all `clean`, `ref_warnings=0`: `vd4-base-1` (main) fire 130/140 (10 lines Q40/Q42), riot 465/465; `vd4-fix-2` (same cells) **fire 124/124, riot 481/481, differences none, 0 resyncs**; `vd4-fix-1` (auto survey, other cells) fire 62/62, riot 546/546. DS41 filed | (1) answer DS41; (2) refresh coverage's disasters shadow numbers; (3) `enable = disasters` (glue20's next step 1) |

## Report

### What changed
- `engine/plugin/FC4DisasterHost.cpp`: `FireCoverageOver` gives the clamped inclusive occupant rectangle to
  `cISC4SimGrid::GetAverageValueInCellRect` as `x, z, w, h` with `w = x1 - x0 + 1`, `h = z1 - z0 + 1`
  (simgrid.md §4.1 and §8 item 2: the SDK's `nBottomRightX/Z` names are wrong; disasters.md §5.4 calls the logged
  `rect=56,86..56,87` "a 1x2 cell rectangle"). The core's `fire_coverage` host callback (§5.4 step 4, §5.6
  spread chance), the shadow's logged Ignite inputs and the script survey all use it. Before the fix, the host
  passed `(56, 86, 56, 87)`, so the average covered a 56x87 area and read 1 where the original reads its real
  coverage.
- `engine/plugin/FC4HarnessLogic.h`: `harness::CellRectArgs` (pure, added at the end of the header);
  `engine/tests/harness_logic_test.cpp` `CellRectArgsTests` (1x2 spread target, one cell, 4x2, empty rectangles).
- Empty rectangle (no overlap with the city): reads 0 (marked in the code); question **DS41** filed in
  docs/specs/questions/disasters.md.

### Diagnosis
The 211/263 / 2/354 coverage figure is glue13's old `dis-final-11`; glue20 (train4) had already reached exact on
its run. On current main, `vd4-base-1` (script: survey, then three `disaster fire auto` giving 58,81 / 57,84 / 58,84,
then `disaster riot auto` 40,33; normal pacing; the riot's emergency pause holds the date at 2026-02-10) gave fire
130/140, riot 465/465. The first primary mismatch is fire step 66. The original refused a spread `Ignite` of a
1x2 building (`rect=56,86..56,87 coverage=1 burning=0`, exits none of 1,2,3,5) and the mirror admitted it, so the
existing Q40 resync put it out. The rest was 8 location lines, one occupant line and the Q42 lot lag that
followed. §5.4 (the DS38 answer) says this is exit 4. Exit 4 could not trigger, because the coverage it reads was
averaged over the wrong rectangle.

### Measured (all `verdict: clean`, `ref_warnings=0`, `disasters_shadow = 1`, Konradshohe)
| Run | Tip | Script | Fire | Riot | Notes |
|---|---|---|---|---|---|
| vd4-base-1 | main 4726020a | vd4-script-1 (auto) | 130/140 (10 tagged Q40/Q42) | 465/465 | 1 refusal resync |
| vd4-fix-1 | e4c645b5 | vd4-script-1 (auto) | 62/62 | 546/546 | the survey now sees 58,81 as covered and picks 9,70 / 10,70 / 11,70: not the same scenario |
| vd4-fix-2 | e4c645b5 | vd4-script-2 (base cells fixed) | **124/124** | **481/481** | differences none, 0 refusals/resyncs; the original's spread refusal at 54,84 (`coverage=57`) is now refused by both (orig 0 ours 0) |

The 56,86 spread target did not come up again in vd4-fix-2. The spread draws depend on the real-time pacing, so
that exact call was not repeated. The refusal path (exit 4) did run and matched the original.

### Questions
DS41 (the empty-rectangle coverage).
