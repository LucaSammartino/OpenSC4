| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| propmgr-query27: closed lot query bounds (engine) | Codex; .claude/worktrees/propmgr-query27; reserved branch propmgr-query27, detached HEAD | ready; code 23e3dbdc4/e00309bec; ABI/seam unchanged; cargo 7860/0, clippy 0 diagnostics, CTest 118/118; four full house shadows propmgr 220/220, zero resyncs; questions none | - |

- Contract/fix: A7.2 / PL24 requires closed upper X and Z bounds; four ordered binary32 comparisons reject NaN X/Z or bounds, ignore Y, preserve list order and release rejected references.
- Code milestones: 23e3dbdc4 fixes the position pass and Rust fixtures; e00309bec aligns C ABI assertions and the PLT-04 conformance vector. No ABI export or seam change.
- Regression fixtures: observed kind-0/flags-7 props on each upper bound, adjacent floats, all ten T.4 rows, caller entries, NaN bounds/Y, signed zeros, infinities and references; all three PL24 tests pass (`build-runs/qbuild-test.log`, core 5192/0).
- Full gate, run once this round: cargo 7860 passed/0 failed/0 ignored (`build-runs/qbuild-test.log`); clippy exit 0, diagnostics 0 (`build-runs/qbuild-clippy.log`).
- Native gate: build PASS 52/52 edges (`build-runs/fc4verify-build.log`); CTest PASS 118/118, 0 failed (`build-runs/fc4verify-ctest.log`); qbuild all PASS.
- Owned-file fmt: manager.rs, core tests.rs and capi prop_manager_tests.rs checked; 3 files, 0 diff lines.
- House profiles train253 and train260 compare identical; all four completed runs use the shared queue, Konradshohe, turbo=1, end_date=2032-01-01.
- Game evidence directory below: `C:/Users/tlori/FreeCity4/build-runs/game-results/`; each label has its `.log` and `.verdict`.
- `propmgr-query27-r3-closed-a1`: clean, date 2032-01-10, ref_warnings=0; propmgr 220/220, lotcfg 293/293. One recovered watchdog pause, outstanding problems 0.
- `propmgr-query27-r3-closed-a2b`: clean, date 2032-01-07, ref_warnings=0; propmgr 220/220, lotcfg 293/293.
- `propmgr-query27-r3-closed-a3`: clean, date 2032-01-11, ref_warnings=0; propmgr 220/220, lotcfg 293/293.
- `propmgr-query27-r3-closed-a4`: clean, date 2032-01-07, ref_warnings=0; propmgr 220/220, lotcfg 293/293.
- Each of these four full runs has 73 query samples, every sample pairs=160/exact=160/different=0/fallback=0/resync=0; May 2029 listsDiffering=0; no differing-entry diagnostics.
- Other keys: all 69 other harness shadow summaries exact, resynced lines=0 in every verdict. Versus train260, the sole mismatch-count change is landvalue 1 to 0; variable simulation sample counts differ (lessons 5.3), with no new mismatch.
- Game DLL SHA256: ae5daf0ec21400d621ad73d7bb3d9a11037fce032e8051e91ece20946f562533. The full gate relinked it to 1350e97130f74585d8a798b394ac90ed0603c95d9878b4d9878e83f315e1e9fe; production source is unchanged since the four runs.
- Excluded failed load: `propmgr-query27-r3-closed-a2` stalled before a city date; a2b is its fresh-label successful retry, not a corrected verdict for a2.
- Causal diagnostic: `propmgr-query27-diag-a2.log` at 2031-12-01 records native-only kind=0, flags=7, exact upper Z=768, added_same_tick=0/removed_same_tick=0; PL24 specifies retention.
- Questions: none new; PL24 specifies the changed behavior. Ready queue entry pins the actual final detached HEAD; no coverage or replace-list edit.
