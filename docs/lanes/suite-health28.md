| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD, reserved branch suite-health28 | WIP round 4; tip a62ac9ec1; ABI unchanged; questions none; prior gate PASS | 1. Collect existing census. 2. Measure causes, coverage and tier1. 3. Run final gate and enqueue tools READY. |

Round 3 committed merged-source harness fixes and save-grace integration; final handoff/enqueue was cut by the guard.
Existing census sh28-census3 has 179 submitted rows; its orchestrator ended. Do not submit a second census.
Operator checkpoint: 38 returned, 33 clean and 5 clean-exit-crash; two short controls clean.
Prior gate: tools pytest 510 PASS; qbuild all PASS (Rust 7880, clippy 0 warnings, CTest 123/123).
Main now 373f13090 adds regionalcity replace coverage and KNOWN original-game quit-time construction-site faults.
Acceptance 1 pending: collect outstanding shared-queue results into sh28-census3 and classify every non-clean row in causes.tsv.
Acceptance 2 pending: measure clean share against 80%, reach union, and complete reach/shadow evidence separately.
Acceptance 3 pending: run coverage.py --suite sh28-census3 and record exercised / verified / replaced bytes.
Acceptance 4 pending: tier1 candidate estimates 95.24% union / 584s on six slots; require one clean merged-source run <=900s before marking.
Acceptance 5 pending: run final pytest and qbuild all once; enqueue tools READY after passes even if health targets remain unmet.
Keep previous source attempts separate; shared queue only, parallel=1, at most eight owned outstanding; Documents regions read-only.
Next: collect the in-flight census, measure acceptance, finish and commit; start no new census.
