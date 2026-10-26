# Widget control owners

Portable owners from widget-controls and its incorporated specs. No plugin or
C ABI wiring is included. Tests use recording foreign services; no game,
pixel or shadow comparison is claimed.

## Owner closure

Grid and TextEdit implement their settled owner specifications. FileBrowser
and TreeView retain the explicit boundaries below. Public fields supply the
thin stored-state accessors.
Grid, ListBox and TreeView implement their settled owner specifications.
TextEdit and FileBrowser have the explicit question boundaries below.
Public fields supply the thin stored-state accessors.
Byte-stage TextEdit fixture entries remain available alongside live methods.

### cGZWinGrid

[Detailed section and test mapping](grid/README.md). All rows there remain
current; the owning final-release adapter below completes the lifetime boundary.

| Spec section | Code item | Coverage |
|---|---|---|
| win-grid 1 identity | query_interface, constants | identity test |
| 2 construction | Grid::new, Format, SizeEntry | six original vectors; WG6-a |
| 3 interface inventory | portable Rust methods | WG8-a-f |
| 4 flags | set_flags, window flag, mutation | owner layout/live-setter tests |
| 5.1 lifecycle/dispatch | init/shutdown/message/input, GridOwner | owner_final_release_destroys_storage_after_double_shutdown; key tests |
| 5.2 fonts/colors/gutters | live setters/default formats | owner_default_format_and_color_slots_share_storage |
| 5.3 scrolling/selection | scrolling/selection/limit methods | WG5-a-d |
| 5.4 sums | size_sum | size-cache/rectangle tests |
| 5.5 extras/headings | data owner methods | identity/copy-axis tests |
| 5.6 resources/notifications | buffer/sound/drop-down/owner-draw methods | live-setter test |
| 5.7 helpers/thunks | Rust containers, sorting, gcd | WG8-c-f |
| 5.8 sizes/images/sounds | setters/live mutation | layout/live-setter tests |
| 5.9 input | mouse/key/focus/selection | owner hit/key tests; WG5-c |
| 5.10 layout | recalculate, bars, visibility/cache | layout/cache/depth tests; WG8-b |
| 5.11 data/windows/movement | data/copy/move/disposal/search | ownership/editor tests; WG7-a |
| 5.12 painting | paint_owner, paint helpers | WG9-a-c; owner paint tests |
| 5.13 rectangles/axes | cell_rect, delete/copy/mutation | axis tests |
| 5.14 format/font | format copying/setters | WG6-b |
| 5.15 settled dependencies | merged/default format words, geometry/faults | WG5-WG7/WG9 including WG6-a-d |
| 5.16 remaining methods | resize, sort, comparison and partial setters | WG8-a-f |
| 6 vectors | six original plus settled vectors | grid/vectors.rs |
| 7/A inventory | portable owner/equivalent containers | detailed 198-body map in grid README |
| widget-controls 1/2/7/8 | shared Foreign and same owner | six original vectors and owner tests |

WG3 delayed gutter effects and WG4 extra-object store quirks are preserved.
Neither is an unsupported owner behavior. GridOwner performs both specified
Shutdown calls, then drops storage; external allocators may use Grid::release.

### cGZWinTextEdit

