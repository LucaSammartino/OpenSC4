| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-listtree-close14 (ListBox/TreeView owner closure) | `Codex` / `core-listtree-close14` (engine lane) | ready; cargo 7512 passed/0 failed/0 ignored; clippy 0 diagnostics; native build 219/219; CTest 81/81; ABI unchanged; no game runs or questions | (1) Supervisor publishes detached HEAD; (2) land the ready merge-queue entry. |

## Report
- Implemented portable cGZWinListBox/cGZWinTreeView slot adapters, output preservation, stale-root/restart preconditions, raw negative properties and every assigned literal WC-L/WC-T vector. Their README sections have no remaining-work cells.
- Final code: 64a1764e9 requires GS before accepted drag allocation; 422120cbe and the final drawing assertion cover refusal ordering and untouched selection, topology, buffer, references, notifications and drawing.
- Focused widget tests: 101 passed/0 failed. Final qbuild all: PASS; cargo totals 7512 passed/0 failed/0 ignored across 51 summaries; clippy 0 diagnostics. Both owned Rust files pass formatting.
- Queued native job qbuild-20261025-083250-build-ctest: PASS, build 219/219 edges, CTest 81/81. Evidence: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log; earlier evidence preserved separately.
- C ABI exports unchanged; plugin, capi, coverage-map, FileBrowser and TextEdit untouched. Game runs: none; shadow: n/a; questions: none.

## Handoff
- No settled portable ListBox/TreeView owner work remains; both can count as implemented. Graphics/font pixels and the synchronous drag runner remain supplied services as specified.
- Full gate session 2591 reached terminal PASS. Build environment: CARGO_BUILD_JOBS=1, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0.
- Supervisor publishes the detached HEAD; build-runs/merge-queue/core-listtree-close14.json names the final full 40-character tip with kind engine and state ready.
