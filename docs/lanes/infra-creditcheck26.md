| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| infra-creditcheck26: coverage claims before ready | Codex / `infra-creditcheck26` (tools) | Ready; r2 `ecfa2f689`; cargo 7839 passed, clippy clean, CTest 114/114; historical train238 credit +169566 B, 10 verified gains, no losses/refusals | Coordinator: rerun six existing Node dry-run tests with child-process access |

R2 preserves main's verified credit (current main coverage.py statuses, coverage-shadows.tsv lane runs and newest main regress summary), adding exact supplied-log evidence; unchanged rows retain every column.
Supplied logs use the target tree's run_summary.ps1 AND coverage.py parser, including underscore labels; temporary files/bytecode stay out of other worktrees. Changed shadow cells still require exact supplied logs and profile membership.
R1 refusal rules, enqueue/plan/land integration and eight-line coordinating obligation remain intact. No coverage-map edits, coverage.py scoring changes, ABI changes, questions or new game runs.
Focused tests: 48/48, including fixture-based ten-row acceptance and inherited-credit controls. Full tools suite: 205 passed, six existing WorkflowDryRun failures; shell and direct Node child probes reproduce sandbox EPERM.
Full qbuild gate ran once: PASS; cargo 7839/0, clippy zero diagnostics, native build 46/46 edges, CTest 114/114.
The exact requested command (--wt tools-shadowlabels26, --log regress-train238-shadows.log) reports `coverage delta: specified +0 B, implemented +0 B, verified +0 B, replaced +0 B`, no refusals: that lane is already on main (train239).
Historical replay pins only merge-base `626043df44ed2c53fd2686c831056fddc2ac9267` and main summary regress-train238.out, using the real lane tree/log, real attribution and checker CLI; no refs changed. Output:
```text
cGZDBSegment*: verified 0->1; specified +0 B, implemented +0 B, verified +19175 B, replaced +0 B
cSC4NetworkTool: verified 0->1; specified +0 B, implemented +0 B, verified +105916 B, replaced +0 B
cSC4PowerLineTool: verified 0->1; specified +0 B, implemented +0 B, verified +14712 B, replaced +0 B
cSCExemplar*: verified 0->1; specified +0 B, implemented +0 B, verified +15351 B, replaced +0 B
cSCProperty: verified 0->1; specified +0 B, implemented +0 B, verified +200 B, replaced +0 B
cSCPropertyHolder: verified 0->1; specified +0 B, implemented +0 B, verified +2778 B, replaced +0 B
cSCResExemplar: verified 0->1; specified +0 B, implemented +0 B, verified +2096 B, replaced +0 B
cSCResExemplar*: verified 0->1; specified +0 B, implemented +0 B, verified +6145 B, replaced +0 B
cSCResExemplarCohort: verified 0->1; specified +0 B, implemented +0 B, verified +2072 B, replaced +0 B
cSCResProperty: verified 0->1; specified +0 B, implemented +0 B, verified +1121 B, replaced +0 B
coverage delta: specified +0 B, implemented +0 B, verified +169566 B, replaced +0 B
```
Evidence: build-runs/replay-creditcheck-r2.py, credit-shadowlabels-replay.json, credit-shadowlabels-current.json, credit-r2-focused.xml, credit-r2-tools.xml and qbuild/FC4Verify logs. Remaining: coordinator rerun of the six sandbox-restricted tests.
