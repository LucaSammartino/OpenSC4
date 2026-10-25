| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin-close12 (cSC4View3DWin) | Codex / `core-view3dwin-close12` (engine lane) | ready; cargo 7421/0; clippy clean; CTest 81/81; ABI unchanged | - |

Implemented column: `fc4_core::ui_windows::view3d`, at `engine/core/src/ui_windows/view3d.rs`.
Trigger assignment is in `engine/core/src/ui_windows/view3d/state.rs`; opaque records are in `engine/core/src/ui_windows/view3d/references.rs`.
VW19 loads assign pairs in stream order without clearing, reserving, adding counts or notifying; only new keys grow.
Owner changes also finish command tools/city guards, the UI toggle, city helper initialization, required GUI lookups,
GUI reference balancing, tutorial/menu identities, mayor bookkeeping, edge/capture routing and shutdown ordering.
Both record adapters now share identity/version validation before reference resolution or trigger mutation.
Tests cover VW-T40–T44 through both adapters, legacy versions, rejected headers and the remaining owner vectors.
All 106 view-owner tests passed. The existing 34-row closure table maps to code/tests; 87 named functions resolve, with no open row.

Final gate: one `qbuild all` PASS; workspace Rust 7421 passed / 0 failed / 0 ignored; clippy -D warnings: 0 diagnostics.
Queued native verify: build 56/56 edges PASS; CTest 81/81 PASS; `engine/build-x86/FreeCity4.dll` produced.
Native job: `qbuild-20261025-005542-build-ctest`. Full console: `build-runs/all-close12-r5.log`.
Formatting passed for 13 explicit owned Rust files and the final changed owner fixture; diff check passed.
Focused failure logs are preserved separately; final workspace results include the corrected fixture and tutorial expectations.
Prior session 74977 was unavailable after resume; its incomplete r4 log is preserved and is not claimed as a pass.
Questions: none new; VW19 is settled. C ABI exports are unchanged. Game runs: none (core-only implementation).
Missing: native adapter/foreign byte-serializer integration is outside this owner; no settled core closure row remains open.
Next: land the reserved branch using the READY merge-queue entry.
