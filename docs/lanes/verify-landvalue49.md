| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-landvalue49: diagnose the landvalue shadow 72/72 -> 70/72 regression between train46 and train49 | engine implementer, `.claude/worktrees/verify-landvalue49` | in progress | see Handoff |

## Handoff

Goal: find why regress-train49-shadows has landvalue 70/72 (train46 72/72), resync 0, without changing sampling/masks/goldens.

Facts (from build-runs/evidence-run4, local copies, gitignored):
- p20 2027-08-08: intrinsic 6 (max 2), total 4; meanAlt orig/ours 270.054/270.05533. The ORIGINAL's p21 meanAlt is
  270.05533 = ours at p20: our pass (inputs read at pass end) saw zoning/terrain the original only used one pass later.
- p44 2029-08-10: one tract (16,58) intrinsic 14/15, total 139/140; meanAlt equal (one unzoned tract's altitude/land?).
- settled=0 on every pass of both runs (flag not discriminating). The two runs' cities diverge from p3 (meanAlt series differ).
- FC4HostGrid unread guard (train49) never fired: 0 "accessed before the store load" lines in either log; the landvalue
  shadow reads the original's grids through its own ReadGrid, not FC4HostGrid.
- Engine delta a94e4296..8375c651 touches no landvalue file.

Plan: add input-provenance diagnostics to capi/src/landvalue_shadow.rs (each SimNewDay, run our pass and keep its
intrinsic + meanAlt; at a differing pass end, report which day's inputs reproduce the original exactly). Do NOT change
the mismatch counting. Then a game run (unique label) to show the differing passes match an in-window day.

Done: baseline `all` PASS (cargo 3961, clippy 0, ctest 64/64). 2792626b: provenance implemented
(Shadow::note_day/provenance, test input_changed_after_the_original_read_it); quick PASS (capi 490).
Outstanding game request: vlv49-a1 (landvalue_shadow=1, turbo to 2032-01-01). Wait with
`qwait.py <WT>/build-runs/game-results/vlv49-a1.verdict` (no --fresh); a finished verdict is valid.
Next: grep vlv49-a1.log for "landvalue shadow 20" lines with "inputs:"; then full gate `all`, report.
