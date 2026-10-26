| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suite-health28: scenario harness health; tools | Codex / suite-health28 / detached HEAD; reserved branch suite-health28 | READY to land tooling; code 4d4097239, 828f60d86, 02c881ea2; pytest 542, cargo 7889, clippy 0 diagnostics, CTest 124/124 PASS; census 175/179 complete; shadows 279185/285410 exact; ABI unchanged; questions none; tier1 qualification FAILED | 1. Resolve registered tier faults and remaining harness causes. 2. Remeasure and qualify tier1; it remains unmarked. |

Final round 2026-10-26: finished the existing census and its planned tier measurement; both owners drained.
Census acceptance PASS: sh28-census3 returned 179/179; clean with complete reach/shadows 175/179 = 97.77% (target 80%); 178 reach traces.
Qualification is narrow: 17 clean rows include the known post-quit null read at SimCity 4.exe+0x1D0F94; raw verdicts/exceptions remain in causes.tsv.
Strict actions: 93/179 = 51.96%; 23070 accepted / 8794 failed actions; the strict 80% gate is NOT met.
Cause acceptance: build-runs/suite/sh28-census3/{causes,cause_groups,evidence_gaps}.tsv group all failures and qualified quit faults with raw evidence.
Remaining census harness causes: Souther Bay dialog-held; Magnolia unestablished; Port of Shiganshina window-exhausted; owner27-effects emergency calendar stall (6586.48s).
Retained prior harness fixes: size-scaled limits, save grace, dialog closing, isolated load paths, fixture repairs, and evidence-preserving verdict parsing.
Finding rows: three tier crashes at FreeCity4.dll+0xEDC7D8 reading 0x00005001; Jinae load AV at SimCity 4.exe+0x193CE1; emergency pause and churn calendar stall; not minimized.
Tier selection estimate: eight scenarios, 95.37% census union, 649s on six slots; this estimate did not qualify the set.
Real tier acceptance FAILED: sh28-tier1-r5 returned 8/8; 3 clean/complete, 1 corrupt, 3 exited, 1 stalled; only 3 reach traces.
Measured tier: 1322179/1546094 census-union implemented bytes = 85.52% (target 95%); six-slot wall 2333.297s (limit 900s).
tier1 remains unmarked; build-runs/suite/sh28-tier1-r5/{health.json,causes.tsv,finding_candidates.tsv,coverage-report.md} retain the failed measurement.
python tools/coverage.py --suite sh28-census3 printed exercised 1546094 bytes (27.7%), verified 329798 (5.9%), replaced 0; docs/coverage-report.md refreshed.
python tools/coverage.py --suite sh28-tier1-r5 --out build-runs/suite/sh28-tier1-r5/coverage-report.md printed exercised 1322999 (23.7%), verified 412296 (7.4%), replaced 0.
Shadows are not all exact: census 279185/285410; tier 9213/9774.
Final tooling gate: python -m pytest tools/tests -q printed 542 passed in 128.88s (0:02:08), exit 0; earlier round outputs remain separate.
One final python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> all printed cargo 7889 PASS, clippy 0 diagnostics, build 50/50 edges, CTest 124/124, qbuild PASS.
Real fixture parse printed: suite.tsv: 180 valid rows; 13 findings; 0 marked tier1. No saves or third-party DLLs committed.
Shared queue only, parallel=1, at most six owned requests outstanding this round; Documents regions remained read-only.
Missing: repair the four remaining harness causes and qualify a clean 95%/900s tier; tooling is ready to land with those limits recorded.
Next: coordinator lands tools, assigns the registered engine/load findings, then remeasures tier1 after fixes.
