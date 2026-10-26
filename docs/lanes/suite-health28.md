| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD, reserved branch suite-health28 | WIP round 5; HEAD c14bb978e; ABI unchanged; questions none | 1. Harvest the existing census. 2. Merge main and measure acceptance. 3. Validate tier1 and enqueue tools after gates. |

Round 5 starts from the round 4 checkpoint; architect resumed sh28-census3 with six slots.
Live owner confirmed via native command-line query: PID 27572, suite.py resume sh28-census3 --slots 6; no second orchestrator.
Existing docs/coverage-report.md modification predates this round and must be preserved.
Acceptance 1 pending: harvest existing results and classify every non-clean row in causes.tsv.
Latest queue snapshot: 153/179 returned, six active, 20 unsubmitted; existing orchestrator owns submission.
Acceptance 2 pending: refresh clean share against 80%, reach union and complete reach/shadow evidence share.
Acceptance 3 priority pending: coverage.py --suite sh28-census3; report suite-exercised / verified / replaced.
Acceptance 4 pending: candidate estimate 95.24% union / 584s on six slots; one merged-source clean run <=900s required before tier1 marking.
Acceptance 5: main through train268 merged at 76eceff28; tools pytest and coverage running; qbuild all pending.
Prior gates: pytest 512 PASS before triage correction; corrected triage tests 9 PASS; qbuild all previously PASS (Rust 7880, CTest 123/123).
Parser now accepts only the known null access at SimCity 4.exe+0x1D0F94 after accepted quit; raw verdict/address retained in causes.tsv.
Shared queue only; parallel=1; at most eight owned outstanding requests; Documents regions read-only.
Next: verify live census owner, merge main, harvest and measure coverage before tier1 verification.
