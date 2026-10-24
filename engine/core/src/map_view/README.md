# Map view owner

`cSC4WinMapView` is implemented in **`engine/core/src/map_view`**, solely from
`docs/specs/map-view.md`. Foreign traits expose raw data and individual primitives;
core code owns palettes, resampling, lot means, capacity ratios, icons, marked-cell
selection, building reconciliation and update commitment. No plugin/C ABI wiring.

## Owner closure

All rows belong to `cSC4WinMapView`; code paths below are relative to this module.
Tests are in `engine/conformance/tests/map_view.rs`; `core::` means `tests.rs` and
`v17`–`v33` mean individually numbered tests in
`engine/conformance/tests/map_view/settled_vectors.rs`.

| Spec section | Implemented code item | Test / spec boundary |
|---|---|---|
| §0 Summary | `execution.rs: select_with, draw_with, tick_with` | `mayor_rating_owner_builds_and_draws_both_extremes` |
| §1 Window | `mod.rs: init, activate, deactivate`; `execution.rs: init_with`; `window.rs: window_message, show_with` | `window_lifecycle_commands_and_open_panel_matrix_are_explicit`, `qv12_init_false_primitives_and_missing_server_latch`; panels Unsupported per QV3 |
| §1.1 Construction/repetition | `mod.rs: from_clock, new, init, hook_update_messages, unhook_update_messages` | `v30_init_latch_and_accumulated_hooks`, `v33_notification_compares_both_records` |
| §2 Exemplar | `exemplar.rs: selection_fields`; `execution.rs: render_settings`; `highlights.rs: HighlightStyle` | `core::raw_fields_use_only_documented_defaults_and_ranges`, `filter_options_terrain_mapping_force_bases_and_weather_keep_raw_geometry` |
| §2.1 Absence/identity | `exemplar.rs: find, exemplar_key, terrain_mode, travel_types`; `mod.rs: store_selection` | `v27_travel_narrowing_and_absence_retention`, `v28_absence_and_terrain`, `v29_wrapped_identity`; unsafe aliases/allocations diagnosed, never native false |
| §3 Choosing a view | `execution.rs: select_with`; `window.rs: button_with, reset_with` | `qv12_selection_guards_and_visible_reset`, `direct_buttons_aliases_tutorial_reset_and_legend` |
| §3.1 Selection/failures | `mod.rs: begin_selection, store_selection`; `execution.rs: select_with` | `qv12_failed_button_replacement_remembers_requested_legend`, `qv12_custom_initialization_factory_and_child_false_are_local`; none/reset Unsupported per QV1 |
| §3.2 Buttons | `window.rs: button_action, canonical_button, button_with` | `direct_buttons_aliases_tutorial_reset_and_legend`; group/panel policy Unsupported per QV3 |
| §3.3 Reset | `window.rs: reset_with` | `qv12_selection_guards_and_visible_reset`; unknown group Unsupported per QV3 |
| §4 Drawing | `execution.rs: draw_with, draw_body` | `v32_update_commit_points`, `mayor_rating_owner_builds_and_draws_both_extremes` |
| §4.1 Sources | `source.rs: select_source`; `execution.rs: draw_body` | `core::every_raw_source_slot_and_empty_range_has_a_literal_fixture`; traffic QV2 and developer default/debug QV11 remain Unsupported |
| §4.1 Layer 5 | `settled.rs: capacity_percentage, word_at`; `execution.rs: draw_body` | `v25_layer_five_signed_population_unsigned_capacity`; subsequent signed-16 display remains QV5 |
| §4.2 Indices | `palette.rs: u8_index, i8_index, float_index, emit_colors` | `core::full_eight_bit_domains_use_distinct_supplied_entries`, `conformance_index_conflict_is_explicit_qv5`; contradictory 16-bit rules Unsupported per QV5 |
| §4.2.1 Float domains | `palette.rs: float_index` | `v22_float_low_byte_and_masked_invalid` |
| §4.3 Palettes/resampling | `palette.rs: build_palettes, premultiply_argb`; `settled.rs: resample_grid` | `v19_ramp_endpoints_duplicates_and_supplied_order`, `v20_channel_half_rounding_and_premultiplication`, `v21_center_aligned_resampling_and_two_stage_rounding` |
| §4.3 Lots/crime | `settled.rs: block_lots, incident_center, blend_incident, blit_incident`; `execution.rs: draw_body` | `v24_lot_masks_means_and_block_presence`, `v26_crime_placement_rotation_clipping_and_order`; missing general lot-mean paragraph filed as QV15; literal vector-compatible choice documented in code |
| §4.3 Marked cells | `settled.rs: marked_cells`; `execution.rs: draw_with, draw_body`; `overlay.rs: OverlayState` | `v23_overlay_alpha_inclusion_coordinates_and_empty_replacement`; raw coordinates/straight colours/sizes sent to separate marked-cell owner; damaged-road selection remains QV2 |
| §5 Highlights | `highlights.rs: highlight_request, collect_highlights`; `execution.rs: poll_highlights` | `core::all_supported_highlight_kinds_select_distinct_raw_inputs_and_slots`, `qv13_hotspot_position_and_strength_independence` |
| §5.0 Building merge | `execution.rs: poll_highlights, reconcile_buildings` | `v17_building_retention_and_signed_pointer_order`, `v18_multiplicity_zero_radius_and_empty_reconciliation` |
| §5.1 Effects/hotspot | `highlights.rs: HighlightRecord::{create_effect,add,remove}`; `execution.rs: poll_highlights` | `qv13_hotspot_two_stage_effect_creation`, `qv13_hotspot_repeated_removal_empty_name_and_kind_zero`, `qv13_hotspot_creation_failures_do_not_abort_selection_or_bitmap` |
| §5.2 Highlight colours | `highlights.rs: decorate_highlights, footprint_rect, ring_arguments, paint_ring, blend_highlight` | `qv13_independent_footprints_rings_and_effect_transforms`, `qv13_ring_blend_clip_and_independent_acquisitions`; literal formula implemented, edge-pixel precision remains QV14 |
| §6 Legend | `window.rs: Legend::from_exemplar, display_legend` | `qv12_legend_scalar_default_lookup_failure_and_title_query_skip`, `qv12_legend_unequal_rows_zero_colors_and_control_skips`; unsafe reads diagnosed |
| §7 Updates | `source.rs: subscription`; `mod.rs: notify_values, hook_update_messages`; `execution.rs: tick_with` | `core::messages_and_source_changes_follow_exact_subscription_table`, `v33_notification_compares_both_records`, `pollution_throttle_vector_reaches_due_tick_without_faking_conversion` |
| §7.1 Clock/commit | `execution.rs: draw_with, tick_with`; `mod.rs: from_clock` | `v31_clock_wrap_regression_and_thresholds`, `v32_update_commit_points` |
| §8 Quirks | `source.rs: select_source`; `exemplar.rs: selection_fields`; `execution.rs: tick_with` | `residential_desirability_selection_commits_but_draws_nothing`, `core::raw_fields_use_only_documented_defaults_and_ranges` |
| §9 Open spec work | `mod.rs: Unsupported` | QV1, QV2, QV3, QV5, QV11, QV14 remain spec-unsettled; QV15 identifies truncated prose. No game shadow claimed |
| §10 Conformance | `engine/conformance/tests/map_view.rs` and `map_view/settled_vectors.rs` | §10.1 QV5 diagnostic; §10.2 mayor extremes; §10.3 throttle/QV5; §10.4 empty replacement; §10.5–10 qv12 tests; §10.11–16 qv13 tests; §10.17–33 individually numbered v17–v33 |

## Adapter contract

`from_clock` samples a fresh CRT clock; legacy `new` accepts an explicit raw
clock and ignores its second argument. `notify_values` takes both message records'
second words, never a retained notification baseline. `Renderer::clock_ms` is
sampled at Update tail, independently of the tick eligibility sample.

Active Update clears dirty at entry. Missing child preserves the bitmap and
commits the clock; missing developer preserves clock/image; inactive Update
preserves all state. Ordinary false presentation/custom results reach the tail.
Faults before the tail do not imply rollback or retry. Missing ramp retains both
tables. First use without initialized entries and unsafe malformed ramps produce
portable diagnostics, not fabricated native false returns.

The renderer receives marked-cell coordinates, straight colours and world cell
sizes. Terrain/water/frustum/diagonal/depth behavior belongs to the separate
marked-cell owner in overlay-views §2. Acquired occupant and created icon
references cross explicit retain/release primitives. The legacy intent-only
`update`, `tick`, `set_data_view` cannot execute foreign calls; use `_with` APIs.
