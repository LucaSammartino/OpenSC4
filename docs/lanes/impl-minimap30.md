| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| impl-minimap30: minimap owners; kind engine | Codex; .claude/worktrees/impl-minimap30; reserved branch impl-minimap30 (detached HEAD) | ready; code commits bad376482 through a9dd62a91; ABI/seams none; cargo 8126/8126; clippy 0 diagnostics; CTest 141/141; final smoke r2 clean, ref_warnings=0; minimap shadow n/a; new questions none | Coordinator: publish detached commits, enqueue staged engine tip and flip implemented credit |

- Owners: cSC4WinMiniMap and nested cTZMap implemented under engine/core/src/minimap; no C ABI or plugin wiring.
- Closure: 13/13 spec sections mapped in minimap/README.md; nested owner has its own README.
- Worked cases: MM-C1 through MM-C10 covered; 28 minimap tests passed in build-runs/qbuild-test.log.
- Window/control reuse: ui_windows::base::Windows and widget_controls::Foreign/replace_ref; foreign data and graphics remain traits.
- Existing open MMW1-MMW4 are explicit Unsupported boundaries, cited per affected README row; docs/specs/questions/minimap.md unchanged.
- SPEC-GAPs: zero new pixels, nonpositive-area n=0, binary64 unstated decimal operands, dual-blit opaque lock code; native unsafe bounds return Unsupported.
- Format: 14 owned Rust files passed; fixed terrain file rechecked; build-runs/minimap-format-results.txt.
- Final full gate: qbuild all PASS after one lint fix; cargo 8126 passed, 0 failed/ignored (build-runs/qbuild-test.log).
- Clippy: -D warnings PASS, 0 diagnostics (build-runs/qbuild-clippy.log); prior precedence failure retained as qbuild-clippy-first-all.log.
- Native: CMake 197/197 edges (build-runs/fc4verify-build.log); CTest 141/141, 0 failed (build-runs/fc4verify-ctest.log).
- Shared-queue smoke impl-minimap30-smoke-20261026-r1: clean, ref_warnings=0, prior DLL c081a5c8; main build-runs/game-results/<label>.verdict/.log.
- Final smoke impl-minimap30-smoke-20261026-r2: clean, ref_warnings=0, exceptions/panics/stalls=0, reached 2026-02-13; same main results paths.
- Final DLL SHA256: 8ea496255a28f190c0726d081718a1ad264376de55cf3c7cfa898afd20bb72a6, matches r2 verdict.
- Shadow exact/total: n/a; this core-only task has no native minimap adapter or shadow key. Game runs establish plugin lifetime only.
- Queue: build-runs/merge-queue/impl-minimap30.json, kind engine, actual detached HEAD; supervisor publishes reserved branch.
