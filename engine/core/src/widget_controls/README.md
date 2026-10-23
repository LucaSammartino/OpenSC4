# Widget control owners

Source: `docs/specs/widget-controls.md` and its incorporated owner specs.
This module extends the existing window/token and legacy property-interpreter
layers. It does not create OS windows or wire the plugin/C ABI. Foreign traits
expose individual reference, resource, timer, scrollbar, message and drawing
operations; control state and notification decisions live here.

Work in progress: ListBox and TreeView have state/message models and all of the
family vectors WC-L01–L15, WC-T01–T12 and WC-F01. Those tests use recording
foreign services and establish no native-game or pixel comparison. Grid has
sparse cell/format/size state helpers and all six incorporated vectors; its
native dispatch/recalculation orchestration, input and painting are unfinished.
TextEdit now has a byte/state stage, including encoding, composition, edits
and history; it records unexecuted layout/view repairs and is not a native
window implementation. FileBrowser
has a state/string layer plus explicit file enumeration/filter/sort/installation
with resolved column and icon inputs (`populate_files`). Automatic refresh still
records pending work; native column sizing, drive population and restoration
remain open FBQ5. It must not be used as a complete browser.
No owner-wide closure is claimed.

Round 2 adds FB-C19–C21/C25–C27: alternative pattern matching, borrowed icon-strip
copy/failed-lock behavior, buffer initialization cleanup and repeated filter
selection. Focused suite: 24 tests passed before the Grid implementation.

`TreeView::insert_at` is a semantic ordered-child-list mutation used to build
fixtures. It deliberately does not map native ordering sentinels or sibling
references; native `insert` reports WC2 until that mapping is specified.
Tree flattening literally includes root; WC6 asks which flags alter that rule.
ListBox `paint_plain_text` covers already-fitting ordinary text; `paint` reports
WC1/WC5 when a font would require unknown escape/ellipsis preprocessing.
Tree `draw_item` covers resolved rectangles and the no-image/text case; full
paint reports WC3 after balancing guarded buffer access. No guessed pixel
algorithm is presented as compatibility behavior.

## Owner closure

This is a progress table, not a completion claim. Section numbers are from
widget-controls unless another spec is explicitly named.

