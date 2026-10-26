| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suitecov27: suite coverage and fast tier; tools | Codex / suitecov27 worktree / detached HEAD, reserved branch suitecov27 | READY to land tooling per final-round instruction; pytest 483; Rust 7848; Clippy 0; CTest 118/118; census/tier1 incomplete; shadows not exact; ABI unchanged; questions none | 1. Supervisor publishes detached tip and lands checked queue entry. 2. Follow-up completes census and measures tier1 after game failures are repaired. |

Code commits: b64c50321, eb4f5a16a, 2fa40c640, 1cbe3c784, e03ebd87e, 1c19e536f. Scenario fixtures changed; engine implementation, ABI exports, coverage-map.tsv and regress.ps1 did not.
Acceptance 1 PARTIAL: census-r2 plus round2-missing/repairs tags preserve original and corrected-source attempts; the current suite has 150 rows. Round 3 submitted no games.
Audited shadows: 134/150 distinct rows returned, 16 unsubmitted, 21/150 with traces, 4/150 with complete reach/end/action evidence. Returned attempts: corrupt=49, exited=24, no-load=6, clean=4, invalid=16, stalled=41.
`python build-runs/audit-suitecov27-r3.py` printed owned queued/running=0/0 in both shared and worktree queues. All claimed games drained; Documents saves were never written.
`suite_coverage.py suitecov27-r2-observed --select` printed union 1,339,131 / 3,361,604 implemented bytes (39.8361%), 5,304 functions; incomplete evidence makes this a lower bound.
Per-set lower bounds: full 1,090,941 bytes (32.4530%), gen 1,223,492 (36.3961%); smoke has no measured trace. Per-scenario and set counts: build-runs/suite/suitecov27-r2-observed/reach_scenarios.tsv and reach_sets.tsv; absent counters are `-`.
Acceptance 2 NOT MET: candidate cover reaches 1,238,014 bytes / 92.4491% of observed union, packed wall 725s on six slots; achieved=false, eligible_for_tier1=false. This is a packing estimate, not a measured tier run.
Selected candidates: lua-lots-empty, gen27-tegel-s0-grow, gen27-tegel-s0-edge, berlin-tempelhof. No tier1 marks or shortened dates were committed; full-suite 98% reach and both-profile <=900s measurements remain missing.
Actual batch wall including queue: round2-repairs=410.828s/6 slots; round2-missing=1952.719s/6 slots (incomplete); round2-replace=149.235s/4 slots. None establishes the tier1 objective.
Acceptance 3 PASS for tooling: --suite adds suite-exercised/verified/replaced beside the legacy four, by function bytes, area and subsystem; raw logs, per-key counts, reach files and requests are checked and hashed.
Verification follows exact comparisons and zero resyncs in every observed key run; later game failure does not erase comparisons. Missing recorded comparisons void a key; unsubmitted rows do not invent key runs. Replacement requires all enabling runs clean and complete.
Real two-profile tag suitecov27-r2-measured combines census-r2, round2-missing, round2-repairs, observed-replace and round2-replace, with bundled source files, summary.tsv and per_key.tsv.
Round 3 real-input command: `python tools/coverage.py --suite suitecov27-r2-measured --out build-runs/suite/suitecov27-r2-measured/coverage-final-r3.md` printed suite-exercised=1,339,131 bytes (24.0% game), suite-verified=364,071 (6.5%), suite-replaced=0 (0.0%).
The same command retained legacy specified=89.2%, implemented=60.2%, verified=30.8%, replaced=27.8%. Earlier incomplete tags, failed attempts and corrected-source tags remain separate.
Replacement evidence: 9/150 distinct rows returned, 6 clean/2 stalled/1 corrupt, 5 complete; 202 actions ok/12 failed. Failure in any enabling run prevents replacement credit, hence zero suite-replaced bytes.
Census shadows 78,355/80,483 exact (not exact); 6,049 actions ok/2,680 failed. Reach-on and reach-off runs remain separate; the census and candidate tier cannot be called passing.
Failure triage is in docs/suitecov27-triage.md and suitecov27-r2-measured/tier1-blockers.tsv. Load detection and mayor-mode fixtures were repaired; stale-DLL preflight, six-request bound, stop-and-drain and duplicate-free resume are tested.
Acceptance 4 PASS: final `python -m pytest tools/tests -q` printed `483 passed in 148.19s (0:02:28)`. The q.py attempt failed before pytest with Bash CreateFileMapping error 5; direct invocation passed.
One final `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> all` printed test exit=0/passed=7848, clippy exit=0/diagnostics=0, native build PASS 46/46, ctest PASS 118/118, `qbuild: PASS` (exit 0).
Gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log. No additional game or gate was started after these final checks.
Acceptance 5 LANDING: ready merge-queue entry uses the full detached HEAD and checked enqueue credit; supervisor must publish the reserved branch. Ready means tooling is available to land; acceptance 1/2 remain incomplete.
Next: coordinator schedules follow-up for reach-on crashes/stalls, the 16 unsubmitted rows and missing traces; then measures and marks a qualifying tier1 and runs it in both profiles. This final round starts no new work.
