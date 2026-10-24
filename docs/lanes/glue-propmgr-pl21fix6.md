| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-pl21fix6: PL21 capture and shutdown repair | Codex; glue-propmgr-pl21fix6 (detached HEAD) | ready; main454438d47 merged at2c1ab60e1; fix dede60ae0; cargo6935 passed, clippy0, CTest77/77; fixed house pairs A/B PASS0, propmgr440/440 and lotcfg586/586 exact, ref0; ABI unchanged | Coordinator publishes detached tip and lands queue entry |

## Round 3 final report

Merged requested main **454438d47** into reserved detached HEAD at **2c1ab60e1**, starting from c46bbdeae. The merged DLL reproduced the train's shutdown heap AV in **both** standard house profiles. [Baseline record 423](../runs/pl21r3-423.md): replace112.609 s, shadows660.078 s, each corrupt/foreign_exceptions1/ref_warnings0. Prop/lot shadows still compared exactly, 220/220 and293/293. The baseline DLL/PDB remain under `build-runs/dlls/pl21fix6-r3-gated.*`, DLL SHA256 `5306a43087f651c5eaa8e34d941a930f259e1117c6619e5eddbf0b96a03b67b8`.

Both failing logs have identical plugin stack offsets. Resolving only our own PDB gives `free → operator delete → GameHashState scalar deleting destructor → FreeGameHashTables → LotConfigManagerAppShutdown → PreAppShutdown`. `ConfigMapState()` and `FamilySetState()` return addresses of function-local static objects, but `FreeGameHashTables()` deleted those addresses. This is a concrete invalid free in the lot configuration replacement's process cleanup, also reached by the shadows-only house profile. The slot-40 capture is inactive in the failing replace profile. No WriteCapture or automata-owner change was needed or made.

**Fix dede60ae0:** retain the static states and reset their owned nodes/bucket arrays. `ProcessHashState::Reset` clears references, frees the correctly typed `void*[]` buffer and resets the exposed table; repeated reset and later static destruction are safe. Rebuild `Clear` retains the bucket storage. The growth path also uses matching `delete[] s.buckets`. The native lifecycle test exercises static address retention, chained map-reference releases, borrowed family values, rebuild, repeated reset, repopulation and destructor cleanup. No core behavior or C ABI export changed; the existing PL21 shared-input capture remains intact and default-off.

