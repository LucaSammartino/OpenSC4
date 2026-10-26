# Window base owners

`base::Windows` implements the portable `cGZWin` base-window algorithms and the
`cGZFadeTmpWin` subclass from `widget-toolkit.md`. It is a core runtime, with no
plugin or C ABI adapter. The pre-existing `win::WinStore` remains the city-window
trace facade used by catalog/dialog conformance; its simplified flags and tree
are not the toolkit implementation. Toolkit clients use `base`.

`base::Host` is the explicit boundary to subclass virtual methods, native
buffers, draw contexts, cursor services, colour conversion and accelerator
processing. `NativeOp` preserves ordered calls and their arguments without
inventing the meanings of opaque native graphics slots. The host owns the
separate drawing (interface slot 4) and presentation (slot 5) context queries.
The host also owns the
implementation of concrete slot 150 (`create_private_buffer`), which the spec
explicitly leaves unsettled. Allocation failure is propagated through all the
specified common tails. Host callbacks can reenter the runtime; child snapshots
retain their old list and copy-on-write reference ownership across mutations.

## Owner closure

Every row refers to `widget-toolkit.md` unless another spec is named. Test names
are in `base/tests.rs`. Public state fields implement the published simple
getters/setters (IDs, manager, keyboard, caption, notification, colours, periods).
The base and temporary owners have all specified settled core bodies implemented.
WT5 and WT10-WT12 are implemented from the updated sections 4.2, 7.2-7.3, 8.1
and 10.1. Constructor references start at zero; callers acquire ownership explicitly.
The raw-byte dispatcher and sender preserve slot 16's AL, including non-Boolean
override bytes. Boolean facades are also available. Filters use live traversal;
pending removals own references and drain only at an outer paint filter pass.
The native repeated-singleton-removal corruption boundary is an explicit panic;
the spec promises no stable continuation after sentinel erasure. Self/cyclic or
dangling parent traversal and destruction during active parent passes are likewise
outside the spec's safe domain; direct parent storage does not reject them.
New section 10.1 vectors are in `base/closure_tests.rs`.

