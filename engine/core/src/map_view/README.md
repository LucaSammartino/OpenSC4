# Map view owner

Portable `cSC4WinMapView`, from `docs/specs/map-view.md` only. This extends the
existing source, palette, highlight and overlay helpers; it does not introduce a
second owner. `MapView::select_with`, `draw_with`, `tick_with`, `button_with`,
`reset_with`, `show_with`, `window_message` and `init_with` execute the settled
interface contract. `Resources`, `WindowSystem`, `Renderer` and `Simulators` are
foreign traits. They return raw resources/grids/buildings or perform individual
interface operations, never compute owner palettes, source selection or timing.
There are no plugin, C ABI or native pointer changes.

`set_data_view`, `update`, `tick` and `init` remain intent/prefix inspection APIs
for existing callers/tests. Their bounded results are not successful native
operations. Use the `_with` methods for execution. All getters use shared
references; the owner is Send + Sync and contains no RefCell.

## Owner closure

Every row below belongs to `cSC4WinMapView`. Test names without a prefix are in
`engine/conformance/tests/map_view.rs`; `core::` refers to this module's
`tests.rs`. An open entry is a measured boundary, not a claim of native closure.

| Spec section | Code item | Conformance / open boundary |
|---|---|---|
| §0 Summary | `select_with`, `tick_with` | `mayor_rating_owner_builds_and_draws_both_extremes`, `successful_redraw_acknowledges_once_and_failed_present_retains_retry`; open: QV2, QV5, QV7, QV8 |
| §1 Window, construction, activation, panels | `new`, `init_with`, `window_message`, `show_with`, `panel_request` | `window_lifecycle_commands_and_open_panel_matrix_are_explicit`, `qv12_init_false_primitives_and_missing_server_latch`, `traffic_group_known_toggle_branch_and_show_reset_are_not_delegated`; open: QV3 (panel matrix), QV9 (native defaults/repeated init) |
| §2 Exemplar | `RawExemplar`, `SelectionFields`, `HighlightStyle::from_exemplar`, `render_settings`, `build_palettes`, `select_with` | `core::raw_fields_use_only_documented_defaults_and_ranges`, `filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry`, `exact_ramps_premultiply_and_refuse_unknown_rounding_or_endpoint_rules`, `qv13_independent_footprints_rings_and_effect_transforms`; open: QV6, QV8 |
| §3 Choosing a view | `select_with`, `button_with`, `reset_with` | Tests and open boundaries in §§3.1-3.3 below |
| §3.1 SetDataView | `begin_selection`, `store_selection`, `select_with` | `qv12_selection_guards_and_visible_reset`, `qv12_failed_button_replacement_remembers_requested_legend`, `qv12_custom_initialization_factory_and_child_false_are_local`, `inactive_same_view_and_foreign_failures_do_not_report_a_completed_selection`; open: QV1 (none/reset calls), QV6-8 |
| §3.2 Buttons and input messages | `button_action`, `canonical_button`, `button_with`, `window_message` | `direct_buttons_aliases_tutorial_reset_and_legend`, `qv12_failed_button_replacement_remembers_requested_legend`, `window_lifecycle_commands_and_open_panel_matrix_are_explicit`; open: QV3 (group memory, panel policy) |
| §3.3 ResetDataView | `reset_with` | `qv12_selection_guards_and_visible_reset`, `qv12_failed_button_replacement_remembers_requested_legend`, `traffic_group_known_toggle_branch_and_show_reset_are_not_delegated`; open: QV3 (unknown group) |
| §4 Drawing contract | `draw_with`, `RenderCommand::Bitmap`, `RawMapGrid` | `mayor_rating_owner_builds_and_draws_both_extremes`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV4-5, QV8, QV10-11 |
| §4.1 Sources and modes | `select_source`, `draw_with` | `core::every_raw_source_slot_and_empty_range_has_a_literal_fixture`, `core::every_developer_type_and_published_layer_is_discriminated`, `residential_desirability_selection_commits_but_draws_nothing`, `filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry`; open: QV2 (traffic volume), QV4 (crime/layer 5), QV11 (developer layer 1/debug construction) |
| §4.2 Palette indices | `u8_index`, `i8_index`, `float_index`, `emit_colors`, `premultiply_argb` | `core::full_eight_bit_domains_use_distinct_supplied_entries`, `core::floats_truncate_before_range_check_and_refuse_unpublished_domains`, `exact_ramps_premultiply_and_refuse_unknown_rounding_or_endpoint_rules`; open: QV5 (all 16-bit conversions), QV10 (nonfinite/out-of-range floats) |
| §4.3 Interpolation, lots, overlay | `draw_with`, `OverlayState::plan` | `interpolation_is_only_required_for_a_coarser_grid_and_overlay_stays_qv10`, `core::overlay_registers_once_and_never_guesses_geometry_or_cell_inclusion`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV8 (coarse resampling), QV4 (lot/crime/traffic markers), QV10 (owner cell geometry/inclusion); explicit-cell overlay helper does not bypass these |
| §5 Highlights | `highlight_request`, `collect_highlights`, `poll_highlights`, `update_highlights` | `core::all_supported_highlight_kinds_select_distinct_raw_inputs_and_slots`, `qv13_hotspot_position_and_strength_independence`, `qv13_hotspot_repeated_removal_empty_name_and_kind_zero`; open: QV7 (building comparison, other kind-0 state, kinds 6-8 radius); hotspot reconciliation is settled |
| §5.1 Named effects and crime hotspot | `poll_highlights`, `HighlightRecord::{create_effect,add,remove}`, `HighlightStyle` | `qv13_hotspot_two_stage_effect_creation`, `qv13_hotspot_repeated_removal_empty_name_and_kind_zero`, `qv13_hotspot_creation_failures_do_not_abort_selection_or_bitmap`, `qv13_common_building_effect_and_reference_primitives`; open: QV7 only for general building-set comparison |
| §5.2 Bitmap highlight colours | `decorate_highlights`, `footprint_rect`, `ring_arguments`, `paint_ring`, `blend_highlight` | `qv13_independent_footprints_rings_and_effect_transforms`, `qv13_ring_blend_clip_and_independent_acquisitions`; open: QV10 (invalid float domains), QV14 (exact x87 edge-pixel rounding); explicit building records test decoration without claiming QV7 reconciliation |
| §6 Legend | `Legend::from_exemplar`, `display_legend` | `qv12_legend_scalar_default_lookup_failure_and_title_query_skip`, `qv12_legend_unequal_rows_zero_colors_and_control_skips`; open: QV6 (other missing/invalid properties); native one-past-end reads produce explicit safe diagnostics |
| §7 Refresh, input notifications, throttle | `subscription`, `notify`, `hook_update_messages`, `tick_with`, `draw_with` | `core::messages_and_source_changes_follow_exact_subscription_table`, `core::notifications_dirty_only_when_the_raw_value_changes`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion`, `qv12_custom_initialization_factory_and_child_false_are_local`, `qv13_hotspot_repeated_removal_empty_name_and_kind_zero`; open: QV7 (building dirty decisions), QV9 (clock/defaults/empty-update commit) |
| §8 Quirks | `select_source`, `selection_fields`, `tick_with` | `core::raw_fields_use_only_documented_defaults_and_ranges`, `residential_desirability_selection_commits_but_draws_nothing`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion` |
| §9 Open questions | `Unsupported`, `OwnerError`, this table | QV1-QV11 and QV14 remain bounded; QV12/QV13 are implemented; no exact game shadow is claimed |
| §10 Conformance tests | `engine/conformance/tests/map_view.rs` | §10.1 `conformance_index_conflict_is_explicit_qv5` (open: QV5); §10.2 `mayor_rating_owner_builds_and_draws_both_extremes`; §10.3 `pollution_throttle_vector_reaches_due_tick_without_faking_conversion` (actual pollution redraw open: QV5); §10.4 `residential_desirability_selection_commits_but_draws_nothing` (bitmap disposition open: QV9); all twelve §10.5-16 vectors map individually below |

