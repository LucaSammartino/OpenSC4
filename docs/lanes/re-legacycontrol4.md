| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacycontrol4: docs/specs/legacy-control.md for owner cGZLegacyControl (45 of 57 complete) | Claude / .claude/worktrees/re-legacycontrol4 | in progress (round 3) | LC4 15 interpreters; LC7 grid cell nesting; LC8 per-type fragment writers |

## Handoff
- Goal (round 3): LC3 serialiser + LC1 vtable (done), then LC7 grid cell nesting and LC4 interpreters largest first.
- Done round 3: section 3 control-area vtable 0x00AD5344 (LC1 answered), section 7.6 script fragment format
  (LC3 answered; per-type writer word sets filed as LC8), quirks Q8/Q9; coverage corrected to 41 complete + Grid partial.
- Done: LC7 answered (9.5 step 11, Q10); Folders 9.6; FlatRect 9.7; Outline 9.8. Next: LC4 remaining 12 largest-first:
  Btn 0x0094F757, TextEdit 0x0094E7C8, ... (sizes in section 4.1 / names_windows.tsv).
- Facts: REA not used (export + decomp/ + raw image bytes via build-runs/peek.py, tbl.py). No ABI rows, no engine, no game runs.
- Earlier rounds: e2534f7b..b81957ea (see git log main..).
