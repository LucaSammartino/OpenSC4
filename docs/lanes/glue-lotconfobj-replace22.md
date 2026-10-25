| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotconfobj-replace22: Rust-backed lot configurations | Codex / `glue-lotconfobj-replace22` (engine lane, detached HEAD) | ready; cargo 7644/0, clippy clean, CTest 94/94; house clean to 2032, ref_warnings=0; lotconf shadow 73/73 exact | Coordinator review and coverage credit under lotcfg. |

Rust `LotConfiguration` owns loaded fields and geometry through four new mutex-backed C ABI exports; ABI version unchanged.
The replacement manager allocates FreeCity4 configurations directly through its shared factory; no native configuration factory is used.
C++ Load projects Rust geometry into mutable game-shaped arrays; its duplicate geometry parser was removed.
Tests cover missing GZCOM configuration registration (spec 1.1), catalogue reference ownership (seam 75), retained geometry after exemplar release, reload replacement, and concurrent snapshots.
Implementation: `c4f269d06`; concurrency test: `3a69b1476`; this commit corrects its element-count byte fixture.
Final `qbuild all`: PASS, Rust 7644 passed / 0 failed / 0 ignored; Clippy 0 diagnostics; owned-file fmt PASS.
Native queue `qbuild-20261025-064840-build-ctest`: PASS, build 46/46 (2.3 s), CTest 94/94 (91.2 s).
Full-house replacement run `lc22-r2-house-20261025b`: clean, 2032-01-01, ref_warnings=0; includes flora, lotdev, commercial, industrial and all three span keys.
Native comparison with `lotconf_shadow=1`: `lc22-r2-native-20261025c 73/73`; totals lines=73 exact=73 mismatched=0 incomplete=0, clean through 2032-01-07, ref_warnings=0.
Evidence: `build-runs/game-results/<label>.log` and `.verdict`, `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-{build,ctest}.log`.
Earlier failed fixture evidence remains in `build-runs/qbuild-test-failed-r2.log`; corrected concurrency and retention tests both pass in the final gate.
Shadows remain default-off. No spec questions, tool edits, coverage edits or run records added.
Remaining limit: legacy version-1 tags use a deterministic process serial without the GZCOM clock seed (`SPEC-GAP`); shipped version-2 data is measured above.
