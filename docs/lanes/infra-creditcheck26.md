| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| infra-creditcheck26: coverage claims before ready | Codex / `infra-creditcheck26` | Ready; cargo 7839 passed, clippy clean, CTest 114/114 | Coordinator: rerun six existing Node dry-run tests with child-process access |

Implemented `tools/creditcheck.py`: merge-base tree comparison using coverage.py attribution, method-first row assignment and status rules; per-row flips and byte deltas; repeatable game logs summarized by run_summary.ps1; JSON claims and exit-2 refusals.
Changed map cells require existing spec paths, Rust source, exact non-resynced shadow evidence in the tree's profile, and replace-list membership.
Unchanged map debt is reported through the scorer without refusing unrelated lanes; lane verification never borrows main's regress or out-of-profile shadow evidence.
The replacement verdict retains the base regress result because the task overrides only verified evidence.
Enqueue records credit and holds refusals as wip; readiness rejects missing, refused or stale-tip claims; plan prints claims and refusal reasons.
Land measures and prints coverage before/after the merge and its byte deltas.
Coverage.py only exposes existing scoring steps and an optional regress revision; its default scoring and console output are preserved.
Added the eight-line coordinating obligation. No coverage-map or regress.ps1 edits, ABI changes, or real-game requests.

Validation: 92/92 focused coverage, regress, merge-train and creditcheck tests pass, including 38 new tests with fixture trees and actual PowerShell summary output.
Full tools suite: 193 passed, six existing WorkflowDryRun tests failed because this sandbox denies Node child creation (`spawnSync ... EPERM`); both shell and direct-child probes reproduce it.
Full gate ran once: cargo 7839/0, clippy zero diagnostics, native build 294/294 edges, CTest 114/114; qbuild PASS.
Evidence: `build-runs/credit-tests-second.log`, `tools-tests-first.log`, `diagnose-credit-shell.mjs`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
Remaining: coordinator rerun of the six Node fixture tests outside the child-process restriction; no known implementation gaps. Questions: none.