| Spec section | Code item | Tests / open contract |
|---|---|---|
| text-edit 1 identity/defaults | new, with_resolved_defaults, query_interface, Runtime::with_sound_words | te9_history_limit_and_preferred_x; te10_fresh_runtime_defaults; te10_construction_preserves_sound_words; resource tests |
| 2 text replacement | live set_text, retained/maximum helpers | set-text/multibyte tests |
| 3 selection/overwrite | set_selection, set_insert_mode | reversed/unsigned selection test |
| 4 character/replacement | live character/key queries, insert_text | live-character/output and overwrite/LF tests |
| 5 caret/timers | toggle/show/start/stop/callback | runtime caret/resource test |
| 6 raw/history | live raw edits, History::append_with, Action storage/replay | raw-domain/history/undo tests; te8_history_growth_capacity_copy_ownership_commit_order; te8_history_stopped_second_copy_keeps_old_and_leaks_prefix; te8_spare_capacity_stop_and_null_allocation_are_not_empty_history |
| 7 keys/clipboard | key_down/up, copy/paste/delete/navigation, composed lengths | runtime clipboard and preferred-X tests; te5_utf8_base_and_decoration_both_directions |
| 8 layout/view | line fitting, metrics, bars, scrolling, ReflowCounter | runtime layout/scroll/threshold/failure tests; te11_nested_editor_still_lays_out_without_automatic_check; te11_signed_threshold_and_nonreturning_check |
| 9 drawing | mask, selection, float caret widths, direct/edge source rectangles and mode | runtime paint/password tests; te12_direct_and_edge_source_destination_mode |
| 10 mouse/messages/lifetime | runtime/input owner methods; explicit service inputs | resource/flag and double-click tests; threshold/bar-failure and clipboard tests |
| 11 conformance | tests.rs, runtime_tests.rs, closure_tests.rs, encoding_tests.rs | original vectors plus constructor/reflow/image, TE8 65-71 and conversion/composition distinguishing vectors |
| 12 inventory/limits | portable owned storage, exact helpers and bounded host services | all TextEdit rows here; TE5/TE8/TE9-TE12 answered contracts covered by closure_tests.rs and encoding_tests.rs |
| encoding: lengths/counts/pending | Encoding helpers | malformed/DBCS and pending vectors 45-49 |
| adapters: text/state | adapters.rs, copy_bytes, borrowed getters, ordered WordOutputs | live-character/output tests (no NUL, alias writes); state repair test |
| adapters: insertion repair | line/count/index/fix methods | asymmetric repair/coherent span test |
| adapters: periods/sound/lifecycle | runtime methods, exact-height SetHeight | caret/resource and exact-height tests |
| encoding: conversion/UTF16 | conversion::convert, character, previous_character, CodePages | te5_same_buffer_*; te5_invalid_utf8_preserves_prefix; te5_utf8_*; te5_encoder_*; te5_two_stage_prefix_is_not_rolled_back; te5_character_adapter_count_and_ascii_shortcuts |
| encoding: Thai/composed lengths | composition::thai_class, thai_level, thai_group, forward, backward; TextForeign defaults | te5_thai_*; te5_fourth_word_is_read_with_count_three; te5_sara_am_forward_backward_asymmetry; te5_utf8_base_and_decoration_both_directions |
| adapters: filtering/composition | filter_buffer, fixed_copy, fix_composition | te8_filter_fixed_count_nul_padding; te5_thai_below_mark_context; composition erase/previous-value test |
| adapters: flags/images/colors | setters and borrowed getters | runtime resource/flag/converted-slot tests |
| adapters: scroll/search | scroll endpoints, adapters::search, find_text | runtime layout/scroll; te8_reverse_search_keeps_overlapping_matches; te8_forward_search_from_interior; te8_empty_forward_search_owner_rejects_end; te8_empty_reverse_search_owner_rejects_end |
| layout: coordinates/spans | step, word_step, byte/pixel metrics | coordinate/word and runtime span tests |
| layout: offsets/bars/caret | bar methods and insertion_in_view | layout/scroll and threshold/bar-failure tests |
| layout: word/page/arrow | select_word and vertical key routes | selection-extension/preferred-X tests |
| widget-controls 3/7 | same owner and incorporated tests | rows above |

TextEdit's answered TE5/TE8/TE9-TE12 contracts are implemented. Sound words
have an explicit portable zero substitute or caller-supplied constructor
storage; native construction leaves them uninitialized. Windows code-page
tables, the uninitialized fourth composition word, fonts, bars, timer and
clipboard outcomes remain controlled service/memory inputs. Native invalid
allocation and nonreturning-copy states are modeled without unsafe reads or
rollback promises. No font rasterization or in-game pixel equality is claimed.

### cGZWinFileBrowser

