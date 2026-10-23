# Lot configuration editor and chooser

Portable owner policy for `cSC4WinLotConfigurationEditor` and
`cSC4WinLotConfigurationChooser`, from `lot-configuration-editor.md` and its
inventory companion. This extends the existing module. It does not install a
window, expose C ABI entries, implement a configuration file format, or claim
ownership of the shared decoder/record constructor.

`Host`, `WindowHost`, `EditorWindowHost`, `MouseHost` and `EditorRenderer` are
foreign boundaries. Handles represent acquired references. Window effects are
ordered requests, not COM calls executed by the Rust core. The existing
`Editor` owns configuration/selection/edit state; `EditorWindow` adds its
reference and storage lifecycle. `Chooser` owns registrations, sort state and
close/delete policy. Generic window behavior remains foreign.

Chooser editing is deliberately staged: `begin_edit` creates/configures a
window and returns its temporary reference and Open key; the adapter invokes
core `Editor::open`, then passes its result and resulting `identity.id` to
`finish_edit`. An Unsupported result must remain Unsupported, never be turned
into success. Close dispatch invokes core `Editor::close` and applies chooser
notifications before returning to the live-set loop. The host does not decide
save/open/close policy. Failed Open cleanup removes, shuts down, and releases
the temporary; successful completion acquires one registration reference.

The renderer shell establishes acquisition, lock, phase, release and trigger
ordering. `RasterPhase` explicitly identifies still-open LE5 geometry. A real
adapter must return Unsupported for these phases unless a published algorithm
exists; recording renderers only test orchestration. Likewise pending
Title/Prepare3D/Refresh effects do not claim implementations of their open
helpers. Native unchecked null buffer/grid/allocation paths are represented by
errors, never by successful empty-state operations.

## Owner closure

Each row covers a spec section assigned to that owner. `open:` identifies the
published residual, not permission to infer its behavior. Tests are in
`tests.rs` and `window_tests.rs`. Inventory-only shared bodies remain foreign.

