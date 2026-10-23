| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-host5b: Lua host owner closure | Codex / core-lua-host5b | ready | Coordinator review and merge; only spec §15 Q1/Q2/Q5/Q8 remain unsettled |

## Changes

Implemented the settled LH4 and LH5 contracts in the Lua host core. The module README's Owner closure table maps each owner/spec section to code and conformance tests; no LH4/LH5 Unsupported rows remain.

- `vm/lifecycle.rs` owns main/child thread records, synchronized shared views, monotonic IDs, registry ownership, reference decisions, destruction, stack movement and result counts. Cloning a view borrows without AddRef; explicit release/destroy operations accept the foreign API.
- `vm/generator.rs` owns setup, command notifications, binding, parameter conversion, dispatch, ordered outputs and cleanup. It initializes 500 records, grows by 20 and reuses free indices from the front.
- Added LH4/LH5 conformance coverage, including failed factories/hooks, shared reference counts and destruction after a null-context failed main hook. Rechecked the existing VM/thread owner coverage and retained its spec-section mappings.
- Lua interpreter primitives remain foreign traits. There are no plugin, capi or ABI changes and no coverage-map edits.

Implementation commits: `021cc87e` (settled owner contracts) and `7c162903` (destructor correction and regression).

## Verification

The final stable source passed `qbuild.py --wt <WT> all`. The completed native request is `qbuild-20261023-215115-build-ctest`; its saved verdict is pass. No operation remains outstanding.

- Workspace cargo tests: **6,380 passed, 0 failed, 0 ignored**, summed over every test-result line in `build-runs/qbuild-test.log`.
- Clippy with `-D warnings`: **passed, 0 diagnostics** (`build-runs/qbuild-clippy.log`).
- Native build: **49/49 edges passed**; full CTest: **68/68 passed**; native verification total **96.6 seconds** (`build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`).
- Focused Lua VM tests: **25/25 passed**; explicit formatting check: **7 owned Rust files passed**.
- Earlier mixed-source evidence remains in `build-runs/gate-before-destructor-correction/`; it is not the final verification claim. The final gate had no tree-change warning. This reporting-only continuation did not rerun the completed gate.

## Open boundaries and game runs

No new questions were needed. Only the spec's existing §15 Q1/Q2 (panic-function identity and Lua 5.0 versus 5.0.1) and Q5/Q8 (default reporter output and exact text, affecting §4.3) remain unsettled. These boundaries are explicit in the Owner closure table; no other behavior assigned to the VM/thread/command-generator owners remains Unsupported.

Game runs: none. This portable core change has no plugin integration and makes no real-game shadow claim.

## Next steps

Coordinator review and merge. A future spec answer can settle the four existing §15 questions; native adapter integration is outside this lane.
