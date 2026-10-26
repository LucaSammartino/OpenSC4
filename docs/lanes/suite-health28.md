| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD, reserved branch suite-health28 | WIP round 3; merging main 25b766ada; ABI unchanged; questions none | 1. Finish merged-source census and causes. 2. Measure clean share and assess tier1. 3. Run tooling gate and enqueue tools. |

Round 2 ended at 70c07036f without a handoff; its raw attempts remain under build-runs/suite/sh28-*.
Round 3 keeps original, corrected-source and merged-source results separate.
Acceptance 1 pending: merged-source reach-on census, causes.tsv and comparison with suitecov27's 134 returned / 4 clean.
Acceptance 2 pending: harness repairs and minimal game/engine findings; report every remaining non-clean row and verdict.
Acceptance 3 pending: 80% clean with complete reach/shadow evidence; do not infer this from verdicts alone.
Acceptance 4 pending: tier1 needs 95% of union and measured <=900s on six slots; leave candidates unmarked without proof.
Acceptance 5 pending: python -m pytest tools/tests -q, real-input runs, final qbuild all for scenario catalogue changes.
Runs use the shared queue with parallel=1 and at most eight owned outstanding requests; Documents regions stay read-only.
Next: merge main, audit prior attempts, run the final merged-source census, and replace this checkpoint with measured results.