| Spec section | Code item | Tests / open contract |
|---|---|---|
| win-filebrowser 1 identity | query_interface/window/constants | C01-C05 identity/path test |
| 2 navigation/path | setters, parent_live, update_path_live | implemented (FBQ5); fbq4_navigation_removal_and_fbq5_live_creation_paint |
| 3 lookup | get_row_of_entry_by_name | C06-C09 lookup test |
| 4 vectors C01-C09 | navigation/lookup and live refresh | implemented (FBQ5); existing C01-C09 plus fbq4_navigation_removal_and_fbq5_live_creation_paint |
| 5 flags | set/add/clear/any-bit | C10-C12 flags test |
| 6 selection/notifications | descriptor, edited_descriptor, custom_entry, live selection handlers | implemented (FBQ4); C14-C18, fbq4_descriptor_background_and_drive_worked_cases, fbq4_columns_custom_records_and_unchanged_outputs |
| 7 directory creation | create_directory_candidate, create_directory_live | implemented (FBQ5); C14-C18 and fbq4_navigation_removal_and_fbq5_live_creation_paint |
| 8.1 population | population.rs, populate_files | implemented (FBQ5); fbq5_c28_c29_case_copy_and_unsigned_capacity, fbq5_c30_c31_c38_restore_empty_grid_and_disabled_columns, fbq5_c34_c35_c36_c37_directory_and_popup_worked_cases |
| 8.1.1 comparator | precedes | browser_compare_full_tags_byte_order_secondary_once_and_fit_boundary |
| 8.1.2 fitting | fit_string | same comparator/fit test |
| 8.2 controls/messages | controls.rs, obtain_control, routing.rs | implemented (FBQ2); fbq2_c39_c40_control_order_configuration_and_factory_boolean, fbq2_c41_c42_c43_message_routes_and_borrowed_notifications; no owner-assigned child rectangles per spec |
| 8.2.1 enables | enable_child | C23-C24 test |
| 8.3 resources | operations.rs, resources.rs; format (3,16), exact arrow, selector/result boundaries | C20/C21/C25/C26 and fbq1_arrow_recipe_failures_and_narrowed_icon_button_boundaries; open: FBQ1 §8.3.3 ordered icon/button primitive and pixel trace (two bodies / 5,361 B) |
| 8.4 buffers | set_icon/set_button_buffer | C23/C24 and lifecycle tests |
| 8.5 lifecycle/paint/area | lifecycle.rs defaults, init_property, paint_live, set_area, shutdown | implemented (FBQ2/FBQ4/FBQ5); browser_factory_prefix_and_settled_init, browser_init_paint_refresh_and_final_release_order, fbq4_navigation_removal_and_fbq5_live_creation_paint; procedural pixel boundary is in 8.3 |
| 8.6 construction/destruction | raw defaults/clears, destroy, BrowserOwner | shutdown-order and lifecycle tests distinguish direct destruction/final release |
| 8.7 properties/forwarding | properties.rs, close_popup, select_directory, remove_subdirectory | implemented (FBQ4); fbq4_style_widths_references_optional_words_and_common_font, fbq4_columns_custom_records_and_unchanged_outputs, fbq4_descriptor_background_and_drive_worked_cases, fbq4_navigation_removal_and_fbq5_live_creation_paint |
| 9 C10-C43 | existing focused vectors, closure_tests.rs | implemented (FBQ5); existing C10-C27 and named C28-C43 tests above |
| 10/A inventory | spec-scoped attribution; alternate_send and owned-record defaults | FBQ2/FBQ4/FBQ5/FBQ6 applied; fbq6_older_table_dispatch_and_completion_channels and fbq1_arrow_recipe_failures_and_narrowed_icon_button_boundaries; open: FBQ1 §8.3.3 two pixel recipes; no false comparator target or extra primary slot |
| widget-controls 4 | same owner | partial: open FBQ1 pixel trace in 8.3; other rows implemented as above |

Default lifecycle dispatch now executes the owner property/control/population
bodies; explicit overrides remain available. Recording services test foreign
calls, not foreign rendering. FBQ1 glyph paths stop with Unsupported at the
missing primitive recipe, never invented pixels. Native unsafe inputs (zero
row height, null required children, column-order scans outside six records)
are explicit preconditions. The callback directory-order property mapping and
new-filter pattern parsing remain one-line SPEC-GAP choices in the code.

### cGZWinListBox

Implemented column paths are relative to `engine/core/src/widget_controls/`.
The slot map below records the portable equivalents; stored-state getters use
public fields, without notifications or reference acquisition.

