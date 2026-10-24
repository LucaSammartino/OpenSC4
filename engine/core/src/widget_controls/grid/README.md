# cGZWinGrid

The column is implemented in `engine/core/src/widget_controls/grid.rs`,
`grid/data.rs`, `grid/runtime.rs`, `grid/contracts.rs` and `grid/paint.rs`. State-stage methods remain
available for callers constructing fixtures. `mutate`, `message`, `input` and
`recalculate` execute the live host/recalculation path. `GridHost` extends the
existing widget `Foreign` and `GridDataForeign` service boundaries; `PaintHost`
adds drawing primitives and owner-draw callbacks. There is no ABI/plugin wiring.

WG5–WG9 are implemented from the settled §§5.15–5.16. `paint_owner` resolves
formats per field in default/row/column/cell order; `paint` also accepts supplied
resolved formats. Initialization obtains fonts and RGB conversion through the
host. Default sorting uses host CRT CTYPE comparison; `compare_c_locale` supplies
the C/default locale implementation. Native invalid-memory operations (including
WG7's terminal null-editor fault) are explicit `Unsupported::Precondition`
results, never successful boolean outcomes or unsafe memory writes.

## Owner closure

Each row names cGZWinGrid; incorporated widget-controls sections apply to that
same owner. Tests without the `owner_` prefix are in `widget_controls/tests.rs`;
the additional owner tests are in `grid/owner_tests.rs`.

| Spec section / owner | Code item | Test or open contract |
|---|---|---|
| win-grid 1 identity | `IID`, `query_interface`, `window` | `grid_identity_clear_result_extra_queries_owner_draw_and_window_text` |
| win-grid 2 construction/layout | `Grid::new`, `SizeEntry`, sentinel `Format`, typed `Value` | original six vectors, WG6-a |
| win-grid 3 interface inventory | Rust methods and opaque handles; no native vtable export | owner methods below, WG8-a–f |
| win-grid 4 flags | `set_flags`, `set_window_flag`, wrapping/outline/fixed-range state | `owner_layout_bars_images_failure_pause_and_second_pass`, `owner_live_setters_formats_helpers_and_resources` |
| win-grid 5.1 lifecycle/area/messages | `init`, `default_formatting`, `release`, `shutdown`, `set_area_live`, `message`, `input` | lifecycle/key tests, WG6-a |
| win-grid 5.2 fonts/colors/gutters | state setters plus `mutate` live hook, shared default color/format storage | `grid_vectors_1_to_6_and_literal_selection_modes`, `owner_live_setters_formats_helpers_and_resources`, `owner_default_format_and_color_slots_share_storage`; WG3 delayed cell-gutter effect preserved |
| win-grid 5.3 scrolling/selection/limits | `scroll_cells`, `scroll_pages`, `mutate`, selection rules, fixed ranges, pause | original six vectors, WG5-a–d |
| win-grid 5.4 dimension sums | `size_sum` wrapping signed sums | `grid_size_cache_float_fraction_and_pause_dirty_contract`, rectangle tests |
| win-grid 5.5 extra/headings | `data.rs` extra queries, `heading`, `set_heading` | `grid_identity_clear_result_extra_queries_owner_draw_and_window_text`, copy-axis tests |
| win-grid 5.6 resources/notification/buffer | `set_owner_draw`, `buffer_to_draw_to`, `set_sound`, `make_drop_down` | identity test and `owner_live_setters_formats_helpers_and_resources` |
| win-grid 5.7 helpers/thunks | Rust maps/vectors, signed gcd, stable sorting, comparator boundary | live-setter/helpers test, WG8-c–f; containers preserve order and payload ownership |
| win-grid 5.8 sizes/explicit scroll/images/sounds | size/scroll/image/sound methods and cell-extra object | layout/live-setter tests; WG4 store quirk preserved; WG5-d |
| win-grid 5.9 hit/selection/mouse/key/focus | `hit_test`, `input`, `mouse`, `arrow`, `key_down`, `select_live`, `selected` | hit/key tests, WG5-c |
| win-grid 5.10 bars/layout/sizes | bars, layout, fractional visibility, sizes, effective cell font | layout/cache/depth tests, WG5-d/e, WG8-b |
| win-grid 5.11 data/windows/movement | text/editor/bitmap/window setters, copy/move/disposal/search | existing ownership/search/movement tests, editor reference test, WG7-a |
| win-grid 5.12 painting | `paint_owner`, `paint`, `header_rect`, paint helpers and operations | paint tests, WG5-e, WG6-c/d, WG9-a–c |
| win-grid 5.13 rectangles/delete/copy | `cell_rect`, axis delete/copy, `mutate` delete hook | `grid_copy_axis_overlap_holes_headings_extras_and_move_order`, `grid_delete_axis_modes_and_default_per_cell_format_ownership` |
| win-grid 5.14 format/font | default/axis/cell format copying, font setter, live mutation | existing ownership tests, WG6-b |
| win-grid 5.15 settled dependencies | bounds/used-range, membership, relative scroll, merge, initialization, text/header geometry, fault | WG5-a–e, WG6-a–d, WG7-a, WG9-a–c |
| win-grid 5.16 remaining owner bodies | partial setters, extra data, resize, SortColumn/SortRow, locale service, Rust container operations | WG8-a–f |
| win-grid 6 conformance | six original plus 19 settled vectors | original vector test and `grid/vectors.rs` (each vector named by ID) |
| win-grid 7 / appendix A | all 198 specified owner functions mapped to the portable owner/contracts and equivalent Rust containers | WG5–WG9 closed; WG3 delayed gutter and WG4 extra-object quirks preserved |
| widget-controls 1 shared boundary | `Foreign`/`GridHost` retain/release, base delegation | identity, ownership, message and editable-text tests |
| widget-controls 2.1 inventory | IID/views and Rust owner API | identity test, WG8-a–f |
| widget-controls 2.2 construction/state/dispatch | `Grid::new`, `init`, `shutdown`, `message`, `input` | original/settled vectors and lifecycle/key tests |
| widget-controls 2.3 drawing/data/failures | typed data, `PaintHost`, explicit native fault preconditions | paint/editor tests, WG7-a, WG9-a–c |
| widget-controls 7 conformance | incorporates win-grid six vectors (no additional WC-G vectors) | `grid_vectors_1_to_6_and_literal_selection_modes` |
| widget-controls 8 unresolved coverage | settled owner contracts implemented | WG3/WG4 preserved compatibility questions only |

The cGZWinGrid portable owner is fully implemented against the current spec.
`GridOwner` supplies owning final-release semantics: virtual Shutdown, the
destructor's second Shutdown, then storage destruction. The non-owning
`Grid::release` remains available when an embedding controls allocation.
This is core conformance, not an in-game shadow claim: a separate plugin/renderer
adapter must supply the host operations and any game comparison.
