| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| suitecov27: suite coverage and fast tier; tools | Codex / suitecov27 worktree / detached HEAD, reserved branch suitecov27 | WIP; pytest 442 passed; census 66/149 results, 11 traces; shadows not exact; ABI unchanged; no questions | 1. Repair reach/load/Lua failures and collect all rows with a validated DLL. 2. Measure and mark tier1, run both profiles on six slots, then enqueue. |

Code commits: b64c50321, eb4f5a16a, 2fa40c640, 1cbe3c784. Engine, ABI exports, coverage-map.tsv and regress.ps1 were untouched; qbuild/cargo/clippy/ctest n/a.
Acceptance 1 PARTIAL: `suite.py run --set all --profile shadows --reach --slots 12 --tag suitecov27-census-r2 --dll <reach27 DLL>` staged 149 rows; resumed with unclaimed seconds=240/timeout=300.
Final census summary: 66 returned results, 83 not-submitted; clean=1, corrupt=28, exited=13, unknown=2, no-load=6, invalid=9, stalled=7. Only 11 reach files; 55 completed attempts have none.
Shared queue plus permitted slots 9-12 through a separate -Once queue; that queue exited 0, all claimed requests drained, zero requests left waiting. Documents saves were never written.
`suite_coverage.py suitecov27-observed-r2 --select` printed union 1,239,511 / 3,361,604 implemented bytes (36.8726%), 4,965 functions; these are lower bounds from incomplete evidence.
Per-set lower bounds: full 453,104 bytes (13.4788%), gen 1,193,917 (35.5163%), smoke 0; per-scenario counts and saturation candidates are in reach_scenarios.tsv, per-set counts in reach_sets.tsv under that tag.
Acceptance 2 NOT MET: greedy selected berlin-tempelhof, 369,613 bytes / 29.8193% of observed union, packed wall 111s; eligible_for_tier1=false, 83 missing scenarios and 55 missing completed traces.
No tier1 rows or shortened end dates were committed; the 98% full-suite reach and six-slot <=900s tier measurements are still missing.
Acceptance 3 IMPLEMENTED: --suite adds suite-exercised/verified/replaced beside the legacy four, by function bytes, area and subsystem; raw logs, per-key counts, reach files and requests are checked and hashed.
Verification follows exact comparisons and zero resyncs in every observed key run; later game failure does not erase comparisons. Missing recorded comparisons void a key; unsubmitted rows do not invent key runs. Replacement requires all enabling runs clean and complete.
`suite_coverage.py suitecov27-metric-r2 --combine suitecov27-observed-r2 suitecov27-observed-replace` made a real 67-run two-profile tag with bundled inputs, summary.tsv and per_key.tsv.
`coverage.py --suite suitecov27-metric-r2 --out build-runs/suite/suitecov27-metric-r2/coverage-final.md` printed suite-exercised=1,239,511 bytes (22.2% game), suite-verified=455,731 (8.2%), suite-replaced=1,550,448 (27.8%).
Authoritative metric output: build-runs/suite/suitecov27-metric-r2/coverage-command-final.out; earlier definitions/results are retained separately, including coverage-command.out.
Replace proof is one load-only Tempelhof run: clean/end=1/actions=0 failed; city wall 76s, six-slot-limit batch wall including queue 305.859s. This is not a measured tier1 set.
Census shadows 34,905/35,788 exact; not exact. Konradshohe reach-enabled probes were corrupt at terminal disasters (00508317); same-DLL reach-off control was game-clean, 18 actions ok/11 refused, shadows 556/573. Cap=1 probe stalled.
Main's default DLL was dated Oct 21 and ignored reach; that abandoned suitecov27-census-full and the separate probe tags remain distinct. New preflight rejects DLLs lacking the tracer before submission.
Acceptance 4 PASS: `python -m pytest tools/tests -q` printed `442 passed in 112.04s`; full log build-runs/pytest-suitecov27-final3.out. Earlier final2 had a 20s PowerShell fixture timeout (440 passed/1 failed), retained separately; final3 ran after games drained.
Tests cover weighted bytes, wall packing, 98% selection, missing/contradictory evidence, every-run mismatches/resyncs, replacement failures/defaults, marked tiers, stale DLLs and duplicate-free resumption.
Acceptance 5 WIP: local merge-queue record holds this lane; train enqueue deferred because cmd_enqueue unconditionally marks an unrefused entry ready while acceptance 1/2 remain unmet.
Next: fix reach-enabled crashes/stalls and Lua/load failures upstream; use a new tag with a freshly validated DLL for corrected-source evidence, complete all 149 rows, measure shortened tier candidates, mark tier1, run both profiles on six slots, and enqueue the final tip.