| Spec section | Implemented code item / module path | Tests |
|---|---|---|
| widget-controls 5.1 identity/inventory | list_box.rs: query_interface, stored fields; L/W slot map below | wc_l01; wc_f01; list_primary_bytes_and_exact_search_preserve_outputs_without_effects; list_residual_slot_adapters_colors_gutters_skin_and_base_flags |
| 5.2 construction/resources | list_box.rs: new/init/shutdown/destroy | wc_l10_l13; wc_l24_l34_l35_l36; wc_l36_timer_is_destroyed_before_retained_resources |
| 5.3 entries/selection | list_box.rs: insert/remove/select/next_page/size_drop_height/size_drop_width | wc_l16_l22, wc_l23_l30, wc_l24_l34_l35_l36 |
| 5.4 messages/capture | list_box.rs: key methods, mouse_move/mouse_up_at, bar_tracking; list_box/adapters.rs: forward_bar | wc_l14_l15; wc_l24_l34_l35_l36; wc_l_residual_outputs_storage_and_bounded_bar_coordinates |
| 5.5 drawing | list_box.rs: paint/paint_plain_text/scan_rgb; mod.rs: Foreign fitting primitives | wc_l26_l31_l34; wc_l24_l34_l35_l36 |
| 5.6 residual contracts | list_box.rs: sorting/search/stored_count/intensity, signed storage helpers; list_box/adapters.rs: color/gutter/skin/output adapters | wc_l23_l30, wc_l26_l31_l34; wc_l_residual_outputs_storage_and_bounded_bar_coordinates; wc_l27_l31_literal_sort_storage_and_highlight |
| 7 conformance | settled_tests.rs, closure_tests.rs and tests.rs | L01-L15 and L19-L36: original grouped tests plus wc_l19_l22_literal_removal_and_page_notifications and wc_l27_l31_literal_sort_storage_and_highlight; no L16-L18 literals are assigned in the spec |
| 8 coverage | portable core only | no plugin/game/shadow claim; WC1-WC6 are settled, not open questions |

### cGZWinTreeView

| Spec section | Code item | Tests |
|---|---|---|
| widget-controls 6.1 identity/interfaces | tree_view.rs: query_interface/constants; tree_view/adapters.rs: primary-slot inventory | T01-T03; tree_primary_insert_wrappers_copy_initialized_defaults_and_failure_data; tree_selection_output_and_borrowed_adapter_inventory |
| 6.2 defaults/lifetime | tree_view.rs: new/internal_init/create_root/shutdown/checked_init; tree_view/settled.rs: initialize_images | T01-T03, wc_t19_t26, wc_t27_t29_t36; wc_t26_restart_is_not_clean_initialization asserts the stale-root precondition and no resource retry |
| 6.3 nodes/selection | tree_view.rs: insert/insert_at/recalculate/selection | T02-T06, wc_t13_t16, wc_t19_t26 |
| 6.4 input/messages | tree_view.rs: wheel/keys/activation/messages; tree_view/input.rs: mouse_button | T07-T11, wc_t37_t38, wc_t39_t43 |
| 6.5 drawing/layout | tree_view/rendering.rs: recalculate_all/layout_viewport/cursor_row/content_width/render_item/paint_tree/connectors | T12, wc_t13_t16, wc_t17_t18_t48_t51; wc_t17_t24_literal_geometry_and_descriptor_children; tree_automatic_layout_sizes_buffers_and_updates_both_bar_requirements |
| 6.6 properties/descriptors/adapters | tree_view/settled.rs and tree_view/adapters.rs: properties, raw aliases, descriptors, insertion, move/removal, images and output copies | wc_t19_t26, wc_t27_t29_t36, wc_t30_t35; tree_negative_property_aliases_preserve_raw_side_effects; tree_property_setters_and_raw_resource_aliases_have_literal_side_effects; tree_primary_insert_wrappers_copy_initialized_defaults_and_failure_data |
| 6.7 input/drag/rendering | tree_view/input.rs: mouse_button/notify_drag/mouse_move/DragHelper, packed-row drawing; tree_view/rendering.rs: fills/connectors | wc_t37_t38, wc_t39_t43, wc_t44_t47, wc_t17_t18_t48_t51; tree_drag_helper_borrows_session_and_balances_capture; wc_t30_literal_move_order_and_t51_open_stroke; runner remains a supplied synchronous service |
| 7 conformance | tests.rs, settled_tests.rs and closure_tests.rs | T01-T51; wc_t17_t24_literal_geometry_and_descriptor_children; wc_t26_restart_is_not_clean_initialization; wc_t30_literal_move_order_and_t51_open_stroke; wc_t40_literal_icon_click_and_t49_failed_access_has_no_release |
| 8 coverage | explicit native preconditions, no WC Open branches | tree_internal_init_failures_do_not_retry_resources; tree_negative_property_aliases_preserve_raw_side_effects |

