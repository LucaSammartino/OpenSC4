| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-shadowlabels26: credit four recorded shadows | Codex / tools-shadowlabels26 | ready; full gate passed | Land; rerun six workflow dry-runs outside the restricted Node sandbox |

`run_summary.ps1` accepts multi-word/underscore labels and Julian stamps, normalizing spaces to underscores.
Formats counts only per-file lines with positive comparisons; differ/rejected entries prevent exact credit. Empty files and totals supply no evidence.
`coverage.py` accepts underscores in comparison and resync rows. Only authorized shadow cells changed in the coverage map.
Fixtures copy train238 planning, DBPF, formats file/total lines; synthetic failures cover differ/rejected entries, mismatches and resets.

Old/new parser comparison: train238 retained all 66 other shadow rows identically; older train231 retained all 64.
Train238 adds nettool_plan 73/73, powerline_plan 73/73, dbpf_bounds 73/73, formats 1/1 (8,957 entries compared).
Coverage with regenerated train238 summary: verified 1,541,406/5,584,344 bytes (27.6%) -> 1,710,972 (30.6%): +169,566 bytes, +3.04 percentage points; 6,705 -> 7,320 functions.
Verified flips: cSC4NetworkTool, cSC4PowerLineTool, cGZDBSegment*, cSCResExemplar, cSCResExemplarCohort,
cSCResExemplar*, cSCExemplar*, cSCPropertyHolder, cSCProperty, cSCResProperty. No rows lost verified status.

Focused tests: 5/5 passed. `python -m pytest tools/tests -q`: 162 passed, 6 existing WorkflowDryRun failures; independent Node spawnSync probes return EPERM for both python and cmd.exe in this sandbox.
Full final `qbuild.py all`: PASS; cargo 7,839 passed/0 failed/0 ignored, clippy 0 diagnostics, CMake 294/294 edges, CTest 114/114.
Reused game evidence: regress-train238-shadows verdict clean, ref_warnings=0; four shadows exact. No game requests submitted.
Local evidence: `build-runs/summary-{train238,train231}-{before,after}.txt`, `coverage-shadowlabels-{before,after}.md`, `statuses-shadowlabels-{before,after}.json`, `pytest-shadowlabels.log`, `qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`.
Remaining: rerun the six environment-blocked workflow tests on the coordinator host. Questions: none. C ABI exports unchanged.
