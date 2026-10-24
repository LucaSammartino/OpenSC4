tag: glue-varfeat-lines5-2032-20261024a-342
run: 349
date: 2026-10-24
lane: glue-varfeat-lines5
dll: FreeCity4-glue-varfeat-lines5; sha256 prefix 38a09e75; implementation 640b40342
pacing: turbo
verdict: clean
switches: variant_shadow=1, feature_shadow=1, turbo=1, popup_dialogs=0; Konradshohe; end_date=2032-01-01
result: clean at 2032-01-01, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0; 33.750 real seconds, 64.17 days/s. Shadow exactness: variant 1/1; feature 75/75; both resynced lines=0. Original variant totals: cases=70 checks=420 exact=420 failures=0 refusals=0 clones=71 scalar_shape_HG7=0 unexplained=0. Original feature totals: samples=74 exact=13172 diff=0 refusal=0 resync=0 lifetime_exact=15 lifetime_diff=0 app_hold_remaining=1.

The queue request/result label is `glue-varfeat-lines5-2032-20261024a`.
Evidence is retained under this worktree's `build-runs/game-results/` as the label's `.log` and `.verdict`.
The coordinator-generated verdict summary reports:

```text
shadow lines:
  variant          1 lines, 1 with every mismatch count 0
  variant resynced lines: 0
  feature         75 lines, 75 with every mismatch count 0
  feature resynced lines: 0
```

Independent validation using the task-supplied summary regex returned `variant 1/1` and `feature 75/75`.
The variant case group is dated 2026-01-27. Feature reports cover 74 query samples and one final totals group;
the final live-city date is 2032-01-02 during shutdown. Every standard line has `mismatches=0`.
No comparison behavior changed, and no excluded native operations gain coverage from these reporting lines.