| Owner | Spec section | Implemented in `engine/core/src/ui_windows/base/` | Test / remaining question |
|---|---|---|---|
| cGZWin | 1 identity — implemented | `IID_WIN`, `BaseWindow`, `Windows::create`, `create_default`, `base_slot`, `Host::handler` (WT1) | `wt01_constructor_and_lifecycle`, `constructors_reparenting_and_tree_forwarders`, `resolved_tests::wt1_callable_inventory_and_constant_stubs` |
| cGZWin | 2 layout and defaults — implemented | `BaseWindow::new`, `set_mouse_down_offset`, unset draw origin and mouse offset (WT2) | `wt01_constructor_and_lifecycle`, `resolved_tests::wt2_untouched_fields_and_mouse_offset_writer` |
| cGZWin | 3 flags — implemented | `BaseWindow::flag`, `Windows::set_flag`; preserve consumer bits without base side effects (WT3) | `wt05_06_exact_mask_focus_capture`, `resolved_tests::wt3_default_consumer_flags_have_no_base_side_effects` |
| cGZWin | 3.1 exact-mask side effects and whole-word return | `set_flag` | `wt05_06_exact_mask_focus_capture`, `wt09_fade_and_unsigned_clock`, `wt14_fade_temporary` |
| cGZWin | 3.2 show/hide | `show`, `hide` | `wt09_fade_and_unsigned_clock` |
| cGZWin | 4.1 copy-on-write children | `tree.rs`: `read_children`, `write_children`, `unlock_children`, `ChildRead` | `snapshot_enumeration_and_busy_destruction`, `wt12_manager_retained_cow_references_with_and_without_caller` |
| cGZWin | 4.2 add/remove/parent/delete | `tree.rs`: `child_add`, `child_remove`, `set_parent`, `child_delete`, `child_delete_absolute`, `child_delete_all`, `destroy` | `wt02_03_front_order_duplicate`, `wt04_failed_init`, `constructors_reparenting_and_tree_forwarders`, `wt5_busy_requests_do_not_detach_and_delete_all_rereads_head`, `wt12_raw_parent_storage_and_invalid_child_rejection` |
| cGZWin | 4.3 z-order and chains | `child_to_front`, `child_to_front_id`, `child_to_back`, `pull_to_front`, `send_to_back`, `child_step`, `child_move_relative`, `in_child_chain` | `wt19_sort_and_reorder`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 5.1 area and conversions | `calc_absolute`, `set_area`, `move_to`, `offset`, `screen_to_window`, `Host::window_to_screen` | `wt17_draw_rectangles_and_converters`, `movement_inherited_buffers_and_child_buffer_rules` |
| cGZWin | 5.2 point tests | `point_window`, `point_parent`, `point_screen`, `point_transparent` | `transparency_native_colors_and_draw_context`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 5.3 point and ID lookup | `child_from_point`, `child_from_id`, `child_from_id_recursive` | `wt02_03_front_order_duplicate`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 6 focus/capture | `set_focus`, `set_focus_to`, manager state | `wt05_06_exact_mask_focus_capture` |
| cGZWin | 7.1 message layout | `Message`, `do_message` | `wt10_dispatch_filters_and_arguments` |
| cGZWin | 7.2 filters | `dispatch.rs`: `add_filter`, `remove_filter`, `run_filters`; `mod.rs`: `shutdown` | `wt10_live_append_including_original_tail_and_nested_pass`, `wt10_nested_message_retains_pending_until_outer_paint`, `wt10_duplicates_and_nested_paint_boolean_restore`, `wt10_consumed_paint_still_drains_self_removal`, `wt10_repeated_singleton_removal_reaches_corruption_boundary`, `shutdown_clears_active_and_pending_owning_entries` |
| cGZWin | 7.3 dispatch and defaults | `dispatch.rs`: `do_message_byte`, `do_message`, `accelerate`; `mod.rs`: `Host::child_delete_byte`, `Host::handler` | `wt10_dispatch_filters_and_arguments`, `wt11_type_one_propagates_manager_and_override_bytes` |
| cGZWin | 7.4 send/post | `dispatch.rs`: `send_byte`, `send`, `post`, `dispatch_posted` | `wt10_dispatch_filters_and_arguments`, `wt11_type_one_propagates_manager_and_override_bytes` |
| cGZWin | 8.1 construction, Init, Shutdown, destructor — implemented | `mod.rs`: `Windows::default`, `create`, `query_interface`, `add_ref`, `release`, `init`, `shutdown`; `tree.rs`: `destroy`, `deleting_destructor` (WT6, WT12) | `wt01_constructor_and_lifecycle`, `wt12_query_interface_outputs_and_zero_constructor_count`, `wt12_reference_wrap_unsigned_release_and_deleting_argument`, `wt12_deleting_destructor_flag_bit_zero_and_teardown_order`, `wt12_busy_ancestor_coverage_and_root_caller_ownership`, `resolved_tests::wt6_fresh_manager_first_registration_is_one` |
| cGZWin | 8.2 fade | `set_fade`, `fade_advance`, `BaseWindow::fade_running` | `wt09_fade_and_unsigned_clock` |
| cGZWin | 9 quirks | exact-mask, front/back, unsigned time, retained notification, parameter ownership | tests for §§3–8 and 17 |
| cGZWin | 10 and 10.1 conformance | `tests.rs`, `closure_tests.rs` | all 21 original cases and all 15 added filter/return/lifetime rows, including corruption-boundary detection |
| cGZWin | 11–12 completion boundary | `mod.rs`: `Host`, `NativeOp`, `base_slot`, `Windows::default` | WT1–WT3 and WT6: `resolved_tests`; WT5 and WT10–WT12: `closure_tests` above; open: WT14 |
| cGZWin | 13.4 accelerator wiring | `set_accelerator`, `accelerate`, `do_message`, `shutdown` | `wt10_dispatch_filters_and_arguments`, `wt01_constructor_and_lifecycle` |
| cGZWin | 14.1 paint state | `dirty`, `buffered`, draw rectangles/flags | `wt13_clean_private_buffer` |
| cGZWin | 14.2 Plot and deferred queue | `plot`, `queue_plot`, `flush_plots` | `wt11_painter_order`, `wt12_overlay_queue_and_present` |
| cGZWin | 14.3 PlotComposite | `paint.rs`: `plot_composite`, `clean_overlays`; `dispatch.rs`: `run_filters` | `wt11_painter_order`, `wt12_overlay_queue_and_present`, `wt13_clean_private_buffer`, `transparency_native_colors_and_draw_context`, `wt10_consumed_paint_still_drains_self_removal`, `wt10_nested_message_retains_pending_until_outer_paint` |
| cGZWin | 14.4 PlotPresent | `plot_present` | `wt12_overlay_queue_and_present`, `transparency_native_colors_and_draw_context` |
| cGZWin | 14.5 PrivateBuffer | `paint.rs`: `private_buffer` | `wt14_fade_temporary`, `private_buffer_failure_and_absent_draw_service`, `ineligible_fade_releases_surface_without_temporary`; concrete allocation internals open: WT14 |
| cGZWin | 14.6 buffer selection | `set_buffer`, `set_buffer_recursive` | `wt17_draw_rectangles_and_converters` |
| cGZFadeTmpWin | 14.7 constructor, StealBuffer, Plot; 10 vector 14 | `paint.rs`: `steal_fade_buffer`, temporary branch of `plot`; `mod.rs` and `tree.rs`: inherited lifetime | `wt14_fade_temporary` |
| cGZFadeTmpWin | 12 derived destructor coverage — implemented | `create_fade`, inherited `deleting_destructor`, `shutdown` (WT13) | `resolved_tests::wt13_fade_shared_deleting_wrapper_and_parameter_lifetime` |
| cGZWin | 15.1 cursor routing / mouse-window transitions | `process_cursor`, `set_mouse_window` | `wt15_cursor_disabled_click_and_move` |
| cGZWin | 15.2 cursor | `update_cursor` | `wt15_cursor_disabled_click_and_move` |
| cGZWin | 16 key equivalents | `make_key_equivalent`, `check_key_equivalent`, `child_key_equivalent` | `wt16_key_equivalents` |
| cGZWin | 17.1 draw rectangles | `set_draw_area`, recursive draw/buffer methods | `wt17_draw_rectangles_and_converters`, `private_buffer_failure_and_absent_draw_service` |
| cGZWin | 17.2 child private buffers | `child_private_buffers` | `wt14_fade_temporary`, `movement_inherited_buffers_and_child_buffer_rules` |
| cGZWin | 17.3 invalidation | `invalidate_self`, `invalidate_all_parents`, `invalidate_self_and_parents` | `wt18_overlay_invalidation` |
| cGZWin | 17.4 conversions, chain and forwards | `window_to_window`, `in_parent_chain`, `move_relative_to`, ID front form, focus default in `do_message` | `wt17_draw_rectangles_and_converters`, `wt07_08_bounds_and_descendant_id`, `wt10_dispatch_filters_and_arguments` |
| cGZWin | 17.5 parameters | `set_param`, `get_param`, `enum_params` | `wt21_params_ownership_order_and_enum` |
| cGZWin | 17.6 enumeration/sort | `enum_children`, `sort_children`, `sort_children_by_id` | `snapshot_enumeration_and_busy_destruction`, `wt19_sort_and_reorder` |
| cGZWin | 17.7 placement | `center`, `fit_rect` | `wt20_placement` |
| cGZWin | 17.8 colours/transparency | colour fields, `packed_to_native`, `native_to_packed`, `set_transparency`, `fill_transparent` | `transparency_native_colors_and_draw_context` |
| cGZWin | ui-windows.md 2 interfaces/tree/ChildDelete forward | `mod.rs`: `query_interface`; `tree.rs`: tree and `child_delete`; `dispatch.rs`: slot-16 byte boundary | `wt12_query_interface_outputs_and_zero_constructor_count`, `wt11_type_one_propagates_manager_and_override_bytes`, `snapshot_enumeration_and_busy_destruction` |

The wildcard `cGZWin*` must not mark other widget owners implemented. Generated,
custom, HTML, control, manager and title-bar owners have separate contracts and
are outside this base-owner implementation. `cRZWinKeyAccelerator` itself remains
in `key_accelerator`; only its window wiring belongs here.
