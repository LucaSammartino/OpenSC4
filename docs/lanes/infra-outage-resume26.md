| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| infra-outage-resume26: Codex outage recovery and round caps | `Codex` / `infra-outage-resume26` (tools) | ready; cargo 7839/0, clippy zero diagnostics, CTest 114/114; focused pytest 42 passed; tools pytest 176 passed, 6 sandbox failures; ABI unchanged | (1) Rerun the six existing WorkflowDryRun tests where Node can spawn child processes. |

Implemented same-session transport recovery with at most three resumes after 60/180/600 seconds, retained CLI permissions/configuration, accumulated usage, and timestamped retry records in codex-status.json.
SessionStart preserves the original round start and refreshes the transcript for the registered session; a different session remains refused.
Active round time excludes resume waits. The default 40-minute budget restricts work at 35 minutes and terminates Codex at 45 minutes with continue; args.roundMinutes shifts both thresholds.
Validated fast/default service tiers pass through codex.service_tier or overriding args.serviceTier. Normal command lines remain unchanged when omitted.
Code: tools/lane_backend.py and .claude/lane-bin/codexguard.py; scripted fake Codex process and expanded tests in tools/tests. Eight lines document the behaviors in docs/ops/coordinating.md.
Focused validation: 42 passed (build-runs/pytest-backends-3.log). Full tools validation: 176 passed, 6 failed (build-runs/pytest-tools-final.log).
The six failures are existing WorkflowDryRun tests: Node cannot spawn cmd.exe (EPERM) in this sandbox. Direct Python spawning did not clear the restriction; diagnostic source changes were reverted. Evidence: build-runs/workflow-environment-diagnostic.txt.
Full final-source gate: qbuild all PASS; cargo 7839 passed across 52 suites, zero failed/ignored; clippy zero diagnostics; native build 294/294 edges; CTest 114/114. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Native queue label: qbuild-20261025-104541-build-ctest, terminal verdict pass. No outstanding requests or game runs; no ABI changes or questions.
Still missing: an environment-independent rerun of the six workflow dry runs; no implementation work remains from the brief.
