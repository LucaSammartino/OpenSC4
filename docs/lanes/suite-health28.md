| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD, reserved branch suite-health28 | WIP round 5; code 618f7e2c0, 6fb072ff7; pytest 536 PASS; ABI unchanged; questions none | 1. Drain the existing census owner. 2. Refresh coverage and validate tier1. 3. Enqueue tools after qbuild all. |

Round 5 starts from the round 4 checkpoint; architect resumed sh28-census3 with six slots.
Live owner confirmed via native command-line query: PID 27572, suite.py resume sh28-census3 --slots 6; no second orchestrator.
Acceptance 1 progress: suite_triage/audit wrote causes.tsv for all missing/invalid rows plus known quit faults; final harvest waits for the owner.
Latest queue snapshot: 164/179 returned; active owner still submitting original rows; no resubmissions.
Acceptance 2 checkpoint: clean and complete reach/shadow evidence 156/179 = 87.15% (target 80%); strict actions 92/179 = 51.40%; not final.
Acceptance 3 snapshot: coverage.py --suite sh28-census3 PASS; suite-verified 0 bytes (0.0%); suite-exercised 1,545,277 (27.7%); suite-replaced 0.
Acceptance 4 pending: candidate estimate 95.24% union / 584s on six slots; one merged-source clean run <=900s required before tier1 marking.
Acceptance 5: main through train268 merged at 76eceff28; full corrected pytest 536 PASS; qbuild quick PASS (178/178); qbuild all pending.
Real commands: python -m pytest tools/tests -q printed 536 passed in 483.11s; initial attempt 535 PASS/1 PowerShell timeout, retained separately.
Harness causes: Souther Bay dialog-held; Magnolia unestablished; Port of Shiganshina window-exhausted. No invented exception addresses.
Parser now accepts only the known null access at SimCity 4.exe+0x1D0F94 after accepted quit; raw verdict/address retained in causes.tsv.
Shared queue only; parallel=1; at most eight owned outstanding requests; Documents regions read-only.
Next: drain PID 27572, run build-runs/health28_acceptance.py --tier (refuses a live owner), refresh coverage and final gate.
