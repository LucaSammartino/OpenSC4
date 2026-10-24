| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-ex56apply11: EX56 registered-file shadow | `Codex` / `glue-dbpf-ex56apply11` (engine lane, detached HEAD) | Ready for measured scope: cargo 7119/0/0, clippy clean, CTest 81/81; runs 492/493 clean, dbpf 73/73 exact each, ref_warnings=0; ABI exports unchanged; EX57 open | Coordinator review scope for coverage credit; spec owner answer EX57 for transient files. |

## Changes

Applied data-formats section 8.3 (EX56), already merged at 4761ef152. Resume's required merge reported already up to date. Git also emits a packed-refs.lock permission diagnostic in this detached worktree; ordinary commits succeed and shared refs were not changed.

- Explicitly identify the current city backing path among every registered packed root and recursively visited multi-packed child. Log discovered/selected/compared city counters and file classes. If the path is missing, add a separate read-only packed-file reader; failures become refusals rather than coverage.
- Preserve initial full sweep followed by one rotating file per month, with city prioritization after save. The summary retains `dbpf shadow YYYY-MM-DD: ... mismatches=N` and reports `scope=registered-packed-roots+multi-children+city-backing transient=unobserved`.
- Apply EX56 indexed payload return/size, null-buffer, short-prefix and missing-key checks. Preserve borrowed traversal ownership, acquired-QI teardown, and created-reference teardown for the fallback reader.
- Revalidate path/open state/file metadata between slices. PreSave releases pending views before city re-registration, advances a generation and schedules city rediscovery. Invalidated or unfinished samples report refusals/nonzero mismatches; they are not silently discarded.
- `dbpf_shadow` stays default-off. No C ABI export/version change, generated ABI file change, or tools/ edit. The supplied house profile already enables the key; coordinator owns any further regress profile edit.

Implementation milestones: 8003d6ddb (EX56 implementation), 7e2d98267 (created-reference teardown), 1f8b8b030 (PreSave ownership), e73dbdebb (SDK return-channel annotation), a4e0d13ef (house record), d17f20344 (final gate and save record). Engine source has not changed since e73dbdebb.

## Validation

Final stable-source `qbuild.py all`: cargo **7119 passed, 0 failed, 0 ignored across 45 suites**; clippy **0 diagnostics**; native build **46/46 edges**, CTest **81/81** (116.8 s), native total 124.9 s. Terminal verdict: `build-runs/build-results/qbuild-20261024-141831-build-ctest.verdict`; Rust/clippy evidence: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`. Transport restart lost process session 70829; its existing terminal PASS verdict was collected instead, without starting a duplicate check.

Explicit queued `qbuild.py verify` also passed: request `qbuild-20261024-132817-build-ctest`, build 47/47, CTest 81/81, total 106.6 s. Archived logs: `build-runs/evidence-run4/ex56-verify-{build,ctest}.log`. Earlier failed all operation is retained separately as `qbuild-20261024-123328-build-ctest.verdict` and `build-runs/evidence-run4/ex56-first-{build,ctest}.log`: CTest 80/81, return-channel checker rejected an unreviewed fallback Open. Corrected SDK annotation passed focused review (0 flagged/78 reviewed) and both later native gates. No MSVC configure/build was launched directly.

## Real-city measurements

`dbpf-ex56-house-492 73/73`

[Run 492](../runs/dbpf-ex56-house-492.md): full supplied house switches, turbo to 2032, clean shutdown at 2032-01-03, **916.234 real seconds / 932 wall seconds**, under 1800 s; `ref_warnings=0`, exceptions/stalls/rtc/overruns=0. Eleven packed files (10 roots plus 1 child); current city backing directly registered and compared seven times, 187 records each. Initial full sweep 111826 records; cumulative 876020 records; rotation cycle 6 covered 11/11 files and 111826/111826 records. Maximum slice 828 ms; refusals=0; every shadow line mismatches=0. DLL hash prefix ec17f769.

`dbpf-ex56-save-493 73/73`

[Run 493](../runs/dbpf-ex56-save-493.md): focused normal-speed save/re-registration run, clean at 2032-01-02, **236.328 real seconds / 251 wall seconds**; `ref_warnings=0`, exceptions/stalls/rtc/overruns=0. Scripted full SaveCity at 2029-06-15 succeeded in 11156 ms. City compared four times before save (187 records) and four after re-registration (153 records); generation 0 to 1. All 73 samples discovered its path. Cumulative 876071 records; rotation cycle 6 covered 11/11 files and 111792/111792 records after save. Maximum slice 921 ms; refusals=0; every shadow line mismatches=0. Same DLL ec17f769.

Run records link the retained verdict/log evidence. Exact/total counts were extracted from every dated DBPF summary line using the lane helper; clean means `mismatches=0`. No tools/coverage.py or other tools/ scripts were read or run. Both runs are terminal; no build or game request remains outstanding.

## Remaining scope limits and questions

1. **EX57**, appended to [data-formats questions](../specs/questions/data-formats.md): section 8.3 identifies temporary load/save, obliteration and regional-city segments outside resource-manager traversal, but supplies no typed lifecycle observation/stable-read callback contract. These files remain explicitly **unobserved**. This lane cannot claim an all-open-files census or transient-file coverage; the coordinator must limit any map credit accordingly.
2. Samples cover stable registered packed roots/children and the current city backing path. Directory/streaming/nonpacked and closed roots are counted separately and excluded; observed counts were zero in both runs. Unregistered files other than the current backing-path fallback are outside discovery.
3. Monthly rotation is temporal sampling, not a simultaneous snapshot: one initial full set, then one file per month. Discovery and individual indexed reads do not lock an entire sweep. Metadata and PreSave generation checks cannot establish atomicity or detect every unannounced same-path close/reopen/write/compact transition; such lifecycle contracts remain outside the claim.
4. The fallback reader is implemented but was **not exercised**: the city was directly registered in both runs and already part of the eleven files. This establishes explicit city evidence, not an expanded eleven-file census. Missing-path, failed-open and invalid-interface branches have no live-city coverage.
5. The save run proves stable city comparisons before and after successful re-registration; it does not sample temporary save files or force a save precisely while a comparison is pending. Save invalidation/refusal branches are not claimed as exercised.
6. This is a read-only indexed raw/payload comparison. It gives no write/delete/compact, access rejection/write-open conflict, or existing record-object cursor/reference/lifetime/noninterference credit. Malformed/failed-input vectors and failure return branches were not induced in the live city.
7. Live reads refuse records over **256 MiB**. No such refusal occurred. Refusals, deadline interruptions and detected changes make a summary non-exact; zero refusals in these runs is not evidence for those recovery branches. Empty-record count was zero in the house run.
8. No additional exemplar/cohort/resource-manager precedence or other data-format map rows are measured by this DBPF extension. Existing conformance results do not imply live coverage of every EX56 vector.

## Next steps

Ready to land the measured registered-file work. Coordinator reviews these limits before assigning map credit and owns coverage-table/regress-profile edits. Spec owner supplies EX57 contracts before any transient-file implementation. No further action is pending for the measured scope.
