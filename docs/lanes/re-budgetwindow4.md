| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budgetwindow4: resolve budget-window.md questions BW1-BW9 (RE spec lane) | fc4-lane agent, .claude/worktrees/re-budgetwindow4 | in progress | BW1, BW2, BW3, BW6 first, then BW7-BW9, BW4/BW5 |

## Handoff

- Goal: answer BW1-BW9 (docs/specs/questions/budget-window.md) into docs/specs/budget-window.md, byte-backed.
- Done: 7856795d spec answers written in budget-window.md: §3 (+0x24 is a department ID), §4.1 (BW1 lists A income/B expense, special-group headers), §4.4 (BW8 slider f32 math), §5 table (no refresh tail, ordinance rows 300-699), §5.1, §5.3 scroll (BW7), §5.4 ApplyDealEdits 0x0077AB40 (BW2), §5.5 SetOccupantFundingLevel (BW3), §5.6 popup child +0x14, §5.7 camera jump.
- Next: move BW1/BW2/BW3/BW7/BW8 entries to docs/specs/questions/answered/budget-window.md (DIFFERS where core differs: no tail refresh, f32 product, +0x24 dept ID, levels f32), run tools/questions.py index; then BW6 (dosub.asm/destroy.asm/create.asm), BW9 (e440 on_shutdown, DoWinMsg returns), BW4/BW5; update §8 and §9 tests.
- Facts: asm dumps in build-runs/*.asm (regenerate with re_query.py asm <addr> --max 5000 --full); build-runs/rd.py reads exe bytes (VA count b|d|f). REA had no binary open (fallback to export). BW8 restore-record fill point not yet checked (initIndividualBudget 0x00786690).
