# Token ledger

Generated 2026-10-20 by `python tools/lanes.py ledger` from 102 workflow run journals (2026-10-14 to 2026-10-20) in `%USERPROFILE%\.claude\projects\<slug>\*\workflows\`. Rerun it to refresh; the per-run and per-lane tables are `build-runs/lane-ledger.tsv` and `build-runs/lane-ledger-by-lane.tsv` (gitignored).

Tokens are the journal's own `tokens` per agent (what the run reports; not split into input, output and cache). A lane is one lane name: runs that were re-run or resumed are summed. `merge` is the agent that merged the lane into main under the old merge-lock protocol (the merge train has no such agent).

## Totals

- 102 runs: 98 lane runs, 4 other (audits, doc cleanups); 98 distinct lanes.
- 38,162k tokens in all; 35,694 tool calls; 211 agent-run hours (wall time of each run, summed).
- Lane agents 24,311k, merge agents 9,723k, other agents 4,128k.
- **Merge share: 28.5% of the tokens of lane runs** (9,723k of 34,121k) went to merging, not to the lane's work.
- Landed: 93 of 98 lanes are on main (branch an ancestor of main, or the branch deleted and the journal's merge commit an ancestor); 58 lanes have a journal merge commit that is on main, 0 have one that is not (reported merged, reverted or rewritten).
- Status of the runs: 74 completed, 24 killed, 4 failed.
- 10,404k tokens (27%) were spent in runs that were killed before they finished.

## Per kind

Per lane (all runs of the lane summed). `engine?` = a wave lane whose kind no journal or queue file names.

| Kind | Lanes | Avg tokens | Median | Avg tool calls | Avg hours | Merge share | Landed |
|---|---:|---:|---:|---:|---:|---:|---:|
| engine | 43 | 377k | 367k | 417 | 2.5 | 33% | 40/43 |
| re | 37 | 321k | 304k | 289 | 1.3 | 23% | 35/37 |
| engine? | 9 | 313k | 308k | 407 | 1.3 | 37% | 9/9 |
| tools | 7 | 264k | 260k | 90 | 0.7 | 33% | 7/7 |
| docs | 2 | 640k | 640k | 598 | 1.4 | 0% | 2/2 |
| **all** | 98 | 347k | 332k | 348 | 1.8 | 29% | 93/98 |

## Ten most expensive lanes

| Lane | Kind | Runs | Tokens | of which merge | Tool calls | Hours | Landed |
|---|---|---:|---:|---:|---:|---:|---|
| glue21 | engine | 3 | 894k | 0k | 2159 | 5.7 | no |
| cores12a | engine | 2 | 784k | 396k | 498 | 2.9 | yes |
| cores27 | engine | 1 | 764k | 0k | 961 | 2.9 | yes |
| re-netocc | re | 1 | 746k | 0k | 853 | 1.0 | yes |
| re-answers34 | docs | 1 | 746k | 0k | 817 | 2.2 | yes |
| re-occprops | re | 1 | 738k | 0k | 990 | 1.6 | no |
| cores15 | engine | 2 | 687k | 292k | 404 | 7.1 | yes |
| re-terrain3d | re | 1 | 665k | 0k | 1171 | 1.3 | yes |
| zones2 | engine? | 1 | 614k | 127k | 527 | 2.0 | yes-commit |
| cores14a | engine | 1 | 550k | 216k | 229 | 4.0 | yes |

## Tokens per coverage point

Headline of `docs/coverage-report.md` (% of game code bytes) at the oldest and newest report commit on main's first-parent line: specified 32.4% -> 35.7%, implemented 21.0% -> 26.0%, verified 8.3% -> 11.3%, replaced 12.1% -> 12.1%.

| Report | Subject | Lanes merged since the previous report | Lane tokens | d specified | d implemented | d verified | d replaced |
|---|---|---|---:|---:|---:|---:|---:|
| 1a15a725 | docs: coverage row in HANDOFF section 0 (merged 50f2bb6); co | none found | 0k | +0.2 | +0.0 | +0.0 | +0.0 |
| f92fb11c | docs: measured progress in README (coverage table and treema | abitidy2, glue16, histclock, re-answers12, re-answers24, re-answers25, re-specs15a (+1) | 2,533k | +0.0 | +0.0 | +1.2 | +0.0 |
| c7ebfd94 | coverage: split the 203 KB draw range into the five span ren | apply14dis, cores12c, cores18, coverage, coverage2, detpacing, mergetrain-docs (+7) [3 without a journal] | 3,595k | +0.2 | +0.2 | -1.2 | +0.0 |
| 9e46f401 | docs: coverage report and treemap regenerated at main (regre | apply14dis, apply15b, apply15c, cores12c, cores17, cores18, coverage2 (+6) [4 without a journal] | 2,671k | +0.1 | +0.1 | +1.2 | +0.0 |
| 4a91d9db | docs: coverage report and treemap regenerated after batch tr | abi-hygiene, apply14traffic, apply15d, apply15terrain, cores12a, cores19, devperf (+7) [4 without a journal] | 3,465k | +0.0 | +0.0 | +0.0 | +0.0 |
| cb19fecf | coverage3: attribute the landed subsystems; shadow evidence  | apply15e, cores21, cores22, cores24, coverage, coverage3, glue20 (+6) [12 without a journal] | 170k | +0.0 | +1.8 | +1.0 | +0.0 |
| 49e64240 | docs: coverage after train6 (implemented 21.3 -> 23.1%, veri | apply15b, apply15terrain, cores17, cores19, cores24, cores25, coverage3 (+7) [7 without a journal] | 2,178k | +0.0 | +0.0 | +0.0 | +0.0 |
| a3689de3 | docs: train8 lane rows and run files get the generator's fro | cores20, cores23, glue18, glue22, glue27, re-answers27, re-answers32 [6 without a journal] | 528k | +0.0 | +0.0 | +0.0 | +0.0 |
| b2412f01 | coverage: attribute the advisor system and the MySims to the | cores26, re-answers33 | 534k | +0.0 | +2.2 | +0.0 | +0.0 |
| 55a071ba | docs: coverage after train9 - implemented 25.3% (cores26's a | none found | 0k | +0.0 | +0.0 | +0.0 | +0.0 |
| a9bfd32b | docs: coverage after train10 - implemented 25.3% | glue19 | 537k | +0.0 | +0.0 | +0.0 | +0.0 |
| 27afb623 | docs: train10 landed (cores27, ABI 75); coverage and ledger  | cores27 | 764k | +0.0 | +0.0 | +0.0 | +0.0 |
| 8317de68 | docs: train12 landed (glue24fix2: the network managers glue, | glue24, glue24fix2, re-answers34 | 1,333k | +0.0 | +0.0 | +0.0 | +0.0 |
| 98662e3b | coverage: wire network-occupants.md into the map; specified  | re-netocc | 746k | +2.1 | +0.0 | +0.0 | +0.0 |
| fb3f3e7e | docs: train14 landed (re-terrain3d: terrain 3D view contract | re-terrain3d | 665k | +0.7 | +0.7 | +0.8 | +0.0 |

Over the 13 report intervals in which a merged lane could be identified (92 lanes, 19,718k lane tokens, 12,982k of them in engine lanes):

- **implemented**: +5.0 points; engine-lane tokens per point 2,596k; all-lane tokens per point 3,944k.
- **replaced**: +0.0 points; not computable (no gain).
- specified: +3.1 points; all-lane tokens per point 6,361k.
- verified: +3.0 points; all-lane tokens per point 6,573k.

How far to trust it: coverage points also move when the coverage tool or its map changes (`coverage:` commits re-split or re-credit code without any lane), so some of the movement is not lane work; lanes are attributed by name found in the merge commit messages and by the merge commit in the journals, so a lane that landed some other way is missed; a lane's tokens count in the interval in which it merged, however long it ran before; tools, docs and RE lanes cost tokens but do not move implemented or replaced, hence the engine-lane column. Treat the numbers as an order of magnitude, not a price list.

