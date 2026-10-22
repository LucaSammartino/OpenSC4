| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities4: SC4ViewUtilities owner | Codex / `core-view-utilities4` (engine lane) | in progress; baseline cargo/clippy PASS, ctest 67/67; quick 2388/0 PASS; VU4–VU11 filed | 1. Full gate on committed code. 2. Reconcile missing owner contracts. |

## Handoff
- Goal: pure Rust SC4ViewUtilities owner from view-utilities sections 1–7,9,11; no ABI/plugin changes.
- Detached HEAD reserved by supervisor; commit normally, never change refs.
- Baseline qbuild all PASS: workspace cargo/clippy, ctest 67/67 (119.46s); build-runs/qbuild-*.log and fc4verify-ctest.log.
- Done: 935a8ff2 owned math/camera/frustum/projection/draw/visible/picking implementation; lib.rs registration only shared change.
- Added 33 unit contract tests, including independent numbers, failure/ownership cases, exhaustive bbox corner check and shared-thread caches.
- Getter caches use OnceLock; no RefCell. Camera-ray and instance-transform providers supply only permitted raw inputs.
- Questions VU4–VU11 appended to docs/specs/questions/view-utilities.md; no generated index edit or tools/ run.
- Initial quick: one over-tight analytic perspective tolerance fixed. Second quick PASS: 2388/0 core tests, dependencies compiled, CMake 4/4.
- Latest three tests and bounded malformed-resource walk will be verified by final all. Both caches now literally clear every draw (VU9).
- Done: 37d82b6b inventory/refinements; full workspace Rust tests and clippy passed, build/ctest pending (session 34962).
- Formatting: all five new .rs files PASS. Shared lib.rs FAIL is baseline module-order debt; build-runs/check-view-format-baseline.py confirms only additive registration.
- Ownership test now explicitly reads mutated source, removing unused-assignment warning; commit then rerun final all on stable code.
- Inventory: engine/core/src/view_utilities/README.md maps 38 contracts and explicitly marks missing RotateVectorByViewRotation; 49-function accounting unresolved.
- Next: commit refinements then qbuild all on committed code; formatting wrapper needs absolute file paths (cwd engine).
- Spec has no complete 49-function inventory; do not award whole-owner/19670-byte credit. Missing lifecycle bodies tracked VU4.
- q.py wrapper failed because Bash access is denied; rustfmt directly on NEW module files succeeded (no mass formatting).
- No game request; pure core scope.
