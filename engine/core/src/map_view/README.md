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
| §1 Window, construction, activation, panels | `new`, `init_with`, `window_message`, `show_with`, `panel_request` | `window_lifecycle_commands_and_open_panel_matrix_are_explicit`, `inactive_same_view_and_foreign_failures_do_not_report_a_completed_selection`, `traffic_group_known_toggle_branch_and_show_reset_are_not_delegated`; open: QV3 (panel matrix), QV9 (native defaults/repeated init), QV12 (native failures) |
| §2 Exemplar | `RawExemplar`, `SelectionFields`, `render_settings`, `build_palettes`, `select_with` | `core::raw_fields_use_only_documented_defaults_and_ranges`, `core::typed_resource_helpers_do_not_supply_filter_or_traffic_outcomes`, `filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry`, `exact_ramps_premultiply_and_refuse_unknown_rounding_or_endpoint_rules`; open: QV6, QV8, QV12 |
| §3 Choosing a view | `select_with`, `button_with`, `reset_with` | Tests and open boundaries in §§3.1-3.3 below |
| §3.1 SetDataView | `begin_selection`, `store_selection`, `select_with` | `mayor_rating_owner_builds_and_draws_both_extremes`, `custom_owned_handle_shutdown_precedes_subscription_drop_and_missing_lookup`, `inactive_same_view_and_foreign_failures_do_not_report_a_completed_selection`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV1 (none/reset calls), QV6-8, QV12 |
| §3.2 Buttons and input messages | `button_action`, `canonical_button`, `button_with`, `window_message` | `direct_buttons_aliases_tutorial_reset_and_legend`, `window_lifecycle_commands_and_open_panel_matrix_are_explicit`, `traffic_group_known_toggle_branch_and_show_reset_are_not_delegated`; open: QV3 (group memory, panel policy), QV12 (selection failure continuation) |
| §3.3 ResetDataView | `reset_with` | `direct_buttons_aliases_tutorial_reset_and_legend`, `traffic_group_known_toggle_branch_and_show_reset_are_not_delegated`; open: QV3 (unknown group), QV12 (same-view guard) |
| §4 Drawing contract | `draw_with`, `RenderCommand::Bitmap`, `RawMapGrid` | `mayor_rating_owner_builds_and_draws_both_extremes`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV4-5, QV8, QV10-11 |
| §4.1 Sources and modes | `select_source`, `draw_with` | `core::every_raw_source_slot_and_empty_range_has_a_literal_fixture`, `core::every_developer_type_and_published_layer_is_discriminated`, `residential_desirability_selection_commits_but_draws_nothing`, `filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry`; open: QV2 (traffic volume), QV4 (crime/layer 5), QV11 (developer layer 1/debug construction) |
| §4.2 Palette indices | `u8_index`, `i8_index`, `float_index`, `emit_colors`, `premultiply_argb` | `core::full_eight_bit_domains_use_distinct_supplied_entries`, `core::floats_truncate_before_range_check_and_refuse_unpublished_domains`, `exact_ramps_premultiply_and_refuse_unknown_rounding_or_endpoint_rules`; open: QV5 (all 16-bit conversions), QV10 (nonfinite/out-of-range floats) |
| §4.3 Interpolation, lots, overlay | `draw_with`, `OverlayState::plan` | `interpolation_is_only_required_for_a_coarser_grid_and_overlay_stays_qv10`, `core::overlay_registers_once_and_never_guesses_geometry_or_cell_inclusion`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV8 (coarse resampling), QV4 (lot/crime/traffic markers), QV10 (owner cell geometry/inclusion); explicit-cell overlay helper does not bypass these |
| §5 Highlights | `highlight_request`, `collect_highlights`, `poll_highlights`, `update_highlights` | `core::all_supported_highlight_kinds_select_distinct_raw_inputs_and_slots`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion`; open: QV7 (comparison, kinds 0/6-8 radius), QV13 (crime hotspot/effect contract) |
| §6 Legend | `Legend::from_exemplar`, `display_legend` | `direct_buttons_aliases_tutorial_reset_and_legend`, `window_lifecycle_commands_and_open_panel_matrix_are_explicit`, `malformed_legend_grid_lot_and_none_paths_have_explicit_boundaries`; open: QV6 (missing/invalid properties), QV12 (title cardinality/unequal rows) |
| §7 Refresh, input notifications, throttle | `subscription`, `notify`, `hook_update_messages`, `tick_with` | `core::messages_and_source_changes_follow_exact_subscription_table`, `core::notifications_dirty_only_when_the_raw_value_changes`, `core::ordinary_and_traffic_tick_thresholds_retain_dirty_on_missing_renderer`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion`, `successful_redraw_acknowledges_once_and_failed_present_retains_retry`; open: QV7 (highlight dirty decisions), QV9 (clock/defaults/empty-update commit), QV12 (foreign failure) |
| §8 Quirks | `select_source`, `selection_fields`, `tick_with` | `core::raw_fields_use_only_documented_defaults_and_ranges`, `residential_desirability_selection_commits_but_draws_nothing`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion` |
| §9 Open questions | `Unsupported`, `OwnerError`, this table | QV1-QV13 remain explicitly bounded; no native success or exact game shadow is claimed |
| §10 Conformance tests | `engine/conformance/tests/map_view.rs` | §10.1 `conformance_index_conflict_is_explicit_qv5` (open: QV5); §10.2 `mayor_rating_owner_builds_and_draws_both_extremes`; §10.3 `pollution_throttle_vector_reaches_due_tick_without_faking_conversion` (deadline tested; actual pollution redraw open: QV5); §10.4 `residential_desirability_selection_commits_but_draws_nothing` (old bitmap disposition open: QV9) |

## Supported execution and limits

Full-range ordered ramps whose linear channel values are exact integers produce
both owned 256-entry tables. This includes a 0-to-255 black/white ramp and constant
ramps. Fractional channel rounding, missing endpoints, duplicate/unordered stops
remain QV8; supplied helper palettes cannot complete owner selection.

Selection shuts down/releases only a successfully created custom-view handle,
then drops update subscriptions before reading the next resource. It applies
filter/terrain/lot/zone settings, polls highlights, draws, sets child 0x4203's flag,
commits the current view and hooks the exact source message/priority. Custom
attachment beyond the specified class/IID is not invented. Raw properties retain
the plop name and highlight colors whose further use is not specified.

Source grids retain their foreign dimensions/cell sizes, including weather's own
cells. Bitmap class/IID/32-bit format and premultiplied pixels cross the renderer
trait. Resampling is required only if interpolation is enabled AND the raw grid
is coarser than city cells. Unsupported transforms never reach the renderer.
Empty sources do not obtain a grid or pretend an old bitmap was cleared.

Ticks poll raw highlight inputs even while throttled. A known dirty map can draw
after 1000 ms (250 ms, always dirty, for source 76). Successful bitmap presentation
acknowledges that redraw; an adapter failure, unsupported conversion, inactive or
empty source preserves dirty/time state. Empty-source clock semantics remain
QV9. Highlight comparison remains QV7 and is reported on every execution result;
an adapter cannot fabricate a completed highlight comparison.

Native failure rollback is unspecified. Portable execution stops on an opaque
foreign error and retains the completed prefix (QV12). It does not translate that
error to a native success/failure code. No game run is applicable until a separate
lane wires these interfaces into the plugin.