| Owner / spec section | Code item | Test or remaining work |
|---|---|---|
| All / 1 shared boundary | `Foreign`, `Handle`, `replace_ref` | `wc_l01_query_views_and_null_retention`, `wc_l10_l13_color_and_reference_quirks` |
| Grid / 2.1 identity/inventory | `grid::Grid`, identity/table constants, `query_interface` | Constructor exercised in Grid tests; explicit query/lifetime test still needed |
| Grid / 2.2 state/dispatch | State and sparse tables, selection/scroll/message state helpers | `grid_vectors_1_to_6_and_literal_selection_modes`; native recalculation dispatch/input still needed |
| Grid / 2.3 mutation/paint/failures | Cell values, ownership, movement, content measurement | `grid_append_copy_window_ownership_and_place_asymmetry`, `grid_size_cache_float_fraction_and_pause_dirty_contract`; full paint and native call orchestration still needed |
| TextEdit / 3.1 identity/inventory | `text_edit::TextEdit`, constants, `with_history_limit` | `te_selection_reversed_unsigned_endpoints_and_insert_mode_vectors`; open: TE9 default; QueryInterface/lifecycle not implemented |
| TextEdit / 3.2 state/input/output | Byte-stage methods and encoding/composition/history | Detailed rows below; layout/input adapter/painting not implemented |
| TextEdit / 3.3 failures | `Unsupported::Precondition` for raw invalid domains; foreign conversion primitives | `te_raw_limits_invalid_domains_and_no_history_or_notify`, `te_encoding_forward_domains_and_malformed_vectors_45_47`; open: TE5/TE8 |
| FileBrowser / 4.1 identity/defaults | `file_browser::FileBrowser::new`, raw offset facts, identity constants | `fb_c01_c05_identity_failed_navigation_and_ordered_refresh_work`, `browser_parent_directory_ids_and_shutdown_reference_order`; lifecycle construction still pending |
| FileBrowser / 4.2 dispatch/drawing/IO | partial state/string/ownership methods | See detailed incorporated-spec rows below; population/resources still pending |
| ListBox / 5.1 identity/inventory | `query_interface`, slot-count constants, `remove_storage`, `select` | `wc_l01_query_views_and_null_retention`, `wc_f01_storage_removal_is_not_selection`; open: WC1 adapters |
| ListBox / 5.2 construction/lifetime | `new`, `init`, `shutdown`, `destroy`, resource setters | `wc_l10_l13_color_and_reference_quirks`; Init/default/lifetime ordering needs more tests; open: WC1/WC4 drop sizing |
| ListBox / 5.3 entries/selection/geometry | insertion/replacement/removal, selection, metrics, hit test, visibility, bars | `wc_l02_l06_selection_indices_ids_and_insertions`, `wc_l07_l09_signed_metrics_hit_geometry`; open: WC1/WC4 removal/drop/page details |
| ListBox / 5.4 input/notifications | key, mouse and emitter methods | `wc_l14_l15_delivery_and_key_protocols`, `list_mouse_timer_strictness_and_no_font_background`; open: WC1 drop-bar capture |
| ListBox / 5.5 drawing/failures | `paint`, `paint_plain_text`, indexed colors | `wc_l10_l13_color_and_reference_quirks`, `list_mouse_timer_strictness_and_no_font_background`; open: WC1/WC5 text preprocessing/fractional fill |
| TreeView / 6.1 identity/inventory | `query_interface`, slot constants | `wc_t01_t03_identity_initialization_and_expansion`; open: WC2 adapters |
| TreeView / 6.2 state/lifetime | `new`, `internal_init`, `create_root`, `init`, `shutdown` | `wc_t01_t03_identity_initialization_and_expansion`, `tree_font_growth_shutdown_and_open_ordering`; open: WC2 restart, WC3 image init |
| TreeView / 6.3 nodes/selection | `insert`, `insert_at`, `set_descriptor`, `set_font`, expansion/selection/visibility | `wc_t04_t06_selection_and_ancestor_visibility`, `tree_font_growth_shutdown_and_open_ordering`; open: WC2 native ordering and descriptor ABI, WC6 root visibility |
| TreeView / 6.4 input/dispatch | `do_message`, `wheel`, `key_down/up`, expander `mouse_down` | `wc_t07_t09_wheel_sign_zero_and_missing_bar`, `wc_t10_activation_sequence_and_branch_toggle`, `wc_t11_bar_messages_always_delegate_to_base`; open: WC3 page/modifier/drag, WC6 navigation anchor |
| TreeView / 6.5 drawing | `recalculate`, `draw_item`, guarded `paint` | `wc_t12_missing_image_does_not_suppress_text`; open: WC3/WC6 layout/gradient/image failures |
| ListBox / 7 family vectors | tests WC-L01–L15, WC-F01 | All represented in named tests above |
| TreeView / 7 family vectors | tests WC-T01–T12 | All represented in named tests above |
| Grid / 7 incorporated vectors | win-grid vectors 1–6 | `grid_vectors_1_to_6_and_literal_selection_modes` |
| FileBrowser / 7 incorporated vectors | C01–C27 represented in focused tests | Known C05 automatic refresh remains incomplete; unproved branches FBQ1/2/4/5/6 |
| TextEdit / 7 incorporated vectors | Byte-stage distinguishing vectors | See TextEdit table; native layout/paint/clipboard/timer vectors not yet implemented |
| All / 8 unresolved contracts | `Unsupported::Open` with question IDs | Known-domain tests plus explicit open returns; owner completion still pending |

### Grid incorporated specification progress

These are state helpers, not a claim that a native slot adapter can bypass the
required recalculation hook. In particular `add_selection` returns whether the
state changed, while a complete owner must recalculate even when it did not.
`clear_cell` currently returns deselection status, not native ClearCell's true.

