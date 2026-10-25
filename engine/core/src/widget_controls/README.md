# Widget control owners

Portable owners from widget-controls and its incorporated specs. No plugin or
C ABI wiring is included. Tests use recording foreign services; no game,
pixel or shadow comparison is claimed.

## Owner closure

Grid and TextEdit implement their settled owner specifications. FileBrowser
and TreeView retain the explicit boundaries below. Public fields supply the
thin stored-state accessors.
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
| 2 navigation/path | setters, parent, update_path_live | path/lifecycle tests; open: FBQ5 population targets |
| 3 lookup | get_row_of_entry_by_name | C06-C09 lookup test |
| 4 vectors C01-C09 | navigation/lookup tests | C05 native refresh targets remain FBQ5 |
| 5 flags | set/add/clear/any-bit | C10-C12 flags test |
| 6 selection/notifications | descriptor/selection/select/double-click | C14-C18 test; remaining ABI FBQ4 |
| 7 directory creation | create_new_directory | C14-C18; restoration FBQ5 |
| 8.1 population | populate_files, resolved columns/icons | C19 pattern test; open: FBQ5 sizing/drives/directories/restoration |
| 8.1.1 comparator | precedes | browser_compare_full_tags_byte_order_secondary_once_and_fit_boundary |
| 8.1.2 fitting | fit_string | same comparator/fit test |
| 8.2 controls | obtain_control factory/builtin prefix | browser_control_prefix_and_unsettled_init_do_not_claim_full_creation; open: FBQ2 full order/configuration/layout |
| 8.2.1 enables | enable_child | C23-C24 test |
| 8.3 resources | strip loader, creation cleanup | C20/C21/C25/C26 test; open: FBQ1 procedural pixels/results |
| 8.4 buffers | set_icon/set_button_buffer | C23/C24 and lifecycle tests |
| 8.5 lifecycle/paint/area | lifecycle.rs, set_area, shutdown | browser_init_paint_refresh_and_final_release_order; open: FBQ2 placement/controls, FBQ4 property targets, FBQ5 refresh targets |
| 8.6 construction/destruction | raw defaults/clears, destroy, BrowserOwner | shutdown-order and lifecycle tests distinguish direct destruction/final release |
| 8.7 properties/forwarding | strings/pending/sort/option/flags, explicit virtual dispatch | pending/flags/lifecycle tests; open: FBQ4 unspecified slot targets |
| 9 C10-C27 | existing focused vectors plus lifecycle tests | native C05/C14 refresh targets remain FBQ5 |
| 10/A inventory | no inferred owner denominator | open: FBQ1/FBQ2/FBQ4/FBQ5/FBQ6 |
| widget-controls 4 | same owner | rows above |

Unsettled returns explicit Unsupported errors at unresolved virtual calls.
Tests supply overrides to exercise independently settled Init continuation.
These overrides are not claimed implementations of missing browser bodies.
Paint accepts resolved background placement. Automatic native refresh cannot
be claimed while directory/drive population and restoration remain FBQ5.

### cGZWinListBox

Implemented column paths are relative to `engine/core/src/widget_controls/`.
This checkpoint applies settled contracts, but does not claim full owner closure.

| Spec section | Implemented code item / module path | Test / remaining work |
|---|---|---|
| widget-controls 5.1 identity/inventory | list_box.rs: query_interface, stored fields | wc_l01; residual ABI inventory still needs the complete adapter audit |
| 5.2 construction/resources | list_box.rs: new/init/shutdown/destroy | wc_l10_l13; wc_l24_l34_l35_l36; wc_l36_timer_is_destroyed_before_retained_resources |
| 5.3 entries/selection | list_box.rs: insert/remove/select/next_page/size_drop_height/size_drop_width | wc_l16_l22, wc_l23_l30, wc_l24_l34_l35_l36 |
| 5.4 messages/capture | list_box.rs: key methods, mouse_move/mouse_up_at, bar_tracking; list_box/adapters.rs: forward_bar | wc_l14_l15; wc_l24_l34_l35_l36; wc_l_residual_outputs_storage_and_bounded_bar_coordinates |
| 5.5 drawing | list_box.rs: paint/paint_plain_text/scan_rgb; mod.rs: Foreign fitting primitives | wc_l26_l31_l34; wc_l24_l34_l35_l36 |
| 5.6 residual contracts | list_box.rs: sorting/search/stored_count/intensity, signed storage helpers; list_box/adapters.rs: color/gutter/skin/output adapters | wc_l23_l30, wc_l26_l31_l34; wc_l_residual_outputs_storage_and_bounded_bar_coordinates; full slot-by-slot audit remains |
| 7 conformance | settled_tests.rs plus tests.rs | grouped WC-L19-L36 scenarios; exhaustive literal vector audit remains |
| 8 coverage | portable core only | no plugin/game/shadow claim; WC1-WC6 are settled, not open questions |

### cGZWinTreeView

| Spec section | Code item | Tests / open contract |
|---|---|---|
| widget-controls 6.1 identity/interfaces | tree_view.rs: query_interface/constants; tree_view/settled.rs: adapters | T01-T03; full inventory audit remains |
| 6.2 defaults/lifetime | tree_view.rs: new/internal_init/root/shutdown; tree_view/settled.rs: initialize_images | T01-T03, wc_t19_t26, wc_t27_t29_t36; restart retains stale root/init fields |
| 6.3 nodes/selection | tree_view.rs: insert/insert_at/recalculate/selection | T02-T06, wc_t13_t16, wc_t19_t26 |
| 6.4 input/messages | tree_view.rs: wheel/keys/activation/messages; tree_view/input.rs: mouse_button | T07-T11, wc_t37_t38, wc_t39_t43 |
| 6.5 drawing/layout | tree_view/rendering.rs: layout_viewport/cursor_row/content_width/render_item/paint_tree/connectors; tree_view.rs: draw_subimage/blit_viewport primitives | T12, wc_t13_t16, wc_t17_t18_t48_t51; wc_t18_centered_images_and_packed_drag_preserve_hit_geometry; automatic buffer/bar recalculation remains incomplete |
| 6.6 properties/descriptors/adapters | tree_view/settled.rs: property setters/descriptor/apply_descriptor/move_node/remove_node/images | wc_t19_t26, wc_t27_t29_t36, wc_t30_t35; raw negative property indices and complete ABI enumeration remain to audit |
| 6.7 input/drag/rendering | tree_view/input.rs: mouse_button/notify_drag/mouse_move/DragHelper, packed-row drawing with cached-hit preservation; tree_view/rendering.rs: fills/connectors | wc_t37_t38, wc_t39_t43, wc_t44_t47, wc_t17_t18_t48_t51; wc_t18_centered_images_and_packed_drag_preserve_hit_geometry; helper integration and exact vector coverage remain incomplete |
| 7 conformance | tests.rs and settled_tests.rs | old vectors plus grouped T13-T51 scenarios; not every literal vector is asserted yet |
| 8 coverage | explicit native preconditions, no WC Open branches | WC2/WC3/WC6 are settled; remaining work is implementation, not unanswered questions |

The WC1-WC6 unsupported placeholders have been replaced. This does not by itself
establish complete owner coverage: the implementation gaps above are still
settled obligations. Unsafe native navigation/index domains use explicit
Precondition results. Shared click state is passed by the host to mouse_button;
the convenience mouse_down method performs an isolated single click.
