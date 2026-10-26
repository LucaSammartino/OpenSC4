| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| propmgr-query27: prop query diagnostics (engine) | Codex; .claude/worktrees/propmgr-query27; reserved branch propmgr-query27, detached HEAD | BLOCKED PL24; diagnostics c599ed479, test 1ea62a4ad; ABI/seam unchanged; Rust 7849 passed, clippy 0 diagnostics, CTest 118/118; propmgr-query27-r2-diag-b1 through b4 clean, each 220/220/ref0/resync0, each watchdog_stalls=1 | 1. Resolve PL24. 2. Obtain three differing samples and implement the specified fix. 3. Repeat four full exact runs and enqueue. |

- Change: added `pl24_untyped_lot_props_at_observed_upper_bounds`, kind 0/flags 7, both upper axes, adjacent floats and release checks; query semantics remain A7.2 strict bounds.
- Unit test passed in `build-runs/fc4verify-quick-test.log` and `build-runs/qbuild-test.log`; owned-file fmt passed.
- Full gate ran once: Rust 7849/0 (`build-runs/qbuild-test.log`), clippy 0 diagnostics (`build-runs/qbuild-clippy.log`), CMake PASS and CTest 118/118 (`build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`).
- Game log root: `C:/Users/tlori/FreeCity4/build-runs/game-results/`; immutable DLL SHA256 prefix 64c299c4; house profile from regress-train253-shadows.ini, four shared-queue slots 7/8/13/14 concurrently, turbo to 2032.
- `propmgr-query27-r2-diag-b1`: clean, propmgr 220/220; totals at `.log:40921`, verdict in `.verdict`.
- `propmgr-query27-r2-diag-b2`: clean, propmgr 220/220; totals at `.log:41138`, verdict in `.verdict`.
- `propmgr-query27-r2-diag-b3`: clean, propmgr 220/220; totals at `.log:41023`, verdict in `.verdict`.
- `propmgr-query27-r2-diag-b4`: clean, propmgr 220/220; totals at `.log:41057`, verdict in `.verdict`.
- Four consecutive full exact runs: observed; resyncs=0, incomplete=0, ref_warnings=0 in all four logs/verdicts. Each verdict also reports watchdog_stalls=1; no cause inferred.
- Other-key criterion: all 61 other parsed shadow keys retain train253's mismatch counts in all four runs; lotcfg remains 293/293. Comparison: `build-runs/analyze-propmgr.py` against main `build-runs/regress-train253-shadows.log`.
- Three differing diagnostic samples: NOT MET. New batch has zero; one verified prior sample remains in `propmgr-query27-diag-a2.log:40212` (native-only kind 0, flags 7, untimed, Z=768 at observed upper bound, no observed same-tick single insert/remove).
- Corrected the unsupported prior a3 citation in `docs/specs/questions/prop-lotconfig.md`; current a3 archive is 220/220 and contributes no sample.
- Fix criterion: NOT MET. No semantic/timing change was guessed; four exact runs do not establish a fix for the earlier intermittent failure. Existing PL24 asks for both axes/NaN behavior and any kind-0 exception.
- Landing: held as wip, not enqueued for verified credit. No coverage files, replace lists, game files or ABI exports changed.
- Next: spec writer resolves PL24, then resume the missing diagnostics and spec-backed fix before claiming deterministic agreement.