| Vector | Test |
|---|---|
| §10.5 | `qv12_selection_guards_and_visible_reset` |
| §10.6 | `qv12_failed_button_replacement_remembers_requested_legend` |
| §10.7 | `qv12_custom_initialization_factory_and_child_false_are_local` |
| §10.8 | `qv12_init_false_primitives_and_missing_server_latch` |
| §10.9 | `qv12_legend_scalar_default_lookup_failure_and_title_query_skip` |
| §10.10 | `qv12_legend_unequal_rows_zero_colors_and_control_skips` |
| §10.11 | `qv13_hotspot_position_and_strength_independence` |
| §10.12 | `qv13_hotspot_two_stage_effect_creation` |
| §10.13 | `qv13_hotspot_repeated_removal_empty_name_and_kind_zero` |
| §10.14 | `qv13_hotspot_creation_failures_do_not_abort_selection_or_bitmap` |
| §10.15 | `qv13_independent_footprints_rings_and_effect_transforms` |
| §10.16 | `qv13_ring_blend_clip_and_independent_acquisitions` |

## Supported execution and limits

Full-range ordered ramps whose linear channel values are exact integers produce
both owned 256-entry tables. This includes a 0-to-255 black/white ramp and constant
ramps. Fractional channel rounding, missing endpoints, duplicate/unordered stops
remain QV8; supplied helper palettes cannot complete owner selection.

Selection shuts down/releases only a successfully created custom-view handle,
then drops update subscriptions before reading the next resource. It applies
filter/terrain/lot/zone settings, polls highlights, draws, sets child 0x4203's flag,
commits the current view and hooks the exact source message/priority. Custom
initialization and updates have no arguments; ordinary false returns do not gate
completion. Missing exemplars preserve the old identity/source after teardown.
Buttons and visible resets load their requested legend afresh even on failure.

Source grids retain their foreign dimensions/cell sizes, including weather's own
cells. Bitmap class/IID/32-bit format and premultiplied pixels cross the renderer
trait. Resampling is required only if interpolation is enabled AND the raw grid
is coarser than city cells. Unsupported transforms never reach the renderer.
Empty sources do not obtain a grid or pretend an old bitmap was cleared.

Ticks poll raw highlight inputs even while throttled. A known dirty map can draw
after 1000 ms (250 ms, always dirty, for source 76). Successful bitmap presentation
acknowledges that redraw; an adapter failure, unsupported conversion, inactive or
empty source preserves dirty/time state. Empty-source clock semantics remain
QV9. General building comparison remains QV7; hotspot replacement/removal and
kind-zero clearing of retained effects now have owner decisions. Named effects
are independent of footprint/ring colours. Footprint and ring buffer failures
skip only their own pass/ring, and acquired buffers are released. Ring numeric
mode equivalence is QV14; the stated binary32 stores and published vectors work.

Normally returning false primitives follow their specified local continuation.
`OwnerError::Foreign` represents invalid adapter/service inputs, faults or
exceptions, not ordinary native false results; it retains the completed prefix.
Unsafe legend reads/layouts may be diagnosed as permitted by §6. No game run is
applicable until a separate lane wires these interfaces into the plugin.
