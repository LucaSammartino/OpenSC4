tag: run-39
run: 39
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: shadow build `D69F64FF…` (merged as `6ca4b65`), `disable = budget` (our clock, the original budget), `budget_shadow = 1`, Spandau
result: **clean, and the budget matches the original exactly.** At each of five month-ends, our tax scan ran on the same live lots (413–417) just before the original settled, in the same process. All 12 tax types, total tax, income, expense and net were **identical every month** (`mismatches=0`, `months=5 clean=5`). On 2002-01-01 income/expense can't be observed because the year-to-date resets (QUESTIONS BQ2), but tax and net still matched. This was also the first in-game run of our clock driving the original budget.
