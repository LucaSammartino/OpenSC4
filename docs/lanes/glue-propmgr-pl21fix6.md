| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-pl21fix6: PL21 round 2 crash investigation | Codex; glue-propmgr-pl21fix6 (detached HEAD) | in progress | Audit ownership and shutdown; full gate and two complete house-profile repeats |

## Handoff

- Goal: fix train177/178/179 heap AV and stall, retaining propmgr 220/220.
- Starting tip: d44eead99c; detached HEAD, do not update refs.
- Audit: slot-40 hook installed only in PropQueries; replace mode disables prop shadow.
- Capture/replay each acquire one reference; comparison and input destruction each release one.
- No implementation changes; no causal defect proven in the three suspects.
- New diagnostic: regress-pl21fix-r2-diag-20261024a (both profiles via explicitly requested regress -Queue).
- Unchanged DLL f86ce9002955b95e10fe2658e67fed08daa72d81db7760b517e7ad35f63b1a59.
- Replace finished clean2032, foreign_exceptions/stalls/ref_warnings=0, 175.313 s.
- Shadows remains active; collect existing exec session 96006 to terminal; do not resubmit.
- Results currently in MAIN/build-runs/game-results/regress-pl21fix-r2-diag-20261024a-*.verdict.
- Main has train176 observer/write-capture changes absent from reserved HEAD; train179 included them.
- Both build caches are Debug/FC4_CHECKED=OFF (configuration difference ruled out).
- Async permission requested to merge main into detached HEAD to reproduce actual failing inputs; pending.
- Next: collect diagnostic shadows, then reproduce on newer base if authorized; no speculative fix.
- Required gate: qbuild all on stable final source, then both house profiles twice.
- Manifest: main build-runs/run4-glue-propmgr-pl21fix6-r2-manifest.json.
- Prior round report below is historical and does not establish round-2 acceptance.

## Report

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