| Owner | Spec section | Code item | Test / explicit open boundary |
|---|---|---|---|
| Editor | §1 state/selection | `Editor::default`, `set_dirty`, `set_selection` | `pointer_identity_and_deletion_do_not_normalize_state`; title remainder open: LE5 |
| Editor | §2 edit popup | `edit_action`, `edit_area_menu`, `execute` | `separator_rows_and_direct_flags`; dependent commands open: LE4/LE5 |
| Editor | §3 object popup | `object_action`, `network_type` | `separator_rows_and_direct_flags`, `redraw_helper_remains_distinct_from_direct_draw_and_invalidate`; types 3/5, markers, edge table, names/help open: LE4/LE5 |
| Editor | §4 compatibility | `compatibility_dialog` | `compatibility_clears_hidden_bits_and_truncates_before_clamp`; initial fractional conversion/default lengths open: LE6 |
| Editor | §4.1 grids/acceptance | `compatibility_dialog` | `compatibility_clears_hidden_bits_and_truncates_before_clamp`; missing accepted grids are explicit faults |
| Editor | §5 vectors | selection, popup and dialog kernels | `pointer_identity_and_deletion_do_not_normalize_state`, `separator_rows_and_direct_flags`, compatibility tests |
| Editor | §6 inventory | `coverage.md`, this table | open: LE3–LE6; no whole-byte implementation claim |
| Editor | §7 interfaces/lifetime | `EditorWindow::{query_interface,init,message,post_init_networks,shutdown}` | `lew18_static_failure_is_not_retried_and_editor_messages_and_networks`, `lew17_editor_shutdown_releases_elements_before_destructor_storage`; view/alpha and exact shutdown fields open: LE5/LE8 |
| Editor | §8.1 new/read/open | `create_new`, `read`, `open`, `open_configuration_stage` | `read_failure_preserves_current_and_success_stops_at_explicit_dependency`; reconciliation bridge/defaults open: LE6, PostInit/size/3D open: LE4/LE5 |
| Editor | §8.2 save/Save As | `save`, `save_as`, `default_path` | `save_as_copy_restores_identity_but_not_dirty_or_title_and_releases_slots`, `save_as_failures_rollback_and_cancellation_never_install`; prompt/path edge details open: LE6 |
| Editor | §8.3 close | `close` | `save_failure_and_close_cancel_preserve_state` |
| Editor | §8.4 identities | `identity_from_path`, `identity_from_key`, `identity_from_type` | Save As reference/rollback tests; non-ASCII/splitter details open: LE6 |
| Editor | §9 record convention | `bounds`, shared `LotConfigObject` | `unordered_containment_and_independent_mover_tests`; cross-spec naming open: LE6 |
| Editor | §9.1 edits | `move_key`, `move_position`, `rotate`, `mirror` | `duplicate_candidates_front_selection_and_cleanup_containment`; remaining numeric cases open: LE3 |
| Editor | §9.2 mover/verify/cleanup | `move_within`, `verify_bounds`, `cleanup` | `unordered_containment_and_independent_mover_tests`, `duplicate_candidates_front_selection_and_cleanup_containment`; verification unordered predicate open: LE3 |
| Editor | §9.3 add/duplicate/delete | `duplicate`, `delete`, insertion kernels | `duplicate_third_and_untested_final_candidates`, `pointer_identity_and_deletion_do_not_normalize_state`; physical type 7 and bounds helpers open: LE3; deletion name open: LE5 |
| Editor | §9.4 containment/insertion | `contained`, `insert` | `unordered_containment_and_independent_mover_tests`; type-7 allocation open: LE3, shared constructor foreign |
| Editor | §9.5 fences/foundation | `toggle_fence`, `toggle_fence_pixels`, `foundation`, `add_foundation` | `fence_order_wrap_and_first_foundation`, `fence_preserves_extended_product_low_byte_and_rejects_invalid_conversion`; invalid conversion open: LE3 |
| Editor | §9.6 stores/unordered mover | `pixel_units`, `move_within` | `unordered_containment_and_independent_mover_tests`, fence conversion tests |
| Editor | §10 vectors | geometry, save and lifecycle kernels | duplicate, cleanup, Save As and close tests; full Open continuation open: LE4–LE6 |
| Editor | §11 key/edit mouse | `key_down`, `mouse_up` | `capture_keyboard_condition_equality_wrap_and_fence_mode`; hit assignment/drag conversion/cycling open: LE3/LE5 |
| Editor | §12 selectors | `selector_rows`, `selector_dialog` | `selector_duplicate_ids_preselection_current_text_append_and_cancel`; names/catalogue and sort interpretation open: LE4/LE5 |
| Editor | §12.1 callback/sort/search | `dialog_callback` | `callback_sorts_raw_options_and_scrolls_only_prop_first_byte` |
| Editor | §13 drawing subset | `draw_current`, `height_label` | `foundation_marker_lock_and_height_label_vectors`, `lew16_draw_failure_phase_order_release_and_trigger`; raster open: LE5 |
| Editor | §14 vectors | compatibility, selectors, geometry, rendering kernels | compatibility/selector/duplicate/unordered tests and height-label vectors; provider/path details open: LE6 |
| Editor | §15 vectors | fence/foundation/mover/style kernels | `fence_order_wrap_and_first_foundation`, fence conversion and unordered tests |
| Editor | §16 dialogs | `text_dialog`, `DialogKind::resource` | name/path/slope tests |
| Editor | §16.1 name/size/path | `name_dialog`, `size_dialog_request`, `size_dialog`, `network_path_dialog` | `name_missing_control_cancel_and_title_twice`, `path_growth_without_dirty_and_slope_limit_precedence`; SetLotSize body open: LE4 |
| Editor | §16.2 slope/fence | `slope_dialog`, `fence_menu` | path/slope and fence tests; preparation/help bodies open: LE5, literal placeholder wording open: LE6 |
| Editor | §16.3 vectors | dialog kernels | name/path/slope and size-request tests |
| Editor | §17 resources/style | `load_cursors`, `placement_mode`, `cycle_condition`, `drawing_style` | `capture_keyboard_condition_equality_wrap_and_fence_mode`; resource-construction fault has no safe success |
| Editor | §17.1 hit priority | `hit_candidates` | `hit_priority_uses_selected_then_vector_then_popup`; integer rectangle/fallback open: LE3/LE5 |
| Editor | §18 stopping boundary | explicit Unsupported barriers | open: LE3/LE4/LE5; shared helper attribution remains foreign |
| Editor | §19 slots | core methods above, no native layout imposed | `lew18_static_failure_is_not_retried_and_editor_messages_and_networks`; inherited slots foreign, unresolved extension 159 open: LE5 |
| Editor | §21 draw/mouse boundary | `mouse_wrapper`, `draw_current`, `trigger_redraw` | `lew15_mouse_half_open_child_region_and_e0_bypass`, `lew16_draw_failure_phase_order_release_and_trigger`; raster/edit interiors open: LE3/LE5 |
| Editor | §21.1 static/destruction | `EditorStaticResources::begin`, `EditorWindow::{init,destroy}` | `lew17_editor_shutdown_releases_elements_before_destructor_storage`, `lew18_static_failure_is_not_retried_and_editor_messages_and_networks`; static parsing open: LE5, Shutdown field detail open: LE8 |
| Editor | §22 LEW15–18 | window kernels above | LEW15/16/17/18 named tests |
| Chooser | §19 owner/window slots | `Chooser`, `ChooserAction` | `lew01_02_init_once_and_message_consumption`; inherited window slots foreign |
| Chooser | §19.1 method inventory | chooser methods below | all chooser methods have corresponding core operations; base QueryInterface foreign |
| Chooser | §20.1 lifecycle | `Chooser::{default,init,post_init,user_close,shutdown,destroy}` | `lew01_02_init_once_and_message_consumption`, `lew12_user_close_rechecks_callbacks_and_preserves_partial_progress`, `chooser_forced_shutdown_and_raw_destructor_are_distinct`; service discovery/acquisition open: LE7 |
| Chooser | §20.2 messages/sort | `message`, `sort` | `lew01_02_init_once_and_message_consumption`, `lew03_04_sort_rebuild_preserves_enumeration_order`; grid collation foreign/open: LE7 |
| Chooser | §20.3 rows/selection/key | `rebuild`, `selected_type`, `selected_key` | `lew03_04_sort_rebuild_preserves_enumeration_order`, `lew05_06_selection_reads_column_zero_and_preserves_failed_key`; lexical malformed hex foreign |
| Chooser | §20.4 edit/registration | `begin_edit`, `finish_edit`, `is_type_being_edited`, `editor_closed` | `lew07_08_no_selection_or_tracked_type_blocks_work`, `lew09_10_11_edit_creation_cleanup_and_pointer_registration`; core editor Open continuation open: LE4–LE6 |
| Chooser | §20.5 deletion | `delete_selected`, `delete_identity`, `remove_row` | `lew13_14_delete_failure_order_row_zero_and_false_return`; backing-store identity/search interpretation open: LE7; full file-error text open: LE8 |
| Chooser | §22 LEW01–14 | chooser kernels above | LEW01–14 named tests |

This table records closure of specified kernels and names all open sections.
It does **not** promote the spec's explicitly partial owner inventory to a
complete drop-in native window. No game verdict or shadow equality is claimed
for this core-only module.
