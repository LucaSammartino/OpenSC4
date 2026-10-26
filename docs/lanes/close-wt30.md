| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-wt30 / widget-toolkit | Codex / close-wt30 / detached HEAD (reserved branch close-wt30); kind engine | ready; commits ab3fc9fbc + final HEAD; ABI/seams unchanged; cargo 7975 passed, clippy 0 diagnostics, CTest 129/129; close-wt30-smoke-20261026b clean, ref_warnings=0; shadow n/a; questions none | 1. Coordinator land queued tip and flip implemented credit for cGZWin (12,876 B) and cGZFadeTmpWin (312 B) |

- Closure acceptance: 42/42 rows implemented, zero open IDs in `engine/core/src/ui_windows/README.md`; no coverage-map edits.
- cGZWin §1 / WT1: 151 callable slots, default stubs and excluded padding; `base_slot`, `Host::handler`; `resolved_tests::wt1_callable_inventory_and_constant_stubs`.
- cGZWin §2 / WT2: explicit unset draw origin/mouse offset, published offset writer; `BaseWindow::new`, `set_mouse_down_offset`; `wt2_untouched_fields_and_mouse_offset_writer`.
- cGZWin §3 / WT3: preserve 0x100/0x8000 without base side effects; `flag`, `set_flag`; `wt3_default_consumer_flags_have_no_base_side_effects`. Dialog/manager consumers stay outside this owner.
- cGZWin §4.2 / WT5, WT12: retained existing tree implementation; `wt5_busy_requests_do_not_detach_and_delete_all_rereads_head`, `wt12_raw_parent_storage_and_invalid_child_rejection` pass.
- cGZWin §7.2 / WT10: retained live filters, nested pass and reference handling; existing `wt10_*` closure vectors pass.
- cGZWin §7.3 / WT11: retained slot-16 raw-byte propagation; `wt11_type_one_propagates_manager_and_override_bytes` passes.
- cGZWin §8.1 / WT6, WT12: fresh manager serial 1 via `Windows::default`; `wt6_fresh_manager_first_registration_is_one`; existing query/refcount/destruction/COW cases pass.
- cGZWin §§11–12 / WT1–WT3, WT5, WT6, WT10–WT12, WT14: completion boundary closed by the code/tests above and concrete graphics requests below; no runtime observation remains open.
- cGZWin §14.5 / WT14: `create_private_buffer` owns destructive replacement, fallback Init and independent failure outputs; `wt14_tests` vectors A–L cover ordering, references, readiness and recursive common tails.
- cGZWin §14.6 / WT14: `set_buffer` owns service/QI lifetimes and borrowed-cache clearing; vectors M–N plus successful-QI/null-screen tests pass.
- cGZFadeTmpWin §14.7: `create_fade`, `steal_fade_buffer`, inherited plot/lifetime; existing `wt14_fade_temporary` passes.
- cGZFadeTmpWin §12 / WT13: shared deleting wrapper returns object, respects bit 0, preserves never-initialized variant leak, clears map nodes; `wt13_fade_shared_deleting_wrapper_and_parameter_lifetime`, `wt13_final_fade_release_has_only_one_base_buffer_teardown` pass.
- Other closure rows retain their named implementations/tests; no owner beyond these two receives credit.
- Tests: 56/56 toolkit-owner cases; full workspace 7,975 passed / 0 failed / 0 ignored (`build-runs/qbuild-test.log`). Earlier quick: 5,360 passed, tree-changed annotation (`build-runs/build-results/qbuild-20261026-174023-quick.verdict`); final all used committed source.
- Gate: qbuild all once, PASS; clippy 0 diagnostics (`build-runs/qbuild-clippy.log`); native 29/29 edges (`build-runs/fc4verify-build.log`); CTest 129/129 (`build-runs/fc4verify-ctest.log`); owned-file fmt 7 files / 0 differences.
- Game: main shared queue, `close-wt30-smoke-20261026b`, DLL f190b456; clean, exceptions/stalls/RTC/overruns/ref_warnings/panics all 0, date 2026-02-07 beyond requested 2026-02-01 (main `build-runs/game-results/close-wt30-smoke-20261026b.verdict` and `.log`, verdict line 3840).
- Request `close-wt30-smoke-20261026a` was rejected for unsupported top-level close_dialogs; corrected under new label b with parallel=1. No failed game execution.
- Shadow: n/a, no toolkit shadow key or plugin/C ABI adapter; game evidence is load/runtime smoke, core behavior is proven by spec vectors. No unexplained comparisons; questions none; C ABI exports unchanged.