The WC1-WC6 unsupported placeholders have been replaced. Unsafe native
navigation/index domains use explicit
Precondition results. Shared click state is passed by the host to mouse_button;
the convenience mouse_down method performs an isolated single click.

#### ListBox slot audit (widget-controls 5.1 and 5.6)

Paths below are relative to this directory. Tests use the matching `wc_l*`
groups in tests.rs, settled_tests.rs and closure_tests.rs.

| Native slots | Portable code item | Covering test group |
|---|---|---|
| L0-L3 | query_interface; Foreign add_ref/release; window | wc_l01 |
| L4-L8 | remove/find_remove/clear/insert/replace_text | wc_l02_l06; wc_l19_l22; wc_l23_l30 |
| L9-L13 | resize_whole_lines/select/select_id/selected/selected_id | wc_l02_l06; wc_l07_l09 |
| L14-L18 | enable_sorting/suppression/sort_now/set_drop_byte/drop_byte | wc_l27_l31; wc_l23_l30; list_primary_bytes_and_exact_search_preserve_outputs_without_effects |
| L19-L25 | text_area/previous/next/first_page/last_page/previous_page/next_page | wc_l_residual_outputs_storage_and_bounded_bar_coordinates; wc_l19_l22 |
| L26-L34 | is_visible/make_visible/select_prefix/set_first; stored_count/row_height/whole_rows/first; hit_test | wc_l07_l09; wc_l23_l30 |
| L35-L38 | set_drop_policy_bytes/drop_policy_bytes/set_background/set_font | list_primary_bytes_and_exact_search_preserve_outputs_without_effects; wc_l10_l13 |
| L39-L46 | set_font_color/font_color/reset_font_color/set_selection_color/selection_color; highlight/set_intensity | wc_l10_l13; wc_l_residual_outputs_storage_and_bounded_bar_coordinates; wc_l27_l31 |
| L47-L50 | size_drop_height/size_drop_width/copy_text/search_with | wc_l23_l30; wc_l24_l34_l35_l36; wc_l_residual_outputs_storage_and_bounded_bar_coordinates |
| L51-L60 | set_flat/flat/set_three_d/three_d; set_outline_colors/outline_colors/set_bar_gaps/bar_gap/bar_gap_y/set_text_gutters/x_gutter/y_gutter | wc_l10_l13; wc_l_residual_outputs_storage_and_bounded_bar_coordinates |
| L61-L65 | set_minimum_height/set_bar_skin/false_stub/set_delivery/delivery_byte | list_residual_slot_adapters_colors_gutters_skin_and_base_flags; list_primary_bytes_and_exact_search_preserve_outputs_without_effects; wc_l14_l15 |
| W0/W4/W5/W148 | query_interface/init/shutdown/destroy | wc_l01; wc_l36_timer_is_destroyed_before_retained_resources |
| W68/W88 | set_flag; paint | list_residual_slot_adapters_colors_gutters_skin_and_base_flags; wc_l26_l31_l34 |
| W130/W131/W134/W136/W138 | key_down/key_up/mouse_down/mouse_up_at/mouse_move | wc_l14_l15; list_mouse_timer_strictness_and_no_font_background; wc_l24_l34_l35_l36 |
| W151-W159 | find_remove/character/set_border/copy_text/find_exact/recompute_metrics/text/remove_storage/insert_storage | list_primary_bytes_and_exact_search_preserve_outputs_without_effects; wc_l23_l30; wc_l_residual_outputs_storage_and_bounded_bar_coordinates; wc_f01 |
| W160-W168 | refresh_bar/update_bar; emit/complete_drop and selection/activation forwarding | wc_l14_l15; wc_l24_l34_l35_l36 |

Raw TreeView O40/O41 accesses use the modeled preceding words where defined.
Other raw object words require host-supplied readable/writable storage in
raw_property_words; no arbitrary memory read, fabricated default, color
conversion, retention or invalidation is introduced. Native container/pointer
corruption still has no safe traversal guarantee. The checked Init adapter
reports a stale-root precondition rather than promising a restart result.
