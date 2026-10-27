| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| impl-dvview30 dispatch-vehicle-view (engine) | Codex / .claude/worktrees/impl-dvview30 / impl-dvview30 (detached HEAD) | ready; code commits 9fc0d0581, 4aaef3d21, 52333d20f, 8c54ce2e0; ABI/seam none; cargo 8115 passed, clippy 0 diagnostics, CTest 141/141; impl-dvview30-r1-smoke1 clean refs=0; dispatch shadow 44/44 exact, view shadow n/a; questions DVV30-1/2 | 1. Publish detached tip and attach implemented credit through the merge train. 2. Resolve Unsupported boundaries before native wiring. |

- Owners: 2 portable modules, `engine/core/src/dispatch_vehicle_view/` and `dispatch_vehicle_track_info/`; each has a README closure table.
- Scope: view sections 1–5 and applicable 7–9 complete; section 6 is the separate input-tool owner, outside this task. No C ABI or plugin wiring.
- Worked cases: 17/17 new tests pass, including spec §8 cases 1–7; `build-runs/qbuild-test.log:3777` through the owner test block.
- Final corrected-source `qbuild all`: cargo 8115 passed / 0 failed / 0 ignored (`build-runs/qbuild-test.log`); clippy 0 diagnostics (`build-runs/qbuild-clippy.log`).
- Native: build 29/29 edges (`build-runs/fc4verify-build.log`); CTest 141/141 (`build-runs/fc4verify-ctest.log`); verdict `build-runs/build-results/qbuild-20261026-215726-build-ctest.verdict`.
- Formatting: 8 owned Rust files, 0 diff lines; recorded output `build-runs/impl-dvview30-fmt.txt`. Shared lib.rs only received additive module declarations.
- Earlier quick/all results preceded the live-handle correction; the final all results above test the frozen Rust source at 8c54ce2e0.
- Game: `impl-dvview30-r1-smoke1` clean, foreign exceptions=0, watchdog stalls=0, ref_warnings=0; reached 2026-02-11, 22.75 days/s, requested 2026-02-01.
- Game evidence: `C:/Users/tlori/FreeCity4/build-runs/game-results/impl-dvview30-r1-smoke1.verdict` and the adjacent `.log` (shadow shutdown line 3675; verdict line 3990).
- Existing dispatch shadow: 44/44 values exact, 2/2 samples exact, resynced=0. New view shadow key: none; no view verification/replacement credit claimed.
- DLL SHA256: `24CFF6CBF197B8C8B94C7184CEA9613EDAA92BC0E4952E950E64B6F8CBC2F688`, matches the game report prefix.
- Open DVV30-1 (Init packed states) and DVV30-2 (row-2 spin spill) are explicit Unsupported boundaries, in `docs/specs/questions/dispatch-vehicle-view.md` and README rows.
- Numeric qualification: finite f64 intermediates with specified f32 stores; arbitrary ambient x87 precision/transcendental bit identity is not certified. Native invalid-index/null-processor boundaries are explicit errors.
- Next: coordinator publishes the reserved branch and attaches implemented engine credit from the queue entry; foreign wiring remains a future task.
