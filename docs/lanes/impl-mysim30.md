| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| impl-mysim30: MySim window and tools; kind engine | Codex; .claude/worktrees/impl-mysim30; detached HEAD, reserved branch impl-mysim30 | ready; six code/test milestones from ee2d4891f through HEAD; ABI/seams none; Cargo 8118/8118, Clippy 0 diagnostics, CTest 141/141; game impl-mysim30-r1-core-smoke-20261026 clean, ref_warnings=0; shadow n/a (unwired); question MW3 | 1. Publish detached HEAD to reserved branch and enqueue kind engine; coordinator flips implemented credit. 2. Resolve open MW1–MW3/MST1–MST3 before future native adapters. |

Acceptance: five owner modules implemented in `engine/core/src/mysim_window` and `engine/core/src/mysim_input_control` (change_house, change_job, move_in, dispatch).
Closure: both owner-family READMEs contain one row per numbered spec section, code items and worked-case tests; pick-sim dialog is a separate owner outside this task.
Reuse: `ui_windows::base::Windows` manages window state; existing script/bitmap/text/callout widgets receive trait calls; no duplicate widget implementation.
Boundaries: existing MW1/MW2 and MST1/MST2/MST3 are explicit Unsupported contracts; raw slot traces remain usable without invented semantic getters.
New question: MW3 in `docs/specs/questions/mysim-windows.md` (HTML/image mappings and vehicle-property widget layout); those operations report Unsupported.
Tests: 20 new owner tests pass, covering window cases 1/4 and MT-C1–MT-C7 plus lifecycle, reference and unsupported-domain checks (`build-runs/qbuild-test.log`).
Cargo: 8118 passed, 0 failed, 0 ignored (`build-runs/qbuild-test.log`); Clippy: 0 diagnostics (`build-runs/qbuild-clippy.log`).
Native: build PASS 35/35 edges (`build-runs/fc4verify-build.log`); CTest PASS 141/141 (`build-runs/fc4verify-ctest.log`).
Format: all 13 owned Rust files pass; changed files rechecked after corrections (`build-runs/validation-summary.txt`).
Game: shared queue label impl-mysim30-r1-core-smoke-20261026; clean, ref_warnings=0, foreign_exceptions=0, Rust panics=0; advanced to 2026-02-07 under turbo.
Game evidence: `C:/Users/tlori/FreeCity4/build-runs/game-results/impl-mysim30-r1-core-smoke-20261026.verdict` and `.log` (verdict at log line 3859).
Runtime scope: smoke only; new owners have no C ABI/plugin/shadow wiring by task, so no runtime parity or shadow-exact claim. Dispatch's documented single-hover-reference leak is measured separately in core tests.
Gate is the corrected source run; earlier MSRV failure is preserved in `build-runs/gate-attempt1.txt` and its separate diagnostic log.
Landing: ready JSON in this worktree's merge queue; coordinator publishes/enqueues it per docs/lanes/README.md. Clean-room rules prohibit this lane from invoking tools/merge-train.py directly.