| win-grid section | Code item | Test or remaining work |
|---|---|---|
| 1 identity | `IID`, `PRIMARY_SLOTS`, `WINDOW_SLOTS`, `window`, `query_interface` | Add explicit view/query/lifetime test |
| 2 defaults | `Grid::new` | Grid tests construct and inspect state; unresolved format words: WG6 |
| 3 slot inventory | State helper methods only | No slot adapter; named-but-unspecified dependencies: WG5 |
| 4 flags | wrapping, outline, fixed ranges, listbox state | `grid_vectors_1_to_6_and_literal_selection_modes`; full SetFlags/capture still needed |
| 5.1 lifecycle/area/wheel | `shutdown`, `grid_area`, `cell_area`, `do_message`, `wheel`, `scroll_step` | `grid_append_copy_window_ownership_and_place_asymmetry`; Init/default formatting WG6, recalc/focus/input integration still needed |
| 5.2 fonts/colors/gutters | Default font/color/wrap/gutter/outline setters | `grid_vectors_1_to_6_and_literal_selection_modes`, `grid_size_cache_float_fraction_and_pause_dirty_contract`; recalc hooks still needed |
| 5.3 selection/limits/pause | `scroll_pages`, `set_selection_rules`, `enable_fixed_ranges`, `pause_recalculations` | All six §6 vectors; relative-scroll dependency WG5 |
| 5.4 dimensions | `size_sum` | Rectangle/layout tests; add signed overflow/count tests |
| 5.5 extras/headings | `set_extra`, `set_extra_object`, axis extras/headings | Vector 6; cell-extra object query/quirk still needed |
| 5.6 notifications/resources | `set_sound`, retained fields | Sound array raw-domain guard; owner-draw/buffer/drop-down adapters still needed |
| 5.7 helpers | Standard owned containers, explicit copy/ref logic | Native allocator implementation is foreign; comparator/sort wrappers still needed |
| 5.8 formats/scroll/sounds | `set_size`, `scroll_to`, `play_sound` | Width truncation/cache test; scrollbar images, cell object quirk and window lookup still needed |
| 5.9 selection/hit/input | `selected`, `add_selection`, `set_selection`, `act_like_listbox` | Selection modes tested; hit/mouse/key routing and notifications still needed; selection predicate WG5 |
| 5.10 layout/bars/sizes | `column_width`, `row_height`, `cell_height`, `reset_cached`, `recalculate_content`, `LayoutContext` | Fraction/depth/pause/cache test; scrollbar create/remove/value/position and recursive second pass still needed |
| 5.11 cells | `resolve_append`, `set_text` (plain), `set_bitmap`, `set_window`, `copy_cell`, `make_room`, `place_all_windows` | Ownership/append/copy/place test; editable text, find, row/column room wrappers and hook integration still needed |
| 5.12 painting | Not implemented | Known painting passes still needed; horizontal text alignment open WG6 |
| 5.13 rectangles/delete/copy | `cell_rect` | Place test exercises geometry; delete/copy row/column wrappers still needed |
| 5.14 formats | Typed `Format`/`SizeEntry` data only | Native setters still needed; unresolved copy-size words WG6 |
| 6 conformance | All six vectors | `grid_vectors_1_to_6_and_literal_selection_modes` |

### FileBrowser incorporated specification progress

| win-filebrowser section | Code item | Test or remaining work |
|---|---|---|
| 1 identity | `query_interface`, constants | `fb_c01_c05_identity_failed_navigation_and_ordered_refresh_work` |
| 2 navigation | directory setter, parent, `update_path` | Same test; actual refresh bodies pending (current `refresh_work` records order only) |
| 3 lookup | `get_row_of_entry_by_name` | `fb_c06_c09_grid_lookup_wraparound_and_ambiguous_zero` |
| 4 vectors C01–C09 | tests above | C05 refresh execution still pending; recorded calls are not population |
| 5 flags | `set_flags`, `add_flags`, `clear_flags`, `any_flag` | `fb_c10_c12_flag_precedence_and_any_bit_add` |
| 6 selection | selection handlers, descriptor/borrowed identities, `set_selection`, grid double-click | `fb_c14_c18_creation_selection_and_double_click`, `browser_parent_directory_ids_and_shutdown_reference_order` |
| 7 new directory | `create_new_directory` candidate loop | `fb_c14_c18_creation_selection_and_double_click`; actual refresh execution before lookup pending |
| 8.1 population | `populate_files`, `BrowserIo`, resolved `Column` inputs | `fb_c19_patterns_are_alternatives_and_directories_bypass_them`; automatic refresh integration, native sizing/icon mapping/restoration remain FBQ5 |
| 8.1.1 comparator | `precedes` | `browser_compare_full_tags_byte_order_secondary_once_and_fit_boundary`; add more secondary/directory-mode cases |
| 8.1.2 fit string | `fit_string` | Same test; add second-fit zero, empty marker and fit early-exit cases |
| 8.2 controls | Not implemented | Known control factory/fallback prefix still needed; open: FBQ2 remainder |
| 8.2.1 enables | `enable_child` | `fb_c23_c24_child_enable_and_reference_assignment` |
| 8.3 resources | `load_default_icon`, `create_default_icon`, `create_default_button_buffer`, `IconIo` | `fb_c20_c21_c25_c26_icon_loader_failure_and_ownership`; nonnull procedural recipes remain FBQ1 |
| 8.4 retained buffers | `set_icon`, `set_button_buffer` | `fb_c23_c24_child_enable_and_reference_assignment` |
| 8.5 lifecycle/paint | `shutdown` only | `browser_parent_directory_ids_and_shutdown_reference_order`; Init/setup/paint still needed; open: FBQ2/FBQ4 parts |
| 8.6 constructor/destruction/clears | constructor raw defaults; three clear methods | Same test; direct destruction, final-release and reference-count channels still needed |
| 8.7 properties/forwarders | pending, sort/option setters, flags/descriptor/identity | `fb_c10_c12_flag_precedence_and_any_bit_add`; remaining specified forwarders/getters and tests still needed |
| 9 vectors C10–C27 | C10–C12/C14–C18/C22–C24 represented | C13/C19–C21/C25–C27 still pending |
| 10/A inventory | No denominator/whole-owner claim | open: FBQ1/FBQ2/FBQ4/FBQ5/FBQ6 |
