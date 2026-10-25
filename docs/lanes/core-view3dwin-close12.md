| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin-close12 (cSC4View3DWin) | Codex / `core-view3dwin-close12` (engine lane) | ready; cargo 7423/0; clippy clean; CTest 81/81; ABI unchanged | - |

Implemented column: `fc4_core::ui_windows::view3d`, at `engine/core/src/ui_windows/view3d.rs` and its submodules.
Trigger assignment: `engine/core/src/ui_windows/view3d/state.rs`; opaque records: `engine/core/src/ui_windows/view3d/references.rs`.
VW19 loads assign pairs in stream order without clearing, reserving, adding counts or notifying; only new keys grow.
Both record adapters validate identity/version before resolving references or mutating triggers.
Core owner also covers commands, input/control/panels, city/GUI lifetimes, status, helpers and paint completion.
VW-T40–T44 cover both record adapters, with legacy-version and invalid-header checks.
This round adds unsigned-key/zero-count and populated 389-bucket reload tests, checking both save adapters.
All 108 view3d module tests passed; the focused trigger-load run passed 6 tests.
The existing 34 closure rows map to implementation and tests; all 47 explicitly named tests resolve; no core row is open.
Final gate: one `qbuild all` PASS on committed engine source; workspace 7423 passed / 0 failed / 0 ignored.
Clippy `-D warnings`: PASS, zero diagnostics. Queued native verify: build 46/46 edges PASS; CTest 81/81 PASS.
Native job: `qbuild-20261025-010653-build-ctest`; native verification completed in 223.7 seconds.
Console: `build-runs/all-close12-r6.log`; focused results: `build-runs/trigger-load-close12-r6.log`.
Explicit owned-file formatting and diff checks passed; `engine/build-x86/FreeCity4.dll` exists.
Questions: none new; VW19 is settled. C ABI exports are unchanged. Game runs: none (core-only implementation).
Missing: native adapter/foreign byte-serializer integration remains outside this owner; no settled core closure is missing.
Next: coordinator lands the READY merge-queue entry using its full 40-character tip.