Stable final engine source passed focused x86 MSVC testing and **qbuild all**: cargo **6935 passed/0 failed/0 ignored**, clippy **0 diagnostics**, CMake **160/160 edges** (6.8 s), CTest **77/77** (77.3 s), native total85.9 s. Verdict: `build-runs/build-results/qbuild-20261024-060405-build-ctest.verdict`; full logs `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The tree-changed notice reflects committing the already-stable engine source and updating the lane file while the gate ran; no engine input changed during that operation. The preceding merged-source gate also passed, cargo6935/clippy0/CMake226/226/CTest76/76, verdict `qbuild-20261024-054612-build-ctest.verdict`.

Both final house repeats used the identical fixed DLL `build-runs/dlls/pl21fix6-r3-fixed.dll`, SHA256 **8b3b9fefcd616b981c26b9c7f448a5605d99887d3b6ebf1ee08d41ff14d3c26f**, with standard `tools/regress.ps1 -Queue` settings, Konradshohe, turbo, end_date2032-01-01. Direct comparison of generated ini files found no differences between baseline/A/B profiles.

- **[Repeat A, record424](../runs/pl21r3fixa-424.md):** `regress-pl21r3fixa-replace` clean136.843 s; `regress-pl21r3fixa-shadows` clean710.672 s; REGRESS PASS0, wall744 s. Prop mgr220/220, lotcfg293/293, mismatched0/incomplete0. Query probes73, captures1694, filter checks64166, input errors0.
- **[Repeat B, record425](../runs/pl21r3fixb-425.md):** `regress-pl21r3fixb-replace` clean141.031 s; `regress-pl21r3fixb-shadows` clean705.250 s; REGRESS PASS0, wall744 s. Prop mgr220/220, lotcfg293/293, mismatched0/incomplete0. Query probes73, captures1694, filter checks64901, input errors0.

All four final runs reached2032 and shut down cleanly with **foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0**. Final combined shadows: **propmgr440/440, lotcfg586/586**, 3388 captures,129067 filter checks,0 input errors. Full logs/verdicts are retained under `build-runs/game-results/regress-pl21r3{,fixa,fixb}-{replace,shadows}.*`; complete fixed-run console logs are `build-runs/pl21r3fixa-console.log` and `pl21r3fixb-console.log`.

Operations73963 (baseline),73784 (A),94349 (B), and both gates were collected through their actual terminal results. Unique labels were retained; no qwait, duplicate request or game launch. The q.py Bash wrapper failed with Win32 error5 before launching a request; the direct output-filter wrapper then ran the requested regress command once per tag. An initial focused cmd invocation used an unsupported forward-slash script spelling; the corrected invocation passed. No additional main-only DLL was run; train180's recorded main-only results remain the comparison supplied by the coordinator. The present evidence identifies and repairs the invalid static deletion, without claiming an explanation for every earlier train artifact.

**Questions: none. ABI changes: none. No game/build outstanding.** Ready queue points to the final detached HEAD; supervisor publishes it to the reserved branch. Coordinator can land the PL21 capture and shutdown repair. Historical round-2 and original implementation reports below describe the older base and are retained for provenance.

## Round 2 report

- Goal: fix train177/178/179 heap AV and stall, retaining propmgr 220/220.
- Starting tip: d44eead99c; detached HEAD, do not update refs.
- Audit: slot-40 hook installed only in PropQueries; replace mode disables prop shadow.
- Capture/replay each acquire one reference; comparison and input destruction each release one.
- No implementation changes; no causal defect proven in the three suspects.
- New diagnostic: regress-pl21fix-r2-diag-20261024a (both profiles via explicitly requested regress -Queue).
- Unchanged DLL f86ce9002955b95e10fe2658e67fed08daa72d81db7760b517e7ad35f63b1a59.
- Replace finished clean2032, foreign_exceptions/stalls/ref_warnings=0, 175.313 s.
- Shadows finished clean2032, foreign_exceptions/stalls/ref_warnings=0, 739.625 s; propmgr220/220, lotcfg293/293.
- Regress operation 96006 collected to terminal exit0; no game request outstanding.
- Both logs/verdicts copied into local build-runs/game-results; diagnostic run record 390.
- Regress PASS/0 problems but eight worse vs train176: missing newer observers and landvalue71/72.
- Main has train176 observer/write-capture changes absent from reserved HEAD; train179 included them.
- Both build caches are Debug/FC4_CHECKED=OFF (configuration difference ruled out).
- Async permission requested to merge main into detached HEAD to reproduce actual failing inputs; pending.
- Gate operation10802 collected to terminal exit0: qbuild PASS, cargo6891 passed/0 failed/0 ignored, clippy0 diagnostics, CMake202/202, CTest72/72.
- Native build13.6 s, CTest90.9 s, total108.3 s; sccache185 hits/0 misses. Verdict: build-runs/build-results/qbuild-20261024-053124-build-ctest.verdict.
- Full gate logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. No tree-change warning found.
- Gated DLL SHA256 f926c2dac765027f6ca5dc821bdefd4136c820cd07c9d7c191fdd1100813003c, saved as build-runs/dlls/pl21fix6-r2-gated.dll; not game-tested.
- Next: reproduce on the newer base if authorized; no speculative fix. No build or game operation outstanding.
- Required gate: qbuild all on stable final source, then both house profiles twice.
- Manifest: main build-runs/run4-glue-propmgr-pl21fix6-r2-manifest.json.
- Prior round report below is historical and does not establish round-2 acceptance.

No engine implementation changed in round 2, and no root fix is claimed. The reserved checkout predates train176's observer and write-capture changes; train179 tested those changes combined with this lane. The existing rule forbids merging main without instruction. An asynchronous request to authorize that merge into detached HEAD remains unanswered. The old diagnostic DLL ignores newer keys lotconf_shadow, lotrec_shadow, ordinance_shadow, simgrid_shadow and weather_shadow. The missing summaries and landvalue71/72 result (six mismatches on an unsettled September2026 pass) are preserved, not waived. This diagnostic is not final acceptance, and the required two repetitions of both complete final-source house profiles have not been performed.

The three-suspect audit found no normal-path reference imbalance: capture retains each entry, replay retains another reference for the core, pruning/comparison releases the replay reference, and snapshot destruction releases the capture reference. Native list entries have separate references that are released and nulled before the SDK destructor. Replacement mode disables the prop shadow before any probe installation or snapshot allocation, so the null global owner cannot release host objects in that mode. Scope cleanup and outer SEH cleanup restore slot40 with conditional exchange. No recorded evidence establishes a restoration failure. These findings do not prove every exceptional or concurrent path safe and do not identify the first corrupting operation. The supplied heap AV occurs after PostAppShutdown without a DLL return address on the recorded stack; that establishes detection, not causation.

The explicitly requested regress script submitted both diagnostic profiles concurrently; its original session96006 was collected to terminal exit0, without qwait or resubmission. The result is recorded in [run390](../runs/pl21fix-r2-diag-20261024a-390.md); full logs/verdicts are local under build-runs/game-results. The full gate ran once on stable unchanged engine source to check the reserved source/artifact inputs. Cached compiler diagnostics mention train-integration filenames, but the build command and CMake home directory point to this lane. Keep the historical diagnostic DLL and the newly gated DLL distinct.

Questions filed: none; no new behavioral interpretation was implemented. ABI exports changed: no. No shared ref was switched or published. The queue remains WIP; this lane is not ready. Next: supervisor authorize merging main into detached HEAD or supply exact train179-based inputs (failing DLL hash prefix83c80323; the train-integration artifact has since been replaced by train180). Reproduce, diagnose the first corrupting operation, implement its root fix, then gate changed source and run both full house profiles twice before marking ready.

## Round 1 historical report (not round-2 acceptance)

PL21 is applied to the shadow's input boundary. The prop-manager record has no prop entries and does not freeze the occupant grid. Two separate live queries therefore did not establish equal query inputs. `FC4PropLotShadow.cpp` now observes the occupant manager's public slot 40 during the native lot query, retaining its ordered result and positions before the prop manager's final position-pruning pass. Rust receives that same dependency input and performs its own complete lot-query algorithm. Native `GetLotProps` output is never replayed.

The forwarding filter independently checks the native owner's predicate decisions against the documented type/interface/usage-2 predicate. Unexpected query arguments, capture/replay counts, filter decisions and position lookups are counted as input errors and contribute to the shadow mismatch count. The final result flag and ordered occupant list still compare exactly. There are no timer, visibility, pending-save or persistence-based exclusions, sorting or deduplication. Historical intermittent failures are not reclassified or suppressed. Snapshot references are acquired and released explicitly; cleanup also restores the forwarding slot and releases retained inputs across the outer SEH boundary. The key remains default-off.

The Rust manager already implements PL21's grid-only appends, whole-caller-list position pruning and retention of duplicate or other-usage caller entries. Two focused tests add PL21-U/C/R/W and PL21-S coverage, including reference accounting and an unchanged manager record across a later grid insertion. No core behavior or C ABI export changed. Implementation commits: `39766bb9c` (tests and diagnostics), `1dc38c56e` (common query inputs).

Validation:

- Focused PL21 tests: 2 passed, 0 failed; explicit owned-file formatting check passed with zero differences.
- Stable-source `qbuild all`: **PASS**, cargo summed across all test-result lines **6891 passed, 0 failed, 0 ignored**; clippy **0 diagnostics** with warnings denied; CMake **91/91 edges**, CTest **72/72**, native total **86.2 s** (CTest 77.7 s).
- Native queue verdict: `build-runs/build-results/qbuild-20261024-032946-build-ctest.verdict`. Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Earlier diagnostic verify also passed (233/233 edges, 72/72 CTest), with a tree-changed warning because focused tests were added during that diagnostic build. It is separate from the final stable-source gate. Initial Bash wrapper startup failed with Win32 error 5 and initial relative fmt path failed; direct focused cargo and corrected absolute-path fmt passed.

All final repeats used identical DLL SHA256 `f86ce9002955b95e10fe2658e67fed08daa72d81db7760b517e7ad35f63b1a59`, Konradshohe, turbo, end date 2032-01-01 and the complete `regress-train172-shadows.ini` house settings. A direct settings comparison was equal after omitting the city path supplied by the harness. Each run ended **clean**, with **ref_warnings=0**, rtc=0 and no incomplete lines. Exact measured lines and records:

- **pl21fix6-house-r1-20261024-b 220/220** — [run 387](../runs/pl21fix6-house-r1-20261024-b-387.md), 649.265 s; 73 query samples, 1694 captures, 64293 independently checked filter predicates, inputErrors=0; lotcfg 293/293.
- **pl21fix6-house-r2-20261024-c 220/220** — [run 388](../runs/pl21fix6-house-r2-20261024-c-388.md), 719.844 s; 73 query samples, 1694 captures, 65928 independently checked filter predicates, inputErrors=0; lotcfg 293/293.
- **pl21fix6-house-r3-20261024-d 220/220** — [run 389](../runs/pl21fix6-house-r3-20261024-d-389.md), 617.656 s; 73 query samples, 1694 captures, 63641 independently checked filter predicates, inputErrors=0; lotcfg 293/293.

Each propmgr totals line is `lines=220 exact=220 mismatched=0 incomplete=0`. Across the three final repeats: **660/660 exact lines**, **5082 captured dependency queries**, **193862 filter checks**, **0 input errors**. Requests were unique and submitted one at a time; waits used a local directory-change watcher without qwait. Nine-minute waits for the diagnostic and repeat 1 expired before their eventual valid verdicts; those same unique labels were retained and collected, without duplicate game requests.

The pre-change diagnostic [run 386](../runs/pl21fix6-diag-20261024-a-386.md), `pl21fix6-diag-20261024-a 220/220`, was also clean2032/ref0 (671.110 s; old DLL SHA prefix `9b041156`). It did not reproduce the earlier intermittent failure. The evidence establishes exact comparison on common observed dependency inputs; it does not establish a causal reconstruction of train169/train171's past failures or deterministic evolution between separate games.

Questions filed: **none**; PL21 supplies the required contract. ABI changes: **none**. No game request or build remains outstanding. The coordinator owns coverage credit and any addition to the existing single shadows string in `tools/regress.ps1`; clean-room lane restrictions and the detached-HEAD instruction leave those tools/main actions to the coordinator. No tools files, coverage tables, main refs or ABI version were changed.
