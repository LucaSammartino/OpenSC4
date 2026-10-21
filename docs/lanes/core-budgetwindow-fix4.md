| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow-fix4: apply BW1-BW4, BW6-BW9 answers to core budget_window (budget-window.md) | fc4-lane agent, .claude/worktrees/core-budgetwindow-fix4 | ready: core only, gate PASS, no game run (core-only lane); BW5 and BW10 open, so not whole-owner | answers to BW5/BW10; glue lane wiring the host |

## Report

### What changed (engine/core/src/budget_window/ only)
- `+0x24` is now a department ID (individual mode when non-zero); service kind `+0x134` from the §5.5 table.
- Group mode: §4.1 header rows (special groups test total income / ideal funding), line-item visibility, list lengths, background `0x140155D2`, SetupWindowBig(min 9), empty window held at `+0x14`.
- Buttons: §5 table with exact returns, no common refresh tail, unmatched IDs 0 with no refresh; 0x68/0x384 RemoveChild without null check; ApplyDealEdits (0xCF, §5.4); check boxes 300-499 and info popup 500-699 through the row map; nested individual dialog (0x0ABCE6xx, `+0x1C`) in core; camera jump 0x0ABCDExx in individual mode.
- Slider: lookup failure does nothing; group fraction is the f32 product (30 -> 0x3E999999, 59 of 0-200 differ from the f64 path); individual level is unscaled f32, SetOccupantFundingLevel in core (hospital/school/police/fire/power).
- DoWinMsg returns bool per BW9; on_init null/second-init; on_shutdown (vectors emptied, count kept, `+0x0C` released); end_dialog passes the code; panel releases `*out` and the generator ref; destructor releases `+0x14`/`+0x0C`; ShowWindow tab commands (BW4).
- Kept: unguarded HiddenResume on create failure (counter -1). BW5: raw bit 0x4 test only, no UI meaning claimed.

### Measured
- Baseline qbuild all: 3944 tests, clippy 0. Final: 3947 passed / 0 failed, clippy 0, ctest 64/64, qbuild PASS. budget_window tests: 24 (cover §9 tests 1-13).

### Questions
- BW10 filed (docs/specs/questions/budget-window.md): who sets `+0x10`; sliders/saved fractions in the empty-group case; initial arrow state in on_init.
