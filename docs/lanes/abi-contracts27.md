| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| abi-contracts27: run-7 entry contracts (tools) | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/abi-contracts27; detached HEAD, reserved branch abi-contracts27 | wip; code commits 69f4208ce, 479cb2d7c, ad03d35ff; ABI/seams unchanged; pytest 249 passed/6 failed; ABI fixtures 17 passed; game/shadow n/a; questions none | 1. Rerun the full tooling suite where Node can launch child processes, then mark the queue ready if it passes. |

Date: 2026-10-25. Added standard-library extraction/validation, shared return rules, reviewed direct-call input, synthetic byte fixtures, and ABI documentation.
Output: docs/abi/functions.tsv has 35,648 rows: all 35,637 exported functions plus 11 vtable-only targets (size 0, explicitly missing export ranges).
Real input: `python tools/abi_functions.py --evidence build-runs/abi-functions/accepted-evidence.json` printed `loaded functions=35637 vtable_only=11 instructions=2193251` and `wrote rows=35648 seconds=68.958` (limit 600 seconds).
Validation: `python tools/abi_validate.py --evidence build-runs/abi-functions/accepted-evidence.json --report build-runs/abi-functions/accepted-validation.json --sample build-runs/abi-functions/post-review-sample.json` exited 0; `schema_errors=0`.
Criterion 1: 8,114 normalized vtable targets covered; 28,724/28,790 rows strictly agree, 66 explained, 0 unexplained (all 8,114 targets accounted for).
Vtable exceptions: 006c2040 (65 rows) and 006c2050 (1 row) have conflicting eax/eax-or-void source labels; high-confidence eax retained, with both labels in notes. Pops agree throughout.
Criterion 2: all 29 complete direct-calls.md contracts checked (including inline insert_unique); 27 strict, 2 explained, 0 unexplained.
Direct exceptions: 0057eb00/0057eb70 specify semantic EAX returns while vtables.tsv says eax-or-void; retained vtable channel plus explicit direct semantic notes.
Criterion 3: all 30 S3 contracts at 77d2819fa agree: 20 RVA/convention/arg_bytes rows mapped to VA/pop/args, plus 10 VA/pop/args rows. The CSVs have no return field; the second has no convention field.
Criterion 4: 10,222/10,636 non-unknown Ghidra conventions agree; all 414 disagreements classified: 262 no observed ECX, 53 additional EDX, 75 no stack arguments/unresolved pop, 13 nonstandard registers, 11 observed register arguments.
Criterion 5: seed 20261025 selected 50 from 3,765 pre-correction cdecl candidates with direct callers; hand checked body and caller evidence for all 50; final count agreement 50/50.
Manual review corrected reserved value-argument construction and false cleanup attribution to a zero-argument function (00913c72 now unknown by the requested convention rule); selection was retained, not resampled after fixes.
Independent source bounds match manual inspection for body 45/50 and caller 47/50; eight partial cases involve indirect/SEH frame effects or split/shared cleanup. Both bounds and uncertainty remain explicit; final maxima agree for all 50.
`python build-runs/final-abi-metrics.py` printed `MANUAL 50 counts 50 body bounds 45 caller bounds 47 reclassified ['00913c72']`.
Synthetic check: `python -m pytest tools/tests/test_abi_functions.py -q` printed `17 passed in 0.06s`.
Required initial `python -m pytest tools/tests -q` printed `6 failed, 246 passed in 145.60s`; final `python -m pytest tools/tests -q --junitxml=build-runs/abi-functions/pytest-final.xml` printed `6 failed, 249 passed in 64.62s`.
All six failures are WorkflowDryRun cases in unchanged test_laneguard.py: diagnostic trace is `spawnSync C:/WINDOWS/system32/cmd.exe EPERM` before the mock gate can run. No failing tests were skipped or weakened.
The q.py wrapper separately failed before launching pytest (Bash CreateFileMapping error 5); the direct pytest runs above are the actual suite results.
Limits: 472 unresolved pops, 1 incomplete decode; confidence exact/high/medium/low = 8073/4492/10882/12201. Ambiguous/low-confidence rows require review before binding; this ledger is not replacement certification.
Raw evidence: build-runs/abi-functions/{accepted-evidence.json,accepted-validation.json,reviewed-sample.json,cdecl-manual.tsv,manual-results.json,pytest-final.xml,pytest-workflow-diagnosis.txt}; per-function original evidence under cdecl-review/. Earlier passes retained separately.
No engine/plugin changes, no C ABI export changes, no ABI version/coverage-definition changes; cargo/clippy/ctest/qbuild and game runs n/a. No questions filed.
Next: coordinator reruns the six workflow cases/full suite in an environment allowing Node child-process launches; queue remains wip until the gate passes.
