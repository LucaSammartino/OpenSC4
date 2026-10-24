# cGZWinGrid

The column is implemented in `engine/core/src/widget_controls/grid.rs`,
`grid/data.rs`, `grid/runtime.rs` and `grid/paint.rs`. State-stage methods remain
available for callers constructing fixtures. `mutate`, `message`, `input` and
`recalculate` execute the live host/recalculation path. `GridHost` extends the
existing widget `Foreign` and `GridDataForeign` service boundaries; `PaintHost`
adds drawing primitives and owner-draw callbacks. There is no ABI/plugin wiring.

Effective format inheritance and default-format initialization remain open.
`paint` therefore requires resolved formats rather than inventing merge rules.
Unknown text alignments return `Unsupported::Open("WG6")`. Init retains its
specified true result; `default_formatting` separately reports WG6. The raw
native negative sound-index memory overwrite is represented as an invalid
precondition, never an unsafe Rust memory write.

## Owner closure

Each row names cGZWinGrid; incorporated widget-controls sections apply to that
same owner. Tests without the `owner_` prefix are in `widget_controls/tests.rs`;
the additional owner tests are in `grid/owner_tests.rs`.

| Spec section / owner | Code item | Test or open contract |
|---|---|---|
| win-grid 1 identity | `IID`, `query_interface`, `window` | `grid_identity_clear_result_extra_queries_owner_draw_and_window_text` |
| win-grid 2 construction/layout | `Grid::new`, `SizeEntry`, `Format`, typed `Value` | `grid_vectors_1_to_6_and_literal_selection_modes`; open: WG6 default formatting |
| win-grid 3 interface inventory | Rust methods and opaque handles; no native vtable export | owner methods below; open: WG8 pending slot bodies |
| win-grid 4 flags | `set_flags`, `set_window_flag`, wrapping/outline/fixed-range state | `owner_layout_bars_images_failure_pause_and_second_pass`, `owner_live_setters_formats_helpers_and_resources` |
| win-grid 5.1 lifecycle/area/messages | `init`, `release`, `shutdown`, `set_area_live`, `message`, `input` | `owner_editable_text_reference_balance_init_and_font`, `owner_keys_arrows_message_focus_and_dropdown_capture`; open: WG6 initializer |
| win-grid 5.2 fonts/colors/gutters | state setters plus `mutate` live hook, shared default color/format storage | `grid_vectors_1_to_6_and_literal_selection_modes`, `owner_live_setters_formats_helpers_and_resources`, `owner_default_format_and_color_slots_share_storage`; WG3 delayed cell-gutter effect preserved |
| win-grid 5.3 scrolling/selection/limits | `scroll_pages`, `mutate`, `set_selection_rules`, `enable_fixed_ranges`, `pause_recalculations` | all six spec vectors; open: WG5 relative-scroll body/bounds |
| win-grid 5.4 dimension sums | `size_sum` wrapping signed sums | `grid_size_cache_float_fraction_and_pause_dirty_contract`, rectangle tests |
| win-grid 5.5 extra/headings | `data.rs` extra queries, `heading`, `set_heading` | `grid_identity_clear_result_extra_queries_owner_draw_and_window_text`, copy-axis tests |
| win-grid 5.6 resources/notification/buffer | `set_owner_draw`, `buffer_to_draw_to`, `set_sound`, `make_drop_down` | identity test and `owner_live_setters_formats_helpers_and_resources` |
| win-grid 5.7 helpers/thunks | Rust maps/vectors, `signed_gcd`, `equal_record`, `stable_sort_values`, `compare_cell_values` | `owner_live_setters_formats_helpers_and_resources`; open: WG8 comparator locale/encoding and SortRow/SortColumn owner bodies |
| win-grid 5.8 sizes/explicit scroll/images/sounds | `set_size`, `scroll_to`, `first_selection`, `set_bar_image`, `set_cell_extra_object`, `play_sound` | `owner_layout_bars_images_failure_pause_and_second_pass`, live-setter test; WG4 store quirk preserved; WG5 relative scrolling open |
| win-grid 5.9 hit/selection/mouse/key/focus | `hit_test`, `input`, `mouse`, `arrow`, `key_down`, `select_live` | `owner_hit_headers_inclusive_edges_and_input_notifications`, `owner_keys_arrows_message_focus_and_dropdown_capture`; open: WG5 selected predicate |
| win-grid 5.10 bars/layout/sizes | `add_bars`, `remove_bars`, `test_bars`, `bar_values`, `bar_positions`, `recalculate`, `calculate_visible`, size caches | `owner_layout_bars_images_failure_pause_and_second_pass`, existing fractional/cache/depth test; open: WG5 effective format used by text height |
| win-grid 5.11 data/windows/movement | `set_text`, `set_editable_text`, `set_bitmap`, `set_window`, `copy_cell`, `make_room`, `place_all_windows`, disposal and text search | existing ownership/search/movement tests, `owner_editable_text_reference_balance_init_and_font`; open: WG7 failed editor creation, WG5 bounds |
| win-grid 5.12 painting | `paint`, `paint_background`, `paint_content`, `paint_text`, `PaintRecord`, `PaintOp` | `owner_paint_layers_clips_selection_owner_veto_and_open_alignment`; open: WG5 resolved formats, WG6 horizontal text alignment, WG9 header geometry/content-right field |
| win-grid 5.13 rectangles/delete/copy | `cell_rect`, axis delete/copy, `mutate` delete hook | `grid_copy_axis_overlap_holes_headings_extras_and_move_order`, `grid_delete_axis_modes_and_default_per_cell_format_ownership` |
| win-grid 5.14 format/font | `set_cell_format_state`, `set_cell_font`, `mutate` | existing format ownership test, `owner_editable_text_reference_balance_init_and_font`; open: WG6 axis-format size words |
| win-grid 6 conformance | six literal vectors | `grid_vectors_1_to_6_and_literal_selection_modes` |
| win-grid 7 / appendix A | explicit open contracts listed here | WG3–WG9; appendix pending bodies are not credited as closed |
| widget-controls 1 shared boundary | `Foreign`/`GridHost` retain/release, base delegation | identity, ownership, message and editable-text tests |
| widget-controls 2.1 inventory | IID/views and Rust owner API | identity test; open: WG8 pending owner methods |
| widget-controls 2.2 construction/state/dispatch | `Grid::new`, `init`, `shutdown`, `message`, `input` | vectors and owner lifecycle/key tests; open: WG5/WG6 |
| widget-controls 2.3 drawing/data/failures | typed data, `PaintHost`, explicit Unsupported | owner paint and editable tests; open: WG5–WG9 as above |
| widget-controls 7 conformance | incorporates win-grid six vectors (no additional WC-G vectors) | `grid_vectors_1_to_6_and_literal_selection_modes` |
| widget-controls 8 unresolved coverage | no credit for pending bodies | WG5–WG9 |

An in-game shadow is feasible next for pure geometry, selection and event traces
through a separate coordinator-owned plugin probe. Full drawing parity needs
WG5/WG6/WG9 answers and an actual renderer adapter. This module does not enqueue
a game run or claim an in-game comparison.
