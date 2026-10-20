# Token ledger

Generated 2026-10-20 by `python tools/lanes.py ledger` from 74 workflow run journals (2026-10-14 to 2026-10-16) in `%USERPROFILE%\.claude\projects\<slug>\*\workflows\`. Rerun it to refresh; the per-run and per-lane tables are `build-runs/lane-ledger.tsv` and `build-runs/lane-ledger-by-lane.tsv` (gitignored).

Tokens are the journal's own `tokens` per agent (what the run reports; not split into input, output and cache). A lane is one lane name: runs that were re-run or resumed are summed. `merge` is the agent that merged the lane into main under the old merge-lock protocol (the merge train has no such agent).

## Totals

- 74 runs: 71 lane runs, 3 other (audits, doc cleanups); 81 distinct lanes.
- 30,538k tokens in all; 20,321 tool calls; 177 agent-run hours (wall time of each run, summed).
- Lane agents 16,686k, merge agents 9,723k, other agents 4,128k.
- **Merge share: 36.7% of the tokens of lane runs** (9,723k of 26,497k) went to merging, not to the lane's work.
- Landed: 74 of 81 lanes are on main (branch an ancestor of main, or the branch deleted and the journal's merge commit an ancestor); 58 lanes have a journal merge commit that is on main, 0 have one that is not (reported merged, reverted or rewritten).
- Status of the runs: 51 completed, 23 killed.
- 10,404k tokens (34%) were spent in runs that were killed before they finished.

## Per kind

Per lane (all runs of the lane summed). `engine?` = a wave lane whose kind no journal or queue file names.

| Kind | Lanes | Avg tokens | Median | Avg tool calls | Avg hours | Merge share | Landed |
|---|---:|---:|---:|---:|---:|---:|---:|
| engine | 35 | 358k | 344k | 270 | 2.6 | 43% | 28/35 |
| re | 31 | 297k | 299k | 194 | 1.3 | 30% | 31/31 |
| engine? | 8 | 352k | 340k | 334 | 1.5 | 37% | 8/8 |
| tools | 7 | 264k | 260k | 90 | 0.7 | 33% | 7/7 |
| **all** | 81 | 326k | 308k | 232 | 1.8 | 37% | 74/81 |

## Ten most expensive lanes

| Lane | Kind | Runs | Tokens | of which merge | Tool calls | Hours | Landed |
|---|---|---:|---:|---:|---:|---:|---|
| cores12a | engine | 2 | 784k | 396k | 498 | 2.9 | yes |
| cores15 | engine | 2 | 687k | 292k | 404 | 7.1 | yes |
| zones2 | engine? | 1 | 614k | 127k | 527 | 2.0 | yes-commit |
| cores14a | engine | 1 | 550k | 216k | 229 | 4.0 | yes |
| seeding | engine | 1 | 547k | 231k | 285 | 4.8 | yes |
| apply12 | engine | 2 | 518k | 152k | 277 | 2.5 | yes |
| re-specs16 | re | 1 | 494k | 146k | 318 | 6.3 | yes |
| cores14c | engine | 1 | 479k | 235k | 244 | 5.9 | yes |
| apply14traffic | engine | 1 | 461k | 143k | 376 | 2.1 | yes |
| apply11traffic | engine | 1 | 461k | 173k | 234 | 3.6 | yes |

## Tokens per coverage point

Headline of `docs/coverage-report.md` (% of game code bytes) at the oldest and newest report commit on main's first-parent line: specified 32.4% -> 32.9%, implemented 21.0% -> 21.3%, verified 8.3% -> 9.5%, replaced 12.1% -> 12.1%.

| Report | Subject | Lanes merged since the previous report | Lane tokens | d specified | d implemented | d verified | d replaced |
|---|---|---|---:|---:|---:|---:|---:|
| 1a15a725 | docs: coverage row in HANDOFF section 0 (merged 50f2bb6); co | none found | 0k | +0.2 | +0.0 | +0.0 | +0.0 |
| f92fb11c | docs: measured progress in README (coverage table and treema | abitidy2, glue16, histclock, re-answers12, re-answers24, re-answers25, re-specs15a (+1) | 2,533k | +0.0 | +0.0 | +1.2 | +0.0 |
| c7ebfd94 | coverage: split the 203 KB draw range into the five span ren | apply14dis, cores12c, cores18, coverage, coverage2, detpacing, mergetrain-docs (+7) [3 without a journal] | 3,595k | +0.2 | +0.2 | -1.2 | +0.0 |
| 9e46f401 | docs: coverage report and treemap regenerated at main (regre | apply14dis, apply15b, apply15c, cores12c, cores17, cores18, coverage2 (+6) [4 without a journal] | 2,671k | +0.1 | +0.1 | +1.2 | +0.0 |
| 4a91d9db | docs: coverage report and treemap regenerated after batch tr | abi-hygiene, apply14traffic, apply15d, apply15terrain, cores12a, cores19, devperf (+7) [4 without a journal] | 3,465k | +0.0 | +0.0 | +0.0 | +0.0 |
| 9c32bad0 | docs: the apply15e lane note, and unblock the lane/run table | apply15e, cores21, cores22, glue20, re-answers30, re-specs19, re-specs20 [7 without a journal] | 0k | +0.0 | +0.0 | +0.0 | +0.0 |
| 6b6d17c7 | docs: coverage after merge-train batch train4 (7 lanes) | none found | 0k | +0.0 | +0.0 | +0.0 | +0.0 |

Over the 5 report intervals in which a merged lane could be identified (56 lanes, 12,264k lane tokens, 8,916k of them in engine lanes):

- **implemented**: +0.3 points; engine-lane tokens per point 29,720k; all-lane tokens per point 40,881k.
- **replaced**: +0.0 points; not computable (no gain).
- specified: +0.3 points; all-lane tokens per point 40,882k.
- verified: +1.2 points; all-lane tokens per point 10,220k.

How far to trust it: coverage points also move when the coverage tool or its map changes (`coverage:` commits re-split or re-credit code without any lane), so some of the movement is not lane work; lanes are attributed by name found in the merge commit messages and by the merge commit in the journals, so a lane that landed some other way is missed; a lane's tokens count in the interval in which it merged, however long it ran before; tools, docs and RE lanes cost tokens but do not move implemented or replaced, hence the engine-lane column. Treat the numbers as an order of magnitude, not a price list.

