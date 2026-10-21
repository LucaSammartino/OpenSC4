| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-landvalue49: diagnose the landvalue shadow 72/72 -> 70/72 drop from train46 to train49 | engine implementer, `.claude/worktrees/verify-landvalue49` | ready: 2792626b adds input provenance to the landvalue shadow (diagnostic only, counts unchanged); diagnosis: the 2 differing passes come from inputs that changed while the original's pass ran, not from the engine change; vlv49-a1 (turbo) clean ref 0, 71/74; vlv49-n1 (normal speed) clean ref 0, 71/73; both input-timing passes traced to exact day inputs; gate PASS (cargo 3962, clippy 0, ctest 64/64); no ABI change, no questions | merge; on later regressions read the `inputs:` tail before suspecting the engine; pass-1 startup desirability limit (see report) |

## Report

### Question

regress-train46-shadows: landvalue 72/72 exact, resync 0. regress-train49-shadows: 70/72, resync 0. Differing passes:
p20 2027-08-08 (intrinsic 6 tracts max|d| 2, total 4) and p44 2029-08-10 (intrinsic 1, total 1, max|d| 1). Slope exact.

### Findings (evidence in build-runs/evidence-run4, local copies)

- No engine path connects the delta to land value. a94e4296..8375c651 changes no landvalue file. The only shared code
  is the FC4HostGrid "unread grid" guard, and it never fired: neither log has any "accessed before the store load"
  line. The landvalue shadow also reads the original's grids through its own ReadGrid, not through FC4HostGrid.
- The two runs are different cities: their meanAlt series split at pass 3 (train46 270.05667, train49 270.05533), so
  a per-pass comparison of 46 against 49 compares different inputs.
- p20 signature: meanAlt orig/ours 270.054/270.05533. The original's own p21 meanAlt is 270.05533. So our pass,
  which reads its inputs at the pass end, saw zoning or terrain that the original only used one pass later.
  meanAlt < modeAlt (272), so the pivot is meanAlt (§7.3), and a pivot shift changes a few tracts by 1 or 2.
- p44 signature: meanAlt is equal and one tract is off by 1. That fits a change to one unzoned tract (altitude or
  land) after states 1-4 read it.
- `settled=0` appears on every pass of both runs, so it cannot tell these passes apart.

### Change (2792626b, engine/capi/src/landvalue_shadow.rs and its tests)

After the first pass end, every SimNewDay also runs our pass (read-only) on that day's live inputs and keeps its
intrinsic grid, total grid, modeAlt and meanAlt. If a pass end's intrinsic grid or altitude scalars differ, the line
ends with `; inputs: ...`. That tail either names the first day whose inputs give the original's intrinsic, modeAlt
and meanAlt bitwise, or says that no day does (and names the closest day). The change is diagnostic only: mismatch
counts, sampling, masks, resync and the desirability rerun are unchanged, and no native output is adopted. The tail
avoids the words "mismatched=" and "cells=" so that log parsers do not miscount. New test:
`input_changed_after_the_original_read_it` (terrain raised between day 1 and day 2: names day 1/2; no days: says so;
only raised days: none reproduces; game unchanged).

### Game runs (DLL from 2792626b, Konradshohe, landvalue_shadow=1 only)

- vlv49-a1, turbo to 2032-01-01: clean, ref_warnings=0, 71/74 exact.
  - p2: intrinsic 6, meanAlt 270.05667/270.05804. "No SimNewDay of 9 reproduces" (closest is day 1, off in 3 tracts).
    This is the train49 p20 signature. Our meanAlt equals the original's p3 meanAlt.
  - p3: intrinsic 5, meanAlt 270.05804/270.05667. **Exact with the inputs of SimNewDay 1/28, total there exact.**
- vlv49-n1, normal speed to 2032-01-01: clean, ref_warnings=0, 71/73 exact.
  - p3: one tract, intrinsic and total each off by 1, meanAlt equal. This is the train49 p44 signature. **Exact with
    the inputs of SimNewDay 1/28, total exact.** So the effect is not limited to turbo.
- Both runs: p1 (2026-01-28) has total 391 or 383 with intrinsic 0. With landvalue_shadow alone the first pass ends
  on the day after load, before any SimNewDay desirability snapshot, so the LV2 previous-map candidate does not exist
  yet. In the regress runs (all shadows) p1 ends 2026-02-02 and is exact. This is an existing startup limit of the
  shadow and has nothing to do with this question.

### Conclusion

The 72 -> 70 drop is caused by the run's path, not by the engine. Some passes catch a zoning or terrain change that
happens after the original's states 1-4 read their inputs. Of the three input-timing passes in this lane's runs, two
are proven exact (bitwise) on the earlier day's inputs. The third (a1 p2) is consistent with the same cause, but no
daily snapshot reproduces it, so it stays unproven: state 3 is budget-sliced (§6), and a change can fall between two
of its calls, which a daily snapshot cannot capture. No unknown contract was found, so no question was filed.

### Gate

Baseline `all` PASS. Final `all` PASS: cargo test --workspace 3962 passed, clippy 0, build 27/27, ctest 64/64.

### Next steps

- Merge. On a later landvalue drop, read the `inputs:` tail before suspecting the engine.
- Optional tooling: take one desirability snapshot at the shadow's first event, so that p1 in a landvalue-only run
  gets its LV2 candidate.
