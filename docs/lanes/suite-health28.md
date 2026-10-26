| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD, reserved branch suite-health28 | WIP round 4; main train265 integrated; code 7123b69b3; ABI unchanged; questions none; final gates running | 1. Collect existing census. 2. Measure causes, coverage and tier1. 3. Enqueue tools READY after final gates. |

Round 3 committed merged-source harness fixes and save-grace integration; final handoff/enqueue was cut by the guard.
Existing census sh28-census3 has 179 submitted rows; its orchestrator ended. Do not submit a second census.
Queue inspection: 43 returned, two running, 134 not submitted; resumed the same manifest/labels with six-slot footprint.
Prior gate: tools pytest 510 PASS; qbuild all PASS (Rust 7880, clippy 0 warnings, CTest 123/123).
Main trains264/265 integrated; config merge preserves off-by-default stall grace and quit/port switches.
Acceptance 1 pending: collect outstanding shared-queue results into sh28-census3 and classify every non-clean row in causes.tsv.
Real-input triage at 107 returned: 99 clean, 7 known quit faults, 1 dialog-held (671s despite close_dialogs=1); 72 not returned at snapshot.
Acceptance 2 pending: measure clean share against 80%, reach union, and complete reach/shadow evidence separately.
Acceptance 3 pending: run coverage.py --suite sh28-census3 and record exercised / verified / replaced bytes.
Acceptance 4 pending: tier1 candidate estimates 95.24% union / 584s on six slots; require one clean merged-source run <=900s before marking.
Acceptance 5: initial pytest 512 PASS; corrected triage tests 9 PASS; full corrected pytest and single qbuild all running.
Keep previous source attempts separate; shared queue only, parallel=1, at most eight owned outstanding; Documents regions read-only.
Next: collect the in-flight census, measure acceptance, finish and commit; start no new census.
