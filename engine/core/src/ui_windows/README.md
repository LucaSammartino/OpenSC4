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
implementation of concrete slot 150 (`create_private_buffer`), which the spec
explicitly leaves unsettled. Allocation failure is propagated through all the
specified common tails. Host callbacks can reenter the runtime; child snapshots
retain their old list and copy-on-write reference ownership across mutations.

## Owner closure

Every row refers to `widget-toolkit.md` unless another spec is named. Test names
are in `base/tests.rs`. Public state fields implement the published simple
getters/setters (IDs, manager, keyboard, caption, notification, colours, periods).
The base and temporary owners have all specified settled core bodies implemented;
neither is claimed as a complete native ABI/lifetime/graphics implementation.

| Owner | Spec section | Code item | Test / remaining question |
|---|---|---|---|
| cGZWin | 1 identity | `IID_WIN`, `BaseWindow`, `Windows::create`, `create_default` | `wt01_constructor_and_lifecycle`, `constructors_reparenting_and_tree_forwarders`; native own vtable open: WT1 |
| cGZWin | 2 layout and defaults | `BaseWindow::new`, public state | `wt01_constructor_and_lifecycle`; untouched A0/A4 zeroed, open: WT2 |
| cGZWin | 3 flags | `BaseWindow::flag`, `Windows::set_flag` | `wt05_06_exact_mask_focus_capture`; unused default bits retained, open: WT3 |
| cGZWin | 3.1 exact-mask side effects and whole-word return | `set_flag` | `wt05_06_exact_mask_focus_capture`, `wt09_fade_and_unsigned_clock`, `wt14_fade_temporary` |
| cGZWin | 3.2 show/hide | `show`, `hide` | `wt09_fade_and_unsigned_clock` |
| cGZWin | 4.1 copy-on-write children | `read_children`, `write_children`, `unlock_children`, `ChildRead` | `snapshot_enumeration_and_busy_destruction`; native destruction with retained readers open: WT12 |
| cGZWin | 4.2 add/remove/parent/delete | `child_add`, `child_remove`, `set_parent`, `child_delete`, `child_delete_absolute`, `child_delete_all` | `wt02_03_front_order_duplicate`, `wt04_failed_init`, `snapshot_enumeration_and_busy_destruction`, `constructors_reparenting_and_tree_forwarders`; busy manager detachment open: WT5 (published assumption used) |
| cGZWin | 4.3 z-order and chains | `child_to_front`, `child_to_front_id`, `child_to_back`, `pull_to_front`, `send_to_back`, `child_step`, `child_move_relative`, `in_child_chain` | `wt19_sort_and_reorder`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 5.1 area and conversions | `calc_absolute`, `set_area`, `move_to`, `offset`, `screen_to_window`, `Host::window_to_screen` | `wt17_draw_rectangles_and_converters`, `movement_inherited_buffers_and_child_buffer_rules` |
| cGZWin | 5.2 point tests | `point_window`, `point_parent`, `point_screen`, `point_transparent` | `transparency_native_colors_and_draw_context`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 5.3 point and ID lookup | `child_from_point`, `child_from_id`, `child_from_id_recursive` | `wt02_03_front_order_duplicate`, `wt07_08_bounds_and_descendant_id` |
| cGZWin | 6 focus/capture | `set_focus`, `set_focus_to`, manager state | `wt05_06_exact_mask_focus_capture` |
| cGZWin | 7.1 message layout | `Message`, `do_message` | `wt10_dispatch_filters_and_arguments` |
| cGZWin | 7.2 filters | `add_filter`, `remove_filter`, `run_filters` | `wt10_dispatch_filters_and_arguments`, `shutdown_during_filter_releases_each_entry_once`; mutation timing open: WT10 |
| cGZWin | 7.3 dispatch and defaults | `do_message`, `Host::handler`, `accelerate` | `wt10_dispatch_filters_and_arguments`; void type-1 return open: WT11 |
| cGZWin | 7.4 send/post | `send`, `post`, `dispatch_posted` | `wt10_dispatch_filters_and_arguments` |
| cGZWin | 8.1 construction, Init, Shutdown, destructor | `create`, `init`, `shutdown`, `destroy_now` | `wt01_constructor_and_lifecycle`; caller supplies serial (WT6); native ref/deleting ABI open: WT12 |
| cGZWin | 8.2 fade | `set_fade`, `fade_advance`, `BaseWindow::fade_running` | `wt09_fade_and_unsigned_clock` |
| cGZWin | 9 quirks | exact-mask, front/back, unsigned time, retained notification, parameter ownership | tests for §§3–8 and 17 |
| cGZWin | 10 conformance | `base/tests.rs` | all 21 numbered cases, grouped into `wt01` through `wt21` tests |
| cGZWin | 11–12 completion boundary | host operations, opaque native handles | open: WT1–WT3, WT5–WT6; no invented graphics-slot/create-buffer internals |
| cGZWin | 13.4 accelerator wiring | `set_accelerator`, `accelerate`, `do_message`, `shutdown` | `wt10_dispatch_filters_and_arguments`, `wt01_constructor_and_lifecycle` |
| cGZWin | 14.1 paint state | `dirty`, `buffered`, draw rectangles/flags | `wt13_clean_private_buffer` |
| cGZWin | 14.2 Plot and deferred queue | `plot`, `queue_plot`, `flush_plots` | `wt11_painter_order`, `wt12_overlay_queue_and_present` |
| cGZWin | 14.3 PlotComposite | `plot_composite`, `clean_overlays` | `wt11_painter_order`, `wt12_overlay_queue_and_present`, `wt13_clean_private_buffer`, `transparency_native_colors_and_draw_context` |
| cGZWin | 14.4 PlotPresent | `plot_present` | `wt12_overlay_queue_and_present`, `transparency_native_colors_and_draw_context` |
| cGZWin | 14.5 PrivateBuffer | `private_buffer` | `wt14_fade_temporary`, `private_buffer_failure_and_absent_draw_service`, `ineligible_fade_releases_surface_without_temporary`; concrete allocation internals open in §12 |
| cGZWin | 14.6 buffer selection | `set_buffer`, `set_buffer_recursive` | `wt17_draw_rectangles_and_converters` |
| cGZFadeTmpWin | 14.7 constructor, StealBuffer, Plot | `steal_fade_buffer`, temporary branch of `plot` | `wt14_fade_temporary`; native allocation/lifetime follows open base boundary |
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
| cGZWin | ui-windows.md 2 interfaces/tree/ChildDelete forward | existing `win` facade; `base` provides full toolkit behavior | existing `win`/conformance tests; `snapshot_enumeration_and_busy_destruction` |

The wildcard `cGZWin*` must not mark other widget owners implemented. Generated,
custom, HTML, control, manager and title-bar owners have separate contracts and
are outside this base-owner implementation. `cRZWinKeyAccelerator` itself remains
in `key_accelerator`; only its window wiring belongs here.
